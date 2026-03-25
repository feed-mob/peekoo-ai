use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDto {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee: String,
    pub labels: Vec<String>,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub estimated_duration_min: Option<u32>,
    pub recurrence_rule: Option<String>,
    pub recurrence_time_of_day: Option<String>,
    pub parent_task_id: Option<String>,
    pub created_at: String,
    pub agent_work_status: Option<String>,
    pub agent_work_session_id: Option<String>,
    pub agent_work_attempt_count: Option<u32>,
    pub agent_work_started_at: Option<String>,
    pub agent_work_completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEventDto {
    pub id: String,
    pub task_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: String,
}
