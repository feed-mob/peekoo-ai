# Mijia Plugin: Python Companion → Rust/WASM Port

**Date**: 2026-04-09
**Status**: Completed
**License**: GPL-3.0 (inherited from upstream [mijia-api](https://github.com/Do1e/mijia-api))

## Problem

The Mijia Smart Home plugin currently shells out to a Python companion script (`companions/mijia_bridge.py`) via `peekoo::process::exec()`. This requires:
- A pre-built Python runtime with `mijiaAPI`, `requests`, `qrcode` installed
- Running `just plugin-install-mijia-python-sdk` to build the shared Python SDK
- Python runtime is ~30-50MB

Python-to-WASM compilation is not viable because:
- `pycryptodome` is a C extension (doesn't compile to WASM)
- `requests` depends on C extensions (`urllib3` SSL bindings)
- WASI target mismatch: Extism uses `wasm32-wasip1`, componentize-py targets `wasm32-wasip2`

## Solution

Port the ~1,370 lines of Python to native Rust running inside the WASM plugin. Use `peekoo::http::request()` host function for all HTTP calls (same pattern as Google Calendar plugin).

## Architecture

Everything in the plugin crate — no separate library needed since `peekoo::http::request()` is only available in WASM context.

```
plugins/mijia-smart-home/
├── Cargo.toml              (update deps, remove peekoo-python-sdk)
├── peekoo-plugin.toml      (add net:http, allowed_hosts)
├── src/
│   ├── lib.rs              (refactor: remove python exec, wire up MijiaClient)
│   ├── crypto.rs           (RC4, SHA1, SHA256, nonce generation)
│   ├── api.rs              (MijiaApi struct: auth, HTTP requests, all endpoints)
│   ├── device.rs           (DeviceInfo, miot-spec scraper, cache)
│   └── error.rs            (MijiaError, error code map)
├── ui/panel.html           (unchanged)
└── companions/             (DELETE)
```

## Source Mapping

| Python Module | Lines | Rust File | Rust Equivalent |
|---------------|-------|-----------|-----------------|
| `miutils.py` | ~60 | `crypto.rs` | `rc4`, `sha2`, `sha1`, `base64` crates |
| `apis.py` | ~600 | `api.rs` | `peekoo::http::request()` for HTTP |
| `devices.py` | ~280 | `device.rs` | `regex`, `serde_json` |
| `errors.py` | ~80 | `error.rs` | `thiserror` |

## Dependencies

```toml
[dependencies]
peekoo-plugin-sdk = { path = "../../crates/peekoo-plugin-sdk" }
extism-pdk = "1.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rc4 = "0.2"           # RC4 stream cipher
sha1 = "0.10"         # Request signature
sha2 = "0.10"         # Nonce signing
base64 = "0.22"       # Encoding
regex = "1"           # miot-spec HTML parsing
flate2 = "1"          # Gzip decompression for responses
```

## API Endpoints

All POST to `https://api.mijia.tech/app` with RC4-encrypted params:

| Endpoint | Purpose |
|----------|---------|
| `/v2/homeroom/gethome_merged` | List homes with rooms |
| `/home/home_device_list` | List devices per home |
| `/v2/home/device_list_page` | List shared devices |
| `/miotspec/prop/get` | Get device properties (batch) |
| `/miotspec/prop/set` | Set device properties (batch) |
| `/miotspec/action` | Execute device action |
| `/v2/user/statistics` | Get usage statistics |
| `/v2/message/v2/check_new_msg` | Token validity check |

Auth endpoints:
| Endpoint | Purpose |
|----------|---------|
| `https://account.xiaomi.com/pass/serviceLogin?...` | Token refresh |
| `https://account.xiaomi.com/longPolling/loginUrl` | QR login flow |
| `https://home.miot-spec.com/spec/{model}` | Device spec scraping |

## Auth Flow

1. **QR Login**: GET `serviceLogin` → GET `longPolling/loginUrl` → long-poll `lp` URL (120s) → extract auth tokens
2. **Token Refresh**: GET `serviceLogin` with cookies → follow `location` → update cookies
3. **Authenticated Request**: Generate nonce → sign with SHA256 → RC4-encrypt params → POST → RC4-decrypt response

## Manifest Changes

```toml
[permissions]
required = ["net:http", "state:read", "state:write"]

allowed_hosts = [
    "api.mijia.tech",
    "account.xiaomi.com",
    "home.miot-spec.com",
]
```

## Implementation Order

1. `error.rs` — Error types + error code map
2. `crypto.rs` — RC4, SHA1, SHA256, nonce functions
3. `device.rs` — miot-spec scraper + cache
4. `api.rs` — Auth flow + all API endpoints (bulk of work)
5. `lib.rs` — Refactor tool handlers
6. `Cargo.toml` — Update deps
7. `peekoo-plugin.toml` — Add permissions
8. Delete `companions/`

## Risks

- **HTTP Timeout**: QR long-poll uses 120s timeout. `peekoo::http::request()` may have shorter default. Need to test.
- **Cookie management**: Python `requests.Session` handles cookies automatically. Rust needs manual Cookie header construction.
- **Response decryption**: Some responses need gzip decompression before RC4 decrypt. `flate2` crate handles this.
