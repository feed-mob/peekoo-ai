use crate::settings::dto::SkillDto;

pub fn discover_skills() -> Vec<SkillDto> {
    use std::collections::HashSet;

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut roots = Vec::new();

    if let Ok(current) = std::env::current_dir() {
        let mut cursor = current;
        loop {
            let candidate = cursor.join(".peekoo").join("skills");
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

    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".peekoo").join("skills"));
    }

    for root in roots {
        if !root.is_dir() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.filter_map(|x| x.ok()) {
                let path = entry.path();

                if path.is_dir() {
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
                    continue;
                }

                if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                    let skill_id = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    if seen.insert(skill_id.clone()) {
                        out.push(SkillDto {
                            skill_id,
                            source_type: "path".into(),
                            path: path.to_string_lossy().to_string(),
                            enabled: true,
                        });
                    }
                }
            }
        }
    }
    out
}
