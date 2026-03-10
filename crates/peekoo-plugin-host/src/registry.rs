use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use peekoo_notifications::NotificationService;
use peekoo_scheduler::Scheduler;
use rusqlite::{Connection, OptionalExtension};

use crate::config::{resolved_config_map, set_config_field};
use crate::error::PluginError;
use crate::events::{EventBus, PluginEvent};
use crate::host_functions;
use crate::manifest::{self, ConfigFieldDef, PluginManifest, ToolDefinition, UiPanelDef};
use crate::permissions::PermissionStore;
use crate::runtime::PluginInstance;
use crate::state::PluginStateStore;

const DEFAULT_MEMORY_MAX_PAGES: u32 = 256;
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(5000);

pub struct PluginRegistry {
    plugins: Mutex<HashMap<String, PluginInstance>>,
    plugin_dirs: Vec<PathBuf>,
    permissions: PermissionStore,
    state: PluginStateStore,
    event_bus: Arc<EventBus>,
    db_conn: Arc<Mutex<Connection>>,
    scheduler: Arc<Scheduler>,
    notifications: Arc<NotificationService>,
    scheduler_started: AtomicBool,
    scheduler_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl PluginRegistry {
    pub fn new(
        plugin_dirs: Vec<PathBuf>,
        db_conn: Arc<Mutex<Connection>>,
        scheduler: Arc<Scheduler>,
        notifications: Arc<NotificationService>,
    ) -> Self {
        let permissions = PermissionStore::new(Arc::clone(&db_conn));
        let state = PluginStateStore::new(Arc::clone(&db_conn));

        Self {
            plugins: Mutex::new(HashMap::new()),
            plugin_dirs,
            permissions,
            state,
            event_bus: Arc::new(EventBus::new()),
            db_conn,
            scheduler,
            notifications,
            scheduler_started: AtomicBool::new(false),
            scheduler_handle: Mutex::new(None),
        }
    }

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
                            Err(e) => tracing::warn!(
                                path = %manifest_path.display(),
                                "Failed to parse plugin manifest: {e}"
                            ),
                        }
                    }
                }
            }
        }
        found
    }

    pub fn load_plugin(&self, plugin_dir: &Path) -> Result<String, PluginError> {
        let manifest_path = plugin_dir.join("peekoo-plugin.toml");
        let manifest = manifest::load_manifest(&manifest_path)?;
        let key = manifest.plugin.key.clone();

        if self
            .loaded_keys()
            .iter()
            .any(|loaded_key| loaded_key == &key)
        {
            return Ok(key);
        }

        self.ensure_plugin_row(&key, &manifest)?;
        self.permissions.check_required(&key, &manifest)?;

        let host_fns = host_functions::build_host_functions(
            &key,
            &self.state,
            &self.event_bus,
            &self.scheduler,
            &self.notifications,
            manifest
                .config
                .as_ref()
                .map(|config| config.fields.clone())
                .unwrap_or_default(),
        );

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
        Ok(key)
    }

    pub fn install_plugin(&self, plugin_dir: &Path) -> Result<String, PluginError> {
        let manifest_path = plugin_dir.join("peekoo-plugin.toml");
        let manifest = manifest::load_manifest(&manifest_path)?;
        let key = manifest.plugin.key.clone();

        self.ensure_plugin_row(&key, &manifest)?;
        self.permissions.grant_all_required(&key, &manifest)?;
        self.set_plugin_enabled(&key, true)?;

        self.load_plugin(plugin_dir)
    }

    pub fn sync_plugin_registration(
        &self,
        plugin_dir: &Path,
    ) -> Result<PluginManifest, PluginError> {
        let manifest_path = plugin_dir.join("peekoo-plugin.toml");
        let manifest = manifest::load_manifest(&manifest_path)?;
        self.ensure_plugin_row(&manifest.plugin.key, &manifest)?;
        Ok(manifest)
    }

    pub fn is_plugin_enabled(&self, plugin_key: &str) -> Result<bool, PluginError> {
        let conn = self
            .db_conn
            .lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        let enabled = conn
            .query_row(
                "SELECT enabled FROM plugins WHERE plugin_key = ?1",
                rusqlite::params![plugin_key],
                |row| row.get::<_, bool>(0),
            )
            .optional()
            .map_err(|e| PluginError::Internal(e.to_string()))?
            .ok_or_else(|| PluginError::NotFound(plugin_key.to_string()))?;

        Ok(enabled)
    }

    pub fn set_plugin_enabled(&self, plugin_key: &str, enabled: bool) -> Result<(), PluginError> {
        let conn = self
            .db_conn
            .lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        let updated = conn
            .execute(
                "UPDATE plugins SET enabled = ?2 WHERE plugin_key = ?1",
                rusqlite::params![plugin_key, enabled],
            )
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        if updated == 0 {
            return Err(PluginError::NotFound(plugin_key.to_string()));
        }

        Ok(())
    }

    pub fn unload_plugin(&self, key: &str) -> Result<(), PluginError> {
        let mut plugins = self
            .plugins
            .lock()
            .map_err(|e| PluginError::Internal(format!("Lock error: {e}")))?;
        if plugins.remove(key).is_some() {
            self.scheduler.cancel_all(key);
            Ok(())
        } else {
            Err(PluginError::NotFound(key.to_string()))
        }
    }

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

    pub fn dispatch_event(&self, event_name: &str, payload_json: &str) -> Vec<PluginEvent> {
        let mut plugins = match self.plugins.lock() {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Lock error dispatching event: {e}");
                return Vec::new();
            }
        };

        for (key, instance) in plugins.iter_mut() {
            let subscribes = instance
                .manifest
                .events
                .as_ref()
                .is_some_and(|events| events.subscribe.iter().any(|name| name == event_name));
            if subscribes && let Err(e) = instance.handle_event(event_name, payload_json) {
                tracing::warn!(
                    plugin = key.as_str(),
                    event = event_name,
                    "Event handler error: {e}"
                );
            }
        }

        drop(plugins);
        self.drain_events()
    }

    pub fn dispatch_event_to_plugin(
        &self,
        plugin_key: &str,
        event_name: &str,
        payload_json: &str,
    ) -> Result<Vec<PluginEvent>, PluginError> {
        let mut plugins = self
            .plugins
            .lock()
            .map_err(|e| PluginError::Internal(format!("Lock error: {e}")))?;
        let instance = plugins
            .get_mut(plugin_key)
            .ok_or_else(|| PluginError::NotFound(plugin_key.to_string()))?;

        instance.handle_event(event_name, payload_json)?;
        drop(plugins);
        Ok(self.drain_events())
    }

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

    pub fn panel_entry_path(&self, label: &str) -> Option<PathBuf> {
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

    pub fn tool_owner(&self, tool_name: &str) -> Option<String> {
        self.all_tool_definitions()
            .into_iter()
            .find(|(_, def)| def.name == tool_name)
            .map(|(key, _)| key)
    }

    pub fn drain_events(&self) -> Vec<PluginEvent> {
        self.event_bus.drain()
    }

    pub fn permissions(&self) -> &PermissionStore {
        &self.permissions
    }

    pub fn loaded_keys(&self) -> Vec<String> {
        match self.plugins.lock() {
            Ok(p) => p.keys().cloned().collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn start_scheduler(self: &Arc<Self>) {
        if self
            .scheduler_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }

        let registry = Arc::clone(self);
        let handle = self.scheduler.start(move |owner, key| {
            let payload = serde_json::json!({ "key": key });
            if let Err(err) =
                registry.dispatch_event_to_plugin(&owner, "schedule:fired", &payload.to_string())
            {
                tracing::warn!(plugin = owner.as_str(), "Scheduler dispatch error: {err}");
            }
        });

        if let Ok(mut guard) = self.scheduler_handle.lock() {
            *guard = Some(handle);
        }
    }

    pub fn notifications(&self) -> Arc<NotificationService> {
        Arc::clone(&self.notifications)
    }

    pub fn scheduler(&self) -> Arc<Scheduler> {
        Arc::clone(&self.scheduler)
    }

    pub fn config_schema(&self, plugin_key: &str) -> Result<Vec<ConfigFieldDef>, PluginError> {
        Ok(self
            .manifest_for(plugin_key)
            .ok_or_else(|| PluginError::NotFound(plugin_key.to_string()))?
            .config
            .map(|config| config.fields)
            .unwrap_or_default())
    }

    pub fn config_values(&self, plugin_key: &str) -> Result<serde_json::Value, PluginError> {
        let fields = self.config_schema(plugin_key)?;
        Ok(serde_json::Value::Object(resolved_config_map(
            &self.state,
            plugin_key,
            &fields,
        )?))
    }

    pub fn set_config_value(
        &self,
        plugin_key: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), PluginError> {
        let fields = self.config_schema(plugin_key)?;
        set_config_field(&self.state, plugin_key, &fields, key, value)
    }

    fn manifest_for(&self, plugin_key: &str) -> Option<PluginManifest> {
        if let Ok(plugins) = self.plugins.lock()
            && let Some(instance) = plugins.get(plugin_key)
        {
            return Some(instance.manifest.clone());
        }

        self.discover()
            .into_iter()
            .find(|(_, manifest)| manifest.plugin.key == plugin_key)
            .map(|(_, manifest)| manifest)
    }

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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use peekoo_notifications::NotificationService;
    use peekoo_scheduler::Scheduler;
    use rusqlite::Connection;

    use super::PluginRegistry;

    fn test_registry(plugin_dirs: Vec<std::path::PathBuf>) -> PluginRegistry {
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
        PluginRegistry::new(
            plugin_dirs,
            Arc::new(Mutex::new(conn)),
            scheduler,
            Arc::new(notifications),
        )
    }

    fn sample_plugin_dir(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins")
            .join(name)
    }

    #[test]
    fn discovered_plugin_defaults_to_enabled_when_registered() {
        let plugin_dir = sample_plugin_dir("health-reminders");
        let registry = test_registry(vec![plugin_dir.clone()]);

        let manifest = registry
            .sync_plugin_registration(&plugin_dir)
            .expect("plugin should register");

        assert_eq!(manifest.plugin.key, "health-reminders");
        assert!(
            registry
                .is_plugin_enabled("health-reminders")
                .expect("enabled state should exist")
        );
    }

    #[test]
    fn disabling_plugin_persists_enabled_state() {
        let plugin_dir = sample_plugin_dir("health-reminders");
        let registry = test_registry(vec![plugin_dir.clone()]);

        registry
            .sync_plugin_registration(&plugin_dir)
            .expect("plugin should register");
        registry
            .set_plugin_enabled("health-reminders", false)
            .expect("plugin should disable");

        assert!(
            !registry
                .is_plugin_enabled("health-reminders")
                .expect("enabled state should exist")
        );
    }
}
