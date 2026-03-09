# Peekoo Plugin System Architecture

This document describes the WASM-based plugin system for the Peekoo AI desktop pet application.

**Related Code:**
- Plugin Host: `crates/peekoo-plugin-host/`
- Application Layer: `crates/peekoo-agent-app/src/plugin.rs`
- UI Bridge: `apps/desktop-ui/src/lib/plugin-panel-bridge.ts`
- Sprite Bubble: `apps/desktop-ui/src/components/sprite/SpriteBubble.tsx`
- Example Plugin: `plugins/health-reminders/`

---

## Diagram 1: High-Level Architecture

```mermaid
graph TB
    subgraph "Frontend Layer (React)"
        UI["SpriteView<br/>Chat & Tasks UI"]
        PanelView["PluginPanelView<br/>Sandboxed Iframe"]
        SpriteBubble["SpriteBubble<br/>Speech Bubble"]
    end
    
    subgraph "Transport Layer (Tauri)"
        TauriCmds["Tauri Commands<br/>plugins_list<br/>plugin_call_tool<br/>plugin_panel_html<br/>..."]
    end
    
    subgraph "Application Layer"
        AgentApp["AgentApplication<br/>Orchestrator"]
        PluginTools["PluginToolBridge<br/>AI Agent Adapter"]
    end
    
    subgraph "Plugin Host (peekoo-plugin-host)"
        Registry["PluginRegistry<br/>Discovery & Lifecycle"]
        EventBus["EventBus<br/>Deferred Event Queue"]
        StateStore["PluginStateStore<br/>KV Persistence"]
        HostFns["Host Functions<br/>peekoo_state_get/set<br/>peekoo_log<br/>peekoo_emit_event<br/>peekoo_notify"]
    end
    
    subgraph "WASM Plugins (Extism)"
        WASM1["health-reminders.wasm"]
        WASM2["example-minimal.wasm"]
    end
    
    subgraph "Persistence"
        SQLite["SQLite Database<br/>plugins table<br/>plugin_state table"]
    end
    
    subgraph "Desktop Integration"
        OSNotify["OS Notifications<br/>notify-send (Linux)<br/>Tauri Notification API"]
    end
    
    UI --> TauriCmds
    PanelView --> TauriCmds
    SpriteBubble -.->|"sprite:bubble event"| TauriCmds
    
    TauriCmds --> AgentApp
    AgentApp --> PluginTools
    AgentApp --> Registry
    
    Registry --> EventBus
    Registry --> StateStore
    Registry --> HostFns
    Registry --> WASM1
    Registry --> WASM2
    
    StateStore --> SQLite
    Registry --> SQLite
    
    EventBus -.->|"drain_events()"| AgentApp
    AgentApp -.->|"emit sprite:bubble"| TauriCmds
    AgentApp -.->|"show notification"| OSNotify
```

---

## Diagram 2: Plugin Lifecycle & Event Flow

```mermaid
sequenceDiagram
    participant App as AgentApplication
    participant Registry as PluginRegistry
    participant WASM as WASM Plugin
    participant EventBus as EventBus
    participant Tauri as Tauri Commands
    participant UI as React UI
    participant OS as OS Notification
    
    Note over App,Registry: Startup Phase
    App->>Registry: create_plugin_registry()
    App->>Registry: discover()
    Registry-->>App: List of (dir, manifest) pairs
    
    loop For each discovered plugin
        App->>Registry: install_plugin(plugin_dir)
        Registry->>Registry: ensure_plugin_row() in DB
        Registry->>Registry: permissions.grant_all_required()
        Registry->>WASM: load WASM module
        Registry->>WASM: plugin_init()
        WASM-->>Registry: {"status":"ok"}
        Registry-->>App: plugin key
    end
    
    Note over App,EventBus: Runtime - Timer Tick
    App->>Registry: dispatch_event("timer:tick", {})
    Registry->>WASM: on_event({event: "timer:tick"})
    
    rect rgb(240, 248, 255)
        Note over WASM: Plugin Logic
        WASM->>WASM: Check reminder timers
        alt Reminder is due
            WASM->>EventBus: peekoo_notify({title, body})
            EventBus-->>WASM: {"ok":true}
        end
    end
    
    WASM-->>Registry: {"ok":true}
    Registry-->>App: void
    
    Note over App,EventBus: Notification Drain
    App->>Registry: drain_events()
    EventBus-->>App: [PluginEvent{event: "plugin:notification", payload}]
    
    rect rgb(255, 240, 245)
        Note over App,OS: Dual Output
        App->>Tauri: show_plugin_notification()
        Tauri->>OS: notify-send / system notification
        
        App->>Tauri: emit("sprite:bubble", payload)
        Tauri->>UI: window.emit("sprite:bubble")
        UI->>UI: showBubble(payload)
        UI->>UI: Auto-dismiss after 5s
    end
    
    Note over App,OS: Tool Invocation
    UI->>Tauri: plugin_call_tool("health_get_status", {})
    Tauri->>App: call_plugin_tool()
    App->>PluginTools: call_tool()
    App->>Registry: call_tool(plugin_key, tool_name)
    Registry->>WASM: tool_health_get_status()
    
    rect rgb(240, 255, 240)
        Note over WASM: Tool Execution
        WASM->>WASM: Load state from KV store
        WASM->>WASM: Format JSON response
    end
    
    WASM-->>Registry: JSON result
    Registry-->>App: JSON string
    App-->>Tauri: JSON string
    Tauri-->>UI: Result
```

---

## Diagram 3: Host Function Interface

```mermaid
graph LR
    subgraph "WASM Plugin (Extism PDK)"
        direction TB
        
        subgraph "Exports (Plugin → Host)"
            PluginInit["plugin_init"]
            OnEvent["on_event"]
            ToolExport["tool_* functions"]
            DataExport["data_* functions"]
        end
        
        subgraph "Imports (Host → Plugin)"
            StateGet["peekoo_state_get"]
            StateSet["peekoo_state_set"]
            Log["peekoo_log"]
            EmitEvent["peekoo_emit_event"]
            Notify["peekoo_notify"]
        end
    end
    
    subgraph "Extism Runtime
    (Sandboxed WASM)"
        Extism["Extism Plugin Instance"]
    end
    
    subgraph "Host Runtime
    (peekoo-plugin-host)"
        direction TB
        
        subgraph "Built-in Functions"
            HostFns["build_host_functions()"]
        end
        
        subgraph "Runtime Services"
            StateStore["PluginStateStore<br/>(SQLite KV)"]
            EventBus["EventBus<br/>(Deferred Queue)"]
            Logger["tracing<br/>(info/warn/error)"]
        end
    end
    
    PluginInit -.->|calls| Extism
    OnEvent -.->|calls| Extism
    ToolExport -.->|calls| Extism
    DataExport -.->|calls| Extism
    
    Extism -.->|"FFI"| StateGet
    Extism -.->|"FFI"| StateSet
    Extism -.->|"FFI"| Log
    Extism -.->|"FFI"| EmitEvent
    Extism -.->|"FFI"| Notify
    
    HostFns --> StateStore
    HostFns --> EventBus
    HostFns --> Logger
    
    Notify -.->|"enqueues"| EventBus
```

### Host Function Details

| Function | Input | Purpose |
|----------|-------|---------|
| `peekoo_state_get` | `{"key": "..."}` | Read from plugin's KV store |
| `peekoo_state_set` | `{"key": "...", "value": {...}}` | Write to plugin's KV store |
| `peekoo_log` | `{"level": "info", "message": "..."}` | Log to Peekoo tracing |
| `peekoo_emit_event` | `{"event": "...", "payload": {...}}` | Emit event to EventBus |
| `peekoo_notify` | `{"title": "...", "body": "..."}` | Queue notification for drain |

---

## Diagram 4: UI Panel Loading & Bridge Flow

```mermaid
sequenceDiagram
    participant UI as PluginPanelView
    participant Tauri as Tauri Command
    participant App as AgentApplication
    participant FS as File System
    participant Iframe as Plugin Panel Iframe
    participant Bridge as PostMessage Bridge
    
    rect rgb(230, 245, 255)
        Note over UI,FS: Panel HTML Loading
        UI->>Tauri: plugin_panel_html(label)
        Tauri->>App: plugin_panel_html("panel-health")
        App->>FS: Read ui/panel.html
        App->>FS: Read ui/panel.css (if exists)
        App->>FS: Read ui/panel.js (if exists)
        App->>App: Inline CSS into {style} tag
        App->>App: Inline JS into {script} tag
        App-->>Tauri: Complete HTML string
        Tauri-->>UI: HTML string
    end
    
    rect rgb(245, 230, 255)
        Note over UI,Iframe: Bridge Injection
        UI->>UI: injectPluginPanelBridge(html)
        UI->>Iframe: Render with srcDoc
    end
    
    rect rgb(255, 245, 230)
        Note over Bridge,Iframe: Plugin Makes Tauri Call
        Iframe->>Bridge: window.__TAURI__.core.invoke(cmd, payload)
        Bridge->>Bridge: Generate unique request ID
        Bridge->>UI: window.parent.postMessage({type, id, command, payload})
    end
    
    rect rgb(230, 255, 245)
        Note over UI,Tauri: Command Execution
        UI->>UI: Receive message event
        UI->>Tauri: invoke(command, payload)
        Tauri-->>UI: Result
    end
    
    rect rgb(255, 230, 245)
        Note over Bridge,UI: Response Routing
        UI->>Bridge: postMessage({type, id, ok, result})
        Bridge->>Bridge: Find pending promise by ID
        Bridge->>Iframe: Resolve promise with result
    end
```

### Why PostMessage Bridge?

The plugin panel iframe is **sandboxed** (`sandbox="allow-scripts"`) and does **not** have access to:
- `window.__TAURI__` directly
- Tauri APIs

The bridge injects a script that:
1. Intercepts `window.__TAURI__.core.invoke()` calls
2. Uses `postMessage` to communicate with parent React component
3. Parent component executes actual Tauri command
4. Response is sent back via `postMessage` and resolves the original promise

---

## Key Design Decisions

### 1. Why Extism WASM Runtime?
- **Sandboxing**: Plugins run in isolated WASM memory, cannot access host filesystem directly
- **Portability**: Same plugin binary runs on Windows, macOS, Linux
- **Performance**: Near-native execution speed
- **Tooling**: Extism PDK provides clean Rust API for host functions

### 2. Why Deferred EventBus?
Plugins emit events via `peekoo_emit_event` during WASM execution while the registry lock is held. Immediate dispatch would be **re-entrant** (calling back into the same plugin). Events are **enqueued** and **drained** after each plugin call returns.

### 3. Why Iframe Sandboxing + PostMessage?
- **Security**: Plugin-provided HTML could contain malicious scripts
- **Isolation**: Sandboxed iframe cannot access parent window's state or Tauri APIs
- **Bridge Pattern**: Clean interface between untrusted plugin code and trusted host

### 4. Why Dual Notification Output?
- **OS Notification**: Works when app is not focused, persists in system tray
- **Sprite Bubble**: Inline UI feedback when app is visible, more personal/engaging

---

## Integration Points

### Agent Integration
- `PluginToolBridge` collects tool definitions from all loaded plugins
- Tool specs are injected into the agent's system prompt
- Agent can call plugin tools by name; bridge routes to correct plugin

### Event System
Plugins subscribe to events in `peekoo-plugin.toml`:
```toml
[events]
subscribe = ["timer:tick", "pomodoro:finished"]
emit = ["health:reminder-due"]
```

System emits `timer:tick` every 60 seconds via background thread.

### State Persistence
Each plugin has isolated KV store in SQLite:
```sql
plugin_state(plugin_id, state_key, value_json)
```
Plugin can only access its own keys (enforced by SQL queries).

---

## Related Documentation

- [Plugin Manifest Format](crates/peekoo-plugin-host/src/manifest.rs)
- [Host Functions Implementation](crates/peekoo-plugin-host/src/host_functions.rs)
- [Example Plugin](plugins/health-reminders/)
