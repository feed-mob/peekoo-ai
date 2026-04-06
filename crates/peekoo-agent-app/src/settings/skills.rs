use crate::settings::dto::{SkillDto, SkillInstallOutcome};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Skill IDs that are bundled with Peekoo and cannot be deleted.
const BUILTIN_SKILLS: &[&str] = &["peekoo-agent-skill", "memory-manager"];

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

fn discover_skills_under_dir(
    dir: &std::path::Path,
    seen: &mut HashSet<String>,
    out: &mut Vec<SkillDto>,
) {
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
                let locked = BUILTIN_SKILLS.contains(&skill_id.as_str());
                out.push(SkillDto {
                    skill_id,
                    source_type: "path".into(),
                    path: skill_md.to_string_lossy().to_string(),
                    enabled: true,
                    locked,
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

/// Install a skill from a zip file into the primary global skills root.
///
/// The zip must contain exactly one top-level directory (the skill name) that
/// directly contains a `SKILL.md` file. If the skill already exists and
/// `force` is `false`, returns `SkillInstallOutcome::Conflict`. If `force` is
/// `true`, the existing directory is removed before extraction.
pub fn install_skill_from_zip(zip_path: &Path, force: bool) -> Result<SkillInstallOutcome, String> {
    let skills_root = primary_global_skills_root()?;
    install_skill_from_zip_into(zip_path, force, &skills_root)
}

fn install_skill_from_zip_into(
    zip_path: &Path,
    force: bool,
    skills_root: &Path,
) -> Result<SkillInstallOutcome, String> {
    let file = std::fs::File::open(zip_path).map_err(|e| format!("Cannot open zip file: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip archive: {e}"))?;

    let skill_id = find_skill_id_in_archive(&mut archive)?;

    let target_dir = skills_root.join(&skill_id);

    if target_dir.exists() {
        if !force {
            return Ok(SkillInstallOutcome::Conflict { skill_id });
        }
        std::fs::remove_dir_all(&target_dir)
            .map_err(|e| format!("Failed to remove existing skill '{skill_id}': {e}"))?;
    }

    std::fs::create_dir_all(skills_root)
        .map_err(|e| format!("Failed to create skills directory: {e}"))?;

    // Re-open archive since ZipArchive borrows the file through iteration.
    let file =
        std::fs::File::open(zip_path).map_err(|e| format!("Cannot re-open zip file: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Invalid zip archive on re-open: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {e}"))?;

        let entry_path = entry.mangled_name();
        let relative = match entry_path.strip_prefix(&skill_id) {
            Ok(r) => r.to_path_buf(),
            Err(_) => continue,
        };

        let dest = target_dir.join(&relative);

        if entry.is_dir() {
            std::fs::create_dir_all(&dest)
                .map_err(|e| format!("Failed to create directory {}: {e}", dest.display()))?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir: {e}"))?;
            }
            let mut out = std::fs::File::create(&dest)
                .map_err(|e| format!("Failed to create file {}: {e}", dest.display()))?;
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| format!("Failed to read zip entry content: {e}"))?;
            std::io::Write::write_all(&mut out, &buf)
                .map_err(|e| format!("Failed to write file {}: {e}", dest.display()))?;
        }
    }

    let skill_md = target_dir.join("SKILL.md");
    if !skill_md.is_file() {
        let _ = std::fs::remove_dir_all(&target_dir);
        return Err(format!(
            "Zip extracted but SKILL.md not found at expected location: {}",
            skill_md.display()
        ));
    }

    Ok(SkillInstallOutcome::Installed {
        skill: SkillDto {
            skill_id,
            source_type: "path".into(),
            path: skill_md.to_string_lossy().to_string(),
            enabled: true,
            locked: false,
        },
    })
}

/// Delete a skill by the path to its `SKILL.md` file.
/// Removes the entire parent directory of the given path.
pub fn delete_skill_by_path(skill_md_path: &str) -> Result<(), String> {
    let skill_md = Path::new(skill_md_path);
    let skill_dir = skill_md
        .parent()
        .ok_or_else(|| format!("Cannot determine skill directory from path: {skill_md_path}"))?;

    let skill_id = skill_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| format!("Cannot determine skill ID from path: {skill_md_path}"))?;

    if BUILTIN_SKILLS.contains(&skill_id.as_str()) {
        return Err(format!("Cannot delete built-in skill '{skill_id}'"));
    }

    if !skill_dir.exists() {
        return Err(format!(
            "Skill directory does not exist: {}",
            skill_dir.display()
        ));
    }

    std::fs::remove_dir_all(skill_dir).map_err(|e| {
        format!(
            "Failed to delete skill directory '{}': {e}",
            skill_dir.display()
        )
    })
}

/// Returns the primary global skills root where user-managed skills are installed.
fn primary_global_skills_root() -> Result<PathBuf, String> {
    let peekoo_home = peekoo_paths::peekoo_global_config_dir()?;
    Ok(skill_root_for_peekoo_home(&peekoo_home))
}

/// Scans the zip archive to find the single top-level skill directory name.
/// Validates that it contains a `SKILL.md` directly inside it.
fn find_skill_id_in_archive(
    archive: &mut zip::ZipArchive<std::fs::File>,
) -> Result<String, String> {
    let mut top_level_dirs: HashSet<String> = HashSet::new();
    let mut has_skill_md = false;

    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {e}"))?;
        let path = entry.mangled_name();
        let components: Vec<_> = path.components().collect();

        if components.is_empty() {
            continue;
        }

        let top = components[0].as_os_str().to_string_lossy().to_string();
        // Skip macOS metadata directories
        if top == "__MACOSX" {
            continue;
        }
        top_level_dirs.insert(top.clone());

        // Check for <skill-name>/SKILL.md at exactly depth 2
        if components.len() == 2 {
            let filename = components[1].as_os_str().to_string_lossy();
            if filename == "SKILL.md" {
                has_skill_md = true;
            }
        }
    }

    if top_level_dirs.len() != 1 {
        return Err(format!(
            "Zip must contain exactly one top-level directory (found: {}). \
             Expected structure: <skill-name>/SKILL.md",
            if top_level_dirs.is_empty() {
                "none".to_string()
            } else {
                top_level_dirs.into_iter().collect::<Vec<_>>().join(", ")
            }
        ));
    }

    if !has_skill_md {
        return Err(
            "Zip must contain SKILL.md directly inside the top-level directory. \
             Expected structure: <skill-name>/SKILL.md"
                .to_string(),
        );
    }

    Ok(top_level_dirs.into_iter().next().unwrap())
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
        assert!(discovered.iter().all(|s| !s.locked));
    }

    #[test]
    fn discover_skills_marks_builtin_skills_as_locked() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("skills");
        fs::create_dir_all(root.join("memory-manager")).unwrap();
        fs::write(root.join("memory-manager/SKILL.md"), "# memory").unwrap();
        fs::create_dir_all(root.join("peekoo-agent-skill")).unwrap();
        fs::write(root.join("peekoo-agent-skill/SKILL.md"), "# peekoo").unwrap();
        fs::create_dir_all(root.join("custom-skill")).unwrap();
        fs::write(root.join("custom-skill/SKILL.md"), "# custom").unwrap();

        let discovered = discover_skills_in_roots(&[root]);

        let mm = discovered
            .iter()
            .find(|s| s.skill_id == "memory-manager")
            .expect("memory-manager discovered");
        assert!(mm.locked);

        let pas = discovered
            .iter()
            .find(|s| s.skill_id == "peekoo-agent-skill")
            .expect("peekoo-agent-skill discovered");
        assert!(pas.locked);

        let custom = discovered
            .iter()
            .find(|s| s.skill_id == "custom-skill")
            .expect("custom-skill discovered");
        assert!(!custom.locked);
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

    // -------------------------------------------------------------------------
    // Helpers for zip-based tests
    // -------------------------------------------------------------------------

    fn make_zip(entries: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let file = std::fs::File::create(tmp.path()).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, content) in entries {
            zip.start_file(*name, options).unwrap();
            std::io::Write::write_all(&mut zip, content).unwrap();
        }
        zip.finish().unwrap();
        tmp
    }

    // -------------------------------------------------------------------------
    // install_skill_from_zip tests
    // -------------------------------------------------------------------------

    #[test]
    fn install_skill_rejects_zip_without_skill_md() {
        let zip = make_zip(&[("my-skill/README.md", b"hello")]);
        let result = install_skill_from_zip(zip.path(), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("SKILL.md"));
    }

    #[test]
    fn install_skill_rejects_zip_with_multiple_top_level_dirs() {
        let zip = make_zip(&[("skill-a/SKILL.md", b"# a"), ("skill-b/SKILL.md", b"# b")]);
        let result = install_skill_from_zip(zip.path(), false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("exactly one top-level directory")
        );
    }

    #[test]
    fn install_skill_extracts_to_target_dir() {
        let zip = make_zip(&[
            ("my-skill/SKILL.md", b"# my skill"),
            ("my-skill/extra.txt", b"extra"),
        ]);

        let tmp_root = tempfile::tempdir().unwrap();
        // Override the global skills root by pointing primary_global_skills_root
        // indirectly — we test the lower-level extract logic via a custom root.
        let skills_root = tmp_root
            .path()
            .join("workspace")
            .join(".agents")
            .join("skills");
        fs::create_dir_all(&skills_root).unwrap();

        // Call the internal extraction logic directly using a known root.
        let outcome = install_skill_from_zip_into(zip.path(), false, &skills_root).unwrap();
        match outcome {
            SkillInstallOutcome::Installed { skill } => {
                assert_eq!(skill.skill_id, "my-skill");
                assert!(skill.path.ends_with("SKILL.md"));
                assert!(Path::new(&skill.path).is_file());
                assert!(skills_root.join("my-skill").join("extra.txt").is_file());
            }
            SkillInstallOutcome::Conflict { .. } => panic!("expected Installed"),
        }
    }

    #[test]
    fn install_skill_returns_conflict_when_exists_and_no_force() {
        let zip = make_zip(&[("my-skill/SKILL.md", b"# my skill")]);
        let tmp_root = tempfile::tempdir().unwrap();
        let skills_root = tmp_root
            .path()
            .join("workspace")
            .join(".agents")
            .join("skills");
        fs::create_dir_all(skills_root.join("my-skill")).unwrap();
        fs::write(skills_root.join("my-skill").join("SKILL.md"), b"old").unwrap();

        let outcome = install_skill_from_zip_into(zip.path(), false, &skills_root).unwrap();
        assert!(
            matches!(outcome, SkillInstallOutcome::Conflict { skill_id } if skill_id == "my-skill")
        );
    }

    #[test]
    fn install_skill_replaces_when_force_true() {
        let zip = make_zip(&[("my-skill/SKILL.md", b"# new content")]);
        let tmp_root = tempfile::tempdir().unwrap();
        let skills_root = tmp_root
            .path()
            .join("workspace")
            .join(".agents")
            .join("skills");
        fs::create_dir_all(skills_root.join("my-skill")).unwrap();
        fs::write(skills_root.join("my-skill").join("SKILL.md"), b"old").unwrap();

        let outcome = install_skill_from_zip_into(zip.path(), true, &skills_root).unwrap();
        assert!(matches!(outcome, SkillInstallOutcome::Installed { .. }));
        let content = fs::read_to_string(skills_root.join("my-skill").join("SKILL.md")).unwrap();
        assert_eq!(content, "# new content");
    }

    #[test]
    fn install_skill_ignores_macos_metadata_dir() {
        let zip = make_zip(&[
            ("my-skill/SKILL.md", b"# skill"),
            ("__MACOSX/my-skill/._SKILL.md", b"metadata"),
        ]);
        let tmp_root = tempfile::tempdir().unwrap();
        let skills_root = tmp_root
            .path()
            .join("workspace")
            .join(".agents")
            .join("skills");
        fs::create_dir_all(&skills_root).unwrap();

        let outcome = install_skill_from_zip_into(zip.path(), false, &skills_root).unwrap();
        assert!(matches!(outcome, SkillInstallOutcome::Installed { .. }));
    }

    // -------------------------------------------------------------------------
    // delete_skill_by_path tests
    // -------------------------------------------------------------------------

    #[test]
    fn delete_skill_removes_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let skill_md = skill_dir.join("SKILL.md");
        fs::write(&skill_md, b"# skill").unwrap();

        delete_skill_by_path(skill_md.to_str().unwrap()).unwrap();
        assert!(!skill_dir.exists());
    }

    #[test]
    fn delete_skill_errors_when_not_found() {
        let result = delete_skill_by_path("/nonexistent/path/SKILL.md");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn delete_skill_rejects_builtin_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("memory-manager");
        fs::create_dir_all(&skill_dir).unwrap();
        let skill_md = skill_dir.join("SKILL.md");
        fs::write(&skill_md, b"# skill").unwrap();

        let result = delete_skill_by_path(skill_md.to_str().unwrap());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Cannot delete built-in skill 'memory-manager'")
        );
        assert!(skill_dir.exists());
    }
}
