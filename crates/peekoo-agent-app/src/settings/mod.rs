mod catalog;
mod dto;
mod pi_models;
mod skills;
mod store;

use std::path::PathBuf;

use peekoo_agent::config::AgentServiceConfig;
use peekoo_agent_auth::{OAuthFlowStatus, OAuthService};
use peekoo_security::{KeyringSecretStore, SecretStore, SecretStoreError};
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

        if let Some(api_key_ref) = self.store.active_api_key_ref(&provider_id)?
            && let Ok(api_key) = self.secret_store.get(&api_key_ref)
        {
            base.api_key = Some(api_key);
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
