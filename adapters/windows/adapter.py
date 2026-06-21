import grpc
import os
import sys
from concurrent import futures
import subprocess
import time
import signal

# Import generated protobuf files
import execution_pb2
import execution_pb2_grpc
from grpc_health.v1 import health
from grpc_health.v1 import health_pb2
from grpc_health.v1 import health_pb2_grpc

# Command allow-list for Windows
ALLOWED_COMMANDS = {"Write-Output", "Get-ChildItem", "Get-Date", "echo", "ls", "cat", "Get-Process", "Get-Service", "ping", "exit"}

def is_safe_command(cmd: str) -> bool:
    cleaned = cmd.strip()
    if not cleaned:
        return False
    # First word of command
    first_word = cleaned.split()[0]
    # Remove extension if any (e.g. ping.exe -> ping)
    first_word = os.path.splitext(os.path.basename(first_word))[0]
    return first_word in ALLOWED_COMMANDS

def sanitize_arguments(args):
    forbidden = {";", "&", "|", "$", "`", ">", "<", "\n", "\r"}
    for arg in args:
        if any(char in arg for char in forbidden):
            raise ValueError(f"Security Alert: Argument '{arg}' contains forbidden shell characters")

def apply_job_object_limits(proc_handle):
    if sys.platform != "win32":
        return
    try:
        import ctypes
        
        kernel32 = ctypes.windll.kernel32
        
        # Create Job Object
        h_job = kernel32.CreateJobObjectW(None, None)
        if not h_job:
            print("Warning: Failed to create Job Object")
            return
            
        # Define Structs
        class IO_COUNTERS(ctypes.Structure):
            _fields_ = [
                ("ReadOperationCount", ctypes.c_uint64),
                ("WriteOperationCount", ctypes.c_uint64),
                ("OtherOperationCount", ctypes.c_uint64),
                ("ReadTransferCount", ctypes.c_uint64),
                ("WriteTransferCount", ctypes.c_uint64),
                ("OtherTransferCount", ctypes.c_uint64),
            ]
            
        class JOBOBJECT_BASIC_LIMIT_INFORMATION(ctypes.Structure):
            _fields_ = [
                ("PerProcessUserTimeLimit", ctypes.c_int64),
                ("PerJobUserTimeLimit", ctypes.c_int64),
                ("LimitFlags", ctypes.c_uint32),
                ("MinimumWorkingSetSize", ctypes.c_size_t),
                ("MaximumWorkingSetSize", ctypes.c_size_t),
                ("ActiveProcessLimit", ctypes.c_uint32),
                ("Affinity", ctypes.c_size_t),
                ("PriorityClass", ctypes.c_uint32),
                ("SchedulingClass", ctypes.c_uint32),
            ]
            
        class JOBOBJECT_EXTENDED_LIMIT_INFORMATION(ctypes.Structure):
            _fields_ = [
                ("BasicLimitInformation", JOBOBJECT_BASIC_LIMIT_INFORMATION),
                ("IoInfo", IO_COUNTERS),
                ("ProcessMemoryLimit", ctypes.c_size_t),
                ("JobMemoryLimit", ctypes.c_size_t),
                ("PeakProcessMemoryLimit", ctypes.c_size_t),
                ("PeakJobMemoryLimit", ctypes.c_size_t),
            ]
            
        # Limit memory to 256MB
        info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
        info.BasicLimitInformation.LimitFlags = 0x100 # JOB_OBJECT_LIMIT_PROCESS_MEMORY
        info.ProcessMemoryLimit = 256 * 1024 * 1024 # 256MB
        
        # Set information
        JOB_OBJECT_INFO_CLASS = 9 # JobObjectExtendedLimitInformation
        success = kernel32.SetInformationJobObject(
            h_job,
            JOB_OBJECT_INFO_CLASS,
            ctypes.byref(info),
            ctypes.sizeof(info)
        )
        if not success:
            print("Warning: Failed to set Job Object limits")
            return
            
        # Assign process to Job Object
        success = kernel32.AssignProcessToJobObject(h_job, int(proc_handle))
        if success:
            print("Successfully assigned process to Job Object sandbox")
        else:
            print("Warning: Failed to assign process to Job Object")
    except Exception as e:
        print(f"Failed to apply Job Object limits: {e}")

class ExecutionAdapterServicer(execution_pb2_grpc.ExecutionAdapterServicer):
    def DispatchCommand(self, request, context):
        cmd = request.command.strip()
        print(f"Dispatching PowerShell request: {cmd} {' '.join(request.args)}")
        
        if not is_safe_command(cmd):
            return execution_pb2.CommandResponse(
                execution_id=request.execution_id,
                exit_code=-1,
                stdout="",
                stderr=f"Security Violation: Command '{cmd}' is blocked by Windows adapter allow-list"
            )
            
        try:
            sanitize_arguments(request.args)
        except ValueError as e:
            return execution_pb2.CommandResponse(
                execution_id=request.execution_id,
                exit_code=-1,
                stdout="",
                stderr=str(e)
            )

        ps_command = ["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", cmd] + list(request.args)

        try:
            # Start process asynchronously to apply Job Object limits before wait
            proc = subprocess.Popen(
                ps_command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                env={**os.environ, **request.env_vars}
            )
            
            # Apply Windows Job limits
            if sys.platform == "win32":
                apply_job_object_limits(proc._handle)
                
            stdout, stderr = proc.communicate(timeout=30)
            return execution_pb2.CommandResponse(
                execution_id=request.execution_id,
                exit_code=proc.returncode,
                stdout=stdout,
                stderr=stderr
            )
        except subprocess.TimeoutExpired:
            proc.kill()
            stdout, stderr = proc.communicate()
            return execution_pb2.CommandResponse(
                execution_id=request.execution_id,
                exit_code=-1,
                stdout=stdout,
                stderr=f"Execution Timeout: PowerShell command took longer than 30 seconds. {stderr}"
            )
        except Exception as e:
            return execution_pb2.CommandResponse(
                execution_id=request.execution_id,
                exit_code=-1,
                stdout="",
                stderr=str(e)
            )

def serve():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    execution_pb2_grpc.add_ExecutionAdapterServicer_to_server(ExecutionAdapterServicer(), server)
    
    # Add gRPC health check service
    health_servicer = health.HealthServicer()
    health_pb2_grpc.add_HealthServicer_to_server(health_servicer, server)
    health_servicer.set("", health_pb2.HealthCheckResponse.SERVING)

    port = 50051
    server.add_insecure_port(f'[::]:{port}')
    server.start()
    print(f"Windows/PowerShell Adapter serving on port {port}...")

    # Graceful shutdown handler
    def handle_shutdown(signum, frame):
        print("Shutdown signal received. Stopping server...")
        server.stop(0)
        sys.exit(0)

    signal.signal(signal.SIGINT, handle_shutdown)
    signal.signal(signal.SIGTERM, handle_shutdown)

    server.wait_for_termination()

if __name__ == '__main__':
    serve()
