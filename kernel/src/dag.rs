use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Task {
    pub execution_id: String,
    pub id: String,
    pub os: String,
    pub shell: String,
    pub command: String,
    pub args: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub dependencies: Vec<String>, // List of task IDs that must complete first
}

pub struct DagEngine {
    tasks: HashMap<String, Task>,
    completed: HashSet<String>,
    running: HashSet<String>,
}

impl DagEngine {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            completed: HashSet::new(),
            running: HashSet::new(),
        }
    }

    /// Adds a new task to the DAG.
    pub fn add_task(&mut self, task: Task) {
        self.tasks.insert(task.id.clone(), task);
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
}
