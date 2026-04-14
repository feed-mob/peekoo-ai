# Plugins

## Overview

Peekoo supports sandboxed WASM plugins. A plugin can add:

- agent tools
- UI panels
- event hooks
- persistent plugin state

Plugins are the main way to extend Peekoo beyond its built-in chat, task, and pomodoro features. They help turn Peekoo from a focused desktop companion into a more adaptable workspace tool.

## Discovery

Plugins are discovered from:

- `~/.peekoo/plugins/`
- workspace-local `plugins/`

## Runtime Model

Plugins run through the plugin host and Extism runtime. UI panels are rendered in sandboxed webviews, and tools can be exposed back to the agent runtime.

## What Plugins Can Feel Like

Depending on the plugin, the experience can look different:

- a tool-only extension that gives the agent new actions
- a panel-based integration that adds a focused UI inside Peekoo
- an event-driven utility that reacts to runtime events

## Picking Your First Plugin

Start with a plugin that solves one clear need. Good candidates are integrations that save repeated manual steps, such as pulling data from an external service or adding a dedicated workflow panel.

## Related Pages

- [Google Calendar Plugin](./google-calendar.md)
- [Plugin Development](../../develop/plugins.md)
