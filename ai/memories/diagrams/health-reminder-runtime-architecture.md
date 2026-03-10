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

## Notes

- `schedule:fired` replaces the old `timer:tick` model for health reminders.
- DND is enforced inside `peekoo-notifications`, not inside the scheduler.
- Pomodoro lifecycle commands dispatch plugin events so the plugin can pause and restore its schedules.
- Manifest-declared config fields are rendered by the desktop UI and stored through the host config API.
