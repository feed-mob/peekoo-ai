## 2026-03-17 16:03 feat: support rotating OpenCode badges across active sessions

**What changed:**
- Fixed `peekoo-opencode-companion` so repeated OpenCode turns keep a valid badge title instead of dropping the badge after the first completed message
- Reworked the OpenCode companion bridge writer to track state per `sessionID`, preserve remembered titles, and emit all active sessions in bridge payloads
- Updated the Rust plugin to deserialize multi-session bridge state and publish one badge item per active session so the existing UI rotation can cycle through them
- Added regression tests for repeated-turn title retention and concurrent-session badge output, rebuilt the bundled companion JS, and refreshed the release WASM artifact
- Bumped the plugin package versions to `0.1.2`

**Why:**
- Follow-up replies in the same OpenCode session were losing the badge because the bridge cleared the cached title during the idle transition
- Concurrent OpenCode sessions previously overwrote each other because the bridge only tracked one global session state at a time
- Recording all active sessions in the bridge lets Peekoo surface multiple active coding tasks without changing the desktop badge rotation behavior

**Files affected:**
- `plugins/peekoo-opencode-companion/opencode-plugin/peekoo-opencode-companion.ts`
- `plugins/peekoo-opencode-companion/opencode-plugin/peekoo-opencode-companion.test.ts`
- `plugins/peekoo-opencode-companion/companions/peekoo-opencode-companion.js`
- `plugins/peekoo-opencode-companion/src/lib.rs`
- `plugins/peekoo-opencode-companion/target/wasm32-wasip1/release/peekoo_opencode_companion.wasm`
- `plugins/peekoo-opencode-companion/Cargo.toml`
- `plugins/peekoo-opencode-companion/peekoo-plugin.toml`
- `plugins/peekoo-opencode-companion/opencode-plugin/package.json`
