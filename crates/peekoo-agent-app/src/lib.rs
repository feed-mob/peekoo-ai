pub mod application;
pub mod conversation;
pub mod plugin;
pub mod plugin_tool_impl;
pub mod productivity;
pub mod settings;
pub mod task_tools;
mod workspace_bootstrap;

pub use application::AgentApplication;
pub use conversation::{LastSessionDto, SessionMessageDto};
pub use peekoo_app_settings::{AppSettingsService, SpriteInfo};
pub use peekoo_notifications::PeekBadgeItem;
pub use peekoo_plugin_store::{PluginSource, StorePluginDto};
pub use peekoo_productivity_domain::task::{TaskDto, TaskEventDto, TaskService};
pub use plugin::{PluginConfigFieldDto, PluginNotificationDto, PluginPanelDto, PluginSummaryDto};
pub use productivity::{PomodoroSessionDto, ProductivityService};
pub use settings::{
    AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto, OauthCancelResponse,
    OauthStartResponse, OauthStatusRequest, OauthStatusResponse, ProviderAuthDto,
    ProviderCatalogDto, ProviderConfigDto, ProviderRequest, SetApiKeyRequest,
    SetProviderConfigRequest, SettingsService, SkillDto,
};
