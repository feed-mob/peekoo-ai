# Desktop release process

## Supported install targets

- Windows: NSIS and MSI installers attached to GitHub Releases.
- macOS: Apple Silicon DMG attached to GitHub Releases.
- Linux: AppImage and DEB attached to GitHub Releases.
- Arch Linux: `peekoo-bin` AUR package sourced from the GitHub Release AppImage.

## One-time setup

1. Generate a Tauri updater key pair locally:

   ```bash
   cargo tauri signer generate -w ~/.tauri/peekoo.key
   ```

2. Commit the generated public key in `apps/desktop-tauri/src-tauri/tauri.conf.json`.
3. Add the private key and password to GitHub Actions secrets:
   - `TAURI_SIGNING_PRIVATE_KEY`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
4. If you want automatic AUR publishing later, add an `AUR_SSH_KEY` secret with push access to `peekoo-bin`.

## Workflows

- `Release`: builds installers on tag push or manual dispatch, creates updater artifacts, and drafts a GitHub Release.
- `PR Release Label`: fails non-draft PRs that do not have a release-note label.
- `CI`: validates the workspace, release tooling tests, and the desktop UI build.

## Required GitHub repo settings

1. Add branch protection for `master` if you want label enforcement before merge.
2. Mark `PR Release Label` and `CI` as required status checks for PRs.
3. Add these repository secrets before the first release:
   - `TAURI_SIGNING_PRIVATE_KEY`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

## Release labels

GitHub generates release notes from merged PRs. The sections come from labels in `.github/release.yml`.

- `feature` or `feat` -> `Features`
- `fix` -> `Fixes`
- `docs` -> `Documentation`
- `test` or `tests` -> `Tests`
- `chore`, `ci`, or `refactor` -> `Maintenance`
- `skip-changelog` -> exclude the PR from release notes

Use `.github/pull_request_template.md` as the default checklist when opening PRs.

## Standard tagged release

1. Merge the changes you want released into `master`.
2. Start from a clean working copy.
3. Run:

   ```bash
   just release-bump 0.x.y
   ```

4. Review the version changes and run the checks you want, for example:

   ```bash
   python -m unittest tests.test_release
   cargo check
   cargo test -p peekoo-desktop-tauri
   cd apps/desktop-ui && bun run build
   ```

5. Create and push the release commit and tag:

   ```bash
   just release 0.x.y
   ```

6. GitHub Actions runs the `Release` workflow automatically from the `v0.x.y` tag.
7. The workflow:
   - builds Windows installers (`nsis`, `msi`)
   - builds a macOS ARM64 `dmg`
   - builds Linux `AppImage` and `deb`
   - signs updater artifacts with the Tauri private key
   - asks GitHub to generate release notes
   - creates a draft GitHub Release
8. Open the draft release in GitHub.
9. Verify the uploaded files and the generated notes.
10. Publish the release.

## Manual workflow dispatch

Use manual dispatch when you want to rebuild artifacts for the checked-in version without creating a new tag first.

1. Open `Actions` in GitHub.
2. Select `Release`.
3. Click `Run workflow`.
4. Pick the branch that already contains the target version.
5. Run the workflow.

This still uses the version already checked into the repo. It does not bump versions for you.

## After the release

1. Install one artifact on each platform you care about.
2. Smoke-test app launch and the updater check.
3. Confirm the release page includes `latest.json` and the signature files.
4. If you maintain AUR manually, update `packaging/aur/PKGBUILD` and `.SRCINFO` with the new AppImage checksum.

## Updating GitHub secrets

Load the values from the generated local files:

- private key: `~/.tauri/peekoo.key`
- key password: `~/.tauri/peekoo.key.password`

In GitHub:

1. Open `Settings` -> `Secrets and variables` -> `Actions`.
2. Create `TAURI_SIGNING_PRIVATE_KEY` with the full private key contents.
3. Create `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` with the password file contents.

## Notes

- macOS signing and notarization are not configured yet. Users must run `xattr -cr /Applications/Peekoo.app` after installing. See [docs/install-macos.md](install-macos.md) for details. Release notes include this instruction automatically.
- Windows code signing is not configured yet, so SmartScreen warnings will still appear.
- The updater only works after the public key placeholder is replaced and release artifacts are signed by the workflow.
- Generated release notes come from GitHub's release notes API and are grouped by `.github/release.yml` labels.
- To get clean sections, label PRs with names like `feature`, `fix`, `docs`, `test`, `chore`, `ci`, or `refactor`.
- Add the `skip-changelog` label to any PR you do not want included in release notes.
- `.github/pull_request_template.md` reminds contributors to pick a release-note label before merge.
- `.github/workflows/pr-release-label.yml` fails non-draft PRs that are missing a release-note label.
