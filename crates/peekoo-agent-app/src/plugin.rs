use std::path::PathBuf;

use serde::Serialize;

use peekoo_plugin_host::{PluginManifest, UiPanelDef};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummaryDto {
    pub plugin_key: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub enabled: bool,
    pub tool_count: usize,
    pub panel_count: usize,
    pub plugin_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPanelDto {
    pub plugin_key: String,
    pub label: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub entry: String,
}

impl PluginPanelDto {
    pub fn from_panel(plugin_key: String, panel: UiPanelDef) -> Self {
        Self {
            plugin_key,
            label: panel.label,
            title: panel.title,
            width: panel.width,
            height: panel.height,
            entry: panel.entry,
        }
    }
}

pub fn manifest_to_summary(
    manifest: &PluginManifest,
    plugin_dir: PathBuf,
    enabled: bool,
) -> PluginSummaryDto {
    PluginSummaryDto {
        plugin_key: manifest.plugin.key.clone(),
        name: manifest.plugin.name.clone(),
        version: manifest.plugin.version.clone(),
        author: manifest.plugin.author.clone(),
        description: manifest.plugin.description.clone(),
        enabled,
        tool_count: manifest
            .tools
            .as_ref()
            .map(|tools| tools.definitions.len())
            .unwrap_or(0),
        panel_count: manifest.ui.as_ref().map(|ui| ui.panels.len()).unwrap_or(0),
        plugin_dir: plugin_dir.display().to_string(),
    }
}
