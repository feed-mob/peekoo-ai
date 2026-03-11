# Peek Badge Events Reference

All events involved in the peek badge system, from plugin to sprite display.

---

## Host Function Call (Plugin -> Host)

### `peekoo_set_peek_badge`

**Direction:** WASM plugin -> host runtime  
**Transport:** Extism host function (shared memory)  
**Input:** JSON string -- array of `PeekBadgeItem`  
**Output:** `{"ok": true}`

```json
[
  {
    "label": "Eye Rest",
    "value": "~4 min",
    "icon": "eye",
    "countdown_secs": 240
  },
  {
    "label": "Water",
    "value": "~20 min",
    "icon": "droplet",
    "countdown_secs": 1200
  }
]
```

**When called by health-reminders plugin:**
- `plugin_init()` -- initial badge state after schedules are set up
- `on_event()` -- after handling `schedule:fired` and `system:wake`
- `tool_health_configure()` -- after config changes modify intervals
- `tool_health_dismiss()` -- after a timer is reset

**Host behavior:** Calls `PeekBadgeService::set(plugin_key, items)` which replaces all badges for that plugin and sets the dirty flag.

---

## Tauri Event (Backend -> Frontend)

### `sprite:peek-badges`

**Direction:** Tauri backend -> main window (sprite)  
**Transport:** `app.emit_to("main", "sprite:peek-badges", &badges)`  
**Emitter:** `flush_peek_badges()` in `apps/desktop-tauri/src-tauri/src/lib.rs`  
**Payload:** JSON array of `PeekBadgeItem` (merged from all plugins, sorted by label)

```json
[
  { "label": "Eye Rest", "value": "~4 min", "icon": "eye", "countdown_secs": 240 },
  { "label": "Standup", "value": "~45 min", "icon": "person-standing", "countdown_secs": 2700 },
  { "label": "Water", "value": "~20 min", "icon": "droplet", "countdown_secs": 1200 }
]
```

**When emitted:**
- Every 250ms in the background flush loop (only when dirty flag is set)
- Also flushed synchronously after: `plugin_call_tool`, `plugin_dispatch_event`, `pomodoro_start/pause/resume/finish`

**Frontend listener:** `usePeekBadge` hook in `apps/desktop-ui/src/hooks/use-peek-badge.ts`

---

## Related Existing Events

These events interact with or affect peek badge visibility:

### `sprite:bubble`

**Direction:** Tauri backend -> main window  
**Payload:** `{ "sourcePlugin": "...", "title": "...", "body": "..." }`  
**Effect on badges:** When a bubble fires, the peek badge collapses and hides until the bubble dismisses (~5s).

### `schedule:fired`

**Direction:** Scheduler -> PluginRegistry -> Plugin `on_event()`  
**Payload:** `{ "key": "water" | "eye_rest" | "standup" }`  
**Effect on badges:** The plugin's `on_event` handler calls `push_peek_badges()` after handling the schedule, which updates badge countdown values.

### `system:wake`

**Direction:** Scheduler -> PluginRegistry -> Plugin `on_event()`  
**Payload:** `{}`  
**Effect on badges:** The plugin re-syncs schedule delays from persisted wall-clock timestamps and calls `push_peek_badges()` so countdowns recover after system sleep.

### `plugins-changed`

**Direction:** Tauri command -> all windows  
**Payload:** `()`  
**Effect on badges:** Not directly related, but if a plugin is enabled/disabled, badges from that plugin will appear/disappear on the next flush cycle.

---

## Event Flow Timeline

```
Plugin init
  |
  v
peekoo_set_peek_badge([items]) -- host function
  |
  v
PeekBadgeService.set("health-reminders", items) -- dirty=true
  |
  v
~250ms later: background flush loop
  |
  v
take_peek_badges_if_changed() -> Some([merged items]) -- dirty=false
  |
  v
app.emit_to("main", "sprite:peek-badges", items) -- Tauri event
  |
  v
usePeekBadge hook receives event -- stores snapshot + timestamp
  |
  v
Every 1s: countdown tick decrements values locally
Every 5s: rotation advances to next item
  |
  v
SpritePeekBadge renders above sprite
```

---

## PeekBadgeItem Schema

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `label` | `string` | Yes | Display name (e.g., "Eye Rest") |
| `value` | `string` | Yes | Human-readable countdown (e.g., "~4 min") |
| `icon` | `string` | No | Lucide icon name (e.g., "eye", "droplet", "person-standing") |
| `countdown_secs` | `number` | No | Raw seconds for frontend local countdown ticking |

The Rust struct is defined in `crates/peekoo-notifications/src/peek_badge.rs`.  
The Zod schema is defined in `apps/desktop-ui/src/types/peek-badge.ts`.  
The WASM plugin struct is defined in `plugins/health-reminders/src/lib.rs`.

---

## Icon Mapping (Health Reminders)

| Reminder Type | Icon Name | Lucide Component |
|---------------|-----------|-----------------|
| `water` | `droplet` | `Droplet` |
| `eye_rest` | `eye` | `Eye` |
| `standup` | `person-standing` | `PersonStanding` |
| (fallback) | `activity` | `Activity` |

Icon mapping is in `SpritePeekBadge.tsx` (`ICON_MAP`) and `plugins/health-reminders/src/lib.rs` (`icon_for`).
