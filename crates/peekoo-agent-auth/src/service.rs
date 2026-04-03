use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use reqwest::Client;
use uuid::Uuid;

use crate::callback::spawn_callback_listener;
use crate::error::OAuthError;
use crate::flow::{
    OAuthFlow, OAuthFlowStatus, OAuthQueryParam, OAuthStartConfig, OAuthStartResult,
    OAuthStatusResult,
};
use crate::pkce::generate_pkce;
use crate::provider::openai_codex;
use crate::url::build_url_with_query;

pub struct OAuthService {
    flows: Arc<Mutex<HashMap<String, OAuthFlow>>>,
}

impl Default for OAuthService {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthService {
    pub fn new() -> Self {
        Self {
            flows: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self, provider_id: &str) -> Result<OAuthStartResult, OAuthError> {
        let config = match provider_id {
            "openai-codex" => openai_codex::start_config(),
            _ => return Err(OAuthError::UnsupportedProvider(provider_id.to_string())),
        };
        self.start_custom(config)
    }

    pub fn start_custom(&self, config: OAuthStartConfig) -> Result<OAuthStartResult, OAuthError> {
        let flow_id = Uuid::new_v4().to_string();
        let (verifier, challenge) = generate_pkce();

        // Try to start the callback listener first to get an available port
        let callback_port = match spawn_callback_listener(self.flows.clone(), flow_id.clone()) {
            Some(port) => port,
            None => {
                return Err(OAuthError::PortBindingFailed(
                    "Failed to bind to any available port for OAuth callback".to_string(),
                ));
            }
        };

        // Build redirect URI dynamically with the actual port
        let redirect_uri = format!("http://127.0.0.1:{}/auth/callback", callback_port);

        let authorize_url = build_authorize_url(&config, &challenge, &verifier, &redirect_uri);

        let mut lock = self
            .flows
            .lock()
            .map_err(|e| OAuthError::FlowLock(e.to_string()))?;
        lock.insert(
            flow_id.clone(),
            OAuthFlow {
                provider_id: config.provider_id.clone(),
                start_config: config,
                verifier,
                auth_code: None,
                status: OAuthFlowStatus::Pending,
                error: None,
                redirect_uri,
            },
        );
        drop(lock);

        Ok(OAuthStartResult {
            flow_id,
            authorize_url,
        })
    }

    pub async fn status(&self, flow_id: &str) -> Result<OAuthStatusResult, OAuthError> {
        let flow = {
            let lock = self
                .flows
                .lock()
                .map_err(|e| OAuthError::FlowLock(e.to_string()))?;
            lock.get(flow_id).cloned()
        };

        let Some(flow) = flow else {
            return Ok(OAuthStatusResult {
                provider_id: String::new(),
                status: OAuthFlowStatus::Expired,
                access_token: None,
                refresh_token: None,
                expires_at: None,
                error: Some("OAuth flow not found".to_string()),
            });
        };

        if let Some(error) = flow.error {
            return Ok(OAuthStatusResult {
                provider_id: flow.provider_id,
                status: OAuthFlowStatus::Failed,
                access_token: None,
                refresh_token: None,
                expires_at: None,
                error: Some(error),
            });
        }

        if flow.status == OAuthFlowStatus::Completed {
            return Ok(OAuthStatusResult {
                provider_id: flow.provider_id,
                status: OAuthFlowStatus::Completed,
                access_token: None,
                refresh_token: None,
                expires_at: None,
                error: None,
            });
        }

        let Some(auth_code) = flow.auth_code else {
            return Ok(OAuthStatusResult {
                provider_id: flow.provider_id,
                status: OAuthFlowStatus::Pending,
                access_token: None,
                refresh_token: None,
                expires_at: None,
                error: None,
            });
        };

        let token = exchange_token(
            &flow.start_config,
            &auth_code,
            &flow.verifier,
            &flow.redirect_uri,
        )
        .await?;

        let mut lock = self
            .flows
            .lock()
            .map_err(|e| OAuthError::FlowLock(e.to_string()))?;
        if let Some(stored) = lock.get_mut(flow_id) {
            stored.status = OAuthFlowStatus::Completed;
            stored.auth_code = None;
        }

        Ok(OAuthStatusResult {
            provider_id: flow.provider_id,
            status: OAuthFlowStatus::Completed,
            access_token: Some(token.access_token),
            refresh_token: token.refresh_token,
            expires_at: Some(oauth_expires_at_iso(token.expires_in)),
            error: None,
        })
    }

    pub fn cancel(&self, flow_id: &str) -> Result<bool, OAuthError> {
        let mut lock = self
            .flows
            .lock()
            .map_err(|e| OAuthError::FlowLock(e.to_string()))?;
        Ok(lock.remove(flow_id).is_some())
    }
}

#[derive(Debug, serde::Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: i64,
    refresh_token: Option<String>,
}

fn build_authorize_url(
    config: &OAuthStartConfig,
    challenge: &str,
    state: &str,
    redirect_uri: &str,
) -> String {
    let mut params = vec![
        ("response_type", "code".to_string()),
        ("client_id", config.client_id.clone()),
        ("redirect_uri", redirect_uri.to_string()),
        ("scope", config.scope.clone()),
        ("code_challenge", challenge.to_string()),
        ("code_challenge_method", "S256".to_string()),
        ("state", state.to_string()),
    ];
    params.extend(
        config
            .authorize_params
            .iter()
            .map(|param| (param.key.as_str(), param.value.clone())),
    );
    let borrowed = params
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect::<Vec<_>>();
    build_url_with_query(&config.authorize_url, &borrowed)
}

async fn exchange_token(
    config: &OAuthStartConfig,
    authorization_code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<OAuthTokenResponse, OAuthError> {
    let form_body = build_token_form_body(config, authorization_code, verifier, redirect_uri);
    let client = Client::new();
    let response = client
        .post(&config.token_exchange.token_url)
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
            "OAuth token exchange failed ({status}): {body}"
        )));
    }

    serde_json::from_str(&body).map_err(|e| OAuthError::InvalidTokenResponse(e.to_string()))
}

fn build_token_form_body(
    config: &OAuthStartConfig,
    authorization_code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> String {
    let mut params = vec![
        OAuthQueryParam::new("grant_type", "authorization_code"),
        OAuthQueryParam::new("client_id", config.client_id.clone()),
        OAuthQueryParam::new("code", authorization_code),
        OAuthQueryParam::new("code_verifier", verifier),
        OAuthQueryParam::new("redirect_uri", redirect_uri),
    ];
    if let Some(client_secret) = config
        .client_secret
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        params.push(OAuthQueryParam::new("client_secret", client_secret));
    }
    params.extend(config.token_exchange.token_params.clone());
    build_form_body(&params)
}

fn build_form_body(params: &[OAuthQueryParam]) -> String {
    params
        .iter()
        .map(|param| {
            format!(
                "{}={}",
                percent_encode_component(&param.key),
                percent_encode_component(&param.value)
            )
        })
        .collect::<Vec<_>>()
        .join("&")
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

fn oauth_expires_at_iso(expires_in_seconds: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let expires = now.saturating_add(expires_in_seconds.max(0) as u64);
    expires.to_string()
}
