//! # Peekoo Plugin SDK
//!
//! Safe, typed wrappers for all Peekoo host functions.
//!
//! This crate eliminates the boilerplate required to write a Peekoo plugin.
//! Instead of declaring `extern "ExtismHost"` blocks, request/response
//! structs, and `unsafe` wrappers by hand, import the [`prelude`] and call
//! the [`peekoo`] module functions directly.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! #![no_main]
//! use peekoo_plugin_sdk::prelude::*;
//!
//! #[derive(Deserialize)]
//! struct EchoInput { input: String }
//!
//! #[derive(Serialize)]
//! struct EchoOutput { echo: String, call_count: u64 }
//!
//! #[plugin_fn]
//! pub fn plugin_init(_: String) -> FnResult<String> {
//!     peekoo::log::info("plugin started");
//!     Ok(r#"{"status":"ok"}"#.into())
//! }
//!
//! #[plugin_fn]
//! pub fn tool_example_echo(Json(req): Json<EchoInput>) -> FnResult<Json<EchoOutput>> {
//!     let count: u64 = peekoo::state::get("call_count")?.unwrap_or(0);
//!     peekoo::state::set("call_count", &(count + 1))?;
//!     Ok(Json(EchoOutput { echo: req.input, call_count: count + 1 }))
//! }
//! ```
//!
//! ## Permissions for networked plugins
//!
//! Plugins that use the newer runtime helpers must declare the matching
//! permissions in `peekoo-plugin.toml`:
//!
//! ```toml
//! [permissions]
//! required = ["net:websocket", "crypto:ed25519"]
//! allowed_hosts = ["127.0.0.1:18789"]
//! ```
//!
//! ## WebSocket + crypto example
//!
//! ```rust,no_run
//! #![no_main]
//! use peekoo_plugin_sdk::prelude::*;
//!
//! #[derive(Serialize)]
//! struct ConnectPayload<'a> {
//!     id: &'a str,
//!     signed_at: u64,
//!     signature: &'a str,
//! }
//!
//! #[plugin_fn]
//! pub fn tool_open_socket(_: String) -> FnResult<String> {
//!     let socket_id = peekoo::websocket::connect("ws://127.0.0.1:18789")?;
//!     let key = peekoo::crypto::ed25519_get_or_create("gateway-device")?;
//!     let signed_at = peekoo::system::time_millis()?;
//!     let signature = peekoo::crypto::ed25519_sign(
//!         "gateway-device",
//!         &format!("{}|{}", key.public_key_sha256_hex, signed_at),
//!     )?;
//!     let payload = serde_json::to_string(&ConnectPayload {
//!         id: &peekoo::system::uuid_v4()?,
//!         signed_at,
//!         signature: &signature,
//!     })?;
//!     peekoo::websocket::send(&socket_id, &payload)?;
//!     let _reply = peekoo::websocket::recv(&socket_id)?;
//!     peekoo::websocket::close(&socket_id)?;
//!     Ok(r#"{"ok":true}"#.into())
//! }
//! ```

// Private: raw host function declarations and request/response types.
pub(crate) mod host_fns;

// Public types re-exported via prelude.
pub mod types;

// Individual API modules.
pub mod badge;
pub mod bridge;
pub mod config;
pub mod crypto;
pub mod events;
pub mod fs;
pub mod http;
pub mod log;
pub mod mood;
pub mod notify;
pub mod oauth;
pub mod process;
pub mod schedule;
pub mod secrets;
pub mod state;
pub mod system;
pub mod tasks;
pub mod websocket;

/// The `peekoo` namespace — plugin authors access all APIs through this.
///
/// ```rust
/// use peekoo_plugin_sdk::prelude::*;
///
/// // peekoo::state::get("key")?;
/// // peekoo::log::info("hello");
/// // peekoo::schedule::set("timer", 300, true, None)?;
/// ```
pub mod peekoo {
    //! Safe wrappers for Peekoo host functions, grouped by concern.

    pub use crate::badge;
    pub use crate::bridge;
    pub use crate::config;
    pub use crate::crypto;
    pub use crate::events;
    pub use crate::fs;
    pub use crate::http;
    pub use crate::log;
    pub use crate::mood;
    pub use crate::notify;
    pub use crate::oauth;
    pub use crate::process;
    pub use crate::schedule;
    pub use crate::secrets;
    pub use crate::state;
    pub use crate::system;
    pub use crate::tasks;
    pub use crate::websocket;
}

/// Prelude — import everything you need with a single `use`.
///
/// ```rust
/// use peekoo_plugin_sdk::prelude::*;
/// ```
pub mod prelude {
    // Re-export essential extism-pdk items so plugins don't need
    // to depend on extism-pdk directly.
    pub use extism_pdk::{plugin_fn, Error, FnResult, Json};

    // Re-export serde derives for convenience.
    pub use serde::{Deserialize, Serialize};

    // Re-export serde_json::Value and json! for ad-hoc JSON.
    pub use serde_json::{self, Value};

    // Re-export the peekoo namespace.
    pub use crate::peekoo;

    // Re-export public types.
    pub use crate::types::{BadgeItem, FsEntry, ScheduleInfo, SystemEvent};
}
