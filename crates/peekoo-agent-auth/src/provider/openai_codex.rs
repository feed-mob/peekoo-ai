use crate::flow::{OAuthQueryParam, OAuthStartConfig, OAuthTokenExchangeConfig};

pub const OPENAI_CODEX_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const OPENAI_CODEX_OAUTH_AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
pub const OPENAI_CODEX_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const OPENAI_CODEX_OAUTH_SCOPES: &str = "openid profile email offline_access";

pub fn start_config() -> OAuthStartConfig {
    OAuthStartConfig {
        provider_id: "openai-codex".to_string(),
        authorize_url: OPENAI_CODEX_OAUTH_AUTHORIZE_URL.to_string(),
        client_id: OPENAI_CODEX_OAUTH_CLIENT_ID.to_string(),
        client_secret: None,
        scope: OPENAI_CODEX_OAUTH_SCOPES.to_string(),
        authorize_params: vec![
            OAuthQueryParam::new("id_token_add_organizations", "true"),
            OAuthQueryParam::new("codex_cli_simplified_flow", "true"),
            OAuthQueryParam::new("originator", "pi"),
        ],
        token_exchange: OAuthTokenExchangeConfig {
            token_url: OPENAI_CODEX_OAUTH_TOKEN_URL.to_string(),
            token_params: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_config_has_expected_provider_id() {
        let config = start_config();
        assert_eq!(config.provider_id, "openai-codex");
        assert_eq!(config.client_id, OPENAI_CODEX_OAUTH_CLIENT_ID);
        assert!(!config.scope.is_empty());
    }
}
