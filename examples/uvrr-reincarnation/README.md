# uvrr-reincarnation — TigerBeetle superblock durability demo

Demo of the Director's durability delta over TigerBeetle 0.17.9's actual
superblock code: four superblock copies with a `flushed`/`unflushed` bit,
any-unflushed ⇒ dirty restart, incarnation bump with a new identity written to
all four copies (higher identity wins on reads), clean shutdown marked flushed
with a real fsync, and the continuation commitment on the clean path.

## What is extracted vs. re-glued

- **Extracted (TB's own code, verbatim):** `SuperBlockHeader` layout, the
  checksum scheme (`vsr.checksum` over the header minus the leading
  checksum/copy fields), all derived constants, the copy layout
  (`superblock_copy_size * copy_index`), and the open quorum (2 of 4) from
  `src/vsr/superblock.zig` — compiled into a C-ABI static library
  (`libuvrr_sb.a`) with TigerBeetle's **pinned Zig 0.14.1** (their
  `zig/download.sh`).
- **Re-glued:** TB's `SuperBlockType(Storage)` is an async callback state
  machine over their `Storage`/`IO` interfaces; `uvrr_sb.zig` performs the
  same flow synchronously over one file descriptor because the consumer is a
  synchronous C API.
- **Delta (the only behavioral change):** the `flushed` bit lives in TB's
  reserved `flags` field; TB's `set_checksum` asserts `flags == 0`, so the
  wrapper assigns via `calculate_checksum()` directly.

## Build + run

`build.sh` (idempotent):

1. Clones TigerBeetle at tag `0.17.9` (shallow) into `.tmp/tb-0.17.9/` if
   absent.
2. Fetches their pinned Zig 0.14.1 via their own `zig/download.sh` if absent.
3. Copies `uvrr_sb.zig` into their `src/` and compiles
   `libuvrr_sb.a` into `.tmp/uvrr-reincarnation-build/`.
4. Runs the demo with `RUSTFLAGS="-L native=..."` so the static lib links.

```sh
examples/uvrr-reincarnation/build.sh
```

The demo writes its datastore under `std::env::temp_dir()`
(`uvrr-reincarnation-demo/superblock.bin`), never in the repo. It exercises:
format → clean restart (continuation commitment) → unflushed in-flight write →
clean shutdown (fsync + flushed mark) → crash with unflushed state → dirty
restart → incarnation bump writing the new identity + flushed to all four
copies → torn-copy restart proving higher-identity-wins. All Zig log lines are
emitted to stderr; the demo reports each step on stdout.

## C ABI (libuvrr_sb.a)

- `uvrr_sb_format(path, cluster, out_handle)` — format four copies
  (sequence 1, flushed).
- `uvrr_sb_open(path, out_handle, out_state)` — read all four, verify
  (version/checksum/copy-tag), adopt the highest sequence a 2-of-4 quorum
  agrees on, report `dirty` if any valid copy is unflushed.
- `uvrr_sb_bump(handle, path)` — sequence + 1, `parent` = old checksum,
  write new identity + flushed to all four, fsync.
- `uvrr_sb_set_flushed(handle, path, flushed)` — set/clear the bit, rewrite
  all four, fsync.
- `uvrr_sb_copy_size()`, `uvrr_sb_close(handle)`.

TigerBeetle is Apache-2.0; its source is fetched at build time, not vendored.
