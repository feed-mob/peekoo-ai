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


def create_release_commit(project_root: Path, version: str) -> None:
    run_git(project_root, "add", *PROJECT_FILES, "Cargo.lock")
    run_git(project_root, "commit", "-S", "-m", f"chore(release): {version}")
    run_git(project_root, "tag", f"v{version}")


def push_release_refs(project_root: Path) -> None:
    run_git(project_root, "push")
    run_git(project_root, "push", "--tags")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Bump release versions across the desktop app"
    )
    parser.add_argument("version", help="release version in MAJOR.MINOR.PATCH format")
    parser.add_argument(
        "--commit",
        action="store_true",
        help="create a signed release commit and tag after updating versions",
    )
    parser.add_argument(
        "--push",
        action="store_true",
        help="push the current branch and tags after creating the release commit",
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
        create_release_commit(project_root, version)
        print(f"Created git commit and tag v{version}")

    if args.push:
        push_release_refs(project_root)
        print("Pushed branch and tags")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
