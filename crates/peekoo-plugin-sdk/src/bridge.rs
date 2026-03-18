//! Bridge filesystem reader.
//!
//! Read data written by external processes to the plugin's bridge file.
//!
//! Bridge file location:
//! - Windows: `%LOCALAPPDATA%/Peekoo/peekoo/bridges/<plugin-key>.json`
//! - Other platforms: `~/.peekoo/bridges/<plugin-key>.json`
//!
//! Requires the `bridge:fs_read` permission.
//!
//! ```no_run
//! use peekoo_plugin_sdk::prelude::*;
//!
//! fn example() -> Result<(), Error> {
//!     if let Some(contents) = peekoo::bridge::read()? {
//!         let data: serde_json::Value = serde_json::from_str(&contents)?;
//!         peekoo::log::info(&format!("bridge data: {data}"));
//!     }
//!     Ok(())
//! }
//! ```

use extism_pdk::Error;

use crate::host_fns::peekoo_bridge_fs_read;

/// Read the bridge file for this plugin.
///
/// Returns `Ok(Some(contents))` if the file exists and is readable,
/// or `Ok(None)` if the file does not exist.
///
/// The file path is platform-specific and always scoped to the current plugin
/// key, which is injected by the host and cannot be overridden.
pub fn read() -> Result<Option<String>, Error> {
    let response = unsafe { peekoo_bridge_fs_read(String::new())? };
    Ok(response.0.content)
}
