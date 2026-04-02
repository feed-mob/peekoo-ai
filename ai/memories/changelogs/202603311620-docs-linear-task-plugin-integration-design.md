## 2026-03-31 16:20: docs: add Linear task plugin integration design

**What changed:**
- Added a new product/technical design document for Linear task integration as an independent plugin.
- Defined scope for OAuth connection, bidirectional task sync, and connection status management in Settings.
- Documented architecture decisions aligned with current plugin runtime, including required plugin permissions and SDK task wrapper extension.
- Added phased implementation plan, sync/conflict strategy, risks, and acceptance mapping.

**Why:**
- Provide a concrete implementation blueprint before coding the Linear integration.
- Ensure requirements are explicit: plugin install-gated behavior, two-way sync, and visible connection state.

**Files affected:**
- `docs/plans/2026-03-31-linear-task-plugin-integration-design.md`
- `ai/memories/changelogs/202603311620-docs-linear-task-plugin-integration-design.md`
