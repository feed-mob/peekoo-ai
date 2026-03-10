# Health Reminder Runtime Architecture

This diagram documents the post-refactor reminder flow after moving scheduling and notification delivery out of the old event-bus-driven timer path.

```mermaid
graph TD
    subgraph Frontend
        PluginList[PluginList + PluginConfigPanel]
        PluginPanel[Health plugin panel iframe]
        SpriteBubble[Sprite bubble]
    end

    subgraph Tauri
        Commands[Tauri commands]
        OsNotify[OS notification API]
    end

    subgraph App
        AgentApp[AgentApplication]
    end

    subgraph Host
        Registry[PluginRegistry]
        Scheduler[peekoo-scheduler]
        Notifications[peekoo-notifications]
        State[PluginStateStore]
        EventBus[Plugin EventBus]
    end

    subgraph Plugin
        Health[health-reminders.wasm]
    end

    PluginList -->|plugin_config_get/set, dnd_get/set| Commands
    PluginPanel -->|plugin_call_tool| Commands
    Commands --> AgentApp
    AgentApp --> Registry
    AgentApp --> Notifications

    Registry --> Scheduler
    Registry --> State
    Registry --> EventBus
    Registry --> Health

    Scheduler -->|schedule:fired| Registry
    Registry -->|on_event| Health
    Health -->|peekoo_notify| Notifications
    Health -->|peekoo_emit_event| EventBus
    Health -->|peekoo_schedule_get/set/cancel| Scheduler
    Health -->|peekoo_config_get, state| State

    Notifications -->|drain_plugin_notifications| AgentApp
    AgentApp --> Commands
    Commands --> OsNotify
    Commands -->|sprite:bubble| SpriteBubble
```

## Countdown Persistence Flow

Timers survive app restarts via wall-clock timestamps stored in SQLite.

```mermaid
sequenceDiagram
    participant Plugin as health-reminders.wasm
    participant State as PluginStateStore (SQLite)
    participant Scheduler as peekoo-scheduler

    Note over Plugin: App start — plugin_init()

    Plugin->>State: load timer_fire_at:water, timer_interval:water
    State-->>Plugin: fire_at=1741628400, interval=2700

    Plugin->>Plugin: remaining = fire_at - now = 1200s
    Plugin->>Scheduler: schedule_set(water, interval=2700, delay_secs=1200)
    Plugin->>State: save timer_fire_at:water = now + 1200

    Note over Scheduler: 1200s later — timer fires

    Scheduler->>Plugin: on_event(schedule:fired, water)
    Plugin->>State: save timer_fire_at:water = now + 2700
    Plugin->>Plugin: notify + push_peek_badges

    Note over Plugin: App closed, 600s pass, app restarts

    Plugin->>State: load timer_fire_at:water
    Plugin->>Plugin: remaining = fire_at - now = 2100s
    Plugin->>Scheduler: schedule_set(water, interval=2700, delay_secs=2100)
```

### Persisted State Keys (per reminder type)

| Key | Type | Description |
|-----|------|-------------|
| `timer_fire_at:<type>` | `u64` (epoch secs) | Wall-clock time when the timer will next fire |
| `timer_interval:<type>` | `u64` (seconds) | Interval used when the timer was set (for stale detection) |

### Edge Cases

| Scenario | `delay_secs` value |
|----------|--------------------|
| No stored timestamp (first run) | `None` (full interval) |
| Interval changed while app closed | `None` (fresh start) |
| Timer overdue, `fire_if_overdue: true` | `Some(0)` (immediate) |
| Timer overdue, `fire_if_overdue: false` | `Some(interval - (overdue % interval))` (skip to next cycle) |
| Timer still pending | `Some(fire_at - now)` |

Health reminders use `fire_if_overdue: false` -- a 45-min timer overdue by 2 min resumes with 43 min remaining, not 0 or 45.

## Notes

- `schedule:fired` replaces the old `timer:tick` model for health reminders.
- DND is enforced inside `peekoo-notifications`, not inside the scheduler.
- Pomodoro lifecycle commands dispatch plugin events so the plugin can pause and restore its schedules.
- Manifest-declared config fields are rendered by the desktop UI and stored through the host config API.
- `Scheduler::set()` accepts `delay_secs: Option<u64>` to override the first fire delay while keeping the full interval for subsequent repeats.
