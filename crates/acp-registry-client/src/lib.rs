//! ACP Registry Client for Peekoo AI
//!
//! Provides access to the ACP (Agent Client Protocol) registry for discovering
//! and installing AI agents.
//!
//! # Example
//!
//! ```rust
//! use acp_registry_client::{RegistryClient, current_platform, is_supported};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let client = RegistryClient::new()?;
//! let registry = client.fetch().await?;
//!
//! for agent in &registry.agents {
//!     if is_supported(agent) {
//!         println!("Available: {} ({})", agent.name, agent.id);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod client;
pub mod filter;
pub mod install;
pub mod platform;
pub mod types;

pub use client::{CACHE_TTL_SECONDS, REGISTRY_URL, RegistryClient};
pub use filter::filter_agents;
pub use install::{InstallConfig, Installation, install, uninstall};
pub use platform::{current_platform, is_supported, preferred_method, supported_methods};
pub use types::*;
