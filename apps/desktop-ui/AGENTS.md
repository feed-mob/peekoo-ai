# AGENTS.md - desktop-ui

## Overview
React + Vite + TypeScript frontend for Peekoo AI desktop pet. Features chat, tasks, pomodoro timer, and animated sprite character.

## Tech Stack
- **Framework**: React 18 + TypeScript 5
- **Build**: Vite 5
- **Styling**: Tailwind CSS v4 + tailwindcss-animate
- **Animations**: Framer Motion + React Spring
- **UI Primitives**: Radix UI (checkbox, scroll-area, separator, tooltip, slot)
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

## Project Structure
```
src/
├── main.tsx                    # Entry point
├── index.css                   # Tailwind CSS entry + custom styles
├── features/
│   ├── chat/                   # Chat feature
│   │   ├── ChatPanel.tsx
│   │   └── ChatMessage.tsx
│   ├── tasks/                  # Tasks feature
│   │   ├── TasksPanel.tsx
│   │   ├── TaskInput.tsx
│   │   └── TaskItem.tsx
│   └── pomodoro/               # Pomodoro timer feature
│       ├── PomodoroPanel.tsx
│       ├── TimerControls.tsx
│       └── TimerDisplay.tsx
├── components/
│   ├── ui/                     # UI primitives (shadcn-style)
│   │   ├── button.tsx
│   │   ├── card.tsx
│   │   ├── checkbox.tsx
│   │   ├── input.tsx
│   │   ├── scroll-area.tsx
│   │   ├── separator.tsx
│   │   ├── tooltip.tsx
│   │   └── badge.tsx
│   ├── panels/                 # Panel components
│   │   └── PanelShell.tsx
│   └── sprite/                 # AI pet character
│       ├── Sprite.tsx
│       ├── SpriteAnimation.tsx
│       ├── SpriteActionMenu.tsx
│       └── chromaKey.ts
├── hooks/
│   ├── use-sprite-state.ts     # Sprite state management
│   ├── use-panel-windows.ts    # Panel window management
│   └── use-sprite-reactions.ts # Sprite reaction triggers
├── lib/
│   └── utils.ts                # Utility functions (cn, etc.)
├── routing/
│   └── (routes if needed)
└── types/
    ├── panel.ts                # Panel type definitions
    ├── sprite.ts               # Sprite type definitions
    ├── chat.ts                 # Chat message types
    ├── task.ts                 # Task type definitions
    └── window.ts               # Window state types
```

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
- Tauri API for desktop integration

### Type Safety
- Explicit TypeScript types over `any`
- Zod for runtime validation
- Type definitions in `types/`

## Dependencies to Note
- `@tauri-apps/api` - Desktop app integration
- `framer-motion` / `@react-spring/web` - Animations
- `streamdown` - Markdown streaming for chat
- `lucide-react` - Icons
- `zod` - Schema validation

## Build Output
Vite builds to `dist/` directory, which is consumed by the Tauri app shell.
