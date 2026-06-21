use ucser_kernel::dag::{DagEngine, Task};
use ucser_kernel::policy::PolicyEngine;
use ucser_kernel::audit::AuditEngine;
use ucser_kernel::client::AdapterClient;
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    single_node: bool,
}

enum SchedulerEvent {
    NewDag(Vec<Task>),
    TaskFinished {
        task_id: String,
        exit_code: i32,
        duration_ms: u128,
        stdout: String,
        stderr: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    if args.single_node {
        println!("Starting UCSER Control Plane in Single-Node Mode...");
    } else {
        println!("Starting UCSER Control Plane (Cluster Mode)...");
    }

    let mut audit = AuditEngine::new("logs/audit.ndjson").or_else(|_| AuditEngine::new("audit.ndjson"))?;
    let mut dag = DagEngine::new();
    let mut policy = PolicyEngine::new();

    // Connect to adapters in cluster mode
    let win_client = if args.single_node {
        None
    } else {
        println!("Connecting to Windows Adapter on port 50051...");
        match AdapterClient::connect("http://[::1]:50051").await {
            Ok(c) => Some(c),
            Err(e) => {
                println!("Warning: Failed to connect to Windows adapter: {}", e);
                None
            }
        }
    };

    let linux_client = if args.single_node {
        None
    } else {
        println!("Connecting to Linux Adapter on port 50052...");
        match AdapterClient::connect("http://[::1]:50052").await {
            Ok(c) => Some(c),
            Err(e) => {
                println!("Warning: Failed to connect to Linux adapter: {}", e);
                None
            }
        }
    };

    // Distributed Mode: etcd coordination
    let mut kv = if args.single_node {
        None
    } else {
        println!("Connecting to etcd on localhost:2379...");
        match ucser_kernel::kv::KvStore::connect(&["http://127.0.0.1:2379"]).await {
            Ok(k) => {
                println!("Successfully connected to etcd.");
                Some(k)
            }
            Err(e) => {
                println!("Warning: etcd not available, running without distributed coordination: {}", e);
                None
            }
        }
    };

    if let Some(ref mut store) = kv {
        let node_id = "node-1";
        if let Err(e) = store.put(format!("/nodes/{}", node_id), "active").await {
            println!("Warning: Failed to register node in etcd: {}", e);
        } else {
            println!("Registered node {} in etcd.", node_id);
        }
        
        match store.get("/leader").await {
            Ok(Some(leader)) => {
                let leader_str = String::from_utf8_lossy(&leader);
                println!("Current cluster leader is: {}", leader_str);
            }
            Ok(None) => {
                println!("No leader found. Claiming leadership...");
                if let Err(e) = store.put("/leader", node_id).await {
                    println!("Warning: Failed to claim leadership: {}", e);
                } else {
                    println!("Successfully became the cluster leader!");
                }
            }
            Err(e) => {
                println!("Warning: Failed to query leader from etcd: {}", e);
            }
        }
    }

    // Set up scheduler communication channel
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<SchedulerEvent>(100);

    // Start API server in the background
    let api_tx = event_tx.clone();
    tokio::spawn(async move {
        let (api_internal_tx, mut api_internal_rx) = tokio::sync::mpsc::channel::<Vec<Task>>(32);
        
        tokio::spawn(async move {
            while let Some(tasks) = api_internal_rx.recv().await {
                let _ = api_tx.send(SchedulerEvent::NewDag(tasks)).await;
            }
        });

        ucser_kernel::api::start_api_server(api_internal_tx).await;
    });

    println!("UCSER Control Plane Scheduler running...");

    // Event-driven scheduler loop
    let shutdown_signal = tokio::signal::ctrl_c();
    tokio::pin!(shutdown_signal);

    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                match event {
                    SchedulerEvent::NewDag(tasks) => {
                        println!("Received new DAG from API.");
                        for task in tasks {
                            let execution_id = &task.execution_id;
                            let _ = audit.log_event(execution_id, serde_json::json!({
                                "task_id": task.id,
                                "event": "started"
                            }));

                            // Validate task against Policy Engine
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
                                continue;
                            }

                            dag.add_task(task);
                        }
                        dispatch_ready_tasks(&mut dag, &event_tx, &win_client, &linux_client, args.single_node, &mut audit).await;
                    }
                    SchedulerEvent::TaskFinished { task_id, exit_code, duration_ms, stdout, stderr } => {
                        let task = match dag.tasks.get(&task_id) {
                            Some(t) => t.clone(),
                            None => continue,
                        };
                        let execution_id = &task.execution_id;

                        if exit_code == 0 {
                            dag.mark_completed(&task_id);
                            let _ = audit.log_event(execution_id, serde_json::json!({
                                "task_id": task_id,
                                "os": task.os,
                                "cmd": task.command,
                                "result": "success",
                                "latency_ms": duration_ms,
                                "policy_decision": "allowed",
                                "stdout": stdout,
                                "stderr": stderr
                            }));
                            let _ = audit.log_event(execution_id, serde_json::json!({
                                "task_id": task_id,
                                "event": "completed"
                            }));
                        } else {
                            let should_retry = dag.mark_failed(&task_id);
                            let _ = audit.log_event(execution_id, serde_json::json!({
                                "task_id": task_id,
                                "os": task.os,
                                "cmd": task.command,
                                "result": "failure",
                                "latency_ms": duration_ms,
                                "policy_decision": "allowed",
                                "retry_attempt": dag.failed.get(&task_id).cloned().unwrap_or(0),
                                "stdout": stdout,
                                "stderr": stderr
                            }));

                            if !should_retry {
                                let _ = audit.log_event(execution_id, serde_json::json!({
                                    "task_id": task_id,
                                    "event": "failed_permanently"
                                }));
                            }
                        }
                        dispatch_ready_tasks(&mut dag, &event_tx, &win_client, &linux_client, args.single_node, &mut audit).await;
                    }
                }
            }
            _ = &mut shutdown_signal => {
                println!("Shutdown signal received. Exiting UCSER Control Plane...");
                break;
            }
        }
    }

    Ok(())
}

async fn dispatch_ready_tasks(
    dag: &mut DagEngine,
    event_tx: &tokio::sync::mpsc::Sender<SchedulerEvent>,
    win_client: &Option<AdapterClient>,
    linux_client: &Option<AdapterClient>,
    single_node: bool,
    audit: &mut AuditEngine,
) {
    let ready_tasks = dag.get_ready_tasks();
    for task in ready_tasks {
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
        let task_id = task.id.clone();
        let command = task.command.clone();
        let args = task.args.clone();
        let env_vars = task.env_vars.clone();
        let os = task.os.clone();
        
        let win_client_clone = win_client.clone();
        let linux_client_clone = linux_client.clone();
        let tx = event_tx.clone();

        tokio::spawn(async move {
            let start = std::time::Instant::now();
            let mut exit_code = 1;
            let mut stdout = String::new();
            let mut stderr = String::new();

            if single_node {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                exit_code = 0;
                stdout = format!("mock output for task {}", task_id);
            } else {
                let client = if os == "windows" { &win_client_clone } else { &linux_client_clone };
                if let Some(c) = client {
                    match c.dispatch(task_id.clone(), command, args, env_vars).await {
                        Ok(res) => {
                            exit_code = res.exit_code;
                            stdout = res.stdout;
                            stderr = res.stderr;
                        }
                        Err(e) => {
                            eprintln!("gRPC dispatch error: {}", e);
                            stderr = e.to_string();
                        }
                    }
                } else {
                    stderr = format!("No active adapter client connected for OS {}", os);
                }
            }

            let duration_ms = start.elapsed().as_millis();
            let _ = tx.send(SchedulerEvent::TaskFinished {
                task_id,
                exit_code,
                duration_ms,
                stdout,
                stderr,
            }).await;
        });
    }
}
