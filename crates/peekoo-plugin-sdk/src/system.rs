//! System helpers for plugins.
//!
//! These wrappers provide host-generated values that are useful for request
//! correlation, signing windows, and other runtime metadata.

use extism_pdk::Error;

use crate::host_fns::{peekoo_system_local_date, peekoo_system_time_millis, peekoo_system_uuid_v4};

/// Returns the current host time in milliseconds since the Unix epoch.
pub fn time_millis() -> Result<u64, Error> {
    let response = unsafe { peekoo_system_time_millis("{}".to_string())? };
    Ok(response.0.time_millis)
}

/// Returns a freshly generated UUIDv4 from the host runtime.
pub fn uuid_v4() -> Result<String, Error> {
    let response = unsafe { peekoo_system_uuid_v4("{}".to_string())? };
    Ok(response.0.uuid)
}

/// Returns the current local date as a string (YYYY-MM-DD) from the host.
pub fn local_date() -> Result<String, Error> {
    let response = unsafe { peekoo_system_local_date("{}".to_string())? };
    Ok(response.0.date)
}
