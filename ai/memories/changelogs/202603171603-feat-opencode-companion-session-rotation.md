## 2026-03-17 16:03 feat: support rotating OpenCode badges across active sessions

**What changed:**
- Fixed `peekoo-opencode-companion` so repeated OpenCode turns keep a valid badge title instead of dropping the badge after the first completed message
- Reworked the OpenCode companion bridge writer to track state per `sessionID`, preserve remembered titles, and emit all active sessions in bridge payloads
- Updated the Rust plugin to deserialize multi-session bridge state and publish one badge item per active session so the existing UI rotation can cycle through them
- Hardened session completion handling so unknown `session.updated` events do not synthesize fake active work, terminal events without a `sessionID` can clear active sessions, and repeated idle events do not enqueue duplicate completions
- Replaced the single completion marker with a `completed_sessions` queue so Peekoo can emit one done notification per finished OpenCode session even when multiple sessions finish between Rust poll cycles
- Preserved the done reaction when one session finishes while other sessions remain active by suppressing the same-poll working mood refresh
- Fixed expanded peek badge rendering so clicking the badge shows every active OpenCode session even when multiple sessions share the same title
- Added a waiting-for-input state so OpenCode permission prompts and question requests switch Peekoo into reminder mood, batch notifications, and show `Needs input` in the badge until all pending requests are answered
- Added bridge staleness detection so interrupted or crashed OpenCode sessions silently clear after 30 seconds instead of leaving Peekoo stuck in a working or waiting state
- Added regression tests for repeated-turn title retention and concurrent-session badge output, rebuilt the bundled companion JS, and refreshed the release WASM artifact
- Bumped the plugin package versions to `0.1.2`

**Why:**
- Follow-up replies in the same OpenCode session were losing the badge because the bridge cleared the cached title during the idle transition
- Concurrent OpenCode sessions previously overwrote each other because the bridge only tracked one global session state at a time
- Recording all active sessions in the bridge lets Peekoo surface multiple active coding tasks without changing the desktop badge rotation behavior
- The first multi-session implementation could miss or duplicate done notifications because it stored only one completion marker and treated repeated terminal events as distinct completions
- The sprite reaction pipeline could overwrite a fresh done reaction with a working reaction in the same poll when another session was still active
- Duplicate badge labels caused the expanded badge list to collapse visually to one row because React keys were not unique
- Users can miss pending OpenCode permissions or question prompts when they are away from the terminal, so Peekoo should surface that blocked state proactively
- Interrupted OpenCode processes can leave stale bridge files behind, so the plugin needs to time out old active states instead of assuming every session exits cleanly

**Files affected:**
- `ai/memories/changelogs/202603171603-feat-opencode-companion-session-rotation.md`
- `plugins/peekoo-opencode-companion/opencode-plugin/peekoo-opencode-companion.ts`
- `plugins/peekoo-opencode-companion/opencode-plugin/peekoo-opencode-companion.test.ts`
- `plugins/peekoo-opencode-companion/companions/peekoo-opencode-companion.js`
- `plugins/peekoo-opencode-companion/src/lib.rs`
- `plugins/peekoo-opencode-companion/target/wasm32-wasip1/release/peekoo_opencode_companion.wasm`
- `apps/desktop-ui/src/components/sprite/SpritePeekBadge.tsx`
- `plugins/peekoo-opencode-companion/Cargo.toml`
- `plugins/peekoo-opencode-companion/peekoo-plugin.toml`
- `plugins/peekoo-opencode-companion/opencode-plugin/package.json`
