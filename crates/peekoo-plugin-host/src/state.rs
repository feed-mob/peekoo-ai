use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension};

use crate::error::PluginError;

/// Key-value persistence for plugin state, backed by the `plugin_state` SQLite table.
#[derive(Clone)]
pub struct PluginStateStore {
    conn: Arc<Mutex<Connection>>,
}

impl PluginStateStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Get a value from the plugin's KV store.
    ///
    /// Returns `Value::Null` when the key does not exist.
    pub fn get(&self, plugin_key: &str, key: &str) -> Result<serde_json::Value, PluginError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT value_json FROM plugin_state
                 WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)
                 AND state_key = ?2",
            )
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        let value: Option<String> = stmt
            .query_row(rusqlite::params![plugin_key, key], |row| row.get(0))
            .optional()
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        match value {
            Some(json_str) => {
                serde_json::from_str(&json_str).map_err(|e| PluginError::Internal(e.to_string()))
            }
            None => Ok(serde_json::Value::Null),
        }
    }

    /// Set a value in the plugin's KV store.
    ///
    /// The delete-then-insert is wrapped in a transaction so the key is never
    /// momentarily absent if the insert fails.
    pub fn set(
        &self,
        plugin_key: &str,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), PluginError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        let value_json =
            serde_json::to_string(value).map_err(|e| PluginError::Internal(e.to_string()))?;

        let tx = conn
            .transaction()
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        tx.execute(
            "DELETE FROM plugin_state
             WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)
             AND state_key = ?2",
            rusqlite::params![plugin_key, key],
        )
        .map_err(|e| PluginError::Internal(e.to_string()))?;

        tx.execute(
            "INSERT INTO plugin_state (id, plugin_id, state_key, value_json, updated_at)
             VALUES (
               ?1,
               (SELECT id FROM plugins WHERE plugin_key = ?2),
               ?3,
               ?4,
               datetime('now')
             )",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                plugin_key,
                key,
                value_json
            ],
        )
        .map_err(|e| PluginError::Internal(e.to_string()))?;

        tx.commit()
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Delete a key from the plugin's KV store.
    pub fn delete(&self, plugin_key: &str, key: &str) -> Result<(), PluginError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM plugin_state
             WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)
             AND state_key = ?2",
            rusqlite::params![plugin_key, key],
        )
        .map_err(|e| PluginError::Internal(e.to_string()))?;
        Ok(())
    }

    /// List all keys for a plugin.
    pub fn list_keys(&self, plugin_key: &str) -> Result<Vec<String>, PluginError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT state_key FROM plugin_state
                 WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)",
            )
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        let keys = stmt
            .query_map(rusqlite::params![plugin_key], |row| row.get::<_, String>(0))
            .map_err(|e| PluginError::Internal(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(keys)
    }
}
