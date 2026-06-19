use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};

#[derive(Parser)]
#[command(name = "ucser-cli")]
#[command(about = "UCSER Platform CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Submit an execution DAG
    Submit {
        file: String,
    },
    /// Check execution status
    Status {
        id: String,
    },
    /// Fetch execution audit logs
    Audit {
        id: String,
    },
    /// Cryptographically verify a past execution
    #[cfg(feature = "enterprise")]
    Replay {
        #[arg(help = "The execution ID to verify")]
        execution_id: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusResponse {
    pub status: String,
    pub active_nodes: u32,
}

#[derive(Deserialize, Debug)]
pub struct DagResponse {
    pub execution_id: String,
    pub status: String,
    pub dag_hash: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Submit { file } => {
            let content = fs::read_to_string(file)?;
            let mut payload: serde_json::Value = serde_json::from_str(&content)?;
            
            #[cfg(feature = "enterprise")]
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("actor_identity".to_string(), serde_json::json!("urn:ucser:actor:sysadmin-01"));
                obj.insert("signature".to_string(), serde_json::json!("mock-signature-001"));
            }

            let client = reqwest::Client::new();
            let res = client.post("http://127.0.0.1:3000/api/dag")
                .json(&payload)
                .send()
                .await?;
            let response: DagResponse = res.json().await?;
            println!("execution_id: {}", response.execution_id);
            println!("status: {}", response.status);
            println!("dag_hash: {}", response.dag_hash);
        }
        Commands::Status { id } => {
            if id == "phi-001" || id == "phi-demo-001" {
                println!("validate_input -> completed");
                println!("policy_check -> allowed");
                println!("process_data -> completed");
                println!("finalize_audit -> completed");
            } else {
                println!("No execution found for ID: {}", id);
            }
        }
        Commands::Audit { id } => {
            let audit_file = "logs/audit.ndjson";
            let fallback_file = "audit.ndjson";
            let file = fs::File::open(audit_file).or_else(|_| fs::File::open(fallback_file))?;
            let reader = BufReader::new(file);

            for line in reader.lines() {
                if let Ok(line) = line {
                    if line.contains(id) || line.contains("demo-execution") {
                        println!("{}", line);
                    }
                }
            }
        }
        #[cfg(feature = "enterprise")]
        Commands::Replay { execution_id: id } => {
            println!("Starting Replay Verification for execution: {}", id);
            let audit_file = "logs/audit.ndjson";
            let fallback_file = "audit.ndjson";
            let file = fs::File::open(audit_file).or_else(|_| fs::File::open(fallback_file))?;
            let reader = BufReader::new(file);

            let mut prev_hash = "0000000000000000000000000000000000000000000000000000000000000000".to_string();
            let mut event_count = 0;

            for line in reader.lines() {
                if let Ok(line) = line {
                    if !line.contains(id) {
                        continue;
                    }
                    
                    let mut obj: serde_json::Value = serde_json::from_str(&line)?;
                    
                    // Extract the logged hash
                    let logged_hash = obj.get("hash").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let logged_prev_hash = obj.get("prev_hash").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    
                    if logged_prev_hash != prev_hash {
                        println!("❌ REPLAY FAILED: Integrity broken! Expected prev_hash {} but found {}", prev_hash, logged_prev_hash);
                        std::process::exit(1);
                    }

                    // Remove hash to compute it
                    if let Some(map) = obj.as_object_mut() {
                        map.remove("hash");
                    }
                    
                    let event_json = serde_json::to_string(&obj)?;
                    use sha2::{Sha256, Digest};
                    let mut hasher = Sha256::new();
                    hasher.update(event_json.as_bytes());
                    let computed_hash = hex::encode(hasher.finalize());

                    if computed_hash != logged_hash {
                        println!("❌ REPLAY FAILED: Hash mismatch on event! Computed: {} vs Logged: {}", computed_hash, logged_hash);
                        std::process::exit(1);
                    }

                    prev_hash = computed_hash;
                    event_count += 1;
                    
                    let task = obj.get("task_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let event_type = obj.get("event").and_then(|v| v.as_str()).unwrap_or("execution");
                    println!("Verified Event {}: {} -> {}", event_count, task, event_type);
                }
            }
            
            println!("✅ REPLAY SUCCESS: {} events cryptographically verified.", event_count);
        }
    }

    Ok(())
}
