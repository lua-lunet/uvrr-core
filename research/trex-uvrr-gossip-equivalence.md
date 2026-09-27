# The equivalence question: VRR-2012 crash-recovery against the uVRR rejoin gossip

This record resolves the design question the rejoin gossip raises against
the protocol it replaces: is the gossip catch-up — the resend request, the
leader's push from the requester's frontier, the fast-forward commit —
equivalent to VRR-2012 §4.3 recovery, the quorum read? The answer is
layered: the two are the same mechanism at the liveness and network-cost
layers, and different mechanisms at the safety layer. The difference is not
incidental; it is the design.

## 1. The two mechanisms, stated

**VRR-2012 §4.3 (Liskov & Cowling).** A recovering replica sends RECOVERY,
stamped with a durable nonce, to every replica. A replica answers only when
Normal; the primary of the current view answers with its log. Recovery
completes when a quorum of answers including the primary's has arrived. The
mechanism is a pull: one quorum read, one transfer. Its preconditions are
memory. The requester must remember its identity across the crash, the
nonce must be durable, and — the load-bearing assumption — the promises the
replica made before the crash must survive it. Michael, Ports, Sharma and
Szekeres (*Providing Stable Storage for the Diskless Crash-Recovery Failure
Model*, UW-CSE-16-08-02; the DISC'17 Appendix B.1 amnesia class) falsified
the diskless case: a replica can send a view-change commitment, crash,
return in an older view, and promise again — a double promise that breaks
the very view change the commitment gated. VRR-2012 recovery is safe only
under stable storage; the diskless form its own economy argues for is the
form the literature proved unsafe.

**The uVRR rejoin gossip.** A crash is final for the protocol identity
(`docs/uvrr-reincarnation.md` §1); no recovery protocol exists. The boot
gate (`docs/uvrr-boot-gate.md`) classifies the start from durable evidence
before the engine sees traffic. A start that cannot vouch for its memory —
blank or dirty — gossips its frontiers to every node it knows
(`docs/uvrr-rejoin-gossip-and-witnesses.md` §2). Every node that hears the
gossip records the sender as a gossip-witness; only the leader acts: it
pushes the phase-2s above the sender's frontier and then a fresh commit,
and it keeps pushing all phase-2s and commits until a committed
reconfiguration promotes the sender and every node drops it from the list.
The mechanism is a push: a resend loop for reliability, a witness list at
every node for failover, the stream for keep-up.

## 2. The layer where the equivalence holds

Strip both to what they transfer and who may transfer it.

- **The content is identical.** Both move exactly the committed-and-accepted
  suffix above the stale node's frontier. Nothing else is ever missing, and
  nothing else is ever sent.
- **The authority is identical in shape.** In VRR-2012 the primary's answer
  inside a quorum of responses is what makes the transferred log
  installation evidence rather than rumour. In the gossip the leader of the
  current view is the streamer, and its right to name history was earned
  from the members by the ordinary phase one.
- **The implicit promise makes the push safe.** A promise at ballot *b*
  covers every lower ballot (Lamport); a phase-2 accepted at view *v*
  therefore raises the receiver's promise floor to *v* exactly as if a
  prepare at *v* had arrived first. The leader needs no recovery round to
  push: contiguous increasing views make the stream self-certifying. This
  is the telescoping the paper states as construction rules H1–H5 with
  kernel-checked consequences (the adopted era never exceeds the committed
  era; once the leader answers, it equals it; the promoted witness's
  committed prefix is indistinguishable from a member that caught up by
  ordinary state transfer).

So the quorum read and the gossip stream are two economic points of one
catch-up mechanism. The pull pays one quorum read and one transfer — the
fewest messages, and it requires the requester to know who it is. The push
pays a resend loop and an open-ended keep-up stream — more messages, and it
requires the requester to know nothing. At the liveness and network-cost
layers the choice between them is accounting, not semantics.

## 3. The layer where the equivalence fails

The safety preconditions differ, and they differ on memory.

VRR-2012 recovery is safe only if the recovering replica's prior promises
survive its crash. That is a stable-storage requirement sitting inside the
protocol, and the amnesia literature shows the requirement is real: remove
it and the protocol admits a counterexample, not merely a degraded run.

The gossip carries no such requirement. The rejoining node's authority
comes from the committed configuration, never from its memory of itself; a
node that remembers nothing gossips from blank and is streamed until a
committed reconfiguration promotes it. Safety is invariant under the
node's amnesia because nothing the node forgot is load-bearing.

The honest statement of the equivalence is therefore:

> gossip ≡ recovery, with the memory precondition removed.

The mechanisms coincide on content, authority, and cost shape; they part
exactly where VRR-2012 trusts the requester's memory and uVRR refuses to.

## 4. The resolution of the split

There were never two competing designs. VRR-2012's recovery is the same
catch-up mechanism carrying a memory requirement uVRR declines to assume —
and must decline, because the deployment it targets is diskless on the
normal path and the amnesia counterexample lives precisely there.

uVRR keeps the mechanism and moves the memory question out of the protocol
into the boot gate. The durable marker answers *may this process trust its
own state?* before the engine sees traffic, and the classification picks
the economics at runtime:

- **Clean start — memory verified.** The `FLUSHED` quorum vouches; the node
  reads its era and re-synchronises in-cluster as though a network
  partition had healed. This is the cheap pull-shaped path, legitimately:
  the node can prove what it remembers, so the in-cluster sync owes it no
  gossip.
- **Dirty or blank start — memory absent.** The gossip path: no memory is
  required, because none is trusted. The stream telescopes the promise, the
  witness list carries the node across failover, and promotion retires the
  stream.

One mechanism, two safety postures, and a durable classification that
chooses between them. The confusion dissolves into that sentence.

## 5. What remains open

- The kernel-checked statement of the telescoping equivalence at the
  mechanism layer. The addendum proves the consequences (H1–H5); a single
  theorem naming the quorum read and the gossip stream as one transfer
  under one authority is future work.
- The join half in the engine. The answer half is shipped (every node
  records the witness; the leader streams and commits); the boot-fenced
  node's own gossip emission is the outstanding producer.
