import grpc
import os
import sys
from concurrent import futures
import subprocess
import time
import shutil
import signal

# Import generated protobuf files
import execution_pb2
import execution_pb2_grpc
from grpc_health.v1 import health
from grpc_health.v1 import health_pb2
from grpc_health.v1 import health_pb2_grpc

# Command allow-list
ALLOWED_COMMANDS = {"echo", "ls", "pwd", "whoami", "date", "cat", "uname", "hostname", "sleep"}

def is_safe_command(cmd: str) -> bool:
    cleaned = cmd.strip()
    if not cleaned:
        return False
    first_word = cleaned.split()[0]
    first_word = os.path.basename(first_word)
    return first_word in ALLOWED_COMMANDS

def sanitize_arguments(args):
    forbidden = {";", "&", "|", "$", "`", ">", "<", "\n", "\r"}
    for arg in args:
        if any(char in arg for char in forbidden):
            raise ValueError(f"Security Alert: Argument '{arg}' contains forbidden shell characters")

class ExecutionAdapterServicer(execution_pb2_grpc.ExecutionAdapterServicer):
    def DispatchCommand(self, request, context):
        cmd = request.command.strip()
        print(f"Dispatching request: {cmd} {' '.join(request.args)}")
        
        if not is_safe_command(cmd):
            return execution_pb2.CommandResponse(
                execution_id=request.execution_id,
                exit_code=-1,
                stdout="",
                stderr=f"Security Violation: Command '{cmd}' is blocked by adapter allow-list"
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

        # Build execution path
        if shutil.which("bwrap"):
            print("Executing in secure Bubblewrap sandbox...")
            bash_command = [
                "bwrap",
                "--ro-bind", "/usr", "/usr",
                "--ro-bind", "/lib", "/lib",
                "--ro-bind", "/lib64", "/lib64",
                "--ro-bind", "/bin", "/bin",
                "--proc", "/proc",
                "--dev", "/dev",
                "--unshare-all",
                "--setenv", "PATH", "/usr/bin:/bin",
                "--",
                cmd
            ] + list(request.args)
        else:
            print("Bubblewrap not found. Falling back to standard execution...")
            bash_command = ["bash", "-c", f"{cmd} " + " ".join(request.args)]

        try:
            result = subprocess.run(
                bash_command,
                capture_output=True,
                text=True,
                env={**os.environ, **request.env_vars},
                timeout=30,
                check=False
            )
            return execution_pb2.CommandResponse(
                execution_id=request.execution_id,
                exit_code=result.returncode,
                stdout=result.stdout,
                stderr=result.stderr
            )
        except subprocess.TimeoutExpired:
            return execution_pb2.CommandResponse(
                execution_id=request.execution_id,
                exit_code=-1,
                stdout="",
                stderr="Execution Timeout: Command took longer than 30 seconds"
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

    port = 50052
    server.add_insecure_port(f'[::]:{port}')
    server.start()
    print(f"Linux/Bash Adapter serving on port {port}...")

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
