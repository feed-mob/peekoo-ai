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
pub(crate) struct WebSocketConnectRequest {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct WebSocketConnectResponse {
    pub socket_id: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct WebSocketSendRequest {
    pub socket_id: String,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct WebSocketRecvRequest {
    pub socket_id: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct WebSocketRecvResponse {
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct WebSocketCloseRequest {
    pub socket_id: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SystemTimeMillisResponse {
    pub time_millis: u64,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SystemUuidV4Response {
    pub uuid: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CryptoEd25519GetOrCreateRequest {
    pub alias: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CryptoEd25519GetOrCreateResponse {
    pub public_key_base64_url: String,
    pub public_key_sha256_hex: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CryptoEd25519SignRequest {
    pub alias: String,
    pub payload: String,
}

#[derive(Serialize, Deserialize)]
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
