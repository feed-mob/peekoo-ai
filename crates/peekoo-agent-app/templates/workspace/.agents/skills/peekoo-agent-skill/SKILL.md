---
name: peekoo-agent-skill
description: Access peekoo productivity tools (tasks, pomodoro, calendar, plugins) via mcporter when MCP is not natively supported.
---

# Peekoo Agent Skill

It provides access to peekoo's productivity tools via the mcporter CLI.

## Setup

The mcporter config file is located at `mcporter.json` in this skill's directory. It is automatically updated with the correct port when peekoo starts.

## Discover Available Tools

Run `npx mcporter list` first to discover available tools and their parameters:

```bash
npx mcporter list peekoo-native --config <path-to-this-directory>/mcporter.json
npx mcporter list peekoo-plugins --config <path-to-this-directory>/mcporter.json
```

## Call a Tool

```bash
npx mcporter call <server>.<tool_name> [args...] --config <path-to-this-directory>/mcporter.json
```

## Servers

- **peekoo-native**: Task management, pomodoro timer, settings
- **peekoo-plugins**: Google Calendar, OpenClaw, and other plugin tools

## Examples

```bash
npx mcporter call peekoo-native.pomodoro_status --config <path-to-this-directory>/mcporter.json
npx mcporter call peekoo-native.list_tasks --config <path-to-this-directory>/mcporter.json
```

Always run `npx mcporter list` first to discover the exact tool names and their parameters.
