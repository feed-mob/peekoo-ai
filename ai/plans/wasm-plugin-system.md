# WASM Plugin System Design

## Overview

A plugin system for Peekoo AI that uses Extism (WebAssembly) to load, sandbox, and execute plugins. Plugins can extend four integration points: agent tools, UI panels/widgets, event hooks, and data providers. V1 is local-only (no remote plugin store).

---

## 1. WASM Runtime: Extism

### Why Extism
- Mature Rust host SDK (`extism` crate on crates.io)
- Plugins can be authored in Rust, Go, JS, AssemblyScript, C, Zig, etc.
- Built-in sandboxing: memory limits, timeouts, host allowlists
- Host functions let us expose a controlled API surface to plugins
- Handles serialization across the WASM boundary (JSON in/out)

### Dependencies
```toml
# In peekoo-plugin-host/Cargo.toml
extism = "1"
extism-manifest = "1"
```

### Resource Limits (defaults, configurable per-plugin)
| Resource | Default Limit |
|---|---|
| Memory | 256 pages (16 MiB) |
| Execution timeout | 5,000 ms per call |
| HTTP | Disabled (no `allowed_hosts` unless granted) |
| Filesystem | Disabled (no `allowed_paths` unless granted) |

---

## 2. Plugin Manifest

Each plugin ships as a directory with a `peekoo-plugin.toml` manifest and one or more `.wasm` files.

### Directory Layout
```
my-plugin/
  peekoo-plugin.toml     # Manifest
  plugin.wasm            # Compiled WASM module
  ui/                    # Optional: UI assets
    panel.html
    panel.js
    panel.css
```

### Manifest Schema (`peekoo-plugin.toml`)
```toml
[plugin]
key = "health-reminders"              # Unique identifier (kebab-case)
name = "Health Reminders"             # Display name
version = "0.1.0"                     # SemVer
author = "Peekoo Team"
description = "Drink water, eye rest, and stand-up reminders"
min_peekoo_version = "0.1.0"         # Compatibility floor
wasm = "plugin.wasm"                  # Path to WASM module relative to manifest

[permissions]
# Capabilities the plugin requests. User must grant each one.
required = ["timer", "notifications", "state:read", "state:write"]
optional = ["agent:register-tool"]

[tools]
# Tools this plugin exposes to the AI agent
[[tools.definitions]]
name = "health_get_status"
description = "Get current health reminder status (next reminders, stats)"
parameters = '''
{
  "type": "object",
  "properties": {},
  "required": []
}
'''
return_type = "object"

[[tools.definitions]]
name = "health_configure"
description = "Configure health reminder intervals"
parameters = '''
{
  "type": "object",
  "properties": {
    "water_interval_min": { "type": "integer", "description": "Minutes between water reminders" },
    "eye_rest_interval_min": { "type": "integer", "description": "Minutes between eye rest reminders" },
    "standup_interval_min": { "type": "integer", "description": "Minutes between stand-up reminders" }
  }
}
'''
return_type = "object"

[events]
# Events this plugin subscribes to
subscribe = ["timer:tick", "pomodoro:finished", "app:focus-changed"]
# Events this plugin may emit
emit = ["health:reminder-due", "health:reminder-dismissed"]

[data]
# Data schemas this plugin provides
[[data.providers]]
name = "health_reminder_status"
description = "Current state of all health reminders"
schema = '''
{
  "type": "object",
  "properties": {
    "water": { "$ref": "#/definitions/ReminderState" },
    "eye_rest": { "$ref": "#/definitions/ReminderState" },
    "standup": { "$ref": "#/definitions/ReminderState" }
  }
}
'''

[ui]
# UI panels/widgets this plugin provides
[[ui.panels]]
label = "panel-health"
title = "Health Reminders"
width = 320
height = 400
entry = "ui/panel.html"              # Relative to plugin dir
```

### Rust Types for Manifest

```rust
// crates/peekoo-plugin-host/src/manifest.rs

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMeta,
    pub permissions: Option<PermissionsBlock>,
    pub tools: Option<ToolsBlock>,
    pub events: Option<EventsBlock>,
    pub data: Option<DataBlock>,
    pub ui: Option<UiBlock>,
}

#[derive(Debug, Deserialize)]
pub struct PluginMeta {
    pub key: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub min_peekoo_version: Option<String>,
    pub wasm: String,
}

#[derive(Debug, Deserialize)]
pub struct PermissionsBlock {
    pub required: Vec<String>,
    #[serde(default)]
    pub optional: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToolsBlock {
    pub definitions: Vec<ToolDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    /// JSON Schema string for parameters
    pub parameters: String,
    pub return_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EventsBlock {
    #[serde(default)]
    pub subscribe: Vec<String>,
    #[serde(default)]
    pub emit: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DataBlock {
    pub providers: Vec<DataProviderDef>,
}

#[derive(Debug, Deserialize)]
pub struct DataProviderDef {
    pub name: String,
    pub description: String,
    /// JSON Schema string
    pub schema: String,
}

#[derive(Debug, Deserialize)]
pub struct UiBlock {
    pub panels: Vec<UiPanelDef>,
}

#[derive(Debug, Deserialize)]
pub struct UiPanelDef {
    pub label: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
    /// Relative path to HTML entry point within plugin directory
    pub entry: String,
}
```

---

## 3. Plugin Host Crate (`crates/peekoo-plugin-host/`)

### Crate Structure
```
crates/peekoo-plugin-host/
  Cargo.toml
  src/
    lib.rs              # Public API re-exports
    manifest.rs         # Manifest parsing (PluginManifest)
    runtime.rs          # WASM runtime wrapper (Extism Plugin lifecycle)
    registry.rs         # Plugin registry (discover, load, track)
    permissions.rs      # Permission model and enforcement
    host_functions.rs   # Host functions exposed to WASM plugins
    events.rs           # Event bus for plugin <-> host communication
    state.rs            # Plugin state persistence (KV store)
    tools.rs            # Agent tool bridge
    error.rs            # Error types
```

### Cargo.toml
```toml
[package]
name = "peekoo-plugin-host"
version = "0.1.0"
edition = "2024"

[dependencies]
extism = "1"
extism-manifest = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "1"
uuid = { version = "1", features = ["v4"] }
tracing = "0.1"
rusqlite = { version = "0.38" }
peekoo-persistence-sqlite = { path = "../persistence-sqlite" }
```

### Plugin Lifecycle

```
discover -> load -> validate -> initialize -> run -> unload
```

#### Lifecycle Detail

1. **Discover**: Scan plugin directories for `peekoo-plugin.toml` files
2. **Load**: Parse manifest, validate fields, check version compatibility
3. **Validate**: Verify WASM module exports required functions, check permissions
4. **Initialize**: Create Extism `Plugin` instance with host functions, call `plugin_init()` export
5. **Run**: Plugin is active; host dispatches tool calls, events, data queries
6. **Unload**: Call `plugin_shutdown()` export (if present), drop Extism Plugin instance

### Core Types

```rust
// crates/peekoo-plugin-host/src/runtime.rs

use extism::{Manifest as ExtismManifest, Plugin, Function};
use std::path::PathBuf;
use std::time::Duration;

use crate::manifest::PluginManifest;
use crate::error::PluginError;

/// A loaded, running plugin instance.
pub struct PluginInstance {
    /// Parsed manifest
    pub manifest: PluginManifest,
    /// Extism plugin handle
    plugin: Plugin,
    /// Directory containing the plugin files
    pub plugin_dir: PathBuf,
    /// Whether the plugin has been initialized
    initialized: bool,
}

impl PluginInstance {
    pub fn load(
        manifest: PluginManifest,
        plugin_dir: PathBuf,
        host_functions: Vec<Function>,
        memory_max_pages: u32,
        timeout: Duration,
    ) -> Result<Self, PluginError> {
        let wasm_path = plugin_dir.join(&manifest.plugin.wasm);

        let extism_manifest = ExtismManifest::new([extism::Wasm::file(&wasm_path)])
            .with_memory_max(memory_max_pages)
            .with_timeout(timeout);

        let plugin = Plugin::new(&extism_manifest, host_functions, true)?;

        Ok(Self {
            manifest,
            plugin,
            plugin_dir,
            initialized: false,
        })
    }

    pub fn initialize(&mut self) -> Result<(), PluginError> {
        if self.plugin.function_exists("plugin_init") {
            let result: String = self.plugin.call("plugin_init", "")?;
            tracing::info!(
                plugin = %self.manifest.plugin.key,
                "Plugin initialized: {result}"
            );
        }
        self.initialized = true;
        Ok(())
    }

    pub fn call_tool(&mut self, tool_name: &str, input_json: &str) -> Result<String, PluginError> {
        if !self.initialized {
            return Err(PluginError::NotInitialized(self.manifest.plugin.key.clone()));
        }
        // Convention: tool functions are exported as "tool_{name}"
        let export_name = format!("tool_{tool_name}");
        if !self.plugin.function_exists(&export_name) {
            return Err(PluginError::ToolNotFound(tool_name.to_string()));
        }
        let result: String = self.plugin.call(&export_name, input_json)?;
        Ok(result)
    }

    pub fn handle_event(&mut self, event_name: &str, payload_json: &str) -> Result<(), PluginError> {
        if !self.initialized {
            return Err(PluginError::NotInitialized(self.manifest.plugin.key.clone()));
        }
        // Convention: event handler is exported as "on_event"
        if self.plugin.function_exists("on_event") {
            let input = serde_json::json!({
                "event": event_name,
                "payload": serde_json::from_str::<serde_json::Value>(payload_json)
                    .unwrap_or(serde_json::Value::Null)
            });
            let _: String = self.plugin.call("on_event", &input.to_string())?;
        }
        Ok(())
    }

    pub fn query_data(&mut self, provider_name: &str) -> Result<String, PluginError> {
        if !self.initialized {
            return Err(PluginError::NotInitialized(self.manifest.plugin.key.clone()));
        }
        let export_name = format!("data_{provider_name}");
        if !self.plugin.function_exists(&export_name) {
            return Err(PluginError::DataProviderNotFound(provider_name.to_string()));
        }
        let result: String = self.plugin.call(&export_name, "")?;
        Ok(result)
    }

    pub fn shutdown(&mut self) {
        if self.initialized && self.plugin.function_exists("plugin_shutdown") {
            let _ = self.plugin.call::<&str, String>("plugin_shutdown", "");
        }
        self.initialized = false;
    }
}

impl Drop for PluginInstance {
    fn drop(&mut self) {
        self.shutdown();
    }
}
```

### Plugin Registry

```rust
// crates/peekoo-plugin-host/src/registry.rs

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use crate::error::PluginError;
use crate::host_functions;
use crate::manifest::PluginManifest;
use crate::permissions::PermissionStore;
use crate::runtime::PluginInstance;
use crate::state::PluginStateStore;

/// Where to find plugins on disk.
const DEFAULT_MEMORY_MAX_PAGES: u32 = 256;  // 16 MiB
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(5000);

pub struct PluginRegistry {
    /// plugin_key -> PluginInstance
    plugins: Mutex<HashMap<String, PluginInstance>>,
    /// Directories to scan for plugins
    plugin_dirs: Vec<PathBuf>,
    /// Permission store (backed by SQLite)
    permissions: PermissionStore,
    /// State store (backed by SQLite)
    state: PluginStateStore,
}

impl PluginRegistry {
    pub fn new(
        plugin_dirs: Vec<PathBuf>,
        permissions: PermissionStore,
        state: PluginStateStore,
    ) -> Self {
        Self {
            plugins: Mutex::new(HashMap::new()),
            plugin_dirs,
            permissions,
            state,
        }
    }

    /// Scan all plugin directories and return discovered manifests.
    pub fn discover(&self) -> Vec<(PathBuf, PluginManifest)> {
        let mut found = Vec::new();
        for dir in &self.plugin_dirs {
            if !dir.is_dir() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let plugin_dir = entry.path();
                    let manifest_path = plugin_dir.join("peekoo-plugin.toml");
                    if manifest_path.is_file() {
                        match load_manifest(&manifest_path) {
                            Ok(manifest) => found.push((plugin_dir, manifest)),
                            Err(e) => {
                                tracing::warn!(
                                    path = %manifest_path.display(),
                                    "Failed to parse plugin manifest: {e}"
                                );
                            }
                        }
                    }
                }
            }
        }
        found
    }

    /// Load and initialize a plugin from a directory.
    pub fn load_plugin(&self, plugin_dir: &Path) -> Result<String, PluginError> {
        let manifest_path = plugin_dir.join("peekoo-plugin.toml");
        let manifest = load_manifest(&manifest_path)?;
        let key = manifest.plugin.key.clone();

        // Check required permissions are granted
        self.permissions.check_required(&key, &manifest)?;

        // Build host functions
        let host_fns = host_functions::build_host_functions(
            &key,
            &self.state,
            &self.permissions,
        );

        let mut instance = PluginInstance::load(
            manifest,
            plugin_dir.to_path_buf(),
            host_fns,
            DEFAULT_MEMORY_MAX_PAGES,
            DEFAULT_TIMEOUT,
        )?;

        instance.initialize()?;

        let mut plugins = self.plugins.lock()
            .map_err(|e| PluginError::Internal(format!("Lock error: {e}")))?;
        plugins.insert(key.clone(), instance);

        Ok(key)
    }

    /// Unload a plugin by key.
    pub fn unload_plugin(&self, key: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.lock()
            .map_err(|e| PluginError::Internal(format!("Lock error: {e}")))?;
        if plugins.remove(key).is_some() {
            tracing::info!(plugin = key, "Plugin unloaded");
            Ok(())
        } else {
            Err(PluginError::NotFound(key.to_string()))
        }
    }

    /// Call a tool on a specific plugin.
    pub fn call_tool(
        &self,
        plugin_key: &str,
        tool_name: &str,
        input_json: &str,
    ) -> Result<String, PluginError> {
        let mut plugins = self.plugins.lock()
            .map_err(|e| PluginError::Internal(format!("Lock error: {e}")))?;
        let instance = plugins.get_mut(plugin_key)
            .ok_or_else(|| PluginError::NotFound(plugin_key.to_string()))?;
        instance.call_tool(tool_name, input_json)
    }

    /// Dispatch an event to all plugins that subscribe to it.
    pub fn dispatch_event(&self, event_name: &str, payload_json: &str) {
        let mut plugins = match self.plugins.lock() {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Lock error dispatching event: {e}");
                return;
            }
        };
        for (key, instance) in plugins.iter_mut() {
            let subscribes = instance.manifest.events.as_ref()
                .map(|e| e.subscribe.iter().any(|s| s == event_name))
                .unwrap_or(false);
            if subscribes {
                if let Err(e) = instance.handle_event(event_name, payload_json) {
                    tracing::warn!(
                        plugin = key.as_str(),
                        event = event_name,
                        "Event handler error: {e}"
                    );
                }
            }
        }
    }

    /// Query a data provider from a specific plugin.
    pub fn query_data(
        &self,
        plugin_key: &str,
        provider_name: &str,
    ) -> Result<String, PluginError> {
        let mut plugins = self.plugins.lock()
            .map_err(|e| PluginError::Internal(format!("Lock error: {e}")))?;
        let instance = plugins.get_mut(plugin_key)
            .ok_or_else(|| PluginError::NotFound(plugin_key.to_string()))?;
        instance.query_data(provider_name)
    }

    /// Return a list of all tool definitions across all loaded plugins.
    pub fn all_tool_definitions(&self) -> Vec<(String, crate::manifest::ToolDefinition)> {
        let plugins = match self.plugins.lock() {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut tools = Vec::new();
        for (key, instance) in plugins.iter() {
            if let Some(tools_block) = &instance.manifest.tools {
                for def in &tools_block.definitions {
                    tools.push((key.clone(), def.clone()));
                }
            }
        }
        tools
    }

    /// Return a list of all UI panel definitions across all loaded plugins.
    pub fn all_ui_panels(&self) -> Vec<(String, crate::manifest::UiPanelDef)> {
        let plugins = match self.plugins.lock() {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut panels = Vec::new();
        for (key, instance) in plugins.iter() {
            if let Some(ui_block) = &instance.manifest.ui {
                for panel in &ui_block.panels {
                    panels.push((key.clone(), panel.clone()));
                }
            }
        }
        panels
    }
}

fn load_manifest(path: &Path) -> Result<PluginManifest, PluginError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PluginError::Io(e.to_string()))?;
    let manifest: PluginManifest = toml::from_str(&content)
        .map_err(|e| PluginError::ManifestParse(e.to_string()))?;
    Ok(manifest)
}
```

### Error Types

```rust
// crates/peekoo-plugin-host/src/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin not initialized: {0}")]
    NotInitialized(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Data provider not found: {0}")]
    DataProviderNotFound(String),

    #[error("Manifest parse error: {0}")]
    ManifestParse(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("WASM runtime error: {0}")]
    Runtime(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<extism::Error> for PluginError {
    fn from(e: extism::Error) -> Self {
        PluginError::Runtime(e.to_string())
    }
}
```

---

## 4. Host Functions Exposed to Plugins

Plugins run in a sandbox. They interact with Peekoo through a narrow set of host functions injected into the WASM runtime.

```rust
// crates/peekoo-plugin-host/src/host_functions.rs

use extism::{host_fn, CurrentPlugin, Function, UserData, ValType};
use crate::state::PluginStateStore;
use crate::permissions::PermissionStore;

/// Build the set of host functions available to a plugin.
pub fn build_host_functions(
    plugin_key: &str,
    state_store: &PluginStateStore,
    _permissions: &PermissionStore,
) -> Vec<Function> {
    let ctx = HostContext {
        plugin_key: plugin_key.to_string(),
        state_store: state_store.clone(),
    };

    vec![
        // State: key-value store
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
        // Logging
        Function::new(
            "peekoo_log",
            [ValType::I64],
            [],
            UserData::new(ctx.clone()),
            host_log,
        ),
        // Emit event
        Function::new(
            "peekoo_emit_event",
            [ValType::I64],
            [],
            UserData::new(ctx.clone()),
            host_emit_event,
        ),
        // Send notification to user
        Function::new(
            "peekoo_notify",
            [ValType::I64],
            [],
            UserData::new(ctx.clone()),
            host_notify,
        ),
    ]
}

#[derive(Clone)]
struct HostContext {
    plugin_key: String,
    state_store: PluginStateStore,
}

// --- Host function implementations ---

/// Read a key from the plugin's KV store.
/// Input JSON: { "key": "some_key" }
/// Output JSON: { "value": <json_value> } or { "value": null }
fn host_state_get(
    plugin: &mut CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<HostContext>,
) {
    let ctx = user_data.get().unwrap();
    let input = plugin.memory_get_val(&inputs[0]).unwrap_or_default();
    let req: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let key = req["key"].as_str().unwrap_or("");

    let value = ctx.state_store
        .get(&ctx.plugin_key, key)
        .unwrap_or(serde_json::Value::Null);

    let response = serde_json::json!({ "value": value }).to_string();
    let offset = plugin.memory_alloc_and_set(&response).unwrap();
    outputs[0] = extism::Val::I64(offset as i64);
}

/// Write a key to the plugin's KV store.
/// Input JSON: { "key": "some_key", "value": <json_value> }
/// Output JSON: { "ok": true }
fn host_state_set(
    plugin: &mut CurrentPlugin,
    inputs: &[extism::Val],
    outputs: &mut [extism::Val],
    user_data: UserData<HostContext>,
) {
    let ctx = user_data.get().unwrap();
    let input = plugin.memory_get_val(&inputs[0]).unwrap_or_default();
    let req: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let key = req["key"].as_str().unwrap_or("");
    let value = &req["value"];

    let ok = ctx.state_store.set(&ctx.plugin_key, key, value).is_ok();

    let response = serde_json::json!({ "ok": ok }).to_string();
    let offset = plugin.memory_alloc_and_set(&response).unwrap();
    outputs[0] = extism::Val::I64(offset as i64);
}

/// Log a message from the plugin.
/// Input JSON: { "level": "info", "message": "..." }
fn host_log(
    plugin: &mut CurrentPlugin,
    inputs: &[extism::Val],
    _outputs: &mut [extism::Val],
    user_data: UserData<HostContext>,
) {
    let ctx = user_data.get().unwrap();
    let input = plugin.memory_get_val(&inputs[0]).unwrap_or_default();
    let req: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let level = req["level"].as_str().unwrap_or("info");
    let message = req["message"].as_str().unwrap_or("");

    match level {
        "error" => tracing::error!(plugin = ctx.plugin_key.as_str(), "{message}"),
        "warn" => tracing::warn!(plugin = ctx.plugin_key.as_str(), "{message}"),
        "debug" => tracing::debug!(plugin = ctx.plugin_key.as_str(), "{message}"),
        _ => tracing::info!(plugin = ctx.plugin_key.as_str(), "{message}"),
    }
}

/// Emit a Peekoo event from the plugin.
/// Input JSON: { "event": "health:reminder-due", "payload": { ... } }
fn host_emit_event(
    plugin: &mut CurrentPlugin,
    inputs: &[extism::Val],
    _outputs: &mut [extism::Val],
    user_data: UserData<HostContext>,
) {
    let ctx = user_data.get().unwrap();
    let input = plugin.memory_get_val(&inputs[0]).unwrap_or_default();
    // Event emission is handled by the registry's event bus (see events.rs).
    // The host function enqueues the event; the registry processes it after
    // the current plugin call returns to avoid re-entrant locking.
    tracing::debug!(
        plugin = ctx.plugin_key.as_str(),
        "Plugin emitted event: {input}"
    );
    // Actual emission is deferred - see Section 6 (Event System).
}

/// Send a desktop notification to the user.
/// Input JSON: { "title": "...", "body": "..." }
fn host_notify(
    plugin: &mut CurrentPlugin,
    inputs: &[extism::Val],
    _outputs: &mut [extism::Val],
    user_data: UserData<HostContext>,
) {
    let ctx = user_data.get().unwrap();
    let input = plugin.memory_get_val(&inputs[0]).unwrap_or_default();
    tracing::info!(
        plugin = ctx.plugin_key.as_str(),
        "Plugin notification request: {input}"
    );
    // Actual notification is delegated to Tauri's notification system
    // via a channel back to the app layer.
}
```

### Host Function Summary

| Host Function | Input | Output | Permission |
|---|---|---|---|
| `peekoo_state_get` | `{ key }` | `{ value }` | `state:read` |
| `peekoo_state_set` | `{ key, value }` | `{ ok }` | `state:write` |
| `peekoo_log` | `{ level, message }` | (none) | Always allowed |
| `peekoo_emit_event` | `{ event, payload }` | (none) | Must match `events.emit` in manifest |
| `peekoo_notify` | `{ title, body }` | (none) | `notifications` |

---

## 5. Permission Model

```rust
// crates/peekoo-plugin-host/src/permissions.rs

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use crate::manifest::PluginManifest;
use crate::error::PluginError;

/// All capabilities a plugin can request.
pub enum Capability {
    Timer,              // Access to timer/tick events
    Notifications,      // Send desktop notifications
    StateRead,          // Read own KV state
    StateWrite,         // Write own KV state
    AgentRegisterTool,  // Register tools with the AI agent
    HttpAccess,         // Make outbound HTTP requests (v2)
    FileRead,           // Read files from allowed paths (v2)
}

impl Capability {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "timer" => Some(Self::Timer),
            "notifications" => Some(Self::Notifications),
            "state:read" => Some(Self::StateRead),
            "state:write" => Some(Self::StateWrite),
            "agent:register-tool" => Some(Self::AgentRegisterTool),
            "http" => Some(Self::HttpAccess),
            "file:read" => Some(Self::FileRead),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct PermissionStore {
    conn: Arc<Mutex<Connection>>,
}

impl PermissionStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Check that all required permissions for a plugin are granted.
    pub fn check_required(
        &self,
        plugin_key: &str,
        manifest: &PluginManifest,
    ) -> Result<(), PluginError> {
        let required = manifest
            .permissions
            .as_ref()
            .map(|p| &p.required)
            .cloned()
            .unwrap_or_default();

        for cap in &required {
            if !self.is_granted(plugin_key, cap)? {
                return Err(PluginError::PermissionDenied(format!(
                    "Plugin '{plugin_key}' requires permission '{cap}' which is not granted"
                )));
            }
        }
        Ok(())
    }

    /// Check if a specific capability is granted.
    pub fn is_granted(&self, plugin_key: &str, capability: &str) -> Result<bool, PluginError> {
        let conn = self.conn.lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT granted FROM plugin_permissions
                 WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)
                 AND capability = ?2"
            )
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        let granted: Option<bool> = stmt
            .query_row(rusqlite::params![plugin_key, capability], |row| row.get(0))
            .ok();

        Ok(granted.unwrap_or(false))
    }

    /// Grant a capability to a plugin.
    pub fn grant(
        &self,
        plugin_key: &str,
        capability: &str,
    ) -> Result<(), PluginError> {
        let conn = self.conn.lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO plugin_permissions (id, plugin_id, capability, granted)
             VALUES (?1, (SELECT id FROM plugins WHERE plugin_key = ?2), ?3, 1)",
            rusqlite::params![uuid::Uuid::new_v4().to_string(), plugin_key, capability],
        )
        .map_err(|e| PluginError::Internal(e.to_string()))?;
        Ok(())
    }

    /// Revoke a capability from a plugin.
    pub fn revoke(
        &self,
        plugin_key: &str,
        capability: &str,
    ) -> Result<(), PluginError> {
        let conn = self.conn.lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        conn.execute(
            "UPDATE plugin_permissions SET granted = 0
             WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)
             AND capability = ?2",
            rusqlite::params![plugin_key, capability],
        )
        .map_err(|e| PluginError::Internal(e.to_string()))?;
        Ok(())
    }
}
```

---

## 6. Event System

Events flow bidirectionally between the host and plugins.

### Event Types (v1)

| Event | Direction | Description |
|---|---|---|
| `timer:tick` | Host -> Plugin | Fires every 60 seconds |
| `pomodoro:started` | Host -> Plugin | Pomodoro session started |
| `pomodoro:finished` | Host -> Plugin | Pomodoro session completed |
| `task:completed` | Host -> Plugin | Task marked as done |
| `agent:message` | Host -> Plugin | Agent produced a message |
| `app:focus-changed` | Host -> Plugin | Window focus gained/lost |
| `health:reminder-due` | Plugin -> Host | A health reminder is due (example) |
| `health:reminder-dismissed` | Plugin -> Host | User dismissed a reminder |

### Event Bus

```rust
// crates/peekoo-plugin-host/src/events.rs

use std::collections::VecDeque;
use std::sync::Mutex;

/// Outbound events emitted by plugins, to be processed by the host.
pub struct EventBus {
    /// Events emitted by plugins during a call, deferred for processing
    outbound_queue: Mutex<VecDeque<PluginEvent>>,
    /// Callback for forwarding plugin events to the Tauri layer
    on_plugin_event: Option<Box<dyn Fn(PluginEvent) + Send + Sync>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginEvent {
    pub source_plugin: String,
    pub event: String,
    pub payload: serde_json::Value,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            outbound_queue: Mutex::new(VecDeque::new()),
            on_plugin_event: None,
        }
    }

    pub fn set_listener(&mut self, listener: impl Fn(PluginEvent) + Send + Sync + 'static) {
        self.on_plugin_event = Some(Box::new(listener));
    }

    /// Called by host functions when a plugin emits an event.
    pub fn enqueue(&self, event: PluginEvent) {
        if let Ok(mut queue) = self.outbound_queue.lock() {
            queue.push_back(event);
        }
    }

    /// Process all queued outbound events. Called after each plugin call returns.
    pub fn drain(&self) -> Vec<PluginEvent> {
        let mut queue = match self.outbound_queue.lock() {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };
        queue.drain(..).collect()
    }
}
```

### Timer Tick Integration

The host runs a background timer that fires `timer:tick` events every 60 seconds. This is the heartbeat that plugins like Health Reminders use to track elapsed time.

```rust
// In PluginRegistry or a dedicated PluginTimer
// Spawned as a tokio/async task or std::thread

pub fn start_tick_timer(registry: Arc<PluginRegistry>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            let payload = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            registry.dispatch_event("timer:tick", &payload.to_string());
        }
    });
}
```

---

## 7. Agent Tool Integration

### How Plugin Tools Reach the Agent

The agent currently uses `pi_agent_rust`'s built-in tools. Plugin-provided tools are injected via pi's custom tool mechanism. The flow:

```
User prompt -> AgentService (pi) -> tool call "health_get_status"
                                        |
                                   pi doesn't know this tool
                                        |
                              PluginToolBridge intercepts
                                        |
                              PluginRegistry.call_tool("health-reminders", "health_get_status", args)
                                        |
                              WASM plugin executes tool_health_get_status()
                                        |
                              JSON result returned to agent
```

### PluginToolBridge

This adapter sits between the agent and the plugin registry. It collects tool definitions from all loaded plugins and provides them to the agent session.

```rust
// crates/peekoo-plugin-host/src/tools.rs

use crate::manifest::ToolDefinition;
use crate::registry::PluginRegistry;
use std::sync::Arc;

/// Describes a plugin-provided tool in a format the agent can consume.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginToolSpec {
    /// Globally unique tool name (plugin_key + tool_name)
    pub name: String,
    pub description: String,
    /// JSON Schema for parameters
    pub parameters_schema: serde_json::Value,
    /// Which plugin owns this tool
    pub plugin_key: String,
}

pub struct PluginToolBridge {
    registry: Arc<PluginRegistry>,
}

impl PluginToolBridge {
    pub fn new(registry: Arc<PluginRegistry>) -> Self {
        Self { registry }
    }

    /// Collect all tool specs from loaded plugins, suitable for injection
    /// into the agent's system prompt as tool definitions.
    pub fn tool_specs(&self) -> Vec<PluginToolSpec> {
        self.registry
            .all_tool_definitions()
            .into_iter()
            .map(|(plugin_key, def)| {
                let params: serde_json::Value =
                    serde_json::from_str(&def.parameters).unwrap_or_default();
                PluginToolSpec {
                    name: def.name.clone(),
                    description: def.description.clone(),
                    parameters_schema: params,
                    plugin_key,
                }
            })
            .collect()
    }

    /// Execute a tool call from the agent.
    pub fn call_tool(&self, tool_name: &str, args_json: &str) -> Result<String, String> {
        // Find which plugin owns this tool
        let tools = self.registry.all_tool_definitions();
        let (plugin_key, _) = tools
            .iter()
            .find(|(_, def)| def.name == tool_name)
            .ok_or_else(|| format!("Plugin tool not found: {tool_name}"))?;

        self.registry
            .call_tool(plugin_key, tool_name, args_json)
            .map_err(|e| e.to_string())
    }

    /// Check if a tool name belongs to a plugin.
    pub fn is_plugin_tool(&self, tool_name: &str) -> bool {
        let tools = self.registry.all_tool_definitions();
        tools.iter().any(|(_, def)| def.name == tool_name)
    }
}
```

### Integration with `peekoo-agent`

The `AgentServiceConfig` needs a new field for plugin tool definitions. These are formatted into the system prompt as additional tool instructions, and tool calls matching plugin tools are intercepted and routed to the `PluginToolBridge`.

```rust
// Addition to AgentServiceConfig
pub struct AgentServiceConfig {
    // ... existing fields ...

    /// Plugin-provided tool definitions to inject into the agent session.
    /// These are formatted into the system prompt and their calls are
    /// routed through the PluginToolBridge.
    pub plugin_tools: Vec<PluginToolSpec>,
}
```

Since `pi_agent_rust` has a fixed tool set (read, write, edit, bash, grep, find, ls), plugin tools are exposed as **skills** - markdown instructions that tell the agent about available plugin tools. When the agent decides to use a plugin tool, it emits a structured output that `peekoo-agent-app` intercepts and routes to the plugin host.

**Alternative (cleaner, depends on pi API)**: If `pi_agent_rust` supports custom tool registration (check `SessionOptions::enabled_tools` or a custom tool callback), register plugin tools directly. This is the preferred path if the API supports it.

---

## 8. Plugin State & Persistence

Uses the existing `plugin_state` table.

```rust
// crates/peekoo-plugin-host/src/state.rs

use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use crate::error::PluginError;

#[derive(Clone)]
pub struct PluginStateStore {
    conn: Arc<Mutex<Connection>>,
}

impl PluginStateStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Get a value from the plugin's KV store.
    pub fn get(&self, plugin_key: &str, key: &str) -> Result<serde_json::Value, PluginError> {
        let conn = self.conn.lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT value_json FROM plugin_state
                 WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)
                 AND state_key = ?2"
            )
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        let value: Option<String> = stmt
            .query_row(rusqlite::params![plugin_key, key], |row| row.get(0))
            .ok();

        match value {
            Some(json_str) => serde_json::from_str(&json_str)
                .map_err(|e| PluginError::Internal(e.to_string())),
            None => Ok(serde_json::Value::Null),
        }
    }

    /// Set a value in the plugin's KV store.
    pub fn set(
        &self,
        plugin_key: &str,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), PluginError> {
        let conn = self.conn.lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        let value_json = serde_json::to_string(value)
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        conn.execute(
            "INSERT INTO plugin_state (id, plugin_id, state_key, value_json, updated_at)
             VALUES (
               ?1,
               (SELECT id FROM plugins WHERE plugin_key = ?2),
               ?3,
               ?4,
               datetime('now')
             )
             ON CONFLICT(plugin_id, state_key) DO UPDATE SET
               value_json = excluded.value_json,
               updated_at = excluded.updated_at",
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                plugin_key,
                key,
                value_json,
            ],
        )
        .map_err(|e| PluginError::Internal(e.to_string()))?;

        Ok(())
    }

    /// Delete a key from the plugin's KV store.
    pub fn delete(&self, plugin_key: &str, key: &str) -> Result<(), PluginError> {
        let conn = self.conn.lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        conn.execute(
            "DELETE FROM plugin_state
             WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)
             AND state_key = ?2",
            rusqlite::params![plugin_key, key],
        )
        .map_err(|e| PluginError::Internal(e.to_string()))?;
        Ok(())
    }

    /// List all keys for a plugin.
    pub fn list_keys(&self, plugin_key: &str) -> Result<Vec<String>, PluginError> {
        let conn = self.conn.lock()
            .map_err(|e| PluginError::Internal(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT state_key FROM plugin_state
                 WHERE plugin_id = (SELECT id FROM plugins WHERE plugin_key = ?1)"
            )
            .map_err(|e| PluginError::Internal(e.to_string()))?;

        let keys = stmt
            .query_map(rusqlite::params![plugin_key], |row| row.get::<_, String>(0))
            .map_err(|e| PluginError::Internal(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(keys)
    }
}
```

---

## 9. UI Integration

### Strategy: Host-rendered panels from plugin-declared data

For v1, plugins declare UI panels in their manifest. The host creates Tauri `WebviewWindow` instances that load plugin-provided HTML/JS/CSS from the plugin directory. Plugin UI communicates with its WASM backend through Tauri commands that proxy to the plugin host.

### How It Works

1. Plugin declares a panel in `peekoo-plugin.toml` under `[ui]`
2. On load, the host registers the panel definition with the frontend
3. Frontend can open plugin panels via the existing `usePanelWindows` pattern
4. Plugin panel HTML is served from the plugin directory via a custom Tauri protocol
5. Plugin UI calls `window.__peekoo.callPlugin(toolName, args)` -> Tauri command -> PluginRegistry -> WASM

### Custom Protocol for Plugin Assets

```rust
// In desktop-tauri/src-tauri/src/lib.rs, register a custom protocol:

tauri::Builder::default()
    .register_asynchronous_uri_scheme_protocol("peekoo-plugin", move |_ctx, request, responder| {
        // URL: peekoo-plugin://plugin-key/path/to/asset
        // Resolve to: {plugin_dir}/path/to/asset
        // Return the file contents with appropriate MIME type
    })
```

### Frontend Changes

```typescript
// apps/desktop-ui/src/types/window.ts - additions

export const PluginPanelConfigSchema = z.object({
  pluginKey: z.string(),
  label: z.string(),
  title: z.string(),
  width: z.number(),
  height: z.number(),
  entry: z.string(), // URL via custom protocol
});
export type PluginPanelConfig = z.infer<typeof PluginPanelConfigSchema>;

// Dynamic panel labels from plugins get the prefix "plugin-"
// e.g., "plugin-health" for the health reminders panel
```

```typescript
// apps/desktop-ui/src/hooks/use-plugin-panels.ts

import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { PluginPanelConfig } from "@/types/window";

export function usePluginPanels() {
  const [pluginPanels, setPluginPanels] = useState<PluginPanelConfig[]>([]);

  useEffect(() => {
    invoke<PluginPanelConfig[]>("plugin_list_panels")
      .then(setPluginPanels)
      .catch(console.error);
  }, []);

  const openPluginPanel = useCallback(async (config: PluginPanelConfig) => {
    const label = `plugin-${config.pluginKey}-${config.label}`;
    const existing = await WebviewWindow.getByLabel(label);
    if (existing) {
      await existing.setFocus();
      return;
    }

    new WebviewWindow(label, {
      url: `peekoo-plugin://${config.pluginKey}/${config.entry}`,
      title: config.title,
      width: config.width,
      height: config.height,
      decorations: false,
      transparent: true,
      alwaysOnTop: true,
      skipTaskbar: true,
      resizable: false,
    });
  }, []);

  return { pluginPanels, openPluginPanel };
}
```

---

## 10. Data Provider Integration

Plugins can expose queryable data. The agent or other parts of the system query plugin data through the registry.

### Flow
```
Agent: "What's my health reminder status?"
  -> AgentService recognizes plugin data query
  -> PluginRegistry.query_data("health-reminders", "health_reminder_status")
  -> WASM plugin executes data_health_reminder_status()
  -> Returns JSON matching declared schema
  -> Agent formats response for user
```

Data providers are exposed to the agent the same way as tools - via system prompt injection describing available data sources and the `query_plugin_data` tool.

---

## 11. Integration with `peekoo-agent-app`

The `AgentApplication` gains a `PluginRegistry` field. Plugin lifecycle is managed alongside the existing agent and productivity services.

```rust
// Modified: crates/peekoo-agent-app/src/application.rs

use peekoo_plugin_host::registry::PluginRegistry;
use peekoo_plugin_host::tools::PluginToolBridge;
use std::sync::Arc;

pub struct AgentApplication {
    agent: Mutex<Option<AgentService>>,
    settings: SettingsService,
    productivity: ProductivityService,
    agent_config_version: Mutex<Option<i64>>,
    // New:
    plugin_registry: Arc<PluginRegistry>,
    plugin_tool_bridge: PluginToolBridge,
}

impl AgentApplication {
    pub fn new() -> Result<Self, String> {
        // ... existing init ...

        let plugin_dirs = vec![
            peekoo_paths::peekoo_global_config_dir()
                .unwrap_or_default()
                .join("plugins"),
        ];

        // These share the same SQLite connection used by settings
        let permission_store = PermissionStore::new(db_conn.clone());
        let state_store = PluginStateStore::new(db_conn.clone());

        let registry = Arc::new(PluginRegistry::new(
            plugin_dirs,
            permission_store,
            state_store,
        ));

        // Auto-load enabled plugins
        for (dir, _manifest) in registry.discover() {
            if let Err(e) = registry.load_plugin(&dir) {
                tracing::warn!("Failed to load plugin from {}: {e}", dir.display());
            }
        }

        // Start timer tick
        peekoo_plugin_host::events::start_tick_timer(registry.clone());

        let tool_bridge = PluginToolBridge::new(registry.clone());

        Ok(Self {
            // ... existing fields ...
            plugin_registry: registry,
            plugin_tool_bridge: tool_bridge,
        })
    }

    // New: Plugin management commands

    pub fn plugin_list(&self) -> Result<Vec<PluginInfoDto>, String> {
        // Return info about all discovered/loaded plugins
        todo!()
    }

    pub fn plugin_enable(&self, key: &str) -> Result<(), String> {
        self.plugin_registry.load_plugin(
            &self.find_plugin_dir(key)?
        ).map(|_| ()).map_err(|e| e.to_string())
    }

    pub fn plugin_disable(&self, key: &str) -> Result<(), String> {
        self.plugin_registry.unload_plugin(key)
            .map_err(|e| e.to_string())
    }

    pub fn plugin_call_tool(
        &self,
        tool_name: &str,
        args_json: &str,
    ) -> Result<String, String> {
        self.plugin_tool_bridge.call_tool(tool_name, args_json)
    }

    pub fn plugin_list_panels(&self) -> Vec<PluginPanelDto> {
        self.plugin_registry.all_ui_panels()
            .into_iter()
            .map(|(key, panel)| PluginPanelDto {
                plugin_key: key,
                label: panel.label,
                title: panel.title,
                width: panel.width,
                height: panel.height,
                entry: panel.entry,
            })
            .collect()
    }
}
```

---

## 12. Tauri Commands (Transport Layer)

New Tauri commands in `desktop-tauri/src-tauri/src/lib.rs`:

```rust
// New commands for plugin management

#[tauri::command]
async fn plugin_list(state: State<'_, AgentState>) -> Result<Vec<PluginInfoDto>, String> {
    state.app.plugin_list()
}

#[tauri::command]
async fn plugin_enable(key: String, state: State<'_, AgentState>) -> Result<(), String> {
    state.app.plugin_enable(&key)
}

#[tauri::command]
async fn plugin_disable(key: String, state: State<'_, AgentState>) -> Result<(), String> {
    state.app.plugin_disable(&key)
}

#[tauri::command]
async fn plugin_call_tool(
    tool_name: String,
    args_json: String,
    state: State<'_, AgentState>,
) -> Result<String, String> {
    state.app.plugin_call_tool(&tool_name, &args_json)
}

#[tauri::command]
async fn plugin_list_panels(state: State<'_, AgentState>) -> Result<Vec<PluginPanelDto>, String> {
    Ok(state.app.plugin_list_panels())
}

#[tauri::command]
async fn plugin_query_data(
    plugin_key: String,
    provider_name: String,
    state: State<'_, AgentState>,
) -> Result<String, String> {
    state.app.plugin_registry
        .query_data(&plugin_key, &provider_name)
        .map_err(|e| e.to_string())
}

// Add to invoke_handler:
// plugin_list, plugin_enable, plugin_disable,
// plugin_call_tool, plugin_list_panels, plugin_query_data
```

---

## 13. Health Reminders Plugin (Reference Implementation)

### Plugin Manifest

`plugins/health-reminders/peekoo-plugin.toml`:
```toml
[plugin]
key = "health-reminders"
name = "Health Reminders"
version = "0.1.0"
author = "Peekoo Team"
description = "Periodic reminders to drink water, rest eyes, and stand up"
min_peekoo_version = "0.1.0"
wasm = "plugin.wasm"

[permissions]
required = ["timer", "notifications", "state:read", "state:write"]
optional = ["agent:register-tool"]

[[tools.definitions]]
name = "health_get_status"
description = "Get current health reminder status including next reminder times and daily stats"
parameters = '{"type": "object", "properties": {}, "required": []}'
return_type = "object"

[[tools.definitions]]
name = "health_configure"
description = "Configure health reminder intervals in minutes"
parameters = '''
{
  "type": "object",
  "properties": {
    "water_interval_min": {"type": "integer", "minimum": 5, "maximum": 120},
    "eye_rest_interval_min": {"type": "integer", "minimum": 5, "maximum": 60},
    "standup_interval_min": {"type": "integer", "minimum": 15, "maximum": 180}
  }
}
'''
return_type = "object"

[[tools.definitions]]
name = "health_dismiss"
description = "Dismiss a currently active reminder"
parameters = '''
{
  "type": "object",
  "properties": {
    "reminder_type": {"type": "string", "enum": ["water", "eye_rest", "standup"]}
  },
  "required": ["reminder_type"]
}
'''
return_type = "object"

[events]
subscribe = ["timer:tick", "pomodoro:finished"]
emit = ["health:reminder-due", "health:reminder-dismissed"]

[[data.providers]]
name = "health_reminder_status"
description = "Current state of all health reminders"
schema = '''
{
  "type": "object",
  "properties": {
    "water": {"type": "object"},
    "eye_rest": {"type": "object"},
    "standup": {"type": "object"}
  }
}
'''

[[ui.panels]]
label = "panel-health"
title = "Health Reminders"
width = 320
height = 400
entry = "ui/panel.html"
```

### WASM Plugin Source (Rust PDK)

```rust
// plugins/health-reminders/src/lib.rs

use extism_pdk::*;
use serde::{Deserialize, Serialize};

// --- Host function imports ---

extern "C" {
    fn peekoo_state_get(input: u64) -> u64;
    fn peekoo_state_set(input: u64) -> u64;
    fn peekoo_log(input: u64);
    fn peekoo_emit_event(input: u64);
    fn peekoo_notify(input: u64);
}

// --- Domain types ---

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReminderConfig {
    water_interval_min: u32,
    eye_rest_interval_min: u32,
    standup_interval_min: u32,
}

impl Default for ReminderConfig {
    fn default() -> Self {
        Self {
            water_interval_min: 45,
            eye_rest_interval_min: 20,
            standup_interval_min: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReminderState {
    reminder_type: String,
    interval_min: u32,
    minutes_since_last: u32,
    is_due: bool,
    times_completed_today: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HealthStatus {
    water: ReminderState,
    eye_rest: ReminderState,
    standup: ReminderState,
    enabled: bool,
}

// --- WASM exports ---

#[plugin_fn]
pub fn plugin_init(_input: String) -> FnResult<String> {
    // Load or initialize config from state
    let config = load_config();
    save_config(&config);

    // Initialize counters
    save_counter("water_minutes", 0);
    save_counter("eye_rest_minutes", 0);
    save_counter("standup_minutes", 0);
    save_counter("water_count", 0);
    save_counter("eye_rest_count", 0);
    save_counter("standup_count", 0);

    log_info("Health Reminders plugin initialized");

    Ok(r#"{"status": "initialized"}"#.to_string())
}

#[plugin_fn]
pub fn on_event(input: String) -> FnResult<String> {
    let event: serde_json::Value = serde_json::from_str(&input)?;
    let event_name = event["event"].as_str().unwrap_or("");

    match event_name {
        "timer:tick" => handle_tick()?,
        "pomodoro:finished" => {
            // After a pomodoro, remind to stand up and drink water
            emit_event("health:reminder-due", &serde_json::json!({
                "type": "water",
                "message": "Great work! Time to drink some water."
            }));
            emit_event("health:reminder-due", &serde_json::json!({
                "type": "standup",
                "message": "Pomodoro complete! Stand up and stretch."
            }));
        }
        _ => {}
    }

    Ok(r#"{"ok": true}"#.to_string())
}

#[plugin_fn]
pub fn tool_health_get_status(_input: String) -> FnResult<String> {
    let status = build_status();
    Ok(serde_json::to_string(&status)?)
}

#[plugin_fn]
pub fn tool_health_configure(input: String) -> FnResult<String> {
    let patch: serde_json::Value = serde_json::from_str(&input)?;
    let mut config = load_config();

    if let Some(v) = patch["water_interval_min"].as_u64() {
        config.water_interval_min = v as u32;
    }
    if let Some(v) = patch["eye_rest_interval_min"].as_u64() {
        config.eye_rest_interval_min = v as u32;
    }
    if let Some(v) = patch["standup_interval_min"].as_u64() {
        config.standup_interval_min = v as u32;
    }

    save_config(&config);
    log_info(&format!("Config updated: {:?}", config));

    Ok(serde_json::to_string(&config)?)
}

#[plugin_fn]
pub fn tool_health_dismiss(input: String) -> FnResult<String> {
    let req: serde_json::Value = serde_json::from_str(&input)?;
    let reminder_type = req["reminder_type"].as_str().unwrap_or("");

    // Reset the counter for this reminder type
    let counter_key = format!("{}_minutes", reminder_type);
    save_counter(&counter_key, 0);

    // Increment completion counter
    let count_key = format!("{}_count", reminder_type);
    let count = load_counter(&count_key);
    save_counter(&count_key, count + 1);

    emit_event("health:reminder-dismissed", &serde_json::json!({
        "type": reminder_type
    }));

    Ok(serde_json::json!({
        "dismissed": reminder_type,
        "times_completed_today": count + 1
    }).to_string())
}

#[plugin_fn]
pub fn data_health_reminder_status(_input: String) -> FnResult<String> {
    let status = build_status();
    Ok(serde_json::to_string(&status)?)
}

// --- Internal helpers ---

fn handle_tick() -> Result<(), Error> {
    let config = load_config();

    // Increment all counters by 1 minute
    for (reminder_type, interval) in [
        ("water", config.water_interval_min),
        ("eye_rest", config.eye_rest_interval_min),
        ("standup", config.standup_interval_min),
    ] {
        let key = format!("{}_minutes", reminder_type);
        let minutes = load_counter(&key) + 1;
        save_counter(&key, minutes);

        if minutes >= interval {
            // Reminder is due!
            let message = match reminder_type {
                "water" => "Time to drink some water!",
                "eye_rest" => "Look away from your screen for 20 seconds (20-20-20 rule)",
                "standup" => "Time to stand up and stretch!",
                _ => "Health reminder",
            };

            notify(reminder_type, message);
            emit_event("health:reminder-due", &serde_json::json!({
                "type": reminder_type,
                "message": message,
                "minutes_elapsed": minutes
            }));
        }
    }

    Ok(())
}

fn build_status() -> HealthStatus {
    let config = load_config();
    HealthStatus {
        water: ReminderState {
            reminder_type: "water".into(),
            interval_min: config.water_interval_min,
            minutes_since_last: load_counter("water_minutes"),
            is_due: load_counter("water_minutes") >= config.water_interval_min,
            times_completed_today: load_counter("water_count"),
        },
        eye_rest: ReminderState {
            reminder_type: "eye_rest".into(),
            interval_min: config.eye_rest_interval_min,
            minutes_since_last: load_counter("eye_rest_minutes"),
            is_due: load_counter("eye_rest_minutes") >= config.eye_rest_interval_min,
            times_completed_today: load_counter("eye_rest_count"),
        },
        standup: ReminderState {
            reminder_type: "standup".into(),
            interval_min: config.standup_interval_min,
            minutes_since_last: load_counter("standup_minutes"),
            is_due: load_counter("standup_minutes") >= config.standup_interval_min,
            times_completed_today: load_counter("standup_count"),
        },
        enabled: true,
    }
}

// --- Host function wrappers ---

fn load_config() -> ReminderConfig {
    let result = state_get("config");
    match result {
        Some(v) => serde_json::from_value(v).unwrap_or_default(),
        None => ReminderConfig::default(),
    }
}

fn save_config(config: &ReminderConfig) {
    let value = serde_json::to_value(config).unwrap();
    state_set("config", &value);
}

fn load_counter(key: &str) -> u32 {
    state_get(key)
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(0)
}

fn save_counter(key: &str, value: u32) {
    state_set(key, &serde_json::json!(value));
}

fn state_get(key: &str) -> Option<serde_json::Value> {
    let input = serde_json::json!({ "key": key }).to_string();
    let mem = Memory::from_bytes(&input);
    unsafe {
        let result_offset = peekoo_state_get(mem.offset());
        let result = Memory::find(result_offset).unwrap();
        let response: serde_json::Value = serde_json::from_slice(result.bytes()).ok()?;
        let value = response.get("value")?.clone();
        if value.is_null() { None } else { Some(value) }
    }
}

fn state_set(key: &str, value: &serde_json::Value) {
    let input = serde_json::json!({ "key": key, "value": value }).to_string();
    let mem = Memory::from_bytes(&input);
    unsafe {
        peekoo_state_set(mem.offset());
    }
}

fn log_info(message: &str) {
    let input = serde_json::json!({ "level": "info", "message": message }).to_string();
    let mem = Memory::from_bytes(&input);
    unsafe {
        peekoo_log(mem.offset());
    }
}

fn emit_event(event: &str, payload: &serde_json::Value) {
    let input = serde_json::json!({ "event": event, "payload": payload }).to_string();
    let mem = Memory::from_bytes(&input);
    unsafe {
        peekoo_emit_event(mem.offset());
    }
}

fn notify(title: &str, body: &str) {
    let input = serde_json::json!({ "title": title, "body": body }).to_string();
    let mem = Memory::from_bytes(&input);
    unsafe {
        peekoo_notify(mem.offset());
    }
}
```

### Plugin UI

`plugins/health-reminders/ui/panel.html`:
```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Health Reminders</title>
  <link rel="stylesheet" href="panel.css">
</head>
<body>
  <div id="app">
    <h2>Health Reminders</h2>
    <div id="reminders"></div>
    <div id="settings"></div>
  </div>
  <script src="panel.js"></script>
</body>
</html>
```

`plugins/health-reminders/ui/panel.js`:
```javascript
// Plugin UI communicates with its WASM backend via Tauri commands
// that proxy through the plugin host.

async function callPlugin(toolName, args = {}) {
  return window.__TAURI__.core.invoke("plugin_call_tool", {
    toolName,
    argsJson: JSON.stringify(args),
  }).then(JSON.parse);
}

async function loadStatus() {
  const status = await callPlugin("health_get_status");
  renderReminders(status);
}

function renderReminders(status) {
  const container = document.getElementById("reminders");
  container.innerHTML = ["water", "eye_rest", "standup"]
    .map(type => {
      const r = status[type];
      const progress = Math.min((r.minutes_since_last / r.interval_min) * 100, 100);
      const label = type.replace("_", " ");
      return `
        <div class="reminder ${r.is_due ? 'due' : ''}">
          <div class="reminder-header">
            <span class="reminder-label">${label}</span>
            <span class="reminder-count">${r.times_completed_today}x today</span>
          </div>
          <div class="progress-bar">
            <div class="progress-fill" style="width: ${progress}%"></div>
          </div>
          <div class="reminder-time">${r.minutes_since_last}/${r.interval_min} min</div>
          ${r.is_due ? `<button onclick="dismiss('${type}')">Done</button>` : ''}
        </div>
      `;
    })
    .join("");
}

async function dismiss(type) {
  await callPlugin("health_dismiss", { reminder_type: type });
  loadStatus();
}

// Poll for updates every 30 seconds
loadStatus();
setInterval(loadStatus, 30000);
```

---

## 14. WASM Plugin Exports Convention

Every WASM plugin must export these functions:

| Export | Signature | Required | Description |
|---|---|---|---|
| `plugin_init` | `(String) -> String` | Yes | Called once on load |
| `plugin_shutdown` | `() -> ()` | No | Called before unload |
| `on_event` | `(String) -> String` | No | Event handler |
| `tool_{name}` | `(String) -> String` | Per manifest | One per declared tool |
| `data_{name}` | `(String) -> String` | Per manifest | One per declared data provider |

All inputs and outputs are JSON strings.

---

## 15. Files to Create/Modify

### New Files

#### Crate: `crates/peekoo-plugin-host/`
| File | Description |
|---|---|
| `Cargo.toml` | Crate manifest with extism, serde, toml, rusqlite deps |
| `src/lib.rs` | Public API re-exports |
| `src/manifest.rs` | `PluginManifest` and related types, TOML parsing |
| `src/runtime.rs` | `PluginInstance` - Extism Plugin lifecycle wrapper |
| `src/registry.rs` | `PluginRegistry` - discover, load, unload, dispatch |
| `src/permissions.rs` | `PermissionStore` - capability checks against SQLite |
| `src/host_functions.rs` | Host functions injected into WASM runtime |
| `src/events.rs` | `EventBus` - plugin event emission and dispatch |
| `src/state.rs` | `PluginStateStore` - KV persistence via plugin_state table |
| `src/tools.rs` | `PluginToolBridge` - agent tool routing |
| `src/error.rs` | `PluginError` enum |

#### Plugin: `plugins/health-reminders/`
| File | Description |
|---|---|
| `peekoo-plugin.toml` | Plugin manifest |
| `Cargo.toml` | Rust PDK project for WASM compilation |
| `src/lib.rs` | Plugin logic (reminders, timer handling, state) |
| `ui/panel.html` | Health reminders panel UI |
| `ui/panel.js` | Panel interaction logic |
| `ui/panel.css` | Panel styles |

#### Frontend: `apps/desktop-ui/src/`
| File | Description |
|---|---|
| `types/plugin.ts` | Zod schemas for plugin DTOs |
| `hooks/use-plugin-panels.ts` | Hook for managing plugin panel windows |
| `hooks/use-plugins.ts` | Hook for plugin list, enable/disable |
| `features/plugins/PluginList.tsx` | Plugin management UI component |
| `views/PluginSettingsView.tsx` | Plugin settings panel view (optional v1) |

### Modified Files

| File | Change |
|---|---|
| `Cargo.toml` (workspace) | Add `"crates/peekoo-plugin-host"` and `"plugins/health-reminders"` to members |
| `crates/peekoo-agent-app/Cargo.toml` | Add `peekoo-plugin-host` dependency |
| `crates/peekoo-agent-app/src/application.rs` | Add `PluginRegistry`, `PluginToolBridge` fields; add plugin management methods |
| `crates/peekoo-agent-app/src/lib.rs` | Re-export plugin DTOs |
| `apps/desktop-tauri/src-tauri/Cargo.toml` | (no change - depends on peekoo-agent-app which pulls in plugin-host) |
| `apps/desktop-tauri/src-tauri/src/lib.rs` | Add plugin Tauri commands, register custom URI protocol |
| `apps/desktop-ui/src/types/window.ts` | Add plugin panel schemas, extend panel label types |
| `apps/desktop-ui/src/hooks/use-panel-windows.ts` | Support dynamic plugin panels alongside static panels |
| `apps/desktop-ui/src/routing/resolve-view.tsx` | Handle `plugin-*` window labels |
| `crates/persistence-sqlite/src/lib.rs` | (No change - existing tables are sufficient) |

### Dependency Graph (Updated)

```
desktop-ui -> desktop-tauri -> peekoo-agent-app -> peekoo-plugin-host
                                                -> peekoo-agent
                                                -> peekoo-agent-auth
                                                -> peekoo-productivity-domain
                                                -> persistence-sqlite
                                                -> security
                                                -> peekoo-paths
```

---

## 16. Implementation Phases

### Phase 1: Plugin Host Foundation
1. Create `crates/peekoo-plugin-host/` with manifest parsing, runtime, registry
2. Implement host functions (state get/set, log)
3. Add to workspace `Cargo.toml`
4. Unit tests for manifest parsing and plugin lifecycle

### Phase 2: Health Reminders Plugin
1. Create `plugins/health-reminders/` with manifest and Rust PDK source
2. Compile to WASM target (`cargo build --target wasm32-wasip1`)
3. Test loading and calling tools via registry
4. Implement timer tick and event dispatch

### Phase 3: Agent Integration
1. Add `PluginToolBridge` to `AgentApplication`
2. Inject plugin tools into agent system prompt
3. Route tool calls from agent to plugin host
4. Test end-to-end: user asks about reminders -> agent calls plugin tool -> response

### Phase 4: UI Integration
1. Add Tauri commands for plugin management
2. Register custom protocol for plugin assets
3. Create frontend hooks and plugin panel support
4. Build health reminders panel UI

### Phase 5: Polish
1. Permission grant/revoke UI
2. Plugin enable/disable persistence
3. Error handling and user-facing error messages
4. Documentation for plugin authors

---

## 17. Open Questions / Future Work

1. **pi_agent_rust custom tools**: Does `pi_agent_rust` v0.1.7 support registering custom tools beyond its built-in set? If yes, plugin tools should be registered directly rather than via system prompt injection. This needs investigation.

2. **Plugin hot-reload**: V1 requires restart to load new plugins. Hot-reload can be added by watching plugin directories and re-creating Extism Plugin instances.

3. **Plugin store (v2)**: Remote plugin registry with versioning, signatures, and auto-updates.

4. **Inter-plugin communication**: V1 plugins are isolated. V2 could allow plugins to call each other's tools or subscribe to each other's events.

5. **Plugin UI framework**: V1 uses raw HTML/JS/CSS. V2 could provide a React-based plugin UI SDK that renders inside the host's React tree using a component registry pattern.

6. **WASM component model**: Extism currently uses WASI preview 1. As the WASM component model matures, the host function interface could be replaced with strongly-typed WIT (WebAssembly Interface Types) definitions.
