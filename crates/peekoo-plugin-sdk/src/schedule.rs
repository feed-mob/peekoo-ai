//! Timer scheduling.
//!
//! Requires the `scheduler` permission. Scheduled timers fire
//! `schedule:fired` events with `{ "key": "<your_key>" }` in the payload.
//!
//! ```no_run
//! use peekoo_plugin_sdk::prelude::*;
//!
//! fn example() -> Result<(), Error> {
//!     peekoo::schedule::set("my_timer", 300, true, None)?;
//!     if let Some(info) = peekoo::schedule::get("my_timer")? {
//!         peekoo::log::info(&format!("{}s remaining", info.time_remaining_secs));
//!     }
//!     peekoo::schedule::cancel("my_timer")?;
//!     Ok(())
//! }
//! ```

use extism_pdk::{Error, Json};

use crate::host_fns::{
    peekoo_schedule_cancel, peekoo_schedule_get, peekoo_schedule_set, ScheduleCancelRequest,
    ScheduleGetRequest, ScheduleSetRequest,
};
use crate::types::ScheduleInfo;

/// Create or replace a schedule timer.
///
/// - `key` — unique identifier for this timer (per plugin).
/// - `interval_secs` — interval in seconds between firings.
/// - `repeat` — if `true`, the timer repeats; if `false`, it fires once.
/// - `delay_secs` — optional initial delay before the first firing.
///   Defaults to `interval_secs` when `None`.
pub fn set(
    key: &str,
    interval_secs: u64,
    repeat: bool,
    delay_secs: Option<u64>,
) -> Result<(), Error> {
    unsafe {
        peekoo_schedule_set(Json(ScheduleSetRequest {
            key: key.to_string(),
            interval_secs,
            repeat,
            delay_secs,
        }))?;
    }
    Ok(())
}

/// Cancel a schedule timer.
pub fn cancel(key: &str) -> Result<(), Error> {
    unsafe {
        peekoo_schedule_cancel(Json(ScheduleCancelRequest {
            key: key.to_string(),
        }))?;
    }
    Ok(())
}

/// Get information about a schedule timer.
///
/// Returns `None` if no timer with this key exists.
pub fn get(key: &str) -> Result<Option<ScheduleInfo>, Error> {
    let response = unsafe {
        peekoo_schedule_get(Json(ScheduleGetRequest {
            key: key.to_string(),
        }))?
    };

    Ok(response.0.schedule)
}
