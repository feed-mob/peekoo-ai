## 2026-03-18 12:40 feat: Add Claude Code companion plugin and generic fs host APIs

**What changed:**
- Added generic `fs:read` and `fs:read_dir` support to `peekoo-plugin-host`, including generic `allowed_paths` manifest configuration and optional `tail_bytes` reads
- Exposed the new filesystem APIs in both the Rust plugin SDK and the AssemblyScript plugin SDK
- Added a new AssemblyScript plugin at `plugins/peekoo-claude-code-companion` that polls Claude Code JSONL session files and drives Peekoo badges, moods, and notifications
- Added an Extism JS-based test harness for the new AssemblyScript plugin and a `just` recipe for building/installing it

**Why:**
- Peekoo needed a Claude Code companion similar to the existing OpenCode companion, but Claude Code session state is best read directly from local JSONL transcripts
- Generic filesystem host APIs make this possible without bridge files and are reusable for future plugins

**Files affected:**
- `crates/peekoo-plugin-host/src/manifest.rs`
- `crates/peekoo-plugin-host/src/host_functions.rs`
- `crates/peekoo-plugin-host/src/registry.rs`
- `crates/peekoo-plugin-sdk/src/host_fns.rs`
- `crates/peekoo-plugin-sdk/src/fs.rs`
- `crates/peekoo-plugin-sdk/src/lib.rs`
- `crates/peekoo-plugin-sdk/src/types.rs`
- `packages/plugin-sdk/assembly/host.ts`
- `packages/plugin-sdk/assembly/fs.ts`
- `packages/plugin-sdk/assembly/index.ts`
- `packages/plugin-sdk/assembly/types.ts`
- `plugins/peekoo-claude-code-companion/`
- `justfile`
