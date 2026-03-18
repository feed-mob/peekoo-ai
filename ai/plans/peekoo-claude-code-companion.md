# Plan: peekoo-claude-code-companion

## Overview

A Peekoo plugin that tracks active Claude Code sessions and reacts with moods, badges, and
notifications — analogous to `peekoo-opencode-companion` but reading Claude Code's JSONL
transcript files directly instead of using a hook-written bridge file.

Requires two new SDK host functions (`fs:read`, `fs:read_dir`) in both the Rust and
AssemblyScript SDKs. The plugin itself is written in AssemblyScript (the first AS plugin in
the repo).

## Goals

- [ ] Add `fs:read` and `fs:read_dir` host functions to `peekoo-plugin-host`
- [ ] Expose both in the Rust SDK (`crates/peekoo-plugin-sdk`)
- [ ] Expose both in the AssemblyScript SDK (`packages/plugin-sdk`)
- [ ] Build `plugins/peekoo-claude-code-companion` as an AS WASM plugin
- [ ] Plugin polls `~/.claude/projects/` every 2 s, infers session status from JSONL tails
- [ ] Drives Peekoo moods / badges / notifications on status changes

## Design

### Architecture

```
Claude Code writes JSONL              Peekoo WASM plugin (polls every 2 s)
──────────────────────────            ────────────────────────────────────
~/.claude/projects/                   fs:read_dir → discover project dirs
  <project-dir>/                      fs:read_dir → list .jsonl files
    <session-id>.jsonl     ──────►    fs:read     → read JSONL content
                                      infer_status() → mood/badge/notify
```

No hook scripts. No bridge file. Direct filesystem polling via new SDK capabilities.

### Session Status Inference (from JSONL tail)

Parse JSONL lines from the end, find the last meaningful `type:"assistant"` or
`type:"user"` (non-meta) record:

| Last record | Inferred status |
|---|---|
| `assistant` + `stop_reason:"end_turn"` | `done` |
| `assistant` + `stop_reason:"tool_use"` | `working` |
| `assistant` + `stop_reason:null` (streaming) | `working` |
| `user` without `toolUseResult` field | `waiting` |
| `user` with `toolUseResult` field | `working` |
| File mtime > 60 s ago AND last was working | `idle` (stale) |
| No files found / no recent files | `idle` |

Session title: the `slug` field from any JSONL line that carries it
(e.g. `"reactive-jumping-moon"` — Claude Code's human-readable session name).

"Potentially active" = JSONL file modified within last 300 s (5 min).

### Multi-session Aggregation

Same logic as OpenCode companion:
- Any session `waiting` → aggregate `waiting`
- Any session `working` → aggregate `working`
- All sessions `done` / `idle` → aggregate `idle` / notify completions

### Status → Peekoo Actions

| Status | Mood trigger | Badge | Notification |
|---|---|---|---|
| `working` | `claude-working` (sticky) | label: "Claude Code", value: slug or "Working..." | — |
| `waiting` | `claude-reminder` (non-sticky) | value: "Needs input" | "Claude Code needs your input" |
| `done` | `claude-done` (non-sticky) | cleared | "✅ `<slug>` is done!" |
| `idle` | `claude-idle` (non-sticky) | cleared | — |

### Security Model: `allowed_paths`

New optional manifest field `[permissions] allowed_paths = [...]`. The host validates that
any path passed to `peekoo_fs_read` / `peekoo_fs_read_dir` starts with one of the declared
prefixes (after `~` expansion and canonicalization). Traversal outside these prefixes is
rejected.

This plugin declares:
```toml
[permissions]
allowed_paths = ["~/.claude/projects/"]
```

## Implementation Steps

### Step 1: Manifest — `allowed_paths`
- [ ] Add `allowed_paths: Vec<String>` to `PermissionsBlock` in
  `crates/peekoo-plugin-host/src/manifest.rs`

### Step 2: `HostContext` — `allowed_paths`
- [ ] Add `allowed_paths: Vec<String>` to `HostContext` struct in
  `crates/peekoo-plugin-host/src/host_functions.rs`
- [ ] Populate it in `registry.rs` from `manifest.permissions.allowed_paths`,
  expanding `~` to the real home dir at load time

### Step 3: Host — two new functions
- [ ] Implement `fn host_fs_read(...)` in `host_functions.rs`:
  - Requires `fs:read` capability
  - Validates path starts with an `allowed_paths` prefix
  - Input: `{ "path": "...", "tail_bytes": 8192 }` — `tail_bytes` optional; if set, reads
    only the last N bytes of the file (like `tail -c N`), enabling efficient JSONL tail reads
    without loading multi-MB files into WASM memory
  - Returns `{ "content": "<string>" }` or `{ "content": null }`
- [ ] Implement `fn host_fs_read_dir(...)` in `host_functions.rs`:
  - Same path validation
  - Returns `{ "entries": [{ "name": "...", "is_dir": bool, "modified_secs": u64 }] }`
  - Returns `{ "entries": [] }` on missing dir or permission error
- [ ] Register both in `build_host_functions` vec

### Step 4: Rust SDK
- [ ] Add request/response structs + 2 extern declarations in
  `crates/peekoo-plugin-sdk/src/host_fns.rs`
- [ ] Create `crates/peekoo-plugin-sdk/src/fs.rs` with `read()` and `read_dir()`
- [ ] Wire up in `crates/peekoo-plugin-sdk/src/lib.rs`

### Step 5: AssemblyScript SDK
- [ ] Add 2 `@external` declarations in `packages/plugin-sdk/assembly/host.ts`
- [ ] Create `packages/plugin-sdk/assembly/fs.ts` with `FsEntry`, `read()`, `readDir()`
  - `readDir` must parse the entries JSON array without `JSON.parse` using existing
    hand-rolled helpers
- [ ] Add `FsEntry` class to `packages/plugin-sdk/assembly/types.ts`
- [ ] Export `fs` module from `packages/plugin-sdk/assembly/index.ts`

### Step 5.5: Test harness
- [ ] Add a JS test harness for the AssemblyScript plugin using Extism's JS runtime
- [ ] Cover JSONL tail parsing, status inference, completion detection, and multi-session
  aggregation with fixture-driven tests
- [ ] Add targeted tests for `tail_bytes` behavior in host `fs:read`

### Step 6: Plugin scaffolding
- [ ] Create `plugins/peekoo-claude-code-companion/package.json`
- [ ] Create `plugins/peekoo-claude-code-companion/tsconfig.json`
- [ ] Create `plugins/peekoo-claude-code-companion/asconfig.json`
- [ ] Create `plugins/peekoo-claude-code-companion/peekoo-plugin.toml`

### Step 7: Plugin logic (`assembly/index.ts`)
- [ ] `plugin_init`: set up 2-second poll schedule, clear badges
- [ ] `on_event`: dispatch `schedule:fired` → `poll_claude_code()`
- [ ] `poll_claude_code()`: discover sessions, infer statuses, aggregate, act
- [ ] `infer_status(jsonl: string, mtime_secs: u64): SessionState`
- [ ] Status change handlers: working / waiting / done / idle
- [ ] Completion deduplication via plugin state (keyed by
  `"<session_id>:<end_turn_timestamp>"`)

### Step 8: Justfile recipe
- [ ] Add `plugin-claude-code-companion` recipe (build + install via AS path)

## Files to Create / Modify

### Modified
- `crates/peekoo-plugin-host/src/manifest.rs` — `allowed_paths` in `PermissionsBlock`
- `crates/peekoo-plugin-host/src/host_functions.rs` — `HostContext`, register + impl 2 fns
- `crates/peekoo-plugin-host/src/registry.rs` — populate `allowed_paths`
- `crates/peekoo-plugin-sdk/src/host_fns.rs` — structs + 2 extern declarations
- `crates/peekoo-plugin-sdk/src/lib.rs` — wire `fs` module
- `packages/plugin-sdk/assembly/host.ts` — 2 `@external` declarations
- `packages/plugin-sdk/assembly/types.ts` — add `FsEntry` class
- `packages/plugin-sdk/assembly/index.ts` — export `fs`
- `Justfile` — add recipe

### Created
- `crates/peekoo-plugin-sdk/src/fs.rs`
- `packages/plugin-sdk/assembly/fs.ts`
- `plugins/peekoo-claude-code-companion/package.json`
- `plugins/peekoo-claude-code-companion/tsconfig.json`
- `plugins/peekoo-claude-code-companion/asconfig.json`
- `plugins/peekoo-claude-code-companion/peekoo-plugin.toml`
- `plugins/peekoo-claude-code-companion/assembly/index.ts`

## Claude Code JSONL Format Reference

Session files live at: `~/.claude/projects/<encoded-cwd>/<session-id>.jsonl`

Each line is a JSON object. Relevant fields:

```jsonc
{
  "type": "assistant" | "user" | "progress" | "file-history-snapshot",
  "subtype": null,                    // no subtypes observed in practice
  "message": {
    "stop_reason": "end_turn" | "tool_use" | null,
    "role": "assistant" | "user"
  },
  "toolUseResult": "...",             // present on user lines that are tool results
  "isMeta": true,                     // skip these user lines (local command output etc.)
  "slug": "reactive-jumping-moon",    // Claude's session name (appears on most lines)
  "sessionId": "68a0a0f8-...",
  "cwd": "/home/richard/...",
  "timestamp": "2026-03-12T07:03:01.738Z"
}
```

Observed `stop_reason` values: `"end_turn"`, `"tool_use"`, `"stop_sequence"`, `null`.

## Decisions

- **JSONL tail reads**: `fs:read` supports optional `tail_bytes`, implementing tail-like
  reads for efficient JSONL inspection.
- **`allowed_paths` granularity**: Generic. Any plugin may declare allowed path prefixes,
  and the host validates requested paths against them.
- **AS tests**: Add a dedicated JS-based Extism test harness for the AssemblyScript plugin.
