use crate::agent_provider_service::ProviderConfig;
use std::collections::HashMap;

pub trait RuntimeAdapter: Send + Sync {
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
    ///
    /// Prepends the managed Node.js bin directory to PATH so `npx` commands
    /// work even when the user doesn't have Node.js installed on their system.
    fn build_launch_env(&self, provider_config: &ProviderConfig) -> HashMap<String, String> {
        let mut env = provider_config.env_vars.clone();

        // Forward critical path vars so runtimes can find credentials/config
        // regardless of how the Tauri app was launched.
        for key in &["HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME"] {
            if !env.contains_key(*key)
                && let Ok(val) = std::env::var(key)
            {
                env.insert((*key).to_string(), val);
            }
        }

        // Forward PATH, prepending managed Node.js bin directory so `npx`
        // works even without a system Node.js installation.
        let current_path = std::env::var("PATH").unwrap_or_default();
        let managed_path = peekoo_node_runtime::paths::node_dir()
            .ok()
            .map(|dir| dir.join("bin"))
            .filter(|bin_dir| bin_dir.exists())
            .map(|bin_dir| format!("{}:{}", bin_dir.display(), current_path))
            .unwrap_or(current_path);
        if !env.contains_key("PATH") {
            env.insert("PATH".to_string(), managed_path);
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

/// All runtimes use the same adapter — command, args, and display name
/// come from the ACP registry database, not hardcoded values.
pub fn adapter_for_runtime(_runtime_id: &str) -> Box<dyn RuntimeAdapter> {
    Box::new(CustomRuntimeAdapter)
}

pub struct CustomRuntimeAdapter;

impl RuntimeAdapter for CustomRuntimeAdapter {
    fn build_terminal_auth_launch(
        &self,
        _command: &str,
        _base_args: &[String],
        _method_args: &[String],
    ) -> Option<(String, Vec<String>)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{CustomRuntimeAdapter, RuntimeAdapter};
    use crate::agent_provider_service::ProviderConfig;
    use std::collections::HashMap;

    #[test]
    fn default_build_launch_env_forwards_home_when_absent() {
        let adapter = CustomRuntimeAdapter;
        let config = ProviderConfig {
            default_model: None,
            env_vars: HashMap::new(),
            custom_args: vec![],
        };

        let env = adapter.build_launch_env(&config);

        if let Ok(expected_home) = std::env::var("HOME") {
            assert_eq!(
                env.get("HOME").map(String::as_str),
                Some(expected_home.as_str())
            );
        }
    }

    #[test]
    fn default_build_launch_env_does_not_override_user_configured_home() {
        let adapter = CustomRuntimeAdapter;
        let config = ProviderConfig {
            default_model: None,
            env_vars: HashMap::from([("HOME".to_string(), "/custom/home".to_string())]),
            custom_args: vec![],
        };

        let env = adapter.build_launch_env(&config);

        assert_eq!(env.get("HOME").map(String::as_str), Some("/custom/home"));
    }
}
