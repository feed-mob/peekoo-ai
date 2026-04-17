//! Agent provider management service
//!
//! This service manages the installation, configuration, and lifecycle of
//! ACP-compatible agent providers.
//!
//! ## ACP Registry Integration
//! The ACP registry is the single source of truth for available agents.
//! The local DB (`agent_runtimes`) only contains installed agents.
//! See: crates/acp-registry-client for registry client implementation.

use anyhow;
use anyhow::Context;
use peekoo_utils::command_available;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use std::sync::{Arc, Mutex, MutexGuard};

use agent_client_protocol as acp;

use crate::runtime_adapters::adapter_for_runtime;

// Re-export registry types for convenience
use acp_registry_client::platform::{is_supported_on, preferred_method_for, supported_methods_on};
pub use acp_registry_client::types::{Agent as RegistryAgent, AvailableAgent};
pub use acp_registry_client::{InstallMethod as RegistryInstallMethod, RegistryClient};

/// Provider installation method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationMethod {
    /// Bundled with the application
    Bundled,
    /// Installed via npx (requires Node.js)
    Npx,
    /// Pre-built binary downloaded from URL
    Binary,
    /// Custom command or path
    Custom,
}

/// Provider status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    /// Not yet installed
    NotInstalled,
    /// Currently installing
    Installing,
    /// Ready to use
    Ready,
    /// Error during installation or operation
    Error,
    /// Requires additional setup (e.g., API key)
    NeedsSetup,
}

/// ACP runtime information DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub provider_id: String,
    pub display_name: String,
    pub description: String,
    pub is_bundled: bool,
    pub installation_method: InstallationMethod,
    pub is_installed: bool,
    pub is_default: bool,
    pub status: ProviderStatus,
    pub status_message: Option<String>,
    pub available_methods: Vec<InstallationMethodInfo>,
    pub config: ProviderConfig,
    /// Command to spawn this provider
    pub command: String,
    /// Arguments to pass to the command
    pub args: Vec<String>,
}

pub type RuntimeInfo = ProviderInfo;

impl RuntimeInfo {
    /// Returns true if this runtime should be visible in chat runtime selection.
    pub fn is_chat_visible(&self) -> bool {
        true
    }
}

/// Registry agent info for displaying ACP registry agents in UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryAgentInfo {
    /// The registry ID (e.g., "gemini", "cursor")
    pub registry_id: String,
    /// Display name from registry
    pub name: String,
    /// Version from registry
    pub version: String,
    /// Description from registry
    pub description: String,
    /// Authors from registry
    pub authors: Vec<String>,
    /// License from registry
    pub license: String,
    /// Website URL from registry
    pub website: Option<String>,
    /// Icon URL from registry
    pub icon_url: Option<String>,
    /// Platforms supported by this agent
    pub supported_platforms: Vec<String>,
    /// Installation methods available (as strings for frontend)
    pub supported_methods: Vec<String>,
    /// Whether this agent is supported on current platform
    pub is_supported_on_current_platform: bool,
    /// Preferred installation method (as string for frontend)
    pub preferred_method: Option<String>,
    /// Whether this agent is already installed
    pub is_installed: bool,
    /// Installed version (if installed)
    pub installed_version: Option<String>,
    /// Display order for custom sorting (built-ins first)
    pub display_order: i32,
}

/// Filter options for fetching registry agents
#[derive(Debug, Clone, Default)]
pub struct RegistryFilterOptions {
    /// Search query to filter agents by name/description
    pub search_query: Option<String>,
    /// Only show agents supported on current platform
    pub platform_only: bool,
    /// Filter by specific installation method (using registry method type)
    pub method_filter: Option<acp_registry_client::InstallMethod>,
    /// Sort order
    pub sort_by: RegistrySortBy,
    /// Page number (1-based)
    pub page: usize,
    /// Page size (default: 20)
    pub page_size: usize,
}

/// Sort options for registry agents
#[derive(Debug, Clone, Copy, Default)]
pub enum RegistrySortBy {
    /// Featured order: built-ins first, then by display_order
    #[default]
    Featured,
    /// Alphabetical by name
    Name,
    /// Platform compatibility (supported first)
    PlatformSupport,
}

/// Calculate display order for custom sorting
/// Built-ins: 0-3, Popular: 4-10, Alphabetical: 100+
pub fn calculate_display_order(registry_id: &str) -> i32 {
    match registry_id {
        "opencode" => 0,
        "codex-acp" => 1,
        "claude-acp" => 2,
        "gemini" => 3,
        "cursor" => 4,
        "goose" => 5,
        "kimi" => 6,
        "qwen-code" => 7,
        "cline" => 8,
        "auggie" => 9,
        _ => {
            // Alphabetical order for rest
            100 + registry_id.chars().next().map(|c| c as i32).unwrap_or(127)
        }
    }
}

/// Installation method information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationMethodInfo {
    pub id: InstallationMethod,
    pub name: String,
    pub description: String,
    pub is_available: bool,
    pub requires_setup: bool,
    pub size_mb: Option<f64>,
}

/// Provider configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub default_model: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub custom_args: Vec<String>,
}

pub type RuntimeConfig = ProviderConfig;

/// Request to install a provider
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProviderRequest {
    pub provider_id: String,
    pub method: InstallationMethod,
    pub custom_path: Option<String>,
}

pub type InstallRuntimeRequest = InstallProviderRequest;

/// Response from provider installation
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProviderResponse {
    pub success: bool,
    pub message: String,
    pub requires_restart: bool,
}

pub type InstallRuntimeResponse = InstallProviderResponse;

/// Test connection result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
    pub available_models: Vec<String>,
    pub provider_version: Option<String>,
}

/// Prerequisites check result
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrerequisitesCheck {
    pub available: bool,
    pub missing_components: Vec<String>,
    pub instructions: Option<String>,
}

pub type RuntimeStatus = ProviderStatus;

/// Discovered model from ACP runtime inspection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredModelInfo {
    pub model_id: String,
    pub name: String,
    pub description: Option<String>,
}

/// ACP authentication method information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthMethodInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Shell command the user can run to manually authenticate (for Terminal auth methods).
    pub manual_login_command: Option<String>,
}

/// Result of runtime inspection via ACP protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInspectionResult {
    pub runtime_id: String,
    pub auth_methods: Vec<AuthMethodInfo>,
    pub native_login_command: Option<String>,
    pub preferred_login_method: Option<crate::runtime_adapters::PreferredLoginMethod>,
    pub auth_required: bool,
    pub discovered_models: Vec<DiscoveredModelInfo>,
    pub current_model_id: Option<String>,
    pub supports_model_selection: bool,
    pub supports_config_options: bool,
    pub error: Option<String>,
}

fn test_connection_result_from_inspection(
    inspection: RuntimeInspectionResult,
) -> TestConnectionResult {
    let available_models = inspection
        .discovered_models
        .iter()
        .map(|model| model.model_id.clone())
        .collect();

    if inspection.auth_required {
        return TestConnectionResult {
            success: true,
            message: "Connection successful. Login required to start a session.".to_string(),
            available_models,
            provider_version: None,
        };
    }

    if let Some(error) = inspection.error {
        return TestConnectionResult {
            success: false,
            message: error,
            available_models: vec![],
            provider_version: None,
        };
    }

    TestConnectionResult {
        success: true,
        message: "Connection successful".to_string(),
        available_models,
        provider_version: None,
    }
}

fn cached_inspection_supports_login_preferences(
    runtime_id: &str,
    inspection: &RuntimeInspectionResult,
) -> bool {
    let expected_preference =
        crate::runtime_adapters::adapter_for_runtime(runtime_id).preferred_login_method();

    match expected_preference {
        Some(expected) => {
            inspection.preferred_login_method == Some(expected)
                && inspection.native_login_command.is_some()
        }
        None => true,
    }
}

/// Service for managing agent providers
pub struct AgentProviderService {
    conn: Arc<Mutex<Connection>>,
    data_dir: PathBuf,
    inspection_cache: Arc<Mutex<HashMap<String, RuntimeInspectionResult>>>,
    /// ACP registry client for fetching and installing agents from registry
    registry_client: Option<RegistryClient>,
    /// Node runtime for NPX-based agent installations
    node_runtime: peekoo_node_runtime::NodeRuntime,
    /// Keep the watch channel sender alive for node_runtime
    _node_options_tx: tokio::sync::watch::Sender<Option<peekoo_node_runtime::NodeBinaryOptions>>,
    /// Directory containing the bundled Node.js binary for PATH injection.
    bundled_node_bin_dir: Option<PathBuf>,
}

impl AgentProviderService {
    /// Create a new provider service
    pub fn new(db_path: &PathBuf, data_dir: PathBuf) -> anyhow::Result<Self> {
        Self::new_with_bundled_opencode(db_path, data_dir, None, None)
    }

    pub fn new_with_bundled_opencode(
        db_path: &PathBuf,
        data_dir: PathBuf,
        bundled_opencode_path: Option<PathBuf>,
        bundled_node_bin_dir: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;

        let bundled_opencode_path =
            bundled_opencode_path.filter(|path| path.exists() && path.is_file());

        Self::seed_installed_opencode(&conn, bundled_opencode_path.as_deref())?;

        // Initialize registry client (may fail if no network/cache, but service still works)
        let registry_client = RegistryClient::new().ok();

        // Initialize NodeRuntime for NPX installations
        let options = peekoo_node_runtime::NodeBinaryOptions {
            allow_path_lookup: true,
            allow_binary_download: true,
            use_paths: None,
        };
        let (tx, rx) = tokio::sync::watch::channel(Some(options));
        let node_runtime =
            peekoo_node_runtime::NodeRuntime::new(peekoo_node_runtime::HttpClient::new(), None, rx);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            data_dir,
            inspection_cache: Arc::new(Mutex::new(HashMap::new())),
            registry_client,
            node_runtime,
            _node_options_tx: tx,
            bundled_node_bin_dir,
        })
    }

    /// Create a new provider service for tests only
    #[cfg(test)]
    pub fn test_only_new(db_path: &PathBuf, data_dir: PathBuf) -> anyhow::Result<Self> {
        Self::test_only_new_with_bundled_opencode(db_path, data_dir, None)
    }

    #[cfg(test)]
    pub fn test_only_new_with_bundled_opencode(
        db_path: &PathBuf,
        data_dir: PathBuf,
        bundled_opencode_path: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;

        peekoo_persistence_sqlite::run_all_migrations(&conn)
            .map_err(|e| anyhow::anyhow!("Failed to run migrations: {e}"))?;

        let bundled_opencode_path =
            bundled_opencode_path.filter(|path| path.exists() && path.is_file());

        Self::seed_installed_opencode(&conn, bundled_opencode_path.as_deref())?;

        // Don't initialize registry client in tests to avoid network calls
        let registry_client = None;

        // Initialize NodeRuntime for NPX installations
        let options = peekoo_node_runtime::NodeBinaryOptions {
            allow_path_lookup: true,
            allow_binary_download: true,
            use_paths: None,
        };
        let (tx, rx) = tokio::sync::watch::channel(Some(options));
        let node_runtime =
            peekoo_node_runtime::NodeRuntime::new(peekoo_node_runtime::HttpClient::new(), None, rx);

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            data_dir,
            inspection_cache: Arc::new(Mutex::new(HashMap::new())),
            registry_client,
            node_runtime,
            _node_options_tx: tx,
            bundled_node_bin_dir: None,
        })
    }

    /// Get a reference to the connection for testing purposes
    ///
    /// # Warning
    /// This is intended for testing only. Direct SQL access may bypass
    /// business logic and invariants.
    #[cfg(test)]
    pub fn test_conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().expect("provider test connection lock")
    }

    fn conn(&self) -> anyhow::Result<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| anyhow::anyhow!("provider db lock poisoned: {e}"))
    }

    /// Directory containing the bundled Node.js binary, if available.
    pub fn node_bin_dir(&self) -> Option<&std::path::Path> {
        self.bundled_node_bin_dir.as_deref()
    }

    fn cached_runtime_inspection(
        &self,
        runtime_id: &str,
    ) -> anyhow::Result<Option<RuntimeInspectionResult>> {
        if let Some(cached) = self.cached_runtime_inspection_memory(runtime_id)? {
            return Ok(Some(cached));
        }

        let conn = self.conn()?;
        let inspection_json: Option<String> = conn
            .query_row(
                "SELECT inspection_json FROM agent_runtimes WHERE runtime_type = ?1",
                params![runtime_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

        let inspection = inspection_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<RuntimeInspectionResult>(json).ok());

        if let Some(ref inspection) = inspection {
            self.store_runtime_inspection_memory(inspection)?;
        }

        Ok(inspection)
    }

    fn cached_runtime_inspection_memory(
        &self,
        runtime_id: &str,
    ) -> anyhow::Result<Option<RuntimeInspectionResult>> {
        let cache = self
            .inspection_cache
            .lock()
            .map_err(|e| anyhow::anyhow!("inspection cache lock poisoned: {e}"))?;
        Ok(cache.get(runtime_id).cloned())
    }

    fn store_runtime_inspection(&self, inspection: &RuntimeInspectionResult) -> anyhow::Result<()> {
        self.store_runtime_inspection_memory(inspection)?;

        let now = chrono::Utc::now().to_rfc3339();
        let inspection_json = serde_json::to_string(inspection)?;
        let conn = self.conn()?;
        conn.execute(
            "UPDATE agent_runtimes
             SET inspection_json = ?1, inspected_at = ?2, updated_at = ?2
             WHERE runtime_type = ?3",
            params![inspection_json, &now, &inspection.runtime_id],
        )?;
        Ok(())
    }

    fn store_runtime_inspection_memory(
        &self,
        inspection: &RuntimeInspectionResult,
    ) -> anyhow::Result<()> {
        let mut cache = self
            .inspection_cache
            .lock()
            .map_err(|e| anyhow::anyhow!("inspection cache lock poisoned: {e}"))?;
        cache.insert(inspection.runtime_id.clone(), inspection.clone());
        Ok(())
    }

    pub fn invalidate_runtime_inspection_cache(&self, runtime_id: &str) -> anyhow::Result<()> {
        let mut cache = self
            .inspection_cache
            .lock()
            .map_err(|e| anyhow::anyhow!("inspection cache lock poisoned: {e}"))?;
        cache.remove(runtime_id);
        drop(cache);

        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE agent_runtimes
             SET inspection_json = NULL, inspected_at = NULL, updated_at = ?1
             WHERE runtime_type = ?2",
            params![&now, runtime_id],
        )?;
        Ok(())
    }

    /// Seed an installed opencode row only if opencode is actually present
    /// (bundled binary or on PATH). This is the only hardcoded seed — everything
    /// else comes from the ACP registry.
    fn seed_installed_opencode(
        conn: &Connection,
        bundled_opencode_path: Option<&std::path::Path>,
    ) -> anyhow::Result<()> {
        let is_bundled = bundled_opencode_path.is_some();
        let is_on_path = command_available("opencode");

        if !is_bundled && !is_on_path {
            // opencode not present — no row to seed; user installs via registry
            return Ok(());
        }

        let command = bundled_opencode_path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "opencode".to_string());
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO agent_runtimes (
                id, runtime_type, display_name, description, command, args_json,
                installation_method, is_bundled, is_installed, is_default,
                status, status_message, config_json, registry_id, created_at, updated_at
            ) VALUES (
                'provider_opencode', 'opencode', 'OpenCode', 'Open source AI coding agent with ACP support',
                ?1, '[\"acp\"]',
                ?2, ?3, 1, 1,
                'ready', NULL, '{}', 'opencode', ?4, ?4
            )
            ON CONFLICT(id) DO UPDATE SET
                command = excluded.command,
                installation_method = excluded.installation_method,
                is_bundled = excluded.is_bundled,
                updated_at = excluded.updated_at",
            params![
                command,
                if is_bundled { "bundled" } else { "binary" },
                if is_bundled { 1i64 } else { 0i64 },
                now,
            ],
        )?;

        Ok(())
    }

    /// List all providers (installed + available)
    pub fn list_providers(&self) -> anyhow::Result<Vec<ProviderInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT 
                id, runtime_type, display_name, description, is_bundled,
                installation_method, is_installed, is_default, status,
                status_message, command, args_json, config_json
            FROM agent_runtimes
            ORDER BY is_installed DESC, is_bundled DESC, display_name ASC",
        )?;

        let providers: Vec<_> = stmt
            .query_map([], |row| {
                let provider_id: String = row.get(1)?;
                let is_bundled: i64 = row.get(4)?;
                let is_installed: i64 = row.get(6)?;
                let is_default: i64 = row.get(7)?;

                let available_methods = Self::get_available_methods(&provider_id, is_bundled != 0);

                let config_json: Option<String> = row.get(12)?;
                let config: ProviderConfig = config_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                let command: String = row.get(10)?;
                let args_json: Option<String> = row.get(11)?;
                let args: Vec<String> = args_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                Ok(ProviderInfo {
                    id: row.get(0)?,
                    provider_id: provider_id.clone(),
                    display_name: row.get(2)?,
                    description: row.get(3)?,
                    is_bundled: is_bundled != 0,
                    installation_method: Self::parse_installation_method(&row.get::<_, String>(5)?),
                    is_installed: is_installed != 0,
                    is_default: is_default != 0,
                    status: Self::parse_provider_status(&row.get::<_, String>(8)?),
                    status_message: row.get(9)?,
                    available_methods,
                    config,
                    command,
                    args,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(providers)
    }

    /// List all ACP runtimes (installed + available).
    pub fn list_runtimes(&self) -> anyhow::Result<Vec<RuntimeInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT 
                id, runtime_type, display_name, description, is_bundled,
                installation_method, is_installed, is_default, status,
                status_message, command, args_json, config_json
            FROM agent_runtimes
            ORDER BY is_installed DESC, is_bundled DESC, display_name ASC",
        )?;

        let runtimes = stmt
            .query_map([], |row| {
                let runtime_type: String = row.get(1)?;
                let is_bundled: i64 = row.get(4)?;
                let is_installed: i64 = row.get(6)?;
                let is_default: i64 = row.get(7)?;
                let available_methods = Self::get_available_methods(&runtime_type, is_bundled != 0);
                let config: ProviderConfig = row
                    .get::<_, Option<String>>(12)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                let command: String = row.get(10)?;
                let args_json: Option<String> = row.get(11)?;
                let args: Vec<String> = args_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                Ok(RuntimeInfo {
                    id: row.get(0)?,
                    provider_id: runtime_type.clone(),
                    display_name: row.get(2)?,
                    description: row.get(3)?,
                    is_bundled: is_bundled != 0,
                    installation_method: Self::parse_installation_method(&row.get::<_, String>(5)?),
                    is_installed: is_installed != 0,
                    is_default: is_default != 0,
                    status: Self::parse_provider_status(&row.get::<_, String>(8)?),
                    status_message: row.get(9)?,
                    available_methods,
                    config,
                    command,
                    args,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(runtimes)
    }

    /// Get available installation methods for a provider
    fn get_available_methods(_provider_id: &str, is_bundled: bool) -> Vec<InstallationMethodInfo> {
        let mut methods = Vec::new();

        if is_bundled {
            methods.push(InstallationMethodInfo {
                id: InstallationMethod::Bundled,
                name: "Bundled".to_string(),
                description: "Pre-installed with Peekoo".to_string(),
                is_available: true,
                requires_setup: false,
                size_mb: None,
            });
        }

        // Check if Node.js is available for npx
        let has_node = command_available("node") && command_available("npm");

        methods.push(InstallationMethodInfo {
            id: InstallationMethod::Npx,
            name: "npx".to_string(),
            description: "Install via npx (requires Node.js)".to_string(),
            is_available: has_node,
            requires_setup: false,
            size_mb: None,
        });

        methods.push(InstallationMethodInfo {
            id: InstallationMethod::Binary,
            name: "Binary".to_string(),
            description: "Download pre-built binary".to_string(),
            is_available: true,
            requires_setup: false,
            size_mb: Some(25.0), // Approximate
        });

        methods.push(InstallationMethodInfo {
            id: InstallationMethod::Custom,
            name: "Custom".to_string(),
            description: "Specify your own path or command".to_string(),
            is_available: true,
            requires_setup: true,
            size_mb: None,
        });

        methods
    }

    /// Get the default provider
    pub fn get_default_provider(&self) -> anyhow::Result<Option<ProviderInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT 
                id, runtime_type, display_name, description, is_bundled,
                installation_method, is_installed, is_default, status,
                status_message, command, args_json, config_json
            FROM agent_runtimes
            WHERE is_default = 1
            LIMIT 1",
        )?;

        let provider = stmt
            .query_row([], |row| {
                let provider_id: String = row.get(1)?;
                let is_bundled: i64 = row.get(4)?;
                let is_installed: i64 = row.get(6)?;
                let is_default: i64 = row.get(7)?;

                let available_methods = Self::get_available_methods(&provider_id, is_bundled != 0);

                let config_json: Option<String> = row.get(12)?;
                let config: ProviderConfig = config_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                let command: String = row.get(10)?;
                let args_json: Option<String> = row.get(11)?;
                let args: Vec<String> = args_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                Ok(ProviderInfo {
                    id: row.get(0)?,
                    provider_id: provider_id.clone(),
                    display_name: row.get(2)?,
                    description: row.get(3)?,
                    is_bundled: is_bundled != 0,
                    installation_method: Self::parse_installation_method(&row.get::<_, String>(5)?),
                    is_installed: is_installed != 0,
                    is_default: is_default != 0,
                    status: Self::parse_provider_status(&row.get::<_, String>(8)?),
                    status_message: row.get(9)?,
                    available_methods,
                    config,
                    command,
                    args,
                })
            })
            .optional()?;

        Ok(provider)
    }

    /// Get the default ACP runtime.
    pub fn get_default_runtime(&self) -> anyhow::Result<Option<RuntimeInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT 
                id, runtime_type, display_name, description, is_bundled,
                installation_method, is_installed, is_default, status,
                status_message, command, args_json, config_json
            FROM agent_runtimes
            WHERE is_default = 1
            LIMIT 1",
        )?;

        let runtime = stmt
            .query_row([], |row| {
                let runtime_type: String = row.get(1)?;
                let is_bundled: i64 = row.get(4)?;
                let is_installed: i64 = row.get(6)?;
                let is_default: i64 = row.get(7)?;
                let available_methods = Self::get_available_methods(&runtime_type, is_bundled != 0);
                let config: ProviderConfig = row
                    .get::<_, Option<String>>(12)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                let command: String = row.get(10)?;
                let args_json: Option<String> = row.get(11)?;
                let args: Vec<String> = args_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                Ok(RuntimeInfo {
                    id: row.get(0)?,
                    provider_id: runtime_type,
                    display_name: row.get(2)?,
                    description: row.get(3)?,
                    is_bundled: is_bundled != 0,
                    installation_method: Self::parse_installation_method(&row.get::<_, String>(5)?),
                    is_installed: is_installed != 0,
                    is_default: is_default != 0,
                    status: Self::parse_provider_status(&row.get::<_, String>(8)?),
                    status_message: row.get(9)?,
                    available_methods,
                    config,
                    command,
                    args,
                })
            })
            .optional()?;

        Ok(runtime)
    }

    /// Get a specific runtime by ID.
    pub fn get_runtime(&self, runtime_id: &str) -> anyhow::Result<Option<RuntimeInfo>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT 
                id, runtime_type, display_name, description, is_bundled,
                installation_method, is_installed, is_default, status,
                status_message, command, args_json, config_json
            FROM agent_runtimes
            WHERE runtime_type = ?1
            LIMIT 1",
        )?;

        let runtime = stmt
            .query_row(params![runtime_id], |row| {
                let runtime_type: String = row.get(1)?;
                let is_bundled: i64 = row.get(4)?;
                let is_installed: i64 = row.get(6)?;
                let is_default: i64 = row.get(7)?;

                let available_methods = Self::get_available_methods(&runtime_type, is_bundled != 0);

                let config_json: Option<String> = row.get(12)?;
                let config: ProviderConfig = config_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                let command: String = row.get(10)?;
                let args_json: Option<String> = row.get(11)?;
                let args: Vec<String> = args_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                Ok(ProviderInfo {
                    id: row.get(0)?,
                    provider_id: runtime_type.clone(),
                    display_name: row.get(2)?,
                    description: row.get(3)?,
                    is_bundled: is_bundled != 0,
                    installation_method: Self::parse_installation_method(&row.get::<_, String>(5)?),
                    is_installed: is_installed != 0,
                    is_default: is_default != 0,
                    status: Self::parse_provider_status(&row.get::<_, String>(8)?),
                    status_message: row.get(9)?,
                    available_methods,
                    config,
                    command,
                    args,
                })
            })
            .optional()?;

        Ok(runtime)
    }

    /// Inspect a runtime by spawning it temporarily and querying ACP capabilities.
    /// This creates a temporary session, collects metadata, and then kills the process.
    pub async fn inspect_runtime(
        &self,
        runtime_id: &str,
    ) -> anyhow::Result<RuntimeInspectionResult> {
        self.inspect_runtime_with_cache(runtime_id, true).await
    }

    pub async fn refresh_runtime_capabilities(
        &self,
        runtime_id: &str,
    ) -> anyhow::Result<RuntimeInspectionResult> {
        self.inspect_runtime_with_cache(runtime_id, false).await
    }

    async fn inspect_runtime_with_cache(
        &self,
        runtime_id: &str,
        use_cache: bool,
    ) -> anyhow::Result<RuntimeInspectionResult> {
        use peekoo_agent::backend::acp::is_auth_required_error;
        use peekoo_agent::backend::{AcpBackend, AgentBackend, BackendConfig};

        if use_cache
            && let Some(cached) = self.cached_runtime_inspection(runtime_id)?
            && cached_inspection_supports_login_preferences(runtime_id, &cached)
        {
            return Ok(cached);
        }

        // Get runtime info
        let runtime = self.get_runtime(runtime_id)?;
        let runtime =
            runtime.ok_or_else(|| anyhow::anyhow!("Runtime not found: {}", runtime_id))?;

        // Get runtime configuration
        let config = self.get_provider_config(runtime_id)?;

        // Build the command from runtime info.
        let (command, args) = if runtime.is_installed {
            // Get the actual command from runtime metadata
            self.get_runtime_command(runtime_id).await?
        } else {
            return Ok(RuntimeInspectionResult {
                runtime_id: runtime_id.to_string(),
                auth_methods: vec![],
                native_login_command: None,
                preferred_login_method: None,
                auth_required: false,
                discovered_models: vec![],
                current_model_id: None,
                supports_model_selection: false,
                supports_config_options: false,
                error: Some("Runtime not installed".to_string()),
            });
        };

        // Create ACP backend for temporary inspection
        let manual_login_command = command.clone();
        let manual_login_args = args.clone();
        let install_dir = self.runtime_install_dir(runtime_id);
        let inspection_adapter = crate::runtime_adapters::adapter_for_runtime(runtime_id);
        let native_login_command = inspection_adapter.build_manual_native_login_command(
            &manual_login_command,
            &manual_login_args,
            &install_dir,
        );
        let preferred_login_method = inspection_adapter.preferred_login_method();
        let mut backend = AcpBackend::new(command, args);

        // Prepare backend config
        let backend_config = BackendConfig {
            working_directory: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from(".")),
            system_prompt: None,
            model: config.default_model.clone(),
            provider: Some(runtime_id.to_string()),
            api_key: None,
            environment: crate::runtime_adapters::adapter_for_runtime(runtime_id)
                .build_launch_env(&config, self.bundled_node_bin_dir.as_deref()),
            mcp_servers: Vec::new(),
        };

        // Initialize the backend (this spawns the process and does ACP initialize)
        match tokio::time::timeout(
            std::time::Duration::from_secs(15),
            backend.initialize(backend_config),
        )
        .await
        {
            Ok(Ok(())) => {
                let adapter = adapter_for_runtime(runtime_id);

                // Extract discovered information
                let auth_methods: Vec<AuthMethodInfo> = backend
                    .auth_methods()
                    .iter()
                    .map(|m| {
                        let manual_login_command = match m {
                            acp::AuthMethod::Terminal(terminal) => adapter
                                .build_manual_login_command(
                                    &manual_login_command,
                                    &manual_login_args,
                                    &terminal.args,
                                ),
                            _ => None,
                        };
                        AuthMethodInfo {
                            id: m.id().to_string(),
                            name: m.name().to_string(),
                            description: m.description().map(|s| s.to_string()),
                            manual_login_command,
                        }
                    })
                    .collect();

                let discovered_models: Vec<DiscoveredModelInfo> = backend
                    .discovered_models()
                    .iter()
                    .map(|m| DiscoveredModelInfo {
                        model_id: m.model_id.clone(),
                        name: m.name.clone(),
                        description: m.description.clone(),
                    })
                    .collect();

                let supports_model_selection = !discovered_models.is_empty();
                let supports_config_options = !discovered_models.is_empty();
                let auth_required = backend.is_auth_required();
                let current_model_id = backend.current_model_id().map(|s| s.to_string());

                let _ = backend.shutdown().await;

                let inspection = RuntimeInspectionResult {
                    runtime_id: runtime_id.to_string(),
                    auth_methods,
                    native_login_command: native_login_command.clone(),
                    preferred_login_method,
                    auth_required,
                    discovered_models,
                    current_model_id,
                    supports_model_selection,
                    supports_config_options,
                    error: None,
                };

                self.store_runtime_inspection(&inspection)?;
                Ok(inspection)
            }
            Ok(Err(e)) => {
                let _ = backend.shutdown().await;
                let error_msg = e.to_string();
                let auth_required = is_auth_required_error(&e);
                let adapter = adapter_for_runtime(runtime_id);
                let auth_methods: Vec<AuthMethodInfo> = backend
                    .auth_methods()
                    .iter()
                    .map(|m| {
                        let manual_login_command = match m {
                            acp::AuthMethod::Terminal(terminal) => adapter
                                .build_manual_login_command(
                                    &manual_login_command,
                                    &manual_login_args,
                                    &terminal.args,
                                ),
                            _ => None,
                        };
                        AuthMethodInfo {
                            id: m.id().to_string(),
                            name: m.name().to_string(),
                            description: m.description().map(|s| s.to_string()),
                            manual_login_command,
                        }
                    })
                    .collect();

                let inspection = RuntimeInspectionResult {
                    runtime_id: runtime_id.to_string(),
                    auth_methods,
                    native_login_command: native_login_command.clone(),
                    preferred_login_method,
                    auth_required,
                    discovered_models: vec![],
                    current_model_id: None,
                    supports_model_selection: false,
                    supports_config_options: false,
                    error: Some(error_msg),
                };

                if inspection.auth_required {
                    self.store_runtime_inspection(&inspection)?;
                }

                Ok(inspection)
            }
            Err(_) => {
                let _ = backend.shutdown().await;
                Ok(RuntimeInspectionResult {
                    runtime_id: runtime_id.to_string(),
                    auth_methods: vec![],
                    native_login_command,
                    preferred_login_method,
                    auth_required: false,
                    discovered_models: vec![],
                    current_model_id: None,
                    supports_model_selection: false,
                    supports_config_options: false,
                    error: Some("Runtime inspection timed out".to_string()),
                })
            }
        }
    }

    /// Get the command and args for a runtime (helper for inspection)
    pub async fn get_runtime_command(
        &self,
        runtime_id: &str,
    ) -> anyhow::Result<(String, Vec<String>)> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT command, args_json FROM agent_runtimes WHERE runtime_type = ?1 LIMIT 1",
        )?;

        let row = stmt
            .query_row(params![runtime_id], |row| {
                let command: Option<String> = row.get(0)?;
                let args_json: Option<String> = row.get(1)?;
                Ok((command, args_json))
            })
            .optional()?;

        if let Some((Some(command), args_json)) = row {
            let args: Vec<String> = args_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            Ok((command, args))
        } else {
            Err(anyhow::anyhow!(
                "Runtime command not configured for: {}",
                runtime_id
            ))
        }
    }

    pub fn runtime_install_dir(&self, runtime_id: &str) -> std::path::PathBuf {
        self.data_dir
            .join("resources")
            .join("agents")
            .join(runtime_id)
    }

    /// Set the default provider
    pub fn set_default_provider(&self, provider_id: &str) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;

        // Clear existing default
        conn.execute(
            "UPDATE agent_runtimes SET is_default = 0, updated_at = ?1",
            params![&now],
        )?;

        // Set new default
        conn.execute(
            "UPDATE agent_runtimes SET is_default = 1, updated_at = ?1 WHERE runtime_type = ?2",
            params![&now, provider_id],
        )?;

        Ok(())
    }

    /// Set the default ACP runtime.
    pub fn set_default_runtime(&self, runtime_id: &str) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE agent_runtimes SET is_default = 0, updated_at = ?1",
            params![&now],
        )?;
        conn.execute(
            "UPDATE agent_runtimes SET is_default = 1, updated_at = ?1 WHERE runtime_type = ?2",
            params![&now, runtime_id],
        )?;
        self.set_default_provider(runtime_id)
    }

    /// Install a provider
    pub fn install_provider(
        &self,
        req: InstallProviderRequest,
    ) -> anyhow::Result<InstallProviderResponse> {
        let now = chrono::Utc::now().to_rfc3339();

        // Update provider to installing status
        {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE agent_runtimes SET status = 'installing', updated_at = ?1 WHERE runtime_type = ?2",
                params![&now, &req.provider_id],
            )?;
        }

        // Perform installation based on method
        let result = match req.method {
            InstallationMethod::Bundled => {
                // Nothing to do for bundled installs.
                Ok(())
            }
            InstallationMethod::Npx => {
                // NPX packages are resolved at runtime via npx.
                // No pre-installation verification needed.
                Ok(())
            }
            InstallationMethod::Binary => {
                // Download binary
                Self::download_provider_binary(&self.data_dir, &req.provider_id)
            }
            InstallationMethod::Custom => {
                // Validate custom path
                if let Some(ref path) = req.custom_path {
                    if !std::path::Path::new(path).exists() {
                        return Err(anyhow::anyhow!("Custom path does not exist: {}", path));
                    }
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Custom path is required for custom installation"
                    ))
                }
            }
        };

        // Update status based on result
        let (success, message, requires_restart) = match result {
            Ok(()) => {
                let conn = self.conn()?;
                conn.execute(
                    "UPDATE agent_runtimes SET 
                        is_installed = 1, 
                        status = 'ready',
                        status_message = NULL,
                        updated_at = ?1
                    WHERE runtime_type = ?2",
                    params![&now, &req.provider_id],
                )?;
                (
                    true,
                    format!("{} installed successfully", req.provider_id),
                    false,
                )
            }
            Err(e) => {
                let conn = self.conn()?;
                conn.execute(
                    "UPDATE agent_runtimes SET 
                        status = 'error',
                        status_message = ?1,
                        updated_at = ?2
                    WHERE runtime_type = ?3",
                    params![&e.to_string(), &now, &req.provider_id],
                )?;
                (false, e.to_string(), false)
            }
        };

        self.invalidate_runtime_inspection_cache(&req.provider_id)?;

        Ok(InstallProviderResponse {
            success,
            message,
            requires_restart,
        })
    }

    /// Install an ACP runtime.
    pub fn install_runtime(
        &self,
        req: InstallRuntimeRequest,
    ) -> anyhow::Result<InstallRuntimeResponse> {
        let response = self.install_provider(req.clone())?;
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE agent_runtimes
             SET is_installed = ?1, status = ?2, status_message = ?3, updated_at = ?4
             WHERE runtime_type = ?5",
            params![
                if response.success { 1 } else { 0 },
                if response.success { "ready" } else { "error" },
                if response.success {
                    None::<String>
                } else {
                    Some(response.message.clone())
                },
                &now,
                &req.provider_id,
            ],
        )?;
        Ok(response)
    }

    /// Download provider binary
    fn download_provider_binary(data_dir: &Path, provider_id: &str) -> anyhow::Result<()> {
        // In production, this would download from a release URL
        // For now, just verify the directory exists
        let provider_dir = data_dir.join("providers").join(provider_id);
        std::fs::create_dir_all(&provider_dir)?;

        tracing::info!("Provider binary directory created: {:?}", provider_dir);

        // TODO: Actually download the binary
        // This is a placeholder for the real implementation

        Ok(())
    }

    /// Uninstall a provider
    pub fn uninstall_provider(&self, provider_id: &str) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        {
            let conn = self.conn()?;

            // Check if it's the default
            let is_default: i64 = conn.query_row(
                "SELECT is_default FROM agent_runtimes WHERE runtime_type = ?1",
                params![provider_id],
                |row| row.get(0),
            )?;

            if is_default != 0 {
                return Err(anyhow::anyhow!(
                    "Cannot uninstall the default runtime. Please set a different runtime as default first."
                ));
            }

            // Update status
            conn.execute(
                "UPDATE agent_runtimes SET 
                    is_installed = 0,
                    status = 'not_installed',
                    status_message = NULL,
                    updated_at = ?1
                WHERE runtime_type = ?2",
                params![&now, provider_id],
            )?;
        }

        self.invalidate_runtime_inspection_cache(provider_id)?;

        Ok(())
    }

    /// Uninstall an ACP runtime.
    pub fn uninstall_runtime(&self, runtime_id: &str) -> anyhow::Result<()> {
        self.uninstall_provider(runtime_id)?;
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "UPDATE agent_runtimes
             SET is_installed = 0, status = 'not_installed', status_message = NULL, updated_at = ?1
             WHERE runtime_type = ?2",
            params![&now, runtime_id],
        )?;
        Ok(())
    }

    /// Get provider configuration
    pub fn get_provider_config(&self, provider_id: &str) -> anyhow::Result<ProviderConfig> {
        let conn = self.conn()?;
        let config_json: Option<String> = conn.query_row(
            "SELECT config_json FROM agent_runtimes WHERE runtime_type = ?1",
            params![provider_id],
            |row| row.get(0),
        )?;

        let config: ProviderConfig = config_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        Ok(config)
    }

    /// Update provider configuration
    pub fn update_provider_config(
        &self,
        provider_id: &str,
        config: &ProviderConfig,
    ) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let config_json = serde_json::to_string(config)?;
        {
            let conn = self.conn()?;
            conn.execute(
                "UPDATE agent_runtimes SET config_json = ?1, updated_at = ?2 WHERE runtime_type = ?3",
                params![config_json, &now, provider_id],
            )?;
        }

        self.invalidate_runtime_inspection_cache(provider_id)?;

        Ok(())
    }

    /// Test provider connection
    pub async fn test_connection(&self, provider_id: &str) -> anyhow::Result<TestConnectionResult> {
        let provider = {
            let conn = self.conn()?;
            conn.query_row(
                "SELECT command, args_json, is_installed, status FROM agent_runtimes WHERE runtime_type = ?1",
                params![provider_id],
                |row| {
                    let command: String = row.get(0)?;
                    let args_json: String = row.get(1)?;
                    let is_installed: i64 = row.get(2)?;
                    let status: String = row.get(3)?;
                    Ok((command, args_json, is_installed != 0, status))
                },
            )
            .optional()?
        };

        match provider {
            Some((_, _, false, status)) => {
                // Return the actual error message if installation failed
                let message = if status == "error" {
                    let conn = self.conn()?;
                    let status_msg: Option<String> = conn
                        .query_row(
                            "SELECT status_message FROM agent_runtimes WHERE runtime_type = ?1",
                            params![provider_id],
                            |row| row.get(0),
                        )
                        .optional()?
                        .flatten();
                    status_msg.unwrap_or_else(|| "Provider installation failed".to_string())
                } else {
                    "Provider is not installed".to_string()
                };
                Ok(TestConnectionResult {
                    success: false,
                    message,
                    available_models: vec![],
                    provider_version: None,
                })
            }
            Some((_command, _args_json, true, status)) if status == "ready" => {
                let inspection = self.inspect_runtime(provider_id).await?;
                Ok(test_connection_result_from_inspection(inspection))
            }
            _ => Ok(TestConnectionResult {
                success: false,
                message: "Provider not found".to_string(),
                available_models: vec![],
                provider_version: None,
            }),
        }
    }

    /// Check installation prerequisites
    pub fn check_prerequisites(
        &self,
        method: InstallationMethod,
    ) -> anyhow::Result<PrerequisitesCheck> {
        match method {
            InstallationMethod::Npx => {
                let has_node = command_available("node");
                let has_npm = command_available("npm");

                if has_node && has_npm {
                    Ok(PrerequisitesCheck {
                        available: true,
                        missing_components: vec![],
                        instructions: None,
                    })
                } else {
                    let mut missing = Vec::new();
                    if !has_node {
                        missing.push("Node.js".to_string());
                    }
                    if !has_npm {
                        missing.push("npm".to_string());
                    }

                    Ok(PrerequisitesCheck {
                        available: false,
                        missing_components: missing,
                        instructions: Some(
                            "Please install Node.js from https://nodejs.org/".to_string(),
                        ),
                    })
                }
            }
            InstallationMethod::Binary => {
                // Binary download just needs internet
                Ok(PrerequisitesCheck {
                    available: true,
                    missing_components: vec![],
                    instructions: None,
                })
            }
            InstallationMethod::Custom => {
                // Custom installation requires user to provide valid path
                Ok(PrerequisitesCheck {
                    available: true,
                    missing_components: vec![],
                    instructions: Some(
                        "Please provide a valid path to the agent binary".to_string(),
                    ),
                })
            }
            InstallationMethod::Bundled => Ok(PrerequisitesCheck {
                available: true,
                missing_components: vec![],
                instructions: None,
            }),
        }
    }

    /// Add a custom provider
    pub fn add_custom_provider(
        &self,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
        _working_dir: Option<&str>,
    ) -> anyhow::Result<ProviderInfo> {
        let id = format!("provider_custom_{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();
        let args_json = serde_json::to_string(args)?;

        let provider = {
            let conn = self.conn()?;

            conn.execute(
                "INSERT INTO agent_runtimes (
                    id, runtime_type, display_name, description, command, args_json,
                    installation_method, is_bundled, is_installed, is_default,
                    status, status_message, config_json, created_at, updated_at
                ) VALUES (?1, ?1, ?2, ?3, ?4, ?5, 'custom', 0, 1, 0, 'ready', NULL, '{}', ?6, ?6)",
                params![
                    id,
                    name,
                    description.unwrap_or("Custom ACP agent"),
                    command,
                    args_json,
                    now
                ],
            )?;

            // Return the new provider info
            let mut stmt = conn.prepare(
                "SELECT 
                    id, runtime_type, display_name, description, is_bundled,
                    installation_method, is_installed, is_default, status,
                    status_message, command, args_json, config_json
                FROM agent_runtimes
                WHERE id = ?1",
            )?;

            stmt.query_row(params![&id], |row| {
                let provider_id: String = row.get(1)?;
                let is_bundled: i64 = row.get(4)?;
                let is_installed: i64 = row.get(6)?;
                let is_default: i64 = row.get(7)?;

                let available_methods = vec![InstallationMethodInfo {
                    id: InstallationMethod::Custom,
                    name: "Custom".to_string(),
                    description: "User-provided binary or command".to_string(),
                    is_available: true,
                    requires_setup: true,
                    size_mb: None,
                }];

                let config: ProviderConfig = row
                    .get::<_, Option<String>>(12)?
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                let command: String = row.get(10)?;
                let args_json: Option<String> = row.get(11)?;
                let args: Vec<String> = args_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                Ok(ProviderInfo {
                    id: row.get(0)?,
                    provider_id: provider_id.clone(),
                    display_name: row.get(2)?,
                    description: row.get(3)?,
                    is_bundled: is_bundled != 0,
                    installation_method: InstallationMethod::Custom,
                    is_installed: is_installed != 0,
                    is_default: is_default != 0,
                    status: ProviderStatus::Ready,
                    status_message: row.get(9)?,
                    available_methods,
                    config,
                    command,
                    args,
                })
            })?
        };

        self.invalidate_runtime_inspection_cache(&id)?;

        Ok(provider)
    }

    /// Remove a custom provider
    pub fn remove_custom_provider(&self, provider_id: &str) -> anyhow::Result<()> {
        {
            let conn = self.conn()?;

            // Bundled agents cannot be removed
            let is_bundled: i64 = conn
                .query_row(
                    "SELECT is_bundled FROM agent_runtimes WHERE runtime_type = ?1 OR id = ?1",
                    params![provider_id],
                    |row| row.get(0),
                )
                .optional()?
                .unwrap_or(0);

            if is_bundled != 0 {
                return Err(anyhow::anyhow!("Cannot remove a bundled provider"));
            }

            // Check if it's the default
            let is_default: i64 = conn.query_row(
                "SELECT is_default FROM agent_runtimes WHERE runtime_type = ?1",
                params![provider_id],
                |row| row.get(0),
            )?;

            if is_default != 0 {
                return Err(anyhow::anyhow!(
                    "Cannot remove the default provider. Please set a different provider as default first."
                ));
            }

            conn.execute(
                "DELETE FROM agent_runtimes WHERE runtime_type = ?1 OR id = ?1",
                params![provider_id],
            )?;
        }

        self.invalidate_runtime_inspection_cache(provider_id)?;

        Ok(())
    }

    // =========================================================================
    // ACP Registry Integration Methods
    // =========================================================================

    /// Fetch agents from ACP registry with filtering and pagination
    ///
    /// Returns (agents, total_count) where total_count is the total number
    /// of agents matching the filter (for pagination UI)
    pub async fn fetch_registry_agents(
        &self,
        filter: &RegistryFilterOptions,
    ) -> anyhow::Result<(Vec<RegistryAgentInfo>, usize)> {
        // Check if registry client is available
        let registry_client = match &self.registry_client {
            Some(client) => client,
            None => {
                return Err(anyhow::anyhow!(
                    "Registry client not available. Cannot fetch agents from ACP registry."
                ));
            }
        };

        // Fetch registry from CDN/cache
        let registry = registry_client
            .fetch()
            .await
            .context("Failed to fetch ACP registry")?;

        // Get current platform
        let platform = acp_registry_client::current_platform();

        // Convert registry agents to RegistryAgentInfo
        let mut agents: Vec<RegistryAgentInfo> = registry
            .agents
            .into_iter()
            .filter_map(|agent| {
                // Check platform support
                let supported_methods = supported_methods_on(&agent, &platform);
                let is_supported = is_supported_on(&agent, &platform);
                let preferred_method = preferred_method_for(&agent, &platform);

                // Filter by platform if requested
                if filter.platform_only && !is_supported {
                    return None;
                }

                // Filter by method if requested
                if let Some(method_filter) = filter.method_filter
                    && !supported_methods.contains(&method_filter)
                {
                    return None;
                }

                // Search filter
                if let Some(query) = &filter.search_query {
                    let query_lower = query.to_lowercase();
                    let matches_search = agent.name.to_lowercase().contains(&query_lower)
                        || agent.id.to_lowercase().contains(&query_lower)
                        || agent.description.to_lowercase().contains(&query_lower);
                    if !matches_search {
                        return None;
                    }
                }

                // Get supported platforms
                let supported_platforms = agent
                    .distribution
                    .binary
                    .as_ref()
                    .map(|b| b.keys().cloned().collect())
                    .unwrap_or_default();

                // Convert methods to strings
                let supported_methods: Vec<String> = supported_methods
                    .iter()
                    .map(|m| format!("{:?}", m).to_lowercase())
                    .collect();
                let preferred_method = preferred_method.map(|m| format!("{:?}", m).to_lowercase());

                // Check if already installed
                let is_installed = self.is_registry_agent_installed(&agent.id).unwrap_or(false);
                let installed_version = if is_installed {
                    self.get_registry_agent_version(&agent.id).ok()
                } else {
                    None
                };

                // Calculate display order
                let display_order = calculate_display_order(&agent.id);

                Some(RegistryAgentInfo {
                    registry_id: agent.id,
                    name: agent.name,
                    version: agent.version,
                    description: agent.description,
                    authors: agent.authors,
                    license: agent.license,
                    website: agent.website,
                    icon_url: agent.icon,
                    supported_platforms,
                    supported_methods,
                    is_supported_on_current_platform: is_supported,
                    preferred_method,
                    is_installed,
                    installed_version,
                    display_order,
                })
            })
            .collect();

        // Sort
        match filter.sort_by {
            RegistrySortBy::Featured => {
                // Sort by display_order, then by name
                agents.sort_by(|a, b| {
                    a.display_order
                        .cmp(&b.display_order)
                        .then_with(|| a.name.cmp(&b.name))
                });
            }
            RegistrySortBy::Name => {
                agents.sort_by(|a, b| a.name.cmp(&b.name));
            }
            RegistrySortBy::PlatformSupport => {
                // Supported agents first, then by display_order
                agents.sort_by(|a, b| {
                    let a_support = if a.is_supported_on_current_platform {
                        0
                    } else {
                        1
                    };
                    let b_support = if b.is_supported_on_current_platform {
                        0
                    } else {
                        1
                    };
                    a_support
                        .cmp(&b_support)
                        .then_with(|| a.display_order.cmp(&b.display_order))
                });
            }
        }

        // Get total count before pagination
        let total_count = agents.len();

        // Apply pagination
        let start = (filter.page - 1) * filter.page_size;
        let end = std::cmp::min(start + filter.page_size, agents.len());
        let paginated: Vec<_> = if start < agents.len() {
            agents[start..end].to_vec()
        } else {
            vec![]
        };

        Ok((paginated, total_count))
    }

    /// Search registry agents by query
    pub async fn search_registry_agents(
        &self,
        query: &str,
    ) -> anyhow::Result<Vec<RegistryAgentInfo>> {
        let filter = RegistryFilterOptions {
            search_query: Some(query.to_string()),
            platform_only: false,
            sort_by: RegistrySortBy::Featured,
            page: 1,
            page_size: 100, // Large page size for search results
            ..Default::default()
        };

        let (agents, _) = self.fetch_registry_agents(&filter).await?;
        Ok(agents)
    }

    /// Force refresh registry from CDN (ignores cache)
    pub async fn refresh_registry(&self) -> anyhow::Result<()> {
        let registry_client = match &self.registry_client {
            Some(client) => client,
            None => {
                return Err(anyhow::anyhow!("Registry client not available"));
            }
        };

        registry_client
            .refresh()
            .await
            .context("Failed to refresh registry from CDN")?;

        Ok(())
    }

    /// Install an agent from ACP registry
    pub async fn install_registry_agent(
        &self,
        registry_id: &str,
        method: InstallationMethod,
    ) -> anyhow::Result<InstallProviderResponse> {
        // Fetch agent info from registry
        let registry_client = match &self.registry_client {
            Some(client) => client,
            None => {
                return Err(anyhow::anyhow!("Registry client not available"));
            }
        };

        let registry = registry_client
            .fetch()
            .await
            .context("Failed to fetch registry")?;

        let agent = registry
            .agents
            .iter()
            .find(|a| a.id == registry_id)
            .ok_or_else(|| anyhow::anyhow!("Agent {} not found in registry", registry_id))?;

        // Check if agent is supported on current platform
        let platform = acp_registry_client::current_platform();
        if !is_supported_on(agent, platform.as_str()) {
            return Err(anyhow::anyhow!(
                "Agent {} is not supported on platform {}",
                registry_id,
                platform
            ));
        }

        // Create install directory
        let install_dir = peekoo_paths::peekoo_global_data_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get data dir: {}", e))?
            .join("resources")
            .join("agents")
            .join(registry_id);

        // Install based on method
        match method {
            InstallationMethod::Npx => {
                // Verify the agent has an NPX distribution
                agent.distribution.npx.as_ref().ok_or_else(|| {
                    anyhow::anyhow!("Agent {} has no NPX distribution", registry_id)
                })?;

                let install_config = acp_registry_client::InstallConfig {
                    agent: agent.clone(),
                    method: Some(acp_registry_client::InstallMethod::Npx),
                    install_dir,
                };

                let installation =
                    acp_registry_client::install(install_config, Some(&self.node_runtime))
                        .await
                        .context("Failed to install NPX agent")?;

                self.create_runtime_from_registry(
                    agent,
                    method,
                    &installation.executable_path,
                    &installation.command,
                )?;

                Ok(InstallProviderResponse {
                    success: true,
                    message: format!("Successfully installed {} ({})", agent.name, agent.version),
                    requires_restart: false,
                })
            }
            InstallationMethod::Binary => {
                // Get binary platform info
                agent
                    .distribution
                    .binary
                    .as_ref()
                    .and_then(|b| b.get(&platform))
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "No binary distribution for agent {} on platform {}",
                            registry_id,
                            platform
                        )
                    })?;

                // Download and extract
                let install_config = acp_registry_client::InstallConfig {
                    agent: agent.clone(),
                    method: Some(acp_registry_client::InstallMethod::Binary),
                    install_dir,
                };

                let installation = acp_registry_client::install(install_config, None)
                    .await
                    .context("Failed to install binary agent")?;

                // Create runtime entry in database
                self.create_runtime_from_registry(
                    agent,
                    method,
                    &installation.executable_path,
                    &installation.command,
                )?;

                Ok(InstallProviderResponse {
                    success: true,
                    message: format!("Successfully installed {} ({})", agent.name, agent.version),
                    requires_restart: false,
                })
            }
            _ => Err(anyhow::anyhow!(
                "Installation method {:?} not supported for registry agents",
                method
            )),
        }
    }

    // Helper function to check if registry agent is installed
    fn is_registry_agent_installed(&self, registry_id: &str) -> anyhow::Result<bool> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM agent_runtimes WHERE registry_id = ?1 AND is_installed = 1",
            params![registry_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    // Helper function to get installed version
    fn get_registry_agent_version(&self, registry_id: &str) -> anyhow::Result<String> {
        let conn = self.conn()?;
        let version: String = conn.query_row(
            "SELECT registry_version FROM agent_runtimes WHERE registry_id = ?1",
            params![registry_id],
            |row| row.get(0),
        )?;
        Ok(version)
    }

    // Helper function to create runtime entry from registry
    fn create_runtime_from_registry(
        &self,
        agent: &acp_registry_client::types::Agent,
        method: InstallationMethod,
        executable_path: &std::path::Path,
        command: &[String],
    ) -> anyhow::Result<()> {
        let conn = self.conn()?;
        let now = chrono::Utc::now().to_rfc3339();

        let runtime_id = format!("provider_{}", agent.id);
        let command_str = command
            .first()
            .map(|s| s.as_str())
            .unwrap_or(executable_path.to_str().unwrap_or(""));
        let args: Vec<String> = command.iter().skip(1).cloned().collect();

        conn.execute(
            "INSERT OR REPLACE INTO agent_runtimes (
                id, runtime_type, display_name, description, command, args_json,
                installation_method, is_bundled, is_installed, is_default,
                status, status_message, config_json,
                registry_id, registry_version,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 1, 0, 'ready', NULL, '{}', ?8, ?9, ?10, ?10)",
            params![
                runtime_id,
                agent.id,
                agent.name,
                agent.description,
                command_str,
                serde_json::to_string(&args)?,
                match method {
                    InstallationMethod::Npx => "npx",
                    InstallationMethod::Binary => "binary",
                    _ => "custom",
                },
                agent.id,
                agent.version,
                now,
            ],
        )?;

        Ok(())
    }

    // Helper functions

    fn parse_installation_method(s: &str) -> InstallationMethod {
        match s {
            "bundled" => InstallationMethod::Bundled,
            "npx" => InstallationMethod::Npx,
            "binary" => InstallationMethod::Binary,
            _ => InstallationMethod::Custom,
        }
    }

    fn parse_provider_status(s: &str) -> ProviderStatus {
        match s {
            "not_installed" => ProviderStatus::NotInstalled,
            "installing" => ProviderStatus::Installing,
            "ready" => ProviderStatus::Ready,
            "error" => ProviderStatus::Error,
            "needs_setup" => ProviderStatus::NeedsSetup,
            _ => ProviderStatus::NotInstalled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_service() -> (AgentProviderService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_providers.db");
        let data_dir = temp_dir.path().join("data");

        let service = AgentProviderService::test_only_new(&db_path, data_dir).unwrap();
        (service, temp_dir)
    }

    fn create_test_service_with_bundled_opencode() -> (AgentProviderService, TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_providers.db");
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

    #[test]
    fn test_list_providers() {
        let (service, _temp) = create_test_service();
        let opencode_available = command_available("opencode");

        let providers = service.list_providers().unwrap();

        if opencode_available {
            // opencode row seeded because it's on PATH
            let opencode = providers
                .iter()
                .find(|p| p.provider_id == "opencode")
                .unwrap();
            assert!(opencode.is_installed);
            assert!(opencode.is_default);
            assert!(!opencode.is_bundled);
        } else {
            // no rows seeded — DB is empty
            assert!(providers.is_empty());
        }
    }

    #[test]
    fn test_get_default_provider() {
        let (service, _temp) = create_test_service();
        let opencode_available = command_available("opencode");

        let default = service.get_default_provider().unwrap();
        if opencode_available {
            assert!(default.is_some());
            assert_eq!(default.unwrap().provider_id, "opencode");
        } else {
            assert!(default.is_none());
        }
    }

    #[test]
    fn test_list_runtimes_uses_runtime_table() {
        let (service, _temp) = create_test_service();
        let opencode_available = command_available("opencode");

        let runtimes = service.list_runtimes().unwrap();
        let runtime_rows: i64 = service
            .test_conn()
            .query_row("SELECT COUNT(*) FROM agent_runtimes", [], |row| row.get(0))
            .unwrap();

        if opencode_available {
            assert_eq!(runtimes.len(), 1);
            assert_eq!(runtime_rows, 1);
            assert_eq!(runtimes[0].provider_id, "opencode");
        } else {
            assert_eq!(runtimes.len(), 0);
            assert_eq!(runtime_rows, 0);
        }
    }

    #[test]
    fn test_set_default_provider() {
        let (service, _temp, _bin) = create_test_service_with_bundled_opencode();

        // Add a second provider to switch default to
        let custom = service
            .add_custom_provider("Other Agent", None, "/tmp/other", &[], None)
            .unwrap();

        // Switch default to custom
        service.set_default_provider(&custom.provider_id).unwrap();
        let default = service.get_default_provider().unwrap().unwrap();
        assert_eq!(default.provider_id, custom.provider_id);

        // opencode is no longer default
        let providers = service.list_providers().unwrap();
        let opencode = providers
            .iter()
            .find(|p| p.provider_id == "opencode")
            .unwrap();
        assert!(!opencode.is_default);
    }

    #[test]
    fn test_get_provider_config() {
        let (service, _temp, _bin) = create_test_service_with_bundled_opencode();

        let config = service.get_provider_config("opencode").unwrap();
        assert!(config.default_model.is_none());
    }

    #[test]
    fn test_update_provider_config() {
        let (service, _temp, _bin) = create_test_service_with_bundled_opencode();

        let new_config = ProviderConfig {
            default_model: Some("claude-3.5-sonnet".to_string()),
            env_vars: [("API_KEY".to_string(), "secret".to_string())].into(),
            custom_args: vec!["--verbose".to_string()],
        };

        service
            .update_provider_config("opencode", &new_config)
            .unwrap();

        let config = service.get_provider_config("opencode").unwrap();
        assert_eq!(config.default_model, Some("claude-3.5-sonnet".to_string()));
        assert_eq!(config.env_vars.get("API_KEY"), Some(&"secret".to_string()));
        assert_eq!(config.custom_args, vec!["--verbose"]);
    }

    #[tokio::test]
    async fn test_add_custom_provider() {
        let (service, _temp) = create_test_service();

        let provider = service
            .add_custom_provider(
                "My Custom Agent",
                Some("A custom ACP agent for testing"),
                "/usr/local/bin/my-agent",
                &["--acp".to_string()],
                None,
            )
            .unwrap();

        assert_eq!(provider.display_name, "My Custom Agent");
        assert!(provider.is_installed);
        assert!(!provider.is_bundled);
        assert!(matches!(
            provider.installation_method,
            InstallationMethod::Custom
        ));

        // Should appear in list
        let providers = service.list_providers().unwrap();
        assert!(
            providers
                .iter()
                .any(|p| p.display_name == "My Custom Agent")
        );

        let runtime_count: i64 = service
            .test_conn()
            .query_row(
                "SELECT COUNT(*) FROM agent_runtimes WHERE id = ?1",
                params![&provider.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(runtime_count, 1);
    }

    #[tokio::test]
    async fn test_remove_custom_provider() {
        let (service, _temp) = create_test_service();

        // Add a custom provider
        let provider = service
            .add_custom_provider("To Be Removed", None, "/tmp/agent", &[], None)
            .unwrap();

        // Remove it
        service
            .remove_custom_provider(&provider.provider_id)
            .unwrap();

        // Should no longer be in list
        let providers = service.list_providers().unwrap();
        assert!(!providers.iter().any(|p| p.display_name == "To Be Removed"));

        let runtime_count: i64 = service
            .test_conn()
            .query_row(
                "SELECT COUNT(*) FROM agent_runtimes WHERE id = ?1",
                params![&provider.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(runtime_count, 0);
    }

    #[test]
    fn test_cannot_remove_bundled_provider() {
        let (service, _temp, _bin) = create_test_service_with_bundled_opencode();

        let result = service.remove_custom_provider("opencode");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bundled"));
    }

    #[test]
    fn test_check_prerequisites_npx() {
        let (service, _temp) = create_test_service();

        let check = service
            .check_prerequisites(InstallationMethod::Npx)
            .unwrap();

        // May or may not have Node.js installed depending on environment
        // Just verify the function works
        assert!(
            !check
                .missing_components
                .contains(&"InvalidMethod".to_string())
        );
    }

    #[test]
    fn test_available_methods_for_external_runtime() {
        let methods = AgentProviderService::get_available_methods("some-agent", false);

        assert!(!methods.iter().any(|m| m.id == InstallationMethod::Bundled));
        assert!(methods.iter().any(|m| m.id == InstallationMethod::Npx));
    }

    #[tokio::test]
    async fn test_opencode_runtime_uses_cli_command_and_tracks_path_availability() {
        let (service, _temp, bundled_bin) = create_test_service_with_bundled_opencode();

        let (command, args) = service.get_runtime_command("opencode").await.unwrap();
        assert_eq!(command, bundled_bin.to_string_lossy());
        assert_eq!(args, vec!["acp"]);

        let provider = service
            .list_providers()
            .unwrap()
            .into_iter()
            .find(|provider| provider.provider_id == "opencode")
            .unwrap();
        assert!(provider.is_installed);
        assert!(provider.is_default);
    }

    #[tokio::test]
    async fn test_opencode_runtime_prefers_bundled_binary_when_available() {
        let (service, _temp, bundled_bin) = create_test_service_with_bundled_opencode();

        let (command, args) = service.get_runtime_command("opencode").await.unwrap();
        assert_eq!(command, bundled_bin.to_string_lossy());
        assert_eq!(args, vec!["acp"]);

        let provider = service
            .list_providers()
            .unwrap()
            .into_iter()
            .find(|provider| provider.provider_id == "opencode")
            .unwrap();
        assert!(provider.is_bundled);
        assert!(provider.is_installed);
    }

    #[test]
    fn test_available_methods_for_third_party() {
        let methods = AgentProviderService::get_available_methods("opencode", false);

        // Should NOT have bundled
        assert!(!methods.iter().any(|m| m.id == InstallationMethod::Bundled));

        // Should have npx, binary, custom
        assert!(methods.iter().any(|m| m.id == InstallationMethod::Npx));
        assert!(methods.iter().any(|m| m.id == InstallationMethod::Binary));
        assert!(methods.iter().any(|m| m.id == InstallationMethod::Custom));
    }

    #[test]
    fn test_connection_result_uses_auth_required_as_successful_probe() {
        let result = test_connection_result_from_inspection(RuntimeInspectionResult {
            runtime_id: "codex".to_string(),
            auth_methods: vec![AuthMethodInfo {
                id: "chatgpt".to_string(),
                name: "ChatGPT".to_string(),
                description: None,
                manual_login_command: None,
            }],
            native_login_command: None,
            preferred_login_method: None,
            auth_required: true,
            discovered_models: vec![DiscoveredModelInfo {
                model_id: "gpt-5.4".to_string(),
                name: "GPT-5.4".to_string(),
                description: None,
            }],
            current_model_id: None,
            supports_model_selection: true,
            supports_config_options: true,
            error: Some("Authentication required".to_string()),
        });

        assert!(result.success);
        assert!(result.message.contains("Login required"));
        assert_eq!(result.available_models, vec!["gpt-5.4".to_string()]);
    }

    #[test]
    fn test_connection_result_surfaces_non_auth_inspection_errors() {
        let result = test_connection_result_from_inspection(RuntimeInspectionResult {
            runtime_id: "codex".to_string(),
            auth_methods: vec![],
            native_login_command: None,
            preferred_login_method: None,
            auth_required: false,
            discovered_models: vec![],
            current_model_id: None,
            supports_model_selection: false,
            supports_config_options: false,
            error: Some("Runtime inspection timed out".to_string()),
        });

        assert!(!result.success);
        assert_eq!(result.message, "Runtime inspection timed out");
    }

    #[test]
    fn stale_cached_inspection_is_rejected_for_native_preferred_runtime() {
        let inspection = RuntimeInspectionResult {
            runtime_id: "kimi".to_string(),
            auth_methods: vec![],
            native_login_command: None,
            preferred_login_method: None,
            auth_required: true,
            discovered_models: vec![],
            current_model_id: None,
            supports_model_selection: false,
            supports_config_options: false,
            error: Some("Authentication required".to_string()),
        };

        assert!(!cached_inspection_supports_login_preferences(
            "kimi",
            &inspection
        ));
    }

    #[test]
    fn cached_inspection_remains_valid_for_non_native_preferred_runtime() {
        let inspection = RuntimeInspectionResult {
            runtime_id: "opencode".to_string(),
            auth_methods: vec![],
            native_login_command: None,
            preferred_login_method: None,
            auth_required: false,
            discovered_models: vec![],
            current_model_id: None,
            supports_model_selection: false,
            supports_config_options: false,
            error: None,
        };

        assert!(cached_inspection_supports_login_preferences(
            "opencode",
            &inspection
        ));
    }

    #[test]
    fn test_runtime_inspection_cache_round_trip() {
        let (service, _temp, _bundled_bin) = create_test_service_with_bundled_opencode();
        let inspection = RuntimeInspectionResult {
            runtime_id: "opencode".to_string(),
            auth_methods: vec![],
            native_login_command: None,
            preferred_login_method: None,
            auth_required: false,
            discovered_models: vec![DiscoveredModelInfo {
                model_id: "opencode/big-pickle".to_string(),
                name: "Big Pickle".to_string(),
                description: None,
            }],
            current_model_id: Some("opencode/big-pickle".to_string()),
            supports_model_selection: true,
            supports_config_options: true,
            error: None,
        };

        service.store_runtime_inspection(&inspection).unwrap();

        let cached = service.cached_runtime_inspection("opencode").unwrap();
        assert!(cached.is_some());
        assert_eq!(
            cached.unwrap().current_model_id.as_deref(),
            Some("opencode/big-pickle")
        );

        let stored_json: Option<String> = service
            .test_conn()
            .query_row(
                "SELECT inspection_json FROM agent_runtimes WHERE runtime_type = 'opencode'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(stored_json.is_some());
    }

    #[test]
    fn test_update_provider_config_invalidates_runtime_inspection_cache() {
        let (service, _temp, _bundled_bin) = create_test_service_with_bundled_opencode();
        let inspection = RuntimeInspectionResult {
            runtime_id: "opencode".to_string(),
            auth_methods: vec![],
            native_login_command: None,
            preferred_login_method: None,
            auth_required: false,
            discovered_models: vec![DiscoveredModelInfo {
                model_id: "opencode/big-pickle".to_string(),
                name: "Big Pickle".to_string(),
                description: None,
            }],
            current_model_id: Some("opencode/big-pickle".to_string()),
            supports_model_selection: true,
            supports_config_options: true,
            error: None,
        };

        service.store_runtime_inspection(&inspection).unwrap();
        assert!(
            service
                .cached_runtime_inspection("opencode")
                .unwrap()
                .is_some()
        );

        service
            .update_provider_config(
                "opencode",
                &ProviderConfig {
                    default_model: Some("opencode/other".to_string()),
                    env_vars: HashMap::new(),
                    custom_args: Vec::new(),
                },
            )
            .unwrap();

        assert!(
            service
                .cached_runtime_inspection("opencode")
                .unwrap()
                .is_none()
        );

        let stored_json: Option<String> = service
            .test_conn()
            .query_row(
                "SELECT inspection_json FROM agent_runtimes WHERE runtime_type = 'opencode'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(stored_json.is_none());
    }

    #[test]
    fn test_cached_runtime_inspection_loads_from_database() {
        let (service, _temp, _bundled_bin) = create_test_service_with_bundled_opencode();
        let inspection = RuntimeInspectionResult {
            runtime_id: "opencode".to_string(),
            auth_methods: vec![],
            native_login_command: None,
            preferred_login_method: None,
            auth_required: false,
            discovered_models: vec![DiscoveredModelInfo {
                model_id: "opencode/big-pickle".to_string(),
                name: "Big Pickle".to_string(),
                description: None,
            }],
            current_model_id: Some("opencode/big-pickle".to_string()),
            supports_model_selection: true,
            supports_config_options: true,
            error: None,
        };

        let inspection_json = serde_json::to_string(&inspection).unwrap();
        service
            .test_conn()
            .execute(
                "UPDATE agent_runtimes SET inspection_json = ?1, inspected_at = ?2 WHERE runtime_type = 'opencode'",
                params![inspection_json, chrono::Utc::now().to_rfc3339()],
            )
            .unwrap();

        service
            .inspection_cache
            .lock()
            .expect("inspection cache test lock")
            .clear();

        let cached = service.cached_runtime_inspection("opencode").unwrap();
        assert!(cached.is_some());
        assert_eq!(
            cached.unwrap().current_model_id.as_deref(),
            Some("opencode/big-pickle")
        );
    }
}
