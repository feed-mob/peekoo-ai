use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::PluginError;

/// Top-level plugin manifest parsed from `peekoo-plugin.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMeta,
    pub permissions: Option<PermissionsBlock>,
    pub tools: Option<ToolsBlock>,
    pub events: Option<EventsBlock>,
    pub data: Option<DataBlock>,
    pub ui: Option<UiBlock>,
    pub config: Option<ConfigBlock>,
}

/// Core plugin metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    /// Unique identifier (kebab-case).
    pub key: String,
    /// Human-readable display name.
    pub name: String,
    /// SemVer version string.
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub min_peekoo_version: Option<String>,
    /// Path to the `.wasm` module relative to the manifest file.
    pub wasm: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PermissionsBlock {
    #[serde(default)]
    pub required: Vec<String>,
    #[serde(default)]
    pub optional: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsBlock {
    pub definitions: Vec<ToolDefinition>,
}

/// A single tool that the plugin exposes to the AI agent.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    /// JSON Schema string describing accepted parameters.
    pub parameters: String,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventsBlock {
    /// Events this plugin wants to receive.
    #[serde(default)]
    pub subscribe: Vec<String>,
    /// Events this plugin may emit.
    #[serde(default)]
    pub emit: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataBlock {
    pub providers: Vec<DataProviderDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DataProviderDef {
    pub name: String,
    pub description: String,
    /// JSON Schema string.
    pub schema: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UiBlock {
    pub panels: Vec<UiPanelDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigBlock {
    pub fields: Vec<ConfigFieldDef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFieldType {
    Integer,
    Boolean,
    String,
    Select,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigOptionDef {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigFieldDef {
    pub key: String,
    pub label: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub field_type: ConfigFieldType,
    pub default: Value,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub options: Option<Vec<ConfigOptionDef>>,
}

/// Declaration of a UI panel provided by the plugin.
#[derive(Debug, Clone, Deserialize)]
pub struct UiPanelDef {
    /// Window label (e.g. `"panel-health"`).
    pub label: String,
    /// Display title.
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// Relative path to the HTML entry point within the plugin directory.
    pub entry: String,
}

/// Parse a `peekoo-plugin.toml` file from disk.
pub fn load_manifest(path: &Path) -> Result<PluginManifest, PluginError> {
    let content = std::fs::read_to_string(path).map_err(|e| PluginError::Io(e.to_string()))?;
    parse_manifest(&content)
}

/// Parse a manifest from a TOML string.
pub fn parse_manifest(toml_str: &str) -> Result<PluginManifest, PluginError> {
    toml::from_str(toml_str).map_err(|e| PluginError::ManifestParse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
[plugin]
key = "example"
name = "Example Plugin"
version = "0.1.0"
wasm = "plugin.wasm"
"#;
        let m = parse_manifest(toml).unwrap();
        assert_eq!(m.plugin.key, "example");
        assert_eq!(m.plugin.name, "Example Plugin");
        assert_eq!(m.plugin.version, "0.1.0");
        assert_eq!(m.plugin.wasm, "plugin.wasm");
        assert!(m.permissions.is_none());
        assert!(m.tools.is_none());
        assert!(m.events.is_none());
        assert!(m.data.is_none());
        assert!(m.ui.is_none());
    }

    #[test]
    fn parse_full_manifest() {
        let toml = r#"
[plugin]
key = "health-reminders"
name = "Health Reminders"
version = "0.1.0"
author = "Peekoo Team"
description = "Periodic health reminders"
min_peekoo_version = "0.1.0"
wasm = "plugin.wasm"

[permissions]
required = ["timer", "notifications", "state:read", "state:write"]
optional = ["agent:register-tool"]

[[tools.definitions]]
name = "health_get_status"
description = "Get current health reminder status"
parameters = '{"type": "object", "properties": {}, "required": []}'
return_type = "object"

[[tools.definitions]]
name = "health_configure"
description = "Configure reminder intervals"
parameters = '{"type": "object", "properties": {"water_interval_min": {"type": "integer"}}}'

[events]
subscribe = ["timer:tick"]
emit = ["health:reminder-due"]

[[data.providers]]
name = "health_reminder_status"
description = "Current state of reminders"
schema = '{"type": "object"}'

[[ui.panels]]
label = "panel-health"
title = "Health Reminders"
width = 320
height = 400
entry = "ui/panel.html"

[[config.fields]]
key = "water_interval_min"
label = "Water reminder interval"
type = "integer"
default = 45
min = 5
max = 180

"#;
        let m = parse_manifest(toml).unwrap();
        assert_eq!(m.plugin.key, "health-reminders");
        assert_eq!(m.plugin.author.as_deref(), Some("Peekoo Team"));

        let perms = m.permissions.as_ref().unwrap();
        assert_eq!(perms.required.len(), 4);
        assert_eq!(perms.optional.len(), 1);

        let tools = m.tools.as_ref().unwrap();
        assert_eq!(tools.definitions.len(), 2);
        assert_eq!(tools.definitions[0].name, "health_get_status");
        assert!(tools.definitions[1].return_type.is_none());

        let events = m.events.as_ref().unwrap();
        assert_eq!(events.subscribe.len(), 1);
        assert_eq!(events.emit.len(), 1);

        let data = m.data.as_ref().unwrap();
        assert_eq!(data.providers.len(), 1);

        let ui = m.ui.as_ref().unwrap();
        assert_eq!(ui.panels.len(), 1);
        assert_eq!(ui.panels[0].width, 320);

        let config = m.config.as_ref().unwrap();
        assert_eq!(config.fields.len(), 1);
        assert_eq!(config.fields[0].key, "water_interval_min");
        assert_eq!(config.fields[0].field_type, ConfigFieldType::Integer);
    }

    #[test]
    fn parse_invalid_manifest_returns_error() {
        let result = parse_manifest("not valid toml {{{}}}");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), PluginError::ManifestParse(_)));
    }

    #[test]
    fn parse_missing_required_field() {
        let toml = r#"
[plugin]
key = "test"
name = "Test"
"#;
        let result = parse_manifest(toml);
        assert!(result.is_err());
    }
}
