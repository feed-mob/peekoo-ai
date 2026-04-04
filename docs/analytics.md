# Analytics

This project uses PostHog for product analytics and Sentry for error tracking.

## Dashboards

- Sentry project: <https://s-sentry.feedmob.info/organizations/feedmob/issues/?project=56#welcome>
- PostHog project: <https://us.posthog.com/project/367700/dashboard/1427579>

## Build-Time Configuration

These values are injected during release builds from GitHub Actions secrets:

- `POSTHOG_API_KEY`
- `POSTHOG_API_HOST`
- `SENTRY_DSN`

## Notes

- PostHog tracks the `app_started` event with `app_version`, `os`, and `arch`.
- Install counts are derived in PostHog using the built-in "First time" filter.
- Self-hosted Sentry is configured entirely through `SENTRY_DSN`; the DSN already includes the host.
- The Tauri adapter layer lives in `crates/peekoo-analytics-tauri/`. The app crate still keeps direct Tauri plugin dependencies so capability permissions like `posthog:default` and `sentry:default` remain discoverable at build time.
