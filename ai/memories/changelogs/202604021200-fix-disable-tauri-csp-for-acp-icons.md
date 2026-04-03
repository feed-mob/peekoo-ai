---
title: "fix: temporarily disable Tauri CSP for ACP runtime icons"
date: 2026-04-02
author: opencode
tags: [fix, tauri, csp, acp, icons]
---

## Summary

Temporarily disabled Content Security Policy (CSP) in the Tauri configuration to resolve ACP runtime icon loading issues on Windows and macOS.

## Problem

- ACP registry and runtime icons are loaded from remote URLs (`https://cdn.agentclientprotocol.com/`)
- Tauri's default CSP policy blocks remote image loading in the webview
- Icons display correctly in Linux development mode (CSP enforcement differs) but fail to load in packaged Windows/macOS builds

## Solution

- Set `app.security.csp` to `null` in `tauri.conf.json` to disable CSP entirely
- This is a temporary unblock while investigating cross-platform asset-loading issues

## Files Changed

- `apps/desktop-tauri/src-tauri/tauri.conf.json`

## Testing

- Frontend production build passes (`bun run build` in `apps/desktop-ui`)
- ACP icons should now load on Windows/macOS packaged builds

## Security Note

- Disabling CSP removes a security layer
- This is a short-term fix; plan to reintroduce a least-privilege CSP once asset loading issues are resolved

## Related

- ACP registry integration feature
- PR: (to be assigned after creation)
