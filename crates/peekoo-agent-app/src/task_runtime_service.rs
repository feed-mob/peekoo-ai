use std::sync::Arc;

use peekoo_notifications::{Notification, NotificationService};
use peekoo_productivity_domain::task::{TaskDto, TaskEventDto, TaskService, TaskStatus};

use crate::productivity::ProductivityService;

#[derive(Clone)]
pub(crate) struct TaskRuntimeService {
    productivity: ProductivityService,
    notifications: Arc<NotificationService>,
    follow_up_trigger: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl TaskRuntimeService {
    pub(crate) fn new(
        productivity: ProductivityService,
        notifications: Arc<NotificationService>,
        follow_up_trigger: Option<Arc<dyn Fn(String) + Send + Sync>>,
    ) -> Self {
        Self {
            productivity,
            notifications,
            follow_up_trigger,
        }
    }

    fn maybe_requeue_agent_on_mention(&self, task: &TaskDto, text: &str, author: &str) {
        if author.eq_ignore_ascii_case("agent") {
            return;
        }

        if task.assignee != "peekoo-agent" || !contains_agent_mention(text) {
            return;
        }

        if let Err(error) = self.productivity.requeue_agent_task(&task.id) {
            tracing::error!(
                "Failed to requeue mentioned agent task {}: {}",
                task.id,
                error
            );
            return;
        }

        if let Some(trigger) = &self.follow_up_trigger {
            trigger(task.id.clone());
        }
    }

    fn maybe_notify_agent_comment(&self, task: &TaskDto, text: &str, author: &str) {
        if !author.eq_ignore_ascii_case("agent") {
            return;
        }

        let delivered = self.notifications.notify(Notification {
            source: "tasks".to_string(),
            title: format!("Agent commented on {}", task.title),
            body: summarize_comment(text),
        });

        tracing::debug!(
            task_id = task.id,
            delivered,
            "Agent comment notification dispatched"
        );
    }

    fn notify_agent_status_change(&self, task: &TaskDto, status: TaskStatus) {
        let delivered = self.notifications.notify(Notification {
            source: "tasks".to_string(),
            title: format!("Agent updated {}", task.title),
            body: format!("Status changed to {}", status_label(status)),
        });

        tracing::debug!(
            task_id = task.id,
            delivered,
            "Agent status notification dispatched"
        );
    }
}

impl TaskService for TaskRuntimeService {
    fn create_task(
        &self,
        title: &str,
        priority: &str,
        assignee: &str,
        labels: &[String],
        desc: Option<&str>,
        scheduled_start_at: Option<&str>,
        scheduled_end_at: Option<&str>,
        estimated_duration_min: Option<u32>,
        recurrence_rule: Option<&str>,
        recurrence_time_of_day: Option<&str>,
    ) -> Result<TaskDto, String> {
        self.productivity.create_task(
            title,
            priority,
            assignee,
            labels,
            desc,
            scheduled_start_at,
            scheduled_end_at,
            estimated_duration_min,
            recurrence_rule,
            recurrence_time_of_day,
        )
    }

    fn list_tasks(&self) -> Result<Vec<TaskDto>, String> {
        self.productivity.list_tasks()
    }

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
        self.productivity.update_task(
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
        self.productivity.delete_task(id)
    }

    fn toggle_task(&self, id: &str) -> Result<TaskDto, String> {
        self.productivity.toggle_task(id)
    }

    fn get_task_activity(&self, task_id: &str, limit: u32) -> Result<Vec<TaskEventDto>, String> {
        self.productivity.get_task_activity(task_id, limit)
    }

    fn add_task_comment(
        &self,
        task_id: &str,
        text: &str,
        author: &str,
    ) -> Result<TaskEventDto, String> {
        let task = self.productivity.load_task(task_id)?;
        let event = self.productivity.add_task_comment(task_id, text, author)?;

        self.maybe_requeue_agent_on_mention(&task, text, author);
        self.maybe_notify_agent_comment(&task, text, author);

        Ok(event)
    }

    fn claim_task_for_agent(&self, task_id: &str) -> Result<bool, String> {
        self.productivity.claim_task_for_agent(task_id)
    }

    fn update_agent_work_status(
        &self,
        task_id: &str,
        status: &str,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        self.productivity
            .update_agent_work_status(task_id, status, session_id)
    }

    fn increment_attempt_count(&self, task_id: &str) -> Result<u32, String> {
        self.productivity.increment_attempt_count(task_id)
    }

    fn list_tasks_for_agent_execution(&self) -> Result<Vec<TaskDto>, String> {
        self.productivity.list_tasks_for_agent_execution()
    }

    fn add_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String> {
        self.productivity.add_task_label(task_id, label)
    }

    fn remove_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String> {
        self.productivity.remove_task_label(task_id, label)
    }

    fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<TaskDto, String> {
        let task = self.productivity.load_task(task_id)?;
        let updated = self.productivity.update_task_status(task_id, status)?;
        if task.assignee == "peekoo-agent" {
            self.notify_agent_status_change(&task, status);
        }
        Ok(updated)
    }

    fn load_task(&self, task_id: &str) -> Result<TaskDto, String> {
        self.productivity.load_task(task_id)
    }
}

fn contains_agent_mention(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        token
            .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '@')
            .eq_ignore_ascii_case("@peekoo-agent")
    })
}

fn summarize_comment(text: &str) -> String {
    const LIMIT: usize = 120;
    let trimmed = text.trim();
    if trimmed.chars().count() <= LIMIT {
        return trimmed.to_string();
    }

    let summary: String = trimmed.chars().take(LIMIT - 1).collect();
    format!("{}...", summary)
}

fn status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
        TaskStatus::Cancelled => "cancelled",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use peekoo_notifications::NotificationService;
    use rusqlite::Connection;

    use super::TaskRuntimeService;
    use crate::productivity::ProductivityService;
    use peekoo_productivity_domain::task::{TaskService, TaskStatus};

    fn test_productivity() -> ProductivityService {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            r#"
            CREATE TABLE tasks (
              id TEXT PRIMARY KEY,
              title TEXT NOT NULL,
              notes TEXT,
              status TEXT NOT NULL,
              priority TEXT NOT NULL,
              assignee TEXT NOT NULL,
              labels_json TEXT NOT NULL DEFAULT '[]',
              scheduled_start_at TEXT,
              scheduled_end_at TEXT,
              estimated_duration_min INTEGER,
              recurrence_rule TEXT,
              recurrence_time_of_day TEXT,
              parent_task_id TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              agent_work_status TEXT,
              agent_work_session_id TEXT,
              agent_work_attempt_count INTEGER DEFAULT 0,
              agent_work_started_at TEXT,
              agent_work_completed_at TEXT
            );
            CREATE TABLE task_events (
              id TEXT PRIMARY KEY,
              task_id TEXT NOT NULL,
              event_type TEXT NOT NULL,
              payload_json TEXT NOT NULL,
              created_at TEXT NOT NULL
            );
            CREATE TABLE pomodoro_sessions (
              id TEXT PRIMARY KEY,
              task_id TEXT,
              started_at TEXT NOT NULL,
              ended_at TEXT,
              duration_sec INTEGER NOT NULL,
              interruptions INTEGER NOT NULL DEFAULT 0,
              notes TEXT
            );
            "#,
        )
        .expect("schema");
        ProductivityService::new(Arc::new(Mutex::new(conn)))
    }

    fn create_task(service: &TaskRuntimeService, assignee: &str) -> String {
        service
            .create_task(
                "Tell me a joke",
                "medium",
                assignee,
                &[],
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .expect("create task")
            .id
    }

    #[test]
    fn mention_requeues_agent_task_without_notification() {
        let productivity = test_productivity();
        let (notifications, mut receiver) = NotificationService::new();
        let service = TaskRuntimeService::new(productivity.clone(), Arc::new(notifications), None);
        let task_id = create_task(&service, "peekoo-agent");

        productivity
            .update_agent_work_status(&task_id, "completed", None)
            .expect("mark completed");

        service
            .add_task_comment(&task_id, "@peekoo-agent can you also add tests?", "user")
            .expect("add comment");

        let task = productivity.load_task(&task_id).expect("load task");
        assert_eq!(task.agent_work_status.as_deref(), Some("pending"));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn mention_requeues_even_if_task_was_marked_executing() {
        let productivity = test_productivity();
        let (notifications, _receiver) = NotificationService::new();
        let service = TaskRuntimeService::new(productivity.clone(), Arc::new(notifications), None);
        let task_id = create_task(&service, "peekoo-agent");

        productivity
            .update_agent_work_status(&task_id, "executing", Some("session-1"))
            .expect("mark executing");

        service
            .add_task_comment(
                &task_id,
                "@peekoo-agent please answer the new request",
                "user",
            )
            .expect("add comment");

        let task = productivity.load_task(&task_id).expect("load task");
        assert_eq!(task.agent_work_status.as_deref(), Some("pending"));
        assert_eq!(task.agent_work_session_id, None);
    }

    #[test]
    fn mention_invokes_follow_up_trigger() {
        let productivity = test_productivity();
        let (notifications, _receiver) = NotificationService::new();
        let triggered = Arc::new(Mutex::new(Vec::<String>::new()));
        let triggered_clone = Arc::clone(&triggered);
        let service = TaskRuntimeService::new(
            productivity.clone(),
            Arc::new(notifications),
            Some(Arc::new(move |task_id| {
                triggered_clone.lock().expect("trigger lock").push(task_id);
            })),
        );
        let task_id = create_task(&service, "peekoo-agent");

        service
            .add_task_comment(&task_id, "@peekoo-agent follow up", "user")
            .expect("add comment");

        let recorded = triggered.lock().expect("triggered lock");
        assert_eq!(recorded.as_slice(), &[task_id]);
    }

    #[test]
    fn agent_comment_sends_notification() {
        let productivity = test_productivity();
        let (notifications, mut receiver) = NotificationService::new();
        let service = TaskRuntimeService::new(productivity.clone(), Arc::new(notifications), None);
        let task_id = create_task(&service, "peekoo-agent");

        service
            .add_task_comment(&task_id, "Here is a joke for you.", "agent")
            .expect("add agent comment");

        let notification = receiver.try_recv().expect("agent notification");
        assert!(notification.title.contains("Agent commented on"));
        assert!(notification.body.contains("Here is a joke"));
    }

    #[test]
    fn agent_status_change_sends_notification() {
        let productivity = test_productivity();
        let (notifications, mut receiver) = NotificationService::new();
        let service = TaskRuntimeService::new(productivity.clone(), Arc::new(notifications), None);
        let task_id = create_task(&service, "peekoo-agent");

        service
            .update_task_status(&task_id, TaskStatus::Done)
            .expect("update status");

        let notification = receiver.try_recv().expect("status notification");
        assert!(notification.title.contains("Agent updated"));
        assert!(notification.body.contains("done"));
    }
}
