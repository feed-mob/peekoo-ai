## 2026-03-27 15:01: fix: release workflow trigger after auto tag

**What changed:**
- Switched `.github/workflows/release.yml` from a tag-push trigger to a `workflow_run` trigger that starts after `Auto Tag` completes successfully
- Added a `prepare-release` job that resolves the version tag from `Cargo.toml`, verifies that the matching tag exists, and confirms that it points at the same commit the auto-tag run processed
- Updated the release jobs to build and generate notes from the resolved commit SHA and tag name instead of relying on the event ref
- Updated `docs/release.md` to describe the PR -> merge -> Auto Tag -> Release flow and the remaining manual finalize step

**Why:**
- Tags created by `Auto Tag` use the default GitHub Actions token, which does not trigger downstream `push` workflows
- Triggering `Release` from `workflow_run` preserves the automated release flow without requiring a PAT or GitHub App token
- Resolving the release tag in a dedicated job keeps the release build tied to the exact commit that `Auto Tag` tagged

**Files affected:**
- `.github/workflows/release.yml`
- `docs/release.md`
