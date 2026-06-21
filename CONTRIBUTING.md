# Contributing to UCSER OSS

Welcome! We are excited that you want to contribute to UCSER (Unified Cross-Shell Execution Runtime). This guide will help you get set up locally and describe our development process.

## Code of Conduct
By participating in this project, you agree to abide by our project's [Code of Conduct](CODE_OF_CONDUCT.md).

## Getting Started

### Prerequisites
1. **Rust**: You will need the latest stable version of Rust.
2. **Python**: Python 3.10+ is required to run and test the execution adapters.
3. **Protocol Buffers Compiler (`protoc`)**: Since we use gRPC for communication between the control plane and adapters, you must have `protoc` installed on your machine.
   - On Windows: Run `install_protoc.ps1`.
   - On Linux: Run `sudo apt-get install protobuf-compiler`.

### Setup
1. Clone the repository:
   ```bash
   git clone https://github.com/dperry713/ucser-oss.git
   cd ucser-oss
   ```
2. Build the workspace:
   ```bash
   # Set the PROTOC environment variable to point to the compiler if not in PATH
   cargo build
   ```
3. Set up the Python virtual environment and generate gRPC stubs:
   ```powershell
   # On Windows
   powershell -File install_python_deps.ps1
   ```

## Development Guidelines

### Rust Guidelines
- Enforce standard formatting: Run `cargo fmt` before committing.
- Resolve all lints: Run `cargo clippy --all-targets -- -D warnings`.
- Ensure tests pass: Run `cargo test --all`.

### Policy Customization
Policies reside in `kernel/policies/` in Rego format. Make sure to define proper unit tests in `kernel/src/integration_tests.rs` for any new policies.

## How to Submit Changes
1. Fork the repository and create your branch from `main`.
2. Commit your changes with clear, descriptive commit messages.
3. Open a Pull Request (PR) against the `main` branch.
4. Ensure the GitHub Actions CI pipeline passes successfully.
