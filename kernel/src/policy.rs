use crate::dag::Task;
use crate::traits::PolicyBackend;
use regorus::{Engine, Value};
use serde_json::json;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolicyViolation {
    #[error("Disallowed command: {0}")]
    DisallowedCommand(String),
    #[error("Restricted environment variable: {0}")]
    RestrictedEnvVar(String),
    #[error("Rego evaluation error: {0}")]
    RegoEvaluationError(String),
}

pub struct PolicyEngine {
    engine: Engine,
    policy_dir: String,
}

impl PolicyEngine {
    pub fn new() -> Self {
        let engine = Engine::new();
        let mut policy_dir = "kernel/policies".to_string();
        if std::env::current_dir().map(|p| p.ends_with("kernel")).unwrap_or(false) {
            policy_dir = "policies".to_string();
        }
        
        let mut s = Self { engine, policy_dir };
        if let Err(e) = s.reload() {
            eprintln!("Warning: Failed to load policies: {:?}", e);
        }
        s
    }

    fn check_violations(&self, res: regorus::QueryResults, task: &Task) -> Result<(), PolicyViolation> {
        let res_json = serde_json::to_value(&res)
            .map_err(|e| PolicyViolation::RegoEvaluationError(e.to_string()))?;
            
        if let Some(result_arr) = res_json.get("result").and_then(|r| r.as_array()) {
            for item in result_arr {
                if let Some(expressions) = item.get("expressions").and_then(|e| e.as_array()) {
                    for expr in expressions {
                        if let Some(value_arr) = expr.get("value").and_then(|v| v.as_array()) {
                            for violation in value_arr {
                                if let Some(msg) = violation.as_str() {
                                    if msg.starts_with("disallowed_command: ") {
                                        return Err(PolicyViolation::DisallowedCommand(task.command.clone()));
                                    } else if msg.starts_with("restricted_env: ") {
                                        let var = msg.trim_start_matches("restricted_env: ").to_string();
                                        return Err(PolicyViolation::RestrictedEnvVar(var));
                                    }
                                }
                            }
                        } else if let Some(msg) = expr.get("value").and_then(|v| v.as_str()) {
                            if msg.starts_with("disallowed_command: ") {
                                return Err(PolicyViolation::DisallowedCommand(task.command.clone()));
                            } else if msg.starts_with("restricted_env: ") {
                                let var = msg.trim_start_matches("restricted_env: ").to_string();
                                return Err(PolicyViolation::RestrictedEnvVar(var));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl PolicyBackend for PolicyEngine {
    /// Hot-reloads all policy files in the policies directory.
    fn reload(&mut self) -> Result<(), PolicyViolation> {
        let mut engine = Engine::new();
        let path = Path::new(&self.policy_dir);
        
        if path.exists() && path.is_dir() {
            let dir_entries = fs::read_dir(path)
                .map_err(|e| PolicyViolation::RegoEvaluationError(e.to_string()))?;

            for entry in dir_entries {
                let entry = entry.map_err(|e| PolicyViolation::RegoEvaluationError(e.to_string()))?;
                let file_path = entry.path();
                if file_path.is_file() && file_path.extension().and_then(|s| s.to_str()) == Some("rego") {
                    println!("Loading policy: {:?}", file_path);
                    let path_str = file_path.to_string_lossy().into_owned();
                    engine.add_policy_from_file(path_str)
                        .map_err(|e| PolicyViolation::RegoEvaluationError(e.to_string()))?;
                }
            }
        } else {
            return Err(PolicyViolation::RegoEvaluationError(format!("Policy directory does not exist: {}", self.policy_dir)));
        }
        
        self.engine = engine;
        Ok(())
    }

    /// Validates a task against security policies using OPA/Rego.
    fn validate_task(&mut self, task: &Task) -> Result<(), PolicyViolation> {
        let input_json = json!({
            "cmd": task.command,
            "args": task.args,
            "env_vars": task.env_vars,
            "os": task.os,
        });

        let input_val = Value::from_json_str(&input_json.to_string())
            .map_err(|e| PolicyViolation::RegoEvaluationError(e.to_string()))?;
        self.engine.set_input(input_val);

        // Evaluate deny rules for HIPAA
        let hipaa_res = self.engine.eval_query("data.ucser.hipaa.deny".to_string(), false)
            .map_err(|e| PolicyViolation::RegoEvaluationError(e.to_string()))?;
        self.check_violations(hipaa_res, task)?;

        // Evaluate deny rules for SOC2
        let soc2_res = self.engine.eval_query("data.ucser.soc2.deny".to_string(), false)
            .map_err(|e| PolicyViolation::RegoEvaluationError(e.to_string()))?;
        self.check_violations(soc2_res, task)?;

        Ok(())
    }
}
