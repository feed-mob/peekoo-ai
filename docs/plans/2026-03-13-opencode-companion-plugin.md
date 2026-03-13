# OpenCode Companion Plugin

**Date**: 2026-03-13
**Status**: Implemented

## Problem

Peekoo has no awareness of OpenCode activity. When a developer is actively using
OpenCode (LLM producing output, thinking, finished answering), the Peekoo pet
sits idle instead of reflecting the coding session state. There is no visual
feedback loop between the AI coding agent and the desktop pet.

## Solution

Create a two-part plugin — `peekoo-opencode-companion` — that bridges OpenCode
session events to Peekoo's sprite and badge systems:

- **Working sprite** when the LLM is producing output
- **Thinking sprite** when OpenCode is processing/thinking
- **Happy sprite** when the agent has answered the user's question
- **Peek badge** showing session title + elapsed time while the LLM is active

## Architecture

### Data Flow

```
OpenCode (terminal)
  │
  │  JS/TS plugin subscribes to session.status, session.idle,
  │  message.part.updated, session.created events
  │
  └──► Writes state to ~/.peekoo/bridges/peekoo-opencode-companion.json
                               │
                     filesystem (JSON file)
                               │
Peekoo (desktop pet)
  │
  │  WASM plugin polls bridge file every 2s via peekoo_bridge_fs_read()
  │
  ├──► peekoo::mood::set("opencode-working", sticky=true)
  │         │
  │         └──► Tauri flush loop emits pet:react → Sprite shows WORKING animation
  │
  └──► peekoo::badge::set([{label:"OpenCode", value:"Fix bug", icon:"activity"}])
              │
              └──► Tauri flush loop emits sprite:peek-badges → Badge overlay
```

### Bridge File Format

Written by the OpenCode plugin to `~/.peekoo/bridges/peekoo-opencode-companion.json`:

```json
{
  "status": "working",
  "session_title": "Fix auth bug",
  "started_at": 1773397003,
  "updated_at": 1773397050
}
```

### New Platform Capabilities

Two new host functions are required to support this plugin (and future plugins):

#### `peekoo_bridge_fs_read`

Allows WASM plugins to read a scoped file from `~/.peekoo/bridges/<plugin-key>.json`.
External processes write to this file; the plugin reads it. Path is always
scoped to the plugin key — no user-controlled path segments, read-only.

**Permission:** `bridge:fs_read`

#### `peekoo_set_mood`

Allows WASM plugins to trigger sprite mood changes directly. Queues a mood
reaction that the Tauri flush loop emits as a `pet:react` frontend event.

**Permission:** none (always available, similar to `peekoo_log`)

### MoodReactionService

New service in `peekoo-notifications` (alongside `NotificationService` and
`PeekBadgeService`). Thread-safe queue of `MoodReaction { trigger, sticky }`
items. The Tauri flush loop drains this queue and emits `pet:react` events
to the frontend.

## Implementation

### SDK Updates (Rust + AssemblyScript)

Both SDKs must stay in sync. For each new host function:

| Layer | Rust SDK | AssemblyScript SDK |
|-------|----------|-------------------|
| Host fn declaration | `host_fns.rs` +extern | `host.ts` +@external |
| High-level wrapper | new module `.rs` | new module `.ts` |
| Barrel export | `lib.rs` +pub mod | `index.ts` +import/export |

### File Changes

| # | File | Action | Layer |
|---|------|--------|-------|
| **SDK — Rust** | | | |
| 1 | `crates/peekoo-plugin-sdk/src/host_fns.rs` | Modify (+2 externs) | Rust SDK |
| 2 | `crates/peekoo-plugin-sdk/src/bridge.rs` | Create | Rust SDK |
| 3 | `crates/peekoo-plugin-sdk/src/mood.rs` | Create | Rust SDK |
| 4 | `crates/peekoo-plugin-sdk/src/lib.rs` | Modify (+2 modules) | Rust SDK |
| **SDK — AssemblyScript** | | | |
| 5 | `packages/plugin-sdk/assembly/host.ts` | Modify (+2 declarations) | AS SDK |
| 6 | `packages/plugin-sdk/assembly/bridge.ts` | Create | AS SDK |
| 7 | `packages/plugin-sdk/assembly/mood.ts` | Create | AS SDK |
| 8 | `packages/plugin-sdk/assembly/index.ts` | Modify (+2 imports/exports) | AS SDK |
| **Plugin Host** | | | |
| 9 | `crates/peekoo-plugin-host/src/host_functions.rs` | Modify (+2 host fns) | Host |
| 10 | `crates/peekoo-plugin-host/src/runtime.rs` | Modify (pass mood service) | Host |
| **Mood Service** | | | |
| 11 | `crates/peekoo-notifications/src/mood.rs` | Create | Service |
| 12 | `crates/peekoo-notifications/src/lib.rs` | Modify (+module) | Service |
| **App Layer + Tauri** | | | |
| 13 | `crates/peekoo-agent-app/src/application.rs` | Modify (+mood wiring) | App |
| 14 | `apps/desktop-tauri/src-tauri/src/lib.rs` | Modify (+flush mood) | Tauri |
| **Frontend** | | | |
| 15 | `apps/desktop-ui/src/types/pet-event.ts` | Modify (+4 triggers) | UI |
| 16 | `apps/desktop-ui/src/hooks/use-sprite-reactions.ts` | Modify (+4 mappings) | UI |
| **WASM Plugin** | | | |
| 17 | `plugins/peekoo-opencode-companion/Cargo.toml` | Create | Plugin |
| 18 | `plugins/peekoo-opencode-companion/.cargo/config.toml` | Create | Plugin |
| 19 | `plugins/peekoo-opencode-companion/peekoo-plugin.toml` | Create | Plugin |
| 20 | `plugins/peekoo-opencode-companion/src/lib.rs` | Create | Plugin |
| **OpenCode Plugin** | | | |
| 21 | `plugins/peekoo-opencode-companion/opencode-plugin/package.json` | Create | OC Plugin |
| 22 | `plugins/peekoo-opencode-companion/opencode-plugin/peekoo-opencode-companion.ts` | Create | OC Plugin |
| **Build** | | | |
| 23 | `justfile` | Modify (+recipe) | Build |

### Frontend Trigger Mappings

New `PetReactionTrigger` values and their mood mappings:

| Trigger | Mood | When |
|---------|------|------|
| `opencode-working` | `working` | OpenCode session is actively running |
| `opencode-done` | `happy` | Agent answered the question |
| `opencode-idle` | `idle` | No active session |

### OpenCode Plugin Events Used

| Event | Derived Status | Notes |
|-------|---------------|-------|
| `session.status` (running) | `working` | LLM actively generating |
| `session.status` (pending) | `thinking` | Request queued/processing |
| `session.created` | `thinking` | New session started |
| `session.idle` | `happy` → `idle` | Finished, then 5s delay to idle |
| `session.error` | `idle` | Error clears state |
| `message.part.updated` | `working` | Promotes thinking → working |
| `session.updated` | (title update) | Captures title changes |

### Companion Auto-Install

The plugin manifest supports a `[[companions]]` section that declares files to
be installed to well-known external directories when the plugin is loaded by
the Peekoo host.

```toml
[[companions]]
source = "companions/peekoo-opencode-companion.js"
target = "opencode-plugin"
```

**Supported targets:**

| Target ID | Resolves to | Purpose |
|-----------|------------|---------|
| `opencode-plugin` | `~/.config/opencode/plugin/` | OpenCode local plugins |

When the plugin host calls `load_plugin()`, it copies each declared companion
file from the plugin's install directory to the resolved target. This means
the OpenCode JS plugin is auto-installed when Peekoo loads the WASM plugin —
no manual copy step needed.

### Build & Install

Single command: `just plugin-opencode-companion`

1. Builds the OpenCode TS plugin via `bun build` → `companions/` directory
2. Builds Rust WASM plugin
3. Installs all files (manifest, WASM, companions) to `~/.peekoo/plugins/`
4. On next Peekoo start, host auto-copies companion JS to `~/.config/opencode/plugin/`
