# Linear Integration Manual QA Checklist

## Preconditions

1. Linear personal/team API Key is created in Linear settings.
2. Plugin is built and installed:
   - `just plugin-build linear`
   - `just plugin-install linear`

## Smoke Flow

1. Open Peekoo -> Plugins -> Installed.
2. Enable `Linear` plugin.
3. Open `Linear` panel.
4. Enter `Linear API Key`, click `Save API Key`.
5. Confirm panel status becomes `Connected`.

## Sync Validation

1. In Linear, create/update one issue in the selected team.
2. Click `Sync Now` in panel.
3. Confirm issue appears/updates in Peekoo Tasks.
4. In Peekoo, create one new task.
5. Enable `Auto push new Peekoo tasks to Linear` and choose default team.
6. Click `Sync Now`.
7. Confirm task is created in Linear.

## Settings Validation

1. Open Peekoo Settings.
2. Confirm `Integrations -> Linear` shows correct state:
   - Installed / Disabled / Connected / Syncing / Error
3. Confirm `Last Sync` and error text render when applicable.

## Failure Paths

1. Disconnect in panel and confirm state returns to `Disconnected`.
2. Input an invalid API key and verify error banner appears.
3. Temporarily block network and verify status transitions to `Error` after sync.
