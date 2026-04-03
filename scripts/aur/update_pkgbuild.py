from __future__ import annotations

import hashlib
import re
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parent
PKGBUILD = ROOT / "PKGBUILD"
SRCINFO = ROOT / ".SRCINFO"
APPIMAGE_SHA_PLACEHOLDER = "REPLACE_WITH_APPIMAGE_SHA256"


def appimage_url(version: str) -> str:
    return f"https://github.com/feed-mob/peekoo-ai/releases/download/v{version}/Peekoo_{version}_amd64.AppImage"


def download_sha256(url: str) -> str:
    digest = hashlib.sha256()
    with urllib.request.urlopen(url) as response:
        while True:
            chunk = response.read(1024 * 1024)
            if not chunk:
                break
            digest.update(chunk)
    return digest.hexdigest()


def update_file(path: Path, version: str, checksum: str) -> None:
    contents = path.read_text(encoding="utf-8")
    contents = re.sub(r"(?m)^pkgver=.+$", f"pkgver={version}", contents)
    contents = re.sub(r"(?m)^\tpkgver = .+$", f"\tpkgver = {version}", contents)
    contents = re.sub(
        r"https://github.com/feed-mob/peekoo-ai/releases/download/v[^/]+/Peekoo_[^_]+_amd64\.AppImage",
        appimage_url(version),
        contents,
    )
    if APPIMAGE_SHA_PLACEHOLDER in contents:
        contents = contents.replace(APPIMAGE_SHA_PLACEHOLDER, checksum)
    else:
        contents = re.sub(
            r"(?m)^\t?sha256sums? = .+$",
            lambda match: re.sub(r"= .+$", f"= {checksum}", match.group(0), count=1),
            contents,
            count=1,
        )
        contents = re.sub(
            r"(?m)^\s+'[0-9a-f]{64}'$", f"  '{checksum}'", contents, count=1
        )
    path.write_text(contents, encoding="utf-8")


def main(version: str) -> int:
    checksum = download_sha256(appimage_url(version))
    update_file(PKGBUILD, version, checksum)
    update_file(SRCINFO, version, checksum)
    return 0


if __name__ == "__main__":
    import sys

    raise SystemExit(main(sys.argv[1]))
