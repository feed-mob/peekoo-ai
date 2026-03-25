//! Desktop notifications.
//!
//! Requires the `notifications` permission.
//!
//! ```no_run
//! use peekoo_plugin_sdk::prelude::*;
//!
//! fn example() -> Result<(), Error> {
//!     let delivered = peekoo::notify::send("Reminder", "Time to drink water")?;
//!     if !delivered {
//!         peekoo::log::debug("notification was suppressed by DND");
//!     }
//!     Ok(())
//! }
//! ```

use extism_pdk::{Error, Json};

use crate::host_fns::{peekoo_notify, NotifyRequest};

/// Send a desktop notification.
///
/// Returns `true` if the notification was delivered, `false` if it was
/// suppressed (e.g. by do-not-disturb mode).
pub fn send(title: &str, body: &str) -> Result<bool, Error> {
    send_with_action(title, body, None, None)
}

/// Send a desktop notification with an optional action payload.
pub fn send_with_action(
    title: &str,
    body: &str,
    action_url: Option<&str>,
    action_label: Option<&str>,
) -> Result<bool, Error> {
    let response = unsafe {
        peekoo_notify(Json(NotifyRequest {
            title: title.to_string(),
            body: body.to_string(),
            action_url: action_url.map(ToString::to_string),
            action_label: action_label.map(ToString::to_string),
        }))?
    };

    Ok(!response.0.suppressed)
}
