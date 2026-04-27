from __future__ import annotations

import argparse
import json
import os
import re
import sys
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def github_headers(accept: str = "application/vnd.github+json") -> dict[str, str]:
    headers = {
        "Accept": accept,
        "User-Agent": "peekoo-release-manifest-builder",
    }
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        headers["Authorization"] = f"Bearer {token}"
    return headers


def fetch_json(url: str) -> dict[str, Any]:
    request = urllib.request.Request(url, headers=github_headers())
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def fetch_asset_text(asset_api_url: str) -> str:
    request = urllib.request.Request(
        asset_api_url,
        headers=github_headers("application/octet-stream"),
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read().decode("utf-8").strip()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build a merged Tauri updater latest.json")
    parser.add_argument("--owner", required=True)
    parser.add_argument("--repo", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument(
        "--prefer-nsis",
        action="store_true",
        help="Use the NSIS installer as the default windows-x86_64 target",
    )
    parser.add_argument(
        "--require-platform",
        action="append",
        default=[],
        help="Fail if the merged manifest does not contain this platform key",
    )
    return parser.parse_args()


def add_platform(
    platforms: dict[str, dict[str, str]],
    key: str,
    asset: dict[str, Any],
    signature: str,
) -> None:
    platforms[key] = {
        "signature": signature,
        "url": asset["browser_download_url"],
    }


def asset_by_suffix(assets: list[dict[str, Any]], suffix: str) -> dict[str, Any] | None:
    for asset in assets:
        if asset["name"].endswith(suffix):
            return asset
    return None


def build_manifest(release: dict[str, Any], version: str, prefer_nsis: bool) -> dict[str, Any]:
    assets: list[dict[str, Any]] = release.get("assets", [])
    platforms: dict[str, dict[str, str]] = {}

    linux_appimage = asset_by_suffix(assets, ".AppImage")
    if linux_appimage is not None:
        linux_appimage_sig = asset_by_suffix(assets, ".AppImage.sig")
        if linux_appimage_sig is None:
            raise RuntimeError("Missing AppImage signature asset")
        signature = fetch_asset_text(linux_appimage_sig["url"])
        add_platform(platforms, "linux-x86_64", linux_appimage, signature)
        add_platform(platforms, "linux-x86_64-appimage", linux_appimage, signature)

    linux_deb = asset_by_suffix(assets, ".deb")
    if linux_deb is not None:
        linux_deb_sig = asset_by_suffix(assets, ".deb.sig")
        if linux_deb_sig is None:
            raise RuntimeError("Missing deb signature asset")
        add_platform(platforms, "linux-x86_64-deb", linux_deb, fetch_asset_text(linux_deb_sig["url"]))

    windows_msi = next((asset for asset in assets if asset["name"].endswith(".msi")), None)
    windows_nsis = next((asset for asset in assets if asset["name"].endswith("-setup.exe")), None)
    if windows_msi is not None:
        windows_msi_sig = asset_by_suffix(assets, ".msi.sig")
        if windows_msi_sig is None:
            raise RuntimeError("Missing MSI signature asset")
        signature = fetch_asset_text(windows_msi_sig["url"])
        if not prefer_nsis:
            add_platform(platforms, "windows-x86_64", windows_msi, signature)
        add_platform(platforms, "windows-x86_64-msi", windows_msi, signature)

    if windows_nsis is not None:
        windows_nsis_sig = asset_by_suffix(assets, "-setup.exe.sig")
        if windows_nsis_sig is None:
            raise RuntimeError("Missing NSIS signature asset")
        signature = fetch_asset_text(windows_nsis_sig["url"])
        if prefer_nsis:
            add_platform(platforms, "windows-x86_64", windows_nsis, signature)
        add_platform(platforms, "windows-x86_64-nsis", windows_nsis, signature)

    mac_arch_pattern = re.compile(r"_(aarch64|x86_64|x64)\.app\.tar\.gz$")
    mac_assets = [
        asset
        for asset in assets
        if asset["name"].endswith(".app.tar.gz") and not asset["name"].endswith(".app.tar.gz.sig")
    ]
    for mac_asset in mac_assets:
        match = mac_arch_pattern.search(mac_asset["name"])
        if match is None:
            raise RuntimeError(f"Cannot infer macOS arch from asset name: {mac_asset['name']}")
        raw_arch = match.group(1)
        arch = "x86_64" if raw_arch in {"x64", "x86_64"} else "aarch64"
        sig_name = f"{mac_asset['name']}.sig"
        mac_sig = next((asset for asset in assets if asset["name"] == sig_name), None)
        if mac_sig is None:
            raise RuntimeError(f"Missing macOS updater signature asset for {mac_asset['name']}")
        signature = fetch_asset_text(mac_sig["url"])
        add_platform(platforms, f"darwin-{arch}", mac_asset, signature)
        add_platform(platforms, f"darwin-{arch}-app", mac_asset, signature)

    if not platforms:
        raise RuntimeError("No updater platforms were discovered from release assets")

    pub_date = (
        release.get("published_at")
        or release.get("created_at")
        or datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    )

    return {
        "version": version,
        "notes": release.get("body") or "",
        "pub_date": pub_date,
        "platforms": platforms,
    }


def main() -> int:
    args = parse_args()
    release_url = f"https://api.github.com/repos/{args.owner}/{args.repo}/releases/tags/{args.tag}"

    try:
      release = fetch_json(release_url)
      manifest = build_manifest(release, args.version, args.prefer_nsis)
    except (RuntimeError, urllib.error.URLError, urllib.error.HTTPError) as error:
      print(str(error), file=sys.stderr)
      return 1

    missing_platforms = [
        platform
        for platform in args.require_platform
        if platform not in manifest["platforms"]
    ]
    if missing_platforms:
        print(
            "Missing required updater platform(s): " + ", ".join(sorted(missing_platforms)),
            file=sys.stderr,
        )
        return 1

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
