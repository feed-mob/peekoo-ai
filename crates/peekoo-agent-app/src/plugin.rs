use std::path::PathBuf;

use serde::Serialize;

use peekoo_plugin_host::{PluginEvent, PluginManifest, UiPanelDef};

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

pub fn plugin_notification_from_event(event: PluginEvent) -> Option<PluginNotificationDto> {
    if event.event != "plugin:notification" {
        return None;
    }

    let title = event.payload.get("title")?.as_str()?.to_string();
    let body = event.payload.get("body")?.as_str()?.to_string();

    Some(PluginNotificationDto {
        source_plugin: event.source_plugin,
        title,
        body,
    })
}

#[cfg(test)]
mod tests {
    use peekoo_plugin_host::PluginEvent;
    use serde_json::json;

    use super::plugin_notification_from_event;

    #[test]
    fn converts_notification_event_payload() {
        let notification = plugin_notification_from_event(PluginEvent {
            source_plugin: "test-notification".to_string(),
            event: "plugin:notification".to_string(),
            payload: json!({
                "title": "Test Notification",
                "body": "Hello from plugin"
            }),
        })
        .expect("notification should parse");

        assert_eq!(notification.source_plugin, "test-notification");
        assert_eq!(notification.title, "Test Notification");
        assert_eq!(notification.body, "Hello from plugin");
    }

    #[test]
    fn ignores_non_notification_events() {
        let notification = plugin_notification_from_event(PluginEvent {
            source_plugin: "health-reminders".to_string(),
            event: "health:reminder-due".to_string(),
            payload: json!({ "reminder_type": "water" }),
        });

        assert!(notification.is_none());
    }
}
