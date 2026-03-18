use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use peekoo_notifications::{
    MoodReactionService, Notification, NotificationService, PeekBadgeItem, PeekBadgeService,
};
use peekoo_scheduler::{ScheduleInfo, Scheduler};

use crate::config::resolved_config_map;
use crate::events::{EventBus, PluginEvent};
use crate::manifest::ConfigFieldDef;
use crate::permissions::PermissionStore;
use crate::state::PluginStateStore;

#[derive(Clone)]
struct HostContext {
    plugin_key: String,
    state_store: PluginStateStore,
    permissions: PermissionStore,
    declared_capabilities: Vec<String>,
    allowed_paths: Vec<PathBuf>,
    event_bus: Arc<EventBus>,
    scheduler: Arc<Scheduler>,
    notifications: Arc<NotificationService>,
    peek_badges: Arc<PeekBadgeService>,
    mood_reactions: Arc<MoodReactionService>,
    config_fields: Vec<ConfigFieldDef>,
}

#[allow(clippy::too_many_arguments)]
pub fn build_host_functions(
    plugin_key: &str,
    state_store: &PluginStateStore,
    permissions: &PermissionStore,
    declared_capabilities: Vec<String>,
    allowed_paths: Vec<PathBuf>,
    event_bus: &Arc<EventBus>,
    scheduler: &Arc<Scheduler>,
    notifications: &Arc<NotificationService>,
    peek_badges: &Arc<PeekBadgeService>,
    mood_reactions: &Arc<MoodReactionService>,
    config_fields: Vec<ConfigFieldDef>,
) -> Vec<Function> {
    let ctx = HostContext {
        plugin_key: plugin_key.to_string(),
        state_store: state_store.clone(),
        permissions: permissions.clone(),
        declared_capabilities,
        allowed_paths,
        event_bus: Arc::clone(event_bus),
        scheduler: Arc::clone(scheduler),
        notifications: Arc::clone(notifications),
        peek_badges: Arc::clone(peek_badges),
        mood_reactions: Arc::clone(mood_reactions),
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
            UserData::new(ctx.clone()),
            host_set_peek_badge,
        ),
        Function::new(
            "peekoo_bridge_fs_read",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_bridge_fs_read,
        ),
        Function::new(
            "peekoo_fs_read",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_fs_read,
        ),
        Function::new(
            "peekoo_fs_read_dir",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_fs_read_dir,
        ),
        Function::new(
            "peekoo_set_mood",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx),
            host_set_mood,
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
    require_capability(&ctx, "state:read")?;
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
    require_capability(&ctx, "state:write")?;
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
    require_capability(&ctx, "notifications")?;
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
    require_capability(&ctx, "scheduler")?;
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
    require_capability(&ctx, "scheduler")?;
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
    require_capability(&ctx, "scheduler")?;
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
    require_capability(&ctx, "notifications")?;
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

fn host_bridge_fs_read(
    plugin: &mut CurrentPlugin,
    _inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "bridge:fs_read")?;

    let data_dir = peekoo_paths::peekoo_global_data_dir()
        .map_err(|e| Error::msg(format!("Bridge data dir error: {e}")))?;
    let bridge_dir = data_dir.join("bridges");
    let bridge_file = bridge_dir.join(format!("{}.json", ctx.plugin_key));

    let content = if bridge_file.exists() {
        match std::fs::read_to_string(&bridge_file) {
            Ok(s) => serde_json::Value::String(s),
            Err(e) => {
                tracing::warn!(
                    plugin = ctx.plugin_key.as_str(),
                    "Bridge file read error: {e}"
                );
                serde_json::Value::Null
            }
        }
    } else {
        serde_json::Value::Null
    };

    let response = serde_json::json!({ "content": content }).to_string();
    write_output(plugin, outputs, &response)?;
    Ok(())
}

fn host_fs_read(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "fs:read")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let path = req["path"].as_str().unwrap_or("");
    let tail_bytes = req["tail_bytes"].as_u64();

    let content = resolve_allowed_path(path, &ctx.allowed_paths)
        .filter(|resolved_path| resolved_path.is_file())
        .and_then(
            |resolved_path| match read_file_content(&resolved_path, tail_bytes) {
                Ok(content) => content,
                Err(err) => {
                    tracing::warn!(
                        plugin = ctx.plugin_key.as_str(),
                        path,
                        "File read error: {err}"
                    );
                    None
                }
            },
        );

    let response = serde_json::json!({ "content": content }).to_string();
    write_output(plugin, outputs, &response)?;
    Ok(())
}

fn host_fs_read_dir(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "fs:read")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let path = req["path"].as_str().unwrap_or("");

    let entries = resolve_allowed_path(path, &ctx.allowed_paths)
        .filter(|resolved_path| resolved_path.is_dir())
        .map(|resolved_path| read_dir_entries(&resolved_path, &ctx.plugin_key))
        .unwrap_or_default();

    let response = serde_json::json!({ "entries": entries }).to_string();
    write_output(plugin, outputs, &response)?;
    Ok(())
}

fn host_set_mood(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "pet:mood")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();

    let trigger = req["trigger"].as_str().unwrap_or_default();
    let sticky = req["sticky"].as_bool().unwrap_or(false);

    ctx.mood_reactions.set(trigger, sticky);

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

fn require_capability(ctx: &HostContext, capability: &str) -> Result<(), Error> {
    if !ctx
        .declared_capabilities
        .iter()
        .any(|declared| declared == capability)
    {
        return Err(Error::msg(format!(
            "Plugin '{}' must declare permission '{}' in peekoo-plugin.toml",
            ctx.plugin_key, capability
        )));
    }

    let granted = ctx
        .permissions
        .is_granted(&ctx.plugin_key, capability)
        .map_err(|e| Error::msg(e.to_string()))?;

    if !granted {
        return Err(Error::msg(format!(
            "Plugin '{}' permission '{}' is not granted",
            ctx.plugin_key, capability
        )));
    }

    Ok(())
}

fn resolve_allowed_path(path: &str, allowed_paths: &[PathBuf]) -> Option<PathBuf> {
    let requested_path = expand_tilde_path(path);
    let requested_path = PathBuf::from(requested_path);

    if !is_path_allowed(&requested_path, allowed_paths) {
        return None;
    }

    Some(requested_path)
}

pub(crate) fn expand_tilde_path(path: &str) -> String {
    let Some(home) = dirs::home_dir() else {
        return path.to_string();
    };

    if path == "~" {
        return home.to_string_lossy().into_owned();
    }

    if let Some(stripped) = path.strip_prefix("~/") {
        return home.join(stripped).to_string_lossy().into_owned();
    }

    path.to_string()
}

fn is_path_allowed(path: &Path, allowed_paths: &[PathBuf]) -> bool {
    let Ok(candidate) = fs::canonicalize(path) else {
        return false;
    };

    allowed_paths
        .iter()
        .any(|allowed_root| candidate.starts_with(allowed_root))
}

fn read_file_content(path: &Path, tail_bytes: Option<u64>) -> std::io::Result<Option<String>> {
    if let Some(tail_bytes) = tail_bytes {
        let mut file = fs::File::open(path)?;
        let metadata = file.metadata()?;
        let file_len = metadata.len();
        let start = file_len.saturating_sub(tail_bytes);
        file.seek(SeekFrom::Start(start))?;
        let mut bytes = Vec::with_capacity((file_len - start) as usize);
        file.read_to_end(&mut bytes)?;
        return Ok(Some(String::from_utf8_lossy(&bytes).into_owned()));
    }

    fs::read_to_string(path).map(Some)
}

fn read_dir_entries(path: &Path, plugin_key: &str) -> Vec<serde_json::Value> {
    let Ok(entries) = fs::read_dir(path) else {
        return vec![];
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(err) => {
                    tracing::warn!(plugin = plugin_key, path = %entry.path().display(), "Directory entry metadata error: {err}");
                    return None;
                }
            };

            let modified_secs = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs());

            Some(serde_json::json!({
                "name": entry.file_name().to_string_lossy().into_owned(),
                "is_dir": metadata.is_dir(),
                "modified_secs": modified_secs,
            }))
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{is_path_allowed, read_file_content};

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("peekoo-host-{prefix}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn tail_bytes_reads_only_last_n_bytes() {
        let dir = temp_dir("tail-bytes");
        let path = dir.join("sample.log");
        fs::write(&path, "abcdef").expect("sample file");

        let content = read_file_content(&path, Some(3)).expect("read succeeds");

        assert_eq!(content.as_deref(), Some("def"));
    }

    #[test]
    fn path_is_allowed_when_inside_prefix() {
        let dir = temp_dir("allowed-path");
        let nested = dir.join("nested");
        fs::create_dir_all(&nested).expect("nested dir");
        let path = nested.join("session.jsonl");
        fs::write(&path, "{}").expect("session file");

        assert!(is_path_allowed(&path, &[dir]));
    }

    #[test]
    fn path_is_rejected_when_outside_prefix() {
        let allowed = temp_dir("allowed-root");
        let outside = temp_dir("outside-root").join("secret.txt");
        fs::write(&outside, "nope").expect("outside file");

        assert!(!is_path_allowed(&outside, &[allowed]));
    }
}
