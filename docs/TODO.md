# Peekoo AI - Implementation TODO

## Tauri Version (Current Focus)

### ✅ Completed
- [x] Project structure setup (workspace with Tauri + GPUI parallel)
- [x] Core Rust business logic crates (core-domain, core-app, plugin-host, etc.)
- [x] Tauri app scaffolding with Rust commands
- [x] React UI components and styling
- [x] Pet UI with animations and moods
- [x] Tab-based navigation (Chat, Tasks, Pomodoro)
- [x] Tauri commands (greet, get_pet_state, send_message, create_task)
- [x] TypeScript and Vite configuration
- [x] All core domain tests passing

### 🔧 In Progress
- [ ] Integrate core-domain with Tauri commands
  - Connect Task and Pomodoro types from core-domain
  - Implement actual task CRUD operations in Rust backend
  - Implement pomodoro state management in Rust backend
  - Wire up event bus for real-time updates

- [ ] Implement Chat component with real AI integration
  - Connect to LLM backend (OpenAI/Anthropic/etc.)
  - Implement streaming responses
  - Store conversation history in SQLite
  - Add tool execution hooks

- [ ] Implement Tasks component with full CRUD
  - Connect to create_task Tauri command
  - Display task list with filters
  - Implement task completion animations
  - Connect to event bus for updates

- [ ] Implement Pomodoro timer
  - Actual countdown timer logic
  - Start/Pause/Reset functionality
  - Session tracking in database
  - Notification triggers

- [ ] Add plugin system integration
  - Plugin discovery and loading
  - Permission management UI
  - MCP server integration
  - JS bridge for OpenCode/OpenClaw plugins

- [ ] Implement Google Calendar integration
  - OAuth PKCE flow
  - Token storage via OS keychain
  - Calendar sync service
  - Bi-directional sync with peekoo tasks

### 📋 Planned (GPUI Version)
- [ ] Implement GPUI native UI as alternative
  - Native Rust window with pet animations
  - Event-driven architecture
  - Compare performance with Tauri version
  - Test on macOS/Linux only

### 🎯 Polish
- [ ] Add system tray icon
- [ ] Global keyboard shortcuts
- [ ] Sound effects for events
- [ ] Desktop notifications
- [ ] Dark mode theme
- [ ] Settings panel

### 🔧 Testing
- [ ] End-to-end integration tests
- [ ] Performance benchmarking
- [ ] Cross-platform testing (Windows/macOS/Linux for Tauri)
- [ ] Accessibility testing
- [ ] Security audit

---

**Last updated**: 2026-02-14
**Status**: Tauri MVP implementation in progress
