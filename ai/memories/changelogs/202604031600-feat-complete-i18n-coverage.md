# 2026-04-03 i18n Completion & Rust i18n Migration

## Summary

Completed full i18n coverage across all frontend features and migrated Rust-side tray menu translations from hardcoded if/else chains to `rust-i18n`.

## Frontend i18n

### New Translation Namespaces (all 5 locales: en, zh, ja, es, fr)
- `chat.authBanner.*` — Auth required banner message and button
- `chat.thinkingBlock` — Thinking block header
- `chat.toolCall.*` — Tool call running status and arguments label
- `chatSettings.noRuntimes`, `chatSettings.installRuntimeHelp`, `chatSettings.runtimeNotFound`, `chatSettings.runtimeUnavailable`, `chatSettings.modelLabel`, `chatSettings.noModelConfigured`, `chatSettings.modelHelp`, `chatSettings.skillsHelp` — Settings panel empty states and helper texts
- `sprite.welcomeMessage` — Default sprite welcome message
- `pomodoro.dateFilter.*` — Date filter options (recent6, today, yesterday, last7days, last30days)
- `pomodoro.memo.*` — Focus/break memo placeholders
- `pomodoro.settings.*` — Long break, cycle, autopilot labels
- `about.openLogs`, `about.unknownError` — About panel strings
- `tasks.activity.you`, `tasks.activity.agent` — Comment author display names
- `tasks.agentWork.attempts_one/other` — Agent attempt count with pluralization
- `tasks.sync.*` — Sync status strings (syncing, waiting, updated just now/seconds/minutes ago)
- `tasks.toast.*` — Toast messages (created, completed, deleted, failed variants)
- `settings.logging`, `settings.loggingHelp`, `settings.acpRuntimes`, `settings.logLevelRestartMsg`, `settings.restartRequired`, `settings.restartNow`, `settings.later` — Settings panel strings
- `updater.*` — Update dialog title, message, buttons
- `agentRuntimes.*` — Entire agent runtimes feature (~55 keys across all components)

### Source Files Updated (20+)
- **Chat**: `ChatPanel.tsx`, `ThinkingBlock.tsx`, `ToolCallCard.tsx`, `ChatSettingsPanel.tsx`
- **Pomodoro**: `PomodoroPanel.tsx`
- **Tasks**: `task-sync.ts`, `task-activity.ts`, `task-agent-work-display.ts`, `TasksPanel.tsx`, `TaskActivitySection.tsx`, `ActivityFeedItem.tsx`, `TaskDetailView.tsx`
- **Settings**: `SettingsPanel.tsx`
- **About**: `AboutPanel.tsx`
- **Lib**: `updater.ts`
- **Agent Runtimes**: All 7 component files + `provider-auth-state.ts` utility

### Utility Function Pattern
Utility functions outside React context accept optional `t?: TFunction` parameter with English fallback:
- `formatSyncStatus(isRefreshing, lastSyncedAt, now, t?)`
- `getCommentAuthorDisplayName(event, t?)`
- `getAgentFailureDetail(task, t?)`
- `getProviderStatusText(status, inspection, statusMessage, t?)`

### Cleanup
- Removed unused `SkillToggleList.tsx` (zero imports anywhere)
- Simplified `TaskDetailView.tsx` recurrence SelectItem: replaced 8-level ternary chain with direct `t(opt.labelKey)` call
- Removed unused `settings.spriteMeta.*` keys from all locale files
- Fixed zh.json syntax errors (unescaped quotation marks)

## Rust i18n Migration

### Before
- 45-line `if/else` chain in `lib.rs` with hardcoded translations for 5 languages
- `TrayMenuLabels` struct with manual per-language matching
- Duplicated strings from frontend locale files

### After
- `rust-i18n` crate with `t!()` macro for compile-time translation lookup
- Locale files in `apps/desktop-tauri/src-tauri/locales/` (en, zh-CN, ja, es, fr JSON)
- Dedicated `tray_i18n.rs` module with clean public API:
  - `set_tray_locale(language)` — sets global locale
  - `tray_toggle()`, `tray_settings()`, `tray_about()`, `tray_quit()` — translated labels
- `lib.rs` reduced by ~50 lines of translation logic

### Benefits
- Adding a new language = adding one JSON file (no code changes)
- Single source of truth for tray menu strings
- Follows project convention: transport layer delegates to dedicated module

## Files Changed
- 28 frontend files (source + locales)
- 4 Rust files (`Cargo.toml`, `lib.rs`, `tray_i18n.rs`, 5 locale files)
- ~1,400 insertions, ~570 deletions
