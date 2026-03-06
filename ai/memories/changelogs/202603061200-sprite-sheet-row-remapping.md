# 2026-03-06 12:00: refactor: Sprite Sheet Row Remapping & Reaction System

## What changed

Refactored the sprite animation system to match a new 7-row sprite sheet layout where several rows changed purpose. Also completed the previously-TODO reaction hook to actually drive mood changes from backend events.

## Why

The sprite asset (`/sprite.jpg`) was replaced with a new character that uses a different row ordering. Additionally the old row set had semantic gaps (`excited` and `angry` had no real product trigger; `thinking` had no dedicated animation). The new layout aligns sprite rows directly with product states (working = pomodoro, thinking = AI, reminder = notifications).

## Files affected

- `apps/desktop-ui/src/types/sprite.ts`
- `apps/desktop-ui/src/components/sprite/SpriteAnimation.tsx`
- `apps/desktop-ui/src/components/sprite/Sprite.tsx`
- `apps/desktop-ui/src/types/pet-event.ts`
- `apps/desktop-ui/src/hooks/use-sprite-reactions.ts`
- `apps/desktop-ui/src/views/SpriteView.tsx`
- `apps/desktop-tauri/src-tauri/src/lib.rs`

## Details

### New row layout

| Row | AnimationType | Description |
|-----|---------------|-------------|
| 0   | `idle`        | Idle/Peek — gentle breathing, peeking from bottom, occasional blinking |
| 1   | `happy`       | Happy/Celebrate — joyful expression, used on task completion |
| 2   | `working`     | Working/Focus — focused posture, used during pomodoro |
| 3   | `thinking`    | Thinking — thinking posture, used during AI processing |
| 4   | `reminder`    | Reminder — notification expression for deadlines/alerts |
| 5   | `sleepy`      | Sleepy/Rest — tired expression, closed eyes, no yawning |
| 6   | `dragging`    | Dragging — being dragged state |

### Removed animation types
- `"excited"` — merged into `"happy"` (both map to joyful states)
- `"angry"` — removed; had no product-meaningful trigger

### New animation types
- `"thinking"` — dedicated row 3, triggered by AI processing events
- `"reminder"` — dedicated row 4, triggered by panel-opened / notification events

### Mood-to-animation mapping (updated)

| Mood        | Animation   | Notes |
|-------------|-------------|-------|
| `happy`     | `happy`     | unchanged |
| `sad`       | `sleepy`    | was `angry`; closest neutral-down state |
| `thinking`  | `thinking`  | was `working`; now has dedicated row |
| `idle`      | `idle`      | unchanged |
| `tired`     | `sleepy`    | unchanged |
| `reminder`  | `reminder`  | new |

### Backend animation string backward compatibility (`ANIMATION_TO_TYPE`)

| String        | Maps to    | Notes |
|---------------|------------|-------|
| `bounce`      | `happy`    | unchanged |
| `bounceFast`  | `happy`    | was `excited` |
| `pulse`       | `idle`     | unchanged |
| `pulseFast`   | `happy`    | was `excited` |
| `shake`       | `reminder` | was `angry` |
| `sway`        | `happy`    | unchanged |
| `idle`        | `idle`     | unchanged |
| `working`     | `working`  | new explicit entry |
| `thinking`    | `thinking` | new explicit entry |
| `reminder`    | `reminder` | new explicit entry |

### Random animations on click

Updated `RANDOM_ANIMATIONS` to: `["happy", "working", "thinking", "reminder", "sleepy"]`

### Reaction system (was TODO, now implemented)

`use-sprite-reactions.ts` now maps `pet:react` trigger events to mood overrides via `TRIGGER_TO_MOOD`:

| Trigger              | Mood        |
|----------------------|-------------|
| `chat-message`       | `thinking`  |
| `ai-processing`      | `thinking`  |
| `task-completed`     | `happy`     |
| `pomodoro-started`   | `working`   |
| `pomodoro-break`     | `idle`      |
| `pomodoro-completed` | `happy`     |
| `panel-opened`       | `reminder`  |
| `panel-closed`       | `idle`      |

`SpriteView` now holds a `moodOverride` state (3-second timeout) wired through `useSpriteReactions({ onMoodChange })`. The effective sprite state merges the override on top of the backend state.

### New event trigger

Added `"ai-processing"` to `PetReactionTriggerSchema` in `pet-event.ts` — emitted when the AI is actively generating a response (distinct from `"chat-message"` which fires on message receipt).

### Backend default

`get_sprite_state` now returns `mood: "idle"` instead of `mood: "happy"` as the initial state.
