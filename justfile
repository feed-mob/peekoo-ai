# Peekoo AI Development Commands

# Run Tauri desktop app in development mode
dev:
    cd ./apps/desktop-tauri/src-tauri/ && cargo tauri dev

# Build Tauri desktop app for production
build:
    cd ./apps/desktop-tauri/src-tauri/ && NO_STRIP=true cargo tauri build

# Build AppImage with linuxdeploy strip workaround
build-appimage:
    cd ./apps/desktop-tauri/src-tauri/ && NO_STRIP=true cargo tauri build --bundles appimage

# Install frontend dependencies with bun
install:
    cd ./apps/desktop-ui && bun install

# Check Rust code without building
check:
    cargo check

# Run all tests
test:
    cargo test

# Format all code
fmt:
    cargo fmt --all

# Lint Rust code
lint:
    cargo clippy --all-targets --all-features

# Clean build artifacts
clean:
    cargo clean
    rm -rf ./apps/desktop-ui/dist
    rm -rf ./apps/desktop-ui/node_modules

# Generate new Tauri icons from source image
icon SOURCE:
    cd ./apps/desktop-tauri/src-tauri/ && cargo tauri icon {{SOURCE}}

# List all available commands
list:
    @just --list
