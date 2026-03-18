use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use peekoo_notifications::{MoodReactionService, NotificationService, PeekBadgeService};
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
    peek_badges: Arc<PeekBadgeService>,
    mood_reactions: Arc<MoodReactionService>,
    scheduler_started: AtomicBool,
    scheduler_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl PluginRegistry {
    pub fn new(
        plugin_dirs: Vec<PathBuf>,
        db_conn: Arc<Mutex<Connection>>,
        scheduler: Arc<Scheduler>,
        notifications: Arc<NotificationService>,
        peek_badges: Arc<PeekBadgeService>,
        mood_reactions: Arc<MoodReactionService>,
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
            peek_badges,
            mood_reactions,
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

        let declared_capabilities = manifest
            .permissions
            .as_ref()
            .map(|permissions| {
                permissions
                    .required
                    .iter()
                    .chain(permissions.optional.iter())
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let allowed_paths = manifest
            .permissions
            .as_ref()
            .map(|permissions| {
                permissions
                    .allowed_paths
                    .iter()
                    .filter_map(|path| {
                        let expanded = host_functions::expand_tilde_path(path);
                        std::fs::canonicalize(&expanded).ok()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let host_fns = host_functions::build_host_functions(
            &key,
            &self.state,
            &self.permissions,
            declared_capabilities,
            allowed_paths,
            &self.event_bus,
            &self.scheduler,
            &self.notifications,
            &self.peek_badges,
            &self.mood_reactions,
            manifest
                .config
                .as_ref()
                .map(|config| config.fields.clone())
                .unwrap_or_default(),
        );

        let companions = manifest.companions.clone();

        let mut instance = PluginInstance::load(
            manifest,
            plugin_dir.to_path_buf(),
            host_fns,
            DEFAULT_MEMORY_MAX_PAGES,
            DEFAULT_TIMEOUT,
        )?;
        instance.initialize()?;

        install_companion_files(&key, plugin_dir, &companions);

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

        match self.load_plugin(plugin_dir) {
            Ok(loaded_key) => {
                self.set_plugin_enabled(&loaded_key, true)?;
                Ok(loaded_key)
            }
            Err(err) => {
                let _ = self.set_plugin_enabled(&key, false);
                Err(err)
            }
        }
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
            self.peek_badges.clear(key);
            self.peek_badges.refresh();
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
        let handle = self.scheduler.start_with_wake_handler(
            move |owner, key| {
                let payload = serde_json::json!({ "key": key });
                if let Err(err) = registry.dispatch_event_to_plugin(
                    &owner,
                    "schedule:fired",
                    &payload.to_string(),
                ) {
                    tracing::warn!(plugin = owner.as_str(), "Scheduler dispatch error: {err}");
                }
            },
            {
                let registry = Arc::clone(self);
                move |owner| {
                    if let Err(err) = registry.dispatch_event_to_plugin(&owner, "system:wake", "{}")
                    {
                        tracing::warn!(plugin = owner.as_str(), "Wake dispatch error: {err}");
                    }
                }
            },
        );

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

    pub fn peek_badges(&self) -> Arc<PeekBadgeService> {
        Arc::clone(&self.peek_badges)
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

        let manifest_json = serde_json::json!({
            "name": manifest.plugin.name,
            "version": manifest.plugin.version,
            "author": manifest.plugin.author,
            "description": manifest.plugin.description,
        })
        .to_string();

        if !exists {
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
        } else {
            conn.execute(
                "UPDATE plugins SET version = ?2, manifest_json = ?3 WHERE plugin_key = ?1",
                rusqlite::params![plugin_key, manifest.plugin.version, manifest_json],
            )
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        }

        Ok(())
    }
}

/// Resolve a well-known companion target ID to a filesystem directory.
///
/// Returns `None` for unrecognised target IDs.
pub fn resolve_companion_target(target: &str) -> Option<PathBuf> {
    match target {
        "opencode-plugin" => {
            let config_base = resolve_config_base_dir()?;
            Some(config_base.join("opencode").join("plugin"))
        }
        _ => None,
    }
}

/// Resolve the platform-appropriate base config directory.
///
/// Respects `XDG_CONFIG_HOME` on Linux, `APPDATA` on Windows, and falls
/// back to `~/.config` on Unix systems.
fn resolve_config_base_dir() -> Option<PathBuf> {
    // XDG_CONFIG_HOME takes priority on any platform
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(&xdg);
        if p.is_absolute() {
            return Some(p);
        }
    }

    // Windows: use APPDATA
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return Some(PathBuf::from(appdata));
        }
    }

    // Unix fallback: ~/.config
    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".config"))
    }

    #[cfg(windows)]
    None
}

fn resolve_companion_filename(
    companion: &manifest::CompanionDef,
    source_path: Option<&Path>,
) -> Option<String> {
    let raw_filename = companion.filename.as_deref().unwrap_or_else(|| {
        source_path
            .and_then(|path| path.file_name())
            .or_else(|| Path::new(&companion.source).file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("companion")
    });

    Path::new(raw_filename)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|name| !name.is_empty() && *name != "." && *name != "..")
        .map(ToString::to_string)
}

pub fn resolve_companion_install_path(
    plugin_dir: &Path,
    companion: &manifest::CompanionDef,
) -> Option<PathBuf> {
    let target_dir = resolve_companion_target(&companion.target)?;
    let source_path = plugin_dir.join(&companion.source);
    let filename = resolve_companion_filename(companion, Some(&source_path))?;
    Some(target_dir.join(filename))
}

/// Copy companion files declared in the manifest to their target directories.
///
/// Errors are logged as warnings but do not prevent plugin loading.
///
/// **Security:** Both the source path and target filename are validated to
/// prevent path-traversal attacks:
/// - The source path is canonicalized and must reside under `plugin_dir`.
/// - The filename is stripped to its final component (no `/`, `\`, `..`).
fn install_companion_files(
    plugin_key: &str,
    plugin_dir: &Path,
    companions: &[manifest::CompanionDef],
) {
    // Canonicalize the plugin directory once for prefix checks.
    let canonical_plugin_dir = match plugin_dir.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                plugin = plugin_key,
                dir = %plugin_dir.display(),
                "Cannot canonicalize plugin directory, skipping companions: {e}"
            );
            return;
        }
    };

    for companion in companions {
        let target_dir = match resolve_companion_target(&companion.target) {
            Some(dir) => dir,
            None => {
                tracing::warn!(
                    plugin = plugin_key,
                    target = companion.target.as_str(),
                    "Unknown companion target, skipping"
                );
                continue;
            }
        };

        // ── Validate source path (prevent reading outside plugin dir) ──
        let source_path = plugin_dir.join(&companion.source);
        if !source_path.exists() {
            tracing::warn!(
                plugin = plugin_key,
                source = %source_path.display(),
                "Companion source file not found, skipping"
            );
            continue;
        }

        let canonical_source = match source_path.canonicalize() {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    plugin = plugin_key,
                    source = %source_path.display(),
                    "Cannot canonicalize companion source path: {e}"
                );
                continue;
            }
        };

        if !canonical_source.starts_with(&canonical_plugin_dir) {
            tracing::warn!(
                plugin = plugin_key,
                source = %companion.source,
                "Companion source path escapes plugin directory, rejecting"
            );
            continue;
        }

        // ── Validate filename (prevent writing outside target dir) ──
        let raw_filename = companion.filename.as_deref().unwrap_or_else(|| {
            canonical_source
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("companion")
        });

        let safe_filename = match resolve_companion_filename(companion, Some(&canonical_source)) {
            Some(name) => name,
            _ => {
                tracing::warn!(
                    plugin = plugin_key,
                    filename = raw_filename,
                    "Companion filename is invalid, rejecting"
                );
                continue;
            }
        };

        let target_path = target_dir.join(safe_filename);

        if let Err(e) = std::fs::create_dir_all(&target_dir) {
            tracing::warn!(
                plugin = plugin_key,
                dir = %target_dir.display(),
                "Failed to create companion target directory: {e}"
            );
            continue;
        }

        match std::fs::copy(&canonical_source, &target_path) {
            Ok(_) => {
                tracing::info!(
                    plugin = plugin_key,
                    source = %canonical_source.display(),
                    target = %target_path.display(),
                    "Installed companion file"
                );
            }
            Err(e) => {
                tracing::warn!(
                    plugin = plugin_key,
                    source = %canonical_source.display(),
                    target = %target_path.display(),
                    "Failed to install companion file: {e}"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use peekoo_notifications::{MoodReactionService, NotificationService, PeekBadgeService};
    use peekoo_scheduler::Scheduler;
    use rusqlite::Connection;

    use crate::PluginError;

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
            Arc::new(PeekBadgeService::new()),
            Arc::new(MoodReactionService::new()),
        )
    }

    fn sample_plugin_dir(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins")
            .join(name)
    }

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("peekoo-{prefix}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn write_manifest(plugin_dir: &Path, key: &str, version: &str, wasm: &str, name: &str) {
        fs::create_dir_all(plugin_dir).expect("plugin dir");
        fs::write(
            plugin_dir.join("peekoo-plugin.toml"),
            format!(
                "[plugin]\nkey = \"{key}\"\nname = \"{name}\"\nversion = \"{version}\"\nwasm = \"{wasm}\"\n"
            ),
        )
        .expect("manifest");
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

    #[test]
    fn install_plugin_disables_plugin_after_failed_load() {
        let plugin_dir = temp_dir("broken-plugin-install").join("broken-plugin");
        write_manifest(
            &plugin_dir,
            "broken-plugin",
            "0.1.0",
            "missing.wasm",
            "Broken Plugin",
        );

        let registry = test_registry(vec![plugin_dir.clone()]);

        let err = registry
            .install_plugin(&plugin_dir)
            .expect_err("broken plugin should fail to load");

        assert!(matches!(
            err,
            PluginError::Io(_) | PluginError::Runtime(_) | PluginError::Internal(_)
        ));
        assert!(
            !registry
                .is_plugin_enabled("broken-plugin")
                .expect("enabled state should exist after failed install")
        );
    }

    #[test]
    fn sync_plugin_registration_updates_existing_manifest_metadata() {
        let plugin_dir = temp_dir("plugin-metadata-update").join("meta-plugin");
        write_manifest(
            &plugin_dir,
            "meta-plugin",
            "0.1.0",
            "plugin.wasm",
            "Meta Plugin",
        );

        let registry = test_registry(vec![plugin_dir.clone()]);
        registry
            .sync_plugin_registration(&plugin_dir)
            .expect("initial registration");

        write_manifest(
            &plugin_dir,
            "meta-plugin",
            "0.2.0",
            "plugin.wasm",
            "Meta Plugin Updated",
        );
        registry
            .sync_plugin_registration(&plugin_dir)
            .expect("updated registration");

        let conn = registry.db_conn.lock().expect("db lock");
        let (version, manifest_json): (String, String) = conn
            .query_row(
                "SELECT version, manifest_json FROM plugins WHERE plugin_key = ?1",
                rusqlite::params!["meta-plugin"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("plugin row");

        let manifest: serde_json::Value =
            serde_json::from_str(&manifest_json).expect("manifest json");
        assert_eq!(version, "0.2.0");
        assert_eq!(manifest["name"], "Meta Plugin Updated");
        assert_eq!(manifest["version"], "0.2.0");
    }
}
