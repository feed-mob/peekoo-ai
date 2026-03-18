## 2026-03-18 15:10: fix: stop Claude companion from over-reporting status changes

**What changed:**
- Removed the Claude companion `waiting` state and now treat plain trailing user messages as `working` until they age out to `idle`
- Changed completion deduplication to fire `done` only once per Claude session instead of once per assistant turn
- Added regression fixtures and tests for multi-turn session completion and user-tail handling

**Why:**
- Claude Code JSONL does not reliably distinguish a normal user turn from a true "needs input" state
- `end_turn` appears on every assistant response, so mtime-based completion keys caused false repeated "done" notifications

**Files affected:**
- `plugins/peekoo-claude-code-companion/assembly/index.ts`
- `plugins/peekoo-claude-code-companion/tests/plugin.test.mjs`
- `plugins/peekoo-claude-code-companion/tests/fixtures/first-turn-done.jsonl`
- `plugins/peekoo-claude-code-companion/tests/fixtures/multi-turn-done.jsonl`
