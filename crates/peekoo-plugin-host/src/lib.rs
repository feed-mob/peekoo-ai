//! Peekoo Plugin Host
//!
//! WASM-based plugin system using Extism. Plugins can extend four
//! integration points:
//!
//! - **Agent tools** – register tools the AI agent can call
//! - **UI panels** – provide HTML/JS/CSS panel windows
//! - **Event hooks** – subscribe to and emit system events
//! - **Data providers** – expose queryable data to the agent

pub mod config;
pub mod error;
pub mod events;
pub mod host_functions;
pub mod manifest;
pub mod permissions;
pub mod registry;
pub mod runtime;
pub mod state;
pub mod tools;

pub use config::{resolved_config_map, set_config_field};
pub use error::PluginError;
pub use events::{EventBus, PluginEvent};
pub use manifest::{
    CompanionDef, ConfigFieldDef, ConfigFieldType, PluginManifest, RuntimeDependencyDef,
    RuntimeDependencyKind, ToolDefinition, UiPanelDef,
};
pub use permissions::PermissionStore;
pub use registry::{PluginRegistry, resolve_companion_install_path, resolve_companion_target};
pub use runtime::PluginInstance;
pub use state::PluginStateStore;
pub use tools::{PluginToolBridge, PluginToolSpec};
