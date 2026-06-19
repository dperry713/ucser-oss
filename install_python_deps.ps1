$ErrorActionPreference = "Stop"

Write-Host "Setting up Python virtual environment..."
python -m venv venv
.\venv\Scripts\Activate.ps1

Write-Host "Installing dependencies..."
pip install grpcio grpcio-tools

Write-Host "Compiling protobufs for adapters..."
python -m grpc_tools.protoc -I./proto --python_out=./adapters/windows --grpc_python_out=./adapters/windows ./proto/execution.proto
python -m grpc_tools.protoc -I./proto --python_out=./adapters/linux --grpc_python_out=./adapters/linux ./proto/execution.proto

Write-Host "Done."
