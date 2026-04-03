# Plan: Rename persona → workspace and restructure

## Overview
Rename the template directory from `persona` to `workspace`, and change the seeding target so that all agent files live in a `workspace/` subdirectory under the peekoo home. Rename `self.workspace_dir` to `self.agent_workspace_dir` to reflect that it points to `<peekoo_home>/workspace/`.

## Goals
- [ ] Template directory renamed: `templates/persona/` → `templates/workspace/`
- [ ] All agent files seeded into `<peekoo_home>/workspace/` subdirectory
- [ ] Field renamed: `workspace_dir` → `agent_workspace_dir` in Application
- [ ] All paths updated consistently across the codebase

## Current vs Proposed Structure

**Current:**
```
~/.peekoo/
├── AGENTS.md
├── SOUL.md
├── IDENTITY.md
├── USER.md
├── MEMORY.md
├── .agents/skills/
└── data/
```

**Proposed:**
```
~/.peekoo/
├── workspace/
│   ├── AGENTS.md
│   ├── SOUL.md
│   ├── IDENTITY.md
│   ├── USER.md
│   ├── MEMORY.md
│   └── .agents/
│       └── skills/
│           ├── peekoo-agent-skill/
│           └── memory-manager/
└── data/
```

## Implementation Steps

### 1. Rename template directory
`templates/persona/` → `templates/workspace/`

### 2. `build.rs`
- Update scan path from `templates/persona/.agents/skills/` to `templates/workspace/.agents/skills/`

### 3. `workspace_bootstrap.rs`
- Update all `include_str!` paths
- Rename local variables: `workspace_dir` → `peekoo_home_dir`
- `ensure_agent_workspace()` creates `<peekoo_home>/workspace/` and seeds all files there
- Returns `<peekoo_home>/workspace/` (the agent workspace directory)
- `sync_skill_templates()` writes to `<workspace>/.agents/skills/`
- `reconcile_bootstrap_file()` and `needs_bootstrap()` check `<workspace>/USER.md`

### 4. `application.rs`
- Rename field: `workspace_dir` → `agent_workspace_dir`
- `resolved_config()` uses `self.agent_workspace_dir` for working_directory and persona_dir

### 5. `mcp_server.rs`
- Rename parameter: `workspace_dir` → `agent_workspace_dir`
- Update `write_mcporter_config()` path to `.agents/skills/peekoo-agent-skill/mcporter.json`

## Files to Modify
- `crates/peekoo-agent-app/build.rs`
- `crates/peekoo-agent-app/src/workspace_bootstrap.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/mcp_server.rs`
- `crates/peekoo-agent-app/templates/persona/` → `templates/workspace/` (rename)

## Testing Strategy
- Update tests in `workspace_bootstrap.rs` to use new paths
- Verify `build.rs` auto-discovers skills from new template location
- Manual test: app starts, workspace directory created, files seeded correctly
