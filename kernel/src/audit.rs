use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;
use serde_json::{json, Value};

#[cfg(feature = "enterprise")]
use sha2::{Sha256, Digest};
#[cfg(feature = "enterprise")]
use std::collections::HashMap;

pub struct AuditEngine {
    log_file: File,
    #[cfg(feature = "enterprise")]
    execution_hashes: HashMap<String, String>,
}

impl AuditEngine {
    /// Initializes a new JSON execution logger (or tamper-resistant logger if enterprise is enabled).
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        #[cfg(feature = "enterprise")]
        return Ok(Self { log_file, execution_hashes: HashMap::new() });

        #[cfg(not(feature = "enterprise"))]
        return Ok(Self { log_file });
    }

    /// Appends an event to the execution log (with cryptographic hashes if enterprise is enabled).
    pub fn log_event<T: Serialize>(&mut self, execution_id: &str, event: T) -> io::Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let mut v = serde_json::to_value(event).unwrap();
        
        #[cfg(feature = "enterprise")]
        {
            let prev_hash = self.execution_hashes.get(execution_id)
                .cloned()
                .unwrap_or_else(|| "0000000000000000000000000000000000000000000000000000000000000000".to_string());

            if let Some(obj) = v.as_object_mut() {
                obj.insert("execution_id".to_string(), json!(execution_id));
                obj.insert("ts".to_string(), json!(timestamp));
                obj.insert("prev_hash".to_string(), json!(prev_hash));
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
        }

        #[cfg(not(feature = "enterprise"))]
        {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("execution_id".to_string(), json!(execution_id));
                obj.insert("ts".to_string(), json!(timestamp));
            }
        }

        // Serialize to final NDJSON format
        let mut log_entry = serde_json::to_string(&v).unwrap();
        log_entry.push('\n');
        
        self.log_file.write_all(log_entry.as_bytes())?;
        self.log_file.sync_data()?;

        Ok(())
    }
}
