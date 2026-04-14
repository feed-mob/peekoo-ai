# Install on macOS

The macOS build is the fastest way to bring Peekoo onto your desktop as a lightweight always-available companion.

## Download

1. Open the [latest GitHub Release](https://github.com/feed-mob/peekoo-ai/releases/latest).
2. Download the macOS `.dmg` file.
3. Open the DMG and drag `Peekoo.app` into `/Applications`.

## Gatekeeper Warning

Peekoo is not notarized yet. macOS may show a warning such as:

> "Peekoo" is damaged and can't be opened. You should move it to the Trash.

This warning is caused by macOS quarantine checks on non-notarized apps.

It does not mean Peekoo is broken. It means macOS is treating the download as an unnotarized app.

## Fix

Run:

```bash
xattr -cr /Applications/Peekoo.app
```

After that, start the app again. You may need to repeat this after upgrading.

## Alternative

On some macOS versions you can also use `System Settings -> Privacy & Security -> Open Anyway`.
