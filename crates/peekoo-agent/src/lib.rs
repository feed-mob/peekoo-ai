//! Peekoo Agent — AI agent service for the peekoo-ai desktop app.
//!
//! This crate wraps [`pi`] (the `pi_agent_rust` library) and provides a
//! simplified, peekoo-specific API for:
//!
//! - Creating agent sessions with chosen LLM providers
//! - Sending prompts and streaming responses
//! - Registering custom domain-specific tools ("skills")
//! - Switching providers/models at runtime

pub mod config;
pub mod service;
pub mod skill;

// Re-export key types for convenience.
pub use pi::sdk::{AgentEvent, AssistantMessage, ContentBlock, SubscriptionId};
pub use pi::error::{Error, Result as PiResult};
