use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::dto::SpriteInfo;
use crate::store::AppSettingsStore;

const SETTING_ACTIVE_SPRITE_ID: &str = "active_sprite_id";
const SETTING_THEME_MODE: &str = "theme_mode";
const SETTING_APP_LANGUAGE: &str = "app_language";
const SETTING_LOG_LEVEL: &str = "log_level";
const DEFAULT_SPRITE_ID: &str = "dark-cat";
const DEFAULT_THEME_MODE: &str = "system";
const DEFAULT_APP_LANGUAGE: &str = "en";

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
    ///
    /// The caller is responsible for running all migrations before calling this.
    pub fn with_conn(conn: Arc<Mutex<Connection>>) -> Result<Self, String> {
        let store = AppSettingsStore::with_conn(conn);
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

    /// Return the currently selected theme mode, falling back to "system".
    pub fn get_theme_mode(&self) -> Result<String, String> {
        Ok(self
            .store
            .get(SETTING_THEME_MODE)?
            .unwrap_or_else(|| DEFAULT_THEME_MODE.to_string()))
    }

    /// Set the theme mode. Valid values: "light", "dark", "system".
    pub fn set_theme_mode(&self, mode: &str) -> Result<(), String> {
        match mode {
            "light" | "dark" | "system" => self.store.set(SETTING_THEME_MODE, mode),
            _ => Err(format!("Invalid theme mode: {mode}")),
        }
    }

    /// Return the currently selected app language, falling back to "en".
    pub fn get_app_language(&self) -> Result<String, String> {
        Ok(self
            .store
            .get(SETTING_APP_LANGUAGE)?
            .unwrap_or_else(|| DEFAULT_APP_LANGUAGE.to_string()))
    }

    /// Set app language. Valid values: "en", "zh-CN", "zh-TW", "ja", "es", "fr".
    pub fn set_app_language(&self, language: &str) -> Result<(), String> {
        match language {
            "en" | "zh-CN" | "zh-TW" | "ja" | "es" | "fr" => {
                self.store.set(SETTING_APP_LANGUAGE, language)
            }
            _ => Err(format!("Invalid app language: {language}")),
        }
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

    /// Return a setting value by key, if present.
    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        self.store.get(key)
    }

    /// Set a single setting by key.
    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        if key == SETTING_ACTIVE_SPRITE_ID {
            return self.set_active_sprite_id(value);
        }
        if key == SETTING_THEME_MODE {
            return self.set_theme_mode(value);
        }
        if key == SETTING_APP_LANGUAGE {
            return self.set_app_language(value);
        }
        if key == SETTING_LOG_LEVEL {
            return match value {
                "error" | "warn" | "info" | "debug" | "trace" => {
                    self.store.set(SETTING_LOG_LEVEL, value)
                }
                _ => Err(format!("Invalid log level: {value}")),
            };
        }
        self.store.set(key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> AppSettingsService {
        let conn = peekoo_persistence_sqlite::setup_test_db();
        AppSettingsService::with_conn(Arc::new(Mutex::new(conn))).expect("service")
    }

    #[test]
    fn default_sprite_is_dark_cat() {
        let svc = test_service();
        assert_eq!(svc.get_active_sprite_id().unwrap(), "dark-cat");
    }

    #[test]
    fn default_theme_is_system() {
        let svc = test_service();
        assert_eq!(svc.get_theme_mode().unwrap(), "system");
    }

    #[test]
    fn default_language_is_en() {
        let svc = test_service();
        assert_eq!(svc.get_app_language().unwrap(), "en");
    }

    #[test]
    fn set_valid_sprite_succeeds() {
        let svc = test_service();
        svc.set_active_sprite_id("cute-dog").unwrap();
        assert_eq!(svc.get_active_sprite_id().unwrap(), "cute-dog");
    }

    #[test]
    fn set_valid_theme_succeeds() {
        let svc = test_service();
        svc.set_theme_mode("dark").unwrap();
        assert_eq!(svc.get_theme_mode().unwrap(), "dark");
        svc.set_theme_mode("light").unwrap();
        assert_eq!(svc.get_theme_mode().unwrap(), "light");
        svc.set_theme_mode("system").unwrap();
        assert_eq!(svc.get_theme_mode().unwrap(), "system");
    }

    #[test]
    fn set_invalid_sprite_returns_error() {
        let svc = test_service();
        let result = svc.set_active_sprite_id("unknown-sprite");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown sprite"));
    }

    #[test]
    fn set_invalid_theme_returns_error() {
        let svc = test_service();
        let result = svc.set_theme_mode("cobalt");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid theme mode"));
    }

    #[test]
    fn set_valid_language_succeeds() {
        let svc = test_service();
        svc.set_app_language("zh-CN").unwrap();
        assert_eq!(svc.get_app_language().unwrap(), "zh-CN");
        svc.set_app_language("ja").unwrap();
        assert_eq!(svc.get_app_language().unwrap(), "ja");
        svc.set_app_language("es").unwrap();
        assert_eq!(svc.get_app_language().unwrap(), "es");
        svc.set_app_language("fr").unwrap();
        assert_eq!(svc.get_app_language().unwrap(), "fr");
    }

    #[test]
    fn set_invalid_language_returns_error() {
        let svc = test_service();
        let result = svc.set_app_language("zh");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid app language"));
    }

    #[test]
    fn generic_set_validates_active_sprite_id() {
        let svc = test_service();

        let result = svc.set(SETTING_ACTIVE_SPRITE_ID, "unknown-sprite");

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown sprite"));
        assert_eq!(svc.get_active_sprite_id().unwrap(), "dark-cat");
    }

    #[test]
    fn generic_set_validates_theme_mode() {
        let svc = test_service();

        let result = svc.set(SETTING_THEME_MODE, "invalid");

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid theme mode"));
        assert_eq!(svc.get_theme_mode().unwrap(), "system");
    }

    #[test]
    fn generic_set_validates_app_language() {
        let svc = test_service();

        let result = svc.set(SETTING_APP_LANGUAGE, "de");

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid app language"));
        assert_eq!(svc.get_app_language().unwrap(), "en");
    }

    #[test]
    fn generic_set_validates_log_level() {
        let svc = test_service();

        let result = svc.set("log_level", "verbose");

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid log level"));
        assert!(!svc.get_all().unwrap().contains_key("log_level"));
    }

    #[test]
    fn generic_set_accepts_supported_log_level() {
        let svc = test_service();

        svc.set("log_level", "debug").unwrap();

        assert_eq!(
            svc.get_all().unwrap().get("log_level").map(String::as_str),
            Some("debug")
        );
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
