//! Tauri command wrappers for agent provider management
//!
//! These functions wrap the AgentProviderService to provide
//! the Tauri command interface for the frontend.

use crate::agent_provider_service::{
    AgentProviderService, InstallProviderRequest, InstallProviderResponse, InstallationMethod,
    PrerequisitesCheck, ProviderConfig, ProviderInfo, RuntimeInspectionResult,
    TestConnectionResult,
};
use crate::runtime_adapters::adapter_for_runtime;
use agent_client_protocol as acp;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RuntimeTerminalAuthLaunch {
    pub command: String,
    pub args: Vec<String>,
    pub env: std::collections::HashMap<String, String>,
    pub cwd: Option<PathBuf>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum RuntimeAuthenticationAction {
    Authenticated { message: String },
    LaunchTerminal(RuntimeTerminalAuthLaunch),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAuthenticationStatus {
    Authenticated,
    TerminalLoginStarted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAuthenticationResult {
    pub status: RuntimeAuthenticationStatus,
    pub message: String,
}

fn decorate_authenticate_runtime_error(
    runtime_id: &str,
    method_id: &str,
    error: anyhow::Error,
) -> anyhow::Error {
    if let Some(acp_error) = error.downcast_ref::<acp::Error>() {
        return match acp_error.code {
            acp::ErrorCode::MethodNotFound => anyhow::anyhow!(
                "Runtime {} advertised authentication method {}, but its ACP server does not implement ACP authenticate. Original error: {}",
                runtime_id,
                method_id,
                acp_error
            ),
            _ => anyhow::anyhow!(
                "Runtime {} authentication via {} failed: {}",
                runtime_id,
                method_id,
                acp_error
            ),
        };
    }

    error
}

async fn run_authentication_phase_with_timeout<T, F>(
    runtime_id: &str,
    phase: &str,
    timeout: std::time::Duration,
    future: F,
) -> anyhow::Result<T>
where
    F: Future<Output = anyhow::Result<T>>,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!(
            "Runtime {runtime_id} authentication timed out during {phase}"
        )),
    }
}

/// Initialize the provider service
pub fn create_provider_service(
    db_path: &PathBuf,
    data_dir: PathBuf,
) -> anyhow::Result<Arc<Mutex<AgentProviderService>>> {
    let service = AgentProviderService::new(db_path, data_dir)?;
    Ok(Arc::new(Mutex::new(service)))
}

/// List all available providers
pub async fn list_agent_providers(
    service: &AgentProviderService,
) -> anyhow::Result<Vec<ProviderInfo>> {
    service.list_providers()
}

/// Install a provider
pub async fn install_agent_provider(
    service: &AgentProviderService,
    req: InstallProviderRequest,
) -> anyhow::Result<InstallProviderResponse> {
    service.install_provider(req)
}

/// Set the default provider
pub fn set_default_provider(
    service: &AgentProviderService,
    provider_id: String,
) -> anyhow::Result<()> {
    service.set_default_provider(&provider_id)
}

/// Get provider configuration
pub fn get_provider_config(
    service: &AgentProviderService,
    provider_id: String,
) -> anyhow::Result<ProviderConfig> {
    service.get_provider_config(&provider_id)
}

/// Update provider configuration
pub fn update_provider_config(
    service: &AgentProviderService,
    provider_id: String,
    config: ProviderConfig,
) -> anyhow::Result<()> {
    service.update_provider_config(&provider_id, &config)
}

/// Test provider connection
pub async fn test_provider_connection(
    service: &AgentProviderService,
    provider_id: String,
) -> anyhow::Result<TestConnectionResult> {
    service.test_connection(&provider_id).await
}

/// Check installation prerequisites
pub async fn check_installation_prerequisites(
    service: &AgentProviderService,
    method: InstallationMethod,
) -> anyhow::Result<PrerequisitesCheck> {
    service.check_prerequisites(method)
}

/// Add a custom provider
pub fn add_custom_provider(
    service: &AgentProviderService,
    name: String,
    description: Option<String>,
    command: String,
    args: Vec<String>,
    working_dir: Option<String>,
) -> anyhow::Result<ProviderInfo> {
    service.add_custom_provider(
        &name,
        description.as_deref(),
        &command,
        &args,
        working_dir.as_deref(),
    )
}

/// Remove a custom provider
pub fn remove_custom_provider(
    service: &AgentProviderService,
    provider_id: String,
) -> anyhow::Result<()> {
    service.remove_custom_provider(&provider_id)
}

/// Get the default provider
pub fn get_default_provider(
    service: &AgentProviderService,
) -> anyhow::Result<Option<ProviderInfo>> {
    service.get_default_provider()
}

/// Uninstall a provider
pub fn uninstall_agent_provider(
    service: &AgentProviderService,
    provider_id: String,
) -> anyhow::Result<()> {
    service.uninstall_provider(&provider_id)
}

/// Inspect a runtime to discover its capabilities
pub async fn inspect_runtime(
    service: &AgentProviderService,
    runtime_id: String,
) -> anyhow::Result<RuntimeInspectionResult> {
    service.inspect_runtime(&runtime_id).await
}

/// Authenticate with a runtime using the specified auth method
pub async fn authenticate_runtime(
    service: &AgentProviderService,
    runtime_id: String,
    method_id: String,
) -> anyhow::Result<RuntimeAuthenticationAction> {
    use peekoo_agent::backend::acp::is_auth_required_error;
    use peekoo_agent::backend::{AcpBackend, AgentBackend, BackendConfig};

    // Get runtime info
    let runtime = service
        .get_runtime(&runtime_id)?
        .ok_or_else(|| anyhow::anyhow!("Runtime not found: {}", runtime_id))?;

    // Only authenticate with chat-visible (external/custom) runtimes
    if !runtime.is_chat_visible() {
        return Err(anyhow::anyhow!(
            "Cannot authenticate internal runtime: {}",
            runtime_id
        ));
    }

    // Get runtime configuration
    let config = service.get_provider_config(&runtime_id)?;

    // Get the command and args
    let (command, args) = if runtime.is_installed {
        service.get_runtime_command(&runtime_id).await?
    } else {
        return Err(anyhow::anyhow!("Runtime not installed: {}", runtime_id));
    };

    // Create ACP backend
    let mut backend = AcpBackend::new(command.clone(), args.clone());

    // Initialize the backend
    let backend_config = BackendConfig {
        working_directory: std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from(".")),
        system_prompt: None,
        model: config.default_model.clone(),
        provider: Some(runtime_id.clone()),
        api_key: None,
        environment: adapter_for_runtime(&runtime_id)
            .build_launch_env(&config, service.node_bin_dir()),
        mcp_servers: Vec::new(),
    };

    let auth_timeout = std::time::Duration::from_secs(20);

    match run_authentication_phase_with_timeout(
        &runtime_id,
        "initialize",
        auth_timeout,
        backend.initialize(backend_config),
    )
    .await
    {
        Ok(()) => {}
        Err(error) if is_auth_required_error(&error) => {
            tracing::info!(
                runtime_id,
                method_id,
                "Runtime initialization reported ACP auth required; continuing with runtime auth handling"
            );
        }
        Err(error) => return Err(error),
    }

    let Some(auth_method) = backend
        .auth_methods()
        .iter()
        .find(|method| method.id().to_string() == method_id)
        .cloned()
    else {
        let _ = backend.shutdown().await;
        return Err(anyhow::anyhow!(
            "Runtime {} does not advertise authentication method {}",
            runtime_id,
            method_id
        ));
    };

    if let acp::AuthMethod::Terminal(terminal_method) = auth_method {
        let adapter = adapter_for_runtime(&runtime_id);
        let Some((terminal_command, terminal_args)) =
            adapter.build_terminal_auth_launch(&command, &args, &terminal_method.args)
        else {
            let _ = backend.shutdown().await;
            return Err(anyhow::anyhow!(
                "Runtime {} advertises terminal authentication, but Peekoo does not yet know how to launch its login command.",
                runtime_id
            ));
        };

        // Build terminal env: start with the same forwarded OS vars that
        // build_launch_env provides, so the terminal session has HOME/PATH/etc.
        let mut terminal_env = adapter.build_launch_env(&config, service.node_bin_dir());
        terminal_env.extend(terminal_method.env);
        let _ = backend.shutdown().await;

        // Invalidate the cached inspection so the next inspect_runtime call
        // re-runs instead of returning the stale auth_required: true result.
        let _ = service.invalidate_runtime_inspection_cache(&runtime_id);

        return Ok(RuntimeAuthenticationAction::LaunchTerminal(
            RuntimeTerminalAuthLaunch {
                command: terminal_command,
                args: terminal_args,
                env: terminal_env,
                cwd: None,
                message: format!(
                    "Terminal login started for {}. Complete the login in the terminal window, then click Refresh.",
                    runtime.display_name
                ),
            },
        ));
    }

    let auth_result = run_authentication_phase_with_timeout(
        &runtime_id,
        "authenticate",
        auth_timeout,
        backend.authenticate(&method_id),
    )
    .await
    .map_err(|error| decorate_authenticate_runtime_error(&runtime_id, &method_id, error));

    auth_result?;
    run_authentication_phase_with_timeout(
        &runtime_id,
        "refresh",
        auth_timeout,
        backend.refresh_session_capabilities(),
    )
    .await
    .map_err(|error| {
            if is_auth_required_error(&error) {
                anyhow::anyhow!(
                    "Runtime {} started authentication with {}, but it still reports login is required. Finish the runtime login flow and refresh again.",
                    runtime_id,
                    method_id
                )
            } else {
                decorate_authenticate_runtime_error(&runtime_id, &method_id, error)
            }
        })?;

    backend.shutdown().await?;
    service.invalidate_runtime_inspection_cache(&runtime_id)?;
    tracing::info!(
        "Runtime {} authenticated successfully with method {}",
        runtime_id,
        method_id
    );
    Ok(RuntimeAuthenticationAction::Authenticated {
        message: format!(
            "Runtime {} authenticated successfully with method {}",
            runtime_id, method_id
        ),
    })
}

pub async fn launch_native_runtime_login(
    service: &AgentProviderService,
    runtime_id: String,
) -> anyhow::Result<RuntimeAuthenticationAction> {
    let runtime = service
        .get_runtime(&runtime_id)?
        .ok_or_else(|| anyhow::anyhow!("Runtime not found: {}", runtime_id))?;

    if !runtime.is_installed {
        return Err(anyhow::anyhow!("Runtime not installed: {}", runtime_id));
    }

    let (command, args) = service.get_runtime_command(&runtime_id).await?;
    let install_dir = service.runtime_install_dir(&runtime_id);
    let config = service.get_provider_config(&runtime_id)?;
    let adapter = adapter_for_runtime(&runtime_id);
    let launch = adapter
        .build_native_login_launch(&command, &args, &install_dir)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Runtime {} does not support native terminal login in Peekoo yet.",
                runtime_id
            )
        })?;

    Ok(RuntimeAuthenticationAction::LaunchTerminal(
        RuntimeTerminalAuthLaunch {
            command: launch.command,
            args: launch.args,
            env: adapter.build_launch_env(&config, service.node_bin_dir()),
            cwd: Some(launch.cwd),
            message: format!(
                "Terminal login started for {}. Complete the login in the terminal window, then click Refresh.",
                runtime.display_name
            ),
        },
    ))
}

/// Refresh runtime capabilities (re-inspect)
pub async fn refresh_runtime_capabilities(
    service: &AgentProviderService,
    runtime_id: String,
) -> anyhow::Result<RuntimeInspectionResult> {
    service.refresh_runtime_capabilities(&runtime_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use std::time::Duration;
    use tempfile::TempDir;

    fn create_test_service() -> (AgentProviderService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let data_dir = temp_dir.path().join("data");

        let service = AgentProviderService::test_only_new(&db_path, data_dir).unwrap();
        (service, temp_dir)
    }

    fn create_test_service_with_bundled_opencode()
    -> (AgentProviderService, TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let data_dir = temp_dir.path().join("data");
        let bundled_dir = temp_dir.path().join("opencode");
        std::fs::create_dir_all(&bundled_dir).unwrap();
        let bundled_bin = bundled_dir.join(if cfg!(windows) {
            "opencode.exe"
        } else {
            "opencode"
        });
        std::fs::write(&bundled_bin, b"fake-opencode").unwrap();

        let service = AgentProviderService::test_only_new_with_bundled_opencode(
            &db_path,
            data_dir,
            Some(bundled_bin.clone()),
        )
        .unwrap();
        (service, temp_dir, bundled_bin)
    }

    #[tokio::test]
    async fn test_list_providers_command() {
        let (service, _temp, _bin) = create_test_service_with_bundled_opencode();

        let providers = list_agent_providers(&service).await.unwrap();
        assert!(!providers.is_empty());
    }

    #[tokio::test]
    async fn test_set_default_provider_command() {
        let (service, _temp, _bin) = create_test_service_with_bundled_opencode();

        // opencode is already installed and default
        set_default_provider(&service, "opencode".to_string()).unwrap();

        let default = get_default_provider(&service).unwrap();
        assert_eq!(default.unwrap().provider_id, "opencode");
    }

    #[tokio::test]
    async fn test_custom_provider_commands() {
        let (service, _temp) = create_test_service();

        // Add custom provider
        let provider = add_custom_provider(
            &service,
            "My Agent".to_string(),
            Some("Custom test agent".to_string()),
            "/usr/bin/agent".to_string(),
            vec!["--acp".to_string()],
            None,
        )
        .unwrap();

        assert_eq!(provider.display_name, "My Agent");

        // Verify it's in the list
        let providers = list_agent_providers(&service).await.unwrap();
        assert!(
            providers
                .iter()
                .any(|p| p.provider_id == provider.provider_id)
        );

        // Remove it
        remove_custom_provider(&service, provider.provider_id).unwrap();

        // Verify it's gone
        let providers = list_agent_providers(&service).await.unwrap();
        assert!(!providers.iter().any(|p| p.display_name == "My Agent"));
    }

    #[test]
    fn test_provider_config_commands() {
        let (service, _temp, _bin) = create_test_service_with_bundled_opencode();

        // Get initial config for opencode (the seeded provider)
        let config = get_provider_config(&service, "opencode".to_string()).unwrap();
        assert!(config.default_model.is_none());

        // Update config
        let new_config = ProviderConfig {
            default_model: Some("gpt-4".to_string()),
            env_vars: [("KEY".to_string(), "value".to_string())].into(),
            custom_args: vec!["--test".to_string()],
        };

        update_provider_config(&service, "opencode".to_string(), new_config.clone()).unwrap();

        // Verify update
        let config = get_provider_config(&service, "opencode".to_string()).unwrap();
        assert_eq!(config.default_model, Some("gpt-4".to_string()));
    }

    #[tokio::test]
    async fn test_check_prerequisites_command() {
        let (service, _temp) = create_test_service();

        let check = check_installation_prerequisites(&service, InstallationMethod::Npx)
            .await
            .unwrap();

        // Should return a valid check result
        assert!(!check.missing_components.contains(&"Invalid".to_string()));
    }

    #[tokio::test]
    async fn native_login_launches_registry_installed_kimi_from_install_dir() {
        let (service, temp_dir) = create_test_service();
        let install_root = temp_dir
            .path()
            .join("data")
            .join("resources")
            .join("agents")
            .join("kimi");
        std::fs::create_dir_all(&install_root).unwrap();

        {
            let conn = service.test_conn();
            conn.execute(
                "INSERT INTO agent_runtimes (
                    id, runtime_type, display_name, description, command, args_json,
                    installation_method, is_bundled, is_installed, is_default,
                    status, status_message, config_json, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'binary', 0, 1, 0, 'ready', NULL, '{}', ?7, ?7)",
                params![
                    "provider_kimi",
                    "kimi",
                    "Kimi CLI",
                    "Moonshot",
                    install_root.join("kimi").to_string_lossy().to_string(),
                    "[]",
                    "2026-04-17T00:00:00Z"
                ],
            )
            .unwrap();
        }

        let action = launch_native_runtime_login(&service, "kimi".to_string())
            .await
            .expect("native login action");

        match action {
            RuntimeAuthenticationAction::LaunchTerminal(launch) => {
                assert_eq!(launch.command, install_root.join("kimi").to_string_lossy());
                assert_eq!(launch.args, vec!["login".to_string()]);
                assert_eq!(launch.cwd.as_deref(), Some(install_root.as_path()));
            }
            other => panic!("expected terminal launch, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn authentication_phase_timeout_returns_targeted_error() {
        let result = run_authentication_phase_with_timeout(
            "kimi",
            "initialize",
            Duration::from_millis(10),
            async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                anyhow::Ok(())
            },
        )
        .await;

        let error = result.expect_err("timeout expected");
        assert!(
            error
                .to_string()
                .contains("Runtime kimi authentication timed out during initialize")
        );
    }
}
