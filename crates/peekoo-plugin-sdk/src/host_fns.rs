//! Private module: raw host function declarations and request/response types.
//!
//! Plugin authors should never use this module directly.
//! Use the safe wrappers in [`crate::peekoo`] instead.

use extism_pdk::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{FsEntry, ScheduleInfo};

// ── Request types ──────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub(crate) struct StateGetRequest {
    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct StateGetResponse {
    pub value: Value,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct StateSetRequest {
    pub key: String,
    pub value: Value,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LogRequest {
    pub level: String,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct EmitEventRequest {
    pub event: String,
    pub payload: Value,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct NotifyRequest {
    pub title: String,
    pub body: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct NotifyResponse {
    pub ok: bool,
    pub suppressed: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ScheduleSetRequest {
    pub key: String,
    pub interval_secs: u64,
    pub repeat: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_secs: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ScheduleCancelRequest {
    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ScheduleGetRequest {
    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ScheduleGetResponse {
    pub schedule: Option<ScheduleInfo>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ConfigGetRequest {
    pub key: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ConfigGetResponse {
    pub value: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OAuthStartRequest {
    pub provider_id: String,
    pub client_id: String,
    pub client_secret: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OAuthStartResponse {
    pub flow_id: String,
    pub authorize_url: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OAuthStatusRequest {
    pub flow_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OAuthStatusResponse {
    pub provider_id: String,
    pub status: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OAuthCancelResponse {
    pub cancelled: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SecretGetRequest {
    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SecretGetResponse {
    pub value: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SecretSetRequest {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SecretDeleteRequest {
    pub key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HttpRequest {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<HttpHeader>,
    pub body: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HttpResponse {
    pub status: u16,
    pub body: String,
    #[serde(default)]
    pub headers: Vec<HttpHeader>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct OkResponse {
    pub ok: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct BridgeFsReadResponse {
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FsReadRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tail_bytes: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FsReadResponse {
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FsReadDirRequest {
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FsReadDirResponse {
    pub entries: Vec<FsEntry>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SetMoodRequest {
    pub trigger: String,
    pub sticky: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebSocketConnectRequest {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebSocketConnectResponse {
    pub socket_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebSocketSendRequest {
    pub socket_id: String,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebSocketRecvRequest {
    pub socket_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebSocketRecvResponse {
    pub text: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebSocketCloseRequest {
    pub socket_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemTimeMillisResponse {
    pub time_millis: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemUuidV4Response {
    pub uuid: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CryptoEd25519GetOrCreateRequest {
    pub alias: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CryptoEd25519GetOrCreateResponse {
    pub public_key_base64_url: String,
    pub public_key_sha256_hex: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CryptoEd25519SignRequest {
    pub alias: String,
    pub payload: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CryptoEd25519SignResponse {
    pub signature_base64_url: String,
}

// ── Host function declarations ─────────────────────────────────

#[host_fn]
extern "ExtismHost" {
    pub(crate) fn peekoo_state_get(input: Json<StateGetRequest>) -> Json<StateGetResponse>;
    pub(crate) fn peekoo_state_set(input: Json<StateSetRequest>) -> Json<OkResponse>;
    pub(crate) fn peekoo_log(input: Json<LogRequest>) -> Json<OkResponse>;
    pub(crate) fn peekoo_emit_event(input: Json<EmitEventRequest>) -> Json<OkResponse>;
    pub(crate) fn peekoo_notify(input: Json<NotifyRequest>) -> Json<NotifyResponse>;
    pub(crate) fn peekoo_schedule_set(input: Json<ScheduleSetRequest>) -> Json<OkResponse>;
    pub(crate) fn peekoo_schedule_cancel(input: Json<ScheduleCancelRequest>) -> Json<OkResponse>;
    pub(crate) fn peekoo_schedule_get(input: Json<ScheduleGetRequest>)
        -> Json<ScheduleGetResponse>;
    pub(crate) fn peekoo_config_get(input: Json<ConfigGetRequest>) -> Json<ConfigGetResponse>;
    pub(crate) fn peekoo_oauth_start(input: Json<OAuthStartRequest>) -> Json<OAuthStartResponse>;
    pub(crate) fn peekoo_oauth_status(input: Json<OAuthStatusRequest>)
        -> Json<OAuthStatusResponse>;
    pub(crate) fn peekoo_oauth_cancel(input: Json<OAuthStatusRequest>)
        -> Json<OAuthCancelResponse>;
    pub(crate) fn peekoo_secret_get(input: Json<SecretGetRequest>) -> Json<SecretGetResponse>;
    pub(crate) fn peekoo_secret_set(input: Json<SecretSetRequest>) -> Json<OkResponse>;
    pub(crate) fn peekoo_secret_delete(input: Json<SecretDeleteRequest>) -> Json<OkResponse>;
    pub(crate) fn peekoo_http_request(input: Json<HttpRequest>) -> Json<HttpResponse>;
    pub(crate) fn peekoo_set_peek_badge(input: String) -> Json<OkResponse>;
    pub(crate) fn peekoo_bridge_fs_read(input: String) -> Json<BridgeFsReadResponse>;
    pub(crate) fn peekoo_fs_read(input: Json<FsReadRequest>) -> Json<FsReadResponse>;
    pub(crate) fn peekoo_fs_read_dir(input: Json<FsReadDirRequest>) -> Json<FsReadDirResponse>;
    pub(crate) fn peekoo_websocket_connect(
        input: Json<WebSocketConnectRequest>,
    ) -> Json<WebSocketConnectResponse>;
    pub(crate) fn peekoo_websocket_send(input: Json<WebSocketSendRequest>) -> Json<OkResponse>;
    pub(crate) fn peekoo_websocket_recv(
        input: Json<WebSocketRecvRequest>,
    ) -> Json<WebSocketRecvResponse>;
    pub(crate) fn peekoo_websocket_close(input: Json<WebSocketCloseRequest>) -> Json<OkResponse>;
    pub(crate) fn peekoo_system_time_millis(input: String) -> Json<SystemTimeMillisResponse>;
    pub(crate) fn peekoo_system_uuid_v4(input: String) -> Json<SystemUuidV4Response>;
    pub(crate) fn peekoo_crypto_ed25519_get_or_create(
        input: Json<CryptoEd25519GetOrCreateRequest>,
    ) -> Json<CryptoEd25519GetOrCreateResponse>;
    pub(crate) fn peekoo_crypto_ed25519_sign(
        input: Json<CryptoEd25519SignRequest>,
    ) -> Json<CryptoEd25519SignResponse>;
    pub(crate) fn peekoo_set_mood(input: Json<SetMoodRequest>) -> Json<OkResponse>;
}

#[cfg(test)]
mod tests {
    use super::{
        CryptoEd25519GetOrCreateResponse, CryptoEd25519SignResponse, HttpRequest,
        OAuthStartRequest, SecretSetRequest, SystemTimeMillisResponse, WebSocketConnectRequest,
        WebSocketConnectResponse,
    };

    #[test]
    fn websocket_connect_request_serializes_camel_case() {
        let json = serde_json::to_value(WebSocketConnectRequest {
            url: "ws://127.0.0.1:18789".to_string(),
        })
        .expect("serialize request");

        assert_eq!(
            json.get("url").and_then(|v| v.as_str()),
            Some("ws://127.0.0.1:18789")
        );
    }

    #[test]
    fn websocket_connect_response_deserializes_camel_case() {
        let response: WebSocketConnectResponse =
            serde_json::from_str(r#"{"socketId":"ws-1"}"#).expect("deserialize response");

        assert_eq!(response.socket_id, "ws-1");
    }

    #[test]
    fn system_response_deserializes_camel_case() {
        let response: SystemTimeMillisResponse =
            serde_json::from_str(r#"{"timeMillis":42}"#).expect("deserialize response");

        assert_eq!(response.time_millis, 42);
    }

    #[test]
    fn crypto_response_deserializes_camel_case() {
        let response: CryptoEd25519GetOrCreateResponse =
            serde_json::from_str(r#"{"publicKeyBase64Url":"abc","publicKeySha256Hex":"def"}"#)
                .expect("deserialize response");

        assert_eq!(response.public_key_base64_url, "abc");
        assert_eq!(response.public_key_sha256_hex, "def");
    }

    #[test]
    fn signature_response_deserializes_camel_case() {
        let response: CryptoEd25519SignResponse =
            serde_json::from_str(r#"{"signatureBase64Url":"sig"}"#).expect("deserialize response");

        assert_eq!(response.signature_base64_url, "sig");
    }

    #[test]
    fn oauth_start_request_serializes_camel_case() {
        let json = serde_json::to_value(OAuthStartRequest {
            provider_id: "google-calendar".to_string(),
            client_id: "client-id".to_string(),
            client_secret: Some("secret".to_string()),
        })
        .expect("serialize request");

        assert_eq!(
            json.get("providerId").and_then(|v| v.as_str()),
            Some("google-calendar")
        );
        assert_eq!(
            json.get("clientId").and_then(|v| v.as_str()),
            Some("client-id")
        );
        assert_eq!(
            json.get("clientSecret").and_then(|v| v.as_str()),
            Some("secret")
        );
    }

    #[test]
    fn secret_set_request_serializes_plain_keys() {
        let json = serde_json::to_value(SecretSetRequest {
            key: "token".to_string(),
            value: "value".to_string(),
        })
        .expect("serialize request");

        assert_eq!(json.get("key").and_then(|v| v.as_str()), Some("token"));
        assert_eq!(json.get("value").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn http_request_serializes_camel_case() {
        let json = serde_json::to_value(HttpRequest {
            method: "GET".to_string(),
            url: "https://www.googleapis.com/calendar/v3/calendars/primary/events".to_string(),
            headers: vec![],
            body: None,
        })
        .expect("serialize request");

        assert_eq!(json.get("method").and_then(|v| v.as_str()), Some("GET"));
        assert_eq!(
            json.get("url").and_then(|v| v.as_str()),
            Some("https://www.googleapis.com/calendar/v3/calendars/primary/events")
        );
    }
}
