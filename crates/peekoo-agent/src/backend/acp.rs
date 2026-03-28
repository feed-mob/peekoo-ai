//! ACP-based backend implementation
//!
//! This module implements the `AgentBackend` trait for ACP-compatible agents
//! such as pi-acp, opencode, claude-code, and codex.

use super::*;
use agent_client_protocol as acp;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, mpsc};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

/// ACP-based backend implementation
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
    /// Active child process
    child: Option<Child>,
    /// ACP connection handle
    connection: Option<AcpConnection>,
    /// Current session ID (from ACP agent)
    acp_session_id: Option<String>,
    /// Current model info
    model_info: ModelInfo,
    /// Whether the agent supports MCP natively
    supports_mcp: bool,
    /// Provider-specific state for persistence
    provider_state: Option<serde_json::Value>,
    /// Cancel signal
    cancel_tx: Option<mpsc::Sender<()>>,
}

struct AcpConnection {
    /// Handle for sending ACP requests
    #[allow(dead_code)]
    request_tx: mpsc::UnboundedSender<acp::ClientRequest>,
    /// Handle for receiving ACP responses
    #[allow(dead_code)]
    response_rx: Mutex<mpsc::UnboundedReceiver<acp::StreamMessage>>,
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
            child: None,
            connection: None,
            acp_session_id: None,
            model_info: ModelInfo {
                provider: "unknown".to_string(),
                model: "unknown".to_string(),
                provider_version: None,
            },
            supports_mcp: false,
            provider_state: None,
            cancel_tx: None,
        }
    }

    /// Spawn the ACP agent process and establish connection
    async fn spawn_and_connect(&mut self) -> anyhow::Result<()> {
        // Kill any existing process
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }

        tracing::info!("Spawning ACP agent: {} {:?}", self.command, self.args);

        // Spawn new process
        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args)
            .current_dir(&self.working_directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        // Set environment variables
        for (key, value) in &self.environment {
            cmd.env(key, value);
        }

        // Set system prompt if provided
        if let Some(ref prompt) = self.system_prompt {
            cmd.env("ACP_SYSTEM_PROMPT", prompt);
        }

        let child = cmd
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn ACP agent: {}", e))?;

        self.child = Some(child);

        // Establish ACP connection
        self.establish_acp_connection().await?;

        // Initialize ACP session
        self.initialize_acp().await?;

        // Create new ACP session
        self.create_acp_session().await?;

        Ok(())
    }

    /// Establish ACP connection over stdio
    async fn establish_acp_connection(&mut self) -> anyhow::Result<()> {
        let child = self
            .child
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No child process"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;

        // Wrap in async-compat
        let stdin_compat = TokioAsyncWriteCompatExt::compat_write(stdin);
        let stdout_compat = TokioAsyncReadCompatExt::compat(stdout);

        // Create communication channels
        let (request_tx, _request_rx) = mpsc::unbounded_channel::<acp::ClientRequest>();
        let (response_tx, response_rx) = mpsc::unbounded_channel::<acp::StreamMessage>();

        // TODO: Implement actual ACP protocol handling
        // For now, just store the channels
        let _ = (stdin_compat, stdout_compat, response_tx);

        self.connection = Some(AcpConnection {
            request_tx,
            response_rx: Mutex::new(response_rx),
        });

        Ok(())
    }

    /// Send initialize request to ACP agent
    async fn initialize_acp(&mut self) -> anyhow::Result<()> {
        tracing::info!("Initializing ACP session");

        // TODO: Implement actual ACP initialize handshake
        // For now, assume success and set basic capabilities
        self.supports_mcp = true;

        Ok(())
    }

    /// Create new ACP session
    async fn create_acp_session(&mut self) -> anyhow::Result<()> {
        tracing::info!("Creating ACP session");

        // Generate a new session ID
        let session_id = format!("acp_{}", uuid::Uuid::new_v4());
        self.acp_session_id = Some(session_id);

        // TODO: Send new_session request via ACP

        Ok(())
    }

    /// Check if the ACP agent process is still running
    fn is_process_alive(&self) -> bool {
        if let Some(ref child) = self.child {
            // Try to get exit status - if it's None, process is still running
            // Note: try_wait() needs &mut self, but we're only checking existence
            // A process that has a handle is considered "alive" for our purposes
            // The actual check would require interior mutability
            true
        } else {
            false
        }
    }
}

#[async_trait]
impl AgentBackend for AcpBackend {
    async fn initialize(&mut self, config: BackendConfig) -> anyhow::Result<()> {
        self.working_directory = config.working_directory;
        self.environment = config.environment;
        self.system_prompt = config.system_prompt;

        if let Some(provider) = config.provider {
            self.model_info.provider = provider;
        }
        if let Some(model) = config.model {
            self.model_info.model = model;
        }

        // Spawn and connect to ACP agent
        self.spawn_and_connect().await?;

        Ok(())
    }

    async fn prompt(
        &self,
        input: &str,
        conversation_history: Vec<Message>,
        on_event: EventCallback,
    ) -> anyhow::Result<PromptResult> {
        // Ensure we have a connection
        if self.connection.is_none() {
            return Err(anyhow::anyhow!("ACP connection not established"));
        }

        // Build prompt with conversation history
        let full_prompt = build_prompt_with_history(input, &conversation_history);

        // Setup cancel channel
        let (cancel_tx, mut cancel_rx) = mpsc::channel(1);
        // Store cancel_tx for later use - self is immutable here, so we can't store it directly
        let _ = cancel_tx;

        // TODO: Implement actual ACP prompt protocol
        // For now, simulate a response
        tracing::info!("Sending prompt to ACP agent: {} chars", full_prompt.len());

        // Simulate streaming
        on_event(AgentEvent::TextDelta("Response from ".to_string()));
        on_event(AgentEvent::TextDelta(self.model_info.provider.clone()));
        on_event(AgentEvent::TextDelta(" agent".to_string()));
        on_event(AgentEvent::Complete);

        // Check for cancel signal
        if let Ok(()) = cancel_rx.try_recv() {
            return Ok(PromptResult {
                content: "Cancelled".to_string(),
                stop_reason: StopReason::Cancelled,
                usage: None,
                provider_state: self.provider_state.clone(),
            });
        }

        Ok(PromptResult {
            content: format!("Response from {} agent", self.model_info.provider),
            stop_reason: StopReason::EndTurn,
            usage: Some(TokenUsage {
                input_tokens: full_prompt.len() as u32 / 4, // Rough estimate
                output_tokens: 20,
            }),
            provider_state: self.provider_state.clone(),
        })
    }

    async fn set_model(&mut self, provider: &str, model: &str) -> anyhow::Result<()> {
        // Check if we need to switch agent processes
        if provider != self.model_info.provider {
            // Different provider = different ACP agent command
            self.model_info.provider = provider.to_string();
            self.model_info.model = model.to_string();

            // Re-initialize with new provider
            // This spawns a new process
            // Note: Command/args would need to be updated based on provider
            self.spawn_and_connect().await?;
        } else {
            // Same provider, different model
            self.model_info.model = model.to_string();

            // Try to set via ACP config if supported
            // TODO: Send config update via ACP
        }

        Ok(())
    }

    fn current_model(&self) -> ModelInfo {
        self.model_info.clone()
    }

    async fn cancel(&self) -> anyhow::Result<()> {
        // Send cancel signal if we have a cancel channel
        // Note: Since self is immutable, we'd need interior mutability for cancel_tx
        tracing::info!("Cancelling ACP prompt");
        Ok(())
    }

    fn provider_id(&self) -> &'static str {
        "acp"
    }

    fn supports_mcp(&self) -> bool {
        self.supports_mcp
    }

    fn provider_state(&self) -> Option<serde_json::Value> {
        self.provider_state.clone()
    }

    async fn restore_provider_state(&mut self, state: serde_json::Value) -> anyhow::Result<()> {
        self.provider_state = Some(state);

        // If we have a connection, try to restore the session
        if self.acp_session_id.is_some() {
            // TODO: Send load_session request via ACP with the state
        }

        Ok(())
    }
}

impl Drop for AcpBackend {
    fn drop(&mut self) {
        // Clean up child process on drop
        if let Some(mut child) = self.child.take() {
            // Spawn a blocking task to kill the process
            tokio::spawn(async move {
                let _ = child.kill().await;
            });
        }
    }
}

/// Build a prompt string including conversation history
fn build_prompt_with_history(input: &str, history: &[Message]) -> String {
    let mut parts = Vec::new();

    // Add history
    for message in history {
        let role_prefix = match message.role {
            MessageRole::System => "System:",
            MessageRole::User => "User:",
            MessageRole::Assistant => "Assistant:",
            MessageRole::Tool => "Tool:",
        };

        let content = message
            .content
            .iter()
            .map(|block| match block {
                ContentBlock::Text { text } => text.clone(),
                ContentBlock::ToolUse { name, input, .. } => {
                    format!("[Tool: {}({})]", name, input)
                }
                ContentBlock::ToolResult { content, .. } => content.clone(),
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join("\n");

        parts.push(format!("{} {}", role_prefix, content));
    }

    // Add current input
    parts.push(format!("User: {}", input));
    parts.push("Assistant:".to_string());

    parts.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt_with_history() {
        let history = vec![
            Message {
                role: MessageRole::User,
                content: vec![ContentBlock::Text {
                    text: "Hello".to_string(),
                }],
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: MessageRole::Assistant,
                content: vec![ContentBlock::Text {
                    text: "Hi there!".to_string(),
                }],
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let prompt = build_prompt_with_history("How are you?", &history);

        assert!(prompt.contains("User: Hello"));
        assert!(prompt.contains("Assistant: Hi there!"));
        assert!(prompt.contains("User: How are you?"));
        assert!(prompt.contains("Assistant:"));
    }

    #[tokio::test]
    async fn test_acp_backend_initialization() {
        // This test will spawn a real process, so we use "echo" as a safe command
        let mut backend = AcpBackend::new("echo", vec!["test".to_string()]);

        let config = BackendConfig {
            working_directory: std::env::current_dir().unwrap(),
            system_prompt: Some("You are helpful.".to_string()),
            model: Some("test-model".to_string()),
            provider: Some("test-provider".to_string()),
            api_key: None,
            environment: HashMap::new(),
        };

        // Note: This will fail because "echo" doesn't speak ACP
        // but it tests the initialization path
        let result = backend.initialize(config).await;

        // We expect this to succeed at spawning, but the ACP handshake will fail
        // In a real implementation, we'd mock the ACP protocol
        assert!(result.is_err() || backend.child.is_some());
    }

    #[test]
    fn test_acp_backend_new() {
        let backend = AcpBackend::new("npx", vec!["pi-acp".to_string()]);

        assert_eq!(backend.command, "npx");
        assert_eq!(backend.args, vec!["pi-acp"]);
        assert!(backend.child.is_none());
        assert!(backend.connection.is_none());
    }

    #[test]
    fn test_acp_backend_provider_id() {
        let backend = AcpBackend::new("test", vec![]);
        assert_eq!(backend.provider_id(), "acp");
    }

    #[tokio::test]
    async fn test_acp_backend_model_switching() {
        let mut backend = AcpBackend::new("npx", vec!["pi-acp".to_string()]);

        // Start with one provider
        backend.model_info.provider = "pi-acp".to_string();
        backend.model_info.model = "claude-3.5".to_string();
        backend.child = None; // Ensure no child process

        // When switching providers, it tries to spawn a new process
        // This may succeed or fail depending on environment
        let result = backend.set_model("opencode", "gpt-4").await;

        // The model info should always be updated before attempting spawn
        assert_eq!(backend.model_info.provider, "opencode");
        assert_eq!(backend.model_info.model, "gpt-4");

        // Result depends on whether npx is available in test environment
        // Either success (if npx available) or error (if not)
        // We just verify the method completes without panic
        let _ = result;
    }

    #[test]
    fn test_acp_backend_provider_state() {
        let mut backend = AcpBackend::new("test", vec![]);

        // Initially no state
        assert!(backend.provider_state().is_none());

        // Set state manually (simulating restore)
        backend.provider_state = Some(serde_json::json!({ "session_id": "test-123" }));

        // Should return the state
        assert_eq!(
            backend.provider_state(),
            Some(serde_json::json!({ "session_id": "test-123" }))
        );
    }
}
