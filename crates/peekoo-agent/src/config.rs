//! Configuration for the agent service.

use std::path::PathBuf;

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

    /// Path to a directory containing OpenClaw-style persona files.
    ///
    /// Supported files (all optional):
    /// - `IDENTITY.md` — Name, role, background context
    /// - `SOUL.md` — Core personality, values, behavioral guidelines
    /// - `MEMORY.md` — Persistent facts, user preferences, project context
    ///
    /// These are composed into the system prompt before any `system_prompt`
    /// or `agent_skills` content.
    pub persona_dir: Option<PathBuf>,

    /// List of paths to markdown files containing AgentSkills (from agentskills.io).
    /// These are loaded, parsed, and injected into the system prompt as instructions.
    pub agent_skills: Vec<PathBuf>,

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
            persona_dir: None,
            agent_skills: Vec::new(),
            max_tool_iterations: 50,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_provider() {
        let config = AgentServiceConfig::default();
        assert!(config.provider.is_none());
    }

    #[test]
    fn default_config_has_no_model() {
        let config = AgentServiceConfig::default();
        assert!(config.model.is_none());
    }

    #[test]
    fn default_config_has_no_api_key() {
        let config = AgentServiceConfig::default();
        assert!(config.api_key.is_none());
    }

    #[test]
    fn default_config_has_no_system_prompt() {
        let config = AgentServiceConfig::default();
        assert!(config.system_prompt.is_none());
    }

    #[test]
    fn default_config_has_empty_skills() {
        let config = AgentServiceConfig::default();
        assert!(config.agent_skills.is_empty());
    }

    #[test]
    fn default_config_max_iterations_is_50() {
        let config = AgentServiceConfig::default();
        assert_eq!(config.max_tool_iterations, 50);
    }

    #[test]
    fn default_config_working_directory_is_set() {
        let config = AgentServiceConfig::default();
        // Should be either current dir or "."
        assert!(!config.working_directory.as_os_str().is_empty());
    }

    #[test]
    fn config_with_custom_values() {
        let config = AgentServiceConfig {
            provider: Some("anthropic".into()),
            model: Some("claude-sonnet-4-6".into()),
            api_key: Some("sk-test-key".into()),
            system_prompt: Some("You are a helpful assistant.".into()),
            working_directory: PathBuf::from("/tmp/test"),
            persona_dir: None,
            agent_skills: Vec::new(),
            max_tool_iterations: 25,
        };
        assert_eq!(config.provider.as_deref(), Some("anthropic"));
        assert_eq!(config.model.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(config.api_key.as_deref(), Some("sk-test-key"));
        assert_eq!(
            config.system_prompt.as_deref(),
            Some("You are a helpful assistant.")
        );
        assert_eq!(config.working_directory, PathBuf::from("/tmp/test"));
        assert_eq!(config.max_tool_iterations, 25);
    }
}
