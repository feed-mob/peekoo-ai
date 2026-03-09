#![no_main]

use extism_pdk::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
struct StateGetRequest {
    key: String,
}

#[derive(Serialize, Deserialize)]
struct StateGetResponse {
    value: Value,
}

#[derive(Serialize, Deserialize)]
struct StateSetRequest {
    key: String,
    value: Value,
}

#[host_fn]
extern "ExtismHost" {
    fn peekoo_state_get(input: Json<StateGetRequest>) -> Json<StateGetResponse>;
    fn peekoo_state_set(input: Json<StateSetRequest>) -> Json<Value>;
}

#[derive(Serialize, Deserialize)]
struct EchoInput {
    input: String,
}

#[derive(Serialize, Deserialize)]
struct EchoOutput {
    echoed: String,
    call_count: u64,
}

#[plugin_fn]
pub fn plugin_init(_input: String) -> FnResult<String> {
    Ok(r#"{"status":"ok"}"#.to_string())
}

#[plugin_fn]
pub fn tool_example_echo(Json(input): Json<EchoInput>) -> FnResult<Json<EchoOutput>> {
    let call_count = state_get_u64("call_count").unwrap_or(0) + 1;
    state_set("call_count", serde_json::json!(call_count));

    Ok(Json(EchoOutput {
        echoed: input.input,
        call_count,
    }))
}

fn state_get_u64(key: &str) -> Option<u64> {
    let response = unsafe {
        peekoo_state_get(Json(StateGetRequest {
            key: key.to_string(),
        }))
    }
    .ok()?;
    response.0.value.as_u64()
}

fn state_set(key: &str, value: Value) {
    let _ = unsafe {
        peekoo_state_set(Json(StateSetRequest {
            key: key.to_string(),
            value,
        }))
    };
}
