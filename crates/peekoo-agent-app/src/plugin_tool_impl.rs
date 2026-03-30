//! Concrete PluginToolProvider implementation backed by PluginToolBridge and PluginRegistry.
//!
//! This bridges the dependency-inverted trait to the concrete plugin host runtime.
//!
//! TODO: Reimplement using new MCP bridge architecture after pi migration
//!
//! This file is temporarily disabled during the migration from pi_agent_rust
//! to the ACP client architecture.

use peekoo_plugin_host::{PluginRegistry, PluginToolBridge};
use std::sync::Arc;

/// Newtype around PluginToolBridge.
///
/// TODO: Reimplement PluginToolProvider trait using new architecture
pub struct PluginToolProviderImpl {
    bridge: PluginToolBridge,
}

impl PluginToolProviderImpl {
    pub fn new(registry: Arc<PluginRegistry>) -> Self {
        Self {
            bridge: PluginToolBridge::new(Arc::clone(&registry)),
        }
    }

    /// Execute a plugin tool by name (delegates to the inner bridge).
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

// TODO: Reimplement PluginToolProvider trait
// impl PluginToolProvider for PluginToolProviderImpl {
//     fn tool_specs(&self) -> Vec<AgentPluginToolSpec> {
//         ...
//     }
//
//     fn call_tool(...)
// }
