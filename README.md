# Peekoo AI

AI-powered desktop pet built with Tauri v2, React, and Rust. A small, transparent, always-on-top companion that lives on your desktop with chat, task management, and a pomodoro timer.

![Peekoo Demo](assets/Peekoo_Peek.gif)

## Features

- **AI Chat** — Conversational AI with streaming responses, configurable LLM providers, and persona/skill loading
- **Task Management** — Create and track tasks from the desktop pet
- **Pomodoro Timer** — Focus sessions with start/pause/resume/finish controls
- **Animated Sprite** — Desktop pet character with mood-based animations and reactions

## Installation

Download the latest release from [GitHub Releases](https://github.com/feed-mob/peekoo-ai/releases).

### Linux

Install Peekoo from the AUR:

```bash
yay -S peekoo-bin
```

- AUR package: [peekoo-bin](https://aur.archlinux.org/packages/peekoo-bin)

### macOS

Download the latest `.dmg` from [GitHub Releases](https://github.com/feed-mob/peekoo-ai/releases), move `Peekoo.app` to `/Applications`, then remove the quarantine attribute:

```bash
xattr -cr /Applications/Peekoo.app
```

- Detailed macOS instructions: [`docs/en/installation/macos.md`](docs/en/installation/macos.md)

### Windows

Download the latest `x64-setup.exe` installer from [GitHub Releases](https://github.com/feed-mob/peekoo-ai/releases), then run it.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop Shell | Tauri v2 |
| Frontend | React 18 + TypeScript 5 + Vite 5 |
| Styling | Tailwind CSS v4 |
| Backend | Rust (edition 2024, MSRV 1.85) |
| Agent Runtime | pi_agent_rust (v0.1.7) |
| Persistence | SQLite (embedded migrations) |
| Secrets | OS keychain (keyring) with filesystem fallback |

## Prerequisites

- [Rust](https://rustup.rs/) 1.85+
- [Bun](https://bun.sh/)
- [just](https://github.com/casey/just) (command runner)
- Tauri v2 system dependencies ([see Tauri docs](https://v2.tauri.app/start/prerequisites/))

## Getting Started

```bash
# Install all dependencies (frontend + Rust CLI tools like cargo-tauri)
just setup

# Run in development mode
just dev
```

## Commands

```bash
just setup        # Install all dependencies (frontend + Rust tools)
just install      # Install frontend dependencies only
just install-tools # Install required Rust CLI tools (cargo-tauri)
just dev          # Run desktop app in dev mode
just build        # Production build
just check        # Check Rust code
just test         # Run all tests
just fmt          # Format Rust code
just lint         # Lint with Clippy
just clean        # Clean all build artifacts
just icon SOURCE  # Generate app icons from source image
```

## Project Structure

```
peekoo-ai/
├── apps/
│   ├── desktop-ui/           # React + Vite frontend
│   └── desktop-tauri/        # Tauri desktop runtime
├── crates/
│   ├── peekoo-agent/         # Agent runtime (wraps pi_agent_rust)
│   ├── peekoo-agent-app/     # Application orchestration and settings
│   ├── peekoo-agent-auth/    # OAuth and provider auth
│   ├── peekoo-productivity-domain/  # Task and pomodoro domain models
│   ├── persistence-sqlite/   # SQLite migrations
│   ├── security/             # Secret storage (keyring, file, fallback)
│   └── peekoo-paths/         # Shared filesystem path helpers
└── docs/                     # User-facing product and developer docs
```

### Architecture

```
desktop-ui  ->  desktop-tauri  ->  peekoo-agent-app  ->  peekoo-agent
                (transport)        (orchestration)        peekoo-agent-auth
                                                          peekoo-productivity-domain
                                                          persistence-sqlite
                                                          security
                                                          peekoo-paths
```

`desktop-tauri` is a transport layer only. All business logic, persistence, and auth orchestration live in the Rust crates behind `peekoo-agent-app`.

## Plugin I18n Convention

For store-installed plugins, localization is now standardized:

1. Plugin ships locale files in `locales/*.json` (at least `en.json`).
2. Host loads `locales/<app_language>.json`, falls back to `locales/en.json`.
3. If no locale file exists, manifest defaults are used.

Recommended references for plugin authors:

- [`docs/en/develop/plugins.md`](docs/en/develop/plugins.md)
- [`plugins/openclaw-sessions/`](plugins/openclaw-sessions) as a complete example (`locales/` + panel i18n usage)

## License

MIT
