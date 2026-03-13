//! Sprite mood control.
//!
//! Trigger sprite mood changes from a plugin. The mood reaction is queued
//! and emitted as a `pet:react` event to the desktop pet frontend by the
//! Tauri flush loop.
//!
//! ```no_run
//! use peekoo_plugin_sdk::prelude::*;
//!
//! // Transient mood — reverts to idle after timeout
//! peekoo::mood::set("opencode-done", false)?;
//!
//! // Sticky mood — persists until explicitly changed
//! peekoo::mood::set("opencode-working", true)?;
//! ```
//!
//! Requires the `pet:mood` permission.

use extism_pdk::{Error, Json};

use crate::host_fns::{peekoo_set_mood, SetMoodRequest};

/// Set the sprite mood.
///
/// `trigger` must be a valid `PetReactionTrigger` string (e.g.
/// `"opencode-working"`, `"opencode-done"`, `"opencode-idle"`).
///
/// When `sticky` is `true`, the mood persists until another mood is set.
/// When `false`, the mood reverts to idle after a short timeout.
pub fn set(trigger: &str, sticky: bool) -> Result<(), Error> {
    unsafe {
        peekoo_set_mood(Json(SetMoodRequest {
            trigger: trigger.to_string(),
            sticky,
        }))?;
    }
    Ok(())
}
