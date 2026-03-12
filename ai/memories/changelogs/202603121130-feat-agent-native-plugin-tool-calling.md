## 2026-03-12 11:30: feat: enable agent to natively call plugin tools during LLM tool loop

**What changed:**
- Created `PluginToolProvider` trait in `peekoo-agent` for dependency-inverted plugin tool integration. The agent crate defines the contract; the app layer provides the implementation.
- Created `PluginToolAdapter` struct that implements pi's `Tool` trait, wrapping each plugin tool so the LLM can invoke it natively during the agent loop (tool call -> WASM execute -> result -> next turn).
- Added `AgentService::extend_plugin_tools()` method to register plugin tools with the pi SDK's `ToolRegistry` after session creation.
- Created `PluginToolProviderImpl` newtype in `peekoo-agent-app` that implements `PluginToolProvider` by delegating to the existing `PluginToolBridge`. Newtype pattern avoids method name collisions between inherent and trait methods.
- Wired `extend_plugin_tools()` in `AgentApplication::create_agent_service()` so plugin tools are registered every time a new agent session is created.
- Removed `with_plugin_prompt()` system prompt injection -- native tool registration replaces the text-only tool descriptions that the LLM could see but not invoke.
- Tool names are namespaced as `plugin__{plugin_key}__{tool_name}` (e.g. `plugin__health-reminders__health_get_status`) to prevent collisions with built-in tools (read, bash, edit, write, grep, find, ls).
- WASM execution uses `tokio::task::spawn_blocking` to avoid blocking the async runtime during synchronous Extism calls.
- Plugin tool errors return `ToolOutput { is_error: true }` so the LLM can retry or adjust rather than crashing the agent loop.
- Added 7 unit tests for `PluginToolAdapter`: namespacing, metadata, provider-to-adapter conversion, execute success, execute error, empty provider.

**Why:**
- Previously, plugin tools were only described in the system prompt as informational text. The LLM could *see* them but had no mechanism to *invoke* them during the agent loop. This was the missing link between the fully-functional WASM plugin system and the AI agent.
- Native tool registration means the LLM sees plugin tools via standard tool definitions (JSON Schema) and can call them autonomously, just like built-in tools.
- Dependency inversion keeps `peekoo-agent` lightweight (no WASM/plugin-host/SQLite dependencies).

**Architecture decisions:**
- `PluginToolProvider` trait lives in `peekoo-agent` (dependency inversion principle).
- `PluginToolProviderImpl` lives in `peekoo-agent-app` (orchestration layer that already depends on both crates).
- Newtype wrapper avoids inherent/trait method name collision on `tool_specs()` and `call_tool()`.
- `is_read_only()` returns `false` for all plugin tools (conservative -- plugins may have side effects).
- Frontend tool calling via Tauri commands (`plugin_call_tool`) continues to work unchanged through `PluginToolProviderImpl::call_plugin_tool()`.

**Files affected:**
- `crates/peekoo-agent/Cargo.toml` (added `async-trait`, `tokio` dependencies)
- `crates/peekoo-agent/src/lib.rs` (added `pub mod plugin_tool` + re-exports)
- `crates/peekoo-agent/src/plugin_tool.rs` (**new** -- trait, spec type, adapter, 7 tests)
- `crates/peekoo-agent/src/service.rs` (added `extend_plugin_tools()` method)
- `crates/peekoo-agent-app/src/lib.rs` (added `pub mod plugin_tool_impl`)
- `crates/peekoo-agent-app/src/plugin_tool_impl.rs` (**new** -- `PluginToolProviderImpl`)
- `crates/peekoo-agent-app/src/application.rs` (replaced `PluginToolBridge` with `Arc<PluginToolProviderImpl>`, wired `extend_plugin_tools()`, removed `with_plugin_prompt()`)

**Verification:**
- `just check` -- 0 warnings
- `just lint` -- 0 warnings
- `just test` -- 126 tests pass (including 7 new plugin_tool tests)
