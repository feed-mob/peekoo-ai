use std::sync::Arc;

use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use peekoo_notifications::{Notification, NotificationService, PeekBadgeItem, PeekBadgeService};
use peekoo_scheduler::{ScheduleInfo, Scheduler};

use crate::config::resolved_config_map;
use crate::events::{EventBus, PluginEvent};
use crate::manifest::ConfigFieldDef;
use crate::state::PluginStateStore;

#[derive(Clone)]
struct HostContext {
    plugin_key: String,
    state_store: PluginStateStore,
    event_bus: Arc<EventBus>,
    scheduler: Arc<Scheduler>,
    notifications: Arc<NotificationService>,
    peek_badges: Arc<PeekBadgeService>,
    config_fields: Vec<ConfigFieldDef>,
}

pub fn build_host_functions(
    plugin_key: &str,
    state_store: &PluginStateStore,
    event_bus: &Arc<EventBus>,
    scheduler: &Arc<Scheduler>,
    notifications: &Arc<NotificationService>,
    peek_badges: &Arc<PeekBadgeService>,
    config_fields: Vec<ConfigFieldDef>,
) -> Vec<Function> {
    let ctx = HostContext {
        plugin_key: plugin_key.to_string(),
        state_store: state_store.clone(),
        event_bus: Arc::clone(event_bus),
        scheduler: Arc::clone(scheduler),
        notifications: Arc::clone(notifications),
        peek_badges: Arc::clone(peek_badges),
        config_fields,
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
            UserData::new(ctx.clone()),
            host_notify,
        ),
        Function::new(
            "peekoo_schedule_set",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_schedule_set,
        ),
        Function::new(
            "peekoo_schedule_cancel",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_schedule_cancel,
        ),
        Function::new(
            "peekoo_schedule_get",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_schedule_get,
        ),
        Function::new(
            "peekoo_config_get",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_config_get,
        ),
        Function::new(
            "peekoo_set_peek_badge",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx),
            host_set_peek_badge,
        ),
    ]
}

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
    write_output(
        plugin,
        outputs,
        &serde_json::json!({ "ok": ok }).to_string(),
    )?;
    Ok(())
}

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

fn host_notify(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();

    let delivered = ctx.notifications.notify(Notification {
        source: ctx.plugin_key.clone(),
        title: req["title"].as_str().unwrap_or_default().to_string(),
        body: req["body"].as_str().unwrap_or_default().to_string(),
    });

    write_output(
        plugin,
        outputs,
        &serde_json::json!({ "ok": true, "suppressed": !delivered }).to_string(),
    )?;
    Ok(())
}

fn host_schedule_set(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let key = req["key"].as_str().unwrap_or_default();
    let interval_secs = req["interval_secs"].as_u64().unwrap_or_default();
    let repeat = req["repeat"].as_bool().unwrap_or(true);
    let delay_secs = req["delay_secs"].as_u64();

    let ok = ctx
        .scheduler
        .set(&ctx.plugin_key, key, interval_secs, repeat, delay_secs)
        .is_ok();
    write_output(
        plugin,
        outputs,
        &serde_json::json!({ "ok": ok }).to_string(),
    )?;
    Ok(())
}

fn host_schedule_cancel(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let key = req["key"].as_str().unwrap_or_default();

    ctx.scheduler.cancel(&ctx.plugin_key, key);
    write_output(plugin, outputs, r#"{"ok":true}"#)?;
    Ok(())
}

fn host_schedule_get(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let key = req["key"].as_str().unwrap_or_default();

    let schedule = ctx
        .scheduler
        .list(&ctx.plugin_key)
        .into_iter()
        .find(|schedule| schedule.key == key);

    write_schedule_response(plugin, outputs, schedule)?;
    Ok(())
}

fn host_config_get(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let resolved = resolved_config_map(&ctx.state_store, &ctx.plugin_key, &ctx.config_fields)
        .map_err(|e| Error::msg(e.to_string()))?;

    let response = match req["key"].as_str() {
        Some(key) if !key.is_empty() => {
            serde_json::json!({ "value": resolved.get(key).cloned().unwrap_or(serde_json::Value::Null) })
        }
        _ => serde_json::json!({ "value": serde_json::Value::Object(resolved) }),
    };

    write_output(plugin, outputs, &response.to_string())?;
    Ok(())
}

fn host_set_peek_badge(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    let input_str = read_input(plugin, inputs)?;
    let items: Vec<PeekBadgeItem> = serde_json::from_str(&input_str).unwrap_or_default();

    ctx.peek_badges.set(&ctx.plugin_key, items);

    write_output(plugin, outputs, r#"{"ok":true}"#)?;
    Ok(())
}

fn write_schedule_response(
    plugin: &mut CurrentPlugin,
    outputs: &mut [Val],
    schedule: Option<ScheduleInfo>,
) -> Result<(), Error> {
    let response = serde_json::json!({ "schedule": schedule });
    write_output(plugin, outputs, &response.to_string())
}

fn read_input(plugin: &mut CurrentPlugin, inputs: &[Val]) -> Result<String, Error> {
    let val = inputs
        .first()
        .ok_or_else(|| Error::msg("Host function received no input values"))?;
    plugin.memory_get_val(val)
}

fn write_output(plugin: &mut CurrentPlugin, outputs: &mut [Val], data: &str) -> Result<(), Error> {
    let handle = plugin.memory_new(data)?;
    outputs[0] = plugin.memory_to_val(handle);
    Ok(())
}
