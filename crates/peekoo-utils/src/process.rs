//! Process execution utilities with Windows extension support
//!
//! This module provides a drop-in replacement for `std::process::Command`
//! and `tokio::process::Command` that automatically resolves Windows
//! executable extensions (.exe, .cmd, .bat).

use std::ffi::OsStr;

/// Resolves a command name to its full path with Windows extension if needed.
///
/// On Windows, if the command doesn't have an extension, this tries:
/// - .exe
/// - .cmd
/// - .bat
///
/// On non-Windows platforms, returns the command unchanged.
pub fn resolve_command<S: AsRef<OsStr>>(command: S) -> std::path::PathBuf {
    let command_ref = command.as_ref();

    #[cfg(windows)]
    {
        // If command already has an extension or is a path, use it as-is
        let path = std::path::Path::new(command_ref);
        if path.extension().is_some() || path.components().count() > 1 {
            return path.to_path_buf();
        }

        let command_str = command_ref.to_string_lossy();

        // Try common Windows executable extensions
        for ext in [".exe", ".cmd", ".bat"] {
            let command_with_ext = format!("{}{}", command_str, ext);
            if which::which(&command_with_ext).is_ok() {
                return std::path::PathBuf::from(command_with_ext);
            }
        }
    }

    // On non-Windows or if no extension found, return as-is
    std::path::PathBuf::from(command_ref)
}

/// Creates a new `std::process::Command` with automatic Windows extension resolution.
///
/// This is a drop-in replacement for `std::process::Command::new`.
///
/// # Example
/// ```
/// use peekoo_utils::process::command;
///
/// let mut cmd = command("npm"); // Will resolve to "npm.cmd" on Windows
/// cmd.arg("--version");
/// ```
pub fn command<S: AsRef<OsStr>>(program: S) -> std::process::Command {
    let resolved = resolve_command(program);
    std::process::Command::new(resolved)
}

/// Creates a new `tokio::process::Command` with automatic Windows extension resolution.
///
/// This is a drop-in replacement for `tokio::process::Command::new`.
///
/// # Example
/// ```
/// use peekoo_utils::process::tokio_command;
///
/// let mut cmd = tokio_command("npm"); // Will resolve to "npm.cmd" on Windows
/// cmd.arg("--version");
/// ```
#[cfg(feature = "tokio")]
pub fn tokio_command<S: AsRef<OsStr>>(program: S) -> tokio::process::Command {
    let resolved = resolve_command(program);
    tokio::process::Command::new(resolved)
}

/// Check if a command is available on the system (with Windows extension support).
///
/// # Example
/// ```
/// use peekoo_utils::process::command_available;
///
/// if command_available("npm") {
///     println!("npm is available");
/// }
/// ```
pub fn command_available<S: AsRef<OsStr>>(command: S) -> bool {
    let resolved = resolve_command(command);
    which::which(&resolved).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_command_keeps_extensions() {
        // Commands with extensions should be returned as-is
        let result = resolve_command("program.exe");
        assert_eq!(result.to_string_lossy(), "program.exe");
    }

    #[test]
    fn test_resolve_command_handles_paths() {
        // Full paths should be returned as-is
        let result = resolve_command("/usr/bin/node");
        assert_eq!(result.to_string_lossy(), "/usr/bin/node");
    }
}
