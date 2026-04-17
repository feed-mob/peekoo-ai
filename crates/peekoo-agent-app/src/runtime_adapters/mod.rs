use crate::agent_provider_service::ProviderConfig;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeLoginLaunch {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreferredLoginMethod {
    Acp,
    Native,
}

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
    /// When `node_bin_dir` is provided (typically the bundled Node.js binary
    /// directory from the Tauri resource bundle), it is prepended to PATH so
    /// agent wrapper scripts that depend on `node` work even when the user
    /// has no system Node.js installation (common on macOS GUI apps).
    fn build_launch_env(
        &self,
        provider_config: &ProviderConfig,
        node_bin_dir: Option<&Path>,
    ) -> HashMap<String, String> {
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

        // Forward PATH, prepending the bundled Node.js bin directory when
        // available so agent wrapper scripts can resolve `node`.
        let current_path = std::env::var("PATH").unwrap_or_default();
        let enriched_path = node_bin_dir
            .filter(|dir| dir.is_dir())
            .map(|dir| format!("{}:{}", dir.display(), current_path))
            .unwrap_or(current_path);
        if !env.contains_key("PATH") {
            env.insert("PATH".to_string(), enriched_path);
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

    fn build_native_login_launch(
        &self,
        _command: &str,
        _base_args: &[String],
        _install_dir: &Path,
    ) -> Option<NativeLoginLaunch> {
        None
    }

    fn build_manual_native_login_command(
        &self,
        command: &str,
        base_args: &[String],
        install_dir: &Path,
    ) -> Option<String> {
        let launch = self.build_native_login_launch(command, base_args, install_dir)?;

        let shell_quote = |s: &str| -> String {
            if s.contains(|c: char| !c.is_alphanumeric() && !"-_./:@".contains(c)) {
                format!("'{}'", s.replace('\'', "'\\''"))
            } else {
                s.to_string()
            }
        };

        let mut parts = vec![shell_quote(&launch.command)];
        for arg in &launch.args {
            parts.push(shell_quote(arg));
        }

        Some(parts.join(" "))
    }

    fn preferred_login_method(&self) -> Option<PreferredLoginMethod> {
        None
    }
}

/// All runtimes use the same adapter by default; specific runtimes can layer on
/// native login behavior without changing the generic ACP launch env handling.
pub fn adapter_for_runtime(runtime_id: &str) -> Box<dyn RuntimeAdapter> {
    match runtime_id {
        "kimi" => Box::new(KimiRuntimeAdapter),
        "qwen-code" => Box::new(QwenCodeRuntimeAdapter),
        _ => Box::new(CustomRuntimeAdapter),
    }
}

pub struct CustomRuntimeAdapter;

pub struct KimiRuntimeAdapter;

pub struct QwenCodeRuntimeAdapter;

impl RuntimeAdapter for CustomRuntimeAdapter {}

impl RuntimeAdapter for KimiRuntimeAdapter {
    fn build_native_login_launch(
        &self,
        command: &str,
        _base_args: &[String],
        install_dir: &Path,
    ) -> Option<NativeLoginLaunch> {
        Some(NativeLoginLaunch {
            command: command.to_string(),
            args: vec!["login".to_string()],
            cwd: install_dir.to_path_buf(),
        })
    }

    fn preferred_login_method(&self) -> Option<PreferredLoginMethod> {
        Some(PreferredLoginMethod::Native)
    }
}

impl RuntimeAdapter for QwenCodeRuntimeAdapter {
    fn build_native_login_launch(
        &self,
        command: &str,
        base_args: &[String],
        install_dir: &Path,
    ) -> Option<NativeLoginLaunch> {
        let mut args = Vec::new();
        let mut inserted = false;

        for arg in base_args {
            if arg == "--" {
                args.push(arg.clone());
                args.push("auth".to_string());
                inserted = true;
                break;
            }
            args.push(arg.clone());
        }

        if !inserted {
            if !args.is_empty() {
                args.push("--".to_string());
            }
            args.push("auth".to_string());
        }

        Some(NativeLoginLaunch {
            command: command.to_string(),
            args,
            cwd: install_dir.to_path_buf(),
        })
    }

    fn preferred_login_method(&self) -> Option<PreferredLoginMethod> {
        Some(PreferredLoginMethod::Native)
    }
}

#[cfg(test)]
mod tests {
    use super::{CustomRuntimeAdapter, RuntimeAdapter, adapter_for_runtime};
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

        let env = adapter.build_launch_env(&config, None);

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

        let env = adapter.build_launch_env(&config, None);

        assert_eq!(env.get("HOME").map(String::as_str), Some("/custom/home"));
    }

    #[test]
    fn build_launch_env_prepends_node_bin_dir_to_path() {
        let adapter = CustomRuntimeAdapter;
        let config = ProviderConfig {
            default_model: None,
            env_vars: HashMap::new(),
            custom_args: vec![],
        };

        // Use a directory that exists on any system.
        let node_bin = std::path::Path::new("/tmp");
        let env = adapter.build_launch_env(&config, Some(node_bin));

        let path = env.get("PATH").expect("PATH should be set");
        assert!(
            path.starts_with("/tmp:"),
            "PATH should start with the node bin dir, got: {path}"
        );
    }

    #[test]
    fn kimi_native_login_uses_installed_binary_with_login_arg() {
        let adapter = adapter_for_runtime("kimi");
        let install_dir = std::path::Path::new("/home/test/.peekoo/resources/agents/kimi");

        let launch = adapter
            .build_native_login_launch(
                "/home/test/.peekoo/resources/agents/kimi/kimi",
                &[],
                install_dir,
            )
            .expect("kimi native login launch");

        assert_eq!(launch.command, "/home/test/.peekoo/resources/agents/kimi/kimi");
        assert_eq!(launch.args, vec!["login".to_string()]);
        assert_eq!(launch.cwd, install_dir);
    }

    #[test]
    fn qwen_native_login_appends_auth_to_npx_command() {
        let adapter = adapter_for_runtime("qwen-code");
        let install_dir = std::path::Path::new("/home/test/.peekoo/resources/agents/qwen-code");

        let launch = adapter
            .build_native_login_launch(
                "/home/test/.peekoo/resources/node/bin/npm",
                &[
                    "exec".to_string(),
                    "@qwen-code/qwen-code".to_string(),
                    "--".to_string(),
                    "acp".to_string(),
                    "--stdio".to_string(),
                ],
                install_dir,
            )
            .expect("qwen native login launch");

        assert_eq!(launch.command, "/home/test/.peekoo/resources/node/bin/npm");
        assert_eq!(
            launch.args,
            vec![
                "exec".to_string(),
                "@qwen-code/qwen-code".to_string(),
                "--".to_string(),
                "auth".to_string()
            ]
        );
        assert_eq!(launch.cwd, install_dir);
    }

    #[test]
    fn kimi_prefers_native_login() {
        let adapter = adapter_for_runtime("kimi");

        assert_eq!(adapter.preferred_login_method(), Some(super::PreferredLoginMethod::Native));
    }

    #[test]
    fn custom_runtime_has_no_login_preference() {
        let adapter = adapter_for_runtime("custom-runtime");

        assert_eq!(adapter.preferred_login_method(), None);
    }
}
