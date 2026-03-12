//! Private module: raw host function declarations and request/response types.
//!
//! Plugin authors should never use this module directly.
//! Use the safe wrappers in [`crate::peekoo`] instead.

use extism_pdk::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::ScheduleInfo;

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
}
