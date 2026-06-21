use async_trait::async_trait;
use crate::dag::Task;
use crate::error::UcserError;
use crate::policy::PolicyViolation;

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[async_trait]
pub trait Executor: Send + Sync {
    async fn execute(&self, task: &Task) -> Result<ExecutionResult, UcserError>;
}

pub trait PolicyBackend: Send + Sync {
    fn validate_task(&mut self, task: &Task) -> Result<(), PolicyViolation>;
    fn reload(&mut self) -> Result<(), PolicyViolation>;
}

pub trait AuditBackend: Send + Sync {
    fn log_event(&mut self, execution_id: &str, event: serde_json::Value) -> Result<(), UcserError>;
    fn export_csv(&self, execution_id: &str) -> Result<String, UcserError>;
}
