#!/bin/zsh
# Build the vendored UVRR static library only (libuvrr_sb.a) with TB's pinned
# Zig 0.14.1. The demo driver is examples/uvrr-reincarnation/build.sh.
set -euo pipefail
cd "$(dirname "$0")/.."   # repo root

SRC="${TB_SRC:-.tmp/tb-0.17.9}"
OUT="${TB_OUT:-.tmp/uvrr-reincarnation-build}"

if [[ ! -x "$SRC/zig/zig/zig" ]]; then
  (cd "$SRC/zig" && bash download.sh)
fi

mkdir -p "$OUT"
"$SRC/zig/zig/zig" build-lib -lc -O ReleaseSafe \
  --dep stdx -Mroot=zig/root.zig \
  -Mstdx=zig/stdx/stdx.zig \
  --name uvrr_sb \
  -femit-bin="$OUT/libuvrr_sb.a"
