use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub assignee: String,
    pub labels: Vec<String>,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub estimated_duration_min: Option<u32>,
    pub recurrence_rule: Option<String>,
    pub recurrence_time_of_day: Option<String>,
    pub created_at: String,
}

impl Task {
    pub fn new(id: impl Into<String>, title: impl Into<String>, priority: TaskPriority) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            status: TaskStatus::Todo,
            priority,
            assignee: "user".to_string(),
            labels: Vec::new(),
            scheduled_start_at: None,
            scheduled_end_at: None,
            estimated_duration_min: None,
            recurrence_rule: None,
            recurrence_time_of_day: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
    }

    pub fn set_assignee(&mut self, assignee: impl Into<String>) {
        self.assignee = assignee.into();
    }

    pub fn add_label(&mut self, label: impl Into<String>) {
        let label = label.into();
        if !self.labels.contains(&label) {
            self.labels.push(label);
        }
    }

    pub fn remove_label(&mut self, label: &str) {
        self.labels.retain(|l| l != label);
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Done;
    }
}

// ── Task Event Types ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskEventType {
    Created,
    StatusChanged,
    Assigned,
    Labeled,
    Unlabeled,
    Deleted,
    Comment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: String,
    pub task_id: String,
    pub event_type: TaskEventType,
    pub payload: serde_json::Value,
    pub created_at: String,
}

// ── TaskService trait (for plugin host functions) ─────────────────────

pub trait TaskService: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn create_task(
        &self,
        title: &str,
        priority: &str,
        assignee: &str,
        labels: &[String],
        description: Option<&str>,
        scheduled_start_at: Option<&str>,
        scheduled_end_at: Option<&str>,
        estimated_duration_min: Option<u32>,
        recurrence_rule: Option<&str>,
        recurrence_time_of_day: Option<&str>,
    ) -> Result<TaskDto, String>;
    fn list_tasks(&self) -> Result<Vec<TaskDto>, String>;
    #[allow(clippy::too_many_arguments)]
    fn update_task(
        &self,
        id: &str,
        title: Option<&str>,
        priority: Option<&str>,
        status: Option<&str>,
        assignee: Option<&str>,
        labels: Option<&[String]>,
        description: Option<&str>,
        scheduled_start_at: Option<&str>,
        scheduled_end_at: Option<&str>,
        estimated_duration_min: Option<Option<u32>>,
        recurrence_rule: Option<Option<&str>>,
        recurrence_time_of_day: Option<Option<&str>>,
    ) -> Result<TaskDto, String>;
    fn delete_task(&self, id: &str) -> Result<(), String>;
    fn toggle_task(&self, id: &str) -> Result<TaskDto, String>;
    fn get_task_activity(&self, task_id: &str, limit: u32) -> Result<Vec<TaskEventDto>, String>;
    fn add_task_comment(
        &self,
        task_id: &str,
        text: &str,
        author: &str,
    ) -> Result<TaskEventDto, String>;
}

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEventDto {
    pub id: String,
    pub task_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_defaults_to_todo() {
        let task = Task::new("task-1", "Write PRD", TaskPriority::High);
        assert_eq!(task.status, TaskStatus::Todo);
    }

    #[test]
    fn new_task_defaults_to_user_assignee() {
        let task = Task::new("task-1", "Write PRD", TaskPriority::High);
        assert_eq!(task.assignee, "user");
        assert!(task.labels.is_empty());
    }

    #[test]
    fn task_can_transition_to_in_progress_and_done() {
        let mut task = Task::new("task-1", "Write PRD", TaskPriority::High);
        task.start();
        assert_eq!(task.status, TaskStatus::InProgress);
        task.complete();
        assert_eq!(task.status, TaskStatus::Done);
    }

    #[test]
    fn task_set_status() {
        let mut task = Task::new("task-1", "Write PRD", TaskPriority::High);
        task.set_status(TaskStatus::InProgress);
        assert_eq!(task.status, TaskStatus::InProgress);
    }

    #[test]
    fn task_set_assignee() {
        let mut task = Task::new("task-1", "Write PRD", TaskPriority::High);
        task.set_assignee("agent");
        assert_eq!(task.assignee, "agent");
    }

    #[test]
    fn task_add_label_no_duplicates() {
        let mut task = Task::new("task-1", "Write PRD", TaskPriority::High);
        task.add_label("bug");
        task.add_label("bug");
        assert_eq!(task.labels, vec!["bug"]);
    }

    #[test]
    fn task_remove_label() {
        let mut task = Task::new("task-1", "Write PRD", TaskPriority::High);
        task.add_label("bug");
        task.add_label("urgent");
        task.remove_label("bug");
        assert_eq!(task.labels, vec!["urgent"]);
    }
}
