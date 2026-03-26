use base64::Engine as _;
use ed25519_dalek::SigningKey;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};
use peekoo_agent_auth::{
    OAuthQueryParam, OAuthService, OAuthStartConfig, OAuthTokenExchangeConfig,
};
use peekoo_notifications::{
    MoodReactionService, Notification, NotificationService, PeekBadgeItem, PeekBadgeService,
};
use peekoo_scheduler::{ScheduleInfo, Scheduler};
use peekoo_security::{
    FallbackSecretStore, FileSecretStore, KeyringSecretStore, SecretStore, SecretStoreError,
};
use peekoo_task_app::TaskService;
use rand::rngs::OsRng;
use reqwest::Method;
use sha2::{Digest, Sha256};
use tungstenite::client::IntoClientRequest;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket, connect};
use url::Url;

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
    allowed_hosts: Vec<String>,
    oauth: Arc<OAuthService>,
    secret_store: Arc<dyn SecretStore>,
    websockets: Arc<Mutex<WebSocketStore>>,
    event_bus: Arc<EventBus>,
    scheduler: Arc<Scheduler>,
    notifications: Arc<NotificationService>,
    peek_badges: Arc<PeekBadgeService>,
    mood_reactions: Arc<MoodReactionService>,
    task_service: Arc<dyn TaskService>,
    config_fields: Vec<ConfigFieldDef>,
}

type PluginWebSocket = WebSocket<MaybeTlsStream<TcpStream>>;

#[derive(Default)]
struct WebSocketStore {
    next_id: u64,
    sockets: std::collections::HashMap<String, PluginWebSocket>,
}

#[allow(clippy::too_many_arguments)]
pub fn build_host_functions(
    plugin_key: &str,
    state_store: &PluginStateStore,
    permissions: &PermissionStore,
    declared_capabilities: Vec<String>,
    allowed_paths: Vec<PathBuf>,
    allowed_hosts: Vec<String>,
    event_bus: &Arc<EventBus>,
    scheduler: &Arc<Scheduler>,
    notifications: &Arc<NotificationService>,
    peek_badges: &Arc<PeekBadgeService>,
    mood_reactions: &Arc<MoodReactionService>,
    task_service: Arc<dyn TaskService>,
    config_fields: Vec<ConfigFieldDef>,
) -> Vec<Function> {
    let secret_root = peekoo_paths::peekoo_global_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("peekoo"))
        .join("plugin-secrets");
    let secret_store: Arc<dyn SecretStore> = Arc::new(FallbackSecretStore::new(
        Box::new(KeyringSecretStore::new("peekoo-desktop")),
        Box::new(FileSecretStore::new(secret_root)),
    ));
    let ctx = HostContext {
        plugin_key: plugin_key.to_string(),
        state_store: state_store.clone(),
        permissions: permissions.clone(),
        declared_capabilities,
        allowed_paths,
        allowed_hosts,
        oauth: Arc::new(OAuthService::new()),
        secret_store,
        websockets: Arc::new(Mutex::new(WebSocketStore::default())),
        event_bus: Arc::clone(event_bus),
        scheduler: Arc::clone(scheduler),
        notifications: Arc::clone(notifications),
        peek_badges: Arc::clone(peek_badges),
        mood_reactions: Arc::clone(mood_reactions),
        task_service,
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
            "peekoo_oauth_start",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_oauth_start,
        ),
        Function::new(
            "peekoo_oauth_status",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_oauth_status,
        ),
        Function::new(
            "peekoo_oauth_cancel",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_oauth_cancel,
        ),
        Function::new(
            "peekoo_secret_get",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_secret_get,
        ),
        Function::new(
            "peekoo_secret_set",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_secret_set,
        ),
        Function::new(
            "peekoo_secret_delete",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_secret_delete,
        ),
        Function::new(
            "peekoo_http_request",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_http_request,
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
            "peekoo_websocket_connect",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_websocket_connect,
        ),
        Function::new(
            "peekoo_websocket_send",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_websocket_send,
        ),
        Function::new(
            "peekoo_websocket_recv",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_websocket_recv,
        ),
        Function::new(
            "peekoo_websocket_close",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_websocket_close,
        ),
        Function::new(
            "peekoo_system_time_millis",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_system_time_millis,
        ),
        Function::new(
            "peekoo_system_uuid_v4",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_system_uuid_v4,
        ),
        Function::new(
            "peekoo_crypto_ed25519_get_or_create",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_crypto_ed25519_get_or_create,
        ),
        Function::new(
            "peekoo_crypto_ed25519_sign",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_crypto_ed25519_sign,
        ),
        Function::new(
            "peekoo_set_mood",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_set_mood,
        ),
        Function::new(
            "peekoo_task_create",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_task_create,
        ),
        Function::new(
            "peekoo_task_list",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_task_list,
        ),
        Function::new(
            "peekoo_task_update",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_task_update,
        ),
        Function::new(
            "peekoo_task_delete",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_task_delete,
        ),
        Function::new(
            "peekoo_task_toggle",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx.clone()),
            host_task_toggle,
        ),
        Function::new(
            "peekoo_task_assign",
            [ValType::I64],
            [ValType::I64],
            UserData::new(ctx),
            host_task_assign,
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
    can_log(&ctx)?;
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
    can_emit_events(&ctx)?;
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
    can_notify(&ctx)?;
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
    can_schedule(&ctx)?;
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
    can_schedule(&ctx)?;
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
    can_schedule(&ctx)?;
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

fn host_oauth_start(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    can_oauth(&ctx)?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let start_config = OAuthStartConfig {
        provider_id: req["providerId"].as_str().unwrap_or_default().to_string(),
        authorize_url: req["authorizeUrl"].as_str().unwrap_or_default().to_string(),
        token_exchange: OAuthTokenExchangeConfig {
            token_url: req["tokenUrl"].as_str().unwrap_or_default().to_string(),
            token_params: json_params_to_query_params(&req["tokenParams"]),
        },
        client_id: req["clientId"].as_str().unwrap_or_default().to_string(),
        client_secret: req["clientSecret"].as_str().map(ToString::to_string),
        redirect_uri: req["redirectUri"].as_str().unwrap_or_default().to_string(),
        scope: req["scope"].as_str().unwrap_or_default().to_string(),
        authorize_params: json_params_to_query_params(&req["authorizeParams"]),
    };

    let started = ctx
        .oauth
        .start_custom(start_config)
        .map_err(|e| Error::msg(format!("Plugin OAuth start error: {e}")))?;

    let response = serde_json::json!({
        "flowId": started.flow_id,
        "authorizeUrl": started.authorize_url,
    })
    .to_string();
    write_output(plugin, outputs, &response)
}

fn host_oauth_status(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    can_oauth(&ctx)?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let flow_id = req["flowId"].as_str().unwrap_or_default();

    let status = block_on_oauth_status(Arc::clone(&ctx.oauth), flow_id)
        .map_err(|e| Error::msg(format!("Plugin OAuth status error: {e}")))?;

    let response = serde_json::json!({
        "providerId": status.provider_id,
        "status": status.status.as_str(),
        "accessToken": status.access_token,
        "refreshToken": status.refresh_token,
        "expiresAt": status.expires_at,
        "error": status.error,
    })
    .to_string();
    write_output(plugin, outputs, &response)
}

fn host_oauth_cancel(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    can_oauth(&ctx)?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let flow_id = req["flowId"].as_str().unwrap_or_default();
    let cancelled = ctx
        .oauth
        .cancel(flow_id)
        .map_err(|e| Error::msg(format!("Plugin OAuth cancel error: {e}")))?;

    write_output(
        plugin,
        outputs,
        &serde_json::json!({ "cancelled": cancelled }).to_string(),
    )
}

fn host_secret_get(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    can_secret_read(&ctx)?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let key = plugin_secret_key(&ctx.plugin_key, req["key"].as_str().unwrap_or_default());

    let value = match ctx.secret_store.get(&key) {
        Ok(value) => Some(value),
        Err(SecretStoreError::NotFound) => None,
        Err(err) => return Err(Error::msg(format!("Plugin secret get error: {err}"))),
    };

    write_output(
        plugin,
        outputs,
        &serde_json::json!({ "value": value }).to_string(),
    )
}

fn host_secret_set(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    can_secret_write(&ctx)?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let key = plugin_secret_key(&ctx.plugin_key, req["key"].as_str().unwrap_or_default());
    let value = req["value"].as_str().unwrap_or_default();
    ctx.secret_store
        .put(&key, value)
        .map_err(|e| Error::msg(format!("Plugin secret set error: {e}")))?;

    write_output(plugin, outputs, r#"{"ok":true}"#)
}

fn host_secret_delete(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    can_secret_write(&ctx)?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let key = plugin_secret_key(&ctx.plugin_key, req["key"].as_str().unwrap_or_default());

    match ctx.secret_store.delete(&key) {
        Ok(()) | Err(SecretStoreError::NotFound) => {}
        Err(err) => return Err(Error::msg(format!("Plugin secret delete error: {err}"))),
    }

    write_output(plugin, outputs, r#"{"ok":true}"#)
}

fn host_http_request(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    can_http(&ctx)?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let method = req["method"].as_str().unwrap_or("GET");
    let url = req["url"].as_str().unwrap_or_default();
    if !is_http_url_allowed(url, &ctx.allowed_hosts) {
        return Err(Error::msg(format!(
            "HTTP host is not allowlisted for plugin '{}': {url}",
            ctx.plugin_key
        )));
    }

    let headers = req["headers"].as_array().cloned().unwrap_or_default();
    let body = req["body"].as_str().map(ToString::to_string);
    let response = execute_http_request(method.to_string(), url.to_string(), headers, body)
        .map_err(Error::msg)?;

    write_output(plugin, outputs, &response)
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

// ── Task host functions ──────────────────────────────────────────────

fn host_task_create(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "tasks")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();

    let title = req["title"].as_str().unwrap_or("");
    let priority = req["priority"].as_str().unwrap_or("medium");
    let assignee = req["assignee"].as_str().unwrap_or("user");
    let labels: Vec<String> = req["labels"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let description = req["description"].as_str();
    let scheduled_start_at = req["scheduled_start_at"].as_str();
    let scheduled_end_at = req["scheduled_end_at"].as_str();
    let estimated_duration_min = req["estimated_duration_min"].as_u64().map(|v| v as u32);
    let recurrence_rule = req["recurrence_rule"].as_str();
    let recurrence_time_of_day = req["recurrence_time_of_day"].as_str();

    match ctx.task_service.create_task(
        title,
        priority,
        assignee,
        &labels,
        description,
        scheduled_start_at,
        scheduled_end_at,
        estimated_duration_min,
        recurrence_rule,
        recurrence_time_of_day,
    ) {
        Ok(dto) => write_output(
            plugin,
            outputs,
            &serde_json::to_string(&dto).unwrap_or_default(),
        ),
        Err(e) => Err(Error::msg(format!("Task create error: {e}"))),
    }
}

fn host_task_list(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "tasks")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();

    match ctx.task_service.list_tasks() {
        Ok(tasks) => {
            let filtered: Vec<_> = match req["status_filter"].as_str() {
                Some(status) => tasks.into_iter().filter(|t| t.status == status).collect(),
                None => tasks,
            };
            write_output(
                plugin,
                outputs,
                &serde_json::to_string(&filtered).unwrap_or_default(),
            )
        }
        Err(e) => Err(Error::msg(format!("Task list error: {e}"))),
    }
}

fn host_task_update(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "tasks")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();

    let id = req["id"].as_str().unwrap_or("");
    let title = req["title"].as_str();
    let priority = req["priority"].as_str();
    let status = req["status"].as_str();
    let assignee = req["assignee"].as_str();
    let labels: Option<Vec<String>> = req["labels"].as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });
    let labels_ref = labels.as_deref();
    let description = req["description"].as_str();
    let scheduled_start_at = req["scheduled_start_at"].as_str();
    let scheduled_end_at = req["scheduled_end_at"].as_str();
    let estimated_duration_min: Option<Option<u32>> = if req.get("estimated_duration_min").is_some()
    {
        Some(req["estimated_duration_min"].as_u64().map(|v| v as u32))
    } else {
        None
    };
    let recurrence_rule: Option<Option<&str>> = if req.get("recurrence_rule").is_some() {
        Some(req["recurrence_rule"].as_str())
    } else {
        None
    };
    let recurrence_time_of_day: Option<Option<&str>> =
        if req.get("recurrence_time_of_day").is_some() {
            Some(req["recurrence_time_of_day"].as_str())
        } else {
            None
        };

    match ctx.task_service.update_task(
        id,
        title,
        priority,
        status,
        assignee,
        labels_ref,
        description,
        scheduled_start_at,
        scheduled_end_at,
        estimated_duration_min,
        recurrence_rule,
        recurrence_time_of_day,
    ) {
        Ok(dto) => write_output(
            plugin,
            outputs,
            &serde_json::to_string(&dto).unwrap_or_default(),
        ),
        Err(e) => Err(Error::msg(format!("Task update error: {e}"))),
    }
}

fn host_task_delete(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "tasks")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();

    let id = req["id"].as_str().unwrap_or("");
    match ctx.task_service.delete_task(id) {
        Ok(()) => write_output(plugin, outputs, r#"{"ok":true}"#),
        Err(e) => Err(Error::msg(format!("Task delete error: {e}"))),
    }
}

fn host_task_toggle(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "tasks")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();

    let id = req["id"].as_str().unwrap_or("");
    match ctx.task_service.toggle_task(id) {
        Ok(dto) => write_output(
            plugin,
            outputs,
            &serde_json::to_string(&dto).unwrap_or_default(),
        ),
        Err(e) => Err(Error::msg(format!("Task toggle error: {e}"))),
    }
}

fn host_task_assign(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "tasks")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();

    let id = req["id"].as_str().unwrap_or("");
    let assignee = req["assignee"].as_str().unwrap_or("user");
    match ctx.task_service.update_task(
        id,
        None,
        None,
        None,
        Some(assignee),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    ) {
        Ok(dto) => write_output(
            plugin,
            outputs,
            &serde_json::to_string(&dto).unwrap_or_default(),
        ),
        Err(e) => Err(Error::msg(format!("Task assign error: {e}"))),
    }
}

fn host_websocket_connect(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "net:websocket")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let url = req["url"].as_str().unwrap_or_default().trim();

    if url.is_empty() {
        return Err(Error::msg("websocket url is required"));
    }

    if !is_websocket_url_allowed(url, &ctx.allowed_hosts) {
        return Err(Error::msg(format!(
            "Plugin '{}' cannot connect to non-allowlisted host: {url}",
            ctx.plugin_key
        )));
    }

    let parsed = Url::parse(url).map_err(|e| Error::msg(format!("Invalid websocket url: {e}")))?;
    match parsed.scheme() {
        "ws" | "wss" => {}
        scheme => {
            return Err(Error::msg(format!(
                "Unsupported websocket scheme: {scheme}"
            )));
        }
    }

    // Build request with a valid HTTP Origin header so servers that validate
    // Origin (e.g. OpenClaw gateway) don't reject the handshake.
    // Include the port when non-default so the origin matches what the server expects.
    let host = parsed.host_str().ok_or_else(|| Error::msg("WebSocket URL missing host"))?;
    let origin = if let Some(port) = parsed.port() {
        if host.contains(':') {
            format!("http://[{}]:{}", host, port)
        } else {
            format!("http://{}:{}", host, port)
        }
    } else {
        format!("http://{}", host)
    };

    tracing::debug!("WebSocket connection origin: {}", origin);
    let mut request = url.into_client_request().map_err(|e| Error::msg(e.to_string()))?;
    request.headers_mut().insert(
        "Origin",
        origin
            .parse()
            .map_err(|e| Error::msg(format!("Invalid origin header '{}': {}", origin, e)))?,
    );
    let (socket, _) = connect(request)
        .map_err(|e| Error::msg(format!("WebSocket connect error: {e}")))?;
    let mut websockets = ctx
        .websockets
        .lock()
        .map_err(|e| Error::msg(format!("{e}")))?;
    websockets.next_id += 1;
    let socket_id = format!("ws-{}", websockets.next_id);
    websockets.sockets.insert(socket_id.clone(), socket);

    write_output(
        plugin,
        outputs,
        &serde_json::json!({ "socketId": socket_id }).to_string(),
    )?;
    Ok(())
}

fn host_websocket_send(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "net:websocket")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let socket_id = req["socketId"].as_str().unwrap_or_default();
    let text = req["text"].as_str().unwrap_or_default();

    let mut websockets = ctx
        .websockets
        .lock()
        .map_err(|e| Error::msg(format!("{e}")))?;
    let socket = websockets
        .sockets
        .get_mut(socket_id)
        .ok_or_else(|| Error::msg(format!("Unknown websocket socketId: {socket_id}")))?;
    socket
        .send(Message::Text(text.to_string().into()))
        .map_err(|e| Error::msg(format!("WebSocket send error: {e}")))?;

    write_output(plugin, outputs, r#"{"ok":true}"#)?;
    Ok(())
}

fn host_websocket_recv(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "net:websocket")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let socket_id = req["socketId"].as_str().unwrap_or_default();

    let mut websockets = ctx
        .websockets
        .lock()
        .map_err(|e| Error::msg(format!("{e}")))?;
    let socket = websockets
        .sockets
        .get_mut(socket_id)
        .ok_or_else(|| Error::msg(format!("Unknown websocket socketId: {socket_id}")))?;

    loop {
        match socket.read() {
            Ok(Message::Text(text)) => {
                write_output(
                    plugin,
                    outputs,
                    &serde_json::json!({ "text": text.to_string() }).to_string(),
                )?;
                return Ok(());
            }
            Ok(Message::Binary(bytes)) => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                write_output(
                    plugin,
                    outputs,
                    &serde_json::json!({ "text": text }).to_string(),
                )?;
                return Ok(());
            }
            Ok(Message::Ping(payload)) => {
                socket
                    .send(Message::Pong(payload))
                    .map_err(|e| Error::msg(format!("WebSocket pong error: {e}")))?;
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Frame(_)) => {}
            Ok(Message::Close(_)) => {
                websockets.sockets.remove(socket_id);
                return Err(Error::msg("WebSocket closed by remote peer"));
            }
            Err(e) => {
                websockets.sockets.remove(socket_id);
                return Err(Error::msg(format!("WebSocket receive error: {e}")));
            }
        }
    }
}

fn host_websocket_close(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "net:websocket")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let socket_id = req["socketId"].as_str().unwrap_or_default();

    let mut websockets = ctx
        .websockets
        .lock()
        .map_err(|e| Error::msg(format!("{e}")))?;
    if let Some(mut socket) = websockets.sockets.remove(socket_id) {
        let _ = socket.close(None);
    }

    write_output(plugin, outputs, r#"{"ok":true}"#)?;
    Ok(())
}

fn host_system_time_millis(
    plugin: &mut CurrentPlugin,
    _inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let time_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    write_output(
        plugin,
        outputs,
        &serde_json::json!({ "timeMillis": time_millis }).to_string(),
    )
}

fn host_system_uuid_v4(
    plugin: &mut CurrentPlugin,
    _inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData<HostContext>,
) -> Result<(), Error> {
    write_output(
        plugin,
        outputs,
        &serde_json::json!({ "uuid": uuid::Uuid::new_v4().to_string() }).to_string(),
    )
}

fn host_crypto_ed25519_get_or_create(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "crypto:ed25519")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let alias = req["alias"].as_str().unwrap_or_default();

    if alias.trim().is_empty() {
        return Err(Error::msg("crypto key alias is required"));
    }

    let key = load_or_create_signing_key(&ctx.plugin_key, alias)
        .map_err(|e| Error::msg(e.to_string()))?;
    let public_key = key.verifying_key().to_bytes();
    let public_key_base64url = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public_key);
    let public_key_sha256_hex = hex::encode(Sha256::digest(public_key));

    write_output(
        plugin,
        outputs,
        &serde_json::json!({
            "publicKeyBase64Url": public_key_base64url,
            "publicKeySha256Hex": public_key_sha256_hex,
        })
        .to_string(),
    )
}

fn host_crypto_ed25519_sign(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostContext>,
) -> Result<(), Error> {
    let ctx = user_data.get().map_err(|e| Error::msg(format!("{e}")))?;
    let ctx = ctx.lock().map_err(|e| Error::msg(format!("{e}")))?;
    require_capability(&ctx, "crypto:ed25519")?;
    let input_str = read_input(plugin, inputs)?;
    let req: serde_json::Value = serde_json::from_str(&input_str).unwrap_or_default();
    let alias = req["alias"].as_str().unwrap_or_default();
    let payload = req["payload"].as_str().unwrap_or_default();

    if alias.trim().is_empty() {
        return Err(Error::msg("crypto key alias is required"));
    }

    let key = load_or_create_signing_key(&ctx.plugin_key, alias)
        .map_err(|e| Error::msg(e.to_string()))?;
    let signature = ed25519_dalek::Signer::sign(&key, payload.as_bytes());
    let signature_base64url =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes());

    write_output(
        plugin,
        outputs,
        &serde_json::json!({ "signatureBase64Url": signature_base64url }).to_string(),
    )
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

fn can_log(_ctx: &HostContext) -> Result<(), Error> {
    Ok(())
}

fn can_emit_events(_ctx: &HostContext) -> Result<(), Error> {
    Ok(())
}

fn can_notify(ctx: &HostContext) -> Result<(), Error> {
    require_capability(ctx, "notifications")
}

fn can_schedule(ctx: &HostContext) -> Result<(), Error> {
    require_capability(ctx, "scheduler")
}

fn can_oauth(ctx: &HostContext) -> Result<(), Error> {
    require_capability(ctx, "oauth")
}

fn can_secret_read(ctx: &HostContext) -> Result<(), Error> {
    require_capability(ctx, "secrets:read")
}

fn can_secret_write(ctx: &HostContext) -> Result<(), Error> {
    require_capability(ctx, "secrets:write")
}

fn can_http(ctx: &HostContext) -> Result<(), Error> {
    require_capability(ctx, "net:http")
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

fn is_websocket_url_allowed(url: &str, allowed_hosts: &[String]) -> bool {
    is_network_url_allowed(url, allowed_hosts)
}

fn is_http_url_allowed(url: &str, allowed_hosts: &[String]) -> bool {
    is_network_url_allowed(url, allowed_hosts)
}

fn is_network_url_allowed(url: &str, allowed_hosts: &[String]) -> bool {
    if allowed_hosts.is_empty() {
        return false;
    }

    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    let port = parsed.port_or_known_default();

    allowed_hosts
        .iter()
        .any(|allowed| host_matches_rule(host, port, allowed))
}

fn plugin_secret_key(plugin_key: &str, key: &str) -> String {
    format!("plugin/{plugin_key}/{key}")
}

fn json_params_to_query_params(value: &serde_json::Value) -> Vec<OAuthQueryParam> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            Some(OAuthQueryParam::new(
                entry.get("key")?.as_str()?,
                entry.get("value")?.as_str()?,
            ))
        })
        .collect()
}

fn block_on_oauth_status(
    oauth: Arc<OAuthService>,
    flow_id: &str,
) -> Result<peekoo_agent_auth::OAuthStatusResult, String> {
    let flow_id = flow_id.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let result = match runtime {
            Ok(runtime) => runtime
                .block_on(oauth.status(&flow_id))
                .map_err(|e| e.to_string()),
            Err(err) => Err(format!("Create OAuth runtime error: {err}")),
        };
        let _ = tx.send(result);
    });

    rx.recv()
        .map_err(|err| format!("Receive OAuth status error: {err}"))?
}

fn execute_http_request(
    method: String,
    url: String,
    headers: Vec<serde_json::Value>,
    body: Option<String>,
) -> Result<String, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = (|| {
            let method = Method::from_bytes(method.as_bytes())
                .map_err(|e| format!("Invalid HTTP method: {e}"))?;
            let client = reqwest::blocking::Client::new();
            let mut request = client.request(method, &url);
            for header in headers {
                if let (Some(name), Some(value)) =
                    (header["name"].as_str(), header["value"].as_str())
                {
                    request = request.header(name, value);
                }
            }
            if let Some(body) = body {
                request = request.body(body);
            }

            let response = request
                .send()
                .map_err(|e| format!("HTTP request failed: {e}"))?;
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .filter_map(|(name, value)| {
                    value.to_str().ok().map(|value| {
                        serde_json::json!({
                            "name": name.as_str(),
                            "value": value,
                        })
                    })
                })
                .collect::<Vec<_>>();
            let body = response
                .text()
                .map_err(|e| format!("HTTP response body read failed: {e}"))?;

            Ok::<String, String>(
                serde_json::json!({
                    "status": status,
                    "body": body,
                    "headers": headers,
                })
                .to_string(),
            )
        })();
        let _ = tx.send(result);
    });

    rx.recv()
        .map_err(|err| format!("Receive HTTP response error: {err}"))?
}

fn host_matches_rule(host: &str, port: Option<u16>, rule: &str) -> bool {
    let rule = rule.trim();
    if rule.is_empty() {
        return false;
    }

    if rule == "*" {
        return true;
    }

    if let Some(suffix) = rule.strip_prefix("*.") {
        return host != suffix && host.ends_with(&format!(".{suffix}"));
    }

    match rule.split_once(':') {
        Some((rule_host, rule_port)) => host == rule_host && port == rule_port.parse::<u16>().ok(),
        None => host == rule,
    }
}

fn load_or_create_signing_key(plugin_key: &str, alias: &str) -> Result<SigningKey, String> {
    let path = crypto_key_alias_path(plugin_key, alias)?;
    if path.exists() {
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Read signing key error ({}): {e}", path.display()))?;
        let stored: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Parse signing key error ({}): {e}", path.display()))?;
        let secret_base64 = stored["secretKeyBase64"]
            .as_str()
            .ok_or_else(|| "Missing secretKeyBase64 field".to_string())?;
        let secret = base64::engine::general_purpose::STANDARD
            .decode(secret_base64.as_bytes())
            .map_err(|e| format!("Decode signing key error: {e}"))?;
        let secret_arr: [u8; 32] = secret
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid ed25519 secret key length".to_string())?;
        return Ok(SigningKey::from_bytes(&secret_arr));
    }

    let signing_key = SigningKey::generate(&mut OsRng);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Create signing key directory error: {e}"))?;
    }

    let payload = serde_json::json!({
        "version": 1,
        "algorithm": "ed25519",
        "secretKeyBase64": base64::engine::general_purpose::STANDARD.encode(signing_key.to_bytes()),
    });
    fs::write(
        &path,
        serde_json::to_string_pretty(&payload)
            .map_err(|e| format!("Serialize signing key error: {e}"))?,
    )
    .map_err(|e| format!("Write signing key error ({}): {e}", path.display()))?;

    Ok(signing_key)
}

fn crypto_key_alias_path(plugin_key: &str, alias: &str) -> Result<PathBuf, String> {
    Ok(peekoo_paths::peekoo_global_config_dir()?
        .join("plugins")
        .join(sanitize_key_component(plugin_key))
        .join("keys")
        .join(format!("{}.json", sanitize_key_component(alias))))
}

fn sanitize_key_component(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    sanitized.trim_matches('_').to_string()
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
    use std::sync::{Arc, Mutex};

    use peekoo_agent_auth::OAuthService;
    use peekoo_notifications::{MoodReactionService, NotificationService, PeekBadgeService};
    use peekoo_scheduler::Scheduler;
    use peekoo_security::InMemorySecretStore;
    use peekoo_task_app::{TaskDto, TaskEventDto, TaskService};
    use peekoo_task_domain::TaskStatus;
    use rusqlite::Connection;

    use crate::events::EventBus;
    use crate::permissions::PermissionStore;
    use crate::state::PluginStateStore;

    use super::{
        HostContext, can_emit_events, can_log, can_notify, can_schedule, crypto_key_alias_path,
        is_http_url_allowed, is_path_allowed, is_websocket_url_allowed, plugin_secret_key,
        read_file_content, sanitize_key_component,
    };

    struct NoopTaskService;
    impl TaskService for NoopTaskService {
        fn create_task(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &[String],
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<u32>,
            _: Option<&str>,
            _: Option<&str>,
        ) -> Result<TaskDto, String> {
            Err("noop".into())
        }
        fn list_tasks(&self) -> Result<Vec<TaskDto>, String> {
            Ok(vec![])
        }
        fn update_task(
            &self,
            _: &str,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&[String]>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<&str>,
            _: Option<Option<u32>>,
            _: Option<Option<&str>>,
            _: Option<Option<&str>>,
        ) -> Result<TaskDto, String> {
            Err("noop".into())
        }
        fn delete_task(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn toggle_task(&self, _: &str) -> Result<TaskDto, String> {
            Err("noop".into())
        }
        fn get_task_activity(&self, _: &str, _: u32) -> Result<Vec<TaskEventDto>, String> {
            Ok(vec![])
        }
        fn add_task_comment(&self, _: &str, _: &str, _: &str) -> Result<TaskEventDto, String> {
            Err("noop".into())
        }
        fn claim_task_for_agent(&self, _: &str) -> Result<bool, String> {
            Err("noop".into())
        }
        fn update_agent_work_status(
            &self,
            _: &str,
            _: &str,
            _: Option<&str>,
        ) -> Result<(), String> {
            Err("noop".into())
        }
        fn increment_attempt_count(&self, _: &str) -> Result<u32, String> {
            Err("noop".into())
        }
        fn list_tasks_for_agent_execution(&self) -> Result<Vec<TaskDto>, String> {
            Ok(vec![])
        }
        fn add_task_label(&self, _: &str, _: &str) -> Result<TaskDto, String> {
            Err("noop".into())
        }
        fn remove_task_label(&self, _: &str, _: &str) -> Result<TaskDto, String> {
            Err("noop".into())
        }
        fn update_task_status(&self, _: &str, _: TaskStatus) -> Result<TaskDto, String> {
            Err("noop".into())
        }
        fn load_task(&self, _: &str) -> Result<TaskDto, String> {
            Err("noop".into())
        }
    }

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

    #[test]
    fn websocket_url_is_allowed_for_exact_host_and_port() {
        assert!(is_websocket_url_allowed(
            "ws://127.0.0.1:18789/socket",
            &["127.0.0.1:18789".to_string()],
        ));
    }

    #[test]
    fn websocket_url_is_allowed_for_wildcard_host() {
        assert!(is_websocket_url_allowed(
            "wss://gateway.feedmob.dev/rpc",
            &["*.feedmob.dev".to_string()],
        ));
    }

    #[test]
    fn websocket_url_is_allowed_for_global_wildcard() {
        assert!(is_websocket_url_allowed(
            "wss://anywhere.example.com:444/rpc",
            &["*".to_string()],
        ));
    }

    #[test]
    fn websocket_url_is_rejected_when_host_not_allowlisted() {
        assert!(!is_websocket_url_allowed(
            "ws://evil.example.com/socket",
            &["127.0.0.1:18789".to_string(), "*.feedmob.dev".to_string()],
        ));
    }

    #[test]
    fn http_url_is_allowed_for_exact_host_and_port() {
        assert!(is_http_url_allowed(
            "https://oauth2.googleapis.com/token",
            &["oauth2.googleapis.com".to_string()],
        ));
    }

    #[test]
    fn http_url_is_rejected_when_host_not_allowlisted() {
        assert!(!is_http_url_allowed(
            "https://evil.example.com/token",
            &[
                "oauth2.googleapis.com".to_string(),
                "www.googleapis.com".to_string()
            ],
        ));
    }

    #[test]
    fn sanitize_key_component_replaces_unsafe_characters() {
        assert_eq!(
            sanitize_key_component("openclaw/device identity:v2"),
            "openclaw_device_identity_v2"
        );
    }

    #[test]
    fn crypto_key_alias_path_is_namespaced_by_plugin() {
        let path = crypto_key_alias_path("openclaw-sessions", "device identity:v2")
            .expect("key path resolves");

        let display = path.to_string_lossy();
        assert!(display.contains("openclaw-sessions"));
        assert!(display.contains("device_identity_v2.json"));
    }

    #[test]
    fn plugin_secret_key_is_namespaced_by_plugin() {
        assert_eq!(
            plugin_secret_key("google-calendar", "oauth-token"),
            "plugin/google-calendar/oauth-token"
        );
    }

    fn permission_test_context(
        declared_capabilities: &[&str],
        granted_capabilities: &[&str],
    ) -> HostContext {
        let conn = Connection::open_in_memory().expect("in-memory db");
        conn.execute_batch(
            r#"
            CREATE TABLE plugins (
              id TEXT PRIMARY KEY,
              plugin_key TEXT NOT NULL,
              version TEXT NOT NULL,
              plugin_type TEXT NOT NULL,
              enabled INTEGER NOT NULL DEFAULT 1,
              manifest_json TEXT NOT NULL,
              installed_at TEXT NOT NULL
            );

            CREATE TABLE plugin_permissions (
              id TEXT PRIMARY KEY,
              plugin_id TEXT NOT NULL,
              capability TEXT NOT NULL,
              granted INTEGER NOT NULL
            );

            CREATE TABLE plugin_state (
              id TEXT PRIMARY KEY,
              plugin_id TEXT NOT NULL,
              state_key TEXT NOT NULL,
              value_json TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            INSERT INTO plugins (id, plugin_key, version, plugin_type, enabled, manifest_json, installed_at)
            VALUES ('plugin-1', 'openclaw-sessions', '1.0.0', 'wasm', 1, '{}', datetime('now'));
            "#,
        )
        .expect("plugin schema");

        let conn = Arc::new(Mutex::new(conn));
        let permissions = PermissionStore::new(Arc::clone(&conn));
        for capability in granted_capabilities {
            permissions
                .grant("openclaw-sessions", capability)
                .expect("grant capability");
        }

        let (notifications, _receiver) = NotificationService::new();

        HostContext {
            plugin_key: "openclaw-sessions".to_string(),
            state_store: PluginStateStore::new(conn),
            permissions,
            declared_capabilities: declared_capabilities
                .iter()
                .map(|cap| (*cap).to_string())
                .collect(),
            allowed_paths: vec![],
            allowed_hosts: vec![],
            oauth: Arc::new(OAuthService::new()),
            secret_store: Arc::new(InMemorySecretStore::default()),
            websockets: Arc::new(Mutex::new(Default::default())),
            event_bus: Arc::new(EventBus::new()),
            scheduler: Arc::new(Scheduler::new()),
            notifications: Arc::new(notifications),
            peek_badges: Arc::new(PeekBadgeService::new()),
            mood_reactions: Arc::new(MoodReactionService::new()),
            task_service: Arc::new(NoopTaskService),
            config_fields: vec![],
        }
    }

    #[test]
    fn notify_requires_notifications_permission() {
        let ctx = permission_test_context(&["notifications"], &[]);

        let err = can_notify(&ctx).expect_err("notify should require a granted permission");

        assert!(
            err.to_string()
                .contains("permission 'notifications' is not granted")
        );
    }

    #[test]
    fn schedule_access_requires_scheduler_permission() {
        let ctx = permission_test_context(&["scheduler"], &[]);

        let err =
            can_schedule(&ctx).expect_err("schedule access should require a granted permission");

        assert!(
            err.to_string()
                .contains("permission 'scheduler' is not granted")
        );
    }

    #[test]
    fn log_does_not_require_extra_permissions() {
        let ctx = permission_test_context(&[], &[]);

        can_log(&ctx).expect("logging should not require a declared capability");
    }

    #[test]
    fn emit_event_does_not_require_extra_permissions() {
        let ctx = permission_test_context(&[], &[]);

        can_emit_events(&ctx).expect("event emission should not require a declared capability");
    }
}
