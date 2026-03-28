//! Tauri command wrappers for agent provider management
//!
//! These functions wrap the AgentProviderService to provide
//! the Tauri command interface for the frontend.

use crate::agent_provider_service::{
    AgentProviderService, InstallProviderRequest, InstallProviderResponse, InstallationMethod,
    PrerequisitesCheck, ProviderConfig, ProviderInfo, TestConnectionResult,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    service.test_connection(&provider_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_service() -> (AgentProviderService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let data_dir = temp_dir.path().join("data");

        let service = AgentProviderService::new(&db_path, data_dir).unwrap();
        (service, temp_dir)
    }

    #[tokio::test]
    async fn test_list_providers_command() {
        let (service, _temp) = create_test_service();

        let providers = list_agent_providers(&service).await.unwrap();
        assert!(!providers.is_empty());
    }

    #[tokio::test]
    async fn test_set_default_provider_command() {
        let (service, _temp) = create_test_service();

        // First mark opencode as installed
        service.test_conn().execute(
            "UPDATE agent_providers SET is_installed = 1, status = 'ready' WHERE provider_id = 'opencode'",
            [],
        ).unwrap();

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
        let (service, _temp) = create_test_service();

        // Get initial config
        let config = get_provider_config(&service, "pi-acp".to_string()).unwrap();
        assert!(config.default_model.is_none());

        // Update config
        let new_config = ProviderConfig {
            default_model: Some("gpt-4".to_string()),
            env_vars: [("KEY".to_string(), "value".to_string())].into(),
            custom_args: vec!["--test".to_string()],
        };

        update_provider_config(&service, "pi-acp".to_string(), new_config.clone()).unwrap();

        // Verify update
        let config = get_provider_config(&service, "pi-acp".to_string()).unwrap();
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
}
