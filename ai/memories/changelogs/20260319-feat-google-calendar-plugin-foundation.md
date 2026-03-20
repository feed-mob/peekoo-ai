# 2026-03-19 - Google Calendar plugin foundation

- Added host-managed Google Calendar OAuth support with PKCE, including Google provider URL generation and token exchange support in `crates/peekoo-agent-auth`.
- Added a new app-layer Google Calendar service that stores OAuth tokens securely, fetches primary calendar events, buckets them into upcoming/today/week views, caches sync state, and emits reminder notifications for imminent timed events.
- Added Tauri commands for Google Calendar connect, status polling, disconnect, and panel snapshot retrieval so plugin panel UIs can use host-managed auth safely.
- Added a new first-party `plugins/google-calendar/` plugin with a dedicated panel UI for upcoming, daily, and weekly agenda views.
- Replaced hardcoded Google OAuth defaults with user-supplied `client.json` upload parsing and secure storage for client id and client secret.
- Refactored Google Calendar sync and token refresh to use async reqwest calls without nested Tokio runtime creation, fixing runtime-drop panics during OAuth completion and refresh.
- Added test coverage for calendar bucketing/reminder logic, Google OAuth authorize URL generation, optional client secret token exchange forms, and Google `client.json` parsing.
