use std::sync::Arc;
use std::sync::Mutex;

use agent_client_protocol::{
    Client, ClientSideConnection, ContentBlock, InitializeRequest, McpServer, McpServerHttp,
    NewSessionRequest, PromptRequest, ProtocolVersion, TextContent,
};
use peekoo_scheduler::Scheduler;
use tokio::process::Command;
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

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
}

impl AgentScheduler {
    pub fn new(task_service: Arc<SqliteTaskService>) -> Self {
        tracing::info!("AgentScheduler initialized");
        Self {
            scheduler: Scheduler::new(),
            task_service,
            shutdown_token: tokio_util::sync::CancellationToken::new(),
            launch_env: Arc::new(Mutex::new(Vec::new())),
            context_prompt: Arc::new(Mutex::new(None)),
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

                Self::spawn_worker(task_service, shutdown, launch_env, context_prompt);
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
                if let Err(e) =
                    check_and_execute_tasks(&task_service, mcp_address, &launch_env, context_prompt.as_deref()).await
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
        if let Err(e) = execute_task_acp(task_service, task, mcp_address, launch_env, context_prompt).await {
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
) -> Result<(), String> {
    use agent_client_protocol::Agent as _;

    tracing::info!(
        "AgentScheduler: Preparing task context for task {}: '{}'",
        task.id,
        task.title
    );

    // Get MCP server address (shared across all tasks)
    let (mcp_host, mcp_port) = if let Some(addr) = mcp_address {
        let host = addr.ip().to_string();
        let port = addr.port();
        tracing::info!(
            "🔗 [MCP] Using shared server at http://{}:{}/mcp for task {}",
            host,
            port,
            task.id
        );
        (host, port)
    } else {
        tracing::warn!(
            "⚠️ [MCP] No MCP server configured for task {}, agents will run without tools",
            task.id
        );
        ("127.0.0.1".to_string(), 0)
    };

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

    tracing::info!(
        "AgentScheduler: Spawning peekoo-agent-acp subprocess for task {}",
        task.id
    );

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

    // Pass MCP server address via environment variables
    let mut cmd = Command::new(&command_path);
    if mcp_address.is_some() {
        cmd.env("PEEKOO_MCP_PORT", mcp_port.to_string())
            .env("PEEKOO_MCP_HOST", &mcp_host);
    }
    for (key, value) in launch_env {
        cmd.env(key, value);
    }

    let mut child = match cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()
    {
        Ok(child) => {
            tracing::info!(
                "AgentScheduler: Successfully spawned {:?} subprocess (pid: {:?})",
                command_path,
                child.id()
            );
            child
        }
        Err(e) => {
            tracing::error!("AgentScheduler: Failed to spawn {:?}: {}", command_path, e);
            return Err(format!("Failed to spawn {:?}: {}", command_path, e));
        }
    };

    let stdin = child.stdin.take().expect("stdin should be available");
    let stdout = child.stdout.take().expect("stdout should be available");

    tracing::info!(
        "AgentScheduler: Setting up ACP LocalSet for task {}",
        task.id
    );
    let local_set = LocalSet::new();

    let task_id_for_spawn = task.id.clone();
    let task_id_for_logs = task.id.clone();

    let _stop_reason = match local_set
        .run_until(async move {
            let task_id = task_id_for_logs.clone();
            tracing::info!(
                "AgentScheduler: Creating ClientSideConnection for task {}",
                task_id
            );

            let (conn, handle_io) = ClientSideConnection::new(
                TaskClient {
                    task_id: task_id.clone(),
                },
                stdin.compat_write(),
                stdout.compat(),
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );

            let task_id_spawn = task_id.clone();
            tokio::task::spawn_local(async move {
                if let Err(e) = handle_io.await {
                    tracing::error!(
                        "AgentScheduler: ACP I/O error for task {}: {}",
                        task_id_spawn,
                        e
                    );
                }
            });

            tracing::info!(
                "AgentScheduler: Sending ACP initialize request for task {}",
                task_id
            );
            let init_result = conn
                .initialize(InitializeRequest::new(ProtocolVersion::V1))
                .await
                .map_err(|e| {
                    tracing::error!(
                        "AgentScheduler: ACP initialize failed for task {}: {}",
                        task_id,
                        e
                    );
                    format!("ACP initialize error: {}", e)
                })?;

            tracing::info!(
                "AgentScheduler: ACP initialize successful for task {} - agent: {:?}",
                task_id,
                init_result.agent_info
            );

            tracing::info!("AgentScheduler: Creating ACP session for task {}", task_id);
            let mcp_servers = build_session_mcp_servers(mcp_address);
            let session = conn
                .new_session(
                    NewSessionRequest::new(std::env::current_dir().unwrap_or_default())
                        .mcp_servers(mcp_servers),
                )
                .await
                .map_err(|e| {
                    tracing::error!(
                        "AgentScheduler: ACP new_session failed for task {}: {}",
                        task_id,
                        e
                    );
                    format!("ACP new_session error: {}", e)
                })?;

            tracing::info!(
                "AgentScheduler: ACP session created for task {} - session_id: {}",
                task_id,
                session.session_id
            );

            let prompt_json = serde_json::to_string(&task_context).map_err(|e| {
                tracing::error!(
                    "AgentScheduler: Failed to serialize task context for task {}: {}",
                    task_id,
                    e
                );
                format!("Failed to serialize task context: {}", e)
            })?;

            tracing::info!(
                "AgentScheduler: Sending ACP prompt to agent for task {} (context size: {} bytes)",
                task_id,
                prompt_json.len()
            );

            // Build content blocks: context prompt first, then task context
            let mut content_blocks = Vec::new();
            if let Some(ctx) = context_prompt {
                content_blocks.push(ContentBlock::Text(TextContent::new(ctx.to_string())));
            }
            content_blocks.push(ContentBlock::Text(TextContent::new(prompt_json)));

            let prompt_response = conn
                .prompt(PromptRequest::new(
                    session.session_id,
                    content_blocks,
                ))
                .await
                .map_err(|e| {
                    tracing::error!(
                        "AgentScheduler: ACP prompt failed for task {}: {}",
                        task_id,
                        e
                    );
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
            tracing::info!(
                "AgentScheduler: ACP communication completed successfully for task {}",
                task_id_for_spawn
            );
            reason
        }
        Err(e) => {
            tracing::error!(
                "AgentScheduler: ACP execution error for task {}: {}",
                task_id_for_spawn,
                e
            );
            return Err(format!("ACP execution error: {}", e));
        }
    };

    let task_id_final = task.id.clone();

    tracing::info!(
        "AgentScheduler: Cleaning up ACP subprocess for task {}",
        task_id_final
    );
    drop(local_set);

    match child.kill().await {
        Ok(_) => tracing::info!(
            "AgentScheduler: Successfully killed peekoo-agent-acp subprocess for task {}",
            task_id_final
        ),
        Err(e) => tracing::warn!(
            "AgentScheduler: Error killing subprocess for task {}: {}",
            task_id_final,
            e
        ),
    }

    tracing::info!(
        "AgentScheduler: Updating task {} agent_work_status to completed",
        task_id_final
    );

    let final_activity_count = task_service
        .get_task_activity(&task_id_final, 100)
        .map(|events| events.len())
        .unwrap_or(initial_activity_count);
    if final_activity_count <= initial_activity_count {
        return Err(
            "Agent completed without recording any task update through MCP tools".to_string(),
        );
    }

    if let Err(e) = task_service.update_agent_work_status(&task_id_final, "completed", None) {
        tracing::error!(
            "AgentScheduler: Failed to update task {} agent_work_status to completed: {}",
            task_id_final,
            e
        );
    }

    tracing::info!(
        "AgentScheduler: Task {} execution completed successfully",
        task_id_final
    );

    Ok(())
}

pub(crate) fn build_session_mcp_servers(
    mcp_address: Option<std::net::SocketAddr>,
) -> Vec<McpServer> {
    mcp_address
        .map(|addr| {
            let base_url = peekoo_mcp_server::mcp_url_for(addr);
            let plugins_url = format!("{}/plugins", base_url);
            vec![
                McpServer::Http(McpServerHttp::new("peekoo-native-tools", base_url)),
                McpServer::Http(McpServerHttp::new("peekoo-plugin-tools", plugins_url)),
            ]
        })
        .unwrap_or_default()
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
    use super::{build_session_mcp_servers, build_task_comment_context};
    use peekoo_task_app::TaskEventDto;

    #[test]
    fn builds_http_mcp_server_for_session() {
        let servers = build_session_mcp_servers(Some(([127, 0, 0, 1], 49152).into()));
        let serialized = serde_json::to_value(&servers).expect("serialize mcp servers");
        assert_eq!(serialized[0]["type"], "http");
        assert_eq!(serialized[0]["name"], "peekoo-native-tools");
        assert_eq!(serialized[0]["url"], "http://127.0.0.1:49152/mcp");
    }

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

#[derive(Clone)]
struct TaskClient {
    task_id: String,
}

#[async_trait::async_trait(?Send)]
impl Client for TaskClient {
    async fn request_permission(
        &self,
        args: agent_client_protocol::RequestPermissionRequest,
    ) -> Result<agent_client_protocol::RequestPermissionResponse, agent_client_protocol::Error>
    {
        tracing::debug!(
            "AgentScheduler: Agent requested permission for task {} - selecting first allow option if available",
            self.task_id
        );
        if let Some(option) = args.options.iter().find(|option| {
            matches!(
                option.kind,
                agent_client_protocol::PermissionOptionKind::AllowOnce
                    | agent_client_protocol::PermissionOptionKind::AllowAlways
            )
        }) {
            Ok(agent_client_protocol::RequestPermissionResponse::new(
                agent_client_protocol::RequestPermissionOutcome::Selected(
                    agent_client_protocol::SelectedPermissionOutcome::new(option.option_id.clone()),
                ),
            ))
        } else {
            Ok(agent_client_protocol::RequestPermissionResponse::new(
                agent_client_protocol::RequestPermissionOutcome::Cancelled,
            ))
        }
    }

    async fn session_notification(
        &self,
        args: agent_client_protocol::SessionNotification,
    ) -> Result<(), agent_client_protocol::Error> {
        match &args.update {
            agent_client_protocol::SessionUpdate::AgentMessageChunk(chunk) => {
                if let agent_client_protocol::ContentBlock::Text(text) = &chunk.content {
                    tracing::info!(
                        "AgentScheduler: Agent message for task {}: {}",
                        self.task_id,
                        text.text.chars().take(200).collect::<String>()
                    );
                } else {
                    tracing::debug!(
                        "AgentScheduler: Agent sent non-text content for task {}",
                        self.task_id
                    );
                }
            }
            agent_client_protocol::SessionUpdate::ToolCall(tool_call) => {
                tracing::info!(
                    "AgentScheduler: Agent tool call for task {}: {} - {:?}",
                    self.task_id,
                    tool_call.title,
                    tool_call.kind
                );
            }
            agent_client_protocol::SessionUpdate::ToolCallUpdate(update) => {
                tracing::info!(
                    "AgentScheduler: Agent tool call update for task {}: {:?}",
                    self.task_id,
                    update.fields
                );
            }
            _ => {
                tracing::debug!(
                    "AgentScheduler: Received session update for task {}: {:?}",
                    self.task_id,
                    args.update
                );
            }
        }
        Ok(())
    }
}
