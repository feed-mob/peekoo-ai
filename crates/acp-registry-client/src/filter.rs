//! Agent filtering and sorting utilities

use crate::platform::current_platform;
use crate::types::{Agent, AvailableAgent, InstallMethod};

/// Filter and sort agents for display
pub fn filter_agents(agents: &[Agent], options: &FilterOptions) -> Vec<AvailableAgent> {
    let platform = current_platform();

    let mut filtered: Vec<_> = agents
        .iter()
        .filter(|agent| filter_single_agent(agent, options, &platform))
        .map(|agent| AvailableAgent::from_agent(agent.clone(), &platform))
        .collect();

    sort_agents(&mut filtered, options.sort_by);
    filtered
}

/// Filter options for agent list
#[derive(Debug, Clone, Default)]
pub struct FilterOptions {
    /// Only show agents supported on current platform
    pub platform_supported_only: bool,
    /// Filter by installation method
    pub method_filter: Option<InstallMethod>,
    /// Search query (matches name, description, id)
    pub search_query: Option<String>,
    /// Sort order
    pub sort_by: SortBy,
}

/// Sort criteria
#[derive(Debug, Clone, Copy, Default)]
pub enum SortBy {
    /// Alphabetical by name
    #[default]
    Name,
    /// By ID
    Id,
    /// Platform compatibility (supported first)
    PlatformSupport,
    /// Popularity (future: could use download counts)
    #[allow(dead_code)]
    Popularity,
}

fn filter_single_agent(agent: &Agent, options: &FilterOptions, platform: &str) -> bool {
    // Platform support filter
    if options.platform_supported_only {
        if !crate::platform::is_supported_on(agent, platform) {
            return false;
        }
    }

    // Method filter
    if let Some(method) = options.method_filter {
        let supported = crate::platform::supported_methods_on(agent, platform);
        if !supported.contains(&method) {
            return false;
        }
    }

    // Search filter
    if let Some(ref query) = options.search_query {
        let query_lower = query.to_lowercase();
        let matches = agent.name.to_lowercase().contains(&query_lower)
            || agent.id.to_lowercase().contains(&query_lower)
            || agent.description.to_lowercase().contains(&query_lower);
        if !matches {
            return false;
        }
    }

    true
}

/// Sort available agents
pub fn sort_agents(agents: &mut [AvailableAgent], sort_by: SortBy) {
    match sort_by {
        SortBy::Name => {
            agents.sort_by(|a, b| a.agent.name.cmp(&b.agent.name));
        }
        SortBy::Id => {
            agents.sort_by(|a, b| a.agent.id.cmp(&b.agent.id));
        }
        SortBy::PlatformSupport => {
            agents.sort_by(|a, b| {
                // Supported agents first
                let a_supported = if a.current_platform_supported { 0 } else { 1 };
                let b_supported = if b.current_platform_supported { 0 } else { 1 };
                a_supported
                    .cmp(&b_supported)
                    .then_with(|| a.agent.name.cmp(&b.agent.name))
            });
        }
        SortBy::Popularity => {
            // For now, just sort by name
            // Future: could use download counts or star ratings
            agents.sort_by(|a, b| a.agent.name.cmp(&b.agent.name));
        }
    }
}

/// Get featured/popular agents (subset of registry)
///
/// This is used to highlight well-known agents in the UI
pub fn featured_agents(agents: &[Agent]) -> Vec<&Agent> {
    let featured_ids = [
        "gemini",     // Google
        "cursor",     // Cursor
        "claude-acp", // Claude
        "codex-acp",  // OpenAI
        "kimi",       // Moonshot AI
        "goose",      // Block
        "qwen-code",  // Alibaba
        "opencode",   // Anomaly
        "pi-acp",     // Community
        "cline",      // Cline Bot
    ];

    let mut featured = Vec::new();
    for agent in agents {
        if featured_ids.contains(&agent.id.as_str()) {
            featured.push(agent);
        }
    }

    // Sort by featured order
    featured.sort_by_key(|agent| {
        featured_ids
            .iter()
            .position(|&id| id == agent.id)
            .unwrap_or(usize::MAX)
    });

    featured
}

/// Group agents by installation method
pub fn group_by_method(agents: &[AvailableAgent]) -> GroupedAgents {
    let mut npx = Vec::new();
    let mut binary = Vec::new();
    let mut uvx = Vec::new();
    let mut other = Vec::new();

    for agent in agents {
        if let Some(method) = agent.preferred_method {
            match method {
                InstallMethod::Npx => npx.push(agent.clone()),
                InstallMethod::Binary => binary.push(agent.clone()),
                InstallMethod::Uvx => uvx.push(agent.clone()),
            }
        } else {
            other.push(agent.clone());
        }
    }

    GroupedAgents {
        npx,
        binary,
        uvx,
        other,
    }
}

/// Agents grouped by installation method
#[derive(Debug, Clone, Default)]
pub struct GroupedAgents {
    pub npx: Vec<AvailableAgent>,
    pub binary: Vec<AvailableAgent>,
    pub uvx: Vec<AvailableAgent>,
    pub other: Vec<AvailableAgent>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Agent, Distribution, NpxDistribution};
    use std::collections::HashMap;

    fn create_test_agent(id: &str, name: &str) -> Agent {
        Agent {
            id: id.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: format!("{} description", name),
            repository: None,
            website: None,
            authors: vec![],
            license: "MIT".to_string(),
            icon: None,
            distribution: Distribution {
                npx: Some(NpxDistribution {
                    package: id.to_string(),
                    args: vec![],
                    env: HashMap::new(),
                }),
                binary: None,
                uvx: None,
            },
        }
    }

    #[test]
    fn test_filter_by_search() {
        let agents = vec![
            create_test_agent("gemini", "Gemini CLI"),
            create_test_agent("cursor", "Cursor"),
            create_test_agent("test", "Test Agent"),
        ];

        let options = FilterOptions {
            search_query: Some("gemini".to_string()),
            ..Default::default()
        };

        let filtered = filter_agents(&agents, &options);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].agent.id, "gemini");
    }

    #[test]
    fn test_sort_by_name() {
        let agents = vec![
            create_test_agent("zebra", "Zebra"),
            create_test_agent("alpha", "Alpha"),
            create_test_agent("beta", "Beta"),
        ];

        let mut available: Vec<_> = agents
            .iter()
            .map(|a| AvailableAgent::from_agent(a.clone(), &current_platform()))
            .collect();

        sort_agents(&mut available, SortBy::Name);

        assert_eq!(available[0].agent.name, "Alpha");
        assert_eq!(available[1].agent.name, "Beta");
        assert_eq!(available[2].agent.name, "Zebra");
    }

    #[test]
    fn test_filter_agents_applies_sort_order() {
        let agents = vec![
            create_test_agent("zebra", "Zebra"),
            create_test_agent("alpha", "Alpha"),
            create_test_agent("beta", "Beta"),
        ];

        let filtered = filter_agents(
            &agents,
            &FilterOptions {
                sort_by: SortBy::Id,
                ..Default::default()
            },
        );

        assert_eq!(filtered[0].agent.id, "alpha");
        assert_eq!(filtered[1].agent.id, "beta");
        assert_eq!(filtered[2].agent.id, "zebra");
    }

    #[test]
    fn test_featured_agents() {
        let agents = vec![
            create_test_agent("gemini", "Gemini"),
            create_test_agent("cursor", "Cursor"),
            create_test_agent("unknown", "Unknown"),
        ];

        let featured = featured_agents(&agents);
        assert_eq!(featured.len(), 2);
        assert_eq!(featured[0].id, "gemini");
        assert_eq!(featured[1].id, "cursor");
    }
}
