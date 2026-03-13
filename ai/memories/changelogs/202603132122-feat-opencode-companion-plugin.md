## 2026-03-13 21:22 feat: add OpenCode companion plugin and host bridge support

**What changed:**
- Added `plugins/peekoo-opencode-companion/`, a new Peekoo plugin that reads OpenCode session state from a bridge file, drives pet mood changes, and updates peek badges
- Added a bundled OpenCode companion plugin in `plugins/peekoo-opencode-companion/opencode-plugin/` plus host-side companion auto-install support via `[[companions]]` in plugin manifests
- Extended both plugin SDKs with `peekoo::bridge::read()` / `bridge.read()` and `peekoo::mood::set()` / `mood.set()` wrappers
- Added `MoodReactionService` and Tauri flush handling so plugins can trigger `pet:react` events through the host
- Enforced runtime permission checks for gated host functions and documented the new `bridge:fs_read` and `pet:mood` capabilities
- Hardened companion installation against path traversal and aligned OpenCode bridge paths with Peekoo's platform-specific data directory rules

**Why:**
- Peekoo needed a first-class integration with OpenCode so the pet can reflect active coding sessions with working, thinking, and happy states
- The bridge and mood APIs make the integration reusable for future plugins instead of baking OpenCode-specific behavior into the app layer
- Runtime permission checks and path validation close security gaps introduced by the new bridge and companion-install features

**Files affected:**
- `plugins/peekoo-opencode-companion/`
- `crates/peekoo-plugin-host/`
- `crates/peekoo-plugin-sdk/`
- `packages/plugin-sdk/assembly/`
- `crates/peekoo-notifications/`
- `crates/peekoo-agent-app/`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-ui/src/types/pet-event.ts`
- `apps/desktop-ui/src/hooks/use-sprite-reactions.ts`
- `docs/plugin-authoring.md`
- `docs/plans/2026-03-13-opencode-companion-plugin.md`
- `justfile`
