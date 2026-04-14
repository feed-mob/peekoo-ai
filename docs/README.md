# Peekoo AI Docs

Choose a language:

- [English](./en/README.md)
- [中文](./zh/README.md)

This directory contains user-facing and developer-facing documentation for Peekoo AI.

## Structure

- `en/`: English documentation
- `zh/`: Chinese documentation

## Suggested Ownership

This mirrors the outline provided for the docs effort.

| Area | Pages | Owner |
|------|-------|-------|
| Overview | `overview.md`, `quick-start.md` | Lunar |
| Installation | `installation/*` | Richard |
| Configuration | `configuration/sprite.md`, `configuration/acp.md`, `configuration/skills.md` | Lunar / Richard / Richard |
| Usage | `usage/ai-agent-chat.md`, `usage/tasks.md`, `usage/pomodoro.md`, `usage/plugins/*` | Kaiji / Richard / Lunar / Kaiji |
| Develop | `develop/sdk.md`, `develop/plugins.md` | Kaiji / Richard |

## Maintenance

- Keep English and Chinese pages mirrored by path.
- Prefer updating both languages in the same change.
- If one language is temporarily behind, add a short note in the affected page rather than leaving silent drift.
- See [English contribution guide](./en/contributing.md) and [中文贡献指南](./zh/contributing.md).
