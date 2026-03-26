use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use peekoo_notifications::{Notification, NotificationService};
use peekoo_scheduler::Scheduler;
use peekoo_task_app::{SqliteTaskService, TaskDto, TaskService};

const TASK_NOTIFICATION_OWNER: &str = "task-notifications";

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
        return Ok(None);
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
