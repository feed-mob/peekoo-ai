# macOS private API usage note

`app.macOSPrivateApi` is enabled via macOS-specific config injection, not in the base config.

Why:
- This app uses a frameless + transparent main window on macOS.
- In our current Tauri/WRY setup, disabling private API usage causes the transparency behavior to regress (window appears opaque).

Risk:
- Private API usage may affect Apple notarization/review policies (especially Mac App Store distribution).

Current policy:
- Keep this enabled only for macOS builds because it is required for the transparent window behavior.
- If we adopt a non-private-API rendering path in the future, remove this setting and related dependency feature.

Configuration model:
- Base config (`tauri.conf.json`) keeps `app.macOSPrivateApi = false` to avoid static warnings.
- macOS-only overrides set `app.macOSPrivateApi = true`:
  - local dev/build via `tauri.macos.conf.json`
  - CI/release via `TAURI_CONFIG` environment injection on macOS runners

Operational guardrail:
- Failures when applying transparent background are logged via `tracing::warn!` in `src/lib.rs` (`apply_macos_transparent_background`).
