//! Concrete [`PluginToolProvider`] implementation backed by
//! [`PluginToolBridge`].
//!
//! This bridges the dependency-inverted trait defined in `peekoo-agent` to the
//! concrete plugin host runtime in `peekoo-plugin-host`, keeping the agent
//! crate free of plugin-host dependencies.
//!
//! Because `PluginToolBridge` has inherent methods with the same names as the
//! trait methods (`tool_specs`, `call_tool`), we wrap it in a newtype to avoid
//! ambiguity.

use std::sync::Arc;

use peekoo_agent::plugin_tool::{PluginToolProvider, PluginToolSpec as AgentPluginToolSpec};
use peekoo_plugin_host::{PluginRegistry, PluginToolBridge};

/// Newtype around [`PluginToolBridge`] that implements [`PluginToolProvider`].
///
/// Using a newtype avoids method name collisions between the trait and
/// `PluginToolBridge`'s inherent methods.
pub struct PluginToolProviderImpl {
    bridge: PluginToolBridge,
}

impl PluginToolProviderImpl {
    pub fn new(registry: Arc<PluginRegistry>) -> Self {
        Self {
            bridge: PluginToolBridge::new(registry),
        }
    }

    /// Execute a plugin tool by name (delegates to the inner bridge).
    ///
    /// This is used by the Tauri command layer for frontend-initiated tool
    /// calls, independent of the agent's tool loop.
    pub fn call_plugin_tool(&self, tool_name: &str, args_json: &str) -> Result<String, String> {
        self.bridge
            .call_tool(tool_name, args_json)
            .map_err(|e| e.to_string())
    }

    /// Check if a tool name belongs to a plugin.
    pub fn is_plugin_tool(&self, tool_name: &str) -> bool {
        self.bridge.is_plugin_tool(tool_name)
    }
}

impl PluginToolProvider for PluginToolProviderImpl {
    fn tool_specs(&self) -> Vec<AgentPluginToolSpec> {
        self.bridge
            .tool_specs()
            .into_iter()
            .map(|s| AgentPluginToolSpec {
                name: s.name,
                description: s.description,
                parameters_schema: s.parameters_schema,
                plugin_key: s.plugin_key,
            })
            .collect()
    }

    fn call_tool(&self, tool_name: &str, args_json: &str) -> Result<String, String> {
        self.bridge
            .call_tool(tool_name, args_json)
            .map_err(|e| e.to_string())
    }
}
