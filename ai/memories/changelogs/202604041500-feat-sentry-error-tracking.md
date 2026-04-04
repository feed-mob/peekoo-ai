# feat: Sentry error tracking integration

## Date
2026-04-04

## Summary
Integrated Sentry error tracking to capture Rust panics, handled errors, and frontend JS errors with minidump support for native crashes. Uses `tauri-plugin-sentry` (v0.5) with curated `sentry` crate features and then refactored provider orchestration into `peekoo-analytics` to respect SRP.

## Changes

### New: `crates/peekoo-analytics/src/sentry.rs`
- Owns Sentry startup orchestration via `init()`, `client()`, and `guard()` helpers
- Stores the global `ClientInitGuard` in a static so the Tauri plugin can receive a `'static` client reference
- 3 unit tests covering disabled fallback client and guard behavior

### Modified: `crates/peekoo-analytics/`
- `Cargo.toml`: moved the heavy `sentry` dependency into the crate behind a `sentry` feature flag
- `src/lib.rs`: gated the `sentry` module behind the `sentry` feature

### Modified: `apps/desktop-tauri/src-tauri/`
- `Cargo.toml`: keeps direct Tauri plugin dependencies for permission discovery, but no longer contains analytics provider orchestration logic
- `capabilities/default.json`: Added `sentry:default` permission
- `src/lib.rs`: 
  - simplified to call adapter-layer Sentry startup before the Tauri builder
  - no longer references `tauri-plugin-sentry` APIs directly

### Follow-up adapter extraction
- added `crates/peekoo-analytics-tauri/` so `desktop-tauri` no longer directly references `tauri-plugin-sentry` or `tauri-plugin-posthog` APIs in code
- kept the plugin crates as direct `desktop-tauri` dependencies because Tauri permission discovery only sees direct plugin dependencies

### Modified: `.github/workflows/release.yml`
- Added `SENTRY_DSN` env from GitHub Secrets

## Design Decisions
- Sentry init happens before Tauri builder (required for panic hooks and minidump fork)
- Minidump enabled for native crash capture via separate crash reporter process
- Curated sentry features: `backtrace`, `contexts`, `panic`, `debug-images`, `reqwest`, `rustls`, `log` -- no `native-tls`, `test`, or `tracing`
- Frontend JS errors auto-captured via injected `@sentry/browser` (no npm package needed)
- Same env var pattern as PostHog -- disabled when `SENTRY_DSN` is absent
- Self-hosted Sentry uses the host embedded in the DSN, so no extra host env var is required
- SRP refactor: provider orchestration now lives in `peekoo-analytics`; `desktop-tauri` only wires plugins and startup ordering

## Testing
- 3 Sentry unit tests in `peekoo-analytics` crate
- All workspace tests pass
- `just check`, `just lint`, `just fmt` all clean

## Setup Required
- Add `SENTRY_DSN` secret in GitHub repo settings before next release
- Sentry free plan: 5K errors/month, 10M spans/month, 30-day retention
