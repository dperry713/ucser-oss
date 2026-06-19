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
        print(f"Dispatching to Bash: {request.command} {' '.join(request.args)}")
        
        # Build the bash command
        bash_command = ["bash", "-c", f"{request.command} " + " ".join(request.args)]
        
        try:
            result = subprocess.run(
                bash_command,
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
    server.add_insecure_port('[::]:50052')
    server.start()
    print("Bash Adapter listening on port 50052")
    server.wait_for_termination()

if __name__ == '__main__':
    serve()
