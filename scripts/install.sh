#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Install the graft CLI from GitHub Releases, or fall back to cargo --git.
#
#   curl -fsSL https://raw.githubusercontent.com/eonik-ai/graft/main/scripts/install.sh | sh
#
# Override: GRAFT_VERSION=v0.2.2 PREFIX=$HOME/.local
set -eu

REPO="${GRAFT_REPO:-eonik-ai/graft}"
PREFIX="${PREFIX:-/usr/local}"
VERSION="${GRAFT_VERSION:-latest}"

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "need $1 on PATH" >&2
    exit 1
  fi
}

is_musl() {
  if [ -f /etc/alpine-release ]; then
    return 0
  fi
  if ldd /bin/sh 2>/dev/null | grep -q musl; then
    return 0
  fi
  return 1
}

target() {
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Darwin)
      case "$arch" in
        arm64) echo "aarch64-apple-darwin" ;;
        x86_64) echo "x86_64-apple-darwin" ;;
        *) echo "unsupported macOS arch: $arch" >&2; exit 1 ;;
      esac
      ;;
    Linux)
      case "$arch" in
        x86_64)
          if is_musl; then
            echo "x86_64-unknown-linux-musl"
          else
            echo "x86_64-unknown-linux-gnu"
          fi
          ;;
        aarch64|arm64)
          if is_musl; then
            echo "musl-aarch64"
          else
            echo "aarch64-unknown-linux-gnu"
          fi
          ;;
        *) echo "unsupported Linux arch: $arch" >&2; exit 1 ;;
      esac
      ;;
    *)
      echo "unsupported OS: $os (use cargo install --git https://github.com/${REPO}.git --locked --bin graft)" >&2
      exit 1
      ;;
  esac
}

install_from_cargo() {
  need cargo
  echo "no GitHub Release binary for this platform/tag; installing from git with cargo" >&2
  cargo install --git "https://github.com/${REPO}.git" --locked --bin graft
}

TARGET="$(target)"
if [ "$TARGET" = "musl-aarch64" ]; then
  echo "no musl aarch64 GitHub binary yet; installing from git with cargo" >&2
  install_from_cargo
  exit 0
fi
need curl
need tar
need mktemp

TMP="$(mktemp -d "${TMPDIR:-/tmp}/graft-install.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

if [ "$VERSION" = "latest" ]; then
  URL="https://github.com/${REPO}/releases/latest/download/graft-${TARGET}.tar.gz"
else
  URL="https://github.com/${REPO}/releases/download/${VERSION}/graft-${TARGET}.tar.gz"
fi

if ! curl -fsSL "$URL" -o "$TMP/graft.tar.gz"; then
  install_from_cargo
  exit 0
fi

tar -xzf "$TMP/graft.tar.gz" -C "$TMP"
BIN=""
for candidate in "$TMP/graft" "$TMP/graft-${TARGET}" "$TMP/bin/graft"; do
  if [ -f "$candidate" ]; then
    BIN="$candidate"
    break
  fi
done
if [ -z "$BIN" ]; then
  echo "archive did not contain graft" >&2
  exit 1
fi
chmod +x "$BIN"

DEST="${PREFIX}/bin"
if [ ! -w "$DEST" ] 2>/dev/null; then
  DEST="${HOME}/.local/bin"
  mkdir -p "$DEST"
fi
mkdir -p "$DEST"
cp "$BIN" "$DEST/graft"
echo "installed $DEST/graft"
"$DEST/graft" --version || true
echo "runtime: ffmpeg and ffprobe on PATH"
