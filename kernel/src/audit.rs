use std::fs::{File, OpenOptions};
use std::io::{self, Write, BufRead, BufReader};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::{json, Value};

use std::collections::HashMap;
use crate::error::UcserError;
use crate::traits::AuditBackend;

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
}

const COMPLIANCE_SECRET: &[u8] = b"ucser-compliance-secret-key-2026";

pub fn compute_keyed_hash(secret: &[u8], prev_hash: &str, payload: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(secret);
    hasher.update(prev_hash.as_bytes());
    hasher.update(payload.as_bytes());
    hex::encode(hasher.finalize())
}

impl AuditBackend for AuditEngine {
    /// Appends an event to the execution log.
    fn log_event(&mut self, execution_id: &str, event: Value) -> Result<(), UcserError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);

        let mut v = event;
        
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

        // Convert payload to key-sorted BTreeMap for canonical serialization
        let sorted_map: std::collections::BTreeMap<String, Value> = serde_json::from_value(v.clone())?;
        let event_json = serde_json::to_string(&sorted_map)?;
        
        let hash_str = compute_keyed_hash(COMPLIANCE_SECRET, &prev_hash, &event_json);

        // Now append the computed hash onto the final log object
        if let Some(obj) = v.as_object_mut() {
            obj.insert("hash".to_string(), json!(hash_str.clone()));
        }

        self.execution_hashes.insert(execution_id.to_string(), hash_str);

        // Serialize to final NDJSON format using BTreeMap to ensure canonical ordering in storage
        let sorted_final_map: std::collections::BTreeMap<String, Value> = serde_json::from_value(v)?;
        let mut log_entry = serde_json::to_string(&sorted_final_map)?;
        log_entry.push('\n');
        
        self.log_file.write_all(log_entry.as_bytes())?;
        self.log_file.sync_data()?;

        Ok(())
    }

    /// Exports compliance logs for a specific execution ID as CSV.
    fn export_csv(&self, execution_id: &str) -> Result<String, UcserError> {
        let file = File::open(&self.log_path)?;
        let reader = BufReader::new(file);
        let mut csv = String::new();
        csv.push_str("timestamp,execution_id,task_id,event,hash,prev_hash\n");

        for line in reader.lines() {
            let line = line?;
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
        Ok(csv)
    }
}
