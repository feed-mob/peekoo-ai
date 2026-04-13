# Plan: Custom Sprite Support

## Overview

Add first-class custom sprite support so users can copy a sprite-generation prompt, upload a sprite image, optionally upload a manifest, generate a starter manifest, validate both the image and manifest, preview the result, and let Peekoo-agent help adjust manifest values before saving and activating the custom sprite.

## Goals

- [x] Add managed storage for custom sprites under Peekoo's app data directory
- [x] Merge built-in and custom sprites into a single selectable sprite catalog
- [x] Validate uploaded sprite images and return structured errors and warnings
- [x] Support both uploaded manifests and generated starter manifests
- [x] Expose custom sprite workflows through Tauri commands and MCP tools for Peekoo-agent
- [x] Add settings UI for prompt copy, upload, preview, manifest editing, save, activate, and delete
- [x] Add tests for backend validation, catalog merging, and frontend custom sprite flows

## Design

### Approach

- Keep built-in sprites bundled in `apps/desktop-ui/public/sprites` and introduce a managed custom sprite library under `peekoo_global_data_dir()/sprites`.
- Add a dedicated custom sprite service in the app settings layer to own file storage, manifest generation, manifest validation, image validation, and catalog scanning.
- Extend the existing settings/app/MCP APIs with explicit custom sprite operations instead of overloading generic settings commands.
- Let the UI and Peekoo-agent operate on draft sprites first, then finalize into installed custom sprites only after validation succeeds.

### Components

- `peekoo-paths`: sprite data and draft directory helpers
- `peekoo-app-settings`: merged sprite catalog, custom sprite storage, image validation, manifest generation and validation
- `peekoo-agent-app` / Tauri: command surface for list/import/generate/validate/save/delete/activate
- `peekoo-mcp-server`: custom sprite MCP tools for Peekoo-agent
- `desktop-ui settings`: prompt copy, image/manifest upload, live preview, manifest form, validation display
- `peekoo-agent skill/tooling`: agent-assisted manifest drafting and adjustment via MCP tools

## Implementation Steps

1. **Path + Storage Foundations**
   - Add path helpers for installed custom sprites and sprite drafts
   - Define custom sprite storage layout under app data
   - Keep built-in sprite IDs valid without migrating current users

2. **Backend Custom Sprite Domain**
   - Extend `SpriteInfo` with source metadata for built-in vs custom
   - Add custom sprite scanning and merged catalog listing
   - Add save/delete/get operations for custom sprites
   - Make active sprite validation work against the merged catalog

3. **Image Validation + Manifest Drafting**
   - Add file type, decode, dimension, grid divisibility, alpha/chroma-key, and blank-frame validation
   - Add starter manifest generation from uploaded image metadata and user-provided draft settings
   - Add manifest validation with blocking errors and non-blocking warnings

4. **Tauri Commands + App API**
   - Add commands for prompt/template retrieval, image import, manifest import, draft generation, validation, save, list, delete, and activate
   - Keep transport handlers thin and delegate logic into app settings services

5. **MCP Tooling for Peekoo-Agent**
   - Add tools for prompt retrieval, manifest template retrieval, image import, draft generation, validation, update, save, delete, and activate
   - Ensure tool responses are structured enough for iterative manifest tuning conversations

6. **Settings UI Flow**
   - Add a custom sprite section with prompt copy and upload actions
   - Add draft manifest form and inline validation results
   - Add preview rendering using the current draft manifest
   - Add custom sprite list actions for activate and delete

7. **Verification + Documentation**
   - Add Rust tests for catalog merging, validation, and custom sprite persistence
   - Add frontend tests for draft and upload flows where practical
   - Record changelog after implementation completes

## Files to Modify/Create

- `ai/plans/2026-04-13-custom-sprite-support.md`
- `crates/peekoo-paths/src/lib.rs`
- `crates/peekoo-app-settings/src/dto.rs`
- `crates/peekoo-app-settings/src/service.rs`
- `crates/peekoo-app-settings/src/lib.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-mcp-server/src/handler.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/types/global-settings.ts`
- `apps/desktop-ui/src/types/sprite.ts`
- `apps/desktop-ui/src/features/settings/useGlobalSettings.ts`
- `apps/desktop-ui/src/features/settings/SettingsPanel.tsx`
- `apps/desktop-ui/src/features/settings/SpriteSelector.tsx`
- new custom sprite UI/helper files under `apps/desktop-ui/src/features/settings/`

## Testing Strategy

- `cargo test -p peekoo-app-settings`
- `cargo test -p peekoo-agent-app`
- `cargo test -p peekoo-mcp-server`
- `bun test` for targeted sprite/settings tests in `apps/desktop-ui`
- `bun run build` in `apps/desktop-ui`
- `just check`

## Open Questions

- Whether v1 should support only PNG or also WebP imports
- Whether draft sprites need persistence across app restarts or can remain in-memory until save
- Whether custom sprite editing after save should reuse the draft workflow or write directly to installed manifests
