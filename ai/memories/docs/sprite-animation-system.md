# Sprite Animation System

How the desktop pet's sprite sheet animation works end-to-end — from asset to screen.

---

## Overview

The pet character is rendered by playing frames from a PNG sprite sheet (`public/sprite.png`). A magenta-ish background is removed at load time via a chroma-key pass on a hidden canvas, producing a transparent character rendered on top of the desktop.

---

## Sprite Sheet Layout

```
sprite.png  —  1024 × 896 px  —  8 columns × 7 rows  —  56 frames total
frameWidth  = 1024 / 8 = 128 px
frameHeight = 896  / 7 = 128 px
```

### Row → Animation mapping

| Row | `AnimationType` | Product state |
|-----|-----------------|---------------|
| 0   | `idle`          | Default / peeking — gentle breathing, occasional blink |
| 1   | `happy`         | Task completed / celebration |
| 2   | `working`       | Pomodoro active / focus mode |
| 3   | `thinking`      | AI is processing a response |
| 4   | `reminder`      | Notification / panel opened / deadline alert |
| 5   | `sleepy`        | Rest / tired / sad states |
| 6   | `dragging`      | User is dragging the window |

Each row has 8 frames; the animation loops left-to-right at **8 fps** (default).

---

## Component Architecture

```
SpriteView.tsx
 ├─ useSpriteState()          → fetches initial SpriteState from Tauri backend
 ├─ useSpriteReactions()      → listens for pet:react events, calls onMoodChange
 ├─ moodOverride (state)      → 3-second temporary mood set by reactions
 └─ <Sprite state={effective}>
      └─ <SpriteAnimation animation={animationType} />
           └─ <canvas> — renders frames via requestAnimationFrame
```

### Data flow

```
Backend event  →  pet:react (Tauri)
                      ↓
           useSpriteReactions
           TRIGGER_TO_MOOD lookup
                      ↓
           SpriteView.moodOverride  (3 s timeout)
                      ↓
           effectiveSpriteState = { ...backendState, mood: override }
                      ↓
           Sprite: MOOD_TO_ANIMATION[mood] → AnimationType
                      ↓
           SpriteAnimation: ANIMATION_ROWS[type] → row index
                      ↓
           Canvas drawImage(col * frameWidth, row * frameHeight, ...)
```

---

## Key Files

| File | Responsibility |
|------|----------------|
| `src/types/sprite.ts` | `AnimationType` union + `SpriteState` interface |
| `src/components/sprite/SpriteAnimation.tsx` | Canvas animation loop; maps `AnimationType` → row index |
| `src/components/sprite/Sprite.tsx` | Maps `SpriteState.mood` → `AnimationType`; holds random-click override |
| `src/components/sprite/chromaKey.ts` | Offline chroma-key pass to remove magenta background |
| `src/hooks/use-sprite-state.ts` | Fetches `SpriteState` from `get_sprite_state` Tauri command |
| `src/hooks/use-sprite-reactions.ts` | Subscribes to `pet:react` events; maps triggers to moods |
| `src/views/SpriteView.tsx` | Composes all hooks; owns `moodOverride` state |
| `src/types/pet-event.ts` | Zod schema for `PetReactionTrigger` enum |
| `apps/desktop-tauri/src-tauri/src/lib.rs` | `get_sprite_state` command (returns default `SpriteState`) |

---

## Mood System

### SpriteState.mood → AnimationType

Defined in `Sprite.tsx` as `MOOD_TO_ANIMATION`:

| Mood        | Animation   |
|-------------|-------------|
| `happy`     | `happy`     |
| `sad`       | `sleepy`    |
| `thinking`  | `thinking`  |
| `idle`      | `idle`      |
| `tired`     | `sleepy`    |
| `reminder`  | `reminder`  |
| _(unknown)_ | `idle`      |

Unknown moods fall through to `idle` as a safe default.

### Backend animation string → AnimationType

`ANIMATION_TO_TYPE` provides backward compatibility for string animation names that may come from the Rust backend:

| String       | AnimationType |
|--------------|---------------|
| `bounce`     | `happy`       |
| `bounceFast` | `happy`       |
| `pulse`      | `idle`        |
| `pulseFast`  | `happy`       |
| `shake`      | `reminder`    |
| `sway`       | `happy`       |
| `idle`       | `idle`        |
| `working`    | `working`     |
| `thinking`   | `thinking`    |
| `reminder`   | `reminder`    |

---

## Reaction System

### Event triggers → Mood

`use-sprite-reactions.ts` listens on the Tauri `pet:react` event and maps triggers to moods via `TRIGGER_TO_MOOD`:

| Trigger              | Mood        | Sprite animation |
|----------------------|-------------|------------------|
| `chat-message`       | `thinking`  | Thinking         |
| `ai-processing`      | `thinking`  | Thinking         |
| `task-completed`     | `happy`     | Happy/Celebrate  |
| `pomodoro-started`   | `working`   | Working/Focus    |
| `pomodoro-break`     | `idle`      | Idle/Peek        |
| `pomodoro-completed` | `happy`     | Happy/Celebrate  |
| `panel-opened`       | `reminder`  | Reminder         |
| `panel-closed`       | `idle`      | Idle/Peek        |

Mood overrides last **3 seconds** before reverting to the base backend state.

### Emitting events

Backend events are emitted via `emitTo("main", "pet:react", { trigger: "..." })` (TypeScript) or `window.emit("pet:react", ...)` (Rust). The `"ai-processing"` trigger should be emitted by the chat feature when the agent starts streaming.

---

## Chroma Key

The sprite sheet has a solid magenta (`#FF00FF`) background. At image load time, `buildKeyedSpriteSheet()` in `chromaKey.ts` draws the image into an offscreen canvas, runs a pixel-level alpha pass to zero out magenta pixels (with soft edge blending and spill suppression), and returns the processed canvas as the draw source.

The chroma key parameters used in production (`Sprite.tsx`):

```typescript
{
  targetColor: [255, 0, 255],
  minRbOverG: 38,
  threshold: 84,
  softness: 64,
  spillSuppression: { enabled: true, threshold: 230, strength: 0.78 }
}
```

---

## Click Interactions

| Interaction   | Behaviour |
|---------------|-----------|
| Left click    | Triggers a random animation for 2 seconds from `RANDOM_ANIMATIONS`: `["happy", "working", "thinking", "reminder", "sleepy"]` |
| Right click   | Toggles the radial action menu (Chat / Tasks / Pomodoro) |
