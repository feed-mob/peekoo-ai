# AGENTS.md - desktop-ui

## Overview
React + Vite + TypeScript frontend for Peekoo AI desktop pet. Features chat, tasks, pomodoro timer, and animated sprite character.

## Tech Stack
- **Framework**: React 18 + TypeScript 5
- **Build**: Vite 5
- **Styling**: Tailwind CSS v4 + tailwindcss-animate
- **Animations**: Framer Motion + React Spring
- **UI Primitives**: Radix UI (checkbox, scroll-area, slot)
- **Icons**: Lucide React
- **Validation**: Zod
- **Markdown Streaming**: streamdown
- **Desktop Integration**: Tauri API + plugin-shell

## Scripts
```bash
bun install     # Install dependencies
bun run dev     # Start dev server
bun run build   # Production build (runs tsc + vite build)
bun run preview # Preview production build
```

## Architecture
- `features/` — Domain features (chat, tasks, pomodoro), each self-contained with components and hooks
- `components/ui/` — Reusable shadcn-style UI primitives
- `components/sprite/` — AI pet character rendering and animation
- `hooks/` — Shared custom hooks (sprite state, panel windows, reactions)
- `views/` — Top-level window views, resolved by window label via `routing/resolve-view.tsx`
- `types/` — Shared TypeScript type definitions
- `lib/` — Utility functions

## Key Conventions

### Styling
- Use Tailwind CSS utility classes
- Use `cn()` utility from `lib/utils.ts` for conditional classes
- Follow shadcn/ui component patterns

### Components
- Use functional components with hooks
- Keep components focused on single responsibilities
- Use Radix UI primitives for accessibility

### State Management
- React hooks for local state
- Custom hooks in `hooks/` for shared logic
- Tauri API for desktop integration and multi-window behavior

### Type Safety
- Explicit TypeScript types over `any`
- Zod for runtime validation
- Type definitions in `types/`

## Build Output
Vite builds to `dist/` directory, which is consumed by the Tauri app shell.
