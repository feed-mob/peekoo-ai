#![no_main]

use peekoo_plugin_sdk::prelude::*;
use peekoo_python_sdk::{all_python_candidates, is_spawn_error_message};
use serde_json::{Value, json};

#[plugin_fn]
pub fn plugin_init(_: String) -> FnResult<String> {
    Ok(r#"{"status":"ok"}"#.into())
}

#[plugin_fn]
pub fn plugin_shutdown(_: String) -> FnResult<String> {
    Ok(r#"{"status":"ok"}"#.into())
}

#[derive(Deserialize)]
struct MijiaBridgeInput {
    action: String,
    #[serde(default)]
    payload: Value,
}

fn python_candidates() -> Vec<String> {
    all_python_candidates()
}

fn run_bridge_once(program: &str, action: &str, payload_json: &str) -> FnResult<String> {
    let args = vec![
        "companions/mijia_bridge.py".to_string(),
        action.to_string(),
        payload_json.to_string(),
    ];
    let result = match peekoo::process::exec(program, &args, Some(".")) {
        Ok(result) => result,
        Err(err) => {
            return Ok(json!({
                "success": false,
                "message": format!("process exec failed: {err}")
            })
            .to_string());
        }
    };
    if result.ok {
        return Ok(result.stdout.trim().to_string());
    }

    Ok(json!({
        "success": false,
        "message": format!(
            "{program}: exit status {} ({})",
            result.status_code,
            if result.stderr.trim().is_empty() {
                result.stdout.trim()
            } else {
                result.stderr.trim()
            }
        )
    })
    .to_string())
}

#[plugin_fn]
pub fn tool_mijia_bridge(Json(input): Json<MijiaBridgeInput>) -> FnResult<String> {
    let action = input.action.trim();
    if action.is_empty() {
        return Ok(
            json!({
                "success": false,
                "message": "action is required"
            })
            .to_string(),
        );
    }

    let payload_json = serde_json::to_string(&input.payload)
        .map_err(|e| Error::msg(format!("payload serialize error: {e}")))?;

    let mut errors = Vec::new();
    for python in python_candidates() {
        let out = run_bridge_once(&python, action, &payload_json)?;
        if let Ok(parsed) = serde_json::from_str::<Value>(&out) {
            let success = parsed
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if success {
                return Ok(out);
            }
            let message = parsed
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            // If the bridge script actually executed and returned a business error
            // (e.g. login timeout / pending session missing), return it directly.
            // Only continue trying other interpreters for spawn-level failures.
            if !is_spawn_error_message(message) {
                return Ok(out);
            }
        }
        errors.push(format!("{python}: {out}"));
    }

    Ok(json!({
        "success": false,
        "message": format!(
            "Failed to run Mijia bridge script after trying bundled and system Python runtimes.\n{}",
            errors.join("\n")
        )
    })
    .to_string())
}
