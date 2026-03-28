use crate::agent_provider_service::{ProviderConfig, RuntimeLlmProviderInfo};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAuthMode {
    AgentLogin,
    ApiKey,
    None,
}

pub trait RuntimeAdapter {
    fn runtime_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn default_command(&self) -> &'static str;
    fn default_args(&self) -> Vec<String>;
    fn install_hint(&self) -> Option<&'static str>;
    fn supported_auth_modes(&self) -> Vec<RuntimeAuthMode>;

    fn build_launch_env(&self, provider_config: &ProviderConfig) -> HashMap<String, String> {
        provider_config.env_vars.clone()
    }

    fn build_launch_args(
        &self,
        base_args: &[String],
        provider_config: &ProviderConfig,
        _model: &str,
    ) -> Vec<String> {
        let mut args = base_args.to_vec();
        args.extend(provider_config.custom_args.clone());
        args
    }

    /// Merge runtime-scoped LLM provider config into the launch environment.
    fn apply_llm_provider_env(
        &self,
        env: &mut HashMap<String, String>,
        llm_provider: &RuntimeLlmProviderInfo,
    ) {
        for (key, value) in &llm_provider.config {
            env.insert(key.clone(), value.clone());
        }
    }

    /// Merge model selection into the launch environment.
    fn apply_model_env(&self, env: &mut HashMap<String, String>, model: &str) {
        if !model.is_empty() {
            env.insert("PEEKOO_AGENT_MODEL".to_string(), model.to_string());
        }
    }
}

/// Look up the adapter for a given runtime id.
pub fn adapter_for_runtime(runtime_id: &str) -> Box<dyn RuntimeAdapter> {
    match runtime_id {
        "codex" => Box::new(CodexRuntimeAdapter),
        "claude-code" => Box::new(ClaudeCodeRuntimeAdapter),
        "opencode" => Box::new(OpencodeRuntimeAdapter),
        "pi-acp" => Box::new(PiAcpRuntimeAdapter),
        _ => Box::new(CustomRuntimeAdapter),
    }
}

pub struct PiAcpRuntimeAdapter;

impl RuntimeAdapter for PiAcpRuntimeAdapter {
    fn runtime_id(&self) -> &'static str {
        "pi-acp"
    }

    fn display_name(&self) -> &'static str {
        "Peekoo ACP"
    }

    fn default_command(&self) -> &'static str {
        "npx"
    }

    fn default_args(&self) -> Vec<String> {
        vec!["pi-acp".to_string()]
    }

    fn install_hint(&self) -> Option<&'static str> {
        Some("Bundled with Peekoo")
    }

    fn supported_auth_modes(&self) -> Vec<RuntimeAuthMode> {
        vec![RuntimeAuthMode::ApiKey]
    }
}

pub struct CustomRuntimeAdapter;

impl RuntimeAdapter for CustomRuntimeAdapter {
    fn runtime_id(&self) -> &'static str {
        "custom"
    }

    fn display_name(&self) -> &'static str {
        "Custom ACP Runtime"
    }

    fn default_command(&self) -> &'static str {
        ""
    }

    fn default_args(&self) -> Vec<String> {
        Vec::new()
    }

    fn install_hint(&self) -> Option<&'static str> {
        None
    }

    fn supported_auth_modes(&self) -> Vec<RuntimeAuthMode> {
        vec![RuntimeAuthMode::ApiKey, RuntimeAuthMode::None]
    }
}

pub struct CodexRuntimeAdapter;

impl RuntimeAdapter for CodexRuntimeAdapter {
    fn runtime_id(&self) -> &'static str {
        "codex"
    }

    fn display_name(&self) -> &'static str {
        "Codex CLI (ACP)"
    }

    fn default_command(&self) -> &'static str {
        "codex"
    }

    fn default_args(&self) -> Vec<String> {
        vec!["--experimental-acp".to_string()]
    }

    fn install_hint(&self) -> Option<&'static str> {
        Some("npm i -g @zed-industries/codex-acp")
    }

    fn supported_auth_modes(&self) -> Vec<RuntimeAuthMode> {
        vec![RuntimeAuthMode::AgentLogin, RuntimeAuthMode::ApiKey]
    }
}

pub struct ClaudeCodeRuntimeAdapter;

impl RuntimeAdapter for ClaudeCodeRuntimeAdapter {
    fn runtime_id(&self) -> &'static str {
        "claude-code"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code (ACP)"
    }

    fn default_command(&self) -> &'static str {
        "claude-code-acp"
    }

    fn default_args(&self) -> Vec<String> {
        Vec::new()
    }

    fn install_hint(&self) -> Option<&'static str> {
        Some("npm i -g @zed-industries/claude-code-acp")
    }

    fn supported_auth_modes(&self) -> Vec<RuntimeAuthMode> {
        vec![RuntimeAuthMode::AgentLogin, RuntimeAuthMode::ApiKey]
    }
}

pub struct OpencodeRuntimeAdapter;

impl RuntimeAdapter for OpencodeRuntimeAdapter {
    fn runtime_id(&self) -> &'static str {
        "opencode"
    }

    fn display_name(&self) -> &'static str {
        "OpenCode (ACP)"
    }

    fn default_command(&self) -> &'static str {
        "opencode"
    }

    fn default_args(&self) -> Vec<String> {
        vec!["agent".to_string()]
    }

    fn install_hint(&self) -> Option<&'static str> {
        Some("npm i -g opencode-ai")
    }

    fn supported_auth_modes(&self) -> Vec<RuntimeAuthMode> {
        vec![RuntimeAuthMode::ApiKey]
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ClaudeCodeRuntimeAdapter, CodexRuntimeAdapter, OpencodeRuntimeAdapter, RuntimeAdapter,
    };
    use crate::agent_provider_service::ProviderConfig;
    use std::collections::HashMap;

    #[test]
    fn codex_adapter_maps_openai_compatible_env() {
        let adapter = CodexRuntimeAdapter;
        let config = ProviderConfig {
            default_model: Some("gpt-5.3-codex".to_string()),
            env_vars: HashMap::from([
                ("OPENAI_API_KEY".to_string(), "sk-test".to_string()),
                (
                    "OPENAI_BASE_URL".to_string(),
                    "https://openrouter.example/v1".to_string(),
                ),
            ]),
            custom_args: vec!["--debug".to_string()],
        };

        let env = adapter.build_launch_env(&config);
        let args = adapter.build_launch_args(&["--acp".to_string()], &config, "gpt-5.3-codex");

        assert_eq!(adapter.runtime_id(), "codex");
        assert_eq!(
            env.get("OPENAI_API_KEY").map(String::as_str),
            Some("sk-test")
        );
        assert_eq!(
            env.get("OPENAI_BASE_URL").map(String::as_str),
            Some("https://openrouter.example/v1")
        );
        assert!(args.contains(&"--acp".to_string()));
        assert!(args.contains(&"--debug".to_string()));
    }

    #[test]
    fn claude_code_adapter_preserves_anthropic_env() {
        let adapter = ClaudeCodeRuntimeAdapter;
        let config = ProviderConfig {
            default_model: Some("claude-sonnet-4-6".to_string()),
            env_vars: HashMap::from([(
                "ANTHROPIC_API_KEY".to_string(),
                "anthropic-key".to_string(),
            )]),
            custom_args: Vec::new(),
        };

        let env = adapter.build_launch_env(&config);

        assert_eq!(adapter.runtime_id(), "claude-code");
        assert_eq!(
            env.get("ANTHROPIC_API_KEY").map(String::as_str),
            Some("anthropic-key")
        );
    }

    #[test]
    fn opencode_adapter_exposes_runtime_metadata() {
        let adapter = OpencodeRuntimeAdapter;

        assert_eq!(adapter.runtime_id(), "opencode");
        assert_eq!(adapter.default_command(), "opencode");
        assert!(!adapter.supported_auth_modes().is_empty());
        assert!(adapter.install_hint().is_some());
    }
}
