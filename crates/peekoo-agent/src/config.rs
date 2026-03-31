//! Configuration for the agent service.

use agent_client_protocol::McpServer;
use std::collections::HashMap;
use std::path::PathBuf;

pub const PEEKOO_OPENCODE_BIN_ENV: &str = "PEEKOO_OPENCODE_BIN";

/// Source of an agent provider
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderSource {
    /// Built-in provider with hardcoded defaults
    Builtin,
    /// Installed from ACP registry
    Registry,
    /// Custom user-defined provider
    Custom,
}

/// Agent provider configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentProvider {
    /// Provider identifier (e.g., "opencode", "cursor", "pi-acp")
    pub id: String,
    /// Command to execute
    pub command: String,
    /// Arguments to pass
    pub args: Vec<String>,
    /// Source of the provider (for special handling)
    pub source: ProviderSource,
}

impl AgentProvider {
    /// Create a provider from registry runtime info
    pub fn from_registry(id: &str, command: &str, args: Vec<String>) -> Self {
        Self {
            id: id.to_string(),
            command: command.to_string(),
            args,
            source: ProviderSource::Registry,
        }
    }

    /// Create a custom provider
    pub fn custom(id: &str, command: &str, args: Vec<String>) -> Self {
        Self {
            id: id.to_string(),
            command: command.to_string(),
            args,
            source: ProviderSource::Custom,
        }
    }

    /// Built-in pi-acp provider
    pub fn pi_acp() -> Self {
        Self {
            id: "pi-acp".to_string(),
            command: "npx".to_string(),
            args: vec!["pi-acp".to_string()],
            source: ProviderSource::Builtin,
        }
    }

    /// Built-in opencode provider
    pub fn opencode() -> Self {
        Self {
            id: "opencode".to_string(),
            command: "opencode".to_string(),
            args: vec!["acp".to_string()],
            source: ProviderSource::Builtin,
        }
    }

    /// Built-in claude-code provider
    pub fn claude_code() -> Self {
        Self {
            id: "claude-code".to_string(),
            command: "npx".to_string(),
            args: vec!["@anthropic-ai/claude-code".to_string()],
            source: ProviderSource::Builtin,
        }
    }

    /// Built-in codex provider
    pub fn codex() -> Self {
        Self {
            id: "codex".to_string(),
            command: "npx".to_string(),
            args: vec!["@zed-industries/codex-acp".to_string()],
            source: ProviderSource::Builtin,
        }
    }

    /// Get the command and arguments to spawn this provider
    pub fn command(&self) -> (String, Vec<String>) {
        (self.command.clone(), self.args.clone())
    }

    /// Get the command and arguments to spawn this provider, allowing environment overrides.
    pub fn command_with_environment(
        &self,
        environment: &HashMap<String, String>,
    ) -> (String, Vec<String>) {
        // Special handling for opencode bundled binary path
        if self.id == "opencode" {
            if let Some(command) = environment
                .get(PEEKOO_OPENCODE_BIN_ENV)
                .filter(|value| !value.trim().is_empty())
                .cloned()
                .or_else(|| {
                    std::env::var(PEEKOO_OPENCODE_BIN_ENV)
                        .ok()
                        .filter(|value| !value.trim().is_empty())
                })
            {
                return (command, vec!["acp".to_string()]);
            }
        }

        self.command()
    }

    /// Provider identifier string
    pub fn id(&self) -> String {
        self.id.clone()
    }

    /// Check if this is a built-in provider
    pub fn is_builtin(&self) -> bool {
        self.source == ProviderSource::Builtin
    }

    /// Check if this is a registry-installed provider
    pub fn is_registry(&self) -> bool {
        self.source == ProviderSource::Registry
    }

    /// Check if this is a custom provider
    pub fn is_custom(&self) -> bool {
        self.source == ProviderSource::Custom
    }
}

impl Default for AgentProvider {
    fn default() -> Self {
        Self::opencode()
    }
}

/// Configuration for creating an [`AgentService`](super::service::AgentService).
///
/// Provides a peekoo-specific configuration surface for ACP-compatible agents.
pub struct AgentServiceConfig {
    /// Provider to use (default: Opencode)
    pub provider: AgentProvider,

    /// Model identifier (provider-specific)
    pub model: Option<String>,

    /// LLM provider identifier selected for the current runtime.
    pub llm_provider_id: Option<String>,

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

    /// MCP servers to attach to ACP sessions.
    pub mcp_servers: Vec<McpServer>,
}

impl Default for AgentServiceConfig {
    fn default() -> Self {
        Self {
            provider: AgentProvider::opencode(),
            model: None,
            llm_provider_id: None,
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
            mcp_servers: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_default_provider() {
        let config = AgentServiceConfig::default();
        assert_eq!(config.provider.id(), "opencode");
    }

    #[test]
    fn default_config_has_no_model() {
        let config = AgentServiceConfig::default();
        assert!(config.model.is_none());
    }

    #[test]
    fn default_config_has_no_llm_provider_id() {
        let config = AgentServiceConfig::default();
        assert!(config.llm_provider_id.is_none());
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
    fn default_config_has_no_mcp_servers() {
        let config = AgentServiceConfig::default();
        assert!(config.mcp_servers.is_empty());
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
        assert_eq!(AgentProvider::pi_acp().id(), "pi-acp");
        assert_eq!(AgentProvider::opencode().id(), "opencode");
        assert_eq!(AgentProvider::claude_code().id(), "claude-code");
        assert_eq!(AgentProvider::codex().id(), "codex");

        let custom = AgentProvider::custom(
            "my-agent",
            "/path/to/agent",
            vec!["--mode".to_string(), "acp".to_string()],
        );
        assert_eq!(custom.id(), "my-agent");
    }

    #[test]
    fn provider_command_returns_expected_commands() {
        let (cmd, args) = AgentProvider::pi_acp().command();
        assert_eq!(cmd, "npx");
        assert_eq!(args, vec!["pi-acp"]);

        let (cmd, args) = AgentProvider::opencode().command();
        assert_eq!(cmd, "opencode");
        assert_eq!(args, vec!["acp"]);

        let custom = AgentProvider::custom("my-agent", "/path/to/agent", vec!["arg1".to_string()]);
        let (cmd, args) = custom.command();
        assert_eq!(cmd, "/path/to/agent");
        assert_eq!(args, vec!["arg1"]);
    }

    #[test]
    fn opencode_command_uses_environment_override() {
        let env = HashMap::from([(
            PEEKOO_OPENCODE_BIN_ENV.to_string(),
            "/tmp/opencode".to_string(),
        )]);

        let (cmd, args) = AgentProvider::opencode().command_with_environment(&env);
        assert_eq!(cmd, "/tmp/opencode");
        assert_eq!(args, vec!["acp"]);
    }

    #[test]
    fn config_with_custom_values() {
        let config = AgentServiceConfig {
            provider: AgentProvider::opencode(),
            model: Some("gpt-4o".to_string()),
            llm_provider_id: Some("openai".to_string()),
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
            mcp_servers: Vec::new(),
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
        assert_eq!(AgentProvider::pi_acp(), AgentProvider::pi_acp());
        assert_ne!(AgentProvider::pi_acp(), AgentProvider::opencode());

        let custom1 = AgentProvider::custom("my-agent", "cmd", vec!["arg1".to_string()]);
        let custom2 = AgentProvider::custom("my-agent", "cmd", vec!["arg1".to_string()]);
        assert_eq!(custom1, custom2);

        let custom3 = AgentProvider::custom("other-agent", "cmd", vec!["arg1".to_string()]);
        assert_ne!(custom1, custom3);
    }
}
