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

                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(rt) => rt,
                        Err(e) => {
                            tracing::error!("Failed to create tokio runtime: {}", e);
                            return;
                        }
                    };

                    rt.block_on(async {
                        if shutdown.is_cancelled() {
                            return;
                        }

                        if let Err(e) = check_and_execute_tasks(&task_service).await {
                            tracing::error!("Agent scheduler error: {}", e);
                        }
                    });
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

        if let Err(e) = execute_task_acp(task_service, &task).await {
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

async fn execute_task_acp(
    task_service: &ProductivityService,
    task: &peekoo_productivity_domain::task::TaskDto,
) -> Result<(), String> {
    use agent_client_protocol::Agent as _;

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

    tracing::info!("Spawning peekoo-agent-acp for task {}", task.id);

    let mut child = Command::new("peekoo-agent-acp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn peekoo-agent-acp: {}. Is the binary in PATH?", e))?;

    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    let local_set = LocalSet::new();

    let stop_reason = local_set
        .run_until(async move {
            let (conn, handle_io) = ClientSideConnection::new(
                TaskClient,
                stdin.compat_write(),
                stdout.compat(),
                |fut| {
                    tokio::task::spawn_local(fut);
                },
            );

            tokio::task::spawn_local(async move {
                if let Err(e) = handle_io.await {
                    tracing::error!("ACP I/O error: {}", e);
                }
            });

            tracing::debug!("Sending initialize request");
            let _init_result = conn
                .initialize(InitializeRequest::new(ProtocolVersion::V1))
                .await
                .map_err(|e| format!("ACP initialize error: {}", e))?;


            tracing::debug!("Creating new session");
            let session = conn
                .new_session(NewSessionRequest::new(
                    std::env::current_dir().unwrap_or_default(),
                ))
                .await
                .map_err(|e| format!("ACP new_session error: {}", e))?;

            let prompt_json = serde_json::to_string(&task_context)
                .map_err(|e| format!("Failed to serialize task context: {}", e))?;

            tracing::debug!("Sending prompt to agent");
            let prompt_response = conn
                .prompt(PromptRequest::new(
                    session.session_id,
                    vec![ContentBlock::Text(TextContent::new(prompt_json))],
                ))
                .await
                .map_err(|e| format!("ACP prompt error: {}", e))?;

            tracing::debug!(
                "Prompt completed with stop_reason: {:?}",
                prompt_response.stop_reason
            );

            Ok::<_, String>(prompt_response.stop_reason)
        })
        .await
        .map_err(|e| format!("ACP execution error: {}", e))?;

    drop(local_set);
    let _ = child.kill().await;

    let response_text = format!(
        "Agent completed task analysis.\n\n**Title:** {}\n**Status:** Processed with reason {:?}",
        task.title, stop_reason
    );

    task_service
        .add_task_comment(&task.id, &response_text, "agent")
        .map_err(|e| e.to_string())?;
    task_service
        .update_agent_work_status(&task.id, "completed", None)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[derive(Clone)]
struct TaskClient;

#[async_trait::async_trait(?Send)]
impl Client for TaskClient {
    async fn request_permission(
        &self,
        _args: agent_client_protocol::RequestPermissionRequest,
    ) -> Result<agent_client_protocol::RequestPermissionResponse, agent_client_protocol::Error>
    {
        Ok(agent_client_protocol::RequestPermissionResponse::new(
            agent_client_protocol::RequestPermissionOutcome::Cancelled,
        ))
    }

    async fn session_notification(
        &self,
        args: agent_client_protocol::SessionNotification,
    ) -> Result<(), agent_client_protocol::Error> {
        if let agent_client_protocol::SessionUpdate::AgentMessageChunk(chunk) = &args.update
            && let agent_client_protocol::ContentBlock::Text(text) = &chunk.content
        {
            tracing::info!("Agent message: {}", text.text);
        }
        Ok(())
    }
}
