use std::collections::{HashMap, HashSet};
use petgraph::graph::DiGraph;
use petgraph::algo::toposort;
use thiserror::Error;
use serde::{Serialize, Deserialize};

#[derive(Error, Debug)]
pub enum DagError {
    #[error("Circular dependency detected in DAG: {0:?}")]
    CircularDependency(Vec<String>),
    #[error("Task {0} references non-existent dependency {1}")]
    MissingDependency(String, String),
    #[error("Task {0} already exists in DAG")]
    DuplicateTask(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub execution_id: String,
    pub id: String,
    pub os: String,
    pub shell: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    #[serde(default)]
    pub dependencies: Vec<String>, // List of task IDs that must complete first
    #[serde(default)]
    pub retries: u32,
    #[serde(default)]
    pub max_retries: u32,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_timeout() -> u64 {
    30
}

pub struct DagEngine {
    pub tasks: HashMap<String, Task>,
    pub completed: HashSet<String>,
    pub running: HashSet<String>,
    pub failed: HashMap<String, u32>, // task_id -> retry_count
}

impl DagEngine {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            completed: HashSet::new(),
            running: HashSet::new(),
            failed: HashMap::new(),
        }
    }

    /// Adds a new task to the DAG.
    pub fn add_task(&mut self, task: Task) {
        self.tasks.insert(task.id.clone(), task);
    }

    /// Validates that the current set of tasks form a valid Directed Acyclic Graph.
    pub fn validate(&self) -> Result<(), DagError> {
        let mut graph = DiGraph::<String, ()>::new();
        let mut node_map = HashMap::new();

        // Add all tasks as nodes
        for id in self.tasks.keys() {
            let idx = graph.add_node(id.clone());
            node_map.insert(id.clone(), idx);
        }

        // Add dependency edges
        for (id, task) in &self.tasks {
            let to_idx = *node_map.get(id).unwrap();
            for dep in &task.dependencies {
                if let Some(from_idx) = node_map.get(dep) {
                    graph.add_edge(*from_idx, to_idx, ());
                } else {
                    return Err(DagError::MissingDependency(id.clone(), dep.clone()));
                }
            }
        }

        // Check for cycles using petgraph's toposort
        if let Err(cycle_err) = toposort(&graph, None) {
            let node_id = graph[cycle_err.node_id()].clone();
            return Err(DagError::CircularDependency(vec![node_id]));
        }

        Ok(())
    }

    /// Retrieves tasks that are ready to be executed (dependencies met, not running or completed).
    pub fn get_ready_tasks(&self) -> Vec<Task> {
        let mut ready = Vec::new();
        for task in self.tasks.values() {
            if self.completed.contains(&task.id) || self.running.contains(&task.id) {
                continue;
            }
            
            let all_deps_met = task.dependencies.iter().all(|dep| self.completed.contains(dep));
            if all_deps_met {
                ready.push(task.clone());
            }
        }
        ready
    }

    /// Marks a task as currently running.
    pub fn mark_running(&mut self, task_id: &str) {
        self.running.insert(task_id.to_string());
    }

    /// Marks a task as completed.
    pub fn mark_completed(&mut self, task_id: &str) {
        self.running.remove(task_id);
        self.completed.insert(task_id.to_string());
    }

    /// Handles task failure. Returns true if task should be retried, false otherwise.
    pub fn mark_failed(&mut self, task_id: &str) -> bool {
        self.running.remove(task_id);
        if let Some(task) = self.tasks.get(task_id) {
            let retries = self.failed.entry(task_id.to_string()).or_insert(0);
            if *retries < task.max_retries {
                *retries += 1;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}
