use serde_json::{Map, Value};

use crate::error::PluginError;
use crate::manifest::{ConfigFieldDef, ConfigFieldType};
use crate::state::PluginStateStore;

pub fn resolved_config_map(
    state_store: &PluginStateStore,
    plugin_key: &str,
    fields: &[ConfigFieldDef],
) -> Result<Map<String, Value>, PluginError> {
    let mut values = Map::new();

    for field in fields {
        let stored = state_store.get(plugin_key, &field.key)?;
        let value = if stored.is_null() {
            field.default.clone()
        } else {
            stored
        };
        values.insert(field.key.clone(), value);
    }

    Ok(values)
}

pub fn set_config_field(
    state_store: &PluginStateStore,
    plugin_key: &str,
    fields: &[ConfigFieldDef],
    key: &str,
    value: Value,
) -> Result<(), PluginError> {
    let field = fields
        .iter()
        .find(|field| field.key == key)
        .ok_or_else(|| PluginError::NotFound(format!("Config field not found: {key}")))?;

    validate_value(field, &value)?;
    state_store.set(plugin_key, key, &value)
}

fn validate_value(field: &ConfigFieldDef, value: &Value) -> Result<(), PluginError> {
    match field.field_type {
        ConfigFieldType::Integer => {
            let Some(number) = value.as_i64() else {
                return Err(PluginError::Internal(format!(
                    "Config field '{}' requires an integer",
                    field.key
                )));
            };

            if let Some(min) = field.min
                && (number as f64) < min
            {
                return Err(PluginError::Internal(format!(
                    "Config field '{}' must be >= {}",
                    field.key, min
                )));
            }
            if let Some(max) = field.max
                && (number as f64) > max
            {
                return Err(PluginError::Internal(format!(
                    "Config field '{}' must be <= {}",
                    field.key, max
                )));
            }
        }
        ConfigFieldType::Boolean => {
            if !value.is_boolean() {
                return Err(PluginError::Internal(format!(
                    "Config field '{}' requires a boolean",
                    field.key
                )));
            }
        }
        ConfigFieldType::String => {
            if !value.is_string() {
                return Err(PluginError::Internal(format!(
                    "Config field '{}' requires a string",
                    field.key
                )));
            }
        }
        ConfigFieldType::Select => {
            let Some(selected) = value.as_str() else {
                return Err(PluginError::Internal(format!(
                    "Config field '{}' requires a string option",
                    field.key
                )));
            };
            let valid = field
                .options
                .as_ref()
                .is_some_and(|options| options.iter().any(|option| option.value == selected));
            if !valid {
                return Err(PluginError::Internal(format!(
                    "Config field '{}' received an unsupported option",
                    field.key
                )));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use rusqlite::Connection;
    use serde_json::json;

    use super::{resolved_config_map, set_config_field};
    use crate::manifest::{ConfigFieldDef, ConfigFieldType};
    use crate::state::PluginStateStore;

    fn state_store() -> PluginStateStore {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE plugins (
                id TEXT PRIMARY KEY,
                plugin_key TEXT NOT NULL,
                version TEXT NOT NULL,
                plugin_type TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                manifest_json TEXT NOT NULL,
                installed_at TEXT NOT NULL
            );
            CREATE TABLE plugin_state (
                id TEXT PRIMARY KEY,
                plugin_id TEXT NOT NULL,
                state_key TEXT NOT NULL,
                value_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            INSERT INTO plugins (id, plugin_key, version, plugin_type, enabled, manifest_json, installed_at)
            VALUES ('plugin-1', 'health-reminders', '0.1.0', 'wasm', 1, '{}', datetime('now'));
            ",
        )
        .unwrap();
        PluginStateStore::new(Arc::new(Mutex::new(conn)))
    }

    fn fields() -> Vec<ConfigFieldDef> {
        vec![
            ConfigFieldDef {
                key: "water_interval_min".to_string(),
                label: "Water".to_string(),
                description: None,
                field_type: ConfigFieldType::Integer,
                default: json!(45),
                min: Some(5.0),
                max: Some(180.0),
                options: None,
            },
            ConfigFieldDef {
                key: "suppress_during_pomodoro".to_string(),
                label: "Suppress".to_string(),
                description: None,
                field_type: ConfigFieldType::Boolean,
                default: json!(true),
                min: None,
                max: None,
                options: None,
            },
        ]
    }

    #[test]
    fn resolves_defaults_for_missing_values() {
        let store = state_store();
        let config = resolved_config_map(&store, "health-reminders", &fields()).unwrap();

        assert_eq!(config.get("water_interval_min"), Some(&json!(45)));
        assert_eq!(config.get("suppress_during_pomodoro"), Some(&json!(true)));
    }

    #[test]
    fn validates_and_persists_config_values() {
        let store = state_store();
        let fields = fields();

        set_config_field(
            &store,
            "health-reminders",
            &fields,
            "water_interval_min",
            json!(60),
        )
        .unwrap();

        let config = resolved_config_map(&store, "health-reminders", &fields).unwrap();
        assert_eq!(config.get("water_interval_min"), Some(&json!(60)));
    }
}
