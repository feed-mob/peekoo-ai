## 2026-04-21 17:30 feat: Built-in Sprite Auto-Discovery

**What changed:**
- Added a `build.rs` step in `peekoo-app-settings` to discover built-in sprites from `apps/desktop-ui/public/sprites/*/manifest.json` at build time
- Replaced the hardcoded built-in sprite catalog in `AppSettingsService` with the generated manifest-backed list
- Added a regression test covering auto-discovered built-in sprites and fixed the `snoopy` manifest image filename mismatch

**Why:**
- New built-in sprite folders should appear in the selectable sprite catalog automatically without requiring Rust source edits for each sprite
- The `snoopy` sprite needed its manifest image path corrected so it can load successfully once discovered

**Files affected:**
- `crates/peekoo-app-settings/build.rs`
- `crates/peekoo-app-settings/Cargo.toml`
- `crates/peekoo-app-settings/src/service.rs`
- `apps/desktop-ui/public/sprites/snoopy/manifest.json`
- `ai/memories/changelogs/202604211730-feat-builtin-sprite-auto-discovery.md`
