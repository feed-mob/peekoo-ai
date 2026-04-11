# Skill Discovery Chat Settings Refactor Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Discover every skill folder that contains `SKILL.md` under the configured skills roots, and remove the misleading chat-settings toggle UI so the panel reflects automatic ACP skill loading.

**Architecture:** Move skill discovery to a recursive, directory-based scan in the backend settings layer. Treat the chat settings panel as informational for skills, not as a source of truth for skill enablement, because ACP can still auto-load skills from configured roots. Keep runtime behavior aligned by listing discovered skills only and dropping persisted toggle state from the chat panel.

**Tech Stack:** Rust, `std::fs`, React, TypeScript, Tauri, Zod, Rust unit tests

---

### Task 1: Add a failing backend test for recursive skill discovery

**Files:**
- Modify: `crates/peekoo-agent-app/src/settings/skills.rs`

**Step 1: Write the failing test**

Add a unit test module in `crates/peekoo-agent-app/src/settings/skills.rs` that:
- creates a temporary root directory
- creates nested skill folders such as:
  - `<tmp>/skills/alpha/SKILL.md`
  - `<tmp>/skills/group/beta/SKILL.md`
  - `<tmp>/skills/group/deeper/gamma/SKILL.md`
- adds a non-skill folder without `SKILL.md`
- calls a new testable helper such as `discover_skills_in_roots(&[root])`
- asserts that `alpha`, `beta`, and `gamma` are discovered
- asserts that the returned `path` values point to each folder's `SKILL.md`

Suggested assertion shape:

```rust
assert_eq!(skill_ids, vec!["alpha", "beta", "gamma"]);
assert!(paths.iter().all(|path| path.ends_with("SKILL.md")));
```

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p peekoo-agent-app discover_skills
```

Expected:
- FAIL because recursive discovery helper does not exist yet, or because nested folders are not discovered

**Step 3: Write minimal implementation**

In `crates/peekoo-agent-app/src/settings/skills.rs`:
- extract root collection into a helper
- add a recursive walker that traverses subdirectories
- treat a directory as a skill when `dir.join("SKILL.md").is_file()`
- use the directory name as `skill_id`
- keep de-duplication by `skill_id`
- ignore plain `.md` files at the root and nested levels unless they are `SKILL.md` inside a directory

Use a testable shape like:

```rust
fn discover_skills_in_roots(roots: &[PathBuf]) -> Vec<SkillDto>
```

Keep `pub fn discover_skills() -> Vec<SkillDto>` as the production entry point that gathers the configured roots and delegates to the helper.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p peekoo-agent-app discover_skills
```

Expected:
- PASS

**Step 5: Commit**

Do not commit yet unless explicitly requested.

### Task 2: Remove settings-backed skill toggles from the backend contract

**Files:**
- Modify: `crates/peekoo-agent-app/src/settings/dto.rs`
- Modify: `crates/peekoo-agent-app/src/settings/store.rs`
- Modify: `crates/peekoo-agent-app/src/settings/mod.rs`

**Step 1: Write the failing test**

In `crates/peekoo-agent-app/src/settings/store.rs`, add a test that loads settings from a fresh DB and asserts skill state is no longer sourced from `agent_skills` rows for chat settings persistence. The test should verify that `load_settings()` can succeed without needing `agent_skills` content and that skill discovery remains catalog-driven.

A minimal direction:

```rust
let settings = store.load_settings().expect("load settings");
assert!(settings.skills.is_empty());
```

This test documents the new contract: chat settings no longer persist a toggleable skill list.

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p peekoo-agent-app load_settings
```

Expected:
- FAIL because `load_settings()` currently falls back to discovered skills

**Step 3: Write minimal implementation**

Backend changes:
- In `dto.rs`, remove `skills` from `AgentSettingsDto` and from `AgentSettingsPatchDto`
- In `store.rs`:
  - stop reading `agent_skills` into `load_settings()`
  - remove fallback `discover_skills()` from `load_settings()`
  - remove the `skills` branch from `apply_patch()`
- In `mod.rs`:
  - stop merging enabled settings into `base.agent_skills`
  - keep `catalog_from_runtimes()` returning `discovered_skills`

Important:
- Leave runtime auto-discovery intact
- Do not remove the `agent_skills` table in this refactor unless it is already unused everywhere and a migration is planned separately

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p peekoo-agent-app load_settings
```

Expected:
- PASS

**Step 5: Commit**

Do not commit yet unless explicitly requested.

### Task 3: Remove toggle UI and render discovered skills as read-only

**Files:**
- Modify: `apps/desktop-ui/src/features/chat/settings/ChatSettingsPanel.tsx`
- Modify: `apps/desktop-ui/src/features/chat/settings/SkillToggleList.tsx`
- Modify: `apps/desktop-ui/src/features/chat/settings/useChatSettings.ts`
- Modify: `apps/desktop-ui/src/types/agent-settings.ts`

**Step 1: Write the failing frontend test or type check target**

If there is no existing component test pattern nearby, use type-check-first as the verification gate for this UI refactor.

Make the intended contract explicit:
- `settings` no longer carries `skills`
- `catalog.discoveredSkills` is the only source for skill listing in the panel
- `SkillToggleList` becomes a read-only list component, or rename it to `SkillList`

**Step 2: Run check to verify it fails**

Run:

```bash
cd apps/desktop-ui && npx tsc --noEmit
```

Expected:
- FAIL after removing `skills` from the shared schema until the component code is updated

**Step 3: Write minimal implementation**

Frontend changes:
- In `apps/desktop-ui/src/types/agent-settings.ts`:
  - remove `skills` from `agentSettingsSchema`
- In `useChatSettings.ts`:
  - remove `SkillSettings` from `SettingsPatch`
  - remove any `updateSettings({ skills })` usage
- In `ChatSettingsPanel.tsx`:
  - remove `effectiveSkills`
  - render `catalog.discoveredSkills` directly
  - change the section copy to something like:
    - `Discovered Skills`
    - `Peekoo finds skills automatically from configured skill folders.`
- In `SkillToggleList.tsx`:
  - remove `Checkbox`
  - render a simple list of `skillId`
  - optionally show `path` as muted secondary text or title tooltip
  - rename the component to `SkillList` if that keeps the API cleaner

**Step 4: Run check to verify it passes**

Run:

```bash
cd apps/desktop-ui && npx tsc --noEmit
```

Expected:
- PASS

**Step 5: Commit**

Do not commit yet unless explicitly requested.

### Task 4: Verify runtime behavior still matches the UI

**Files:**
- Review: `crates/peekoo-agent/src/service.rs`
- Review: `crates/peekoo-agent/src/config.rs`
- Review: `crates/peekoo-agent-app/src/application.rs`
- Review: `crates/peekoo-agent-app/src/settings/mod.rs`

**Step 1: Write the failing test**

Add or extend a backend test in `crates/peekoo-agent-app/src/settings/mod.rs` that proves `to_agent_config()` no longer depends on chat-settings skill state and still returns a valid config.

A minimal test can assert:

```rust
let (config, _version) = svc.to_agent_config(base, AgentProvider::opencode(), None)?;
assert!(config.agent_skills.len() >= base_skill_count);
```

The exact assertion depends on how `base.agent_skills` is seeded in the test fixture. The key point is that this method should no longer merge a persisted enabled-skill subset from settings.

**Step 2: Run test to verify it fails**

Run:

```bash
cargo test -p peekoo-agent-app to_agent_config
```

Expected:
- FAIL until `to_agent_config()` stops reading `settings.skills`

**Step 3: Write minimal implementation**

Update `to_agent_config()` so it:
- still applies system prompt, max iterations, provider config, API key, and OAuth token
- does not read or merge `settings.skills`

Keep the runtime's existing auto-discovery behavior untouched. This matches the new read-only UI.

**Step 4: Run test to verify it passes**

Run:

```bash
cargo test -p peekoo-agent-app to_agent_config
```

Expected:
- PASS

**Step 5: Commit**

Do not commit yet unless explicitly requested.

### Task 5: Run final verification

**Files:**
- Review: all modified files

**Step 1: Run targeted Rust tests**

Run:

```bash
cargo test -p peekoo-agent-app discover_skills
cargo test -p peekoo-agent-app load_settings
cargo test -p peekoo-agent-app to_agent_config
```

Expected:
- PASS

**Step 2: Run frontend type check**

Run:

```bash
cd apps/desktop-ui && npx tsc --noEmit
```

Expected:
- PASS

**Step 3: Run broader backend check if the targeted tests pass**

Run:

```bash
just check
```

Expected:
- PASS

**Step 4: Manual verification**

Verify in the app:
- the chat settings panel opens without errors
- every folder containing `SKILL.md` under the configured skills roots appears in the list
- nested skill folders appear
- folders without `SKILL.md` do not appear
- no checkbox or toggle is shown
- the section text makes it clear that discovery is automatic

**Step 5: Commit**

If the user asks for a commit later, use a focused conventional commit such as:

```bash
git add crates/peekoo-agent-app/src/settings apps/desktop-ui/src/features/chat/settings apps/desktop-ui/src/types/agent-settings.ts docs/plans/2026-04-01-skill-discovery-chat-settings-refactor.md
git commit -m "refactor(chat-settings): make skill discovery automatic"
```
