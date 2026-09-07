#!/bin/zsh
# Build the TigerBeetle-extracted superblock static library and run the demo.
#
# Uses TigerBeetle 0.17.9's own source tree and their pinned Zig 0.14.1
# (fetched by their zig/download.sh). The wrapper source (uvrr_sb.zig) is the
# only file added to their tree; it imports their superblock.zig verbatim.
set -euo pipefail
cd "$(dirname "$0")/../.."   # repo root

SRC="${TB_SRC:-.tmp/tb-0.17.9}"
OUT="${TB_OUT:-.tmp/uvrr-reincarnation-build}"

if [[ ! -d "$SRC/src" ]]; then
  git clone --depth 1 --branch 0.17.9 --filter=blob:none \
    https://github.com/tigerbeetle/tigerbeetle.git "$SRC"
fi
if [[ ! -x "$SRC/zig/zig/zig" ]]; then
  (cd "$SRC/zig" && bash download.sh)
fi

cp examples/uvrr-reincarnation/uvrr_sb.zig "$SRC/src/uvrr_sb.zig"
mkdir -p "$OUT"
"$SRC/zig/zig/zig" build-lib -lc -O ReleaseSafe \
  --dep stdx -Mroot="$SRC/src/uvrr_sb.zig" \
  -Mstdx="$SRC/src/stdx/stdx.zig" \
  -femit-bin="$OUT/libuvrr_sb.a"

RUSTFLAGS="-L native=$OUT" cargo run --example uvrr-reincarnation --features uvrr
