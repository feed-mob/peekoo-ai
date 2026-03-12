#![no_main]

use peekoo_plugin_sdk::prelude::*;

#[derive(Deserialize)]
struct GreetInput {
    name: String,
}

#[derive(Serialize)]
struct GreetOutput {
    message: String,
}

#[plugin_fn]
pub fn plugin_init(_input: String) -> FnResult<String> {
    peekoo::log::info("{{project-name}} plugin initialized");
    Ok(r#"{"status":"ok"}"#.to_string())
}

#[plugin_fn]
pub fn tool_greet(Json(input): Json<GreetInput>) -> FnResult<Json<GreetOutput>> {
    peekoo::log::info(&format!("greeting {}", input.name));

    Ok(Json(GreetOutput {
        message: format!("Hello, {}!", input.name),
    }))
}
