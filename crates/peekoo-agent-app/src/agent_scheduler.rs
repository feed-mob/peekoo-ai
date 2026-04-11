use std::sync::Arc;
use std::sync::Mutex;

use agent_client_protocol::{ContentBlock, TextContent};
use peekoo_scheduler::Scheduler;

use peekoo_task_app::SqliteTaskService;

/// Maximum number of agent execution attempts before a task is permanently marked as failed.
const MAX_AGENT_ATTEMPTS: u32 = 3;
const TASK_CONTEXT_ACTIVITY_LIMIT: u32 = u32::MAX;

pub struct AgentScheduler {
    scheduler: Scheduler,
    task_service: Arc<SqliteTaskService>,
    shutdown_token: tokio_util::sync::CancellationToken,
    launch_env: Arc<Mutex<Vec<(String, String)>>>,
    context_prompt: Arc<Mutex<Option<String>>>,
    bundled_acp_path: Option<std::path::PathBuf>,
}

impl AgentScheduler {
    pub fn new(
        task_service: Arc<SqliteTaskService>,
        bundled_acp_path: Option<std::path::PathBuf>,
    ) -> Self {
        tracing::info!("AgentScheduler initialized");
        Self {
            scheduler: Scheduler::new(),
            task_service,
            shutdown_token: tokio_util::sync::CancellationToken::new(),
            launch_env: Arc::new(Mutex::new(Vec::new())),
            context_prompt: Arc::new(Mutex::new(None)),
            bundled_acp_path,
        }
    }

    pub fn set_agent_launch_env(&self, launch_env: Vec<(String, String)>) {
        if let Ok(mut guard) = self.launch_env.lock() {
            *guard = launch_env;
        }
    }

    pub fn set_context_prompt(&self, prompt: String) {
        if let Ok(mut guard) = self.context_prompt.lock() {
            *guard = Some(prompt);
        }
    }

    pub fn start(&self) {
        tracing::info!("AgentScheduler starting - will check for tasks every 30 seconds");

        let task_service = Arc::clone(&self.task_service);
        let shutdown = self.shutdown_token.clone();
        let launch_env = Arc::clone(&self.launch_env);
        let context_prompt = Arc::clone(&self.context_prompt);
        let bundled_acp_path = self.bundled_acp_path.clone();

        let _ = self
            .scheduler
            .set("agent-scheduler", "check-tasks", 30, true, Some(5));

        self.scheduler.start(move |owner, key| {
            if owner == "agent-scheduler" && key == "check-tasks" {
                tracing::info!("AgentScheduler tick - checking for agent tasks");

                let task_service = Arc::clone(&task_service);
                let shutdown = shutdown.clone();
                let launch_env = Arc::clone(&launch_env);
                let context_prompt = Arc::clone(&context_prompt);
                let bundled_acp_path = bundled_acp_path.clone();

                Self::spawn_worker(
                    task_service,
                    shutdown,
                    launch_env,
                    context_prompt,
                    bundled_acp_path,
                );
            }
        });

        tracing::info!("AgentScheduler started successfully");
    }

    pub fn trigger_now(&self) {
        tracing::info!("AgentScheduler: Triggering immediate task check");
        Self::spawn_worker(
            Arc::clone(&self.task_service),
            self.shutdown_token.clone(),
            Arc::clone(&self.launch_env),
            Arc::clone(&self.context_prompt),
            self.bundled_acp_path.clone(),
        );
    }

    pub fn shutdown(&self) {
        tracing::info!("AgentScheduler shutting down");
        self.shutdown_token.cancel();
        self.scheduler.cancel_all("agent-scheduler");
        tracing::info!("AgentScheduler shutdown complete");
    }

    fn spawn_worker(
        task_service: Arc<SqliteTaskService>,
        shutdown: tokio_util::sync::CancellationToken,
        launch_env: Arc<Mutex<Vec<(String, String)>>>,
        context_prompt: Arc<Mutex<Option<String>>>,
        bundled_acp_path: Option<std::path::PathBuf>,
    ) {
        std::thread::spawn(move || {
            tracing::info!("AgentScheduler: Spawning worker thread for task execution");

            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => {
                    tracing::info!("AgentScheduler: Tokio runtime created successfully");
                    rt
                }
                Err(e) => {
                    tracing::error!("AgentScheduler: Failed to create tokio runtime: {}", e);
                    return;
                }
            };

            rt.block_on(async {
                if shutdown.is_cancelled() {
                    tracing::info!("AgentScheduler: Shutdown requested, skipping task check");
                    return;
                }

                let mcp_address = crate::mcp_server::get_mcp_address();
                if mcp_address.is_none() {
                    tracing::warn!(
                        "AgentScheduler: MCP server not running, agents will run without tools"
                    );
                }

                let launch_env = launch_env.lock().map(|g| g.clone()).unwrap_or_default();
                let context_prompt = context_prompt.lock().ok().and_then(|g| g.clone());
                if let Err(e) = check_and_execute_tasks(
                    &task_service,
                    mcp_address,
                    &launch_env,
                    context_prompt.as_deref(),
                    bundled_acp_path.as_deref(),
                )
                .await
                {
                    tracing::error!("AgentScheduler: Error during task execution: {}", e);
                } else {
                    tracing::info!("AgentScheduler: Task check completed successfully");
                }
            });

            tracing::info!("AgentScheduler: Worker thread finished");
        });
    }
}

async fn check_and_execute_tasks(
    task_service: &SqliteTaskService,
    mcp_address: Option<std::net::SocketAddr>,
    launch_env: &[(String, String)],
    context_prompt: Option<&str>,
    bundled_acp_path: Option<&std::path::Path>,
) -> Result<(), String> {
    tracing::info!("AgentScheduler: Querying database for agent tasks");

    let tasks = task_service.list_tasks_for_agent_execution().map_err(|e| {
        tracing::error!("AgentScheduler: Failed to list tasks: {}", e);
        e.to_string()
    })?;

    if tasks.is_empty() {
        tracing::info!("AgentScheduler: No agent tasks found for execution");
        return Ok(());
    }

    tracing::info!(
        "AgentScheduler: Found {} tasks for agent execution",
        tasks.len()
    );

    for task in &tasks {
        tracing::info!(
            "AgentScheduler: Processing task {} - '{}' (assignee: {}, status: {})",
            task.id,
            task.title,
            task.assignee,
            task.agent_work_status.as_deref().unwrap_or("none")
        );

        let claimed = match task_service.claim_task_for_agent(&task.id) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("AgentScheduler: Failed to claim task {}: {}", task.id, e);
                continue;
            }
        };

        if !claimed {
            tracing::warn!(
                "AgentScheduler: Task {} already claimed by another scheduler, skipping",
                task.id
            );
            continue;
        }

        tracing::info!(
            "AgentScheduler: Successfully claimed task {} for agent execution",
            task.id
        );

        if let Err(e) = task_service.update_agent_work_status(&task.id, "executing", None) {
            tracing::error!(
                "AgentScheduler: Failed to update task {} status to executing: {}",
                task.id,
                e
            );
        } else {
            tracing::info!(
                "AgentScheduler: Updated task {} status to executing",
                task.id
            );
        }

        tracing::info!(
            "AgentScheduler: Starting ACP execution for task {}",
            task.id
        );
        if let Err(e) = execute_task_acp(
            task_service,
            task,
            mcp_address,
            launch_env,
            context_prompt,
            bundled_acp_path,
        )
        .await
        {
            tracing::error!(
                "AgentScheduler: Failed to execute task {} via ACP: {}",
                task.id,
                e
            );

            if let Err(e) = task_service.update_agent_work_status(&task.id, "failed", None) {
                tracing::error!(
                    "AgentScheduler: Failed to update task {} status to failed: {}",
                    task.id,
                    e
                );
            }

            match task_service.increment_attempt_count(&task.id) {
                Ok(count) => {
                    tracing::info!(
                        "AgentScheduler: Incremented attempt count for task {} to {}",
                        task.id,
                        count
                    );
                    if count >= MAX_AGENT_ATTEMPTS {
                        tracing::warn!(
                            "AgentScheduler: Task {} has exhausted all {} retry attempts, will not be retried",
                            task.id,
                            MAX_AGENT_ATTEMPTS
                        );
                    }
                }
                Err(e) => tracing::error!(
                    "AgentScheduler: Failed to increment attempt count for task {}: {}",
                    task.id,
                    e
                ),
            }
        } else {
            tracing::info!(
                "AgentScheduler: Successfully executed task {} via ACP",
                task.id
            );
        }
    }

    Ok(())
}

async fn execute_task_acp(
    task_service: &SqliteTaskService,
    task: &peekoo_task_app::TaskDto,
    mcp_address: Option<std::net::SocketAddr>,
    launch_env: &[(String, String)],
    context_prompt: Option<&str>,
    bundled_acp_path: Option<&std::path::Path>,
) -> Result<(), String> {
    tracing::info!(
        "AgentScheduler: Preparing task context for task {}: '{}'",
        task.id,
        task.title
    );

    if let Some(addr) = mcp_address {
        tracing::info!(
            "🔗 [MCP] Using shared server at http://{}:{}/mcp for task {}",
            addr.ip(),
            addr.port(),
            task.id
        );
    } else {
        tracing::warn!(
            "⚠️ [MCP] No MCP server configured for task {}, agents will run without tools",
            task.id
        );
    }

    let comments = task_service
        .get_task_activity(&task.id, TASK_CONTEXT_ACTIVITY_LIMIT)
        .map_err(|e| {
            tracing::error!(
                "AgentScheduler: Failed to get task activity for {}: {}",
                task.id,
                e
            );
            e.to_string()
        })?;

    tracing::info!(
        "AgentScheduler: Retrieved {} comments for task {}",
        comments.len(),
        task.id
    );
    let initial_activity_count = comments.len();

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
        "comments": build_task_comment_context(&comments)
    });

    let prompt_json = serde_json::to_string(&task_context)
        .map_err(|e| format!("Failed to serialize task context: {e}"))?;

    let mut content_blocks = Vec::new();
    if let Some(ctx) = context_prompt {
        content_blocks.push(ContentBlock::Text(TextContent::new(ctx.to_string())));
    }
    content_blocks.push(ContentBlock::Text(TextContent::new(prompt_json)));

    let prompt_result = crate::acp_client::run_prompt_and_collect(
        &format!("task-execution:{}", task.id),
        content_blocks,
        launch_env.to_vec(),
        mcp_address,
        bundled_acp_path.map(std::path::Path::to_path_buf),
        None,
    )
    .await
    .map_err(|e| format!("ACP execution error: {e}"))?;

    tracing::info!(
        "AgentScheduler: ACP prompt completed for task {} - stop_reason: {:?}",
        task.id,
        prompt_result.stop_reason
    );

    tracing::info!(
        "AgentScheduler: Updating task {} agent_work_status to completed",
        task.id
    );

    let final_activity_count = task_service
        .get_task_activity(&task.id, 100)
        .map(|events| events.len())
        .unwrap_or(initial_activity_count);
    if final_activity_count <= initial_activity_count {
        return Err(
            "Agent completed without recording any task update through MCP tools".to_string(),
        );
    }

    if let Err(e) = task_service.update_agent_work_status(&task.id, "completed", None) {
        tracing::error!(
            "AgentScheduler: Failed to update task {} agent_work_status to completed: {}",
            task.id,
            e
        );
    }

    tracing::info!(
        "AgentScheduler: Task {} execution completed successfully",
        task.id
    );

    Ok(())
}

fn build_task_comment_context(events: &[peekoo_task_app::TaskEventDto]) -> Vec<serde_json::Value> {
    let mut comments = events
        .iter()
        .filter(|event| event.event_type == "comment")
        .filter_map(|event| {
            let text = event.payload.get("text")?.as_str()?.trim();
            if text.is_empty() {
                return None;
            }

            Some(serde_json::json!({
                "id": event.id,
                "author": event.payload.get("author").and_then(|v| v.as_str()).unwrap_or("unknown"),
                "text": text,
                "created_at": event.created_at
            }))
        })
        .collect::<Vec<_>>();

    comments.reverse();
    comments
}

#[cfg(test)]
mod tests {
    use super::build_task_comment_context;
    use peekoo_task_app::TaskEventDto;

    #[test]
    fn builds_comment_context_from_comment_events_only_in_chronological_order() {
        let events = vec![
            TaskEventDto {
                id: "status-1".into(),
                task_id: "task-1".into(),
                event_type: "status_changed".into(),
                payload: serde_json::json!({"from": "todo", "to": "done"}),
                created_at: "2026-03-24T08:10:00Z".into(),
            },
            TaskEventDto {
                id: "comment-2".into(),
                task_id: "task-1".into(),
                event_type: "comment".into(),
                payload: serde_json::json!({"author": "user", "text": "@peekoo-agent follow up"}),
                created_at: "2026-03-24T08:20:00Z".into(),
            },
            TaskEventDto {
                id: "comment-1".into(),
                task_id: "task-1".into(),
                event_type: "comment".into(),
                payload: serde_json::json!({"author": "agent", "text": "First reply"}),
                created_at: "2026-03-24T08:00:00Z".into(),
            },
        ];

        let comments = build_task_comment_context(&events);

        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0]["id"], "comment-1");
        assert_eq!(comments[1]["id"], "comment-2");
        assert_eq!(comments[1]["text"], "@peekoo-agent follow up");
    }
}
