use std::path::Path;
use std::sync::{Arc, Mutex};

use peekoo_persistence_sqlite::{
    MIGRATION_0001_INIT, MIGRATION_0002_AGENT_SETTINGS, MIGRATION_0003_PROVIDER_COMPAT,
    MIGRATION_0005_TASK_EXTENSIONS,
};
use rusqlite::{params, Connection, OptionalExtension};

use crate::settings::catalog::{
    default_model_for_provider, normalize_model_for_provider, DEFAULT_MODEL, DEFAULT_PROVIDER,
};
use crate::settings::dto::{
    AgentSettingsDto, AgentSettingsPatchDto, ProviderAuthDto, ProviderConfigDto, SkillDto,
};

const DEFAULT_MAX_TOOL_ITERATIONS: i64 = 50;
const AUTH_MODE_NONE: &str = "none";
const AUTH_MODE_API_KEY: &str = "api_key";
const AUTH_MODE_OAUTH: &str = "oauth";
const SQL_UPSERT_PROVIDER_AUTH: &str = concat!(
    "INSERT INTO agent_provider_auth (provider_id, auth_mode, api_key_ref, oauth_token_ref, oauth_expires_at, oauth_scopes_json, last_error, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, datetime('now'))",
    " ON CONFLICT(provider_id) DO UPDATE SET auth_mode = excluded.auth_mode, api_key_ref = excluded.api_key_ref, oauth_token_ref = excluded.oauth_token_ref, oauth_expires_at = excluded.oauth_expires_at, updated_at = datetime('now')"
);

use crate::settings::skills::discover_skills;

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
            .prepare("SELECT active_provider_id, active_model_id, system_prompt, max_tool_iterations, version FROM agent_settings WHERE id = 1")
            .map_err(|e| format!("Prepare settings query error: {e}"))?;

        let row = stmt
            .query_row([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
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

        let mut skill_stmt = conn
            .prepare(
                "SELECT skill_id, source_type, path, enabled FROM agent_skills ORDER BY skill_id",
            )
            .map_err(|e| format!("Prepare skills query error: {e}"))?;
        let skill_rows = skill_stmt
            .query_map([], |row| {
                Ok(SkillDto {
                    skill_id: row.get(0)?,
                    source_type: row.get(1)?,
                    path: row.get(2)?,
                    enabled: row.get::<_, i64>(3)? == 1,
                })
            })
            .map_err(|e| format!("Query skill rows error: {e}"))?;
        let skills: Result<Vec<_>, _> = skill_rows.collect();
        let mut skills = skills.map_err(|e| format!("Map skill rows error: {e}"))?;
        if skills.is_empty() {
            skills = discover_skills();
        }

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
            active_provider_id: row.0,
            active_model_id: row.1,
            system_prompt: row.2,
            max_tool_iterations: row.3 as usize,
            version: row.4,
            provider_auth,
            provider_configs,
            skills,
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
            active_provider_id,
            active_model_id,
            system_prompt,
            max_tool_iterations,
            skills,
        } = patch;

        let active_provider_id = active_provider_id
            .map(|provider_id| validate_non_empty_setting("Active provider id", provider_id))
            .transpose()?;
        let active_model_id = active_model_id
            .map(|model_id| validate_non_empty_setting("Active model id", model_id))
            .transpose()?;
        if max_tool_iterations == Some(0) {
            return Err("Max tool iterations must be greater than 0".to_string());
        }

        let current_provider: String = conn
            .query_row(
                "SELECT active_provider_id FROM agent_settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Read current provider error: {e}"))?;

        let effective_provider = active_provider_id.clone().unwrap_or(current_provider);
        let provider_changed = active_provider_id.is_some();

        if let Some(provider) = active_provider_id {
            conn.execute(
                "UPDATE agent_settings SET active_provider_id = ?1, version = version + 1, updated_at = datetime('now') WHERE id = 1",
                params![provider],
            )
            .map_err(|e| format!("Update provider error: {e}"))?;
        }

        if let Some(model) = active_model_id {
            let normalized_model = normalize_model_for_provider(&effective_provider, &model);
            conn.execute(
                "UPDATE agent_settings SET active_model_id = ?1, version = version + 1, updated_at = datetime('now') WHERE id = 1",
                params![normalized_model],
            )
            .map_err(|e| format!("Update model error: {e}"))?;
        } else if provider_changed {
            let fallback_model = default_model_for_provider(&effective_provider).to_string();
            conn.execute(
                "UPDATE agent_settings SET active_model_id = ?1, version = version + 1, updated_at = datetime('now') WHERE id = 1",
                params![fallback_model],
            )
            .map_err(|e| format!("Reset model on provider change error: {e}"))?;
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

        if let Some(skills) = skills {
            conn.execute("DELETE FROM agent_skills", [])
                .map_err(|e| format!("Delete existing skill rows error: {e}"))?;
            for skill in skills {
                conn.execute(
                    "INSERT INTO agent_skills (skill_id, source_type, path, enabled, updated_at) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
                    params![skill.skill_id, skill.source_type, skill.path, if skill.enabled { 1 } else { 0 }],
                )
                .map_err(|e| format!("Insert skill row error: {e}"))?;
            }
            conn.execute(
                "UPDATE agent_settings SET version = version + 1, updated_at = datetime('now') WHERE id = 1",
                [],
            )
            .map_err(|e| format!("Bump settings version error: {e}"))?;
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
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _peekoo_migrations (id TEXT PRIMARY KEY)",
        [],
    )
    .map_err(|e| format!("Create migrations table error: {e}"))?;

    apply_migration_if_needed(conn, "0001_init", "tasks", MIGRATION_0001_INIT)?;
    apply_migration_if_needed(
        conn,
        "0002_agent_settings",
        "agent_settings",
        MIGRATION_0002_AGENT_SETTINGS,
    )?;
    apply_migration_if_needed(
        conn,
        "0003_provider_compat",
        "agent_provider_configs",
        MIGRATION_0003_PROVIDER_COMPAT,
    )?;

    conn.execute(
        &format!(
            "INSERT OR IGNORE INTO agent_settings (id, active_provider_id, active_model_id, system_prompt, max_tool_iterations, version, updated_at) VALUES (1, ?1, ?2, NULL, {}, 1, datetime('now'))",
            DEFAULT_MAX_TOOL_ITERATIONS
        ),
        params![DEFAULT_PROVIDER, DEFAULT_MODEL],
    )
    .map_err(|e| format!("Insert default agent settings error: {e}"))?;

    // ALTER TABLE migration — sentinel table already exists, so check the
    // migration record directly instead of the sentinel table.
    let already_applied: bool = conn
        .query_row(
            "SELECT 1 FROM _peekoo_migrations WHERE id = '0005_task_extensions'",
            [],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("Check migration 0005 state error: {e}"))?
        .unwrap_or(false);

    if !already_applied {
        conn.execute_batch(MIGRATION_0005_TASK_EXTENSIONS)
            .map_err(|e| format!("Apply migration 0005_task_extensions error: {e}"))?;
        conn.execute(
            "INSERT OR IGNORE INTO _peekoo_migrations (id) VALUES ('0005_task_extensions')",
            [],
        )
        .map_err(|e| format!("Record migration 0005 state error: {e}"))?;
    }

    Ok(())
}

fn apply_migration_if_needed(
    conn: &Connection,
    migration_id: &str,
    sentinel_table: &str,
    sql: &str,
) -> Result<(), String> {
    let exists: Option<String> = conn
        .query_row(
            "SELECT id FROM _peekoo_migrations WHERE id = ?1",
            params![migration_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Check migration state error: {e}"))?;

    if exists.is_some() {
        return Ok(());
    }

    let table_exists = sqlite_table_exists(conn, sentinel_table)?;
    if !table_exists {
        conn.execute_batch(sql)
            .map_err(|e| format!("Apply migration {migration_id} error: {e}"))?;
    }

    conn.execute(
        "INSERT OR IGNORE INTO _peekoo_migrations (id) VALUES (?1)",
        params![migration_id],
    )
    .map_err(|e| format!("Record migration state error: {e}"))?;

    Ok(())
}

fn sqlite_table_exists(conn: &Connection, table_name: &str) -> Result<bool, String> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            params![table_name],
            |_| Ok(true),
        )
        .optional()
        .map_err(|e| format!("Query sqlite_master error: {e}"))?
        .unwrap_or(false);
    Ok(exists)
}

fn validate_non_empty_setting(field_name: &str, value: String) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} cannot be empty"));
    }
    Ok(trimmed.to_string())
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

        let second = SettingsStore::from_path(&path);
        assert!(second.is_ok());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn apply_patch_rejects_empty_provider_id() {
        let (store, path) = new_store();

        let result = store.apply_patch(AgentSettingsPatchDto {
            active_provider_id: Some("   ".into()),
            active_model_id: None,
            system_prompt: None,
            max_tool_iterations: None,
            skills: None,
        });

        match result {
            Ok(_) => panic!("empty provider should fail"),
            Err(err) => assert_eq!(err, "Active provider id cannot be empty"),
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn apply_patch_rejects_empty_model_id() {
        let (store, path) = new_store();

        let result = store.apply_patch(AgentSettingsPatchDto {
            active_provider_id: None,
            active_model_id: Some("\n\t ".into()),
            system_prompt: None,
            max_tool_iterations: None,
            skills: None,
        });

        match result {
            Ok(_) => panic!("empty model should fail"),
            Err(err) => assert_eq!(err, "Active model id cannot be empty"),
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn apply_patch_rejects_zero_max_tool_iterations() {
        let (store, path) = new_store();

        let result = store.apply_patch(AgentSettingsPatchDto {
            active_provider_id: None,
            active_model_id: None,
            system_prompt: None,
            max_tool_iterations: Some(0),
            skills: None,
        });

        match result {
            Ok(_) => panic!("zero max tool iterations should fail"),
            Err(err) => assert_eq!(err, "Max tool iterations must be greater than 0"),
        }

        let _ = std::fs::remove_file(path);
    }
}
