use std::sync::OnceLock;

use tauri::{Runtime, plugin::TauriPlugin};

static MINIDUMP_HANDLE: OnceLock<Option<tauri_plugin_sentry::minidump::Handle>> = OnceLock::new();

/// Initialise Sentry and its native-crash minidump support.
pub fn init() {
    peekoo_analytics::sentry::init();
    MINIDUMP_HANDLE.get_or_init(|| {
        if peekoo_analytics::sentry::guard().is_some() {
            tauri_plugin_sentry::minidump::init(peekoo_analytics::sentry::client()).ok()
        } else {
            None
        }
    });
}

/// Build the Tauri Sentry plugin using the core analytics crate's client.
pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_sentry::init(peekoo_analytics::sentry::client())
}
