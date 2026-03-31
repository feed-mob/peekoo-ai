//! Path helpers backed by the shared Peekoo path conventions.

use anyhow::Result;
use std::path::PathBuf;

/// Get the Peekoo data directory shared across the workspace.
pub fn data_dir() -> Result<PathBuf> {
    peekoo_paths::peekoo_global_data_dir().map_err(|err| anyhow::anyhow!(err))
}

/// Get the directory for Node.js runtime.
pub fn node_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("resources").join("node"))
}

/// Get the directory for agent installations.
pub fn agents_dir() -> Result<PathBuf> {
    Ok(data_dir()?.join("resources").join("agents"))
}

/// Get the directory for a specific agent.
pub fn agent_dir(agent_id: &str) -> Result<PathBuf> {
    Ok(agents_dir()?.join(agent_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_matches_workspace_convention() {
        let expected = peekoo_paths::peekoo_global_data_dir().expect("peekoo global data dir");
        let actual = data_dir().expect("node runtime data dir");
        assert_eq!(actual, expected);
    }

    #[test]
    fn resource_dirs_are_nested_under_shared_data_dir() {
        let root = data_dir().expect("data dir");
        assert_eq!(
            node_dir().expect("node dir"),
            root.join("resources").join("node")
        );
        assert_eq!(
            agents_dir().expect("agents dir"),
            root.join("resources").join("agents")
        );
        assert_eq!(
            agent_dir("gemini").expect("agent dir"),
            root.join("resources").join("agents").join("gemini")
        );
    }
}
