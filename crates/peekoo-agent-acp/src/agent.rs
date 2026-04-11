use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;

use agent_client_protocol as acp;
use agent_client_protocol::{
    AgentCapabilities, Client, ContentChunk, McpCapabilities, SessionNotification, SessionUpdate,
    StopReason,
};
use anyhow::Result;
use async_trait::async_trait;
use peekoo_agent::service::AgentService;
use peekoo_agent::{
    AgentEvent, SessionType,
    config::{
        AgentProvider, AgentServiceConfig, PEEKOO_AGENT_PROVIDER_ARGS_ENV,
        PEEKOO_AGENT_PROVIDER_COMMAND_ENV, PEEKOO_OPENCODE_BIN_ENV,
    },
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::context::{TaskContext, TaskCreationContext};
// TODO: Re-enable after MCP bridge migration
// use crate::mcp_tools::{TaskScopedTool, summarize_agent_event};

#[derive(Clone)]
struct SessionContext {
    cwd: PathBuf,
    mcp_servers: Vec<acp::McpServer>,
}

pub struct PeekooAgent {
    session_update_tx: mpsc::UnboundedSender<(SessionNotification, oneshot::Sender<()>)>,
    next_session_id: Cell<u64>,
    sessions: RefCell<HashMap<String, SessionContext>>,
}

impl PeekooAgent {
    pub fn new(
        session_update_tx: mpsc::UnboundedSender<(SessionNotification, oneshot::Sender<()>)>,
    ) -> Self {
        Self {
            session_update_tx,
            next_session_id: Cell::new(0),
            sessions: RefCell::new(HashMap::new()),
        }
    }
}

enum PromptContext {
    TaskExecution(TaskContext),
    TaskCreation(TaskCreationContext),
}

fn extract_prompt_context(prompt: &[acp::ContentBlock]) -> PromptContext {
    for block in prompt {
        let acp::ContentBlock::Text(text) = block else {
            continue;
        };

        if let Ok(context) = serde_json::from_str::<TaskCreationContext>(&text.text)
            && context.request_type == "task_creation_parse"
        {
            return PromptContext::TaskCreation(context);
        }

        if let Ok(context) = serde_json::from_str::<TaskContext>(&text.text) {
            return PromptContext::TaskExecution(context);
        }
    }

    tracing::warn!("No task context provided, using default");
    PromptContext::TaskExecution(TaskContext {
        task_id: "unknown".to_string(),
        title: "Untitled Task".to_string(),
        description: None,
        status: "todo".to_string(),
        priority: "medium".to_string(),
        labels: vec![],
        scheduled_start_at: None,
        scheduled_end_at: None,
        estimated_duration_min: None,
        comments: vec![],
    })
}

#[async_trait(?Send)]
impl acp::Agent for PeekooAgent {
    async fn initialize(
        &self,
        arguments: acp::InitializeRequest,
    ) -> Result<acp::InitializeResponse, acp::Error> {
        tracing::info!("Received initialize request {arguments:?}");
        Ok(acp::InitializeResponse::new(acp::ProtocolVersion::V1)
            .agent_info(
                acp::Implementation::new("peekoo-agent-acp", "0.1.0")
                    .title(Some("Peekoo Agent".to_string())),
            )
            .agent_capabilities(
                AgentCapabilities::new().mcp_capabilities(McpCapabilities::new().http(true)),
            ))
    }

    async fn authenticate(
        &self,
        arguments: acp::AuthenticateRequest,
    ) -> Result<acp::AuthenticateResponse, acp::Error> {
        tracing::info!("Received authenticate request {arguments:?}");
        Ok(acp::AuthenticateResponse::default())
    }

    async fn new_session(
        &self,
        arguments: acp::NewSessionRequest,
    ) -> Result<acp::NewSessionResponse, acp::Error> {
        tracing::info!("Received new session request {arguments:?}");
        let session_id = self.next_session_id.get();
        self.next_session_id.set(session_id + 1);
        let session_id_string = session_id.to_string();
        self.sessions.borrow_mut().insert(
            session_id_string.clone(),
            SessionContext {
                cwd: arguments.cwd,
                mcp_servers: arguments.mcp_servers,
            },
        );
        Ok(acp::NewSessionResponse::new(session_id_string))
    }

    async fn load_session(
        &self,
        arguments: acp::LoadSessionRequest,
    ) -> Result<acp::LoadSessionResponse, acp::Error> {
        tracing::info!("Received load session request {arguments:?}");
        self.sessions.borrow_mut().insert(
            arguments.session_id.to_string(),
            SessionContext {
                cwd: arguments.cwd,
                mcp_servers: arguments.mcp_servers,
            },
        );
        Ok(acp::LoadSessionResponse::default())
    }

    async fn prompt(
        &self,
        arguments: acp::PromptRequest,
    ) -> Result<acp::PromptResponse, acp::Error> {
        tracing::info!(
            "Received prompt request for session {}",
            arguments.session_id
        );

        let prompt_context = extract_prompt_context(&arguments.prompt);

        let session_context = self
            .sessions
            .borrow()
            .get(&arguments.session_id.to_string())
            .cloned();

        let (task_prompt, task_id, startup_text, preparing_text, emit_progress_updates) =
            match &prompt_context {
                PromptContext::TaskExecution(task_context) => (
                    task_context.to_prompt(),
                    task_context.task_id.clone(),
                    format!(
                        "Task received: {}\n\nMCP servers forwarded: {}\n\nRunning agent...",
                        task_context.task_id,
                        session_context
                            .as_ref()
                            .map(|session| session.mcp_servers.len())
                            .unwrap_or_default()
                    ),
                    format!(
                        "Processing task: {}\n\nPreparing agent session...\n\n",
                        task_context.title
                    ),
                    true,
                ),
                PromptContext::TaskCreation(parse_context) => (
                    parse_context.to_prompt(),
                    "task-creation-parse".to_string(),
                    String::new(),
                    String::new(),
                    false,
                ),
            };

        if emit_progress_updates {
            let (tx, rx) = oneshot::channel();
            self.session_update_tx
                .send((
                    SessionNotification::new(
                        arguments.session_id.clone(),
                        SessionUpdate::AgentMessageChunk(ContentChunk::new(preparing_text.into())),
                    ),
                    tx,
                ))
                .map_err(|_| anyhow::anyhow!("failed to send session update"))?;
            rx.await
                .map_err(|_| anyhow::anyhow!("session update failed"))?;
        }

        let mut agent = build_agent_service(&task_id, session_context.as_ref())
            .await
            .map_err(|error| {
                tracing::error!("Failed to create task agent: {}", error);
                acp::Error::internal_error()
            })?;

        if emit_progress_updates {
            let (tx, rx) = oneshot::channel();
            self.session_update_tx
                .send((
                    SessionNotification::new(
                        arguments.session_id.clone(),
                        SessionUpdate::AgentMessageChunk(ContentChunk::new(startup_text.into())),
                    ),
                    tx,
                ))
                .map_err(|_| anyhow::anyhow!("failed to send session update"))?;
            rx.await
                .map_err(|_| anyhow::anyhow!("session update failed"))?;
        }

        let final_text = agent
            .prompt(&task_prompt, move |event: AgentEvent| {
                // TODO: Re-enable after MCP bridge migration
                // if let Some(summary) = summarize_agent_event(&event) {
                //     let (tx, _rx) = oneshot::channel();
                //     let _ = session_tx.send((
                //         SessionNotification::new(
                //             session_id.clone(),
                //             SessionUpdate::AgentMessageChunk(ContentChunk::new(summary.into())),
                //         ),
                //         tx,
                //     ));
                // }
                // For now, just emit the event without MCP summary
                let _ = event; // Silence unused warning
            })
            .await
            .map_err(|error| {
                tracing::error!("Task agent prompt failed: {}", error);
                acp::Error::internal_error()
            })?;

        let (tx, rx) = oneshot::channel();
        self.session_update_tx
            .send((
                SessionNotification::new(
                    arguments.session_id.clone(),
                    SessionUpdate::AgentMessageChunk(ContentChunk::new(final_text.into())),
                ),
                tx,
            ))
            .map_err(|_| anyhow::anyhow!("failed to send session update"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("session update failed"))?;

        Ok(acp::PromptResponse::new(StopReason::EndTurn))
    }

    async fn cancel(&self, args: acp::CancelNotification) -> Result<(), acp::Error> {
        tracing::info!("Received cancel request for session {}", args.session_id);
        Ok(())
    }

    async fn set_session_mode(
        &self,
        args: acp::SetSessionModeRequest,
    ) -> Result<acp::SetSessionModeResponse, acp::Error> {
        tracing::info!("Received set session mode request {:?}", args);
        Ok(acp::SetSessionModeResponse::default())
    }

    async fn ext_method(&self, args: acp::ExtRequest) -> Result<acp::ExtResponse, acp::Error> {
        tracing::info!("Received extension method call: method={}", args.method);
        Ok(acp::ExtResponse::new(
            serde_json::value::to_raw_value(&serde_json::json!({"status": "ok"}))?.into(),
        ))
    }

    async fn ext_notification(&self, args: acp::ExtNotification) -> Result<(), acp::Error> {
        tracing::info!("Received extension notification: method={}", args.method);
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct TaskSessionStorage {
    session_dir: Option<PathBuf>,
    session_path: Option<PathBuf>,
    no_session: bool,
}

fn build_task_session_storage(
    task_session_root: Option<&PathBuf>,
    task_id: &str,
) -> TaskSessionStorage {
    let Some(root) = task_session_root else {
        return TaskSessionStorage {
            session_dir: None,
            session_path: None,
            no_session: true,
        };
    };

    let legacy_session_path = root.join(format!("{task_id}.jsonl"));
    if legacy_session_path.exists() {
        return TaskSessionStorage {
            session_dir: None,
            session_path: Some(legacy_session_path),
            no_session: false,
        };
    }

    TaskSessionStorage {
        session_dir: Some(root.join(task_id)),
        session_path: None,
        no_session: false,
    }
}

async fn build_agent_service(
    task_id: &str,
    session_context: Option<&SessionContext>,
) -> anyhow::Result<AgentService> {
    let config = build_agent_service_config(task_id, session_context);
    AgentService::new(config).await
}

fn build_agent_service_config(
    task_id: &str,
    session_context: Option<&SessionContext>,
) -> AgentServiceConfig {
    let cwd = session_context
        .map(|session| session.cwd.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let task_session_dir = std::env::var("PEEKOO_AGENT_TASK_SESSION_DIR")
        .ok()
        .map(PathBuf::from);
    let session_storage = build_task_session_storage(task_session_dir.as_ref(), task_id);

    let mut config = AgentServiceConfig {
        working_directory: cwd,
        auto_discover: true,
        no_session: session_storage.no_session,
        session_dir: session_storage.session_dir,
        mcp_servers: session_context
            .map(|session| session.mcp_servers.clone())
            .unwrap_or_default(),
        session_type: SessionType::AcpTask,
        ..Default::default()
    };

    if let Ok(provider_str) = std::env::var("PEEKOO_AGENT_PROVIDER") {
        config.provider = match provider_str.as_str() {
            "opencode" => AgentProvider::opencode(),
            _ => AgentProvider::from_registry(&provider_str, &provider_str, vec![]),
        };
    }
    if let Ok(model) = std::env::var("PEEKOO_AGENT_MODEL") {
        config.model = Some(model);
    }
    if let Ok(api_key) = std::env::var("PEEKOO_AGENT_API_KEY") {
        config.api_key = Some(api_key);
    }
    if let Ok(opencode_bin) = std::env::var(PEEKOO_OPENCODE_BIN_ENV)
        && !opencode_bin.trim().is_empty()
    {
        config
            .environment
            .insert(PEEKOO_OPENCODE_BIN_ENV.to_string(), opencode_bin);
    }
    if let Ok(provider_command) = std::env::var(PEEKOO_AGENT_PROVIDER_COMMAND_ENV)
        && !provider_command.trim().is_empty()
    {
        config.environment.insert(
            PEEKOO_AGENT_PROVIDER_COMMAND_ENV.to_string(),
            provider_command,
        );
    }
    if let Ok(provider_args) = std::env::var(PEEKOO_AGENT_PROVIDER_ARGS_ENV)
        && !provider_args.trim().is_empty()
    {
        config
            .environment
            .insert(PEEKOO_AGENT_PROVIDER_ARGS_ENV.to_string(), provider_args);
    }

    config
}

pub async fn run_agent() -> acp::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Log MCP server connection info
    if let (Ok(port), Ok(host)) = (
        std::env::var("PEEKOO_MCP_PORT"),
        std::env::var("PEEKOO_MCP_HOST"),
    ) {
        tracing::info!("🔗 [MCP] Server configured at http://{}:{}/mcp", host, port);
    } else {
        tracing::info!("⚙️ [MCP] No MCP server configured (running without tools)");
    }

    let outgoing = TokioAsyncWriteCompatExt::compat_write(tokio::io::stdout());
    let incoming = TokioAsyncReadCompatExt::compat(tokio::io::stdin());

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async move {
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

            let (conn, handle_io) =
                acp::AgentSideConnection::new(PeekooAgent::new(tx), outgoing, incoming, |fut| {
                    tokio::task::spawn_local(fut);
                });

            tokio::task::spawn_local(async move {
                while let Some((session_notification, tx)) = rx.recv().await {
                    let result = conn.session_notification(session_notification).await;
                    if let Err(e) = result {
                        tracing::error!("Failed to send session notification: {}", e);
                        break;
                    }
                    tx.send(()).ok();
                }
            });

            handle_io.await
        })
        .await
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use agent_client_protocol::{McpServer, McpServerHttp, TextContent};

    use super::{
        SessionContext, TaskSessionStorage, build_agent_service_config, build_task_session_storage,
        extract_prompt_context,
    };
    use crate::context::{Comment, TaskCreationContext};

    #[test]
    fn reuses_legacy_session_file_when_it_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();
        let legacy = root.join("task-123.jsonl");
        fs::write(&legacy, "{}").expect("write legacy session");

        let storage = build_task_session_storage(Some(&root), "task-123");

        assert_eq!(
            storage,
            TaskSessionStorage {
                session_dir: None,
                session_path: Some(legacy),
                no_session: false,
            }
        );
    }

    #[test]
    fn creates_task_scoped_session_dir_when_no_legacy_file_exists() {
        let root = PathBuf::from("/tmp/peekoo-task-sessions");
        let storage = build_task_session_storage(Some(&root), "task-123");

        assert_eq!(
            storage,
            TaskSessionStorage {
                session_dir: Some(root.join("task-123")),
                session_path: None,
                no_session: false,
            }
        );
    }

    #[test]
    fn disables_persistence_when_no_task_session_root_is_configured() {
        assert_eq!(
            build_task_session_storage(None, "task-123"),
            TaskSessionStorage {
                session_dir: None,
                session_path: None,
                no_session: true,
            }
        );
    }

    #[test]
    fn build_agent_service_config_forwards_session_mcp_servers() {
        let session_context = SessionContext {
            cwd: PathBuf::from("/tmp/peekoo-task"),
            mcp_servers: vec![
                McpServer::Http(McpServerHttp::new(
                    "peekoo-native-tools",
                    "http://127.0.0.1:49152/mcp",
                )),
                McpServer::Http(McpServerHttp::new(
                    "peekoo-plugin-tools",
                    "http://127.0.0.1:49152/mcp/plugins",
                )),
            ],
        };

        let config = build_agent_service_config("task-123", Some(&session_context));

        assert_eq!(config.working_directory, PathBuf::from("/tmp/peekoo-task"));
        assert_eq!(config.mcp_servers, session_context.mcp_servers);
    }

    #[test]
    fn extract_task_context_finds_json_after_context_prompt() {
        let task_json = serde_json::json!({
            "task_id": "task-123",
            "title": "Finish task",
            "description": "do the thing",
            "status": "pending",
            "priority": "high",
            "labels": ["agent"],
            "scheduled_start_at": null,
            "scheduled_end_at": null,
            "estimated_duration_min": 30,
            "comments": [{
                "id": "c1",
                "author": "user",
                "text": "please handle this",
                "created_at": "2026-04-02T00:00:00Z"
            }]
        })
        .to_string();

        let prompt = vec![
            agent_client_protocol::ContentBlock::Text(TextContent::new(
                "workspace context goes here".to_string(),
            )),
            agent_client_protocol::ContentBlock::Text(TextContent::new(task_json)),
        ];

        let prompt_context = extract_prompt_context(&prompt);
        let super::PromptContext::TaskExecution(task_context) = prompt_context else {
            panic!("expected task execution context");
        };

        assert_eq!(task_context.task_id, "task-123");
        assert_eq!(task_context.title, "Finish task");
        assert_eq!(task_context.comments.len(), 1);
        assert_eq!(
            task_context.comments[0].text,
            Comment {
                id: "c1".into(),
                author: "user".into(),
                text: "please handle this".into(),
                created_at: "2026-04-02T00:00:00Z".into(),
            }
            .text
        );
    }

    #[test]
    fn extract_prompt_context_detects_task_creation_payload() {
        let parse_json = serde_json::to_string(&TaskCreationContext {
            request_type: "task_creation_parse".into(),
            raw_text: "call mom tomorrow at 3pm".into(),
            locale: Some("en-US".into()),
            timezone: Some("UTC".into()),
        })
        .expect("serialize parse context");

        let prompt = vec![agent_client_protocol::ContentBlock::Text(TextContent::new(
            parse_json,
        ))];
        let prompt_context = extract_prompt_context(&prompt);

        let super::PromptContext::TaskCreation(context) = prompt_context else {
            panic!("expected task creation context");
        };

        assert_eq!(context.request_type, "task_creation_parse");
        assert_eq!(context.raw_text, "call mom tomorrow at 3pm");
    }
}
