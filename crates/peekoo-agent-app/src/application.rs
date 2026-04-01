use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

use peekoo_agent::AgentEvent;
use peekoo_agent::config::{AgentServiceConfig, PEEKOO_OPENCODE_BIN_ENV};
use peekoo_agent::service::AgentService;
use peekoo_app_settings::{AppSettingsService, SpriteInfo};
use peekoo_notifications::{
    MoodReaction, MoodReactionService, Notification, NotificationService, PeekBadgeItem,
    PeekBadgeService,
};
use peekoo_paths::ensure_windows_pi_agent_env;
use peekoo_plugin_host::PluginRegistry;
use peekoo_pomodoro_app::{
    PomodoroAppService, PomodoroCycleDto, PomodoroSettingsInput, PomodoroStatusDto,
};
use peekoo_scheduler::Scheduler;

use crate::agent_provider_service::{
    AgentProviderService, InstallProviderRequest, InstallProviderResponse, InstallationMethod,
    PrerequisitesCheck, ProviderConfig, ProviderInfo, RuntimeInfo, RuntimeInspectionResult,
    TestConnectionResult,
};
use crate::conversation::{self, LastSessionDto, json_messages_to_dtos};
use crate::plugin::{
    PluginConfigFieldDto, PluginNotificationDto, PluginPanelDto, PluginSummaryDto,
    manifest_to_summary, plugin_notification_from_message,
};
use crate::plugin_tool_impl::PluginToolProviderImpl;
use peekoo_task_app::{TaskDto, TaskEventDto, TaskService};

use crate::settings::{
    AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto, OauthCancelResponse,
    OauthStartResponse, OauthStatusRequest, OauthStatusResponse, ProviderAuthDto,
    ProviderConfigDto, ProviderRequest, SetApiKeyRequest, SetProviderConfigRequest,
    SettingsService,
};
use crate::task_notification_scheduler::TaskNotificationScheduler;
use crate::task_runtime_service::TaskRuntimeService;
use peekoo_plugin_store::{PluginStoreService, StorePluginDto};
use peekoo_task_app::SqliteTaskService;

use crate::workspace_bootstrap::ensure_agent_workspace;

type TaskChangeCallback = Arc<dyn Fn(Option<String>) + Send + Sync>;

fn format_error_chain(err: &dyn std::error::Error) -> String {
    let mut chain = Vec::new();
    let mut current = err.source();
    while let Some(source) = current {
        chain.push(source.to_string());
        current = source.source();
    }

    if chain.is_empty() {
        "<none>".to_string()
    } else {
        chain.join(" -> ")
    }
}

pub struct AgentApplication {
    agent: Arc<Mutex<Option<AgentService>>>,
    settings: SettingsService,
    app_settings: Arc<AppSettingsService>,
    task_service: SqliteTaskService,
    task_change_callback: Mutex<Option<TaskChangeCallback>>,
    pomodoro: Arc<PomodoroAppService>,
    plugin_registry: Arc<PluginRegistry>,
    plugin_tools: Arc<PluginToolProviderImpl>,
    plugin_store: PluginStoreService,
    provider_service: Arc<AgentProviderService>,
    notifications: Arc<NotificationService>,
    notification_receiver: Mutex<UnboundedReceiver<Notification>>,
    task_notifications: Arc<TaskNotificationScheduler>,
    peek_badges: Arc<PeekBadgeService>,
    mood_reactions: Arc<MoodReactionService>,
    shutdown_token: CancellationToken,
    agent_config_version: Mutex<Option<i64>>,
    /// Directory where pi session files are stored.
    session_dir: PathBuf,
    /// Agent workspace root (e.g. `~/.peekoo/workspace/`).
    agent_workspace_dir: PathBuf,
    /// Path to the last session file, used to resume context on the next prompt.
    resume_session_path: Mutex<Option<PathBuf>>,
    /// Monotonic generation that invalidates in-flight agents after `new_session`.
    conversation_generation: AtomicU64,
    /// Scheduler for agent task execution.
    agent_scheduler: Arc<Mutex<Option<crate::agent_scheduler::AgentScheduler>>>,
    bundled_opencode_path: Option<PathBuf>,
}

impl AgentApplication {
    pub fn new() -> Result<Self, String> {
        Self::new_with_bundled_opencode(None)
    }

    pub fn new_with_bundled_opencode(
        bundled_opencode_path: Option<PathBuf>,
    ) -> Result<Self, String> {
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
        let app_settings = Arc::new(AppSettingsService::with_conn(Arc::clone(&db_conn))?);
        let sqlite_task_service = SqliteTaskService::new(Arc::clone(&db_conn));
        let task_service: Arc<dyn peekoo_task_app::TaskService> =
            Arc::new(sqlite_task_service.clone());
        let (notifications, notification_receiver) = NotificationService::new();
        let notifications = Arc::new(notifications);
        let task_notifications = Arc::new(TaskNotificationScheduler::new(
            sqlite_task_service.clone(),
            Arc::clone(&notifications),
        ));
        task_notifications.start()?;
        let peek_badges = Arc::new(PeekBadgeService::new());
        let mood_reactions = Arc::new(MoodReactionService::new());
        let pomodoro = Arc::new(PomodoroAppService::new(
            Arc::clone(&db_conn),
            Arc::clone(&notifications),
            Arc::clone(&peek_badges),
            Arc::clone(&mood_reactions),
        )?);
        let plugin_registry = create_plugin_registry(
            db_conn,
            task_service,
            Arc::clone(&notifications),
            Arc::clone(&peek_badges),
            Arc::clone(&mood_reactions),
        )?;
        let shutdown_token = plugin_registry.scheduler().shutdown_token();
        install_discovered_plugins(&plugin_registry);

        let session_dir = peekoo_paths::peekoo_global_data_dir()?.join("sessions");
        if !session_dir.exists() {
            std::fs::create_dir_all(&session_dir)
                .map_err(|e| format!("Create session dir error: {e}"))?;
        }
        let provider_service = Arc::new(
            AgentProviderService::new_with_bundled_opencode(
                &db_path,
                peekoo_paths::peekoo_global_data_dir()?,
                bundled_opencode_path.clone(),
            )
            .map_err(|e| format!("Create provider service error: {e}"))?,
        );
        let agent_workspace_dir = ensure_agent_workspace()?;

        // Create agent scheduler for task execution
        let agent_scheduler =
            crate::agent_scheduler::AgentScheduler::new(Arc::new(sqlite_task_service.clone()));

        Ok(Self {
            agent: Arc::new(Mutex::new(None)),
            settings,
            app_settings,
            task_service: sqlite_task_service,
            task_change_callback: Mutex::new(None),
            pomodoro,
            plugin_tools: Arc::new(PluginToolProviderImpl::new(Arc::clone(&plugin_registry))),
            plugin_registry,
            plugin_store: PluginStoreService::new(),
            provider_service,
            notifications,
            notification_receiver: Mutex::new(notification_receiver),
            task_notifications,
            peek_badges,
            mood_reactions,
            shutdown_token,
            agent_config_version: Mutex::new(None),
            session_dir,
            agent_workspace_dir,
            resume_session_path: Mutex::new(None),
            conversation_generation: AtomicU64::new(0),
            agent_scheduler: Arc::new(Mutex::new(Some(agent_scheduler))),
            bundled_opencode_path,
        })
    }

    pub fn set_task_change_callback(
        &self,
        callback: Arc<dyn Fn(Option<String>) + Send + Sync>,
    ) -> Result<(), String> {
        let mut guard = self
            .task_change_callback
            .lock()
            .map_err(|e| format!("Task change callback lock error: {e}"))?;
        *guard = Some(callback);
        Ok(())
    }

    pub fn start_plugin_runtime(&self) {
        self.plugin_registry.start_scheduler();

        eprintln!("[peekoo][mcp] starting MCP server during app startup");

        // Start MCP server on a dedicated thread (survives app lifetime)
        let task_service: Arc<dyn peekoo_task_app::TaskService> =
            Arc::new(self.task_runtime_service());
        let pomodoro_service = Arc::clone(&self.pomodoro);
        let app_settings_service = Arc::clone(&self.app_settings);
        let mcp_shutdown = self.shutdown_token.clone();

        let plugin_registry = Some(Arc::clone(&self.plugin_registry));
        match crate::mcp_server::start_sync(
            task_service,
            pomodoro_service,
            app_settings_service,
            plugin_registry,
            mcp_shutdown,
            self.agent_workspace_dir.clone(),
        ) {
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
            // Build context prompt for task scheduler
            if let Ok(config) = self.resolved_config()
                && let Ok(prompt) =
                    peekoo_agent::service::AgentService::build_system_prompt(&config)
            {
                scheduler.set_context_prompt(prompt);
            }
            scheduler.start();
        }
    }

    pub async fn prompt_streaming<F>(&self, message: &str, on_event: F) -> Result<String, String>
    where
        F: Fn(AgentEvent) + Send + Sync + 'static,
    {
        tracing::info!(
            message_len = message.chars().count(),
            "Application prompt_streaming requested"
        );
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

        if let Err(err) = &result {
            tracing::error!(
                error = %err,
                debug = ?err,
                sources = %format_error_chain(err.as_ref()),
                conversation_generation = generation,
                message_len = message.chars().count(),
                "Agent prompt failed"
            );
        } else if let Ok(reply) = &result {
            tracing::info!(
                message_len = message.chars().count(),
                response_len = reply.chars().count(),
                conversation_generation = generation,
                "Application prompt_streaming completed"
            );
        }

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

    pub async fn settings_catalog(&self) -> Result<AgentSettingsCatalogDto, String> {
        self.settings
            .catalog_from_runtimes(&self.provider_service)
            .await
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

    pub fn list_agent_providers(&self) -> Result<Vec<ProviderInfo>, String> {
        self.provider_service
            .list_providers()
            .map_err(|e| format!("List providers error: {e}"))
    }

    pub fn list_agent_runtimes(&self) -> Result<Vec<RuntimeInfo>, String> {
        self.provider_service
            .list_runtimes()
            .map_err(|e| format!("List runtimes error: {e}"))
    }

    pub fn install_agent_provider(
        &self,
        req: InstallProviderRequest,
    ) -> Result<InstallProviderResponse, String> {
        let response = self
            .provider_service
            .install_provider(req.clone())
            .map_err(|e| format!("Install provider error: {e}"))?;

        if response.success {
            // Only auto-promote runtimes that are visible in chat/runtime selection.
            let runtime_info = self
                .provider_service
                .get_runtime(&req.provider_id)
                .map_err(|e| format!("Get runtime info error: {e}"))?;

            if runtime_info.map(|r| r.is_chat_visible()).unwrap_or(false) {
                let provider = self
                    .provider_service
                    .get_default_provider()
                    .map_err(|e| format!("Get default provider error: {e}"))?;

                let should_promote_to_default =
                    provider.map(|p| !p.is_chat_visible()).unwrap_or(true);

                if should_promote_to_default {
                    self.provider_service
                        .set_default_provider(&req.provider_id)
                        .map_err(|e| format!("Set default provider error: {e}"))?;
                }
            }
        }

        Ok(response)
    }

    pub fn install_agent_runtime(
        &self,
        req: InstallProviderRequest,
    ) -> Result<InstallProviderResponse, String> {
        self.install_agent_provider(req)
    }

    pub fn uninstall_agent_provider(&self, provider_id: &str) -> Result<(), String> {
        self.provider_service
            .uninstall_provider(provider_id)
            .map_err(|e| format!("Uninstall provider error: {e}"))
    }

    pub fn uninstall_agent_runtime(&self, runtime_id: &str) -> Result<(), String> {
        self.provider_service
            .uninstall_runtime(runtime_id)
            .map_err(|e| format!("Uninstall runtime error: {e}"))
    }

    pub fn set_default_agent_provider(&self, provider_id: &str) -> Result<(), String> {
        self.provider_service
            .set_default_provider(provider_id)
            .map_err(|e| format!("Set default provider error: {e}"))?;

        self.settings.bump_version()
    }

    pub fn set_default_agent_runtime(&self, runtime_id: &str) -> Result<(), String> {
        self.set_default_agent_provider(runtime_id)
    }

    pub fn get_agent_provider_config(&self, provider_id: &str) -> Result<ProviderConfig, String> {
        self.provider_service
            .get_provider_config(provider_id)
            .map_err(|e| format!("Get provider config error: {e}"))
    }

    pub fn update_agent_provider_config(
        &self,
        provider_id: &str,
        config: &ProviderConfig,
    ) -> Result<(), String> {
        self.provider_service
            .update_provider_config(provider_id, config)
            .map_err(|e| format!("Update provider config error: {e}"))?;

        // Bump version if this is the default runtime, so the agent recreates on next prompt.
        let is_default = self
            .provider_service
            .get_default_runtime()
            .map_err(|e| format!("Get default runtime error: {e}"))?
            .map(|r| r.provider_id == provider_id)
            .unwrap_or(false);

        if is_default {
            self.settings.bump_version()?;
        }

        Ok(())
    }

    pub async fn test_agent_provider_connection(
        &self,
        provider_id: &str,
    ) -> Result<TestConnectionResult, String> {
        self.provider_service
            .test_connection(provider_id)
            .await
            .map_err(|e| format!("Test provider connection error: {e}"))
    }

    pub fn check_agent_provider_prerequisites(
        &self,
        method: InstallationMethod,
    ) -> Result<PrerequisitesCheck, String> {
        self.provider_service
            .check_prerequisites(method)
            .map_err(|e| format!("Check prerequisites error: {e}"))
    }

    pub fn add_custom_agent_provider(
        &self,
        name: &str,
        description: Option<&str>,
        command: &str,
        args: &[String],
        working_dir: Option<&str>,
    ) -> Result<ProviderInfo, String> {
        self.provider_service
            .add_custom_provider(name, description, command, args, working_dir)
            .map_err(|e| format!("Add custom provider error: {e}"))
    }

    pub fn remove_custom_agent_provider(&self, provider_id: &str) -> Result<(), String> {
        self.provider_service
            .remove_custom_provider(provider_id)
            .map_err(|e| format!("Remove custom provider error: {e}"))
    }

    pub fn default_agent_provider(&self) -> Result<Option<ProviderInfo>, String> {
        self.provider_service
            .get_default_provider()
            .map_err(|e| format!("Get default provider error: {e}"))
    }

    pub fn default_agent_runtime(&self) -> Result<Option<RuntimeInfo>, String> {
        self.provider_service
            .get_default_runtime()
            .map_err(|e| format!("Get default runtime error: {e}"))
    }

    /// Get the default runtime for chat use.
    /// Returns None if no chat-visible runtime is set as default.
    pub fn default_chat_runtime(&self) -> Result<Option<RuntimeInfo>, String> {
        match self.default_agent_runtime()? {
            Some(runtime) if runtime.is_chat_visible() => Ok(Some(runtime)),
            _ => Ok(None),
        }
    }

    /// Get the first installed chat-visible runtime, or None if none installed.
    pub fn first_chat_runtime(&self) -> Result<Option<RuntimeInfo>, String> {
        let runtimes = self.list_agent_runtimes()?;
        Ok(runtimes
            .into_iter()
            .find(|r| r.is_installed && r.is_chat_visible()))
    }

    /// Inspect a runtime to discover its capabilities via ACP
    pub async fn inspect_runtime(
        &self,
        runtime_id: &str,
    ) -> Result<RuntimeInspectionResult, String> {
        crate::agent_provider_commands::inspect_runtime(
            &self.provider_service,
            runtime_id.to_string(),
        )
        .await
        .map_err(|e| format!("Runtime inspection error: {e}"))
    }

    /// Authenticate with a runtime using the specified auth method
    pub async fn authenticate_runtime(
        &self,
        runtime_id: &str,
        method_id: &str,
    ) -> Result<crate::agent_provider_commands::RuntimeAuthenticationAction, String> {
        crate::agent_provider_commands::authenticate_runtime(
            &self.provider_service,
            runtime_id.to_string(),
            method_id.to_string(),
        )
        .await
        .map_err(|e| format!("Runtime authentication error: {e}"))
    }

    /// Refresh runtime capabilities by re-inspecting
    pub async fn refresh_runtime_capabilities(
        &self,
        runtime_id: &str,
    ) -> Result<RuntimeInspectionResult, String> {
        crate::agent_provider_commands::refresh_runtime_capabilities(
            &self.provider_service,
            runtime_id.to_string(),
        )
        .await
        .map_err(|e| format!("Runtime refresh error: {e}"))
    }

    // =========================================================================
    // ACP Registry Methods
    // =========================================================================

    /// Fetch agents from ACP registry with filtering and pagination
    pub async fn fetch_registry_agents(
        &self,
        filter: &crate::agent_provider_service::RegistryFilterOptions,
    ) -> Result<(Vec<crate::agent_provider_service::RegistryAgentInfo>, usize), String> {
        self.provider_service
            .fetch_registry_agents(filter)
            .await
            .map_err(|e| format!("Fetch registry agents error: {e}"))
    }

    /// Search registry agents by query
    pub async fn search_registry_agents(
        &self,
        query: &str,
    ) -> Result<Vec<crate::agent_provider_service::RegistryAgentInfo>, String> {
        self.provider_service
            .search_registry_agents(query)
            .await
            .map_err(|e| format!("Search registry agents error: {e}"))
    }

    /// Install an agent from ACP registry
    pub async fn install_registry_agent(
        &self,
        registry_id: &str,
        method: crate::agent_provider_service::InstallationMethod,
    ) -> Result<crate::agent_provider_service::InstallProviderResponse, String> {
        self.provider_service
            .install_registry_agent(registry_id, method)
            .await
            .map_err(|e| format!("Install registry agent error: {e}"))
    }

    /// Force refresh registry from CDN
    pub async fn refresh_registry(&self) -> Result<(), String> {
        self.provider_service
            .refresh_registry()
            .await
            .map_err(|e| format!("Refresh registry error: {e}"))
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

    // ── Tasks ───────────────────────────────────────────────────────────

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
        self.task_runtime_service().create_task(
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
        self.task_service.list_tasks()
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
        self.task_runtime_service().update_task(
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
        self.task_runtime_service().delete_task(id)
    }

    pub fn toggle_task(&self, id: &str) -> Result<TaskDto, String> {
        self.task_runtime_service().toggle_task(id)
    }

    /// Create a task from natural language text
    /// Parses the text to extract title, priority, schedule, duration, etc.
    /// Falls back to using the whole text as title if parsing fails.
    pub fn create_task_from_text(&self, text: &str) -> Result<TaskDto, String> {
        use crate::task_parser::parse_task_text;

        let parsed = parse_task_text(text);

        self.task_runtime_service().create_task(
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
        self.task_service.get_task_activity(task_id, limit)
    }

    pub fn list_task_events(&self, limit: i64) -> Result<Vec<TaskEventDto>, String> {
        self.task_service.list_task_events(limit)
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
        self.task_service.delete_task_event(event_id)
    }

    pub fn task_activity_summary(&self) -> Result<String, String> {
        self.task_service.task_activity_summary()
    }

    pub fn pomodoro_status(&self) -> Result<PomodoroStatusDto, String> {
        self.pomodoro.get_status()
    }

    pub fn pomodoro_set_settings(
        &self,
        input: PomodoroSettingsInput,
    ) -> Result<PomodoroStatusDto, String> {
        self.pomodoro.set_settings(input)
    }

    pub fn start_pomodoro(&self, mode: &str, minutes: u32) -> Result<PomodoroStatusDto, String> {
        self.pomodoro.start(mode, minutes)
    }

    pub fn pause_pomodoro(&self) -> Result<PomodoroStatusDto, String> {
        self.pomodoro.pause()
    }

    pub fn resume_pomodoro(&self) -> Result<PomodoroStatusDto, String> {
        self.pomodoro.resume()
    }

    pub fn finish_pomodoro(&self) -> Result<PomodoroStatusDto, String> {
        self.pomodoro.finish()
    }

    pub fn switch_pomodoro_mode(&self, mode: &str) -> Result<PomodoroStatusDto, String> {
        self.pomodoro.switch_mode(mode)
    }

    pub fn save_pomodoro_memo(
        &self,
        id: Option<String>,
        memo: String,
    ) -> Result<PomodoroStatusDto, String> {
        self.pomodoro.save_pomodoro_memo(id, memo)
    }

    pub fn pomodoro_history(&self, limit: usize) -> Result<Vec<PomodoroCycleDto>, String> {
        self.pomodoro.history(limit)
    }

    pub fn pomodoro_history_by_date_range(
        &self,
        start_date: String,
        end_date: String,
        limit: usize,
    ) -> Result<Vec<PomodoroCycleDto>, String> {
        self.pomodoro
            .history_by_date_range(&start_date, &end_date, limit)
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

        if !notifications.is_empty() {
            let sources = notifications
                .iter()
                .map(|notification| notification.source_plugin.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            tracing::debug!(
                count = notifications.len(),
                sources,
                "Drained notifications"
            );
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
                    last_message_timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64,
                    messages: dtos,
                }));
            }
        }

        // Slow path: load from disk.
        let result = conversation::find_last_session()
            .map_err(|e| format!("Failed to load last session: {e}"))?;

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

    /// Build a fresh `AgentService` from current settings + MCP-backed tools.
    fn create_agent_service(&self) -> Result<(AgentService, i64), String> {
        let config = self.resolved_config()?;

        // Resolve provider and model from the default runtime (single source of truth).
        let default_runtime = self
            .provider_service
            .get_default_runtime()
            .map_err(|e| format!("Get default runtime error: {e}"))?
            .ok_or_else(|| "No default runtime configured".to_string())?;

        // Build AgentProvider from the runtime
        let provider = if default_runtime.is_bundled {
            // Built-in providers have factory functions
            match default_runtime.provider_id.as_str() {
                "opencode" => peekoo_agent::config::AgentProvider::opencode(),
                "pi-acp" => peekoo_agent::config::AgentProvider::pi_acp(),
                "claude-code" => peekoo_agent::config::AgentProvider::claude_code(),
                "codex" => peekoo_agent::config::AgentProvider::codex(),
                _ => peekoo_agent::config::AgentProvider::from_registry(
                    &default_runtime.provider_id,
                    &default_runtime.command,
                    default_runtime.args.clone(),
                ),
            }
        } else {
            // Registry-installed or custom providers use from_registry
            peekoo_agent::config::AgentProvider::from_registry(
                &default_runtime.provider_id,
                &default_runtime.command,
                default_runtime.args.clone(),
            )
        };

        let model_id = default_runtime.config.default_model.as_deref();

        let (mut config, settings_version) =
            self.settings.to_agent_config(config, provider, model_id)?;

        // Enable session persistence.
        config.no_session = false;
        config.session_dir = Some(self.session_dir.clone());

        let runtime_id = config.provider.id();

        // Apply adapter-specific launch env from the runtime's ProviderConfig.
        if let Ok(runtime_config) = self.provider_service.get_provider_config(&runtime_id) {
            let adapter = crate::runtime_adapters::adapter_for_runtime(&runtime_id);
            let adapter_env = adapter.build_launch_env(&runtime_config);
            for (key, value) in adapter_env {
                config.environment.entry(key).or_insert(value);
            }
        }

        if let Some(path) = &self.bundled_opencode_path {
            config.environment.insert(
                PEEKOO_OPENCODE_BIN_ENV.to_string(),
                path.to_string_lossy().into_owned(),
            );
        }

        config.mcp_servers =
            crate::agent_scheduler::build_session_mcp_servers(crate::mcp_server::get_mcp_address());

        // If get_last_session stashed a path, resume that session for full
        // context restore. The path is consumed so it is only used once.
        if let Ok(mut guard) = self.resume_session_path.lock()
            && let Some(path) = guard.take()
        {
            config.resume_session_id = Some(path.to_string_lossy().into_owned());
        }

        let reactor = asupersync::runtime::reactor::create_reactor()
            .map_err(|e| format!("Reactor error: {e}"))?;
        let runtime = asupersync::runtime::RuntimeBuilder::current_thread()
            .with_reactor(reactor)
            .build()
            .map_err(|e| format!("Runtime error: {e}"))?;

        let service = runtime.block_on(AgentService::new(config)).map_err(|e| {
            if peekoo_agent::backend::acp::is_auth_required_error(&e) {
                // Use a structured prefix so callers (e.g. Tauri command layer)
                // can distinguish auth failures from generic init errors.
                format!("AUTH_REQUIRED:{runtime_id}")
            } else {
                format!("Agent init error: {e}")
            }
        })?;

        Ok((service, settings_version))
    }

    fn resolved_config(&self) -> Result<AgentServiceConfig, String> {
        let skills_dir = self.agent_workspace_dir.join("skills");
        let agent_skills = if skills_dir.is_dir() {
            vec![skills_dir]
        } else {
            Vec::new()
        };

        // Inject task activity summary so the agent has context.
        let system_prompt = self.task_service.task_activity_summary().ok();

        Ok(AgentServiceConfig {
            working_directory: self.agent_workspace_dir.clone(),
            persona_dir: Some(self.agent_workspace_dir.clone()),
            agent_skills,
            system_prompt,
            auto_discover: false,
            ..Default::default()
        })
    }

    fn agent_launch_env(&self) -> Vec<(String, String)> {
        let mut env = Vec::new();

        if let Ok(config) = self.resolved_config()
            && let Ok(Some(runtime)) = self.provider_service.get_default_runtime()
        {
            // Build AgentProvider from the runtime
            let provider = if runtime.is_bundled {
                match runtime.provider_id.as_str() {
                    "opencode" => peekoo_agent::config::AgentProvider::opencode(),
                    "pi-acp" => peekoo_agent::config::AgentProvider::pi_acp(),
                    "claude-code" => peekoo_agent::config::AgentProvider::claude_code(),
                    "codex" => peekoo_agent::config::AgentProvider::codex(),
                    _ => peekoo_agent::config::AgentProvider::from_registry(
                        &runtime.provider_id,
                        &runtime.command,
                        runtime.args.clone(),
                    ),
                }
            } else {
                peekoo_agent::config::AgentProvider::from_registry(
                    &runtime.provider_id,
                    &runtime.command,
                    runtime.args.clone(),
                )
            };

            if let Ok((resolved, _)) = self.settings.to_agent_config(
                config,
                provider,
                runtime.config.default_model.as_deref(),
            ) {
                let runtime_id = resolved.provider.id();

                env.push(("PEEKOO_AGENT_PROVIDER".to_string(), runtime_id.clone()));

                // Apply model from resolved config
                if let Some(model) = resolved.model {
                    env.push(("PEEKOO_AGENT_MODEL".to_string(), model));
                }
                if let Some(api_key) = resolved.api_key {
                    env.push(("PEEKOO_AGENT_API_KEY".to_string(), api_key));
                }
            }
        }

        if let Some(path) = &self.bundled_opencode_path {
            env.push((
                PEEKOO_OPENCODE_BIN_ENV.to_string(),
                path.to_string_lossy().into_owned(),
            ));
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

        let task_change_callback = self
            .task_change_callback
            .lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(Arc::clone));

        TaskRuntimeService::new(
            self.task_service.clone(),
            Arc::clone(&self.notifications),
            Arc::clone(&self.task_notifications),
            follow_up_trigger,
            task_change_callback,
        )
    }
}

fn should_restore_agent(captured_generation: u64, current_generation: u64) -> bool {
    captured_generation == current_generation
}

fn create_plugin_registry(
    db_conn: Arc<Mutex<Connection>>,
    task_service: Arc<dyn peekoo_task_app::TaskService>,
    notifications: Arc<NotificationService>,
    peek_badges: Arc<PeekBadgeService>,
    mood_reactions: Arc<MoodReactionService>,
) -> Result<Arc<PluginRegistry>, String> {
    let global_plugins_dir = peekoo_paths::peekoo_global_data_dir()?.join("plugins");
    if !global_plugins_dir.exists() {
        std::fs::create_dir_all(&global_plugins_dir)
            .map_err(|e| format!("Create plugin dir error: {e}"))?;
    }

    let scheduler = Arc::new(Scheduler::new());
    let registry = Arc::new(PluginRegistry::new(
        vec![global_plugins_dir],
        db_conn,
        scheduler,
        Arc::clone(&notifications),
        Arc::clone(&peek_badges),
        Arc::clone(&mood_reactions),
        task_service,
    ));

    Ok(registry)
}

fn install_discovered_plugins(plugin_registry: &Arc<PluginRegistry>) {
    let discovered = plugin_registry.discover();
    tracing::info!("Plugin discovery: found {} plugin(s)", discovered.len());
    for (plugin_dir, manifest) in discovered {
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
            Err(err) => tracing::error!(
                plugin = manifest.plugin.key.as_str(),
                dir = %plugin_dir.display(),
                "Plugin load FAILED: {err}"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use peekoo_notifications::{MoodReactionService, NotificationService, PeekBadgeService};
    use peekoo_plugin_host::PluginRegistry;
    use peekoo_scheduler::Scheduler;
    use peekoo_task_app::{TaskDto, TaskService};
    use peekoo_task_domain::TaskStatus;
    use rusqlite::Connection;

    use super::{format_error_chain, install_discovered_plugins, should_restore_agent};
    use std::error::Error as StdError;
    use std::fmt;

    #[derive(Debug)]
    struct TestError {
        message: &'static str,
        source: Option<Box<dyn StdError + Send + Sync>>,
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl StdError for TestError {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            self.source
                .as_deref()
                .map(|err| err as &(dyn StdError + 'static))
        }
    }

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
        ) -> Result<Vec<peekoo_task_app::TaskEventDto>, String> {
            Ok(vec![])
        }
        fn add_task_comment(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<peekoo_task_app::TaskEventDto, String> {
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
    fn format_error_chain_returns_none_for_error_without_sources() {
        let err = TestError {
            message: "top",
            source: None,
        };

        assert_eq!(format_error_chain(&err), "<none>");
    }

    #[test]
    fn format_error_chain_flattens_nested_sources() {
        let err = TestError {
            message: "top",
            source: Some(Box::new(TestError {
                message: "middle",
                source: Some(Box::new(TestError {
                    message: "root",
                    source: None,
                })),
            })),
        };

        assert_eq!(format_error_chain(&err), "middle -> root");
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
