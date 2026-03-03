use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use peekoo_agent::config::AgentServiceConfig;
use peekoo_persistence_sqlite::{MIGRATION_0001_INIT, MIGRATION_0002_AGENT_SETTINGS};
use peekoo_security::{KeyringSecretStore, SecretStore, SecretStoreError};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use uuid::Uuid;

const DEFAULT_PROVIDER: &str = "anthropic";
const DEFAULT_MODEL: &str = "claude-sonnet-4-6";
const OPENAI_CODEX_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OPENAI_CODEX_OAUTH_AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const OPENAI_CODEX_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OPENAI_CODEX_OAUTH_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const OPENAI_CODEX_OAUTH_SCOPES: &str = "openid profile email offline_access";

fn models_for_provider(provider_id: &str) -> &'static [&'static str] {
    match provider_id {
        "anthropic" => &["claude-sonnet-4-6", "claude-opus-4-5"],
        "openai" => &["gpt-4o", "gpt-4.1"],
        "openai-codex" => &["gpt-5.3-codex"],
        _ => &[DEFAULT_MODEL],
    }
}

fn default_model_for_provider(provider_id: &str) -> &'static str {
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
    pub skills: Vec<SkillDto>,
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

#[derive(Clone)]
struct OauthFlow {
    provider_id: String,
    verifier: String,
    auth_code: Option<String>,
    status: String,
    error: Option<String>,
}

pub struct SettingsStore {
    conn: Mutex<Connection>,
}

pub struct SettingsService {
    store: SettingsStore,
    secret_store: Box<dyn SecretStore>,
    oauth_flows: Arc<Mutex<HashMap<String, OauthFlow>>>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OpenAiCodexTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
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
        let skills = skills.map_err(|e| format!("Map skill rows error: {e}"))?;

        Ok(AgentSettingsDto {
            active_provider_id: row.0,
            active_model_id: row.1,
            system_prompt: row.2,
            max_tool_iterations: row.3 as usize,
            version: row.4,
            provider_auth,
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
            oauth_flows: Arc::new(Mutex::new(HashMap::new())),
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
        let flow_id = Uuid::new_v4().to_string();
        let (verifier, challenge) = generate_pkce();

        let authorize_url = match req.provider_id.as_str() {
            "openai-codex" => build_url_with_query(
                OPENAI_CODEX_OAUTH_AUTHORIZE_URL,
                &[
                    ("response_type", "code"),
                    ("client_id", OPENAI_CODEX_OAUTH_CLIENT_ID),
                    ("redirect_uri", OPENAI_CODEX_OAUTH_REDIRECT_URI),
                    ("scope", OPENAI_CODEX_OAUTH_SCOPES),
                    ("code_challenge", &challenge),
                    ("code_challenge_method", "S256"),
                    ("state", &verifier),
                    ("id_token_add_organizations", "true"),
                    ("codex_cli_simplified_flow", "true"),
                    ("originator", "pi"),
                ],
            ),
            _ => {
                return Err(format!(
                    "OAuth not supported for provider {}",
                    req.provider_id
                ));
            }
        };

        let mut lock = self
            .oauth_flows
            .lock()
            .map_err(|e| format!("OAuth flow lock error: {e}"))?;
        lock.insert(
            flow_id.clone(),
            OauthFlow {
                provider_id: req.provider_id,
                verifier,
                auth_code: None,
                status: "pending".into(),
                error: None,
            },
        );
        drop(lock);

        spawn_oauth_callback_listener(self.oauth_flows.clone(), flow_id.clone());

        Ok(OauthStartResponse {
            flow_id,
            authorize_url,
            opened_browser: false,
        })
    }

    pub fn oauth_status(&self, req: OauthStatusRequest) -> Result<OauthStatusResponse, String> {
        let flow = {
            let lock = self
                .oauth_flows
                .lock()
                .map_err(|e| format!("OAuth flow lock error: {e}"))?;
            lock.get(&req.flow_id).cloned()
        };

        let Some(flow) = flow else {
            return Ok(OauthStatusResponse {
                status: "expired".into(),
                provider_auth: None,
                error: Some("OAuth flow not found".into()),
            });
        };

        if let Some(error) = flow.error {
            return Ok(OauthStatusResponse {
                status: "failed".into(),
                provider_auth: None,
                error: Some(error),
            });
        }

        if flow.status == "completed" {
            let provider_auth = self.store.provider_auth_for(&flow.provider_id).ok();
            return Ok(OauthStatusResponse {
                status: "completed".into(),
                provider_auth,
                error: None,
            });
        }

        let Some(auth_code) = flow.auth_code else {
            return Ok(OauthStatusResponse {
                status: "pending".into(),
                provider_auth: None,
                error: None,
            });
        };

        match flow.provider_id.as_str() {
            "openai-codex" => {
                let token = exchange_openai_codex_token(&auth_code, &flow.verifier)?;
                let token_ref = format!("peekoo/auth/openai-codex/oauth/{}", Uuid::new_v4());
                self.secret_store
                    .put(&token_ref, &token.access_token)
                    .map_err(secret_error)?;

                let expires_at = oauth_expires_at_iso(token.expires_in);
                self.store.set_provider_auth_refs(
                    "openai-codex",
                    "oauth",
                    None,
                    Some(token_ref),
                    Some(expires_at),
                )?;

                let mut lock = self
                    .oauth_flows
                    .lock()
                    .map_err(|e| format!("OAuth flow lock error: {e}"))?;
                if let Some(stored) = lock.get_mut(&req.flow_id) {
                    stored.status = "completed".into();
                    stored.auth_code = None;
                }

                let provider_auth = self.store.provider_auth_for("openai-codex")?;
                Ok(OauthStatusResponse {
                    status: "completed".into(),
                    provider_auth: Some(provider_auth),
                    error: None,
                })
            }
            _ => Ok(OauthStatusResponse {
                status: "failed".into(),
                provider_auth: None,
                error: Some(format!(
                    "OAuth not supported for provider {}",
                    flow.provider_id
                )),
            }),
        }
    }

    pub fn cancel_oauth(&self, req: OauthStatusRequest) -> Result<OauthCancelResponse, String> {
        let mut lock = self
            .oauth_flows
            .lock()
            .map_err(|e| format!("OAuth flow lock error: {e}"))?;
        Ok(OauthCancelResponse {
            cancelled: lock.remove(&req.flow_id).is_some(),
        })
    }

    pub fn to_agent_config(
        &self,
        mut base: AgentServiceConfig,
    ) -> Result<(AgentServiceConfig, i64), String> {
        let settings = self.store.load_settings()?;
        let provider_id = settings.active_provider_id.clone();
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

fn default_db_path() -> Result<PathBuf, String> {
    let Some(home) = dirs::home_dir() else {
        return Err("Cannot determine home directory".into());
    };
    Ok(home.join(".peekoo").join("peekoo.sqlite"))
}

fn discover_skills() -> Vec<SkillDto> {
    let mut out = Vec::new();
    let mut roots = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        roots.push(current.join(".peekoo").join("skills"));
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
                if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                    let skill_id = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
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
    out
}

fn spawn_oauth_callback_listener(flows: Arc<Mutex<HashMap<String, OauthFlow>>>, flow_id: String) {
    std::thread::spawn(move || {
        let listener = match TcpListener::bind("127.0.0.1:1455") {
            Ok(listener) => listener,
            Err(err) => {
                set_oauth_flow_error(
                    &flows,
                    &flow_id,
                    format!("Failed to bind OAuth callback listener on 127.0.0.1:1455: {err}"),
                );
                return;
            }
        };

        let _ = listener.set_nonblocking(true);
        let started_at = std::time::Instant::now();

        loop {
            if started_at.elapsed() > Duration::from_secs(300) {
                set_oauth_flow_error(&flows, &flow_id, "OAuth flow timed out".to_string());
                return;
            }

            match listener.accept() {
                Ok((mut stream, _addr)) => {
                    let mut first_line = String::new();
                    {
                        let mut reader = BufReader::new(&mut stream);
                        if reader.read_line(&mut first_line).is_err() {
                            set_oauth_flow_error(
                                &flows,
                                &flow_id,
                                "Failed to read OAuth callback request".to_string(),
                            );
                            return;
                        }
                    }

                    let path = first_line
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("/")
                        .to_string();
                    let query = path
                        .split_once('?')
                        .map(|(_, query)| query)
                        .unwrap_or("")
                        .split('#')
                        .next()
                        .unwrap_or("");

                    let pairs = parse_query_pairs(query);
                    let code = pairs
                        .iter()
                        .find_map(|(k, v)| (k == "code").then(|| v.clone()));
                    let state = pairs
                        .iter()
                        .find_map(|(k, v)| (k == "state").then(|| v.clone()));
                    let oauth_error = pairs
                        .iter()
                        .find_map(|(k, v)| (k == "error").then(|| v.clone()));

                    let mut success = false;
                    let mut message =
                        "OAuth callback received. You can close this window.".to_string();

                    {
                        let mut lock = match flows.lock() {
                            Ok(lock) => lock,
                            Err(_) => return,
                        };

                        let Some(flow) = lock.get_mut(&flow_id) else {
                            return;
                        };

                        if let Some(error) = oauth_error {
                            flow.error = Some(format!("OAuth provider returned error: {error}"));
                            flow.status = "failed".into();
                            message = "OAuth failed. You can close this window.".to_string();
                        } else if code.is_none() {
                            flow.error = Some("Missing OAuth authorization code".to_string());
                            flow.status = "failed".into();
                            message = "OAuth failed. Missing authorization code.".to_string();
                        } else if state.as_deref() != Some(flow.verifier.as_str()) {
                            flow.error = Some("OAuth state mismatch".to_string());
                            flow.status = "failed".into();
                            message = "OAuth failed. State mismatch.".to_string();
                        } else {
                            flow.auth_code = code;
                            flow.status = "code_received".into();
                            success = true;
                        }
                    }

                    let status_line = if success {
                        "HTTP/1.1 200 OK"
                    } else {
                        "HTTP/1.1 400 Bad Request"
                    };
                    let body = format!(
                        "<html><body><h2>{}</h2><p>Return to Peekoo.</p></body></html>",
                        message
                    );
                    let response = format!(
                        "{status_line}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(response.as_bytes());
                    let _ = stream.flush();
                    return;
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(err) => {
                    set_oauth_flow_error(
                        &flows,
                        &flow_id,
                        format!("OAuth callback listener error: {err}"),
                    );
                    return;
                }
            }
        }
    });
}

fn set_oauth_flow_error(
    flows: &Arc<Mutex<HashMap<String, OauthFlow>>>,
    flow_id: &str,
    error_message: String,
) {
    if let Ok(mut lock) = flows.lock()
        && let Some(flow) = lock.get_mut(flow_id)
    {
        flow.status = "failed".into();
        flow.error = Some(error_message);
    }
}

fn generate_pkce() -> (String, String) {
    let uuid1 = Uuid::new_v4();
    let uuid2 = Uuid::new_v4();
    let mut random = [0u8; 32];
    random[..16].copy_from_slice(uuid1.as_bytes());
    random[16..].copy_from_slice(uuid2.as_bytes());

    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random);
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(sha2::Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

fn build_url_with_query(base: &str, params: &[(&str, &str)]) -> String {
    let mut url = String::with_capacity(base.len() + 128);
    url.push_str(base);
    url.push('?');

    for (index, (key, value)) in params.iter().enumerate() {
        if index > 0 {
            url.push('&');
        }
        url.push_str(&percent_encode_component(key));
        url.push('=');
        url.push_str(&percent_encode_component(value));
    }
    url
}

fn percent_encode_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push_str("%20"),
            other => {
                let _ = std::fmt::Write::write_fmt(&mut out, format_args!("%{other:02X}"));
            }
        }
    }
    out
}

fn percent_decode_component(value: &str) -> Option<String> {
    if !value.as_bytes().contains(&b'%') && !value.as_bytes().contains(&b'+') {
        return Some(value.to_string());
    }

    let mut out = Vec::with_capacity(value.len());
    let mut bytes = value.as_bytes().iter().copied();
    while let Some(byte) = bytes.next() {
        match byte {
            b'+' => out.push(b' '),
            b'%' => {
                let hi = bytes.next()?;
                let lo = bytes.next()?;
                let hex_bytes = [hi, lo];
                let hex = std::str::from_utf8(&hex_bytes).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
            }
            other => out.push(other),
        }
    }

    String::from_utf8(out).ok()
}

fn parse_query_pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|part| !part.trim().is_empty())
        .filter_map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            let key = percent_decode_component(key.trim())?;
            let value = percent_decode_component(value.trim())?;
            Some((key, value))
        })
        .collect()
}

fn exchange_openai_codex_token(
    authorization_code: &str,
    verifier: &str,
) -> Result<OpenAiCodexTokenResponse, String> {
    let form_body = format!(
        "grant_type=authorization_code&client_id={}&code={}&code_verifier={}&redirect_uri={}",
        percent_encode_component(OPENAI_CODEX_OAUTH_CLIENT_ID),
        percent_encode_component(authorization_code),
        percent_encode_component(verifier),
        percent_encode_component(OPENAI_CODEX_OAUTH_REDIRECT_URI)
    );

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(OPENAI_CODEX_OAUTH_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(form_body)
        .send()
        .map_err(|e| format!("OpenAI Codex token exchange request failed: {e}"))?;

    let status = response.status();
    let body = response
        .text()
        .unwrap_or_else(|_| "<failed to read body>".to_string());
    if !status.is_success() {
        return Err(format!(
            "OpenAI Codex token exchange failed ({status}): {body}"
        ));
    }

    serde_json::from_str(&body)
        .map_err(|e| format!("Invalid OpenAI Codex token response: {e}; body: {body}"))
}

fn oauth_expires_at_iso(expires_in_seconds: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let expires = now.saturating_add(expires_in_seconds.max(0) as u64);
    expires.to_string()
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

    #[test]
    fn build_openai_codex_oauth_url_has_required_params() {
        let (verifier, challenge) = generate_pkce();
        let url = build_url_with_query(
            OPENAI_CODEX_OAUTH_AUTHORIZE_URL,
            &[
                ("response_type", "code"),
                ("client_id", OPENAI_CODEX_OAUTH_CLIENT_ID),
                ("redirect_uri", OPENAI_CODEX_OAUTH_REDIRECT_URI),
                ("scope", OPENAI_CODEX_OAUTH_SCOPES),
                ("code_challenge", &challenge),
                ("code_challenge_method", "S256"),
                ("state", &verifier),
            ],
        );

        assert!(url.starts_with(OPENAI_CODEX_OAUTH_AUTHORIZE_URL));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state="));
    }

    #[test]
    fn parse_query_pairs_decodes_values() {
        let pairs = parse_query_pairs("code=abc123&state=hello%20world");
        assert!(pairs.iter().any(|(k, v)| k == "code" && v == "abc123"));
        assert!(
            pairs
                .iter()
                .any(|(k, v)| k == "state" && v == "hello world")
        );
    }
}
