use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{Timelike, Utc};
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

    /// Checkpoint the WAL to ensure data is persisted to disk.
    /// This is called after write operations. Errors are logged but not returned
    /// to avoid failing operations when the database is busy.
    fn checkpoint(&self) {
        if let Ok(conn) = self.conn()
            && let Err(e) = conn.execute_batch("PRAGMA wal_checkpoint(PASSIVE);")
        {
            tracing::warn!("WAL checkpoint failed (this is usually ok): {e}");
        }
    }

    // ── Task CRUD ───────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn create_task(
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
    ) -> Result<TaskDto, String> {
        let title = title.trim();
        if title.is_empty() {
            return Err("Task title cannot be empty".to_string());
        }

        if let (Some(start), Some(end)) = (scheduled_start_at, scheduled_end_at)
            && start >= end
        {
            return Err("scheduled_end_at must be after scheduled_start_at".to_string());
        }

        let parsed_priority = parse_task_priority(priority)?;
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        let labels_json = serde_json::to_string(labels).unwrap_or_else(|_| "[]".to_string());
        let desc = description.filter(|d| !d.is_empty());

        let mut conn = self.conn()?;

        let tx = conn
            .transaction()
            .map_err(|e| format!("Begin transaction error: {e}"))?;

        // Set agent_work_status to 'pending' for agent-assigned tasks
        let agent_work_status: Option<&str> = if assignee != "user" {
            Some("pending")
        } else {
            None
        };

        tx.execute(
            "INSERT INTO tasks (id, title, notes, status, priority, assignee, labels_json, scheduled_start_at, scheduled_end_at, estimated_duration_min, recurrence_rule, recurrence_time_of_day, parent_task_id, created_at, updated_at, agent_work_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, ?13, ?14, ?15)",
            params![&id, title, desc, "todo", priority.to_lowercase(), assignee, labels_json, scheduled_start_at, scheduled_end_at, estimated_duration_min.map(|v| v as i64), recurrence_rule, recurrence_time_of_day, now, now, agent_work_status],
        )
        .map_err(|e| format!("Insert task error: {e}"))?;

        let event_id = Uuid::new_v4().to_string();
        let payload_json = serde_json::to_string(
            &serde_json::json!({"title": title, "priority": priority, "assignee": assignee}),
        )
        .unwrap_or_else(|_| "{}".to_string());
        tx.execute(
            "INSERT INTO task_events (id, task_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_id, &id, "created", payload_json, now],
        )
        .map_err(|e| format!("Write task event error: {e}"))?;

        tx.commit().map_err(|e| format!("Commit error: {e}"))?;

        drop(conn);
        self.checkpoint();

        Ok(TaskDto {
            id,
            title: title.to_string(),
            description: desc.map(String::from),
            status: "todo".to_string(),
            priority: task_priority_to_str(parsed_priority).to_string(),
            assignee: assignee.to_string(),
            labels: labels.to_vec(),
            scheduled_start_at: scheduled_start_at.map(String::from),
            scheduled_end_at: scheduled_end_at.map(String::from),
            estimated_duration_min,
            recurrence_rule: recurrence_rule.map(String::from),
            recurrence_time_of_day: recurrence_time_of_day.map(String::from),
            parent_task_id: None,
            created_at: now,
            agent_work_status: agent_work_status.map(String::from),
            agent_work_session_id: None,
            agent_work_attempt_count: Some(0),
            agent_work_started_at: None,
            agent_work_completed_at: None,
        })
    }

    pub fn list_tasks(&self) -> Result<Vec<TaskDto>, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, notes, status, priority, assignee, labels_json, scheduled_start_at, scheduled_end_at, estimated_duration_min, recurrence_rule, recurrence_time_of_day, parent_task_id, created_at, agent_work_status, agent_work_session_id, agent_work_attempt_count, agent_work_started_at, agent_work_completed_at FROM tasks ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Prepare list_tasks error: {e}"))?;

        let tasks = stmt
            .query_map([], |row| {
                let labels_json: String = row.get(6)?;
                let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();
                let notes: Option<String> = row.get(2)?;
                Ok(TaskDto {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: notes,
                    status: row.get(3)?,
                    priority: row.get(4)?,
                    assignee: row.get(5)?,
                    labels,
                    scheduled_start_at: row.get(7)?,
                    scheduled_end_at: row.get(8)?,
                    estimated_duration_min: row.get::<_, Option<i64>>(9)?.map(|v| v as u32),
                    recurrence_rule: row.get(10)?,
                    recurrence_time_of_day: row.get(11)?,
                    parent_task_id: row.get(12)?,
                    created_at: row.get(13)?,
                    agent_work_status: row.get(14)?,
                    agent_work_session_id: row.get(15)?,
                    agent_work_attempt_count: row.get(16)?,
                    agent_work_started_at: row.get(17)?,
                    agent_work_completed_at: row.get(18)?,
                })
            })
            .map_err(|e| format!("Query tasks error: {e}"))?;

        tasks
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect tasks error: {e}"))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_task(
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

        if let Some(d) = description {
            conn.execute(
                "UPDATE tasks SET notes = ?1, updated_at = ?2 WHERE id = ?3",
                params![d, Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task description error: {e}"))?;
            current.description = Some(d.to_string());
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

            // Reset agent work tracking when reassigning
            if a != "user" {
                // Assigned to an agent — reset status to pending so the scheduler picks it up
                conn.execute(
                    "UPDATE tasks SET agent_work_status = 'pending', agent_work_session_id = NULL, agent_work_attempt_count = 0, agent_work_started_at = NULL, agent_work_completed_at = NULL WHERE id = ?1",
                    params![id],
                )
                .map_err(|e| format!("Reset agent work status error: {e}"))?;
                current.agent_work_status = Some("pending".to_string());
                current.agent_work_session_id = None;
                current.agent_work_attempt_count = Some(0);
                current.agent_work_started_at = None;
                current.agent_work_completed_at = None;
            } else {
                // Assigned back to user — clear agent work tracking
                conn.execute(
                    "UPDATE tasks SET agent_work_status = NULL, agent_work_session_id = NULL, agent_work_attempt_count = 0, agent_work_started_at = NULL, agent_work_completed_at = NULL WHERE id = ?1",
                    params![id],
                )
                .map_err(|e| format!("Clear agent work status error: {e}"))?;
                current.agent_work_status = None;
                current.agent_work_session_id = None;
                current.agent_work_attempt_count = Some(0);
                current.agent_work_started_at = None;
                current.agent_work_completed_at = None;
            }

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

        if let Some(s) = scheduled_start_at {
            conn.execute(
                "UPDATE tasks SET scheduled_start_at = ?1, updated_at = ?2 WHERE id = ?3",
                params![s, Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task scheduled_start_at error: {e}"))?;
            current.scheduled_start_at = Some(s.to_string());
        }

        if let Some(e) = scheduled_end_at {
            if let Some(ref start) = current.scheduled_start_at
                && start.as_str() >= e
            {
                return Err("scheduled_end_at must be after scheduled_start_at".to_string());
            }
            conn.execute(
                "UPDATE tasks SET scheduled_end_at = ?1, updated_at = ?2 WHERE id = ?3",
                params![e, Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task scheduled_end_at error: {e}"))?;
            current.scheduled_end_at = Some(e.to_string());
        }

        if let Some(dur) = estimated_duration_min {
            conn.execute(
                "UPDATE tasks SET estimated_duration_min = ?1, updated_at = ?2 WHERE id = ?3",
                params![dur.map(|v| v as i64), Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task estimated_duration_min error: {e}"))?;
            current.estimated_duration_min = dur;
        }

        if let Some(rr) = recurrence_rule {
            tracing::info!(
                "[productivity] update_task recurrence_rule received: {:?}",
                rr
            );
            conn.execute(
                "UPDATE tasks SET recurrence_rule = ?1, updated_at = ?2 WHERE id = ?3",
                params![rr, Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task recurrence_rule error: {e}"))?;
            current.recurrence_rule = rr.map(String::from);
        }

        if let Some(rtod) = recurrence_time_of_day {
            conn.execute(
                "UPDATE tasks SET recurrence_time_of_day = ?1, updated_at = ?2 WHERE id = ?3",
                params![rtod, Utc::now().to_rfc3339(), id],
            )
            .map_err(|e| format!("Update task recurrence_time_of_day error: {e}"))?;
            current.recurrence_time_of_day = rtod.map(String::from);
        }

        // Ensure data is persisted to disk
        drop(conn);
        self.checkpoint();

        Ok(current)
    }

    pub fn delete_task(&self, id: &str) -> Result<(), String> {
        let mut conn = self.conn()?;
        let task = self.load_task(&conn, id)?;

        // Use rusqlite's transaction API for atomic operations
        let tx = conn
            .transaction()
            .map_err(|e| format!("Begin transaction error: {e}"))?;

        tx.execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .map_err(|e| format!("Delete task error: {e}"))?;

        // Write event using the transaction
        let event_id = Uuid::new_v4().to_string();
        let payload_json = serde_json::to_string(&serde_json::json!({"title": task.title}))
            .unwrap_or_else(|_| "{}".to_string());
        let now = Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO task_events (id, task_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_id, id, "deleted", payload_json, now],
        )
        .map_err(|e| format!("Write task event error: {e}"))?;

        tx.commit().map_err(|e| format!("Commit error: {e}"))?;

        // Ensure data is persisted to disk
        drop(conn);
        self.checkpoint();

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

        // Just-in-Time recurrence: when marking a recurring task done, generate the next occurrence
        if new_status == "done"
            && let Some(ref rule) = current.recurrence_rule
        {
            let next_start = calculate_next_occurrence(
                rule,
                &current.scheduled_start_at,
                current.recurrence_time_of_day.as_deref(),
            );
            if let Some(next_start) = next_start {
                let id = Uuid::new_v4().to_string();
                let duration_min = current.estimated_duration_min;

                // Calculate next end by adding duration to next_start
                let next_end = duration_min.and_then(|min| {
                    chrono::DateTime::parse_from_rfc3339(&next_start)
                        .ok()
                        .map(|start| (start + chrono::Duration::minutes(min as i64)).to_rfc3339())
                });

                conn.execute(
                    "INSERT INTO tasks (id, title, notes, status, priority, assignee, labels_json, scheduled_start_at, scheduled_end_at, estimated_duration_min, recurrence_rule, recurrence_time_of_day, parent_task_id, created_at, updated_at) VALUES (?1, ?2, ?3, 'todo', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                    params![
                        id, current.title, current.description.as_deref().unwrap_or(""),
                        current.priority, current.assignee,
                        serde_json::to_string(&current.labels).unwrap_or_else(|_| "[]".to_string()),
                        next_start, next_end, duration_min.map(|v| v as i64),
                        rule, current.recurrence_time_of_day.as_deref(),
                        current.id, now, now
                    ],
                )
                .map_err(|e| format!("Insert next recurring task error: {e}"))?;

                self.write_event_inner(
                    &conn,
                    &id,
                    TaskEventType::Created,
                    &serde_json::json!({"title": current.title, "recurrence": "next_occurrence", "from_task": current.id}),
                )?;
            }
        }

        // Ensure data is persisted to disk
        drop(conn);
        self.checkpoint();

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
            "SELECT id, title, notes, status, priority, assignee, labels_json, scheduled_start_at, scheduled_end_at, estimated_duration_min, recurrence_rule, recurrence_time_of_day, parent_task_id, created_at, agent_work_status, agent_work_session_id, agent_work_attempt_count, agent_work_started_at, agent_work_completed_at FROM tasks WHERE id = ?1",
            params![id],
            |row| {
                let labels_json: String = row.get(6)?;
                let labels: Vec<String> = serde_json::from_str(&labels_json).unwrap_or_default();
                let notes: Option<String> = row.get(2)?;
                Ok(TaskDto {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: notes,
                    status: row.get(3)?,
                    priority: row.get(4)?,
                    assignee: row.get(5)?,
                    labels,
                    scheduled_start_at: row.get(7)?,
                    scheduled_end_at: row.get(8)?,
                    estimated_duration_min: row.get::<_, Option<i64>>(9)?.map(|v| v as u32),
                    recurrence_rule: row.get(10)?,
                    recurrence_time_of_day: row.get(11)?,
                    parent_task_id: row.get(12)?,
                    created_at: row.get(13)?,
                    agent_work_status: row.get(14)?,
                    agent_work_session_id: row.get(15)?,
                    agent_work_attempt_count: row.get(16)?,
                    agent_work_started_at: row.get(17)?,
                    agent_work_completed_at: row.get(18)?,
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
            TaskEventType::Comment => "comment",
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

    pub fn get_task_activity(
        &self,
        task_id: &str,
        limit: u32,
    ) -> Result<Vec<TaskEventDto>, String> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, task_id, event_type, payload_json, created_at FROM task_events WHERE task_id = ?1 ORDER BY created_at DESC LIMIT ?2",
            )
            .map_err(|e| format!("Prepare get_task_activity error: {e}"))?;

        let events = stmt
            .query_map(params![task_id, limit], |row| {
                let payload_json: String = row.get(3)?;
                let payload: serde_json::Value =
                    serde_json::from_str(&payload_json).unwrap_or_default();
                Ok(TaskEventDto {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    event_type: row.get(2)?,
                    payload,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| format!("Query task activity error: {e}"))?;

        events
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Collect task activity error: {e}"))
    }

    pub fn add_task_comment(
        &self,
        task_id: &str,
        text: &str,
        author: &str,
    ) -> Result<TaskEventDto, String> {
        let conn = self.conn()?;

        // Verify task exists
        let _task = self.load_task(&conn, task_id)?;

        let event_id = Uuid::new_v4().to_string();
        let payload = serde_json::json!({
            "text": text,
            "author": author,
        });
        let payload_json = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO task_events (id, task_id, event_type, payload_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![event_id, task_id, "comment", payload_json, now],
        )
        .map_err(|e| format!("Insert comment error: {e}"))?;

        drop(conn);
        self.checkpoint();

        Ok(TaskEventDto {
            id: event_id,
            task_id: task_id.to_string(),
            event_type: "comment".to_string(),
            payload,
            created_at: now,
        })
    }

    pub fn delete_task_event(&self, event_id: &str) -> Result<(), String> {
        let conn = self.conn()?;

        conn.execute("DELETE FROM task_events WHERE id = ?1", params![event_id])
            .map_err(|e| format!("Delete task event error: {e}"))?;

        drop(conn);
        self.checkpoint();

        Ok(())
    }

    pub fn claim_task_for_agent(&self, task_id: &str) -> Result<bool, String> {
        let conn = self.conn()?;

        // Use Option<Option<String>> to distinguish "row not found" from "NULL column value"
        let row_result: Option<Option<String>> = conn
            .query_row(
                "SELECT agent_work_status FROM tasks WHERE id = ?1",
                params![task_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(|e| format!("Query task error: {e}"))?;

        match row_result {
            None => return Err("Task not found".to_string()),
            Some(None) => return Ok(false), // NULL status — not an agent task
            Some(Some(ref status)) if status == "pending" || status == "failed" => {
                let rows = conn.execute(
                    "UPDATE tasks SET agent_work_status = 'claimed' WHERE id = ?1 AND agent_work_status IN ('pending', 'failed')",
                    params![task_id],
                )
                .map_err(|e| format!("Claim task error: {e}"))?;
                drop(conn);
                self.checkpoint();
                return Ok(rows > 0);
            }
            _ => return Ok(false), // Already claimed, executing, completed, etc.
        }
    }

    pub fn update_agent_work_status(
        &self,
        task_id: &str,
        status: &str,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        match status {
            "executing" => {
                conn.execute(
                    "UPDATE tasks SET agent_work_status = ?1, agent_work_session_id = ?2, agent_work_started_at = ?3 WHERE id = ?4",
                    params![status, session_id, now, task_id],
                )
                .map_err(|e| format!("Update agent work status error: {e}"))?;
            }
            "completed" | "failed" => {
                conn.execute(
                    "UPDATE tasks SET agent_work_status = ?1, agent_work_completed_at = ?2 WHERE id = ?3",
                    params![status, now, task_id],
                )
                .map_err(|e| format!("Update agent work status error: {e}"))?;
            }
            _ => {
                conn.execute(
                    "UPDATE tasks SET agent_work_status = ?1 WHERE id = ?2",
                    params![status, task_id],
                )
                .map_err(|e| format!("Update agent work status error: {e}"))?;
            }
        }

        drop(conn);
        self.checkpoint();
        Ok(())
    }

    pub fn requeue_agent_task(&self, task_id: &str) -> Result<(), String> {
        let conn = self.conn()?;
        let task = self.load_task(&conn, task_id)?;

        if task.assignee != "peekoo-agent" {
            return Ok(());
        }

        conn.execute(
            "UPDATE tasks
             SET agent_work_status = 'pending',
                 agent_work_session_id = NULL,
                 agent_work_attempt_count = 0,
                 agent_work_started_at = NULL,
                 agent_work_completed_at = NULL
             WHERE id = ?1",
            params![task_id],
        )
        .map_err(|e| format!("Requeue agent task error: {e}"))?;

        drop(conn);
        self.checkpoint();
        Ok(())
    }

    pub fn increment_attempt_count(&self, task_id: &str) -> Result<u32, String> {
        let conn = self.conn()?;

        conn.execute(
            "UPDATE tasks SET agent_work_attempt_count = agent_work_attempt_count + 1 WHERE id = ?1",
            params![task_id],
        )
        .map_err(|e| format!("Increment attempt count error: {e}"))?;

        let count: u32 = conn
            .query_row(
                "SELECT agent_work_attempt_count FROM tasks WHERE id = ?1",
                params![task_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Query attempt count error: {e}"))?;

        drop(conn);
        self.checkpoint();
        Ok(count)
    }

    pub fn list_tasks_for_agent_execution(&self) -> Result<Vec<TaskDto>, String> {
        let conn = self.conn()?;
        let now = Utc::now().to_rfc3339();

        let mut stmt = conn
            .prepare(
                "SELECT id, title, notes, status, priority, assignee, labels_json,
                        scheduled_start_at, scheduled_end_at, estimated_duration_min,
                        recurrence_rule, recurrence_time_of_day, parent_task_id, created_at,
                        agent_work_status, agent_work_session_id, agent_work_attempt_count,
                        agent_work_started_at, agent_work_completed_at
                 FROM tasks
                 WHERE assignee != 'user'
                   AND status != 'done'
                   AND agent_work_status IN ('pending', 'failed')
                   AND (agent_work_attempt_count IS NULL OR agent_work_attempt_count < 3)
                   AND (scheduled_start_at IS NULL OR scheduled_start_at <= ?1)
                 ORDER BY created_at DESC",
            )
            .map_err(|e| format!("Prepare statement error: {e}"))?;

        let tasks = stmt
            .query_map(params![now], |row| {
                Ok(TaskDto {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    priority: row.get(4)?,
                    assignee: row.get(5)?,
                    labels: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    scheduled_start_at: row.get(7)?,
                    scheduled_end_at: row.get(8)?,
                    estimated_duration_min: row.get(9)?,
                    recurrence_rule: row.get(10)?,
                    recurrence_time_of_day: row.get(11)?,
                    parent_task_id: row.get(12)?,
                    created_at: row.get(13)?,
                    agent_work_status: row.get(14)?,
                    agent_work_session_id: row.get(15)?,
                    agent_work_attempt_count: row.get(16)?,
                    agent_work_started_at: row.get(17)?,
                    agent_work_completed_at: row.get(18)?,
                })
            })
            .map_err(|e| format!("Query tasks error: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tasks)
    }

    pub fn add_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String> {
        let conn = self.conn()?;
        let mut task = self.load_task(&conn, task_id)?;

        if !task.labels.contains(&label.to_string()) {
            task.labels.push(label.to_string());
            let labels_json = serde_json::to_string(&task.labels).map_err(|e| e.to_string())?;
            let now = Utc::now().to_rfc3339();

            conn.execute(
                "UPDATE tasks SET labels_json = ?1, updated_at = ?2 WHERE id = ?3",
                params![labels_json, now, task_id],
            )
            .map_err(|e| format!("Update labels error: {e}"))?;

            self.write_event_inner(
                &conn,
                task_id,
                TaskEventType::Labeled,
                &serde_json::json!({"label": label}),
            )?;

            drop(conn);
            self.checkpoint();
        }

        Ok(task)
    }

    pub fn remove_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String> {
        let conn = self.conn()?;
        let mut task = self.load_task(&conn, task_id)?;

        task.labels.retain(|l| l != label);
        let labels_json = serde_json::to_string(&task.labels).map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE tasks SET labels_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![labels_json, now, task_id],
        )
        .map_err(|e| format!("Update labels error: {e}"))?;

        self.write_event_inner(
            &conn,
            task_id,
            TaskEventType::Unlabeled,
            &serde_json::json!({"label": label}),
        )?;

        drop(conn);
        self.checkpoint();

        Ok(task)
    }

    pub fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<TaskDto, String> {
        let conn = self.conn()?;
        let current = self.load_task(&conn, task_id)?;

        let status_str = match status {
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
            TaskStatus::Cancelled => "cancelled",
        };
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status_str, now, task_id],
        )
        .map_err(|e| format!("Update status error: {e}"))?;

        self.write_event_inner(
            &conn,
            task_id,
            TaskEventType::StatusChanged,
            &serde_json::json!({"from": current.status, "to": status_str}),
        )?;

        drop(conn);
        self.checkpoint();

        // Reload to get updated task
        let conn = self.conn()?;
        self.load_task(&conn, task_id)
    }

    pub fn load_task_by_id(&self, task_id: &str) -> Result<TaskDto, String> {
        let conn = self.conn()?;
        self.load_task(&conn, task_id)
    }
}

impl TaskService for ProductivityService {
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
    ) -> Result<TaskDto, String> {
        self.create_task(
            title,
            priority,
            assignee,
            labels,
            description,
            scheduled_start_at,
            scheduled_end_at,
            estimated_duration_min,
            recurrence_rule,
            recurrence_time_of_day,
        )
    }

    fn list_tasks(&self) -> Result<Vec<TaskDto>, String> {
        self.list_tasks()
    }

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
    ) -> Result<TaskDto, String> {
        self.update_task(
            id,
            title,
            priority,
            status,
            assignee,
            labels,
            description,
            scheduled_start_at,
            scheduled_end_at,
            estimated_duration_min,
            recurrence_rule,
            recurrence_time_of_day,
        )
    }

    fn delete_task(&self, id: &str) -> Result<(), String> {
        self.delete_task(id)
    }

    fn toggle_task(&self, id: &str) -> Result<TaskDto, String> {
        self.toggle_task(id)
    }

    fn get_task_activity(&self, task_id: &str, limit: u32) -> Result<Vec<TaskEventDto>, String> {
        self.get_task_activity(task_id, limit)
    }

    fn add_task_comment(
        &self,
        task_id: &str,
        text: &str,
        author: &str,
    ) -> Result<TaskEventDto, String> {
        self.add_task_comment(task_id, text, author)
    }

    fn claim_task_for_agent(&self, task_id: &str) -> Result<bool, String> {
        self.claim_task_for_agent(task_id)
    }

    fn update_agent_work_status(
        &self,
        task_id: &str,
        status: &str,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        self.update_agent_work_status(task_id, status, session_id)
    }

    fn increment_attempt_count(&self, task_id: &str) -> Result<u32, String> {
        self.increment_attempt_count(task_id)
    }

    fn list_tasks_for_agent_execution(&self) -> Result<Vec<TaskDto>, String> {
        self.list_tasks_for_agent_execution()
    }

    fn add_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String> {
        self.add_task_label(task_id, label)
    }

    fn remove_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String> {
        self.remove_task_label(task_id, label)
    }

    fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<TaskDto, String> {
        self.update_task_status(task_id, status)
    }

    fn load_task(&self, task_id: &str) -> Result<TaskDto, String> {
        self.load_task_by_id(task_id)
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
        TaskStatus::Cancelled => "cancelled",
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
        "created" => {
            if let Some(recurrence) = event.payload.get("recurrence").and_then(|v| v.as_str())
                && recurrence == "next_occurrence"
            {
                return format!("Generated next recurring instance of \"{title}\"");
            }
            format!("Created \"{title}\"")
        }
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

// ── RRULE recurrence calculation ──────────────────────────────────────

fn parse_time_of_day(tod: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = tod.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let hour: u32 = parts[0].parse().ok()?;
    let minute: u32 = parts[1].parse().ok()?;
    if hour > 23 || minute > 59 {
        return None;
    }
    Some((hour, minute))
}

fn apply_time_of_day(
    date: chrono::DateTime<chrono::FixedOffset>,
    tod: &str,
) -> chrono::DateTime<chrono::FixedOffset> {
    if let Some((hour, minute)) = parse_time_of_day(tod) {
        let mut d = date;
        d = d.with_hour(hour).unwrap_or(d);
        d = d.with_minute(minute).unwrap_or(d);
        d = d.with_second(0).unwrap_or(d);
        d = d.with_nanosecond(0).unwrap_or(d);
        d
    } else {
        date
    }
}

/// Calculate the next occurrence date from an RRULE string.
/// When `time_of_day` is provided (e.g. "09:00"), the returned datetime will have
/// that time applied. When `time_of_day` is None, the base_time's time is preserved.
fn calculate_next_occurrence(
    rule: &str,
    current_start: &Option<String>,
    time_of_day: Option<&str>,
) -> Option<String> {
    let base_time = current_start
        .as_ref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .unwrap_or_else(|| {
            let now = Utc::now();
            now.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
        });

    let mut interval: u32 = 1;
    let mut freq: Option<&str> = None;
    let mut by_day: Vec<String> = Vec::new();
    let mut count: Option<u32> = None;

    for part in rule.split(';') {
        let mut kv = part.splitn(2, '=');
        let key = kv.next()?.trim();
        let value = kv.next()?.trim();
        match key.to_ascii_uppercase().as_str() {
            "FREQ" => freq = Some(value),
            "INTERVAL" => interval = value.parse().unwrap_or(1),
            "BYDAY" => by_day = value.split(',').map(|s| s.trim().to_string()).collect(),
            "COUNT" => count = value.parse().ok(),
            _ => {}
        }
    }

    let freq = freq?;

    let next = match freq.to_ascii_uppercase().as_str() {
        "DAILY" => base_time + chrono::Duration::days(interval as i64),
        "WEEKLY" => {
            if by_day.is_empty() {
                base_time + chrono::Duration::weeks(interval as i64)
            } else {
                find_next_weekday(&base_time, &by_day, interval)
            }
        }
        "MONTHLY" => base_time + chrono::Months::new(interval),
        "HOURLY" => base_time + chrono::Duration::hours(interval as i64),
        _ => return None,
    };

    if let Some(max) = count
        && max == 0
    {
        return None;
    }

    let final_next = if let Some(tod) = time_of_day {
        apply_time_of_day(next, tod)
    } else {
        next
    };

    Some(final_next.to_rfc3339())
}

fn find_next_weekday(
    base: &chrono::DateTime<chrono::FixedOffset>,
    by_day: &[String],
    interval: u32,
) -> chrono::DateTime<chrono::FixedOffset> {
    use chrono::Datelike;

    let day_offset = |abbrev: &str| -> Option<u32> {
        match abbrev {
            "MO" => Some(1),
            "TU" => Some(2),
            "WE" => Some(3),
            "TH" => Some(4),
            "FR" => Some(5),
            "SA" => Some(6),
            "SU" => Some(7),
            _ => None,
        }
    };

    let target_days: Vec<u32> = by_day.iter().filter_map(|d| day_offset(d)).collect();

    let mut candidate = *base + chrono::Duration::days(1);

    for _ in 0..14 {
        let wd = candidate.weekday().num_days_from_monday() + 1;
        if target_days.contains(&wd) {
            return candidate;
        }
        candidate += chrono::Duration::days(1);
    }

    *base + chrono::Duration::weeks(interval as i64)
}
