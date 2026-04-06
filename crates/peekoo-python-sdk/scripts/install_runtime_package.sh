#!/usr/bin/env bash
set -euo pipefail

PACKAGE_FILE="${1:-}"
TARGET_DIR="${2:-$HOME/.peekoo/python-sdk}"

if [[ -z "$PACKAGE_FILE" ]]; then
  echo "usage: $0 <runtime-package.tar.gz> [target-dir]" >&2
  exit 1
fi

if [[ ! -f "$PACKAGE_FILE" ]]; then
  echo "package file not found: $PACKAGE_FILE" >&2
  exit 1
fi

mkdir -p "$TARGET_DIR"
rm -rf "$TARGET_DIR/python"
tar -xzf "$PACKAGE_FILE" -C "$TARGET_DIR"

echo "Python runtime installed to: $TARGET_DIR/python"
