mod catalog;
mod dto;
mod pi_models;
mod skills;
mod store;

use std::path::{Path, PathBuf};

use peekoo_agent::config::AgentServiceConfig;
use peekoo_agent_auth::{OAuthFlowStatus, OAuthService};
use peekoo_security::{
    FallbackSecretStore, FileSecretStore, KeyringSecretStore, SecretStore, SecretStoreError,
};
use uuid::Uuid;

use crate::settings::catalog::{
    default_api_for_provider, default_auth_header_for_provider, is_compatible_provider,
    normalize_model_for_provider, provider_catalog,
};
use crate::settings::pi_models::ensure_pi_models_provider;
use crate::settings::skills::discover_skills;
use crate::settings::store::SettingsStore;

pub use dto::{
    AgentSettingsCatalogDto, AgentSettingsDto, AgentSettingsPatchDto, OauthCancelResponse,
    OauthStartResponse, OauthStatusRequest, OauthStatusResponse, ProviderAuthDto,
    ProviderCatalogDto, ProviderConfigDto, ProviderRequest, SetApiKeyRequest,
    SetProviderConfigRequest, SkillDto,
};

pub struct SettingsService {
    store: SettingsStore,
    secret_store: Box<dyn SecretStore>,
    oauth: OAuthService,
}

impl SettingsService {
    pub fn new() -> Result<Self, String> {
        let db_path = default_db_path()?;
        migrate_legacy_settings_db_if_needed(&db_path)?;
        let store = SettingsStore::from_path(&db_path)?;
        let fallback_root = peekoo_paths::peekoo_global_data_dir()?.join("secrets");
        Ok(Self {
            store,
            secret_store: Box::new(FallbackSecretStore::new(
                Box::new(KeyringSecretStore::new("peekoo-desktop")),
                Box::new(FileSecretStore::new(fallback_root)),
            )),
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
        let provider_id = req.provider_id.trim();
        let api_key = req.api_key.trim();

        if provider_id.is_empty() {
            return Err("Provider id cannot be empty".into());
        }
        if api_key.is_empty() {
            return Err("API key cannot be empty".into());
        }

        let key_ref = format!("peekoo/auth/{provider_id}/api-key");
        self.secret_store
            .put(&key_ref, api_key)
            .map_err(secret_error)?;

        if let Err(err) = self.secret_store.get(&key_ref) {
            let _ = self.secret_store.delete(&key_ref);
            return Err(format!(
                "Failed to verify API key in secure storage: {err}. \
                 Please check keyring/local storage availability and try again."
            ));
        }

        self.store
            .set_provider_auth_refs(provider_id, "api_key", Some(key_ref), None, None)?;

        self.store.provider_auth_for(provider_id)
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
            match self.secret_store.get(&api_key_ref) {
                Ok(api_key) => base.api_key = Some(api_key),
                Err(e) => {
                    return Err(format!(
                        "Failed to retrieve API key from secure storage: {e}. \
                         Try re-saving your API key in settings."
                    ));
                }
            }
        }

        if base.api_key.is_none()
            && let Some(oauth_token_ref) = self.store.active_oauth_token_ref(&provider_id)?
        {
            match self.secret_store.get(&oauth_token_ref) {
                Ok(access_token) => base.api_key = Some(access_token),
                Err(e) => {
                    return Err(format!(
                        "Failed to retrieve OAuth token from secure storage: {e}. \
                         Try reconnecting OAuth in settings."
                    ));
                }
            }
        }

        Ok((base, settings.version))
    }
}

fn secret_error(err: SecretStoreError) -> String {
    format!("Secret store error: {err}")
}

fn default_db_path() -> Result<PathBuf, String> {
    peekoo_paths::peekoo_settings_db_path()
}

fn migrate_legacy_settings_db_if_needed(target_db_path: &Path) -> Result<(), String> {
    migrate_settings_db_if_needed(target_db_path, peekoo_paths::peekoo_legacy_home_dir())
}

fn migrate_settings_db_if_needed(
    target_db_path: &Path,
    legacy_root: Option<PathBuf>,
) -> Result<(), String> {
    if target_db_path.exists() {
        return Ok(());
    }

    let Some(legacy_root) = legacy_root else {
        return Ok(());
    };
    let legacy_db_path = legacy_root.join("peekoo.sqlite");
    if !legacy_db_path.is_file() {
        return Ok(());
    }

    if let Some(parent) = target_db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Create settings db dir error ({}): {e}", parent.display()))?;
    }

    std::fs::copy(&legacy_db_path, target_db_path).map_err(|e| {
        format!(
            "Migrate settings db error ({} -> {}): {e}",
            legacy_db_path.display(),
            target_db_path.display()
        )
    })?;

    Ok(())
}

#[cfg(test)]
impl SettingsService {
    fn with_secret_store(
        db_path: &Path,
        secret_store: Box<dyn SecretStore>,
    ) -> Result<Self, String> {
        let store = store::SettingsStore::from_path(db_path)?;
        Ok(Self {
            store,
            secret_store,
            oauth: OAuthService::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use peekoo_security::{FallbackSecretStore, InMemorySecretStore};

    struct WriteOnlySecretStore;

    impl SecretStore for WriteOnlySecretStore {
        fn put(&self, _key: &str, _value: &str) -> Result<(), SecretStoreError> {
            Ok(())
        }

        fn get(&self, _key: &str) -> Result<String, SecretStoreError> {
            Err(SecretStoreError::NotFound)
        }

        fn delete(&self, _key: &str) -> Result<(), SecretStoreError> {
            Ok(())
        }
    }

    struct UnavailableSecretStore;

    impl SecretStore for UnavailableSecretStore {
        fn put(&self, _key: &str, _value: &str) -> Result<(), SecretStoreError> {
            Err(SecretStoreError::Unavailable)
        }

        fn get(&self, _key: &str) -> Result<String, SecretStoreError> {
            Err(SecretStoreError::Unavailable)
        }

        fn delete(&self, _key: &str) -> Result<(), SecretStoreError> {
            Err(SecretStoreError::Unavailable)
        }
    }

    fn temp_db_path(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "peekoo-settings-{prefix}-{}.sqlite",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    #[test]
    fn default_db_path_uses_shared_paths_crate() {
        let expected = peekoo_paths::peekoo_settings_db_path().expect("shared db path");
        let actual = default_db_path().expect("settings db path");
        assert_eq!(actual, expected);
    }

    #[test]
    fn migration_copies_legacy_db_when_target_missing() {
        let temp = std::env::temp_dir().join(format!(
            "peekoo-settings-migration-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let legacy_root = temp.join("legacy");
        let target_root = temp.join("target");
        std::fs::create_dir_all(&legacy_root).expect("create legacy root");
        std::fs::create_dir_all(&target_root).expect("create target root");

        let legacy_db = legacy_root.join("peekoo.sqlite");
        std::fs::write(&legacy_db, b"legacy-db").expect("write legacy db");
        let target_db = target_root.join("peekoo.sqlite");

        migrate_settings_db_if_needed(&target_db, Some(legacy_root)).expect("migrate db");
        let copied = std::fs::read(&target_db).expect("read copied db");
        assert_eq!(copied, b"legacy-db");

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn to_agent_config_returns_error_when_keyring_get_fails() {
        let db_path = temp_db_path("keyring-fail");
        let secret_store = InMemorySecretStore::default();
        let svc =
            SettingsService::with_secret_store(&db_path, Box::new(secret_store.clone())).unwrap();

        // Save an API key (puts it in both DB and secret store)
        svc.set_provider_api_key(SetApiKeyRequest {
            provider_id: "anthropic".into(),
            api_key: "sk-test-123".into(),
        })
        .expect("save api key");

        // Verify it works when the secret exists
        let base = AgentServiceConfig::default();
        let (config, _version) = svc.to_agent_config(base).expect("config with key");
        assert_eq!(config.api_key.as_deref(), Some("sk-test-123"));

        // Now remove the secret from the store (simulates keyring failure)
        let settings = svc.get_settings().unwrap();
        let auth = settings
            .provider_auth
            .iter()
            .find(|a| a.provider_id == "anthropic")
            .expect("auth entry");
        assert!(auth.configured);

        // Read the ref from DB and delete it from the in-memory secret store
        let api_key_ref = svc
            .store
            .active_api_key_ref("anthropic")
            .unwrap()
            .expect("ref exists");
        secret_store.delete(&api_key_ref).expect("delete secret");

        // Now to_agent_config should return an error, not silently proceed
        let base = AgentServiceConfig::default();
        let result = svc.to_agent_config(base);
        match result {
            Ok(_) => panic!("Expected error when keyring secret is missing"),
            Err(err) => assert!(
                err.contains("Failed to retrieve API key from secure storage"),
                "Expected descriptive error, got: {err}"
            ),
        }

        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn set_provider_api_key_fails_when_secret_cannot_be_read_back() {
        let db_path = temp_db_path("write-only-secret-store");
        let svc = SettingsService::with_secret_store(&db_path, Box::new(WriteOnlySecretStore))
            .expect("create settings service");

        let result = svc.set_provider_api_key(SetApiKeyRequest {
            provider_id: "anthropic-compatible".into(),
            api_key: "sk-test-123".into(),
        });

        match result {
            Ok(_) => panic!("Expected save failure when secret store cannot read back"),
            Err(err) => assert!(
                err.contains("Failed to verify API key in secure storage"),
                "Expected descriptive verification error, got: {err}"
            ),
        }

        let settings = svc.get_settings().expect("load settings");
        let auth = settings
            .provider_auth
            .iter()
            .find(|entry| entry.provider_id == "anthropic-compatible");
        assert!(
            auth.is_none(),
            "auth row should not be created on failed save"
        );

        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn set_provider_api_key_succeeds_with_fallback_store_when_primary_unavailable() {
        let db_path = temp_db_path("fallback-save-success");
        let fallback_mem = InMemorySecretStore::default();
        let composite = FallbackSecretStore::new(
            Box::new(UnavailableSecretStore),
            Box::new(fallback_mem.clone()),
        );
        let svc = SettingsService::with_secret_store(&db_path, Box::new(composite))
            .expect("create settings service");

        let auth = svc
            .set_provider_api_key(SetApiKeyRequest {
                provider_id: "anthropic".into(),
                api_key: "sk-fallback".into(),
            })
            .expect("save through fallback");
        assert!(auth.configured);
        assert_eq!(auth.auth_mode, "api_key");

        let base = AgentServiceConfig::default();
        let (config, _version) = svc.to_agent_config(base).expect("resolve config");
        assert_eq!(config.api_key.as_deref(), Some("sk-fallback"));

        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn to_agent_config_reads_oauth_token_via_fallback_when_primary_unavailable() {
        let db_path = temp_db_path("fallback-oauth-read");
        let fallback_mem = InMemorySecretStore::default();
        let composite = FallbackSecretStore::new(
            Box::new(UnavailableSecretStore),
            Box::new(fallback_mem.clone()),
        );
        let svc = SettingsService::with_secret_store(&db_path, Box::new(composite))
            .expect("create settings service");

        svc.update_settings(AgentSettingsPatchDto {
            active_provider_id: Some("openai-codex".into()),
            active_model_id: Some("gpt-5.3-codex".into()),
            system_prompt: None,
            max_tool_iterations: None,
            skills: None,
        })
        .expect("set provider/model");

        let token_ref = "peekoo/auth/openai-codex/oauth/test-token".to_string();
        fallback_mem
            .put(&token_ref, "oauth-fallback-token")
            .expect("seed fallback oauth token");
        svc.store
            .set_provider_auth_refs("openai-codex", "oauth", None, Some(token_ref), None)
            .expect("set oauth refs");

        let base = AgentServiceConfig::default();
        let (config, _version) = svc.to_agent_config(base).expect("resolve config");
        assert_eq!(config.api_key.as_deref(), Some("oauth-fallback-token"));

        let _ = std::fs::remove_file(&db_path);
    }
}
