//! Configuration for the agent service.

use std::path::PathBuf;

use crate::skill::Skill;

/// Configuration for creating an [`AgentService`](super::service::AgentService).
///
/// Maps to pi's [`SessionOptions`](pi::sdk::SessionOptions) under the hood,
/// but provides a simpler, peekoo-specific surface.
pub struct AgentServiceConfig {
    /// LLM provider identifier (e.g. `"anthropic"`, `"openai"`, `"google"`).
    ///
    /// When `None`, the default provider from pi's config is used.
    pub provider: Option<String>,

    /// Model identifier within the provider (e.g. `"claude-sonnet-4-6"`, `"gpt-4o"`).
    ///
    /// When `None`, the default model is used.
    pub model: Option<String>,

    /// API key for the chosen provider.
    ///
    /// When `None`, pi resolves the key from environment variables or its
    /// auth storage (`~/.pi/auth.json`).
    pub api_key: Option<String>,

    /// System prompt prepended to every conversation.
    pub system_prompt: Option<String>,

    /// Working directory for file-system tools (read, write, bash, etc.).
    pub working_directory: PathBuf,

    /// Custom skills (domain-specific tools) to register alongside the
    /// built-in tools.
    pub skills: Vec<Box<dyn Skill>>,

    /// Maximum number of consecutive tool iterations before the agent stops.
    pub max_tool_iterations: usize,
}

impl Default for AgentServiceConfig {
    fn default() -> Self {
        Self {
            provider: None,
            model: None,
            api_key: None,
            system_prompt: None,
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            skills: Vec::new(),
            max_tool_iterations: 50,
        }
    }
}
