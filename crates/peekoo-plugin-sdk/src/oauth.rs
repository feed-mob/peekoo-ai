use extism_pdk::{Error, Json};

use crate::host_fns::{
    peekoo_oauth_cancel, peekoo_oauth_start, peekoo_oauth_status, OAuthCancelResponse,
    OAuthKeyValue, OAuthStartRequest, OAuthStartResponse, OAuthStatusRequest, OAuthStatusResponse,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRequest<'a> {
    pub provider_id: &'a str,
    pub authorize_url: &'a str,
    pub token_url: &'a str,
    pub client_id: &'a str,
    pub client_secret: Option<&'a str>,
    pub redirect_uri: &'a str,
    pub scope: &'a str,
    pub authorize_params: Vec<(&'a str, &'a str)>,
    pub token_params: Vec<(&'a str, &'a str)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartResponse {
    pub flow_id: String,
    pub authorize_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusResponse {
    pub provider_id: String,
    pub status: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub error: Option<String>,
}

pub fn start(req: StartRequest<'_>) -> Result<StartResponse, Error> {
    let response = unsafe {
        peekoo_oauth_start(Json(OAuthStartRequest {
            provider_id: req.provider_id.to_string(),
            authorize_url: req.authorize_url.to_string(),
            token_url: req.token_url.to_string(),
            client_id: req.client_id.to_string(),
            client_secret: req.client_secret.map(ToString::to_string),
            redirect_uri: req.redirect_uri.to_string(),
            scope: req.scope.to_string(),
            authorize_params: req
                .authorize_params
                .into_iter()
                .map(|(key, value)| OAuthKeyValue {
                    key: key.to_string(),
                    value: value.to_string(),
                })
                .collect(),
            token_params: req
                .token_params
                .into_iter()
                .map(|(key, value)| OAuthKeyValue {
                    key: key.to_string(),
                    value: value.to_string(),
                })
                .collect(),
        }))?
    };

    Ok(StartResponse {
        flow_id: response.0.flow_id,
        authorize_url: response.0.authorize_url,
    })
}

pub fn status(flow_id: &str) -> Result<StatusResponse, Error> {
    let response = unsafe {
        peekoo_oauth_status(Json(OAuthStatusRequest {
            flow_id: flow_id.to_string(),
        }))?
    };

    Ok(StatusResponse {
        provider_id: response.0.provider_id,
        status: response.0.status,
        access_token: response.0.access_token,
        refresh_token: response.0.refresh_token,
        expires_at: response.0.expires_at,
        error: response.0.error,
    })
}

pub fn cancel(flow_id: &str) -> Result<bool, Error> {
    let response = unsafe {
        peekoo_oauth_cancel(Json(OAuthStatusRequest {
            flow_id: flow_id.to_string(),
        }))?
    };

    Ok(response.0.cancelled)
}

#[allow(dead_code)]
fn _assert_response_types(_: OAuthStartResponse, _: OAuthStatusResponse, _: OAuthCancelResponse) {}
