# uVRR I/O obligations: what every host owes a starting and a stopping replica

The host's durability obligations in one place. The library is sans-I/O: it
owns the protocol and the marker state machine; the host owns the event
loop, the wire, and the disk. The boot-gate chapter states the
classification of every start from durable state alone and the write
schedules each path owes; the termination chapter states the obligations
every host owes an embedded replica at shutdown. Each chapter keeps its own
section numbering: a reference of the form "the boot-gate chapter §5" names
a section within that chapter.

## uVRR boot gate: what every host owes a starting replica

uVRR is a sans-I/O library: the host owns the event loop, the wire, and the
disk (the termination chapter). Termination's dual is the boot
gate. Before a starting process may take its first message off the wire, the
host must answer one question from durable state alone: **did the previous
process reach its drain point?** The answer classifies the start, and the
classification chooses the path. A host that cannot answer the question is
unsafe, full stop.

### 1. The words are ruled

- A **controlled halt** is not a crash. The host stops the read loop, drains
  the handlers, forces the durable state to storage, and records the halt.
  Nothing was lost; nothing needs recovering.
- A **controlled start** is not a recovery. A process booting over a vouched
  durable state continues that state; no recovery protocol runs.
- **Resume** names nothing in uVRR for a crashed identity. The resumption of a
  crashed identity is the unrepresentable case (`docs/uvrr-protocols.md`, the
  reincarnation chapter §1): a
  crash is final for the protocol identity, and the process returns through
  crash-stop reincarnation, never through recovery. The constructor called
  `Replica::resume` is the ordinary controlled start of a halted process,
  Run → Halt → Run, vouched by the stopped quorum, not a resumption of a
  crashed identity and not a recovery.

The lifecycles, stated once:

```text
ordinary:  Run → Halt → Run → Halt → …
crashed:   Crash → Stop → Reincarnate → Run → Halt → …
```

`Run → Halt → Run` carries no protocol event at all: the cluster served a
quorum throughout, and the returning node re-synchronises in-cluster exactly
as though a network partition had healed. `Crash → … → Reincarnate` is the
reconfiguration path of `docs/uvrr-protocols.md` (the reincarnation chapter).

### 2. Why three marker states

The classification reads a durable marker. Two marker states are not enough.
A marker written as several spaced copies can tear: a process killed
mid-write leaves some copies advanced and some not. With only *running* and
*stopped*, a torn write presents a mix, and the mix is ambiguous, the
reader cannot tell a start from a halt. Three states remove the ambiguity,
because each transition is logged at its beginning and at its end:

| marker | written | vouches for |
|---|---|---|
| `RUNNING` | at boot, before the first message is processed | nothing, the process is up and the state beneath is unvouched |
| `STOPPED` | when the halt begins: the wire closed, the handlers drained | nothing yet, the durable write beneath is still in flight |
| `FLUSHED` | when the durable state write has completed | everything beneath it |

A read of `RUNNING` mixed with `STOPPED` is a process that died halting: no
`FLUSHED` quorum exists, nothing is vouched, crashed. A read of `STOPPED`
mixed with `FLUSHED` is a process that died completing its flush: the
`FLUSHED` copies vouch for the state beneath them, clean. Every mix
classifies; no mix is ambiguous. The classification takes the *minimum*
progress observed across the working copies, because a crash during the
marker writes must never read as a clean stop
(the termination chapter §3).

### 3. The write schedules

The marker is four spaced, checksummed copies, quorum-read (§6). The rounds
of forced writes are fixed by the path.

**Controlled halt, two rounds.** Halt the read loop and drain the handlers;
the first round writes `STOPPED` to the copies; the dual-ring WAL is then
forced to stable storage; the second round writes `FLUSHED`. The deferred
application-state write follows, and the process exits. The shutdown monitor
watches the marker, not the process.

**Clean start, one round.** Read the copies; a `FLUSHED` quorum vouches for
the state beneath. One round writes `RUNNING`, the boot-gate latch,
before the first message is processed, so that a later crash can never be
mistaken for this clean start. The process reads its era from the vouched
state and runs.

**Dirty fast start, one round now, the latch later.** The read shows no
`FLUSHED` quorum: the previous process never reached its drain point, the
identity is crashed, and this process is uninitialised. Two durable writes
now separate. The identity write is paid at the boundary, before any
emission: the boot increments the crash counter, commits the bumped pair,
and only then speaks (§5). The state flush is not paid at the boundary:
the process finds the cluster, catches up, and rejoins first
(`docs/uvrr-protocols.md`, the rejoin chapter §2); the `RUNNING` latch write is
deferred until the rejoin completes. A second crash before the deferred
latch still reads no `FLUSHED` quorum and classifies crashed again, and
because the bumped identity was already flushed, the second life increments
again: no value is ever served twice.

### 4. The era rule

The genesis era is 1. Era 0 is not a protocol state, and there is no
in-cluster state synchronisation to a node at era 0: a node that cannot name
era 1 or later has nothing the cluster can synchronise against, and its only
route in is the rejoin gossip. A clean start reads its era from the vouched
durable state and re-synchronises in-cluster from that era; a blank or dirty
start adopts the cluster's era through the gossip before its engine sees
traffic.

### 5. The identity law: universally unique, durable before emission

A node identity carries the obligation that Paxos Made Simple puts on
ballots: an identity is **universally unique, durable before use, and never
recycled**. A bump assigns a value that no life of any node has ever used
or will ever use; a value is burnt once and serves one life.

The marker names the identity as the explicit pair
`{systemIdentifier, crashCounter}`:

- the `systemIdentifier` is assigned by the sysadmin and burnt into the
  marker before first boot. It is one-indexed and never read as zero: a
  boot that reads zero asserts, because zero is not an identity but a
  corrupt marker.
- the `crashCounter` is durable in the same file, one-indexed and never
  read as zero; the life the disk last saw is the counter it read.

The wire identifier stays `u32`, the transport (paxe) and the descriptor
table demand it, and the durable pair in the marker is the truth the wire
name stands for.

**The emission gate: no announcement until the incremented counter is
flushed.** A crashed classification increments the counter, commits the
bumped pair, and only then emits its first message, identity before
emission, not emission before seat, unconditionally, seated or not. The
gate owes nothing to the rejoin, the witness, or any later seat: the
cluster must be able to distinguish lives, and a life that never reached
the disk is indistinguishable from its predecessor.

The deferral of §3 is untouched, because the two durable facts differ: the
`RUNNING` latch vouches for the *state* beneath the marker, and its
deferral is safe exactly as §3 argues; the identity pair vouches for the
*life* itself, and it never defers.

The re-crash replay under the law: a crash after the flush leaves the
marker at the bumped identity; the next boot reads it, classifies crashed
again, and increments again. Each life is one counter advance past the
last identity the disk saw, and no value is revisited.

The identity write is absorbed into the single small boot-fence write: no
new flush is paid on any critical path. The §3 dirty fast start separates
two durable writes at the boundary, and the identity commit is the first of
them; the deferred `RUNNING` latch and the state flush remain off the
boundary exactly as §3 argues.

The gate rules two roles, once. A pure witness is a passive data sink
outside the roster (`docs/uvrr-protocols.md`, the rejoin chapter §3): it owes the
boot gate nothing, no identity, no latch, no announcements. A weight-0
member is inside the roster, a learner and not an acceptor, and it owes the
boot gate everything, the identity law of this section included. The latch
mints at the engine's seated observation; for a weight-0 member that
observation is `Normal` at weight 0, which is sound because a weight-0
member is invisible to every majority. The ruling is discharged in the
formalization as `IdentityLaw.weight0_majority_irrelevant`
(`formal/uvrr-lean/`).

### 6. The construction the schedules assume

The schedules assume a marker store whose reads survive the failure modes of
real disks: torn sectors, misdirected writes, and silent rot are single-copy
events, so no single copy is ever trusted. The worked construction is
TigerBeetle's, whose techniques are named here, not restated
([github.com/tigerbeetle/tigerbeetle](https://github.com/tigerbeetle/tigerbeetle)):
four superblock copies in fixed zones, checksummed and hash-chained, with
quorum writes and quorum reads that repair the lagging copies; direct I/O
with explicit forcing at the points where the marker must vouch for data
beneath it; and a dual-ring WAL underneath. The flush literature is cited at
the termination chapter §4: a single `fsync`ed flag file is not
reliable enough (Pillai et al., OSDI 2014; Chidambaram et al., SOSP 2013),
which is why the marker is a quorum of copies and not a flag.

### 7. The crate contract: `uvrr::lifecycle`

The crate owns the machine; the host owns the writes. `uvrr::lifecycle`
ships the marker state machine, the quorum-read classification, and a
**typestate driver** whose types fix the write schedules of §3, a
transition called out of order has no type to be called on.

The host implements one trait, `LifecycleStore`, and nothing else:

- `read_copies`, the quorum read: the working set of copies as the store
  observed them (§2; the termination chapter §4).
- `commit`, the forced 4x write of a decided rewrite; the marker vouches
  for what the drain has already put beneath it (§3).
- `drain`, the host forces its WALs and grids; strictly between the two
  halt rounds (the termination chapter §1).

Downstream wraps its superblock quorum writes in the trait; a test host
writes plain marker files. The driver calls the three operations in the
legal order only: a controlled halt is `begin_stop` (round one), `drain`,
`finish_stop` (round two), `finish_stop` is unreachable without the
intervening drain because the drain is what the flushed marker vouches
for. A clean start is one round: `latch` before the first message. A
dirty fast start commits the bumped pair before the driver releases the
first announcement, and `latch` stays unreachable until the node presents
the engine's own seated observation (`Replica::rejoined`, `Normal` at any
weight): the identity gate of §5 and the deferral of §3 are both
enforced by the driver, not recommended. A crash models as the
absence of transitions, no write, no state, nothing.

The classification's verdict is carried in **proof tokens** that no host
can construct: a stopped-quorum read mints `Vouched`, the crashed branch
mints the `Bumped` pair once its commit is durable (§5), and the seated
observation mints `Rejoined`.
The replica constructors take the tokens: `Replica::resume` requires
`Vouched` and is the only same-identity path; `Replica::reincarnate`
requires the `Bumped` pair. A blank or dirty boot holds no `Vouched`, and
no function exists from a crashed classification to a same-identity
replica: the amnesiac blank boot is unrepresentable by construction, not
refused by a runtime check.

The deferral's safety argument is the re-crash replay: a crash before the
deferred latch leaves the marker at the bumped identity, flushed before
the first announcement (§5), and the next boot re-reads it, re-classifies
crashed, and increments again; no replayed value is ever announced. The
latch lands after the rejoin, off the critical path; the identity write
never does.

### 8. The test harness obligation

The contract is exercised in tests through crash-stop alone. The harness
implements `LifecycleStore` with plain marker files in a temporary
directory, no superblock writes are required of a test host, and routes
every restart through the driver: `halt` drives the controlled halt;
`restart_with` is the clean classification, the driver's `Vouched` token
constructs the same-identity resume; `restart_as` is the crashed
classification, the driver's `Bumped` pair constructs the reincarnation,
the bumped commit is observed durable before the harness network sees any
announcement (the §5 gate), and the deferred latch fires when the seated
observation mints `Rejoined`.
What the harness must never do is restart a crashed node silently as a
clean one, error-on-crashed is the contract
(the termination chapter §3).

### Figures

- [Figure 1, the marker state machine](diagrams/uvrr-boot-gate-states.svg):
  the three states, the transition that writes each, and the crash edges
  that write nothing.
- [Figure 2, the three boot classifications](diagrams/uvrr-boot-gate-sequence.svg):
  clean start, controlled halt and start, and the dirty fast start with its
  deferred latch.

## uVRR termination obligations: what every host owes an embedded replica

uVRR is a sans-I/O library embedded through an FFI boundary into any host
runtime (a Lua runtime, a Zig process, a Rust service). The library owns the
protocol; the host owns the event loop, the wire, and the disk. Termination
is therefore a contract between the two: the library defines the obligations,
and every future host application, an unbounded set, must meet them to
embed uVRR safely. The obligations are recorded here as the formal contract.
The same requirement is raised with the reference host runtime as
embedding contract, which is raised with the reference host runtime by
its tracker. Termination's dual, the classification of a start as
clean or crashed, and the write schedules each path owes, is the boot gate
(the boot-gate chapter).

### 1. The obligations, ranked

Two obligations rest on the host runtime at termination. They are not equal
in rank.

**Mandatory, the drain point closes the wire.** Once the host has said the
replica is stopped, no further inbound messages are picked up: no new reads,
no new accepts, no new task processing at or after the drain point. This is
the obligation that makes everything else possible, because it is the only
thing that makes the in-memory state final, the state in memory is the
state, so the in-flight state can be written to disk at the drain point and
will not be contradicted by later work arriving from the wire.

**Desirable, outbound is flushed.** Knowing that every outbound message has
been flushed is nice but not mandatory. Outbound writes either complete or
are abandoned safely: a message that was never sent is safe to abandon,
because a receiver cannot tell "never sent" from "lost on the wire". The host
must not let the desirable obligation delay the mandatory one.

The mandatory obligation is the termination dual of the restart rule in
`docs/uvrr-protocols.md` (the reincarnation chapter): a clean stop is a
protocol-visible event, and what
makes it clean is that the wire is closed before the state is declared final.

### 2. The lifecycle

The lifecycle below is stated in marker-agnostic terms; the deployed marker
transition machine (the boot-gate chapter §2; the code twins:
`src/lifecycle.rs`, `zig/vsr/superblock.zig`) implements it with
the ordered states `Stopping → Stopped → Restarting/Joining`. The
terminology is one language: `running` here means the marker's **not-`Stopped`**
operational states (`Restarting` after a clean stop, `Joining` after a bump,
the `unflushed` of the reincarnation doc), and `flushed` here means the
**`Stopped` quorum after the drain**, the copy that vouches for the WAL
under it. `Running` itself is never written: the boot writes `Restarting`
or `Joining` before the first message, and no safety logic looks for
anything else.

```
startup:  marker := running               (before the loop starts)
stop:     marker := stopped               (termination begins; wire closed)
drained:  WAL write, marker := flushed    (at the drain point)
```

The transitions flip the usual expectation in a useful direction: a copy that
has transitioned to `stopped` or `flushed` is by that fact not running, so a
marker in those states is evidence of a controlled ending.

The write ordering carries the safety argument. The durable state write (WAL
or the host's equivalent) completes **before** the `flushed` marker is
written, and the marker is written only at the drain point where no new task
processing can follow. A marker at `stopped` or later therefore vouches for
the durable state beneath it. A shutdown that dies partway through the marker
writes leaves some copies advanced and some not, and the advanced ones are
still truthful, which is why partial marker writes read as clean, not as a
crash.

### 3. Startup classification

On start the marker is read before the loop starts, and `running` is written
before any message is processed. The classification reads the *minimum*
progress observed across the marker copies, because a crash during the marker
writes must never be mistaken for a clean stop:

- **Clean stop, no resurrection needed.** Every copy in the working read
  shows `stopped` or `flushed` (a `flushed` copy mixed with `stopped` copies
  is the normal mid-flush shape: the previous process got through the drain
  point and was completing its flush). The state is final; the node continues
  normally without the reincarnation path.
- **Crashed, error on crashed.** Any copy in the working read is still at
  `running`, whether the process was killed outright (all copies at
  `running`) or died mid-shutdown (some copies at `stopped`, some still at
  `running`), the previous process cannot be shown to have reached the drain
  point. This is error-on-crashed: the reincarnation path runs, and the
  runtime must not silently resume as a clean restart.

The classification applies to every identity start, the bumped one included:
the bump write claims (X+1, `Joining`) as the wire-phase marker, it claims
no `Stopped` checkpoint, because the bumped identity has no WAL under it to
vouch for, so a second crash mid-wire-phase reads no stopped quorum and
bumps again (X+2). The lifecycle makes same-identity re-entry after
volatile-state loss unrepresentable **by construction**, not by argument.

### 4. The marker storage: expectation and example

The lifecycle requires one durable marker write at each transition. The
obligation is stated as an expectation, not a mechanism: whatever storage the
host has that is reliable enough to answer "was this process stopped?" after
a crash is acceptable. Superblock writes are not prescribed, because there
may be other very reliable storage to flush; the marker is the host's concern
once the drain point has made the in-memory state final.

The worked example of how such a marker is made trustworthy is TigerBeetle's
superblock, which the vendored store already uses. Its construction:

- **Four copies** of the superblock in fixed, sector-aligned zones of the
  data file, each with a checksum and a hash-chained `sequence`/`parent`
  chain, so a torn, misdirected, or rotted sector is detectable rather than
  trusted.
- **Quorum writes and quorum reads**: a write completes when a quorum of
  copies is durable; a read takes the working quorum, resolves by highest
  identity within it, and repairs the lagging copies. A single lying sector
  cannot decide the read, and no single-sector atomicity is assumed.
- **Ordering over flushing**: writes are forced where the marker must vouch
  for data beneath it, and the marker write happens only after the data write
  completes.

This is the same trick as stable storage itself: Lampson and Sturgis (1979,
*Crash Recovery in a Distributed Data Storage System*, §5.1) build stable
storage from ordinary unreliable disk by writing two copies and reading "the
good, the complete, or the newest", the quorum-of-copies scheme is the
generalisation of that idea to four copies and checksums.

The expectation is also stated against the flush literature, because a marker
built on `fsync` trust alone is not reliable enough:

- "All File Systems Are Not Created Equal: On the Complexity of Crafting
  Crash Consistent Applications" (Pillai et al., OSDI 2014) shows file
  systems, drivers, and virtual machines that silently ignore or degrade
  flushes.
- "Optimistic Crash Consistency" (Chidambaram et al., SOSP 2013) documents
  how `fsync` conflates ordering with durability and how real systems
  (macOS's `F_FULLFSYNC`, per-filesystem quirks) fail to deliver what
  applications assume it delivers.

Hence the example: a multi-copy, checksummed, quorum-read marker written with
forced I/O is the construction whose guarantees survive those filesystem
behaviours; a single `fsync`ed flag file is not.
