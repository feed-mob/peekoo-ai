# Desktop UI

React + Vite frontend for the Peekoo desktop app.

## Run

```bash
cd apps/desktop-ui
bun install
bun run dev
```

## Build

```bash
cd apps/desktop-ui
bun run build
```

## Integration

- Talks to Tauri commands from `apps/desktop-tauri/src-tauri/src/lib.rs`.
- Agent behavior (providers/models/auth/settings) is backed by `crates/peekoo-agent-app`.

## Main Areas

- `src/features/chat/` chat experience and agent settings
- `src/features/tasks/` tasks panel UI
- `src/features/pomodoro/` pomodoro panel UI
