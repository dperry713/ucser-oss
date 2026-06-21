use thiserror::Error;

#[derive(Error, Debug)]
pub enum UcserError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("gRPC status error: {0}")]
    GrpcStatus(#[from] tonic::Status),

    #[error("gRPC transport error: {0}")]
    GrpcTransport(#[from] tonic::transport::Error),

    #[error("DAG engine error: {0}")]
    Dag(#[from] crate::dag::DagError),

    #[error("Policy violation: {0}")]
    Policy(#[from] crate::policy::PolicyViolation),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Rego evaluation error: {0}")]
    Rego(String),

    #[error("etcd client error: {0}")]
    Etcd(#[from] etcd_client::Error),
}
