use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    None,
    ApiKey,
    Oauth,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderAuthSettings {
    pub provider_id: String,
    pub auth_mode: AuthMode,
    pub api_key_ref: Option<String>,
    pub oauth_token_ref: Option<String>,
    pub oauth_expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSettings {
    pub skill_id: String,
    pub path: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSettings {
    pub active_provider_id: String,
    pub active_model_id: String,
    pub system_prompt: Option<String>,
    pub max_tool_iterations: usize,
    pub version: i64,
    pub skills: Vec<SkillSettings>,
    pub provider_auth: Vec<ProviderAuthSettings>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AgentSettingsValidationError {
    #[error("active provider id cannot be empty")]
    EmptyProvider,
    #[error("active model id cannot be empty")]
    EmptyModel,
    #[error("max tool iterations must be greater than 0")]
    InvalidMaxToolIterations,
    #[error("api_key auth mode requires api_key_ref")]
    MissingApiKeyRef,
    #[error("oauth auth mode requires oauth_token_ref")]
    MissingOauthTokenRef,
}

impl AgentSettings {
    pub fn validate(&self) -> Result<(), AgentSettingsValidationError> {
        if self.active_provider_id.trim().is_empty() {
            return Err(AgentSettingsValidationError::EmptyProvider);
        }
        if self.active_model_id.trim().is_empty() {
            return Err(AgentSettingsValidationError::EmptyModel);
        }
        if self.max_tool_iterations == 0 {
            return Err(AgentSettingsValidationError::InvalidMaxToolIterations);
        }

        for auth in &self.provider_auth {
            match auth.auth_mode {
                AuthMode::None => {}
                AuthMode::ApiKey if auth.api_key_ref.is_none() => {
                    return Err(AgentSettingsValidationError::MissingApiKeyRef);
                }
                AuthMode::Oauth if auth.oauth_token_ref.is_none() => {
                    return Err(AgentSettingsValidationError::MissingOauthTokenRef);
                }
                _ => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_settings() -> AgentSettings {
        AgentSettings {
            active_provider_id: "anthropic".into(),
            active_model_id: "claude-sonnet-4-6".into(),
            system_prompt: None,
            max_tool_iterations: 50,
            version: 1,
            skills: vec![],
            provider_auth: vec![],
        }
    }

    #[test]
    fn validate_rejects_empty_provider() {
        let mut settings = sample_settings();
        settings.active_provider_id.clear();
        assert_eq!(
            settings.validate(),
            Err(AgentSettingsValidationError::EmptyProvider)
        );
    }

    #[test]
    fn validate_rejects_empty_model() {
        let mut settings = sample_settings();
        settings.active_model_id.clear();
        assert_eq!(
            settings.validate(),
            Err(AgentSettingsValidationError::EmptyModel)
        );
    }

    #[test]
    fn validate_rejects_missing_api_key_ref() {
        let mut settings = sample_settings();
        settings.provider_auth.push(ProviderAuthSettings {
            provider_id: "openai".into(),
            auth_mode: AuthMode::ApiKey,
            api_key_ref: None,
            oauth_token_ref: None,
            oauth_expires_at: None,
        });
        assert_eq!(
            settings.validate(),
            Err(AgentSettingsValidationError::MissingApiKeyRef)
        );
    }

    #[test]
    fn validate_accepts_valid_oauth_ref() {
        let mut settings = sample_settings();
        settings.provider_auth.push(ProviderAuthSettings {
            provider_id: "openai-codex".into(),
            auth_mode: AuthMode::Oauth,
            api_key_ref: None,
            oauth_token_ref: Some("peekoo/auth/openai-codex/123".into()),
            oauth_expires_at: None,
        });
        assert!(settings.validate().is_ok());
    }
}
