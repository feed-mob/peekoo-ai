//! Process command utilities for npm/npx
//! Replaces util::command module from Zed

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Create a new command with the given program
pub fn new_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    Command::new(program)
}

/// Configure npm command with directory and prefix options
pub fn configure_npm_command(command: &mut Command, directory: Option<&Path>) {
    if let Some(directory) = directory {
        command.current_dir(directory);
        command.arg("--prefix").arg(directory);
    }
}

/// Build npm command arguments with proper configuration
pub fn build_npm_command_args(
    entrypoint: Option<&Path>,
    cache_dir: &Path,
    user_config: Option<&Path>,
    global_config: Option<&Path>,
    subcommand: &str,
    args: &[&str],
) -> Vec<String> {
    let mut command_args = Vec::new();

    if let Some(entrypoint) = entrypoint {
        command_args.push(entrypoint.to_string_lossy().into_owned());
    }

    command_args.push(subcommand.to_string());
    command_args.push(format!("--cache={}", cache_dir.display()));

    if let Some(user_config) = user_config {
        command_args.push("--userconfig".to_string());
        command_args.push(user_config.to_string_lossy().into_owned());
    }

    if let Some(global_config) = global_config {
        command_args.push("--globalconfig".to_string());
        command_args.push(global_config.to_string_lossy().into_owned());
    }

    command_args.extend(args.iter().map(|a| a.to_string()));
    command_args
}

/// Build environment variables for npm commands
pub fn npm_command_env(node_binary: Option<&Path>) -> HashMap<String, String> {
    let mut command_env = HashMap::new();

    if let Some(node_binary) = node_binary {
        let env_path = path_with_node_binary_prepended(node_binary).unwrap_or_default();
        command_env.insert("PATH".to_string(), env_path.to_string_lossy().into_owned());
    }

    if let Ok(node_ca_certs) = std::env::var("NODE_EXTRA_CA_CERTS")
        && !node_ca_certs.is_empty()
    {
        command_env.insert("NODE_EXTRA_CA_CERTS".to_string(), node_ca_certs);
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(val) = std::env::var("SYSTEMROOT") {
            command_env.insert("SYSTEMROOT".to_string(), val);
        }
        if let Ok(val) = std::env::var("ComSpec") {
            command_env.insert("ComSpec".to_string(), val);
        }
    }

    command_env
}

/// Prepend Node binary directory to PATH
fn path_with_node_binary_prepended(node_binary: &Path) -> Option<std::ffi::OsString> {
    let existing_path = std::env::var_os("PATH");
    let node_bin_dir = node_binary.parent().map(|dir| dir.as_os_str());

    match (existing_path, node_bin_dir) {
        (Some(existing_path), Some(node_bin_dir)) => {
            if let Ok(joined) = std::env::join_paths(
                std::iter::once(PathBuf::from(node_bin_dir))
                    .chain(std::env::split_paths(&existing_path)),
            ) {
                Some(joined)
            } else {
                Some(existing_path)
            }
        }
        (Some(existing_path), None) => Some(existing_path),
        (None, Some(node_bin_dir)) => Some(node_bin_dir.to_owned()),
        _ => None,
    }
}
