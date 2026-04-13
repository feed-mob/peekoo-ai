## 2026-04-13 10:31: feat: add custom sprite import and agent-assisted manifest tooling

**What changed:**
- Added managed custom sprite storage under Peekoo's app data directory, plus helpers for sprite data paths.
- Extended `peekoo-app-settings` with a merged built-in/custom sprite catalog, sprite prompt/template generation, sprite image validation, manifest generation/validation, and custom sprite save/delete support.
- Added Tauri commands and MCP tools for custom sprite prompt retrieval, manifest draft generation, manifest validation, custom sprite save/delete, and loading uploaded manifests.
- Updated sprite loading in the desktop UI so built-in sprites still use bundled assets while custom sprites load via Tauri file asset URLs.
- Added a new settings flow for copying the sprite prompt, uploading an image, optionally loading a manifest, previewing the draft, validating it, saving it, and deleting saved custom sprites.
- Added Rust tests covering custom sprite draft generation, save/list behavior, and delete fallback behavior.

**Why:**
- Users need a first-class way to bring their own sprite sheets into Peekoo instead of being limited to bundled sprites.
- Peekoo-agent needs structured sprite tools so it can help users draft and adjust manifests after they upload a sprite image.
- Validating the image and manifest together reduces confusing rendering failures and gives users actionable feedback earlier in the flow.

**Files affected:**
- `ai/plans/2026-04-13-custom-sprite-support.md`
- `crates/peekoo-paths/src/lib.rs`
- `crates/peekoo-app-settings/Cargo.toml`
- `crates/peekoo-app-settings/src/dto.rs`
- `crates/peekoo-app-settings/src/lib.rs`
- `crates/peekoo-app-settings/src/service.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-mcp-server/src/handler.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/components/sprite/Sprite.tsx`
- `apps/desktop-ui/src/components/sprite/spriteAsset.ts`
- `apps/desktop-ui/src/features/settings/SettingsPanel.tsx`
- `apps/desktop-ui/src/features/settings/SpriteSelector.tsx`
- `apps/desktop-ui/src/features/settings/CustomSpriteManager.tsx`
- `apps/desktop-ui/src/features/settings/useGlobalSettings.ts`
- `apps/desktop-ui/src/types/global-settings.ts`
- `apps/desktop-ui/src/types/sprite.ts`
