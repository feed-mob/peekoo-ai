# Installing Peekoo on macOS

## Download

1. Go to the [latest GitHub Release](https://github.com/feed-mob/peekoo-ai/releases/latest).
2. Download the `.dmg` file for macOS (Apple Silicon / ARM64).
3. Open the DMG and drag **Peekoo** into your **Applications** folder.

## Gatekeeper warning

When you first open Peekoo you will see a macOS Gatekeeper warning:

> "Peekoo" is damaged and can't be opened. You should move it to the Trash.

This happens because the app is not yet notarized with Apple. The app is
safe to use — macOS shows this warning for any app downloaded from the
internet that has not been submitted to Apple's notarization service.

## Fix: remove the quarantine attribute

Open **Terminal** (Spotlight → type `Terminal`) and run:

```bash
xattr -cr /Applications/Peekoo.app
```

This strips the quarantine flag that macOS sets on downloaded files. After
running the command, Peekoo will open normally.

You only need to do this once per install. If you update Peekoo to a new
version, you may need to run the command again.

## Alternative fix (System Settings)

On some macOS versions you can allow the app through System Settings instead:

1. Try to open Peekoo (the warning appears).
2. Open **System Settings → Privacy & Security**.
3. Scroll down — you should see a message about Peekoo being blocked.
4. Click **Open Anyway** and confirm.

> **Note:** This method does not always work when macOS reports the app as
> "damaged". If it does not appear in Privacy & Security, use the Terminal
> command above.

## Why does this happen?

Apple requires developers to pay $99/year for an Apple Developer Program
membership to sign and notarize apps. Until Peekoo enrolls in this program,
macOS will flag the app as unverified. This does not mean the app is
malicious — it means Apple has not reviewed it.

Apps like VS Code, Slack, and Discord do not show this warning because their
developers have paid for code signing and notarization.
