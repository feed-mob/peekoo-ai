//! Peek badge overlay.
//!
//! Set badge items that are displayed on the desktop pet's peek overlay.
//! Requires the `notifications` permission.
//!
//! ```no_run
//! use peekoo_plugin_sdk::prelude::*;
//!
//! fn example() -> Result<(), Error> {
//!     peekoo::badge::set(&[
//!         BadgeItem {
//!             label: "Water".into(),
//!             value: "~5 min".into(),
//!             icon: Some("droplet".into()),
//!             countdown_secs: Some(300),
//!         },
//!     ])?;
//!     Ok(())
//! }
//! ```

use extism_pdk::Error;

use crate::host_fns::peekoo_set_peek_badge;
use crate::types::BadgeItem;

/// Replace all badge items for this plugin.
///
/// Pass an empty slice to clear the badge.
///
/// Internally serialises the items to a JSON string because the
/// `peekoo_set_peek_badge` host function accepts a raw `String`
/// (unlike the other host functions which use `Json<T>`).
pub fn set(items: &[BadgeItem]) -> Result<(), Error> {
    let json = serde_json::to_string(items)
        .map_err(|e| Error::msg(format!("badge::set serialize error: {e}")))?;

    unsafe {
        peekoo_set_peek_badge(json)?;
    }
    Ok(())
}
