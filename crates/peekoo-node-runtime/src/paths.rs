//! Path helpers following XDG Base Directory Specification
//! Replaces Zed's paths module

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Get the Peekoo data directory
///
/// Follows XDG spec: ~/.local/share/peekoo/ on Linux
/// Platform-specific on macOS/Windows
pub fn data_dir() -> Result<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
            return Ok(PathBuf::from(xdg_data).join("peekoo"));
        }
    }

    let home = dirs::data_dir().context("Failed to determine data directory")?;

    Ok(home.join("peekoo"))
}

/// Get the directory for Node.js runtime
pub fn node_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("resources/node"))
}

/// Get the directory for agent installations
pub fn agents_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("resources/agents"))
}

/// Get the directory for a specific agent
pub fn agent_dir(agent_id: &str) -> Result<PathBuf> {
    Ok(agents_dir()?.join(agent_id))
}
