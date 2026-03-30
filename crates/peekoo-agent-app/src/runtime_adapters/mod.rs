use crate::agent_provider_service::ProviderConfig;
use std::collections::HashMap;

pub trait RuntimeAdapter: Send + Sync {
    fn runtime_id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn default_command(&self) -> &'static str;
    fn default_args(&self) -> Vec<String>;
    fn install_hint(&self) -> Option<&'static str>;

    fn build_terminal_auth_launch(
        &self,
        command: &str,
        base_args: &[String],
        method_args: &[String],
    ) -> Option<(String, Vec<String>)> {
        let args = if command == "npx" || command.ends_with("/npx") {
            let mut args = base_args.to_vec();
            args.extend(method_args.to_vec());
            args
        } else {
            method_args.to_vec()
        };

        Some((command.to_string(), args))
    }

    /// Build the environment for the spawned runtime process.
    ///
    /// Forwards critical OS path variables from the parent process so child
    /// runtimes can locate config/credentials even when the Tauri app was
    /// launched from a desktop entry with a stripped environment.
    /// User-configured values in `provider_config.env_vars` take precedence.
    fn build_launch_env(&self, provider_config: &ProviderConfig) -> HashMap<String, String> {
        let mut env = provider_config.env_vars.clone();

        // Forward critical path vars so runtimes can find credentials/config
        // regardless of how the Tauri app was launched.
        for key in &["HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "PATH"] {
            if !env.contains_key(*key) {
                if let Ok(val) = std::env::var(key) {
                    env.insert((*key).to_string(), val);
                }
            }
        }

        env
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

    /// Merge model selection into the launch environment.
    fn apply_model_env(&self, env: &mut HashMap<String, String>, model: &str) {
        if !model.is_empty() {
            env.insert("PEEKOO_AGENT_MODEL".to_string(), model.to_string());
        }
    }

    /// Build a shell-escaped manual login command string for display in the UI.
    fn build_manual_login_command(
        &self,
        command: &str,
        base_args: &[String],
        method_args: &[String],
    ) -> Option<String> {
        let (_, args) = self.build_terminal_auth_launch(command, base_args, method_args)?;

        // Quote arguments containing spaces or shell metacharacters.
        let shell_quote = |s: &str| -> String {
            if s.contains(|c: char| !c.is_alphanumeric() && !"-_./:@".contains(c)) {
                format!("'{}'", s.replace('\'', "'\\''"))
            } else {
                s.to_string()
            }
        };

        let mut parts = vec![shell_quote(command)];
        for arg in &args {
            parts.push(shell_quote(arg));
        }

        Some(parts.join(" "))
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
        Some("npm i -g pi-acp")
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

    fn build_terminal_auth_launch(
        &self,
        _command: &str,
        _base_args: &[String],
        _method_args: &[String],
    ) -> Option<(String, Vec<String>)> {
        None
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
        "npx"
    }

    fn default_args(&self) -> Vec<String> {
        vec!["@anthropic-ai/claude-code".to_string()]
    }

    fn install_hint(&self) -> Option<&'static str> {
        Some("npm i -g @zed-industries/claude-code-acp")
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
        vec!["acp".to_string()]
    }

    fn install_hint(&self) -> Option<&'static str> {
        Some("Install OpenCode and make the `opencode` command available on PATH.")
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
    fn default_build_launch_env_forwards_home_when_absent() {
        // Use PiAcpRuntimeAdapter as a representative of the default impl.
        use super::PiAcpRuntimeAdapter;
        let adapter = PiAcpRuntimeAdapter;
        let config = ProviderConfig {
            default_model: None,
            env_vars: HashMap::new(),
            custom_args: vec![],
        };

        let env = adapter.build_launch_env(&config);

        // HOME should be forwarded from the parent process (always set in test env).
        if let Ok(expected_home) = std::env::var("HOME") {
            assert_eq!(
                env.get("HOME").map(String::as_str),
                Some(expected_home.as_str())
            );
        }
    }

    #[test]
    fn default_build_launch_env_does_not_override_user_configured_home() {
        use super::PiAcpRuntimeAdapter;
        let adapter = PiAcpRuntimeAdapter;
        let config = ProviderConfig {
            default_model: None,
            env_vars: HashMap::from([("HOME".to_string(), "/custom/home".to_string())]),
            custom_args: vec![],
        };

        let env = adapter.build_launch_env(&config);

        assert_eq!(env.get("HOME").map(String::as_str), Some("/custom/home"));
    }

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
        assert!(adapter.install_hint().is_some());
    }
}
