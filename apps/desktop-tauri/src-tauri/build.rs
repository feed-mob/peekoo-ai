fn main() {
    prepare_acp_sidecar();
    tauri_build::build()
}

fn prepare_acp_sidecar() {
    use std::fs;
    use std::path::PathBuf;

    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=TARGET");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let Some(workspace_root) = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
    else {
        return;
    };

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.is_empty() {
        return;
    }

    let binary_name = if target.contains("windows") {
        "peekoo-agent-acp.exe"
    } else {
        "peekoo-agent-acp"
    };

    let source = workspace_root
        .join("target")
        .join(&profile)
        .join(binary_name);
    println!("cargo:rerun-if-changed={}", source.display());
    if !source.exists() {
        println!(
            "cargo:warning=peekoo-agent-acp sidecar not found at {}",
            source.display()
        );
        return;
    }

    let binaries_dir = manifest_dir.join("binaries");
    if std::fs::create_dir_all(&binaries_dir).is_err() {
        return;
    }

    let sidecar_name = if target.contains("windows") {
        format!("peekoo-agent-acp-{target}.exe")
    } else {
        format!("peekoo-agent-acp-{target}")
    };
    let destination = binaries_dir.join(sidecar_name);

    let source_bytes = match fs::read(&source) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!(
                "cargo:warning=failed to read peekoo-agent-acp sidecar {}: {}",
                source.display(),
                err
            );
            return;
        }
    };

    let destination_matches = fs::read(&destination)
        .map(|existing| existing == source_bytes)
        .unwrap_or(false);
    if destination_matches {
        return;
    }

    if let Err(err) = fs::write(&destination, source_bytes) {
        println!(
            "cargo:warning=failed to write peekoo-agent-acp sidecar to {}: {}",
            destination.display(),
            err
        );
    }
}
