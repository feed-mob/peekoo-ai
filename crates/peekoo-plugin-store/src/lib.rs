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
use peekoo_plugin_host::{
    resolve_companion_install_path, CompanionDef, PluginManifest, PluginRegistry,
};

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
    pub permissions: Vec<String>,
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

        self.replace_installed_plugin(plugin_key, &dest_dir, registry, |dest_dir| {
            self.download_plugin_files(plugin_key, dest_dir)
        })?;

        let local_plugins = registry.discover();

        info!(plugin = plugin_key, "Plugin updated successfully");

        Ok(self.build_dto(remote_manifest, &local_plugins, false))
    }

    fn replace_installed_plugin<F>(
        &self,
        plugin_key: &str,
        dest_dir: &Path,
        registry: &Arc<PluginRegistry>,
        prepare_new_files: F,
    ) -> Result<(), String>
    where
        F: FnOnce(&Path) -> Result<(), String>,
    {
        let was_loaded = registry.loaded_keys().iter().any(|key| key == plugin_key);
        let old_manifest =
            peekoo_plugin_host::manifest::load_manifest(&dest_dir.join("peekoo-plugin.toml")).ok();

        if was_loaded {
            let _ = registry.unload_plugin(plugin_key);
        }

        let result = self.with_backup_guard(dest_dir, || {
            prepare_new_files(dest_dir)?;
            registry
                .install_plugin(dest_dir)
                .map(|_| ())
                .map_err(|e| format!("Failed to load updated plugin: {e}"))?;

            let new_manifest =
                peekoo_plugin_host::manifest::load_manifest(&dest_dir.join("peekoo-plugin.toml"))
                    .map_err(|e| format!("Failed to read updated manifest: {e}"))?;

            if let Some(old_manifest) = &old_manifest {
                Self::cleanup_removed_companions(
                    plugin_key,
                    dest_dir,
                    &old_manifest.companions,
                    &new_manifest.companions,
                );
            }

            Ok(())
        });

        if let Err(err) = result {
            if was_loaded {
                registry.install_plugin(dest_dir).map_err(|reload_err| {
                    format!("{err}; failed to reload restored plugin: {reload_err}")
                })?;
            }
            return Err(err);
        }

        Ok(())
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

        match peekoo_plugin_host::manifest::load_manifest(&plugin_dir.join("peekoo-plugin.toml")) {
            Ok(manifest) => {
                Self::cleanup_all_companions(plugin_key, &plugin_dir, &manifest.companions)
            }
            Err(err) => warn!(
                plugin = plugin_key,
                "Failed to read manifest during uninstall: {err}"
            ),
        }

        // Best-effort unload - plugin may not be in memory if initialization failed
        let _ = registry.unload_plugin(plugin_key);

        std::fs::remove_dir_all(&plugin_dir)
            .map_err(|e| format!("Failed to remove plugin directory: {e}"))?;

        info!(plugin = plugin_key, "Plugin uninstalled successfully");

        Ok(())
    }

    fn companion_paths(plugin_dir: &Path, companions: &[CompanionDef]) -> Vec<PathBuf> {
        companions
            .iter()
            .filter_map(|companion| resolve_companion_install_path(plugin_dir, companion))
            .collect()
    }

    fn cleanup_all_companions(plugin_key: &str, plugin_dir: &Path, companions: &[CompanionDef]) {
        for path in Self::companion_paths(plugin_dir, companions) {
            Self::remove_companion_file(plugin_key, &path);
        }
    }

    fn cleanup_removed_companions(
        plugin_key: &str,
        plugin_dir: &Path,
        old_companions: &[CompanionDef],
        new_companions: &[CompanionDef],
    ) {
        let old_paths = Self::companion_paths(plugin_dir, old_companions);
        let new_paths: std::collections::HashSet<PathBuf> =
            Self::companion_paths(plugin_dir, new_companions)
                .into_iter()
                .collect();

        for path in old_paths {
            if !new_paths.contains(&path) {
                Self::remove_companion_file(plugin_key, &path);
            }
        }
    }

    fn remove_companion_file(plugin_key: &str, path: &Path) {
        if !path.exists() {
            warn!(plugin = plugin_key, target = %path.display(), "Companion file already missing during cleanup");
            return;
        }

        if let Err(e) = std::fs::remove_file(path) {
            warn!(plugin = plugin_key, target = %path.display(), "Failed to remove companion file: {e}");
        }
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
        let permissions = manifest
            .permissions
            .as_ref()
            .map(|permissions| permissions.required.clone())
            .unwrap_or_default();
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
            permissions,
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
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::OnceLock;

    use peekoo_notifications::{MoodReactionService, NotificationService, PeekBadgeService};
    use peekoo_plugin_host::{resolve_companion_target, PluginRegistry};
    use peekoo_scheduler::Scheduler;
    use rusqlite::Connection;

    use super::*;
    use peekoo_plugin_host::manifest::parse_manifest;

    fn make_store() -> PluginStoreService {
        PluginStoreService::new()
    }

    fn make_registry(plugin_dirs: Vec<PathBuf>) -> Arc<PluginRegistry> {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            r#"
            CREATE TABLE plugins (
              id TEXT PRIMARY KEY,
              plugin_key TEXT NOT NULL,
              version TEXT NOT NULL,
              plugin_type TEXT NOT NULL,
              enabled INTEGER NOT NULL DEFAULT 1,
              manifest_json TEXT NOT NULL,
              installed_at TEXT NOT NULL
            );

            CREATE TABLE plugin_permissions (
              id TEXT PRIMARY KEY,
              plugin_id TEXT NOT NULL,
              capability TEXT NOT NULL,
              granted INTEGER NOT NULL
            );

            CREATE TABLE plugin_state (
              id TEXT PRIMARY KEY,
              plugin_id TEXT NOT NULL,
              state_key TEXT NOT NULL,
              value_json TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            "#,
        )
        .expect("plugin schema");

        let scheduler = Arc::new(Scheduler::new());
        let (notifications, _receiver) = NotificationService::new();
        Arc::new(PluginRegistry::new(
            plugin_dirs,
            Arc::new(Mutex::new(conn)),
            scheduler,
            Arc::new(notifications),
            Arc::new(PeekBadgeService::new()),
            Arc::new(MoodReactionService::new()),
        ))
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("peekoo-store-{prefix}-{nanos}"));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn with_test_config_home<T>(name: &str, f: impl FnOnce(PathBuf) -> T) -> T {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock");
        let config_home = temp_dir(name).join("config-home");
        fs::create_dir_all(&config_home).expect("config home");

        let old = std::env::var("XDG_CONFIG_HOME").ok();
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &config_home) };
        let result = f(config_home.clone());
        if let Some(value) = old {
            unsafe { std::env::set_var("XDG_CONFIG_HOME", value) };
        } else {
            unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        }
        result
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) {
        fs::create_dir_all(dst).expect("create dst dir");
        for entry in fs::read_dir(src).expect("read dir") {
            let entry = entry.expect("dir entry");
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            let file_type = entry.file_type().expect("file type");
            if file_type.is_dir() {
                copy_dir_recursive(&src_path, &dst_path);
            } else {
                fs::copy(&src_path, &dst_path).expect("copy file");
            }
        }
    }

    fn write_plugin_with_companion(plugin_dir: &Path, key: &str, version: &str) {
        fs::create_dir_all(plugin_dir.join("companions")).expect("companions dir");
        let wasm_src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins/peekoo-opencode-companion/target/wasm32-wasip1/release/peekoo_opencode_companion.wasm");
        let wasm_dst =
            plugin_dir.join("target/wasm32-wasip1/release/peekoo_opencode_companion.wasm");
        fs::create_dir_all(wasm_dst.parent().expect("wasm parent")).expect("wasm dir");
        fs::copy(wasm_src, &wasm_dst).expect("copy wasm");
        fs::write(
            plugin_dir.join("peekoo-plugin.toml"),
            format!(
                r#"[plugin]
key = "{key}"
name = "OpenCode Companion"
version = "{version}"
wasm = "target/wasm32-wasip1/release/peekoo_opencode_companion.wasm"

[permissions]
required = ["bridge:fs_read", "notifications", "pet:mood", "scheduler", "state:read", "state:write"]

[[companions]]
source = "companions/{key}.js"
target = "opencode-plugin"
"#
            ),
        )
        .expect("manifest");
        fs::write(
            plugin_dir.join(format!("companions/{key}.js")),
            format!("console.log('v{version}')"),
        )
        .expect("companion file");
    }

    #[test]
    fn store_plugin_dto_serializes_to_camel_case() {
        let dto = StorePluginDto {
            plugin_key: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            author: Some("Test Author".to_string()),
            description: Some("A test plugin".to_string()),
            permissions: vec!["notifications".to_string(), "scheduler".to_string()],
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
        assert!(json.contains("permissions"));
    }

    #[test]
    fn store_plugin_dto_not_installed_has_none_source() {
        let dto = StorePluginDto {
            plugin_key: "example-minimal".to_string(),
            name: "Example Minimal".to_string(),
            version: "0.1.0".to_string(),
            author: None,
            description: None,
            permissions: vec![],
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
        assert_eq!(parsed["permissions"], serde_json::json!([]));
    }

    #[test]
    fn build_dto_includes_required_permissions() {
        let store = make_store();
        let manifest = parse_manifest(
            r#"
[plugin]
key = "secure-plugin"
name = "Secure Plugin"
version = "0.2.0"
wasm = "plugin.wasm"

[permissions]
required = ["bridge:fs_read", "pet:mood", "scheduler"]
optional = ["notifications"]
"#,
        )
        .unwrap();

        let dto = store.build_dto(manifest, &[], false);

        assert_eq!(
            dto.permissions,
            vec![
                "bridge:fs_read".to_string(),
                "pet:mood".to_string(),
                "scheduler".to_string()
            ]
        );
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
        let scheduler = Arc::new(Scheduler::new());
        let (notifications, _receiver) = NotificationService::new();
        let result = store.uninstall_plugin(
            "nonexistent-plugin",
            &Arc::new(PluginRegistry::new(
                vec![],
                Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
                scheduler,
                Arc::new(notifications),
                Arc::new(PeekBadgeService::new()),
                Arc::new(MoodReactionService::new()),
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
        let scheduler = Arc::new(Scheduler::new());
        let (notifications, _receiver) = NotificationService::new();
        let result = store.update_plugin(
            "nonexistent-plugin",
            &Arc::new(PluginRegistry::new(
                vec![],
                Arc::new(Mutex::new(Connection::open_in_memory().unwrap())),
                scheduler,
                Arc::new(notifications),
                Arc::new(PeekBadgeService::new()),
                Arc::new(MoodReactionService::new()),
            )),
        );
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("is not installed"), "unexpected error: {msg}");
    }

    #[test]
    fn failed_replace_reloads_previous_plugin_after_backup_restore() {
        let store = make_store();
        let temp_root = temp_dir("replace-reload");
        let plugin_dir = temp_root.join("example-minimal");
        let sample_plugin_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/example-minimal");
        copy_dir_recursive(&sample_plugin_dir, &plugin_dir);

        let registry = make_registry(vec![temp_root.clone()]);
        registry
            .install_plugin(&plugin_dir)
            .expect("initial plugin install should succeed");
        assert!(registry
            .loaded_keys()
            .iter()
            .any(|key| key == "example-minimal"));

        let err = store
            .replace_installed_plugin("example-minimal", &plugin_dir, &registry, |_| {
                Err("simulated update failure".to_string())
            })
            .expect_err("replace should fail");

        assert!(err.contains("simulated update failure"));
        assert!(
            registry
                .loaded_keys()
                .iter()
                .any(|key| key == "example-minimal"),
            "restored plugin should be reloaded into memory after rollback"
        );
    }

    #[test]
    fn uninstall_plugin_removes_declared_companion_file_only() {
        with_test_config_home("uninstall-companion", |_| {
            let store = make_store();
            let global_plugins_dir = peekoo_global_data_dir()
                .expect("global dir")
                .join("plugins");
            let plugin_key = format!("test-opencode-uninstall-{}", std::process::id());
            let plugin_dir = global_plugins_dir.join(&plugin_key);
            write_plugin_with_companion(&plugin_dir, &plugin_key, "0.1.0");

            let target_dir = resolve_companion_target("opencode-plugin").expect("companion target");
            fs::create_dir_all(&target_dir).expect("target dir");
            let companion_path = target_dir.join(format!("{plugin_key}.js"));
            let unrelated_path = target_dir.join("user-plugin.js");
            fs::write(&companion_path, "companion").expect("companion install");
            fs::write(&unrelated_path, "keep me").expect("unrelated file");

            let registry = make_registry(vec![global_plugins_dir.clone()]);

            store
                .uninstall_plugin(&plugin_key, &registry)
                .expect("uninstall should succeed");

            assert!(!plugin_dir.exists(), "plugin directory should be removed");
            assert!(
                !companion_path.exists(),
                "declared companion file should be removed"
            );
            assert!(
                unrelated_path.exists(),
                "unrelated files in target directory must be preserved"
            );

            let _ = fs::remove_file(unrelated_path);
            let _ = fs::remove_dir_all(&plugin_dir);
        });
    }

    #[test]
    fn update_plugin_removes_stale_companion_file_when_manifest_drops_it() {
        with_test_config_home("update-stale-companion", |_| {
            let store = make_store();
            let temp_root = temp_dir("update-stale-plugin-root");
            let plugin_key = format!("test-opencode-update-{}", std::process::id());
            let plugin_dir = temp_root.join(&plugin_key);
            write_plugin_with_companion(&plugin_dir, &plugin_key, "0.1.0");

            let target_dir = resolve_companion_target("opencode-plugin").expect("companion target");
            fs::create_dir_all(&target_dir).expect("target dir");
            let companion_path = target_dir.join(format!("{plugin_key}.js"));
            fs::write(&companion_path, "old-companion").expect("companion install");

            let registry = make_registry(vec![temp_root.clone()]);

            store
                .replace_installed_plugin(&plugin_key, &plugin_dir, &registry, |dest_dir| {
                    let wasm_src = Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../plugins/peekoo-opencode-companion/target/wasm32-wasip1/release/peekoo_opencode_companion.wasm");
                    let wasm_dst = dest_dir.join("target/wasm32-wasip1/release/peekoo_opencode_companion.wasm");
                    fs::create_dir_all(wasm_dst.parent().expect("wasm parent"))
                        .map_err(|e| e.to_string())?;
                    fs::copy(wasm_src, &wasm_dst).map_err(|e| e.to_string())?;

                    fs::write(
                        dest_dir.join("peekoo-plugin.toml"),
                        format!(
                            r#"[plugin]
key = "{plugin_key}"
name = "OpenCode Companion"
version = "0.2.0"
wasm = "target/wasm32-wasip1/release/peekoo_opencode_companion.wasm"

[permissions]
required = ["bridge:fs_read", "notifications", "pet:mood", "scheduler", "state:read", "state:write"]
"#
                        ),
                    )
                    .map_err(|e| e.to_string())?;
                    Ok(())
                })
                .expect("update should succeed");

            assert!(
                !companion_path.exists(),
                "stale companion file should be removed when no longer declared"
            );
        });
    }
}
