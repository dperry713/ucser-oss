use std::fs::{File, OpenOptions};
use std::io::{self, Write, BufRead, BufReader};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

pub struct AuditEngine {
    log_file: File,
    log_path: String,
    execution_hashes: HashMap<String, String>,
}

impl AuditEngine {
    /// Initializes a new JSON execution logger.
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let log_path = path.as_ref().to_string_lossy().into_owned();
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        Ok(Self {
            log_file,
            log_path,
            execution_hashes: HashMap::new(),
        })
    }

    /// Appends an event to the execution log.
    pub fn log_event<T: Serialize>(&mut self, execution_id: &str, event: T) -> io::Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let mut v = serde_json::to_value(event).unwrap();
        
        let prev_hash = self.execution_hashes.get(execution_id)
            .cloned()
            .unwrap_or_else(|| "0000000000000000000000000000000000000000000000000000000000000000".to_string());

        if let Some(obj) = v.as_object_mut() {
            obj.insert("execution_id".to_string(), json!(execution_id));
            obj.insert("ts".to_string(), json!(timestamp));
            obj.insert("prev_hash".to_string(), json!(prev_hash));
        } else {
            v = json!({
                "execution_id": execution_id,
                "ts": timestamp,
                "event": v,
                "prev_hash": prev_hash
            });
        }

        // Serialize the JSON specifically for hashing
        let event_json = serde_json::to_string(&v).unwrap();
        
        let mut hasher = Sha256::new();
        hasher.update(event_json.as_bytes());
        let hash_str = hex::encode(hasher.finalize());

        // Now append the computed hash onto the final log object
        if let Some(obj) = v.as_object_mut() {
            obj.insert("hash".to_string(), json!(hash_str.clone()));
        }

        self.execution_hashes.insert(execution_id.to_string(), hash_str);

        // Serialize to final NDJSON format
        let mut log_entry = serde_json::to_string(&v).unwrap();
        log_entry.push('\n');
        
        self.log_file.write_all(log_entry.as_bytes())?;
        self.log_file.sync_data()?;

        Ok(())
    }

    /// Exports compliance logs for a specific execution ID as CSV.
    pub fn export_csv(&self, execution_id: &str) -> io::Result<String> {
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut csv = String::new();
        csv.push_str("timestamp,execution_id,task_id,event,hash,prev_hash\n");

        for line in reader.lines() {
            if let Ok(line) = line {
                if let Ok(obj) = serde_json::from_str::<Value>(&line) {
                    if obj.get("execution_id").and_then(|v| v.as_str()) == Some(execution_id) {
                        let ts = obj.get("ts").and_then(|v| v.as_i64()).unwrap_or(0).to_string();
                        let task_id = obj.get("task_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let event = obj.get("event").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let hash = obj.get("hash").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let prev_hash = obj.get("prev_hash").and_then(|v| v.as_str()).unwrap_or("").to_string();

                        csv.push_str(&format!("{},{},{},{},{},{}\n", ts, execution_id, task_id, event, hash, prev_hash));
                    }
                }
            }
        }
        Ok(csv)
    }
}
