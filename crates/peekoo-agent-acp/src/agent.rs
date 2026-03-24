use std::cell::Cell;

use agent_client_protocol as acp;
use agent_client_protocol::{Client, ContentChunk, SessionNotification, SessionUpdate, StopReason};
use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::{mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::context::TaskContext;

pub struct PeekooAgent {
    session_update_tx: mpsc::UnboundedSender<(SessionNotification, oneshot::Sender<()>)>,
    next_session_id: Cell<u64>,
}

impl PeekooAgent {
    pub fn new(
        session_update_tx: mpsc::UnboundedSender<(SessionNotification, oneshot::Sender<()>)>,
    ) -> Self {
        Self {
            session_update_tx,
            next_session_id: Cell::new(0),
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
        Ok(
            acp::InitializeResponse::new(acp::ProtocolVersion::V1).agent_info(
                acp::Implementation::new("peekoo-agent-acp", "0.1.0")
                    .title(Some("Peekoo Agent".to_string())),
            ),
        )
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
        Ok(acp::NewSessionResponse::new(session_id.to_string()))
    }

    async fn load_session(
        &self,
        arguments: acp::LoadSessionRequest,
    ) -> Result<acp::LoadSessionResponse, acp::Error> {
        tracing::info!("Received load session request {arguments:?}");
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

        // Connect to MCP server if configured via environment variables
        let mcp_tools = if let (Ok(port), Ok(host)) = (
            std::env::var("PEEKOO_MCP_PORT"),
            std::env::var("PEEKOO_MCP_HOST"),
        ) {
            if let Ok(port) = port.parse::<u16>() {
                match connect_mcp_and_get_tools(&host, port).await {
                    Ok(tools) => {
                        tracing::info!("Connected to MCP server and got {} tools", tools.len());
                        Some(tools)
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect to MCP server: {}", e);
                        None
                    }
                }
            } else {
                tracing::warn!("Invalid MCP port: {}", port);
                None
            }
        } else {
            tracing::info!("No MCP server configured, running without tools");
            None
        };

        let _ = task_context.to_prompt();

        let (tx, rx) = oneshot::channel();
        self.session_update_tx
            .send((
                SessionNotification::new(
                    arguments.session_id.clone(),
                    SessionUpdate::AgentMessageChunk(ContentChunk::new(
                        format!(
                            "Processing task: {}\n\nConnecting to LLM...\n\n",
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

        // For now, echo the context back without tools since AgentService requires complex setup
        // TODO: Properly integrate with AgentService when time permits
        let mcp_info = if let (Ok(port), Ok(host)) = (
            std::env::var("PEEKOO_MCP_PORT"),
            std::env::var("PEEKOO_MCP_HOST"),
        ) {
            format!("{}:{}", host, port)
        } else {
            "N/A".to_string()
        };

        let tools_count = mcp_tools
            .as_ref()
            .map(|t| t.len().to_string())
            .unwrap_or_else(|| "0".to_string());

        let response_text = format!(
            "Task received: {}\n\nTitle: {}\nDescription: {}\n\nMCP Server: {}\nTools available: {}\n\nProcessing...",
            task_context.task_id,
            task_context.title,
            task_context
                .description
                .as_deref()
                .unwrap_or("No description"),
            mcp_info,
            tools_count
        );

        let (tx, rx) = oneshot::channel();
        self.session_update_tx
            .send((
                SessionNotification::new(
                    arguments.session_id.clone(),
                    SessionUpdate::AgentMessageChunk(ContentChunk::new(response_text.into())),
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

async fn connect_mcp_and_get_tools(host: &str, port: u16) -> anyhow::Result<Vec<String>> {
    use rmcp::service::ServiceExt;

    let mcp_url = format!("tcp://{}:{}", host, port);
    tracing::info!("🔗 [MCP] Connecting to server at {}", mcp_url);

    let stream = tokio::net::TcpStream::connect((host, port)).await?;
    tracing::info!("✅ [MCP] TCP connection established");

    let client: rmcp::service::RunningService<rmcp::service::RoleClient, ()> =
        ().serve(stream).await?;
    tracing::info!("✅ [MCP] Protocol handshake completed");

    let tools_result = client.list_tools(Default::default()).await?;
    let tool_names: Vec<String> = tools_result
        .tools
        .iter()
        .map(|t| t.name.to_string())
        .collect();

    tracing::info!("📋 [MCP] Available tools: {:?}", tool_names);

    Ok(tool_names)
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
        tracing::info!("🔗 [MCP] Server configured at tcp://{}:{}", host, port);
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
