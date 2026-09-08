# uVRR experiment design: advisory-lock cluster under Crash-Stop-Reincarnation

> **WARNING: THIS WORK HAS NOT BEEN DONE.** This document is an experimental
> design, not a report. Nothing here has been executed; no cluster was killed,
> no flush was forced, no VM was timed. Every timing, percentile, and result in
> this document is the placeholder `xxxx`. Any document or paper that quotes a
> number from here as a measurement is wrong.

## 1. System under test

The system under test is the advisory-lock service built on this repository's
sans-IO VRR core. The reference implementation is the
[lunet-locks](https://github.com/lua-lunet/lunet-locks) demo, described here
from a fresh clone of that repository; every component name below is that
implementation's.

### 1.1 Components

| Component | Role |
|---|---|
| `vrr-core` | The sans-IO VRR core. One total function; the host owns time, transport, storage, packetization, and naming. The adapter instantiates `Replica<SegmentedLog, WeightedMajority>` running `Stability::Volatile`: protocol state lives in quorum memory, not local storage. |
| Rust advisory-lock adapter | Owns the lock state machine, the tick clock, exactly-once reply correlation by `message_id`, and the durable recovery nonce. Executes only committed lock commands. |
| Teal/Lunet service process | `lunet-run build/server.lua`. Owns the TCP client endpoint (newline-delimited JSON), the UDP peer sockets, peer source validation, and forwarding. |
| LAL Peer Protocol | The service's raw-UDP framing and forwarding layer: envelope `\0LUNET_ADVISORY_LOCK_PEER\0` + kind + membership fingerprint + payload. The fingerprint is the first 16 lowercase hex characters of SHA-256 over the validated lexical member list. |
| Recovery nonce file | The `--state` path. The only bytes the service ever fsyncs: write, fsync, rename, parent-directory sync, per boot and per recovery retry. |
| Status surface | Each replica reports `normal`, `view_change`, `recovering`, or `replaying`, plus the current era, view, and the positional index of the current view's primary; `leader_for_view(era, view)` resolves the primary of an arbitrary era-and-view pair. |
| Event journal and console | Append-only binary journal of committed lock transitions (`hold`, `renew`, `release`), served by `lock-feed` over REST/WebSocket to the console SPA. Observability only: it never participates in replication or recovery. |

Timing knobs are the host flags `--heartbeat-ms`, `--election-ms`, and
`--recovery-ms`. Membership is fixed for one process lifetime.

### 1.2 The lock and the lease

The client object is an advisory **lock** held under a **lease**. It is never a
"leader"; the word leader names the VRR protocol role only.

The lock is a single compare-and-swap register, held entirely in quorum memory:
no disk. The register state is a `u32` client identifier and a `u32` holder
value; holder `0x0` means no holder, and no client identifier is `0x0`.

- **Startup / probe (`GET`):** the client reads the lock. If it is absent or
  expired, `GET` reports holder `0x0`. Whether a recorded lease has expired is
  judged in the **leader's clock**: the leader compares the lease's expiry
  against its own `now()` at execution time.
- **Acquisition (`SET`):** clients race to `SET` themselves as holder. Exactly
  one wins; the losers are rejected. The winner's lease duration is 100 ms: the
  leader reads `now() + 100 ms` as the future expiry, replicates the command
  through VRR, and responds.
- **Renewal (`BUMP`):** at approximately 80% of the 100 ms window the holder
  renews. `BUMP` is a CAS-style command conditional on the holder match: it
  **increases** the expiry by one window (100 ms), it does not write a new
  absolute time.
- **Contenders:** a rejected `SET` loser times out at `rand() * 100 ms` beyond
  the observed expiry, probes with `GET`, and sees the leader has renewed the
  lease out to the future timestamp; it does not take the lock while a live
  incumbent stands.
- **Round trip:** the client measures the round trip of every operation from
  its own clock (§7).

### 1.3 Cluster and labelling

Six nodes over raw UDP on the LAL Peer Protocol. Nodes are labelled `DC1_A`,
`DC1_B`, `DC2_A`, `DC2_B`, `DC3_A`, `DC3_B` — three data centres, two nodes
each. This is the FPAXOS label scheme, stated here as our labelling. The
lexically sorted member list is identical on every node; the membership
fingerprint guards against accidentally joining a differently configured
cluster.

## 2. Decision rule

The claim under test is diskless low latency: Crash-Stop-Reincarnation (CSR)
rejoin is faster than paying a forced disk flush at the recovery boundary. The
decision rule for the whole campaign:

> If the measured CSR rejoin latency (E1) is **slower** than the forced-flush
> baseline (E2 variant 1) at the same sample count, the diskless claim
> **collapses**. Success is CSR rejoin latency matching or beating the
> forced-flush baseline.

If recovery is slower than doing disk flushes, nothing has been achieved.

## 3. E1 — node-kill rejoin latency under CSR

**Question.** With normal lock traffic running, how long does a killed voting
node take to rejoin serving after reincarnation?

**Variables.**

- Node killed: one **voting** node that is **not** the uVRR leader, rotated
  across all non-leader voting nodes.
- Traffic: continuous `SET`/`BUMP`/`GET` lock traffic from competing clients at
  the 100 ms lease cadence, throughout.
- Iterations: `k = xxxx` kill/rejoin cycles, **cold counts**: each iteration
  begins only after the previous rejoin has completed serving and a soak
  interval has elapsed, so no measurement is contaminated by the previous one.

**Procedure.** For each iteration: wait for steady state (all replicas
`normal`, leader serving); kill the selected node's process
(Crash-Stop: volatile state lost, the node is by construction a different node
on reopen); the cluster continues serving at quorum; the killed node bumps its
incarnation, rejoins as a weight-0 standby, and is walked back to voting
weight by the leader's forced reconfiguration sequence; the iteration ends when
the reincarnated node is again a voting, serving replica.

**Metric.** Kill→rejoin-serving latency, one sample per iteration:
distribution percentiles p50, p90, p99, max; mean; and the count of eras/view
changes consumed per rejoin. All values `xxxx`.

**Sample count.** `k = xxxx` iterations, split evenly across the rotated
non-leader voting nodes.

**Success rule.** Every iteration rejoins (no permanent loss of the killed
node); the distribution is reported for comparison under §2.

## 4. E2 — disk modes at the recovery boundary

**Question.** What does the diskless default save, in latency, against paying a
forced flush at the recovery boundary?

Three variants, identical in every respect other than the durability behavior
at the recovery boundary (the point at which a reincarnating node durably
records state before rejoining serving):

**Variant 0 — diskless (our default).** No durable writes at all at the
recovery boundary. Quorum memory is the only protocol state; the recovery nonce
file remains as shipped.

**Variant 1 — naive single write (latency baseline).** One 4 KiB block write
followed by `fsync` at the recovery boundary: the direct-write approach of the
classic designs. Per FAST'18 (Alagappan et al., *Protocol-Aware Recovery for
Consensus-Based Storage*, USENIX FAST '18, Best Paper), even this single write
plus `fsync` is **not** fault-safe against real disk faults — it is included
only as the latency baseline, never as a safety claim.

**Variant 2 — double-ring write.** A TigerBeetle-style double write, at the
recovery boundary: write one 4 KiB block containing one 64-byte cache line of
data plus a checksum header to the **first WAL ring**; then write the
header+checksum, with **no payload**, to the **second WAL ring**. The rings are
spaced apart per TigerBeetle's real layout: two separate fixed zones of the
data file, 4 KiB-sector aligned, so the two copies of the checksum sit in
different erasure blocks. Grounding: the TigerBeetle 0.17.9 protocol and disk
analysis ([the Director's gist](https://gist.github.com/simbo1905/3818c439bb08774f87bc8a92cc0a6ad5))
documents the production geometry this variant mirrors — 4x superblock writes
plus the WAL header flush, two durable direct writes per prepare (body, then
redundant header) before the acknowledgement, 1024-slot two-ring WAL, headers
ring separate from the prepares ring. Wild support for double-write/dual-ring
spacing is surveyed in §9.

**Both flush variants carry fake data only.** The payload may use the value of
the message being sent as junk; there is **never any read-back** in either
variant. The variants measure write-and-flush latency only; they never
reconstruct state from what was written.

**Variables.** Variant (0, 1, 2); node killed and traffic as in E1; iterations
`k = xxxx` per variant, cold counts.

**Procedure.** Exactly E1's procedure, per variant, against a custom build that
implements the variant's recovery-boundary behavior.

**Metric.** Kill→rejoin-serving latency distribution (percentiles as in E1)
per variant; plus, for variants 1 and 2, the recovery-boundary flush latency
itself (time from flush start to write completion) per iteration. All values
`xxxx`.

**Sample count.** `k = xxxx` iterations per variant.

**Success rule.** Variant 0 matches or beats variant 1 on the rejoin
distribution (§2). Variant 2 quantifies what the fault-safe flush would cost
against both.

## 5. E3 — Maelstrom brute force at three and five nodes

**Question.** Does CSR hold consistency under brute-force fault injection?

Jepsen's Maelstrom drives the cluster under its fault profile with CSR as the
failure model: node crashes with volatile-state loss (the crashed node
reincarnates rather than crash-recovers), plus partitions. A Maelstrom adapter
process fronts the lock service, exposing the lock as a compare-and-set
register over Maelstrom's node protocol.

**Variables.** Cluster size: **three nodes**, then **five nodes**. Fault
profile: Maelstrom's crash and partition injections with the CSR
reincarnation behavior. Workload: competing lock clients at the §1.2 cadence.

**Checks.**

1. **Consistency under crash:** Maelstrom's linearizability check over the
   register passes for every run — no committed operation is lost, no
   amnesiac node rejoins under its old identity and overwrites a newer
   decision.
2. **Rejoin success:** every killed node reincarnates and rejoins serving; no
   run ends with a permanently lost node.
3. **No split decisions:** `GET` never reports two holders; expiry is judged
   only in the leader's clock.

**Procedure.** At three nodes, run Maelstrom with the CSR fault profile; then
repeat at five nodes. Iterations per cluster size: `xxxx`. All values `xxxx`.

**Sample count.** `xxxx` runs per cluster size.

**Success rule.** All runs pass all three checks. The result is presented as
supporting the case that CSR eliminates the FAST'18 weakness: consistency
under crash with successful rejoin, by construction rather than by recovery.

## 6. E4 — cloud repetition

E1 and E2 are repeated on commodity cloud virtual machines, with the six-node
cluster laid out over three regions (two VMs each) matching the §1.3
labelling. Variables, procedure, metrics, and sample counts are E1's and E2's
unchanged; the environment differs (§8). All values `xxxx`.

## 7. Timing measurement

- **Client round trip:** every operation's round trip is measured by the
  client: a monotonic clock reading taken immediately before the request line
  is written to the TCP connection and immediately after the response line is
  read. One sample per operation; distributions reported as percentiles.
- **Expiry decisions:** expiry is judged in the **leader's** clock. The leader
  reads `now()` on its own host at execution time — the adapter's monotonic,
  nondecreasing milliseconds-since-Unix-epoch tick, clamped per node so a
  wall-clock regression never reaches the core. The lease expiry is stamped as
  leader `now() + 100 ms` on `SET` and advanced by one window on `BUMP`.
- **Cross-host comparison:** the leader's response echoes the execution-time
  tick, so a client can interpret expiry against the leader's timeline without
  assuming synchronized clocks.

## 8. Environment plan

- **Local timing runs:** a lima virtual machine on the Director's machine runs
  the timing experiments (E1, E2 local legs). All timing runs happen inside
  the VM so the host's load does not contaminate percentiles.
- **Docker is functional only:** the local Colima Docker daemon has no BuildKit
  and no disk sharing, so Docker never produces timings. Docker is for
  **functional tests** — docker-compose the six-node cluster and prove the
  cluster boots and serves with its dependencies. Base image: Debian Trixie;
  `aarch64` on this host, `x86_64` elsewhere; the arm64 build must boot with
  dependencies installed.
- **Cloud:** commodity virtual machines for E4, three regions, two VMs each.
- **Target hardware note:** the Director's target box class is M5-series Macs
  with 2T disk. Local timings target that class.

## 9. Sources

- Director's TigerBeetle 0.17.9 VSR/VRR protocol and disk analysis (the
  grounding for variant 2's geometry: 4x superblock writes + WAL header flush,
  two-ring WAL, spacing):
  <https://gist.github.com/simbo1905/3818c439bb08774f87bc8a92cc0a6ad5>
- Alagappan et al., *Protocol-Aware Recovery for Consensus-Based Storage*,
  USENIX FAST '18 (Best Paper) — the amnesia-recovery flaw; why even the
  naive single write+fsync baseline is not fault-safe:
  <https://www.usenix.org/conference/fast18/presentation/alagappan>
- TigerBeetle architecture: storage can fail; checksummed, replicated repair:
  <https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/ARCHITECTURE.md>
- TigerBeetle safety: strict storage fault model (corruption, misdirected IO,
  gray failure rates): <https://docs.tigerbeetle.com/concepts/safety>
- TigerBeetle overview: Protocol-Aware Recovery, out-of-band 128-bit
  checksums: <https://tigerbeetle.com>
- Jepsen, *TigerBeetle 0.16.11*: checksums stored separately from data
  blocks; multiple copies for critical data:
  <https://jepsen.io/analyses/tigerbeetle-0.16.11>
- TigerBeetle on checksums vs torn writes (committed-WAL vs uncommitted-WAL
  tear ambiguity; FAST '18 single-sector propagation):
  <https://news.ycombinator.com/item?id=36683353>
- TigerBeetle design notes: header alignment on cache-line boundaries:
  <https://github.com/tigerbeetle/tigerbeetle-history-archive/blob/main/docs/DESIGN.md>
- InnoDB doublewrite buffer (the canonical wild double-write: separate files,
  write-torn-page protection):
  <https://dev.mysql.com/doc/refman/8.4/en/innodb-doublewrite-buffer.html>
- InnoDB disk I/O: the doublewrite flush technique:
  <https://dev.mysql.com/doc/refman/8.0/en/innodb-disk-io.html>
- Torn write detection and protection (double-write buffer history,
  `DETECT_ONLY`, placement on separate drives):
  <https://transactional.blog/blog/2025-torn-writes>
- PostgreSQL vs MySQL torn-page handling (WAL redundancy vs doublewrite):
  <https://www.percona.com/blog/a-tale-of-two-databases-how-postgresql-and-mysql-handle-torn-pages>

> **WARNING: THIS WORK HAS NOT BEEN DONE.** This document is an experimental
> design, not a report. No experiment above has been executed. Every timing,
> percentile, sample count to be filled at run time, and result is the
> placeholder `xxxx`; nothing here may be quoted as a measurement.
