## 2026-03-11 22:33 fix: workspace-scoped chat history and new-session invalidation

**What changed:**
- Scoped chat history restore to the active `.peekoo` workspace instead of the global session pool
- Prevented in-flight prompts from restoring an invalidated agent after `New Chat`
- Preserved assistant text block whitespace when rebuilding chat history
- Added Rust regression tests for workspace filtering, whitespace preservation, and generation invalidation
- Disabled the `New Chat` button while a response is streaming

**Why:**
- Restoring the globally newest session could leak context across unrelated workspaces
- Starting a new chat during an active prompt could silently resurrect the old session
- Trimming and concatenating text blocks corrupted restored prose and Markdown formatting

**Files affected:**
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/conversation.rs`
- `crates/peekoo-agent-app/Cargo.toml`
- `apps/desktop-ui/src/features/chat/ChatPanel.tsx`
