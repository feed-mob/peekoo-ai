# feat(plugin-i18n): localize openclaw-sessions plugin

## Summary
- Added plugin locale resources for `openclaw-sessions`:
  - `locales/en.json`
  - `locales/zh-CN.json`
- Migrated panel static text to i18n attributes (`data-i18n`, `data-i18n-placeholder`, `data-i18n-title`).
- Replaced runtime hardcoded strings with `peekooPluginT` lookups for:
  - connection status
  - session table empty states and action labels
  - pagination labels
  - chat role/empty/loading/error strings
  - config validation and gateway error messages
- Added interpolation helper for `{{vars}}` placeholders in panel runtime.

## Files
- `plugins/openclaw-sessions/locales/en.json`
- `plugins/openclaw-sessions/locales/zh-CN.json`
- `plugins/openclaw-sessions/ui/panel.html`

## Validation
- Locale JSON parse check passed.
- `cargo check -p peekoo-agent-app` passed.
