//! Plugin state storage.
//!
//! Key-value store scoped to the current plugin.
//! Requires `state:read` / `state:write` permissions.
//!
//! ```no_run
//! use peekoo_plugin_sdk::prelude::*;
//!
//! let count: u64 = peekoo::state::get("call_count")?.unwrap_or(0);
//! peekoo::state::set("call_count", &(count + 1))?;
//! peekoo::state::delete("old_key")?;
//! ```

use extism_pdk::{Error, Json};
use serde::{de::DeserializeOwned, Serialize};

use crate::host_fns::{peekoo_state_get, peekoo_state_set, StateGetRequest, StateSetRequest};

/// Get a value from plugin state, deserialised into `T`.
///
/// Returns `Ok(None)` when the key does not exist (host returns `null`).
pub fn get<T: DeserializeOwned>(key: &str) -> Result<Option<T>, Error> {
    let response = unsafe {
        peekoo_state_get(Json(StateGetRequest {
            key: key.to_string(),
        }))?
    };

    if response.0.value.is_null() {
        return Ok(None);
    }

    let value = serde_json::from_value(response.0.value)
        .map_err(|e| Error::msg(format!("state::get deserialize error: {e}")))?;
    Ok(Some(value))
}

/// Set a value in plugin state.
///
/// The value is serialised to JSON before storage.
pub fn set<T: Serialize>(key: &str, value: &T) -> Result<(), Error> {
    let json_value = serde_json::to_value(value)
        .map_err(|e| Error::msg(format!("state::set serialize error: {e}")))?;

    unsafe {
        peekoo_state_set(Json(StateSetRequest {
            key: key.to_string(),
            value: json_value,
        }))?;
    }
    Ok(())
}

/// Delete a key from plugin state (sets it to `null`).
pub fn delete(key: &str) -> Result<(), Error> {
    unsafe {
        peekoo_state_set(Json(StateSetRequest {
            key: key.to_string(),
            value: serde_json::Value::Null,
        }))?;
    }
    Ok(())
}
