# Issue #55 — OpenClaw-Style User Memory & Bootstrap Personalization

**GitHub**: https://github.com/feed-mob/peekoo-ai/issues/55
**Issue title**: `[P0] Implement User Information Memory and Personalized Interaction`
**Updated**: 2026-03-12

## Goal

Implement the remaining Issue #55 work using the same broad pattern OpenClaw uses:

- bootstrap missing identity/profile files on first run
- let the LLM gather missing information conversationally
- persist durable identity and preference data in markdown files
- preserve prior conversation history separately from long-term memory

This replaces the earlier assumption that a path-only markdown tweak is sufficient.

## Remaining Acceptance Criteria

- [ ] Users only need to introduce themselves once; the AI can remember user information in future conversations
- [ ] The system can remember user preference settings (work habits, reminder preferences, etc.)
- [ ] The AI can reference previous interaction history during conversations

Already completed in issue scope:

- [x] Users can view and edit personal information in settings

## OpenClaw Pattern To Reuse

OpenClaw’s relevant design is:

1. Seed a small set of workspace markdown files on first run
2. Inject a one-time `BOOTSTRAP.md` into the system prompt when profile/persona setup is incomplete
3. Have the LLM ask one short question at a time and write the answers into durable files
4. Delete `BOOTSTRAP.md` after initialization is complete
5. Keep long-term memory distinct from session/daily history

Important implication for Peekoo:

- `BOOTSTRAP.md` is not just prompt copy; it is part of a first-run lifecycle
- missing-file/bootstrap detection needs code support
- file seeding needs an explicit implementation target

## Current Peekoo State

### What already exists

- Persona markdown loading from a discovered `persona_dir`
- Prompt composition for `AGENTS.md`, `SOUL.md`, `IDENTITY.md`, `USER.md`, and memory files
- Session persistence and session resume in the desktop app
- Workspace-colocated persona files and tool working directory under `.peekoo/`

Relevant code:

- `crates/peekoo-agent/src/service.rs`
- `crates/peekoo-agent/src/config.rs`
- `crates/peekoo-agent-app/src/application.rs`

### Current mismatches vs OpenClaw

1. No `BOOTSTRAP.md` support in prompt composition
2. No known first-run seeding path for persona markdown defaults
3. There was historical path confusion between persona files and tool `cwd`; the implemented direction is to colocate them in `.peekoo/`
4. Memory loading is top-level `memories/*.md` only, not recursive
5. Session history already persists, but there is no clear strategy for converting that history into compact, durable personalization memory

## Root Problem Breakdown

Issue #55 is not one bug. It is three related concerns:

### 1. Bootstrap missing identity/profile state

The agent needs a reliable first-run ritual when identity or user profile information is absent.

### 2. Durable preference memory

The agent needs a stable place and clear rules for storing long-lived user facts and preferences.

### 3. Historical conversation reference

The app already persists chat sessions, but that does not automatically guarantee good future personalization. We need a defined strategy for how prior interaction history informs later responses.

## Proposed Implementation

### Task 1 — Add first-run persona file seeding

Create or identify the code path responsible for initializing a new Peekoo persona directory, and ensure it seeds these files when missing:

- `AGENTS.md`
- `SOUL.md`
- `IDENTITY.md`
- `USER.md`
- `MEMORY.md` or `memory.md`
- `BOOTSTRAP.md`

Requirements:

- Seeding must be idempotent
- Existing user-edited files must never be overwritten
- Seeded file contents should live in versioned defaults in the repo, not be assembled ad hoc in Rust strings

Likely implementation area:

- `peekoo-agent-app` startup/config bootstrap path
- possibly a shared helper crate if multiple app entry points need this

### Task 2 — Add `BOOTSTRAP.md` support to prompt composition

Extend prompt composition so `BOOTSTRAP.md` is loaded when present.

Recommended order:

1. `AGENTS.md`
2. `BOOTSTRAP.md`
3. `SOUL.md`
4. `IDENTITY.md`
5. `USER.md`
6. `Memory`
7. `system_prompt`
8. `agent_skills`

Why this order:

- `AGENTS.md` defines operating rules
- `BOOTSTRAP.md` defines first-run workflow
- identity and user files remain visible for partially initialized states

Required code work:

- update `compose_prompt_parts()`
- update tests covering prompt ordering and optional-file behavior
- update crate docs describing supported startup files

### Task 3 — Define OpenClaw-style bootstrap behavior in markdown

Add a repo-owned `BOOTSTRAP.md` template that instructs the LLM to:

- recognize this as first-run setup
- ask one short question at a time
- collect only durable identity/profile information
- write results into the correct persona files
- delete `BOOTSTRAP.md` once setup is complete

Peekoo now uses the `.peekoo/` directory itself as the tool `cwd`, so bootstrap instructions can use direct workspace-relative paths:

- `IDENTITY.md`
- `USER.md`
- `SOUL.md`
- `MEMORY.md`
- `BOOTSTRAP.md`

The bootstrap should gather:

- Peekoo identity and tone if missing
- user name and preferred form of address
- durable user preferences
- optional role/work context if the user volunteers it

The bootstrap must explicitly avoid:

- lengthy onboarding
- transient to-do items
- saving sensitive data unless the user intentionally shares it for future use

### Task 4 — Normalize persona file instructions around real tool capabilities

Update the shipped markdown instructions to match the actual built-in tools.

Do not reference non-existent tools like:

- `replace_file_content`
- `write_to_file`

Use the documented built-ins instead:

- `read`
- `write`
- `edit`

Also clarify the working-directory model in `AGENTS.md` so the LLM understands where persona files live relative to tool `cwd`.

### Task 5 — Make durable memory rules explicit

Adopt a simple file contract:

- `USER.md`: structured user profile and addressing preferences
- `IDENTITY.md`: Peekoo self-description
- `SOUL.md`: behavioral style and boundaries
- `MEMORY.md`: durable long-term facts and preferences
- `memories/*.md`: optional topic-specific durable notes

Guidance to encode in markdown:

- write to memory only for durable facts
- prefer editing an existing section over full rewrites
- avoid storing transient session chatter
- update `USER.md` when the user corrects or changes profile information

### Task 6 — Decide how to satisfy “reference previous interaction history”

Peekoo already restores prior sessions. The remaining question is whether raw restored history is enough for the product requirement.

Recommended approach:

- treat session persistence as the primary source of recent conversation context
- treat `MEMORY.md` and `memories/*.md` as the durable summary layer
- do not inject all historical sessions into the system prompt

Optional OpenClaw-like enhancement after the main fix:

- write compact session summaries into durable memory files only when the conversation produces a durable fact or preference

Important constraint:

- the current loader only reads top-level `memories/*.md`
- if we want nested notes like `memories/sessions/YYYY-MM-DD.md`, Rust code must be updated to load recursively

### Task 7 — Validation and tests

Add tests for:

1. prompt composition includes `BOOTSTRAP.md` in the expected order
2. prompt composition skips missing/empty bootstrap files cleanly
3. first-run seeding creates missing persona files without overwriting existing ones
4. the bootstrap path plus relative file references are documented correctly
5. memory loading behavior matches the intended file layout

Manual verification should cover:

1. fresh install / empty persona directory
2. first conversation causes bootstrap behavior
3. user shares their name and preference
4. LLM writes profile/memory files successfully
5. `BOOTSTRAP.md` is removed after completion
6. later session remembers the user without re-asking

## Proposed File/Code Changes

### Rust

- `crates/peekoo-agent/src/service.rs`
  - add `BOOTSTRAP.md` support in prompt composition
  - possibly extend memory loading if nested memory directories are desired
- tests in `crates/peekoo-agent/src/service.rs`
  - add prompt-order and bootstrap presence tests
- app/bootstrap path in `crates/peekoo-agent-app`
  - seed default persona files on first run

### Repo-owned markdown defaults

Add versioned template files for seeded persona defaults, likely under a dedicated template/defaults directory:

- `AGENTS.md`
- `BOOTSTRAP.md`
- `SOUL.md`
- `IDENTITY.md`
- `USER.md`
- `MEMORY.md`

Exact location should be chosen so:

- the app can copy them at bootstrap
- tests can load them deterministically
- future edits do not require Rust string changes

## Non-Goals

- Replacing settings-based profile editing with chat-only editing
- Storing all conversations permanently in prompt memory
- Building a complex retrieval system in the same PR unless needed
- Implementing a full “affinity scoring” system unless product explicitly requires it for issue completion

## Risks

### Bootstrap never exits

If `BOOTSTRAP.md` is present but deletion never occurs, every session will feel like first run.

Mitigation:

- strong markdown instructions
- test a successful bootstrap flow
- consider a future explicit completion marker if deletion alone is too fragile

### File path confusion remains

Because persona files are outside tool `cwd`, markdown instructions must be exact.

Mitigation:

- centralize path rules in seeded `AGENTS.md`
- keep bootstrap and memory skill examples consistent

### Too much memory gets injected

If session summaries are treated like long-term memory indiscriminately, prompts will bloat and personalization quality may degrade.

Mitigation:

- only store durable facts in `MEMORY.md` / `memories/*.md`
- keep raw session history in session persistence, not prompt bootstrap files

## Recommended Delivery Sequence

### Phase 1 — Foundation

1. Identify/create persona seeding path
2. Add versioned markdown defaults
3. Add `BOOTSTRAP.md` prompt support
4. Update prompt-order tests and docs

### Phase 2 — Behavior

1. Author `BOOTSTRAP.md`
2. Update `AGENTS.md` and related markdown instructions with correct paths/tools
3. Structure `USER.md` and `MEMORY.md`

### Phase 3 — History strategy

1. Validate that session restore satisfies recent-history expectations
2. If needed, add compact durable summaries for long-lived facts
3. Only add recursive memory loading if nested memory directories are actually required

## Decision Notes

### Keep current split-path design or flatten it?

Implemented recommendation:

- keep persona files in the `.peekoo/` workspace itself so the LLM reads and writes the same paths it sees in the prompt
- use seeded defaults plus `BOOTSTRAP.md` for first-run initialization

### `MEMORY.md` vs `memory.md`

Recommendation:

- standardize on one canonical file name in seeded defaults
- keep read compatibility for both until migration is unnecessary

## Definition of Done

This issue is done when:

1. A fresh Peekoo install seeds bootstrap/persona markdown defaults
2. First conversation gathers missing profile information conversationally
3. The LLM persists user identity and preferences into durable markdown files
4. `BOOTSTRAP.md` is removed after successful initialization
5. Later sessions remember the user without asking for re-introduction
6. Prior conversation context is available via restored sessions and/or durable summaries in a clearly defined way
