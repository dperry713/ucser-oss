use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Adversarial Test Harness...");
    let client = reqwest::Client::new();
    let url = "http://127.0.0.1:3000/api/dag";

    // 1. Policy Rejection Path (Injection)
    println!("\n[Test 1] Injecting Malicious Policy Violation...");
    let malicious_dag = json!({
        "dag_id": "adversarial-001",
        "actor_identity": "urn:ucser:actor:malicious",
        "signature": "invalid-sig",
        "tasks": [
            {
                "id": "exploit",
                "os": "linux",
                "shell": "bash",
                "cmd": "rm -rf /",
                "args": [],
                "env_vars": {},
                "dependencies": [],
                "risk": "high"
            }
        ],
        "edges": []
    });

    let res = client.post(url).json(&malicious_dag).send().await?;
    println!("Response: {}", res.status());

    // 2. Corrupted DAG (Missing fields, circular dependency)
    println!("\n[Test 2] Injecting Circular Dependency DAG...");
    let circular_dag = json!({
        "dag_id": "adversarial-002",
        "actor_identity": "urn:ucser:actor:sysadmin-01",
        "signature": "valid-sig",
        "tasks": [
            {
                "id": "A",
                "os": "linux",
                "shell": "bash",
                "cmd": "echo A",
                "args": [],
                "env_vars": {},
                "dependencies": [],
                "risk": "low"
            },
            {
                "id": "B",
                "os": "linux",
                "shell": "bash",
                "cmd": "echo B",
                "args": [],
                "env_vars": {},
                "dependencies": [],
                "risk": "low"
            }
        ],
        "edges": [
            ["A", "B"],
            ["B", "A"]
        ]
    });

    let res2 = client.post(url).json(&circular_dag).send().await?;
    println!("Response: {}", res2.status());

    // 3. Simulated Node Failure
    println!("\n[Test 3] Injecting Node Failure Simulation Payload...");
    let fail_dag = json!({
        "dag_id": "adversarial-003",
        "actor_identity": "urn:ucser:actor:sysadmin-01",
        "signature": "valid-sig",
        "tasks": [
            {
                "id": "fail_task",
                "os": "windows",
                "shell": "powershell",
                "cmd": "exit 1", // Causes execution failure
                "args": [],
                "env_vars": {},
                "dependencies": [],
                "risk": "low"
            }
        ],
        "edges": []
    });

    let res3 = client.post(url).json(&fail_dag).send().await?;
    println!("Response: {}", res3.status());

    println!("\nAdversarial injections complete! Check the audit log for resilient behavior.");
    Ok(())
}
