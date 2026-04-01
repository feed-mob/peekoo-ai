# 2026-03-31 Migrate System Prompt from Env Var to ACP Prompt Injection

## Problem
The `ACP_SYSTEM_PROMPT` environment variable approach was not part of the ACP protocol and not supported by any major ACP agent (OpenCode, Cursor, Claude Code). The full persona+skill content was assembled into a large markdown string and delivered via an env var that agents didn't read.

## Solution
Migrated to injecting the system prompt as the first content block in the ACP `session/prompt` request. Replaced the full content assembly with a minimal context prompt that directs the agent to read files natively.

### Context Prompt Format
```
Read AGENTS.md first — it contains all instructions for working with this workspace, including how to use SOUL.md, IDENTITY.md, USER.md, and MEMORY.md.

Skills are available in .agents/skills/. Use the skill tool to load skills on demand.

<Task activity summary if any>
```

### Key Changes
- **Removed** `ACP_SYSTEM_PROMPT` env var injection from ACP spawn
- **Removed** `load_persona_sections()` and `load_skill_sections()` dead code
- **Rewrote** `build_system_prompt()` to output minimal context prompt instead of full content assembly
- **Modified** ACP prompt handler to prepend context prompt as first `ContentBlock` in `PromptRequest`
- **Threaded** context prompt through task scheduler (`agent_scheduler.rs`) so background tasks also receive the context prompt

### Files Changed
- `crates/peekoo-agent/src/service.rs`
- `crates/peekoo-agent/src/backend/acp.rs`
- `crates/peekoo-agent-app/src/agent_scheduler.rs`
- `crates/peekoo-agent-app/src/application.rs`

### Benefits
- Works with all ACP agents regardless of env var support
- Agent reads AGENTS.md natively (no duplication)
- Agent loads skills on-demand via skill tool (no upfront content cost)
- Much smaller token budget per prompt turn
