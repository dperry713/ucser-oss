use ucser_kernel::dag::{DagEngine, Task};
use ucser_kernel::policy::PolicyEngine;
use ucser_kernel::audit::AuditEngine;
use ucser_kernel::client::AdapterClient;
use ucser_kernel::error::UcserError;
use ucser_kernel::traits::{Executor, PolicyBackend, AuditBackend, ExecutionResult};
use clap::Parser;
use std::sync::Arc;

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

struct MockExecutor;

#[async_trait::async_trait]
impl Executor for MockExecutor {
    async fn execute(&self, task: &Task) -> Result<ExecutionResult, UcserError> {
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        Ok(ExecutionResult {
            exit_code: 0,
            stdout: format!("mock output for task {}", task.id),
            stderr: String::new(),
        })
    }
}

struct GrpcExecutor {
    kv_store: Option<Arc<tokio::sync::Mutex<ucser_kernel::kv::KvStore>>>,
    clients: Arc<tokio::sync::Mutex<std::collections::HashMap<String, AdapterClient>>>,
}

#[async_trait::async_trait]
impl Executor for GrpcExecutor {
    async fn execute(&self, task: &Task) -> Result<ExecutionResult, UcserError> {
        let mut target_address = None;

        // 1. Try to discover node from etcd prefix query
        if let Some(ref kv_mutex) = self.kv_store {
            let mut kv = kv_mutex.lock().await;
            match kv.get_prefix("/nodes/").await {
                Ok(nodes) => {
                    for (_key, val_bytes) in nodes {
                        if let Ok(val_str) = std::str::from_utf8(&val_bytes) {
                            if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(val_str) {
                                if metadata.get("os").and_then(|v| v.as_str()) == Some(&task.os) {
                                    if let Some(address) = metadata.get("address").and_then(|v| v.as_str()) {
                                        target_address = Some(address.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to fetch nodes from etcd: {}", e);
                }
            }
        }

        // 2. If not found in etcd, fall back to default ports
        let address = target_address.unwrap_or_else(|| {
            if task.os == "windows" {
                "http://[::1]:50051".to_string()
            } else {
                "http://[::1]:50052".to_string()
            }
        });

        // 3. Resolve or construct gRPC client
        let mut clients = self.clients.lock().await;
        let client = if let Some(cached_client) = clients.get(&address) {
            cached_client.clone()
        } else {
            println!("Connecting to adapter endpoint: {}", address);
            match AdapterClient::connect(&address).await {
                Ok(c) => {
                    clients.insert(address.clone(), c.clone());
                    c
                }
                Err(e) => {
                    return Ok(ExecutionResult {
                        exit_code: 1,
                        stdout: String::new(),
                        stderr: format!("Failed to connect to adapter at {}: {}", address, e),
                    });
                }
            }
        };

        // 4. Dispatch task
        let res = client.dispatch(
            task.execution_id.clone(),
            task.command.clone(),
            task.args.clone(),
            task.env_vars.clone(),
        ).await?;

        Ok(ExecutionResult {
            exit_code: res.exit_code,
            stdout: res.stdout,
            stderr: res.stderr,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), UcserError> {
    let args = Args::parse();
    
    if args.single_node {
        println!("Starting UCSER Control Plane in Single-Node Mode...");
    } else {
        println!("Starting UCSER Control Plane (Cluster Mode)...");
    }

    let mut audit = AuditEngine::new("logs/audit.ndjson").or_else(|_| AuditEngine::new("audit.ndjson"))?;
    let mut dag = DagEngine::new();
    let mut policy = PolicyEngine::new();

    // Distributed Mode: etcd coordination
    let kv = if args.single_node {
        None
    } else {
        println!("Connecting to etcd on localhost:2379...");
        match ucser_kernel::kv::KvStore::connect(&["http://127.0.0.1:2379"]).await {
            Ok(k) => {
                println!("Successfully connected to etcd.");
                Some(Arc::new(tokio::sync::Mutex::new(k)))
            }
            Err(e) => {
                println!("Warning: etcd not available, running without distributed coordination: {}", e);
                None
            }
        }
    };

    if let Some(ref kv_mutex) = kv {
        let mut store = kv_mutex.lock().await;
        let node_id = "node-1";
        if let Err(e) = store.put(format!("/nodes/{}", node_id), "active").await {
            println!("Warning: Failed to register node in etcd: {}", e);
        } else {
            println!("Registered node {} in etcd.", node_id);
        }

        // Register worker node adapters dynamically in etcd
        let _ = store.put("/nodes/windows-node-1", r#"{"os":"windows","address":"http://[::1]:50051"}"#).await;
        let _ = store.put("/nodes/linux-node-1", r#"{"os":"linux","address":"http://[::1]:50052"}"#).await;
        
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

    let executor: Arc<dyn Executor> = if args.single_node {
        Arc::new(MockExecutor)
    } else {
        Arc::new(GrpcExecutor {
            kv_store: kv.clone(),
            clients: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        })
    };

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
                        dispatch_ready_tasks(&mut dag, &event_tx, executor.clone(), &mut audit).await;
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
                        dispatch_ready_tasks(&mut dag, &event_tx, executor.clone(), &mut audit).await;
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
    executor: Arc<dyn Executor>,
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
        let executor_clone = executor.clone();
        let tx = event_tx.clone();
        let task_clone = task.clone();

        tokio::spawn(async move {
            let start = std::time::Instant::now();
            let (exit_code, stdout, stderr) = match executor_clone.execute(&task_clone).await {
                Ok(res) => (res.exit_code, res.stdout, res.stderr),
                Err(e) => {
                    eprintln!("Execution error: {}", e);
                    (1, String::new(), e.to_string())
                }
            };

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
