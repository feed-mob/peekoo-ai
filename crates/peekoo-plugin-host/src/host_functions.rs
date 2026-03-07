use std::sync::Arc;

use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};

use crate::events::{EventBus, PluginEvent};
use crate::state::PluginStateStore;

/// Shared context passed to every host function via `UserData`.
#[derive(Clone)]
struct HostContext {
    plugin_key: String,
    state_store: PluginStateStore,
    event_bus: Arc<EventBus>,
}

/// Build the set of host functions injected into a plugin's WASM runtime.
///
/// These functions are registered under the default Extism namespace
/// (`extism:host/user`) so the plugin PDK can import them with
/// `#[host_fn] extern "ExtismHost" { ... }`.
pub fn build_host_functions(
    plugin_key: &str,
    state_store: &PluginStateStore,
    event_bus: &Arc<EventBus>,
) -> Vec<Function> {
    let ctx = HostContext {
        plugin_key: plugin_key.to_string(),
        state_store: state_store.clone(),
        event_bus: Arc::clone(event_bus),
    };

    vec![
        Function::new(
            "peekoo_state_get",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_state_get,
        ),
        Function::new(
            "peekoo_state_set",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_state_set,
        ),
        Function::new(
            "peekoo_log",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_log,
        ),
        Function::new(
            "peekoo_emit_event",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_emit_event,
        ),
        Function::new(
            "peekoo_notify",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx),
            host_notify,
        ),
    ]
}

// ─── Host function implementations ──────────────────────────────────────────

/// Read a key from the plugin's KV store.
/// Input:  `{ "key": "some_key" }`
/// Output: `{ "value": <json_value> }`
fn host_state_get(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let key = req["key"].as_str().unwrap_or("");

    let value = ctx
        .state_store
        .get(&ctx.plugin_key, key)
        .unwrap_or(serde_json::Value::Null);

    let response = serde_json::json!({ "value": value }).to_string();
    write_output(plugin, outputs, &response)?;
    Ok(())
}

/// Write a key to the plugin's KV store.
/// Input:  `{ "key": "some_key", "value": <json_value> }`
/// Output: `{ "ok": true }`
fn host_state_set(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let key = req["key"].as_str().unwrap_or("");
    let value = &req["value"];

    let ok = ctx.state_store.set(&ctx.plugin_key, key, value).is_ok();
    let response = serde_json::json!({ "ok": ok }).to_string();
    write_output(plugin, outputs, &response)?;
    Ok(())
}

/// Log a message from the plugin.
/// Input: `{ "level": "info", "message": "..." }`
fn host_log(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let level = req["level"].as_str().unwrap_or("info");
    let message = req["message"].as_str().unwrap_or("");
    let pk = ctx.plugin_key.as_str();

    match level {
        "error" => tracing::error!(plugin = pk, message = %message, "Plugin log"),
        "warn" => tracing::warn!(plugin = pk, message = %message, "Plugin log"),
        "debug" => tracing::debug!(plugin = pk, message = %message, "Plugin log"),
        _ => tracing::info!(plugin = pk, message = %message, "Plugin log"),
    }

    write_output(plugin, outputs, r#"{"ok":true}"#)?;
    Ok(())
}

/// Emit a Peekoo event from the plugin. The event is enqueued in the
/// [`EventBus`] and processed after the current plugin call returns.
/// Input: `{ "event": "health:reminder-due", "payload": { ... } }`
fn host_emit_event(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();

    let event_name = req["event"].as_str().unwrap_or("").to_string();
    let payload = req["payload"].clone();

    ctx.event_bus.enqueue(PluginEvent {
        source_plugin: ctx.plugin_key.clone(),
        event: event_name,
        payload,
    });

    write_output(plugin, outputs, r#"{"ok":true}"#)?;
    Ok(())
}

/// Send a desktop notification to the user.
/// Input: `{ "title": "...", "body": "..." }`
fn host_notify(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;

    let pk = ctx.plugin_key.as_str();
    tracing::info!(plugin = pk, "Plugin notification request: {input_str}");

    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    ctx.event_bus.enqueue(PluginEvent {
        source_plugin: ctx.plugin_key.clone(),
        event: "plugin:notification".to_string(),
        payload: req,
    });

    write_output(plugin, outputs, r#"{"ok":true}"#)?;
    Ok(())
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn read_input(plugin: &mut CurrentPlugin, inputs: &[Val]) -> Result<String, Error> {
    let val = inputs
        .first()
        .ok_or_else(|| Error::msg("Host function received no input values"))?;
    let s: String = plugin.memory_get_val(val)?;
    Ok(s)
}

fn write_output(plugin: &mut CurrentPlugin, outputs: &mut [Val], data: &str) -> Result<(), Error> {
    let handle = plugin.memory_new(data)?;
    outputs[0] = plugin.memory_to_val(handle);
    Ok(())
}
