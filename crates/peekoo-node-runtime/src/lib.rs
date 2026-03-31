//! Node.js runtime management for Peekoo AI
//!
//! This module provides Node.js runtime detection and management:
//! 1. System Node.js - Use node/npm from PATH (>= v18.0.0)
//! 2. Managed Node.js - Download and manage our own Node.js v20.18.0
//!
//! # Example
//!
//! ```rust
//! use peekoo_node_runtime::{NodeRuntime, NodeBinaryOptions, HttpClient};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let http = HttpClient::new();
//! let (tx, rx) = tokio::sync::watch::channel(Some(NodeBinaryOptions::default()));
//! let runtime = NodeRuntime::new(http, None, rx);
//!
//! // Install a package
//! runtime.npm_install_packages(
//!     std::path::Path::new("~/.peekoo/resources/agents/gemini"),
//!     &[("@google/gemini-cli", "0.35.3")]
//! ).await?;
//! # Ok(())
//! # }
//! ```

pub mod archive;
pub mod command;
pub mod http_client;
pub mod node_runtime;
pub mod paths;

pub use node_runtime::{
    NodeBinaryOptions, NodeRuntime, NpmCommand, NpmInfo, NpmInfoDistTags, VersionStrategy,
    read_package_installed_version,
};

pub use http_client::HttpClient;
