# zig/ — vendored TigerBeetle 0.17.9 storage stack (item31)

The Director rejected the filesystem-based scaffolding demo (tag `rust_disk`).
This tree vendors TigerBeetle 0.17.9's actual storage stack and drives it
through a C-ABI static library; `examples/uvrr-reincarnation/` is the consumer.

## Vendored (verbatim, from `.tmp/tb-0.17.9/src/`)

- `io.zig`, `io/{linux,darwin,windows,common}.zig` — the direct-IO subsystem
- `queue.zig`, `time.zig`, `trace.zig`, `trace/*.zig`, `stdx/` (support closure)
- `testing/{time,exhaustigen}.zig` (imported by the above)
- `vsr/checksum.zig` (AEGIS-128L checksum), `vsr/superblock_quorums.zig`
  (3-of-4 write / 2-of-4 read quorum logic, verbatim)
- `toolchain/download.sh` — TB's pinned Zig 0.14.1 fetcher

## Patched (every changed line in PATCH_MANIFEST.md)

- `constants.zig` — reduced to the tiny data file (sector 4096 kept; block
  size 4 KiB for the checkpoint zone; no TB WAL ring buffers, no 512 KiB LSM
  grid, no client replies)
- `vsr/superblock.zig` — slimmed VSRState, `uvrr_incarnation` +
  `uvrr_flushed` fields, checksum masked over the flushed bit, no async
  SuperBlockType (sync driver in `uvrr/store.zig`)
- `trace/event.zig`, `trace.zig`, `queue.zig`, `time.zig` — dropped imports
  of the state machine / replica / fuzz closure that this tree doesn't vendor

## New (ours, TB idiom)

- `vsr.zig` — minimal `vsr` shim (checksum re-export, `Header` extern layout,
  `Release`, `Members`, `Zone` sized for the three-zone data file). NOT TB's
  vsr.zig.
- `stdx.zig` — named-module shim (`@import("stdx")` → vendored stdx dir)
- `uvrr/membership.zig` — ops WAL (add_one/remove_one/double/halve), one
  4 KiB checkpoint block, replay; modeled on TB's `src/aof.zig`
- `uvrr/store.zig` — synchronous driver over the vendored IO + quorums
- `uvrr/capi.zig` — `pub export` C-ABI (`callconv(.C)`)
- `root.zig` — compilation root

## Build

`examples/uvrr-reincarnation/build.sh` compiles `zig/root.zig` with TB's
pinned Zig 0.14.1 into `libuvrr_sb.a` (or run `zig/build.sh` for the lib only).
