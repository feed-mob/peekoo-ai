# Quick Start

Peekoo is designed to be useful within minutes: install it, launch it, and start interacting with your desktop companion.

## Install Peekoo

Download the latest release from GitHub Releases, then follow the platform guide:

- [macOS](./installation/macos.md)
- [Windows](./installation/windows.md)
- [Linux](./installation/linux.md)

## Meet the Interface

After launch, Peekoo appears as a small desktop character on your screen.

<!-- Image placeholder: Peekoo desktop screenshot -->

- Single-click the sprite → returns to default idle state
- Double-click the sprite → opens Mini Chat for a quick conversation
- Right-click the sprite → opens the menu to access Chat, Tasks, Pomodoro, or Plugins
- Drag the sprite → move it anywhere on your desktop
- System tray icon right-click → Show/Hide sprite, Settings, About Peekoo, Quit Peekoo

## Configure an Agent

Before using AI chat, you need to set up an agent runtime.

Go to: System tray icon right-click → Settings → ACP Runtimes

**Recommended for new users: OpenCode (free to start)**

1. Find OpenCode in the available runtimes list and click Install
2. Click "Test Connection" to confirm everything is working
3. Go back to the runtimes list and set OpenCode as default

**Other runtimes:**

Find the runtime you want in the available list, install it, then click Configure and follow the on-screen prompts to log in or enter your API key, and select a model.

## Start Your First Conversation

Once your agent is configured:

1. Double-click the sprite to open Mini Chat and send a short message
2. Or right-click the sprite → Chat to open the full chat panel

If this is your first time using Peekoo, start simple:

1. Open chat and send a short message
2. Create one task you want to keep visible
3. Start one focus session with pomodoro
4. Explore plugins only after the core flow feels familiar

## Tasks and Pomodoro

**Create a task:**

Right-click the sprite → Tasks, then describe your task in natural language, for example:

```
Team meeting tomorrow at 3pm for 1 hour, high priority
```

Peekoo parses the time, priority, and other details automatically.

**Start a focus session:**

Right-click the sprite → Pomodoro, pick the task you want to focus on, and hit Start. When the session ends, you can save a short memo to record what you got done or link it to the corresponding task.

## Install Plugins

Right-click the sprite → Plugins → Store. Browse available plugins from GitHub, install and enable the ones you want.

Available plugins include Health Reminders, Google Calendar, Linear, Mijia Smart Home, and more. See the [plugin list](./usage/plugins/index.md).

## Appearance and Language

System tray icon right-click → Settings → Appearance:

- Theme: Light / Dark / System
- Language: English, 简体中文, 繁體中文, 日本語, Español, Français
- Sprite: switch between different desktop character designs

## Check for Updates

System tray icon right-click → About Peekoo → Check for Updates. When a new version is available, click "Install and Restart" to upgrade.

## Key Concepts

- `Sprite`: the visible desktop character
- `ACP`: the agent communication layer used to run agents
- `Skill`: instructions or capability bundles agents can load on demand
- `Plugin`: an extension that can add tools, UI panels, and events

## For Developers

If you are running Peekoo from source:

```bash
just setup
just dev
```

See [Developer Overview](./develop/index.md) for more.
