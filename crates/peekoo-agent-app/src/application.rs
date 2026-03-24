use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

use peekoo_agent::AgentEvent;
use peekoo_agent::PluginToolProvider;
use peekoo_agent::config::AgentServiceConfig;
use peekoo_agent::service::AgentService;
use peekoo_app_settings::{AppSettingsService, SpriteInfo};
use peekoo_notifications::{
    MoodReaction, MoodReactionService, Notification, NotificationService, PeekBadgeItem,
    PeekBadgeService,
};
use peekoo_paths::ensure_windows_pi_agent_env;
use peekoo_plugin_host::PluginRegistry;
use peekoo_scheduler::Scheduler;

use crate::conversation::{self, LastSessionDto, json_messages_to_dtos};
use crate::plugin::{
    PluginConfigFieldDto, PluginNotificationDto, PluginPanelDto, PluginSummaryDto,
    manifest_to_summary, plugin_notification_from_message,
};
use crate::plugin_tool_impl::PluginToolProviderImpl;
use peekoo_productivity_domain::task::{TaskDto, TaskEventDto, TaskService};

use crate::productivity::{PomodoroSessionDto, ProductivityService};
use crate::settings::{
    AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto, OauthCancelResponse,
    OauthStartResponse, OauthStatusRequest, OauthStatusResponse, ProviderAuthDto,
    ProviderConfigDto, ProviderRequest, SetApiKeyRequest, SetProviderConfigRequest,
    SettingsService,
};
use crate::task_runtime_service::TaskRuntimeService;
use peekoo_plugin_store::{PluginStoreService, StorePluginDto};

use crate::workspace_bootstrap::ensure_agent_workspace;

pub struct AgentApplication {
    agent: Mutex<Option<AgentService>>,
    settings: SettingsService,
    app_settings: AppSettingsService,
    productivity: ProductivityService,
    plugin_registry: Arc<PluginRegistry>,
    plugin_tools: Arc<PluginToolProviderImpl>,
    plugin_store: PluginStoreService,
    notifications: Arc<NotificationService>,
    notification_receiver: Mutex<UnboundedReceiver<Notification>>,
    peek_badges: Arc<PeekBadgeService>,
    mood_reactions: Arc<MoodReactionService>,
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
    /// Scheduler for agent task execution.
    agent_scheduler: Arc<Mutex<Option<crate::agent_scheduler::AgentScheduler>>>,
}

impl AgentApplication {
    pub fn new() -> Result<Self, String> {
        ensure_windows_pi_agent_env()?;

        // Open a single shared SQLite connection for the entire application.
        // WAL mode allows concurrent readers and serialises writers gracefully,
        // avoiding OS-level "Access Denied" (error 5) on Windows where SQLite
        // uses mandatory file locks in the default DELETE journal mode.
        //
        // Legacy migration MUST run before Connection::open because open()
        let db_path = peekoo_paths::peekoo_settings_db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Create db dir error: {e}"))?;
        }
        let conn =
            Connection::open(&db_path).map_err(|e| format!("Open settings db error: {e}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")
            .map_err(|e| format!("Set db pragmas error: {e}"))?;
        let db_conn = Arc::new(Mutex::new(conn));

        let settings = SettingsService::with_conn(Arc::clone(&db_conn))?;
        let app_settings = AppSettingsService::with_conn(Arc::clone(&db_conn))?;
        let productivity = ProductivityService::new(Arc::clone(&db_conn));
        let task_service: Arc<dyn peekoo_productivity_domain::task::TaskService> =
            Arc::new(productivity.clone());
        let (plugin_registry, notifications, notification_receiver, peek_badges, mood_reactions) =
            create_plugin_registry(db_conn, task_service)?;
        let shutdown_token = plugin_registry.scheduler().shutdown_token();
        install_discovered_plugins(&plugin_registry);

        let session_dir = peekoo_paths::peekoo_global_data_dir()?.join("sessions");
        if !session_dir.exists() {
            std::fs::create_dir_all(&session_dir)
                .map_err(|e| format!("Create session dir error: {e}"))?;
        }
        let workspace_dir = ensure_agent_workspace()?;

        // Create agent scheduler for task execution
        let agent_scheduler =
            crate::agent_scheduler::AgentScheduler::new(Arc::new(productivity.clone()));

        Ok(Self {
            agent: Mutex::new(None),
            settings,
            app_settings,
            productivity,
            plugin_tools: Arc::new(PluginToolProviderImpl::new(Arc::clone(&plugin_registry))),
            plugin_registry,
            plugin_store: PluginStoreService::new(),
            notifications,
            notification_receiver: Mutex::new(notification_receiver),
            peek_badges,
            mood_reactions,
            shutdown_token,
            agent_config_version: Mutex::new(None),
            session_dir,
            workspace_dir,
            resume_session_path: Mutex::new(None),
            conversation_generation: AtomicU64::new(0),
            agent_scheduler: Arc::new(Mutex::new(Some(agent_scheduler))),
        })
    }

    pub fn start_plugin_runtime(&self) {
        self.plugin_registry.start_scheduler();

        eprintln!("[peekoo][mcp] starting MCP server during app startup");

        // Start MCP server on a dedicated thread (survives app lifetime)
        let task_service: Arc<dyn peekoo_productivity_domain::task::TaskService> =
            Arc::new(self.task_runtime_service());
        let mcp_shutdown = self.shutdown_token.clone();

        match crate::mcp_server::start_sync(task_service, mcp_shutdown) {
            Ok(addr) => {
                let url = peekoo_mcp_server::mcp_url_for(addr);
                eprintln!("[peekoo][mcp] server ready at {}", url);
                tracing::info!("✅ [MCP] Server ready at {}", url);
            }
            Err(e) => {
                eprintln!("[peekoo][mcp] failed to start: {}", e);
                tracing::error!("❌ [MCP] Failed to start server: {}", e);
            }
        }

        // Start agent scheduler for task execution
        if let Ok(guard) = self.agent_scheduler.lock()
            && let Some(ref scheduler) = *guard
        {
            scheduler.set_agent_launch_env(self.agent_launch_env());
            scheduler.start();
        }
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

    // ── Global app settings ────────────────────────────────────────────

    pub fn get_active_sprite_id(&self) -> Result<String, String> {
        self.app_settings.get_active_sprite_id()
    }

    pub fn set_active_sprite_id(&self, sprite_id: &str) -> Result<(), String> {
        self.app_settings.set_active_sprite_id(sprite_id)
    }

    pub fn list_sprites(&self) -> Vec<SpriteInfo> {
        self.app_settings.list_sprites()
    }

    pub fn get_app_settings(&self) -> Result<std::collections::HashMap<String, String>, String> {
        self.app_settings.get_all()
    }

    pub fn set_app_setting(&self, key: &str, value: &str) -> Result<(), String> {
        self.app_settings.set(key, value)
    }

    // ── Productivity ────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn create_task(
        &self,
        title: &str,
        priority: &str,
        assignee: &str,
        labels: &[String],
        description: Option<&str>,
        scheduled_start_at: Option<&str>,
        scheduled_end_at: Option<&str>,
        estimated_duration_min: Option<u32>,
        recurrence_rule: Option<&str>,
        recurrence_time_of_day: Option<&str>,
    ) -> Result<TaskDto, String> {
        self.productivity.create_task(
            title,
            priority,
            assignee,
            labels,
            description,
            scheduled_start_at,
            scheduled_end_at,
            estimated_duration_min,
            recurrence_rule,
            recurrence_time_of_day,
        )
    }

    pub fn list_tasks(&self) -> Result<Vec<TaskDto>, String> {
        self.productivity.list_tasks()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_task(
        &self,
        id: &str,
        title: Option<&str>,
        priority: Option<&str>,
        status: Option<&str>,
        assignee: Option<&str>,
        labels: Option<&[String]>,
        description: Option<&str>,
        scheduled_start_at: Option<&str>,
        scheduled_end_at: Option<&str>,
        estimated_duration_min: Option<Option<u32>>,
        recurrence_rule: Option<Option<&str>>,
        recurrence_time_of_day: Option<Option<&str>>,
    ) -> Result<TaskDto, String> {
        self.productivity.update_task(
            id,
            title,
            priority,
            status,
            assignee,
            labels,
            description,
            scheduled_start_at,
            scheduled_end_at,
            estimated_duration_min,
            recurrence_rule,
            recurrence_time_of_day,
        )
    }

    pub fn delete_task(&self, id: &str) -> Result<(), String> {
        self.productivity.delete_task(id)
    }

    pub fn toggle_task(&self, id: &str) -> Result<TaskDto, String> {
        self.productivity.toggle_task(id)
    }

    /// Create a task from natural language text
    /// Parses the text to extract title, priority, schedule, duration, etc.
    /// Falls back to using the whole text as title if parsing fails.
    pub fn create_task_from_text(&self, text: &str) -> Result<TaskDto, String> {
        use crate::task_parser::parse_task_text;

        let parsed = parse_task_text(text);

        self.productivity.create_task(
            &parsed.title,
            parsed.priority.as_deref().unwrap_or("medium"),
            parsed.assignee.as_deref().unwrap_or("user"),
            &parsed.labels,
            parsed.description.as_deref(),
            parsed.scheduled_start_at.as_deref(),
            parsed.scheduled_end_at.as_deref(),
            parsed.estimated_duration_min,
            parsed.recurrence_rule.as_deref(),
            parsed.recurrence_time_of_day.as_deref(),
        )
    }

    pub fn get_task_activity(
        &self,
        task_id: &str,
        limit: u32,
    ) -> Result<Vec<TaskEventDto>, String> {
        self.productivity.get_task_activity(task_id, limit)
    }

    pub fn list_task_events(&self, limit: i64) -> Result<Vec<TaskEventDto>, String> {
        self.productivity.list_task_events(limit)
    }

    pub fn add_task_comment(
        &self,
        task_id: &str,
        text: &str,
        author: &str,
    ) -> Result<TaskEventDto, String> {
        self.task_runtime_service()
            .add_task_comment(task_id, text, author)
    }

    pub fn delete_task_event(&self, event_id: &str) -> Result<(), String> {
        self.productivity.delete_task_event(event_id)
    }

    pub fn task_activity_summary(&self) -> Result<String, String> {
        self.productivity.task_activity_summary()
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
            .map_err(|e| e.to_string())?;

        self.peek_badges.refresh();
        self.invalidate_agent_for_plugin_change();
        Ok(())
    }

    pub fn disable_plugin(&self, plugin_key: &str) -> Result<(), String> {
        self.peek_badges.clear(plugin_key);
        self.peek_badges.refresh();

        self.plugin_registry
            .set_plugin_enabled(plugin_key, false)
            .map_err(|e| e.to_string())?;

        match self.plugin_registry.unload_plugin(plugin_key) {
            Ok(()) | Err(peekoo_plugin_host::PluginError::NotFound(_)) => {}
            Err(err) => return Err(err.to_string()),
        }

        self.invalidate_agent_for_plugin_change();
        Ok(())
    }

    pub fn call_plugin_tool(&self, tool_name: &str, args_json: &str) -> Result<String, String> {
        self.plugin_tools.call_plugin_tool(tool_name, args_json)
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

    /// Signal that the UI has mounted and is listening for badge events.
    ///
    /// Badges pushed before this call are retained and will be emitted on the
    /// next background flush tick.
    pub fn mark_ui_ready(&self) {
        self.peek_badges.mark_ui_ready();
        self.mood_reactions.mark_ui_ready();
    }

    /// Return the merged peek-badge list if any plugin pushed an update since the last call.
    pub fn take_peek_badges_if_changed(&self) -> Option<Vec<PeekBadgeItem>> {
        self.peek_badges.take_if_changed()
    }

    /// Drain all queued mood reactions pushed by plugins.
    pub fn drain_mood_reactions(&self) -> Vec<MoodReaction> {
        self.mood_reactions.drain()
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

    pub fn call_plugin_panel_tool(
        &self,
        plugin_key: &str,
        tool_name: &str,
        args_json: &str,
    ) -> Result<String, String> {
        self.plugin_registry
            .call_tool(plugin_key, tool_name, args_json)
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
        let result = conversation::load_last_session(&self.session_dir).await?;

        // Stash the path so the next prompt resumes this session.
        if let Some(ref dto) = result
            && !dto.session_path.is_empty()
            && let Ok(mut guard) = self.resume_session_path.lock()
        {
            *guard = Some(PathBuf::from(&dto.session_path));
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
        let result = self
            .plugin_store
            .install_plugin(plugin_key, &self.plugin_registry)?;
        self.invalidate_agent_for_plugin_change();
        Ok(result)
    }

    pub fn store_update(&self, plugin_key: &str) -> Result<StorePluginDto, String> {
        let result = self
            .plugin_store
            .update_plugin(plugin_key, &self.plugin_registry)?;
        self.invalidate_agent_for_plugin_change();
        Ok(result)
    }

    pub fn store_uninstall(&self, plugin_key: &str) -> Result<(), String> {
        self.plugin_store
            .uninstall_plugin(plugin_key, &self.plugin_registry)?;
        self.invalidate_agent_for_plugin_change();
        Ok(())
    }

    /// Drop the current agent so the next prompt rebuilds it with the latest
    /// plugin tool set. Unlike [`new_session`] this does not reset session
    /// persistence or the resume path — it only forces a tool-registry refresh.
    ///
    /// Before dropping, the live session path is stashed into
    /// `resume_session_path` so the next [`create_agent_service`] call resumes
    /// the same conversation instead of starting fresh.
    fn invalidate_agent_for_plugin_change(&self) {
        if let Ok(mut guard) = self.agent.lock() {
            if let Some(agent) = guard.as_ref()
                && let Some(path) = agent.session_path()
                && let Ok(mut resume) = self.resume_session_path.lock()
            {
                *resume = Some(path);
            }
            *guard = None;
        }
    }

    /// Build a fresh `AgentService` from current settings + plugin tools.
    fn create_agent_service(&self) -> Result<(AgentService, i64), String> {
        let config = self.resolved_config()?;
        let (mut config, settings_version) = self.settings.to_agent_config(config)?;

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

        let mut service = runtime
            .block_on(AgentService::new(config))
            .map_err(|e| format!("Agent init error: {e}"))?;

        // Register plugin tools natively in the agent's tool registry so the
        // LLM can invoke them during the agent loop (tool call -> execute ->
        // result -> next turn).
        service.extend_plugin_tools(Arc::clone(&self.plugin_tools) as Arc<dyn PluginToolProvider>);

        // Register native task tools so the LLM can manage tasks.
        let task_service: Arc<dyn peekoo_productivity_domain::task::TaskService> =
            Arc::new(self.productivity.clone());
        let task_tools = crate::task_tools::create_task_tools(task_service);
        service.register_native_tools(task_tools);

        Ok((service, settings_version))
    }

    fn resolved_config(&self) -> Result<AgentServiceConfig, String> {
        let skills_dir = self.workspace_dir.join("skills");
        let agent_skills = if skills_dir.is_dir() {
            vec![skills_dir]
        } else {
            Vec::new()
        };

        // Inject task activity summary so the agent has context.
        let system_prompt = self.productivity.task_activity_summary().ok();

        Ok(AgentServiceConfig {
            working_directory: self.workspace_dir.clone(),
            persona_dir: Some(self.workspace_dir.clone()),
            agent_skills,
            system_prompt,
            auto_discover: false,
            ..Default::default()
        })
    }

    fn agent_launch_env(&self) -> Vec<(String, String)> {
        let mut env = Vec::new();

        if let Ok(config) = self.resolved_config()
            && let Ok((resolved, _)) = self.settings.to_agent_config(config)
        {
            if let Some(provider) = resolved.provider {
                env.push(("PEEKOO_AGENT_PROVIDER".to_string(), provider));
            }
            if let Some(model) = resolved.model {
                env.push(("PEEKOO_AGENT_MODEL".to_string(), model));
            }
            if let Some(api_key) = resolved.api_key {
                env.push(("PEEKOO_AGENT_API_KEY".to_string(), api_key));
            }
        }

        if let Ok(data_dir) = peekoo_paths::peekoo_global_data_dir() {
            let task_session_dir = data_dir.join("task-agent-sessions");
            let _ = std::fs::create_dir_all(&task_session_dir);
            env.push((
                "PEEKOO_AGENT_TASK_SESSION_DIR".to_string(),
                task_session_dir.to_string_lossy().into_owned(),
            ));
        }

        env
    }

    fn task_runtime_service(&self) -> TaskRuntimeService {
        let scheduler_ref = Arc::clone(&self.agent_scheduler);
        let follow_up_trigger = Some(Arc::new(move |_task_id: String| {
            if let Ok(guard) = scheduler_ref.lock()
                && let Some(ref scheduler) = *guard
            {
                scheduler.trigger_now();
            }
        }) as Arc<dyn Fn(String) + Send + Sync>);

        TaskRuntimeService::new(
            self.productivity.clone(),
            Arc::clone(&self.notifications),
            follow_up_trigger,
        )
    }
}

fn should_restore_agent(captured_generation: u64, current_generation: u64) -> bool {
    captured_generation == current_generation
}

#[allow(clippy::type_complexity)]
fn create_plugin_registry(
    db_conn: Arc<Mutex<Connection>>,
    task_service: Arc<dyn peekoo_productivity_domain::task::TaskService>,
) -> Result<
    (
        Arc<PluginRegistry>,
        Arc<NotificationService>,
        UnboundedReceiver<Notification>,
        Arc<PeekBadgeService>,
        Arc<MoodReactionService>,
    ),
    String,
> {
    let global_plugins_dir = peekoo_paths::peekoo_global_data_dir()?.join("plugins");
    if !global_plugins_dir.exists() {
        std::fs::create_dir_all(&global_plugins_dir)
            .map_err(|e| format!("Create plugin dir error: {e}"))?;
    }

    let scheduler = Arc::new(Scheduler::new());
    let (notifications, receiver) = NotificationService::new();
    let notifications = Arc::new(notifications);
    let peek_badges = Arc::new(PeekBadgeService::new());
    let mood_reactions = Arc::new(MoodReactionService::new());
    let registry = Arc::new(PluginRegistry::new(
        vec![global_plugins_dir],
        db_conn,
        scheduler,
        Arc::clone(&notifications),
        Arc::clone(&peek_badges),
        Arc::clone(&mood_reactions),
        task_service,
    ));

    Ok((
        registry,
        notifications,
        receiver,
        peek_badges,
        mood_reactions,
    ))
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

    use peekoo_notifications::{MoodReactionService, NotificationService, PeekBadgeService};
    use peekoo_plugin_host::PluginRegistry;
    use peekoo_productivity_domain::task::{TaskDto, TaskService, TaskStatus};
    use peekoo_scheduler::Scheduler;
    use rusqlite::Connection;

    use super::{install_discovered_plugins, should_restore_agent};

    struct NoopTaskService;

    impl TaskService for NoopTaskService {
        fn create_task(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &[String],
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<u32>,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<TaskDto, String> {
            Err("not implemented".into())
        }
        fn list_tasks(&self) -> Result<Vec<TaskDto>, String> {
            Ok(vec![])
        }
        fn update_task(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&[String]>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<Option<u32>>,
            _: Option<Option<&str>>,
            _: Option<Option<&str>>,
        ) -> Result<TaskDto, String> {
            Err("not implemented".into())
        }
        fn delete_task(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn toggle_task(&self, _: &str) -> Result<TaskDto, String> {
            Err("not implemented".into())
        }
        fn get_task_activity(
            &self,
            _: &str,
            _: u32,
        ) -> Result<Vec<peekoo_productivity_domain::task::TaskEventDto>, String> {
            Ok(vec![])
        }
        fn add_task_comment(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<peekoo_productivity_domain::task::TaskEventDto, String> {
            Err("not implemented".into())
        }
        fn claim_task_for_agent(&self, _: &str) -> Result<bool, String> {
            Err("not implemented".into())
        }
        fn update_agent_work_status(
            &self,
            _: &str,
            _: &str,
            _: Option<&str>,
        ) -> Result<(), String> {
            Err("not implemented".into())
        }
        fn increment_attempt_count(&self, _: &str) -> Result<u32, String> {
            Err("not implemented".into())
        }
        fn list_tasks_for_agent_execution(&self) -> Result<Vec<TaskDto>, String> {
            Ok(vec![])
        }
        fn add_task_label(&self, _: &str, _: &str) -> Result<TaskDto, String> {
            Err("not implemented".into())
        }
        fn remove_task_label(&self, _: &str, _: &str) -> Result<TaskDto, String> {
            Err("not implemented".into())
        }
        fn update_task_status(&self, _: &str, _: TaskStatus) -> Result<TaskDto, String> {
            Err("not implemented".into())
        }
        fn load_task(&self, _: &str) -> Result<TaskDto, String> {
            Err("not implemented".into())
        }
    }

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
            Arc::new(MoodReactionService::new()),
            Arc::new(NoopTaskService),
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
