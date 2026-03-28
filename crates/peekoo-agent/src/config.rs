//! Configuration for the agent service.

use std::collections::HashMap;
use std::path::PathBuf;

/// Agent provider types (built-in + custom)
#[derive(Debug, Clone, PartialEq)]
pub enum AgentProvider {
    /// pi-acp (default, preferred)
    PiAcp,
    /// Zed's opencode agent
    Opencode,
    /// Anthropic's Claude Code
    ClaudeCode,
    /// Zed's Codex agent
    Codex,
    /// Custom command
    Custom { command: String, args: Vec<String> },
}

impl AgentProvider {
    /// Get the command and arguments to spawn this provider
    pub fn command(&self) -> (String, Vec<String>) {
        match self {
            AgentProvider::PiAcp => ("npx".to_string(), vec!["pi-acp".to_string()]),
            AgentProvider::Opencode => ("npx".to_string(), vec!["opencode-ai".to_string()]),
            AgentProvider::ClaudeCode => (
                "npx".to_string(),
                vec!["@anthropic-ai/claude-code".to_string()],
            ),
            AgentProvider::Codex => (
                "npx".to_string(),
                vec!["@zed-industries/codex-acp".to_string()],
            ),
            AgentProvider::Custom { command, args } => (command.clone(), args.clone()),
        }
    }

    /// Provider identifier string
    pub fn id(&self) -> String {
        match self {
            AgentProvider::PiAcp => "pi-acp".to_string(),
            AgentProvider::Opencode => "opencode".to_string(),
            AgentProvider::ClaudeCode => "claude-code".to_string(),
            AgentProvider::Codex => "codex".to_string(),
            AgentProvider::Custom { command, .. } => format!("custom:{}", command),
        }
    }
}

/// Configuration for creating an [`AgentService`](super::service::AgentService).
///
/// Provides a peekoo-specific configuration surface for ACP-compatible agents.
pub struct AgentServiceConfig {
    /// Provider to use (default: PiAcp)
    pub provider: AgentProvider,

    /// Model identifier (provider-specific)
    pub model: Option<String>,

    /// API key (if needed, passed to provider via env)
    pub api_key: Option<String>,

    /// System prompt
    pub system_prompt: Option<String>,

    /// Working directory
    pub working_directory: PathBuf,

    /// Persona directory
    pub persona_dir: Option<PathBuf>,

    /// Agent skills
    pub agent_skills: Vec<PathBuf>,

    /// Auto-discover configuration
    pub auto_discover: bool,

    /// Maximum tool iterations
    pub max_tool_iterations: usize,

    /// Session directory for peekoo-managed persistence
    pub session_dir: Option<PathBuf>,

    /// Whether to disable session persistence
    pub no_session: bool,

    /// Specific session to resume
    pub resume_session_id: Option<String>,

    /// Session file path to resume (for stashed sessions)
    pub session_path: Option<PathBuf>,

    /// Environment variables to pass to ACP agent
    pub environment: HashMap<String, String>,
}

impl Default for AgentServiceConfig {
    fn default() -> Self {
        Self {
            provider: AgentProvider::PiAcp,
            model: None,
            api_key: None,
            system_prompt: None,
            working_directory: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            persona_dir: None,
            agent_skills: Vec::new(),
            auto_discover: true,
            max_tool_iterations: 50,
            session_dir: None,
            no_session: false,
            resume_session_id: None,
            session_path: None,
            environment: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_default_provider() {
        let config = AgentServiceConfig::default();
        assert_eq!(config.provider, AgentProvider::PiAcp);
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
    fn default_config_enables_session_persistence() {
        let config = AgentServiceConfig::default();
        assert!(!config.no_session);
        assert!(config.session_dir.is_none());
        assert!(config.resume_session_id.is_none());
    }

    #[test]
    fn provider_id_returns_expected_values() {
        assert_eq!(AgentProvider::PiAcp.id(), "pi-acp");
        assert_eq!(AgentProvider::Opencode.id(), "opencode");
        assert_eq!(AgentProvider::ClaudeCode.id(), "claude-code");
        assert_eq!(AgentProvider::Codex.id(), "codex");

        let custom = AgentProvider::Custom {
            command: "/path/to/agent".to_string(),
            args: vec!["--mode".to_string(), "acp".to_string()],
        };
        assert!(custom.id().starts_with("custom:"));
    }

    #[test]
    fn provider_command_returns_expected_commands() {
        let (cmd, args) = AgentProvider::PiAcp.command();
        assert_eq!(cmd, "npx");
        assert_eq!(args, vec!["pi-acp"]);

        let (cmd, args) = AgentProvider::Opencode.command();
        assert_eq!(cmd, "npx");
        assert_eq!(args, vec!["opencode-ai"]);

        let (cmd, args) = AgentProvider::Custom {
            command: "/path/to/agent".to_string(),
            args: vec!["arg1".to_string()],
        }
        .command();
        assert_eq!(cmd, "/path/to/agent");
        assert_eq!(args, vec!["arg1"]);
    }

    #[test]
    fn config_with_custom_values() {
        let config = AgentServiceConfig {
            provider: AgentProvider::Opencode,
            model: Some("gpt-4o".to_string()),
            api_key: Some("sk-test-key".to_string()),
            system_prompt: Some("You are a helpful assistant.".to_string()),
            working_directory: PathBuf::from("/tmp/test"),
            persona_dir: None,
            agent_skills: Vec::new(),
            auto_discover: false,
            max_tool_iterations: 25,
            session_dir: Some(PathBuf::from("/tmp/sessions")),
            no_session: false,
            resume_session_id: None,
            session_path: None,
            environment: HashMap::new(),
        };
        assert_eq!(config.provider.id(), "opencode");
        assert_eq!(config.model.as_deref(), Some("gpt-4o"));
        assert_eq!(config.api_key.as_deref(), Some("sk-test-key"));
        assert_eq!(
            config.system_prompt.as_deref(),
            Some("You are a helpful assistant.")
        );
        assert_eq!(config.working_directory, PathBuf::from("/tmp/test"));
        assert_eq!(config.max_tool_iterations, 25);
        assert_eq!(config.session_dir, Some(PathBuf::from("/tmp/sessions")));
        assert!(!config.no_session);
    }

    #[test]
    fn provider_equality() {
        assert_eq!(AgentProvider::PiAcp, AgentProvider::PiAcp);
        assert_ne!(AgentProvider::PiAcp, AgentProvider::Opencode);

        let custom1 = AgentProvider::Custom {
            command: "cmd".to_string(),
            args: vec!["arg1".to_string()],
        };
        let custom2 = AgentProvider::Custom {
            command: "cmd".to_string(),
            args: vec!["arg1".to_string()],
        };
        assert_eq!(custom1, custom2);
    }
}
