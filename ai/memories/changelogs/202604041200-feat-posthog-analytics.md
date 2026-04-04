# feat: PostHog analytics integration

## Date
2026-04-04

## Summary
Integrated PostHog product analytics to track active users by app version and total installs. Created a dedicated `peekoo-analytics` crate for event definitions and provider configuration, then followed up with a small SRP refactor to move PostHog capture payload construction out of `desktop-tauri`.

## Changes

### New crate: `crates/peekoo-analytics/`
- `events.rs`: Event name constants (`APP_STARTED`) and property builder (`app_started_properties`) with unit tests
- `posthog.rs`: `PostHogAnalyticsConfig` plus `PostHogCapture`/`app_started_capture()` helpers for provider-owned payload construction

### New crate: `crates/peekoo-analytics-tauri/`
- `posthog.rs`: Tauri adapter that registers `tauri-plugin-posthog` and translates core capture payloads into plugin request types
- `sentry.rs`: Tauri adapter that initializes minidump support and registers `tauri-plugin-sentry`

### Modified: `apps/desktop-tauri/src-tauri/`
- `Cargo.toml`: Added `tauri-plugin-posthog = "0.2"` and `peekoo-analytics` dependencies
- `capabilities/default.json`: Added `posthog:default` permission
- `src/lib.rs`: Registered PostHog plugin and maps `peekoo-analytics` capture payloads into `tauri-plugin-posthog` requests

### Modified: `apps/desktop-ui/`
- `package.json`: Added `tauri-plugin-posthog-api` for future frontend event tracking

### Modified: workspace
- `Cargo.toml`: Added `peekoo-analytics` to workspace members

## Design Decisions
- Single `app_started` event instead of separate `app_installed` -- PostHog's "First time" filter handles install counting server-side
- No client timestamps -- server receive time is more reliable
- Device identity handled automatically by plugin via `machine-uid` crate
- PostHog free tier (1M events/month) is sufficient for current scale
- API key injected via `POSTHOG_API_KEY` compile-time env var (`option_env!`), not hardcoded -- analytics disabled in local dev and open-source forks
- SRP follow-up: PostHog event payload construction now lives in `peekoo-analytics`, leaving `desktop-tauri` with transport wiring only
- Tauri plugin glue was extracted again into `peekoo-analytics-tauri`; direct plugin dependencies remain in `desktop-tauri` only for Tauri permission discovery

## CI/CD
- `POSTHOG_API_KEY` added to `release.yml` `publish-tauri` job env from GitHub Secrets
- Must add `POSTHOG_API_KEY` secret in GitHub repo settings before first release with analytics

## Testing
- 10 PostHog-related tests in `peekoo-analytics` after the payload-helper refactor
- All 359 workspace tests pass
- `just check` and `just lint` clean
