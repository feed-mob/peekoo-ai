## 2026-03-28 01:30: fix: stabilize Windows agent runtime TLS setup

**What changed:**
- Kept the desktop agent service and its `asupersync` runtime together in a `ManagedAgent` so prompts and model switches reuse the same runtime instead of recreating one per call
- Added shared process TLS initialization and a reusable `ensure_rustls_provider()` helper for the desktop agent path
- Added targeted logging around agent session creation and prompt failures to make Windows transport issues easier to diagnose
- Added a regression test proving process TLS initialization is safe to call more than once

**Why:**
- Windows agent prompts were failing with TLS connect errors and Winsock `10057`, which pointed to runtime/reactor lifetime mismatches during network setup
- Initializing Rustls once per process and keeping the agent session on one runtime reduces transport instability during provider calls

**Files affected:**
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-agent/src/service.rs`
- `crates/peekoo-agent/src/lib.rs`
- `crates/peekoo-agent/Cargo.toml`
- `apps/desktop-tauri/src-tauri/src/lib.rs`
- `Cargo.lock`
- `ai/memories/todo.md`
