# Plan: Sentry Error Tracking Integration

## Overview

Add Sentry error tracking to Peekoo using `tauri-plugin-sentry` (v0.5) + `sentry` (v0.42). Captures both Rust backend and frontend JS errors in a unified view, with minidump support for native crashes. DSN injected via compile-time env var (`SENTRY_DSN`), same pattern as PostHog.

## Goals

- [x] Track Rust panics and handled errors
- [x] Track frontend JS errors (via injected `@sentry/browser`)
- [x] Support native crash dumps (minidump)
- [x] Keep analytics concerns in `peekoo-analytics` crate
- [x] Disable in local dev and open-source forks (env var gated)

## Architecture

```
peekoo-analytics
  ├── events.rs       # Shared analytics event definitions
  ├── posthog.rs      # PostHog config helpers
  └── sentry.rs       # Sentry init/client/guard helpers

peekoo-analytics-tauri
  ├── posthog.rs      # Tauri PostHog adapter helpers
  └── sentry.rs       # Tauri Sentry adapter helpers

desktop-tauri (transport)
  └── lib.rs          # Calls adapter crate helpers only
```

Key ordering:
1. `peekoo_analytics::sentry::init()` -- must run before Tauri builder to capture startup panics
2. `minidump::init()` -- must run before Tauri builder (forks crash reporter process)
3. `tauri::Builder::default()` -- registers Sentry plugin for browser event forwarding

## Sentry Features

| Feature | Status |
|---------|--------|
| Rust panic capture | Enabled via `panic` feature |
| Stack traces | Enabled via `backtrace` feature |
| OS/device context | Enabled via `contexts` feature |
| Log breadcrumbs | Enabled via `log` feature |
| Minidump crash dumps | Enabled via `minidump` feature |
| Frontend JS errors | Auto-injected via plugin |
| Unified breadcrumbs | Merged from Rust + browser |

## Sentry Feature Flags

```toml
sentry = { version = "0.42", default-features = false, features = [
    "backtrace",
    "contexts",
    "panic",
    "debug-images",
    "reqwest",
    "rustls",
    "log",
] }
```

## Sentry Free Plan Limits

| Resource | Limit |
|----------|-------|
| Errors | 5K/month |
| Performance spans | 10M/month |
| Users | 1 |
| Retention | 30 days |

## Setup Required

1. Sign up at https://sentry.io (free developer plan)
2. Create a Rust project, copy the DSN
3. Add `SENTRY_DSN` secret in GitHub repo settings
4. Next release build will have Sentry enabled

## Files Modified/Created

- `crates/peekoo-analytics/Cargo.toml` -- Added `sentry` feature flag and moved `sentry` dependency into the crate
- `crates/peekoo-analytics/src/sentry.rs` -- Sentry init/client/guard helpers with static lifetime handling
- `crates/peekoo-analytics/src/lib.rs` -- Gated `sentry` module behind a crate feature
- `apps/desktop-tauri/src-tauri/Cargo.toml` -- Kept only `tauri-plugin-sentry`, enabled `peekoo-analytics` `sentry` feature
- `apps/desktop-tauri/src-tauri/capabilities/default.json` -- Added `sentry:default`
- `apps/desktop-tauri/src-tauri/src/lib.rs` -- Simplified to call analytics helpers and register plugin
- `.github/workflows/release.yml` -- Added `SENTRY_DSN` env from GitHub Secrets

## Notes

- Self-hosted Sentry does not need a separate host env var. The DSN already embeds the host, for example `https://<key>@sentry.example.com/<project>`.
- SRP refactor: provider orchestration now lives in `peekoo-analytics`, leaving `desktop-tauri` responsible only for transport wiring.
- Tauri plugin registration now lives in `peekoo-analytics-tauri`. `desktop-tauri` still keeps the direct plugin dependencies so Tauri can discover `sentry:default` permission metadata at build time.
