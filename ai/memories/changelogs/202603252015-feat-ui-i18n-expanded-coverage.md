# feat(ui): expand localization coverage across chat/about/plugins/tasks

## Summary
- Localized panel titles for Chat, Tasks, About, and Plugins views.
- Localized chat panel and chat settings subcomponents (provider/model/auth/skills flows).
- Localized sprite mini-chat input and mini-chat bubble status labels.
- Localized about panel labels/actions and updater status text.
- Localized plugins manager/store/runtime settings UI labels and actions.
- Localized task module core surfaces:
  - main panel tabs, empty states, counters
  - quick input
  - list section headers
  - delete confirm dialog
  - activity feed + task activity section
  - task detail labels and recurrence option labels
- Added comprehensive translation keys in:
  - `apps/desktop-ui/src/locales/en.json`
  - `apps/desktop-ui/src/locales/zh.json`

## Validation
- `npx tsc --noEmit` passed.
