#![no_main]

use peekoo_plugin_sdk::prelude::*;

#[derive(Deserialize)]
struct EchoInput {
    input: String,
}

#[derive(Serialize)]
struct EchoOutput {
    echoed: String,
    call_count: u64,
}

#[plugin_fn]
pub fn plugin_init(_input: String) -> FnResult<String> {
    peekoo::log::info("example-minimal plugin initialized");
    Ok(r#"{"status":"ok"}"#.to_string())
}

#[plugin_fn]
pub fn tool_example_echo(Json(input): Json<EchoInput>) -> FnResult<Json<EchoOutput>> {
    let call_count: u64 = peekoo::state::get("call_count")?.unwrap_or(0) + 1;
    peekoo::state::set("call_count", &call_count)?;

    Ok(Json(EchoOutput {
        echoed: input.input,
        call_count,
    }))
}
