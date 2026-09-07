# uvrr-reincarnation — vendored TigerBeetle storage-stack demo

Drives the vendored TigerBeetle 0.17.9 IO + superblock stack (`zig/` — see
`zig/PATCH_MANIFEST.md` for every patched line) through a C-ABI static library
(`libuvrr_sb.a`), built with TB's pinned **Zig 0.14.1**.

## Paths demonstrated

1. **Clean shutdown** — flushed mark written to all four superblock copies
   through TB's IO layer (darwin: `O_DSYNC` + `F_NOCACHE` + `F_FULLFSYNC`
   flush; linux: `O_DIRECT`), sequence hash-chain advanced, 3-of-4 verify
   quorum.
2. **Dirty restart** — a crash that left a copy recorded unflushed (any valid
   unflushed copy ⇒ dirty); the incarnation is bumped, all four copies are
   rewritten, and the higher identity wins on subsequent reads.
3. **Membership replay** — voting-weights ops (`add_one`, `remove_one`,
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
