//! System helpers for plugins.

use extism_pdk::Error;

use crate::host_fns::{peekoo_system_time_millis, peekoo_system_uuid_v4};

pub fn time_millis() -> Result<u64, Error> {
    let response = unsafe { peekoo_system_time_millis("{}".to_string())? };
    Ok(response.0.time_millis)
}

pub fn uuid_v4() -> Result<String, Error> {
    let response = unsafe { peekoo_system_uuid_v4("{}".to_string())? };
    Ok(response.0.uuid)
}
