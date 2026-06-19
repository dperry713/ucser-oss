# UCSER Compliance Platform

## 1. Overview
UCSER (Unified Cross-Shell Execution Runtime) is a distributed execution engine built for DevSecOps, regulated automation, and strict compliance environments.

It solves three critical problems:
- **Deterministic cross-shell execution**: Unifying Windows (PowerShell) and Linux (Bash) under a single execution graph.
- **Auditability**: Emitting structured, tamper-evident execution logs in real-time.
- **Compliance Enforcement**: Native Open Policy Agent (Rego) policy-as-code guarantees.

## 2. Architecture

```
[ CLI / CI/CD ]
       │
 (REST / HTTP)
       │
  [ Axum API ]
       │
[ DAG Engine ] ─── [ Policy Engine (Rego) ]
       │
  [ Executors ] ── (gRPC) ── [ Adapters ]
       │
 [ Audit Log ]
```

## 3. Quick Start

### Build the system
```bash
cargo build --release
```

### Start the single-node demo environment
```bash
./target/release/ucser-kernel --single-node
```

### Submit the reference workload
```bash
./target/release/ucser-cli submit examples/reference_workload/phi_pipeline.json
```

### Monitor and Audit
```bash
./target/release/ucser-cli status phi-001
./target/release/ucser-cli audit phi-001
```

## 4. Compliance Model
UCSER is explicitly built for highly regulated environments.
- **HIPAA**: Strict audit trailing and execution blocklists prevent unapproved data exfiltration or access payloads.
- **SOC 2**: The immutable execution runtime guarantees system integrity and structured NDJSON logging ensures observability controls.

## 5. API Reference

- `POST /api/dag`: Submits a new DAG payload.
- `GET /api/status`: Retrieves the current health and metrics of the system.
- `GET /api/audit`: (Roadmap) Streams the NDJSON audit log over HTTP.
