use std::collections::HashMap;
use std::sync::Mutex;

use peekoo_productivity_domain::pomodoro::{PomodoroError, PomodoroSession, PomodoroState};
use peekoo_productivity_domain::task::{Task, TaskPriority, TaskStatus};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskDto {
    pub id: String,
    pub title: String,
    pub priority: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PomodoroSessionDto {
    pub id: String,
    pub minutes: u32,
    pub state: String,
}

pub struct ProductivityService {
    pomodoros: Mutex<HashMap<String, PomodoroSession>>,
}

impl ProductivityService {
    pub fn new() -> Self {
        Self {
            pomodoros: Mutex::new(HashMap::new()),
        }
    }

    pub fn create_task(&self, title: &str, priority: &str) -> Result<TaskDto, String> {
        let title = title.trim();
        if title.is_empty() {
            return Err("Task title cannot be empty".to_string());
        }

        let parsed_priority = parse_task_priority(priority)?;
        let task = Task::new(
            Uuid::new_v4().to_string(),
            title.to_string(),
            parsed_priority,
        );

        Ok(task_to_dto(task))
    }

    pub fn start_pomodoro(&self, minutes: u32) -> Result<PomodoroSessionDto, String> {
        if minutes == 0 {
            return Err("Pomodoro minutes must be greater than 0".to_string());
        }

        let id = Uuid::new_v4().to_string();
        let mut session = PomodoroSession::new(id.clone(), minutes);
        session.start().map_err(|err| err.to_string())?;

        let mut lock = self
            .pomodoros
            .lock()
            .map_err(|err| format!("Lock error: {err}"))?;
        lock.insert(id, session.clone());

        Ok(pomodoro_to_dto(session))
    }

    pub fn pause_pomodoro(&self, session_id: &str) -> Result<PomodoroSessionDto, String> {
        self.update_pomodoro(session_id, |session| session.pause())
    }

    pub fn resume_pomodoro(&self, session_id: &str) -> Result<PomodoroSessionDto, String> {
        self.update_pomodoro(session_id, |session| session.resume())
    }

    pub fn finish_pomodoro(&self, session_id: &str) -> Result<PomodoroSessionDto, String> {
        self.update_pomodoro(session_id, |session| session.finish())
    }

    fn update_pomodoro(
        &self,
        session_id: &str,
        transition: impl FnOnce(&mut PomodoroSession) -> Result<(), PomodoroError>,
    ) -> Result<PomodoroSessionDto, String> {
        let mut lock = self
            .pomodoros
            .lock()
            .map_err(|err| format!("Lock error: {err}"))?;
        let Some(session) = lock.get_mut(session_id) else {
            return Err(format!("Pomodoro session not found: {session_id}"));
        };

        transition(session).map_err(|err| err.to_string())?;
        Ok(pomodoro_to_dto(session.clone()))
    }
}

impl Default for ProductivityService {
    fn default() -> Self {
        Self::new()
    }
}

fn parse_task_priority(priority: &str) -> Result<TaskPriority, String> {
    let normalized = priority.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "low" => Ok(TaskPriority::Low),
        "medium" => Ok(TaskPriority::Medium),
        "high" => Ok(TaskPriority::High),
        _ => Err(format!("Invalid task priority: {}", priority.trim())),
    }
}

fn task_to_dto(task: Task) -> TaskDto {
    TaskDto {
        id: task.id,
        title: task.title,
        priority: task_priority_to_str(task.priority).to_string(),
        status: task_status_to_str(task.status).to_string(),
    }
}

fn pomodoro_to_dto(session: PomodoroSession) -> PomodoroSessionDto {
    PomodoroSessionDto {
        id: session.id,
        minutes: session.minutes,
        state: pomodoro_state_to_str(session.state).to_string(),
    }
}

fn task_priority_to_str(priority: TaskPriority) -> &'static str {
    match priority {
        TaskPriority::Low => "low",
        TaskPriority::Medium => "medium",
        TaskPriority::High => "high",
    }
}

fn task_status_to_str(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
    }
}

fn pomodoro_state_to_str(state: PomodoroState) -> &'static str {
    match state {
        PomodoroState::Idle => "idle",
        PomodoroState::Running => "running",
        PomodoroState::Paused => "paused",
        PomodoroState::Completed => "completed",
    }
}
