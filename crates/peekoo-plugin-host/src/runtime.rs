use std::path::PathBuf;
use std::time::Duration;

use extism::{Manifest as ExtismManifest, Plugin};

use crate::error::PluginError;
use crate::manifest::PluginManifest;

/// A loaded, running plugin instance backed by an Extism WASM runtime.
pub struct PluginInstance {
    /// Parsed manifest.
    pub manifest: PluginManifest,
    /// Extism plugin handle.
    plugin: Plugin,
    /// Directory containing the plugin files.
    pub plugin_dir: PathBuf,
    /// Whether `plugin_init` has been called.
    initialized: bool,
}

impl PluginInstance {
    /// Load a WASM module from disk and create an Extism plugin instance.
    pub fn load(
        manifest: PluginManifest,
        plugin_dir: PathBuf,
        host_functions: Vec<extism::Function>,
        memory_max_pages: u32,
        timeout: Duration,
    ) -> Result<Self, PluginError> {
        let wasm_path = plugin_dir.join(&manifest.plugin.wasm);
        if !wasm_path.exists() {
            return Err(PluginError::Io(format!(
                "WASM file not found: {}",
                wasm_path.display()
            )));
        }

        let extism_manifest = ExtismManifest::new([extism::Wasm::file(&wasm_path)])
            .with_memory_max(memory_max_pages)
            .with_timeout(timeout);

        // Enable WASI support so plugins can use basic I/O (e.g. clock_time_get for timing).
        let plugin = Plugin::new(&extism_manifest, host_functions, true)?;

        Ok(Self {
            manifest,
            plugin,
            plugin_dir,
            initialized: false,
        })
    }

    /// Call the `plugin_init` export if it exists.
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

    /// Call a tool exported by the plugin.
    ///
    /// Tool functions follow the convention `tool_{name}`.
    pub fn call_tool(&mut self, tool_name: &str, input_json: &str) -> Result<String, PluginError> {
        if !self.initialized {
            return Err(PluginError::NotInitialized(
                self.manifest.plugin.key.clone(),
            ));
        }
        let export_name = format!("tool_{tool_name}");
        if !self.plugin.function_exists(&export_name) {
            return Err(PluginError::ToolNotFound(tool_name.to_string()));
        }
        let result: String = self.plugin.call(&export_name, input_json)?;
        Ok(result)
    }

    /// Dispatch an event to the plugin's `on_event` handler.
    pub fn handle_event(
        &mut self,
        event_name: &str,
        payload_json: &str,
    ) -> Result<(), PluginError> {
        if !self.initialized {
            return Err(PluginError::NotInitialized(
                self.manifest.plugin.key.clone(),
            ));
        }
        if self.plugin.function_exists("on_event") {
            let input = serde_json::json!({
                "event": event_name,
                "payload": serde_json::from_str::<serde_json::Value>(payload_json)
                    .unwrap_or(serde_json::Value::Null)
            });
            let _: String = self.plugin.call("on_event", input.to_string())?;
        }
        Ok(())
    }

    /// Query a data provider exported by the plugin.
    ///
    /// Data providers follow the convention `data_{name}`.
    pub fn query_data(&mut self, provider_name: &str) -> Result<String, PluginError> {
        if !self.initialized {
            return Err(PluginError::NotInitialized(
                self.manifest.plugin.key.clone(),
            ));
        }
        let export_name = format!("data_{provider_name}");
        if !self.plugin.function_exists(&export_name) {
            return Err(PluginError::DataProviderNotFound(provider_name.to_string()));
        }
        let result: String = self.plugin.call(&export_name, "")?;
        Ok(result)
    }

    /// Call the `plugin_shutdown` export if it exists.
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
