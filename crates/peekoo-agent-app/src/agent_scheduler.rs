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

        let _ = self
            .scheduler
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

        // TODO: Implement full ACP integration
        // This requires spawning the peekoo-agent-acp subprocess and communicating
        // via the ACP protocol over stdio. The current stub adds a comment to the task.
        //
        // Full implementation would:
        // 1. Spawn `peekoo-agent-acp` subprocess
        // 2. Create ClientSideConnection with stdin/stdout pipes
        // 3. Send ACP initialize/new_session/prompt messages
        // 4. Process agent responses and tool calls
        // 5. Update task status based on agent actions

        if let Err(e) = execute_task_stub(task_service, &task).await {
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

async fn execute_task_stub(
    task_service: &ProductivityService,
    task: &peekoo_productivity_domain::task::TaskDto,
) -> Result<(), String> {
    let comments = task_service
        .get_task_activity(&task.id, 100)
        .map_err(|e| e.to_string())?;

    let task_summary = format!(
        "**Task:** {}\n**Description:** {}\n**Priority:** {}\n**Comments:** {}",
        task.title,
        task.description.as_deref().unwrap_or("None"),
        task.priority,
        comments.len()
    );

    tracing::info!("Would execute task {}:\n{}", task.id, task_summary);

    let comment_text = format!(
        "Agent would process this task.\n\n{}",
        task_summary
    );

    task_service
        .add_task_comment(&task.id, &comment_text, "agent")
        .map_err(|e| e.to_string())?;

    task_service
        .update_agent_work_status(&task.id, "completed", None)
        .map_err(|e| e.to_string())?;

    Ok(())
}
