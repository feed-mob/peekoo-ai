use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::callback::spawn_callback_listener;
use crate::error::OAuthError;
use crate::flow::{OAuthFlow, OAuthFlowStatus, OAuthStartResult, OAuthStatusResult};
use crate::pkce::generate_pkce;
use crate::provider::openai_codex;

pub struct OAuthService {
    flows: Arc<Mutex<HashMap<String, OAuthFlow>>>,
}

impl OAuthService {
    pub fn new() -> Self {
        Self {
            flows: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&self, provider_id: &str) -> Result<OAuthStartResult, OAuthError> {
        let flow_id = Uuid::new_v4().to_string();
        let (verifier, challenge) = generate_pkce();

        let authorize_url = match provider_id {
            "openai-codex" => openai_codex::build_authorize_url(&challenge, &verifier),
            _ => return Err(OAuthError::UnsupportedProvider(provider_id.to_string())),
        };

        let mut lock = self
            .flows
            .lock()
            .map_err(|e| OAuthError::FlowLock(e.to_string()))?;
        lock.insert(
            flow_id.clone(),
            OAuthFlow {
                provider_id: provider_id.to_string(),
                verifier,
                auth_code: None,
                status: OAuthFlowStatus::Pending,
                error: None,
            },
        );
        drop(lock);

        spawn_callback_listener(self.flows.clone(), flow_id.clone());

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
                expires_at: None,
                error: Some("OAuth flow not found".to_string()),
            });
        };

        if let Some(error) = flow.error {
            return Ok(OAuthStatusResult {
                provider_id: flow.provider_id,
                status: OAuthFlowStatus::Failed,
                access_token: None,
                expires_at: None,
                error: Some(error),
            });
        }

        if flow.status == OAuthFlowStatus::Completed {
            return Ok(OAuthStatusResult {
                provider_id: flow.provider_id,
                status: OAuthFlowStatus::Completed,
                access_token: None,
                expires_at: None,
                error: None,
            });
        }

        let Some(auth_code) = flow.auth_code else {
            return Ok(OAuthStatusResult {
                provider_id: flow.provider_id,
                status: OAuthFlowStatus::Pending,
                access_token: None,
                expires_at: None,
                error: None,
            });
        };

        match flow.provider_id.as_str() {
            "openai-codex" => {
                let token = openai_codex::exchange_token(&auth_code, &flow.verifier).await?;

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
                    expires_at: Some(oauth_expires_at_iso(token.expires_in)),
                    error: None,
                })
            }
            _ => Err(OAuthError::UnsupportedProvider(flow.provider_id)),
        }
    }

    pub fn cancel(&self, flow_id: &str) -> Result<bool, OAuthError> {
        let mut lock = self
            .flows
            .lock()
            .map_err(|e| OAuthError::FlowLock(e.to_string()))?;
        Ok(lock.remove(flow_id).is_some())
    }
}

fn oauth_expires_at_iso(expires_in_seconds: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let expires = now.saturating_add(expires_in_seconds.max(0) as u64);
    expires.to_string()
}
