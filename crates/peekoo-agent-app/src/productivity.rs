use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use peekoo_productivity_domain::pomodoro::{PomodoroError, PomodoroSession, PomodoroState};
use peekoo_productivity_domain::task::{
    TaskDto, TaskEventDto, TaskEventType, TaskPriority, TaskService, TaskStatus,
};
use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PomodoroSessionDto {
    pub id: String,
    pub minutes: u32,
    pub state: String,
}

pub struct ProductivityService {
    pomodoros: Mutex<HashMap<String, PomodoroSession>>,
    pub(crate) db_conn: Arc<Mutex<Connection>>,
}

impl Clone for ProductivityService {
    fn clone(&self) -> Self {
        Self {
            pomodoros: Mutex::new(HashMap::new()),
            db_conn: Arc::clone(&self.db_conn),
        }
    }
}

impl ProductivityService {
    pub fn new(db_conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            pomodoros: Mutex::new(HashMap::new()),
            db_conn,
        }
    }

    fn conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.db_conn
            .lock()
            .map_err(|e| format!("DB lock error: {e}"))
    }

    // ── Task CRUD ───────────────────────────────────────────────────

    pub fn create_task(
        &self,
        title: &str,
        priority: &str,
        assignee: &str,
        labels: &[String],
    ) -> Result<TaskDto, String> {
        let title = title.trim();
        if title.is_empty() {
            return Err("Task title cannot be empty".to_string());
        }

        let parsed_priority = parse_task_priority(priority)?;
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        let labels_json = serde_json::to_string(labels).unwrap_or_else(|_| "[]".to_string());

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO tasks (id, title, status, priority, assignee, labels_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, title, "todo", priority.to_lowercase(), assignee, labels_json, now, now],
        )
        .map_err(|e| format!("Insert task error: {e}"))?;

        self.write_event_inner(
            &conn,
            &id,
            TaskEventType::Created,
            &serde_json::json!({"title": title, "priority": priority, "assignee": assignee}),
        )?;

        Ok(TaskDto {
            id,
            title: title.to_string(),
            status: "todo".to_string(),
            priority: task_priority_to_str(parsed_priority).to_string(),
            assignee: assignee.to_string(),
            labels: labels.to_vec(),
        })
    }

    pub fn list_tasks(&self) -> Result<Vec<TaskDto>, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, status, priority, assignee, labels_json FROM tasks ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Prepare list_tasks error: {e}"))?;

        let tasks = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let status: String = row.get(2)?;
                let priority: String = row.get(3)?;
                let assignee: String = row.get(4)?;
                let labels_json: String = row.get(5)?;
                let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();
                Ok(TaskDto {
                    id,
                    title,
                    status,
                    priority,
                    assignee,
                    labels,
                })
            })
            .map_err(|e| format!("Query tasks error: {e}"))?;

        tasks
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect tasks error: {e}"))
    }

    pub fn update_task(
        &self,
        id: &str,
        title: Option<&str>,
        priority: Option<&str>,
        status: Option<&str>,
        assignee: Option<&str>,
        labels: Option<&[String]>,
    ) -> Result<TaskDto, String> {
        let conn = self.conn()?;

        // Load current task
        let mut current = self.load_task(&conn, id)?;

        // Apply changes and write events
        if let Some(t) = title {
            if t.trim().is_empty() {
                return Err("Task title cannot be empty".to_string());
            }
            conn.execute(
                "UPDATE tasks SET title = ?1, updated_at = ?2 WHERE id = ?3",
                params![t, Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task title error: {e}"))?;
            current.title = t.to_string();
        }

        if let Some(p) = priority {
            let _ = parse_task_priority(p)?;
            conn.execute(
                "UPDATE tasks SET priority = ?1, updated_at = ?2 WHERE id = ?3",
                params![p.to_lowercase(), Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task priority error: {e}"))?;
            current.priority = p.to_lowercase();
            self.write_event_inner(
                &conn,
                id,
                TaskEventType::StatusChanged,
                &serde_json::json!({"title": current.title, "field": "priority", "to": p}),
            )?;
        }

        if let Some(s) = status {
            let parsed = parse_task_status(s)?;
            conn.execute(
                "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![s.to_lowercase(), Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task status error: {e}"))?;
            let from_status = current.status.clone();
            current.status = task_status_to_str(parsed).to_string();
            self.write_event_inner(
                &conn,
                id,
                TaskEventType::StatusChanged,
                &serde_json::json!({"title": current.title, "from": from_status, "to": s}),
            )?;
        }

        if let Some(a) = assignee {
            let from = current.assignee.clone();
            conn.execute(
                "UPDATE tasks SET assignee = ?1, updated_at = ?2 WHERE id = ?3",
                params![a, Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task assignee error: {e}"))?;
            current.assignee = a.to_string();
            self.write_event_inner(
                &conn,
                id,
                TaskEventType::Assigned,
                &serde_json::json!({"title": current.title, "from": from, "to": a}),
            )?;
        }

        if let Some(l) = labels {
            let labels_json = serde_json::to_string(l).unwrap_or_else(|_| "[]".to_string());
            conn.execute(
                "UPDATE tasks SET labels_json = ?1, updated_at = ?2 WHERE id = ?3",
                params![labels_json, Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task labels error: {e}"))?;
            current.labels = l.to_vec();
        }

        Ok(current)
    }

    pub fn delete_task(&self, id: &str) -> Result<(), String> {
        let conn = self.conn()?;
        let task = self.load_task(&conn, id)?;

        conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .map_err(|e| format!("Delete task error: {e}"))?;

        self.write_event_inner(
            &conn,
            id,
            TaskEventType::Deleted,
            &serde_json::json!({"title": task.title}),
        )?;

        Ok(())
    }

    pub fn toggle_task(&self, id: &str) -> Result<TaskDto, String> {
        let conn = self.conn()?;
        let current = self.load_task(&conn, id)?;

        let new_status = if current.status == "done" {
            "todo"
        } else {
            "done"
        };
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status, now, id],
        )
        .map_err(|e| format!("Toggle task error: {e}"))?;

        self.write_event_inner(
            &conn,
            id,
            TaskEventType::StatusChanged,
            &serde_json::json!({"title": current.title, "from": current.status, "to": new_status}),
        )?;

        let mut updated = current;
        updated.status = new_status.to_string();
        Ok(updated)
    }

    // ── Task Events ─────────────────────────────────────────────────

    pub fn list_task_events(&self, limit: i64) -> Result<Vec<TaskEventDto>, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, event_type, payload_json, created_at FROM task_events ORDER BY created_at DESC LIMIT ?1",
            )
            .map_err(|e| format!("Prepare list_task_events error: {e}"))?;

        let events = stmt
            .query_map(params![limit], |row| {
                let payload_str: String = row.get(3)?;
                let payload: serde_json::Value =
                    serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
                Ok(TaskEventDto {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    event_type: row.get(2)?,
                    payload,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Query task events error: {e}"))?;

        events
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect task events error: {e}"))
    }

    pub fn task_activity_summary(&self) -> Result<String, String> {
        let tasks = self.list_tasks()?;
        let total = tasks.len();
        let todo = tasks.iter().filter(|t| t.status == "todo").count();
        let in_progress = tasks.iter().filter(|t| t.status == "in_progress").count();
        let done = tasks.iter().filter(|t| t.status == "done").count();

        let events = self.list_task_events(20)?;
        let today = Utc::now().date_naive().to_string();

        let mut lines = Vec::new();
        lines.push("## Current Tasks".to_string());
        lines.push(format!(
            "{total} total ({todo} todo, {in_progress} in progress, {done} done)"
        ));

        let today_events: Vec<_> = events
            .iter()
            .filter(|e| e.created_at.starts_with(&today))
            .collect();

        if !today_events.is_empty() {
            lines.push("\n## Today's Activity".to_string());
            for event in &today_events {
                let desc = format_event_summary(event);
                lines.push(format!("- {desc}"));
            }
        }

        Ok(lines.join("\n"))
    }

    // ── Pomodoro (unchanged) ────────────────────────────────────────

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

    // ── Internal helpers ────────────────────────────────────────────

    fn load_task(&self, conn: &Connection, id: &str) -> Result<TaskDto, String> {
        conn.query_row(
            "SELECT id, title, status, priority, assignee, labels_json FROM tasks WHERE id = ?1",
            params![id],
            |row| {
                let labels_json: String = row.get(5)?;
                let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();
                Ok(TaskDto {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    status: row.get(2)?,
                    priority: row.get(3)?,
                    assignee: row.get(4)?,
                    labels,
                })
            },
        )
        .optional()
        .map_err(|e| format!("Load task error: {e}"))?
        .ok_or_else(|| format!("Task not found: {id}"))
    }

    fn write_event_inner(
        &self,
        conn: &Connection,
        task_id: &str,
        event_type: TaskEventType,
        payload: &serde_json::Value,
    ) -> Result<(), String> {
        let event_id = Uuid::new_v4().to_string();
        let event_type_str = match event_type {
            TaskEventType::Created => "created",
            TaskEventType::StatusChanged => "status_changed",
            TaskEventType::Assigned => "assigned",
            TaskEventType::Labeled => "labeled",
            TaskEventType::Unlabeled => "unlabeled",
            TaskEventType::Deleted => "deleted",
        };
        let payload_json = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO task_events (id, task_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_id, task_id, event_type_str, payload_json, now],
        )
        .map_err(|e| format!("Write task event error: {e}"))?;

        Ok(())
    }
}

impl TaskService for ProductivityService {
    fn create_task(
        &self,
        title: &str,
        priority: &str,
        assignee: &str,
        labels: &[String],
    ) -> Result<TaskDto, String> {
        self.create_task(title, priority, assignee, labels)
    }

    fn list_tasks(&self) -> Result<Vec<TaskDto>, String> {
        self.list_tasks()
    }

    fn update_task(
        &self,
        id: &str,
        title: Option<&str>,
        priority: Option<&str>,
        status: Option<&str>,
        assignee: Option<&str>,
        labels: Option<&[String]>,
    ) -> Result<TaskDto, String> {
        self.update_task(id, title, priority, status, assignee, labels)
    }

    fn delete_task(&self, id: &str) -> Result<(), String> {
        self.delete_task(id)
    }

    fn toggle_task(&self, id: &str) -> Result<TaskDto, String> {
        self.toggle_task(id)
    }
}

// ── Utility functions ────────────────────────────────────────────────

fn parse_task_priority(priority: &str) -> Result<TaskPriority, String> {
    let normalized = priority.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "low" => Ok(TaskPriority::Low),
        "medium" => Ok(TaskPriority::Medium),
        "high" => Ok(TaskPriority::High),
        _ => Err(format!("Invalid task priority: {}", priority.trim())),
    }
}

fn parse_task_status(status: &str) -> Result<TaskStatus, String> {
    let normalized = status.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "todo" => Ok(TaskStatus::Todo),
        "in_progress" => Ok(TaskStatus::InProgress),
        "done" => Ok(TaskStatus::Done),
        _ => Err(format!("Invalid task status: {}", status.trim())),
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

fn pomodoro_to_dto(session: PomodoroSession) -> PomodoroSessionDto {
    PomodoroSessionDto {
        id: session.id,
        minutes: session.minutes,
        state: pomodoro_state_to_str(session.state).to_string(),
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

fn format_event_summary(event: &TaskEventDto) -> String {
    let title = event
        .payload
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown task");
    match event.event_type.as_str() {
        "created" => format!("Created \"{title}\""),
        "status_changed" => {
            let from = event
                .payload
                .get("from")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let to = event
                .payload
                .get("to")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("Changed \"{title}\" from {from} to {to}")
        }
        "assigned" => {
            let to = event
                .payload
                .get("to")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("Assigned \"{title}\" to {to}")
        }
        "labeled" => {
            let label = event
                .payload
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("Added label \"{label}\" to \"{title}\"")
        }
        "unlabeled" => {
            let label = event
                .payload
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("Removed label \"{label}\" from \"{title}\"")
        }
        "deleted" => format!("Deleted \"{title}\""),
        other => format!("{other} \"{title}\""),
    }
}
