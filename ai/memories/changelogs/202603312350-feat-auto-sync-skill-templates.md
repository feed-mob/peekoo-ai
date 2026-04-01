# 2026-03-31 Auto-Sync Skill Templates via build.rs

## Problem
Skill templates were hardcoded — each skill required manual registration via `include_str!` and a `seed_if_missing` call. Adding a new bundled skill required modifying Rust code in two places.

## Solution
Created a `build.rs` script that auto-discovers all files under `templates/persona/.agents/skills/` at compile time and generates a Rust array of `(relative_path, content)` entries using `include_str!`. The bootstrap now syncs all discovered skills to the workspace, always overwriting so users receive updates.

## Changes

### 1. build.rs (new)
- Scans `templates/persona/.agents/skills/` recursively
- For each file found, emits a `("relative/path", include_str!("..."))` entry
- Generates `$OUT_DIR/skill_templates.rs` containing `SKILL_FILES` constant
- Watches skill templates for changes via `cargo:rerun-if-changed`

### 2. workspace_bootstrap.rs
- Removed hardcoded `MEMORY_MANAGER_SKILL_TEMPLATE` constant
- Added `mod skill_templates` that includes generated file
- Replaced single `seed_if_missing` call with `sync_skill_templates()` loop
- New `sync_skill_templates()` function always writes/overwrites all bundled skills

### 3. Directory structure
- Moved `templates/persona/skills/` to `templates/persona/.agents/skills/`

## Workflow for Adding New Skills

1. Create `templates/persona/.agents/skills/<name>/`
2. Add `SKILL.md` and any supporting files
3. Run `cargo build` — skills auto-discovered and embedded
4. App start syncs new skills to user workspaces

No Rust code changes required.

## Files Changed
- `crates/peekoo-agent-app/build.rs` (new)
- `crates/peekoo-agent-app/src/workspace_bootstrap.rs`
- `crates/peekoo-agent-app/templates/persona/.agents/skills/` (moved from skills/)
