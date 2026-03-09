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
    /// `"store"` — installed via the store, `"workspace"` — found in a
    /// workspace `plugins/` directory, `"none"` — not installed.
    pub source: String,
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
        let loaded_keys = registry.loaded_keys();

        let mut store_plugins = Vec::new();

        for item in contents {
            if item.content_type != "dir" {
                continue;
            }

            match self.fetch_plugin_metadata(&item.name) {
                Ok(manifest) => {
                    let (installed, source) =
                        self.check_installation(&manifest.plugin.key, &local_plugins, &loaded_keys);
                    let tool_count = manifest
                        .tools
                        .as_ref()
                        .map(|t| t.definitions.len())
                        .unwrap_or(0);
                    let panel_count = manifest.ui.as_ref().map(|u| u.panels.len()).unwrap_or(0);

                    store_plugins.push(StorePluginDto {
                        plugin_key: manifest.plugin.key,
                        name: manifest.plugin.name,
                        version: manifest.plugin.version,
                        author: manifest.plugin.author,
                        description: manifest.plugin.description,
                        tool_count,
                        panel_count,
                        installed,
                        source,
                    });
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

        self.download_plugin_files(plugin_key, &dest_dir)?;

        let plugin_key_loaded = registry
            .install_plugin(&dest_dir)
            .map_err(|e| format!("Failed to load plugin: {e}"))?;

        let local_plugins = registry.discover();
        let loaded_keys = registry.loaded_keys();
        let (installed, source) =
            self.check_installation(&plugin_key_loaded, &local_plugins, &loaded_keys);

        let tool_count = manifest
            .tools
            .as_ref()
            .map(|t| t.definitions.len())
            .unwrap_or(0);
        let panel_count = manifest.ui.as_ref().map(|u| u.panels.len()).unwrap_or(0);

        Ok(StorePluginDto {
            plugin_key: manifest.plugin.key,
            name: manifest.plugin.name,
            version: manifest.plugin.version,
            author: manifest.plugin.author,
            description: manifest.plugin.description,
            tool_count,
            panel_count,
            installed,
            source,
        })
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

        registry
            .unload_plugin(plugin_key)
            .map_err(|e| format!("Failed to unload plugin: {e}"))?;

        std::fs::remove_dir_all(&plugin_dir)
            .map_err(|e| format!("Failed to remove plugin directory: {e}"))?;

        info!(plugin = plugin_key, "Plugin uninstalled successfully");

        Ok(())
    }

    fn check_installation(
        &self,
        plugin_key: &str,
        local_plugins: &[(PathBuf, PluginManifest)],
        loaded_keys: &[String],
    ) -> (bool, String) {
        let global_plugins_dir = peekoo_global_data_dir()
            .map(|d| d.join("plugins"))
            .unwrap_or_default();

        for (plugin_dir, manifest) in local_plugins {
            if manifest.plugin.key == plugin_key {
                let source = if plugin_dir.starts_with(&global_plugins_dir) {
                    "store".to_string()
                } else {
                    "workspace".to_string()
                };
                let installed = loaded_keys.iter().any(|k| k == plugin_key);
                return (installed, source);
            }
        }

        (false, "none".to_string())
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

        self.download_directory_recursive(plugin_key, "", dest_dir, &contents)?;

        Ok(())
    }

    fn download_directory_recursive(
        &self,
        plugin_key: &str,
        sub_path: &str,
        dest_dir: &Path,
        contents: &[GitHubContent],
    ) -> Result<(), String> {
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
                    )?;
                }
                _ => {}
            }
        }

        Ok(())
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
            source: "store".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("pluginKey"));
        assert!(json.contains("toolCount"));
        assert!(json.contains("panelCount"));
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
            source: "none".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["installed"], false);
        assert_eq!(parsed["source"], "none");
    }

    #[test]
    fn check_installation_returns_not_installed_when_not_in_registry() {
        let store = make_store();
        let local_plugins: Vec<(PathBuf, PluginManifest)> = vec![];
        let loaded_keys: Vec<String> = vec![];
        let (installed, source) =
            store.check_installation("unknown-plugin", &local_plugins, &loaded_keys);
        assert!(!installed);
        assert_eq!(source, "none");
    }

    #[test]
    fn check_installation_detects_workspace_plugin() {
        let store = make_store();
        let manifest = parse_manifest(
            r#"
[plugin]
key = "my-plugin"
name = "My Plugin"
version = "1.0.0"
wasm = "plugin.wasm"
"#,
        )
        .unwrap();

        // Simulate a workspace path (not under global plugins dir)
        let workspace_dir = PathBuf::from("/home/user/myproject/plugins/my-plugin");
        let local_plugins = vec![(workspace_dir, manifest)];
        let loaded_keys = vec!["my-plugin".to_string()];

        let (installed, source) =
            store.check_installation("my-plugin", &local_plugins, &loaded_keys);
        assert!(installed);
        assert_eq!(source, "workspace");
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
        let loaded_keys = vec!["store-plugin".to_string()];

        let (installed, source) =
            store.check_installation("store-plugin", &local_plugins, &loaded_keys);
        assert!(installed);
        assert_eq!(source, "store");
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
}
