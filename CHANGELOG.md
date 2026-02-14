# Changelog

- 2026-02-15 02:07 - Redesigned desktop UI into a multi-window desktop pet flow: small always-on-top transparent sprite window, label-based view routing, on-demand Chat/Tasks/Pomodoro panel windows, and removal of Three.js background dependencies/components.
- 2026-02-15 02:07 - Fixed Vite dependency update error after removing `@react-three/*` by clearing optimized-deps cache (`node_modules/.vite`, `node_modules/.cache/vite`) and restarting with `bun run dev --force`.
