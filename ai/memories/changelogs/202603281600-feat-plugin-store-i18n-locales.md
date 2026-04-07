# feat(i18n): support locales for store-installed plugins

## Summary
- Added plugin-level locale loading in `peekoo-agent-app` from `plugin_dir/locales/*.json`.
- Implemented language fallback strategy:
  1. current app language (e.g. `zh-CN`)
  2. `en.json`
  3. manifest defaults
- Localized plugin metadata returned to frontend:
  - installed plugin list (`plugins_list`): `name`, `description`
  - plugin panels (`plugin_panels_list`): `title`
  - plugin config schema (`plugin_config_schema`): `label`, `description`, option labels
  - store APIs (`plugin_store_catalog`, `plugin_store_install`, `plugin_store_update`): `name`, `description` for installed plugins
- Injected panel runtime locale bootstrap when returning `plugin_panel_html`:
  - `window.__PEEKOO_PLUGIN_LOCALE__`
  - `window.__PEEKOO_PLUGIN_LANG__`
  - `window.peekooPluginT(key, fallback?)`
  - auto-apply attributes: `data-i18n`, `data-i18n-placeholder`, `data-i18n-title`
- Updated plugin authoring docs with the new `locales` folder convention and JSON schema.

## Files
- `crates/peekoo-agent-app/src/plugin_localization.rs`
- `crates/peekoo-agent-app/src/application.rs`
- `crates/peekoo-agent-app/src/lib.rs`
- `docs/plugin-authoring.md`

## Validation
- `cargo check -p peekoo-agent-app` passed.
- `cargo check -p peekoo-desktop-tauri` passed.
