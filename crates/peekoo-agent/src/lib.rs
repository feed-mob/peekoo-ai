//! Peekoo Agent — AI agent service for the peekoo-ai desktop app.
//!
//! This crate provides a peekoo-specific API for:
//! - Creating agent sessions with chosen LLM providers via ACP
//! - Sending prompts and streaming responses
//! - Forwarding ACP session MCP servers to compatible runtimes
//! - Switching providers/models at runtime

pub mod backend;
pub mod config;
pub mod service;
pub mod session_store;

// Temporary compatibility types (replacing pi re-exports)
// These will be removed once migration is complete

/// Error type for agent operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Agent error: {0}")]
    Message(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Configuration error: {0}")]
    Config(String),
}

/// Result type alias
pub type PiResult<T> = Result<T, Error>;

/// Re-export key types from backend for convenience
pub use backend::{
    AgentEvent, BackendConfig, ContentBlock, Message, MessageRole, ModelInfo, PromptResult,
    StopReason, TokenUsage,
};

/// Re-export session types
pub use session_store::SessionType;

/// Re-export process utilities from peekoo-utils for cross-platform command execution
pub use peekoo_utils::{command_available, resolve_command};
