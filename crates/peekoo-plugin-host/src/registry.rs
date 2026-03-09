use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::Connection;

use crate::error::PluginError;
use crate::events::EventBus;
use crate::host_functions;
use crate::manifest::{self, PluginManifest, ToolDefinition, UiPanelDef};
use crate::permissions::PermissionStore;
use crate::runtime::PluginInstance;
use crate::state::PluginStateStore;

const DEFAULT_MEMORY_MAX_PAGES: u32 = 256; // 16 MiB
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(5000);

/// Central registry that discovers, loads, and manages plugin instances.
pub struct PluginRegistry {
    plugins: Mutex<HashMap<String, PluginInstance>>,
    plugin_dirs: Vec<PathBuf>,
    permissions: PermissionStore,
    state: PluginStateStore,
    event_bus: Arc<EventBus>,
    db_conn: Arc<Mutex<Connection>>,
}

impl PluginRegistry {
    pub fn new(plugin_dirs: Vec<PathBuf>, db_conn: Arc<Mutex<Connection>>) -> Self {
        let permissions = PermissionStore::new(Arc::clone(&db_conn));
        let state = PluginStateStore::new(Arc::clone(&db_conn));

        Self {
            plugins: Mutex::new(HashMap::new()),
            plugin_dirs,
            permissions,
            state,
            event_bus: Arc::new(EventBus::new()),
            db_conn,
        }
    }

    /// Scan all plugin directories and return discovered manifests.
    pub fn discover(&self) -> Vec<(PathBuf, PluginManifest)> {
        let mut found = Vec::new();
        for dir in &self.plugin_dirs {
            if !dir.is_dir() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let plugin_dir = entry.path();
                    let manifest_path = plugin_dir.join("peekoo-plugin.toml");
                    if manifest_path.is_file() {
                        match manifest::load_manifest(&manifest_path) {
                            Ok(m) => found.push((plugin_dir, m)),
                            Err(e) => {
                                tracing::warn!(
                                    path = %manifest_path.display(),
                                    "Failed to parse plugin manifest: {e}"
                                );
                            }
                        }
                    }
                }
            }
        }
        found
    }

    /// Load and initialize a plugin from a directory.
    ///
    /// The plugin is registered in the `plugins` table if it is not already
    /// present, and its required permissions are checked.
    pub fn load_plugin(&self, plugin_dir: &Path) -> Result<String, PluginError> {
        let manifest_path = plugin_dir.join("peekoo-plugin.toml");
        let manifest = manifest::load_manifest(&manifest_path)?;
        let key = manifest.plugin.key.clone();

        // Ensure the plugin row exists in the DB so permission / state queries work.
        self.ensure_plugin_row(&key, &manifest)?;

        // Check required permissions are granted.
        self.permissions.check_required(&key, &manifest)?;

        // Build host functions for this plugin.
        let host_fns = host_functions::build_host_functions(&key, &self.state, &self.event_bus);

        let mut instance = PluginInstance::load(
            manifest,
            plugin_dir.to_path_buf(),
            host_fns,
            DEFAULT_MEMORY_MAX_PAGES,
            DEFAULT_TIMEOUT,
        )?;

        instance.initialize()?;

        let mut plugins = self
            .plugins
            .lock()
            .map_err(|e| PluginError::Internal(format!("Lock error: {e}")))?;
        plugins.insert(key.clone(), instance);

        tracing::info!(plugin = key.as_str(), "Plugin loaded");
        Ok(key)
    }

    /// Install a plugin into the registry by ensuring its DB row exists,
    /// granting all required permissions, and then loading it.
    pub fn install_plugin(&self, plugin_dir: &Path) -> Result<String, PluginError> {
        let manifest_path = plugin_dir.join("peekoo-plugin.toml");
        let manifest = manifest::load_manifest(&manifest_path)?;
        let key = manifest.plugin.key.clone();

        self.ensure_plugin_row(&key, &manifest)?;
        self.permissions.grant_all_required(&key, &manifest)?;

        self.load_plugin(plugin_dir)
    }

    /// Unload a plugin by key.
    pub fn unload_plugin(&self, key: &str) -> Result<(), PluginError> {
        let mut plugins = self
            .plugins
            .lock()
            .map_err(|e| PluginError::Internal(format!("Lock error: {e}")))?;
        if plugins.remove(key).is_some() {
            tracing::info!(plugin = key, "Plugin unloaded");
            Ok(())
        } else {
            Err(PluginError::NotFound(key.to_string()))
        }
    }

    /// Call a tool on a specific plugin.
    pub fn call_tool(
        &self,
        plugin_key: &str,
        tool_name: &str,
        input_json: &str,
    ) -> Result<String, PluginError> {
        let mut plugins = self
            .plugins
            .lock()
            .map_err(|e| PluginError::Internal(format!("Lock error: {e}")))?;
        let instance = plugins
            .get_mut(plugin_key)
            .ok_or_else(|| PluginError::NotFound(plugin_key.to_string()))?;
        instance.call_tool(tool_name, input_json)
    }

    /// Dispatch an event to all plugins that subscribe to it.
    pub fn dispatch_event(&self, event_name: &str, payload_json: &str) {
        let mut plugins = match self.plugins.lock() {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Lock error dispatching event: {e}");
                return;
            }
        };
        for (key, instance) in plugins.iter_mut() {
            let subscribes = instance
                .manifest
                .events
                .as_ref()
                .is_some_and(|e| e.subscribe.iter().any(|s| s == event_name));
            if subscribes && let Err(e) = instance.handle_event(event_name, payload_json) {
                tracing::warn!(
                    plugin = key.as_str(),
                    event = event_name,
                    "Event handler error: {e}"
                );
            }
        }
    }

    /// Query a data provider from a specific plugin.
    pub fn query_data(&self, plugin_key: &str, provider_name: &str) -> Result<String, PluginError> {
        let mut plugins = self
            .plugins
            .lock()
            .map_err(|e| PluginError::Internal(format!("Lock error: {e}")))?;
        let instance = plugins
            .get_mut(plugin_key)
            .ok_or_else(|| PluginError::NotFound(plugin_key.to_string()))?;
        instance.query_data(provider_name)
    }

    /// Return all tool definitions across all loaded plugins.
    pub fn all_tool_definitions(&self) -> Vec<(String, ToolDefinition)> {
        let plugins = match self.plugins.lock() {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut tools = Vec::new();
        for (key, instance) in plugins.iter() {
            if let Some(tools_block) = &instance.manifest.tools {
                for def in &tools_block.definitions {
                    tools.push((key.clone(), def.clone()));
                }
            }
        }
        tools
    }

    /// Return all UI panel definitions across all loaded plugins.
    pub fn all_ui_panels(&self) -> Vec<(String, UiPanelDef)> {
        let plugins = match self.plugins.lock() {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut panels = Vec::new();
        for (key, instance) in plugins.iter() {
            if let Some(ui_block) = &instance.manifest.ui {
                for panel in &ui_block.panels {
                    panels.push((key.clone(), panel.clone()));
                }
            }
        }
        panels
    }

    /// Return all UI panel definitions across all discovered plugins (on-disk),
    /// not just runtime-loaded ones.
    pub fn all_discovered_ui_panels(&self) -> Vec<(String, UiPanelDef)> {
        let mut panels = Vec::new();
        for (_, manifest) in self.discover() {
            if let Some(ui_block) = &manifest.ui {
                for panel in &ui_block.panels {
                    panels.push((manifest.plugin.key.clone(), panel.clone()));
                }
            }
        }
        panels
    }

    /// Resolve the HTML entry path for a panel label.
    ///
    /// Checks loaded plugins first, then falls back to discovered plugins
    /// so panels remain accessible even when the WASM runtime failed to load.
    pub fn panel_entry_path(&self, label: &str) -> Option<PathBuf> {
        // Check loaded plugins first (fast path).
        if let Ok(plugins) = self.plugins.lock() {
            for instance in plugins.values() {
                if let Some(ui_block) = &instance.manifest.ui {
                    for panel in &ui_block.panels {
                        if panel.label == label {
                            return Some(instance.plugin_dir.join(&panel.entry));
                        }
                    }
                }
            }
        }

        // Fall back to discovered plugins (on-disk manifests).
        for (plugin_dir, manifest) in self.discover() {
            if let Some(ui_block) = &manifest.ui {
                for panel in &ui_block.panels {
                    if panel.label == label {
                        return Some(plugin_dir.join(&panel.entry));
                    }
                }
            }
        }

        None
    }

    /// Return the plugin key that owns a given tool name.
    pub fn tool_owner(&self, tool_name: &str) -> Option<String> {
        self.all_tool_definitions()
            .into_iter()
            .find(|(_, def)| def.name == tool_name)
            .map(|(key, _)| key)
    }

    /// Drain plugin-emitted events from the bus.
    pub fn drain_events(&self) -> Vec<crate::events::PluginEvent> {
        self.event_bus.drain()
    }

    /// Access the permission store (e.g. to grant permissions during install).
    pub fn permissions(&self) -> &PermissionStore {
        &self.permissions
    }

    /// Return the list of loaded plugin keys.
    pub fn loaded_keys(&self) -> Vec<String> {
        match self.plugins.lock() {
            Ok(p) => p.keys().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    // ── Private helpers ─────────────────────────────────────────────────────

    /// Make sure a row exists in the `plugins` table for this key.
    fn ensure_plugin_row(
        &self,
        plugin_key: &str,
        manifest: &PluginManifest,
    ) -> Result<(), PluginError> {
        let conn = self
            .db_conn
            .lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM plugins WHERE plugin_key = ?1",
                rusqlite::params![plugin_key],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        if !exists {
            let manifest_json = serde_json::json!({
                "name": manifest.plugin.name,
                "version": manifest.plugin.version,
                "author": manifest.plugin.author,
                "description": manifest.plugin.description,
            })
            .to_string();

            conn.execute(
                "INSERT INTO plugins (id, plugin_key, version, plugin_type, enabled, manifest_json, installed_at)
                 VALUES (?1, ?2, ?3, 'wasm', 1, ?4, datetime('now'))",
                rusqlite::params![
                    uuid::Uuid::new_v4().to_string(),
                    plugin_key,
                    manifest.plugin.version,
                    manifest_json,
                ],
            )
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        }

        Ok(())
    }
}

/// Start a background thread that emits `timer:tick` events every 60 seconds.
pub fn start_tick_timer(registry: Arc<PluginRegistry>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60));
            let payload = serde_json::json!({});
            registry.dispatch_event("timer:tick", &payload.to_string());
        }
    });
}
