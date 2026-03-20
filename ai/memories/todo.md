# Peekoo AI - Implementation TODO

## Tauri Version (Current Focus)

### Completed
- [x] Project structure setup (agent-first workspace)
- [x] Core Rust business logic crates (`peekoo-agent`, `peekoo-agent-app`, `peekoo-agent-auth`, `peekoo-productivity-domain`, `persistence-sqlite`, `security`, `peekoo-paths`)
- [x] Tauri app scaffolding with 19 Rust commands (`greet`, `get_sprite_state`, `agent_prompt`, `agent_set_model`, `agent_get_model`, `agent_settings_get`, `agent_settings_update`, `agent_settings_catalog`, `agent_provider_auth_set_api_key`, `agent_provider_auth_clear`, `agent_provider_config_set`, `agent_oauth_start`, `agent_oauth_status`, `agent_oauth_cancel`, `create_task`, `pomodoro_start`, `pomodoro_pause`, `pomodoro_resume`, `pomodoro_finish`)
- [x] React UI components and styling
- [x] Pet UI with animations and moods
- [x] Tab-based navigation (Chat, Tasks, Pomodoro)
- [x] TypeScript and Vite configuration
- [x] Productivity domain tests passing
- [x] Agent service wrapping pi_agent_rust with persona file loading, skills, and auto-discovery
- [x] Chat panel with settings UI (provider/model selection, auth, skills)
- [x] Streaming responses via agent_prompt with Tauri event emission
- [x] Pomodoro backend commands (start/pause/resume/finish) wired to AgentApplication
- [x] Security crate with KeyringSecretStore, FileSecretStore, FallbackSecretStore
- [x] Settings input validation (non-empty provider/model, max_tool_iterations > 0)

### In Progress
- [ ] Implement Tasks component with full CRUD
  - Connect to create_task Tauri command
  - Display task list with filters
  - Implement task completion animations
  - Connect to backend-driven refresh/update flow

- [ ] Implement Pomodoro timer UI
  - Actual countdown timer logic in frontend
  - Session tracking display
  - Notification triggers

- [x] Store conversation history in SQLite
  - Persist chat sessions across app restarts (pi handles JSONL; indexed via SQLite)
  - Load previous conversations on cold start (fixed CWD mismatch, PR #72)

### Planned (GPUI Version)
- [ ] Implement GPUI native UI as alternative
  - Native Rust window with pet animations
  - Event-driven architecture
  - Compare performance with Tauri version
  - Test on macOS/Linux only

### Polish
- [x] Add system tray icon
- [ ] Global keyboard shortcuts
- [ ] Sound effects for events
- [x] Desktop notifications
- [ ] Dark mode theme
- [x] Sprite window auto-resize for mini chat and reply bubble states
  - Main sprite window now auto-expands/shrinks for mini chat open/close
  - Expanded reading mode uses wider/taller constrained window sizing
  - Rust-managed resize constraints improve behavior on Linux/Wayland
  - See changelog: `ai/memories/changelogs/202603200347-fix-sprite-window-constrained-resize.md`

### Testing
- [ ] End-to-end integration tests
- [ ] Performance benchmarking
- [ ] Cross-platform testing (Windows/macOS/Linux for Tauri)
- [ ] Accessibility testing
- [ ] Security audit

---

**Last updated**: 2026-03-20
**Status**: Tauri MVP implementation in progress, including constrained sprite window auto-resize improvements
