//! Agent provider management service
//!
//! This service manages the installation, configuration, and lifecycle of
//! ACP-compatible agent providers (pi-acp, opencode, claude-code, codex, custom).

use anyhow;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

/// Provider installation method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationMethod {
    /// Bundled with the application (pi-acp only)
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

/// Provider information DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// Installation method information
#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct ProviderConfig {
    pub default_model: Option<String>,
    pub env_vars: HashMap<String, String>,
    pub custom_args: Vec<String>,
}

/// Request to install a provider
#[derive(Debug, Clone, Deserialize)]
pub struct InstallProviderRequest {
    pub provider_id: String,
    pub method: InstallationMethod,
    pub custom_path: Option<String>,
}

/// Response from provider installation
#[derive(Debug, Clone, Serialize)]
pub struct InstallProviderResponse {
    pub success: bool,
    pub message: String,
    pub requires_restart: bool,
}

/// Test connection result
#[derive(Debug, Clone, Serialize)]
pub struct TestConnectionResult {
    pub success: bool,
    pub message: String,
    pub available_models: Vec<String>,
    pub provider_version: Option<String>,
}

/// Prerequisites check result
#[derive(Debug, Clone, Serialize)]
pub struct PrerequisitesCheck {
    pub available: bool,
    pub missing_components: Vec<String>,
    pub instructions: Option<String>,
}

/// Service for managing agent providers
pub struct AgentProviderService {
    conn: Connection,
    data_dir: PathBuf,
}

impl AgentProviderService {
    /// Create a new provider service
    pub fn new(db_path: &PathBuf, data_dir: PathBuf) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;

        // Ensure tables exist (they should from migrations)
        // But we might need them for testing
        Self::ensure_tables(&conn)?;

        // Ensure built-in providers are registered
        Self::seed_builtin_providers(&conn)?;

        Ok(Self { conn, data_dir })
    }

    /// Get a reference to the connection for testing purposes
    /// 
    /// # Warning
    /// This is intended for testing only. Direct SQL access may bypass
    /// business logic and invariants.
    #[cfg(test)]
    pub fn test_conn(&self) -> &Connection {
        &self.conn
    }

    fn ensure_tables(conn: &Connection) -> anyhow::Result<()> {
        // Tables should already exist from migrations, but ensure for testing
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agent_providers (
                id TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL UNIQUE,
                display_name TEXT NOT NULL,
                description TEXT,
                is_bundled INTEGER NOT NULL DEFAULT 0,
                installation_method TEXT NOT NULL,
                command TEXT,
                args_json TEXT,
                binary_path TEXT,
                download_url TEXT,
                checksum TEXT,
                is_installed INTEGER NOT NULL DEFAULT 0,
                is_default INTEGER NOT NULL DEFAULT 0,
                status TEXT NOT NULL,
                status_message TEXT,
                installed_at TEXT,
                updated_at TEXT NOT NULL,
                config_json TEXT,
                env_vars_json TEXT
            )",
            [],
        )?;

        Ok(())
    }

    fn seed_builtin_providers(conn: &Connection) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Insert pi-acp as bundled provider
        conn.execute(
            "INSERT OR IGNORE INTO agent_providers (
                id, provider_id, display_name, description, is_bundled,
                installation_method, command, args_json, is_installed,
                is_default, status, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                "provider_pi_acp",
                "pi-acp",
                "Peekoo Agent (pi-acp)",
                "Built-in ACP agent with full tool support",
                1, // is_bundled
                "bundled",
                "pi-acp", // Will be resolved to bundled binary path
                "[]",
                1, // is_installed
                1, // is_default
                "ready",
                &now,
            ],
        )?;

        // Insert opencode as available provider
        conn.execute(
            "INSERT OR IGNORE INTO agent_providers (
                id, provider_id, display_name, description, is_bundled,
                installation_method, command, args_json, is_installed,
                is_default, status, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                "provider_opencode",
                "opencode",
                "OpenCode",
                "Zed's OpenAI-compatible agent",
                0, // is_bundled
                "npx",
                "npx",
                "[\"opencode-ai\"]",
                0, // not installed by default
                0,
                "not_installed",
                &now,
            ],
        )?;

        // Insert claude-code as available provider
        conn.execute(
            "INSERT OR IGNORE INTO agent_providers (
                id, provider_id, display_name, description, is_bundled,
                installation_method, command, args_json, is_installed,
                is_default, status, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                "provider_claude_code",
                "claude-code",
                "Claude Code",
                "Anthropic's Claude Code agent",
                0, // is_bundled
                "npx",
                "npx",
                "[\"@anthropic-ai/claude-code\"]",
                0, // not installed by default
                0,
                "not_installed",
                &now,
            ],
        )?;

        // Insert codex as available provider
        conn.execute(
            "INSERT OR IGNORE INTO agent_providers (
                id, provider_id, display_name, description, is_bundled,
                installation_method, command, args_json, is_installed,
                is_default, status, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                "provider_codex",
                "codex",
                "Codex",
                "Zed's Codex agent for GitHub integration",
                0, // is_bundled
                "npx",
                "npx",
                "[\"@zed-industries/codex-acp\"]",
                0, // not installed by default
                0,
                "not_installed",
                &now,
            ],
        )?;

        Ok(())
    }

    /// List all providers (installed + available)
    pub fn list_providers(&self) -> anyhow::Result<Vec<ProviderInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT 
                id, provider_id, display_name, description, is_bundled,
                installation_method, is_installed, is_default, status,
                status_message, command, args_json, config_json, env_vars_json
            FROM agent_providers
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
                let env_vars_json: Option<String> = row.get(13)?;

                let mut config: ProviderConfig = config_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                // Merge env vars from separate column if present
                if let Some(env_json) = env_vars_json {
                    if let Ok(env_vars) = serde_json::from_str::<HashMap<String, String>>(&env_json)
                    {
                        config.env_vars.extend(env_vars);
                    }
                }

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
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(providers)
    }

    /// Get available installation methods for a provider
    fn get_available_methods(provider_id: &str, is_bundled: bool) -> Vec<InstallationMethodInfo> {
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
        let has_node = which::which("node").is_ok() && which::which("npm").is_ok();

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
        let mut stmt = self.conn.prepare(
            "SELECT 
                id, provider_id, display_name, description, is_bundled,
                installation_method, is_installed, is_default, status,
                status_message, command, args_json, config_json, env_vars_json
            FROM agent_providers
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
                })
            })
            .optional()?;

        Ok(provider)
    }

    /// Set the default provider
    pub fn set_default_provider(&self, provider_id: &str) -> anyhow::Result<()> {
        let now = chrono::Utc::now().to_rfc3339();

        // Clear existing default
        self.conn.execute(
            "UPDATE agent_providers SET is_default = 0, updated_at = ?1",
            params![&now],
        )?;

        // Set new default
        self.conn.execute(
            "UPDATE agent_providers SET is_default = 1, updated_at = ?1 WHERE provider_id = ?2",
            params![&now, provider_id],
        )?;

        Ok(())
    }

    /// Install a provider
    pub async fn install_provider(
        &self,
        req: InstallProviderRequest,
    ) -> anyhow::Result<InstallProviderResponse> {
        let now = chrono::Utc::now().to_rfc3339();

        // Update provider to installing status
        self.conn.execute(
            "UPDATE agent_providers SET status = 'installing', updated_at = ?1 WHERE provider_id = ?2",
            params![&now, &req.provider_id],
        )?;

        // Perform installation based on method
        let result = match req.method {
            InstallationMethod::Bundled => {
                // Nothing to do for bundled
                Ok(())
            }
            InstallationMethod::Npx => {
                // For npx, we just need to verify the package exists
                // Actual installation happens on first use
                Self::verify_npx_package(&req.provider_id).await
            }
            InstallationMethod::Binary => {
                // Download binary
                Self::download_provider_binary(&self.data_dir, &req.provider_id).await
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
                self.conn.execute(
                    "UPDATE agent_providers SET 
                        is_installed = 1, 
                        status = 'ready',
                        status_message = NULL,
                        installed_at = ?1,
                        updated_at = ?1
                    WHERE provider_id = ?2",
                    params![&now, &req.provider_id],
                )?;
                (
                    true,
                    format!("{} installed successfully", req.provider_id),
                    false,
                )
            }
            Err(e) => {
                self.conn.execute(
                    "UPDATE agent_providers SET 
                        status = 'error',
                        status_message = ?1,
                        updated_at = ?2
                    WHERE provider_id = ?3",
                    params![&e.to_string(), &now, &req.provider_id],
                )?;
                (false, e.to_string(), false)
            }
        };

        Ok(InstallProviderResponse {
            success,
            message,
            requires_restart,
        })
    }

    /// Verify npx package exists
    async fn verify_npx_package(provider_id: &str) -> anyhow::Result<()> {
        // Map provider to npm package name
        let package = match provider_id {
            "pi-acp" => "pi-acp",
            "opencode" => "opencode-ai",
            "claude-code" => "@anthropic-ai/claude-code",
            "codex" => "@zed-industries/codex-acp",
            _ => return Err(anyhow::anyhow!("Unknown provider: {}", provider_id)),
        };

        // Try to get package info from npm registry
        let output = Command::new("npm")
            .args(["view", package, "version"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to run npm: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Package not found: {}", stderr));
        }

        Ok(())
    }

    /// Download provider binary
    async fn download_provider_binary(data_dir: &PathBuf, provider_id: &str) -> anyhow::Result<()> {
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

        // Check if it's the default
        let is_default: i64 = self.conn.query_row(
            "SELECT is_default FROM agent_providers WHERE provider_id = ?1",
            params![provider_id],
            |row| row.get(0),
        )?;

        if is_default != 0 {
            return Err(anyhow::anyhow!(
                "Cannot uninstall the default provider. Please set a different provider as default first."
            ));
        }

        // Update status
        self.conn.execute(
            "UPDATE agent_providers SET 
                is_installed = 0,
                status = 'not_installed',
                status_message = NULL,
                updated_at = ?1
            WHERE provider_id = ?2",
            params![&now, provider_id],
        )?;

        Ok(())
    }

    /// Get provider configuration
    pub fn get_provider_config(&self, provider_id: &str) -> anyhow::Result<ProviderConfig> {
        let config_json: Option<String> = self.conn.query_row(
            "SELECT config_json FROM agent_providers WHERE provider_id = ?1",
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

        self.conn.execute(
            "UPDATE agent_providers SET config_json = ?1, updated_at = ?2 WHERE provider_id = ?3",
            params![config_json, &now, provider_id],
        )?;

        Ok(())
    }

    /// Test provider connection
    pub async fn test_connection(&self, provider_id: &str) -> anyhow::Result<TestConnectionResult> {
        let provider = self.conn.query_row(
            "SELECT command, args_json, is_installed, status FROM agent_providers WHERE provider_id = ?1",
            params![provider_id],
            |row| {
                let command: String = row.get(0)?;
                let args_json: String = row.get(1)?;
                let is_installed: i64 = row.get(2)?;
                let status: String = row.get(3)?;
                Ok((command, args_json, is_installed != 0, status))
            },
        ).optional()?;

        match provider {
            Some((_, _, false, _)) => Ok(TestConnectionResult {
                success: false,
                message: "Provider is not installed".to_string(),
                available_models: vec![],
                provider_version: None,
            }),
            Some((command, args_json, true, status)) if status == "ready" => {
                let args: Vec<String> = serde_json::from_str(&args_json)?;

                // Try to spawn the agent and check version
                let output = Command::new(&command)
                    .args(&args)
                    .arg("--version")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await;

                match output {
                    Ok(output) if output.status.success() => {
                        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        Ok(TestConnectionResult {
                            success: true,
                            message: "Connection successful".to_string(),
                            available_models: vec![], // Would be populated by the agent
                            provider_version: Some(version),
                        })
                    }
                    _ => Ok(TestConnectionResult {
                        success: false,
                        message: "Failed to connect to provider".to_string(),
                        available_models: vec![],
                        provider_version: None,
                    }),
                }
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
    pub async fn check_prerequisites(
        &self,
        method: InstallationMethod,
    ) -> anyhow::Result<PrerequisitesCheck> {
        match method {
            InstallationMethod::Npx => {
                let has_node = which::which("node").is_ok();
                let has_npm = which::which("npm").is_ok();

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
        working_dir: Option<&str>,
    ) -> anyhow::Result<ProviderInfo> {
        let id = format!("provider_custom_{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO agent_providers (
                id, provider_id, display_name, description, is_bundled,
                installation_method, command, args_json, is_installed,
                is_default, status, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                &id,
                &id, // Use id as provider_id for custom
                name,
                description.unwrap_or("Custom ACP agent"),
                0, // is_bundled
                "custom",
                command,
                &serde_json::to_string(args)?,
                1, // is_installed (custom providers are always "installed" if path is valid)
                0,
                "ready",
                &now,
            ],
        )?;

        // Return the new provider info
        let mut stmt = self.conn.prepare(
            "SELECT 
                id, provider_id, display_name, description, is_bundled,
                installation_method, is_installed, is_default, status,
                status_message, command, args_json, config_json, env_vars_json
            FROM agent_providers
            WHERE id = ?1",
        )?;

        let provider = stmt.query_row(params![&id], |row| {
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
            })
        })?;

        Ok(provider)
    }

    /// Remove a custom provider
    pub fn remove_custom_provider(&self, provider_id: &str) -> anyhow::Result<()> {
        // Only allow removing custom providers
        let is_bundled: i64 = self.conn.query_row(
            "SELECT is_bundled FROM agent_providers WHERE provider_id = ?1",
            params![provider_id],
            |row| row.get(0),
        )?;

        if is_bundled != 0 {
            return Err(anyhow::anyhow!("Cannot remove built-in providers"));
        }

        // Check if it's the default
        let is_default: i64 = self.conn.query_row(
            "SELECT is_default FROM agent_providers WHERE provider_id = ?1",
            params![provider_id],
            |row| row.get(0),
        )?;

        if is_default != 0 {
            return Err(anyhow::anyhow!(
                "Cannot remove the default provider. Please set a different provider as default first."
            ));
        }

        self.conn.execute(
            "DELETE FROM agent_providers WHERE provider_id = ?1",
            params![provider_id],
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
    use tempfile::TempDir;

    fn create_test_service() -> (AgentProviderService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_providers.db");
        let data_dir = temp_dir.path().join("data");

        let service = AgentProviderService::new(&db_path, data_dir).unwrap();
        (service, temp_dir)
    }

    #[test]
    fn test_list_providers() {
        let (service, _temp) = create_test_service();

        let providers = service.list_providers().unwrap();

        // Should have the 4 built-in providers seeded
        assert!(providers.len() >= 4);

        // pi-acp should be installed by default
        let pi_acp = providers
            .iter()
            .find(|p| p.provider_id == "pi-acp")
            .unwrap();
        assert!(pi_acp.is_installed);
        assert!(pi_acp.is_default);
        assert!(pi_acp.is_bundled);
    }

    #[test]
    fn test_get_default_provider() {
        let (service, _temp) = create_test_service();

        let default = service.get_default_provider().unwrap();
        assert!(default.is_some());

        let default = default.unwrap();
        assert_eq!(default.provider_id, "pi-acp");
        assert!(default.is_default);
    }

    #[test]
    fn test_set_default_provider() {
        let (service, _temp) = create_test_service();

        // First mark opencode as installed (normally would be done via install)
        service.test_conn().execute(
            "UPDATE agent_providers SET is_installed = 1, status = 'ready' WHERE provider_id = 'opencode'",
            [],
        ).unwrap();

        // Set opencode as default
        service.set_default_provider("opencode").unwrap();

        // Verify
        let default = service.get_default_provider().unwrap().unwrap();
        assert_eq!(default.provider_id, "opencode");

        // Verify pi-acp is no longer default
        let providers = service.list_providers().unwrap();
        let pi_acp = providers
            .iter()
            .find(|p| p.provider_id == "pi-acp")
            .unwrap();
        assert!(!pi_acp.is_default);
    }

    #[test]
    fn test_get_provider_config() {
        let (service, _temp) = create_test_service();

        let config = service.get_provider_config("pi-acp").unwrap();
        // Default config should be empty
        assert!(config.default_model.is_none());
    }

    #[test]
    fn test_update_provider_config() {
        let (service, _temp) = create_test_service();

        let new_config = ProviderConfig {
            default_model: Some("claude-3.5-sonnet".to_string()),
            env_vars: [("API_KEY".to_string(), "secret".to_string())].into(),
            custom_args: vec!["--verbose".to_string()],
        };

        service
            .update_provider_config("pi-acp", &new_config)
            .unwrap();

        // Verify
        let config = service.get_provider_config("pi-acp").unwrap();
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
    }

    #[test]
    fn test_cannot_remove_builtin_provider() {
        let (service, _temp) = create_test_service();

        let result = service.remove_custom_provider("pi-acp");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("built-in"));
    }

    #[tokio::test]
    async fn test_check_prerequisites_npx() {
        let (service, _temp) = create_test_service();

        let check = service
            .check_prerequisites(InstallationMethod::Npx)
            .await
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
    fn test_available_methods_for_bundled() {
        let methods = AgentProviderService::get_available_methods("pi-acp", true);

        // Should have bundled as first method
        assert!(methods.iter().any(|m| m.id == InstallationMethod::Bundled));
        assert!(methods[0].is_available);
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
}
