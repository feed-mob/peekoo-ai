# AI Agent Chat

## What It Does

Peekoo includes a built-in chat surface for conversational AI. The agent runtime supports streaming responses, configurable providers, runtime model changes, and workspace context loading.

Use chat when you want fast help without leaving your desktop. It is the main surface for asking questions, brainstorming, inspecting project context, or using the agent together with Peekoo's productivity tools.

## How Chat Fits the App

The chat layer is agent-first. It sits on top of the agent runtime and works with:

- provider configuration and authentication
- workspace instruction files such as `AGENTS.md`
- discovered skills
- MCP-backed tools for tasks, pomodoro, settings, and plugins

## Runtime Model

At a high level, the chat flow looks like this:

1. The UI sends a prompt through a Tauri command.
2. The backend resolves agent configuration.
3. The agent runtime prompts the selected provider.
4. If the model needs tools, it can use built-in tools, loaded skills, and Peekoo MCP tools.
5. The response streams back to the UI.

## Common Ways to Use Chat

- ask for help with a task you are already working on
- inspect files and project context through the agent's tool access
- turn rough ideas into clear next steps
- use chat together with tasks and pomodoro for a tighter workflow

## Configuration Surface

The current repository supports:

- switching providers and models at runtime
- loading workspace instruction files
- discovering skills from configured roots
- auth flows for supported providers

## Choosing a Provider or Model

Choose the provider and model that match your goal:

- fast everyday assistance: use your default general-purpose model
- deeper reasoning: switch to a stronger model when accuracy matters more than speed
- cost-sensitive usage: keep a lighter model selected for routine prompts

If your setup supports multiple providers, Peekoo can switch models at runtime without requiring a full restart.

## Workspace Context

Peekoo's agent runtime can load instruction and memory files from the workspace, including files such as:

- `AGENTS.md`
- `SOUL.md`
- `IDENTITY.md`
- `USER.md`
- `MEMORY.md`

This lets the agent keep project-specific and user-specific context without hardcoding it into the app.

## Good First Prompts

If you are new to Peekoo chat, try prompts like:

- `Summarize what this project does.`
- `Help me break this feature into tasks.`
- `Review this part of the codebase for risks.`
- `Draft a pomodoro plan for my next hour.`

## Related Concepts

- Providers and model configuration
- Workspace files such as `AGENTS.md`
- Skills loaded on demand
- MCP-backed tools for productivity features

## Status Note

The repository has strong runtime support, but a fuller UI-focused walkthrough for every chat setting and provider flow still needs dedicated product documentation.
