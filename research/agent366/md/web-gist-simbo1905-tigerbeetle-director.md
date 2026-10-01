<!-- source: https://gist.github.com/simbo1905/3818c439bb08774f87bc8a92cc0a6ad5 -->

Instantly share code, notes, and snippets.

![@simbo1905](https://avatars.githubusercontent.com/u/322608?s=64&v=4)

# [simbo1905](/simbo1905)/**[README.md](/simbo1905/3818c439bb08774f87bc8a92cc0a6ad5)**

# Select an option

## No results found

# Select an option

## No results found

# TigerBeetle 0.17.9 — VSR/VRR Protocol & Disk Analysis

A code-level reverse engineering of TigerBeetle release **0.17.9** (published
2026-07-06), done against the actual release source. All `file:line` citations
are from the 0.17.9 tree. Sections that quote the academic literature
(VSR-1988 Oki & Liskov; VRR-2012 Liskov & Cowling, MIT-CSAIL-TR-2012-021;
DISC'17 extended TR UW-CSE-17-08-01) carry their own attribution inline.

`file:line`

## What the code is

TigerBeetle 0.17.9 runs the VRR-2012 message family (its own docs map
`JoinView`/`View`/`ExitView` to `DoViewChange`/`StartView`/`StartViewChange`)
with a durable-at-prepare placement: every replica foreground-journals every
prepare with write-completion-equals-durability (O\_DIRECT|O\_DSYNC on Linux,
F\_FULLFSYNC on macOS) *before* sending `prepare_ok`; commit is flush-free and
derived; views are durably fenced in a quorum-verified multi-copy superblock
before advertisement. There is no recovery/recovery-response message in the
wire set (`message_header.zig:174`); a nonzero `epoch` field is rejected at
dispatch.

`JoinView`
`View`
`ExitView`
`DoViewChange`
`StartView`
`StartViewChange`
`prepare_ok`
`message_header.zig:174`
`epoch`

## Files

| File | What it is |
| --- | --- |
| `tigerbeetle-0.17.9-vsr-vrr-protocol-and-disk-analysis.jsonl` | Raw analysis transcript (Chat-JSONL): task prompt, assistant reasoning, and the full final report; tool-call rows filtered out. |
| `write-methods.md` | The three write methods: async (client replies — nothing waits, deadline = checkpoint), forced (WAL — the protocol waits, completion == durability), write-through (grid + superblock — the checkpoint barrier waits). Same direct-IO machinery; three answers to "who may continue before this write completes". |
| `summary-flush-model.md` | The whole flush model in one page: three triggers — force the WAL per prepare, checkpoint + superblock seal every 960 ops, durable view fence before advertisement — with one sequence diagram and the volatile-state list. Start here. |
| `report-0.17.9-protocol.md` | The protocol as built: message set, normal-case flow (sequence diagrams), star fanout, flush-free commit, view change, WAL format with the 16-case recovery table, the complete flush catalog, quorums, external-literature context. |
| `report-0.17.9-recovery.md` | Recovery as built: the durability chain, restart and superblock open, journal recovery decision table, tear detection, the two-write prepare path, scenario-by-scenario behavior, what stays volatile and why that is safe, RTO/RPO shape. |
| `tigerbeetle-0.17.9-io-subsystem.md` | The IO subsystem as built: the direct-IO axiom (O\_DIRECT |
| `report-0.17.9-io-and-ondisk-factcheck.md` | The on-disk subsystem: data-file zones, superblock geometry and quorums, grid read/repair and free-set release, journal slot math, direct-IO discipline per OS — plus `## Appendix: AOF (optional feature)` with the AOF's own flow, mechanics table, and citations. |

`tigerbeetle-0.17.9-vsr-vrr-protocol-and-disk-analysis.jsonl`
`write-methods.md`
`summary-flush-model.md`
`report-0.17.9-protocol.md`
`report-0.17.9-recovery.md`
`tigerbeetle-0.17.9-io-subsystem.md`
`report-0.17.9-io-and-ondisk-factcheck.md`
`## Appendix: AOF (optional feature)`

Each report embeds mermaid sequence diagrams (autonumbered), call hierarchies
with `file:line` for every step, mechanics tables (flush point, trigger,
batch cardinality, tear detection), and a constants table with the defining
citation for every number.

`file:line`

## Key numbers

`vsr_checkpoint_ops`
`constants.zig:40-42`

# Appendix: AOF — the optional write-behind log

TigerBeetle 0.17.9 ships an optional per-replica **append-only file (AOF)**: a hash-chained log
of committed prepares, written synchronously on the commit path but never force-flushed there.
It is opt-in, off by default, and is not part of the consensus durability argument — "this is
not a WAL" (`src/vsr/replica.zig:5409`).

`src/vsr/replica.zig:5409`

## Enable / configure

`--aof-file=<path>.aof`
`--aof`
`<data file>.aof`
`src/tigerbeetle/cli.zig:105,109`
`cli.zig:1055-1072`
`--aof-recovery`
`src/tigerbeetle/cli.zig:113`
`src/vsr/client.zig:66,294`
`src/tigerbeetle/main.zig:454-456`

## Write path (verified against 0.17.9)

`execute_op`
`src/vsr/replica.zig:5417-5425`
`0xbcd8d3fee406119ed192c4f4c4fc82`
`src/aof.zig:28`
`src/aof.zig:36`
`src/aof.zig:81-87`
`AOF.write`
`IO.aof_blocking_write_all`
`std.fs.File.writeAll`
`src/aof.zig:185`
`src/io/common.zig:123-126`
`O_DIRECT`
`O_SYNC`
`src/aof.zig:169-171`
`assert(self.state.writing.unflushed < constants.journal_slot_count)`
`src/aof.zig:174`
`src/aof.zig:130-132`
`journal_slot_count = 1024`
`src/config.zig:158`

## Durable flush (checkpoint only, never per message)

`src/vsr/replica.zig:5031-5036`
`AOF.checkpoint`
`src/aof.zig:197-220`
`io.fsync`
`src/aof.zig:213-219`
`on_fsync`
`unflushed = 0`
`src/aof.zig:222-229`
`src/aof.zig:235-262`
`src/vsr/replica.zig:10485`
`AOF.sync`
`src/aof.zig:191-195`
`AOF.init`
`src/aof.zig:148-160`
`IO.aof_blocking_open`
`file.sync()`
`src/io/common.zig:147-166`

## Relationship to the direct-IO path

`src/io/linux.zig`
`pwrite`
`F_FULLFSYNC`
`src/io/darwin.zig:829,1092-1098`
`src/vsr/replica.zig:5402-5404`
`src/aof.zig:36`
`src/io/common.zig`

## Recovery / replay tooling (offline CLI)

`CLIArgs` and help at `src/aof.zig:875-931`; dispatch at `src/aof.zig:942-1003`:

`CLIArgs`
`src/aof.zig:875-931`
`src/aof.zig:942-1003`
`aof recover --cluster=<id> --addresses=<addr> <path>`
`--aof-recovery`
`src/aof.zig:956-966`
`aof debug <path>`
`src/aof.zig:968-990`
`aof merge -- path.aof ...`
`src/aof.zig:996-1003`
`prepared.aof`
`src/aof.zig:1002`
`merged.aof`
`AOF.Iterator.next`
`error.AOFChecksumChainMismatch`
`src/aof.zig:512,544-547`

## Adaptation for a diskless system

A diskless system has no WAL to borrow durability from, so the same shape becomes an **async
write-behind telemetry log** with a weaker, documented contract:

# TigerBeetle 0.17.9: IO and on-disk architecture

## The data file: one file per replica, six fixed zones

**One data file per replica; every byte of it is addressed by fixed zones; all consensus-critical
IO is direct, synchronous, checksummed, and sector-aligned.** The `Zone` enum fixes the layout:
`superblock, wal_headers, wal_prepares, client_replies, grid_padding, grid`
(`src/vsr/vsr.zig:125-135`); `Zone.start` accumulates zone sizes in enum order (`vsr.zig:179-186`),
and `grid_padding` exists purely to make `Zone.start(.grid)` block-aligned
(`vsr.zig:141-152`, asserted at `vsr.zig:168`).

`Zone`
`superblock, wal_headers, wal_prepares, client_replies, grid_padding, grid`
`src/vsr/vsr.zig:125-135`
`Zone.start`
`vsr.zig:179-186`
`grid_padding`
`Zone.start(.grid)`
`vsr.zig:141-152`
`vsr.zig:168`

## Flow: Superblock write path (4 copies, write quorum 3) and open path (read quorum 2)

Trigger write: checkpoint, view change, or format on a replica. Trigger open: replica startup.
Participants: `Replica`, `SuperBlock` + `Quorums`, `Storage`, disk.

`Replica`
`SuperBlock`
`Quorums`
`Storage`

Geometry (production config): `@sizeOf(SuperBlockHeader)` = 8 KiB (fields before
`view_headers_all` fill one 4 KiB sector — `src/vsr/superblock.zig:105`; plus
`view_headers_all: [11]Header.Prepare` = 11·256 B — `superblock.zig:97`,
`view_headers_max = view_change_headers_suffix_max + 2 = 11` — `src/constants.zig:304`,
`src/config.zig:156`; plus `view_headers_reserved_size` = 4096 − (11·256 % 4096) = 1280 B —
`superblock.zig:49-50`; size asserted a sector multiple and ≥ 2 sectors at `superblock.zig:101-102`)

`@sizeOf(SuperBlockHeader)`
`view_headers_all`
`src/vsr/superblock.zig:105`
`view_headers_all: [11]Header.Prepare`
`superblock.zig:97`
`view_headers_max = view_change_headers_suffix_max + 2 = 11`
`src/constants.zig:304`
`src/config.zig:156`
`view_headers_reserved_size`
`superblock.zig:49-50`
`superblock.zig:101-102`
`superblock_copy_padding`
`superblock.zig:607-610`
`clients_max = 64`
`src/config.zig:228`
`pipeline_prepare_queue_max = 8`
`src/config.zig:155`
`@sizeOf(Header) == 256`
`src/vsr/message_header.zig:72`
`superblock_copy_size`
`superblock.zig:613`
`superblock_zone_size`
`superblock.zig:602`
`Zone.start(.superblock) == 0`
`src/vsr/vsr.zig:125-135,179-186`

### Call hierarchy

Write (checkpoint / view\_change / format — all funnel through one serializer):

`Replica.commit_checkpoint_superblock`
`src/vsr/replica.zig:5110`
`SuperBlock.checkpoint`
`src/vsr/superblock.zig:891`
`:1010`
`:761`
`SuperBlock.acquire`
`superblock.zig:1431`
`SuperBlock.write_staging`
`superblock.zig:1088`
`staging.sequence = working.sequence + 1`
`staging.parent = working.checksum`
`superblock.zig:1097-1099`
`staging.set_checksum()`
`superblock.zig:1120`
`SuperBlock.write_header`
`superblock.zig:1124`
`offset = superblock_copy_size * copy`
`superblock.zig:1150-1151`
`storage.write_sectors(..., .superblock, offset)`
`superblock.zig:1166-1172`
`SuperBlock.write_header_callback`
`superblock.zig:1175`
`context.copy = copy + 1; write_header(context)`
`superblock.zig:1195-1196`
`read_working(.verify)`
`superblock.zig:1191-1193`
`read_header`
`superblock.zig:1220-1226`
`:1226`
`Quorums.working(threshold=.verify)`
`src/vsr/superblock_quorums.zig:89`
`.verify`
`superblock_quorums.zig:61-66`

Open (startup, with repair):

`SuperBlock.open`
`src/vsr/superblock.zig:854`
`acquire`
`read_working(.open)`
`superblock.zig:1460-1461`
`read_header`
`superblock.zig:1220-1226`
`Quorums.working(threshold=.open)`
`superblock_quorums.zig:89`
`.open`
`superblock_quorums.zig:69-74`
`:57`
`context.repairs = quorum.repairs()`
`superblock.zig:1397-1399`
`SuperBlock.repair`
`superblock.zig:1415`
`RepairIterator.next`
`superblock_quorums.zig:326`
`superblock_quorums.zig:344-365`
`write_header`
`superblock.zig:1424-1425`

### Sequence (write path)

### Sequence (open + repair)

### Mechanics table

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection |
| --- | --- | --- | --- | --- | --- |
| Stage new state | `src/vsr/superblock.zig:1088-1121` | `staging = working`; `sequence += 1`; `parent = working.checksum`; embed `CheckpointState` + view headers; `set_checksum()` | none | 1 staged header (8 KiB) | parent chain on next open (`superblock_quorums.zig:146-164` rejects unconnected parents) |
| Write one copy | `src/vsr/superblock.zig:1124-1173` | whole 24 KiB copy to `offset = copy * 24 KiB`, zone `.superblock`, on the O\_DSYNC data-file fd | per copy, per checkpoint/view-change/format | `superblock_copy_size` = 24 KiB (`superblock.zig:613`); `superblock_copies` = 4 (`src/config.zig:160`) | copy's own checksum; `copy` field excluded from checksum so all copies can share one checksum (`superblock.zig:57-62`) |
| Advance copies | `src/vsr/superblock.zig:1175-1198` | sequential writes; after 4th copy, read-back verify | per copy | 4 writes per update | crash mid-update leaves a mixed old/new quorum; recovered below |
| Verify read-back | `src/vsr/superblock.zig:1191-1193,1200-1226` → `superblock_quorums.zig:89-197` | re-read all copies, group into quorums by (sequence, checksum, parent), sort by priority | per superblock update (after writes) | 4 reads | `.verify` quorum = 3 (`superblock_quorums.zig:61-66`) must confirm the new state |
| Open | `src/vsr/superblock.zig:854-869,1460-1461` | read all 4 copies, `Quorums.working(.open)` picks highest-sequence connected quorum | per startup | 4 reads | `.open` quorum = 2 (`superblock_quorums.zig:69-74`) tolerates 2 copy faults — required because in-place updates transiently impair one copy (`:67-68`) |
| Repair | `src/vsr/superblock.zig:1415-1428` + `superblock_quorums.zig:317-367` | rewrite stale/missing/misdirected copies from the working quorum | per open with faults | up to 4 copy writes | `RepairIterator` priority: empty slot > duplicate-held slot > misdirected slot (`superblock_quorums.zig:344-365`) |
| CheckpointState anchor | `src/vsr/superblock.zig:320-397` | 1024-byte extern struct carrying free-set acquired/released checksums, last-block addresses and sizes (`:343-344,357-358,376-377`), initialized in `root()` (`:169-176`) | written with each copy | 1 per checkpoint | checksums over EWAH-encoded free-set block chains |

`src/vsr/superblock.zig:1088-1121`
`staging = working`
`sequence += 1`
`parent = working.checksum`
`CheckpointState`
`set_checksum()`
`superblock_quorums.zig:146-164`
`src/vsr/superblock.zig:1124-1173`
`offset = copy * 24 KiB`
`.superblock`
`superblock_copy_size`
`superblock.zig:613`
`superblock_copies`
`src/config.zig:160`
`copy`
`superblock.zig:57-62`
`src/vsr/superblock.zig:1175-1198`
`src/vsr/superblock.zig:1191-1193,1200-1226`
`superblock_quorums.zig:89-197`
`.verify`
`superblock_quorums.zig:61-66`
`src/vsr/superblock.zig:854-869,1460-1461`
`Quorums.working(.open)`
`.open`
`superblock_quorums.zig:69-74`
`:67-68`
`src/vsr/superblock.zig:1415-1428`
`superblock_quorums.zig:317-367`
`RepairIterator`
`superblock_quorums.zig:344-365`
`src/vsr/superblock.zig:320-397`
`:343-344,357-358,376-377`
`root()`
`:169-176`

### Callouts

`copy * 24 KiB`
`src/vsr/superblock.zig:1150-1151`
`:1226`
`src/vsr/vsr.zig:125-135,179-186`
`superblock_quorums.zig:61-66`
`:69-74`
`:57`
`superblock_copies`
`src/config.zig:160`
`src/constants.zig:553-559`
`src/vsr/superblock.zig:594-598`
`sequence = sequence + 1`
`parent = previous checksum`
`src/vsr/superblock.zig:1097-1099`
`Quorums.working`
`superblock_quorums.zig:141-164`
`superblock.zig:49-50,97-98,105,101-102`
`pipeline_prepare_queue_max`
`clients_max`
`:604-610`
`:613,602`
`RepairIterator`
`superblock_quorums.zig:317-367`
`:300-316`

### Test evidence

`test "SuperBlockHeader"`
`src/vsr/superblock.zig:1587`
`view_headers_all`
`test "Cluster: recovery: reformat unrecoverable replica"`
`src/vsr/replica_test.zig:329`
`...: too many faults`
`:357`
`test "FreeSet checkpoint"`
`src/vsr/free_set.zig:928`
`test "FreeSet encode, decode, encode"`
`free_set.zig:1032`
`src/vsr/superblock_quorums.zig`
`src/vsr/superblock_test.zig`
`superblock.zig`
`src/vsr/replica_test.zig`

## Flow: Journal (WAL) slot math and the two-write prepare path (O\_DSYNC placement)

Trigger: a prepare to be durably logged (primary or accepted backup). Participants: `Replica`,
`Journal`, `Storage`, disk.

`Replica`
`Journal`
`Storage`

### Call hierarchy

`Replica.write_prepare`
`src/vsr/replica.zig:11178`
`:11189-11196`
`:11198-11212`
`Journal.write_prepare`
`src/vsr/journal.zig:1789`
`journal.zig:1811-1820`
`journal_iops_write_max`
`src/config.zig:117`
`null`
`journal.zig:1824-1830`
`sector_ceil`
`journal.zig:1842`
`:1846`
`slot_with_header`
`slot_for_op = op % slot_count`
`journal.zig:396-398`
`:63-64`
`Journal.write_sectors`
`journal.zig:2015`
`lock_sectors`
`journal.zig:2036`
`storage.write_sectors(..., .wal_prepares, offset)`
`journal.zig:2069-2077`
`journal.zig:1851`
`Ring.prepares.offset(slot) = message_size_max * slot.index`
`journal.zig:38-42`
`write_prepare_header`
`journal.zig:1856`
`journal.zig:1865-1882`
`Ring.headers.offset(slot) = sector_floor(slot.index * 256)`
`journal.zig:32-37`
`journal.zig:1893-1896,2036-2067`
`storage.write_sectors(..., .headers, offset)`
`journal.zig:1908`
`write_prepare_on_write_header`
`journal.zig:1911`
`journal.zig:1915-1949`
`dirty.clear(slot)`
`faulty.clear(slot)`
`journal.zig:1953-1954`
`write_prepare_release(write, message)`
`journal.zig:1956`
`Replica.write_prepare_callback`
`src/vsr/replica.zig:11219`
`send_prepare_ok`
`replica.zig:11225`
`flush_loopback_queue`
`replica.zig:11226`

Ring geometry: `journal_size_headers` = 1024 · 256 B = 256 KiB, `journal_size_prepares` =
1024 · 1 MiB = 1 GiB, total 1 GiB + 256 KiB (`src/constants.zig:196-198`;
`journal_slot_count` = 1024 `src/config.zig:158`, `message_size_max` = 1 MiB
`src/config.zig:159`). Zone order: `superblock, wal_headers, wal_prepares, client_replies, grid_padding, grid` (`src/vsr/vsr.zig:125-135`).

`journal_size_headers`
`journal_size_prepares`
`src/constants.zig:196-198`
`journal_slot_count`
`src/config.zig:158`
`message_size_max`
`src/config.zig:159`
`superblock, wal_headers, wal_prepares, client_replies, grid_padding, grid`
`src/vsr/vsr.zig:125-135`

O\_DSYNC placement: the single data-file fd per replica is opened `RDWR | O_DSYNC`
(`src/io/linux.zig:1429-1433`, `.DSYNC = true` at `:1432`), plus `O_DIRECT` when supported
(`:1508`), with `assert(flags.DSYNC)` — "we rely on O\_DSYNC for fsync() whenever we write to
the file" (`:1539-1540`). All journal/grid/superblock/client-replies IO is io\_uring SQEs on
that fd (`sqe.prep_fsync` exists at `linux.zig:401-403` but is used off the data path); there
is no `IOSQE_IO_LINK` anywhere in `linux.zig`. Darwin substitutes
`F_NOCACHE` + `pwrite` + `F_FULLFSYNC` per write
(`src/io/darwin.zig:824-834,1047-1051,1092-1100`); Windows substitutes `FILE_WRITE_THROUGH`
(`src/io/windows.zig:1272-1280`).

`RDWR | O_DSYNC`
`src/io/linux.zig:1429-1433`
`.DSYNC = true`
`:1432`
`O_DIRECT`
`:1508`
`assert(flags.DSYNC)`
`:1539-1540`
`sqe.prep_fsync`
`linux.zig:401-403`
`IOSQE_IO_LINK`
`linux.zig`
`F_NOCACHE`
`pwrite`
`F_FULLFSYNC`
`src/io/darwin.zig:824-834,1047-1051,1092-1100`
`FILE_WRITE_THROUGH`
`src/io/windows.zig:1272-1280`

### Sequence

### Mechanics table

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection |
| --- | --- | --- | --- | --- | --- |
| Slot selection | `src/vsr/journal.zig:396-398` | `slot = op % journal_slot_count`; slot identity shared by both rings and in-memory bitsets (`:54-64`) | — | `journal_slot_count` = 1024 slots (`src/config.zig:158`) | recovery checks `op == slot index` for reserved headers (`journal.zig:20-21`) |
| Write 1: prepare body | `src/vsr/journal.zig:1789-1852` → `:2069-2077` | body sliced to `sector_ceil`, written to `.wal_prepares` at `slot · 1 MiB` on the O\_DSYNC fd | per write (O\_DSYNC: completion = durable) | 1 prepare per slot; slot = `message_size_max` = 1 MiB (`src/config.zig:159`) | sector padding zeroed before write (`:1846`), zero-padding checked on recovery (`journal.zig:905-911,1152-1158`); body checksum covers unpadded bytes only (`message_pool.zig:147` `body_used`, e.g. `src/vsr/replica.zig:3601`) |
| Write 2: redundant header | `src/vsr/journal.zig:1856-1909` | same header rewritten into `.wal_headers` at `sector_floor(slot · 256)` — 4 slots share one 4 KiB sector; sector locked while writing | per write (O\_DSYNC) | 4 headers per sector (`headers_per_sector`, `journal.zig:47`) | shared-sector lock (`:2036-2067`) prevents a concurrent header tearing its neighbor |
| Chain guards | `src/vsr/journal.zig:1869-1875,1915-1949` | if the entry changed while writes were in flight, the slot is re-dirtied or the write abandoned | — | — | `has_header` / checksum comparisons abort stale writes |
| Slot marked clean | `src/vsr/journal.zig:1951-1956` | `dirty`/`faulty` cleared only after both durable writes complete | after write 2 | 1 slot per prepare | — |
| `prepare_ok` | `src/vsr/replica.zig:11219-11227` | consensus acknowledgment only after the two-write chain; `null` (IOP starvation, aborted write) suppresses it | — | 1 per prepare | — |
| Recovery | `src/vsr/journal.zig:2507` test-driven; header/body checksums + slot-op agreement | a torn prepare is detected and repaired from peers (`src/vsr/replica_test.zig:75-206`) | per recovery | 1024 slots scanned | torn tail, misdirected header, corrupt prepare all distinguished by checksum + reserved-op trick |

`src/vsr/journal.zig:396-398`
`slot = op % journal_slot_count`
`:54-64`
`journal_slot_count`
`src/config.zig:158`
`op == slot index`
`journal.zig:20-21`
`src/vsr/journal.zig:1789-1852`
`:2069-2077`
`sector_ceil`
`.wal_prepares`
`slot · 1 MiB`
`message_size_max`
`src/config.zig:159`
`:1846`
`journal.zig:905-911,1152-1158`
`message_pool.zig:147`
`body_used`
`src/vsr/replica.zig:3601`
`src/vsr/journal.zig:1856-1909`
`.wal_headers`
`sector_floor(slot · 256)`
`headers_per_sector`
`journal.zig:47`
`:2036-2067`
`src/vsr/journal.zig:1869-1875,1915-1949`
`has_header`
`src/vsr/journal.zig:1951-1956`
`dirty`
`faulty`
`prepare_ok`
`src/vsr/replica.zig:11219-11227`
`null`
`src/vsr/journal.zig:2507`
`src/vsr/replica_test.zig:75-206`

### Callouts

`wal_prepares`
`wal_headers`
`src/vsr/journal.zig:16-26`
`:1953-1954`
`prepare_ok`
`src/vsr/replica.zig:11225`
`slot = op % 1024`
`src/vsr/journal.zig:396-398`
`journal_slot_count = 1024`
`src/config.zig:158`
`slot · 1 MiB`
`journal.zig:39`
`sector_floor(slot · 256)`
`journal.zig:34`
`src/constants.zig:196-198`
`O_DSYNC`
`src/io/linux.zig:1432`
`:1540`
`linux.zig`
`prep_fsync`
`:401-403`
`journal_iops_write_max`
`src/config.zig:117`
`callback(null)`
`src/vsr/journal.zig:1824-1830`
`prepare_ok`
`journal.zig:47,2036-2067`
`journal.zig:20-21`
`journal.zig:905-911,1152-1158`
`recovery_cases`
`src/vsr/replica_test.zig:75-206`

### Test evidence

`test "recovery_cases"`
`src/vsr/journal.zig:2507`
`test "Cluster: recovery: WAL prepare corruption (R=3, ...)"`
`src/vsr/replica_test.zig:75,94,113,131,154`
`test "...: WAL header corruption (R=1)"`
`:171`
`test "Cluster: recovery: WAL torn prepare, standby with intact prepare (R=1 S=1)"`
`:184`
`src/config.zig:116-131`
`test_min`
`config.zig:243-246`
`src/vsr/journal_test.zig`
`journal.zig`
`src/vsr/replica_test.zig`

## Flow: Grid read / peer repair (512 KiB blocks) and free-set release at checkpoint

Trigger: an LSM/scrubber/grid-client read of a grid block, or a checksum fault triggering
repair. Participants: caller (LSM/scrubber), `Grid`, `Storage`, disk, local `Replica`, peer
`Replica`.

`Grid`
`Storage`
`Replica`
`Replica`

### Call hierarchy

Read:

`Grid.read_block`
`src/vsr/grid.zig:1179`
`Grid.read_block_tick_callback`
`grid.zig:1231`
`grid.zig:1249-1253`
`read_queue`
`read_global_queue`
`grid.zig:1256-1277`
`grid.zig:1281-1290`
`grid.zig:1295`
`grid_iops_read_max`
`src/config.zig:130`
`grid.zig:1300-1303`
`Grid.read_block_with`
`grid.zig:1308`
`storage.read_sectors(..., .grid, block_offset(address))`
`grid.zig:1324-1330`
`block_offset = (address - 1) * block_size`
`grid.zig:1560-1564`
`Grid.read_block_callback`
`grid.zig:1333`
`read_block_validate`
`grid.zig:1396-1421`
`:1402`
`:1403`
`:1409-1411`
`:1413`
`:1415-1417`
`grid.zig:1371-1374`
`verify_read_fault`
`grid.zig:1375-1391`
`Grid.read_block_resolve`
`grid.zig:1423`
`read_global_queue`
`grid.zig:1490-1504`
`blocks_missing.repair_block(address, checksum)`
`grid.zig:1506-1511`

Repair (peer fetch, ≤ `grid_repair_request_max` = 4 blocks per request, `src/config.zig:133`):

`grid_repair_request_max`
`src/config.zig:133`
`Replica.send_get_blocks(destination)`
`src/vsr/replica.zig:11229`
`.get_blocks`
`BlockRequest{block_address, block_checksum}`
`replica.zig:11244-11290`
`Replica.on_get_blocks`
`replica.zig:3324`
`Grid.read_block(from_local_storage)`
`replica.zig:3401`
`Replica.on_get_blocks_read_block`
`replica.zig:3410`
`.block`
`replica.zig:3449`
`replica.zig:3419-3428`
`Replica.on_block`
`replica.zig:3453`
`Grid.fulfill_block`
`grid.zig:844`
`Grid.repair_block_waiting`
`Grid.repair_block`
`grid.zig:866,877`
`Grid.write_block(..., .repair)`
`grid.zig:891,918`
`write_block_with`
`storage.write_sectors`
`grid.zig:963-988`

Free-set release at checkpoint:

`Grid.release`
`src/lsm/compaction.zig:1527-1533`
`src/lsm/manifest_log.zig:792`
`FreeSet.release`
`src/vsr/free_set.zig:516`
`blocks_released`
`free_set.zig:75`
`:509-515`
`Replica.commit_checkpoint_durable`
`src/vsr/replica.zig:4967`
`Grid.checkpoint_durable`
`replica.zig:4985`
`grid.zig:544-556`
`FreeSet.mark_checkpoint_durable`
`grid.zig:581`
`free_set.zig:600-607`
`blocks_released`
`:607`
`Grid.checkpoint`
`grid.zig:458`
`:462-470`
`CheckpointState`
`src/vsr/superblock.zig:343-344,357-358,376-377`

### Sequence

### Mechanics table

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection |
| --- | --- | --- | --- | --- | --- |
| Read submit | `src/vsr/grid.zig:1179-1229` | (address, checksum) reference — misdirected-read protection (`src/vsr/vsr.zig:216-227`) | — | 1 block (512 KiB, `src/config.zig:161`; `test_min` = 4 KiB, `config.zig:262`) | none |
| Read IO | `src/vsr/grid.zig:1308-1331` | sector/block-aligned read at `(address−1)·block_size`, zone `.grid` (`verify_iop` enforces block alignment, `src/vsr/vsr.zig:212`) | none (read) | 1 IOP; `grid_iops_read_max` = 32 (`src/config.zig:130`) | — |
| Validate | `src/vsr/grid.zig:1396-1421` | header checksum, command, body checksum, expected checksum, zero-padding all checked | — | 1 block | all five failure classes (`:1402,1403,1409-1411,1413,1415-1417`) |
| Fault → repair queue | `src/vsr/grid.zig:1490-1511` | root read + duplicates parked on `read_global_queue`; `blocks_missing.repair_block` schedules peer fetch | — | 1 faulted block | checksum failure is the trigger itself |
| Peer request | `src/vsr/replica.zig:11229-11317` | `.get_blocks` message carrying (address, checksum) pairs; budgeted per destination | per `grid_repair_timeout` tick | `grid_repair_request_max` = 4 blocks/request (`src/config.zig:133`) | requester re-validates on arrival (same `read_block_validate` path) |
| Peer serve | `src/vsr/replica.zig:3324-3451` | peer reads its own copy (`.from_local_storage`) and sends `.block` | — | ≤ 4 blocks per request | peer skips invalid reads (`replica.zig:3419-3428`) |
| Repair write | `src/vsr/replica.zig:3453-3525` → `src/vsr/grid.zig:877-988` | local write of donated block; O\_DSYNC completion = durable | per repaired block | 1 block per write IOP; `grid_iops_write_max` = 32 (`src/config.zig:131`) | written block carries donor's checksum; verified by any subsequent read |
| Release | `src/lsm/compaction.zig:1527-1533`, `src/lsm/manifest_log.zig:792` → `src/vsr/free_set.zig:516` | released block stays acquired for the current checkpoint (still needed after a crash) | none | per block | — |
| Free at durability | `src/vsr/replica.zig:4967,4985` → `src/vsr/grid.zig:544-581` → `src/vsr/free_set.zig:600-607` | `mark_checkpoint_durable` frees all `blocks_released` once the checkpoint is durable on a commit quorum, and only after pending repair-writes to those blocks drain (`grid.zig:548-576`) | per checkpoint durability (commit-quorum trigger) | all blocks released during the checkpoint interval | crash before durability → block still readable from current checkpoint |

`src/vsr/grid.zig:1179-1229`
`src/vsr/vsr.zig:216-227`
`src/config.zig:161`
`test_min`
`config.zig:262`
`src/vsr/grid.zig:1308-1331`
`(address−1)·block_size`
`.grid`
`verify_iop`
`src/vsr/vsr.zig:212`
`grid_iops_read_max`
`src/config.zig:130`
`src/vsr/grid.zig:1396-1421`
`:1402,1403,1409-1411,1413,1415-1417`
`src/vsr/grid.zig:1490-1511`
`read_global_queue`
`blocks_missing.repair_block`
`src/vsr/replica.zig:11229-11317`
`.get_blocks`
`grid_repair_timeout`
`grid_repair_request_max`
`src/config.zig:133`
`read_block_validate`
`src/vsr/replica.zig:3324-3451`
`.from_local_storage`
`.block`
`replica.zig:3419-3428`
`src/vsr/replica.zig:3453-3525`
`src/vsr/grid.zig:877-988`
`grid_iops_write_max`
`src/config.zig:131`
`src/lsm/compaction.zig:1527-1533`
`src/lsm/manifest_log.zig:792`
`src/vsr/free_set.zig:516`
`src/vsr/replica.zig:4967,4985`
`src/vsr/grid.zig:544-581`
`src/vsr/free_set.zig:600-607`
`mark_checkpoint_durable`
`blocks_released`
`grid.zig:548-576`

### Callouts

`block_size = 512 * KiB`
`src/config.zig:161`
`test_min`
`sector_size`
`config.zig:262`
`grid_padding`
`src/vsr/vsr.zig:130-135,141-152,168`
`src/vsr/vsr.zig:218-220`
`read_global_queue`
`src/vsr/grid.zig:1490-1511`
`src/vsr/replica.zig:11229`
`get_blocks`
`grid_repair_request_max`
`src/config.zig:133`
`replica.zig:11254-11313`
`config.zig:130-131`
`mark_checkpoint_durable`
`src/vsr/free_set.zig:600-607`
`src/vsr/grid.zig:548-556`
`src/vsr/grid.zig:1396-1421`

### Test evidence

`test "Cluster: recovery: grid corruption (disjoint)"`
`src/vsr/replica_test.zig:206`
`test "Cluster: scrub: background scrubber, fully corrupt grid"`
`replica_test.zig:1561`
`test "Cluster: repair: ack committed prepare"`
`replica_test.zig:735`
`test "FreeSet acquire/release"`
`src/vsr/free_set.zig:833`
`test "FreeSet checkpoint"`
`free_set.zig:928`
`test "FreeSet encode, decode, encode"`
`free_set.zig:1032`
`src/vsr/grid_test.zig`
`grid.zig`
`free_set.zig`
`src/vsr/replica_test.zig`

## Direct IO per OS, open-time fsync, sector discipline, IOP budgets

`O_DIRECT | O_DSYNC`
`src/io/linux.zig:1429-1433`
`.DSYNC = true`
`:1432`
`flags.DIRECT = true`
`:1508`
`:1505-1507`
`:1539-1540`
`:1516-1517`
`:1511-1521`
`:1521`
`:1495-1504`
`user_data`
`sqe.user_data`
`linux.zig:446`
`:437-443`
`sqe.prep_fsync`
`linux.zig:401-403`
`IOSQE_IO_LINK`
`linux.zig`
`F_NOCACHE`
`src/io/darwin.zig:1047-1051`
`pwrite`
`F_FULLFSYNC`
`darwin.zig:824-834`
`fs_sync`
`darwin.zig:1092-1100`
`fsync`
`FILE_WRITE_THROUGH`
`src/io/windows.zig:1272-1280`
`:1277`
`:1279-1280`
`src/io/linux.zig:1627-1636`
`fsync(fd)`
`:1631`
`fsync(dir_fd)`
`:1636`
`src/io/darwin.zig:1074-1083`
`fs_sync(fd)`
`:1078`
`fs_sync(dir_fd)`
`:1083`
`src/io/windows.zig:1385-1396`
`linux.zig:1627-1636`
`sector_size = 4096`
`src/constants.zig:489`
`src/vsr/vsr.zig:203-213`
`verify_iop`
`vsr.zig:212`
`sector_ceil`
`src/vsr/replica.zig:1665-1677`
`set_checksum_body`
`body_used()`
`src/message_pool.zig:147`
`replica.zig:3601`
`src/vsr/journal.zig:905-911,1152-1158`
`src/message_pool.zig:134,192`
`src/config.zig:116-131`
`src/constants.zig:509-510`
`wal_prepares`
`src/vsr/journal.zig:1851`
`wal_headers`
`journal.zig:1908`
`journal.zig:1953-1954`
`prepare_ok`
`src/vsr/replica.zig:11225`
`docs/ARCHITECTURE.md:86-90`

## Constants

| Constant | Definition (file:line) | Value (production) |
| --- | --- | --- |
| `journal_slot_count` | `src/config.zig:158` | 1024 |
| `message_size_max` | `src/config.zig:159` | 1 MiB |
| `superblock_copies` | `src/config.zig:160` (range check `src/constants.zig:553-559`) | 4 (even, 4–8) |
| `block_size` | `src/config.zig:161` | 512 KiB (`test_min`: 4096, `config.zig:262`) |
| `grid_repair_request_max` | `src/config.zig:133` | 4 blocks per `get_blocks` |
| `view_change_headers_suffix_max` | `src/config.zig:156` | 9 |
| `view_headers_max` | `src/constants.zig:304` | 11 |
| `@sizeOf(Header)` | `src/vsr/message_header.zig:72` (assert) | 256 B |
| `superblock_copy_size` | `src/vsr/superblock.zig:613` (padding `:607-610`) | 8 KiB + 16 KiB = 24 KiB |
| `superblock_zone_size` | `src/vsr/superblock.zig:602` | 96 KiB |
| `journal_size_headers` / `journal_size_prepares` | `src/constants.zig:197-198` | 256 KiB / 1 GiB |
| `sector_size` | `src/constants.zig:489` | 4096 |
| verify (write) quorum | `src/vsr/superblock_quorums.zig:61-66` (rule `:57`) | 3 of 4 |
| open (read) quorum | `src/vsr/superblock_quorums.zig:69-74` | 2 of 4 |
| IOP budgets | `src/config.zig:116-131`; superblock `src/constants.zig:509-510` | journal 8r/32w, client-replies 1r/2w, grid 32r/32w, superblock 1r/1w |

`journal_slot_count`
`src/config.zig:158`
`message_size_max`
`src/config.zig:159`
`superblock_copies`
`src/config.zig:160`
`src/constants.zig:553-559`
`block_size`
`src/config.zig:161`
`test_min`
`config.zig:262`
`grid_repair_request_max`
`src/config.zig:133`
`get_blocks`
`view_change_headers_suffix_max`
`src/config.zig:156`
`view_headers_max`
`src/constants.zig:304`
`@sizeOf(Header)`
`src/vsr/message_header.zig:72`
`superblock_copy_size`
`src/vsr/superblock.zig:613`
`:607-610`
`superblock_zone_size`
`src/vsr/superblock.zig:602`
`journal_size_headers`
`journal_size_prepares`
`src/constants.zig:197-198`
`sector_size`
`src/constants.zig:489`
`src/vsr/superblock_quorums.zig:61-66`
`:57`
`src/vsr/superblock_quorums.zig:69-74`
`src/config.zig:116-131`
`src/constants.zig:509-510`

## One-paragraph summary

Every consensus-critical write in TigerBeetle 0.17.9 is a direct-to-disk synchronous write whose
completion means durability (O\_DIRECT|O\_DSYNC / F\_FULLFSYNC / FILE\_WRITE\_THROUGH), issued
through io\_uring with per-zone IOP budgets, onto 4-KiB-aligned, checksummed sectors of a
fixed-zone data file whose superblock (4 on-node copies at 24 KiB each, write quorum 3, read
quorum 2, `src/vsr/superblock_quorums.zig:57,61-74`) anchors recovery.

`src/vsr/superblock_quorums.zig:57,61-74`

## Appendix: AOF (optional feature)

Trigger: one committed prepare reaching `execute_op` on a replica started with an AOF.
Participants: `Replica` (commit path), `AOF` (append-only file state machine), `IO` (blocking
AOF helpers), page cache, disk. Recovery participants: CLI (`aof recover|debug|merge`),
`AOF.Iterator`, `AOFReplayClient`. Entries carry the u128 magic
`0xbcd8d3fee406119ed192c4f4c4fc82` (`src/aof.zig:28`).

`execute_op`
`Replica`
`AOF`
`IO`
`aof recover|debug|merge`
`AOF.Iterator`
`AOFReplayClient`
`0xbcd8d3fee406119ed192c4f4c4fc82`
`src/aof.zig:28`

### Call hierarchy

Append (per prepare, synchronous, on the commit path):

`Replica.commit_execute`
`src/vsr/replica.zig:4891`
`Replica.execute_op`
`src/vsr/replica.zig:5344`
`AOF.write`
`src/aof.zig:172`
`replica.zig:5417-5425`
`AOFEntry.from_message`
`src/aof.zig:66`
`last_checksum`
`:87`
`AOFEntry.size_disk`
`src/aof.zig:48`
`IO.aof_blocking_write_all`
`src/io/linux.zig:1816`
`src/io/darwin.zig:1165`
`src/io/windows.zig:1464`
`common.aof_blocking_write_all`
`src/io/common.zig:123`
`std.fs.File.writeAll`
`src/aof.zig:36`
`src/io/common.zig:152-157`

Open (once, at startup):

`AOF.init`
`src/aof.zig:148`
`IO.aof_blocking_open`
`src/io/linux.zig:1836`
`common.aof_blocking_open`
`src/io/common.zig:147`
`dir.createFile`
`io/common.zig:152-157`
`file.sync()`
`io/common.zig:160`
`fsync(dir_fd)`
`io/common.zig:164-166`

Durable flush (once per checkpoint):

`Replica`
`src/vsr/replica.zig:5031-5034`
`AOF.checkpoint`
`src/aof.zig:197`
`IO.fsync`
`src/aof.zig:213-219`
`AOF.on_fsync`
`src/aof.zig:222`
`unflushed = 0`
`:229`

Unflushed-counter reset without fsync (state sync): `Replica` — `src/vsr/replica.zig:10485` →
`AOF.sync` — `src/aof.zig:191`.

`Replica`
`src/vsr/replica.zig:10485`
`AOF.sync`
`src/aof.zig:191`

Recovery merge / replay (offline CLI):

`src/aof.zig:955-1004`
`.recover`
`AOFReplayClient.init`
`replay(&it)`
`src/aof.zig:961-966`
`.debug`
`src/aof.zig:969-990`
`.merge`
`AOF.merge`
`src/aof.zig:595`
`Iterator.next`
`src/aof.zig:512`
`validate_chain`
`error.AOFChecksumChainMismatch`
`src/aof.zig:544-547`

Opt-in flags: `aof_file`/`aof`/`aof_recovery` — `src/tigerbeetle/cli.zig:105,109,113`; legacy
`--aof` builds `<data file>.aof` — `cli.zig:1055-1072`; passed to the command —
`cli.zig:1145-1146`; replay mode relaxes client timestamps — `src/vsr/client.zig:66,294`.

`aof_file`
`aof`
`aof_recovery`
`src/tigerbeetle/cli.zig:105,109,113`
`--aof`
`<data file>.aof`
`cli.zig:1055-1072`
`cli.zig:1145-1146`
`src/vsr/client.zig:66,294`

### Sequence

### Mechanics table

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection |
| --- | --- | --- | --- | --- | --- |
| Entry encode | `src/aof.zig:66-113` | `AOFEntry` = u128 magic + embedded VSR `Header.Prepare`; chains by `parent == previous checksum` (`:81-87`) | none (memory only) | 1 entry per prepare; entry = 16 B + `header.size` (`size_disk`, `src/aof.zig:48-50`) | none yet |
| Borrow bound | `src/aof.zig:174` | `assert(unflushed < constants.journal_slot_count)` — the WAL can always re-durable the unflushed AOF tail | — | max unflushed = `journal_slot_count` = 1024 (`src/config.zig:158`) | crash of AOF only → WAL replays tail |
| Append write | `src/io/common.zig:123-126` via `src/aof.zig:185` | one blocking `writeAll` per prepare through the page cache; no O\_DIRECT/O\_SYNC (`src/io/common.zig:152-157`, `src/aof.zig:36`) | none; writeback by kernel | 1 write call per op | none |
| Open-time flush | `src/io/common.zig:160-166` | `file.sync()` + dir fsync (non-Windows) at open; also drains any pending page-cache writes from a crashed prior process | per open (one-shot) | n/a | stale page-cache hole closed at open |
| Checkpoint flush | `src/aof.zig:213-219` (call site `src/vsr/replica.zig:5031-5034`) | single `io.fsync` on the AOF fd; the only durability point in the append path | per checkpoint (op-count trigger: checkpoint interval) | whole unflushed tail ≤ 1024 ops | `on_fsync` error → panic (`src/aof.zig:224`) |
| Counter reset (state sync) | `src/aof.zig:191-195` via `src/vsr/replica.zig:10485` | `unflushed = 0` without fsync (state machine will re-sync from peers) | per state-sync | ≤ 1024 | none |
| Recovery read | `src/aof.zig:512-549` | iterator validates entry checksum, body checksum, then chain (`parent` vs `last_checksum`) | offline (CLI) | 1 entry at a time; file size unbounded | `AOFChecksumMismatch` (`:536`), `AOFBodyChecksumMismatch` (`:540`), `AOFChecksumChainMismatch` (`:546`); magic u128 (`src/aof.zig:28`) lets a scanner resync past garbage (`aof.zig:530,568-592`) |
| Merge | `src/aof.zig:595` (dispatch `:991-1004`) | merges ≤ `members_max` AOFs, dropping duplicate/backtracking entries, into one consistent file | offline | ≤ `members_max` input files | chain re-validated while merging (`:646` disables strict mode selectively) |

`src/aof.zig:66-113`
`AOFEntry`
`Header.Prepare`
`parent == previous checksum`
`:81-87`
`header.size`
`size_disk`
`src/aof.zig:48-50`
`src/aof.zig:174`
`assert(unflushed < constants.journal_slot_count)`
`journal_slot_count`
`src/config.zig:158`
`src/io/common.zig:123-126`
`src/aof.zig:185`
`writeAll`
`src/io/common.zig:152-157`
`src/aof.zig:36`
`src/io/common.zig:160-166`
`file.sync()`
`src/aof.zig:213-219`
`src/vsr/replica.zig:5031-5034`
`io.fsync`
`on_fsync`
`src/aof.zig:224`
`src/aof.zig:191-195`
`src/vsr/replica.zig:10485`
`unflushed = 0`
`src/aof.zig:512-549`
`parent`
`last_checksum`
`AOFChecksumMismatch`
`:536`
`AOFBodyChecksumMismatch`
`:540`
`AOFChecksumChainMismatch`
`:546`
`src/aof.zig:28`
`aof.zig:530,568-592`
`src/aof.zig:595`
`:991-1004`
`members_max`
`members_max`
`:646`

### Callouts

`src/aof.zig:169-171`
`journal_slot_count`
`src/aof.zig:174`
`src/config.zig:158`
`src/aof.zig:213-219`
`src/vsr/replica.zig:5034`
`src/io/common.zig:160-166`
`execute_op`
`src/vsr/replica.zig:5417-5425`
`src/aof.zig:48-50`
`0xbcd8d3fee406119ed192c4f4c4fc82`
`src/aof.zig:28`
`src/aof.zig:536-547`
`aof merge`

### Test evidence

`test "aof write / read"`
`src/aof.zig:792`
`AOFType`
`Iterator`
`test "aof merge"`
`src/aof.zig:873`
`AOF.merge`
`src/testing/cluster.zig:126-128`
`aofs`
`aof_ios`
`aof_io_files`
`AOF.checkpoint`
`test`
`src/aof.zig`
`src/aof_test.zig`

# TigerBeetle 0.17.9 — its flavour of Viewstamped Replication, the mechanics

All `file:line` citations are from the 0.17.9 release source tree. Path convention:
paths without a directory prefix are `src/vsr/<name>`; `vsr.zig`, `storage.zig`, `message_bus.zig`,
`config.zig`, `constants.zig`, `state_machine.zig`, `tigerbeetle.zig` and `io/*` are `src/<name>`.

`file:line`
`src/vsr/<name>`
`vsr.zig`
`storage.zig`
`message_bus.zig`
`config.zig`
`constants.zig`
`state_machine.zig`
`tigerbeetle.zig`
`io/*`
`src/<name>`

## 0. Summary

TigerBeetle 0.17.9 is **the VRR-2012 message family running on a durability substrate that is
TigerBeetle's own**:

`JoinView`
`View`
`ExitView`
`DoViewChange`
`StartView`
`StartViewChange`
`docs/internals/vsr.md:17`
`prepare_ok`
`on_prepare`
`append`
`replica.zig:4210`
`write_prepare`
`replica.zig:11178`
`journal.write_prepare`
`journal.zig:1789`
`journal.zig:1842-1851`
`journal.zig:1890-1908`
`journal.zig:1951-1956`
`write_prepare_callback`
`replica.zig:11219-11227`
`prepare_ok`
`io/linux.zig:1539-1542`
`io/linux.zig:437-443`
`pwrite`
`F_FULLFSYNC`
`io/darwin.zig:826-833`
`:1095-1100`
`recovery`
`recovery_response`
`Command`
`src/vsr.zig:230-276`
`docs/internals/vsr.md:55-56`
`epoch`
`message_header.zig:174`

## 1. Message set (src/vsr.zig:230, dispatch at src/vsr/replica.zig:1802-1834)

| Command | Direction | Purpose | Handler (replica.zig) |
| --- | --- | --- | --- |
| `request` | client → primary | client batch (up to 1 MiB / 8,190 transfers per op; `message_size_max` `config.zig:159` → `constants.zig:234`; `batch_max.create_transfers` derived `state_machine.zig:336-358`, `tigerbeetle.zig:113`, stated `constants.zig:640`) | `on_request` :1944 |
| `prepare` | primary → replicas + standbys (star) | one op, piggybacks primary's `commit_max` | `on_prepare` :2021 |
| `prepare_ok` | replica → primary | sent **only after** the prepare is durable in the WAL | `on_prepare_ok` :2248 |
| `reply` | primary + one random backup → client | committed result (dedup via client sessions, `on_reply` :2346-2362) | `on_reply` :2342 |
| `commit` | primary → replicas + standbys | lazy commit notification, doubles as 500 ms liveness heartbeat | `on_commit` :2396 |
| `exit_view` | replica → all | StartViewChange analogue; view-change quorum starts a view change | `on_exit_view` :2549 |
| `join_view` | replica → all | DoViewChange analogue; carries log-suffix headers + present/nack bitsets + `commit_min` | `on_join_view` :2608 |
| `view` | new primary → backups | StartView analogue; also resets a `.recovering_head` replica (nonce-checked) | `on_view` :2759 |
| `get_view`/`ping_client`/`pong_client` | backup/client → primary | learn current view | `on_get_view` :3022 |
| `get_headers`/`headers` | replica ↔ replica | WAL header hash-chain repair | :3159/:3300 |
| `get_prepare` | replica ↔ replica | fetch one prepare from an "underwriter"; never nack an op possibly acked | :3056 |
| `get_reply` | replica ↔ replica | client-reply-zone repair | :3213 |
| `get_blocks`/`block` | replica ↔ replica | LSM grid block repair / state sync (rate-limited via `grid_repair_message_budget`, :11235-11238) | :3324/:3453 |
| `ping`/`pong` | replica ↔ replica | Marzullo clock sync (`clock.learn` :1912-1917) + release advertisement (`vsr_releases_max`, `constants.zig:87-89`) | :1849/:1898 |
| `eviction` | primary → client | client session evicted (replicas warn "misdirected message" at :1822-1828) | (not handled by replicas) |

`request`
`message_size_max`
`config.zig:159`
`constants.zig:234`
`batch_max.create_transfers`
`state_machine.zig:336-358`
`tigerbeetle.zig:113`
`constants.zig:640`
`on_request`
`prepare`
`commit_max`
`on_prepare`
`prepare_ok`
`on_prepare_ok`
`reply`
`on_reply`
`on_reply`
`commit`
`on_commit`
`exit_view`
`on_exit_view`
`join_view`
`commit_min`
`on_join_view`
`view`
`.recovering_head`
`on_view`
`get_view`
`ping_client`
`pong_client`
`on_get_view`
`get_headers`
`headers`
`get_prepare`
`get_reply`
`get_blocks`
`block`
`grid_repair_message_budget`
`ping`
`pong`
`clock.learn`
`vsr_releases_max`
`constants.zig:87-89`
`eviction`

20 live types (`vsr.zig:234-269`), 4 deprecated ordinals (:266-269). **No `recovery`/
`recovery_response` exists.** `docs/internals/vsr.md:55-56`: "Unlike [VRR], TigerBeetle does not
implement Recovery Protocol (see §4.3). Instead, replicas persist their VSR state to the
superblock." The `epoch: u32` header field is reserved and rejected when nonzero
(`message_header.zig:174`). There is **no cross-cluster replication protocol** in 0.17.9 —
`cluster: u128` is a misdirection guard (`message_header.zig:35-38`, enforced by dropping
mismatched messages at dispatch, `replica.zig:1758-1764`); the `reconfigure` operation is
reserved (`vsr.zig:291-292`) but scaffolding marked TODO in docs
(`docs/internals/vsr.md:273-275`).

`vsr.zig:234-269`
`recovery`
`recovery_response`
`docs/internals/vsr.md:55-56`
`epoch: u32`
`message_header.zig:174`
`cluster: u128`
`message_header.zig:35-38`
`replica.zig:1758-1764`
`reconfigure`
`vsr.zig:291-292`
`docs/internals/vsr.md:273-275`

## 2. Normal case: request → prepare → journal double-write → prepare\_ok quorum

Trigger: a client `request` lands on the primary while `pipeline.queue` has room.
Participants: Client, R0 (primary), R1/R2 (backups), WAL (Journal), IO.

`request`
`pipeline.queue`

Call hierarchy:

`on_message`
`.request => on_request`
`replica.zig:1806`
`on_request`
`replica.zig:1944`
`primary_pipeline_prepare(request)`
`primary_pipeline_prepare`
`replica.zig:7283`
`state_machine.prepare`
`build(.prepare)`
`.op = self.op + 1`
`.commit = self.commit_max`
`prepare_timeout.start()`
`pipeline.queue.push_prepare`
`self.on_prepare(message)`
`on_prepare`
`replica.zig:2021`
`self.replicate(message)`
`send_message_to_other_replicas_and_standbys`
`send_message_to_replica_base`
`message_bus.send_message_to_replica`
`message_bus.zig:900`
`on_prepare`
`self.op = message.header.op`
`journal.set_header_as_dirty`
`self.append(message)`
`append`
`replica.zig:4210`
`write_prepare`
`write_prepare`
`replica.zig:11178`
`journal.write_prepare`
`Journal.write_prepare`
`journal.zig:1789`
`journal.writes.acquire()`
`journal_iops_write_max`
`write_sectors(write_prepare_header, …, .prepares, offset)`
`Journal.write_sectors`
`journal.zig:2015`
`lock_sectors`
`storage.write_sectors`
`Storage.write_sectors`
`storage.zig:348`
`start_write`
`io.write`
`io/linux.zig:437-443`
`sqe.prep_write`
`O_DIRECT|O_DSYNC`
`io/linux.zig:1508`
`:1539-1542`
`io/darwin.zig:826-833`
`pwrite`
`fs_sync`
`F_FULLFSYNC`
`:1095-1100`
`write_prepare_header`
`journal.zig:1856`
`write_sectors(…, .headers, …)`
`headers_redundant`
`write_prepare_on_write_header`
`journal.zig:1911`
`dirty.clear(slot)`
`faulty.clear(slot)`
`write_prepare_release`
`write_prepare_callback`
`replica.zig:11219`
`send_prepare_ok(message.header)`
`send_prepare_ok`
`replica.zig:8615`
`journal.has_prepare(header)`
`journal.zig:540-549`
`if (self.standby()) return`
`on_prepare_ok`
`replica.zig:2248`
`threshold = self.quorum_replication`
`count_message_and_receive_quorum_exactly_once`
`prepare.ok_quorum_received = true`
`commit_pipeline()`

Mechanics:

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection (where, how) |
| --- | --- | --- | --- | --- | --- |
| Request accepted | `replica.zig:1944`, :1997-2001 | primary asserts `commit_max + prepare_queue.count == op`; pipelines or queues the request | none | pipeline holds ≤ `pipeline_prepare_queue_max`=8 prepares (`config.zig:155` → `constants.zig:255`); overflow goes to request queue | n/a |
| Request → prepare | `replica.zig:7370-7414` | zero-copy: Request buffer's header is overwritten in place; `.op = self.op + 1` :7393; commit watermark piggybacked `.commit = self.commit_max` :7394 | none | one op = one prepare = up to `batch_max.create_transfers`=8,190 transfers, derived `(message_size_max − 256)/128` (`state_machine.zig:336-358`, `tigerbeetle.zig:113`, `constants.zig:235`, stated `constants.zig:640`) | n/a |
| Star fanout | `replica.zig:8550-8552` → `:9111-9118` → `message_bus.zig:900` | flat loop over `0..node_count` (replicas + standbys, `standbys_max`=6, `constants.zig:26-28`); unconnected peers silently dropped (`message_bus.zig:904-911`) | none | one message per member per op | n/a |
| Journal write (body) | `journal.zig:1789`, :1842-1851 → `:2015` → `:2069` → `storage.zig:348/:374` | `wal_prepares` slot at `op % 1024` (`journal.zig:396-398`); write slot acquired from a 32-deep pool (`journal_iops_write_max`, `config.zig:117` → `constants.zig:516`) | **Flush 1 — per write, O\_DSYNC** (`io/linux.zig:1539-1542`; Darwin `pwrite+F_FULLFSYNC` `io/darwin.zig:829-830, :1095-1100`); triggered by every prepare on every replica, before any ack | ≤ `journal_iops_write_max`=32 concurrent WAL writes (`config.zig:117`); slot padded to `message_size_max`=1 MiB (`config.zig:159` → `constants.zig:234`, `journal_size_prepares` `constants.zig:198`) | a torn body leaves the slot's `headers_redundant` entry stale; recovery classifies vs. the redundant header (see below) |
| Journal write (redundant header) | `journal.zig:1856` → `:1890-1908` | sector holding 16 headers (`headers_per_sector`, `journal.zig:47`) copied from `headers_redundant` so a header whose prepare is absent is written as reserved (`journal.zig:2144-2151`, rationale :2144-2145 and worked example :225-233) | **Flush 2 — per write, chained after Flush 1 completes**; both writes gate `prepare_ok` | 1 sector per 16 slots; every op is on disk twice (body + header) | a leaked header without its prepare would be case @K (remote repair); writing reserved-until-durable keeps it case @L (local `nil`) |
| Slot marked clean | `journal.zig:1951-1956` | `dirty.clear` + `faulty.clear`; `write_prepare_release` fires the replica callback | durability point for the slot | n/a | n/a |
| prepare\_ok gating | `replica.zig:11219-11227` → `:8615`, :8665 (`journal.zig:540-549`) | ack only if the slot is clean (`has_prepare` requires `!dirty`); standbys journal but return before voting :8672 (`standby()` :6925-6928) | ack is a durability acknowledgment, not a volatile vote | 1 prepare\_ok per replica per op; bitset dedup `ok_from_all_replicas` | a dirty/faulty slot silently withholds the ack |
| Quorum gate | `replica.zig:2294`, :2307-2319 | commit fires exactly when the `threshold`-th unique prepare\_ok arrives | none | `quorum_replication = min(3, ceil(N/2))`, forced 2 for N=2 (`vsr.zig:1185-1188`, cap `config.zig:157`); N=3→2, N=4→2, N=5→3, N=6→3 (`vsr.zig:1239-1240`) | n/a |

`replica.zig:1944`
`commit_max + prepare_queue.count == op`
`pipeline_prepare_queue_max`
`config.zig:155`
`constants.zig:255`
`replica.zig:7370-7414`
`.op = self.op + 1`
`.commit = self.commit_max`
`batch_max.create_transfers`
`(message_size_max − 256)/128`
`state_machine.zig:336-358`
`tigerbeetle.zig:113`
`constants.zig:235`
`constants.zig:640`
`replica.zig:8550-8552`
`:9111-9118`
`message_bus.zig:900`
`0..node_count`
`standbys_max`
`constants.zig:26-28`
`message_bus.zig:904-911`
`journal.zig:1789`
`:2015`
`:2069`
`storage.zig:348/:374`
`wal_prepares`
`op % 1024`
`journal.zig:396-398`
`journal_iops_write_max`
`config.zig:117`
`constants.zig:516`
`io/linux.zig:1539-1542`
`pwrite+F_FULLFSYNC`
`io/darwin.zig:829-830, :1095-1100`
`journal_iops_write_max`
`config.zig:117`
`message_size_max`
`config.zig:159`
`constants.zig:234`
`journal_size_prepares`
`constants.zig:198`
`headers_redundant`
`journal.zig:1856`
`:1890-1908`
`headers_per_sector`
`journal.zig:47`
`headers_redundant`
`journal.zig:2144-2151`
`prepare_ok`
`nil`
`journal.zig:1951-1956`
`dirty.clear`
`faulty.clear`
`write_prepare_release`
`replica.zig:11219-11227`
`:8615`
`journal.zig:540-549`
`has_prepare`
`!dirty`
`standby()`
`ok_from_all_replicas`
`replica.zig:2294`
`threshold`
`quorum_replication = min(3, ceil(N/2))`
`vsr.zig:1185-1188`
`config.zig:157`
`vsr.zig:1239-1240`

Callouts:

`wal_prepares`
`wal_headers`
`journal.zig:2144-2145`
`journal.zig:16-26`
`constants.zig:196-198`
`message_header.zig:72`
`journal.write_sectors`
`storage.write_sectors`
`io.write`
`io/linux.zig:1539-1542`
`F_FULLFSYNC`
`pwrite`
`io/darwin.zig:829-830`
`batch_max`
`state_machine.zig:336-358`
`constants.zig:235`
`tigerbeetle.zig:113`
`pipeline_prepare_queue_max`
`config.zig:155`
`journal_iops_write_max`
`config.zig:117`
`pipeline.queue.push_request`
`replica.zig:1997-1998`
`journal.zig:1824-1830`
`prepare_timeout`
`replica.zig:1366-1370`
`on_prepare_timeout`
`commit_message_timeout`
`replica.zig:1376-1380`
`quorum_replication`
`replica.zig:2294-2319`
`quorum_view_change`
`quorum_nack_prepare`
`quorum_view_change + quorum_replication > N`
`vsr.zig:1203`
`replica.zig:8672`
`Journal.recover_slots`
`journal.zig:1240`
`journal.zig:1204-1241`
`torn_prepares`
`journal.zig:1359-1362`
`journal_iops_write_max`
`journal.zig:1469-1478`
`vsr`
`fix`
`cut`
`nil`
`eql`

Test evidence (cluster-level, `src/vsr/replica_test.zig`):

`src/vsr/replica_test.zig`
`replica_test.zig:75-92`
`recovering_head`
`replica_test.zig:154-169`
`error.WALCorrupt`
`replica_test.zig:171-182`
`fix`
`replica_test.zig:184-204`
`replica_test.zig:391-402`
`quorum_replication=2`
`replica_test.zig:530-543`
`replica_test.zig:1427-1452`
`io/test.zig:25-155`
`io/linux.zig:1539-1540`

## 3. Star fanout of prepares from the primary

Trigger: every `on_prepare` on the primary (new op), plus timed re-sends. Participants: P
(primary), R1, R2, S0 (standby), MessageBus, prepare\_timeout.

`on_prepare`

Call hierarchy:

`on_prepare`
`replica.zig:2021`
`replicate`
`send_message_to_other_replicas_and_standbys`
`status == .normal and primary()`
`0..node_count`
`send_message_to_replica_base`
`message_bus.send_message_to_replica`
`message_bus.zig:900-913`
`bus.send_message`
`prepare_timeout`
`replica.zig:1366-1370`
`on_prepare_timeout`
`replica.zig:3608-3676`
`prepare.ok_from_all_replicas`
`backoff`
`on_repair`
`node_count = replica_count + standby_count`
`constants.zig:24-28`
`replica.zig:8672`
`:6925-6928`
`send_message_to_other_replicas`
`0..replica_count`
`..._and_standbys`
`exit_view`
`send_header_to_other_replicas`

Mechanics:

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection (where, how) |
| --- | --- | --- | --- | --- | --- |
| First fanout | `replica.zig:8550-8552` → `:9111-9118` | star: one direct message per member (no chain); runs in parallel with the primary's own journal write (:2008-2009, :2011-2012) | none (network only) | `node_count` messages per op; `members_max` = 6 replicas + 6 standbys (`constants.zig:24-28`) | unconnected peer → silently dropped (`message_bus.zig:904-911`); full send queue → dropped with log (`message_bus.zig:941-944`) |
| Re-send | `replica.zig:3608-3676` | on 250 ms timeout, message each replica missing from `prepare.ok_from_all_replicas` | none | one re-send per unacked replica per expiry | the ack bitset *is* the fanout-progress detector |
| Abdication | `primary_abdicate_timeout` 10 s (`replica.zig:1371-1375`) → `:3678-3689` | if no requests can be accepted (clock desync) or no quorum returns, the primary stops heartbeating rather than split-braining | none | 10 s | `send_commit: primary abdicating` (log mark, `replica.zig:11352-11360`) |
| Standby handling | `replica.zig:8672`, `:6925-6928` | standbys journal prepares (§2 path) but return before `prepare_ok` | same WAL flushes as replicas | n/a | n/a |

`replica.zig:8550-8552`
`:9111-9118`
`node_count`
`members_max`
`constants.zig:24-28`
`message_bus.zig:904-911`
`message_bus.zig:941-944`
`replica.zig:3608-3676`
`prepare.ok_from_all_replicas`
`primary_abdicate_timeout`
`replica.zig:1371-1375`
`:3678-3689`
`send_commit: primary abdicating`
`replica.zig:11352-11360`
`replica.zig:8672`
`:6925-6928`
`prepare_ok`

Callouts:

`replica.zig:9111-9118`
`replica.zig:7443-7444`
`replica.zig:1366-1370`
`quorum_replication`
`prepare_ok`
`message_bus.zig:904-911`

Test evidence (`src/vsr/replica_test.zig` unless noted):

`src/vsr/replica_test.zig`
`replica_test.zig:391-432`
`replica_test.zig:450-464`
`replica_test.zig:479-495`
`replica_test.zig:497-528`
`execute_op_reply_to_client`
`vsr.zig:1239-1243`
`expect_replication`
`expect_view_change`

## 4. Commit: flush-free determination, dissemination, checkpoint

Trigger: the `threshold`-th unique prepare\_ok (primary), a `commit` message or a piggybacked
`.commit` field (backups). Participants: P (primary), R1/R2, FaultDetector, Superblock, Client.

`threshold`
`commit`
`.commit`

Call hierarchy:

`on_prepare_ok`
`commit_pipeline`
`replica.zig:4285`
`commit_dispatch_enter`
`commit_dispatch`
`.execute`
`.checkpoint_durable`
`.compact`
`.checkpoint_data`
`.checkpoint_superblock`
`commit_finish`
`commit_execute`
`replica.zig:4893`
`execute_op`
`execute_op_reply_to_client`
`send_reply_message_to_client`
`message_bus.send_message_to_client`
`message_bus.zig:917-929`
`commit_checkpoint_superblock`
`replica.zig:5110`
`superblock.checkpoint`
`superblock.zig:891-987`
`vsr_state.commit_max = update.commit_max`
`write_staging`
`write_header`
`superblock_copies`
`config.zig:160`
`constants.zig:553`
`read_working(.verify)`
`quorums.working`
`on_prepare`
`advance_commit_max(message.header.commit)`
`commit_journal`
`on_commit`
`replica.zig:2396`
`commit_fault.signal`
`advance_commit_max`
`commit_journal`
`commit_dispatch_enter`
`commit_message_timeout`
`replica.zig:1376-1380`
`tick_ms`
`config.zig:120`
`constants.zig:717`
`on_commit_message_timeout`
`send_commit`
`replica.zig:11342`
`commit_fault.signal(now)`
`Header.Commit{ .commit = self.commit_max, .commit_checksum = latest_committed_entry }`
`FaultDetector`
`fault_detector.zig:32-36`
`interval_min=100ms`
`interval_max=2000ms`
`replica.zig:1381-1388`

Mechanics:

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection (where, how) |
| --- | --- | --- | --- | --- | --- |
| Commit determination | `replica.zig:2339` → `:4285-4308` | primary-side: fired by the `quorum_replication`-th unique prepare\_ok; commit\_max is a volatile counter | **none — zero durability writes on the commit path** | threshold = `quorum_replication` (`vsr.zig:1185-1188`) | n/a |
| Execute + reply | `replica.zig:4893`, `execute_op` :5344, reply :5525-5536 | op applied to state machine; client table updated; reply sent by primary *and* one op-seeded random backup (`:5328-5342`) | none (client reply zone writes are async: `client_replies.zig:11-14`) | replies coalesced per client session | n/a |
| Piggybacked commit | `.commit = self.commit_max` `replica.zig:7394`; consumed `:2107-2110` | every prepare header carries the primary's watermark; backups advance on receipt | none | 1 watermark per prepare | none — stale watermarks are simply lower |
| Commit message | `send_commit` `replica.zig:11342-11380`; `on_commit` :2396-2452 | network-only header carrying `commit` + `commit_checksum`; backup verifies the checksum against its own journal header :2438-2448 | none | 1 heartbeat per `commit_message_timeout` = 500 ms (`replica.zig:1376-1380`) | checksum mismatch with a valid local hash chain → `@panic("commit checksum verification failed")` :2441-2442 |
| Heartbeat / liveness | `on_commit_message_timeout` :3691-3699; `commit_fault.signal` :11351 and :2429-2431 | the same message doubles as the fault detector's liveness signal; EWMA over interarrival (`fault_detector.zig:32-36`, bounds `replica.zig:1381-1388`) | none | `interval_min=100ms`, `interval_max=2000ms` | a red fault detector drives `exit_view` (§5) |
| Checkpoint | `replica.zig:5110-5116` gate → `superblock.zig:891-987`, `:1088-1122`, `:1124-1173`, `:1264-1281` | when the committing op equals `op_checkpoint_next_trigger()`: LSM, client replies, client sessions, grid flushed, then the superblock is rewritten on 4 copies and read-verified on a flexible quorum (2/4 or 3/4) | **Flush — timer-free, op-count-triggered**: every `vsr_checkpoint_ops` = 960 ops = 1024 − 32 − 32·⌈16/32⌉ (`constants.zig:40-42`, inputs `config.zig:158/155/164`) | `superblock_copies`=4 (`config.zig:160`); read quorum ≥2 valid copies (`superblock.zig:1264`, quorum classes `superblock.zig:631-651`) | post-write verify read: working checksum ≠ staging checksum → panic (`superblock.zig:1276-1281`) |

`replica.zig:2339`
`:4285-4308`
`quorum_replication`
`quorum_replication`
`vsr.zig:1185-1188`
`replica.zig:4893`
`execute_op`
`:5328-5342`
`client_replies.zig:11-14`
`.commit = self.commit_max`
`replica.zig:7394`
`:2107-2110`
`send_commit`
`replica.zig:11342-11380`
`on_commit`
`commit`
`commit_checksum`
`commit_message_timeout`
`replica.zig:1376-1380`
`@panic("commit checksum verification failed")`
`on_commit_message_timeout`
`commit_fault.signal`
`fault_detector.zig:32-36`
`replica.zig:1381-1388`
`interval_min=100ms`
`interval_max=2000ms`
`exit_view`
`replica.zig:5110-5116`
`superblock.zig:891-987`
`:1088-1122`
`:1124-1173`
`:1264-1281`
`op_checkpoint_next_trigger()`
`vsr_checkpoint_ops`
`constants.zig:40-42`
`config.zig:158/155/164`
`superblock_copies`
`config.zig:160`
`superblock.zig:1264`
`superblock.zig:631-651`
`superblock.zig:1276-1281`

Callouts:

`quorum_replication`
`commit`
`replica.zig:2451`
`commit_checkpoint_superblock`
`commit_op == op_checkpoint_next_trigger()`
`replica.zig:5114-5116`
`vsr_state.commit_max`
`superblock.zig:959`
`.commit`
`replica.zig:7394`
`vsr_checkpoint_ops`
`constants.zig:40-42`
`replica.zig:1376-1380`
`send_commit`
`quorum_replication`
`quorum_view_change`
`superblock.zig:1264-1268`
`superblock.zig:1277-1279`
`constants.zig:44-73`

Test evidence (`src/vsr/replica_test.zig`):

`src/vsr/replica_test.zig`
`replica_test.zig:66-73`
`checkpoint_2_trigger`
`replica_test.zig:391-528`
`replica_test.zig:450-464`
`send_commit: primary abdicating`
`replica_test.zig:568-599`
`replica_test.zig:2084+`
`replica.zig:2157-2178`

## 5. View change: exit\_view → join\_view quorum → truncate/repair → view

Trigger: primary timeout (primary side) or EWMA fault detector (backup side). Participants: R0
(old primary), R1, R2 (candidate primary), Superblock, Journal.

Call hierarchy:

`on_prepare_timeout`
`replica.zig:3608`
`commit_fault`
`send_exit_view`
`replica.zig:8762-8781`
`on_exit_view`
`replica.zig:2549`
`threshold = self.quorum_view_change`
`transition_to_view_change_status(self.view + 1)`
`transition_to_view_change_status`
`replica.zig:10160`
`update_join_view_headers`
`status = .view_change`
`view = view_new`
`view_durable_update()`
`send_join_view()`
`view_durable_update`
`replica.zig:9448`
`superblock.view_change`
`superblock.zig:1010-1030`
`commit_max`
`view`
`log_view`
`view_headers`
`view_durable_update_callback`
`replica.zig:9533`
`send_join_view`
`replica.zig:8783`
`on_join_view`
`replica.zig:2608`
`JVQuorum.verify`
`JVQuorum.quorum_headers(... .quorum_view_change ...)`
`awaiting_quorum`
`commit_max = max(self.commit_max, JVQuorum.commit_max(...))`
`transition_to_view_change_status(next_view)`
`join_view_quorum = true`
`primary_set_log_from_join_view_messages`
`journal_repair_timeout.start()`
`repair()`
`set_op_and_commit_max`
`replica.zig:9619`
`self.op = op`
`journal.remove_entries_from(self.op + 1)`
`journal.zig:1705`
`on_get_prepare`
`replica.zig:3056-3125`
`on_get_headers`
`primary_send_view`
`replica.zig:9515-9531`
`on_view`
`replica.zig:2759`
`on_view_set_checkpoint`
`on_view_set_journal`
`replica.zig:2913`
`set_op_and_commit_max`
`transition_to_normal_from_view_change_status`
`view_durable_update`
`send_prepare_oks_after_view_change`
`commit_journal`

Mechanics:

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection (where, how) |
| --- | --- | --- | --- | --- | --- |
| Detection | `replica.zig:3608` (prepare\_timeout 250 ms, :1366-1370) / `:2429-2431` + `fault_detector.zig` (EWMA, bounds :1381-1388) | primary missing acks re-sends; backups signal exit\_view when the commit/prepare flow stops | none | EWMA `interval_min=100ms`/`interval_max=2000ms` | liveness only |
| exit\_view quorum | `replica.zig:2549-2592` | bitset `exit_view_from_all_replicas`; at `quorum_view_change` the view is bumped | **Flush gate:** nothing is advertised until the superblock write below completes | `quorum_view_change = N − R + 1`, forced 2 for N=2 (`vsr.zig:1194-1195`; table `vsr.zig:1240`) | n/a |
| Durable view evidence | `replica.zig:10203-10210` → `view_durable_update` :9448-9513 → `superblock.zig:1010-1030` (+ `write_staging` :1088, `write_header` :1124, verify :1264) | superblock gains `view`, `log_view`, `commit_max`, JV header suffix before any JV is sent ("cannot advertise until certain they will never backtrack", `superblock.zig:1003-1004`) | **Flush — per view change entry**, superblock on 4 copies, read-verified | `view_change_headers_suffix_max` = 8+1 headers (`config.zig:156` → `constants.zig:284`); `superblock_copies`=4 | verify-read panic on mismatch (`superblock.zig:1276-1281`) |
| join\_view fanout | `replica.zig:8783-8837` (sent at :10251) | each replica sends its log suffix + present/nack bitsets + `commit_min` (`message_header.zig:1137-1163`) | none | suffix ≤ `view_change_headers_suffix_max`=9 headers | nack bit built from journal dirty/faulty state :8813-8837 |
| JV quorum | `replica.zig:2608-2636` | `JVQuorum.quorum_headers` classifies `awaiting_quorum` / `awaiting_repair` / `complete_invalid` / `complete_valid` | none | `quorum_view_change` JVs; nack decisions use `quorum_nack_prepare = N − R + 1` (`vsr.zig:1207`) | a possibly-committed (faulty) slot is never nacked (`:8856`-adjacent rule: nacks require `!faulty`) |
| Log selection / forfeit | `replica.zig:2677-2730` | new log = quorum's canonical headers; `commit_max` = max over quorum; a lagging candidate forfeits to a more up-to-date primary | none | checkpoint-distance heuristic | n/a |
| Truncation | `set_op_and_commit_max` `replica.zig:9619-9668` → `journal.remove_entries_from` (`journal.zig:1705`) | uncommitted suffix removed in memory+headers; committed ops never truncated (assert :9635-9636) | none (in-memory + header rewrite; durability comes from the next superblock `view_change`) | truncation bounded to the uncommitted window `≤ pipeline_prepare_queue_max` | n/a |
| Repair | `on_get_prepare` `replica.zig:3056-3125`, `on_get_headers` :3159 | underwriter rule: serve only guaranteed prepares; nack only ops *provably* never durably prepared | reads, not writes | per-op | missing/faulty slots are not served (:3073-3077, :3118-3124) |
| View install | `primary_send_view` `replica.zig:9515-9531`; backups `on_view` :2759 → `on_view_set_journal` :2913-2996 → `transition_to_normal_from_view_change_status` :2994 | backups replace their log with the View suffix, durably record `view=log_view` :10123-10125, then flood stale prepare\_oks :2995 | **Flush — superblock `view_change` per backup** (:10125) | View suffix ≤ 9 headers | stale Views rejected by view/op/nonce freshness (`:2763-2777` recovering\_head) |
| Restart fence | `journal.zig:1299-1332` | headers outside the durable `log_view` range are dropped on restart — truncation survives a crash only because log\_view was flushed | restart-time read | whole WAL (1024 slots) | view-range membership per header (:1319-1330) |

`replica.zig:3608`
`:2429-2431`
`fault_detector.zig`
`interval_min=100ms`
`interval_max=2000ms`
`replica.zig:2549-2592`
`exit_view_from_all_replicas`
`quorum_view_change`
`quorum_view_change = N − R + 1`
`vsr.zig:1194-1195`
`vsr.zig:1240`
`replica.zig:10203-10210`
`view_durable_update`
`superblock.zig:1010-1030`
`write_staging`
`write_header`
`view`
`log_view`
`commit_max`
`superblock.zig:1003-1004`
`view_change_headers_suffix_max`
`config.zig:156`
`constants.zig:284`
`superblock_copies`
`superblock.zig:1276-1281`
`replica.zig:8783-8837`
`commit_min`
`message_header.zig:1137-1163`
`view_change_headers_suffix_max`
`replica.zig:2608-2636`
`JVQuorum.quorum_headers`
`awaiting_quorum`
`awaiting_repair`
`complete_invalid`
`complete_valid`
`quorum_view_change`
`quorum_nack_prepare = N − R + 1`
`vsr.zig:1207`
`:8856`
`!faulty`
`replica.zig:2677-2730`
`commit_max`
`set_op_and_commit_max`
`replica.zig:9619-9668`
`journal.remove_entries_from`
`journal.zig:1705`
`view_change`
`≤ pipeline_prepare_queue_max`
`on_get_prepare`
`replica.zig:3056-3125`
`on_get_headers`
`primary_send_view`
`replica.zig:9515-9531`
`on_view`
`on_view_set_journal`
`transition_to_normal_from_view_change_status`
`view=log_view`
`view_change`
`:2763-2777`
`journal.zig:1299-1332`
`log_view`

Callouts:

`exit_view`
`view_durable_update`
`replica.zig:10209`
`:10251`
`superblock.zig:1003-1004`
`replica.zig:9390-9433`
`replica.zig:10209`
`:10125`
`view_change_headers_suffix_max`
`config.zig:156`
`constants.zig:284`
`op − commit_max ≤ pipeline_prepare_queue_max`
`replica.zig:2792-2793`
`replica.zig:1394-1413`
`quorum_view_change`
`.view_change`
`quorum_view_change`
`quorum_nack_prepare`
`quorum_view_change + quorum_replication > N`
`vsr.zig:1203`
`log_view`
`journal.zig:1299-1332`
`log_view`
`recovering_head`
`replica.zig:2763-2777`
`replica.zig:9632-9638`
`!faulty`
`replica.zig:8813-8837`

Test evidence (`src/vsr/replica_test.zig` unless noted):

`src/vsr/replica_test.zig`
`replica_test.zig:601-649`
`on_join_view: lagging primary; forfeiting`
`replica_test.zig:846-879`
`awaiting_repair`
`replica_test.zig:881-900`
`quorum received, deadlocked`
`replica_test.zig:902-951`
`replica_test.zig:1066-1119`
`replica_test.zig:1787+`
`replica_test.zig:251-328`
`recovering_head`
`journal.zig:2507-2541`
`test "recovery_cases"`
`src/vsr/journal.zig`
`journal_test.zig`
`replica_test.zig:184-204`

## 6. WAL format: two rings, 16-case recovery

`journal.zig:16-26`
`wal_headers`
`journal_size_headers`
`constants.zig:197`
`journal_slot_count * @sizeOf(vsr.Header)`
`wal_prepares`
`journal_size_prepares`
`constants.zig:198`
`journal_slot_count * message_size_max`
`journal_slot_count`
`config.zig:158`
`constants.zig:189`
`journal.zig:92`
`op % 1024`
`slot_for_op`
`journal.zig:396-398`
`journal.zig:2144-2145`
`headers_per_sector`
`journal.zig:47`
`sector_size`
`constants.zig:489`
`vsr`
`fix`
`cut`
`nil`
`eql`
`journal.zig:1204-1241`
`recover_slots`
`journal_iops_write_max`
`torn_prepares`
`BoundedArrayType(Slot, journal_iops_write_max)`
`journal.zig:1359-1362`
`journal.zig:1469-1478`
`constants.zig:44-46`
`constants.zig:69-70`
`op_prepare_max`
`on_prepare`
`replica.zig:2157-2178`
`test "recovery_cases"`
`journal.zig:2507-2541`
`src/vsr/replica_test.zig`

## 7. Quorums

`src/vsr.zig:1173-1234`: flexible quorums (Flexible-Paxos style) —

`src/vsr.zig:1173-1234`
`quorum_replication = min(3, ⌈N/2⌉)`
`vsr.zig:1185-1188`
`quorum_replication_max = 3`
`config.zig:157`
`quorum_view_change = N − R + 1`
`vsr.zig:1194-1195`
`quorum_nack_prepare = N − R + 1`
`vsr.zig:1207`
`quorum_view_change + quorum_replication > replica_count`
`vsr.zig:1203`

N=3 → 2/2, N=4 → 2/3, N=5 → 3/3, N=6 → 3/4 (table test `vsr.zig:1239-1243`). Max 6 replicas + 6
standbys (`replicas_max`/`standbys_max`/`members_max`, `constants.zig:24-28`); standbys replicate
(and journal) but never vote (`replica.zig:8672` in `send_prepare_ok`; `standby()` at
`replica.zig:6925-6928`).

`vsr.zig:1239-1243`
`replicas_max`
`standbys_max`
`members_max`
`constants.zig:24-28`
`replica.zig:8672`
`send_prepare_ok`
`standby()`
`replica.zig:6925-6928`

## 8. The complete flush catalog (what flushes, when)

| # | Event | Flush | Blocking? | Evidence |
| --- | --- | --- | --- | --- |
| 1 | prepare received/sent (every replica, every op) | prepare body → `wal_prepares` (O\_DIRECT/O\_DSYNC) | **Yes** — gates prepare\_ok | `journal.zig:1842-1851` |
| 2 | after #1 completes | redundant header → `wal_headers` | **Yes** — also gates prepare\_ok | `journal.zig:1890-1908`, `replica.zig:11219-11227` |
| 3 | commit | *none* | — | `replica.zig:11370-11380` |
| 4 | every 960 commits | checkpoint: LSM/state, client replies, client sessions, grid, then superblock (all copies, quorum-verified) | checkpoint pipeline; amortized | `constants.zig:40-42`, `superblock.zig:891-987` |
| 5 | entering a view change | superblock: `view`, `log_view`, `commit_max`, JV/View header suffix — **before the view is advertised** | **Yes** — gates participation | `replica.zig:10203-10210`, `:9448-9513` |
| 6 | completing a view change (assuming/receiving View) | superblock again | **Yes** | `replica.zig:10125` |
| 7 | replica restart | solo: superblock `view`/`log_view` increment, durably written before serving (`replica.zig:876-885`); non-solo: durable view write **only** when the recovering replica is the next primary (`:894` → `:10209`; otherwise the backup branch `:883-901` writes no view) | **Yes** (solo and restarting-primary) | `replica.zig:876-885`, `:894` → `:10209`, `:883-901` |
| 8 | client replies | written asynchronously to `client_replies` zone, durable at each checkpoint | No | `src/vsr/client_replies.zig:11-14` |

`wal_prepares`
`journal.zig:1842-1851`
`wal_headers`
`journal.zig:1890-1908`
`replica.zig:11219-11227`
`replica.zig:11370-11380`
`constants.zig:40-42`
`superblock.zig:891-987`
`view`
`log_view`
`commit_max`
`replica.zig:10203-10210`
`:9448-9513`
`replica.zig:10125`
`view`
`log_view`
`replica.zig:876-885`
`:894`
`:10209`
`:883-901`
`replica.zig:876-885`
`:894`
`:10209`
`:883-901`
`client_replies`
`src/vsr/client_replies.zig:11-14`

AOF is optional; its mechanics are in the appendix of the on-disk report.

Amortization: one op is a batch of up to 1 MiB (~8,190 transfers), the pipeline allows 8 prepares
in flight and 32 concurrent WAL writes (`config.zig:117,155,159`), so the per-transfer sync cost
is tiny; commit is free; the superblock write is once per 960 ops (`constants.zig:40-42`).

`config.zig:117,155,159`
`constants.zig:40-42`

## 9. External literature

External literature: VSR-1988 (Oki & Liskov), VRR-2012 (Liskov & Cowling, MIT-CSAIL-TR-2012-021),
and the extended TR UW-CSE-17-08-01 (Michael, Ports, Sharma, Szekeres, DISC'17). Statements here
quote the cited papers; they are not verifiable from the 0.17.9 source tree, whose own facts are
established in §0-§8.

`cur-viewid`

**TigerBeetle (the code-tree facts, §0-§8):** it flushes per-prepare, per-replica, in the
foreground *and* persists durable view evidence before advertisement: superblock
`view`/`log_view` ("it cannot advertise either value until it is certain they will never
backtrack", `superblock.zig:1003-1004`; anti-fork proof at `replica.zig:9390-9433`).

`view`
`log_view`
`superblock.zig:1003-1004`
`replica.zig:9390-9433`

## 10. Constants

| constant | file:line | value | meaning |
| --- | --- | --- | --- |
| `batch_max` (transfers/op) | `src/state_machine.zig:336-358` (derived); `tigerbeetle.zig:113` (`assert(@sizeOf(Transfer) == 128)`) | **8,190** | derived: `(message_size_max − 256) / 128`; stated in `constants.zig:640` comment ("Each prepare can contain at most 8\_190 transfers") |
| `message_size_max` | `config.zig:159` → `constants.zig:234` | `1 * MiB` (1,048,576) | max message/WAL-slot size |
| `message_body_size_max` | `constants.zig:235` | `message_size_max - @sizeOf(vsr.Header)` = 1,048,320 | body budget for batch\_max |
| journal slots | `config.zig:158` → `constants.zig:189` | `journal_slot_count: u32 = 1024` | WAL ring slots; slot = `op % 1024` (`journal.zig:396-398`) |
| `journal_size_headers` | `constants.zig:197` | `journal_slot_count * @sizeOf(vsr.Header)` = 1024 × 256 B | `wal_headers` ring |
| `journal_size_prepares` | `constants.zig:198` | `journal_slot_count * message_size_max` = 1024 × 1 MiB | `wal_prepares` ring |
| header size | `message_header.zig:72` | `assert(@sizeOf(Header) == 256)` | wire/WAL header bytes |
| sector size | `constants.zig:489` | `pub const sector_size = 4096;` | direct-I/O / WAL sector granularity; 16 headers per sector (`journal.zig:47`) |
| checkpoint interval | `constants.zig:40-42` (derived) | **960 ops** = 1024 − 32 − 32·⌈16/32⌉ (production: `config.zig:158,155,164`) | commits between superblock checkpoints |
| commit heartbeat | `replica.zig:1376-1380` | `.after = 500 / constants.tick_ms` = 500 ms (`config.zig:120` `tick_ms: u63 = 10`) | idle-commit broadcast / liveness |
| prepare timeout | `replica.zig:1366-1370` | `250 / constants.tick_ms` = 250 ms | primary re-send window |
| primary abdication timeout | `replica.zig:1371-1375` | `10_000 / constants.tick_ms` = 10 s | clock-not-synced / abdication |
| fault detector EWMA bounds | `replica.zig:1381-1388` | `interval_min = .ms(100)`, `interval_max = .ms(2_000)` | primary-fault detector |
| replica count max | `constants.zig:24-28` | `replicas_max = 6`, `standbys_max = 6`, `members_max = 12` | cluster membership caps (no default *replica\_count*; it is a CLI/runtime arg, `quorum_replication_max = 3` at `config.zig:157`) |
| quorum\_replication | `vsr.zig:1185-1188` | `min(3, ⌈N/2⌉)`, forced 2 for N=2 | replication quorum |
| quorum\_view\_change | `vsr.zig:1194-1195` | `N − R + 1` (forced 2 for N=2) | view-change quorum |
| quorum\_nack\_prepare | `vsr.zig:1207` | `N − R + 1` | nack quorum |
| superblock copies | `config.zig:160` → `constants.zig:553` | `superblock_copies: comptime_int = 4` | superblock write fanout; read quorums 2/4 or 3/4 per sequence (`superblock.zig:631-651`) |
| `pipeline_prepare_queue_max` | `config.zig:155` → `constants.zig:255` | 8 | prepares in flight per primary |
| `journal_iops_write_max` | `config.zig:117` → `constants.zig:516` | 32 | concurrent WAL writes (also torn-write budget, `journal.zig:1362`) |
| `lsm_compaction_ops` | `config.zig:164` → `constants.zig:647` | 32 | checkpoint-interval derivation input |
| `view_change_headers_suffix_max` | `config.zig:156` → `constants.zig:284` | 8 + 1 | headers in JV/View suffix |
| `clients_max` (default) | `config.zig:228` | 64 | default client table size (production config) |
| `block_size` | `config.zig:161` | `512 * KiB` | grid block size |

`batch_max`
`src/state_machine.zig:336-358`
`tigerbeetle.zig:113`
`assert(@sizeOf(Transfer) == 128)`
`(message_size_max − 256) / 128`
`constants.zig:640`
`message_size_max`
`config.zig:159`
`constants.zig:234`
`1 * MiB`
`message_body_size_max`
`constants.zig:235`
`message_size_max - @sizeOf(vsr.Header)`
`config.zig:158`
`constants.zig:189`
`journal_slot_count: u32 = 1024`
`op % 1024`
`journal.zig:396-398`
`journal_size_headers`
`constants.zig:197`
`journal_slot_count * @sizeOf(vsr.Header)`
`wal_headers`
`journal_size_prepares`
`constants.zig:198`
`journal_slot_count * message_size_max`
`wal_prepares`
`message_header.zig:72`
`assert(@sizeOf(Header) == 256)`
`constants.zig:489`
`pub const sector_size = 4096;`
`journal.zig:47`
`constants.zig:40-42`
`config.zig:158,155,164`
`replica.zig:1376-1380`
`.after = 500 / constants.tick_ms`
`config.zig:120`
`tick_ms: u63 = 10`
`replica.zig:1366-1370`
`250 / constants.tick_ms`
`replica.zig:1371-1375`
`10_000 / constants.tick_ms`
`replica.zig:1381-1388`
`interval_min = .ms(100)`
`interval_max = .ms(2_000)`
`constants.zig:24-28`
`replicas_max = 6`
`standbys_max = 6`
`members_max = 12`
`quorum_replication_max = 3`
`config.zig:157`
`vsr.zig:1185-1188`
`min(3, ⌈N/2⌉)`
`vsr.zig:1194-1195`
`N − R + 1`
`vsr.zig:1207`
`N − R + 1`
`config.zig:160`
`constants.zig:553`
`superblock_copies: comptime_int = 4`
`superblock.zig:631-651`
`pipeline_prepare_queue_max`
`config.zig:155`
`constants.zig:255`
`journal_iops_write_max`
`config.zig:117`
`constants.zig:516`
`journal.zig:1362`
`lsm_compaction_ops`
`config.zig:164`
`constants.zig:647`
`view_change_headers_suffix_max`
`config.zig:156`
`constants.zig:284`
`clients_max`
`config.zig:228`
`block_size`
`config.zig:161`
`512 * KiB`

## 11. Classification

**TigerBeetle 0.17.9 = VRR-2012's message family (request/prepare/prepare\_ok/commit/reply +
exit\_view/join\_view/view, same commit-piggyback mechanism, same DoViewChange selection rule
paraphrased at `replica.zig:2598-2608`) + a durability placement of its own:
every replica makes every prepare stable (O\_DIRECT|O\_DSYNC / F\_FULLFSYNC) *before* acking, commit
is flush-free and derived, views/log\_views are durably fenced in a quorum-verified multi-copy
superblock before advertisement, checkpoints every 960 ops fence WAL wrap, and there is no
quorum-recovery protocol.** The only
volatile component on the consensus critical path is the commit watermark itself, whose safety
rests on the durable-prepare quorum intersection argument (`vsr.zig:1203`), not on a durable
commit record.

`replica.zig:2598-2608`
`vsr.zig:1203`

# TigerBeetle 0.17.9 — Recovery: the mechanics of restart, durability, and repair

Companion to `report-0.17.9-protocol.md`.

`report-0.17.9-protocol.md`

## 1. The recovery model in one sentence

**Recovery = read your own disk; the network is for repair, never for state synthesis.**

A restarted replica opens its own superblock (`Replica.open` → `SuperBlock.open`,
`src/vsr/replica.zig:654`, `:688` → `src/vsr/superblock.zig:854`), replays its own journal
(`src/vsr/replica.zig:767` → `Journal.recover`, `src/vsr/journal.zig:941`), and only then
participates. It never broadcasts a "what view am I in?" query to a quorum. The nearest message,
`get_view`, is a *unicast* to one peer — the current view's primary, issued from `repair()`
(`src/vsr/replica.zig:7596-7605`; `src/vsr.zig:250`; `docs/internals/vsr.md:159-178`).

`Replica.open`
`SuperBlock.open`
`src/vsr/replica.zig:654`
`:688`
`src/vsr/superblock.zig:854`
`src/vsr/replica.zig:767`
`Journal.recover`
`src/vsr/journal.zig:941`
`get_view`
`repair()`
`src/vsr/replica.zig:7596-7605`
`src/vsr.zig:250`
`docs/internals/vsr.md:159-178`

There is no RECOVERY protocol in the code: `docs/internals/vsr.md:53-57` — "Unlike [VRR],
TigerBeetle does not implement Recovery Protocol (see §4.3). Instead, replicas persist their VSR
state to the superblock. This ensures that a recovering replica never backtracks to an older
view." The wire confirms it: `Command` (`src/vsr.zig:230-276`) contains no recovery command, and
the old recovery-flavored commands were removed with their ordinals retired
(`deprecated_21 = 21, // .request_sync_checkpoint`, `deprecated_22 = 22, // .sync_checkpoint`).
So the failure UW-CSE-17-08-01 Appendix B.1 (external literature)
identifies for quorum-recovery protocols — a replica *re-learning* a stale view from a quorum of
peers and forgetting a view-change promise it already made — has no message with which to happen.
The `epoch` field that protocol depends on is wire-rejected when nonzero:
`src/vsr/message_header.zig:174` — `if (self.epoch != 0) return "epoch != 0";` inside
`Header.invalid`.

`docs/internals/vsr.md:53-57`
`Command`
`src/vsr.zig:230-276`
`deprecated_21 = 21, // .request_sync_checkpoint`
`deprecated_22 = 22, // .sync_checkpoint`
`epoch`
`src/vsr/message_header.zig:174`
`if (self.epoch != 0) return "epoch != 0";`
`Header.invalid`

## 2. Why an acknowledged request cannot be lost

This is a chain of code facts, each mechanically enforced:

`src/vsr/replica.zig:5536`
`self.send_reply_message_to_client(reply);`
`quorum_replication`
`src/vsr/replica.zig:2293-2294`
`const threshold = self.quorum_replication;`
`:2307-2311`
`count_message_and_receive_quorum_exactly_once(&prepare.ok_from_all_replicas, message, threshold)`
`Replica.write_prepare_callback`
`send_prepare_ok`
`src/vsr/replica.zig:11219-11227`
`journal.zig:1851`
`journal.zig:1908`
`journal.dirty.clear(slot); journal.faulty.clear(slot);`
`journal.zig:1953-1954`
`has_prepare`
`journal.zig:540-548`
`src/vsr.zig:1203`
`assert(quorum_view_change + quorum_replication > replica_count);`
`pub fn quorums(replica_count: u8)`
`quorum_nack_prepare = replica_count - quorum_replication + 1`
`src/vsr.zig:1207`
`quorum_replication`
`:1208`
`quorum_replication`
`:1205-1206`
`replica.zig:8818`
`dirty and !faulty`
`replica.zig:8826-8829`
`replica.zig:8831-8835`
`replica.zig:8803-8858`
`docs/internals/vsr.md:218`
`pipeline_prepare_queue_max`
`journal.zig:2264-2270`

A nack quorum and a replication quorum cannot both form for the same op. Consequences:

## 3. Restart: superblock open → state machine resume

Trigger: a replica process starts (`Replica.open`, `replica.zig:654`). Everything is read from
its own disk before any network participation. Restart ordering is strict: superblock open
completes before `journal.recover` is called (`replica.zig:688` vs `:767` — both spin
`storage.run()` until `opened`, `:689`/`:768`), and journal recovery completes before any status
transition. No message is sent inside `open()`.

`Replica.open`
`replica.zig:654`
`journal.recover`
`replica.zig:688`
`:767`
`storage.run()`
`opened`
`:689`
`:768`
`open()`

### Call hierarchy

`Replica.open (replica.zig:654)
├─ SuperBlock.init (replica.zig:677)
├─ SuperBlock.open (replica.zig:688 → superblock.zig:854)
│ ├─ SuperBlock.acquire (superblock.zig:1431) — caller=.open → read_working(.open) (superblock.zig:1460-1461)
│ │ ├─ SuperBlock.read_header ×4 copies, sequential (superblock.zig:1220-1245)
│ │ │ └─ Storage.read_sectors .superblock (superblock.zig:1238)
│ │ └─ read_header_callback (superblock.zig:1247)
│ │ ├─ Quorums.working(.open) (superblock.zig:1264 → superblock_quorums.zig:89)
│ │ │ ├─ count_copy per copy (superblock_quorums.zig:199-258)
│ │ │ └─ parent-hash chain + VSRState monotonic checks (superblock_quorums.zig:172-186)
│ │ ├─ SuperBlock.repair (superblock.zig:1415) — one broken copy at a time
│ │ │ └─ SuperBlock.write_header (superblock.zig:1124)
│ │ └─ SuperBlock.read_working(.verify) (superblock.zig:1193) — 3-of-4 read-back verify
│ └─ SuperBlock.release (superblock.zig:1468) — opened=true (superblock.zig:1486)
├─ (replica opens) Replica.superblock_open_callback (replica.zig:935)
├─ Replica.init — view/log_view/commit_max seeded from working.vsr_state (replica.zig:1347-1351)
├─ Journal.recover (replica.zig:767 → journal.zig:941) [detail: §4]
│ └─ Journal.recover_done (journal.zig:1665) → Replica.journal_recover_callback (replica.zig:943)
├─ all-slots-faulty abort (replica.zig:771: `error.WALInvalid`)
├─ op_head derivation (replica.zig:790-856)
│ ├─ log_view==view: max op over journal.headers (replica.zig:792-801)
│ ├─ else: head from durable JV view_headers (replica.zig:827-838)
│ └─ no usable headers: head = journal.op_maximum() (replica.zig:842-855)
├─ self.op / self.commit_max seed (replica.zig:863-867)
├─ state machine resume (replica.zig:872-907)
│ ├─ solo: view bump + view_durable_update → normal (replica.zig:872-889) [detail: §7]
│ ├─ primary of current view: transition_to_view_change_status(view+1) (replica.zig:893-894)
│ ├─ backup, head certain: transition_to_normal_from_recovering_status (replica.zig:895-896)
│ ├─ head not locally certain: transition_to_recovering_head_from_recovering_status (replica.zig:899)
│ └─ durable JV (log_view<view): transition_to_view_change_status(view) (replica.zig:901-905)
├─ pending state sync resume (replica.zig:914-926)
└─ Grid.open (replica.zig:930)`

### Sequence

### Mechanics table

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection |
| --- | --- | --- | --- | --- | --- |
| 1 | replica.zig:688 → superblock.zig:854 | `Replica.open` calls `SuperBlock.open`; context caller=`.open` | none (read phase) | 4 copies (`superblock_copies=4`, config.zig:160; asserted {4..8}, constants.zig:556-558) | copy checksum validated in `count_copy` (superblock\_quorums.zig:208) |
| 2 | superblock.zig:1200-1245 | `read_working(.open)` reads each copy's header sequentially (deliberately not parallel — temporal-locality fault risk, :1209-1212) | none | 4 copies, read 1 at a time | per-copy checksum at quorum counting (superblock\_quorums.zig:208-211) |
| 3 | superblock.zig:1264 → superblock\_quorums.zig:89-197 | group copies by (checksum,sequence), sort by validity/sequence/count, pick highest | none | open threshold `2` of 4 (superblock\_quorums.zig:69-74; write+read = copies+1, :57) | same-sequence different-checksum ⇒ `error.Fork` (:169); sequence gap ⇒ `ParentSkipped` (:175) |
| 4 | superblock\_quorums.zig:178-192 | parent hash chaining: older quorum's checksum must equal `b.header.parent`; VSRState must be monotonic | none | — | mismatch ⇒ `ParentNotConnected` (:184) / `VSRStateNotMonotonic` (:186) |
| 5 | superblock.zig:1389-1400, 1415-1429 | repair broken copies: staging←working, rewrite each missing/misplaced copy in `RepairIterator` priority order (a-then-b-then-c, superblock\_quorums.zig:344-366) | per-copy `write_header` (superblock.zig:1166-1172), triggered by open's repair pass | ≤4 copies rewritten, one at a time; never to a foreign slot (:324-326) | torn repair-write guard: documented backtrack scenario superblock\_quorums.zig:299-316 |
| 6 | superblock.zig:1191-1193, 1264, 1276-1281 | after repair writes, all copies re-read (`read_working(.verify)`) and matched against staging checksum | post-write verify read; trigger: every superblock write batch (open-repair, checkpoint, view\_change) | verify threshold `3` of 4 (superblock\_quorums.zig:61-66) | mismatch ⇒ `@panic("superblock failed verification after writing")` (superblock.zig:1278) |
| 7 | superblock.zig:1484-1491 | `opened=true`; replica index derived from members | — | — | — |
| 8 | replica.zig:767 → journal.zig:941 | `Journal.recover`: status→`.recovering`, read all redundant-header chunks | none | header chunks = `headers_size/message_size_max` chunks (journal.zig:989-990); reads bounded by `journal_iops_read_max` (constants.zig:513) | none yet (raw copy, unvalidated — journal.zig:1037-1047) |
| 9 | journal.zig:1076-1168 | `recover_prepares`: read every slot of `.wal_prepares`, whole `message_size_max` per slot | none | one read per slot; slot\_count = `journal_slot_count=1024` (config.zig:158, constants.zig:189) | header + body checksum + zero padding (journal.zig:1151-1158) — see §5 |
| 10 | journal.zig:1240-1341 | `recover_slots`: classify each slot against the 16-case table, refine torn, apply decisions, drop out-of-log\_view headers | none (in-memory) | 1 case per slot × slot\_count | torn refinement (journal.zig:1278-1287) |
| 11 | journal.zig:1609-1655 | `recover_fix`: rewrite `.wal_headers` sectors for `fix` slots | per-sector write (journal.zig:1649-1654), one sector at a time, triggered by dirty&clean-faulty fix slots | `headers_per_sector = sector_size/@sizeOf(Header)` (journal.zig:47) | — |
| 12 | journal.zig:1665-1701 | `recover_done`: assert all in-memory headers valid + consistent; status→`.recovered` | — | — | assertion sweep journal.zig:1686-1699 |
| 13 | replica.zig:771 | abort if every slot faulty (`error.WALInvalid`) | — | all `journal_slot_count` slots | — |
| 14 | replica.zig:792-856 | op\_head from WAL (log\_view==view) or from durable JV `view_headers` (log\_view<view) | — | JV array: `view_headers.slice` (superblock working.vsr\_state) | header checksum/slot via `jv_header_type` (replica.zig:828) |
| 15 | replica.zig:863-867 | `self.op = op_head`; `commit_max = max(commit_max, op - pipeline_prepare_queue_max)`; seeds from superblock at replica.zig:1347-1351 | — | `pipeline_prepare_queue_max=8` (config.zig:155) | — |
| 16 | replica.zig:872-907 | resume: solo / primary / backup / recovering\_head / durable-JV branches (see §7) | solo: superblock write via `view_durable_update` (replica.zig:885) | solo: 1 replica; multi: transitions gated on `op_head_certain()` (replica.zig:892) | solo: any faulty slot ⇒ `error.WALCorrupt` (replica.zig:873) |

`Replica.open`
`SuperBlock.open`
`.open`
`superblock_copies=4`
`count_copy`
`read_working(.open)`
`2`
`error.Fork`
`ParentSkipped`
`b.header.parent`
`ParentNotConnected`
`VSRStateNotMonotonic`
`RepairIterator`
`write_header`
`read_working(.verify)`
`3`
`@panic("superblock failed verification after writing")`
`opened=true`
`Journal.recover`
`.recovering`
`headers_size/message_size_max`
`journal_iops_read_max`
`recover_prepares`
`.wal_prepares`
`message_size_max`
`journal_slot_count=1024`
`recover_slots`
`recover_fix`
`.wal_headers`
`fix`
`headers_per_sector = sector_size/@sizeOf(Header)`
`recover_done`
`.recovered`
`error.WALInvalid`
`journal_slot_count`
`view_headers`
`view_headers.slice`
`jv_header_type`
`self.op = op_head`
`commit_max = max(commit_max, op - pipeline_prepare_queue_max)`
`pipeline_prepare_queue_max=8`
`view_durable_update`
`op_head_certain()`
`error.WALCorrupt`

### Callouts

`write_header`
`checkpoint()`
`view_change()`
`vsr_checkpoint_ops = journal_slot_count - lsm_compaction_ops - lsm_compaction_ops * div_ceil(2*pipeline_prepare_queue_max, lsm_compaction_ops)`
`view_durable_update`
`superblock.view_change`
`transition_to_view_change_status(self.view + 1)`
`normal`
`.recovering_head`
`op_prepare_max()`
`get_view`
`repair()`

### Test evidence

`src/vsr/replica_test.zig:75-92`
`.recovering_head`
`src/vsr/replica_test.zig:94-111`
`.view_change`
`src/vsr/replica_test.zig:154-169`
`open()`
`error.WALCorrupt`
`src/vsr/superblock_quorums_fuzz.zig:20-98`
`ParentNotConnected`
`Fork`
`ParentSkipped`
`VSRStateNotMonotonic`
`src/unit_tests.zig:69`
`src/vsr/superblock_quorums_fuzz.zig:100-112`
`fuzz_quorum_repairs`

## 4. Journal recovery: read, classify, decide, repair

Trigger: `Journal.recover` at restart (`replica.zig:767`). Every case below is read out of the
table literal `recovery_cases` (`journal.zig:2303-2345`) and applied in `recover_slot`
(`journal.zig:1485-1606`). The WAL is two contiguous circular buffers on disk,
`vsr.Zone.wal_headers` and `vsr.Zone.wal_prepares` (`journal.zig:16-18`; zone enum
`src/vsr.zig:125-135`) — named `Ring.prepares` (messages, slot = op % `journal_slot_count`) and
`Ring.headers` (redundant headers, `headers_redundant`) in code (`journal.zig:22-45`).

`Journal.recover`
`replica.zig:767`
`recovery_cases`
`journal.zig:2303-2345`
`recover_slot`
`journal.zig:1485-1606`
`vsr.Zone.wal_headers`
`vsr.Zone.wal_prepares`
`journal.zig:16-18`
`src/vsr.zig:125-135`
`Ring.prepares`
`journal_slot_count`
`Ring.headers`
`headers_redundant`
`journal.zig:22-45`

### Call hierarchy

`Journal.recover (journal.zig:941)
├─ recover_headers (journal.zig:960) — chunked reads of .wal_headers
│ └─ recover_headers_callback (journal.zig:1013)
│ └─ copy raw chunk into journal.headers_redundant, unvalidated (journal.zig:1040-1047)
├─ recover_prepares (journal.zig:1076) — entered when header chunks complete (journal.zig:968-971)
│ ├─ recover_prepare (journal.zig:1092) — whole-slot read of .wal_prepares (journal.zig:1124-1132)
│ └─ recover_prepare_callback (journal.zig:1135)
│ └─ valid ⇒ journal.headers[slot] = prepare's own header (journal.zig:1159)
├─ recover_slots (journal.zig:1240)
│ ├─ header_ok per ring: headers_redundant (redundant header) and headers (prepare header) (journal.zig:1255-1256)
│ ├─ recovery_case (journal.zig:1258-1265 → 2431-2477) — 11 boolean parameters per slot
│ ├─ torn_prepares refinement ⇒ case_cut_torn (journal.zig:1278-1287 → 1359-1483)
│ ├─ recover_slot per slot (journal.zig:1289 → 1485)
│ ├─ headers_redundant ← headers sync (journal.zig:1292-1297)
│ └─ drop headers outside durable log_view's view range (journal.zig:1313-1332)
├─ recover_fix (journal.zig:1609) — write repaired .headers sectors (journal.zig:1649-1654)
└─ recover_done (journal.zig:1665) → Replica.journal_recover_callback (replica.zig:943)`

### Sequence

### Mechanics table — read/copy/flush pipeline

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection |
| --- | --- | --- | --- | --- | --- |
| recover\_headers | journal.zig:960-1011 | read `.wal_headers` in `message_size_max`-sized chunks | none | chunks = headers\_size/message\_size\_max (offset math journal.zig:989-990) | none yet — copied raw (journal.zig:1040-1047) |
| recover\_prepares | journal.zig:1076-1090 | read every slot of `.wal_prepares` concurrently | none | reads limited by IOPS pool; slot = op % slot\_count (journal.zig:63) | deferred to callback |
| recover\_prepare\_callback | journal.zig:1135-1168 | validate header checksum, body checksum, zero sector padding; on pass, the prepares ring's own header becomes the recovery candidate in `journal.headers[slot]` | none | whole slot read: `message.buffer[0..constants.message_size_max]` (journal.zig:1129) | checksum (journal.zig:1151-1152) + `stdx.zeroed(message_padding)` (journal.zig:1158) |
| recover\_slots classify | journal.zig:1253-1265 | `header_ok` on `headers_redundant` (redundant header) and `headers` (prepare header); `recovery_case` with op\_prepare\_max / op\_checkpoint / untrusted op\_max | none | 11-parameter pattern per slot (journal.zig:2393, pattern\_size=11) | `header_ok` rejects wrong cluster/slot = misdirect evidence (journal.zig:2495-2501) |
| recover\_slots torn refinement | journal.zig:1278-1287 | @B/@C-class slots inside (op\_max, op\_prepare\_max] with valid stale redundant header + invalid prepare ⇒ `case_cut_torn` | none (decision only) | torn slot list bounded by `journal_iops_write_max=32` (journal.zig:1408) | see §5 |
| recover\_slot apply | journal.zig:1485-1606 | per-decision slot effects (table below) | vsr/fix leave dirty+faulty/dirty set for later repair; nothing else writes | — | faulty slots forced to invalid redundant header (journal.zig:1572-1577) |
| recover\_fix | journal.zig:1609-1655 | rewrite the `.wal_headers` sector holding repaired `fix` headers, from `headers_redundant` via `header_sector` | per-sector write, one at a time, until `dirty`∩clean-of-faulty is empty (journal.zig:1616-1639) | exactly one sector (4 KiB, `headers_per_sector` headers) per write (journal.zig:1630, 1653) | — |
| drop out-of-view headers | journal.zig:1313-1332 | valid non-reserved header whose view ∉ view\_headers.view\_for\_op(op, log\_view) is removed (`remove_entry`) — fork prevention | none | — | view-range check (journal.zig:1316-1319) |
| recover\_done | journal.zig:1665-1701 | assert headers==headers\_redundant, all valid, slot-consistent; callback | — | all `journal_slot_count` slots asserted | assertion sweep |

`.wal_headers`
`message_size_max`
`.wal_prepares`
`journal.headers[slot]`
`message.buffer[0..constants.message_size_max]`
`stdx.zeroed(message_padding)`
`header_ok`
`headers_redundant`
`headers`
`recovery_case`
`header_ok`
`case_cut_torn`
`journal_iops_write_max=32`
`.wal_headers`
`fix`
`headers_redundant`
`header_sector`
`dirty`
`headers_per_sector`
`remove_entry`
`journal_slot_count`

### Mechanics table — the 16 cases (journal.zig:2328-2343)

"R>1/R=1" = decision for replica\_count>1 / replica\_count=1. Slot effects from `recover_slot`
(journal.zig:1497-1568):

`recover_slot`
`eql`
`@P`
`nil`
`@L`
`fix`
`@D`
`@F`
`@G`
`@M`
`vsr`
`@A`
`@B`
`@C`
`@D`
`@E`
`@K`
`@N`
`@O`
`get_prepare`
`cut_torn`
`@TruncateTorn`
`cut`
`@H`
`@I`
`@J`
`op_prepare_max`

| Case | Lines | R>1 / R=1 | Situation (header ring vs prepares ring) | What is copied where | What is flushed |
| --- | --- | --- | --- | --- | --- |
| @A | 2328 | vsr / vsr | no valid header, no valid prepare | slot → reserved header | none; dirty+faulty stay ⇒ remote `get_prepare` repair |
| @B | 2329 | vsr / vsr | valid reserved header, invalid prepare (possibly torn append) | slot → reserved header | none; dirty+faulty stay (torn subset refined to `@TruncateTorn`) |
| @C | 2330 | vsr / vsr | valid non-reserved header, invalid prepare | slot → reserved header | none; dirty+faulty stay |
| @D | 2331 | vsr / **fix** | invalid header, valid reserved prepare | prepare header → `headers[slot]` | solo: header-sector rewrite in `recover_fix` (journal.zig:1654) |
| @E | 2332 | vsr / **fix** | invalid header, valid non-max prepare | prepare header → `headers[slot]` | solo: header-sector rewrite |
| @F | 2333 | fix / fix | invalid header, valid prepare that is the op maximum (crash between the two writes, latest op) | prepare header → `headers[slot]`, dirty stays, faulty cleared (journal.zig:1526-1527) | header-sector rewrite (recover\_fix) |
| @G | 2334 | fix / fix | valid reserved header, valid prepare (header write lost/misdirected) | same as @F | header-sector rewrite |
| @H | 2335 | cut / unr | prepare.op > op\_prepare\_max (next checkpoint's slot) | slot → reserved header (journal.zig:1565) | none; dirty+faulty cleared (1566-1567) |
| @I | 2336 | cut / unr | header.op > op\_prepare\_max, prepare present | slot → reserved header | none |
| @J | 2337 | cut / unr | header.op > op\_prepare\_max, prepare reserved | slot → reserved header | none |
| @K | 2338 | vsr / vsr | valid header, prepare lost/corrupt | slot → reserved header | none; dirty+faulty stay |
| @L | 2339 | nil / nil | both reserved — first fill of the log | redundant header → `headers[slot]` (identical) | none |
| @M | 2340 | fix / fix | both valid, prepare.op > header.op (redundant header write lost after wrap) | prepare header → `headers[slot]` | header-sector rewrite |
| @N | 2341 | vsr / vsr | both valid, header.op > prepare.op | slot → reserved header | none; dirty+faulty stay |
| @O | 2342 | vsr / vsr | same op, different views — cannot tell which is newer | slot → reserved header | none; dirty+faulty stay |
| @P | 2343 | eql / eql | header and prepare identical — normal path | redundant header → `headers[slot]` (identical) | none |
| @TruncateTorn | 2347-2352 | cut\_torn | refinement of @B/@C: torn append identified by torn\_prepares() | slot → reserved header | none; dirty+faulty cleared (journal.zig:1549-1550) |

`get_prepare`
`@TruncateTorn`
`headers[slot]`
`recover_fix`
`headers[slot]`
`headers[slot]`
`headers[slot]`
`headers[slot]`
`headers[slot]`

Totality is pinned in-module: the inline test `test "recovery_cases"` (`journal.zig:2507-2541`)
verifies that every 11-bit parameter combination matches exactly one case — exhaustiveness and
mutual exclusion. Superblock quorum behavior is pinned by
`src/vsr/superblock_quorums_fuzz.zig:20-98,100-112` (imported by `src/unit_tests.zig:69`).

`test "recovery_cases"`
`journal.zig:2507-2541`
`src/vsr/superblock_quorums_fuzz.zig:20-98,100-112`
`src/unit_tests.zig:69`

### Callouts

`wal_headers`
`message_size_max`
`wal_prepares`
`message_size_max`
`journal_iops_read_max`
`reads_repair_count_max`
`.wal_headers`
`headers_redundant`
`headers`
`headers`
`headers_redundant`
`fix`
`.wal_headers`
`recover_fix`
`vsr`
`get_prepare`
`op_prepare_max`
`op_prepare_max`
`prepare_max_for_checkpoint(op_checkpoint_next())`
`journal_slot_count = 1024`
`journal_iops_write_max = 32`
`op_checkpoint`
`op_prepare_max`

### Test evidence

`journal.zig:2507-2541`
`test "recovery_cases"`
`src/vsr/replica_test.zig:75, 94, 113, 131, 154, 171`
`WALCorrupt`
`src/vsr/replica_test.zig:184-204`

## 5. Tear detection at recovery (checksum + zero padding)

Trigger: a slot's prepare body or header fails validation during `recover_prepare_callback` /
`torn_prepares`.

`recover_prepare_callback`
`torn_prepares`

### Call hierarchy

`recover_prepare (journal.zig:1092)
└─ Storage.read_sectors .wal_prepares, whole message_size_max (journal.zig:1124-1132)
└─ recover_prepare_callback (journal.zig:1135)
├─ header.valid_checksum() (journal.zig:1151)
├─ header.valid_checksum_body(body_used()) (journal.zig:1152)
├─ stdx.zeroed(message_padding) (journal.zig:1155-1158)
└─ valid ⇒ headers[slot] = header (journal.zig:1159); else slot stays faulty (:1166)
recover_slots (journal.zig:1240)
├─ header_ok (journal.zig:2485-2505): valid_checksum + command + cluster + slot
├─ torn_prepares (journal.zig:1359-1483)
│ ├─ op_max certainty over both rings (journal.zig:1369-1372)
│ ├─ fault-free range (op_max+1 .. op_prepare_max) requirement (journal.zig:1391-1403)
│ ├─ eligibility: valid redundant header from previous wrap + invalid prepare checksum (journal.zig:1447-1466)
│ └─ bound: journal_iops_write_max slots (journal.zig:1405-1408, 1469-1478)
└─ case_cut_torn applied (journal.zig:1280-1287) → recover_slot .cut_torn (journal.zig:1543-1551)`

### Sequence

### Mechanics table

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection |
| --- | --- | --- | --- | --- | --- |
| whole-slot read | journal.zig:1124-1132 | read the entire `message_size_max` slot — size unknown a priori, must checksum the whole sector span | none | `message_size_max = 1 MiB` (config.zig:159); slot\_count=1024 reads issued up to read-IOPS limit | read scope itself is the guard: nothing partially read is trusted |
| header checksum | journal.zig:1151 | `valid_checksum()` on the prepare's own header | — | — | fail ⇒ slot stays faulty (journal.zig:1166) |
| body checksum | journal.zig:1152 | `valid_checksum_body(body_used())` — checked here, not in `recover_done`, so the whole message can be released (comment journal.zig:1149-1150) | — | body ≤ message\_size\_max | same |
| zero padding | journal.zig:1155-1158 | `stdx.zeroed(buffer[size..sector_ceil(size)])` — a torn sector's tail is not zeroed | — | padding = sector\_ceil(size) − size ≤ sector\_size−1 (constants.zig:489 sector\_size=4096) | non-zero padding ⇒ header not adopted |
| misdirect defense | journal.zig:20-21, 2495-2501 | reserved headers carry the slot index as op; `header_ok` requires cluster match + `slot == op % slot_count` | — | — | wrong cluster/slot ⇒ header rejected, slot faulty |
| torn eligibility | journal.zig:1443-1466 | slot is "torn" iff: redundant header valid (not null, not reserved), prepare checksum invalid, redundant header.op < op\_max (from a previous wrap) | — | — | three abort conditions: corrupt redundant header (:1447), reserved redundant header (:1455), valid prepare checksum (:1459) |
| op\_max certainty | journal.zig:1369-1381, 1391-1403 | truncation allowed only if no non-torn fault in (op\_max, op\_prepare\_max]; op\_max = max over both rings' untrusted op maximum | — | — | any such fault ⇒ return empty, nothing truncated |
| torn bound | journal.zig:1405-1408, 1469-1478 | at most `journal_iops_write_max` slots considered torn — the max concurrent prepare writes; more ⇒ corruption, not torn | — | `journal_iops_write_max = 32` (config.zig:117, constants.zig:516) | overflow ⇒ `torn_prepares: not truncating` warn, return empty (journal.zig:1472-1477) |
| truncation | journal.zig:1280-1287, 1543-1551 | refine case to `@TruncateTorn`; slot reset to reserved; dirty+faulty cleared | header rewrite only if slot later needs `fix`; cut\_torn slots are clean | ≤32 torn slots | torn slots identified as append-in-progress, not corruption |
| head certainty | replica.zig:6990-7040 | `op_head_certain()`: op==op\_prepare\_max, or no faulty slot in (op, op\_prepare\_max] | — | window = `pipeline_prepare_queue_max`-bounded suffix | faulty head/prepare\_max slot ⇒ head uncertain ⇒ `.recovering_head` |

`message_size_max`
`message_size_max = 1 MiB`
`valid_checksum()`
`valid_checksum_body(body_used())`
`recover_done`
`stdx.zeroed(buffer[size..sector_ceil(size)])`
`header_ok`
`slot == op % slot_count`
`journal_iops_write_max`
`journal_iops_write_max = 32`
`torn_prepares: not truncating`
`@TruncateTorn`
`fix`
`op_head_certain()`
`pipeline_prepare_queue_max`
`.recovering_head`

### Callouts

`@TruncateTorn`
`journal_iops_write_max = 32`
`op`
`header_ok`

### Test evidence

`src/vsr/replica_test.zig:184-204`
`src/vsr/replica_test.zig:154-169`
`error.WALCorrupt`
`journal.zig:2507-2541`
`.cut_torn`

## 6. The two-write prepare path (backup-ring repair)

Trigger: every prepare accepted by a replica (primary pipeline, backup on\_prepare, repair). The
in-place ring is `Ring.prepares` (slot = op % `journal_slot_count`); the backup ring is
`Ring.headers` (redundant headers, `headers_redundant`) — `journal.zig:22-45`. Why two writes:
the prepares ring holds the message; the headers ring is the durable, independently-addressable
evidence copy of its header. A prepare write that lands but whose header-sector write is lost is
@F/@G — locally fixable from the intact prepare. A valid header above a corrupt prepare (@B/@C +
torn refinement) identifies a torn append — safe to truncate. Each alone would be ambiguous; the
pair is the test.

`Ring.prepares`
`journal_slot_count`
`Ring.headers`
`headers_redundant`
`journal.zig:22-45`

### Call hierarchy

`Replica.on_prepare / repair path (replica.zig:2524, 4233, 4927, 8376)
└─ Replica.write_prepare (replica.zig:11178)
└─ Journal.write_prepare (replica.zig:11214 → journal.zig:1789)
├─ clean-slot skip: callback immediately (journal.zig:1811-1819)
├─ IOP acquire; none free ⇒ callback(null) "waiting for IOP" (journal.zig:1824-1830)
└─ write_sectors(.prepares, in-place slot) (journal.zig:1851)
└─ write_prepare_header (journal.zig:1856)
├─ prepare_inhabited/prepare_checksums set (journal.zig:1866-1867)
├─ headers_redundant[slot] = message.header (journal.zig:1882)
├─ header_sector(...) — copies from headers_redundant, never headers (journal.zig:1893-1896, 2121-2167)
└─ write_sectors(.headers, one sector) (journal.zig:1908)
└─ write_prepare_on_write_header (journal.zig:1911)
├─ staleness re-checks (journal.zig:1915-1949)
├─ dirty.clear + faulty.clear — clean only after the SECOND write (journal.zig:1953-1954)
└─ write_prepare_release → callback (journal.zig:1959-1976)
└─ Replica.write_prepare_callback (replica.zig:11219)
└─ send_prepare_ok (replica.zig:11225 → 8615)
Recovery-side pairing repair:
Journal.recover_fix (journal.zig:1609) → write_sectors(.headers) (journal.zig:1654)
Fencing of slot overwrite:
recovery cases @H/@I/@J cut (journal.zig:2335-2337) + constants.zig:69-70`

### Sequence

### Mechanics table

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection |
| --- | --- | --- | --- | --- | --- |
| write\_prepare | journal.zig:1789-1852 | slice message to sector\_ceil(size), zero-padding asserted (journal.zig:1842-1846); clear inhabited/checksum bookkeeping (1848-1849); issue prepares-ring write | per-write, immediate; no batching, no timer | 1 message per write; concurrency ≤ `journal_iops_write_max=32` (config.zig:117; acquire at journal.zig:1824) | padding assert journal.zig:1846 |
| write\_prepare\_header | journal.zig:1856-1909 | after the prepare lands: mirror header into `headers_redundant` (1882), build the header sector from `headers_redundant` — deliberately never from `headers`, to avoid leaking a redundant header before its prepare is on disk (comment journal.zig:2144-2145) | second write, chained on the first's completion | 1 sector = `headers_per_sector = sector_size/@sizeOf(Header)` headers (journal.zig:47) | stale-entry guards: entry changed while writing ⇒ release(null) + re-dirty (journal.zig:1869-1875) |
| header\_sector faulty slots | journal.zig:2153-2164 | faulty slots' redundant headers are deliberately written invalid (reserved + checksum 0) so a crash-recovery still classifies them faulty | on every header-sector write | same sector as above | post-recovery consistency assert journal.zig:1576-1577 |
| write\_prepare\_on\_write\_header | journal.zig:1911-1957 | completion of the headers-ring write: verify entry still current (1915-1949), then `dirty.clear(slot); faulty.clear(slot)` (1953-1954) — the slot becomes clean/`has_prepare()==true` (journal.zig:540-548) | end of the two-write pair | — | header/prepare checksum divergence mid-write ⇒ callback(null), slot stays dirty |
| write\_prepare\_release → callback | journal.zig:1959-1976 → replica.zig:11219-11227 | write object released before callback (solo enforces ≤1 in flight, journal.zig:1969-1970); replica resumes receive and sends prepare\_ok | — | — | — |
| crash between the two writes | recovery cases @F/@G (journal.zig:2333-2334, rationale 2228-2240) | prepare durable, redundant header stale/missing ⇒ locally repairable: header ← intact prepare | `recover_fix` header-sector rewrite (journal.zig:1649-1654) | one sector per write | checksum comparison header vs prepare (journal.zig:2432-2434, 2454) |
| checkpoint fencing of slot overwrite | constants.zig:69-70 (`vsr_checkpoint_ops + lsm_compaction_ops + pipeline_prepare_queue_max*2 <= journal_slot_count`); vsr.md:9 ("After a checkpoint, the checkpointed WAL entries are safe to be overwritten by the next wrap") | a slot is overwritten only after the ops it held from the previous wrap are checkpointed on a quorum; interval leaves `lsm_compaction_ops + 2*pipeline_prepare_queue_max` spare slots | superblock write per checkpoint (every `vsr_checkpoint_ops=960` ops) and per view transition | 960 = 1024−32−32 (constants.zig:40-42; assert fenced at :69-70) | recovery cuts anything past prepare\_max: @H/@I/@J (journal.zig:2335-2337, rationale 2243-2248) |
| overwrite inside JV window | replica.zig:2264-2270 comment ("pipeline is the upper bound on how many ops can be reordered") + nack case 3 (replica.zig:8831-8835) | within (op\_max, op\_prepare\_max] a slot cannot hold a different op the replica acked — the wrap that would overwrite it is fenced by checkpointing | — | `pipeline_prepare_queue_max=8` | nack bitset abstention: case 2 requires `dirty and !faulty` (replica.zig:8826-8829); case 1 requires `!faulty` (8818) |

`journal_iops_write_max=32`
`headers_redundant`
`headers_redundant`
`headers`
`headers_per_sector = sector_size/@sizeOf(Header)`
`dirty.clear(slot); faulty.clear(slot)`
`has_prepare()==true`
`recover_fix`
`vsr_checkpoint_ops + lsm_compaction_ops + pipeline_prepare_queue_max*2 <= journal_slot_count`
`lsm_compaction_ops + 2*pipeline_prepare_queue_max`
`vsr_checkpoint_ops=960`
`pipeline_prepare_queue_max=8`
`dirty and !faulty`
`!faulty`

### Callouts

`storage.write_sectors`
`.wal_prepares`
`.wal_headers`
`write_prepare_header`
`write_prepare_on_write_header`
`has_prepare`
`send_prepare_ok`
`message_size_max`
`headers_per_sector`
`journal_iops_write_max=32`
`vsr_checkpoint_ops=960`
`view_durable_update`
`send_commit`
`N − R + 1`
`R`
`quorum_nack_prepare + quorum_replication > replica_count`

### Test evidence

`src/vsr/replica_test.zig:650-702`
`dont nack`
`src/vsr/replica_test.zig:2084-2123`
`src/vsr/replica_test.zig:171-182`
`src/vsr/replica_test.zig:113-129`

## 7. Durable superblock view increment on restart

Trigger: replica resumes from `.recovering` after journal recovery. On restart, exactly two
paths durably advance the superblock view: (a) solo, always (`replica.zig:882-885`), because a
torn prepare truncated by the next boot must never be re-prepared under the same view number;
(b) non-solo, only when the recovering replica is the primary of the *next* view
(`replica.zig:894` → `:10208-10209`). A recovering backup (`replica.zig:895-896`) and a
durable-JV resume (`replica.zig:905` → `:10204`) never write the superblock during recovery.

`.recovering`
`replica.zig:882-885`
`replica.zig:894`
`:10208-10209`
`replica.zig:895-896`
`replica.zig:905`
`:10204`

### Call hierarchy

`Replica.open (replica.zig:654)
├─ solo branch (replica.zig:872-889)
│ ├─ error.WALCorrupt if any faulty slot (replica.zig:873)
│ ├─ log_view += 1; view += 1 (replica.zig:882-883)
│ ├─ primary_update_view_headers (replica.zig:884 → 5939)
│ ├─ view_durable_update (replica.zig:885 → 9448)
│ │ └─ SuperBlock.view_change (replica.zig:9473 → superblock.zig:1010)
│ │ ├─ write_staging: sequence+1, parent=old checksum (superblock.zig:1096-1099)
│ │ │ ├─ write_header ×4 copies (superblock.zig:1124-1173)
│ │ │ └─ read_working(.verify) — 3-of-4 (superblock.zig:1191-1193, 1276-1281)
│ │ └─ view_durable_update_callback (replica.zig:9533)
│ └─ transition_to_normal_from_recovering_status (replica.zig:887-889 → 9935)
├─ multi-replica branches (replica.zig:890-907)
│ ├─ primary of current view: transition_to_view_change_status(self.view + 1) (replica.zig:893-894)
│ │ └─ inside: view = view_new; view_durable_update() (replica.zig:10206-10209)
│ ├─ backup, head certain: transition_to_normal_from_recovering_status (replica.zig:895-896) — NO superblock write
│ ├─ head uncertain: transition_to_recovering_head_from_recovering_status (replica.zig:899) — NO superblock write
│ └─ durable JV: transition_to_view_change_status(self.view) (replica.zig:901-905)
│ └─ inside: self.view == view_new ⇒ only assert, NO view_durable_update (replica.zig:10204-10205)
└─ request gates while durability in flight
├─ ignore_request_message: view_durable_updating ⇒ drop request (replica.zig:6381-6386)
└─ pulse_enabled: solo && view_durable_updating ⇒ no pulse (replica.zig:11395)`

### Sequence

### Mechanics table

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection |
| --- | --- | --- | --- | --- | --- |
| solo: always increments | replica.zig:876-885 | `log_view += 1; view += 1; primary_update_view_headers(); view_durable_update()` — unconditionally after recovery; comment :876-878: otherwise two versions of an op could share a view | superblock write (`superblock.view_change`), per restart | 1 replica; superblock copies = 4, verify quorum 3 (superblock\_quorums.zig:61-64) | pre-gate: any faulty slot ⇒ `error.WALCorrupt` (replica.zig:873) |
| solo: requests held until durable | replica.zig:6381-6386 (requests), 11395 (pulses) | `ignore_request_message` drops requests while `view_durable_updating()`; solo stops pulsing (accepting work) until the view is durable | gate releases at `view_durable_update_callback` (replica.zig:9533) | — | — |
| non-solo primary-of-view+1: flushes | replica.zig:893-894 → 10160-10210 | `transition_to_view_change_status(view+1)`: `view = view_new` (10208) then `view_durable_update()` (10209) — persist before JV advertisement ("cannot advertise either value until it is certain they will never backtrack", superblock.zig:1001-1004) | superblock write, per view transition | superblock write quorum 3-of-4; view-change participation gated on this write (replica.zig:10209 before send\_join\_view at 10251) | monotonicity asserts superblock.zig:1018-1022; anti-fork proof comment replica.zig:9395-9433 |
| non-solo backup: never flushes | replica.zig:895-896 | backup with certain head → `normal` in its existing durable view; view/log\_view already durable in the superblock it just opened | none | — | view sanity: header.view ≤ durable log\_view (replica.zig:870) |
| durable-JV resume: no new write | replica.zig:901-905 → 10204-10205 | resuming an interrupted view change at the *same* view (view==view\_new): status→`.view_change`, no `view_durable_update` (the durable JV already fenced it) | none | — | `assert(status_before == .recovering)` (replica.zig:10205) |
| JV resume head rule | replica.zig:902-903 | `op_head_certain()` deliberately NOT called — head comes from view\_headers, and a JV always has a certain head op | — | — | out-of-log\_view WAL headers dropped at journal.zig:1313-1332 |
| durable sources of view state | superblock.zig:140-143 (`log_view`, `view`), :119 (`commit_max`); seeded at replica.zig:1347-1351 | view/log\_view/commit\_max live in `working.vsr_state`, monotonic by `view_change()` asserts | per view transition + per checkpoint (960 ops) | — | VSRState monotonic check on open (superblock\_quorums.zig:185) |

`log_view += 1; view += 1; primary_update_view_headers(); view_durable_update()`
`superblock.view_change`
`error.WALCorrupt`
`ignore_request_message`
`view_durable_updating()`
`view_durable_update_callback`
`transition_to_view_change_status(view+1)`
`view = view_new`
`view_durable_update()`
`normal`
`.view_change`
`view_durable_update`
`assert(status_before == .recovering)`
`op_head_certain()`
`log_view`
`view`
`commit_max`
`working.vsr_state`
`view_change()`

### Callouts

`transition_to_view_change_status`
`view = view_new`
`view_durable_update()`
`replica.zig:10202-10210`
`superblock.zig:1001-1004`
`replica.zig:9395-9433`
`error.WALCorrupt`
`N − R + 1`

### Test evidence

`src/vsr/replica_test.zig:184-204`
`src/vsr/replica_test.zig:251-306`
`src/vsr/replica_test.zig:94-111`
`.view_change`

## 8. Scenario-by-scenario recovery behavior

| Scenario | What happens | Evidence |
| --- | --- | --- |
| **Backup crashes, restarts** | No view change. Open superblock, replay WAL, rejoin `normal`, repair hash-chain headers/prepares from peers (`get_headers`/`get_prepare`), consume `commit` piggybacked on prepares + heartbeats. | replica.zig:894-896, :7544, :3771 |
| **Primary crashes, restarts** | It *must* trigger a view change (`transition_to_view_change_status(view+1)`) — it cannot resume leadership in its old view. Its durable evidence (view/log\_view/JV headers) seeds the election. | replica.zig:893-894 |
| **Backup crashed while view change in flight** | Durable JV in superblock ⇒ resume the view change at restart; journal headers outside the durable `log_view` are dropped (fork prevention). | replica.zig:905, journal.zig:1299-1332 |
| **Head not locally recoverable** (torn/corrupt head) | `.recovering_head`: refuse to guess; wait for the primary's `view`, accepted only if provably post-crash (newer view, op ≥ prepare max, or nonce match). | replica.zig:9894-9937, :2763-2777 |
| **Replica lagging > checkpoint** | No log replay path; **state sync**: fetch checkpoint + grid blocks + reply zone from peers (`get_blocks`/`block`/`get_reply`, `sync.zig`), rate-limited by repair budget. | replica.zig:2851-2882, :11229+ |
| **Full-cluster restart** | Every replica reads its own disk; the quorum elects via view change using durable views. Journals + superblocks alone suffice; docs state no recovery protocol is needed. | vsr.md:53-57, replica.zig:654-905 |
| **Solo replica (N=1)** | Unconditionally increments its view and durably records it before serving, so two versions of an op can never exist in one view. | replica.zig:876-885 |
| **Torn write at crash** | At most `journal_iops_write_max` slots considered torn (the max in-flight writes); torn prepares are truncated; more than that = corruption, nothing truncated. | journal.zig:1405-1408, :1278-1287 |
| **Corrupt-but-not-torn data** | Checksums on header + body; misdirected I/O defeated by per-slot reserved ops and `(checksum, address)` addressing; repair from peers' identical copies (the protocol-aware-recovery posture, quoted from the tree's docs: "TigerBeetle doesn't make a distinction whether checksummed data comes from a local disk or another replica", ARCHITECTURE.md:87-90; "we do not nack corrupt entries", vsr.md:218). | journal.zig:1151-1161, docs/ARCHITECTURE.md:87-90 |

`normal`
`get_headers`
`get_prepare`
`commit`
`transition_to_view_change_status(view+1)`
`log_view`
`.recovering_head`
`view`
`get_blocks`
`block`
`get_reply`
`sync.zig`
`journal_iops_write_max`
`(checksum, address)`

Notes on the scenarios, from the code:

`fn repair`
`replica.zig:7544`
`:7531-7543`
`on_journal_repair_timeout`
`replica.zig:3771-3777`
`docs/internals/vsr.md:79-81`
`command=commit`
`replica.zig:2763-2777`
`message.header.view > self.view or message.header.op >= self.op_prepare_max() or message.header.nonce == self.nonce`
`replica.zig:9905`
`self.status = .recovering_head;`
`on_view_set_checkpoint`
`replica.zig:2851`
`:2875-2882`
`far_behind`
`constants.vsr_checkpoint_ops`
`commit_max`
`send_get_blocks`
`replica.zig:11229`
`:11235-11239`
`src/vsr/sync.zig`
`src/vsr/repair_budget.zig`
`journal.zig:1299-1312`
`:1313-1330`
`remove_entry`
`!view_range.contains(header.view)`

## 9. What stays volatile, and why that is safe

`send_commit`
`replica.zig:11370-11380`
`commit_max`
`VSRState.commit_max`
`superblock.zig:119`
`superblock.working.vsr_state.commit_max`
`replica.zig:1351`
`commit_max`
`replica.zig:2677-2680`
`vsr.md:156`
`get_reply`
`docs/internals/vsr.md:229-232`
`header.commit >= session`
`client_sessions.zig:174`
`client_sessions.zig:33-34`
`client_replies.zig:11-14`
`client_replies`
`client_replies_size = clients_max * message_size_max`
`constants.zig:178`
`CheckpointState`
`grid_scrubber.zig`

## 10. The corruption fault model

TigerBeetle has *no* amnesia class to handle — a restarting replica cannot forget a view-change
promise, because participation is gated on the durable superblock write (`§7`;
"cannot advertise either value until it is certain they will never backtrack",
`superblock.zig:1001-1004`; anti-fork proof `replica.zig:9395-9433`). What it does face is the
**corruption class**: durable state can be wrong on disk (bit rot, misdirected writes). That class
is answered by checksums, hash-chaining, the 16-case recovery table (§4), PAR-style nacks/repair,
and safe halt — and it carries documented residual risks (from TigerBeetle's own docs):

`§7`
`superblock.zig:1001-1004`
`replica.zig:9395-9433`
`docs/operating/recovering.md:6-13`
`tigerbeetle format`
`tigerbeetle recover`
`CHANGELOG.md:2273`
`recovering_head`
`4`
`6`
`replica.zig:8803-8858`

## 11. RTO shape and operational implications

`replica.zig:895-896`
`replica.zig:893-894`
`tigerbeetle format`
`tigerbeetle recover`
`CHANGELOG.md:2273`
`message_size_max = 1 MiB`
`pipeline_prepare_queue_max = 8`
`journal_iops_write_max = 32`
`vsr_checkpoint_ops = 960`

AOF is optional; its mechanics are in the appendix of the on-disk report.

## 12. Relationship to the TR's constructive program

External literature: UW-CSE-17-08-01 (Michael, Ports, Sharma, Szekeres, extended DISC'17), the
traces it reports for VR-Revisited (B.1), Paxos Made Live (B.2), and JPaxos (B.3).

The TR (Michael, Ports, Sharma, Szekeres) solves *diskless* crash recovery with crash-consistent
quorums and virtual stable storage (VSS): persist promises to a crash-consistent quorum before
acting on them. TigerBeetle's stable storage is real, and its "promises" (view, log\_view,
checkpoint) are persisted *to its own disk*, quorum-verified across 4 superblock copies, before
advertisement. The TR's generalized lesson — *a recovered node must not act on promises it no
longer remembers making* — is exactly the invariant TigerBeetle enforces mechanically; it enforces
it with a disk instead of a crash-consistent quorum.

**Bottom line:** TigerBeetle's flavour of Viewstamped Replication Revisited means recovery is
disk-first and amnesia-free by construction: every prepare is durable before ack, every view
transition is durable before advertisement, the commit point is safely re-derivable, and the
network is used only to repair checksummed data — so the system tolerates arbitrary crash
patterns including full-cluster simultaneous crashes (RPO 0 for acknowledged work), at the price
of two foreground flushes per op and a corruption fault-model that is answered, and bounded, by
protocol-aware repair.

## 13. Constants

| Constant | Definition (file:line) | Value | Role here |
| --- | --- | --- | --- |
| `replicas_max` | `src/constants.zig:24` | 6 | max replicas per cluster |
| `superblock_copies` | `config.zig:160` (`= 4`); asserted even, ∈ [4,8] at `constants.zig:556-558`, `{4,6,8}` at `superblock.zig:595-597` | 4 | superblock copies per data file |
| superblock write/verify quorum | `superblock_quorums.zig:61-66` (4→3); comment "When writing, we must verify that at least 3/4 copies were written" (:48) | 3 of 4 | post-write verify gate (panic on mismatch, superblock.zig:1278) |
| superblock open quorum | `superblock_quorums.zig:69-74` (4→2); rule "write quorum + read quorum = copies + 1" (:57), comment :47-52 | 2 of 4 | open() quorum gate |
| `journal_slot_count` | `config.zig:158`, `constants.zig:189`; slot = `op % slot_count` (journal.zig:63) | 1024 | WAL slots |
| `journal_size_headers` / `journal_size_prepares` | `constants.zig:197-198` | 1024·sizeof(Header) / 1024·1 MiB | the two rings |
| `vsr_checkpoint_ops` | `constants.zig:40-42` = `journal_slot_count − lsm_compaction_ops − lsm_compaction_ops·div_ceil(2·pipeline_prepare_queue_max, lsm_compaction_ops)` = 1024−32−32 | 960 | superblock write cadence; fencing assert `constants.zig:69-70` (invariant comment :44-74) |
| `lsm_compaction_ops` | `config.zig:164` | 32 | one LSM compaction bar = 32 ops; 2 half-bars of 16 beats; checkpoints align to bar boundaries (vsr.zig:1673) |
| `pipeline_prepare_queue_max` | `config.zig:155`; JV reorder window (journal.zig:2264-2270) | 8 | pipeline depth |
| `message_size_max` | `config.zig:159` | 1 MiB | max prepare (whole-slot recovery reads) |
| `journal_iops_write_max` | `config.zig:117`, `constants.zig:516`; torn bound at `journal.zig:1408` | 32 | concurrent WAL writes; torn-slot cap |
| `journal_iops_read_max` | `constants.zig:513`; repair share capped `journal.zig:101` | (config.process) | recovery read concurrency |
| `quorum_replication_max` | `config.zig:157` | 3 | replication-quorum cap |
| `quorum_replication` | `vsr.zig:1185-1188` (`min(quorum_replication_max=3, ceil(N/2))`, config.zig:157) | 3 (N≥6) | prepare\_ok threshold |
| `quorum_view_change` | `vsr.zig:1194-1195` = `N − R + 1`; intersection assert :1203 | N−R+1 | election gate |
| `quorum_nack_prepare` | `vsr.zig:1207` = `N − R + 1`; sum assert :1208 | N−R+1 | truncation gate |
| `sector_size` | `constants.zig:489` | 4096 | I/O granularity |
| `headers_per_sector` | `journal.zig:47` = 4096/sizeof(Header); `sector_size` constants.zig:489 | — | header-ring write batch |
| `client_replies_size` | `constants.zig:178` = `clients_max * message_size_max` | — | reply-cache zone (durable at checkpoint) |
| `superblock_zone_size` | `superblock.zig:602` = `superblock_copy_size * superblock_copies` | — | superblock zone |
| `VSRState.log_view` / `.view` / `.commit_max` | `superblock.zig:140-143`, `:119` | — | durable view evidence seeded at replica.zig:1347-1351 |

`replicas_max`
`src/constants.zig:24`
`superblock_copies`
`config.zig:160`
`= 4`
`constants.zig:556-558`
`{4,6,8}`
`superblock.zig:595-597`
`superblock_quorums.zig:61-66`
`superblock_quorums.zig:69-74`
`journal_slot_count`
`config.zig:158`
`constants.zig:189`
`op % slot_count`
`journal_size_headers`
`journal_size_prepares`
`constants.zig:197-198`
`vsr_checkpoint_ops`
`constants.zig:40-42`
`journal_slot_count − lsm_compaction_ops − lsm_compaction_ops·div_ceil(2·pipeline_prepare_queue_max, lsm_compaction_ops)`
`constants.zig:69-70`
`lsm_compaction_ops`
`config.zig:164`
`pipeline_prepare_queue_max`
`config.zig:155`
`message_size_max`
`config.zig:159`
`journal_iops_write_max`
`config.zig:117`
`constants.zig:516`
`journal.zig:1408`
`journal_iops_read_max`
`constants.zig:513`
`journal.zig:101`
`quorum_replication_max`
`config.zig:157`
`quorum_replication`
`vsr.zig:1185-1188`
`min(quorum_replication_max=3, ceil(N/2))`
`quorum_view_change`
`vsr.zig:1194-1195`
`N − R + 1`
`quorum_nack_prepare`
`vsr.zig:1207`
`N − R + 1`
`sector_size`
`constants.zig:489`
`headers_per_sector`
`journal.zig:47`
`sector_size`
`client_replies_size`
`constants.zig:178`
`clients_max * message_size_max`
`superblock_zone_size`
`superblock.zig:602`
`superblock_copy_size * superblock_copies`
`VSRState.log_view`
`.view`
`.commit_max`
`superblock.zig:140-143`
`:119`

# The flush model in one page

Everything TigerBeetle 0.17.9 puts on disk happens under one of three
triggers. Every section in the other reports is detail under one of these.

## Trigger 1 — force the WAL (per prepare, per replica)

`body ──► Ring.prepares (journal.zig:1851) O_DSYNC, slot = op % 1024 (config.zig:158)
header ──► Ring.headers (journal.zig:1908) written only after the body's completion
both ──► dirty.clear (journal.zig:1953) ─► prepare_ok (replica.zig:11225)`
`O_DSYNC`
`io/linux.zig:1432`
`F_FULLFSYNC`
`io/darwin.zig:829-830`
`FILE_WRITE_THROUGH`
`io/windows.zig:1277`
`message_size_max`
`config.zig:159`
`pipeline_prepare_queue_max`
`config.zig:155`
`journal_iops_write_max`
`config.zig:117`
`slot = op % 1024`
`.wal_headers`
`journal.zig:1204-1241`

## Trigger 2 — checkpoint (every 960 ops)

`vsr_checkpoint_ops = 1024 − 32 − 32 = 960` (`constants.zig:40-42`).

`vsr_checkpoint_ops = 1024 − 32 − 32 = 960`
`constants.zig:40-42`

Order inside a checkpoint — the superblock is written LAST, as the seal:

`state machine ──► grid blocks, client replies, client sessions, free-set
(each to its zone, all direct IO)
then ──► superblock ×4 copies, verify 3-of-4
(superblock.zig:1124-1172; superblock_quorums.zig:61-66)`
`sequence + 1`
`parent`
`superblock.zig:1097-1099`
`commit_max`
`free_set.zig:600-607`

## Trigger 3 — durable view fence (view change, restart)

`view`
`log_view`
`commit_max`
`replica.zig:9448-9513 → :10209`
`replica.zig:876-885`
`:894 → :10209`
`:896`

## What stays volatile, deliberately

`prepare_ok`

One sentence: **force the WAL per op; snapshot the state machine and seal it
with the superblock every 960 ops; fence every view durably before
advertising it.**

# TigerBeetle 0.17.9 — the IO subsystem explained

The production IO path of the 0.17.9 release.

The system is **one fixed-size file per replica**, pre-formatted into zones (`src/vsr.zig:125-214`:
`Zone` = `superblock`, `wal_headers`, `wal_prepares`, `client_replies`, `grid_padding`, `grid`),
accessed only through `Storage` (`src/storage.zig`; `StorageType` at `src/storage.zig:11`) and one
of three per-OS IO backends behind `src/io.zig:9-14` (`src/io/linux.zig`, `src/io/darwin.zig`,
`src/io/windows.zig`; `src/io/common.zig` holds shared POSIX helpers only).

`src/vsr.zig:125-214`
`Zone`
`superblock`
`wal_headers`
`wal_prepares`
`client_replies`
`grid_padding`
`grid`
`Storage`
`src/storage.zig`
`StorageType`
`src/storage.zig:11`
`src/io.zig:9-14`
`src/io/linux.zig`
`src/io/darwin.zig`
`src/io/windows.zig`
`src/io/common.zig`

## 1. The design axiom: write completion == durability

TigerBeetle refuses to trust the kernel page cache. Every consensus-critical write is issued
directly to the disk such that the write's completion is the durability point — there is no fsync
in the data path, and none is needed:

`src/io/linux.zig`
`O_RDWR | O_DSYNC`
`.DSYNC = true`
`O_DIRECT`
`flags.DIRECT = true`
`assert(flags.DSYNC)`
`direct_io_required`
`@panic("file system does not support Direct I/O")`
`fs_is_tmpfs`
`src/io/darwin.zig`
`O_DIRECT`
`fcntl(fd, F.NOCACHE, 1)`
`posix.pwrite`
`fs_sync(fd)`
`do_operation`
`fs_sync`
`fcntl(fd, F.FULLFSYNC, 1) catch return posix.fsync(fd)`
`src/io/windows.zig`
`FILE_NO_INTERMEDIATE_BUFFERING`
`FILE_WRITE_THROUGH`

Two corollaries:

`IO.fsync`
`sqe.prep_fsync(op.fd, op.flags)`
`IORING_FSYNC_DATASYNC`
`Storage`
`io.read`
`io.write`
`posix.fsync`
`O_DSYNC`
`FILE_WRITE_THROUGH`
`F_FULLFSYNC`
`pwrite`
`cache: Cache`
`src/vsr/grid.zig:174`

### Flow: open path per OS

Trigger: `Storage.init` (replica format or open) → per-OS `IO.open_data_file`.

`Storage.init`
`IO.open_data_file`

Call hierarchy (shared):

`src/tigerbeetle/main.zig:97-103`
`io_entries`
`IO.init(io_entries, 0)`
`main.zig:148-154`
`direct_io`
`.direct_io_disabled`
`!constants.direct_io`
`.direct_io_optional`
`args.development`
`.direct_io_required`
`storage.zig:97-126`
`Storage.init`
`IO.open_dir(dirname)`
`io.open_data_file(dir_fd, basename, size_min, purpose, direct_io)`

Per backend:

`open_data_file`
`.CLOEXEC`
`.ACCMODE`
`.DSYNC`
`fstatat`
`.LARGEFILE`
`.DIRECT`
`.EXCL`
`fs_supports_direct_io`
`flags.DIRECT = true`
`.CREAT`
`.EXCL`
`assert(flags.DSYNC)`
`posix.openat`
`flock(LOCK_EX|LOCK_NB)`
`fallocate`
`posix.fsync(fd)`
`posix.fsync(dir_fd)`
`@panic("data file inode size was truncated or corrupted")`
`open_data_file`
`.DSYNC = true`
`assert(flags.DSYNC)`
`posix.openat`
`F_NOCACHE`
`direct_io != .direct_io_disabled`
`flock`
`fs_allocate`
`fs_sync(fd)`
`fs_sync(dir_fd)`
`open_data_file`
`open_file_handle`
`FILE_CREATE`
`OPEN_EXISTING`
`SYNCHRONIZE | GENERIC_READ [| GENERIC_WRITE]`
`FILE_NO_INTERMEDIATE_BUFFERING | FILE_WRITE_THROUGH`
`NtCreateFile`
`windows_open_file`
`register_handle`
`fs_lock`
`fs_allocate`
`posix.fsync(handle)`

Linux:

Darwin:

Windows:

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality | Tear detection |
| --- | --- | --- | --- | --- | --- |
| Durability flags Linux | linux.zig:1429-1433, :1540 | `.DSYNC` unconditional; `.DIRECT` when fs supports and direct\_io enabled | per-write O\_DSYNC — kernel flush piggybacked on every write; no separate fsync in data path | n/a (single fd) | none at open; read-path verification later (journal.zig:871-914) |
| Direct-I/O gating Linux | linux.zig:1505-1521; main.zig:148-154 | probe via throwaway O\_DIRECT open; `direct_io_required` + unsupported ⇒ panic; tmpfs carve-out | n/a | 1 probe file (linux.zig:1764-1768) | none |
| Cache bypass Darwin | darwin.zig:1049-1051 | `F_NOCACHE` fcntl post-open (no O\_DIRECT on Darwin) | n/a | n/a | none |
| Cache bypass Windows | windows.zig:1275-1277 | `FILE_NO_INTERMEDIATE_BUFFERING` + `FILE_WRITE_THROUGH` at NtCreateFile | per-write write-through | n/a | none |
| Open-time fsync Linux | linux.zig:1627-1636 | blocking `fsync(fd)` then `fsync(dir_fd)`; FoundationDB-derived discipline: never read data that a crashed predecessor may not have durably written | checkpoint of the *open* itself: blocking fsync, file + dir | 2 fsyncs | covers any pending O\_DSYNC before first read |
| Open-time flush Darwin | darwin.zig:1074-1083, :1095-1100 | `fs_sync` = `F_FULLFSYNC` (fsync does not clear the disk cache); file + dir | blocking, file + dir | 2 F\_FULLFSYNCs | same as Linux, stronger (past disk cache) |
| Open-time fsync Windows | windows.zig:1385-1391, :1393-1396 | blocking `posix.fsync(handle)` only; directory fsync impossible (no write-access dir handle) and commented out | blocking, file only | 1 fsync | hole: file inode may not be durably linked |
| Contiguous allocation | linux.zig:1601-1625; darwin.zig:1072; windows.zig:1363-1383 | fallocate / F\_PREALLOCATE+ftruncate / SetEndOfFile; fallback: write last sector | n/a | 1 sector (4096 B) fallback write | none |
| Lock | linux.zig:1566-1583; darwin.zig:1055-1067; windows.zig:1348 | flock / flock / LockFileEx exclusive, fail-immediately; 5×50 ms retry on Linux (io\_uring fd lifetime) | n/a | n/a | another process holding the lock panics :1596 / :1063 / :1356 |
| Size verification | linux.zig:1638-1642; darwin.zig:1086-1087; windows.zig:1398-1399 | fstat/GetFileSizeEx vs `size_min`; panic on truncation | n/a | n/a | detects torn/corrupted inode size at startup |

`.DSYNC`
`.DIRECT`
`direct_io_required`
`F_NOCACHE`
`FILE_NO_INTERMEDIATE_BUFFERING`
`FILE_WRITE_THROUGH`
`fsync(fd)`
`fsync(dir_fd)`
`fs_sync`
`F_FULLFSYNC`
`posix.fsync(handle)`
`size_min`

Callouts:

`O_DIRECT`
`F_NOCACHE`
`FILE_NO_INTERMEDIATE_BUFFERING`
`O_DSYNC`
`F_FULLFSYNC`
`pwrite`
`FILE_WRITE_THROUGH`
`posix.fsync`
`posix.fsync(fd)`
`posix.fsync(dir_fd)`
`fs_sync(fd)`
`fs_sync(dir_fd)`
`io_entries`

Test evidence:

`open_data_file`
`open_data_file`
`storage.zig:111`
`src/io/test.zig:25-155`
`openat`
`RDWR|TRUNC|CREAT`
`open_data_file`

## 2. The event loop: `run()` / `run_for_ns()`

`run()`
`run_for_ns()`

All IO is asynchronous and callback-driven behind one API; the backend is selected at compile time
by `src/io.zig:9-14`. Every operation — accept, close, connect, fsync, openat, read, recv, send,
statx, timeout, write — is one SQE with the completion pointer stashed in
`sqe.user_data = @intFromPtr(completion)` (linux.zig:446, prep switch :437-443), so a CQE
dispatches straight to the owning completion (linux.zig:449+). Per-syscall errnos are translated
into typed Zig errors (e.g. `.DQUOT → error.DiskQuota`, `.IO → error.InputOutput`,
`.NOSPC → error.NoSpaceLeft`; linux.zig:744, :748, :749); `.INTR` re-enqueues instead of erroring
(:737-740). Completion callbacks are plain Zig fn pointers, no `callconv(.c)` (linux.zig:375-379;
the only `callconv` in the whole IO tree is the Win32 `LPFN_CONNECTEX` API pointer,
windows.zig:522).

`src/io.zig:9-14`
`sqe.user_data = @intFromPtr(completion)`
`.DQUOT → error.DiskQuota`
`.IO → error.InputOutput`
`.NOSPC → error.NoSpaceLeft`
`.INTR`
`callconv(.c)`
`callconv`
`LPFN_CONNECTEX`

Call hierarchy:

`IO.run`
`flush_submissions(0)`
`flush_completions(0)`
`completed`
`run_callback`
`flush_submissions(0)`
`IO.run_for_ns`
`while (now.ns < deadline.ns)`
`block_ns = if (completed.count() == 0) deadline.ns -| now.ns else 0`
`flush_submissions(block_ns)`
`flush_completions(0)`
`run_callback`
`flush_submissions(0)`
`submit_and_wait_timeout`
`io_uring_enter`
`IORING_ENTER_EXT_ARG`
`kernel_timespec`
`IORING_FEAT_EXT_ARG`
`flush_completions(1)`
`IO.run`
`flush(false)`
`run_for_ns`
`flush(true)`
`timed_out`
`flush`
`flush_timeouts`
`completed`
`flush_io`
`io_pending`
`kevent()`
`[256]posix.Kevent`
`wait_for_completions`
`IO.run`
`flush(.non_blocking)`
`run_for_ns`
`flush(.blocking)`
`flush`
`flush_timeouts`
`GetQueuedCompletionStatusEx`
`completed`

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality | Tear detection |
| --- | --- | --- | --- | --- | --- |
| Tick Linux | linux.zig:71-85 | submit → peek → drain callbacks → re-submit callbacks' new SQEs | submissions flushed twice per tick (:74, :84); no timer flush | drain = whole `completed` queue | none |
| Timed run Linux | linux.zig:90-120 | blocking wait inside `io_uring_enter` with inline `EXT_ARG` timeout (:241-258); skip wait when callbacks pending (:110) | deadline = `nanoseconds` (u63, coerced to `kernel_timespec` :250-253) | `[256]io_uring_cqe` per `copy_cqes` (linux.zig:172) | `CompletionQueueOvercommitted` ⇒ `flush_completions(1)` and retry (:149-159) |
| Kernel gate | linux.zig:58-61 | `IORING_FEAT_EXT_ARG` required; panic below 5.11 | n/a | n/a | none |
| Timed run Darwin | darwin.zig:57-99, :184-209 | userspace timeout queue; kevent blocks until min-deadline (:119-127); "kevent() blocking forever" panic if no timeout :121 | timer: kevent `ts` = min deadline | kevent batch `[256]Kevent` (darwin.zig:102) | none |
| Timed run Windows | windows.zig:60-93, :100-174 | IOCP wait with ms-rounded, capped timeout (:105-116); pure-timeout workloads block instead of busy-looping (:118-124) | timer: IOCP timeout ms | `[64]OVERLAPPED_ENTRY` (windows.zig:130) | none |
| Callback chaining | linux.zig:77-79; darwin.zig:147-153; windows.zig:162-171 | drain loop keeps popping; callbacks enqueued mid-drain run in the same pass (pinned by test, io/test.zig:835-875) | n/a | unbounded per pass | none |
| Storage wrapper | storage.zig:139-144 | `Storage.run` panics if `IO.run` errors — a broken event loop is fail-stop | n/a | n/a | panic on `io.tick()` error |

`completed`
`io_uring_enter`
`EXT_ARG`
`nanoseconds`
`kernel_timespec`
`[256]io_uring_cqe`
`copy_cqes`
`CompletionQueueOvercommitted`
`flush_completions(1)`
`IORING_FEAT_EXT_ARG`
`ts`
`[256]Kevent`
`[64]OVERLAPPED_ENTRY`
`Storage.run`
`IO.run`
`io.tick()`

Callouts:

`IORING_ENTER_EXT_ARG`
`io_uring_enter`
`IO.timeout()`
`run_for_ns`
`block_ns = 0`
`flush_submissions(0)`
`copy_cqes`
`io_entries`
`after = 500 / constants.tick_ms`
`tick_ms = 10`
`src/config.zig:120`
`src/vsr/replica.zig:1376-1380`

Test evidence:

`io/test.zig:285-346`
`elapsed >= delay`
`io/test.zig:460-605`
`IO.run()`
`io/test.zig:607-786`
`run()`
`run_for_ns`
`io/test.zig:788-833`
`completed`
`io/test.zig:835-875`

## 3. The Storage layer: one fixed file, zones, and the alignment gate

`Storage` (src/storage.zig) maps `(zone, offset_in_zone)` to an absolute file offset
(`Zone.offset`, vsr.zig:171-177) and enforces the alignment discipline that makes direct I/O
possible. Zone layout guarantees — every zone size and start sector-aligned, grid start
block-aligned — are comptime-asserted (vsr.zig:154-169); `grid_padding` exists solely to align the
grid to `block_size`.

`Storage`
`(zone, offset_in_zone)`
`Zone.offset`
`grid_padding`
`block_size`

The gate, enforced at every public entry: `read_sectors` → `zone.verify_iop(buffer, offset_in_zone)` (storage.zig:185); `write_sectors` → same (storage.zig:356). `Zone.verify_iop`
(vsr.zig:203-213) asserts, verbatim (vsr.zig:204-212):

`read_sectors`
`zone.verify_iop(buffer, offset_in_zone)`
`write_sectors`
`Zone.verify_iop`

`start_read`/`start_write` re-assert the raw offset at every (re)start, including partial-IO
retries: `assert(read.offset % constants.sector_size == 0)` (storage.zig:204) and the same for
writes (storage.zig:375). The journal double-checks its header write offset (journal.zig:1891).
`sector_ceil`/`sector_floor` (vsr.zig:1163-1171) size every IO to whole sectors: the prepare body
is sliced to `vsr.sector_ceil(header.size)` before its write (journal.zig:1842), with the padding
pre-zeroed upstream (replica-level memset at `src/vsr/replica.zig:1665-1675`, memset :1675) and
asserted zero at dispatch (journal.zig:1846).

`start_read`
`start_write`
`assert(read.offset % constants.sector_size == 0)`
`sector_ceil`
`sector_floor`
`vsr.sector_ceil(header.size)`
`src/vsr/replica.zig:1665-1675`

Buffer *address* alignment is guaranteed at allocation: message-pool buffers are
`align(constants.sector_size)` (`src/message_pool.zig:134`, :192, :328), which is what makes the
assert pass without runtime bouncing.

`align(constants.sector_size)`
`src/message_pool.zig:134`

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality | Tear detection |
| --- | --- | --- | --- | --- | --- |
| verify\_iop | vsr.zig:203-213 | buffer address, length, file offset all `% 4096 == 0`; grid additionally `% block_size == 0`; in-zone bounds | n/a | `sector_size` = 4096 (src/constants.zig:489); `block_size` = 512 KiB (config.zig:161) | none — panic asserts |
| offset re-assert | storage.zig:204, :375 | `read.offset % sector_size == 0` / `write.offset % sector_size == 0` at every (re)start, including partial-IO retries | n/a | n/a | none |
| sector sizing | vsr.zig:1163-1171; journal.zig:1842 | `sector_ceil(size)` slice so O\_DIRECT transfers only whole sectors; padding pre-zeroed (journal.zig:1846) and asserted zero on read-back (journal.zig:1846) | n/a | ceil to 4096 | zero-padding re-checked on recovery: journal.zig:908-912, :1156-1160 |
| partial-read realignment | storage.zig:45-75 | `Read.target` caps the slice to the next logical-sector boundary after a 512-byte physical partial read | n/a | physical sector may be 512 B | n/a |
| zone layout | vsr.zig:125-214 | `Zone.start/.offset/.size` — superblock, wal\_headers, wal\_prepares, client\_replies, grid\_padding, grid; `grid_padding` aligns grid to block size | n/a | headers ring 1024×256 B (constants.zig:197), prepares ring 1024×1 MiB (constants.zig:198) | n/a |
| failure mode | — | EINVAL from the kernel would be `error.Alignment` (linux.zig:608/:747, darwin.zig:613) — unreachable in practice because the asserts run first | n/a | n/a | EINVAL is classified "impossible" in the assert-first design |

`% 4096 == 0`
`% block_size == 0`
`sector_size`
`block_size`
`read.offset % sector_size == 0`
`write.offset % sector_size == 0`
`sector_ceil(size)`
`Read.target`
`Zone.start/.offset/.size`
`grid_padding`
`error.Alignment`

Callouts:

`verify_iop`
`start_read`
`start_write`
`constants.sector_size = 4096`
`block_size = 512 KiB`
`.grid`
`checksum_body`
`body_used()`
`message.buffer[@sizeOf(Header)..message.header.size]`
`src/message_pool.zig:147-149`
`src/vsr/replica.zig:3601`
`io/test.zig`
`storage_fuzz.zig`
`src/vsr/replica_test.zig:75-206`

## 4. The WAL: two chained durable writes per prepare

Two fixed rings (`src/vsr/journal.zig:19-42`): `wal_headers` — `journal_size_headers = journal_slot_count * @sizeOf(vsr.Header)` = 1024 × 256 B = 256 KiB (`src/constants.zig:197`;
`@sizeOf(Header) == 256` asserted, `src/vsr/message_header.zig:72`; 16 headers per 4 KiB sector,
`headers_per_sector`, journal.zig:47) — and `wal_prepares` — `journal_size_prepares = journal_slot_count * message_size_max` = 1024 × 1 MiB = 1 GiB (`src/constants.zig:198`;
`message_size_max = 1 * MiB`, `src/config.zig:159`). A header's slot is
`header.op % constants.journal_slot_count` (journal.zig:63; `journal_slot_count = 1024`,
`src/config.zig:158`). Overlapping writes to the same header slot are serialized by an in-memory
range lock chain: `Journal.write_sectors` → `lock_sectors` (journal.zig:2015-2032, :2036-2091);
overlapping `.headers` ranges chain on `range.next` (:2054-2057) and the chain is drained on
completion in `write_sectors_on_write` (:2108-2116).

`src/vsr/journal.zig:19-42`
`wal_headers`
`journal_size_headers = journal_slot_count * @sizeOf(vsr.Header)`
`src/constants.zig:197`
`@sizeOf(Header) == 256`
`src/vsr/message_header.zig:72`
`headers_per_sector`
`wal_prepares`
`journal_size_prepares = journal_slot_count * message_size_max`
`src/constants.zig:198`
`message_size_max = 1 * MiB`
`src/config.zig:159`
`header.op % constants.journal_slot_count`
`journal_slot_count = 1024`
`src/config.zig:158`
`Journal.write_sectors`
`lock_sectors`
`.headers`
`range.next`
`write_sectors_on_write`

### Linux (io\_uring) — the reference chain

Write:

`Replica.write_prepare`
`journal.write_prepare`
`vsr.sector_ceil(header.size)`
`Journal.write_sectors(write_prepare_header, write, buffer, .prepares, offset)`
`lock_sectors`
`Storage.write_sectors`
`zone.verify_iop(buffer, offset_in_zone)`
`zone.offset`
`Storage.start_write`
`assert(write.offset % sector_size == 0)`
`io.write(..., on_write, ...)`
`IO.write`
`erase_types`
`enqueue(completion)`
`flush_submissions(0)`
`Completion.prep`
`.write`
`sqe.prep_write(fd, buf[0..buffer_limit(len)], offset)`
`sqe.user_data = @intFromPtr(completion)`
`flush_submissions`
`submit_and_wait_timeout`
`IORING_ENTER_EXT_ARG`
`flush_completions`
`copy_cqes`
`[256]io_uring_cqe`
`completion.result = cqe.res`
`completed`
`run_callback`
`Completion.complete()`
`.write`
`DQUOT`
`INVAL`
`IO`
`NOSPC`
`.INTR`
`completion.callback(context, completion, &result)`
`Storage.on_write`
`write.callback(write)`
`Journal.write_sectors_on_write`
`range.callback(write)`
`write_prepare_header`
`.headers`
`write_prepare_on_write_header`
`write_prepare_release(write, null)`
`dirty.clear(slot)`
`faulty.clear(slot)`
`write_prepare_release(write, message)`
`write_prepare_callback`
`send_prepare_ok`
`journal.has_prepare`
`!dirty`

Read:

`Storage.read_sectors`
`verify_iop`
`start_read(read, null)`
`start_read`
`assert(read.offset % sector_size == 0)`
`io.read`
`IO.read`
`.read`
`complete()`
`.read`
`.INTR/.AGAIN`
`EINVAL`
`EIO`
`Storage.on_read`
`target_max`
`target_max`
`read_prepare_on_read`
`faulty.set(s); dirty.set(s)`

### Darwin — synchronous file IO in the drain

`IO.write` (darwin.zig:799-836) pushes the completion straight onto `completed` (:318); the drain
loop in `flush` (:151-153) invokes `on_complete` (:279-307) which calls `do_operation` →
`posix.pwrite` (:829) then `fs_sync(fd)` F\_FULLFSYNC (:830), then the user callback (:301).
`EWOULDBLOCK` requeues onto `io_pending` (:286-298) and kevent re-arms
(`EV_ADD|EV_ENABLE|EV_ONESHOT`, :172-179). Reads are likewise synchronous: `posix.pread`
(:600-605), `EINVAL`→`error.Alignment` (:613).

`IO.write`
`completed`
`flush`
`on_complete`
`do_operation`
`posix.pwrite`
`fs_sync(fd)`
`EWOULDBLOCK`
`io_pending`
`EV_ADD|EV_ENABLE|EV_ONESHOT`
`posix.pread`
`EINVAL`
`error.Alignment`

### Windows — overlapped IOCP

`IO.write` (windows.zig:944-977) pushes to `completed` (:326); drain (:166-171) invokes
`onComplete` (:286-313) → `do_file_io` (:848-893) → `WriteFile` with OVERLAPPED (:875).
`IO_PENDING` → `error.WouldBlock` (:884) → `io_pending += 1` (:297) and the OVERLAPPED rides IOCP.
Next flush: `GetQueuedCompletionStatusEx` into `[64]OVERLAPPED_ENTRY` (:130-140), completions
pushed to `completed` (:145-154), second `onComplete` polls `GetOverlappedResult` (:852-857).
`HANDLE_EOF` ⇒ return 0 (windows.zig:888-890; ReadFile/WriteFile do no partial IO).

`IO.write`
`completed`
`onComplete`
`do_file_io`
`WriteFile`
`IO_PENDING`
`error.WouldBlock`
`io_pending += 1`
`GetQueuedCompletionStatusEx`
`[64]OVERLAPPED_ENTRY`
`completed`
`onComplete`
`GetOverlappedResult`
`HANDLE_EOF`

Darwin:

Windows:

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality (constant + file:line) | Tear detection |
| --- | --- | --- | --- | --- | --- |
| Alignment gate | storage.zig:356 / :185 → vsr.zig:203-213 | `Zone.verify_iop` before every read/write; EINVAL otherwise guaranteed | n/a | n/a | asserts, not runtime repair |
| Enqueue | linux.zig:206-223 | `get_sqe` or flush-then-retry on SQ full | n/a | ring entries: 128 prod / 2048 format (main.zig:97-102) | SQ-full is self-healing, warned :210 |
| SQE prep | linux.zig:437-443, :412-417 | `prep_write`/`prep_read` capped by `buffer_limit` = `0x7ffff000` (io.zig:29) | n/a | 1 buffer ≤ 0x7ffff000 B | none |
| Correlation | linux.zig:446 | `sqe.user_data = @intFromPtr(completion)` | n/a | 1 pointer per SQE | none |
| Submission | linux.zig:122-168, :225-268 | `io_uring_enter` with `IORING_ENTER_EXT_ARG`; `wait_nr>0` only when blocking on a deadline | n/a (no fsync SQE in data path) | all queued SQEs per enter | `EINTR` retry :144 |
| Harvest | linux.zig:171-194 | `copy_cqes` into stack array; push onto `completed` linked list (avoids recursion :185-188) | n/a | `[256]io_uring_cqe` per call (linux.zig:172) | none — CQE is not a durability proof by itself; durability is O\_DSYNC per-write |
| Dispatch | linux.zig:196-204 → :449-768 | pop one completion, errno→typed error, invoke callback. Callbacks are plain Zig fn pointers (linux.zig:375-379), no `callconv(.c)` | n/a | 1 callback per pop | none |
| Write result | storage.zig:389-441 | partial writes advance `write.offset`/`buffer` and resubmit :427-440; 0-byte write panics :416-425 | flush = O\_DSYNC at write CQE (per-write) | per-write; WAL budget 32 in-flight (config.zig:117) | fail-stop: panic/fatal on error, see §5 |
| Read result | storage.zig:241-346 | EIO: halve `target_max` and retry same offset (:268-276); ≤1 sector ⇒ zero the sector :277-300; success at floor ⇒ +1 sector AIMD :336-343 | n/a | window shrinks to 4096 (`constants.sector_size`, src/constants.zig:489) | EIO zeroing is the LSE detector; downstream checksum judges the zeroed bytes |
| Journal second write | journal.zig:1856-1908 | header write issued only from body-write completion; header sector built by `header_sector` (:2123) | per-write O\_DSYNC (same mechanism) | 1 sector of headers = 16 headers (`headers_per_sector` journal.zig:47, sector\_size 4096 / @sizeOf(Header) 256) | entry-changed-while-writing races release with `wrote = null` (journal.zig:1869-1874, :1915-1948) — not an IO-error path |
| Slot clean | journal.zig:1953-1956 | `dirty.clear` then `faulty.clear` then release → `prepare_ok` can flow (replica.zig:11219-11227, gated by `has_prepare` journal.zig:540-549) | per-write (both writes' CQEs) | slot = op % 1024 (journal.zig:63, config.zig:158) | `prepare_ok` is the durability certificate |

`Zone.verify_iop`
`get_sqe`
`prep_write`
`prep_read`
`buffer_limit`
`0x7ffff000`
`sqe.user_data = @intFromPtr(completion)`
`io_uring_enter`
`IORING_ENTER_EXT_ARG`
`wait_nr>0`
`EINTR`
`copy_cqes`
`completed`
`[256]io_uring_cqe`
`callconv(.c)`
`write.offset`
`buffer`
`target_max`
`constants.sector_size`
`header_sector`
`headers_per_sector`
`wrote = null`
`dirty.clear`
`faulty.clear`
`prepare_ok`
`has_prepare`
`prepare_ok`

Darwin/Windows deltas:

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality | Tear detection |
| --- | --- | --- | --- | --- | --- |
| Darwin submission | darwin.zig:270-320 | file ops push straight to `completed`; the syscall runs inside the drain, not the kernel | per-write: `pwrite` :829 then `fs_sync` F\_FULLFSYNC :830 | drain loop unbounded per flush; kevent batch `[256]` (darwin.zig:102) | none at IO layer |
| Darwin requeue | darwin.zig:286-298 | EWOULDBLOCK → `io_pending`, kevent `EV_ADD|EV_ENABLE|EV_ONESHOT` re-arm (:172-179) | n/a | n/a | none |
| Windows start | windows.zig:848-893 | first call starts overlapped IO; `IO_PENDING`→WouldBlock; second call polls `GetOverlappedResult` (:852-857) | per-write `FILE_WRITE_THROUGH` (windows.zig:1277) | GQCS batch `[64]` (windows.zig:130) | none at IO layer |
| Windows EOF | windows.zig:888-890 | `HANDLE_EOF` ⇒ return 0 (ReadFile/WriteFile do no partial IO) | n/a | n/a | 0-byte read → `on_read` zeroes the rest (storage.zig:330-334) |

`completed`
`pwrite`
`fs_sync`
`[256]`
`io_pending`
`EV_ADD|EV_ENABLE|EV_ONESHOT`
`IO_PENDING`
`GetOverlappedResult`
`FILE_WRITE_THROUGH`
`[64]`
`HANDLE_EOF`
`on_read`

Callouts:

`wal_prepares`
`write_sectors`
`write_prepare_header`
`wal_headers`
`@L`
`@K`
`F_FULLFSYNC`
`pwrite`
`vsr_checkpoint_ops`
`io_entries`
`copy_cqes`
`journal_iops_write_max = 32`
`journal_iops_read_max = 8`
`grid_iops_read_max = 32`
`grid_iops_write_max = 32`
`client_replies_iops_read_max = 1`
`client_replies_iops_write_max = 2`
`pipeline_prepare_queue_max = 8`
`iops_write_max`
`dirty.clear`
`has_prepare`
`!dirty`
`prepare_ok`
`faulty`
`!faulty`
`faulty`
`dirty`

Test evidence:

`src/io/test.zig:25-155`
`IO.write`
`IO.read`
`src/io/test.zig:413-458`
`IO.init(1,0)`
`enqueue`
`src/io/test.zig:835-875`
`src/storage_fuzz.zig`
`storage`
`src/testing/io.zig`
`error.InputOutput`
`src/vsr/replica_test.zig:75`
`testing/io.zig`

## 5. Fail-stop completion semantics

### Writes: fail-stop, no error delivery

`Completion.complete()`
`.write`
`IO.WriteError!usize`
`Storage.on_write`
`error.InputOutput`
`@panic("latent sector error: no spare sectors to reallocate")`
`error.NoSpaceLeft`
`vsr.fatal(.no_space_left, "write failed: no space left on device…")`
`@panic("impossible write")`
`bytes_written == 0`
`@panic("write operation returned 0 bytes written")`
`write.callback(write)`
`Storage.Write`
`callback: *const fn (write: *Storage.Write) void`
`wrote = null`

### Reads: error-tolerant, corruption → repair

`Storage.on_read`
`@panic("impossible read")`
`journal.faulty.set(s); journal.dirty.set(s)`
`journal_header == null and !faulty`
`dirty and !faulty`
`error.WALInvalid`

| Step | Code (file:line) | Mechanism | Flush point & trigger | Batch cardinality | Tear detection |
| --- | --- | --- | --- | --- | --- |
| EIO on write | storage.zig:393-395 | assumes drive-level spare-sector reallocation; a surfaced EIO means spares exhausted | n/a (error surfaced at write CQE) | n/a | none — stop the world |
| ENOSPC on write | storage.zig:398-406 | `vsr.fatal(.no_space_left, …)` with offset/size; logical space is bounded by `--limit-storage`, not handled here | n/a | n/a | none |
| Impossible write error | storage.zig:407-413 | unknown errno ⇒ `@panic("impossible write")` | n/a | n/a | none |
| 0-byte write | storage.zig:416-425 | panic — "block devices are known to exhibit this behavior in the wild" | n/a | n/a | none |
| Partial write | storage.zig:427-440 | advance offset/buffer, `start_write` again until empty | per-write remainder, same O\_DSYNC/write-through flag | remainder < `buffer_limit` (io.zig:29-31) | none |
| No error union | storage.zig:80, :350, :436 | `Storage.Write.callback` takes only `*Storage.Write`; `on_write` consumes the error union internally | n/a | n/a | n/a |
| Read EIO halving | storage.zig:244-276 | recursive binary search for the failing sector: `target_max = ceil(target_sectors/2) * 4096`, retry same offset | n/a | window halves to ≥1 sector (4096) | the halving loop itself |
| Read EIO floor | storage.zig:277-300 | zero the single unreadable sector; "treat these EIO errors the same as a checksum failure" | n/a | 1 sector = 4096 | downstream checksum |
| Read AIMD | storage.zig:336-343 | success at `target_max == 4096` ⇒ grow by one sector | n/a | n/a | n/a |
| Corruption classification | journal.zig:871-914 | five named reasons: corrupt header / wrong cluster / op changed / checksum changed / corrupt body / corrupt sector padding | n/a | n/a | header checksum + body checksum over `body_used()` + zero-padding check |
| faulty bit | journal.zig:921-924 | `faulty.set(s); dirty.set(s)` for the slot matching (op, checksum) — read path ONLY | n/a | n/a | this IS the detector |
| nack exclusion | replica.zig:8817-8835 | nack requires `!faulty` (cases 1-2) or a *different* prepare (case 3); possibly-acked ops survive view change | n/a | nack quorum `quorum_nack_prepare` (replica.zig:1161) | n/a |

`vsr.fatal(.no_space_left, …)`
`--limit-storage`
`@panic("impossible write")`
`start_write`
`buffer_limit`
`Storage.Write.callback`
`*Storage.Write`
`on_write`
`target_max = ceil(target_sectors/2) * 4096`
`target_max == 4096`
`body_used()`
`faulty.set(s); dirty.set(s)`
`!faulty`
`quorum_nack_prepare`

Callouts:

`Storage.on_write`
`vsr.fatal(.no_space_left, …)`
`Storage.Write`
`faulty`
`journal_header == null and !faulty`
`dirty and !faulty`
`wrote = null`
`has_prepare`
`!dirty`

Test evidence:

`on_write`
`src/storage_fuzz.zig`
`Storage`
`src/vsr/replica_test.zig:75`
`src/vsr/replica_test.zig:846`
`result catch @panic(...)`

## 6. The other three writers

### 6.1 The grid (LSM block store)

`block_size`
`(address, checksum)`
`BlockReference`
`src/vsr.zig:215-227`
`grid.cache_upsert(completed_write.address, completed_write.block.*)`
`cache_write`
`cache_read`
`cache_write`
`(address, checksum)`
`on_get_blocks`
`on_block`
`(address, checksum)`
`grid_iops_read_max = 32`
`grid_iops_write_max = 32`

### 6.2 Client replies

The latest reply per active client is written asynchronously and coalesced — "Subsequent writes
for the same client may be coalesced – we only care about the last reply to each client session"
— and "ClientReplies guarantees that the latest replies are durable at checkpoint"
(`src/vsr/client_replies.zig:10-13`; only non-empty bodies stored, :7). Budgets:
`client_replies_iops_read_max = 1` / `client_replies_iops_write_max = 2` (config.zig:118-119).

`src/vsr/client_replies.zig:10-13`
`client_replies_iops_read_max = 1`
`client_replies_iops_write_max = 2`

### 6.3 The superblock

`superblock_copy_size * copy`
`superblock_copy_size`
`superblock_copies`
`@sizeOf(SuperBlockHeader)`
`sector_size`
`clients_max - pipeline_prepare_queue_max`
`pipeline_prepare_queue_max = 8`
`superblock_copy_size`
`superblock_zone_size = superblock_copy_size * superblock_copies`
`Threshold.count`
`sequence`
`parent`

## 7. When durable state is written (the flush calendar)

| Event | Write | Durable when |
| --- | --- | --- |
| every prepare, every replica | 2 writes: body into `wal_prepares` (journal.zig:1851), then redundant header into `wal_headers` from the body's completion callback (:1908) | per-write O\_DSYNC / F\_FULLFSYNC / FILE\_WRITE\_THROUGH completion; slot clean (journal.zig:1953) gates `prepare_ok` |
| commit | **none** | `send_commit` is a network message only (replica.zig:11370-11380); backups advance volatile commit state from prepare piggyback + 500 ms heartbeats (replica.zig:1376-1380, `tick_ms` config.zig:120) |
| every 960 ops (`vsr_checkpoint_ops`, src/constants.zig:40-42) | checkpoint: client replies, client sessions, grid, then superblock (all 4 copies, 3/4 verify) | superblock verify quorum (superblock\_quorums.zig:61-63) |
| entering/leaving a view change, restart | superblock (`view`, `log_view`, JV/View headers) | before the view is advertised (superblock.zig:1001-1004) |
| client replies | async, coalesced writes to the `client_replies` zone (client\_replies.zig:10-13) | durable at each checkpoint |

`wal_prepares`
`wal_headers`
`prepare_ok`
`send_commit`
`tick_ms`
`vsr_checkpoint_ops`
`view`
`log_view`
`client_replies`

## 8. Failure model

`Storage.on_write`
`Storage.Write`
`faulty`
`src/constants.zig:497-499`
`docs/ARCHITECTURE.md:89-90`

## 9. Constants (verified with definitions)

| Constant | file:line | Value | Meaning |
| --- | --- | --- | --- |
| `sector_size` | src/constants.zig:489 | `4096` | logical sector / direct-I/O alignment floor |
| `block_size` | src/config.zig:161 | `512 * KiB` | grid block / grid-zone alignment |
| `journal_slot_count` | src/config.zig:158 | `1024` | WAL slots; slot = op % 1024 (journal.zig:63) |
| `message_size_max` | src/config.zig:159 | `1 * MiB` | prepare slot size |
| `journal_size_headers` | src/constants.zig:197 | `1024 * @sizeOf(Header)` = 256 KiB | headers ring |
| `journal_size_prepares` | src/constants.zig:198 | `1024 * message_size_max` = 1 GiB | prepares ring |
| `@sizeOf(Header)` | src/vsr/message\_header.zig:72 | `256` (asserted) | redundant header; 16 per 4 KiB sector (journal.zig:47) |
| `buffer_limit` Linux | src/io.zig:29 | `0x7ffff000` | max single write/read syscall length |
| `buffer_limit` Darwin | src/io.zig:30 | `maxInt(i32)` | Darwin EINVAL cap |
| `buffer_limit` else | src/io.zig:31 | `maxInt(isize)` | POSIX limit |
| `journal_iops_read_max` | src/config.zig:116 | `8` | WAL read IOP budget |
| `journal_iops_write_max` | src/config.zig:117 | `32` | WAL write IOP budget |
| `client_replies_iops_read_max` | src/config.zig:118 | `1` | client-replies read IOPs |
| `client_replies_iops_write_max` | src/config.zig:119 | `2` | client-replies write IOPs |
| `grid_iops_read_max` | src/config.zig:130 | `32` | grid read IOPs |
| `grid_iops_write_max` | src/config.zig:131 | `32` | grid write IOPs |
| `pipeline_prepare_queue_max` | src/config.zig:155 | `8` | pipelined prepares in flight |
| `io_entries` | src/tigerbeetle/main.zig:97-102 | `2048` format/recover, `128` else | io\_uring ring size (`IO.init(entries, 0)` :103) |
| CQE harvest batch | src/io/linux.zig:172 | `[256]io_uring_cqe` | completions copied per `copy_cqes` |
| kevent batch | src/io/darwin.zig:102 | `[256]posix.Kevent` | change/event batch |
| IOCP batch | src/io/windows.zig:130 | `[64]OVERLAPPED_ENTRY` | completions per GQCS call |
| `IORING_FEAT_EXT_ARG` | src/io/linux.zig:58-61 | kernel ≥ 5.11 | inline `io_uring_enter` timeout; panic otherwise |
| flock retry | src/io/linux.zig:1566-1583 | 5 × 50 ms | io\_uring fd-lifetime workaround |
| `tick_ms` | src/config.zig:120 | `10` | run-loop cadence at the VSR layer; commit heartbeat = 500 ms (replica.zig:1376-1380) |
| `vsr_checkpoint_ops` | src/constants.zig:40-42 | 1024 − 32 − 32 = `960` | checkpoint interval (op-count flush trigger above the IO layer) |
| `superblock_copies` | src/config.zig:160 | `4` (even 4–8 asserted, src/constants.zig:556-558; {4,6,8} superblock.zig:595-597) | on-node superblock copies |
| `superblock_copy_size` | src/vsr/superblock.zig:613 | 24 KiB (8 KiB header :101-102 + 16 KiB padding :607-611) | per-copy size; offsets `copy * superblock_copy_size` (:1151) |
| `superblock_zone_size` | src/vsr/superblock.zig:602 | 96 KiB | 4-copy superblock zone |
| verify threshold | src/vsr/superblock\_quorums.zig:61-63 | 4→**3**, 6→4, 8→5 | superblock write/verify quorum |
| open threshold | src/vsr/superblock\_quorums.zig:68-70 | 4→**2**, 6→3, 8→4 | superblock open quorum |
| `iops_write_max` (aggregate) | src/constants.zig:505-506 | journal 32 + client-replies 2 + grid 32 + superblock 1 | per-process write IOP cap |

`sector_size`
`4096`
`block_size`
`512 * KiB`
`journal_slot_count`
`1024`
`message_size_max`
`1 * MiB`
`journal_size_headers`
`1024 * @sizeOf(Header)`
`journal_size_prepares`
`1024 * message_size_max`
`@sizeOf(Header)`
`256`
`buffer_limit`
`0x7ffff000`
`buffer_limit`
`maxInt(i32)`
`buffer_limit`
`maxInt(isize)`
`journal_iops_read_max`
`8`
`journal_iops_write_max`
`32`
`client_replies_iops_read_max`
`1`
`client_replies_iops_write_max`
`2`
`grid_iops_read_max`
`32`
`grid_iops_write_max`
`32`
`pipeline_prepare_queue_max`
`8`
`io_entries`
`2048`
`128`
`IO.init(entries, 0)`
`[256]io_uring_cqe`
`copy_cqes`
`[256]posix.Kevent`
`[64]OVERLAPPED_ENTRY`
`IORING_FEAT_EXT_ARG`
`io_uring_enter`
`tick_ms`
`10`
`vsr_checkpoint_ops`
`960`
`superblock_copies`
`4`
`superblock_copy_size`
`copy * superblock_copy_size`
`superblock_zone_size`
`iops_write_max`

AOF is optional; its mechanics are in the appendix of the on-disk report.

**One paragraph:** TigerBeetle 0.17.9's IO system is a single fixed-zone data file accessed
exclusively through direct, synchronous, 4-KiB-aligned writes (`O_DIRECT|O_DSYNC` linux.zig:1432,
`F_FULLFSYNC` per pwrite darwin.zig:829-830, `FILE_WRITE_THROUGH` windows.zig:1277) driven by an
io\_uring event loop with per-zone IOP budgets (`journal_iops_write_max` config.zig:117 et al.) —
two durable writes per prepare (journal.zig:1851, :1908) gate every `prepare_ok`
(journal.zig:1953 → replica.zig:8665), commit is flush-free (replica.zig:11370-11380), the
superblock's 4 on-node copies anchor checkpoints every 960 ops (constants.zig:40-42) and view
changes with a 3-of-4 write quorum (superblock\_quorums.zig:61-63), reads are checksum-verified and
repaired from peers instead of failing (grid.zig:1375-1380) — so write completion is durability,
and the network is only ever asked for repair, never for belief.

`O_DIRECT|O_DSYNC`
`F_FULLFSYNC`
`FILE_WRITE_THROUGH`
`journal_iops_write_max`
`prepare_ok`

|  |  |
| --- | --- |
|  | {"role":"user","content":"RESEARCH TASK (read-only; do not write or modify any files, do not commit anything). You are reverse engineering the replication protocol of TigerBeetle release 0.17.9 from its source code, which is unzipped at:\n\n/Users/Shared/lua-lunet/vrr-core/.tmp/tigerbeetle-release/tigerbeetle-0.17.9/\n\nIt is a Zig codebase. Key locations to start (verify, don't assume): src/vsr.zig (Command enum at line 230), src/vsr/replica.zig, src/vsr/journal.zig, src/vsr/superblock.zig, src/vsr/grid.zig, src/vsr/sync.zig, src/vsr/message\_header.zig, src/message\_bus.zig, src/state\_machine/, src/io/ (io\_uring/kqueue/direct IO), docs/ in the repo root, src/constants.zig. Also src/vsr/replica\_test.zig and src/testing/ may reveal intended semantics, but ground all claims in production code paths.\n\nYour job: report EXACTLY what the latest code does, with file:line evidence (relative paths like src/vsr/replica.zig:1234) and short code quotes for every load-bearing claim. No speculation — if code is unclear, say so. This is research only; return your findings as your final message.\n\nAnswer these sections, in order:\n\nA. MESSAGE SET: Enumerate the full Command enum (src/vsr.zig:230) — every message type, its direction (client↔replica, replica↔replica), when it is sent, and the handler function in replica.zig (file:line) that processes it. Note any messages that are TigerBeetle-specific (cluster/cross-cluster, prober/keepalive, catchup/fetch, handoff, etc.) vs the classic VSR set (request, prepare, prepare\_ok, commit, reply, view\_change, do\_view\_change/start\_election, start\_view, keepalive...).\n\nB. NORMAL-CASE FLOW AND DISK WRITES (THE MOST IMPORTANT SECTION): Trace the full path of a client request batch through the primary: request → prepare → prepare\_ok → commit → reply. For EVERY step identify: (1) what is written to disk (journal/WAL appends, commit records, header writes), (2) what triggers the actual write syscall path — direct I/O? io\_uring linked write+fsync (look in src/io/ and src/linux/), kqueue on macOS, F\_FULLFSYNC, O\_DSYNC, writev — and (3) which steps BLOCK on durable storage (sync) before the replica proceeds or sends a message. The key question: does a backup sync the prepare batch to disk BEFORE sending prepare\_ok? Cite the exact call chain (handler → journal open\_batch/append → write path → completion callback → send prepare\_ok). Then: what happens at COMMIT time — is a commit record written to the journal, is it on the critical path, is it batched/lazy (look for commit\_min/commit\_max, 'seal', 'commit batch' logic in journal.zig)? How does batching amortize syncs (how many requests per prepare batch / commit batch)?\n\nC. JOURNAL/WAL FORMAT: Describe journal.zig's on-disk layout: header sectors, commit batch records, prepare batch records, seals (if any), checksums, and the two-sided header (header at both ends of the journal?). What does the header store (committed prefix / checkpoint / view numbers / op numbers)? What is the role of superblock.zig and grid.zig (block device abstraction, two grid copies? repair?)? What are 'seals' — when are they written and what do they fence?\n\nD. VIEW CHANGE: Trace the view-change code paths: view\_change (or start\_election/do\_view\_change in this codebase's naming), start\_view, how a new primary is elected, how the committed prefix is determined from the responses, how the new primary reconciles logs (does it overwrite/truncate its own log suffix beyond the committed prefix — look for the handling of a new primary whose journal has a 'dirty' suffix beyond commit\_max, e.g., in on\_start\_view / start\_view\_as\_primary paths), and crucially: ANY disk writes during view change (seal writes before starting an election? writes of new-view state when assuming primary?). Is the log suffix beyond the committed prefix at the new primary discarded/truncated, and where in code?\n\nE. RECOVERY / RESTART: What happens when a replica process restarts: superblock/journal open, header repair (there may be a header\_repair or grid repair path), how the durable committed prefix is determined vs possibly-uncommitted suffix, how the replica rejoins (must it trigger or await a view change? does it fetch/catch up from the primary — look for fetch/catchup message handling and truncate logic), and what guarantees acknowledged (replied) client requests survive crashes (quorum + durable prepare argument as embodied in code). What happens on a full-cluster restart (all replicas down) — does the cluster recover from journals alone? Look for anything about 'recovery' / 'rejoin' / 'catch\_up' in replica.zig and docs.\n\nF. CLASSIFICATION (analysis grounded strictly in sections A-E): Classify TigerBeetle 0.17.9's protocol relative to: (1) VSR-1988 — Oki & Liskov PODC 1988, the original Viewstamped Replication, where the original paper has replicas write prepared operations to stable storage (disk) before acknowledging prepare — pay particular attention to where TigerBeetle's disk flushes match or differ from that; (2) VRR-2012 — Liskov & Cowling 'Viewstamped Replication Revisited' (MIT-CSAIL-TR-2012-021), which eliminated disk writes in normal operation (volatile state, disk only at view change/recovery, with state-transfer-to-disk rejoin) and whose protocol has the problems documented in Appendix B.1 of UW TR UW-CSE-17-08-01 ('Recovering Shared Objects Without Stable Storage'); (3) TigerBeetle-specific mechanisms absent from both papers (seals, two-sided header repair, direct-IO WAL, batched commit records, client sessions, cluster messages, superblock/grid). State clearly which base design TigerBeetle follows on the disk-flush axis (durable-at-prepare like 1988? or no-disk normal operation like 2012?), and whether the VRR-2012 Appendix-B.1-style failure modes (which arise from non-durable prepares and log reconciliation from volatile state) are structurally excluded by what the code does, and why or why not, citing code.\n\nG. QUORUM & CONFIG: Replication factor options and quorum sizes (constants.zig), view/op/session numbering, client sessions (request IDs, session eviction — src/vsr/client\_sessions.zig), keepalive/ping-pong machinery (prober, fault\_detector.zig), and just note the existence/purpose of cluster (cross-cluster) messages without a deep dive.\n\nDeliverable format: a single structured markdown report, sections A–G, every claim with file:line references and short verbatim code quotes for the critical lines (the prepare\_ok send after sync; the actual sync calls; seal writes; header repair; truncation of dirty suffix at start\_view). Include exact function names. Where docs/ in the repo states something the code contradicts, note it explicitly. End with a one-paragraph verdict: 'TigerBeetle 0.17.9 implements ...' — your best precise characterization of its flavour of Viewstamped Replication on the disk-durability axis. Be thorough; this report is the evidence base for a formal write-up."} |
|  | {"role":"assistant","content":"My grep accidentally used `-r` (replace). Let me redo it properly:"} |
|  | {"role":"assistant","content":"All evidence gathered. Here is the report.\n\n---\n\n# Reverse-Engineering Report: TigerBeetle 0.17.9 Replication Protocol\n\nSource: `/Users/Shared/lua-lunet/vrr-core/.tmp/tigerbeetle-release/tigerbeetle-0.17.9/` (read-only). All paths relative to repo root. Line numbers verified against this exact tree.\n\n---\n\n## A. MESSAGE SET\n\nThe `Command` enum is at `src/vsr.zig:230`:\n\n```zig\npub const Command = enum(u8) {\n reserved = 0,\n ping = 1, pong = 2,\n ping\_client = 3, pong\_client = 4,\n request = 5, prepare = 6, prepare\_ok = 7,\n reply = 8, commit = 9,\n exit\_view = 10, join\_view = 11, get\_view = 13,\n get\_headers = 14, get\_prepare = 15, get\_reply = 16,\n get\_blocks = 19,\n headers = 17,\n eviction = 18,\n block = 20,\n view = 24,\n deprecated\_12 = 12, // .view without checkpoint\n deprecated\_21 = 21, // .request\_sync\_checkpoint\n deprecated\_22 = 22, // .sync\_checkpoint\n deprecated\_23 = 23, // .view with an older version of CheckpointState\n```\n\nDispatch is a single switch in `Replica.on\_message` (`src/vsr/replica.zig:1802-1834`); every command resolves to one handler:\n\n| Command | Direction | When sent | Handler (`src/vsr/replica.zig`) |\n|---|---|---|---|\n| `ping`/`pong` | replica↔replica | clock synchronization probes + release advertisement (upgrade candidates) | `on\_ping` :1849, `on\_pong` :1898 |\n| `ping\_client`/`pong\_client` | client→replica / replica→client | client learns current view | `on\_ping\_client` :1919; `pong\_client` is never handled by a replica (`on\_message` :1822 routes it to \"misdirected message\") |\n| `request` | client→primary | client batch submission | `on\_request` :1944 |\n| `prepare` | primary→replicas+standbys (star broadcast) | per client request (one op) | `on\_prepare` :2021 (also reused by the primary for its own prepare via loopback, `primary\_pipeline\_prepare` :7444) |\n| `prepare\_ok` | replica→primary | after the prepare is \*\*durably\*\* in the WAL (see §B) | `on\_prepare\_ok` :2248 |\n| `reply` | primary (and one random backup)→client; replica→replica for repair | after commit; repair of the durable reply cache | `on\_reply` :2342 |\n| `commit` | primary→backups+standbys | heartbeat (~500 ms, `on\_commit\_message\_timeout` :3691) | `on\_commit` :2396 |\n| `exit\_view` | replica→all replicas | fault-detector alarm (backup lost commit heartbeat :1630; or view-change retry) — the classic \*start-view-change\* | `on\_exit\_view` :2549 |\n| `join\_view` | replica→all replicas | on entering `status=view\_change` — the classic \*do-view-change\* | `on\_join\_view` :2608 |\n| `view` | primary→backups | new primary completes view change, or catch-up reply to `get\_view` — the classic \*start-view\* | `on\_view` :2759 |\n| `get\_view` | backup→primary | learn new view / recover head (`on\_view` :2763-2777 special-cases `recovering\_head` with a nonce freshness check) | `on\_get\_view` :3022 |\n| `get\_headers`/`headers` | replica↔replica | journal-header (hash-chain) repair | `on\_get\_headers` :3159, `on\_headers` :3300 |\n| `get\_prepare` | replica↔replica | WAL prepare repair (backup or new primary); no nack on corrupt (CTRL protocol, docs `docs/internals/vsr.md:218`) | `on\_get\_prepare` :3056, `on\_get\_prepare\_read` :3127 |\n| `get\_reply` | replica↔replica | client-reply-zone repair | `on\_get\_reply` :3213 |\n| `get\_blocks`/`block` | replica↔replica | LSM grid-block repair/state-sync | `on\_get\_blocks` :3324, `on\_block` :3453 |\n| `eviction` | primary→client | client session evicted | not handled by replicas (:1822 \"misdirected\") |\n\n\*\*Classic VSR set vs TigerBeetle-specific.\*\* `request/prepare/prepare\_ok/commit/reply` and `exit\_view/join\_view/view` are the classic VSR set (with TigerBeetle's renaming — `docs/internals/vsr.md:17`: \"uses `JoinView`/`View`/`ExitView` instead of `DoViewChange`/`StartView`/`StartViewChange`\"). TigerBeetle-specific: the entire repair/catch-up family (`get\_view/view/get\_headers/headers/get\_prepare/get\_reply`), the grid-repair pair (`get\_blocks/block`), the client-session machinery (`eviction`, register operation), the clock/liveness pings, and the `commit` heartbeat doubling as a fault-detector signal (`src/vsr/fault\_detector.zig:17-18`: \"primary broadcasts Commit messages periodically, which are interchangeable with Prepare messages as a signal that the primary is alive\").\n\n\*\*Cross-cluster:\*\* there are \*\*no cluster↔cluster (cross-cluster) replication messages in 0.17.9\*\*. `cluster` is a `u128` identifier embedded in every header \"so that a client or replica can indicate the cluster it believes it is speaking to, instead of accidentally talking to the wrong cluster\" (`src/vsr/message\_header.zig:29-32`). The reserved `reconfigure` operation exists with full request validation (`src/vsr.zig:417-504`) and `execute\_op\_reconfiguration` (:5593), but `docs/internals/vsr.md:273-275` marks the protocol \"TODO (Unimplemented)\" — the docs lag the code's scaffolding (member-set change machinery exists; only same-count member permutation is allowed, `src/vsr.zig:472-473`).\n\n---\n\n## B. NORMAL-CASE FLOW AND DISK WRITES\n\n### B1. request → prepare (primary)\n\n`on\_request` (:1944) → `primary\_pipeline\_prepare` (:7283). The primary builds a `Header.Prepare` \*\*reusing the request buffer\*\* (\"Reuse the Request message as a Prepare message by replacing the header\", :7369), with op = `self.op + 1`, the hash-chain parent, and — importantly — the current commit point piggybacked:\n\n```zig\n.op = self.op + 1,\n.commit = self.commit\_max, // src/vsr/replica.zig:7393-7394\n```\n\nThen (:7443-7444):\n\n```zig\nself.pipeline.queue.push\_prepare(message);\nself.on\_prepare(message);\n```\n\nThe primary handles its own prepare through the same `on\_prepare` path as backups. `on\_prepare` (:2021) first \*\*broadcasts\*\* (star topology, not a chain):\n\n```zig\n// :2008-2009: \"The primary broadcasts this prepare message to every other replica and standby,\n// in parallel to writing to its own journal.\"\nif (self.status == .normal and self.primary()) {\n self.send\_message\_to\_other\_replicas\_and\_standbys(message.base()); // :8550-8552\n}\n```\n\nthen updates in-memory state and starts its own journal write (:2242-2245):\n\n```zig\nself.op = message.header.op;\nself.journal.set\_header\_as\_dirty(message.header); // in-memory only; marks the slot dirty\nself.append(message);\n```\n\n> \*\*Docs/code contradiction:\*\* `docs/internals/vsr.md:73-75` still describes \*chain\* replication (\"Forward the prepare to the next replica in the chain\", mermaid `R1->>R2`). The code is \*\*star\*\* replication: only the primary ever forwards, and the code comment (:2011) says \"Star replication prioritizes latency: with Flexible Paxos the primary can commit after the fastest prepare\_ok quorum returns.\"\n\n### B2. prepare → durable WAL write → prepare\_ok (THE KEY QUESTION: \*\*yes\*\*)\n\nCall chain on a backup (and identically for the primary's own copy):\n\n1. `on\_prepare` (:2021) → `append` (:4210) → `write\_prepare` (:11178) → `journal.write\_prepare` (`src/vsr/journal.zig:1789`).\n2. `journal.write\_prepare` slices the message to sector granularity and issues the \*\*first durable write\*\* — the prepare body into the `wal\_prepares` ring (`src/vsr/journal.zig:1842-1851`):\n\n```zig\nconst buffer = message.buffer[0..vsr.sector\_ceil(message.header.size)];\nconst offset = Ring.prepares.offset(slot);\njournal.write\_sectors(write\_prepare\_header, write, buffer, .prepares, offset);\n```\n\n3. Only when that write \*\*completes\*\*, the completion callback `write\_prepare\_header` (:1856) issues the \*\*second durable write\*\* — the redundant header sector into the `wal\_headers` ring (:1890-1908). The ordering is deliberate (`src/vsr/journal.zig:2144-2145`): \"we need to avoid writing (leaking) a redundant header before its corresponding prepare is on disk.\"\n4. When the header write completes, `write\_prepare\_on\_write\_header` (:1911) marks the slot clean:\n\n```zig\njournal.dirty.clear(slot); // src/vsr/journal.zig:1953\njournal.faulty.clear(slot);\njournal.write\_prepare\_release(write, message); // :1956\n```\n\n5. The release invokes the replica callback `write\_prepare\_callback` (`src/vsr/replica.zig:11219-11227`):\n\n```zig\nfn write\_prepare\_callback(self: \*Replica, wrote: ?\*Message.Prepare) void {\n self.message\_bus.resume\_receive();\n // `null` indicates that we did not complete the write for some reason.\n const message = wrote orelse return;\n self.send\_prepare\_ok(message.header); // :11225 <-- AFTER the durable write\n```\n\n6. `send\_prepare\_ok` (:8615) re-checks durability before sending (:8665): `if (self.journal.has\_prepare(header)) { ... } else { \"not sending (dirty)\" }`, where `has\_prepare` requires the slot to be clean (`src/vsr/journal.zig:540-548`):\n\n```zig\nif (journal.slot\_with\_op\_and\_checksum(header.op, header.checksum)) |slot| {\n if (!journal.dirty.bit(slot)) { ... return true; }\n}\n```\n\nThe `prepare\_ok` carries `commit\_min` of the sender (:8704), which the primary uses to track backup lag (:2264-2271). Standbys replicate but never ack (:8672: `if (self.standby()) return;`, and :6924: \"standbys receive and replicate prepares, but never send prepare-oks\").\n\n\*\*So: the backup does NOT send `prepare\_ok` until both the prepare body (wal\_prepares ring) and the redundant header (wal\_headers ring) have completed durable writes.\*\* A duplicate prepare that is already clean is acked immediately (`on\_repair` :2510-2514).\n\n### B3. What makes a \"completed write\" durable (the syscall path)\n\nAll journal writes go through `Storage.write\_sectors` → per-OS IO:\n\n- \*\*Linux\*\* (`src/io/linux.zig`): the data file is opened `O\_DIRECT | O\_DSYNC` — `src/io/linux.zig:1539-1540`: `\"// This is critical as we rely on O\_DSYNC for fsync() whenever we write to the file:\" assert(flags.DSYNC);` and `:1508` `flags.DIRECT = true`. Writes are submitted as io\_uring `prep\_write` SQEs (`:437-443`); there are \*\*no linked SQEs, no separate `fsync`/`fdatasync` in the data path\*\* (`rg IOSQE|IO\_LINK` finds nothing; the only `.fsync` op use is at file open, `:1631`). Durability is per-write via `O\_DIRECT|O\_DSYNC` — `:1516-1517`: \"TigerBeetle uses Direct I/O to bypass the kernel page cache, \*\*to ensure that data is durable when writes complete\*\*.\"\n- \*\*macOS\*\* (`src/io/darwin.zig`): `O\_DSYNC` is not honored past the disk cache, so every `write` explicitly issues `F\_FULLFSYNC` after the `pwrite` (`:825-833`):\n\n```zig\nfn do\_operation(op: anytype) WriteError!usize {\n // In the current implementation, Darwin file IO ... is \_synchronous\_ ...\n const result = posix.pwrite(op.fd, op.buf[0..op.len], op.offset);\n try fs\_sync(op.fd);\n```\n\nwith `:1092-1099`: `\"Darwin's fsync() syscall does not flush past the disk cache. We must use F\_FULLFSYNC instead.\"` Page cache is disabled with `F\_NOCACHE` (`:1048-1050`). The file is `flock`ed and `F\_FULLFSYNC`ed at open (`:1078`).\n\nSo each of the two journal writes per op is a \*\*synchronous-to-stable-storage direct write\*\*; write completion == durability. Windows similarly relies on `O\_DSYNC`-equivalent write-through (`src/io/windows.zig:1279`).\n\n### B4. prepare\_ok quorum → commit → reply\n\n`on\_prepare\_ok` (:2248) counts unique senders via `count\_message\_and\_receive\_quorum\_exactly\_once` (:4120) against `threshold = self.quorum\_replication` (:2294), sets `prepare.ok\_quorum\_received` (:2319), and calls `commit\_pipeline` (:2339). `commit\_pipeline` (:4285) enters the manual state machine `commit\_dispatch` (:4374) with stages `start → check\_prepare → prefetch → stall → reply\_setup → execute → compact` (enum at `src/vsr/replica.zig:83-118`), plus `checkpoint\_durable/checkpoint\_data/checkpoint\_superblock` every `vsr\_checkpoint\_ops`.\n\n`execute\_op` (:5344) runs the state machine, then:\n\n```zig\nself.commit\_min += 1; // :5463\nself.advance\_commit\_max(self.commit\_min, @src());\n...\nself.send\_reply\_message\_to\_client(reply); // :5536\n```\n\nA randomly chosen backup also replies directly to the client for logical availability (:5328-5342, comment :5319-5325). Replies are additionally written (asynchronously, coalesced) to the dedicated `client\_replies` disk zone — `self.client\_replies.write\_reply(reply\_slot, reply, .commit)` (:5747 for register, :5785 for normal ops); per `src/vsr/client\_replies.zig:11-14`: \"Replies are written asynchronously... ClientReplies guarantees that the latest replies are durable at checkpoint.\"\n\n### B5. COMMIT: no commit record, no disk write on the commit path\n\n- \*\*No commit record is written to the journal.\*\* `send\_commit` (:11342) only sends a network message carrying `commit = self.commit\_max` and the committed entry's checksum (:11370-11380).\n- `on\_commit` on a backup (:2396) just verifies the checksum against its journal header (:2438-2449) and advances volatile state: `self.advance\_commit\_max(message.header.commit, ...); self.commit\_journal();` (:2451-2452). Zero disk writes.\n- Commit notification is \*\*lazy and batched\*\*: (a) piggybacked on every prepare header (`.commit = self.commit\_max`, :7394; backups consume it at :2107-2109), and (b) a periodic heartbeat every 500 ms — `commit\_message\_timeout` `after = 500 / constants.tick\_ms` (:1376-1380) → `on\_commit\_message\_timeout` (:3691) → `send\_commit`. The primary also injects extra commits to keep the fault detector's EWMA fed (`src/vsr/fault\_detector.zig:22-24`).\n- The commit point becomes \*\*durable\*\* only via checkpoints: every `vsr\_checkpoint\_ops` commits (default = `1024 − 32 − 32 = 960`, `src/constants.zig:40-42`, `src/config.zig:158,164`), `commit\_checkpoint\_data` (:4989) flushes aof/state machine/client replies/client sessions/grid, and `commit\_checkpoint\_superblock` (:5110) writes the \*\*superblock\*\* (all copies + read-verify, see §C), persisting `commit\_max`, checkpoint header, view/log\_view and the view-headers suffix (`superblock.checkpoint` `src/vsr/superblock.zig:891-987`).\n\n### B6. How batching amortizes syncs\n\n- One op = one client request message, which is itself a \*\*batch\*\* of up to `message\_size\_max = 1 MiB` (default, `src/config.zig:159`) — \"Each prepare can contain at most 8\_190 transfers\" (`src/constants.zig:640-641`). So the two durable writes per op are amortized across the whole client batch.\n- Pipelining: up to `pipeline\_prepare\_queue\_max = 8` prepares in flight (`src/config.zig:155`), up to `journal\_iops\_write\_max = 32` concurrent WAL writes (`src/config.zig:117`), so the WAL absorbs many ops concurrently; commit executes per op from the in-memory pipeline (primary, `commit\_start\_pipeline` :4578) or from the WAL/cache (backup, `commit\_start\_journal` :4606) without extra syncs.\n- Commit syncs are amortized to ~1 superblock write + grid flush per 960 ops.\n- Note for `replica\_count=1`: writes are strictly serialized to avoid WAL gaps (`append` :4219-4226: \"In a cluster-of-one, the prepares must always be written to the WAL sequentially... This ensures that there will be no gaps in the WAL during crash recovery\").\n\n---\n\n## C. JOURNAL / WAL FORMAT\n\nLayout (`src/vsr/journal.zig:16-25`):\n\n```\n\"The WAL consists of two contiguous circular buffers on disk:\n - `vsr.Zone.wal\_headers`\n - `vsr.Zone.wal\_prepares`\"\n```\n\n- \*\*Two rings\*\*, sized `journal\_slot\_count` (default 1024, `src/config.zig:158`): a `headers` ring of redundant prepare headers (`journal\_slot\_count \* @sizeOf(Header)` bytes, Header = 256 B, `src/vsr/message\_header.zig:72`; 16 headers per 4 KiB sector, `journal.zig:47`) and a `prepares` ring of prepare messages (`journal\_slot\_count \* message\_size\_max`).\n- \*\*Slot = `op % journal\_slot\_count`\*\* (`src/vsr/journal.zig:63`). The WAL wraps; overwriting is legal only when the overwritten op is fenced by the durable-checkpoint rules (`on\_prepare` :2217-2225; invariant block `src/constants.zig:44-74`).\n- \*\*Two-sided headers\*\*: every op is stored twice on disk — the header inside the prepare message in `wal\_prepares`, plus a \*redundant\* header in `wal\_headers`. The redundant copy is written \*\*only after\*\* the prepare body is durable (`journal.zig:2144-2145`), which lets recovery disentangle a crash/torn write between the two copies: e.g. `src/vsr/journal.zig:229-233` — if the redundant header were written first, \"if at this point the replica crashes & restarts, slot 6 is in case `@L` (decision=nil) which can be locally repaired. In contrast, if op 6's prepare header was written in step 3, it would be case `@K`, which requires remote repair.\"\n- \*\*Checksums everywhere\*\*: `checksum` covers the header, `checksum\_body` covers the body (`src/vsr/message\_header.zig:12-22`); recovery validates both plus zero sector padding (`recover\_prepare\_callback`, `journal.zig:1151-1161`). \"In each ring, the `op` for reserved headers is set to the corresponding slot index. This helps WAL recovery detect misdirected reads/writes.\" (`journal.zig:20-21`). Grid blocks are addressed by `(checksum, address)` pairs to defeat misdirected reads (`src/vsr.zig:216-227`).\n- \*\*Recovery decision table\*\* (`src/vsr/journal.zig:1204-1241`): a 16-case table (`@A`..`@P`) classifying each slot by header validity/reserved, prepare validity/reserved, and op/checksum/view match, deciding `vsr` (remote `get\_prepare` repair), `fix` (local header repair), `cut` (truncate torn prepare), `nil`, or `eql`. Torn prepares are truncated so the replica doesn't start in `recovering\_head` (:1278-1287, `torn\_prepares` :1359).\n- \*\*Seals: there are none.\*\* `rg -ni \"seal\"` over `src/vsr/` and `src/constants.zig` finds no protocol concept of seals in 0.17.9 (the term only appears in unrelated C# client code). WAL wrap is fenced by the \*\*checkpoint machinery\*\*, not seals: `vsr\_checkpoint\_ops`, `Checkpoint.durable(...)` predicates, and the comptime invariant `src/constants.zig:69-70` — \"to guarantee that a log entry from a previous checkpoint can be overwritten only when there is a quorum of replicas at the next checkpoint.\"\n- \*\*What the superblock stores\*\* (`src/vsr/superblock.zig:110-148`): `vsr\_state = { checkpoint: CheckpointState, replica\_id, members, commit\_max, sync\_op\_min/sync\_op\_max, log\_view, view, replica\_count }`, where `CheckpointState` (`:320-355`) holds the checkpoint \*\*header\*\* (an op + checksum: the durable committed prefix anchor), parent/grandparent checkpoint ids, LSM manifest block references, free-set references, client-sessions reference, storage size, release. Plus a \*\*`view\_headers\_all` suffix\*\* (`:87-97`): \"When `vsr\_state.log\_view < vsr\_state.view`, the headers are for a JV. When `=`, the headers are for a View.\" The superblock is stored as `superblock\_copies` (default 4, `src/config.zig:160`) hashed-chained copies (`sequence`, `parent`, `:71-78`), written \*\*all copies sequentially then read back and verified by flexible quorum\*\* (`write\_header` :1124-1198, `quorums.working(...)` :1264). Per `docs/internals/data\_file.md:75-77`: \"the superblock is physically stored as 4 distinct copies on disk. After startup, replica picks the latest superblock which has at least 2 copies written.\"\n- \*\*Grid\*\* (`src/vsr/grid.zig:26-29`): \"The Grid provides access to on-disk blocks (blobs of `block\_size` bytes). Each block is identified by an 'address' (u64, beginning at 1).\" It is the block-device abstraction for the LSM forest: free-set allocation, write-through cache, checkpoint trailers. There are \*\*not\*\* \"two grid copies\" — blocks are single-copy but checksum-addressed and repaired from peers via `get\_blocks`/`block` (docs `docs/internals/data\_file.md:28-41`), plus a long-cycle scrubber (`grid\_scrubber\_cycle`, `src/constants.zig:381-391`).\n\n---\n\n## D. VIEW CHANGE\n\nNaming (docs `docs/internals/vsr.md:17`): `exit\_view` = StartViewChange, `join\_view` = DoViewChange, `view` = StartView.\n\n\*\*Trigger.\*\* Backups run an EWMA `FaultDetector` (`src/vsr/fault\_detector.zig:1-24`); on heartbeat loss `tick\_normal\_heartbeat\_fault` broadcasts `exit\_view` (:1630). `on\_exit\_view` (:2549) waits for a \*\*view-change quorum\*\* of EVs (\"Wait until we have a view-change quorum of messages (possibly including ourself). This ensures that we do not start a view-change while normal request processing is possible.\" :2557-2559) then `transition\_to\_view\_change\_status(self.view + 1)` (:2590).\n\n\*\*Disk writes during view change — yes, superblock writes (the closest thing to a \"seal\" write):\*\*\n\n1. \*\*On entering the view change\*\*: `transition\_to\_view\_change\_status` (:10160) builds JV headers (`update\_join\_view\_headers` :10254), then (:10204-10210):\n\n```zig\nself.status = .view\_change;\n...\nself.view = view\_new;\nself.view\_durable\_update();\n```\n\n`view\_durable\_update` (:9448-9513) calls `superblock.view\_change(...)` (`src/vsr/superblock.zig:1010`), durably persisting `view`, `log\_view`, `commit\_max` and the JV/View header suffix (all superblock copies + quorum verify). Rationale, :10001-1006: \"to persist its view/log\_view — \*\*it cannot advertise either value until it is certain they will never backtrack\*\*.\"\n\n2. \*\*On completing the view change\*\* (backup installing View, or new primary assuming leadership): `transition\_to\_normal\_from\_view\_change\_status` (:10056) → `self.view\_durable\_update()` (:10125).\n\n\*\*New primary election and log reconciliation.\*\* Each replica broadcasts its `join\_view`; JV headers are per-op classified `present/missing/corrupt/blank/fault` with nack/present bitsets (`send\_join\_view` :8783-8833; docs `vsr.md:134-141`). Nacks are \*\*conservative\*\* — a replica nacks only ops it definitely did not durably prepare (nack case 2 :8824-8829 requires `dirty and !faulty`); a slot that was possibly written-then-lost (`faulty`) is never nacked — PAR's CTRL protocol (`docs/internals/vsr.md:218`, `src/vsr/journal.zig:1170-1171`).\n\n`on\_join\_view` (:2608): the primary of the new view collects `quorum\_view\_change` JVs, computes `commit\_max = max(self.commit\_max, JVQuorum.commit\_max(...))` (:2677-2680); a lagging primary may \*\*forfeit\*\* to a more up-to-date replica (:2682-2725). Then `primary\_set\_log\_from\_join\_view\_messages` (:9705) installs the quorum's canonical headers and truncates its own dirty suffix via `set\_op\_and\_commit\_max` (:9619):\n\n```zig\nself.op = op;\nself.journal.remove\_entries\_from(self.op + 1); // :9663-9664 — truncate suffix beyond new head\n```\n\nwith the safety invariant (:9632-9636): \"Uncommitted ops may not survive a view change, but \*\*never truncate committed ops\*\*\" (`assert(op >= @max(commit\_max, self.commit\_max))`). `remove\_entries\_from` (`src/vsr/journal.zig:1705`) resets the slots in memory; the stale bytes remain on disk but are fenced by the durable `log\_view` (see below). Ops nacked by a \*\*nack quorum\*\* (`quorum\_nack\_prepare = replica\_count - quorum\_replication + 1`, `src/vsr.zig:1207`) are truncated (\"If a new primary collects a \_nack quorum\_ of \_blank\_ headers for a particular possibly-uncommitted op, it truncates the log.\" `docs/internals/vsr.md:142`).\n\nThe primary then repairs its journal headers (`get\_headers`) and prepares (`get\_prepare`) — \"Then primary commits all prepares which are not known to be uncommitted\" (`docs/internals/vsr.md:156`) — and becomes primary via `primary\_start\_view\_as\_the\_new\_primary` (:9842), which re-populates the pipeline from the repaired WAL and sends prepare\_oks \*\*to itself\*\* (:9891: \"Send prepare\_ok messages to ourself to contribute to the pipeline\"), then broadcasts `view` (`primary\_send\_view` :9515).\n\n\*\*Backups\*\* reconcile in `on\_view` → `on\_view\_set\_journal` (:2913): they pick the first View header that fits their `op\_prepare\_max\_sync()`, call the same `set\_op\_and\_commit\_max` (truncating any dirty suffix, :2946-2950), `replace\_header` for each header (:2978-2982), transition to normal, and `send\_prepare\_oks\_after\_view\_change()` (:2995).\n\n\*\*Is the discard durable?\*\* The truncation itself is in-memory, but it is made durable by the superblock: a replica only advertises `log\_view` once it is durable (:9390-9393, \"A replica's advertised `log\_view` must never backtrack after a crash\" — with the full forked-log proof at :9395-9433), and on restart `recover\_slots` drops any journal header whose view is not in the durable log-view range (`src/vsr/journal.zig:1299-1332`: \"Discard headers which we are certain do not belong in the current log\_view... (These headers can originate if we join a view, write some prepares from the new view, and then crash before the view\_durable\_update() finished.)\").\n\n---\n\n## E. RECOVERY / RESTART\n\n\*\*Startup\*\* (`Replica.open`, :654): `superblock.open` (:688) reads all superblock copies and picks the latest with a valid copy-quorum; then `journal.recover` (:767 → `src/vsr/journal.zig:941`) reads the entire headers ring (`recover\_headers` :960) and the entire prepares ring (`recover\_prepare` :1092 — \"We load the entire message to verify that it isn't torn or corrupt\" :1127-1128), then `recover\_slots` (:1240) applies the decision table (§C), truncates torn prepares, and drops headers outside the durable log\_view.\n\n\*\*Head-op recovery\*\* (:778-867): if `log\_view == view` (crashed in normal status), the head is the max valid op in the WAL (:792-801); otherwise the head comes from the superblock's durable `view\_headers` (JV) (:802-856). Then:\n\n```zig\nif (self.log\_view == self.view) {\n if (self.op\_head\_certain()) {\n if (self.primary\_index(self.view) == self.replica) {\n self.transition\_to\_view\_change\_status(self.view + 1); // :894 — restarting PRIMARY must trigger a view change\n } else {\n self.transition\_to\_normal\_from\_recovering\_status(); // :896 — backup rejoins in normal\n }\n } else {\n self.transition\_to\_recovering\_head\_from\_recovering\_status(); // :899 — head uncertain, wait for View\n }\n} else {\n // durable JV: resume the view change // :905\n self.transition\_to\_view\_change\_status(self.view);\n}\n```\n\nA solo replica unconditionally increments its view and durably records it before serving (:872-885: \"Solo replicas must increment their view after recovery. Otherwise, two different versions of an op could exist within a single view... on\_request() will ignore incoming requests until the view\_durable\_update() completes.\").\n\n\*\*Rejoin / catch-up.\*\* A restarting backup does \*\*not\*\* need a view change: it comes back `status=normal` and catches up via the repair protocol (`get\_headers`/`headers` for the hash chain, `get\_prepare` for missing prepares, driven by `repair()` :7544 and `on\_journal\_repair\_timeout` :3771) and by consuming the `commit` field piggybacked on prepares/commit heartbeats. If it cannot determine its head (`recovering\_head`, :9905), it waits for a fresh `view` from the primary (nonce-checked :2763-2777). If it is more than a checkpoint behind, `on\_view\_set\_checkpoint` (:2851) triggers \*\*state sync\*\* — checkpoint + grid + client replies fetched via `get\_blocks`/`block`/`get\_reply` — rather than WAL repair (:2868-2882).\n\n\*\*Why acknowledged requests survive crashes.\*\* A client reply is sent only after commit, which requires a `quorum\_replication` of prepare\_oks (self included), and each prepare\_ok certifies a \*\*durable\*\* WAL write (§B2 — `has\_prepare` requires `!dirty`, and `dirty` clears only after both O\_DSYNC direct writes complete). Replication and view-change quorums intersect (`src/vsr/replica.zig:1176-1177`: \"Flexible quorums are safe if these two quorums intersect so that this relation holds: `assert(quorum\_replication + quorum\_view\_change > replica\_count)`\"), so any possibly-committed op is present, durably, in the next view-change quorum; truncation requires a nack quorum, which cannot form if the op reached a replication quorum (nacks are only sent for ops never durably written, §D). Replies themselves are additionally persisted in the `client\_replies` zone (durable at each checkpoint, `src/vsr/client\_replies.zig:14`) and repairable from peers.\n\n\*\*Full-cluster restart.\*\* Yes — the cluster recovers from journals + superblocks alone; there is no separate recovery protocol. `docs/internals/vsr.md:53-57`: \"Unlike [VRR], TigerBeetle does not implement Recovery Protocol (see §4.3). Instead, replicas persist their VSR state to the superblock. This ensures that a recovering replica never backtracks to an older view.\" The restart-time view change (primary-elect at :894, or resumed JV at :905) is exactly the mechanism that reconciles the cluster, using the durable view/log\_view in every replica's superblock to prevent view backtracking (:9385-9433).\n\n---\n\n## F. CLASSIFICATION\n\n\*\*(1) vs VSR-1988 (Oki & Liskov, PODC'88).\*\* TigerBeetle matches the 1988 design on the disk-durability axis: replicas write prepared operations to stable storage \*\*before\*\* acknowledging the prepare. The code enforces the 1988 invariant mechanically: `write\_prepare\_callback` sends `prepare\_ok` only after both journal writes complete (`src/vsr/replica.zig:11219-11227`), and `send\_prepare\_ok` refuses for dirty slots (:8665, :8710: \"not sending (dirty)\"). Durability is real, not advisory: O\_DIRECT|O\_DSYNC on Linux (\"to ensure that data is durable when writes complete\", `src/io/linux.zig:1516-1517`), explicit `F\_FULLFSYNC` per write on macOS (`src/io/darwin.zig:829-830`). Where TigerBeetle differs from 1988: commit is completely lazy (no commit record, no disk I/O at commit — §B5), replication is star-shaped with flexible quorums, and the 1988 recovery protocol is replaced by durable superblock state.\n\n\*\*(2) vs VRR-2012 (Liskov & Cowling).\*\* TigerBeetle does \*\*not\*\* follow VRR-2012's no-disk normal operation. There is no volatile-only log: every op is on disk before ack. Consequently the VRR-2012 recovery regime — reconciling logs from volatile state, with the Appendix B.1 (UW-CSE-17-08-01) failure modes that arise when a recovering replica must reconstruct/merge logs it no longer durably holds — is \*\*structurally excluded\*\*, for three code-anchored reasons:\n- The log suffix that survives into view change is durable on a replication quorum. The new primary reconciles from JV headers backed by \*clean\* journal slots; it only truncates an op given a nack quorum of replicas that provably never durably prepared it (`send\_join\_view` nack cases :8817-8833; `quorum\_nack\_prepare` `src/vsr.zig:1207`). A replica that may have acked (faulty slot) never nacks — so a possibly-committed op cannot be truncated away, eliminating the B.1-style \"committed op lost during reconciliation\" failure.\n- Views and log\_views are themselves durable before being advertised (`view\_durable\_update` :9448; `transition\_to\_view\_change\_status` :10209), and a restarting replica's journal is fenced by the durable log\_view on recovery (`src/vsr/journal.zig:1299-1332`), with the explicit anti-fork proof at :9395-9433 (\"If replica 0 crashes and its log\_view regresses to a... The log has forked. Therefore, a replica's log\_view must never regress.\"). This removes the volatile-view backtracking class of failures.\n- Rejoin is never \"state transfer to disk then assume leadership\" from a volatile log: a restarting primary \*must\* run a view change (:893-894), and a lagging rejoiner either repairs its WAL from peers or state-syncs the durable checkpoint (`on\_view\_set\_checkpoint` :2851).\n\nCaveat (stated, not speculated): TigerBeetle does share one property with both papers — the \*commit point itself\* is volatile on backups and is only periodically checkpointed (§B5); correctness across crashes rests on the quorum-durable prepares plus the quorum intersection argument above, not on a durable commit record.\n\n\*\*(3) TigerBeetle-specific mechanisms absent from both papers:\*\* the two-ring WAL with redundant headers written after prepares for torn-write disentanglement + the 16-case recovery decision table (§C); \*\*no seals\*\* (explicitly: the term does not exist in this codebase); batched/lazy commit notification via prepare piggybacking + a commit heartbeat that doubles as fault detection (`src/vsr/fault\_detector.zig:17-24`); multi-copy flexible-quorum superblock with read-after-write verification (§C); checkpoints every 960 ops fencing WAL wrap; explicit client sessions (register/eviction, session = commit number :5699, deterministic oldest-commit eviction :5706-5719, and the fix for the two VRR client-table bugs documented at `src/vsr/client\_sessions.zig:16-31`); the durable `client\_replies` zone; ping/pong clock synchronization (Marzullo-style, `src/vsr/clock.zig`, `src/vsr/marzullo.zig`) and release advertisement for live upgrades; standbys (replicate, never vote, :6921-6927); grid block repair + scrubbing; the optional synchronous AOF for disaster recovery (\"Internally, write() will only return once the data has been written to disk with O\_DIRECT and O\_SYNC\", :5402-5404); direct-IO/O\_DSYNC/F\_FULLFSYNC IO stack; `cluster` u128 misdirection guard (no cross-cluster protocol exists in 0.17.9).\n\n---\n\n## G. QUORUM & CONFIG\n\n- \*\*Replication factors\*\*: `replicas\_max = 6`, `standbys\_max = 6`, `members\_max = 12` (`src/constants.zig:24-28`). Supported counts 1..6 replicas (plus standbys).\n- \*\*Quorums\*\* (`src/vsr.zig:1173-1235`): `quorum\_replication = min(quorum\_replication\_max=3, ⌈N/2⌉)` (forced to 2 for N=2, :1183-1188); `quorum\_view\_change = N − quorum\_replication + 1`; `quorum\_nack\_prepare = N − quorum\_replication + 1` (:1205-1207); intersection asserted (`src/vsr/replica.zig:1177`). Resulting default table (matches `docs/internals/vsr.md:285-289`): N=3 → replication 2 / view-change 2; N=4 → 2/3; N=5 → 3/3; N=6 → 3/4.\n- \*\*Numbering\*\*: view `u32` (monotonic, persisted), op `u64` (hash-chained via `parent` checksum), session `u64` = \"The commit number becomes the session number\" (:5699); request numbers per client; `epoch` field reserved for reconfiguration.\n- \*\*Client sessions\*\* (`src/vsr/client\_sessions.zig`): each active client holds `{session, reply header}` (:32-38); the table fixes the two VRR-2012 client-table bugs (correctness: request-number collision across client crashes; liveness: lockout on reordered uncommitted requests — :16-31). Eviction is deterministic (oldest commit among `clients\_max` entries, :5706-5719); evicted clients learn via the `eviction` message and must re-`register`.\n- \*\*Keepalive/ping-pong\*\*: replica↔replica `ping/pong` synchronizes clocks (Marzullo, `on\_pong` :1912) and piggybacks candidate release versions for upgrades (:1871-1895); client `ping\_client/pong\_client` returns the durable view (:1929). Liveness is the `FaultDetector` EWMA over prepare/commit flow (`src/vsr/fault\_detector.zig`); the primary broadcasts `commit` every 500 ms (`:1376-1380`) as heartbeat, and an idle/abdicating primary is detected by `primary\_abdicate\_timeout` (10 s, :1371-1375). Prepare retry is RTT-based (`on\_prepare\_timeout` :3608, RTT from pong median :1913-1915).\n- \*\*Cross-cluster\*\*: none in 0.17.9 — see §A.\n\n---\n\n## VERDICT\n\n\*\*TigerBeetle 0.17.9 implements a durable-at-prepare Viewstamped Replication in the VSR-1988 lineage — every op is written to a two-ring direct-I/O WAL (O\_DIRECT|O\_DSYNC on Linux, F\_FULLFSYNC on macOS) and made stable before any `prepare\_ok` is sent — but modernized with a lazy, record-free commit (piggybacked on prepares plus a 500 ms heartbeat that doubles as the fault-detector signal), flexible (Paxos-style) star replication quorums, nack-quorum (PAR CTRL) log truncation at view change, and a quorum-verified multi-copy superblock that durably fences views/log\_views and checkpoints every 960 ops; it is emphatically not VRR-2012's no-disk design, and its always-durable prepares plus never-nack-a-possibly-acked-slot rule and durable view fencing structurally exclude the VRR-2012 Appendix-B.1 class of log-reconciliation failures — the only component left volatile is the commit watermark itself, whose safety rests entirely on the quorum intersection between replication and view-change quorums.\*\*"} |

# The three write methods

All disk writes in TigerBeetle 0.17.9 ride the same direct-IO machinery: one
syscall-shaped path — a write posted to the io backend (io\_uring SQE on Linux,
AIO on Darwin, overlapped IO on Windows) whose completion arrives as a
callback on the event loop (`src/vsr/storage.zig:80`, `:350`; `src/io.zig:9-14`).
There is no second disk mode. The three "write methods" differ in exactly one
thing: **who waits on the completion.**

`src/vsr/storage.zig:80`
`:350`
`src/io.zig:9-14`

## 1. Async writes — client replies (nothing waits, off the critical path)

The one that looks magical. `ClientReplies` stores the latest reply to every
active client session in the `client_replies` zone, so a reply can be RE-SENT
if the client missed the original (dropped packet) — that is its entire job
(`src/vsr/client_replies.zig:1-8`).

`ClientReplies`
`client_replies`
`src/vsr/client_replies.zig:1-8`

The doc comment states the discipline outright: "Replies are written
asynchronously. Subsequent writes for the same client may be coalesced — we
only care about the last reply to each client session" (`:11-12`).

`:11-12`

Async **to what**: async to the commit path. When a request commits, the
replica sends the reply to the client immediately and hands the reply to
`client_replies.write_reply(slot, reply, .commit)` — and moves on
(`src/vsr/replica.zig:5747`, trigger enum at `client_replies.zig:72`). The
commit does not wait for the reply to be durable. What waits instead:

`client_replies.write_reply(slot, reply, .commit)`
`src/vsr/replica.zig:5747`
`client_replies.zig:72`
`writes: IOPSType(Write, client_replies_iops_write_max)`
`client_replies.zig:83`
`client_replies_iops_write_max = 2`
`client_replies_iops_read_max = 1`
`config.zig:118-119`
`write_queue`
`client_replies_iops_write_max`
`:74-76`
`:409-413`
`:11-12`
`ready(callback)`
`:306-318`

The durability deadline is the checkpoint: "ClientReplies guarantees that the
latest replies are durable at checkpoint" (`:14`). At checkpoint the replica
calls `client_replies.checkpoint(...)` and its completion is **joined** into
the checkpoint-completion barrier before the superblock is sealed
(`replica.zig:5041`, join at `:5071-5074`). That is the "forced during the
snapshot": nothing forced the reply writes per-op; the checkpoint barrier
waits for them.

`:14`
`client_replies.checkpoint(...)`
`replica.zig:5041`
`:5071-5074`

Why it is safe to be lazy here: the reply **header** is already durable in the
`client_sessions` trailer (checkpointed with the superblock data), so a
body-less reply never needs a zone write (`client_replies.zig:341-343`). The
body is only needed to re-send a reply the client missed. If a reply body is
corrupted on all replicas AND the client missed the reply, that one client can
be locked out (retrying an executed request) until the operator restarts it —
an explicitly bounded availability trade (`:16-19`). Corrupt replies are
repaired from other replicas' copies (`:8`).

`client_sessions`
`client_replies.zig:341-343`
`:16-19`
`:8`

Zone geometry: one slot per client, `client_replies_size = clients_max × message_size_max` (`constants.zig:178`), slot address = `slot.index × message_size_max` (`client_replies.zig:27-28`).

`client_replies_size = clients_max × message_size_max`
`constants.zig:178`
`slot.index × message_size_max`
`client_replies.zig:27-28`

## 2. Forced writes — WAL prepares (the protocol waits, per op)

Every prepare, every replica: body into `Ring.prepares`
(`src/vsr/journal.zig:1851`), then the redundant header into `Ring.headers`
(`:1908`), both writes carrying durability in the open flags — `O_DSYNC`
(`src/io/linux.zig:1432`), `F_FULLFSYNC` per pwrite (`src/io/darwin.zig:829-830`),
`FILE_WRITE_THROUGH` (`src/io/windows.zig:1277`). Write completion == on disk;
no fsync call exists in the data path. Only when both completions arrive does
`dirty.clear` (`journal.zig:1953`) permit `prepare_ok`
(`src/vsr/replica.zig:11225`). The completion is the durability event, and the
consensus protocol blocks on it per op. Budget: ≤ 32 concurrent WAL writes
(`config.zig:117`). The two-write is torn-write armor for in-place ring
overwrite; the 16-case recovery table classifies every slot locally
(`journal.zig:1204-1241`).

`Ring.prepares`
`src/vsr/journal.zig:1851`
`Ring.headers`
`:1908`
`O_DSYNC`
`src/io/linux.zig:1432`
`F_FULLFSYNC`
`src/io/darwin.zig:829-830`
`FILE_WRITE_THROUGH`
`src/io/windows.zig:1277`
`dirty.clear`
`journal.zig:1953`
`prepare_ok`
`src/vsr/replica.zig:11225`
`config.zig:117`
`journal.zig:1204-1241`

## 3. Write-through blocks — grid and superblock (the checkpoint barrier waits)

Grid blocks and the superblock use the same direct-IO write path as
everything else. They are "write-through" in the sense that their writes are
not lazily deferred past a barrier: at a checkpoint, the state machine writes
the updated grid blocks, client sessions and replies to their zones, and the
checkpoint cannot complete until those writes finish — then the superblock is
written last, 4 copies, verified 3-of-4, as the seal
(`src/vsr/superblock.zig:1124-1172`; `src/vsr/superblock_quorums.zig:61-66`).
Released grid blocks stay pinned until that seal (`src/vsr/free_set.zig:600-607`).

`src/vsr/superblock.zig:1124-1172`
`src/vsr/superblock_quorums.zig:61-66`
`src/vsr/free_set.zig:600-607`

## The taxonomy in one table

| Writer | Trigger | Who waits on the completion | Durability deadline | Budget | Cited at |
| --- | --- | --- | --- | --- | --- |
| Client replies | per commit, per changed client | nobody (ready-callback resumes the pipeline) | checkpoint barrier (joined before the superblock seal) | 2 writes in flight, 1 read | `client_replies.zig:11-14,72-83`; `replica.zig:5747,5041` |
| WAL prepare (body + header) | per prepare, per replica | the protocol — `prepare_ok` is gated | immediately (flag-carried; completion == durability) | ≤ 32 concurrent writes | `journal.zig:1851,1908,1953`; `replica.zig:11225`; `config.zig:117` |
| Grid blocks / client sessions | per checkpoint (state machine apply) | the checkpoint barrier | before the superblock seal | grid repair/compaction budgets | `superblock.zig:1124-1172`; `free_set.zig:600-607` |
| Superblock | per checkpoint / view fence / restart | the checkpoint or view-change barrier | quorum-verified (3-of-4) before it counts | 4 copies × 24 KiB | `superblock.zig:1097-1099,1124-1172`; `superblock_quorums.zig:61-66` |

`client_replies.zig:11-14,72-83`
`replica.zig:5747,5041`
`prepare_ok`
`journal.zig:1851,1908,1953`
`replica.zig:11225`
`config.zig:117`
`superblock.zig:1124-1172`
`free_set.zig:600-607`
`superblock.zig:1097-1099,1124-1172`
`superblock_quorums.zig:61-66`

One sentence: **the machinery is identical; "async", "forced", and
"write-through" are three answers to the question "who is allowed to continue
before this write completes" — nobody (client replies, deadline = checkpoint),
the consensus protocol (WAL, deadline = now), or the checkpoint/view barrier
(grid + superblock, deadline = before the seal).**

## Footer

### Footer navigation
