#!/usr/bin/env python3
"""Stage OpenCode CLI and a bundled Node.js runtime for Tauri release bundling.

Installs opencode via npm for all platforms, downloads a matching Node.js
binary, and exposes a stable top-level executable (`opencode` on Unix,
`opencode.cmd` on Windows) for runtime lookup.  The wrapper script
automatically uses the bundled Node.js so the app works even when the
user has no system Node.js installation (common on macOS GUI apps).
"""

from __future__ import annotations

import argparse
import io
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tarfile
import urllib.request
import zipfile
from typing import Final

DEFAULT_VERSION = os.environ.get("OPENCODE_BUNDLE_VERSION", "latest")
OPENCODE_NPM_PACKAGE: Final[str] = os.environ.get("OPENCODE_NPM_PACKAGE", "opencode-ai")

# Must match the version pinned in crates/peekoo-node-runtime/src/node_runtime.rs
NODE_VERSION: Final[str] = os.environ.get("NODE_BUNDLE_VERSION", "v20.18.0")

SUPPORTED_TARGETS: Final[set[str]] = {
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
}

WINDOWS_TARGETS: Final[set[str]] = {
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
}

# Rust target triple -> (Node.js os, Node.js arch)
_TARGET_TO_NODE_PLATFORM: Final[dict[str, tuple[str, str]]] = {
    "aarch64-apple-darwin": ("darwin", "arm64"),
    "x86_64-apple-darwin": ("darwin", "x64"),
    "x86_64-unknown-linux-gnu": ("linux", "x64"),
    "aarch64-unknown-linux-gnu": ("linux", "arm64"),
    "x86_64-pc-windows-msvc": ("win", "x64"),
    "aarch64-pc-windows-msvc": ("win", "arm64"),
}


def ensure_npm_available() -> str:
    npm = shutil.which("npm")
    if npm:
        return npm
    raise SystemExit("npm is required to stage OpenCode but was not found on PATH")


def write_package_json(destination_dir: Path) -> None:
    package_json_path = destination_dir / "package.json"
    package_json_path.write_text(
        json.dumps(
            {
                "name": "peekoo-opencode-bundle",
                "private": True,
                "version": "0.0.0",
                "description": "Temporary package used to stage opencode for release bundling",
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )


def run_npm_install(destination_dir: Path, package_spec: str) -> None:
    npm = ensure_npm_available()
    subprocess.run(
        [
            npm,
            "install",
            "--no-package-lock",
            "--no-audit",
            "--no-fund",
            package_spec,
        ],
        cwd=destination_dir,
        check=True,
    )


def create_unix_wrapper(destination_dir: Path) -> Path:
    bin_path = destination_dir / "node_modules" / ".bin" / "opencode"
    if not bin_path.exists() or not bin_path.is_file():
        raise SystemExit(f"Expected npm binary not found at {bin_path}")

    wrapper = destination_dir / "opencode"
    wrapper.write_text(
        "#!/usr/bin/env sh\n"
        "set -eu\n"
        'DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"\n'
        "# Use bundled Node.js if available, otherwise fall back to system node\n"
        'if [ -x "$DIR/node/bin/node" ]; then\n'
        '  export PATH="$DIR/node/bin:$PATH"\n'
        "fi\n"
        'exec "$DIR/node_modules/.bin/opencode" "$@"\n',
        encoding="utf-8",
    )
    mode = wrapper.stat().st_mode
    wrapper.chmod(mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return wrapper


def create_windows_wrapper(destination_dir: Path) -> Path:
    bin_path = destination_dir / "node_modules" / ".bin" / "opencode.cmd"
    if not bin_path.exists() or not bin_path.is_file():
        raise SystemExit(f"Expected npm binary not found at {bin_path}")

    wrapper = destination_dir / "opencode.cmd"
    wrapper.write_text(
        "@echo off\r\n"
        "setlocal\r\n"
        'set "SCRIPT_DIR=%~dp0"\r\n'
        "rem Use bundled Node.js if available\r\n"
        'if exist "%SCRIPT_DIR%node\\node.exe" (\r\n'
        '  set "PATH=%SCRIPT_DIR%node;%PATH%"\r\n'
        ")\r\n"
        'call "%SCRIPT_DIR%node_modules\\.bin\\opencode.cmd" %*\r\n',
        encoding="utf-8",
    )
    return wrapper


def stage_npm_opencode(
    destination_dir: Path, package: str, version: str, target: str
) -> Path:
    destination_dir.mkdir(parents=True, exist_ok=True)
    write_package_json(destination_dir)

    package_spec = package if version == "latest" else f"{package}@{version}"
    run_npm_install(destination_dir, package_spec)

    if target in WINDOWS_TARGETS:
        return create_windows_wrapper(destination_dir)
    return create_unix_wrapper(destination_dir)


# ---------------------------------------------------------------------------
# Node.js binary bundling
# ---------------------------------------------------------------------------


def node_download_url(node_version: str, target: str) -> str:
    """Build the nodejs.org download URL for the given Rust target triple."""
    node_os, node_arch = _TARGET_TO_NODE_PLATFORM[target]
    ext = "zip" if target in WINDOWS_TARGETS else "tar.gz"
    return f"https://nodejs.org/dist/{node_version}/node-{node_version}-{node_os}-{node_arch}.{ext}"


def _extract_tar_gz(archive_bytes: bytes, destination: Path) -> None:
    with tarfile.open(fileobj=io.BytesIO(archive_bytes), mode="r:gz") as tar:
        tar.extractall(path=destination, filter="data")


def _extract_zip(archive_bytes: bytes, destination: Path) -> None:
    with zipfile.ZipFile(io.BytesIO(archive_bytes)) as zf:
        zf.extractall(path=destination)


def stage_node_binary(destination_dir: Path, target: str, node_version: str) -> Path:
    """Download and stage a Node.js binary into ``destination_dir/node/``.

    The resulting layout is:
    - Unix:    ``destination_dir/node/bin/node``
    - Windows: ``destination_dir/node/node.exe``

    Returns the path to the ``node`` directory.
    """
    url = node_download_url(node_version, target)
    print(f"Downloading Node.js from {url}")
    with urllib.request.urlopen(url) as resp:
        archive_bytes = resp.read()
    print(f"Downloaded {len(archive_bytes)} bytes")

    # Extract into a temporary location, then move into the canonical layout.
    tmp_extract = destination_dir / "_node_extract"
    if tmp_extract.exists():
        shutil.rmtree(tmp_extract)
    tmp_extract.mkdir(parents=True)

    if target in WINDOWS_TARGETS:
        _extract_zip(archive_bytes, tmp_extract)
    else:
        _extract_tar_gz(archive_bytes, tmp_extract)

    # The archive extracts into a single top-level directory like
    # ``node-v20.18.0-darwin-arm64/``.  Rename it to ``node/``.
    extracted_dirs = [p for p in tmp_extract.iterdir() if p.is_dir()]
    if len(extracted_dirs) != 1:
        raise SystemExit(
            f"Expected exactly one directory in Node.js archive, got: {extracted_dirs}"
        )

    node_dir = destination_dir / "node"
    if node_dir.exists():
        shutil.rmtree(node_dir)
    extracted_dirs[0].rename(node_dir)
    shutil.rmtree(tmp_extract)

    # Verify the binary exists.
    if target in WINDOWS_TARGETS:
        node_bin = node_dir / "node.exe"
    else:
        node_bin = node_dir / "bin" / "node"
    if not node_bin.exists():
        raise SystemExit(f"Node.js binary not found at expected path: {node_bin}")

    print(f"Staged Node.js {node_version} at {node_dir}")
    return node_dir


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, help="Rust target triple")
    parser.add_argument(
        "--output", required=True, help="Directory to place the opencode binary in"
    )
    parser.add_argument(
        "--version",
        default=DEFAULT_VERSION,
        help="npm package version to install, or 'latest' (default: latest or OPENCODE_BUNDLE_VERSION)",
    )
    parser.add_argument(
        "--node-version",
        default=NODE_VERSION,
        help=f"Node.js version to bundle (default: {NODE_VERSION} or NODE_BUNDLE_VERSION)",
    )
    parser.add_argument(
        "--skip-node",
        action="store_true",
        help="Skip bundling Node.js (for dev/testing only)",
    )
    args = parser.parse_args()

    if args.target not in SUPPORTED_TARGETS:
        supported = ", ".join(sorted(SUPPORTED_TARGETS))
        raise SystemExit(
            f"Unsupported OpenCode bundle target '{args.target}'. Supported: {supported}"
        )

    output = Path(args.output)

    destination = stage_npm_opencode(
        output, OPENCODE_NPM_PACKAGE, args.version, args.target
    )
    print(destination)

    if not args.skip_node:
        stage_node_binary(output, args.target, args.node_version)

    return 0


if __name__ == "__main__":
    sys.exit(main())
