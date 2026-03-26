use std::sync::Arc;

use peekoo_notifications::{Notification, NotificationService};
use peekoo_task_app::{TaskDto, TaskEventDto, TaskService};
use peekoo_task_domain::TaskStatus;

use crate::task_notification_scheduler::TaskNotificationScheduler;
use peekoo_task_app::SqliteTaskService;

#[derive(Clone)]
pub(crate) struct TaskRuntimeService {
    task_service: SqliteTaskService,
    notifications: Arc<NotificationService>,
    task_notifications: Arc<TaskNotificationScheduler>,
    follow_up_trigger: Option<Arc<dyn Fn(String) + Send + Sync>>,
    task_change_callback: Option<Arc<dyn Fn(Option<String>) + Send + Sync>>,
}

impl TaskRuntimeService {
    pub(crate) fn new(
        task_service: SqliteTaskService,
        notifications: Arc<NotificationService>,
        task_notifications: Arc<TaskNotificationScheduler>,
        follow_up_trigger: Option<Arc<dyn Fn(String) + Send + Sync>>,
        task_change_callback: Option<Arc<dyn Fn(Option<String>) + Send + Sync>>,
    ) -> Self {
        Self {
            task_service,
            notifications,
            task_notifications,
            follow_up_trigger,
            task_change_callback,
        }
    }

    fn maybe_requeue_agent_on_mention(&self, task: &TaskDto, text: &str, author: &str) {
        if is_agent_author(author) {
            return;
        }

        if task.assignee != "peekoo-agent" || !contains_agent_mention(text) {
            return;
        }

        if let Err(error) = self.task_service.requeue_agent_task(&task.id) {
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
        if !is_agent_author(author) {
            return;
        }

        let delivered = self.notifications.notify(Notification {
            source: "tasks".to_string(),
            title: format!("Agent commented on {}", task.title),
            body: summarize_comment(text),
            action_url: None,
            action_label: None,
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
            action_url: None,
            action_label: None,
        });

        tracing::debug!(
            task_id = task.id,
            delivered,
            "Agent status notification dispatched"
        );
    }

    fn emit_task_changed(&self, task_id: Option<&str>) {
        if let Some(callback) = &self.task_change_callback {
            callback(task_id.map(|id| id.to_string()));
        }
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
        let task = self.task_service.create_task(
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
        )?;
        self.task_notifications.sync_task(&task)?;
        self.emit_task_changed(Some(&task.id));
        Ok(task)
    }

    fn list_tasks(&self) -> Result<Vec<TaskDto>, String> {
        self.task_service.list_tasks()
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
        let task = self.task_service.update_task(
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
        )?;
        self.task_notifications.sync_task(&task)?;
        self.emit_task_changed(Some(&task.id));
        Ok(task)
    }

    fn delete_task(&self, id: &str) -> Result<(), String> {
        self.task_service.delete_task(id)?;
        self.task_notifications.remove_task(id);
        self.emit_task_changed(Some(id));
        Ok(())
    }

    fn toggle_task(&self, id: &str) -> Result<TaskDto, String> {
        let task = self.task_service.toggle_task(id)?;
        self.task_notifications.sync_task(&task)?;
        self.emit_task_changed(Some(&task.id));
        Ok(task)
    }

    fn get_task_activity(&self, task_id: &str, limit: u32) -> Result<Vec<TaskEventDto>, String> {
        self.task_service.get_task_activity(task_id, limit)
    }

    fn add_task_comment(
        &self,
        task_id: &str,
        text: &str,
        author: &str,
    ) -> Result<TaskEventDto, String> {
        let task = self.task_service.load_task(task_id)?;
        let event = self.task_service.add_task_comment(task_id, text, author)?;

        self.maybe_requeue_agent_on_mention(&task, text, author);
        self.maybe_notify_agent_comment(&task, text, author);
        self.emit_task_changed(Some(task_id));

        Ok(event)
    }

    fn claim_task_for_agent(&self, task_id: &str) -> Result<bool, String> {
        self.task_service.claim_task_for_agent(task_id)
    }

    fn update_agent_work_status(
        &self,
        task_id: &str,
        status: &str,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        self.task_service
            .update_agent_work_status(task_id, status, session_id)?;
        self.emit_task_changed(Some(task_id));
        Ok(())
    }

    fn increment_attempt_count(&self, task_id: &str) -> Result<u32, String> {
        let count = self.task_service.increment_attempt_count(task_id)?;
        self.emit_task_changed(Some(task_id));
        Ok(count)
    }

    fn list_tasks_for_agent_execution(&self) -> Result<Vec<TaskDto>, String> {
        self.task_service.list_tasks_for_agent_execution()
    }

    fn add_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String> {
        let task = self.task_service.add_task_label(task_id, label)?;
        self.emit_task_changed(Some(task_id));
        Ok(task)
    }

    fn remove_task_label(&self, task_id: &str, label: &str) -> Result<TaskDto, String> {
        let task = self.task_service.remove_task_label(task_id, label)?;
        self.emit_task_changed(Some(task_id));
        Ok(task)
    }

    fn update_task_status(&self, task_id: &str, status: TaskStatus) -> Result<TaskDto, String> {
        let task = self.task_service.load_task(task_id)?;
        let updated = self.task_service.update_task_status(task_id, status)?;
        if task.assignee == "peekoo-agent" {
            self.notify_agent_status_change(&task, status);
        }
        self.task_notifications.sync_task(&updated)?;
        self.emit_task_changed(Some(task_id));
        Ok(updated)
    }

    fn load_task(&self, task_id: &str) -> Result<TaskDto, String> {
        self.task_service.load_task(task_id)
    }
}

fn is_agent_author(author: &str) -> bool {
    author.eq_ignore_ascii_case("agent") || author.eq_ignore_ascii_case("peekoo-agent")
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
    use std::thread;
    use std::time::{Duration, Instant};

    use chrono::Utc;
    use peekoo_notifications::NotificationService;
    use rusqlite::Connection;

    use super::TaskRuntimeService;
    use crate::task_notification_scheduler::TaskNotificationScheduler;
    use peekoo_task_app::SqliteTaskService;
    use peekoo_task_app::TaskService;
    use peekoo_task_domain::TaskStatus;

    fn test_task_service() -> SqliteTaskService {
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
              finished_at TEXT,
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
        SqliteTaskService::new(Arc::new(Mutex::new(conn)))
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

    fn test_task_notifications(
        task_service: &SqliteTaskService,
        notifications: Arc<NotificationService>,
    ) -> Arc<TaskNotificationScheduler> {
        let task_notifications = Arc::new(TaskNotificationScheduler::new(
            task_service.clone(),
            notifications,
        ));
        task_notifications
            .start()
            .expect("start task notifications");
        task_notifications
    }

    fn wait_for_notification(
        receiver: &mut tokio::sync::mpsc::UnboundedReceiver<peekoo_notifications::Notification>,
        timeout: Duration,
    ) -> Option<peekoo_notifications::Notification> {
        let start = Instant::now();
        loop {
            match receiver.try_recv() {
                Ok(notification) => return Some(notification),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    if start.elapsed() >= timeout {
                        return None;
                    }
                    thread::sleep(Duration::from_millis(20));
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return None,
            }
        }
    }

    #[test]
    fn mention_requeues_agent_task_without_notification() {
        let task_service = test_task_service();
        let (notifications, mut receiver) = NotificationService::new();
        let notifications = Arc::new(notifications);
        let task_notifications = test_task_notifications(&task_service, Arc::clone(&notifications));
        let service = TaskRuntimeService::new(
            task_service.clone(),
            notifications,
            task_notifications,
            None,
            None,
        );
        let task_id = create_task(&service, "peekoo-agent");

        task_service
            .update_agent_work_status(&task_id, "completed", None)
            .expect("mark completed");

        service
            .add_task_comment(&task_id, "@peekoo-agent can you also add tests?", "user")
            .expect("add comment");

        let task = task_service.load_task(&task_id).expect("load task");
        assert_eq!(task.agent_work_status.as_deref(), Some("pending"));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn scheduled_task_sends_notification_at_start_time() {
        let task_service = test_task_service();
        let (notifications, mut receiver) = NotificationService::new();
        let notifications = Arc::new(notifications);
        let task_notifications = test_task_notifications(&task_service, Arc::clone(&notifications));
        let service = TaskRuntimeService::new(
            task_service.clone(),
            notifications,
            Arc::clone(&task_notifications),
            None,
            None,
        );

        let start_at = (Utc::now() + chrono::Duration::seconds(2)).to_rfc3339();

        service
            .create_task(
                "Join standup",
                "medium",
                "user",
                &[],
                None,
                Some(&start_at),
                None,
                None,
                None,
                None,
            )
            .expect("create scheduled task");

        let notification = wait_for_notification(&mut receiver, Duration::from_secs(4))
            .expect("scheduled task notification");
        assert_eq!(notification.source, "tasks");
        assert_eq!(notification.title, "Task reminder");
        assert_eq!(notification.body, "Join standup starts now");
        task_notifications.shutdown();
    }

    #[test]
    fn completing_scheduled_task_cancels_due_notification() {
        let task_service = test_task_service();
        let (notifications, mut receiver) = NotificationService::new();
        let notifications = Arc::new(notifications);
        let task_notifications = test_task_notifications(&task_service, Arc::clone(&notifications));
        let service = TaskRuntimeService::new(
            task_service.clone(),
            notifications,
            Arc::clone(&task_notifications),
            None,
            None,
        );

        let start_at = (Utc::now() + chrono::Duration::seconds(2)).to_rfc3339();
        let task_id = service
            .create_task(
                "Send report",
                "medium",
                "user",
                &[],
                None,
                Some(&start_at),
                None,
                None,
                None,
                None,
            )
            .expect("create scheduled task")
            .id;

        service
            .update_task_status(&task_id, TaskStatus::Done)
            .expect("complete task");

        assert!(wait_for_notification(&mut receiver, Duration::from_secs(4)).is_none());
        task_notifications.shutdown();
    }

    #[test]
    fn mention_requeues_even_if_task_was_marked_executing() {
        let task_service = test_task_service();
        let (notifications, _receiver) = NotificationService::new();
        let notifications = Arc::new(notifications);
        let task_notifications = test_task_notifications(&task_service, Arc::clone(&notifications));
        let service = TaskRuntimeService::new(
            task_service.clone(),
            notifications,
            task_notifications,
            None,
            None,
        );
        let task_id = create_task(&service, "peekoo-agent");

        task_service
            .update_agent_work_status(&task_id, "executing", Some("session-1"))
            .expect("mark executing");

        service
            .add_task_comment(
                &task_id,
                "@peekoo-agent please answer the new request",
                "user",
            )
            .expect("add comment");

        let task = task_service.load_task(&task_id).expect("load task");
        assert_eq!(task.agent_work_status.as_deref(), Some("pending"));
        assert_eq!(task.agent_work_session_id, None);
    }

    #[test]
    fn mention_invokes_follow_up_trigger() {
        let task_service = test_task_service();
        let (notifications, _receiver) = NotificationService::new();
        let notifications = Arc::new(notifications);
        let task_notifications = test_task_notifications(&task_service, Arc::clone(&notifications));
        let triggered = Arc::new(Mutex::new(Vec::<String>::new()));
        let triggered_clone = Arc::clone(&triggered);
        let service = TaskRuntimeService::new(
            task_service.clone(),
            notifications,
            task_notifications,
            Some(Arc::new(move |task_id| {
                triggered_clone.lock().expect("trigger lock").push(task_id);
            })),
            None,
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
        let task_service = test_task_service();
        let (notifications, mut receiver) = NotificationService::new();
        let notifications = Arc::new(notifications);
        let task_notifications = test_task_notifications(&task_service, Arc::clone(&notifications));
        let service = TaskRuntimeService::new(
            task_service.clone(),
            notifications,
            task_notifications,
            None,
            None,
        );
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
        let task_service = test_task_service();
        let (notifications, mut receiver) = NotificationService::new();
        let notifications = Arc::new(notifications);
        let task_notifications = test_task_notifications(&task_service, Arc::clone(&notifications));
        let service = TaskRuntimeService::new(
            task_service.clone(),
            notifications,
            task_notifications,
            None,
            None,
        );
        let task_id = create_task(&service, "peekoo-agent");

        service
            .update_task_status(&task_id, TaskStatus::Done)
            .expect("update status");

        let notification = receiver.try_recv().expect("status notification");
        assert!(notification.title.contains("Agent updated"));
        assert!(notification.body.contains("done"));
    }

    #[test]
    fn agent_comment_emits_task_changed_callback() {
        let task_service = test_task_service();
        let (notifications, _receiver) = NotificationService::new();
        let notifications = Arc::new(notifications);
        let task_notifications = test_task_notifications(&task_service, Arc::clone(&notifications));
        let changed = Arc::new(Mutex::new(Vec::<Option<String>>::new()));
        let changed_clone = Arc::clone(&changed);
        let service = TaskRuntimeService::new(
            task_service.clone(),
            notifications,
            task_notifications,
            None,
            Some(Arc::new(move |task_id| {
                changed_clone.lock().expect("changed lock").push(task_id);
            })),
        );
        let task_id = create_task(&service, "peekoo-agent");
        changed.lock().expect("changed lock").clear();

        service
            .add_task_comment(&task_id, "Background update", "peekoo-agent")
            .expect("add agent comment");

        let recorded = changed.lock().expect("changed lock");
        assert_eq!(recorded.as_slice(), &[Some(task_id)]);
    }
}
