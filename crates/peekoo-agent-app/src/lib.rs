pub mod application;
pub mod conversation;
pub mod plugin;
pub mod productivity;
pub mod settings;

pub use application::AgentApplication;
pub use conversation::{LastSessionDto, SessionMessageDto};
pub use peekoo_notifications::PeekBadgeItem;
pub use peekoo_plugin_store::{PluginSource, StorePluginDto};
pub use plugin::{PluginConfigFieldDto, PluginNotificationDto, PluginPanelDto, PluginSummaryDto};
pub use productivity::{PomodoroSessionDto, ProductivityService, TaskDto};
pub use settings::{
    AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto, OauthCancelResponse,
    OauthStartResponse, OauthStatusRequest, OauthStatusResponse, ProviderAuthDto,
    ProviderCatalogDto, ProviderConfigDto, ProviderRequest, SetApiKeyRequest,
    SetProviderConfigRequest, SettingsService, SkillDto,
};
