use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerState {
    Idle,
    Running,
    Disabled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub interval_secs: u64,
    pub state: WorkerState,
    pub last_run_at: Option<u64>,
    pub next_run_at: Option<u64>,
    pub run_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchedulerSnapshot {
    pub workers: Vec<WorkerInfo>,
    pub scheduler_running: bool,
    pub observed_at: u64,
}
