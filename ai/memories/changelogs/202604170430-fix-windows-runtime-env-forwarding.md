## 2026-04-17 04:30: fix: forward Windows runtime environment variables

**What changed:**
- Updated runtime launch environment construction to also forward `USERPROFILE`, `APPDATA`, and `LOCALAPPDATA` when those variables are missing from runtime configuration.
- Added adapter tests covering Windows env forwarding and precedence over user-configured env vars.

**Why:**
- Some Windows runtime login flows expect standard user-profile and app-data environment variables even when Peekoo is launched from a stripped desktop environment.

**Files affected:**
- `crates/peekoo-agent-app/src/runtime_adapters/mod.rs`
