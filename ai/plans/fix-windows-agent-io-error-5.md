# Plan: Fix Windows "IO error: 拒绝访问。 (os error 5)" in peekoo-agent

## Overview

The agent fails on Windows with `Error: Agent error: IO error: 拒绝访问。 (os error 5)` during prompting. This is a Windows `ERROR_ACCESS_DENIED` originating inside `pi_agent_rust` v0.1.7 during session file save operations. The error is intermittent/random because it depends on transient file locks held by antivirus, Windows Search indexer, or Windows Defender's Controlled Folder Access.

## Background

### Error Chain

```
application.rs:257  format!("Agent error: {e}")
  └── pi::error::Error::Io  (#[error("IO error: {0}")])
        └── std::io::Error  (os error 5 = ERROR_ACCESS_DENIED)
```

The `"Agent error:"` prefix (not `"Agent init error:"`) confirms this happens during `prompt_streaming`, not app startup.

### Previous Fix Attempts

1. **2026-03-13 13:00** — Fixed peekoo's own `peekoo.sqlite` dual-connection issue with WAL mode + `Arc<Mutex<Connection>>`. ✅ Still active and working.

2. **2026-03-13 16:00** — Added retry with exponential backoff to `pi_agent/src/session.rs` and `pi_agent/src/session_index.rs`. ❌ These patches were applied to local source files that no longer exist. The current dependency is `pi_agent_rust = "0.1.7"` from crates.io, which does **not** contain these patches.

### Root Cause: 5 Code Paths in `pi_agent_rust` v0.1.7

All lack retry logic and fail immediately on Windows `ERROR_ACCESS_DENIED`:

| # | File | Operation | Why it fails on Windows |
|---|------|-----------|------------------------|
| 1 | `session.rs:~1627` | `temp_file.persist(&path)` | Atomic rename (`MoveFileExW`) fails if target held by antivirus/indexer |
| 2 | `session.rs:~1717` | `OpenOptions::new().append(true).open(&path)` | File locked by concurrent save or antivirus scan |
| 3 | `session_index.rs:238-243` | `File::options().open(&self.lock_path)` | Lock file held by another operation |
| 4 | `session_index.rs:244` | `lock_file_guard(..., Duration::from_secs(5))` | 5s timeout too short for concurrent streaming saves |
| 5 | `config.rs:~1179` | `NamedTempFile::new_in(parent)?.persist(path)` | Atomic config write races with reads |

Most likely culprits are #1 and #2 — session file saves during streaming. During a streaming response, `pi_agent_rust` saves session state multiple times in quick succession. On Windows, if antivirus or Windows Search briefly holds the file, the operation fails immediately with no retry.

### Upstream Status

- `pi_agent_rust` v0.1.7 is the **latest version on crates.io** (published 2026-02-23).
- Versions 0.1.8–0.1.10 exist as GitHub releases but were **never published to crates.io**.
- The `main` branch `session_index.rs::with_lock()` and `lock_file_guard()` are **identical** to v0.1.7 — no Windows fixes upstream.
- v0.1.8–v0.1.10 changelogs contain no explicit Windows file locking fixes, but v0.1.8 has relevant improvements:
  - "Move index snapshot writes to background thread" — reduces lock contention
  - "Add `sync_all()` before atomic renames for crash safety"
  - "Defer session picker prune on permission errors"

## Goals

- [ ] Eliminate `IO error: 拒绝访问。 (os error 5)` on Windows during agent prompting
- [ ] Avoid introducing breaking changes to the agent API
- [ ] Keep the fix maintainable (not a large diverging fork)

## Approach: Phased

### Phase 1: Upgrade to `pi_agent_rust` v0.1.10 via git dependency

Switch from the crates.io v0.1.7 to the v0.1.10 git tag, which has the most relevant upstream improvements.

**Files to modify:**
- `crates/peekoo-agent/Cargo.toml`
- `crates/peekoo-agent-app/Cargo.toml`
- `crates/peekoo-agent-acp/Cargo.toml`

```toml
# Before
pi = { version = "0.1.7", package = "pi_agent_rust", default-features = false, features = [...] }

# After
pi = { git = "https://github.com/Dicklesworthstone/pi_agent_rust", tag = "v0.1.10", package = "pi_agent_rust", default-features = false, features = [...] }
```

**Risks:**
- API changes between 0.1.7 and 0.1.10 may require code updates
- Need to verify `create_agent_session`, `AgentSessionHandle`, `SessionOptions`, `AgentEvent`, `Session`, `SessionIndex`, `SessionMeta` APIs are still compatible
- `asupersync` version may have changed

**Verification:**
- `just check` — must compile clean
- `just test` — all tests must pass
- Manual Windows test: send multiple rapid messages and confirm no OS error 5

### Phase 2 (if Phase 1 doesn't fully resolve): Fork and patch `pi_agent_rust`

If upgrading to v0.1.10 doesn't eliminate the error, fork the repo and re-apply the retry patches from the 2026-03-13 changelog:

**Patches to apply (based on 2026-03-13 16:00 changelog):**

1. `session_index.rs` — Increase lock timeout from 5s to 30s:
   ```rust
   // Before
   let _lock = lock_file_guard(&lock_file, Duration::from_secs(5))?;
   // After
   let _lock = lock_file_guard(&lock_file, Duration::from_secs(30))?;
   ```

2. `session.rs` — Add retry with exponential backoff for file open:
   ```rust
   // Retry up to 10 times with 50ms * attempt backoff
   for attempt in 1..=10 {
       match OpenOptions::new().append(true).open(&path) {
           Ok(f) => { file = Some(f); break; }
           Err(e) if attempt < 10 => {
               std::thread::sleep(Duration::from_millis(50 * attempt));
           }
           Err(e) => return Err(Error::Io(Box::new(e))),
       }
   }
   ```

3. `session.rs` — Add retry for `temp_file.persist()`:
   ```rust
   // Retry up to 10 times with 50ms * attempt backoff
   let mut temp_opt = Some(temp_file);
   for attempt in 1..=10 {
       let temp = temp_opt.take().unwrap();
       match temp.persist(&path) {
           Ok(_) => break,
           Err(e) if attempt < 10 => {
               temp_opt = Some(e.file);
               std::thread::sleep(Duration::from_millis(50 * attempt));
           }
           Err(e) => return Err(Error::Io(Box::new(e.error))),
       }
   }
   ```

**Fork setup:**
```toml
pi = { git = "https://github.com/<our-fork>/pi_agent_rust", branch = "peekoo-windows-fixes", package = "pi_agent_rust", ... }
```

### Phase 3 (defensive fallback): Retry wrapper at app level

Add a retry wrapper in `application.rs::prompt_streaming` that catches `IO error` and retries the prompt up to 3 times with a short delay. This is a blunt instrument but provides a safety net for any remaining edge cases.

```rust
// In application.rs
let mut last_err = String::new();
for attempt in 1..=3 {
    match runtime.block_on(agent.prompt(message, on_event)) {
        Ok(result) => return Ok(result),
        Err(e) if e.to_string().contains("IO error") && attempt < 3 => {
            tracing::warn!("Agent IO error on attempt {attempt}, retrying: {e}");
            std::thread::sleep(std::time::Duration::from_millis(200 * attempt as u64));
            last_err = e.to_string();
        }
        Err(e) => return Err(e),
    }
}
Err(pi::error::Error::session(last_err))
```

**Note:** This approach re-runs the prompt, which is safe because the session state is restored from the persisted file on retry. However, it should only be used as a last resort since it adds latency.

## Implementation Steps

1. **Phase 1: Upgrade to v0.1.10**
   - [ ] Update all three `Cargo.toml` files to use git dependency
   - [ ] Run `just check` and fix any API incompatibilities
   - [ ] Run `just test` and fix any test failures
   - [ ] Deploy to Windows and test

2. **Phase 2: Fork + patch (if needed)**
   - [ ] Fork `pi_agent_rust` at v0.1.10
   - [ ] Apply retry patches to `session.rs` and `session_index.rs`
   - [ ] Update Cargo.toml to point to fork
   - [ ] Run `just check` and `just test`
   - [ ] Deploy to Windows and test

3. **Phase 3: App-level retry (if needed)**
   - [ ] Add retry wrapper in `application.rs::prompt_streaming`
   - [ ] Ensure retry only triggers on IO errors, not auth/provider errors
   - [ ] Add tracing log on retry attempts
   - [ ] Run `just test`

## Files to Modify

- `crates/peekoo-agent/Cargo.toml` — update pi dependency
- `crates/peekoo-agent-app/Cargo.toml` — update pi dependency
- `crates/peekoo-agent-acp/Cargo.toml` — update pi dependency
- `crates/peekoo-agent-app/src/application.rs` — (Phase 3 only) retry wrapper
- Possibly `crates/peekoo-agent/src/service.rs` — if API changed in v0.1.10

## Testing Strategy

- `just check` — compile clean on Linux
- `just test` — all tests pass
- Windows manual test: send 10+ rapid messages in succession, confirm no OS error 5
- Windows manual test: send a message while Windows Defender scan is running

## Open Questions

- Does v0.1.10 have any breaking API changes that affect `create_agent_session`, `AgentSessionHandle`, or `SessionOptions`? (Need to check when implementing Phase 1)
- Is the `asupersync` version compatible between v0.1.7 and v0.1.10? (Check `Cargo.lock` after upgrade)
- Should we submit the Windows retry patches upstream to `pi_agent_rust`? (Low priority, but good citizenship)
