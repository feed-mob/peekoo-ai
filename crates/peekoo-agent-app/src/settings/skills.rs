use crate::settings::dto::SkillDto;
use std::collections::HashSet;
use std::path::PathBuf;

/// Discovers skills under the given root directories. A skill is any directory that
/// directly contains a `SKILL.md` file. Subdirectories are scanned recursively.
/// Loose `.md` files are ignored; only `SKILL.md` inside a directory marks a skill.
pub fn discover_skills_in_roots(roots: &[PathBuf]) -> Vec<SkillDto> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for root in roots {
        if root.is_dir() {
            discover_skills_under_dir(root, &mut seen, &mut out);
        }
    }
    out
}

fn discover_skills_under_dir(dir: &std::path::Path, seen: &mut HashSet<String>, out: &mut Vec<SkillDto>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let skill_md = path.join("SKILL.md");
        if skill_md.is_file() {
            let skill_id = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if seen.insert(skill_id.clone()) {
                out.push(SkillDto {
                    skill_id,
                    source_type: "path".into(),
                    path: skill_md.to_string_lossy().to_string(),
                    enabled: true,
                });
            }
        }

        discover_skills_under_dir(&path, seen, out);
    }
}

pub(crate) fn skill_discovery_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(current) = std::env::current_dir() {
        let mut cursor = current;
        loop {
            let candidate = skill_root_for_peekoo_home(&cursor.join(".peekoo"));
            if candidate.is_dir() {
                roots.push(candidate);
                break;
            }

            let Some(parent) = cursor.parent() else {
                break;
            };
            cursor = parent.to_path_buf();
        }
    }

    if let Ok(peekoo_home) = peekoo_paths::peekoo_global_config_dir() {
        let global_skills_dir = skill_root_for_peekoo_home(&peekoo_home);
        roots.push(global_skills_dir);
    }

    if let Some(legacy_home) = peekoo_paths::peekoo_legacy_home_dir_if_distinct() {
        roots.push(skill_root_for_peekoo_home(&legacy_home));
    }

    roots
}

fn skill_root_for_peekoo_home(peekoo_home: &std::path::Path) -> PathBuf {
    peekoo_home.join("workspace").join(".agents").join("skills")
}

pub fn discover_skills() -> Vec<SkillDto> {
    let roots = skill_discovery_roots();
    discover_skills_in_roots(&roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn discover_skills_finds_nested_skill_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("skills");
        fs::create_dir_all(root.join("alpha")).unwrap();
        fs::write(root.join("alpha/SKILL.md"), "# alpha").unwrap();
        fs::create_dir_all(root.join("group/beta")).unwrap();
        fs::write(root.join("group/beta/SKILL.md"), "# beta").unwrap();
        fs::create_dir_all(root.join("group/deeper/gamma")).unwrap();
        fs::write(root.join("group/deeper/gamma/SKILL.md"), "# gamma").unwrap();
        fs::create_dir_all(root.join("not-a-skill")).unwrap();
        fs::write(root.join("noise.md"), "# not a skill").unwrap();

        let discovered = discover_skills_in_roots(&[root]);
        let mut skill_ids: Vec<_> = discovered.iter().map(|s| s.skill_id.as_str()).collect();
        skill_ids.sort();
        assert_eq!(skill_ids, vec!["alpha", "beta", "gamma"]);
        assert!(discovered.iter().all(|s| s.path.ends_with("SKILL.md")));
    }

    #[test]
    fn skill_root_uses_workspace_agents_skills_under_peekoo_home() {
        let peekoo_home = PathBuf::from("/tmp/example/.peekoo");
        let root = skill_root_for_peekoo_home(&peekoo_home);

        assert_eq!(
            root,
            PathBuf::from("/tmp/example/.peekoo/workspace/.agents/skills")
        );
    }
}
