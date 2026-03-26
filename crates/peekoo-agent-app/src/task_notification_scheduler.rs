use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use peekoo_notifications::{Notification, NotificationService};
use peekoo_scheduler::Scheduler;
use peekoo_task_app::{SqliteTaskService, TaskDto, TaskService};

const TASK_NOTIFICATION_OWNER: &str = "task-notifications";
/// Tasks overdue by more than this are considered stale and skipped on startup.
const MAX_OVERDUE_GRACE_SECS: i64 = 300;

#[derive(Clone)]
pub(crate) struct TaskNotificationScheduler {
    task_service: SqliteTaskService,
    notifications: Arc<NotificationService>,
    scheduler: Scheduler,
    started: Arc<AtomicBool>,
}

impl TaskNotificationScheduler {
    pub(crate) fn new(
        task_service: SqliteTaskService,
        notifications: Arc<NotificationService>,
    ) -> Self {
        Self {
            task_service,
            notifications,
            scheduler: Scheduler::new(),
            started: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn start(&self) -> Result<(), String> {
        if self.started.swap(true, Ordering::AcqRel) {
            return Ok(());
        }

        self.sync_all()?;

        let task_service = self.task_service.clone();
        let notifications = Arc::clone(&self.notifications);
        let scheduler = self.scheduler.clone();
        self.scheduler.start(move |owner, task_id| {
            if owner != TASK_NOTIFICATION_OWNER {
                return;
            }

            if let Err(error) =
                fire_task_notification(&task_service, &notifications, &scheduler, &task_id)
            {
                tracing::warn!(task_id, "Task notification dispatch failed: {error}");
            }
        });
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn shutdown(&self) {
        self.scheduler.shutdown_token().cancel();
    }

    pub(crate) fn sync_all(&self) -> Result<(), String> {
        for task in self.task_service.list_tasks()? {
            self.sync_task(&task)?;
        }
        Ok(())
    }

    pub(crate) fn sync_task(&self, task: &TaskDto) -> Result<(), String> {
        self.scheduler.cancel(TASK_NOTIFICATION_OWNER, &task.id);

        let Some(delay_secs) = notification_delay_secs(task, Utc::now())? else {
            return Ok(());
        };

        self.scheduler
            .set(
                TASK_NOTIFICATION_OWNER,
                &task.id,
                delay_secs.max(1),
                false,
                Some(delay_secs),
            )
            .map_err(|e| e.to_string())
    }

    pub(crate) fn remove_task(&self, task_id: &str) {
        self.scheduler.cancel(TASK_NOTIFICATION_OWNER, task_id);
    }
}

fn fire_task_notification(
    task_service: &SqliteTaskService,
    notifications: &Arc<NotificationService>,
    scheduler: &Scheduler,
    task_id: &str,
) -> Result<(), String> {
    scheduler.cancel(TASK_NOTIFICATION_OWNER, task_id);

    let task = match task_service.load_task(task_id) {
        Ok(task) => task,
        Err(error) => {
            tracing::debug!(task_id, "Skipping task notification: {error}");
            return Ok(());
        }
    };

    if !should_notify_task(&task, Utc::now())? {
        return Ok(());
    }

    let delivered = notifications.notify(Notification {
        source: "tasks".to_string(),
        title: "Task reminder".to_string(),
        body: format!("{} starts now", task.title),
        action_url: None,
        action_label: None,
        panel_label: Some("panel-tasks".to_string()),
    });

    tracing::debug!(
        task_id = task.id,
        delivered,
        "Scheduled task notification dispatched"
    );
    Ok(())
}

fn notification_delay_secs(task: &TaskDto, now: DateTime<Utc>) -> Result<Option<u64>, String> {
    let Some(start_at) = scheduled_start_at(task)? else {
        return Ok(None);
    };

    if !is_notifiable_status(&task.status) {
        return Ok(None);
    }

    let delay_ms = start_at.signed_duration_since(now).num_milliseconds();
    if delay_ms <= 0 {
        let overdue_secs = now.signed_duration_since(start_at).num_seconds();
        if overdue_secs > MAX_OVERDUE_GRACE_SECS {
            tracing::debug!(
                task_id = task.id,
                overdue_secs,
                "Skipping stale overdue task reminder"
            );
            return Ok(None);
        }

        tracing::debug!(
            task_id = task.id,
            overdue_secs,
            "Scheduling overdue task reminder immediately within grace window"
        );
        return Ok(Some(0));
    }

    Ok(Some(((delay_ms + 999) / 1000) as u64))
}

fn should_notify_task(task: &TaskDto, now: DateTime<Utc>) -> Result<bool, String> {
    let Some(start_at) = scheduled_start_at(task)? else {
        return Ok(false);
    };

    Ok(is_notifiable_status(&task.status) && start_at <= now)
}

fn scheduled_start_at(task: &TaskDto) -> Result<Option<DateTime<Utc>>, String> {
    task.scheduled_start_at
        .as_deref()
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| format!("Parse task scheduled_start_at error: {e}"))
        })
        .transpose()
}

fn is_notifiable_status(status: &str) -> bool {
    !matches!(status, "done" | "cancelled")
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::{notification_delay_secs, MAX_OVERDUE_GRACE_SECS};
    use peekoo_task_app::TaskDto;
    use peekoo_task_domain::TaskStatus;

    fn sample_task(start_at: &str) -> TaskDto {
        TaskDto {
            id: "task-1".to_string(),
            title: "Task".to_string(),
            description: None,
            status: "todo".to_string(),
            priority: "medium".to_string(),
            assignee: "user".to_string(),
            labels: vec![],
            scheduled_start_at: Some(start_at.to_string()),
            scheduled_end_at: None,
            estimated_duration_min: None,
            recurrence_rule: None,
            recurrence_time_of_day: None,
            parent_task_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            agent_work_status: None,
            agent_work_session_id: None,
            agent_work_attempt_count: None,
            agent_work_started_at: None,
            agent_work_completed_at: None,
        }
    }

    #[test]
    fn overdue_task_within_grace_window_fires_immediately() {
        let now = chrono::Utc::now();
        let task = sample_task(&(now - Duration::seconds(30)).to_rfc3339());

        let delay = notification_delay_secs(&task, now).expect("delay");

        assert_eq!(delay, Some(0));
    }

    #[test]
    fn overdue_task_beyond_grace_window_is_skipped() {
        let now = chrono::Utc::now();
        let task =
            sample_task(&(now - Duration::seconds(MAX_OVERDUE_GRACE_SECS + 30)).to_rfc3339());

        let delay = notification_delay_secs(&task, now).expect("delay");

        assert_eq!(delay, None);
    }
}
