# Migrate System Prompt from Env Var to ACP Prompt Injection

## Problem

The current `ACP_SYSTEM_PROMPT` environment variable approach is not part of the ACP protocol and is not supported by any major ACP agent (OpenCode, Cursor, Claude Code). The system prompt is assembled from persona files and skills but delivered via an env var that agents don't read.

## Solution

Migrate to injecting the system prompt as the first content block in the ACP `session/prompt` request. Replace the full persona+skill content assembly with a minimal context prompt that directs the agent to read files natively.

## Context Prompt Format

```
Read AGENTS.md first — it contains all instructions for working with this workspace, including how to use SOUL.md, IDENTITY.md, USER.md, and MEMORY.md.

Skills are available in .agents/skills/. Use the skill tool to load skills on demand.

<Task activity summary if any>
```

## Changes

### 1. `peekoo-agent/src/service.rs` — Rewrite `build_system_prompt()`

- Remove `load_persona_sections()` call — agent reads AGENTS.md natively
- Remove `load_skill_sections()` call — agent uses skill tool on-demand
- Output minimal context prompt with AGENTS.md directive, skills path hint, and task summary

### 2. `peekoo-agent/src/backend/acp.rs` — Remove env var, inject via prompt

- Remove `cmd.env("ACP_SYSTEM_PROMPT", prompt)` from `spawn_and_connect()`
- In the Prompt handler, prepend `self.system_prompt` as first `ContentBlock` in `PromptRequest`
- User input becomes the second content block

### 3. `peekoo-agent-app/src/agent_scheduler.rs` — Thread context prompt

- Add `context_prompt: Option<String>` parameter through the call chain:
  - `spawn_worker()`
  - `check_and_execute_tasks()`
  - `execute_task_acp()`
- In `execute_task_acp()`, prepend context prompt to task `PromptRequest`

### 4. `peekoo-agent-app/src/application.rs` — Pass context prompt to scheduler

- In `start_plugin_runtime()`, pass the context prompt through to the scheduler

### 5. Dead code removal

- Remove `load_persona_sections()` function
- Remove `load_skill_sections()` function
- Remove env var test in acp.rs

## Files Modified

| File | Change |
|------|--------|
| `peekoo-agent/src/service.rs` | Rewrite `build_system_prompt()`, remove persona/skill loaders |
| `peekoo-agent/src/backend/acp.rs` | Remove env var, prepend prompt via ContentBlock |
| `peekoo-agent-app/src/agent_scheduler.rs` | Thread context prompt through scheduler |
| `peekoo-agent-app/src/application.rs` | Pass context prompt to scheduler |

## What Stays Unchanged

- `AgentServiceConfig`, `BackendConfig`, `AgentService::prompt()`, session creation/resumption, skill discovery

## Testing

- Update `build_system_prompt` unit tests
- Remove env var test
- Manual test: verify OpenCode reads AGENTS.md and follows instructions
