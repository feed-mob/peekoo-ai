# feat(i18n): add Japanese, Spanish, and French language support

## Summary
- Added three new selectable UI languages:
  - `ja` (Japanese)
  - `es` (Spanish)
  - `fr` (French)
- Updated frontend i18n bootstrap/resources and settings language selector.
- Extended backend app language validation and persistence to accept the new language codes.
- Extended tray context menu localization for the new languages.

## Files
- `apps/desktop-ui/src/lib/i18n.ts`
- `apps/desktop-ui/src/features/settings/SettingsPanel.tsx`
- `apps/desktop-ui/src/locales/en.json`
- `apps/desktop-ui/src/locales/zh.json`
- `apps/desktop-ui/src/locales/ja.json`
- `apps/desktop-ui/src/locales/es.json`
- `apps/desktop-ui/src/locales/fr.json`
- `crates/peekoo-app-settings/src/service.rs`
- `apps/desktop-tauri/src-tauri/src/lib.rs`

## Validation
- `npx tsc --noEmit` passed.
- `cargo test -p peekoo-app-settings` passed.
- `cargo check -p peekoo-desktop-tauri` passed.
