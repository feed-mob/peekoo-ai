#!/usr/bin/env python3
"""Fetch and stage an OpenCode CLI binary for Tauri release bundling."""

from __future__ import annotations

import argparse
import io
import json
import os
from pathlib import Path
import shutil
import stat
import sys
import urllib.request
import zipfile

DEFAULT_REPO = os.environ.get("OPENCODE_RELEASE_REPO", "sst/opencode")
DEFAULT_VERSION = os.environ.get("OPENCODE_BUNDLE_VERSION", "latest")

ASSET_NAMES = {
    "aarch64-apple-darwin": "opencode-darwin-arm64.zip",
    "x86_64-apple-darwin": "opencode-darwin-x64.zip",
    "x86_64-unknown-linux-gnu": "opencode-linux-x64.zip",
    "aarch64-unknown-linux-gnu": "opencode-linux-arm64.zip",
    "x86_64-pc-windows-msvc": "opencode-windows-x64.zip",
    "aarch64-pc-windows-msvc": "opencode-windows-arm64.zip",
}


def github_json(url: str) -> dict:
    req = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json",
            "User-Agent": "peekoo-release-bundler",
        },
    )
    with urllib.request.urlopen(req) as response:
        return json.load(response)


def release_api_url(repo: str, version: str) -> str:
    if version == "latest":
        return f"https://api.github.com/repos/{repo}/releases/latest"
    return f"https://api.github.com/repos/{repo}/releases/tags/{version}"


def asset_name_for_target(target: str) -> str:
    try:
        return ASSET_NAMES[target]
    except KeyError as exc:
        supported = ", ".join(sorted(ASSET_NAMES))
        raise SystemExit(f"Unsupported OpenCode bundle target '{target}'. Supported: {supported}") from exc


def download_bytes(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "peekoo-release-bundler"})
    with urllib.request.urlopen(req) as response:
        return response.read()


def find_asset_download_url(release: dict, asset_name: str) -> str:
    for asset in release.get("assets", []):
        if asset.get("name") == asset_name:
            return asset["browser_download_url"]
    available = ", ".join(asset.get("name", "<unknown>") for asset in release.get("assets", []))
    raise SystemExit(f"OpenCode release asset '{asset_name}' not found. Available assets: {available}")


def stage_binary(archive_bytes: bytes, destination_dir: Path) -> Path:
    destination_dir.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(io.BytesIO(archive_bytes)) as archive:
        candidates = [
            name
            for name in archive.namelist()
            if name.endswith("/opencode")
            or name.endswith("\\opencode")
            or name == "opencode"
            or name.endswith("/opencode.exe")
            or name.endswith("\\opencode.exe")
            or name == "opencode.exe"
        ]
        if not candidates:
            raise SystemExit("Downloaded OpenCode archive did not contain an opencode binary")

        member = candidates[0]
        file_name = "opencode.exe" if member.endswith(".exe") else "opencode"
        destination = destination_dir / file_name

        with archive.open(member) as source, destination.open("wb") as target:
            shutil.copyfileobj(source, target)

        current_mode = destination.stat().st_mode
        destination.chmod(current_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        return destination


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, help="Rust target triple")
    parser.add_argument("--output", required=True, help="Directory to place the opencode binary in")
    parser.add_argument(
        "--repo",
        default=DEFAULT_REPO,
        help=f"GitHub repo providing OpenCode release assets (default: {DEFAULT_REPO})",
    )
    parser.add_argument(
        "--version",
        default=DEFAULT_VERSION,
        help="Release tag to fetch, or 'latest' (default: latest or OPENCODE_BUNDLE_VERSION)",
    )
    args = parser.parse_args()

    asset_name = asset_name_for_target(args.target)
    release = github_json(release_api_url(args.repo, args.version))
    download_url = find_asset_download_url(release, asset_name)
    archive_bytes = download_bytes(download_url)
    destination = stage_binary(archive_bytes, Path(args.output))
    print(destination)
    return 0


if __name__ == "__main__":
    sys.exit(main())
