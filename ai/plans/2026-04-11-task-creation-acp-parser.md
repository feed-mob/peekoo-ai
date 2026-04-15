# Plan: ACP Prompt-Based Task Creation Parsing

## Overview

Use `peekoo-agent-acp` to parse task quick-input text into structured fields via ACP `prompt`, while keeping the existing regex parser as a permanent fallback.

## Goals

- [x] Add ACP parse payload support in `peekoo-agent-acp`
- [ ] Parse task creation text via ACP in `peekoo-agent-app`
- [ ] Preserve existing fallback behavior using local regex parser
- [ ] Keep frontend and Tauri command surface unchanged
- [ ] Add tests for parse-context detection and fallback behavior

## Design

### Approach

- Extend ACP prompt payload detection with a dedicated `task_creation_parse` request type.
- Reuse ACP `prompt` to run parsing through the same sidecar binary used for task execution.
- Keep parsing robust by validating ACP output and falling back to `task_parser::parse_task_text` on any failure.

### Components

- `peekoo-agent-acp context`: add `TaskCreationContext` with `request_type`, `raw_text`, `locale`, `timezone`
- `peekoo-agent-acp agent`: route prompt payloads to task execution or task creation parse mode
- `peekoo-agent-app application`: call ACP parser in `create_task_from_text` and fallback to regex parser

## Implementation Steps

1. **ACP Parse Context**
   - Add `TaskCreationContext` in `crates/peekoo-agent-acp/src/context.rs`
   - Add parser prompt builder that enforces JSON-only output

2. **ACP Prompt Routing**
   - Update prompt context extraction in `crates/peekoo-agent-acp/src/agent.rs`
   - Detect `task_creation_parse` payloads and run parse prompt mode
   - Keep existing task execution flow unchanged

3. **App-Layer ACP Parse Integration**
   - Update `AgentApplication::create_task_from_text` to try ACP parse first
   - Deserialize and validate returned JSON
   - Fallback to `task_parser::parse_task_text` when ACP parse fails

4. **Validation + Fallback Safety**
   - Normalize enum fields (`priority`, `assignee`)
   - Guard against invalid JSON/no content/transport failures
   - Preserve existing create-task behavior and defaults

5. **Testing**
   - Add/extend unit tests for ACP context detection and parse payload mode
   - Ensure fallback behavior remains intact

## Files to Modify

- `ai/plans/2026-04-11-task-creation-acp-parser.md`
- `crates/peekoo-agent-acp/src/context.rs`
- `crates/peekoo-agent-acp/src/agent.rs`
- `crates/peekoo-agent-app/src/application.rs`

## Testing Strategy

- `cargo test -p peekoo-agent-acp`
- `cargo test -p peekoo-agent-app`
- `just check`
