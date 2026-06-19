import grpc
import os
from concurrent import futures
import subprocess
import time

# Assuming generated protobuf files are available
import execution_pb2
import execution_pb2_grpc

class ExecutionAdapterServicer(execution_pb2_grpc.ExecutionAdapterServicer):
    def DispatchCommand(self, request, context):
        print(f"Dispatching to PowerShell: {request.command} {' '.join(request.args)}")
        
        # Build the powershell command
        ps_command = ["powershell.exe", "-NoProfile", "-NonInteractive", "-Command", request.command] + list(request.args)
        
        try:
            result = subprocess.run(
                ps_command,
                capture_output=True,
                text=True,
                env={**os.environ, **request.env_vars},
                check=False
            )
            return execution_pb2.CommandResponse(
                execution_id=request.execution_id,
                exit_code=result.returncode,
                stdout=result.stdout,
                stderr=result.stderr
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
    server.add_insecure_port('[::]:50051')
    server.start()
    print("PowerShell Adapter listening on port 50051")
    server.wait_for_termination()

if __name__ == '__main__':
    serve()
