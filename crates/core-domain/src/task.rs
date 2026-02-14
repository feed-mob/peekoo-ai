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
    pub status: TaskStatus,
    pub priority: TaskPriority,
}

impl Task {
    pub fn new(id: impl Into<String>, title: impl Into<String>, priority: TaskPriority) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status: TaskStatus::Todo,
            priority,
        }
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Done;
    }
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
    fn task_can_transition_to_in_progress_and_done() {
        let mut task = Task::new("task-1", "Write PRD", TaskPriority::High);
        task.start();
        assert_eq!(task.status, TaskStatus::InProgress);
        task.complete();
        assert_eq!(task.status, TaskStatus::Done);
    }
}
