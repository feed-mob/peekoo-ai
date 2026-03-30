//! Peekoo Agent — AI agent service for the peekoo-ai desktop app.
//!
//! This crate provides a peekoo-specific API for:
//! - Creating agent sessions with chosen LLM providers via ACP
//! - Sending prompts and streaming responses
//! - Registering custom domain-specific tools ("skills")
//! - Switching providers/models at runtime

pub mod backend;
pub mod config;
pub mod mcp_bridge;
pub mod process;
pub mod service;
pub mod session_store;

// TODO: Re-enable after migration
// pub mod mcp_client;
// pub mod plugin_tool;
// pub mod service;

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

/// Re-export process utilities for cross-platform command execution
pub use crate::process::{command_available, resolve_command};
