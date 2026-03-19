pub mod application;
pub mod conversation;
pub mod google_calendar;
pub mod google_calendar_service;
pub mod plugin;
pub mod plugin_tool_impl;
pub mod productivity;
pub mod settings;
mod workspace_bootstrap;

pub use application::AgentApplication;
pub use conversation::{LastSessionDto, SessionMessageDto};
pub use peekoo_app_settings::{AppSettingsService, SpriteInfo};
pub use peekoo_notifications::PeekBadgeItem;
pub use peekoo_plugin_store::{PluginSource, StorePluginDto};
pub use plugin::{PluginConfigFieldDto, PluginNotificationDto, PluginPanelDto, PluginSummaryDto};
pub use productivity::{PomodoroSessionDto, ProductivityService, TaskDto};
pub use google_calendar_service::{
    GoogleAccountProfile, GoogleCalendarOauthStatusDto, GoogleCalendarPanelDto,
    GoogleCalendarStatusDto, parse_google_account_profile, parse_google_client_json,
};
pub use settings::{
    AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto, OauthCancelResponse,
    OauthStartResponse, OauthStatusRequest, OauthStatusResponse, ProviderAuthDto,
    ProviderCatalogDto, ProviderConfigDto, ProviderRequest, SetApiKeyRequest,
    SetProviderConfigRequest, SettingsService, SkillDto,
};
