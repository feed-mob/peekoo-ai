# Quick Start

Peekoo is designed to be useful within minutes: install it, launch it, and start interacting with your desktop companion.

## Install Peekoo

Download the latest release from GitHub Releases, then follow the platform guide:

- [macOS](./installation/macos.md)
- [Windows](./installation/windows.md)
- [Linux](./installation/linux.md)

## First Run

After launch, Peekoo appears as a small desktop companion. From there you can immediately:

- open chat
- create and track tasks
- start pomodoro sessions
- open plugin panels

If this is your first time using Peekoo, start simple:

1. Open chat and send a short message.
2. Create one task you want to keep visible.
3. Start one focus session with pomodoro.
4. Explore plugins only after the core flow feels familiar.

## Key Concepts

- `Sprite`: the visible desktop character
- `ACP`: the agent communication layer used to run agents
- `Skill`: instructions or capability bundles agents can load on demand
- `Plugin`: an extension that can add tools, UI panels, and events

## Recommended First Session

Use this sequence to understand the product quickly:

1. Ask Peekoo a practical question in chat.
2. Capture one real task you need to finish today.
3. Start a focus session for that task.
4. Review the result in task and pomodoro history.

This gives you a quick feel for the three main product surfaces: assistance, planning, and focus.

## For Developers

If you are running Peekoo from source:

```bash
just setup
just dev
```

See [Developer Overview](./develop/index.md) for more.
