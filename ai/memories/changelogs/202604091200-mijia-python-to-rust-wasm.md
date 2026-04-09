# 2026-04-09 — Mijia plugin: Python companion → Rust/WASM port

## Summary
Ported the Mijia Smart Home plugin's Python companion script to native Rust code running inside the WASM plugin. This eliminates the Python runtime dependency and makes the plugin fully self-contained.

## Changes

### New Files
- `plugins/mijia-smart-home/src/crypto.rs` — RC4 (1024-byte discard), SHA1 request signing, SHA256 nonce signing, URL encoding
- `plugins/mijia-smart-home/src/api.rs` — MijiaApi struct with auth flow (QR login, token refresh), all API endpoints (homes, devices, properties, actions, statistics)
- `plugins/mijia-smart-home/src/device.rs` — Device info scraper (miot-spec.com), device property/action metadata, caching via `peekoo::state`
- `plugins/mijia-smart-home/src/error.rs` — MijiaError enum, Xiaomi API error code map

### Modified Files
- `plugins/mijia-smart-home/src/lib.rs` — Refactored: removed `process:exec` / Python companion pattern, wired up direct Rust API calls
- `plugins/mijia-smart-home/Cargo.toml` — Added `rc4`, `sha1`, `sha2`, `base64`, `regex`, `flate2`; removed `peekoo-python-sdk`; added `license = "GPL-3.0"`
- `plugins/mijia-smart-home/peekoo-plugin.toml` — Added `net:http`, `state:read`, `state:write` permissions; added `allowed_hosts` for Xiaomi API domains; removed `process:exec`

### Deleted
- `plugins/mijia-smart-home/companions/` — Python bridge script and requirements.txt no longer needed

## Technical Details
- All HTTP via `peekoo::http::request()` host function (same pattern as Google Calendar plugin)
- Auth data persisted via `peekoo::state::get/set` instead of file I/O
- RC4 implementation uses `rc4` crate with `new_from_slice` for variable-length keys
- miot-spec scraper extracts JSON from HTML `data-page` attribute via regex
- Device info cached in plugin state keyed by `mijia-devinfo:{model}`
- License: GPL-3.0 (inherited from upstream [mijia-api](https://github.com/Do1e/mijia-api))

## Known Limitations
- HTTP timeout for QR long-poll depends on host function default (may need tuning)
- Cookie management is manual (no session cookie jar)
- No Python runtime fallback (fully native now)
