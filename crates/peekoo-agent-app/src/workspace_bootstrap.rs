use std::fs;
use std::path::{Path, PathBuf};

use peekoo_paths::peekoo_global_config_dir;

const AGENTS_TEMPLATE: &str = include_str!("../templates/workspace/AGENTS.md");
const BOOTSTRAP_TEMPLATE: &str = include_str!("../templates/workspace/BOOTSTRAP.md");
const IDENTITY_TEMPLATE: &str = include_str!("../templates/workspace/IDENTITY.md");
const MEMORY_TEMPLATE: &str = include_str!("../templates/workspace/MEMORY.md");
const SOUL_TEMPLATE: &str = include_str!("../templates/workspace/SOUL.md");
const USER_TEMPLATE: &str = include_str!("../templates/workspace/USER.md");

mod skill_templates {
    include!(concat!(env!("OUT_DIR"), "/skill_templates.rs"));
}

const REQUIRED_USER_FIELDS: &[&str] = &["- Name: [NOT_SET]"];

/// The subdirectory name under the peekoo home where the agent workspace lives.
const WORKSPACE_SUBDIR: &str = "workspace";

pub fn ensure_agent_workspace() -> Result<PathBuf, String> {
    let peekoo_home = resolve_peekoo_home()?;
    let workspace_dir = peekoo_home.join(WORKSPACE_SUBDIR);

    if !workspace_dir.exists() {
        fs::create_dir_all(&workspace_dir)
            .map_err(|e| format!("Create agent workspace error: {e}"))?;
    }

    seed_if_missing(&workspace_dir, "AGENTS.md", AGENTS_TEMPLATE)?;
    seed_if_missing(&workspace_dir, "IDENTITY.md", IDENTITY_TEMPLATE)?;
    seed_if_missing(&workspace_dir, "SOUL.md", SOUL_TEMPLATE)?;
    seed_if_missing(&workspace_dir, "USER.md", USER_TEMPLATE)?;
    seed_if_missing(&workspace_dir, "MEMORY.md", MEMORY_TEMPLATE)?;
    sync_skill_templates(&workspace_dir)?;
    reconcile_bootstrap_file(&workspace_dir)?;

    Ok(workspace_dir)
}

/// Sync all bundled skill templates to the workspace.
/// Always overwrites so users get the latest versions on app start.
fn sync_skill_templates(workspace_dir: &Path) -> Result<(), String> {
    for (rel_path, content) in skill_templates::SKILL_FILES {
        let dest = workspace_dir.join(".agents/skills").join(rel_path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Create skill dir error: {e}"))?;
        }
        fs::write(&dest, content).map_err(|e| format!("Write skill file {rel_path} error: {e}"))?;
    }
    Ok(())
}

fn resolve_peekoo_home() -> Result<PathBuf, String> {
    if let Some(local_dir) = find_local_peekoo_dir() {
        return Ok(local_dir);
    }

    peekoo_global_config_dir()
}

fn find_local_peekoo_dir() -> Option<PathBuf> {
    let current_dir = std::env::current_dir().ok()?;
    let mut current = current_dir.as_path();

    loop {
        let candidate = current.join(".peekoo");
        if candidate.is_dir() {
            return Some(candidate);
        }

        current = current.parent()?;
    }
}

fn seed_if_missing(workspace_dir: &Path, file_name: &str, template: &str) -> Result<(), String> {
    let path = workspace_dir.join(file_name);
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Create parent directory for {file_name} error: {e}"))?;
    }

    fs::write(&path, template).map_err(|e| format!("Seed {file_name} error: {e}"))
}

fn reconcile_bootstrap_file(workspace_dir: &Path) -> Result<(), String> {
    let bootstrap_path = workspace_dir.join("BOOTSTRAP.md");
    if needs_bootstrap(workspace_dir)? {
        if !bootstrap_path.exists() {
            fs::write(&bootstrap_path, BOOTSTRAP_TEMPLATE)
                .map_err(|e| format!("Seed BOOTSTRAP.md error: {e}"))?;
        }
        return Ok(());
    }

    if bootstrap_path.exists() {
        fs::remove_file(&bootstrap_path).map_err(|e| format!("Remove BOOTSTRAP.md error: {e}"))?;
    }

    Ok(())
}

fn needs_bootstrap(workspace_dir: &Path) -> Result<bool, String> {
    let user_path = workspace_dir.join("USER.md");
    if !user_path.is_file() {
        return Ok(true);
    }

    let user_content =
        fs::read_to_string(&user_path).map_err(|e| format!("Read USER.md error: {e}"))?;
    Ok(REQUIRED_USER_FIELDS
        .iter()
        .any(|field| user_content.contains(field)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_test_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        path.push(format!("peekoo-agent-workspace-bootstrap-{prefix}-{nanos}"));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn reconcile_bootstrap_file_creates_bootstrap_when_user_profile_incomplete() {
        let dir = temp_test_dir("needs-bootstrap");
        fs::write(dir.join("USER.md"), USER_TEMPLATE).expect("write user template");

        reconcile_bootstrap_file(&dir).expect("reconcile bootstrap");

        assert!(dir.join("BOOTSTRAP.md").is_file());
    }

    #[test]
    fn reconcile_bootstrap_file_removes_bootstrap_when_profile_complete() {
        let dir = temp_test_dir("bootstrap-complete");
        fs::write(
            dir.join("USER.md"),
            "# USER.md - About Your Human\n\n- Name: Richard\n- Pronouns: Unknown\n",
        )
        .expect("write complete user profile");
        fs::write(dir.join("BOOTSTRAP.md"), BOOTSTRAP_TEMPLATE).expect("write bootstrap");

        reconcile_bootstrap_file(&dir).expect("reconcile bootstrap");

        assert!(!dir.join("BOOTSTRAP.md").exists());
    }

    #[test]
    fn seed_if_missing_preserves_existing_files() {
        let dir = temp_test_dir("preserve-existing");
        let path = dir.join("AGENTS.md");
        fs::write(&path, "custom instructions").expect("write existing file");

        seed_if_missing(&dir, "AGENTS.md", AGENTS_TEMPLATE).expect("seed agents");

        let content = fs::read_to_string(path).expect("read agents");
        assert_eq!(content, "custom instructions");
    }

    #[test]
    fn sync_skill_templates_writes_all_bundled_skills() {
        let dir = temp_test_dir("skill-sync");

        sync_skill_templates(&dir).expect("sync skill templates");

        assert!(dir.join(".agents/skills/memory-manager/SKILL.md").is_file());
    }

    #[test]
    fn sync_skill_templates_overwrites_existing_files() {
        let dir = temp_test_dir("skill-overwrite");
        let skill_path = dir.join(".agents/skills/memory-manager/SKILL.md");
        fs::create_dir_all(skill_path.parent().unwrap()).expect("create dirs");
        fs::write(&skill_path, "old content").expect("write old content");

        sync_skill_templates(&dir).expect("sync skill templates");

        let content = fs::read_to_string(&skill_path).expect("read skill");
        assert_ne!(content, "old content");
        assert!(content.contains("memory-manager"));
    }
}
