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
        tracing::debug!(
            plugin_dir = %plugin_dir.display(),
            "No locales/ directory found, skipping plugin locale loading"
        );
        return None;
    }

    let mut candidates = locale_candidates(language);
    if !language.eq_ignore_ascii_case("en") {
        candidates.extend(locale_candidates("en"));
    }

    for filename in &candidates {
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

    tracing::debug!(
        plugin_dir = %plugin_dir.display(),
        language,
        ?candidates,
        "No matching locale file found among candidates"
    );
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
    serde_json::from_str::<Value>(&content).map_err(|e| format!("Parse locale json error: {e}"))
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
    if normalized.eq_ignore_ascii_case("zh-tw") {
        tags.push("zh-TW".to_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_plugin_dir_with_locale(locale_filename: &str, content: &str) -> TempDir {
        let tmp = TempDir::new().unwrap();
        let locales_dir = tmp.path().join("locales");
        fs::create_dir_all(&locales_dir).unwrap();
        fs::write(locales_dir.join(locale_filename), content).unwrap();
        tmp
    }

    #[test]
    fn load_locale_with_valid_en_json() {
        let content = r#"{
            "plugin": { "name": "Test Plugin", "description": "A test" },
            "config": { "fields": {} }
        }"#;
        let tmp = make_plugin_dir_with_locale("en.json", content);

        let bundle = load_plugin_locale(tmp.path(), "en").expect("should load locale");
        assert_eq!(bundle.plugin.name.as_deref(), Some("Test Plugin"));
        assert_eq!(bundle.plugin.description.as_deref(), Some("A test"));
    }

    #[test]
    fn fallback_to_en_when_requested_language_missing() {
        let content = r#"{ "plugin": { "name": "English Fallback" } }"#;
        let tmp = make_plugin_dir_with_locale("en.json", content);

        let bundle = load_plugin_locale(tmp.path(), "ja").expect("should fall back to en");
        assert_eq!(bundle.plugin.name.as_deref(), Some("English Fallback"));
    }

    #[test]
    fn returns_none_when_no_locales_dir() {
        let tmp = TempDir::new().unwrap();
        assert!(load_plugin_locale(tmp.path(), "en").is_none());
    }

    #[test]
    fn returns_none_for_malformed_json() {
        let tmp = make_plugin_dir_with_locale("en.json", "{ not valid json }}}");
        assert!(load_plugin_locale(tmp.path(), "en").is_none());
    }

    #[test]
    fn localize_plugin_summary_applies_overrides() {
        let mut summary = PluginSummaryDto {
            plugin_key: "test".into(),
            name: "Original".into(),
            version: "1.0".into(),
            author: None,
            description: Some("Original desc".into()),
            enabled: true,
            tool_count: 0,
            panel_count: 0,
            plugin_dir: "/tmp".into(),
        };

        let locale = PluginLocaleBundle {
            plugin: PluginLocaleMeta {
                name: Some("Localized Name".into()),
                description: Some("Localized Desc".into()),
            },
            ..Default::default()
        };

        localize_plugin_summary(&mut summary, &locale);
        assert_eq!(summary.name, "Localized Name");
        assert_eq!(summary.description.as_deref(), Some("Localized Desc"));
    }

    #[test]
    fn localize_panel_title_applies_override() {
        let mut panel = PluginPanelDto {
            plugin_key: "test".into(),
            label: "panel-main".into(),
            title: "Original Title".into(),
            width: 400,
            height: 300,
            entry: "index.html".into(),
        };

        let mut panels = HashMap::new();
        panels.insert(
            "panel-main".to_string(),
            PanelLocale {
                title: Some("Localized Title".into()),
            },
        );

        let locale = PluginLocaleBundle {
            panels,
            ..Default::default()
        };

        localize_panel_title(&mut panel, &locale);
        assert_eq!(panel.title, "Localized Title");
    }

    #[test]
    fn localize_config_field_applies_label_and_description() {
        let mut field = ConfigFieldDef {
            key: "api_key".into(),
            label: "API Key".into(),
            description: Some("Enter your key".into()),
            field_type: peekoo_plugin_host::ConfigFieldType::String,
            default: serde_json::Value::String(String::new()),
            min: None,
            max: None,
            options: None,
        };

        let mut fields = HashMap::new();
        fields.insert(
            "api_key".to_string(),
            ConfigFieldLocale {
                label: Some("Clé API".into()),
                description: Some("Entrez votre clé".into()),
                options: HashMap::new(),
            },
        );

        let locale = PluginLocaleBundle {
            config: ConfigLocale { fields },
            ..Default::default()
        };

        localize_config_field(&mut field, &locale);
        assert_eq!(field.label, "Clé API");
        assert_eq!(field.description.as_deref(), Some("Entrez votre clé"));
    }

    #[test]
    fn locale_candidates_includes_fallback_variants() {
        let candidates = locale_candidates("zh-CN");
        assert!(candidates.contains(&"zh-CN.json".to_string()));
        assert!(candidates.contains(&"zh.json".to_string()));
    }

    #[test]
    fn preferred_language_takes_priority_over_en_fallback() {
        let tmp = TempDir::new().unwrap();
        let locales_dir = tmp.path().join("locales");
        fs::create_dir_all(&locales_dir).unwrap();
        fs::write(
            locales_dir.join("ja.json"),
            r#"{ "plugin": { "name": "日本語" } }"#,
        )
        .unwrap();
        fs::write(
            locales_dir.join("en.json"),
            r#"{ "plugin": { "name": "English" } }"#,
        )
        .unwrap();

        let bundle = load_plugin_locale(tmp.path(), "ja").expect("should load ja");
        assert_eq!(bundle.plugin.name.as_deref(), Some("日本語"));
    }
}
