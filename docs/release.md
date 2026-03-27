# Desktop release process

## Supported install targets

- Windows: NSIS and MSI installers attached to GitHub Releases.
- macOS: Apple Silicon DMG for manual installs plus updater archives for in-app updates.
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
4. Set up AUR publishing (see [AUR setup](#aur-setup) below).

## Workflows

- `Auto Tag`: watches `master` for version bumps in `Cargo.toml`, then creates the matching `v0.x.y` tag.
- `Release`: runs after `Auto Tag` succeeds or from manual dispatch, builds installers, creates updater artifacts, drafts a GitHub Release, can publish a draft release as latest, and publishes to AUR when the release is published.
- `PR Release Label`: fails non-draft PRs that do not have a release-note label.
- `CI`: validates the workspace, release tooling tests, and the desktop UI build.

## Required GitHub repo settings

1. Add branch protection for `master` if you want label enforcement before merge.
2. Mark `PR Release Label` and `CI` as required status checks for PRs.
3. Add these repository secrets before the first release:
   - `TAURI_SIGNING_PRIVATE_KEY`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
   - `AUR_SSH_PRIVATE_KEY` (for AUR publishing)
   - `AUR_USERNAME` (for AUR publishing)
   - `AUR_EMAIL` (for AUR publishing)

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

1. Start from a clean working copy.
2. Create a branch for the release bump.
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

5. Create and push the release commit:

   ```bash
   git add Cargo.toml Cargo.lock apps/desktop-tauri/src-tauri/Cargo.toml apps/desktop-tauri/src-tauri/tauri.conf.json apps/desktop-ui/package.json
   git commit -S -m "chore(release): bump version to 0.x.y"
   git push origin <branch>
   ```

6. Open a PR for the release bump and merge it into `master`.
7. `Auto Tag` creates `v0.x.y` from the merged `master` commit.
8. GitHub Actions runs the `Release` workflow automatically after `Auto Tag` finishes.
9. The workflow:
    - builds Windows installers (`nsis`, `msi`)
    - builds a macOS ARM64 `dmg` plus updater archives
    - builds Linux `AppImage` and `deb`
    - signs updater artifacts with the Tauri private key
    - asks GitHub to generate release notes
    - creates a draft GitHub Release
10. Open the draft release in GitHub.
11. Verify the uploaded files and the generated notes.
12. Open `Actions` -> `Release` -> `Run workflow`.
13. Set `finalize_release_tag` to `v0.x.y` and run the workflow.
14. The workflow publishes the draft release, marks it as GitHub's latest release, and triggers the `publish-aur` job automatically.

## Manual workflow dispatch

Use manual dispatch when you want to rebuild artifacts for the checked-in version without creating a new tag first, publish AUR for the latest release, or finalize an already-built draft release.

1. Open `Actions` in GitHub.
2. Select `Release`.
3. Click `Run workflow`.
4. Pick the branch that already contains the target version.
5. Choose one action:
   - leave both inputs empty/false to rebuild draft release artifacts for the checked-in version
   - set `finalize_release_tag` to `v0.x.y` to publish a draft release and mark it as latest
   - set `publish_aur` to `true` to republish the AUR package from the current latest release
6. Run the workflow.

This still uses the version already checked into the repo. It does not bump versions for you.

## After the release

1. Install one artifact on each platform you care about.
2. Smoke-test app launch and the updater check.
3. Confirm the release page includes `latest.json`, the signature files, and the macOS updater archive.
4. AUR is updated automatically when the release is published. Verify the `peekoo-bin` AUR package shows the new version.

## AUR setup

The `publish-aur` job runs automatically when a GitHub Release is published.
It requires one-time setup:

1. Register the `peekoo-bin` package on [aur.archlinux.org](https://aur.archlinux.org/).
2. Generate an SSH key pair for AUR access:

   ```bash
   ssh-keygen -t ed25519 -C "aur" -f ~/.ssh/aur
   ```

3. Add the **public key** (`~/.ssh/aur.pub`) to your AUR account under SSH Keys.
4. Add three secrets to GitHub Actions (`Settings` -> `Secrets and variables` -> `Actions`):
   - `AUR_SSH_PRIVATE_KEY`: contents of `~/.ssh/aur`
   - `AUR_USERNAME`: your AUR username (used as the git commit author)
   - `AUR_EMAIL`: your email (used as the git commit author)

The job patches `packaging/aur/PKGBUILD` with the release version, computes
the AppImage SHA256 checksum automatically, and pushes to the AUR git repo.

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
