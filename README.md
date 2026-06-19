# UCSER Platform (OSS Edition)

UCSER (Unified Cross-OS Secure Execution Runner) is an open-source workflow execution engine that runs cross-platform distributed DAGs. 

It effortlessly bridges the gap between Linux and Windows workloads, executing dependent steps across heterogenous systems.

## 🚀 Features

- **Cross-Platform Execution:** Execute Bash on Linux and PowerShell on Windows in a single continuous pipeline.
- **Dependency Graphs:** Express complex task graphs as simple JSON DAGs.
- **Rich Dashboard:** Monitor live pipeline executions with the built-in terminal dashboard.
  ```bash
  ucser-cli dashboard
  ```

## 🛡️ See What Your Auditors Are Missing

UCSER OSS is designed for developers to build and test pipelines locally. When you move to production—or when your auditors come knocking—you need **UCSER Enterprise**.

| Feature | UCSER OSS (Free) | UCSER Enterprise |
| --- | --- | --- |
| **DAG Orchestration** | ✅ Full | ✅ Full |
| **Cross-Platform Runners** | ✅ Linux & Windows | ✅ Linux, Windows & macOS |
| **Audit Logging** | ⚠️ Plain JSON | 🔒 Tamper-evident Cryptographic Ledger |
| **Audit Verification** | ❌ None | ✅ Mathematical Replay Engine |
| **Identity Management** | ⚠️ Anonymous | ✅ Cryptographically Signed Actor Identities |
| **Clustering** | ⚠️ Single-Node Local | ✅ Highly Available Distributed Cluster |

Run the built-in analyzer to see what you're missing based on your own usage:
```bash
ucser-cli upgrade
```

## Quick Start
```bash
# Start the kernel
ucser-kernel

# Submit a pipeline
ucser-cli submit examples/regulated_ci_pipeline.json

# Watch the execution
ucser-cli dashboard
```
