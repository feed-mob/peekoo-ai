use std::sync::Arc;

use crate::error::PluginError;
use crate::registry::PluginRegistry;

/// Describes a plugin-provided tool in a format the agent can consume.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginToolSpec {
    /// Tool name as declared by the plugin.
    pub name: String,
    pub description: String,
    /// JSON Schema for parameters.
    pub parameters_schema: serde_json::Value,
    /// Which plugin owns this tool.
    pub plugin_key: String,
}

/// Adapter that sits between the AI agent and the plugin registry.
///
/// It collects tool definitions from all loaded plugins and routes agent
/// tool calls to the correct plugin.
pub struct PluginToolBridge {
    registry: Arc<PluginRegistry>,
}

impl PluginToolBridge {
    pub fn new(registry: Arc<PluginRegistry>) -> Self {
        Self { registry }
    }

    /// Collect all tool specs from loaded plugins, suitable for injection
    /// into the agent's system prompt.
    pub fn tool_specs(&self) -> Vec<PluginToolSpec> {
        self.registry
            .all_tool_definitions()
            .into_iter()
            .map(|(plugin_key, def)| {
                let params: serde_json::Value =
                    serde_json::from_str(&def.parameters).unwrap_or_default();
                PluginToolSpec {
                    name: def.name.clone(),
                    description: def.description.clone(),
                    parameters_schema: params,
                    plugin_key,
                }
            })
            .collect()
    }

    /// Execute a tool call from the agent.
    pub fn call_tool(&self, tool_name: &str, args_json: &str) -> Result<String, PluginError> {
        let tools = self.registry.all_tool_definitions();
        let (plugin_key, _) = tools
            .iter()
            .find(|(_, def)| def.name == tool_name)
            .ok_or_else(|| PluginError::ToolNotFound(tool_name.to_string()))?;

        self.registry.call_tool(plugin_key, tool_name, args_json)
    }

    /// Check if a tool name belongs to a plugin.
    pub fn is_plugin_tool(&self, tool_name: &str) -> bool {
        let tools = self.registry.all_tool_definitions();
        tools.iter().any(|(_, def)| def.name == tool_name)
    }
}
