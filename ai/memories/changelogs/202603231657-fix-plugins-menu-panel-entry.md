## 2026-03-23 16:57: fix: show plugin panel entries in sprite plugins menu

**What changed:**
- Updated sprite action `Plugins` popup to render entries by plugin panel (not by plugin summary).
- Plugin entries now display panel title directly, so `mijia-smart-home` appears as `米家智能设备管理` in the menu.
- Added icon mapping for `mijia-smart-home` in the popup.

**Why:**
- Make plugin open entry clearer and ensure direct discoverability from the `Plugins` menu.

**Files affected:**
- apps/desktop-ui/src/components/sprite/SpriteActionMenu.tsx
- ai/memories/changelogs/202603231657-fix-plugins-menu-panel-entry.md
