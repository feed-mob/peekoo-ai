# Google Calendar Plugin

## Overview

Peekoo's Google Calendar integration is provided through a plugin. It does not ship with Google OAuth credentials.

This plugin is useful when you want Peekoo to work with your calendar data while keeping account ownership and credentials in your own hands.

## What You Need

- a Google account
- access to Google Cloud Console
- the Peekoo desktop app

## Setup Flow

1. Create or choose a Google Cloud project.
2. Enable the Google Calendar API.
3. Configure the OAuth consent screen.
4. Add yourself as a test user if the app is still in testing mode.
5. Create a `Desktop app` OAuth client.
6. Download the resulting `client.json`.
7. Open the Google Calendar panel in Peekoo.
8. Upload `client.json`.
9. Click `Connect` and complete Google sign-in.

## After Setup

After the connection succeeds, Peekoo can use the plugin to refresh and read calendar events. The exact UI surface may evolve over time, but the credential flow remains the same: your app instance uses the OAuth client you created.

## Important

Keep `client.json` private. Do not commit it to git or share it publicly.

## Common Errors

- Calendar API not enabled
- account not listed as a test user
- wrong credential type selected

Use `Desktop app`, not `Web application`.

## Privacy Note

Because the OAuth client belongs to you, you control the Google Cloud project, its test users, and its usage. That makes the setup more manual, but it also keeps account ownership explicit.
