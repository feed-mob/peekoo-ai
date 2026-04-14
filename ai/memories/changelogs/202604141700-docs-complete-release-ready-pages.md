## 2026-04-14 17:00: docs: complete release-ready docs pages

**What changed:**
- Replaced the remaining placeholder and handoff copy in the bilingual docs with complete user-facing content.
- Expanded overview, quick start, sprite, chat, pomodoro, plugins, Google Calendar, and SDK pages so they can ship as part of a release.
- Removed the temporary `TBD owner` references from both the pages and the docs index.

**Why:**
- The docs tree structure already existed, but some pages still contained handoff markers meant for internal coordination rather than release.
- This pass makes the documentation suitable for end users and contributors without exposing internal drafting notes.

**Files affected:**
- `docs/en/overview.md`
- `docs/zh/overview.md`
- `docs/en/quick-start.md`
- `docs/zh/quick-start.md`
- `docs/en/configuration/sprite.md`
- `docs/zh/configuration/sprite.md`
- `docs/en/usage/ai-agent-chat.md`
- `docs/zh/usage/ai-agent-chat.md`
- `docs/en/usage/pomodoro.md`
- `docs/zh/usage/pomodoro.md`
- `docs/en/usage/plugins/index.md`
- `docs/zh/usage/plugins/index.md`
- `docs/en/usage/plugins/google-calendar.md`
- `docs/zh/usage/plugins/google-calendar.md`
- `docs/en/develop/sdk.md`
- `docs/zh/develop/sdk.md`
- `docs/en/README.md`
- `docs/zh/README.md`
