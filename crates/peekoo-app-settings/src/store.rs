use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use peekoo_persistence_sqlite::MIGRATIONS;
use rusqlite::{Connection, OptionalExtension, params};

/// Key-value store backed by the `app_settings` SQLite table.
pub(crate) struct AppSettingsStore {
    conn: Arc<Mutex<Connection>>,
}

impl AppSettingsStore {
    /// Create a store using a shared database connection.
    ///
    /// The caller is responsible for opening the connection and setting
    /// pragmas. This constructor runs the migration that creates the
    /// `app_settings` table if it does not already exist.
    pub(crate) fn with_conn(conn: Arc<Mutex<Connection>>) -> Result<Self, String> {
        {
            let c = conn
                .lock()
                .map_err(|e| format!("App settings conn lock error: {e}"))?;
            let migration = MIGRATIONS
                .iter()
                .find(|m| m.id == "0004_global_settings")
                .ok_or("Migration 0004_global_settings not found")?;
            c.execute_batch(migration.sql)
                .map_err(|e| format!("App settings migration error: {e}"))?;
        }
        Ok(Self { conn })
    }

    /// Read a single setting by key.
    pub(crate) fn get(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("App settings lock error: {e}"))?;
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("App settings get error: {e}"))
    }

    /// Upsert a key-value pair.
    pub(crate) fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("App settings lock error: {e}"))?;
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, datetime('now')) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value],
        )
        .map_err(|e| format!("App settings set error: {e}"))?;
        Ok(())
    }

    /// Return all settings as a key-value map.
    pub(crate) fn get_all(&self) -> Result<HashMap<String, String>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("App settings lock error: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT key, value FROM app_settings")
            .map_err(|e| format!("App settings prepare error: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("App settings query error: {e}"))?;
        let mut map = HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(|e| format!("App settings row error: {e}"))?;
            map.insert(k, v);
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory_store() -> AppSettingsStore {
        let conn = Connection::open_in_memory().expect("in-memory db");
        AppSettingsStore::with_conn(Arc::new(Mutex::new(conn))).expect("store")
    }

    #[test]
    fn get_missing_key_returns_none() {
        let store = in_memory_store();
        assert_eq!(store.get("nonexistent").unwrap(), None);
    }

    #[test]
    fn set_and_get_round_trips() {
        let store = in_memory_store();
        store.set("active_sprite_id", "cute-dog").unwrap();
        assert_eq!(
            store.get("active_sprite_id").unwrap(),
            Some("cute-dog".to_string())
        );
    }

    #[test]
    fn set_overwrites_existing_value() {
        let store = in_memory_store();
        store.set("theme", "light").unwrap();
        store.set("theme", "dark").unwrap();
        assert_eq!(store.get("theme").unwrap(), Some("dark".to_string()));
    }

    #[test]
    fn get_all_returns_all_entries() {
        let store = in_memory_store();
        store.set("a", "1").unwrap();
        store.set("b", "2").unwrap();
        let all = store.get_all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all.get("a").unwrap(), "1");
        assert_eq!(all.get("b").unwrap(), "2");
    }
}
