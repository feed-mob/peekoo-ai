use std::sync::Arc;

use agent_client_protocol::{
    Client, ClientSideConnection, ContentBlock, InitializeRequest, NewSessionRequest,
    ProtocolVersion, PromptRequest, TextContent,
};
use peekoo_scheduler::Scheduler;
use tokio::process::Command;
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::productivity::ProductivityService;

/// Maximum number of agent execution attempts before a task is permanently marked as failed.
const MAX_AGENT_ATTEMPTS: u32 = 3;

pub struct AgentScheduler {
    scheduler: Scheduler,
    task_service: Arc<ProductivityService>,
    shutdown_token: tokio_util::sync::CancellationToken,
}

impl AgentScheduler {
    pub fn new(task_service: Arc<ProductivityService>) -> Self {
        tracing::info!("AgentScheduler initialized");
        Self {
            scheduler: Scheduler::new(),
            task_service,
            shutdown_token: tokio_util::sync::CancellationToken::new(),
        }
    }

    pub fn start(&self) {
        tracing::info!("AgentScheduler starting - will check for tasks every 30 seconds");
        
        let task_service = Arc::clone(&self.task_service);
        let shutdown = self.shutdown_token.clone();

        let _ = self
            .scheduler
            .set("agent-scheduler", "check-tasks", 30, true, Some(5));

        self.scheduler.start(move |owner, key| {
            if owner == "agent-scheduler" && key == "check-tasks" {
                tracing::info!("AgentScheduler tick - checking for agent tasks");
                
                let task_service = Arc::clone(&task_service);
                let shutdown = shutdown.clone();

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

                        if let Err(e) = check_and_execute_tasks(&task_service).await {
                            tracing::error!("AgentScheduler: Error during task execution: {}", e);
                        } else {
                            tracing::info!("AgentScheduler: Task check completed successfully");
                        }
                    });
                    
                    tracing::info!("AgentScheduler: Worker thread finished");
                });
            }
        });
        
        tracing::info!("AgentScheduler started successfully");
    }

    pub fn shutdown(&self) {
        tracing::info!("AgentScheduler shutting down");
        self.shutdown_token.cancel();
        self.scheduler.cancel_all("agent-scheduler");
        tracing::info!("AgentScheduler shutdown complete");
    }
}

async fn check_and_execute_tasks(task_service: &ProductivityService) -> Result<(), String> {
    tracing::info!("AgentScheduler: Querying database for agent tasks");
    
    let tasks = task_service
        .list_tasks_for_agent_execution()
        .map_err(|e| {
            tracing::error!("AgentScheduler: Failed to list tasks: {}", e);
            e.to_string()
        })?;

    if tasks.is_empty() {
        tracing::info!("AgentScheduler: No agent tasks found for execution");
        return Ok(());
    }
    
    tracing::info!("AgentScheduler: Found {} tasks for agent execution", tasks.len());

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
            tracing::warn!("AgentScheduler: Task {} already claimed by another scheduler, skipping", task.id);
            continue;
        }

        tracing::info!("AgentScheduler: Successfully claimed task {} for agent execution", task.id);

        if let Err(e) = task_service.update_agent_work_status(&task.id, "executing", None) {
            tracing::error!("AgentScheduler: Failed to update task {} status to executing: {}", task.id, e);
        } else {
            tracing::info!("AgentScheduler: Updated task {} status to executing", task.id);
        }

        tracing::info!("AgentScheduler: Starting ACP execution for task {}", task.id);
        if let Err(e) = execute_task_acp(task_service, task).await {
            tracing::error!("AgentScheduler: Failed to execute task {} via ACP: {}", task.id, e);
            
            if let Err(e) = task_service.update_agent_work_status(&task.id, "failed", None) {
                tracing::error!("AgentScheduler: Failed to update task {} status to failed: {}", task.id, e);
            }
            
            match task_service.increment_attempt_count(&task.id) {
                Ok(count) => {
                    tracing::info!("AgentScheduler: Incremented attempt count for task {} to {}", task.id, count);
                    if count >= MAX_AGENT_ATTEMPTS {
                        tracing::warn!("AgentScheduler: Task {} has exhausted all {} retry attempts, will not be retried", task.id, MAX_AGENT_ATTEMPTS);
                    }
                }
                Err(e) => tracing::error!("AgentScheduler: Failed to increment attempt count for task {}: {}", task.id, e),
            }
        } else {
            tracing::info!("AgentScheduler: Successfully executed task {} via ACP", task.id);
        }
    }

    Ok(())
}

async fn execute_task_acp(
    task_service: &ProductivityService,
    task: &peekoo_productivity_domain::task::TaskDto,
) -> Result<(), String> {
    use agent_client_protocol::Agent as _;

    tracing::info!("AgentScheduler: Preparing task context for task {}: '{}'", task.id, task.title);

    let comments = task_service
        .get_task_activity(&task.id, 100)
        .map_err(|e| {
            tracing::error!("AgentScheduler: Failed to get task activity for {}: {}", task.id, e);
            e.to_string()
        })?;

    tracing::info!("AgentScheduler: Retrieved {} comments for task {}", comments.len(), task.id);

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

    tracing::info!("AgentScheduler: Spawning peekoo-agent-acp subprocess for task {}", task.id);

    let bin_name = if cfg!(windows) {
        "peekoo-agent-acp.exe"
    } else {
        "peekoo-agent-acp"
    };

    let command_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join(bin_name)))
        .filter(|p| p.exists())
        .unwrap_or_else(|| std::path::PathBuf::from(bin_name));

    let mut child = match Command::new(&command_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
    {
        Ok(child) => {
            tracing::info!("AgentScheduler: Successfully spawned {:?} subprocess (pid: {:?})", command_path, child.id());
            child
        }
        Err(e) => {
            tracing::error!("AgentScheduler: Failed to spawn {:?}: {}", command_path, e);
            return Err(format!("Failed to spawn {:?}: {}", command_path, e));
        }
    };

    let stdin = child.stdin.take().expect("stdin should be available");
    let stdout = child.stdout.take().expect("stdout should be available");

    tracing::info!("AgentScheduler: Setting up ACP LocalSet for task {}", task.id);
    let local_set = LocalSet::new();

    let task_id_for_spawn = task.id.clone();
    let task_id_for_logs = task.id.clone();
    
    let stop_reason = match local_set
        .run_until(async move {
            let task_id = task_id_for_logs.clone();
            tracing::info!("AgentScheduler: Creating ClientSideConnection for task {}", task_id);
            
            let (conn, handle_io) = ClientSideConnection::new(
                TaskClient { task_id: task_id.clone() },
                stdin.compat_write(),
                stdout.compat(),
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );

            let task_id_spawn = task_id.clone();
            tokio::task::spawn_local(async move {
                if let Err(e) = handle_io.await {
                    tracing::error!("AgentScheduler: ACP I/O error for task {}: {}", task_id_spawn, e);
                }
            });

            tracing::info!("AgentScheduler: Sending ACP initialize request for task {}", task_id);
            let init_result = conn
                .initialize(InitializeRequest::new(ProtocolVersion::V1))
                .await
                .map_err(|e| {
                    tracing::error!("AgentScheduler: ACP initialize failed for task {}: {}", task_id, e);
                    format!("ACP initialize error: {}", e)
                })?;
            
            tracing::info!(
                "AgentScheduler: ACP initialize successful for task {} - agent: {:?}",
                task_id,
                init_result.agent_info
            );

            tracing::info!("AgentScheduler: Creating ACP session for task {}", task_id);
            let session = conn
                .new_session(NewSessionRequest::new(
                    std::env::current_dir().unwrap_or_default(),
                ))
                .await
                .map_err(|e| {
                    tracing::error!("AgentScheduler: ACP new_session failed for task {}: {}", task_id, e);
                    format!("ACP new_session error: {}", e)
                })?;
            
            tracing::info!(
                "AgentScheduler: ACP session created for task {} - session_id: {}",
                task_id,
                session.session_id
            );

            let prompt_json = serde_json::to_string(&task_context)
                .map_err(|e| {
                    tracing::error!("AgentScheduler: Failed to serialize task context for task {}: {}", task_id, e);
                    format!("Failed to serialize task context: {}", e)
                })?;

            tracing::info!(
                "AgentScheduler: Sending ACP prompt to agent for task {} (context size: {} bytes)",
                task_id,
                prompt_json.len()
            );
            
            let prompt_response = conn
                .prompt(PromptRequest::new(
                    session.session_id,
                    vec![ContentBlock::Text(TextContent::new(prompt_json))],
                ))
                .await
                .map_err(|e| {
                    tracing::error!("AgentScheduler: ACP prompt failed for task {}: {}", task_id, e);
                    format!("ACP prompt error: {}", e)
                })?;

            tracing::info!(
                "AgentScheduler: ACP prompt completed for task {} - stop_reason: {:?}",
                task_id,
                prompt_response.stop_reason
            );

            Ok::<_, String>(prompt_response.stop_reason)
        })
        .await
    {
        Ok(reason) => {
            tracing::info!("AgentScheduler: ACP communication completed successfully for task {}", task_id_for_spawn);
            reason
        }
        Err(e) => {
            tracing::error!("AgentScheduler: ACP execution error for task {}: {}", task_id_for_spawn, e);
            return Err(format!("ACP execution error: {}", e));
        }
    };

    let task_id_final = task.id.clone();
    let task_title = task.title.clone();
    
    tracing::info!("AgentScheduler: Cleaning up ACP subprocess for task {}", task_id_final);
    drop(local_set);
    
    match child.kill().await {
        Ok(_) => tracing::info!("AgentScheduler: Successfully killed peekoo-agent-acp subprocess for task {}", task_id_final),
        Err(e) => tracing::warn!("AgentScheduler: Error killing subprocess for task {}: {}", task_id_final, e),
    }

    let response_text = format!(
        "Agent completed task analysis.\n\n**Title:** {}\n**Status:** Processed with reason {:?}",
        task_title, stop_reason
    );

    tracing::info!("AgentScheduler: Adding completion comment to task {}", task_id_final);
    task_service
        .add_task_comment(&task_id_final, &response_text, "agent")
        .map_err(|e| {
            tracing::error!("AgentScheduler: Failed to add comment to task {}: {}", task_id_final, e);
            e.to_string()
        })?;
    
    tracing::info!("AgentScheduler: Updating task {} status to completed", task_id_final);
    task_service
        .update_agent_work_status(&task_id_final, "completed", None)
        .map_err(|e| {
            tracing::error!("AgentScheduler: Failed to update task {} status to completed: {}", task_id_final, e);
            e.to_string()
        })?;
    
    tracing::info!("AgentScheduler: Task {} execution completed successfully", task_id_final);

    Ok(())
}

#[derive(Clone)]
struct TaskClient {
    task_id: String,
}

#[async_trait::async_trait(?Send)]
impl Client for TaskClient {
    async fn request_permission(
        &self,
        _args: agent_client_protocol::RequestPermissionRequest,
    ) -> Result<agent_client_protocol::RequestPermissionResponse, agent_client_protocol::Error>
    {
        tracing::debug!("AgentScheduler: Agent requested permission for task {} - auto-granting", self.task_id);
        Ok(agent_client_protocol::RequestPermissionResponse::new(
            agent_client_protocol::RequestPermissionOutcome::Cancelled,
        ))
    }

    async fn session_notification(
        &self,
        args: agent_client_protocol::SessionNotification,
    ) -> Result<(), agent_client_protocol::Error> {
        match &args.update {
            agent_client_protocol::SessionUpdate::AgentMessageChunk(chunk) => {
                if let agent_client_protocol::ContentBlock::Text(text) = &chunk.content {
                    tracing::info!("AgentScheduler: Agent message for task {}: {}", self.task_id, text.text.chars().take(200).collect::<String>());
                } else {
                    tracing::debug!("AgentScheduler: Agent sent non-text content for task {}", self.task_id);
                }
            }
            agent_client_protocol::SessionUpdate::ToolCall(tool_call) => {
                tracing::info!("AgentScheduler: Agent tool call for task {}: {} - {:?}", self.task_id, tool_call.title, tool_call.kind);
            }
            agent_client_protocol::SessionUpdate::ToolCallUpdate(update) => {
                tracing::info!("AgentScheduler: Agent tool call update for task {}: {:?}", self.task_id, update.fields);
            }
            _ => {
                tracing::debug!("AgentScheduler: Received session update for task {}: {:?}", self.task_id, args.update);
            }
        }
        Ok(())
    }
}
