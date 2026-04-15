## 2026-03-31 16:44: feat: switch Linear integration auth from OAuth to API key

**What changed:**
- Reworked `plugins/linear` authentication flow from OAuth to API key secret storage (`linear-api-key`).
- Replaced panel connect UX with API key input/save flow, keeping disconnect and manual sync actions.
- Updated plugin manifest tools to `linear_set_api_key` and removed OAuth-specific permissions/tool surface.
- Updated integration design and manual QA docs to align with API key onboarding, validation, and failure paths.
- Rebuilt and installed latest plugin artifact to `~/.peekoo/plugins/linear` for immediate local validation.

**Why:**
- Match product requirement update to use Linear personal/team API keys.
- Simplify desktop setup by avoiding browser callback/OAuth token lifecycle handling.

**Files affected:**
- `plugins/linear/peekoo-plugin.toml`
- `plugins/linear/src/lib.rs`
- `plugins/linear/ui/panel.html`
- `plugins/linear/ui/panel.js`
- `docs/plans/2026-03-31-linear-task-plugin-integration-design.md`
- `docs/plans/2026-03-31-linear-integration-manual-qa.md`
