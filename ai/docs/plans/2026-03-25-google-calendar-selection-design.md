# Google Calendar selection design

## Goal
Add calendar selection to the Google Calendar plugin while keeping the default behavior of syncing all readable calendars.

## Chosen approach
Use a collapsible Settings section inside the existing Google Calendar panel.

## UX
- Show a collapsible Settings section in the panel.
- After the first successful sync, list readable calendars with checkboxes.
- Mark the primary calendar visually.
- Save the enabled state with an explicit Save button.
- Refresh the agenda after saving.

## Data model
- Keep per-calendar metadata in plugin state:
  - `id`
  - `name`
  - `primary`
  - `access_role`
  - `enabled`
- Newly discovered readable calendars default to `enabled = true`.

## Plugin API changes
- Extend `panel_snapshot` to include stored calendars.
- Add a tool to update selection state from the panel.

## Sync behavior
- Sync only enabled calendars.
- Preserve existing enablement preferences across refreshes.
- If all calendars are disabled, return an empty agenda instead of an error.
- If Google removes a calendar from the account, drop it from future sync state when calendar metadata refreshes.

## Error handling
- If saving settings fails, keep the previous panel state and show an error.
- If the account is not connected, show a helpful message in Settings.

## Testing
- Rust tests for preference updates and enabled-calendar filtering.
- Panel tests for Settings markup and runtime interaction.
