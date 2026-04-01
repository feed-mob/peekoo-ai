use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let skills_dir = Path::new("templates/workspace/.agents/skills");
    println!("cargo:rerun-if-changed={}", skills_dir.display());

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest = Path::new(&out_dir).join("skill_templates.rs");
    let mut out = fs::File::create(&dest).expect("create skill_templates.rs");

    let mut entries = Vec::new();

    if skills_dir.is_dir() {
        collect_files(skills_dir, skills_dir, &mut entries);
    }

    entries.sort();

    writeln!(out, "pub const SKILL_FILES: &[(&str, &str)] = &[").unwrap();
    for (rel_path, abs_path) in &entries {
        // Emit rerun-if-changed for each file so edits trigger rebuild
        println!("cargo:rerun-if-changed={abs_path}");
        writeln!(out, "    ({rel_path:?}, include_str!({abs_path:?})),").unwrap();
    }
    writeln!(out, "];").unwrap();
}

fn collect_files(base: &Path, dir: &Path, entries: &mut Vec<(String, String)>) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, entries);
        } else if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .expect("strip prefix")
                .to_string_lossy()
                .replace('\\', "/");
            let abs = fs::canonicalize(&path)
                .expect("canonicalize")
                .to_string_lossy()
                .to_string();
            entries.push((rel, abs));
        }
    }
}
