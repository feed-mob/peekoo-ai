//! Registry data structures matching ACP registry JSON format
//!
//! See: https://github.com/agentclientprotocol/registry/blob/main/FORMAT.md

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root registry structure
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Registry {
    pub version: String,
    pub agents: Vec<Agent>,
    #[serde(default)]
    pub extensions: Vec<Extension>,
}

/// ACP agent from registry
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Agent {
    pub id: String,   // "gemini", "cursor"
    pub name: String, // "Gemini CLI"
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    pub license: String,
    #[serde(default)]
    pub icon: Option<String>, // URL to SVG
    pub distribution: Distribution,
}

impl Agent {
    /// Get the icon URL or a default placeholder
    pub fn icon_url(&self) -> String {
        self.icon.clone().unwrap_or_else(|| {
            format!(
                "https://cdn.agentclientprotocol.com/registry/v1/latest/{}.svg",
                self.id
            )
        })
    }
}

/// Distribution methods (NPX, Binary, UVX)
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Default)]
pub struct Distribution {
    #[serde(default)]
    pub npx: Option<NpxDistribution>,
    #[serde(default)]
    pub binary: Option<HashMap<String, BinaryPlatform>>, // key = "darwin-aarch64"
    #[serde(default)]
    pub uvx: Option<UvxDistribution>,
}

impl Distribution {
    /// Check if this distribution has any supported method
    pub fn has_any_method(&self) -> bool {
        self.npx.is_some() || self.binary.is_some() || self.uvx.is_some()
    }
}

/// NPX distribution method
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct NpxDistribution {
    pub package: String, // "@google/gemini-cli"
    #[serde(default)]
    pub args: Vec<String>, // ["--acp"]
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Binary distribution for a specific platform
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BinaryPlatform {
    pub archive: String, // Download URL
    pub cmd: String,     // "./gemini" or "gemini.exe"
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// UVX distribution method (Python-based)
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct UvxDistribution {
    pub package: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// Extension from registry (future use)
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub version: String,
    // Extension-specific fields can be added later
}

/// Supported installation methods for an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallMethod {
    Npx,
    Binary,
    Uvx,
}

impl std::fmt::Display for InstallMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallMethod::Npx => write!(f, "NPX"),
            InstallMethod::Binary => write!(f, "Binary"),
            InstallMethod::Uvx => write!(f, "UVX"),
        }
    }
}

/// Agent with platform compatibility info
#[derive(Debug, Clone)]
pub struct AvailableAgent {
    pub agent: Agent,
    pub supported_methods: Vec<InstallMethod>,
    pub current_platform_supported: bool,
    pub preferred_method: Option<InstallMethod>,
}

impl AvailableAgent {
    /// Create from agent and current platform
    pub fn from_agent(agent: Agent, platform: &str) -> Self {
        use crate::platform::{is_supported_on, preferred_method_for, supported_methods_on};

        let supported_methods = supported_methods_on(&agent, platform);
        let current_platform_supported = is_supported_on(&agent, platform);
        let preferred_method = preferred_method_for(&agent, platform);

        Self {
            agent,
            supported_methods,
            current_platform_supported,
            preferred_method,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_icon_url_default() {
        let agent = Agent {
            id: "gemini".to_string(),
            name: "Gemini CLI".to_string(),
            version: "0.35.3".to_string(),
            description: "Test".to_string(),
            repository: None,
            website: None,
            authors: vec![],
            license: "Apache-2.0".to_string(),
            icon: None,
            distribution: Distribution::default(),
        };

        assert_eq!(
            agent.icon_url(),
            "https://cdn.agentclientprotocol.com/registry/v1/latest/gemini.svg"
        );
    }

    #[test]
    fn test_distribution_has_any_method() {
        let mut dist = Distribution::default();
        assert!(!dist.has_any_method());

        dist.npx = Some(NpxDistribution {
            package: "test".to_string(),
            args: vec![],
            env: HashMap::new(),
        });
        assert!(dist.has_any_method());
    }
}
