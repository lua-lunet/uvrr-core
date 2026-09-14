# uvrr-reincarnation — vendored TigerBeetle storage-stack demo

Drives the vendored TigerBeetle 0.17.9 IO + superblock stack (`zig/` — see
`zig/PATCH_MANIFEST.md` for every patched line) through a C-ABI static library
(`libuvrr_sb.a`), built with TB's pinned **Zig 0.14.1**.

## Paths demonstrated

The marker transition machine (§5.1 of `docs/vrr-durability-model.md`; the
Rust twin `src/replica/reincarnation.rs`), over TB's actual code paths
(darwin: `O_DSYNC` + `F_NOCACHE` + `F_FULLFSYNC` flush; linux: `O_DIRECT`):

1. **T1 — the stop** — `uvrr_begin_stop` writes `stopping` 4x; the host
   drain (flush WALs and grids) sits strictly between; `uvrr_finish_stop`
   writes `stopped` 4x. The marker order IS the drain's proof: a `stopped`
   copy vouches for the WAL under it. Each marker write is a new
   parent-chained copyset.
2. **T2 — the clean stop boots** — `uvrr_open` reads the working quorum at
   the 2-of-4 open threshold; 2-of-4 `stopped` proves the controlled
   shutdown, so the node continues under the same identity and writes
   `restarting` 4x (the boot's uniform write is also the repair).
3. **T3 — the reincarnation** — a boot reading no stopped quorum (a crash
   that left the running state's `restarting` markers, or death mid-join
   with `joining` markers) bumps the identity and writes `joining` 4x; the
   identity only moves forward, and no `started` state is ever written.
4. **Membership replay** — voting-weights ops (`add_one`, `remove_one`,
   `double`, `halve`) appended to the ops WAL (checksum hash-chain,
   direct-IO writes), checkpointed into one 4 KiB block before the WAL wraps
   or every N ops, and replayed (checkpoint + WAL suffix) on reopen.

## Build

`./build.sh` — compiles `zig/root.zig` with TB's pinned Zig 0.14.1 into
`libuvrr_sb.a`, then runs the demo with `RUSTFLAGS="-L native=…"`. No new Rust
dependencies; the feature gate `uvrr` is unchanged
(`cargo run --example uvrr-reincarnation --features uvrr`).

## Darwin note (fact-checked)

TB's darwin IO has **no `O_DIRECT`**: it uses `O_DSYNC` + `F_NOCACHE` (page
cache bypass) and `F_FULLFSYNC` for a real cache-flushing sync
(`src/io/darwin.zig` — vendored verbatim at `zig/io/darwin.zig`). TB's linux
IO uses true `O_DIRECT` (`zig/io/linux.zig`). TB's darwin path never touches
a raw block device — it is a data file opened through the filesystem with the
page cache disabled and full fsync. The demo therefore runs the real TB
darwin IO semantics on this machine; linux semantics (`O_DIRECT`) come from
the same vendored code on linux.

## History

The pre-replacement filesystem-based scaffolding version of this example
lives at tag `rust_disk`; it was replaced because filesystem I/O does not
match the TigerBeetle storage model this example vendors.
