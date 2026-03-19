use crate::error::OAuthError;
use crate::url::build_url_with_query;
use serde::Deserialize;

pub const GOOGLE_CALENDAR_OAUTH_AUTHORIZE_URL: &str =
    "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_CALENDAR_OAUTH_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_CALENDAR_OAUTH_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
pub const GOOGLE_CALENDAR_OAUTH_SCOPES: &str =
    "https://www.googleapis.com/auth/calendar.readonly openid email profile";

#[derive(Debug, Deserialize)]
pub struct GoogleCalendarTokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
}

pub fn build_authorize_url(client_id: &str, challenge: &str, state: &str) -> String {
    build_url_with_query(
        GOOGLE_CALENDAR_OAUTH_AUTHORIZE_URL,
        &[
            ("response_type", "code"),
            ("client_id", client_id),
            ("redirect_uri", GOOGLE_CALENDAR_OAUTH_REDIRECT_URI),
            ("scope", GOOGLE_CALENDAR_OAUTH_SCOPES),
            ("code_challenge", challenge),
            ("code_challenge_method", "S256"),
            ("state", state),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ],
    )
}

pub async fn exchange_token(
    client_id: &str,
    client_secret: Option<&str>,
    authorization_code: &str,
    verifier: &str,
) -> Result<GoogleCalendarTokenResponse, OAuthError> {
    let form_body = build_authorization_code_form_body(
        client_id,
        client_secret,
        authorization_code,
        verifier,
    );

    post_form(&form_body).await
}

pub async fn refresh_access_token(
    client_id: &str,
    client_secret: Option<&str>,
    refresh_token: &str,
) -> Result<GoogleCalendarTokenResponse, OAuthError> {
    let form_body = build_refresh_token_form_body(client_id, client_secret, refresh_token);

    post_form(&form_body).await
}

pub fn build_authorization_code_form_body(
    client_id: &str,
    client_secret: Option<&str>,
    authorization_code: &str,
    verifier: &str,
) -> String {
    let mut form_body = format!(
        "grant_type=authorization_code&client_id={}&code={}&code_verifier={}&redirect_uri={}",
        percent_encode_component(client_id),
        percent_encode_component(authorization_code),
        percent_encode_component(verifier),
        percent_encode_component(GOOGLE_CALENDAR_OAUTH_REDIRECT_URI)
    );
    if let Some(client_secret) = client_secret.filter(|value| !value.trim().is_empty()) {
        form_body.push_str("&client_secret=");
        form_body.push_str(&percent_encode_component(client_secret));
    }
    form_body
}

pub fn build_refresh_token_form_body(
    client_id: &str,
    client_secret: Option<&str>,
    refresh_token: &str,
) -> String {
    let mut form_body = format!(
        "grant_type=refresh_token&client_id={}&refresh_token={}",
        percent_encode_component(client_id),
        percent_encode_component(refresh_token),
    );
    if let Some(client_secret) = client_secret.filter(|value| !value.trim().is_empty()) {
        form_body.push_str("&client_secret=");
        form_body.push_str(&percent_encode_component(client_secret));
    }
    form_body
}

async fn post_form(form_body: &str) -> Result<GoogleCalendarTokenResponse, OAuthError> {
    let client = reqwest::Client::new();
    let response = client
        .post(GOOGLE_CALENDAR_OAUTH_TOKEN_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(form_body.to_string())
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
            "Google token exchange failed ({status}): {body}"
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
