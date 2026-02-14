# AI Knowledge Base

This folder contains persistent AI memories and plans for the project.

## Directory Structure

```
ai/
├── AGENTS.md          # This file
├── memories/
│   ├── changelogs     # Record of significant changes
│   └── diagrams/      # Architecture and flow diagrams
└── plans/             # Implementation plans
```

---

## Memories

AI memories persist important context across sessions. Load memories when context is needed.

### Changelogs (`memories/changelogs`)

Record significant changes after completing work:

**File Naming Convention:** Use timestamp prefix with hours and minutes:
- Format: `YYYYMMDDHHMM-brief-description.md`
- Example: `202502151430-add-user-authentication.md`

**Changelog Prefix Rule:** Use conventional commit prefixes in changelog titles:
- `feat:` - New features
- `fix:` - Bug fixes  
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `docs:` - Documentation changes
- `chore:` - Maintenance tasks

```markdown
## YYYY-MM-DD HH:MM: prefix: Brief Title

**What changed:**
- Description of changes made

**Why:**
- Reason for the change

**Files affected:**
- List of modified files
```

**When to write:**
- After implementing a feature
- After fixing a significant bug
- After refactoring code
- After architectural changes

**When to load:**
- Starting work on a feature that may overlap with past changes
- Debugging issues that may relate to recent changes
- Understanding the evolution of a component

### Mermaid Diagrams (`memories/diagrams/`)

Store visual representations of architecture and flows using Mermaid.

**Naming convention:** `<component>-<type>.md` (e.g., `auth-flow.md`, `database-schema.md`)

**Format:** Use Mermaid diagrams in markdown files:

```markdown
# Component Name

Brief description of what this diagram represents.

\`\`\`mermaid
graph TD
    A[Start] --> B[Process]
    B --> C[End]
\`\`\`

## Notes
- Additional context about the diagram
```

**When to create/update:**
- Designing new features
- Documenting complex flows
- After architectural changes

**When to load:**
- Before implementing features that interact with documented components
- When onboarding to a new area of the codebase
- When debugging complex interactions

---

## Plans

Store implementation plans after completing the planning phase.

### File Location

`plans/<feature-name>.md`

### Plan Format

```markdown
# Plan: Feature Name

## Overview
Brief description of what we're building and why.

## Goals
- [ ] Goal 1
- [ ] Goal 2

## Design

### Approach
Description of the chosen approach.

### Components
- Component A: Purpose
- Component B: Purpose

## Implementation Steps

1. **Step 1: Description**
   - Sub-task 1.1
   - Sub-task 1.2

2. **Step 2: Description**
   - Sub-task 2.1

## Files to Modify/Create
- `path/to/file.ts` - Description of changes

## Testing Strategy
- Unit tests for X
- Integration tests for Y

## Open Questions
- Question 1?
- Question 2?
```

### Workflow

1. **After planning:** Save the finalized plan to `plans/<feature>.md`
2. **Before implementing:** Load the relevant plan
3. **During implementation:** Update plan status (check off completed items)
4. **After completion:** Archive or delete the plan

**When to load plans:**
- Starting implementation of a planned feature
- Resuming work on an in-progress feature
- Reviewing what was planned before making changes

---

## Quick Reference

| Action | Location | When |
|--------|----------|------|
| Write changelog | `memories/changelogs` | After significant changes |
| Create diagram | `memories/diagrams/<name>.md` | When documenting architecture |
| Save plan | `plans/<feature>.md` | After planning phase |
| Load memories | `memories/` | When context needed |
| Load plan | `plans/<feature>.md` | Before/during implementation |
