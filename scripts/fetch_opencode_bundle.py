#!/usr/bin/env python3
"""Stage OpenCode CLI for Tauri release bundling.

Installs opencode via npm for all platforms and exposes a stable
top-level executable (`opencode` on Unix, `opencode.cmd` on Windows)
for runtime lookup.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
from typing import Final

DEFAULT_VERSION = os.environ.get("OPENCODE_BUNDLE_VERSION", "latest")
OPENCODE_NPM_PACKAGE: Final[str] = os.environ.get("OPENCODE_NPM_PACKAGE", "opencode-ai")

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

LINUX_TARGETS: Final[set[str]] = {
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
}

LINUX_PACKAGE_BY_TARGET: Final[dict[str, str]] = {
    "x86_64-unknown-linux-gnu": "opencode-linux-x64-baseline",
    "aarch64-unknown-linux-gnu": "opencode-linux-arm64",
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


def recreate_directory(destination_dir: Path) -> None:
    if destination_dir.exists():
        shutil.rmtree(destination_dir)
    destination_dir.mkdir(parents=True, exist_ok=True)


def create_unix_wrapper(destination_dir: Path) -> Path:
    bin_path = destination_dir / "node_modules" / ".bin" / "opencode"
    if not bin_path.exists() or not bin_path.is_file():
        raise SystemExit(f"Expected npm binary not found at {bin_path}")

    wrapper = destination_dir / "opencode"
    wrapper.write_text(
        "#!/usr/bin/env sh\n"
        "set -eu\n"
        'DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"\n'
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
        'call "%SCRIPT_DIR%node_modules\\.bin\\opencode.cmd" %*\r\n',
        encoding="utf-8",
    )
    return wrapper


def stage_npm_opencode(
    destination_dir: Path, package: str, version: str, target: str
) -> Path:
    recreate_directory(destination_dir)
    write_package_json(destination_dir)

    package_spec = package if version == "latest" else f"{package}@{version}"
    run_npm_install(destination_dir, package_spec)

    if target in WINDOWS_TARGETS:
        return create_windows_wrapper(destination_dir)
    return create_unix_wrapper(destination_dir)


def stage_linux_opencode(destination_dir: Path, version: str, target: str) -> Path:
    package_name = LINUX_PACKAGE_BY_TARGET[target]
    package_spec = package_name if version == "latest" else f"{package_name}@{version}"

    with tempfile.TemporaryDirectory(prefix="peekoo-opencode-") as temp_dir:
        temp_path = Path(temp_dir)
        write_package_json(temp_path)
        run_npm_install(temp_path, package_spec)

        binary_path = temp_path / "node_modules" / package_name / "bin" / "opencode"
        if not binary_path.exists() or not binary_path.is_file():
            raise SystemExit(f"Expected Linux opencode binary not found at {binary_path}")

        recreate_directory(destination_dir)
        staged_binary = destination_dir / "opencode"
        shutil.copy2(binary_path, staged_binary)
        mode = staged_binary.stat().st_mode
        staged_binary.chmod(mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        return staged_binary


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
    args = parser.parse_args()

    if args.target not in SUPPORTED_TARGETS:
        supported = ", ".join(sorted(SUPPORTED_TARGETS))
        raise SystemExit(
            f"Unsupported OpenCode bundle target '{args.target}'. Supported: {supported}"
        )

    if args.target in LINUX_TARGETS:
        destination = stage_linux_opencode(Path(args.output), args.version, args.target)
    else:
        destination = stage_npm_opencode(
            Path(args.output), OPENCODE_NPM_PACKAGE, args.version, args.target
        )
    print(destination)
    return 0


if __name__ == "__main__":
    sys.exit(main())
