## 2026-03-18 13:00: feat: about tray updater panel and release finalization flow

**What changed:**
- Added a new `About Peekoo` system tray menu item that emits `open-about` and opens a dedicated `panel-about` window from the frontend
- Added `AboutView`, `AboutPanel`, `useAboutPanel`, and `about-state` to show the installed app name/version, latest available version, release date, release notes, and updater actions
- Added focused tests for tray opening and About updater state loading in `SpriteView.test.ts` and `about-state.test.ts`
- Granted `core:app:default` to the desktop capability set so the frontend can call `getName()` and `getVersion()`
- Updated the release workflow to support a `finalize_release_tag` manual path that publishes a draft release and marks it as GitHub latest
- Updated the macOS release build to generate `dmg,updater` artifacts so future releases can support in-app updates on macOS
- Resolved the feature branch conflict with `master` by merging the latest settings-panel tray changes into the About panel branch

**Why:**
- Users needed a visible place to compare the currently installed version with the latest available release and trigger update actions without waiting for the startup prompt
- The tray menu is the most discoverable place for app-level metadata and update controls in this desktop app
- The Tauri updater was blocked by GitHub's `latest` pointer falling behind published releases, so the release flow now includes an explicit finalization step
- macOS self-update needed updater-specific artifacts instead of only a DMG bundle

**Files affected:**
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `apps/desktop-tauri/src-tauri/capabilities/default.json`
- `apps/desktop-tauri/src-tauri/gen/schemas/capabilities.json`
- `.github/workflows/release.yml`
- `docs/release.md`
- `apps/desktop-ui/src/types/window.ts`
- `apps/desktop-ui/src/routing/resolve-view.tsx`
- `apps/desktop-ui/src/views/SpriteView.tsx`
- `apps/desktop-ui/src/views/SpriteView.test.ts`
- `apps/desktop-ui/src/views/AboutView.tsx` (new)
- `apps/desktop-ui/src/features/about/AboutPanel.tsx` (new)
- `apps/desktop-ui/src/features/about/useAboutPanel.ts` (new)
- `apps/desktop-ui/src/features/about/about-state.ts` (new)
- `apps/desktop-ui/src/features/about/about-state.test.ts` (new)
