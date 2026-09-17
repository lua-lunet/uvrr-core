# uVRR boot gate: what every host owes a starting replica

uVRR is a sans-I/O library: the host owns the event loop, the wire, and the
disk (`uvrr-termination-obligations.md`). Termination's dual is the boot
gate. Before a starting process may take its first message off the wire, the
host must answer one question from durable state alone: **did the previous
process reach its drain point?** The answer classifies the start, and the
classification chooses the path. A host that cannot answer the question is
unsafe, full stop.

## 1. The words are ruled

- A **controlled halt** is not a crash. The host stops the read loop, drains
  the handlers, forces the durable state to storage, and records the halt.
  Nothing was lost; nothing needs recovering.
- A **controlled start** is not a recovery. A process booting over a vouched
  durable state continues that state; no recovery protocol runs.
- **Resume** names nothing in uVRR. The resumption of a crashed identity is
  the unrepresentable case (`uvrr-reincarnation.md` §1): a crash is final
  for the protocol identity, and the process returns through crash-stop
  reincarnation, never through recovery.

The lifecycles, stated once:

```text
ordinary:  Run → Halt → Run → Halt → …
crashed:   Crash → Stop → Reincarnate → Run → Halt → …
```

`Run → Halt → Run` carries no protocol event at all: the cluster served a
quorum throughout, and the returning node re-synchronises in-cluster exactly
as though a network partition had healed. `Crash → … → Reincarnate` is the
reconfiguration path of `uvrr-reincarnation.md`.

## 2. Why three marker states

The classification reads a durable marker. Two marker states are not enough.
A marker written as several spaced copies can tear: a process killed
mid-write leaves some copies advanced and some not. With only *running* and
*stopped*, a torn write presents a mix, and the mix is ambiguous — the
reader cannot tell a start from a halt. Three states remove the ambiguity,
because each transition is logged at its beginning and at its end:

| marker | written | vouches for |
|---|---|---|
| `RUNNING` | at boot, before the first message is processed | nothing — the process is up and the state beneath is unvouched |
| `STOPPED` | when the halt begins: the wire closed, the handlers drained | nothing yet — the durable write beneath is still in flight |
| `FLUSHED` | when the durable state write has completed | everything beneath it |

A read of `RUNNING` mixed with `STOPPED` is a process that died halting: no
`FLUSHED` quorum exists, nothing is vouched — crashed. A read of `STOPPED`
mixed with `FLUSHED` is a process that died completing its flush: the
`FLUSHED` copies vouch for the state beneath them — clean. Every mix
classifies; no mix is ambiguous. The classification takes the *minimum*
progress observed across the working copies, because a crash during the
marker writes must never read as a clean stop
(`uvrr-termination-obligations.md` §3).

## 3. The write schedules

The marker is four spaced, checksummed copies, quorum-read (§5). The rounds
of forced writes are fixed by the path.

**Controlled halt — two rounds.** Halt the read loop and drain the handlers;
the first round writes `STOPPED` to the copies; the dual-ring WAL is then
forced to stable storage; the second round writes `FLUSHED`. The deferred
application-state write follows, and the process exits. The shutdown monitor
watches the marker, not the process.

**Clean start — one round.** Read the copies; a `FLUSHED` quorum vouches for
the state beneath. One round writes `RUNNING` — the boot-gate latch —
before the first message is processed, so that a later crash can never be
mistaken for this clean start. The process reads its era from the vouched
state and runs.

**Dirty fast start — no round now, one round later.** The read shows no
`FLUSHED` quorum: the previous process never reached its drain point, the
identity is crashed, and this process is uninitialised. Time is of the
essence — the flush is not paid at the boundary. The process finds the
cluster, catches up, and rejoins first (`uvrr-rejoin-gossip-and-witnesses.md`
§2); the `RUNNING` latch write is deferred until the rejoin completes. A
second crash before the deferred write still reads no `FLUSHED` quorum and
classifies crashed again — the deferral is safe by construction.

## 4. The era rule

The genesis era is 1. Era 0 is not a protocol state, and there is no
in-cluster state synchronisation to a node at era 0: a node that cannot name
era 1 or later has nothing the cluster can synchronise against, and its only
route in is the rejoin gossip. A clean start reads its era from the vouched
durable state and re-synchronises in-cluster from that era; a blank or dirty
start adopts the cluster's era through the gossip before its engine sees
traffic.

## 5. The construction the schedules assume

The schedules assume a marker store whose reads survive the failure modes of
real disks: torn sectors, misdirected writes, and silent rot are single-copy
events, so no single copy is ever trusted. The worked construction is
TigerBeetle's, whose techniques are named here, not restated
([github.com/tigerbeetle/tigerbeetle](https://github.com/tigerbeetle/tigerbeetle)):
four superblock copies in fixed zones, checksummed and hash-chained, with
quorum writes and quorum reads that repair the lagging copies; direct I/O
with explicit forcing at the points where the marker must vouch for data
beneath it; and a dual-ring WAL underneath. The flush literature is cited at
`uvrr-termination-obligations.md` §4: a single `fsync`ed flag file is not
reliable enough (Pillai et al., OSDI 2014; Chidambaram et al., SOSP 2013),
which is why the marker is a quorum of copies and not a flag.

## 6. The test harness obligation

The contract is exercised in tests through crash-stop alone. The harness
records a node's durable state at crash and classifies the restart:
`restart_with` is the clean classification — the vouched disk reopens as
`Restarting` under the same identity; `restart_as` is the crashed
classification — the identity is retired and the node reincarnates under a
fresh one. No superblock writes are required of a test host: a plain marker
record on the test filesystem meets the obligation. What the harness must
never do is restart a crashed node silently as a clean one — error-on-
crashed is the contract (`uvrr-termination-obligations.md` §3).

## Figures

- [Figure 1 — the marker state machine](diagrams/uvrr-boot-gate-states.svg):
  the three states, the transition that writes each, and the crash edges
  that write nothing.
- [Figure 2 — the three boot classifications](diagrams/uvrr-boot-gate-sequence.svg):
  clean start, controlled halt and start, and the dirty fast start with its
  deferred latch.
