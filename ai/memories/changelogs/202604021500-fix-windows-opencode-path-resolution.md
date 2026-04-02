---
title: "fix(windows): resolve bundled opencode path for both .cmd and .exe variants"
date: 2026-04-02
author: opencode
tags: [fix, windows, bundling, opencode, acp]
---

## Summary

Fixed Windows users not seeing OpenCode as a built-in ACP provider in settings by checking for both `opencode.cmd` and `opencode.exe` in the bundled resources directory.

## Problem

- The bundling strategy switched from direct binary fetch (`opencode.exe` from GitHub releases) to npm-based wrapper (`opencode.cmd`) in commit `cc461e2`
- `resolve_bundled_opencode_path` only looked for `opencode.cmd` on Windows
- Users who installed a version built with the old strategy still had `opencode.exe` in their resources
- These users never got the OpenCode row seeded into the database, so it didn't appear in settings

## Solution

- Updated `resolve_bundled_opencode_path` to try both `opencode.cmd` and `opencode.exe` on Windows (in that order — prefer the new npm wrapper, fall back to the legacy direct binary)
- This handles users across different release versions without requiring a migration

## Files Changed

- `apps/desktop-tauri/src-tauri/src/lib.rs`

## Testing

- `cargo check -p peekoo-desktop-tauri` passes
