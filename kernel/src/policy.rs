use crate::dag::Task;
use regorus::{Engine, Value};
use serde_json::json;

#[derive(Debug)]
pub enum PolicyViolation {
    DisallowedCommand(String),
    RegoEvaluationError(String),
}

pub struct PolicyEngine {
    engine: Engine,
}

impl PolicyEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        // Load the HIPAA compliance rego bundle
        if let Err(e) = engine.add_policy_from_file("kernel/policies/hipaa.rego") {
            eprintln!("Warning: Failed to load hipaa.rego: {}", e);
        }
        Self { engine }
    }

    /// Validates a task against security policies using OPA/Rego.
    pub fn validate_task(&mut self, task: &Task) -> Result<(), PolicyViolation> {
        let input_json = json!({
            "cmd": task.command,
            "args": task.args,
            "env_vars": task.env_vars,
        });

        let input_val = Value::from_json_str(&input_json.to_string()).map_err(|e| PolicyViolation::RegoEvaluationError(e.to_string()))?;
        self.engine.set_input(input_val);

        let result = self.engine.eval_query("data.ucser.hipaa.allow".to_string(), false)
            .map_err(|e| PolicyViolation::RegoEvaluationError(e.to_string()))?;
        
        let output = format!("{:?}", result);
        if !output.contains("Bool(true)") {
            return Err(PolicyViolation::DisallowedCommand(task.command.clone()));
        }

        Ok(())
    }
}
