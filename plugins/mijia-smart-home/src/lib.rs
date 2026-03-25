#![no_main]

use peekoo_plugin_sdk::prelude::*;

#[plugin_fn]
pub fn plugin_init(_: String) -> FnResult<String> {
    Ok(r#"{"status":"ok"}"#.into())
}

#[plugin_fn]
pub fn plugin_shutdown(_: String) -> FnResult<String> {
    Ok(r#"{"status":"ok"}"#.into())
}
