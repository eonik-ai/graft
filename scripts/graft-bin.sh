#!/bin/sh
# SPDX-License-Identifier: Apache-2.0
# Print the graft binary cargo just built (respects CARGO_TARGET_DIR).
set -eu
cd "$(dirname "$0")/.."
if [ -n "${GRAFT:-}" ] && [ -x "$GRAFT" ]; then
  printf '%s\n' "$GRAFT"
  exit 0
fi
target="$(cargo metadata --format-version 1 --no-deps --offline --locked \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
for bin in "$target/release/graft" "$PWD/target/release/graft" "$PWD/target/debug/graft"; do
  if [ -x "$bin" ]; then
    printf '%s\n' "$bin"
    exit 0
  fi
done
echo "build graft first (cargo build --release --bin graft)" >&2
exit 1
