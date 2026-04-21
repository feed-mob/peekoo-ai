use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const MANIFEST_FILE_NAME: &str = "manifest.json";

#[derive(Debug, Deserialize)]
struct BuiltinSpriteManifest {
    id: String,
    name: String,
    description: String,
}

#[derive(Debug)]
struct BuiltinSprite {
    id: String,
    name: String,
    description: String,
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let sprites_dir = manifest_dir.join("../../apps/desktop-ui/public/sprites");
    let builtins = discover_builtin_sprites(&sprites_dir);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set"));
    let dest_path = out_dir.join("builtin_sprites.rs");

    fs::write(&dest_path, render_builtin_sprites(&builtins)).unwrap_or_else(|err| {
        panic!("Failed to write {}: {err}", dest_path.display());
    });
}

fn discover_builtin_sprites(sprites_dir: &Path) -> Vec<BuiltinSprite> {
    println!("cargo:rerun-if-changed={}", sprites_dir.display());

    let entries = fs::read_dir(sprites_dir)
        .unwrap_or_else(|err| panic!("Failed to read {}: {err}", sprites_dir.display()));
    let mut builtins = Vec::new();

    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("Failed to read sprite entry: {err}"));
        let sprite_dir = entry.path();
        if !sprite_dir.is_dir() {
            continue;
        }

        let manifest_path = sprite_dir.join(MANIFEST_FILE_NAME);
        if !manifest_path.is_file() {
            continue;
        }

        println!("cargo:rerun-if-changed={}", manifest_path.display());

        let manifest_contents = fs::read_to_string(&manifest_path)
            .unwrap_or_else(|err| panic!("Failed to read {}: {err}", manifest_path.display()));
        let manifest: BuiltinSpriteManifest = serde_json::from_str(&manifest_contents)
            .unwrap_or_else(|err| panic!("Failed to parse {}: {err}", manifest_path.display()));

        builtins.push(BuiltinSprite {
            id: manifest.id,
            name: manifest.name,
            description: manifest.description,
        });
    }

    builtins.sort_by(|left, right| match (left.id == "dark-cat", right.id == "dark-cat") {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => left.id.cmp(&right.id),
    });

    builtins
}

fn render_builtin_sprites(builtins: &[BuiltinSprite]) -> String {
    let mut rendered = String::from("const BUILTIN_SPRITES: &[BuiltinSprite] = &[\n");

    for builtin in builtins {
        rendered.push_str(&format!(
            "    BuiltinSprite {{ id: {:?}, name: {:?}, description: {:?} }},\n",
            builtin.id, builtin.name, builtin.description,
        ));
    }

    rendered.push_str("];\n");
    rendered
}
