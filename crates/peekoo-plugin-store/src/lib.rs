//! Peekoo Plugin Store
//!
//! Remote plugin catalog backed by the GitHub repository. Handles:
//!
//! - Fetching the available plugin list from GitHub
//! - Downloading and installing plugins into `~/.peekoo/plugins/`
//! - Uninstalling store-managed plugins

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use peekoo_paths::peekoo_global_data_dir;
use peekoo_plugin_host::manifest::parse_manifest;
use peekoo_plugin_host::{PluginManifest, PluginRegistry};

const GITHUB_API_CONTENTS_URL: &str =
    "https://api.github.com/repos/feed-mob/peekoo-ai/contents/plugins";
const GITHUB_RAW_BASE_URL: &str =
    "https://raw.githubusercontent.com/feed-mob/peekoo-ai/master/plugins";

/// Where a plugin was installed from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginSource {
    /// Installed from the remote store into `~/.peekoo/plugins/`.
    Store,
    /// Not installed.
    None,
}

/// DTO describing a plugin available in the remote store.
///
/// The `installed` flag and `source` field are cross-referenced against the
/// local [`PluginRegistry`] at the time the catalog is fetched.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorePluginDto {
    pub plugin_key: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub tool_count: usize,
    pub panel_count: usize,
    /// Whether the plugin is currently installed locally.
    pub installed: bool,
    /// Origin of the installation.
    pub source: PluginSource,
    /// Whether a newer version is available in the store.
    pub has_update: bool,
}

#[derive(Debug, Deserialize)]
struct GitHubContent {
    name: String,
    #[serde(rename = "type")]
    content_type: String,
}

/// Stateless service that manages the remote plugin catalog.
pub struct PluginStoreService;

impl Default for PluginStoreService {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginStoreService {
    pub fn new() -> Self {
        Self
    }

    /// Fetch the full catalog from GitHub and cross-reference with the local
    /// registry to populate `installed` and `source` on each entry.
    pub fn fetch_catalog(
        &self,
        registry: &Arc<PluginRegistry>,
    ) -> Result<Vec<StorePluginDto>, String> {
        let response = ureq::get(GITHUB_API_CONTENTS_URL)
            .set("User-Agent", "Peekoo-AI/1.0")
            .set("Accept", "application/vnd.github.v3+json")
            .call()
            .map_err(|e| format!("GitHub API request failed: {e}"))?;

        let contents: Vec<GitHubContent> = response
            .into_json()
            .map_err(|e| format!("Failed to parse GitHub API response: {e}"))?;

        let local_plugins = registry.discover();

        let mut store_plugins = Vec::new();

        for item in contents {
            if item.content_type != "dir" {
                continue;
            }

            match self.fetch_plugin_metadata(&item.name) {
                Ok(manifest) => {
                    let local_version = local_plugins
                        .iter()
                        .find(|(_, m)| m.plugin.key == manifest.plugin.key)
                        .map(|(_, m)| m.plugin.version.as_str());
                    let has_update = local_version
                        .map(|lv| is_newer_version(&manifest.plugin.version, lv))
                        .unwrap_or(false);

                    store_plugins.push(self.build_dto(manifest, &local_plugins, has_update));
                }
                Err(e) => {
                    warn!(
                        plugin = item.name.as_str(),
                        "Failed to fetch plugin metadata: {e}"
                    );
                }
            }
        }

        Ok(store_plugins)
    }

    /// Download a plugin from GitHub into `~/.peekoo/plugins/<key>/` and load
    /// it into the registry.
    pub fn install_plugin(
        &self,
        plugin_key: &str,
        registry: &Arc<PluginRegistry>,
    ) -> Result<StorePluginDto, String> {
        let dest_dir = peekoo_global_data_dir()?.join("plugins").join(plugin_key);

        if dest_dir.exists() {
            return Err(format!("Plugin {plugin_key} is already installed"));
        }

        info!(plugin = plugin_key, "Installing plugin from store");

        let manifest = self.fetch_plugin_metadata(plugin_key)?;

        let install_result = self
            .download_plugin_files(plugin_key, &dest_dir)
            .and_then(|()| {
                registry
                    .install_plugin(&dest_dir)
                    .map(|_| ())
                    .map_err(|e| format!("Failed to load plugin: {e}"))
            });

        if let Err(e) = install_result {
            let _ = std::fs::remove_dir_all(&dest_dir);
            return Err(e);
        }

        let local_plugins = registry.discover();
        Ok(self.build_dto(manifest, &local_plugins, false))
    }

    /// Update an installed plugin to the latest version from GitHub.
    ///
    /// Returns an error if the plugin is not installed. If the remote version
    /// is not newer than the installed version, returns `Ok` with the current
    /// plugin state (no update performed).
    pub fn update_plugin(
        &self,
        plugin_key: &str,
        registry: &Arc<PluginRegistry>,
    ) -> Result<StorePluginDto, String> {
        let dest_dir = peekoo_global_data_dir()?.join("plugins").join(plugin_key);

        if !dest_dir.exists() {
            return Err(format!("Plugin {plugin_key} is not installed"));
        }

        let local_plugins = registry.discover();
        let local_version = local_plugins
            .iter()
            .find(|(_, m)| m.plugin.key == plugin_key)
            .map(|(_, m)| m.plugin.version.as_str())
            .unwrap_or("0.0.0");

        let remote_manifest = self.fetch_plugin_metadata(plugin_key)?;

        if !is_newer_version(&remote_manifest.plugin.version, local_version) {
            return Ok(self.build_dto(remote_manifest, &local_plugins, false));
        }

        info!(
            plugin = plugin_key,
            from = local_version,
            to = remote_manifest.plugin.version.as_str(),
            "Updating plugin from store"
        );

        // Best-effort unload before replacing files.
        let _ = registry.unload_plugin(plugin_key);

        self.with_backup_guard(&dest_dir, || {
            self.download_plugin_files(plugin_key, &dest_dir)?;
            registry
                .install_plugin(&dest_dir)
                .map(|_| ())
                .map_err(|e| format!("Failed to load updated plugin: {e}"))
        })?;

        let local_plugins = registry.discover();

        info!(plugin = plugin_key, "Plugin updated successfully");

        Ok(self.build_dto(remote_manifest, &local_plugins, false))
    }

    /// Unload a plugin from the registry and delete its directory from
    /// `~/.peekoo/plugins/<key>/`. Only plugins installed via the store
    /// (i.e. present in the global plugins dir) can be removed.
    pub fn uninstall_plugin(
        &self,
        plugin_key: &str,
        registry: &Arc<PluginRegistry>,
    ) -> Result<(), String> {
        let global_plugins_dir = peekoo_global_data_dir()?.join("plugins");
        let plugin_dir = global_plugins_dir.join(plugin_key);

        if !plugin_dir.exists() {
            return Err(format!(
                "Plugin {plugin_key} is not installed in the store directory"
            ));
        }

        info!(plugin = plugin_key, "Uninstalling plugin");

        // Best-effort unload - plugin may not be in memory if initialization failed
        let _ = registry.unload_plugin(plugin_key);

        std::fs::remove_dir_all(&plugin_dir)
            .map_err(|e| format!("Failed to remove plugin directory: {e}"))?;

        info!(plugin = plugin_key, "Plugin uninstalled successfully");

        Ok(())
    }

    fn check_installation(
        &self,
        plugin_key: &str,
        local_plugins: &[(PathBuf, PluginManifest)],
    ) -> (bool, PluginSource) {
        for (_, manifest) in local_plugins {
            if manifest.plugin.key == plugin_key {
                return (true, PluginSource::Store);
            }
        }
        (false, PluginSource::None)
    }

    fn build_dto(
        &self,
        manifest: PluginManifest,
        local_plugins: &[(PathBuf, PluginManifest)],
        has_update: bool,
    ) -> StorePluginDto {
        let (installed, source) = self.check_installation(&manifest.plugin.key, local_plugins);
        let tool_count = manifest
            .tools
            .as_ref()
            .map(|t| t.definitions.len())
            .unwrap_or(0);
        let panel_count = manifest.ui.as_ref().map(|u| u.panels.len()).unwrap_or(0);

        StorePluginDto {
            plugin_key: manifest.plugin.key,
            name: manifest.plugin.name,
            version: manifest.plugin.version,
            author: manifest.plugin.author,
            description: manifest.plugin.description,
            tool_count,
            panel_count,
            installed,
            source,
            has_update,
        }
    }

    /// Perform an operation within a backup/restore guard.
    ///
    /// Renames `dest_dir` to `<dest_dir>.old` before calling `operation`.
    /// On failure the backup is restored; on success it is removed.
    fn with_backup_guard<F>(&self, dest_dir: &Path, operation: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<(), String>,
    {
        let backup_dir = dest_dir.with_extension("old");

        if backup_dir.exists() {
            std::fs::remove_dir_all(&backup_dir)
                .map_err(|e| format!("Failed to remove stale backup: {e}"))?;
        }

        std::fs::rename(dest_dir, &backup_dir)
            .map_err(|e| format!("Failed to backup plugin directory: {e}"))?;

        if let Err(e) = operation() {
            // Attempt cleanup of the partial new directory before restoring.
            if dest_dir.exists() {
                let _ = std::fs::remove_dir_all(dest_dir);
            }
            if let Err(restore_err) = std::fs::rename(&backup_dir, dest_dir) {
                warn!("Failed to restore backup after failure: {restore_err}");
            }
            return Err(e);
        }

        if let Err(e) = std::fs::remove_dir_all(&backup_dir) {
            warn!("Failed to remove backup directory: {e}");
        }

        Ok(())
    }

    fn fetch_plugin_metadata(&self, plugin_key: &str) -> Result<PluginManifest, String> {
        let manifest_url = format!("{GITHUB_RAW_BASE_URL}/{plugin_key}/peekoo-plugin.toml");

        let response = ureq::get(&manifest_url)
            .set("User-Agent", "Peekoo-AI/1.0")
            .call()
            .map_err(|e| format!("Failed to fetch manifest: {e}"))?;

        let manifest_toml = response
            .into_string()
            .map_err(|e| format!("Failed to read manifest: {e}"))?;

        parse_manifest(&manifest_toml).map_err(|e| format!("Failed to parse manifest: {e}"))
    }

    fn download_plugin_files(&self, plugin_key: &str, dest_dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dest_dir)
            .map_err(|e| format!("Failed to create plugin directory: {e}"))?;

        let contents_url = format!("{GITHUB_API_CONTENTS_URL}/{plugin_key}");

        let response = ureq::get(&contents_url)
            .set("User-Agent", "Peekoo-AI/1.0")
            .set("Accept", "application/vnd.github.v3+json")
            .call()
            .map_err(|e| format!("Failed to list plugin contents: {e}"))?;

        let contents: Vec<GitHubContent> = response
            .into_json()
            .map_err(|e| format!("Failed to parse plugin contents: {e}"))?;

        const MAX_RECURSION_DEPTH: u32 = 10;
        self.download_directory_recursive(
            plugin_key,
            "",
            dest_dir,
            &contents,
            MAX_RECURSION_DEPTH,
        )?;

        Ok(())
    }

    fn download_directory_recursive(
        &self,
        plugin_key: &str,
        sub_path: &str,
        dest_dir: &Path,
        contents: &[GitHubContent],
        depth: u32,
    ) -> Result<(), String> {
        if depth == 0 {
            return Err("Maximum directory recursion depth exceeded".to_string());
        }

        for item in contents {
            match item.content_type.as_str() {
                "file" => {
                    let file_url =
                        format!("{GITHUB_RAW_BASE_URL}/{plugin_key}/{sub_path}{}", item.name);
                    let file_path = dest_dir.join(sub_path).join(&item.name);

                    if let Some(parent) = file_path.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| format!("Failed to create parent directory: {e}"))?;
                    }

                    let response = ureq::get(&file_url)
                        .set("User-Agent", "Peekoo-AI/1.0")
                        .call()
                        .map_err(|e| format!("Failed to download file {}: {e}", item.name))?;

                    let mut content = Vec::new();
                    response
                        .into_reader()
                        .read_to_end(&mut content)
                        .map_err(|e| format!("Failed to read file {}: {e}", item.name))?;

                    std::fs::write(&file_path, &content)
                        .map_err(|e| format!("Failed to write file {}: {e}", item.name))?;
                }
                "dir" => {
                    let sub_dir_path = if sub_path.is_empty() {
                        format!("{}/", item.name)
                    } else {
                        format!("{sub_path}{}/", item.name)
                    };

                    let sub_contents_url =
                        format!("{GITHUB_API_CONTENTS_URL}/{plugin_key}/{sub_dir_path}");

                    let response = ureq::get(&sub_contents_url)
                        .set("User-Agent", "Peekoo-AI/1.0")
                        .set("Accept", "application/vnd.github.v3+json")
                        .call()
                        .map_err(|e| format!("Failed to list subdirectory {}: {e}", item.name))?;

                    let sub_contents: Vec<GitHubContent> = response
                        .into_json()
                        .map_err(|e| format!("Failed to parse subdirectory contents: {e}"))?;

                    self.download_directory_recursive(
                        plugin_key,
                        &sub_dir_path,
                        dest_dir,
                        &sub_contents,
                        depth - 1,
                    )?;
                }
                _ => {}
            }
        }

        Ok(())
    }
}

fn is_newer_version(remote: &str, local: &str) -> bool {
    let remote_ver = semver::Version::parse(remote).ok();
    let local_ver = semver::Version::parse(local).ok();

    match (remote_ver, local_ver) {
        (Some(remote), Some(local)) => remote > local,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use rusqlite::Connection;

    use super::*;
    use peekoo_plugin_host::manifest::parse_manifest;

    fn make_store() -> PluginStoreService {
        PluginStoreService::new()
    }

    #[test]
    fn store_plugin_dto_serializes_to_camel_case() {
        let dto = StorePluginDto {
            plugin_key: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            author: Some("Test Author".to_string()),
            description: Some("A test plugin".to_string()),
            tool_count: 2,
            panel_count: 1,
            installed: true,
            source: PluginSource::Store,
            has_update: false,
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("pluginKey"));
        assert!(json.contains("toolCount"));
        assert!(json.contains("panelCount"));
        assert!(json.contains("hasUpdate"));
    }

    #[test]
    fn store_plugin_dto_not_installed_has_none_source() {
        let dto = StorePluginDto {
            plugin_key: "example-minimal".to_string(),
            name: "Example Minimal".to_string(),
            version: "0.1.0".to_string(),
            author: None,
            description: None,
            tool_count: 0,
            panel_count: 0,
            installed: false,
            source: PluginSource::None,
            has_update: false,
        };

        let json = serde_json::to_string(&dto).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["installed"], false);
        assert_eq!(parsed["source"], "none");
        assert_eq!(parsed["hasUpdate"], false);
    }

    #[test]
    fn check_installation_returns_not_installed_when_not_in_registry() {
        let store = make_store();
        let local_plugins: Vec<(PathBuf, PluginManifest)> = vec![];
        let (installed, source) = store.check_installation("unknown-plugin", &local_plugins);
        assert!(!installed);
        assert_eq!(source, PluginSource::None);
    }

    #[test]
    fn check_installation_detects_store_plugin() {
        let store = make_store();
        let manifest = parse_manifest(
            r#"
[plugin]
key = "store-plugin"
name = "Store Plugin"
version = "0.2.0"
wasm = "plugin.wasm"
"#,
        )
        .unwrap();

        // Simulate a global store path
        let global_dir = peekoo_global_data_dir()
            .unwrap()
            .join("plugins")
            .join("store-plugin");
        let local_plugins = vec![(global_dir, manifest)];

        let (installed, source) = store.check_installation("store-plugin", &local_plugins);
        assert!(installed);
        assert_eq!(source, PluginSource::Store);
    }

    #[test]
    fn check_installation_treats_discovered_but_not_loaded_plugin_as_installed() {
        // A plugin that exists on disk (returned by discover()) but failed to
        // load into the runtime should still be considered "installed".
        let store = make_store();
        let manifest = parse_manifest(
            r#"
[plugin]
key = "broken-plugin"
name = "Broken Plugin"
version = "1.0.0"
wasm = "plugin.wasm"
"#,
        )
        .unwrap();

        let global_dir = peekoo_global_data_dir()
            .unwrap()
            .join("plugins")
            .join("broken-plugin");
        let local_plugins = vec![(global_dir, manifest)];

        let (installed, source) = store.check_installation("broken-plugin", &local_plugins);
        assert!(
            installed,
            "discovered plugin should be installed=true even if not loaded"
        );
        assert_eq!(source, PluginSource::Store);
    }

    #[test]
    fn uninstall_returns_error_when_plugin_not_in_global_dir() {
        let store = make_store();
        let result = store.uninstall_plugin(
            "nonexistent-plugin",
            &Arc::new(PluginRegistry::new(
                vec![],
                Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            )),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.contains("not installed in the store directory"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn is_newer_version_detects_major_update() {
        assert!(is_newer_version("2.0.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "2.0.0"));
    }

    #[test]
    fn is_newer_version_detects_minor_update() {
        assert!(is_newer_version("1.1.0", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "1.1.0"));
    }

    #[test]
    fn is_newer_version_detects_patch_update() {
        assert!(is_newer_version("1.0.1", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "1.0.1"));
    }

    #[test]
    fn is_newer_version_returns_false_for_same_version() {
        assert!(!is_newer_version("1.0.0", "1.0.0"));
    }

    #[test]
    fn is_newer_version_handles_pre_release() {
        assert!(is_newer_version("1.0.0", "1.0.0-alpha"));
        assert!(is_newer_version("1.0.0-beta.2", "1.0.0-beta.1"));
        assert!(!is_newer_version("1.0.0-alpha", "1.0.0"));
    }

    #[test]
    fn is_newer_version_handles_invalid_versions() {
        assert!(!is_newer_version("invalid", "1.0.0"));
        assert!(!is_newer_version("1.0.0", "invalid"));
        assert!(!is_newer_version("invalid", "invalid"));
    }

    #[test]
    fn update_plugin_returns_error_when_not_installed() {
        let store = make_store();
        let result = store.update_plugin(
            "nonexistent-plugin",
            &Arc::new(PluginRegistry::new(
                vec![],
                Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
            )),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("is not installed"), "unexpected error: {msg}");
    }
}
