use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::State,
};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use std::sync::Arc;
use crate::dag::Task;

#[derive(Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub active_nodes: u32,
}

#[derive(Deserialize)]
pub struct DagRequest {
    pub dag_id: String,
    #[cfg(feature = "enterprise")]
    pub actor_identity: String,
    #[cfg(feature = "enterprise")]
    pub signature: String,
    pub tasks: Vec<TaskRequest>,
    pub edges: Vec<Vec<String>>,
}

#[derive(Deserialize)]
pub struct TaskRequest {
    pub id: String,
    pub os: String,
    pub shell: String,
    pub cmd: String,
    pub risk: String,
}

#[derive(Serialize)]
pub struct DagResponse {
    pub execution_id: String,
    pub status: String,
    pub dag_hash: String,
}

pub struct ApiState {
    pub tx: Sender<Vec<Task>>,
}

pub async fn start_api_server(tx: Sender<Vec<Task>>) {
    let state = Arc::new(ApiState { tx });

    let app = Router::new()
        .route("/api/status", get(get_status))
        .route("/api/dag", post(submit_dag))
        .with_state(state)
        .fallback_service(ServeDir::new("static"));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("API edge layer listening on http://{}", addr);
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("Failed to start Axum server");
}

async fn get_status() -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "Running".to_string(),
        active_nodes: 1,
    })
}

async fn submit_dag(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<DagRequest>,
) -> Json<DagResponse> {
    let mut tasks = Vec::new();
    
    for tr in payload.tasks {
        tasks.push(Task {
            execution_id: payload.dag_id.clone(),
            id: tr.id.clone(),
            os: tr.os.clone(),
            shell: tr.shell.clone(),
            command: tr.cmd.clone(),
            args: vec![],
            env_vars: std::collections::HashMap::new(),
            dependencies: vec![], // Mocked edges for demo
        });
    }

    let _ = state.tx.send(tasks).await;

    Json(DagResponse {
        execution_id: payload.dag_id.clone(),
        status: "accepted".to_string(),
        dag_hash: format!("sha256:dummy-hash-{}", payload.dag_id),
    })
}
