## 2026-05-05 16:49: fix: macos about updater fallback and release manifest merging

**What changed:**
- Updated the About panel state loading to keep app name and version visible even when updater checks fail, and to suppress the known macOS missing-platform updater error instead of surfacing a generic `Unknown error`
- Added a shared frontend error message helper so string and object-shaped exceptions render consistently in the About panel
- Granted `core:window:allow-show` in the default Tauri capability set to remove ACL failures when showing windows on macOS
- Changed the release workflow so per-platform build jobs no longer upload their own `latest.json`, and instead upload platform assets for a final manifest merge step
- Added a release helper script that scans GitHub release assets, builds a single merged updater `latest.json`, and fails the workflow if required macOS, Linux, or Windows updater platforms are missing
- Added tests covering updater failure fallback and macOS missing-platform suppression in the About state layer

**Why:**
- The macOS About panel was failing even when local app metadata was available because updater errors were allowed to poison the entire snapshot load
- The real updater failure came from release metadata that omitted macOS updater platforms, so silencing the UI alone would not prevent the bug from returning on future releases
- Generating one merged `latest.json` from the final release asset set prevents matrix jobs from overwriting each other and makes missing platform coverage fail fast during release

**Files affected:**
- `apps/desktop-ui/src/features/about/about-state.ts`
- `apps/desktop-ui/src/features/about/about-state.test.ts`
- `apps/desktop-ui/src/features/about/useAboutPanel.ts`
- `apps/desktop-ui/src/lib/error-message.ts`
- `apps/desktop-tauri/src-tauri/capabilities/default.json`
- `apps/desktop-tauri/src-tauri/gen/schemas/capabilities.json`
- `.github/workflows/release.yml`
- `scripts/build_updater_manifest.py`
