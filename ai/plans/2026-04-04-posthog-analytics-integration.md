# PostHog Analytics Integration

## Goal

Track active users per app version and total installs using PostHog's free tier via `tauri-plugin-posthog`.

## Decisions

- **Single event approach:** Only `app_started` is sent. PostHog's built-in "First time" filter provides total installs without a separate `app_installed` event or local persistence flag.
- **No timestamp override:** PostHog server timestamps are more reliable than client clocks.
- **No user ID management:** The plugin auto-generates a stable `$device:{machine_uid}` distinct ID.
- **Dedicated `peekoo-analytics` crate:** Owns event definitions and provider config. Keeps analytics concerns out of `desktop-tauri` (transport layer) and positions for future Sentry integration.
- **PostHog API key embedded directly:** It is a public project key, not a secret.

## Architecture

```
peekoo-analytics (crate)
  ├── events.rs      # Event names + property builders (testable, no Tauri dep)
  └── posthog.rs     # PostHog config (api_key, api_host)

desktop-tauri (transport)
  └── lib.rs          # Plugin registration + fire app_started in setup()
```

Dependency flow: `desktop-tauri` -> `peekoo-analytics` (event definitions) + `tauri-plugin-posthog` (plugin wiring).

## PostHog Dashboard Queries

- **Active Users by Version:** Trends > `app_started` > Unique users > Breakdown by `app_version`
- **Total Installs:** Trends > `app_started` > Unique users > "First time" filter
- **OS Distribution:** Pie chart > `app_started` > Breakdown by `os`
- **Version Adoption:** Trends > `app_started` > Breakdown by `app_version` over time

## Free Tier Budget

1M events/month. At 1 event per launch per device:
- 1K DAU = ~30K events/month (3%)
- 10K DAU = ~300K events/month (30%)

## Future Extensions

- Add Sentry error tracking to `peekoo-analytics` crate
- Add frontend event tracking via `tauri-plugin-posthog-api` JS bindings (already installed)
- Add heartbeat events for session duration if needed
