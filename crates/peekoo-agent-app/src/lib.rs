pub mod agent_scheduler;
pub mod application;
pub mod conversation;
pub mod mcp_server;
pub mod plugin;
pub mod plugin_tool_impl;
pub mod productivity;
pub mod settings;
pub mod task_parser;
mod task_runtime_service;
pub mod task_tools;
mod workspace_bootstrap;

pub use application::AgentApplication;
pub use conversation::{LastSessionDto, SessionMessageDto};
pub use peekoo_app_settings::{AppSettingsService, SpriteInfo};
pub use peekoo_notifications::PeekBadgeItem;
pub use peekoo_pomodoro_app::{PomodoroCycleDto, PomodoroSettingsInput, PomodoroStatusDto};
pub use peekoo_plugin_store::{PluginSource, StorePluginDto};
pub use peekoo_productivity_domain::task::{TaskDto, TaskEventDto, TaskService};
pub use plugin::{PluginConfigFieldDto, PluginNotificationDto, PluginPanelDto, PluginSummaryDto};
pub use productivity::ProductivityService;
pub use settings::{
    AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto, OauthCancelResponse,
    OauthStartResponse, OauthStatusRequest, OauthStatusResponse, ProviderAuthDto,
    ProviderCatalogDto, ProviderConfigDto, ProviderRequest, SetApiKeyRequest,
    SetProviderConfigRequest, SettingsService, SkillDto,
};
