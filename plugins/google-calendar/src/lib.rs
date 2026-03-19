#![no_main]

use extism_pdk::{plugin_fn, FnResult};

#[plugin_fn]
pub fn plugin_init(_: String) -> FnResult<String> {
    Ok(r#"{"status":"ok"}"#.to_string())
}
