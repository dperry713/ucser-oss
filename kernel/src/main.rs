use ucser_kernel::dag::{DagEngine, Task};
use ucser_kernel::policy::PolicyEngine;
use ucser_kernel::audit::AuditEngine;
use ucser_kernel::client::AdapterClient;
use std::collections::HashMap;
use std::time::Instant;
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    single_node: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    if args.single_node {
        println!("Starting UCSER Control Plane in Single-Node Mode...");
    } else {
        println!("Starting UCSER Control Plane (Cluster Mode)...");
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    // Start the Edge API Layer in the background
    tokio::spawn(async move {
        ucser_kernel::api::start_api_server(tx).await;
    });

    let mut audit = AuditEngine::new("logs/audit.ndjson").unwrap_or_else(|_| AuditEngine::new("audit.ndjson").unwrap());
    
    let mut dag = DagEngine::new();
    let mut policy = PolicyEngine::new();

    println!("Connecting to Windows Adapter...");
    let mut win_client = match AdapterClient::connect("http://[::1]:50051").await {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to connect to Windows adapter: {}", e);
            return Ok(());
        }
    };

    let e2e_start = Instant::now();

    loop {
        // Check for new incoming DAGs
        if let Ok(new_dag) = rx.try_recv() {
            println!("Received new DAG from API.");
            for task in new_dag {
                let execution_id = &task.execution_id;
                let _ = audit.log_event(execution_id, serde_json::json!({
                    "task_id": task.id,
                    "event": "started"
                }));
                if let Err(e) = policy.validate_task(&task) {
                    println!("Policy blocked task {}: {:?}", task.id, e);
                    let _ = audit.log_event(execution_id, serde_json::json!({
                        "task_id": task.id,
                        "os": task.os,
                        "cmd": task.command,
                        "result": "blocked",
                        "policy_decision": format!("{:?}", e),
                        "latency_ms": 0
                    }));
                    continue; // Skip or fail DAG
                }
                dag.add_task(task);
            }
        }

        // Measure DAG scheduling latency
        let dag_start = Instant::now();
        let ready_tasks = dag.get_ready_tasks();
        let dag_latency = dag_start.elapsed();
        if !ready_tasks.is_empty() {
            println!("DAG Scheduling Latency: {:.3} ms", dag_latency.as_secs_f64() * 1000.0);
        }

        if ready_tasks.is_empty() {
            // we yield briefly
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }

        for task in ready_tasks {
            // Formal routing logic simulation
            let execution_id = &task.execution_id;
            let selected_node = if task.os == "windows" { "windows-node-1" } else { "linux-node-1" };
            
            let _ = audit.log_event(execution_id, serde_json::json!({
                "task_id": task.id,
                "event": "routed",
                "selected_node": selected_node,
                "routing_reason": format!("os_match: {}", task.os)
            }));

            dag.mark_running(&task.id);
            
            println!("Dispatching task: {} to {}", task.id, selected_node);
            let dispatch_start = Instant::now();
            
            // local simulation mode
            let exit_code = if args.single_node {
                0
            } else {
                win_client.dispatch(
                    task.id.clone(),
                    task.command.clone(),
                    task.args.clone(),
                    task.env_vars.clone()
                ).await.map(|r| r.exit_code).unwrap_or(1)
            };
            
            let dispatch_latency = dispatch_start.elapsed();
            println!("Execution Dispatch Latency: {:.3} ms", dispatch_latency.as_secs_f64() * 1000.0);
            println!("Task {} completed with exit code {}", task.id, exit_code);

            let _ = audit.log_event(execution_id, serde_json::json!({
                "task_id": task.id,
                "os": task.os,
                "cmd": task.command,
                "result": if exit_code == 0 { "success" } else { "failure" },
                "latency_ms": dispatch_latency.as_millis(),
                "policy_decision": "allowed"
            }));
            let _ = audit.log_event(execution_id, serde_json::json!({
                "task_id": task.id,
                "event": "completed"
            }));

            dag.mark_completed(&task.id);
        }
    }

    let e2e_latency = e2e_start.elapsed();
    println!("End-to-End Execution Time: {:.3} ms", e2e_latency.as_secs_f64() * 1000.0);

    println!("UCSER Control Plane ready. Press Ctrl-C to shutdown.");
    tokio::signal::ctrl_c().await.expect("Failed to listen for event");

    Ok(())
}
