use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    pub state: HealthState,
    pub uptime_ms: u64,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub db_connected: bool,
    pub platform: String,
    pub checked_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthState {
    Starting,
    Running,
    Draining,
    Stopped,
}
