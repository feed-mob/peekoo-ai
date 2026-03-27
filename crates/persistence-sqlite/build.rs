use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let migrations_dir = Path::new(&manifest_dir).join("migrations");
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("migrations.rs");

    println!("cargo:rerun-if-changed=migrations");

    let mut entries: Vec<_> = fs::read_dir(&migrations_dir)
        .unwrap_or_else(|e| panic!("Cannot read migrations dir {:?}: {e}", migrations_dir))
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "sql"))
        .collect();

    entries.sort_by_key(|e| e.file_name());

    let mut generated = String::from("pub static MIGRATIONS: &[MigrationDef] = &[\n");

    for entry in &entries {
        let file_name = entry.file_name();
        let file_name_str = file_name.to_str().unwrap();
        let stem = file_name_str.strip_suffix(".sql").unwrap();

        let sql_content = fs::read_to_string(entry.path())
            .unwrap_or_else(|e| panic!("Cannot read {file_name_str}: {e}"));

        let meta = parse_metadata(&sql_content);

        let strategy_str = meta.strategy.unwrap_or_else(|| {
            panic!("Migration {file_name_str} missing -- @migrate: (create|alter) header")
        });
        let strategy = match strategy_str {
            "create" => "MigrationStrategy::Create",
            "alter" => "MigrationStrategy::Alter",
            other => panic!(
                "Migration {file_name_str} has unknown strategy '{other}'; expected 'create' or 'alter'"
            ),
        };

        let id = meta.id.unwrap_or(stem);

        let sentinel = match meta.sentinel {
            Some(s) => format!("Some(\"{s}\")"),
            None => "None".to_string(),
        };

        let tolerates_items: Vec<String> = meta
            .tolerates
            .iter()
            .map(|t| format!("\"{}\"", t.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();
        let tolerates = if tolerates_items.is_empty() {
            "&[]".to_string()
        } else {
            format!("&[{}]", tolerates_items.join(", "))
        };

        let sql_path = format!("{}/migrations/{}", manifest_dir, file_name_str);

        generated.push_str(&format!(
            "    MigrationDef {{ id: \"{id}\", sql: include_str!(\"{sql_path}\"), strategy: {strategy}, sentinel: {sentinel}, tolerates: {tolerates} }},\n",
        ));
    }

    generated.push_str("];\n");

    fs::write(&dest_path, generated).unwrap_or_else(|e| {
        panic!("Cannot write {}: {e}", dest_path.display());
    });
}

struct Metadata<'a> {
    strategy: Option<&'a str>,
    id: Option<&'a str>,
    sentinel: Option<&'a str>,
    tolerates: Vec<&'a str>,
}

fn parse_metadata(sql: &str) -> Metadata<'_> {
    let mut strategy = None;
    let mut id = None;
    let mut sentinel = None;
    let mut tolerates = Vec::new();

    for line in sql.lines() {
        let line = line.trim();

        // Stop parsing metadata at first non-comment, non-blank line
        if !line.starts_with("--") && !line.is_empty() {
            break;
        }

        let Some(rest) = line.strip_prefix("--") else {
            continue;
        };
        let rest = rest.trim();

        if let Some(val) = rest.strip_prefix("@migrate:") {
            strategy = Some(val.trim());
        } else if let Some(val) = rest.strip_prefix("@id:") {
            id = Some(val.trim());
        } else if let Some(val) = rest.strip_prefix("@sentinel:") {
            sentinel = Some(val.trim());
        } else if let Some(val) = rest.strip_prefix("@tolerates:") {
            tolerates = parse_tolerates(val.trim());
        }
    }

    Metadata {
        strategy,
        id,
        sentinel,
        tolerates,
    }
}

fn parse_tolerates(input: &str) -> Vec<&str> {
    // Parse: "err1", "err2", "err3"
    let mut result = Vec::new();
    let mut remaining = input;

    while let Some(start) = remaining.find('"') {
        let after_start = &remaining[start + 1..];
        if let Some(end) = after_start.find('"') {
            result.push(&after_start[..end]);
            remaining = &after_start[end + 1..];
        } else {
            break;
        }
    }

    result
}
