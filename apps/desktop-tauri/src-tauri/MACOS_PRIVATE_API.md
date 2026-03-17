# macOS private API usage note

`app.macOSPrivateApi` is intentionally enabled in `tauri.conf.json`.

Why:
- This app uses a frameless + transparent main window on macOS.
- In our current Tauri/WRY setup, disabling private API usage causes the transparency behavior to regress (window appears opaque).

Risk:
- Private API usage may affect Apple notarization/review policies (especially Mac App Store distribution).

Current policy:
- Keep this enabled only because it is required for the current transparent window behavior.
- If we adopt a non-private-API rendering path in the future, remove this setting and related dependency feature.

Operational guardrail:
- Failures when applying transparent background are logged via `tracing::warn!` in `src/lib.rs` (`apply_macos_transparent_background`).
