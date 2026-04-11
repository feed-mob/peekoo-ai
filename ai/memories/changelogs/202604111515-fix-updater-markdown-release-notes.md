## 2026-04-11 15:15: fix: render updater release notes as markdown in in-app dialog

**What changed:**
- Replaced native updater confirmation prompt usage (`@tauri-apps/plugin-dialog` `ask`) with an in-app update dialog that renders release notes via `Streamdown`.
- Added release-note normalization utility to strip leading generated HTML comments, normalize newlines, and trim empty content before rendering.
- Refactored updater flow to separate concerns: check for update metadata (`checkForAppUpdates`) and install/relaunch action (`installAppUpdate`).
- Wired the main app window to fetch update metadata on startup and show the new dialog, including install-in-progress and install-error states.
- Added update download/install progress UI in the dialog using Tauri updater download events (`Started`, `Progress`, `Finished`), with percentage and byte counters.
- Added estimated remaining download time (ETA) based on observed throughput and elapsed download time.
- Added a "View full changelog" action that opens the GitHub release page in the system browser.
- Added a dev-only forced updater dialog mode (via Vite env vars) so markdown rendering and changelog-link UI can be tested quickly in local dev without publishing a real update.
- Moved updater UX into a dedicated `panel-updater` window (larger dimensions) so markdown changelog content is readable on small sprite/main window sizes.
- Added/updated tests and i18n keys for updater dialog copy (`versionAvailable`, `installing`).

**Why:**
- GitHub release notes are markdown; native system dialogs only support plain text, causing malformed changelog rendering in the update popup.
- The in-app dialog preserves markdown structure and links, improving readability and update UX.

**Files affected:**
- `apps/desktop-ui/src/lib/updater.ts`
- `apps/desktop-ui/src/lib/release-notes.ts`
- `apps/desktop-ui/src/lib/release-notes.test.ts`
- `apps/desktop-ui/src/features/about/UpdatePromptDialog.tsx`
- `apps/desktop-ui/src/main.tsx`
- `apps/desktop-ui/src/locales/en.json`
- `apps/desktop-ui/src/locales/ja.json`
- `apps/desktop-ui/src/locales/zh.json`
- `apps/desktop-ui/src/locales/zh-TW.json`
- `apps/desktop-ui/src/locales/es.json`
- `apps/desktop-ui/src/locales/fr.json`
