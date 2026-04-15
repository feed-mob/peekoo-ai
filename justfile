# Peekoo AI Development Commands

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# Run Tauri desktop app in development mode (trace logging by default)
dev:
    cargo build -p peekoo-agent-acp
    {{ if os_family() == "windows" { "$env:RUST_LOG='debug'; $env:PEEKOO_PROJECT_ROOT=(Get-Location).Path; Push-Location 'apps/desktop-tauri/src-tauri'; try { cargo tauri dev } finally { Pop-Location }" } else { "RUST_LOG=debug PEEKOO_PROJECT_ROOT=\"$PWD\"; cd apps/desktop-tauri/src-tauri && cargo tauri dev" } }}

# Build Tauri desktop app for production
build:
    cargo build --release -p peekoo-agent-acp
    {{ if os_family() == "windows" { "Push-Location 'apps/desktop-tauri/src-tauri'; try { cargo tauri build } finally { Pop-Location }" } else { "cd apps/desktop-tauri/src-tauri && cargo tauri build" } }}

# Build AppImage with linuxdeploy strip workaround
build-appimage:
    cargo build --release -p peekoo-agent-acp
    {{ if os_family() == "windows" { "Write-Error 'build-appimage is only supported from a POSIX shell on Linux.'; exit 1" } else { "cd apps/desktop-tauri/src-tauri && NO_STRIP=true cargo tauri build --bundles appimage" } }}

# Install all dependencies (frontend + Rust tools)
setup: install install-tools

# Install frontend dependencies with bun
install:
    bun install --cwd apps/desktop-ui

# Install required Rust CLI tools
install-tools:
    cargo install tauri-cli --version "^2"

# Check Rust code without building
check:
    cargo check

# Run all tests
test:
    cargo test
    python -m unittest scripts.tests.test_release

# Bump release versions without creating git refs
release-bump version:
    python ./scripts/release.py {{version}}

# Create a signed release branch, push it, and open a PR
release version:
    python ./scripts/release.py {{version}} --commit --push

# Format all code
fmt:
    cargo fmt --all

# Lint Rust code
lint:
    cargo clippy --all-targets --all-features

# Clean build artifacts
clean:
    cargo clean
    python -c "import shutil; shutil.rmtree('apps/desktop-ui/dist', ignore_errors=True); shutil.rmtree('apps/desktop-ui/node_modules', ignore_errors=True)"

# Generate new Tauri icons from source image
icon SOURCE:
    {{ if os_family() == "windows" { "Push-Location 'apps/desktop-tauri/src-tauri'; try { cargo tauri icon {{SOURCE}} } finally { Pop-Location }" } else { "cd apps/desktop-tauri/src-tauri && cargo tauri icon {{SOURCE}}" } }}

# Check the plugin SDK (wasm32-wasip1 target)
check-sdk:
    cargo check --manifest-path crates/peekoo-plugin-sdk/Cargo.toml

# Build a Rust plugin to WASM
plugin-build name:
    cargo build --release --target wasm32-wasip1 --manifest-path plugins/{{name}}/Cargo.toml

# Build an AssemblyScript plugin to WASM
plugin-build-as name:
    bun install --cwd plugins/{{name}}
    bun --cwd plugins/{{name}} run build

# Install a plugin into the local Peekoo plugin dir
plugin-install name:
    mkdir -p ~/.peekoo/plugins/{{name}}
    cp plugins/{{name}}/peekoo-plugin.toml ~/.peekoo/plugins/{{name}}/
    python -c "import pathlib, shutil, tomllib; src = pathlib.Path('plugins/{{name}}'); manifest = tomllib.loads((src / 'peekoo-plugin.toml').read_text()); wasm_rel = pathlib.Path(manifest['plugin']['wasm']); wasm_src = src / wasm_rel; wasm_dst = pathlib.Path.home() / '.peekoo' / 'plugins' / '{{name}}' / wasm_rel; wasm_dst.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(wasm_src, wasm_dst)"
    if [ -d plugins/{{name}}/ui ]; then cp -r plugins/{{name}}/ui ~/.peekoo/plugins/{{name}}/; fi
    if [ -d plugins/{{name}}/companions ]; then cp -r plugins/{{name}}/companions ~/.peekoo/plugins/{{name}}/; fi
    if [ -d plugins/{{name}}/runtime ]; then cp -r plugins/{{name}}/runtime ~/.peekoo/plugins/{{name}}/; fi
    if [ -d plugins/{{name}}/vendor ]; then cp -r plugins/{{name}}/vendor ~/.peekoo/plugins/{{name}}/; fi

# Install an AssemblyScript plugin into the local Peekoo plugin dir
plugin-install-as name:
    mkdir -p ~/.peekoo/plugins/{{name}}
    cp plugins/{{name}}/peekoo-plugin.toml ~/.peekoo/plugins/{{name}}/
    python -c "import pathlib, shutil, tomllib; src = pathlib.Path('plugins/{{name}}'); manifest = tomllib.loads((src / 'peekoo-plugin.toml').read_text()); wasm_rel = pathlib.Path(manifest['plugin']['wasm']); wasm_src = src / wasm_rel; wasm_dst = pathlib.Path.home() / '.peekoo' / 'plugins' / '{{name}}' / wasm_rel; wasm_dst.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(wasm_src, wasm_dst)"

# Build and install a Rust plugin
plugin name: (plugin-build name) (plugin-install name)

# Build and install Mijia plugin
plugin-mijia-smart-home: (plugin-build "mijia-smart-home") (plugin-install "mijia-smart-home")

# Build all maintained first-party plugins
plugin-build-all:
    just plugin-build health-reminders
    just plugin-build peekoo-opencode-companion

# Build the OpenCode Companion plugin (WASM + OpenCode JS companion)
plugin-build-opencode-companion:
    bun install --cwd plugins/peekoo-opencode-companion/opencode-plugin
    bun --cwd plugins/peekoo-opencode-companion/opencode-plugin run build
    python -c "import pathlib, shutil; dst = pathlib.Path('plugins/peekoo-opencode-companion/companions'); dst.mkdir(parents=True, exist_ok=True); shutil.copy2('plugins/peekoo-opencode-companion/opencode-plugin/dist/peekoo-opencode-companion.js', dst / 'peekoo-opencode-companion.js')"
    just plugin-build peekoo-opencode-companion

# Build and install the OpenCode Companion plugin
plugin-opencode-companion: plugin-build-opencode-companion (plugin-install "peekoo-opencode-companion")

# Build and install the Claude Code Companion plugin
plugin-claude-code-companion: (plugin-build-as "peekoo-claude-code-companion") (plugin-install-as "peekoo-claude-code-companion")

# List all available commands
list:
    @just --list
