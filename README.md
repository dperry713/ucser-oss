UCSER — Unified Cross-System Execution Runtime

Deterministic, auditable workflow orchestration across Linux and Windows systems.

UCSER is a cross-platform execution engine that compiles structured workflows into deterministic DAGs, executes them across heterogeneous systems, and produces tamper-evident audit records.

Overview

UCSER solves a core problem in infrastructure and compliance:

How do you execute workflows across systems in a way that is reproducible, secure, and auditable?

This system provides:

Deterministic workflow execution (same input → same result)
Cross-platform orchestration (Linux + Windows)
Policy enforcement before execution
Full execution trace logging
Replayable audit records
Core Concepts
1. Workflow → DAG Compilation

User-defined workflows are compiled into directed acyclic graphs (DAGs):

{
  "workflow_id": "patch-check-001",
  "nodes": [
    {"id": "inventory", "type": "linux_cmd"},
    {"id": "scan", "type": "security_check"},
    {"id": "report", "type": "aggregate"}
  ],
  "edges": [
    ["inventory", "scan"],
    ["scan", "report"]
  ]
}
2. Deterministic Execution Engine
Each node executes with controlled inputs
Execution order is strictly defined by DAG topology
Results are normalized for reproducibility
3. Cross-Platform Adapters

UCSER executes workflows across systems via adapters:

Linux adapter (bash / system tools)
Windows adapter (PowerShell / native APIs)

Adapters provide a uniform execution interface across OS boundaries.

4. Policy Enforcement Layer

Before execution, workflows are validated against policies:

Access control (who can run what)
Allowed command sets
Execution constraints
Logging requirements
5. Audit & Replay System

Every execution produces a structured record:

{
  "execution_id": "exec-001",
  "status": "completed",
  "steps": [
    {"node": "inventory", "status": "ok"},
    {"node": "scan", "status": "ok"},
    {"node": "report", "status": "ok"}
  ]
}

Executions can be:

Replayed deterministically
Verified for integrity
Used for compliance audits
Architecture
5
CLI / API
   ↓
Workflow Compiler
   ↓
DAG Scheduler
   ↓
Execution Engine
   ↓
Adapters (Linux / Windows)
   ↓
Audit Ledger
Example Execution Flow
Submit Workflow
   ↓
Validate Input
   ↓
Policy Check
   ↓
Compile DAG
   ↓
Execute Nodes
   ↓
Collect Results
   ↓
Generate Audit Record
API
Submit Workflow
POST /workflows
Execute Workflow
POST /execute
Execution Status
GET /execution/{execution_id}
Replay Execution
POST /replay/{execution_id}
AI-Assisted Workflow Generation (Optional)

UCSER can integrate with local LLMs such as llama.cpp or LM Studio to convert natural language into workflows.

Example:

Input:

"Run a security scan on all Linux servers and generate a report"

Generated DAG:

{
  "nodes": ["inventory", "scan", "report"]
}

Execution remains deterministic — AI is only used for compilation, not runtime control.

Tech Stack
Backend: FastAPI
Data: SQLite / DuckDB
Graph: NetworkX or custom engine
Runtime: Python + Rust components
Adapters: Bash / PowerShell
Serialization: Protobuf / JSON
Key Features
Deterministic DAG execution
Cross-platform orchestration
Policy-based execution control
Replayable execution traces
Audit-ready logs
Extensible adapter system
Roadmap
Phase 1 (Current)
DAG execution engine
Basic adapters
Execution tracking
Phase 2
Policy engine (SOC2 / HIPAA mapping)
Structured audit export
Phase 3
Cryptographic execution signing
Tamper-evident audit ledger
Phase 4
Distributed execution (multi-node)
Agent-based execution model
Why This Project Matters

Most workflow systems optimize for flexibility.

UCSER optimizes for:

Determinism
Auditability
Security
Compliance

This makes it applicable to:

Compliance automation (SOC2, HIPAA)
Security operations
Infrastructure orchestration
Regulated environments
Getting Started
git clone https://github.com/dperry713/ucser-oss
cd ucser-oss

# Python environment
python -m venv venv
source venv/bin/activate  # or Windows equivalent

pip install -r requirements.txt

# Run API
uvicorn app.main:app --reload
Example Use Cases
Run compliance checks across servers
Execute secure automation workflows
Generate audit logs for infrastructure changes
Reproduce execution failures deterministically
Resume Impact Statement

This project demonstrates:

Distributed systems design
Workflow orchestration (DAG execution)
Cross-platform runtime engineering
Security and policy enforcement
Audit and compliance architecture
License

MIT License

Final Note

This project is designed to model real-world infrastructure systems where reproducibility, traceability, and control are required.

It is not a task runner.

It is an execution system with audit guarantees.
