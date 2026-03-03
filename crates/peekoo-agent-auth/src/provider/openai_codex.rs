use crate::error::OAuthError;
use crate::url::build_url_with_query;
use serde::Deserialize;

pub const OPENAI_CODEX_OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const OPENAI_CODEX_OAUTH_AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
pub const OPENAI_CODEX_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const OPENAI_CODEX_OAUTH_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
pub const OPENAI_CODEX_OAUTH_SCOPES: &str = "openid profile email offline_access";

#[derive(Deserialize)]
pub struct OpenAiCodexTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

pub fn build_authorize_url(challenge: &str, verifier: &str) -> String {
    build_url_with_query(
        OPENAI_CODEX_OAUTH_AUTHORIZE_URL,
        &[
            ("response_type", "code"),
            ("client_id", OPENAI_CODEX_OAUTH_CLIENT_ID),
            ("redirect_uri", OPENAI_CODEX_OAUTH_REDIRECT_URI),
            ("scope", OPENAI_CODEX_OAUTH_SCOPES),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
            ("state", verifier),
            ("id_token_add_organizations", "true"),
            ("codex_cli_simplified_flow", "true"),
            ("originator", "pi"),
        ],
    )
}

pub async fn exchange_token(
    authorization_code: &str,
    verifier: &str,
) -> Result<OpenAiCodexTokenResponse, OAuthError> {
    let form_body = format!(
        "grant_type=authorization_code&client_id={}&code={}&code_verifier={}&redirect_uri={}",
        percent_encode_component(OPENAI_CODEX_OAUTH_CLIENT_ID),
        percent_encode_component(authorization_code),
        percent_encode_component(verifier),
        percent_encode_component(OPENAI_CODEX_OAUTH_REDIRECT_URI)
    );

    let client = reqwest::Client::new();
    let response = client
        .post(OPENAI_CODEX_OAUTH_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(form_body)
        .send()
        .await
        .map_err(|e| OAuthError::TokenExchange(e.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<failed to read body>".to_string());
    if !status.is_success() {
        return Err(OAuthError::TokenExchange(format!(
            "OpenAI Codex token exchange failed ({status}): {body}"
        )));
    }

    serde_json::from_str(&body).map_err(|e| OAuthError::InvalidTokenResponse(e.to_string()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_authorize_url_has_required_params() {
        let url = build_authorize_url("challenge123", "state123");
        assert!(url.starts_with(OPENAI_CODEX_OAUTH_AUTHORIZE_URL));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=state123"));
    }
}
