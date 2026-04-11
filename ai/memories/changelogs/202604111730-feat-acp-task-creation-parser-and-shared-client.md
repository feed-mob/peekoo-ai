## 2026-04-11 17:30: feat: ACP task creation parser and shared client

**What changed:**
- Added ACP prompt-based task creation parsing via `peekoo-agent-acp` using a new `task_creation_parse` payload mode.
- Updated task creation in `peekoo-agent-app` to parse with ACP first, then fallback to the existing regex parser.
- Added parser output normalization and validation (priority/assignee guards, timestamp parsing, string trimming, label normalization).
- Added ACP prompt timeout for task parsing to avoid blocking on slow/failed runtimes.
- Extracted duplicated ACP subprocess/session/prompt flow into a shared helper module and reused it from both scheduler task execution and task-creation parsing.
- Added tests for ACP prompt context detection and parser normalization helpers.

**Why:**
- Improve natural-language task input quality by leveraging agent parsing while keeping reliability through permanent fallback.
- Reduce maintenance risk by centralizing ACP client orchestration logic.
- Keep UI and Tauri command contracts unchanged while evolving backend parsing behavior.

**Files affected:**
- `ai/plans/2026-04-11-task-creation-acp-parser.md`
- `crates/peekoo-agent-acp/src/context.rs`
- `crates/peekoo-agent-acp/src/agent.rs`
- `crates/peekoo-agent-app/src/acp_client.rs`
- `crates/peekoo-agent-app/src/agent_scheduler.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-agent-app/Cargo.toml`
