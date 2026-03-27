use std::path::PathBuf;

use peekoo_notifications::Notification;
use peekoo_plugin_host::{ConfigFieldDef, PluginManifest, UiPanelDef};
use serde::Serialize;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginNotificationDto {
    pub source_plugin: String,
    pub title: String,
    pub body: String,
    pub action_url: Option<String>,
    pub action_label: Option<String>,
    pub panel_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfigFieldDto {
    pub plugin_key: String,
    #[serde(flatten)]
    pub field: ConfigFieldDef,
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

pub fn plugin_notification_from_message(notification: Notification) -> PluginNotificationDto {
    PluginNotificationDto {
        source_plugin: notification.source,
        title: notification.title,
        body: notification.body,
        action_url: notification.action_url,
        action_label: notification.action_label,
        panel_label: notification.panel_label,
    }
}

#[cfg(test)]
mod tests {
    use peekoo_notifications::Notification;

    use super::plugin_notification_from_message;

    #[test]
    fn converts_notification_message_payload() {
        let notification = plugin_notification_from_message(Notification {
            source: "health-reminders".to_string(),
            title: "Drink water".to_string(),
            body: "Time for a break".to_string(),
            action_url: Some("https://example.com/join".to_string()),
            action_label: Some("Join".to_string()),
            panel_label: None,
        });

        assert_eq!(notification.source_plugin, "health-reminders");
        assert_eq!(notification.title, "Drink water");
        assert_eq!(notification.body, "Time for a break");
        assert_eq!(
            notification.action_url.as_deref(),
            Some("https://example.com/join")
        );
        assert_eq!(notification.action_label.as_deref(), Some("Join"));
    }
}
