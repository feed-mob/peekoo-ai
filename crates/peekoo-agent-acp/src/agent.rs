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
    AgentEvent,
    config::{AgentProvider, AgentServiceConfig, PEEKOO_OPENCODE_BIN_ENV},
};
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::context::TaskContext;
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

        let task_context: TaskContext = arguments
            .prompt
            .first()
            .and_then(|block| {
                if let acp::ContentBlock::Text(text) = block {
                    serde_json::from_str(&text.text).ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                tracing::warn!("No task context provided, using default");
                TaskContext {
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
                }
            });

        let session_context = self
            .sessions
            .borrow()
            .get(&arguments.session_id.to_string())
            .cloned();

        let task_prompt = task_context.to_prompt();

        let (tx, rx) = oneshot::channel();
        self.session_update_tx
            .send((
                SessionNotification::new(
                    arguments.session_id.clone(),
                    SessionUpdate::AgentMessageChunk(ContentChunk::new(
                        format!(
                            "Processing task: {}\n\nPreparing agent session...\n\n",
                            task_context.title
                        )
                        .into(),
                    )),
                ),
                tx,
            ))
            .map_err(|_| anyhow::anyhow!("failed to send session update"))?;
        rx.await
            .map_err(|_| anyhow::anyhow!("session update failed"))?;

        let mut agent = build_agent_service(&task_context.task_id, session_context.as_ref())
            .await
            .map_err(|error| {
                tracing::error!("Failed to create task agent: {}", error);
                acp::Error::internal_error()
            })?;

        // TODO: Re-enable MCP tool registration after migration
        // let mut _mcp_handles = Vec::new();
        // let tools_count = if let Some(session) = &session_context {
        //     let mut all_tools = Vec::new();
        //     for server in &session.mcp_servers {
        //         let url = match server {
        //             acp::McpServer::Http(http) => http.url.clone(),
        //             _ => {
        //                 tracing::warn!("Skipping non-HTTP MCP server: {:?}", server);
        //                 continue;
        //             }
        //         };
        //         match peekoo_agent::mcp_client::connect_http_mcp_tools(&url).await {
        //             Ok((tools, handle)) => {
        //                 tracing::info!(
        //                     url = url.as_str(),
        //                     tool_count = tools.len(),
        //                     "Connected MCP server for task {}",
        //                     task_context.task_id
        //                 );
        //                 let task_id = task_context.task_id.clone();
        //                 let wrapped: Vec<Box<dyn pi::tools::Tool>> = tools
        //                     .into_iter()
        //                     .map(|t| -> Box<dyn pi::tools::Tool> {
        //                         if TaskScopedTool::needs_scoping(t.name()) {
        //                             Box::new(TaskScopedTool::new(t, task_id.clone()))
        //                         } else {
        //                             t
        //                         }
        //                     })
        //                     .collect();
        //                 all_tools.extend(wrapped);
        //                 _mcp_handles.push(handle);
        //             }
        //             Err(e) => {
        //                 tracing::error!("Failed to connect MCP server {}: {}", url, e);
        //             }
        //         }
        //     }
        //     let count = all_tools.len();
        //     agent.register_native_tools(all_tools);
        //     count
        // } else {
        //     0
        // };
        let tools_count = 0; // Temporary: no MCP tools during migration

        let startup_text = format!(
            "Task received: {}\n\nMCP tools available: {}\n\nRunning agent...",
            task_context.task_id, tools_count
        );

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

        let session_id = arguments.session_id.clone();
        let session_tx = self.session_update_tx.clone();
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
        ..Default::default()
    };

    if let Ok(provider_str) = std::env::var("PEEKOO_AGENT_PROVIDER") {
        // Parse provider string to AgentProvider enum
        config.provider = match provider_str.as_str() {
            "opencode" => AgentProvider::Opencode,
            "claude-code" => AgentProvider::ClaudeCode,
            "codex" => AgentProvider::Codex,
            _ => AgentProvider::Opencode, // default
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

    AgentService::new(config).await.map_err(Into::into)
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

    use super::{TaskSessionStorage, build_task_session_storage};

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
}
