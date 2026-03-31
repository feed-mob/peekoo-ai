//! Agent installation from ACP registry
//!
//! Supports three installation methods:
//! - NPX: Install Node.js packages via npm/npx
//! - Binary: Download and extract platform-specific archives
//! - UVX: Install Python packages via uvx (future)

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::platform::{current_platform, get_binary_platform_for, preferred_method_for};
use crate::types::{Agent, BinaryPlatform, InstallMethod, NpxDistribution};

/// Configuration for agent installation
#[derive(Debug, Clone)]
pub struct InstallConfig {
    /// Agent to install
    pub agent: Agent,
    /// Installation method (if None, uses preferred method)
    pub method: Option<InstallMethod>,
    /// Directory to install to (e.g., ~/.peekoo/resources/agents/<agent-id>)
    pub install_dir: PathBuf,
}

/// Result of successful installation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Installation {
    /// Agent ID
    pub agent_id: String,
    /// Agent name
    pub agent_name: String,
    /// Installation method used
    pub method: InstallMethod,
    /// Path to executable
    pub executable_path: PathBuf,
    /// Agent version
    pub version: String,
    /// Command and arguments to run
    pub command: Vec<String>,
    /// Environment variables to set
    pub env: std::collections::HashMap<String, String>,
}

/// Install an agent from registry
///
/// # Arguments
/// * `config` - Installation configuration
/// * `node_runtime` - Required for NPX installations
///
/// # Example
/// ```rust,no_run
/// use acp_registry_client::{install, InstallConfig, InstallMethod};
/// use acp_registry_client::types::Agent;
/// use std::path::PathBuf;
///
/// # async fn example() -> anyhow::Result<()> {
/// # let agent = Agent {
/// #     id: "test".to_string(),
/// #     name: "Test".to_string(),
/// #     version: "1.0.0".to_string(),
/// #     description: "Test agent".to_string(),
/// #     repository: None,
/// #     website: None,
/// #     authors: vec![],
/// #     license: "MIT".to_string(),
/// #     icon: None,
/// #     distribution: Default::default(),
/// # };
/// let config = InstallConfig {
///     agent,
///     method: Some(InstallMethod::Npx),
///     install_dir: PathBuf::from("/path/to/install"),
/// };
///
/// # let node_runtime = unimplemented!();
/// let installation = install(config, Some(&node_runtime)).await?;
/// println!("Installed to: {:?}", installation.executable_path);
/// # Ok(())
/// # }
/// ```
pub async fn install(
    config: InstallConfig,
    node_runtime: Option<&peekoo_node_runtime::NodeRuntime>,
) -> Result<Installation> {
    let platform = current_platform();

    // Determine installation method
    let method = config
        .method
        .or_else(|| preferred_method_for(&config.agent, &platform))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Agent {} is not supported on platform {}",
                config.agent.id,
                platform
            )
        })?;

    // Create install directory
    fs::create_dir_all(&config.install_dir)
        .await
        .with_context(|| {
            format!(
                "Failed to create install directory: {}",
                config.install_dir.display()
            )
        })?;

    match method {
        InstallMethod::Npx => {
            if let Some(npx_dist) = &config.agent.distribution.npx {
                install_npx(&config, npx_dist, node_runtime).await
            } else {
                Err(anyhow::anyhow!(
                    "Agent {} has no NPX distribution",
                    config.agent.id
                ))
            }
        }
        InstallMethod::Binary => {
            if let Some(platform_info) = get_binary_platform_for(&config.agent, &platform) {
                install_binary(&config, platform_info).await
            } else {
                Err(anyhow::anyhow!(
                    "Agent {} has no binary for platform {}",
                    config.agent.id,
                    platform
                ))
            }
        }
        InstallMethod::Uvx => Err(anyhow::anyhow!("UVX installation not yet implemented")),
    }
}

/// Install via NPX
async fn install_npx(
    config: &InstallConfig,
    npx_dist: &NpxDistribution,
    node_runtime: Option<&peekoo_node_runtime::NodeRuntime>,
) -> Result<Installation> {
    let node_runtime = node_runtime
        .ok_or_else(|| anyhow::anyhow!("Node runtime required for NPX installation"))?;

    // Parse package name and version
    let package_spec = &npx_dist.package;
    let (package_name, version) = parse_package_spec(package_spec);

    // Install package using node_runtime
    node_runtime
        .npm_install_packages(&config.install_dir, &[(package_name, version)])
        .await
        .with_context(|| format!("Failed to install NPX package {}", package_spec))?;

    let npm_command = build_npx_command(node_runtime, package_spec, &npx_dist.args).await?;
    let mut env = npm_command.env;
    env.extend(npx_dist.env.clone());
    let executable_path = npm_command.path.clone();
    let mut command = vec![executable_path.to_string_lossy().to_string()];
    command.extend(npm_command.args);

    Ok(Installation {
        agent_id: config.agent.id.clone(),
        agent_name: config.agent.name.clone(),
        method: InstallMethod::Npx,
        executable_path,
        version: config.agent.version.clone(),
        command,
        env,
    })
}

/// Install via Binary download
async fn install_binary(
    config: &InstallConfig,
    platform_info: &BinaryPlatform,
) -> Result<Installation> {
    // Download archive
    let archive_url = &platform_info.archive;
    let archive_bytes: bytes::Bytes = download_file(archive_url)
        .await
        .with_context(|| format!("Failed to download binary archive from {}", archive_url))?;

    // Determine archive type from extension
    let is_zip = archive_url.ends_with(".zip");
    let is_tar_gz = archive_url.ends_with(".tar.gz") || archive_url.ends_with(".tgz");

    // Extract archive
    if is_zip {
        peekoo_node_runtime::archive::extract_zip(archive_bytes, &config.install_dir)
            .await
            .context("Failed to extract zip archive")?;
    } else if is_tar_gz {
        peekoo_node_runtime::archive::extract_targz(archive_bytes, &config.install_dir)
            .await
            .context("Failed to extract tar.gz archive")?;
    } else {
        return Err(anyhow::anyhow!(
            "Unsupported archive format: {}",
            archive_url
        ));
    }

    // Ensure binaries are executable
    let executable_name = Path::new(&platform_info.cmd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&platform_info.cmd);
    make_binaries_executable(&config.install_dir, executable_name).await?;

    // Build executable path
    let executable_path = config.install_dir.join(&platform_info.cmd);

    // Verify executable exists
    if !executable_path.exists() {
        // Try to find it by walking the directory
        let found = find_executable_in_dir(&config.install_dir, &platform_info.cmd).await?;
        if let Some(path) = found {
            let mut command = vec![path.to_string_lossy().to_string()];
            command.extend(platform_info.args.iter().cloned());
            return Ok(Installation {
                agent_id: config.agent.id.clone(),
                agent_name: config.agent.name.clone(),
                method: InstallMethod::Binary,
                executable_path: path.clone(),
                version: config.agent.version.clone(),
                command,
                env: platform_info.env.clone(),
            });
        }

        return Err(anyhow::anyhow!(
            "Expected executable not found after extraction: {:?}",
            executable_path
        ));
    }

    // Build command with args
    let mut command = vec![executable_path.to_string_lossy().to_string()];
    command.extend(platform_info.args.iter().cloned());

    Ok(Installation {
        agent_id: config.agent.id.clone(),
        agent_name: config.agent.name.clone(),
        method: InstallMethod::Binary,
        executable_path,
        version: config.agent.version.clone(),
        command,
        env: platform_info.env.clone(),
    })
}

/// Uninstall an agent (remove install directory)
pub async fn uninstall(_agent_id: &str, install_dir: PathBuf) -> Result<()> {
    if install_dir.exists() {
        fs::remove_dir_all(&install_dir).await.with_context(|| {
            format!(
                "Failed to remove agent directory: {}",
                install_dir.display()
            )
        })?;
    }

    Ok(())
}

/// Check if agent is installed
pub fn is_installed(_agent_id: &str, install_dir: PathBuf) -> bool {
    install_dir.exists() && install_dir.join(".installed").exists()
}

/// Mark agent as installed (create .installed marker file)
pub async fn mark_installed(install_dir: &PathBuf, installation: &Installation) -> Result<()> {
    let marker_path = install_dir.join(".installed");
    let json =
        serde_json::to_string(installation).context("Failed to serialize installation info")?;
    fs::write(&marker_path, json)
        .await
        .context("Failed to write installation marker")?;
    Ok(())
}

/// Load installation info from marker file
pub async fn load_installation_info(install_dir: &PathBuf) -> Result<Option<Installation>> {
    let marker_path = install_dir.join(".installed");

    if !marker_path.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(&marker_path)
        .await
        .context("Failed to read installation marker")?;
    let installation: Installation =
        serde_json::from_str(&json).context("Failed to parse installation info")?;

    Ok(Some(installation))
}

// Helper functions

fn parse_package_spec(spec: &str) -> (&str, &str) {
    // Parse "package@version" or just "package"
    if let Some(idx) = spec
        .char_indices()
        .skip(1)
        .filter_map(|(idx, ch)| (ch == '@').then_some(idx))
        .last()
    {
        (&spec[..idx], &spec[idx + 1..])
    } else {
        (spec, "latest")
    }
}

async fn build_npx_command(
    node_runtime: &peekoo_node_runtime::NodeRuntime,
    package_spec: &str,
    package_args: &[String],
) -> Result<peekoo_node_runtime::NpmCommand> {
    let mut exec_args = Vec::with_capacity(package_args.len() + 2);
    exec_args.push(package_spec.to_string());
    if !package_args.is_empty() {
        exec_args.push("--".to_string());
        exec_args.extend(package_args.iter().cloned());
    }
    let exec_args_refs: Vec<_> = exec_args.iter().map(String::as_str).collect();

    node_runtime
        .npm_command("exec", &exec_args_refs)
        .await
        .context("Failed to build NPX command")
}

async fn download_file(url: &str) -> Result<bytes::Bytes> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .context("Failed to build HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to download from {}", url))?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .context("Failed to read response body")?;

    Ok(bytes)
}

async fn find_executable_in_dir(dir: &Path, cmd: &str) -> Result<Option<PathBuf>> {
    let cmd_name = Path::new(cmd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(cmd);

    let mut stack = vec![dir.to_path_buf()];
    while let Some(current_dir) = stack.pop() {
        let mut entries = fs::read_dir(&current_dir).await.with_context(|| {
            format!(
                "Failed to read install directory: {}",
                current_dir.display()
            )
        })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_type = entry.file_type().await?;

            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if file_type.is_file() {
                let matches = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|name| name == cmd_name || name == format!("{}.exe", cmd_name))
                    .unwrap_or(false);
                if matches {
                    return Ok(Some(path));
                }
            }
        }
    }

    Ok(None)
}

#[cfg(unix)]
async fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .await
        .with_context(|| format!("Failed to set execute permission on {:?}", path))?;
    Ok(())
}

#[cfg(windows)]
async fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn should_be_executable(name: &str, main_executable: &str) -> bool {
    name == main_executable
        || (!name.contains('.') && !name.starts_with('.'))
        || name.ends_with(".sh")
}

#[cfg(windows)]
fn should_be_executable(name: &str, main_executable: &str) -> bool {
    name == main_executable
        || name.ends_with(".exe")
        || name.ends_with(".bat")
        || name.ends_with(".cmd")
        || name.ends_with(".ps1")
}

async fn make_binaries_executable(install_dir: &Path, main_executable: &str) -> Result<()> {
    let mut stack = vec![install_dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries = fs::read_dir(&dir).await.with_context(|| {
            format!("Failed to read directory: {}", dir.display())
        })?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_type = entry.file_type().await?;

            if file_type.is_dir() {
                stack.push(path);
                continue;
            }

            if file_type.is_file() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if should_be_executable(name, main_executable) {
                    make_executable(&path).await?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec() {
        assert_eq!(parse_package_spec("package@1.0.0"), ("package", "1.0.0"));
        assert_eq!(parse_package_spec("package"), ("package", "latest"));
        assert_eq!(
            parse_package_spec("@scope/package"),
            ("@scope/package", "latest")
        );
        assert_eq!(
            parse_package_spec("@scope/package@2.0.0"),
            ("@scope/package", "2.0.0")
        );
    }

    #[tokio::test]
    async fn test_find_executable_in_nested_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let nested_dir = temp_dir.path().join("agent").join("bin");
        fs::create_dir_all(&nested_dir).await.unwrap();
        let executable_path = nested_dir.join("agent-cli");
        fs::write(&executable_path, b"#!/bin/sh").await.unwrap();

        let found = find_executable_in_dir(temp_dir.path(), "agent-cli")
            .await
            .unwrap();

        assert_eq!(found, Some(executable_path));
    }
}
