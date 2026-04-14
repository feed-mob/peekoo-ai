# Tasks

## Overview

Peekoo tasks are part of the built-in productivity system. They give you a lightweight place to capture work, keep it visible, and connect it to the rest of the app.

The task surface is shared across the React UI, agent tools, and plugin integrations.

## What Tasks Support

Current task capabilities in the codebase include:

- create tasks
- list tasks
- update tasks
- delete tasks
- toggle completion
- assign tasks
- add comments
- update labels and status

## Why This Matters

The same task service can be used by:

- the desktop UI
- the built-in agent
- plugins

This keeps task data consistent across manual and agent-driven workflows.

## Notes

Task UX is still evolving. Some earlier planning docs discuss richer filters, activity feeds, and label flows. The current repo already exposes the core CRUD and agent-facing task tools.
