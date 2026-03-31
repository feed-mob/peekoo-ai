//! Agent backend trait and implementations
//!
//! This module defines the `AgentBackend` trait for abstracting over
//! different LLM agent implementations, and provides an ACP-based implementation.

use agent_client_protocol::McpServer;
use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;

/// Event callback for streaming agent responses
pub type EventCallback = Box<dyn Fn(AgentEvent) + Send + Sync>;

/// Trait for agent backend implementations
#[async_trait]
pub trait AgentBackend: Send + Sync {
    /// Initialize the backend with configuration
    async fn initialize(&mut self, config: BackendConfig) -> anyhow::Result<()>;

    /// Send a prompt and receive streaming response
    async fn prompt(
        &self,
        input: &str,
        conversation_history: Vec<Message>,
        on_event: EventCallback,
    ) -> anyhow::Result<PromptResult>;

    /// Switch to a different model/provider at runtime
    async fn set_model(&mut self, provider: &str, model: &str) -> anyhow::Result<()>;

    /// Get current model information
    fn current_model(&self) -> ModelInfo;

    /// Cancel an in-flight prompt
    async fn cancel(&self) -> anyhow::Result<()>;

    /// Get backend provider identifier
    fn provider_id(&self) -> &'static str;

    /// Check if backend supports MCP tools natively
    fn supports_mcp(&self) -> bool;

    /// Get opaque provider state for session persistence
    fn provider_state(&self) -> Option<serde_json::Value>;

    /// Restore provider state from persisted data
    async fn restore_provider_state(&mut self, state: serde_json::Value) -> anyhow::Result<()>;
}

/// Backend configuration
#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub working_directory: std::path::PathBuf,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub environment: HashMap<String, String>,
    pub mcp_servers: Vec<McpServer>,
}

/// Conversation message (provider-agnostic)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: Vec<ContentBlock>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
    Image {
        source: ImageSource,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    Base64 { media_type: String, data: String },
    Url { url: String },
}

/// Result of a prompt
#[derive(Debug, Clone)]
pub struct PromptResult {
    pub content: String,
    pub stop_reason: StopReason,
    pub usage: Option<TokenUsage>,
    pub provider_state: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    Error,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub provider: String,
    pub model: String,
    pub provider_version: Option<String>,
}

/// Events streamed during prompt execution
#[derive(Debug, Clone, Serialize)]
pub enum AgentEvent {
    /// Text delta (incremental content)
    TextDelta(String),
    /// Thinking/reasoning content
    ThinkingDelta(String),
    /// Tool call requested
    ToolCallStart { id: String, name: String },
    /// Tool call arguments (streaming JSON)
    ToolCallDelta { id: String, arguments: String },
    /// Tool call completed
    ToolCallComplete { id: String },
    /// Tool result being sent to agent
    ToolResult { id: String, content: String },
    /// Response complete
    Complete,
    /// Error occurred
    Error(String),
}

/// ACP-based backend implementation
pub mod acp;

// Re-export AcpBackend for convenience
pub use acp::AcpBackend;

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock backend for testing the trait interface
    struct MockBackend {
        provider: String,
        model: String,
        initialized: bool,
        provider_state: Option<serde_json::Value>,
    }

    #[async_trait]
    impl AgentBackend for MockBackend {
        async fn initialize(&mut self, config: BackendConfig) -> anyhow::Result<()> {
            if let Some(provider) = config.provider {
                self.provider = provider;
            }
            if let Some(model) = config.model {
                self.model = model;
            }
            self.initialized = true;
            Ok(())
        }

        async fn prompt(
            &self,
            _input: &str,
            _conversation_history: Vec<Message>,
            on_event: EventCallback,
        ) -> anyhow::Result<PromptResult> {
            // Simulate streaming response
            on_event(AgentEvent::TextDelta("Hello".to_string()));
            on_event(AgentEvent::TextDelta(", ".to_string()));
            on_event(AgentEvent::TextDelta("world!".to_string()));
            on_event(AgentEvent::Complete);

            Ok(PromptResult {
                content: "Hello, world!".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: Some(TokenUsage {
                    input_tokens: 10,
                    output_tokens: 3,
                }),
                provider_state: None,
            })
        }

        async fn set_model(&mut self, provider: &str, model: &str) -> anyhow::Result<()> {
            self.provider = provider.to_string();
            self.model = model.to_string();
            Ok(())
        }

        fn current_model(&self) -> ModelInfo {
            ModelInfo {
                provider: self.provider.clone(),
                model: self.model.clone(),
                provider_version: None,
            }
        }

        async fn cancel(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn provider_id(&self) -> &'static str {
            "mock"
        }

        fn supports_mcp(&self) -> bool {
            true
        }

        fn provider_state(&self) -> Option<serde_json::Value> {
            self.provider_state.clone()
        }

        async fn restore_provider_state(&mut self, state: serde_json::Value) -> anyhow::Result<()> {
            self.provider_state = Some(state);
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_backend_initialization() {
        let mut backend = MockBackend {
            provider: "test".to_string(),
            model: "test-model".to_string(),
            initialized: false,
            provider_state: None,
        };

        let config = BackendConfig {
            working_directory: std::env::current_dir().unwrap(),
            system_prompt: Some("You are a test assistant.".to_string()),
            model: Some("new-model".to_string()),
            provider: Some("new-provider".to_string()),
            api_key: None,
            environment: HashMap::new(),
            mcp_servers: Vec::new(),
        };

        backend.initialize(config).await.unwrap();

        assert!(backend.initialized);
        assert_eq!(backend.provider, "new-provider");
        assert_eq!(backend.model, "new-model");
    }

    #[tokio::test]
    async fn test_backend_prompt_streaming() {
        let backend = MockBackend {
            provider: "test".to_string(),
            model: "test-model".to_string(),
            initialized: true,
            provider_state: None,
        };

        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events.clone();

        let result = backend
            .prompt(
                "Test input",
                vec![],
                Box::new(move |event| {
                    events_clone.lock().unwrap().push(event);
                }),
            )
            .await
            .unwrap();

        assert_eq!(result.content, "Hello, world!");
        assert_eq!(result.stop_reason, StopReason::EndTurn);

        let captured_events = events.lock().unwrap();
        assert_eq!(captured_events.len(), 4); // 3 text deltas + complete
    }

    #[tokio::test]
    async fn test_backend_model_switching() {
        let mut backend = MockBackend {
            provider: "old-provider".to_string(),
            model: "old-model".to_string(),
            initialized: true,
            provider_state: None,
        };

        backend
            .set_model("new-provider", "new-model")
            .await
            .unwrap();

        let model_info = backend.current_model();
        assert_eq!(model_info.provider, "new-provider");
        assert_eq!(model_info.model, "new-model");
    }

    #[tokio::test]
    async fn test_backend_provider_state() {
        let mut backend = MockBackend {
            provider: "test".to_string(),
            model: "test-model".to_string(),
            initialized: true,
            provider_state: None,
        };

        // Initially no state
        assert!(backend.provider_state().is_none());

        // Restore state
        let state = serde_json::json!({ "session_id": "abc123", "context": "test" });
        backend.restore_provider_state(state.clone()).await.unwrap();

        // State should be restored
        assert_eq!(backend.provider_state(), Some(state));
    }

    #[test]
    fn test_message_serialization() {
        let message = Message {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
            tool_calls: None,
            tool_call_id: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized.role, MessageRole::User));
        assert_eq!(deserialized.content.len(), 1);
    }

    #[test]
    fn test_content_block_tool_use() {
        let block = ContentBlock::ToolUse {
            id: "call_123".to_string(),
            name: "read_file".to_string(),
            input: serde_json::json!({ "path": "/tmp/test.txt" }),
        };

        let json = serde_json::to_string(&block).unwrap();
        let deserialized: ContentBlock = serde_json::from_str(&json).unwrap();

        match deserialized {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_123");
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "/tmp/test.txt");
            }
            _ => panic!("Expected ToolUse variant"),
        }
    }
}
