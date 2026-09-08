# Protocol-Aware Recovery for Consensus-Based Storage — reference summary

Ramnatthan Alagappan, Aishwarya Ganesan, Eric Lee, Aws Albarghouthi, Vijay
Chidambaram, Andrea C. Arpaci-Dusseau, and Remzi H. Arpaci-Dusseau,
"Protocol-Aware Recovery for Consensus-Based Storage," Proceedings of the
16th USENIX Conference on File and Storage Technologies (FAST '18), Oakland,
CA, February 2018 (Best Paper Award). Printed pages 15–31; page pointers
below are the paper's own printed page numbers. This summary is reference
use with attribution; the paper itself is the source of record.

## Problem

Replicated state machine (RSM) systems (LogCabin/Raft, ZooKeeper/ZAB, etcd,
Paxos systems) keep three persistent structures per node: the replicated
log, periodic snapshots of the state machine, and metainfo (e.g. log start
index, current term/epoch). Storage faults — block errors and silent
corruption from media errors, read disturbance, lost/misdirected writes,
and firmware/driver/file-system bugs — can strike any of the three
(§2.1–2.2, pp. 16–17). Because losing or overwriting committed log
commands violates the safety of the state machine (§2.2, p. 16), recovery
from storage faults is the hardest case: "a small misstep in recovery
could violate the guarantees" (§1, p. 15).

## The catalogue of recovery failures (§2.3, pp. 16–18)

The taxonomy divides existing practice into protocol-oblivious and
protocol-aware approaches, each classified by consequence in Table 1
(p. 17; E = return corrupted, L = data loss, U = unavailable, C = correct):

- **NoDetection** (p. 17): no integrity checks; corrupted data is served
  obliviously. Trivially violates safety.
- **Crash** (p. 17): checksums + crash on fault (LogCabin, ZooKeeper, etcd
  on faulty logs; ZooKeeper also on corrupt snapshots). Safe, but a single
  storage fault renders the system unavailable; storage faults are
  persistent, so restart alone re-crashes until manual, error-prone
  intervention.
- **Truncate** (pp. 17–18): delete the faulty tail and continue. Causes
  the paper's central safety violation (see the pinned statement below)
  and slow recovery: the leader must retransfer everything lost, wasting
  bandwidth. Found in ZooKeeper and LogCabin.
- **DeleteRebuild** (p. 18): manually wipe the faulty node and restart.
  Same silent-loss shape as Truncate — the emptied node can form a
  majority with lagging nodes — and equally slow; a widespread operator
  practice the authors cite.
- **MarkNonVoting** (p. 18, Google's Paxos system): the node deletes all
  data and abstains from elections until it observes one round of
  consensus. Still unsafe: once a faulty node has lost its promise given
  to a leader, it can accept an entry from an older leader and overwrite
  a previously committed entry; also unavailable in several scenarios.
- **Reconfigure** (p. 18): remove the faulty node and add a fresh one.
  Unavailable whenever the change needs a majority that is not up.
- **Byzantine FT** (p. 18): tolerates corruption but needs 3f+1 nodes and
  about a halving of throughput; unavailable in most scenarios.

### The amnesia class — a recovered node that has lost its recently accepted writes

This is the failure class the FAST'18 catalogue is best known for in this
project's register. The paper's own words pin it; note that the term
"amnesia" never appears in the paper and no appendix exists in the
proceedings copy — the flaw is stated in the body:

- **Primary statement — §2.3, the Truncate discussion, printed page 17:**
  "In summary, although the faulty node detects the corruption, it
  truncates its log, losing the data locally. When this node forms a
  majority along with other nodes that are lagging, data is silently
  lost, violating safety. We find this safety violation in ZooKeeper and
  LogCabin."
- **Figure 2 walkthrough, printed page 18:** entries 1, 2, 3 are
  committed on S1–S3; entry 1 is corrupted on S1; S1 truncates 1–3;
  S2, S3 crash; S1 then forms a majority with lagging S4, S5 and the
  cluster, ignorant of 1–3, commits x, y, z in their place; when S2, S3
  recover they follow S1's log and entries 1–3 are completely removed —
  "resulting in a silent data loss."
- **DeleteRebuild restatement, printed page 18:** "a node whose data is
  deleted could form a majority along with the lagging nodes, leading to
  a silent data loss."
- **Root intuition, §1 Introduction, printed page 15:** nodes differ —
  "some nodes may have recently accepted data while others may hold a
  stale version. If enough care is not exercised, the node could 'fix'
  its data from a stale node, overwriting the new data, potentially
  leading to a data loss."
- **Empirical confirmation, §5.1.1, printed page 25 (Table 4(a)):** of
  2401 recoverable targeted-corruption cases over LogCabin and
  ZooKeeper, the original systems recover correctly in only 46 (cases
  with no or one corrupted node); in the remaining 2355 they are either
  unsafe (truncate) or unavailable (crash). ZooKeeper truncates on
  transaction-tail corruption but crashes on header corruption — both
  policies are wrong somewhere.

Placement note: a pointer of the form "Appendix X 3.1" resolves to
nothing in the proceedings copy — there is no appendix (the paper runs
§1–§7 plus references), and §3.1 is the Fault Model. The amnesia recovery
flaw lives in §2.3 (printed pp. 17–18) as above, with the Figure 2
safety-violation walkthrough on printed page 18.

## The CTRL remedy

**Protocol-aware recovery (PAR)** exploits protocol-specific knowledge —
how updates are performed, how leaders are elected, what committed means
— instead of acting locally and blindly. CTRL (corruption-tolerant
replication) is its instantiation for RSMs (§3, pp. 18–24):

- **Fault model and guarantees (§3.1–3.2, pp. 18–19):** Table 2 (p. 19)
  fixes the storage-fault model over user data blocks and file-system
  metadata (lost data, missing files, inaccessible files/dirs, corrupt
  metadata, unmountable file systems). CTRL guarantees safety — if at
  least one correct copy of a committed item exists it is recovered, and
  CTRL "correctly recovers even if portions of data on all nodes could be
  corrupted" so long as a majority is up and one non-faulty copy exists;
  when recovery is impossible it stays unavailable by design rather than
  violate safety — and availability — uncommitted faulty data is
  discarded as early as possible (§3.2, p. 19).
- **Local storage layer, clstore (§3.3, pp. 19–21):** checksums for
  corruption plus error codes for inaccessibility (§3.3.2, p. 19). The
  key trick is *disentangling crashes from corruption* in the log
  (§3.3.3, p. 20) using **persist records** — small, atomically writable
  per-entry identifiers ⟨index, epoch, offset, cksum⟩ written with
  pwrite before/after the entry (Figure 3, p. 19): a persist record
  present with a corrupted entry means corruption (recover from other
  replicas — truncating would be unsafe), its absence means the entry
  never persisted, so it is a crash (safe to discard). The last entry is
  the exception; when disentangling fails, clstore marks the entry
  corrupted and hands the decision to the distributed layer. Faulty-data
  identifiers are stored physically separated from payloads (§3.3.4,
  p. 20) so one lost/misdirected write cannot destroy both; metainfo is
  node-specific and unrecoverable from others, so it is stored
  redundantly (two local copies).
- **Distributed log recovery (§3.4, pp. 21–22):** the leader coordinates
  per-entry recovery. For its own faulty entry it queries followers with
  responses dontHave / have / haveFaulty (§3.4.2–3.4.3, p. 22): any
  `have` → recover; majority `dontHave` → the entry is uncommitted,
  discard (and discard all subsequent entries); `haveFaulty` → wait.
  It must gather ⌊N/2⌋+1 dontHave responses before declaring an entry
  uncommitted — fewer provably violates safety (model checking, §5.1.1,
  p. 26). Waiting is the price of safety; the recovery protocol removes
  the leader-restriction that would otherwise block progress (§3.4.1,
  p. 21).
- **Distributed snapshot recovery (§3.5, p. 23):** leader-initiated
  identical snapshots — a `snap` marker commits at the same log index
  everywhere, so snapshots are byte-identical across nodes; faulty
  chunks (id ⟨snapshot index, chunk number⟩) are then replaced chunk-wise
  from other nodes, locally from the log when still present, or from a
  majority snapshot after garbage collection (§3.5.2, p. 23).
- **Implementation (§4, pp. 24–25):** LogCabin v1.0 (Raft) and
  ZooKeeper v3.4.8 (ZAB). Raft's AppendEntries and ZAB's Phase 1/Phase 2
  (newEpoch) are extended to carry faulty-data information and repaired
  entries/chunks; the leader does not proceed to normal operation (or to
  ZAB Phase 2) until its own faulty data is fixed, and steps down after a
  configurable recovery timeout.

## Evaluation (§5, pp. 25–26)

- **Targeted log corruptions (Table 4(a), p. 25):** 4096 corruption
  combinations on 3 nodes; 2401 recoverable — CTRL correct in all 2401;
  originals correct in only 46, unsafe (truncate) or unavailable (crash)
  in 2355. In the 1695 unrecoverable cases CTRL stays unavailable by
  design.
- **Random block corruptions/errors (Table 4(b), p. 25):** 5000 cases
  each; originals unsafe or unavailable in ≈30% under corruption, and on
  block errors the originals crash → ≈50% unavailable; CTRL correct in
  all cases.
- **Crashed/lagging nodes (Table 4(c), pp. 24–25):** 5000 states with
  lagging nodes, uncommitted entries, mixed epochs; CTRL recovers all,
  originals unsafe or unavailable in many cases.
- **Model checking (§5.1.1, p. 26):** a Python checker explored >2.5M log
  states, all correctly recovered; weakening the ⌊N/2⌋+1 dontHave rule
  immediately yields a safety violation. CTRL's log recovery is also
  added to Raft's TLA+ specification, which then recovers correctly,
  while the original violates safety.
- **Snapshot recovery (§5.1.2, p. 26, Table 5(a)):** 1000 mixed
  log/snapshot states; CTRL correct in all; original LogCabin incorrect
  in about half (obliviously loads faulty snapshots or crashes);
  original ZooKeeper crashes the node (unavailability; unsafety because
  a faulty log is truncated in some cases).
- **File-system metadata faults (§5.1.3, p. 26, Table 5(b)):** 1000
  cases; originals sometimes crash, sometimes miss the fault entirely
  and continue, violating safety in 36 (LogCabin) and 192 (ZooKeeper)
  cases; CTRL reliably crashes the node, preserving safety.
- **Performance overhead (§5.2, pp. 26–27, Figure 5):** write-only
  workloads (reads unaffected, served from memory); 1 KB entries,
  300 s runs, five-run averages. On HDD the identifier/data separation
  induces a seek, amortized by batching: 8%–10% overhead at 32 clients
  on disks;   on SSD ≤4% worst case. The write-only workload is the worst
  case; read-heavy workloads would see less. The abstract-level claim
  is simply "little performance overhead" (§1, p. 15) — small against
  the safety failures eliminated.
- **Fast log recovery (§5.2, p. 27):** with 30K entries (1 KB each) and
  one corrupted first entry, original LogCabin truncates and retransfers
  everything: 1.24 s and 32 MB; CTRL fixes only the faulty entry:
  1.2 ms and 7 KB — a ~1000× reduction in recovery time.

## Placement

The paper builds on fault-injection studies of storage faults and on the
authors' prior finding that redundancy does not imply fault tolerance
(§6, p. 27); its contribution is the protocol-aware recovery design and
its demonstration that redundancy without protocol awareness — Truncate,
DeleteRebuild, MarkNonVoting, Reconfigure — is unsafe or unavailable
precisely when it matters. Conclusions (§7, p. 27) name PAR as a first
step: primary-backup and Dynamo-style quorum systems remain to be
analyzed.
