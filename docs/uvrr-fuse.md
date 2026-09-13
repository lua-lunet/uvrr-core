# uVRR Fuse: one reconfiguration schedule, one datagram, one round trip

Fuse is a wire envelope that packs the per-slot Phase2 (`Prepare`) messages of one
reconfiguration schedule into a single datagram. It is not a new protocol semantic:
receiving one `Fuse` is *defined* as receiving the equivalent sequence of `Prepare`
messages at the same ballot, one per slot, in batch order. Per-op safety is the
ordinary accept path; only the wire shape changes. This document states what Fuse
IS; the command alphabet and its boundaries are `docs/uvrr-reconfiguration-rules.md`,
and the durability framing is `docs/vrr-durability-model.md` §13.8.

## 1. The envelope

A `Fuse` is one protocol message with the standard 20-byte header. The header
carries the common ballot once:

| Field | Role |
|---|---|
| `view`, `era` | the ballot shared by every packed op |
| `slot` | `first_slot`: the slot of the first packed op; each subsequent op occupies `first_slot + i` |
| body | `count`, then `count × SystemOperation`, in batch order |

The body discriminant and pack/unpack live beside the membership command codec
(`src/configuration.rs`); the tag tables carry the mapping. No range encodings:
the count names the things and the things follow, one slot at a time.

## 2. The atomic batch property

A Fuse envelope is a single datagram carrying a cache-line-scale payload (a full
cluster reconfiguration is at most seven operations). It travels under the
transport's checksum. Loss or interruption *inside* a batch is impossible: the
envelope arrives whole or not at all.

Therefore the first operation in the batch decides the whole batch. Membership,
view, era, and slot discipline are checked on `first_slot`; if the first op is
acceptable, every following op is acceptable at its own slot under the same
ballot — a promise is a promise to all future accepts of that ballot. "First
accepts ⇒ all accept" is a finite induction over at most `FUSE_MAX_OPS`
operations, not a proof over interruption points mid-sequence. Majority and
quorum accounting are computed on the first op in the batch.

Consequences:

- No inductive proof over interruptions during cluster reconfiguration is owed.
  The only interruption point in the state space is at the envelope boundary.
- Loss and partition testing acts at envelope granularity: one atomic action per
  reconfiguration batch.

## 3. The acceptor transition

On receiving a `Fuse` from the primary:

1. Validate the header once, exactly as `plan_prepare` validates a `Prepare`:
   sender is primary of the message's view, view discipline, era discipline of
   the first op, and the boot-adoption carve-outs.
2. If the header refuses, the whole envelope is dropped and one named
   diagnostic is recorded. Nothing is partially applied; there is no partial
   acceptance and no wire nack.
3. If the header passes, explode the batch logically: for each `i`, slot
   `first_slot + i` must equal the local accept frontier's next slot, and the
   op folds through the ordinary system-op perimeter (the same `Configuration`
   refusals R1–R15 that an individual `Prepare` would meet). The accept frontier
   advances per op and the journal records the batch as one contiguous accept.
4. Any op refusing mid-batch is unrepresentable when the header passed and the
   schedule is legal: the planner already certified the sequence, and slots are
   consecutive. If an implementation-level refusal nevertheless occurs, the
   whole envelope is dropped with a `FuseRefusal` diagnostic — never a partial
   fold.
5. All ops accepted: reply `FuseOk` — `count`, then one accepted slot per op in
   batch order. No ranges.

## 4. The leader transition

On proposing an establishing batch of at least two operations (a single-op
batch travels as an ordinary `Prepare` and needs no envelope):

1. Build one `Fuse` per recipient backup: header `first_slot` = the next
   unsent slot, body = the batch's operations in plan order, one op per
   consecutive slot. The batch's legality is gated whole before the
   envelope is built: the same reconfiguration gates an ordinary
   establishing `Prepare` passes, run on the batch the envelope packs.
2. Register one proposal record per packed slot; the leader's own vote is
   implicit, as in the ordinary path.
3. One `FuseOk` is ONE atomic vote (§2): the leader counts the sender
   once, cumulatively onto every outstanding slot the header slot covers —
   the header slot is the batch's last slot as the acceptor stamps it, so
   the vouch spans the batch whole. Majority is computed on the first
   message in batch and the remaining slots telescope. The `acks` body is
   the acceptor's wire evidence and is not examined for counting.
4. When every packed slot holds a quorum, the commit cascade commits the
   batch whole: the packed schedule folds as the ONE establishing batch it
   is — a maximal run of consecutive system entries in the journal — and
   establishes exactly one era, at the batch's first slot (§8.7.1). The
   commit emission is PER ERA: one `CommitBatch` per establishing batch
   committed — `count`, then one committed frontier per slot of that
   batch, in batch order. No ranges. The `CommitBatch` is a broadcast
   fired the instant the batch's quorum completes, never a round trip;
   backups advance their frontiers and fold the era through the ordinary
   commit announcement that travels with it. A fast-forward single-commit
   collapsing the batch is future work.
5. Fallback: if the batch exceeds `FUSE_MAX_OPS` or the builder's envelope
   budget, the leader emits the ordinary per-op `Prepare`s instead. The
   codec has no size constant (W5); the builder's cap is a build-site
   policy, and both paths are the same sequence of logical accepts, so
   safety is identical.
6. Leadership loss while awaiting the majority clears the pending fuse slots
   with the epoch, exactly as ordinary pending proposals die at view-change
   install. No new failure mode.

## 5. What Fuse does not do

- Client operations never fuse. The envelope carries reconfiguration operations
  only; client traffic continues at higher slots during the fuse round with no
  head-of-line blocking. No interleaving of client and admin commands is
  required: the leader chooses the whole admin schedule at once.
- Fuse slots are ordinary journal slots. No new persistence obligation, no new
  flush, and no flush elision: the durability profile in force applies
  unchanged.
- Fuse introduces no new quorum family, no new configuration state, and no new
  era rule. It is message packing.

## 6. The single round trip

One establishing batch — the two-operation crossing batch of a three-node
reincarnation, one batch of the five-node weighted sequence — travels as
one datagram per recipient: leader to each backup, `FuseOk` back,
`CommitBatch` out. One round trip per era. The cross-DC maximum RTT of
3.3 ms measured in `docs/uvrr-experiment-design.md` bounds each
reconfiguration round trip; the per-op `Prepare` round trips within an era
no longer multiply it. The eras themselves stay paced by the closed
era/slot discipline (§8.7.3): a commit transition folds one era
establishment, and the next era's batch is proposed after the ordinary
view change into the era its predecessor established — exactly the
sequence the ordinary path runs, with each era's Phase2 round collapsed
into one datagram.

## 7. Test obligations

- The leader of an armed schedule emits exactly one `Fuse` per backup; the
  body carries the schedule's operations in order with the correct
  `first_slot`.
- A stale-view acceptor refuses the whole envelope: zero slots accepted, one
  diagnostic, no partial fold.
- A valid acceptor accepts all slots, records one contiguous journal accept,
  and replies one `FuseOk`.
- On quorum, every packed slot commits in order; one `CommitBatch` is
  broadcast; the era table advances through the whole schedule; subsequent
  client proposals land above the fused range.
- The oversized schedule falls back to ordinary `Prepare`s.
- The existing reconfiguration, reincarnation, and cold-start corpora pass
  unmodified: Fuse changes the wire, not the machine.
