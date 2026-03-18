//! Event emission.
//!
//! Emit custom events that other plugins can subscribe to.
//!
//! ```no_run
//! use peekoo_plugin_sdk::prelude::*;
//! use serde_json::json;
//!
//! fn example() -> Result<(), Error> {
//!     peekoo::events::emit("health:reminder-due", json!({ "reminder_type": "water" }))?;
//!     Ok(())
//! }
//! ```

use extism_pdk::{Error, Json};
use serde::Serialize;

use crate::host_fns::{peekoo_emit_event, EmitEventRequest};

/// Emit a named event with a JSON-serialisable payload.
///
/// Other plugins subscribed to this event name (via `[events] subscribe`
/// in their manifest) will receive it in their `on_event` export.
pub fn emit<T: Serialize>(event: &str, payload: T) -> Result<(), Error> {
    let payload_value = serde_json::to_value(payload)
        .map_err(|e| Error::msg(format!("events::emit serialize error: {e}")))?;

    unsafe {
        peekoo_emit_event(Json(EmitEventRequest {
            event: event.to_string(),
            payload: payload_value,
        }))?;
    }
    Ok(())
}
