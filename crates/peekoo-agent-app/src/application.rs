use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use peekoo_agent::AgentEvent;
use peekoo_agent::config::AgentServiceConfig;
use peekoo_agent::service::AgentService;
use peekoo_notifications::{Notification, NotificationService, PeekBadgeItem, PeekBadgeService};
use peekoo_paths::ensure_windows_pi_agent_env;
use peekoo_plugin_host::{PluginRegistry, PluginToolBridge};
use peekoo_scheduler::Scheduler;
use rusqlite::Connection;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

use crate::conversation::{self, LastSessionDto, json_messages_to_dtos};
use crate::plugin::{
    PluginConfigFieldDto, PluginNotificationDto, PluginPanelDto, PluginSummaryDto,
    manifest_to_summary, plugin_notification_from_message,
};
use crate::productivity::{PomodoroSessionDto, ProductivityService, TaskDto};
use crate::settings::{
    AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto, OauthCancelResponse,
    OauthStartResponse, OauthStatusRequest, OauthStatusResponse, ProviderAuthDto,
    ProviderConfigDto, ProviderRequest, SetApiKeyRequest, SetProviderConfigRequest,
    SettingsService,
};
use peekoo_plugin_store::{PluginStoreService, StorePluginDto};

pub struct AgentApplication {
    agent: Mutex<Option<AgentService>>,
    settings: SettingsService,
    productivity: ProductivityService,
    plugin_registry: Arc<PluginRegistry>,
    plugin_tools: PluginToolBridge,
    plugin_store: PluginStoreService,
    notifications: Arc<NotificationService>,
    notification_receiver: Mutex<UnboundedReceiver<Notification>>,
    peek_badges: Arc<PeekBadgeService>,
    shutdown_token: CancellationToken,
    agent_config_version: Mutex<Option<i64>>,
    /// Directory where pi session files are stored.
    session_dir: PathBuf,
    /// Workspace root used to scope session restore and resumption.
    workspace_dir: PathBuf,
    /// Path to the last session file, used to resume context on the next prompt.
    resume_session_path: Mutex<Option<PathBuf>>,
    /// Monotonic generation that invalidates in-flight agents after `new_session`.
    conversation_generation: AtomicU64,
}

impl AgentApplication {
    pub fn new() -> Result<Self, String> {
        ensure_windows_pi_agent_env()?;
        let settings = SettingsService::new()?;
        let (plugin_registry, notifications, notification_receiver, peek_badges) =
            create_plugin_registry()?;
        let shutdown_token = plugin_registry.scheduler().shutdown_token();
        install_discovered_plugins(&plugin_registry);

        let session_dir = peekoo_paths::peekoo_global_data_dir()?.join("sessions");
        if !session_dir.exists() {
            std::fs::create_dir_all(&session_dir)
                .map_err(|e| format!("Create session dir error: {e}"))?;
        }
        let workspace_dir = resolve_workspace_dir();

        Ok(Self {
            agent: Mutex::new(None),
            settings,
            productivity: ProductivityService::new(),
            plugin_tools: PluginToolBridge::new(Arc::clone(&plugin_registry)),
            plugin_registry,
            plugin_store: PluginStoreService::new(),
            notifications,
            notification_receiver: Mutex::new(notification_receiver),
            peek_badges,
            shutdown_token,
            agent_config_version: Mutex::new(None),
            session_dir,
            workspace_dir,
            resume_session_path: Mutex::new(None),
            conversation_generation: AtomicU64::new(0),
        })
    }

    pub fn start_plugin_runtime(&self) {
        self.plugin_registry.start_scheduler();
    }

    pub async fn prompt_streaming<F>(&self, message: &str, on_event: F) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static,
    {
        let generation = self.conversation_generation.load(Ordering::SeqCst);
        let mut agent = {
            let mut guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
            let mut version_guard = self
                .agent_config_version
                .lock()
                .map_err(|e| format!("Version lock error: {e}"))?;

            let should_recreate = guard.is_none();

            if should_recreate {
                let (service, settings_version) = self.create_agent_service()?;
                *guard = Some(service);
                *version_guard = Some(settings_version);
            } else {
                let current_version = self.settings.get_settings()?.version;
                if (*version_guard) != Some(current_version) {
                    let (service, settings_version) = self.create_agent_service()?;
                    *guard = Some(service);
                    *version_guard = Some(settings_version);
                }
            }

            guard
                .take()
                .ok_or_else(|| "Agent not initialized after creation".to_string())?
        };

        let reactor = asupersync::runtime::reactor::create_reactor()
            .map_err(|e| format!("Reactor error: {e}"))?;
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .with_reactor(reactor)
            .build()
            .map_err(|e| format!("Runtime error: {e}"))?;

        let result = runtime.block_on(agent.prompt(message, on_event));

        {
            let mut guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
            if should_restore_agent(
                generation,
                self.conversation_generation.load(Ordering::SeqCst),
            ) {
                *guard = Some(agent);
            }
        }

        result.map_err(|e| format!("Agent error: {e}"))
    }

    pub async fn set_model(&self, provider: &str, model: &str) -> Result<(), String> {
        let mut agent = {
            let mut guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
            guard
                .take()
                .ok_or("Agent not initialized. Send a message first.")?
        };

        let reactor = asupersync::runtime::reactor::create_reactor()
            .map_err(|e| format!("Reactor error: {e}"))?;
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .with_reactor(reactor)
            .build()
            .map_err(|e| format!("Runtime error: {e}"))?;

        let result = runtime.block_on(agent.set_model(provider, model));

        {
            let mut guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
            *guard = Some(agent);
        }

        result.map_err(|e| format!("Set model error: {e}"))
    }

    pub fn get_model(&self) -> Result<(String, String), String> {
        let guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
        let agent = guard
            .as_ref()
            .ok_or("Agent not initialized. Send a message first.")?;
        Ok(agent.model())
    }

    pub fn get_settings(&self) -> Result<AgentSettingsDto, String> {
        self.settings.get_settings()
    }

    pub fn update_settings(
        &self,
        patch: AgentSettingsPatchDto,
    ) -> Result<AgentSettingsDto, String> {
        self.settings.update_settings(patch)
    }

    pub fn settings_catalog(&self) -> Result<AgentSettingsCatalogDto, String> {
        self.settings.catalog()
    }

    pub fn set_provider_api_key(&self, req: SetApiKeyRequest) -> Result<ProviderAuthDto, String> {
        self.settings.set_provider_api_key(req)
    }

    pub fn clear_provider_auth(&self, req: ProviderRequest) -> Result<ProviderAuthDto, String> {
        self.settings.clear_provider_auth(req)
    }

    pub fn set_provider_config(
        &self,
        req: SetProviderConfigRequest,
    ) -> Result<ProviderConfigDto, String> {
        self.settings.set_provider_config(req)
    }

    pub fn oauth_start(&self, req: ProviderRequest) -> Result<OauthStartResponse, String> {
        self.settings.start_oauth(req)
    }

    pub async fn oauth_status(
        &self,
        req: OauthStatusRequest,
    ) -> Result<OauthStatusResponse, String> {
        self.settings.oauth_status(req).await
    }

    pub fn oauth_cancel(&self, req: OauthStatusRequest) -> Result<OauthCancelResponse, String> {
        self.settings.cancel_oauth(req)
    }

    pub fn create_task(&self, title: &str, priority: &str) -> Result<TaskDto, String> {
        self.productivity.create_task(title, priority)
    }

    pub fn start_pomodoro(&self, minutes: u32) -> Result<PomodoroSessionDto, String> {
        self.productivity.start_pomodoro(minutes)
    }

    pub fn pause_pomodoro(&self, session_id: &str) -> Result<PomodoroSessionDto, String> {
        self.productivity.pause_pomodoro(session_id)
    }

    pub fn resume_pomodoro(&self, session_id: &str) -> Result<PomodoroSessionDto, String> {
        self.productivity.resume_pomodoro(session_id)
    }

    pub fn finish_pomodoro(&self, session_id: &str) -> Result<PomodoroSessionDto, String> {
        self.productivity.finish_pomodoro(session_id)
    }

    pub fn list_plugins(&self) -> Result<Vec<PluginSummaryDto>, String> {
        let plugins = self
            .plugin_registry
            .discover()
            .into_iter()
            .map(
                |(plugin_dir, manifest)| -> Result<PluginSummaryDto, String> {
                    self.plugin_registry
                        .sync_plugin_registration(&plugin_dir)
                        .map_err(|e| e.to_string())?;
                    let enabled = self
                        .plugin_registry
                        .is_plugin_enabled(&manifest.plugin.key)
                        .map_err(|e| e.to_string())?;
                    Ok(manifest_to_summary(&manifest, plugin_dir, enabled))
                },
            )
            .collect::<Result<Vec<_>, String>>()?;
        Ok(plugins)
    }

    pub fn list_plugin_panels(&self) -> Result<Vec<PluginPanelDto>, String> {
        Ok(self
            .plugin_registry
            .all_ui_panels()
            .into_iter()
            .map(|(plugin_key, panel)| PluginPanelDto::from_panel(plugin_key, panel))
            .collect())
    }

    pub fn enable_plugin(&self, plugin_key: &str) -> Result<(), String> {
        let plugin_dir = self
            .plugin_registry
            .discover()
            .into_iter()
            .find_map(|(plugin_dir, manifest)| {
                (manifest.plugin.key == plugin_key).then_some(plugin_dir)
            })
            .ok_or_else(|| format!("Plugin not found: {plugin_key}"))?;

        self.plugin_registry
            .install_plugin(&plugin_dir)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn disable_plugin(&self, plugin_key: &str) -> Result<(), String> {
        self.plugin_registry
            .set_plugin_enabled(plugin_key, false)
            .map_err(|e| e.to_string())?;

        match self.plugin_registry.unload_plugin(plugin_key) {
            Ok(()) | Err(peekoo_plugin_host::PluginError::NotFound(_)) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }

    pub fn call_plugin_tool(&self, tool_name: &str, args_json: &str) -> Result<String, String> {
        self.plugin_tools
            .call_tool(tool_name, args_json)
            .map_err(|e| e.to_string())
    }

    pub fn drain_plugin_notifications(&self) -> Vec<PluginNotificationDto> {
        let mut receiver = match self.notification_receiver.lock() {
            Ok(receiver) => receiver,
            Err(err) => {
                tracing::warn!("Notification receiver lock error: {err}");
                return Vec::new();
            }
        };

        let mut notifications = Vec::new();
        while let Ok(notification) = receiver.try_recv() {
            notifications.push(plugin_notification_from_message(notification));
        }

        notifications
    }

    /// Return the merged peek-badge list if any plugin pushed an update since the last call.
    pub fn take_peek_badges_if_changed(&self) -> Option<Vec<PeekBadgeItem>> {
        self.peek_badges.take_if_changed()
    }

    pub fn plugin_config_schema(
        &self,
        plugin_key: &str,
    ) -> Result<Vec<PluginConfigFieldDto>, String> {
        self.plugin_registry
            .config_schema(plugin_key)
            .map(|fields| {
                fields
                    .into_iter()
                    .map(|field| PluginConfigFieldDto {
                        plugin_key: plugin_key.to_string(),
                        field,
                    })
                    .collect()
            })
            .map_err(|e| e.to_string())
    }

    pub fn plugin_config_values(&self, plugin_key: &str) -> Result<serde_json::Value, String> {
        self.plugin_registry
            .config_values(plugin_key)
            .map_err(|e| e.to_string())
    }

    pub fn plugin_config_set(
        &self,
        plugin_key: &str,
        key: &str,
        value: serde_json::Value,
    ) -> Result<(), String> {
        self.plugin_registry
            .set_config_value(plugin_key, key, value)
            .map_err(|e| e.to_string())
    }

    pub fn query_plugin_data(
        &self,
        plugin_key: &str,
        provider_name: &str,
    ) -> Result<String, String> {
        self.plugin_registry
            .query_data(plugin_key, provider_name)
            .map_err(|e| e.to_string())
    }

    /// Read the plugin panel HTML and inline any sibling CSS/JS files.
    ///
    /// This keeps file-system assembly logic in the app layer rather than the
    /// Tauri transport layer.
    pub fn plugin_panel_html(&self, label: &str) -> Result<String, String> {
        let path = self
            .plugin_registry
            .panel_entry_path(label)
            .ok_or_else(|| format!("Plugin panel not found: {label}"))?;

        let mut html = std::fs::read_to_string(&path)
            .map_err(|e| format!("Read plugin panel html error: {e}"))?;

        if let Some(parent) = path.parent() {
            let css_path = parent.join("panel.css");
            if css_path.is_file() {
                let css = std::fs::read_to_string(&css_path)
                    .map_err(|e| format!("Read plugin panel css error: {e}"))?;
                html = html.replace(
                    "<link rel=\"stylesheet\" href=\"panel.css\" />",
                    &format!("<style>{css}</style>"),
                );
            }

            let js_path = parent.join("panel.js");
            if js_path.is_file() {
                let js = std::fs::read_to_string(&js_path)
                    .map_err(|e| format!("Read plugin panel js error: {e}"))?;
                html = html.replace(
                    "<script src=\"panel.js\"></script>",
                    &format!("<script>{js}</script>"),
                );
            }
        }

        Ok(html)
    }

    pub fn dispatch_plugin_event(
        &self,
        event_name: &str,
        payload_json: &str,
    ) -> Result<(), String> {
        self.plugin_registry
            .dispatch_event(event_name, payload_json);
        Ok(())
    }

    pub fn set_dnd(&self, active: bool) {
        self.notifications.set_dnd(active);
    }

    pub fn is_dnd(&self) -> bool {
        self.notifications.is_dnd()
    }

    /// Return the most recent session's messages for the chat panel.
    ///
    /// If the agent is already alive (i.e. a prompt has been sent this
    /// session), messages are taken from the in-memory agent. Otherwise the
    /// most recent session file on disk is read.
    ///
    /// A side-effect: when messages are loaded from disk, the session file
    /// path is stored so that [`create_agent_service`] can resume it on the
    /// next prompt (full context restore).
    pub async fn get_last_session(&self) -> Result<Option<LastSessionDto>, String> {
        // Fast path: agent is alive — use its in-memory messages.
        {
            let guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
            if let Some(agent) = guard.as_ref() {
                let json_msgs = agent.messages_json();
                let dtos = json_messages_to_dtos(&json_msgs);
                if dtos.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(LastSessionDto {
                    session_path: String::new(),
                    messages: dtos,
                }));
            }
        }

        // Slow path: load from disk.
        let result =
            conversation::load_last_session(&self.session_dir, &self.workspace_dir).await?;

        // Stash the path so the next prompt resumes this session.
        if let Some(ref dto) = result {
            if !dto.session_path.is_empty() {
                if let Ok(mut guard) = self.resume_session_path.lock() {
                    *guard = Some(PathBuf::from(&dto.session_path));
                }
            }
        }

        Ok(result)
    }

    /// Start a fresh conversation. Drops the current agent so the next prompt
    /// creates a brand-new session file.
    pub fn new_session(&self) -> Result<(), String> {
        self.conversation_generation.fetch_add(1, Ordering::SeqCst);
        {
            let mut guard = self.agent.lock().map_err(|e| format!("Lock error: {e}"))?;
            *guard = None;
        }
        {
            let mut version_guard = self
                .agent_config_version
                .lock()
                .map_err(|e| format!("Version lock error: {e}"))?;
            *version_guard = None;
        }
        {
            let mut resume_guard = self
                .resume_session_path
                .lock()
                .map_err(|e| format!("Resume path lock error: {e}"))?;
            *resume_guard = None;
        }
        Ok(())
    }

    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown_token.clone()
    }

    pub fn store_catalog(&self) -> Result<Vec<StorePluginDto>, String> {
        self.plugin_store.fetch_catalog(&self.plugin_registry)
    }

    pub fn store_install(&self, plugin_key: &str) -> Result<StorePluginDto, String> {
        self.plugin_store
            .install_plugin(plugin_key, &self.plugin_registry)
    }

    pub fn store_update(&self, plugin_key: &str) -> Result<StorePluginDto, String> {
        self.plugin_store
            .update_plugin(plugin_key, &self.plugin_registry)
    }

    pub fn store_uninstall(&self, plugin_key: &str) -> Result<(), String> {
        self.plugin_store
            .uninstall_plugin(plugin_key, &self.plugin_registry)
    }

    /// Build a fresh `AgentService` from current settings + plugin prompt.
    fn create_agent_service(&self) -> Result<(AgentService, i64), String> {
        let config = self.resolved_config()?;
        let (config, settings_version) = self.settings.to_agent_config(config)?;
        let mut config = self.with_plugin_prompt(config);

        // Enable session persistence.
        config.no_session = false;
        config.session_dir = Some(self.session_dir.clone());

        // If get_last_session stashed a path, resume that session for full
        // context restore. The path is consumed so it is only used once.
        if let Ok(mut guard) = self.resume_session_path.lock() {
            config.session_path = guard.take();
        }

        let reactor = asupersync::runtime::reactor::create_reactor()
            .map_err(|e| format!("Reactor error: {e}"))?;
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .with_reactor(reactor)
            .build()
            .map_err(|e| format!("Runtime error: {e}"))?;

        let service = runtime
            .block_on(AgentService::new(config))
            .map_err(|e| format!("Agent init error: {e}"))?;

        Ok((service, settings_version))
    }

    fn resolved_config(&self) -> Result<AgentServiceConfig, String> {
        let mut config = AgentServiceConfig::default();
        config.working_directory = self.workspace_dir.clone();

        Ok(config)
    }

    fn with_plugin_prompt(&self, mut config: AgentServiceConfig) -> AgentServiceConfig {
        let specs = self.plugin_tools.tool_specs();
        if specs.is_empty() {
            return config;
        }

        let mut lines = Vec::with_capacity(specs.len() + 2);
        lines.push("## Plugin Capabilities".to_string());
        lines.push(
            "The app has installed plugins with the following externally callable capabilities:"
                .to_string(),
        );

        for spec in specs {
            lines.push(format!(
                "- `{}` (plugin `{}`): {} | parameters: {}",
                spec.name, spec.plugin_key, spec.description, spec.parameters_schema
            ));
        }

        let plugin_prompt = lines.join("\n");
        config.system_prompt = match config.system_prompt.take() {
            Some(existing) if !existing.trim().is_empty() => {
                Some(format!("{existing}\n\n{plugin_prompt}"))
            }
            _ => Some(plugin_prompt),
        };

        config
    }
}

fn resolve_workspace_dir() -> PathBuf {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut current = current_dir.clone();

    loop {
        if current.join(".peekoo").is_dir() {
            return current;
        }

        let Some(parent) = current.parent() else {
            return current_dir;
        };
        current = parent.to_path_buf();
    }
}

fn should_restore_agent(captured_generation: u64, current_generation: u64) -> bool {
    captured_generation == current_generation
}

#[allow(clippy::type_complexity)]
fn create_plugin_registry() -> Result<
    (
        Arc<PluginRegistry>,
        Arc<NotificationService>,
        UnboundedReceiver<Notification>,
        Arc<PeekBadgeService>,
    ),
    String,
> {
    let db_path = peekoo_paths::peekoo_settings_db_path()?;
    let db_conn = Connection::open(&db_path).map_err(|e| format!("Open plugin db error: {e}"))?;

    let global_plugins_dir = peekoo_paths::peekoo_global_data_dir()?.join("plugins");
    if !global_plugins_dir.exists() {
        std::fs::create_dir_all(&global_plugins_dir)
            .map_err(|e| format!("Create plugin dir error: {e}"))?;
    }

    let scheduler = Arc::new(Scheduler::new());
    let (notifications, receiver) = NotificationService::new();
    let notifications = Arc::new(notifications);
    let peek_badges = Arc::new(PeekBadgeService::new());
    let registry = Arc::new(PluginRegistry::new(
        vec![global_plugins_dir],
        Arc::new(Mutex::new(db_conn)),
        scheduler,
        Arc::clone(&notifications),
        Arc::clone(&peek_badges),
    ));

    Ok((registry, notifications, receiver, peek_badges))
}

fn install_discovered_plugins(plugin_registry: &Arc<PluginRegistry>) {
    for (plugin_dir, manifest) in plugin_registry.discover() {
        let enabled = match plugin_registry.sync_plugin_registration(&plugin_dir) {
            Ok(_) => match plugin_registry.is_plugin_enabled(&manifest.plugin.key) {
                Ok(enabled) => enabled,
                Err(err) => {
                    tracing::warn!(
                        plugin = manifest.plugin.key.as_str(),
                        dir = %plugin_dir.display(),
                        "Skipping plugin during startup: {err}"
                    );
                    continue;
                }
            },
            Err(err) => {
                tracing::warn!(
                    plugin = manifest.plugin.key.as_str(),
                    dir = %plugin_dir.display(),
                    "Skipping plugin during startup: {err}"
                );
                continue;
            }
        };

        if !enabled {
            tracing::info!(
                plugin = manifest.plugin.key.as_str(),
                dir = %plugin_dir.display(),
                "Plugin discovered but left disabled during startup"
            );
            continue;
        }

        match plugin_registry.install_plugin(&plugin_dir) {
            Ok(key) => tracing::info!(plugin = key.as_str(), "Plugin installed and loaded"),
            Err(err) => tracing::warn!(
                plugin = manifest.plugin.key.as_str(),
                dir = %plugin_dir.display(),
                "Skipping plugin during startup: {err}"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use peekoo_notifications::{NotificationService, PeekBadgeService};
    use peekoo_plugin_host::PluginRegistry;
    use peekoo_scheduler::Scheduler;
    use rusqlite::Connection;

    use super::{install_discovered_plugins, should_restore_agent};

    fn test_registry(plugin_name: &str) -> Arc<PluginRegistry> {
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

        let plugin_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../plugins")
            .join(plugin_name);

        let scheduler = Arc::new(Scheduler::new());
        let (notifications, _receiver) = NotificationService::new();

        Arc::new(PluginRegistry::new(
            vec![plugin_dir],
            Arc::new(Mutex::new(conn)),
            scheduler,
            Arc::new(notifications),
            Arc::new(PeekBadgeService::new()),
        ))
    }

    #[test]
    fn startup_skips_loading_plugins_marked_disabled() {
        let registry = test_registry("health-reminders");
        let plugin_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/health-reminders");

        registry
            .sync_plugin_registration(&plugin_dir)
            .expect("plugin should register");
        registry
            .set_plugin_enabled("health-reminders", false)
            .expect("plugin should disable");

        install_discovered_plugins(&registry);

        assert!(registry.loaded_keys().is_empty());
        assert!(registry.all_ui_panels().is_empty());
    }

    #[test]
    fn agent_restore_is_blocked_after_generation_changes() {
        assert!(should_restore_agent(4, 4));
        assert!(!should_restore_agent(4, 5));
    }
}
