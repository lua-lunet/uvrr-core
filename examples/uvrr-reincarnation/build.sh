#!/bin/zsh
# Build the VENDORED TigerBeetle IO + superblock static library (zig/) and run
# the demo. Uses TB's pinned Zig 0.14.1 (their zig/download.sh, vendored at
# zig/toolchain/download.sh). The vendored tree is patched minimally — every
# changed line is listed in zig/PATCH_MANIFEST.md.
set -euo pipefail
cd "$(dirname "$0")/../.."   # repo root

SRC="${TB_SRC:-.tmp/tb-0.17.9}"
OUT="${TB_OUT:-.tmp/uvrr-reincarnation-build}"

# TB's pinned Zig 0.14.1 toolchain (source tree already present under $SRC).
if [[ ! -d "$SRC/src" ]]; then
  git clone --depth 1 --branch 0.17.9 --filter=blob:none \
    https://github.com/tigerbeetle/tigerbeetle.git "$SRC"
fi
if [[ ! -x "$SRC/zig/zig/zig" ]]; then
  (cd "$SRC/zig" && bash download.sh)
fi

mkdir -p "$OUT"
"$SRC/zig/zig/zig" build-lib -lc -O ReleaseSafe \
  --dep stdx -Mroot=zig/root.zig \
  -Mstdx=zig/stdx/stdx.zig \
  --name uvrr_sb \
  -femit-bin="$OUT/libuvrr_sb.a"

# Ensure cargo re-links when only the Zig static lib changed (cargo does not
# fingerprint external .a inputs):
touch examples/uvrr-reincarnation/main.rs
RUSTFLAGS="-L native=$OUT" cargo run --example uvrr-reincarnation --features uvrr
