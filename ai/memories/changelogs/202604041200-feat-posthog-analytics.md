# feat: PostHog analytics integration

## Date
2026-04-04

## Summary
Integrated PostHog product analytics to track active users by app version and total installs. Created a dedicated `peekoo-analytics` crate for event definitions and provider configuration, keeping analytics concerns separated from the Tauri transport layer.

## Changes

### New crate: `crates/peekoo-analytics/`
- `events.rs`: Event name constants (`APP_STARTED`) and property builder (`app_started_properties`) with unit tests
- `posthog.rs`: `PostHogAnalyticsConfig` struct for API key and host URL management with unit tests

### Modified: `apps/desktop-tauri/src-tauri/`
- `Cargo.toml`: Added `tauri-plugin-posthog = "0.2"` and `peekoo-analytics` dependencies
- `capabilities/default.json`: Added `posthog:default` permission
- `src/lib.rs`: Registered PostHog plugin, fires `app_started` event on every launch with `app_version`, `os`, and `arch` properties

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

## CI/CD
- `POSTHOG_API_KEY` added to `release.yml` `publish-tauri` job env from GitHub Secrets
- Must add `POSTHOG_API_KEY` secret in GitHub repo settings before first release with analytics

## Testing
- 8 unit tests added to `peekoo-analytics` crate (TDD: red-green-refactor)
- All 359 workspace tests pass
- `just check` and `just lint` clean
