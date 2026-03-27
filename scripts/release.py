from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path


PROJECT_FILES = [
    "Cargo.toml",
    "apps/desktop-tauri/src-tauri/tauri.conf.json",
    "apps/desktop-ui/package.json",
    "apps/desktop-tauri/src-tauri/Cargo.toml",
]
RELEASE_NOTES_CHECKLIST = [
    "- [ ] Add at least one release-note label: `feature`, `fix`, `docs`, `test`, `chore`, `ci`, or `refactor`",
    "- [ ] Use `skip-changelog` if this PR should be excluded from generated release notes",
]
VERIFICATION_CHECKLIST = ["- [ ] Tests pass locally"]

SEMVER_PATTERN = re.compile(r"^\d+\.\d+\.\d+$")
TOML_VERSION_PATTERN = re.compile(r'(?m)^(version\s*=\s*")(\d+\.\d+\.\d+)(")$')


def validate_version(version: str) -> str:
    if not SEMVER_PATTERN.fullmatch(version):
        raise ValueError("version must match MAJOR.MINOR.PATCH")
    return version


def replace_toml_version(contents: str, version: str) -> str:
    updated, replacements = TOML_VERSION_PATTERN.subn(
        rf"\g<1>{version}\g<3>", contents, count=1
    )
    if replacements != 1:
        raise ValueError("expected exactly one TOML version entry")
    return updated


def replace_json_version(contents: str, version: str) -> str:
    data = json.loads(contents)
    data["version"] = version
    return json.dumps(data, indent=2) + "\n"


def bump_version(project_root: Path, version: str) -> list[str]:
    validate_version(version)

    changed_files: list[str] = []
    for relative_path in PROJECT_FILES:
        path = project_root / relative_path
        contents = path.read_text(encoding="utf-8")

        if path.suffix == ".json":
            updated = replace_json_version(contents, version)
        else:
            updated = replace_toml_version(contents, version)

        if updated != contents:
            path.write_text(updated, encoding="utf-8")
            changed_files.append(relative_path)

    if changed_files:
        subprocess.run(
            ["cargo", "update", "--workspace"],
            cwd=project_root,
            check=True,
        )
        changed_files.append("Cargo.lock")

    return changed_files


def run_git(project_root: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=project_root, check=True)


def run_gh(project_root: Path, *args: str) -> None:
    subprocess.run(["gh", *args], cwd=project_root, check=True)


def parse_gh_json(project_root: Path, *args: str) -> object:
    result = subprocess.run(
        ["gh", *args],
        cwd=project_root,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def ensure_gh_authentication(project_root: Path) -> None:
    try:
        subprocess.run(
            ["gh", "auth", "status"],
            cwd=project_root,
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError as error:
        stderr = error.stderr.strip() if isinstance(error.stderr, str) else ""
        message = "GitHub CLI is not authenticated. Run `gh auth login` and try again."
        if stderr:
            message = f"{message}\n\n`gh auth status` output:\n{stderr}"
        raise SystemExit(message) from error


def release_branch_name(version: str) -> str:
    return f"release/{version}"


def create_release_branch(project_root: Path, version: str) -> str:
    branch_name = release_branch_name(version)
    run_git(project_root, "switch", "-c", branch_name)
    return branch_name


def create_release_commit(project_root: Path, version: str) -> None:
    run_git(project_root, "add", *PROJECT_FILES, "Cargo.lock")
    run_git(project_root, "commit", "-S", "-m", f"chore(release): {version}")


def push_release_branch(project_root: Path, version: str) -> str:
    branch_name = release_branch_name(version)
    run_git(project_root, "push", "-u", "origin", branch_name)
    return branch_name


def build_release_pr_body(version: str) -> str:
    release_notes = "\n".join(RELEASE_NOTES_CHECKLIST)
    verification = "\n".join(VERIFICATION_CHECKLIST)
    return (
        "## What changed\n\n"
        f"- Bump release version to {version}\n\n"
        "## Release notes\n\n"
        f"{release_notes}\n\n"
        "## Verification\n\n"
        f"{verification}\n"
    )


def create_release_pr(project_root: Path, version: str) -> None:
    ensure_gh_authentication(project_root)
    run_gh(
        project_root,
        "pr",
        "create",
        "--base",
        "master",
        "--head",
        release_branch_name(version),
        "--title",
        f"chore(release): {version}",
        "--body",
        build_release_pr_body(version),
    )
    apply_release_label_if_available(project_root, version)


def apply_release_label_if_available(project_root: Path, version: str) -> None:
    labels = parse_gh_json(
        project_root, "label", "list", "--json", "name", "--limit", "200"
    )
    if not isinstance(labels, list):
        return

    label_names = {
        label.get("name")
        for label in labels
        if isinstance(label, dict) and "name" in label
    }
    if "chore" not in label_names:
        return

    run_gh(
        project_root,
        "pr",
        "edit",
        release_branch_name(version),
        "--add-label",
        "chore",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Bump release versions across the desktop app"
    )
    parser.add_argument("version", help="release version in MAJOR.MINOR.PATCH format")
    parser.add_argument(
        "--commit",
        action="store_true",
        help="create a release branch and signed commit after updating versions",
    )
    parser.add_argument(
        "--push",
        action="store_true",
        help="push the release branch and open a pull request after creating the release commit",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    project_root = Path(__file__).resolve().parents[1]
    version = validate_version(args.version)

    if args.push and not args.commit:
        raise SystemExit("--push requires --commit")

    changed_files = bump_version(project_root, version)

    if not changed_files:
        print(f"Release files already set to {version}")
        return 0

    print(f"Updated {len(changed_files)} files for version {version}:")
    for path in changed_files:
        print(f"- {path}")

    if args.commit:
        branch_name = create_release_branch(project_root, version)
        create_release_commit(project_root, version)
        print(f"Created release branch {branch_name} and signed commit")

    if args.push:
        branch_name = push_release_branch(project_root, version)
        create_release_pr(project_root, version)
        print(f"Pushed {branch_name} and opened a pull request")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
