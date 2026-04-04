# feat(ui): add EN/简体中文 localization with persisted app language

## Summary
- Integrated `i18next` + `react-i18next` in `apps/desktop-ui`.
- Added locale resources:
  - `apps/desktop-ui/src/locales/en.json`
  - `apps/desktop-ui/src/locales/zh.json`
- Added app bootstrap i18n initialization in `apps/desktop-ui/src/main.tsx` and `apps/desktop-ui/src/lib/i18n.ts`.
- Added language dropdown in settings UI (`English`, `简体中文`) and synced selection through Tauri command.

## Backend Persistence
- Added `app_language` handling in `crates/peekoo-app-settings/src/service.rs`:
  - default: `en`
  - valid values: `en`, `zh-CN`
- Added app-layer wrappers in `crates/peekoo-agent-app/src/application.rs`.
- Added Tauri commands in `apps/desktop-tauri/src-tauri/src/lib.rs`:
  - `app_settings_get_language`
  - `app_settings_set_language`

## Localization Coverage
- Settings panel labels and status/error messages.
- Pomodoro panel texts, statuses, controls, and settings actions.
- Sprite action menu labels (chat/tasks/pomodoro/plugins).
- Pomodoro badge labels in sprite peek badge (including paused state).

## Validation
- `npx tsc --noEmit` (frontend): passed.
- `cargo test -p peekoo-app-settings`: passed (17 tests).
- `cargo fmt --all`: passed.
