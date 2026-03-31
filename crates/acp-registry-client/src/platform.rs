//! Platform detection and agent compatibility filtering
//!
//! Supports platform strings like:
//! - "darwin-aarch64" (macOS Apple Silicon)
//! - "darwin-x86_64" (macOS Intel)
//! - "linux-aarch64" (Linux ARM64)
//! - "linux-x86_64" (Linux x64)
//! - "windows-x86_64" (Windows x64)
//! - "windows-aarch64" (Windows ARM64)

use crate::types::{Agent, BinaryPlatform, InstallMethod};

/// Get the current platform string
///
/// Format: "{os}-{arch}"
/// Examples: "darwin-aarch64", "linux-x86_64"
pub fn current_platform() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // Normalize OS names
    let os_normalized = match os {
        "macos" => "darwin",
        "linux" => "linux",
        "windows" => "windows",
        other => other,
    };

    // Normalize architecture names
    let arch_normalized = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        "arm" => "aarch64", // Treat ARM as aarch64 for compatibility
        other => other,
    };

    format!("{}-{}", os_normalized, arch_normalized)
}

/// Check if an agent is supported on the current platform
pub fn is_supported(agent: &Agent) -> bool {
    is_supported_on(agent, &current_platform())
}

/// Check if an agent is supported on a specific platform
pub fn is_supported_on(agent: &Agent, platform: &str) -> bool {
    let dist = &agent.distribution;

    // Check NPX (supported on all platforms if available)
    if dist.npx.is_some() {
        return true;
    }

    // Check UVX (supported on all platforms if available)
    if dist.uvx.is_some() {
        return true;
    }

    // Check Binary for specific platform
    if let Some(ref binaries) = dist.binary {
        return binaries.contains_key(platform);
    }

    false
}

/// Get supported installation methods for an agent on current platform
pub fn supported_methods(agent: &Agent) -> Vec<InstallMethod> {
    supported_methods_on(agent, &current_platform())
}

/// Get supported installation methods for an agent on a specific platform
pub fn supported_methods_on(agent: &Agent, platform: &str) -> Vec<InstallMethod> {
    let mut methods = Vec::new();
    let dist = &agent.distribution;

    // NPX is supported on all platforms
    if dist.npx.is_some() {
        methods.push(InstallMethod::Npx);
    }

    // UVX is supported on all platforms
    if dist.uvx.is_some() {
        methods.push(InstallMethod::Uvx);
    }

    // Binary is platform-specific
    if let Some(ref binaries) = dist.binary
        && binaries.contains_key(platform)
    {
        methods.push(InstallMethod::Binary);
    }

    methods
}

/// Get the preferred installation method for an agent
///
/// Priority: Binary > NPX > UVX
/// Binary is preferred when available because it's standalone
pub fn preferred_method(agent: &Agent) -> Option<InstallMethod> {
    preferred_method_for(agent, &current_platform())
}

/// Get the preferred installation method for a specific platform
///
/// Priority: Binary > NPX > UVX
pub fn preferred_method_for(agent: &Agent, platform: &str) -> Option<InstallMethod> {
    let methods = supported_methods_on(agent, platform);

    // Priority order
    if methods.contains(&InstallMethod::Binary) {
        return Some(InstallMethod::Binary);
    }
    if methods.contains(&InstallMethod::Npx) {
        return Some(InstallMethod::Npx);
    }
    if methods.contains(&InstallMethod::Uvx) {
        return Some(InstallMethod::Uvx);
    }

    None
}

/// Get binary platform info for an agent on current platform
pub fn get_binary_platform(agent: &Agent) -> Option<&BinaryPlatform> {
    get_binary_platform_for(agent, &current_platform())
}

/// Get binary platform info for an agent on a specific platform
pub fn get_binary_platform_for<'a>(agent: &'a Agent, platform: &str) -> Option<&'a BinaryPlatform> {
    agent.distribution.binary.as_ref()?.get(platform)
}

/// List all supported platforms for an agent
pub fn list_supported_platforms(agent: &Agent) -> Vec<String> {
    let mut platforms = Vec::new();

    // NPX and UVX support all platforms
    if agent.distribution.npx.is_some() || agent.distribution.uvx.is_some() {
        platforms.push("all platforms".to_string());
    }

    // Binary supports specific platforms
    if let Some(ref binaries) = agent.distribution.binary {
        platforms.extend(binaries.keys().cloned());
    }

    platforms
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BinaryPlatform, Distribution, NpxDistribution};
    use std::collections::HashMap;

    fn create_test_agent_with_npx() -> Agent {
        Agent {
            id: "test".to_string(),
            name: "Test Agent".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            repository: None,
            website: None,
            authors: vec![],
            license: "MIT".to_string(),
            icon: None,
            distribution: Distribution {
                npx: Some(NpxDistribution {
                    package: "test".to_string(),
                    args: vec![],
                    env: HashMap::new(),
                }),
                binary: None,
                uvx: None,
            },
        }
    }

    fn create_test_agent_with_binary() -> Agent {
        let mut binaries = HashMap::new();
        binaries.insert(
            "darwin-aarch64".to_string(),
            BinaryPlatform {
                archive: "https://example.com/test.tar.gz".to_string(),
                cmd: "./test".to_string(),
                args: vec![],
                env: HashMap::new(),
            },
        );

        Agent {
            id: "test-binary".to_string(),
            name: "Test Binary Agent".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            repository: None,
            website: None,
            authors: vec![],
            license: "MIT".to_string(),
            icon: None,
            distribution: Distribution {
                npx: None,
                binary: Some(binaries),
                uvx: None,
            },
        }
    }

    #[test]
    fn test_current_platform_format() {
        let platform = current_platform();
        assert!(platform.contains('-'));
        let parts: Vec<&str> = platform.split('-').collect();
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn test_is_supported_npx() {
        let agent = create_test_agent_with_npx();
        assert!(is_supported(&agent));
        assert!(is_supported_on(&agent, "any-platform"));
    }

    #[test]
    fn test_is_supported_binary_specific() {
        let agent = create_test_agent_with_binary();
        assert!(is_supported_on(&agent, "darwin-aarch64"));
        assert!(!is_supported_on(&agent, "linux-x86_64"));
    }

    #[test]
    fn test_preferred_method_priority() {
        let mut binaries = HashMap::new();
        binaries.insert(
            "test-platform".to_string(),
            BinaryPlatform {
                archive: "test".to_string(),
                cmd: "./test".to_string(),
                args: vec![],
                env: HashMap::new(),
            },
        );

        let agent = Agent {
            id: "multi".to_string(),
            name: "Multi".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            repository: None,
            website: None,
            authors: vec![],
            license: "MIT".to_string(),
            icon: None,
            distribution: Distribution {
                npx: Some(NpxDistribution {
                    package: "test".to_string(),
                    args: vec![],
                    env: HashMap::new(),
                }),
                binary: Some(binaries),
                uvx: None,
            },
        };

        // Binary should be preferred over NPX
        assert_eq!(
            preferred_method_for(&agent, "test-platform"),
            Some(InstallMethod::Binary)
        );
    }
}
