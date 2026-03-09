use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension};

use crate::error::PluginError;
use crate::manifest::PluginManifest;

/// Permission enforcement backed by the `plugin_permissions` SQLite table.
#[derive(Clone)]
pub struct PermissionStore {
    conn: Arc<Mutex<Connection>>,
}

impl PermissionStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Check that all required permissions for a plugin are granted.
    pub fn check_required(
        &self,
        plugin_key: &str,
        manifest: &PluginManifest,
    ) -> Result<(), PluginError> {
        let required = manifest
            .permissions
            .as_ref()
            .map(|p| p.required.as_slice())
            .unwrap_or_default();

        for cap in required {
            if !self.is_granted(plugin_key, cap)? {
                return Err(PluginError::PermissionDenied(format!(
                    "Plugin '{plugin_key}' requires permission '{cap}' which is not granted"
                )));
            }
        }
        Ok(())
    }

    /// Check if a specific capability is granted to a plugin.
    pub fn is_granted(&self, plugin_key: &str, capability: &str) -> Result<bool, PluginError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT granted FROM plugin_permissions
                 WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)
                 AND capability = ?2",
            )
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        let granted: Option<bool> = stmt
            .query_row(rusqlite::params![plugin_key, capability], |row| row.get(0))
            .optional()
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        Ok(granted.unwrap_or(false))
    }

    /// Grant a capability to a plugin.
    ///
    /// The delete-then-insert is wrapped in a transaction for atomicity.
    pub fn grant(&self, plugin_key: &str, capability: &str) -> Result<(), PluginError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        let tx = conn
            .transaction()
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        // The schema does not have a UNIQUE constraint on (plugin_id, capability),
        // so we delete-then-insert inside a transaction.
        tx.execute(
            "DELETE FROM plugin_permissions
             WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)
             AND capability = ?2",
            rusqlite::params![plugin_key, capability],
        )
        .map_err(|e| PluginError::Internal(e.to_string()))?;

        tx.execute(
            "INSERT INTO plugin_permissions (id, plugin_id, capability, granted)
             VALUES (?1, (SELECT id FROM plugins WHERE plugin_key = ?2), ?3, 1)",
            rusqlite::params![uuid::Uuid::new_v4().to_string(), plugin_key, capability],
        )
        .map_err(|e| PluginError::Internal(e.to_string()))?;

        tx.commit()
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Revoke a capability from a plugin.
    pub fn revoke(&self, plugin_key: &str, capability: &str) -> Result<(), PluginError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        conn.execute(
            "UPDATE plugin_permissions SET granted = 0
             WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)
             AND capability = ?2",
            rusqlite::params![plugin_key, capability],
        )
        .map_err(|e| PluginError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Grant all required (and optionally optional) permissions for a plugin
    /// in one batch. Used during plugin installation.
    pub fn grant_all_required(
        &self,
        plugin_key: &str,
        manifest: &PluginManifest,
    ) -> Result<(), PluginError> {
        let required = manifest
            .permissions
            .as_ref()
            .map(|p| p.required.as_slice())
            .unwrap_or_default();

        for cap in required {
            self.grant(plugin_key, cap)?;
        }
        Ok(())
    }
}
