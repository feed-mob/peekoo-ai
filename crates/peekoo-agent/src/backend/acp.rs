//! ACP-based backend implementation using the agent-client-protocol crate
//!
//! This module implements the `AgentBackend` trait for ACP-compatible agents
//! using the official agent-client-protocol crate.
//!
//! Implementation note: The ACP crate uses non-Send futures (#[async_trait(?Send)]).
//! To satisfy the AgentBackend trait's Send requirement, we spawn a dedicated thread
//! with a single-threaded tokio runtime that runs within a LocalSet.

use super::*;
use agent_client_protocol as acp;
use peekoo_utils::process::resolve_command;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use tokio::process::Command;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

/// ACP-based backend implementation that wraps non-Send ACP operations
pub struct AcpBackend {
    /// Command to spawn the ACP agent
    command: String,
    args: Vec<String>,
    /// Working directory for the agent
    working_directory: std::path::PathBuf,
    /// Environment variables
    environment: HashMap<String, String>,
    /// System prompt
    system_prompt: Option<String>,
    /// Command sender for the ACP task
    cmd_tx: Option<mpsc::Sender<AcpCommand>>,
    /// Current model info
    model_info: ModelInfo,
    /// Whether the agent supports MCP natively
    supports_mcp: bool,
    /// Provider-specific state for persistence
    provider_state: Option<serde_json::Value>,
    /// Discovered auth methods from ACP initialize
    auth_methods: Vec<acp::AuthMethod>,
    /// Discovered models/config from ACP session
    discovered_models: Vec<DiscoveredModel>,
    /// Current model from ACP
    current_model_id: Option<String>,
    /// MCP servers to attach when creating ACP sessions.
    mcp_servers: Vec<acp::McpServer>,
    /// Whether the runtime has explicitly reported that authentication is required.
    auth_required: AtomicBool,
    /// Event callback for streaming
    event_callback: Arc<Mutex<Option<EventCallback>>>,
}

/// Discovered model from ACP session
#[derive(Debug, Clone)]
pub struct DiscoveredModel {
    pub model_id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Commands sent to the ACP task
#[derive(Debug)]
enum AcpCommand {
    Initialize {
        resp: oneshot::Sender<anyhow::Result<InitializeResult>>,
    },
    CreateSession {
        working_dir: std::path::PathBuf,
        resp: oneshot::Sender<anyhow::Result<SessionResult>>,
    },
    SetModel {
        model: String,
        resp: oneshot::Sender<anyhow::Result<()>>,
    },
    Prompt {
        input: String,
        resp: oneshot::Sender<anyhow::Result<PromptResult>>,
    },
    Cancel {
        resp: oneshot::Sender<anyhow::Result<()>>,
    },
    Authenticate {
        method_id: String,
        resp: oneshot::Sender<anyhow::Result<()>>,
    },
    Shutdown,
}

/// Internal result types
#[derive(Debug)]
struct InitializeResult {
    supports_mcp: bool,
    auth_methods: Vec<acp::AuthMethod>,
}

#[derive(Debug)]
struct SessionResult {
    models: Vec<DiscoveredModel>,
    current_model: Option<String>,
}

fn extract_models_from_session_response(
    response: &acp::NewSessionResponse,
) -> (Vec<DiscoveredModel>, Option<String>) {
    if let Some(models) = &response.models {
        let discovered_models = models
            .available_models
            .iter()
            .map(|model| DiscoveredModel {
                model_id: model.model_id.to_string(),
                name: model.name.clone(),
                description: model.description.clone(),
            })
            .collect();

        return (discovered_models, Some(models.current_model_id.to_string()));
    }

    let mut models = Vec::new();
    let mut current_model = None;

    if let Some(config_options) = &response.config_options {
        for option in config_options {
            if let Some(acp::SessionConfigOptionCategory::Model) = option.category {
                if let acp::SessionConfigKind::Select(select) = &option.kind {
                    match &select.options {
                        acp::SessionConfigSelectOptions::Ungrouped(opts) => {
                            for opt in opts {
                                models.push(DiscoveredModel {
                                    model_id: opt.value.to_string(),
                                    name: opt.name.to_string(),
                                    description: opt.description.clone(),
                                });
                            }
                        }
                        acp::SessionConfigSelectOptions::Grouped(groups) => {
                            for group in groups {
                                for opt in &group.options {
                                    models.push(DiscoveredModel {
                                        model_id: opt.value.to_string(),
                                        name: opt.name.to_string(),
                                        description: opt.description.clone(),
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                    current_model = Some(select.current_value.to_string());
                }
            }
        }
    }

    (models, current_model)
}

async fn collect_prompt_content(mut content_rx: mpsc::Receiver<String>) -> String {
    let mut collected_content = String::new();
    while let Some(chunk) = content_rx.recv().await {
        collected_content.push_str(&chunk);
    }
    collected_content
}

/// Client handler for ACP - handles requests FROM the agent
#[derive(Clone)]
struct AcpClientHandler {
    event_callback: Arc<Mutex<Option<EventCallback>>>,
    content_sender: Arc<Mutex<Option<mpsc::Sender<String>>>>,
}

impl AcpClientHandler {
    fn new(event_callback: Arc<Mutex<Option<EventCallback>>>) -> Self {
        Self {
            event_callback,
            content_sender: Arc::new(Mutex::new(None)),
        }
    }

    async fn set_content_sender(&self, sender: mpsc::Sender<String>) {
        let mut guard = self.content_sender.lock().await;
        *guard = Some(sender);
    }

    async fn clear_content_sender(&self) {
        let mut guard = self.content_sender.lock().await;
        *guard = None;
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for AcpClientHandler {
    async fn request_permission(
        &self,
        _args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        // Auto-cancel for safety until we have proper UI integration
        Ok(acp::RequestPermissionResponse::new(
            acp::RequestPermissionOutcome::Cancelled,
        ))
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        use acp::SessionUpdate;
        match args.update {
            SessionUpdate::AgentMessageChunk(chunk) => {
                if let acp::ContentBlock::Text(text_content) = &chunk.content {
                    tracing::info!(
                        session_id = %args.session_id,
                        chunk_len = text_content.text.chars().count(),
                        "ACP text chunk received"
                    );
                    // Send to content channel for collection
                    let content_guard = self.content_sender.lock().await;
                    if let Some(ref sender) = *content_guard {
                        let _ = sender.try_send(text_content.text.clone());
                    }

                    // Emit to event callback for streaming UI
                    let event_guard = self.event_callback.lock().await;
                    if let Some(ref callback) = *event_guard {
                        callback(AgentEvent::TextDelta(text_content.text.clone()));
                    }
                }
            }
            SessionUpdate::ToolCall(tool_call) => {
                tracing::info!(
                    session_id = %args.session_id,
                    tool_call_id = %tool_call.tool_call_id,
                    title = %tool_call.title,
                    "ACP tool call notification received"
                );
                let guard = self.event_callback.lock().await;
                if let Some(ref callback) = *guard {
                    callback(AgentEvent::ToolCallStart {
                        id: tool_call.tool_call_id.to_string(),
                        name: tool_call.title.clone(),
                    });
                }
            }
            other => {
                tracing::info!(
                    session_id = %args.session_id,
                    update = ?other,
                    "ACP session update received"
                );
            }
        }
        Ok(())
    }
}

impl AcpBackend {
    /// Create a new ACP backend with the given command and arguments
    pub fn new(command: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            command: command.into(),
            args,
            working_directory: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from(".")),
            environment: HashMap::new(),
            system_prompt: None,
            cmd_tx: None,
            model_info: ModelInfo {
                provider: "unknown".to_string(),
                model: "unknown".to_string(),
                provider_version: None,
            },
            supports_mcp: false,
            provider_state: None,
            auth_methods: Vec::new(),
            discovered_models: Vec::new(),
            current_model_id: None,
            mcp_servers: Vec::new(),
            auth_required: AtomicBool::new(false),
            event_callback: Arc::new(Mutex::new(None)),
        }
    }

    /// Get discovered auth methods from the last initialize
    pub fn auth_methods(&self) -> &[acp::AuthMethod] {
        &self.auth_methods
    }

    /// Get discovered models from the last session
    pub fn discovered_models(&self) -> &[DiscoveredModel] {
        &self.discovered_models
    }

    /// Get current model ID from ACP
    pub fn current_model_id(&self) -> Option<&str> {
        self.current_model_id.as_deref()
    }

    /// Check if auth is required based on last operation
    pub fn is_auth_required(&self) -> bool {
        self.auth_required.load(Ordering::Relaxed)
    }

    /// Authenticate with the ACP runtime using the specified auth method
    pub async fn authenticate(&self, method_id: &str) -> anyhow::Result<()> {
        let cmd_tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ACP not connected"))?;

        let (tx, rx) = oneshot::channel();
        cmd_tx
            .send(AcpCommand::Authenticate {
                method_id: method_id.to_string(),
                resp: tx,
            })
            .await?;

        match rx.await? {
            Ok(()) => {
                self.auth_required.store(false, Ordering::Relaxed);
            }
            Err(error) => {
                self.auth_required
                    .store(is_auth_required_error(&error), Ordering::Relaxed);
                return Err(error);
            }
        }
        Ok(())
    }

    /// Gracefully shut down the ACP worker.
    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        if let Some(tx) = self.cmd_tx.take() {
            let _ = tx.send(AcpCommand::Shutdown).await;
        }
        Ok(())
    }

    /// Spawn the ACP agent process and start the local task
    async fn spawn_and_connect(&mut self) -> anyhow::Result<()> {
        // Shutdown any existing task
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.send(AcpCommand::Shutdown).await;
        }

        tracing::info!("Spawning ACP agent: {} {:?}", self.command, self.args);

        // Resolve command with Windows extensions if needed
        let resolved_command = resolve_command(&self.command);
        tracing::debug!(
            "Resolved command: {} -> {:?}",
            self.command,
            resolved_command
        );

        // Spawn new process
        let mut cmd = Command::new(&resolved_command);
        cmd.args(&self.args)
            .current_dir(&self.working_directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        // Set environment variables
        for (key, value) in &self.environment {
            cmd.env(key, value);
        }

        if let Some(ref prompt) = self.system_prompt {
            cmd.env("ACP_SYSTEM_PROMPT", prompt);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn ACP agent: {}", e))?;

        // Take stdin/stdout and convert to futures-compatible streams
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?
            .compat_write();
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?
            .compat();

        // Create command channel
        let (cmd_tx, cmd_rx) = mpsc::channel::<AcpCommand>(32);
        self.cmd_tx = Some(cmd_tx.clone());

        // Clone fields for the thread
        let event_callback = self.event_callback.clone();
        let mcp_servers = self.mcp_servers.clone();

        // Spawn a dedicated thread for ACP operations with its own single-threaded runtime
        // This is necessary because ACP uses !Send futures which require LocalSet
        thread::spawn(move || {
            // Create a single-threaded runtime for this thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create single-threaded runtime");

            rt.block_on(async move {
                let local_set = tokio::task::LocalSet::new();

                local_set.run_until(async move {
                    use agent_client_protocol::Agent;
                    // Create client handler
                    let client_handler = AcpClientHandler::new(event_callback.clone());

                    // Create ACP connection
                    let (connection, io_task) = acp::ClientSideConnection::new(
                        client_handler.clone(),
                        stdin,
                        stdout,
                        |fut| {
                            tokio::task::spawn_local(async move {
                                let _ = fut.await;
                            });
                        },
                    );

                    // Spawn the IO task
                    let io_handle = tokio::task::spawn_local(io_task);
                    let mut should_shutdown = false;

                    // State maintained within the local task
                    let mut acp_session_id: Option<acp::SessionId> = None;
                    let mut current_model_id: Option<String> = None;

                    // Command processing loop
                    let mut cmd_rx = cmd_rx;
                    while let Some(cmd) = cmd_rx.recv().await {
                        match cmd {
                            AcpCommand::Initialize { resp } => {
                                let result = async {
                                    let client_info = acp::Implementation::new(
                                        "peekoo",
                                        env!("CARGO_PKG_VERSION"),
                                    );
                                    let capabilities = acp::ClientCapabilities::new();
                                    let request = acp::InitializeRequest::new(acp::ProtocolVersion::V1)
                                        .client_capabilities(capabilities)
                                        .client_info(client_info);

                                    let response = connection.initialize(request).await?;

                                    let mcp_caps = &response.agent_capabilities.mcp_capabilities;
                                    let supports_mcp = mcp_caps.http || mcp_caps.sse;

                                    tracing::info!(
                                        auth_methods = response.auth_methods.len(),
                                        supports_mcp,
                                        agent_info = ?response.agent_info,
                                        "ACP initialized"
                                    );

                                    Ok(InitializeResult {
                                        supports_mcp,
                                        auth_methods: response.auth_methods,
                                    })
                                }
                                .await;
                                let _ = resp.send(result);
                            }

                            AcpCommand::CreateSession { working_dir, resp } => {
                                let result = async {
                                    tracing::info!(
                                        cwd = %working_dir.display(),
                                        mcp_servers = mcp_servers.len(),
                                        "ACP creating session"
                                    );
                                    let request = acp::NewSessionRequest::new(working_dir)
                                        .mcp_servers(mcp_servers.clone());
                                    let response = connection.new_session(request).await?;

                                    let session_id = response.session_id.clone();
                                    acp_session_id = Some(session_id.clone());

                                    let (models, current_model) =
                                        extract_models_from_session_response(&response);

                                    // Do not treat ACP session modes as model choices.
                                    // Some runtimes (like OpenCode) expose workflow modes such as
                                    // `build` or `plan` here, which are not actual model IDs.

                                    tracing::info!(
                                        session_id = %session_id,
                                        model_count = models.len(),
                                        current_model = ?current_model,
                                        has_config_options = response.config_options.as_ref().map(|v| v.len()).unwrap_or(0),
                                        "ACP session created"
                                    );

                                    Ok(SessionResult { models, current_model })
                                }
                                .await;
                                let _ = resp.send(result);
                            }

                            AcpCommand::SetModel { model, resp } => {
                                let result = async {
                                    if let Some(ref session_id) = acp_session_id {
                                        let request = acp::SetSessionModelRequest::new(
                                            session_id.clone(),
                                            acp::ModelId::new(model.as_str()),
                                        );
                                        connection.set_session_model(request).await?;
                                        current_model_id = Some(model);
                                        Ok(())
                                    } else {
                                        Err(anyhow::anyhow!("No active ACP session"))
                                    }
                                }
                                .await;
                                let _ = resp.send(result);
                            }

                            AcpCommand::Prompt { input, resp } => {
                                let result = async {
                                    let session_id = acp_session_id.clone()
                                        .ok_or_else(|| anyhow::anyhow!("No active ACP session"))?;
                                    tracing::info!(
                                        session_id = %session_id,
                                        input_len = input.chars().count(),
                                        "ACP prompt starting"
                                    );

                                    // Set up content collection channel
                                    let (content_tx, content_rx) = mpsc::channel::<String>(100);
                                    client_handler.set_content_sender(content_tx.clone()).await;

                                    let content = acp::TextContent::new(input);
                                    let request =
                                        acp::PromptRequest::new(session_id, vec![acp::ContentBlock::Text(content)]);

                                    // Send the prompt - content streams via notifications
                                    let response = connection.prompt(request).await?;
                                    tracing::info!(
                                        stop_reason = ?response.stop_reason,
                                        "ACP prompt response received"
                                    );

                                    client_handler.clear_content_sender().await;
                                    drop(content_tx);
                                    let collected_content = collect_prompt_content(content_rx).await;
                                    tracing::info!(
                                        content_len = collected_content.chars().count(),
                                        "ACP prompt content collected"
                                    );

                                    // Emit completion event
                                    let guard = event_callback.lock().await;
                                    if let Some(ref callback) = *guard {
                                        callback(AgentEvent::Complete);
                                    }

                                    let stop_reason = match response.stop_reason {
                                        acp::StopReason::EndTurn => StopReason::EndTurn,
                                        acp::StopReason::MaxTokens => StopReason::MaxTokens,
                                        _ => StopReason::EndTurn,
                                    };

                                    Ok(PromptResult {
                                        content: collected_content,
                                        stop_reason,
                                        usage: None,
                                        provider_state: None,
                                    })
                                }
                                .await;
                                let _ = resp.send(result);
                            }

                            AcpCommand::Cancel { resp } => {
                                let result = async {
                                    if let Some(ref session_id) = acp_session_id {
                                        let notification = acp::CancelNotification::new(session_id.clone());
                                        connection.cancel(notification).await?;
                                    }
                                    Ok(())
                                }
                                .await;
                                let _ = resp.send(result);
                            }

                            AcpCommand::Authenticate { method_id, resp } => {
                                let result = async {
                                    let method_id = acp::AuthMethodId::new(method_id);
                                    let request = acp::AuthenticateRequest::new(method_id);
                                    let _response = connection.authenticate(request).await?;
                                    tracing::info!("ACP authentication successful");
                                    Ok(())
                                }
                                .await;
                                let _ = resp.send(result);
                            }

                            AcpCommand::Shutdown => {
                                should_shutdown = true;
                                break;
                            }
                        }
                    }

                    if should_shutdown {
                        drop(connection);
                        let _ = tokio::time::timeout(
                            std::time::Duration::from_secs(2),
                            child.wait(),
                        )
                        .await;
                        if !child.id().is_none() {
                            let _ = child.kill().await;
                        }
                        io_handle.abort();
                    }

                    tracing::info!("ACP task shutdown complete");
                }).await;
            });
        });

        tracing::info!("ACP agent spawned and connected via local task");
        Ok(())
    }
}

pub fn is_auth_required_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<acp::Error>()
        .is_some_and(|acp_error| acp_error.code == acp::ErrorCode::AuthRequired)
}

fn should_apply_configured_model<'a>(
    configured_model: Option<&'a str>,
    current_model: Option<&str>,
) -> Option<&'a str> {
    let configured_model = configured_model?.trim();
    if configured_model.is_empty() || current_model == Some(configured_model) {
        return None;
    }
    Some(configured_model)
}

#[async_trait]
impl AgentBackend for AcpBackend {
    async fn initialize(&mut self, config: BackendConfig) -> anyhow::Result<()> {
        tracing::info!(
            provider = ?config.provider,
            model = ?config.model,
            env_keys = ?config.environment.keys().collect::<Vec<_>>(),
            cwd = %config.working_directory.display(),
            "ACP backend initialize requested"
        );
        self.working_directory = config.working_directory.clone();
        self.environment = config.environment.clone();
        self.system_prompt = config.system_prompt.clone();
        self.mcp_servers = config.mcp_servers.clone();
        self.auth_required.store(false, Ordering::Relaxed);
        let desired_model = config.model.clone();

        if let Some(provider) = config.provider.clone() {
            self.model_info.provider = provider;
        }
        if let Some(model) = config.model.clone() {
            self.model_info.model = model;
        }

        // Spawn the ACP process and start the local task
        self.spawn_and_connect().await?;

        // Send initialize command
        let (tx, rx) = oneshot::channel();
        let cmd_tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ACP not connected"))?;
        cmd_tx.send(AcpCommand::Initialize { resp: tx }).await?;

        let init_result = rx.await??;
        self.supports_mcp = init_result.supports_mcp;
        self.auth_methods = init_result.auth_methods;

        // Create session
        let (tx, rx) = oneshot::channel();
        cmd_tx
            .send(AcpCommand::CreateSession {
                working_dir: self.working_directory.clone(),
                resp: tx,
            })
            .await?;

        let session_result = match rx.await? {
            Ok(result) => {
                self.auth_required.store(false, Ordering::Relaxed);
                result
            }
            Err(error) => {
                self.auth_required
                    .store(is_auth_required_error(&error), Ordering::Relaxed);
                return Err(error);
            }
        };
        self.discovered_models = session_result.models;
        self.current_model_id = session_result.current_model;

        if let Some(model) = should_apply_configured_model(
            desired_model.as_deref(),
            self.current_model_id.as_deref(),
        ) {
            let (tx, rx) = oneshot::channel();
            cmd_tx
                .send(AcpCommand::SetModel {
                    model: model.to_string(),
                    resp: tx,
                })
                .await?;

            match rx.await? {
                Ok(()) => {
                    self.auth_required.store(false, Ordering::Relaxed);
                    self.current_model_id = Some(model.to_string());
                }
                Err(error) => {
                    self.auth_required
                        .store(is_auth_required_error(&error), Ordering::Relaxed);
                    return Err(error);
                }
            }
        }

        if let Some(ref current) = self.current_model_id {
            self.model_info.model = current.clone();
        } else if let Some(first) = self.discovered_models.first() {
            self.model_info.model = first.model_id.clone();
        }

        tracing::info!(
            provider = %self.model_info.provider,
            model = %self.model_info.model,
            discovered_models = self.discovered_models.len(),
            auth_required = self.is_auth_required(),
            "ACP backend initialize complete"
        );

        Ok(())
    }

    async fn prompt(
        &self,
        input: &str,
        _conversation_history: Vec<Message>,
        on_event: EventCallback,
    ) -> anyhow::Result<PromptResult> {
        let cmd_tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ACP not connected"))?;
        tracing::info!(
            provider = %self.model_info.provider,
            model = %self.model_info.model,
            input_len = input.chars().count(),
            "ACP backend prompt requested"
        );

        // Set up event callback
        {
            let mut guard = self.event_callback.lock().await;
            *guard = Some(on_event);
        }

        let (tx, rx) = oneshot::channel();
        cmd_tx
            .send(AcpCommand::Prompt {
                input: input.to_string(),
                resp: tx,
            })
            .await?;

        let result = match rx.await? {
            Ok(result) => {
                self.auth_required.store(false, Ordering::Relaxed);
                tracing::info!(
                    output_len = result.content.chars().count(),
                    stop_reason = ?result.stop_reason,
                    "ACP backend prompt completed"
                );
                result
            }
            Err(error) => {
                self.auth_required
                    .store(is_auth_required_error(&error), Ordering::Relaxed);
                tracing::error!(
                    error = %error,
                    is_auth_required = is_auth_required_error(&error),
                    "ACP backend prompt failed"
                );
                return Err(error);
            }
        };
        {
            let mut guard = self.event_callback.lock().await;
            *guard = None;
        }
        Ok(result)
    }

    async fn set_model(&mut self, provider: &str, model: &str) -> anyhow::Result<()> {
        if provider != self.model_info.provider {
            // Provider change requires re-initialization
            self.model_info.provider = provider.to_string();
            self.model_info.model = model.to_string();
            // Would need to re-spawn with new provider config
            // For now, just update the model ID
        }

        let cmd_tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ACP not connected"))?;

        let (tx, rx) = oneshot::channel();
        cmd_tx
            .send(AcpCommand::SetModel {
                model: model.to_string(),
                resp: tx,
            })
            .await?;

        match rx.await? {
            Ok(()) => {
                self.auth_required.store(false, Ordering::Relaxed);
            }
            Err(error) => {
                self.auth_required
                    .store(is_auth_required_error(&error), Ordering::Relaxed);
                return Err(error);
            }
        }
        self.current_model_id = Some(model.to_string());
        self.model_info.model = model.to_string();

        Ok(())
    }

    fn current_model(&self) -> ModelInfo {
        self.model_info.clone()
    }

    async fn cancel(&self) -> anyhow::Result<()> {
        let cmd_tx = self
            .cmd_tx
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("ACP not connected"))?;

        let (tx, rx) = oneshot::channel();
        cmd_tx.send(AcpCommand::Cancel { resp: tx }).await?;

        rx.await??;
        Ok(())
    }

    fn provider_id(&self) -> &'static str {
        "acp"
    }

    fn supports_mcp(&self) -> bool {
        self.supports_mcp
    }

    fn provider_state(&self) -> Option<serde_json::Value> {
        let state = serde_json::json!({
            "modelId": self.current_model_id,
            "discoveredModels": self.discovered_models.iter().map(|m| {
                serde_json::json!({
                    "modelId": m.model_id,
                    "name": m.name,
                    "description": m.description
                })
            }).collect::<Vec<_>>()
        });
        Some(state)
    }

    async fn restore_provider_state(&mut self, state: serde_json::Value) -> anyhow::Result<()> {
        self.provider_state = Some(state.clone());
        if let Some(model_id) = state.get("modelId").and_then(|m| m.as_str()) {
            self.current_model_id = Some(model_id.to_string());
            self.model_info.model = model_id.to_string();
        }
        Ok(())
    }
}

impl Drop for AcpBackend {
    fn drop(&mut self) {
        if let Some(ref tx) = self.cmd_tx {
            let _ = tx.try_send(AcpCommand::Shutdown);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_required_is_explicit_state_not_auth_method_presence() {
        let mut backend = AcpBackend::new("test-agent", Vec::new());
        backend.auth_methods = vec![acp::AuthMethod::Agent(acp::AuthMethodAgent::new(
            "browser",
            "Browser Login",
        ))];

        assert!(!backend.is_auth_required());

        backend.auth_required.store(true, Ordering::Relaxed);
        assert!(backend.is_auth_required());
    }

    #[test]
    fn detects_typed_acp_auth_required_errors() {
        let error = anyhow::Error::new(acp::Error::auth_required());

        assert!(is_auth_required_error(&error));
    }

    #[test]
    fn ignores_non_auth_acp_errors() {
        let error = anyhow::Error::new(acp::Error::internal_error());

        assert!(!is_auth_required_error(&error));
    }

    #[test]
    fn applies_configured_model_when_session_model_differs() {
        assert_eq!(
            should_apply_configured_model(
                Some("opencode/mimo-v2-omni-free"),
                Some("opencode/big-pickle")
            ),
            Some("opencode/mimo-v2-omni-free")
        );
    }

    #[test]
    fn skips_configured_model_when_session_already_matches() {
        assert_eq!(
            should_apply_configured_model(
                Some("opencode/mimo-v2-omni-free"),
                Some("opencode/mimo-v2-omni-free")
            ),
            None
        );
    }

    #[test]
    fn skips_empty_or_missing_configured_model() {
        assert_eq!(
            should_apply_configured_model(None, Some("opencode/big-pickle")),
            None
        );
        assert_eq!(
            should_apply_configured_model(Some(""), Some("opencode/big-pickle")),
            None
        );
        assert_eq!(
            should_apply_configured_model(Some("   "), Some("opencode/big-pickle")),
            None
        );
    }

    #[test]
    fn extracts_models_from_unstable_session_model_state() {
        let response =
            acp::NewSessionResponse::new("session-1").models(acp::SessionModelState::new(
                "gpt-5.4",
                vec![
                    acp::ModelInfo::new("gpt-5.4", "GPT-5.4").description("Latest frontier model"),
                    acp::ModelInfo::new("gpt-5.3", "GPT-5.3"),
                ],
            ));

        let (models, current_model) = extract_models_from_session_response(&response);

        assert_eq!(current_model.as_deref(), Some("gpt-5.4"));
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model_id, "gpt-5.4");
        assert_eq!(models[0].name, "GPT-5.4");
        assert_eq!(
            models[0].description.as_deref(),
            Some("Latest frontier model")
        );
    }

    #[tokio::test]
    async fn collects_prompt_content_after_sender_is_dropped() {
        let (tx, rx) = mpsc::channel::<String>(4);
        tx.send("Hello".to_string()).await.unwrap();
        tx.send(", world".to_string()).await.unwrap();
        drop(tx);

        let content = collect_prompt_content(rx).await;

        assert_eq!(content, "Hello, world");
    }
}
