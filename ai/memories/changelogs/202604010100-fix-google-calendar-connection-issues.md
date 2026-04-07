---
title: "fix: Google Calendar plugin connection issues"
date: 2026-04-01
author: opencode
tags: [fix, google-calendar, plugin]
---

## Summary

Fixed two issues preventing users from connecting Google Calendar plugin:

1. **Removed Origin header from Google API requests** - The `Origin: http://localhost:1455` header was being sent with server-to-server HTTP calls to Google Calendar APIs, causing "Origin header is not a valid URL" errors. This header is only relevant for browser CORS requests, not reqwest HTTP client calls.

2. **Added upload success feedback** - Users received no visual confirmation after uploading client.json credentials file.

## Files Changed

- `plugins/google-calendar/src/lib.rs` - Removed Origin header from `fetch_google_calendar_list()` and `create_google_event()`
- `plugins/google-calendar/ui/panel.js` - Added success message after client.json upload

## Testing

- All 17 plugin unit tests pass
- Plugin builds and installs successfully via `just plugin google-calendar`

## Related

- Closes #154
- PR: #179
