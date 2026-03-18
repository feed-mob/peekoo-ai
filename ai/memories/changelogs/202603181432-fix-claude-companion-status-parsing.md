## 2026-03-18 14:32 fix: Harden Claude Code companion status parsing

**What changed:**
- Fixed Claude Code JSONL parsing to read the top-level `type` field instead of accidentally matching nested `message.type` values
- Updated the Claude companion to ignore stale project sessions unless they are recent relative to the newest active transcript
- Aged out stale `waiting` sessions in addition to stale `working` sessions so old Claude badges stop lingering
- Added desktop UI trigger support for `claude-working`, `claude-reminder`, `claude-done`, and `claude-idle`
- Expanded the AssemblyScript plugin regression suite with realistic Claude Code JSONL fixtures and recency-selection coverage

**Why:**
- Real Claude Code transcript lines contain nested `type` fields, which caused the plugin to misclassify active assistant/tool-use turns as `waiting`
- Old Claude sessions from unrelated projects were still being surfaced beside active OpenCode/Claude work
- The frontend ignored Claude-specific mood triggers, so sprite reactions did not change even when the plugin emitted them

**Files affected:**
- `plugins/peekoo-claude-code-companion/assembly/index.ts`
- `plugins/peekoo-claude-code-companion/tests/plugin.test.mjs`
- `plugins/peekoo-claude-code-companion/tests/fixtures/working-tool-use.jsonl`
- `plugins/peekoo-claude-code-companion/tests/fixtures/done-end-turn.jsonl`
- `plugins/peekoo-claude-code-companion/tests/fixtures/waiting-user.jsonl`
- `apps/desktop-ui/src/types/pet-event.ts`
- `apps/desktop-ui/src/hooks/use-sprite-reactions.ts`
