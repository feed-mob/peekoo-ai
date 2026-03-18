//! Plugin configuration.
//!
//! Reads resolved configuration values (user overrides merged with
//! defaults from `[[config.fields]]` in `peekoo-plugin.toml`).
//!
//! ```no_run
//! use peekoo_plugin_sdk::prelude::*;
//!
//! fn example() -> Result<(), Error> {
//!     let interval: u64 = peekoo::config::get("water_interval_min")?.unwrap_or(45);
//!     let all = peekoo::config::get_all()?;
//!     let _ = (interval, all);
//!     Ok(())
//! }
//! ```

use extism_pdk::{Error, Json};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::host_fns::{peekoo_config_get, ConfigGetRequest};

/// Get a single configuration value, deserialised into `T`.
///
/// Returns `Ok(None)` if the key does not exist.
pub fn get<T: DeserializeOwned>(key: &str) -> Result<Option<T>, Error> {
    let response = unsafe {
        peekoo_config_get(Json(ConfigGetRequest {
            key: Some(key.to_string()),
        }))?
    };

    if response.0.value.is_null() {
        return Ok(None);
    }

    let value = serde_json::from_value(response.0.value)
        .map_err(|e| Error::msg(format!("config::get deserialize error: {e}")))?;
    Ok(Some(value))
}

/// Get all configuration values as a JSON object.
pub fn get_all() -> Result<Value, Error> {
    let response = unsafe { peekoo_config_get(Json(ConfigGetRequest { key: None }))? };

    Ok(response.0.value)
}
