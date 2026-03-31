use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow, bail};
use futures::{FutureExt as _, channel::oneshot, future::Shared};
use semver::Version;
use serde::Deserialize;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{Mutex, watch};
use tracing::{info, warn};

use crate::http_client::HttpClient;
use crate::paths::data_dir;

const NODE_VERSION: &str = "v20.18.0";
const NODE_CA_CERTS_ENV_VAR: &str = "NODE_EXTRA_CA_CERTS";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NodeBinaryOptions {
    pub allow_path_lookup: bool,
    pub allow_binary_download: bool,
    pub use_paths: Option<(PathBuf, PathBuf)>,
}

/// Use this when you need to launch npm as a long-lived process
#[derive(Clone, Debug)]
pub struct NpmCommand {
    pub path: PathBuf,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

pub enum VersionStrategy<'a> {
    /// Install if current version doesn't match pinned version
    Pin(&'a Version),
    /// Install if current version is older than latest version
    Latest(&'a Version),
}

#[derive(Clone)]
pub struct NodeRuntime {
    state: Arc<Mutex<NodeRuntimeState>>,
}

struct NodeRuntimeState {
    http: HttpClient,
    instance: Option<Box<dyn NodeRuntimeTrait>>,
    last_options: Option<NodeBinaryOptions>,
    options: watch::Receiver<Option<NodeBinaryOptions>>,
    shell_env_loaded: Shared<oneshot::Receiver<()>>,
}

impl NodeRuntime {
    pub fn new(
        http: HttpClient,
        shell_env_loaded: Option<oneshot::Receiver<()>>,
        options: watch::Receiver<Option<NodeBinaryOptions>>,
    ) -> Self {
        NodeRuntime {
            state: Arc::new(Mutex::new(NodeRuntimeState {
                http,
                instance: None,
                last_options: None,
                options,
                shell_env_loaded: shell_env_loaded.unwrap_or(oneshot::channel().1).shared(),
            })),
        }
    }

    pub fn unavailable() -> Self {
        NodeRuntime {
            state: Arc::new(Mutex::new(NodeRuntimeState {
                http: HttpClient::blocked(),
                instance: None,
                last_options: None,
                options: watch::channel(Some(NodeBinaryOptions::default())).1,
                shell_env_loaded: oneshot::channel().1.shared(),
            })),
        }
    }

    async fn instance(&self) -> Box<dyn NodeRuntimeTrait> {
        let mut state = self.state.lock().await;

        let options = loop {
            if let Some(options) = state.options.borrow().as_ref() {
                break options.clone();
            }
            match state.options.changed().await {
                Ok(()) => {}
                Err(err) => {
                    return Box::new(UnavailableNodeRuntime {
                        error_message: err.to_string().into(),
                    }) as Box<dyn NodeRuntimeTrait>;
                }
            }
        };

        if state.last_options.as_ref() != Some(&options) {
            state.instance.take();
        }
        if let Some(instance) = state.instance.as_ref() {
            return instance.boxed_clone();
        }

        if let Some((node, npm)) = options.use_paths.as_ref() {
            let instance = match SystemNodeRuntime::new(node.clone(), npm.clone()).await {
                Ok(instance) => {
                    info!("using Node.js from settings: {:?}", instance);
                    Box::new(instance)
                }
                Err(err) => {
                    return Box::new(UnavailableNodeRuntime {
                        error_message: format!(
                            "failure checking Node.js from settings ({}): {:?}",
                            node.display(),
                            err
                        )
                        .into(),
                    });
                }
            };
            state.instance = Some(instance.boxed_clone());
            state.last_options = Some(options);
            return instance;
        }

        let _ = state.shell_env_loaded.clone().await;

        let system_node_error = if options.allow_path_lookup {
            match SystemNodeRuntime::detect().await {
                Ok(instance) => {
                    info!("using Node.js found on PATH: {:?}", instance);
                    state.instance = Some(instance.boxed_clone());
                    state.last_options = Some(options);
                    return Box::new(instance) as Box<dyn NodeRuntimeTrait>;
                }
                Err(err) => Some(err),
            }
        } else {
            None
        };

        let instance = if options.allow_binary_download {
            let why_using_managed = match system_node_error {
                Some(err @ DetectError::NotInPath(_)) => err.to_string(),
                Some(err @ DetectError::Other(_)) => err.to_string(),
                None => "system Node.js not available".to_string(),
            };
            match ManagedNodeRuntime::install_if_needed(&state.http).await {
                Ok(instance) => {
                    info!(
                        "using Peekoo managed Node.js at {} since {}",
                        instance.installation_path.display(),
                        why_using_managed
                    );
                    Box::new(instance) as Box<dyn NodeRuntimeTrait>
                }
                Err(err) => Box::new(UnavailableNodeRuntime {
                    error_message: format!(
                        "failure while downloading and/or installing Peekoo managed Node.js: {}",
                        err
                    )
                    .into(),
                }) as Box<dyn NodeRuntimeTrait>,
            }
        } else if let Some(system_node_error) = system_node_error {
            return Box::new(UnavailableNodeRuntime {
                error_message: format!(
                    "failure while checking system Node.js from PATH: {}",
                    system_node_error
                )
                .into(),
            });
        } else {
            Box::new(UnavailableNodeRuntime {
                error_message: "Node.js settings do not allow any way to use Node.js"
                    .to_string()
                    .into(),
            })
        };

        state.instance = Some(instance.boxed_clone());
        state.last_options = Some(options);
        instance
    }

    pub async fn binary_path(&self) -> Result<PathBuf> {
        self.instance().await.binary_path()
    }

    pub async fn run_npm_subcommand(
        &self,
        directory: Option<&Path>,
        subcommand: &str,
        args: &[&str],
    ) -> Result<Output> {
        self.instance()
            .await
            .run_npm_subcommand(directory, subcommand, args)
            .await
    }

    pub async fn npm_package_installed_version(
        &self,
        local_package_directory: &Path,
        name: &str,
    ) -> Result<Option<Version>> {
        self.instance()
            .await
            .npm_package_installed_version(local_package_directory, name)
            .await
    }

    pub async fn npm_command(&self, subcommand: &str, args: &[&str]) -> Result<NpmCommand> {
        self.instance().await.npm_command(subcommand, args).await
    }

    pub async fn npm_package_latest_version(&self, name: &str) -> Result<Version> {
        let output = self
            .run_npm_subcommand(
                None,
                "info",
                &[
                    name,
                    "--json",
                    "--fetch-retry-mintimeout",
                    "2000",
                    "--fetch-retry-maxtimeout",
                    "5000",
                    "--fetch-timeout",
                    "5000",
                ],
            )
            .await?;

        let mut info: NpmInfo = serde_json::from_slice(&output.stdout)?;
        info.dist_tags
            .latest
            .or_else(|| info.versions.pop())
            .with_context(|| format!("no version found for npm package {name}"))
    }

    pub async fn npm_install_packages(
        &self,
        directory: &Path,
        packages: &[(&str, &str)],
    ) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let packages: Vec<_> = packages
            .iter()
            .map(|(name, version)| format!("{name}@{version}"))
            .collect();

        let arguments: Vec<_> = packages
            .iter()
            .map(|p| p.as_str())
            .chain([
                "--save-exact",
                "--fetch-retry-mintimeout",
                "2000",
                "--fetch-retry-maxtimeout",
                "5000",
                "--fetch-timeout",
                "5000",
            ])
            .collect();

        self.run_npm_subcommand(Some(directory), "install", &arguments)
            .await?;
        Ok(())
    }

    pub async fn should_install_npm_package(
        &self,
        package_name: &str,
        local_executable_path: &Path,
        local_package_directory: &Path,
        version_strategy: VersionStrategy<'_>,
    ) -> bool {
        if fs::metadata(local_executable_path).await.is_err() {
            return true;
        }

        let Some(installed_version) = self
            .npm_package_installed_version(local_package_directory, package_name)
            .await
            .ok()
            .flatten()
        else {
            return true;
        };

        match version_strategy {
            VersionStrategy::Pin(pinned_version) => &installed_version != pinned_version,
            VersionStrategy::Latest(latest_version) => &installed_version < latest_version,
        }
    }
}

enum ArchiveType {
    TarGz,
    Zip,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NpmInfo {
    #[serde(default)]
    dist_tags: NpmInfoDistTags,
    versions: Vec<Version>,
}

#[derive(Debug, Deserialize, Default)]
pub struct NpmInfoDistTags {
    latest: Option<Version>,
}

#[async_trait::async_trait]
trait NodeRuntimeTrait: Send + Sync {
    fn boxed_clone(&self) -> Box<dyn NodeRuntimeTrait>;
    fn binary_path(&self) -> Result<PathBuf>;

    async fn run_npm_subcommand(
        &self,
        directory: Option<&Path>,
        subcommand: &str,
        args: &[&str],
    ) -> Result<Output>;

    async fn npm_command(&self, subcommand: &str, args: &[&str]) -> Result<NpmCommand>;

    async fn npm_package_installed_version(
        &self,
        local_package_directory: &Path,
        name: &str,
    ) -> Result<Option<Version>>;
}

#[derive(Clone)]
struct ManagedNodeRuntime {
    installation_path: PathBuf,
}

impl ManagedNodeRuntime {
    #[cfg(not(target_os = "windows"))]
    const NODE_PATH: &str = "bin/node";
    #[cfg(target_os = "windows")]
    const NODE_PATH: &str = "node.exe";

    #[cfg(not(target_os = "windows"))]
    const NPM_PATH: &str = "bin/npm";
    #[cfg(target_os = "windows")]
    const NPM_PATH: &str = "node_modules/npm/bin/npm-cli.js";

    async fn install_if_needed(http: &HttpClient) -> Result<Self> {
        info!("Node runtime install_if_needed");

        let os = match std::env::consts::OS {
            "macos" => "darwin",
            "linux" => "linux",
            "windows" => "win",
            other => bail!("Running on unsupported os: {other}"),
        };

        let arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            other => bail!("Running on unsupported architecture: {other}"),
        };

        let version = NODE_VERSION;
        let folder_name = format!("node-{version}-{os}-{arch}");
        let node_containing_dir = data_dir()?.join("resources/node");
        let node_dir = node_containing_dir.join(&folder_name);
        let node_binary = node_dir.join(Self::NODE_PATH);
        let npm_file = node_dir.join(Self::NPM_PATH);
        let node_ca_certs = std::env::var(NODE_CA_CERTS_ENV_VAR).unwrap_or_default();

        let valid = if fs::metadata(&node_binary).await.is_ok() {
            let result = Command::new(&node_binary)
                .env(NODE_CA_CERTS_ENV_VAR, &node_ca_certs)
                .arg(&npm_file)
                .arg("--version")
                .arg("--cache")
                .arg(node_dir.join("cache"))
                .arg("--userconfig")
                .arg(node_dir.join("blank_user_npmrc"))
                .arg("--globalconfig")
                .arg(node_dir.join("blank_global_npmrc"))
                .output()
                .await;
            match result {
                Ok(output) => output.status.success(),
                Err(err) => {
                    warn!(
                        "Peekoo managed Node.js binary at {} failed check: {}",
                        node_binary.display(),
                        err
                    );
                    false
                }
            }
        } else {
            false
        };

        if !valid {
            let _ = fs::remove_dir_all(&node_containing_dir).await;
            fs::create_dir(&node_containing_dir)
                .await
                .context("error creating node containing dir")?;

            let archive_type = match std::env::consts::OS {
                "macos" | "linux" => ArchiveType::TarGz,
                "windows" => ArchiveType::Zip,
                other => bail!("Running on unsupported os: {other}"),
            };

            let file_name = format!(
                "node-{version}-{os}-{arch}.{}",
                match archive_type {
                    ArchiveType::TarGz => "tar.gz",
                    ArchiveType::Zip => "zip",
                }
            );

            let url = format!("https://nodejs.org/dist/{version}/{file_name}");
            info!("Downloading Node.js binary from {url}");

            let bytes = http
                .get(&url)
                .await
                .context("error downloading Node binary")?;

            info!("Download of Node.js complete, extracting...");

            match archive_type {
                ArchiveType::TarGz => {
                    crate::archive::extract_targz(bytes, &node_containing_dir).await?;
                }
                ArchiveType::Zip => {
                    crate::archive::extract_zip(bytes, &node_containing_dir).await?;
                }
            }
            info!("Extracted Node.js to {}", node_containing_dir.display())
        }

        fs::create_dir(node_dir.join("cache")).await.ok();
        fs::write(node_dir.join("blank_user_npmrc"), []).await.ok();
        fs::write(node_dir.join("blank_global_npmrc"), [])
            .await
            .ok();

        Ok(ManagedNodeRuntime {
            installation_path: node_dir,
        })
    }
}

fn path_with_node_binary_prepended(node_binary: &Path) -> Option<std::ffi::OsString> {
    let existing_path = std::env::var_os("PATH");
    let node_bin_dir = node_binary.parent().map(|dir| dir.as_os_str());
    match (existing_path, node_bin_dir) {
        (Some(existing_path), Some(node_bin_dir)) => {
            if let Ok(joined) = std::env::join_paths(
                [PathBuf::from(node_bin_dir)]
                    .into_iter()
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

#[async_trait::async_trait]
impl NodeRuntimeTrait for ManagedNodeRuntime {
    fn boxed_clone(&self) -> Box<dyn NodeRuntimeTrait> {
        Box::new(self.clone())
    }

    fn binary_path(&self) -> Result<PathBuf> {
        Ok(self.installation_path.join(Self::NODE_PATH))
    }

    async fn run_npm_subcommand(
        &self,
        directory: Option<&Path>,
        subcommand: &str,
        args: &[&str],
    ) -> Result<Output> {
        let attempt = || async {
            let npm_command = self.npm_command(subcommand, args).await?;
            let mut command = Command::new(&npm_command.path);
            command.args(&npm_command.args);
            command.envs(&npm_command.env);
            crate::command::configure_npm_command(&mut command, directory);
            command.output().await.map_err(|e| anyhow!("{e}"))
        };

        let output = attempt().await;
        let output = if output.is_err() {
            attempt().await?
        } else {
            output?
        };

        if !output.status.success() {
            bail!(
                "failed to execute npm {subcommand}:\nstdout: {:?}\nstderr: {:?}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(output)
    }

    async fn npm_command(&self, subcommand: &str, args: &[&str]) -> Result<NpmCommand> {
        let node_binary = self.installation_path.join(Self::NODE_PATH);
        let npm_file = self.installation_path.join(Self::NPM_PATH);

        anyhow::ensure!(
            fs::metadata(&node_binary).await.is_ok(),
            "missing node binary file"
        );
        anyhow::ensure!(fs::metadata(&npm_file).await.is_ok(), "missing npm file");

        let command_args = crate::command::build_npm_command_args(
            Some(&npm_file),
            &self.installation_path.join("cache"),
            Some(&self.installation_path.join("blank_user_npmrc")),
            Some(&self.installation_path.join("blank_global_npmrc")),
            subcommand,
            args,
        );
        let command_env = crate::command::npm_command_env(Some(&node_binary));

        Ok(NpmCommand {
            path: node_binary,
            args: command_args,
            env: command_env,
        })
    }

    async fn npm_package_installed_version(
        &self,
        local_package_directory: &Path,
        name: &str,
    ) -> Result<Option<Version>> {
        read_package_installed_version(local_package_directory.join("node_modules"), name).await
    }
}

#[derive(Debug, Clone)]
pub struct SystemNodeRuntime {
    node: PathBuf,
    npm: PathBuf,
    global_node_modules: PathBuf,
    scratch_dir: PathBuf,
}

impl SystemNodeRuntime {
    const MIN_VERSION: semver::Version = Version::new(18, 0, 0);

    async fn new(node: PathBuf, npm: PathBuf) -> Result<Self> {
        let output = Command::new(&node)
            .arg("--version")
            .output()
            .await
            .with_context(|| format!("running node from {:?}", node))?;

        if !output.status.success() {
            bail!(
                "failed to run node --version. stdout: {}, stderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = semver::Version::parse(version_str.trim().trim_start_matches('v'))?;

        if version < Self::MIN_VERSION {
            bail!(
                "node at {} is too old. want: {}, got: {}",
                node.to_string_lossy(),
                Self::MIN_VERSION,
                version
            )
        }

        let scratch_dir = data_dir()?.join("resources/node");
        fs::create_dir(&scratch_dir).await.ok();
        fs::create_dir(scratch_dir.join("cache")).await.ok();

        let mut this = Self {
            node,
            npm,
            global_node_modules: PathBuf::default(),
            scratch_dir,
        };

        let output = this.run_npm_subcommand(None, "root", &["-g"]).await?;
        this.global_node_modules = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());

        Ok(this)
    }

    async fn detect() -> std::result::Result<Self, DetectError> {
        let node = which::which("node").map_err(DetectError::NotInPath)?;
        let npm = which::which("npm").map_err(DetectError::NotInPath)?;
        Self::new(node, npm).await.map_err(DetectError::Other)
    }
}

#[derive(Debug)]
enum DetectError {
    NotInPath(which::Error),
    Other(anyhow::Error),
}

impl std::fmt::Display for DetectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectError::NotInPath(err) => {
                write!(f, "system Node.js wasn't found on PATH: {}", err)
            }
            DetectError::Other(err) => {
                write!(f, "checking system Node.js failed with error: {}", err)
            }
        }
    }
}

#[async_trait::async_trait]
impl NodeRuntimeTrait for SystemNodeRuntime {
    fn boxed_clone(&self) -> Box<dyn NodeRuntimeTrait> {
        Box::new(self.clone())
    }

    fn binary_path(&self) -> Result<PathBuf> {
        Ok(self.node.clone())
    }

    async fn run_npm_subcommand(
        &self,
        directory: Option<&Path>,
        subcommand: &str,
        args: &[&str],
    ) -> anyhow::Result<Output> {
        let npm_command = self.npm_command(subcommand, args).await?;
        let mut command = Command::new(&npm_command.path);
        command.args(&npm_command.args);
        command.envs(&npm_command.env);
        crate::command::configure_npm_command(&mut command, directory);
        let output = command.output().await?;

        if !output.status.success() {
            bail!(
                "failed to execute npm {subcommand}:\nstdout: {:?}\nstderr: {:?}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(output)
    }

    async fn npm_command(&self, subcommand: &str, args: &[&str]) -> Result<NpmCommand> {
        let command_args = crate::command::build_npm_command_args(
            None,
            &self.scratch_dir.join("cache"),
            None,
            None,
            subcommand,
            args,
        );
        let command_env = crate::command::npm_command_env(Some(&self.node));

        Ok(NpmCommand {
            path: self.npm.clone(),
            args: command_args,
            env: command_env,
        })
    }

    async fn npm_package_installed_version(
        &self,
        local_package_directory: &Path,
        name: &str,
    ) -> Result<Option<Version>> {
        read_package_installed_version(local_package_directory.join("node_modules"), name).await
    }
}

pub async fn read_package_installed_version(
    node_module_directory: PathBuf,
    name: &str,
) -> Result<Option<Version>> {
    let package_json_path = node_module_directory.join(name).join("package.json");

    let mut file = match fs::File::open(&package_json_path).await {
        Ok(file) => file,
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }
            return Err(err.into());
        }
    };

    #[derive(Deserialize)]
    struct PackageJson {
        version: Version,
    }

    let mut contents = String::new();
    file.read_to_string(&mut contents).await?;
    let package_json: PackageJson = serde_json::from_str(&contents)?;
    Ok(Some(package_json.version))
}

#[derive(Clone)]
pub struct UnavailableNodeRuntime {
    error_message: Arc<String>,
}

#[async_trait::async_trait]
impl NodeRuntimeTrait for UnavailableNodeRuntime {
    fn boxed_clone(&self) -> Box<dyn NodeRuntimeTrait> {
        Box::new(self.clone())
    }

    fn binary_path(&self) -> Result<PathBuf> {
        bail!("{}", self.error_message)
    }

    async fn run_npm_subcommand(
        &self,
        _: Option<&Path>,
        _: &str,
        _: &[&str],
    ) -> anyhow::Result<Output> {
        bail!("{}", self.error_message)
    }

    async fn npm_command(&self, _subcommand: &str, _args: &[&str]) -> Result<NpmCommand> {
        bail!("{}", self.error_message)
    }

    async fn npm_package_installed_version(
        &self,
        _local_package_directory: &Path,
        _: &str,
    ) -> Result<Option<Version>> {
        bail!("{}", self.error_message)
    }
}
