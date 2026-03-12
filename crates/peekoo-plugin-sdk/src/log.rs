//! Structured logging that routes through the Peekoo host.
//!
//! Log messages are tagged with the plugin key and forwarded
//! to the application's `tracing` subscriber.
//!
//! ```no_run
//! use peekoo_plugin_sdk::prelude::*;
//!
//! peekoo::log::info("plugin started");
//! peekoo::log::warn("something looks off");
//! peekoo::log::error("failed to do thing");
//! peekoo::log::debug("detailed trace info");
//! ```

use extism_pdk::Json;

use crate::host_fns::{peekoo_log, LogRequest};

fn log(level: &str, message: &str) {
    let _ = unsafe {
        peekoo_log(Json(LogRequest {
            level: level.to_string(),
            message: message.to_string(),
        }))
    };
}

/// Log an info-level message.
pub fn info(message: &str) {
    log("info", message);
}

/// Log a warning-level message.
pub fn warn(message: &str) {
    log("warn", message);
}

/// Log an error-level message.
pub fn error(message: &str) {
    log("error", message);
}

/// Log a debug-level message.
pub fn debug(message: &str) {
    log("debug", message);
}
