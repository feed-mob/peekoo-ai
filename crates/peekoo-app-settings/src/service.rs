use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::dto::SpriteInfo;
use crate::store::AppSettingsStore;

const SETTING_ACTIVE_SPRITE_ID: &str = "active_sprite_id";
const DEFAULT_SPRITE_ID: &str = "dark-cat";

/// Internal static representation of built-in sprites.
///
/// When user-added sprites are supported in the future this list can be
/// extended dynamically by scanning a sprites directory.
struct BuiltinSprite {
    id: &'static str,
    name: &'static str,
    description: &'static str,
}

const BUILTIN_SPRITES: &[BuiltinSprite] = &[
    BuiltinSprite {
        id: "dark-cat",
        name: "Dark Cat",
        description: "Default dark-themed AI pet.",
    },
    BuiltinSprite {
        id: "cute-dog",
        name: "Cute Dog",
        description: "A cute alternative AI pet.",
    },
];

/// Application-level settings service for user preferences.
///
/// Manages global settings such as the active sprite. Backed by a key-value
/// SQLite table (`app_settings`).
pub struct AppSettingsService {
    store: AppSettingsStore,
}

impl AppSettingsService {
    /// Create a service using a shared database connection.
    pub fn with_conn(conn: Arc<Mutex<Connection>>) -> Result<Self, String> {
        let store = AppSettingsStore::with_conn(conn)?;
        Ok(Self { store })
    }

    /// Return the currently selected sprite ID, falling back to the default.
    pub fn get_active_sprite_id(&self) -> Result<String, String> {
        Ok(self
            .store
            .get(SETTING_ACTIVE_SPRITE_ID)?
            .unwrap_or_else(|| DEFAULT_SPRITE_ID.to_string()))
    }

    /// Set the active sprite.
    ///
    /// Returns an error if `sprite_id` does not match any known sprite.
    pub fn set_active_sprite_id(&self, sprite_id: &str) -> Result<(), String> {
        let valid = BUILTIN_SPRITES.iter().any(|s| s.id == sprite_id);
        if !valid {
            return Err(format!("Unknown sprite: {sprite_id}"));
        }
        self.store.set(SETTING_ACTIVE_SPRITE_ID, sprite_id)
    }

    /// List all available sprites.
    pub fn list_sprites(&self) -> Vec<SpriteInfo> {
        BUILTIN_SPRITES
            .iter()
            .map(|s| SpriteInfo {
                id: s.id.to_string(),
                name: s.name.to_string(),
                description: s.description.to_string(),
            })
            .collect()
    }

    /// Return all settings as a key-value map.
    pub fn get_all(&self) -> Result<HashMap<String, String>, String> {
        self.store.get_all()
    }

    /// Set a single setting by key.
    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.store.set(key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> AppSettingsService {
        let conn = Connection::open_in_memory().expect("in-memory db");
        AppSettingsService::with_conn(Arc::new(Mutex::new(conn))).expect("service")
    }

    #[test]
    fn default_sprite_is_dark_cat() {
        let svc = test_service();
        assert_eq!(svc.get_active_sprite_id().unwrap(), "dark-cat");
    }

    #[test]
    fn set_valid_sprite_succeeds() {
        let svc = test_service();
        svc.set_active_sprite_id("cute-dog").unwrap();
        assert_eq!(svc.get_active_sprite_id().unwrap(), "cute-dog");
    }

    #[test]
    fn set_invalid_sprite_returns_error() {
        let svc = test_service();
        let result = svc.set_active_sprite_id("unknown-sprite");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown sprite"));
    }

    #[test]
    fn list_sprites_returns_builtins() {
        let svc = test_service();
        let sprites = svc.list_sprites();
        assert_eq!(sprites.len(), 2);
        assert_eq!(sprites[0].id, "dark-cat");
        assert_eq!(sprites[1].id, "cute-dog");
    }
}
