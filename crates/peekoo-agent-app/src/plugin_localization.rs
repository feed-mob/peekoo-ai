use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use peekoo_plugin_host::ConfigFieldDef;
use serde::Deserialize;
use serde_json::Value;

use crate::plugin::{PluginPanelDto, PluginSummaryDto};
use peekoo_plugin_store::StorePluginDto;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PluginLocaleBundle {
    pub plugin: PluginLocaleMeta,
    pub panels: HashMap<String, PanelLocale>,
    pub ui: UiLocale,
    pub config: ConfigLocale,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PluginLocaleMeta {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UiLocale {
    pub panels: HashMap<String, PanelLocale>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PanelLocale {
    pub title: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ConfigLocale {
    pub fields: HashMap<String, ConfigFieldLocale>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ConfigFieldLocale {
    pub label: Option<String>,
    pub description: Option<String>,
    pub options: HashMap<String, String>,
}

impl PluginLocaleBundle {
    fn panel_locale(&self, panel_label: &str) -> Option<&PanelLocale> {
        self.panels
            .get(panel_label)
            .or_else(|| self.ui.panels.get(panel_label))
    }

    fn config_field_locale(&self, field_key: &str) -> Option<&ConfigFieldLocale> {
        self.config.fields.get(field_key)
    }
}

pub fn load_plugin_locale(plugin_dir: &Path, language: &str) -> Option<PluginLocaleBundle> {
    let raw = load_plugin_locale_json(plugin_dir, language)?;
    match serde_json::from_value::<PluginLocaleBundle>(raw) {
        Ok(locale) => Some(locale),
        Err(err) => {
            tracing::warn!(
                plugin_dir = %plugin_dir.display(),
                "Failed to map plugin locale json to structured metadata: {err}"
            );
            None
        }
    }
}

pub fn load_plugin_locale_json(plugin_dir: &Path, language: &str) -> Option<Value> {
    let locales_dir = plugin_dir.join("locales");
    if !locales_dir.is_dir() {
        return None;
    }

    let mut candidates = locale_candidates(language);
    if !language.eq_ignore_ascii_case("en") {
        candidates.extend(locale_candidates("en"));
    }

    for filename in candidates {
        let path = locales_dir.join(filename);
        if !path.is_file() {
            continue;
        }
        match read_locale_file(&path) {
            Ok(locale) => return Some(locale),
            Err(err) => tracing::warn!(
                plugin_dir = %plugin_dir.display(),
                path = %path.display(),
                "Failed to parse plugin locale file: {err}"
            ),
        }
    }

    None
}

pub fn localize_plugin_summary(summary: &mut PluginSummaryDto, locale: &PluginLocaleBundle) {
    if let Some(name) = locale.plugin.name.as_ref() {
        summary.name = name.clone();
    }
    if let Some(description) = locale.plugin.description.as_ref() {
        summary.description = Some(description.clone());
    }
}

pub fn localize_store_plugin(plugin: &mut StorePluginDto, locale: &PluginLocaleBundle) {
    if let Some(name) = locale.plugin.name.as_ref() {
        plugin.name = name.clone();
    }
    if let Some(description) = locale.plugin.description.as_ref() {
        plugin.description = Some(description.clone());
    }
}

pub fn localize_panel_title(panel: &mut PluginPanelDto, locale: &PluginLocaleBundle) {
    if let Some(panel_locale) = locale.panel_locale(&panel.label)
        && let Some(title) = panel_locale.title.as_ref()
    {
        panel.title = title.clone();
    }
}

pub fn localize_config_field(field: &mut ConfigFieldDef, locale: &PluginLocaleBundle) {
    if let Some(field_locale) = locale.config_field_locale(&field.key) {
        if let Some(label) = field_locale.label.as_ref() {
            field.label = label.clone();
        }
        if let Some(description) = field_locale.description.as_ref() {
            field.description = Some(description.clone());
        }
        if let Some(options) = field.options.as_mut() {
            for option in options {
                if let Some(localized_label) = field_locale.options.get(&option.value) {
                    option.label = localized_label.clone();
                }
            }
        }
    }
}

fn read_locale_file(path: &Path) -> Result<Value, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Read locale error: {e}"))?;
    serde_json::from_str::<Value>(&content)
        .map_err(|e| format!("Parse locale json error: {e}"))
}

fn locale_candidates(language: &str) -> Vec<String> {
    let normalized = language.trim();
    if normalized.is_empty() {
        return vec!["en.json".to_string()];
    }

    let mut tags = Vec::<String>::new();
    tags.push(normalized.to_string());
    tags.push(normalized.to_ascii_lowercase());

    if normalized.contains('_') {
        tags.push(normalized.replace('_', "-"));
    }
    if normalized.contains('-') {
        tags.push(normalized.replace('-', "_"));
        if let Some((base, _)) = normalized.split_once('-') {
            tags.push(base.to_string());
        }
    }

    if normalized.eq_ignore_ascii_case("zh-cn") {
        tags.push("zh-CN".to_string());
        tags.push("zh".to_string());
    }

    let mut unique = HashSet::<String>::new();
    tags.into_iter()
        .filter(|tag| unique.insert(tag.to_string()))
        .map(|tag| format!("{tag}.json"))
        .collect()
}

pub fn discover_plugin_dirs_by_key(
    discovered: &[(PathBuf, peekoo_plugin_host::PluginManifest)],
) -> HashMap<String, PathBuf> {
    discovered
        .iter()
        .map(|(dir, manifest)| (manifest.plugin.key.clone(), dir.clone()))
        .collect()
}
