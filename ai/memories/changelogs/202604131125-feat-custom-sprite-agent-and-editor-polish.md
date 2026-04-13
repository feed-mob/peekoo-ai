## 2026-04-13 11:25: feat: polish custom sprite flow with agent draft, full editor, and load fixes

**What changed:**
- Added and iterated custom sprite support across settings, app services, Tauri commands, and MCP tools so users can upload images, generate manifests, validate/save sprites, and activate them.
- Added localized custom-sprite UI strings and updated the sprite generation prompt in all supported locales to use stricter sheet/background requirements.
- Added agent-backed manifest generation after image upload using an ephemeral no-session agent call, with deterministic fallback when agent generation fails.
- Added full manifest JSON editing in settings alongside form fields, plus generation status and copy success toasts.
- Fixed local preview and saved custom sprite loading by serving file-backed sprite images as data URLs instead of restricted `asset://` paths.
- Fixed custom image filename sanitization for non-ASCII names and added image-format fallback detection by content bytes.
- Relaxed non-even grid divisibility checks from hard errors to warnings in backend validation and removed warnings UI display in the custom sprite panel.

**Why:**
- Users need a reliable end-to-end custom sprite workflow, including multilingual guidance and meaningful manifest defaults.
- Agent-assisted manifest drafting improves first-pass quality while still allowing full user control via JSON editing.
- Tauri asset scope restrictions and filename edge cases were causing preview/load failures for real user files.

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
- `apps/desktop-ui/src/locales/en.json`
- `apps/desktop-ui/src/locales/zh.json`
- `apps/desktop-ui/src/locales/zh-TW.json`
- `apps/desktop-ui/src/locales/ja.json`
- `apps/desktop-ui/src/locales/es.json`
- `apps/desktop-ui/src/locales/fr.json`
