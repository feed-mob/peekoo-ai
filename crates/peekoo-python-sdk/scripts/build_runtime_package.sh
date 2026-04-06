#!/usr/bin/env bash
set -euo pipefail

REQ_FILE="${1:-}"
OUT_FILE="${2:-}"

if [[ -z "$REQ_FILE" || -z "$OUT_FILE" ]]; then
  echo "usage: $0 <requirements.txt> <output.tar.gz>" >&2
  exit 1
fi

if [[ ! -f "$REQ_FILE" ]]; then
  echo "requirements file not found: $REQ_FILE" >&2
  exit 1
fi

platform="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
case "$platform:$arch" in
  darwin:arm64) target_triple="aarch64-apple-darwin" ;;
  darwin:x86_64) target_triple="x86_64-apple-darwin" ;;
  linux:x86_64) target_triple="x86_64-unknown-linux-gnu" ;;
  linux:aarch64) target_triple="aarch64-unknown-linux-gnu" ;;
  *)
    echo "unsupported platform: $platform/$arch" >&2
    exit 1
    ;;
esac

release_tag="${PEEKOO_PYTHON_STANDALONE_TAG:-20250317}"
archive_url="${PEEKOO_PYTHON_STANDALONE_URL:-https://github.com/indygreg/python-build-standalone/releases/download/${release_tag}/cpython-3.12.9+${release_tag}-${target_triple}-install_only.tar.gz}"

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT

archive_path="$workdir/python-runtime.tar.gz"
curl -fL "$archive_url" -o "$archive_path"
tar -xzf "$archive_path" -C "$workdir"

if [[ -x "$workdir/python/bin/python3" ]]; then
  py_bin="$workdir/python/bin/python3"
elif [[ -x "$workdir/python/bin/python" ]]; then
  py_bin="$workdir/python/bin/python"
else
  echo "python binary not found in extracted runtime" >&2
  exit 1
fi

"$py_bin" -m ensurepip --upgrade || true
"$py_bin" -m pip install --upgrade pip setuptools wheel
"$py_bin" -m pip install --no-cache-dir -r "$REQ_FILE"

mkdir -p "$(dirname "$OUT_FILE")"
tar -czf "$OUT_FILE" -C "$workdir" python

echo "Python runtime package created: $OUT_FILE"
