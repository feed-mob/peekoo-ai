use std::path::{Path, PathBuf};
use std::sync::Mutex;

use peekoo_agent::config::AgentServiceConfig;
use peekoo_agent_auth::{OAuthFlowStatus, OAuthService};
use peekoo_persistence_sqlite::{
    MIGRATION_0001_INIT, MIGRATION_0002_AGENT_SETTINGS, MIGRATION_0003_PROVIDER_COMPAT,
};
use peekoo_security::{KeyringSecretStore, SecretStore, SecretStoreError};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_PROVIDER: &str = "anthropic";
const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
const OPENAI_COMPAT_PROVIDER_ID: &str = "openai-compatible";
const ANTHROPIC_COMPAT_PROVIDER_ID: &str = "anthropic-compatible";

fn models_for_provider(provider_id: &str) -> &'static [&'static str] {
    match provider_id {
        "anthropic" => &["claude-sonnet-4-6", "claude-opus-4-5"],
        "openai" => &["gpt-4o", "gpt-4.1"],
        "openai-codex" => &["gpt-5.3-codex"],
        _ => &[DEFAULT_MODEL],
    }
}

fn default_model_for_provider(provider_id: &str) -> &'static str {
    if provider_id == OPENAI_COMPAT_PROVIDER_ID {
        return "gpt-4o-mini";
    }
    if provider_id == ANTHROPIC_COMPAT_PROVIDER_ID {
        return "claude-3-5-haiku-latest";
    }

    models_for_provider(provider_id)
        .first()
        .copied()
        .unwrap_or(DEFAULT_MODEL)
}

fn normalize_model_for_provider(provider_id: &str, model_id: &str) -> String {
    if models_for_provider(provider_id)
        .iter()
        .any(|candidate| *candidate == model_id)
    {
        return model_id.to_string();
    }
    default_model_for_provider(provider_id).to_string()
}

fn provider_catalog() -> Vec<ProviderCatalogDto> {
    vec![
        ProviderCatalogDto {
            id: "anthropic".into(),
            name: "Anthropic".into(),
            auth_modes: vec!["api_key".into()],
            models: models_for_provider("anthropic")
                .iter()
                .map(|model| (*model).to_string())
                .collect(),
        },
        ProviderCatalogDto {
            id: "openai".into(),
            name: "OpenAI".into(),
            auth_modes: vec!["api_key".into()],
            models: models_for_provider("openai")
                .iter()
                .map(|model| (*model).to_string())
                .collect(),
        },
        ProviderCatalogDto {
            id: "openai-codex".into(),
            name: "OpenAI Codex".into(),
            auth_modes: vec!["oauth".into()],
            models: models_for_provider("openai-codex")
                .iter()
                .map(|model| (*model).to_string())
                .collect(),
        },
        ProviderCatalogDto {
            id: OPENAI_COMPAT_PROVIDER_ID.into(),
            name: "OpenAI-Compatible".into(),
            auth_modes: vec!["api_key".into()],
            models: Vec::new(),
        },
        ProviderCatalogDto {
            id: ANTHROPIC_COMPAT_PROVIDER_ID.into(),
            name: "Anthropic-Compatible".into(),
            auth_modes: vec!["api_key".into()],
            models: Vec::new(),
        },
    ]
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAuthDto {
    pub provider_id: String,
    pub auth_mode: String,
    pub configured: bool,
    pub oauth_expires_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SkillDto {
    pub skill_id: String,
    pub source_type: String,
    pub path: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentSettingsDto {
    pub active_provider_id: String,
    pub active_model_id: String,
    pub system_prompt: Option<String>,
    pub max_tool_iterations: usize,
    pub version: i64,
    pub provider_auth: Vec<ProviderAuthDto>,
    pub provider_configs: Vec<ProviderConfigDto>,
    pub skills: Vec<SkillDto>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfigDto {
    pub provider_id: String,
    pub base_url: String,
    pub api: String,
    pub auth_header: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSettingsPatchDto {
    pub active_provider_id: Option<String>,
    pub active_model_id: Option<String>,
    pub system_prompt: Option<String>,
    pub max_tool_iterations: Option<usize>,
    pub skills: Option<Vec<SkillDto>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogDto {
    pub id: String,
    pub name: String,
    pub auth_modes: Vec<String>,
    pub models: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSettingsCatalogDto {
    pub providers: Vec<ProviderCatalogDto>,
    pub discovered_skills: Vec<SkillDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetApiKeyRequest {
    pub provider_id: String,
    pub api_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetProviderConfigRequest {
    pub provider_id: String,
    pub base_url: String,
    pub api: Option<String>,
    pub auth_header: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRequest {
    pub provider_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthStatusRequest {
    pub flow_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthStartResponse {
    pub flow_id: String,
    pub authorize_url: String,
    pub opened_browser: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthStatusResponse {
    pub status: String,
    pub provider_auth: Option<ProviderAuthDto>,
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OauthCancelResponse {
    pub cancelled: bool,
}

pub struct SettingsStore {
    conn: Mutex<Connection>,
}

pub struct SettingsService {
    store: SettingsStore,
    secret_store: Box<dyn SecretStore>,
    oauth: OAuthService,
}

impl SettingsStore {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Create settings db dir error: {e}"))?;
        }

        let conn = Connection::open(path).map_err(|e| format!("Open settings db error: {e}"))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS _peekoo_migrations (id TEXT PRIMARY KEY)",
            [],
        )
        .map_err(|e| format!("Create migrations table error: {e}"))?;

        apply_migration_if_needed(&conn, "0001_init", "tasks", MIGRATION_0001_INIT)?;
        apply_migration_if_needed(
            &conn,
            "0002_agent_settings",
            "agent_settings",
            MIGRATION_0002_AGENT_SETTINGS,
        )?;
        apply_migration_if_needed(
            &conn,
            "0003_provider_compat",
            "agent_provider_configs",
            MIGRATION_0003_PROVIDER_COMPAT,
        )?;

        conn.execute(
            "INSERT OR IGNORE INTO agent_settings (id, active_provider_id, active_model_id, system_prompt, max_tool_iterations, version, updated_at) VALUES (1, ?1, ?2, NULL, 50, 1, datetime('now'))",
            params![DEFAULT_PROVIDER, DEFAULT_MODEL],
        )
        .map_err(|e| format!("Insert default agent settings error: {e}"))?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn load_settings(&self) -> Result<AgentSettingsDto, String> {
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

    fn apply_patch(&self, patch: AgentSettingsPatchDto) -> Result<AgentSettingsDto, String> {
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

        let current_provider: String = conn
            .query_row(
                "SELECT active_provider_id FROM agent_settings WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Read current provider error: {e}"))?;

        let effective_provider = active_provider_id
            .clone()
            .unwrap_or_else(|| current_provider.clone());
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

    fn set_provider_auth_refs(
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
            "INSERT INTO agent_provider_auth (provider_id, auth_mode, api_key_ref, oauth_token_ref, oauth_expires_at, oauth_scopes_json, last_error, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, datetime('now'))
             ON CONFLICT(provider_id) DO UPDATE SET auth_mode = excluded.auth_mode, api_key_ref = excluded.api_key_ref, oauth_token_ref = excluded.oauth_token_ref, oauth_expires_at = excluded.oauth_expires_at, updated_at = datetime('now')",
            params![provider_id, auth_mode, api_key_ref, oauth_token_ref, oauth_expires_at],
        )
        .map_err(|e| format!("Upsert provider auth row error: {e}"))?;

        conn.execute(
            "UPDATE agent_settings SET version = version + 1, updated_at = datetime('now') WHERE id = 1",
            [],
        )
        .map_err(|e| format!("Bump settings version error: {e}"))?;

        Ok(())
    }

    fn clear_provider_auth_refs(
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
            "INSERT INTO agent_provider_auth (provider_id, auth_mode, api_key_ref, oauth_token_ref, oauth_expires_at, oauth_scopes_json, last_error, updated_at) VALUES (?1, 'none', NULL, NULL, NULL, NULL, NULL, datetime('now'))
             ON CONFLICT(provider_id) DO UPDATE SET auth_mode = 'none', api_key_ref = NULL, oauth_token_ref = NULL, oauth_expires_at = NULL, updated_at = datetime('now')",
            params![provider_id],
        )
        .map_err(|e| format!("Clear provider auth row error: {e}"))?;

        conn.execute(
            "UPDATE agent_settings SET version = version + 1, updated_at = datetime('now') WHERE id = 1",
            [],
        )
        .map_err(|e| format!("Bump settings version error: {e}"))?;

        Ok(refs)
    }

    fn provider_auth_for(&self, provider_id: &str) -> Result<ProviderAuthDto, String> {
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

    fn active_api_key_ref(&self, provider_id: &str) -> Result<Option<String>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;
        conn.query_row(
            "SELECT api_key_ref FROM agent_provider_auth WHERE provider_id = ?1 AND auth_mode = 'api_key'",
            params![provider_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| format!("Read provider api key ref error: {e}"))
        .map(|v| v.flatten())
    }

    fn active_oauth_token_ref(&self, provider_id: &str) -> Result<Option<String>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("Settings lock error: {e}"))?;
        conn.query_row(
            "SELECT oauth_token_ref FROM agent_provider_auth WHERE provider_id = ?1 AND auth_mode = 'oauth'",
            params![provider_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|e| format!("Read provider oauth token ref error: {e}"))
        .map(|v| v.flatten())
    }

    fn set_provider_config(&self, cfg: ProviderConfigDto) -> Result<ProviderConfigDto, String> {
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

    fn provider_config_for(&self, provider_id: &str) -> Option<ProviderConfigDto> {
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

impl SettingsService {
    pub fn default() -> Result<Self, String> {
        let db_path = default_db_path()?;
        let store = SettingsStore::from_path(&db_path)?;
        Ok(Self {
            store,
            secret_store: Box::new(KeyringSecretStore::new("peekoo-desktop")),
            oauth: OAuthService::new(),
        })
    }

    pub fn get_settings(&self) -> Result<AgentSettingsDto, String> {
        self.store.load_settings()
    }

    pub fn update_settings(
        &self,
        patch: AgentSettingsPatchDto,
    ) -> Result<AgentSettingsDto, String> {
        self.store.apply_patch(patch)
    }

    pub fn catalog(&self) -> Result<AgentSettingsCatalogDto, String> {
        Ok(AgentSettingsCatalogDto {
            providers: provider_catalog(),
            discovered_skills: discover_skills(),
        })
    }

    pub fn set_provider_api_key(&self, req: SetApiKeyRequest) -> Result<ProviderAuthDto, String> {
        if req.api_key.trim().is_empty() {
            return Err("API key cannot be empty".into());
        }

        let key_ref = format!("peekoo/auth/{}/api-key/{}", req.provider_id, Uuid::new_v4());
        self.secret_store
            .put(&key_ref, req.api_key.trim())
            .map_err(secret_error)?;

        self.store.set_provider_auth_refs(
            &req.provider_id,
            "api_key",
            Some(key_ref),
            None,
            None,
        )?;

        self.store.provider_auth_for(&req.provider_id)
    }

    pub fn set_provider_config(
        &self,
        req: SetProviderConfigRequest,
    ) -> Result<ProviderConfigDto, String> {
        if req.base_url.trim().is_empty() {
            return Err("Provider base URL cannot be empty".into());
        }

        let provider_id = req.provider_id.trim().to_string();
        let api = req
            .api
            .unwrap_or_else(|| default_api_for_provider(&provider_id).to_string());
        let auth_header = req
            .auth_header
            .unwrap_or_else(|| default_auth_header_for_provider(&provider_id));

        self.store.set_provider_config(ProviderConfigDto {
            provider_id,
            base_url: req.base_url.trim().to_string(),
            api,
            auth_header,
        })
    }

    pub fn clear_provider_auth(&self, req: ProviderRequest) -> Result<ProviderAuthDto, String> {
        let (api_key_ref, oauth_token_ref) =
            self.store.clear_provider_auth_refs(&req.provider_id)?;

        if let Some(ref_value) = api_key_ref {
            let _ = self.secret_store.delete(&ref_value);
        }
        if let Some(ref_value) = oauth_token_ref {
            let _ = self.secret_store.delete(&ref_value);
        }

        self.store.provider_auth_for(&req.provider_id)
    }

    pub fn start_oauth(&self, req: ProviderRequest) -> Result<OauthStartResponse, String> {
        let started = self
            .oauth
            .start(&req.provider_id)
            .map_err(|e| format!("OAuth start error: {e}"))?;

        Ok(OauthStartResponse {
            flow_id: started.flow_id,
            authorize_url: started.authorize_url,
            opened_browser: false,
        })
    }

    pub async fn oauth_status(
        &self,
        req: OauthStatusRequest,
    ) -> Result<OauthStatusResponse, String> {
        let status = self
            .oauth
            .status(&req.flow_id)
            .await
            .map_err(|e| format!("OAuth status error: {e}"))?;

        if let Some(access_token) = status.access_token {
            if status.provider_id.is_empty() {
                return Err("OAuth status missing provider id".to_string());
            }
            let token_ref = format!(
                "peekoo/auth/{}/oauth/{}",
                status.provider_id,
                Uuid::new_v4()
            );
            self.secret_store
                .put(&token_ref, &access_token)
                .map_err(secret_error)?;
            self.store.set_provider_auth_refs(
                &status.provider_id,
                "oauth",
                None,
                Some(token_ref),
                status.expires_at,
            )?;
            let provider_auth = self.store.provider_auth_for(&status.provider_id)?;
            return Ok(OauthStatusResponse {
                status: OAuthFlowStatus::Completed.as_str().to_string(),
                provider_auth: Some(provider_auth),
                error: None,
            });
        }

        let provider_auth = if status.status == OAuthFlowStatus::Completed {
            self.store.provider_auth_for(&status.provider_id).ok()
        } else {
            None
        };

        Ok(OauthStatusResponse {
            status: status.status.as_str().to_string(),
            provider_auth,
            error: status.error,
        })
    }

    pub fn cancel_oauth(&self, req: OauthStatusRequest) -> Result<OauthCancelResponse, String> {
        Ok(OauthCancelResponse {
            cancelled: self
                .oauth
                .cancel(&req.flow_id)
                .map_err(|e| format!("OAuth cancel error: {e}"))?,
        })
    }

    pub fn to_agent_config(
        &self,
        mut base: AgentServiceConfig,
    ) -> Result<(AgentServiceConfig, i64), String> {
        let settings = self.store.load_settings()?;
        let provider_id = settings.active_provider_id.clone();
        if is_compatible_provider(&provider_id) {
            let provider_cfg = self
                .store
                .provider_config_for(&provider_id)
                .ok_or_else(|| {
                    format!(
                        "Provider '{}' requires base URL configuration in settings",
                        provider_id
                    )
                })?;
            ensure_pi_models_provider(&provider_cfg, &settings.active_model_id)?;
        }
        let model_id = normalize_model_for_provider(&provider_id, &settings.active_model_id);
        base.provider = Some(provider_id.clone());
        base.model = Some(model_id);
        base.system_prompt = settings.system_prompt.clone();
        base.max_tool_iterations = settings.max_tool_iterations;
        base.agent_skills = settings
            .skills
            .iter()
            .filter(|skill| skill.enabled)
            .map(|skill| PathBuf::from(skill.path.clone()))
            .collect();

        if let Some(api_key_ref) = self.store.active_api_key_ref(&provider_id)? {
            if let Ok(api_key) = self.secret_store.get(&api_key_ref) {
                base.api_key = Some(api_key);
            }
        }

        if base.api_key.is_none()
            && let Some(oauth_token_ref) = self.store.active_oauth_token_ref(&provider_id)?
            && let Ok(access_token) = self.secret_store.get(&oauth_token_ref)
        {
            base.api_key = Some(access_token);
        }

        Ok((base, settings.version))
    }
}

fn secret_error(err: SecretStoreError) -> String {
    format!("Secret store error: {err}")
}

fn is_compatible_provider(provider_id: &str) -> bool {
    provider_id == OPENAI_COMPAT_PROVIDER_ID || provider_id == ANTHROPIC_COMPAT_PROVIDER_ID
}

fn default_api_for_provider(provider_id: &str) -> &'static str {
    match provider_id {
        OPENAI_COMPAT_PROVIDER_ID => "openai-completions",
        ANTHROPIC_COMPAT_PROVIDER_ID => "anthropic-messages",
        _ => "openai-completions",
    }
}

fn default_auth_header_for_provider(provider_id: &str) -> bool {
    match provider_id {
        OPENAI_COMPAT_PROVIDER_ID => true,
        ANTHROPIC_COMPAT_PROVIDER_ID => false,
        _ => true,
    }
}

fn ensure_pi_models_provider(cfg: &ProviderConfigDto, model_id: &str) -> Result<(), String> {
    let Some(home) = dirs::home_dir() else {
        return Err("Cannot determine home directory".into());
    };
    let pi_dir = home.join(".pi");
    std::fs::create_dir_all(&pi_dir).map_err(|e| format!("Create ~/.pi dir error: {e}"))?;
    let models_path = pi_dir.join("models.json");

    let mut root: serde_json::Value = if models_path.is_file() {
        let content = std::fs::read_to_string(&models_path)
            .map_err(|e| format!("Read ~/.pi/models.json error: {e}"))?;
        serde_json::from_str(&content).map_err(|e| format!("Parse ~/.pi/models.json error: {e}"))?
    } else {
        serde_json::json!({ "providers": {} })
    };

    if !root.is_object() {
        root = serde_json::json!({ "providers": {} });
    }

    let providers = root
        .as_object_mut()
        .expect("root object")
        .entry("providers")
        .or_insert_with(|| serde_json::json!({}));
    if !providers.is_object() {
        *providers = serde_json::json!({});
    }

    providers.as_object_mut().expect("providers object").insert(
        cfg.provider_id.clone(),
        serde_json::json!({
            "baseUrl": cfg.base_url,
            "api": cfg.api,
            "authHeader": cfg.auth_header,
            "models": [
                {
                    "id": model_id,
                    "name": model_id
                }
            ]
        }),
    );

    let serialized = serde_json::to_string_pretty(&root)
        .map_err(|e| format!("Serialize ~/.pi/models.json error: {e}"))?;
    std::fs::write(&models_path, serialized)
        .map_err(|e| format!("Write ~/.pi/models.json error: {e}"))?;
    Ok(())
}

fn default_db_path() -> Result<PathBuf, String> {
    let Some(home) = dirs::home_dir() else {
        return Err("Cannot determine home directory".into());
    };
    Ok(home.join(".peekoo").join("peekoo.sqlite"))
}

fn discover_skills() -> Vec<SkillDto> {
    use std::collections::HashSet;

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut roots = Vec::new();

    if let Ok(current) = std::env::current_dir() {
        let mut cursor = current;
        loop {
            let candidate = cursor.join(".peekoo").join("skills");
            if candidate.is_dir() {
                roots.push(candidate);
                break;
            }

            let Some(parent) = cursor.parent() else {
                break;
            };
            cursor = parent.to_path_buf();
        }
    }

    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".peekoo").join("skills"));
    }

    for root in roots {
        if !root.is_dir() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.filter_map(|x| x.ok()) {
                let path = entry.path();

                if path.is_dir() {
                    let skill_md = path.join("SKILL.md");
                    if skill_md.is_file() {
                        let skill_id = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        if seen.insert(skill_id.clone()) {
                            out.push(SkillDto {
                                skill_id,
                                source_type: "path".into(),
                                path: skill_md.to_string_lossy().to_string(),
                                enabled: true,
                            });
                        }
                    }
                    continue;
                }

                if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                    let skill_id = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    if seen.insert(skill_id.clone()) {
                        out.push(SkillDto {
                            skill_id,
                            source_type: "path".into(),
                            path: path.to_string_lossy().to_string(),
                            enabled: true,
                        });
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_model_keeps_valid_model() {
        let normalized = normalize_model_for_provider("openai-codex", "gpt-5.3-codex");
        assert_eq!(normalized, "gpt-5.3-codex");
    }

    #[test]
    fn normalize_model_falls_back_for_invalid_pair() {
        let normalized = normalize_model_for_provider("openai-codex", "claude-sonnet-4-6");
        assert_eq!(normalized, "gpt-5.3-codex");
    }

    #[test]
    fn from_path_is_idempotent_with_existing_db() {
        let path = std::env::temp_dir().join(format!("peekoo-settings-{}.sqlite", Uuid::new_v4()));
        let first = SettingsStore::from_path(&path);
        assert!(first.is_ok());

        let second = SettingsStore::from_path(&path);
        assert!(second.is_ok());

        let _ = std::fs::remove_file(path);
    }
}
