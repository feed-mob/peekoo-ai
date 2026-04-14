# Product Overview

Peekoo AI is an AI desktop companion that lives directly on your screen. It combines a lightweight desktop pet, a chat interface, productivity tools, and an extension system into one always-available workspace companion.

Peekoo is built for people who want AI support to feel present, fast, and close to their daily workflow. Instead of opening a separate browser tab or full desktop window every time you want help, you keep Peekoo nearby and use it as a companion for thinking, planning, and staying focused.

## Core Features

- AI chat with streaming responses and configurable providers
- Built-in task management that stays close to your desktop flow
- Pomodoro focus sessions with pause, resume, finish, and history
- Animated sprites with mood-based reactions and presence on the desktop
- Plugin support for extra tools, panels, and integrations

## Architecture at a Glance

Peekoo uses a layered desktop architecture:

```text
desktop-ui -> desktop-tauri -> peekoo-agent-app -> domain/runtime crates
```

`desktop-ui` renders the React experience. `desktop-tauri` handles transport. `peekoo-agent-app` owns orchestration for agent runtime, settings, tasks, pomodoro, and plugin integration.

## Why Peekoo Feels Different

Peekoo is designed around presence rather than interruption. The sprite gives the app a visible place on the desktop, the chat surface keeps AI interaction close at hand, and the built-in productivity tools reduce context switching. You can move from asking a question, to tracking a task, to starting a focus session without leaving the same companion.

## Built for Extension

Peekoo is not limited to its built-in tools. Its plugin system allows new tools, panels, and integrations to be added over time. That makes it useful for both individual workflows and more specialized setups.

## Who This Docs Set Is For

- Users who want to install and use Peekoo
- Power users who want to configure ACP and skills
- Developers who want to understand the SDK and plugin system

## Next Steps

1. Start with [Quick Start](./quick-start.md).
2. If you are installing from a release, read [Installation](./installation/index.md).
3. If you want to extend Peekoo, read [Develop](./develop/index.md).
