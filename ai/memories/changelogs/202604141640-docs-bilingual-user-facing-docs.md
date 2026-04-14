## 2026-04-14 16:40: docs: add bilingual user-facing docs tree

**What changed:**
- Added a new bilingual `docs/` tree with mirrored English and Chinese pages for overview, quick start, installation, configuration, usage, and development topics.
- Added user-facing pages for sprite configuration, ACP, skills, tasks, pomodoro, plugins, Google Calendar, SDK scope, and plugin development.
- Updated repo links that pointed at missing legacy docs so they now reference the new docs structure.

**Why:**
- The repository had an empty `docs/` directory, broken references to non-existent doc pages, and no structured bilingual documentation surface.
- The new tree gives users and contributors a single place to discover current product and extension guidance in both English and Chinese.

**Files affected:**
- `docs/README.md`
- `docs/en/**`
- `docs/zh/**`
- `README.md`
- `packages/plugin-sdk/README.md`
- `crates/peekoo-plugin-sdk/README.md`
- `ai/docs/release.md`
