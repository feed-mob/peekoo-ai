use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, params};

use crate::settings::dto::{
    AgentSettingsDto, AgentSettingsPatchDto, ProviderAuthDto, ProviderConfigDto,
};

const DEFAULT_MAX_TOOL_ITERATIONS: i64 = 50;
const AUTH_MODE_NONE: &str = "none";
const AUTH_MODE_API_KEY: &str = "api_key";
const AUTH_MODE_OAUTH: &str = "oauth";
const SQL_UPSERT_PROVIDER_AUTH: &str = concat!(
    "INSERT INTO agent_provider_auth (provider_id, auth_mode, api_key_ref, oauth_token_ref, oauth_expires_at, oauth_scopes_json, last_error, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, datetime('now'))",
    " ON CONFLICT(provider_id) DO UPDATE SET auth_mode = excluded.auth_mode, api_key_ref = excluded.api_key_ref, oauth_token_ref = excluded.oauth_token_ref, oauth_expires_at = excluded.oauth_expires_at, updated_at = datetime('now')"
);

pub(crate) struct SettingsStore {
    conn: Arc<Mutex<Connection>>,
}

impl SettingsStore {
    /// Create a `SettingsStore` backed by an already-opened shared connection.
    ///
    /// The caller is responsible for opening the connection and setting any
    /// desired PRAGMAs (WAL mode, busy_timeout, etc.) before calling this.
    /// Migrations and default seed rows are applied on the shared connection.
    pub(crate) fn with_conn(conn: Arc<Mutex<Connection>>) -> Result<Self, String> {
        {
            let c = conn
                .lock()
                .map_err(|e| format!("Settings conn lock error: {e}"))?;
            run_migrations_and_seed(&c)?;
        }
        Ok(Self { conn })
    }

    /// Convenience constructor that opens a new connection from a file path.
    ///
    /// Used by tests and the legacy `SettingsService::new()` code-path.
    pub(crate) fn from_path(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Create settings db dir error: {e}"))?;
        }

        let conn = Connection::open(path).map_err(|e| format!("Open settings db error: {e}"))?;
        run_migrations_and_seed(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub(crate) fn load_settings(&self) -> Result<AgentSettingsDto, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;

        let mut stmt = conn
            .prepare("SELECT system_prompt, max_tool_iterations, version FROM agent_settings WHERE id = 1")
            .map_err(|e| format!("Prepare settings query error: {e}"))?;

        let row = stmt
            .query_row([], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| format!("Query settings row error: {e}"))?;

        let mut auth_stmt = conn
            .prepare("SELECT provider_id, auth_mode, oauth_expires_at, api_key_ref, oauth_token_ref FROM agent_provider_auth")
            .map_err(|e| format!("Prepare auth query error: {e}"))?;
        let auth_rows = auth_stmt
            .query_map([], |row| {
                let api_key_ref: Option<String> = row.get(3)?;
                let oauth_token_ref: Option<String> = row.get(4)?;
                Ok(ProviderAuthDto {
                    provider_id: row.get(0)?,
                    auth_mode: row.get(1)?,
                    configured: api_key_ref.is_some() || oauth_token_ref.is_some(),
                    oauth_expires_at: row.get(2)?,
                })
            })
            .map_err(|e| format!("Query auth rows error: {e}"))?;

        let provider_auth: Result<Vec<_>, _> = auth_rows.collect();
        let provider_auth = provider_auth.map_err(|e| format!("Map auth rows error: {e}"))?;

        let mut provider_cfg_stmt = conn
            .prepare("SELECT provider_id, base_url, api, auth_header FROM agent_provider_configs")
            .map_err(|e| format!("Prepare provider config query error: {e}"))?;
        let provider_cfg_rows = provider_cfg_stmt
            .query_map([], |row| {
                Ok(ProviderConfigDto {
                    provider_id: row.get(0)?,
                    base_url: row.get(1)?,
                    api: row.get(2)?,
                    auth_header: row.get::<_, i64>(3)? == 1,
                })
            })
            .map_err(|e| format!("Query provider config rows error: {e}"))?;
        let provider_configs: Result<Vec<_>, _> = provider_cfg_rows.collect();
        let provider_configs =
            provider_configs.map_err(|e| format!("Map provider config rows error: {e}"))?;

        Ok(AgentSettingsDto {
            system_prompt: row.0,
            max_tool_iterations: row.1 as usize,
            version: row.2,
            provider_auth,
            provider_configs,
        })
    }

    pub(crate) fn apply_patch(
        &self,
        patch: AgentSettingsPatchDto,
    ) -> Result<AgentSettingsDto, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;

        let AgentSettingsPatchDto {
            system_prompt,
            max_tool_iterations,
        } = patch;

        if max_tool_iterations == Some(0) {
            return Err("Max tool iterations must be greater than 0".to_string());
        }

        if let Some(system_prompt) = system_prompt {
            conn.execute(
                "UPDATE agent_settings SET system_prompt = ?1, version = version + 1, updated_at = datetime('now') WHERE id = 1",
                params![system_prompt],
            )
            .map_err(|e| format!("Update system prompt error: {e}"))?;
        }

        if let Some(max_tool_iterations) = max_tool_iterations {
            conn.execute(
                "UPDATE agent_settings SET max_tool_iterations = ?1, version = version + 1, updated_at = datetime('now') WHERE id = 1",
                params![max_tool_iterations as i64],
            )
            .map_err(|e| format!("Update max tool iterations error: {e}"))?;
        }

        drop(conn);
        self.load_settings()
    }

    pub(crate) fn set_provider_auth_refs(
        &self,
        provider_id: &str,
        auth_mode: &str,
        api_key_ref: Option<String>,
        oauth_token_ref: Option<String>,
        oauth_expires_at: Option<String>,
    ) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;
        conn.execute(
            SQL_UPSERT_PROVIDER_AUTH,
            params![
                provider_id,
                auth_mode,
                api_key_ref,
                oauth_token_ref,
                oauth_expires_at
            ],
        )
        .map_err(|e| format!("Upsert provider auth row error: {e}"))?;

        conn.execute(
            "UPDATE agent_settings SET version = version + 1, updated_at = datetime('now') WHERE id = 1",
            [],
        )
        .map_err(|e| format!("Bump settings version error: {e}"))?;

        Ok(())
    }

    pub(crate) fn clear_provider_auth_refs(
        &self,
        provider_id: &str,
    ) -> Result<(Option<String>, Option<String>), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;
        let refs = conn
            .query_row(
                "SELECT api_key_ref, oauth_token_ref FROM agent_provider_auth WHERE provider_id = ?1",
                params![provider_id],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(|e| format!("Read provider auth refs error: {e}"))?
            .unwrap_or((None, None));

        conn.execute(
            "INSERT INTO agent_provider_auth (provider_id, auth_mode, api_key_ref, oauth_token_ref, oauth_expires_at, oauth_scopes_json, last_error, updated_at) VALUES (?1, ?2, NULL, NULL, NULL, NULL, NULL, datetime('now'))
             ON CONFLICT(provider_id) DO UPDATE SET auth_mode = excluded.auth_mode, api_key_ref = NULL, oauth_token_ref = NULL, oauth_expires_at = NULL, updated_at = datetime('now')",
            params![provider_id, AUTH_MODE_NONE],
        )
        .map_err(|e| format!("Clear provider auth row error: {e}"))?;

        conn.execute(
            "UPDATE agent_settings SET version = version + 1, updated_at = datetime('now') WHERE id = 1",
            [],
        )
        .map_err(|e| format!("Bump settings version error: {e}"))?;

        Ok(refs)
    }

    pub(crate) fn provider_auth_for(&self, provider_id: &str) -> Result<ProviderAuthDto, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;
        conn.query_row(
            "SELECT auth_mode, api_key_ref, oauth_token_ref, oauth_expires_at FROM agent_provider_auth WHERE provider_id = ?1",
            params![provider_id],
            |row| {
                let api_key_ref: Option<String> = row.get(1)?;
                let oauth_token_ref: Option<String> = row.get(2)?;
                Ok(ProviderAuthDto {
                    provider_id: provider_id.to_string(),
                    auth_mode: row.get(0)?,
                    configured: api_key_ref.is_some() || oauth_token_ref.is_some(),
                    oauth_expires_at: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("Read provider auth error: {e}"))?
        .ok_or_else(|| format!("Provider auth not found for {provider_id}"))
    }

    pub(crate) fn active_api_key_ref(&self, provider_id: &str) -> Result<Option<String>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;
        conn.query_row(
            "SELECT api_key_ref FROM agent_provider_auth WHERE provider_id = ?1 AND auth_mode = ?2",
            params![provider_id, AUTH_MODE_API_KEY],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| format!("Read provider api key ref error: {e}"))
        .map(|v| v.flatten())
    }

    pub(crate) fn active_oauth_token_ref(
        &self,
        provider_id: &str,
    ) -> Result<Option<String>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;
        conn.query_row(
            "SELECT oauth_token_ref FROM agent_provider_auth WHERE provider_id = ?1 AND auth_mode = ?2",
            params![provider_id, AUTH_MODE_OAUTH],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| format!("Read provider oauth token ref error: {e}"))
        .map(|v| v.flatten())
    }

    pub(crate) fn bump_version(&self) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;
        conn.execute(
            "UPDATE agent_settings SET version = version + 1, updated_at = datetime('now') WHERE id = 1",
            [],
        )
        .map_err(|e| format!("Bump settings version error: {e}"))?;
        Ok(())
    }

    pub(crate) fn set_provider_config(
        &self,
        cfg: ProviderConfigDto,
    ) -> Result<ProviderConfigDto, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;

        conn.execute(
            "INSERT INTO agent_provider_configs (provider_id, base_url, api, auth_header, updated_at) VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(provider_id) DO UPDATE SET base_url = excluded.base_url, api = excluded.api, auth_header = excluded.auth_header, updated_at = datetime('now')",
            params![
                cfg.provider_id,
                cfg.base_url,
                cfg.api,
                if cfg.auth_header { 1 } else { 0 }
            ],
        )
        .map_err(|e| format!("Upsert provider config error: {e}"))?;

        conn.execute(
            "UPDATE agent_settings SET version = version + 1, updated_at = datetime('now') WHERE id = 1",
            [],
        )
        .map_err(|e| format!("Bump settings version error: {e}"))?;

        drop(conn);
        self.provider_config_for(&cfg.provider_id)
            .ok_or_else(|| format!("Provider config not found for {}", cfg.provider_id))
    }

    pub(crate) fn provider_config_for(&self, provider_id: &str) -> Option<ProviderConfigDto> {
        let conn = self.conn.lock().ok()?;
        conn.query_row(
            "SELECT provider_id, base_url, api, auth_header FROM agent_provider_configs WHERE provider_id = ?1",
            params![provider_id],
            |row| {
                Ok(ProviderConfigDto {
                    provider_id: row.get(0)?,
                    base_url: row.get(1)?,
                    api: row.get(2)?,
                    auth_header: row.get::<_, i64>(3)? == 1,
                })
            },
        )
        .optional()
        .ok()
        .flatten()
    }
}

fn run_migrations_and_seed(conn: &Connection) -> Result<(), String> {
    peekoo_persistence_sqlite::run_all_migrations(conn)?;

    // Seed default agent settings (must run after schema migrations)
    conn.execute(
        &format!(
            "INSERT OR IGNORE INTO agent_settings (id, system_prompt, max_tool_iterations, version, updated_at) VALUES (1, NULL, {}, 1, datetime('now'))",
            DEFAULT_MAX_TOOL_ITERATIONS
        ),
        [],
    )
    .map_err(|e| format!("Insert default agent settings error: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_db_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("peekoo-settings-{}.sqlite", Uuid::new_v4()))
    }

    fn new_store() -> (SettingsStore, std::path::PathBuf) {
        let path = temp_db_path();
        let store = SettingsStore::from_path(&path).expect("create settings store");
        (store, path)
    }

    #[test]
    fn from_path_is_idempotent_with_existing_db() {
        let path = temp_db_path();
        let first = SettingsStore::from_path(&path);
        assert!(first.is_ok());

        // Drop the first connection before creating the second to avoid lock contention
        drop(first);

        let second = SettingsStore::from_path(&path);
        assert!(second.is_ok());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn apply_patch_rejects_zero_max_tool_iterations() {
        let (store, path) = new_store();

        let result = store.apply_patch(AgentSettingsPatchDto {
            system_prompt: None,
            max_tool_iterations: Some(0),
        });

        match result {
            Ok(_) => panic!("zero max tool iterations should fail"),
            Err(err) => assert_eq!(err, "Max tool iterations must be greater than 0"),
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn bump_version_increments_version() {
        let (store, path) = new_store();

        let before = store.load_settings().expect("load settings").version;
        store.bump_version().expect("bump version");
        let after = store.load_settings().expect("load settings").version;

        assert_eq!(after, before + 1);

        let _ = std::fs::remove_file(path);
    }

    /// Chat settings DTOs should not surface skill state; discovery is catalog-driven.
    #[test]
    fn load_settings_chat_dto_omits_skills() {
        let (store, path) = new_store();
        let settings = store.load_settings().expect("load settings");
        let json = serde_json::to_value(&settings).expect("serialize settings");
        assert!(
            json.get("skills").is_none(),
            "AgentSettingsDto must not expose skills for chat settings; got {json:?}"
        );

        let _ = std::fs::remove_file(path);
    }
}
