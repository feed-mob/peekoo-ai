use std::sync::Arc;

use peekoo_scheduler::Scheduler;

use crate::productivity::ProductivityService;

pub struct AgentScheduler {
    scheduler: Scheduler,
    task_service: Arc<ProductivityService>,
    shutdown_token: tokio_util::sync::CancellationToken,
}

impl AgentScheduler {
    pub fn new(task_service: Arc<ProductivityService>) -> Self {
        Self {
            scheduler: Scheduler::new(),
            task_service,
            shutdown_token: tokio_util::sync::CancellationToken::new(),
        }
    }

    pub fn start(&self) {
        let task_service = Arc::clone(&self.task_service);
        let shutdown = self.shutdown_token.clone();

        let _ = self.scheduler
            .set("agent-scheduler", "check-tasks", 30, true, Some(5));

        self.scheduler.start(move |owner, key| {
            if owner == "agent-scheduler" && key == "check-tasks" {
                let task_service = Arc::clone(&task_service);
                let shutdown = shutdown.clone();

                tokio::spawn(async move {
                    if shutdown.is_cancelled() {
                        return;
                    }

                    if let Err(e) = check_and_execute_tasks(&task_service).await {
                        tracing::error!("Agent scheduler error: {}", e);
                    }
                });
            }
        });
    }

    pub fn shutdown(&self) {
        self.shutdown_token.cancel();
        self.scheduler.cancel_all("agent-scheduler");
    }
}

async fn check_and_execute_tasks(task_service: &ProductivityService) -> Result<(), String> {
    let tasks = task_service
        .list_tasks_for_agent_execution()
        .map_err(|e| e.to_string())?;

    for task in tasks {
        tracing::info!("Found agent task: {} - {}", task.id, task.title);

        let claimed = task_service
            .claim_task_for_agent(&task.id)
            .map_err(|e| e.to_string())?;

        if !claimed {
            tracing::debug!("Task {} already claimed by another scheduler", task.id);
            continue;
        }

        tracing::info!("Claimed task {} for agent execution", task.id);

        let _ = task_service
            .update_agent_work_status(&task.id, "executing", None)
            .map_err(|e| e.to_string());

        if let Err(e) = execute_task(task_service, &task).await {
            tracing::error!("Failed to execute task {}: {}", task.id, e);
            let _ = task_service
                .update_agent_work_status(&task.id, "failed", None)
                .map_err(|e| e.to_string());
            let _ = task_service
                .increment_attempt_count(&task.id)
                .map_err(|e| e.to_string());
        }
    }

    Ok(())
}

async fn execute_task(task_service: &ProductivityService, task: &peekoo_productivity_domain::task::TaskDto) -> Result<(), String> {
    let comments = task_service
        .get_task_activity(&task.id, 100)
        .map_err(|e| e.to_string())?;

    let task_context = serde_json::json!({
        "task_id": task.id,
        "title": task.title,
        "description": task.description,
        "status": task.status,
        "priority": task.priority,
        "labels": task.labels,
        "scheduled_start_at": task.scheduled_start_at,
        "scheduled_end_at": task.scheduled_end_at,
        "estimated_duration_min": task.estimated_duration_min,
        "comments": comments.iter().map(|c| {
            serde_json::json!({
                "id": c.id,
                "author": c.payload.get("author").and_then(|v| v.as_str()).unwrap_or("unknown"),
                "text": c.payload.get("text").and_then(|v| v.as_str()).unwrap_or(""),
                "created_at": c.created_at
            })
        }).collect::<Vec<_>>()
    });

    let prompt_text = format!(
        "# Task Assignment\n\nYou have been assigned the following task:\n\n**Title:** {}\n**Description:** {}\n**Priority:** {}\n**Status:** {}\n\nPlease analyze this task and take appropriate action.",
        task.title,
        task_context["description"].as_str().unwrap_or("None"),
        task.priority,
        task.status
    );

    tracing::info!("Would execute task {} with prompt: {}", task.id, prompt_text);

    task_service
        .add_task_comment(&task.id, &prompt_text, "agent")
        .map_err(|e| e.to_string())?;

    task_service
        .update_agent_work_status(&task.id, "completed", None)
        .map_err(|e| e.to_string())?;

    Ok(())
}
