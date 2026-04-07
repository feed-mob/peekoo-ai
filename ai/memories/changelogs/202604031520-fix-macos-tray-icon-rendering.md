## 2026-04-03 15:20: fix: macOS tray icon rendering

**What changed:**
- Disabled macOS template rendering for the Tauri tray icon so the app uses the bundled icon as a regular image instead of coercing it into a template glyph.

**Why:**
- The current tray icon asset is a full-color application icon. On macOS, forcing `icon_as_template(true)` caused the menu bar icon to render incorrectly, while Windows and Linux continued to display it normally.

**Files affected:**
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `ai/memories/changelogs/202604031520-fix-macos-tray-icon-rendering.md`
