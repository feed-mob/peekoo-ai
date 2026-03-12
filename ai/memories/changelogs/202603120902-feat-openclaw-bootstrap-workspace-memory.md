## 2026-03-12 09:02: feat: openclaw bootstrap workspace memory

**What changed:**
- Added app-side agent workspace bootstrapping that seeds default persona markdown files and manages one-time `BOOTSTRAP.md`.
- Switched the desktop app to use the `.peekoo` directory itself as both the agent workspace and persona directory.
- Added `BOOTSTRAP.md` prompt support and updated prompt ordering/docs in `peekoo-agent`.
- Added tests covering bootstrap file reconciliation and prompt composition.
- Removed the repo-local tracked `.peekoo/` files so seeded templates are now the single product-owned source of default persona and memory skill content.

**Why:**
- Peekoo needed an OpenClaw-style first-run setup flow so the LLM can initialize missing user profile data without fragile `../` path instructions.
- Colocating persona files with the agent working directory removes path confusion and keeps session restore scoped to the same workspace the agent actually uses.

**Files affected:**
- `crates/peekoo-agent-app/src/workspace_bootstrap.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `crates/peekoo-agent-app/templates/persona/*`
- `crates/peekoo-agent/src/service.rs`
- `crates/peekoo-agent/src/config.rs`
- `crates/peekoo-agent/README.md`
- `crates/peekoo-agent/AGENTS.md`
