## 2026-03-12 10:00 fix: chat history not loading on cold start

**What changed:**
- Removed CWD filter in `load_last_session`; now passes `None` to `list_sessions` instead of `Some(&workspace_cwd)`
- Removed the `load_last_session_filters_to_workspace` test (was testing a broken feature)
- `load_last_session` signature simplified: `workspace_dir` parameter removed

**Why:**
- The CWD filter introduced in PR #69 was broken by design: pi's `SessionHeader::new()` records the OS process CWD (e.g. `/home/richard`) but peekoo was filtering by `workspace_dir` (e.g. `~/.config/peekoo`) — a path that never matches, so `list_sessions` always returned 0 results on cold start
- The warm path (in-memory agent) masked the bug within a single run
- Removing the filter is safe: `session_dir` is already scoped to `peekoo_global_data_dir()/sessions` — no other app writes there, so no cross-workspace leakage is possible in a desktop app context

**Files affected:**
- `crates/peekoo-agent-app/src/conversation.rs`
- `crates/peekoo-agent-app/src/application.rs`

**Note:** The previous changelog `202603112233` described workspace-scoped filtering as intentional — that approach was correct in design but unimplementable without forking pi, so it has been superseded by this simpler fix.
