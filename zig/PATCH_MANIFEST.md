# PATCH MANIFEST — zig/ vendored TB 0.17.9 stack

Upstream: TigerBeetle 0.17.9 (`src/`). Compiler: TB's pinned
Zig 0.14.1 (`zig/toolchain/download.sh`, ZIG_RELEASE="0.14.1").

## Fact-check table (upstream behavior vs. vendored code)

| Claim | Verdict | Evidence (real path, checked) |
|---|---|---|
| TB does DIRECT BLOCK-DEVICE writes, NO filesystem | **CORRECTED** | TB opens a *data file* through the filesystem: `open_data_file` uses `openat(2)`, `flock(2)`, and fsyncs the parent directory (`src/io/darwin.zig:1001`, `src/io/linux.zig:1419`). "Direct" = page-cache bypass, per-OS: linux `O_DIRECT` (`src/io/linux.zig:1472`); darwin has NO O_DIRECT — TB "assumes Direct I/O is always supported" via `F_NOCACHE` + `O_DSYNC`, and fsyncs with `F_FULLFSYNC` because darwin fsync does not flush past the disk cache (`src/io/darwin.zig:1048-1053`, `:1092-1099`). Writes CAN target a block device (O_EXCL TODO comment), but TB's IO is not raw-block-only and never touches a filesystem-free path on darwin. |
| IO subsystem in per-OS files `src/io/{linux,darwin,windows}.zig` + top-level `src/{linux,darwin,windows}.zig` | **CORRECTED** (second half) | `src/io/{linux,darwin,windows,common}.zig` exist and are the per-OS IO (VERIFIED). There are NO top-level `src/{linux,darwin,windows}.zig`; dispatch lives in `src/io.zig:5-17` (comptime switch on target). |
| TB vendor/toolchain tree carries the Zig stdlib at `zig/zig/lib/std/` | VERIFIED | `zig/zig/lib/std/` present; compiler binary `zig/zig/zig` (Mach-O arm64). |
| `superblock_copies = 4`, 4/6/8 configurable | VERIFIED | `src/vsr/superblock.zig:595` compile switch `{4,6,8}`; default 4 via config. |
| write quorum 3-of-4, read quorum 2-of-4 | VERIFIED | `src/vsr/superblock_quorums.zig` `Threshold.count()`: verify=3, open=2 for 4 copies ("write quorum plus read quorum must be exactly copies + 1"). |
| copies at different byte offsets in the SAME single data file, same node | VERIFIED | `superblock_copy_size * copy_index` offsets (`superblock.zig:602-611`); one `data_file_size_min` file per replica. |
| header: checksum u128, sequence, cluster, parent hash-chain, vsr_state, view_headers_all | VERIFIED | `SuperBlockHeader` fields at `superblock.zig:54-107`; `parent` = "checksum of the previous superblock to hash chain across sequence numbers". |
| set_checksum asserts flags==0 | VERIFIED | `superblock.zig:426`. |
| Grid = 512KiB blocks, CoW, hash-chained, LSM + FreeSet | VERIFIED | `block_size = 512 KiB` default (`config.zig:161`); grid blocks addressed by u64 (`grid.zig:27`), cache hash (`grid.zig:139`); LSM tables immutable → blocks written once (copy-on-write) (`lsm/tree.zig:70-140`); FreeSet tracks free grid blocks (`vsr/free_set.zig`). |
| WAL = 2 ring buffers (headers + prepares) | VERIFIED | `vsr.Zone.wal_headers` / `wal_prepares` (`vsr.zig:125-141`); journal headers "circular buffer" (`vsr/journal.zig:17-24`). |
| `src/aof.zig` = optional append-only hash-chained DR log | VERIFIED | `aof.zig` (`magic_number`, hash-chained entries, "Reconstruct a cluster from one or more AOF files"). |
| TB pins Zig 0.14.1 (0.16.0 rejected); already downloaded | VERIFIED | `zig/download.sh:5` `ZIG_RELEASE="0.14.1"`; binary present. |

## Vendored verbatim (14 files + stdx/)

`io.zig`, `io/linux.zig`, `io/darwin.zig`, `io/windows.zig`, `io/common.zig`,
`queue.zig`, `time.zig`, `trace.zig`, `trace/{event,statsd,event_metric,…}.zig`,
`testing/time.zig`, `testing/exhaustigen.zig`, `stdx/` (whole directory),
`vsr/checksum.zig`, `vsr/superblock_quorums.zig`, `toolchain/download.sh` —
byte-identical to upstream `src/*` except where listed below.

## Patched files (changed lines)

### zig/constants.zig (PATCHED, reduced)
- Removed: TB's config/`Config.Cluster` plumbing and every constant not in the
  vendored closure (message sizing, journal ring buffers, client replies,
  LSM/grid sizing, lsm_compaction_ops, view_change_headers_suffix_max…).
- Kept verbatim: `sector_size = 4096`.
- Changed: `block_size` 512 KiB → **4096** (checkpoint zone block size).
- Kept: `superblock_copies` (4; compile switch for 4/6/8 preserved in
  superblock.zig).
- New: `view_headers_max = 2` (TB default 7), `members_max = 12` (= TB's
  replicas_max 6 + standbys_max 6), `verify = true` (TB config default),
  `membership_checkpoint_interval_default = 4`.

### zig/vsr/superblock.zig (PATCHED)
- SuperBlockVersion: fixed at 2 (TB derives it from
  `constants.config.process.release`; config module not vendored).
- `VSRState` slimmed: keeps `replica_id`, `members` (TB `vsr.Members` layout),
  `commit_max`, `log_view`, `view`, `replica_count`; drops the 2048-byte
  `CheckpointState` (manifest/FreeSet/client-sessions references) — size 240.
  `monotonic()` patched to the same view/commit_max/log_view rules over the
  slimmed struct.
- New fields: `uvrr_incarnation: u64` (reincarnation identity, checksummed)
  and `uvrr_flushed: u8` (+ explicit `uvrr_flushed_padding [3]u8`) — the
  reincarnation `flushed`/`unflushed` state as declared fields
  (`flags` stays zero as TB asserts).
- `calculate_checksum`: patched to mask `uvrr_flushed` before hashing, so all
  copies of one sequence share one checksum (the flushed mark is per-copy
  durability state like `copy`; unmasked, an unflushed copy would read as a
  fork in TB's quorum logic).
- `set_checksum`: drops `assert(copy == 0)` (TB stamps `copy` at write time
  in its async path; our sync store computes one checksum per copyset);
  keeps every other assert verbatim (flags==0, version, reserved zeroed…).
- `equal`: patched to also mask `uvrr_flushed` (same reason).
- Layout: `reserved` size computed for our prefix (view_headers_all stays at
  offset 4096 = sector_size; header = 8192 = 2 sectors); copy padding
  removed (no view-change traffic); `data_file_size_min` = superblock zone +
  membership WAL zone + one 4 KiB checkpoint block.
- Removed: the async `SuperBlockType(Storage)` state machine, manifest /
  client-sessions / FreeSet trailer references, grid references, TB WAL ring
  buffer references — replaced by the synchronous driver (`uvrr/store.zig`).
- Kept verbatim: checksum scheme (AEGIS-128L via vendored `vsr/checksum.zig`),
  `copy_offset = superblock_copy_size * copy_index`, the Quorums import, the
  4/6/8 compile switch, the sequence-progression table.

### zig/trace/event.zig (PATCHED)
- `Operation`/`TreeEnum`/`GrooveEnum` comptime builders emptied (they enumerated
  TB's `vsr.Operation` / state-machine tree_ids; not vendored). One placeholder
  field each so statsd name-casting still compiles.
- `vsr.Peer` → local two-value enum; `CommitStage` → u8; grid/LSM cardinality
  constants → placeholders.

### zig/trace.zig, zig/queue.zig, zig/time.zig (PATCHED, imports only)
- Removed `test` blocks importing `testing/fixtures.zig` / `testing/fuzz.zig`
  (not vendored; would pull storage.zig, vsr/replica.zig, tigerbeetle.zig).

## New files (ours)

- `vsr.zig` — shim (checksum re-export, `Header`/`Header.Prepare` extern
  layouts verbatim to TB's `message_header.zig` field order, `Release`,
  `Members`, `root_members`, `member_index`, `Zone` for the three-zone file).
- `stdx.zig` — module shim to the vendored `stdx/` directory.
- `uvrr/membership.zig` — ops WAL (`add_one/remove_one/double/halve`, fixed
  4096-byte extern slots, magic 0x…01, checksum hash-chain via `parent`),
  ONE 4 KiB checkpoint block (compile-time size assert; entries
  {identity u128, weight u16, learner u8}; capacity 12 members), checkpoint
  before WAL wrap or every N ops (N = host param), replay on open.
- `uvrr/store.zig` — sync driver: format / open (2-of-4 quorum + higher-
  identity-wins + repair), clean shutdown (flushed mark on all four copies
  through the IO layer's F_FULLFSYNC/O_DSYNC path), dirty restart (bump
  incarnation + sequence, clear flushed, rewrite all four), membership ops,
  checkpoint, WAL replay.
- `uvrr/capi.zig` — `pub export fn … callconv(.C)` surface.
- `root.zig` — build root. `zig/README.md`.

## Data-file layout (tiny)

| Zone | Offset | Size |
|---|---|---|
| superblock copy 0..3 | 0 | 4 × 8192 |
| membership ops WAL (8 slots) | 32768 | 32768 |
| checkpoint (one 4 KiB block) | 65536 | 4096 |

Total 69632 bytes (TB `data_file_size_min` shape, tiny zones).
