# Skills

## What Skills Are

Skills are instruction bundles that agents can load on demand. They let the agent grow more capable without changing the visible product surface every time. In Peekoo, skills live under `.agents/skills/` inside the workspace.

## Built-In Peekoo Agent Skill

Peekoo ships a bundled `peekoo-agent-skill`. It helps ACP agents that do not support MCP natively reach Peekoo tools through `mcporter`, which keeps advanced workflows available even in less capable environments.

The skill directory contains:

- `SKILL.md`
- `mcporter.json`

`mcporter.json` is updated with the actual local MCP server port when Peekoo starts.

## Discover Tools

```bash
npx mcporter list peekoo-native --config <path-to-skill>/mcporter.json
npx mcporter list peekoo-plugins --config <path-to-skill>/mcporter.json
```

## Call Tools

```bash
npx mcporter call <server>.<tool_name> [args...] --config <path-to-skill>/mcporter.json
```

## Servers

- `peekoo-native`: tasks, pomodoro, settings
- `peekoo-plugins`: plugin-provided tools such as Google Calendar

## Current Behavior

Skill discovery is folder-based. A directory is treated as a skill when it contains `SKILL.md`.
