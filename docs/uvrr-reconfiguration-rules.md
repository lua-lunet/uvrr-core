# uVRR reconfiguration rules: the command alphabet, its boundaries, and the era planner

The rules of this document are mechanically confirmable. Each one is checked by
exactly one refusal in the fold (`src/configuration.rs`) or the era planner
(`src/reconfiguration.rs`), and each is exercised by a named test in
`tests/reconfiguration_plan.rs` or `tests/reincarnation.rs`. The safety argument
behind them is integer arithmetic over weighted majorities; its named forms are in
§8. This document states what the rules ARE; `docs/uvrr-reincarnation.md` states how
reincarnation uses them; `docs/architecture.md` records the decision.

## 1. The voting-weight domain: {0, 1, 2}

Voting weights are common factors. A configuration whose weights are
`9, 9, 9` decides with two of three nodes: `18/27 = 2/3`, the same split as
`(1,1,1)`. Any uniform scaling is a zero effect on quorums, so no node ever needs a
weight above `2`:

| Rule | Statement |
|---|---|
| R1 | A member's voting weight is `0`, `1`, or `2`. No operation may produce any other value; the fold refuses. |

`Double` and `Halve` are therefore usable exactly once per doubling cycle (the
§3 scaling rules), which keeps every worked schedule inside the domain the closure
arguments were checked over.

A weight-0 member is a **standby** — TigerBeetle's term for its non-voting cluster
members (older drafts of this document called it a learner). Standby nodes have a zero
voting weight so cannot form part of any quorum nor actively participate in the VSR
algorithm. TigerBeetle ships exactly this role: its constants cap `standbys_max` alongside
`replicas_max` and sum them into `members_max`, and its primary replicates to other
replicas *and* standbys (`send_message_to_other_replicas_and_standbys` in
`src/vsr/replica.zig`) while every quorum is sized on the replica count alone.

A nine-node deployment of three voting nodes across three data centres with six
standbys is a **three-node cluster** with six warm standbys — the quorum arithmetic
runs entirely over the voters. There is no bound on the number of standbys other
than the membership cap, and standbys may come and go.

## 2. Membership rules

| Rule | Statement |
|---|---|
| R2 | A node joins the cluster at voting weight zero (`Join` inserts a standby; there is no operation that joins at any other weight). |
| R3 | A node leaves the cluster only after its voting weight has been changed to zero (`Leave` is legal only at weight 0). |
| R4 | A zero-weight member never votes: it is not a voter in any quorum evaluation, so it is not counted against any quorum. |
| R5 | A leader does not count any response from a zero-weight member toward any vote, and every replica drops view-change-like messages from a zero-weight member. The one message a non-voting identity may send that is never dropped is the `Reincarnation` announcement (§4 of `docs/uvrr-reincarnation.md`) — the entry ticket. The one response a departing identity's answer still counts toward is the non-stop transition's own planned quorum (§8.7.7): the pivot places the departing member inside `qI` and the construction solicits exactly that vote, so the §6 membership-discard (`docs/uvrr-reincarnation.md`) admits precisely that solicited `EvidenceKind::Planned` answer past ingress — every other message from the departed identity stays refused by name. |
| R6 | A leader sends prepare and commit to zero-weight standbys so they stay caught up: warm standbys, swappable in by uVRR. |

## 3. The operation alphabet

`ABDICATE`, `DOUBLE`, `HALVE`, `INCREMENT(n)`, `DECREMENT(n)`, `JOIN(n, position)`,
`LEAVE(n)` — plus the two genesis operations `VOID` and `INIT` (genesis is not
plannable; see §6). Each operation has its own refusal, and refusals are total:
nothing saturates, nothing rounds, no partial application.

| Rule | Operation | Boundary | Refusal |
|---|---|---|---|
| R7 | `INCREMENT(n)` | `W(n) + 1 ≤ 2` | the cap variant: the node would leave the domain |
| R8 | `DECREMENT(n)` | `W(n) ≥ 1`; resulting total ≥ 1 | the standby has no weight to give; a zero total is quorum-impossible |
| R9 | `DOUBLE` | every `W(n) ≤ 1`, so every doubled weight stays in the domain | the first member that would pass 2 is named |
| R10 | `HALVE` | every `W(n)` even (`0` or `2`); no rounding | the first odd member is named |
| R11 | `JOIN { node, position }` | `node` not a member; inserts at weight 0 | duplicate, cap, or position out of range |
| R12 | `LEAVE(n)` | `W(n) == 0` | the member still votes |

## 4. The era rule: what may commit as one reconfiguration

One reconfiguration commits a **batch**: a set of operations applied together, in
order, establishing one era. The batch is legal exactly when:

| Rule | Statement |
|---|---|
| R13 | A batch containing `DOUBLE` or `HALVE` contains nothing else. A scaling op is solitary. |
| R14 | Every other batch is a **unit batch**: over the union of both memberships, `Σ_a |W_before(a) − W_after(a)| ≤ 1`, where a node absent on one side weighs 0. |
| R15 | `VOID` and `INIT` never appear in a batch, and a batch never nests a batch. Genesis is not plannable. |

The unit rule R14 is deliberately stated over **per-node mass moved**, not over the
net total change. The net total change of any legal batch is `−1`, `0`, or `+1`
(R14 implies it), but the converse is false: `[DECREMENT(c), JOIN(d), INCREMENT(d),
LEAVE(c)]` applied to `(a:1, b:1, c:1)` has net change `0` yet moves mass `2`
(`c` down one, `d` up one), and its endpoint is the identity swap
`(a:1, b:1, c:0, d:1)` whose era-`e` majority `{b, c}` and era-`e+1` majority
`{a, d}` are disjoint. Per-node mass moved is the checkable condition; net change
alone would admit the swap it exists to forbid.

Zero-weight joins and leaves move no mass (`|0 − 0| = 0`), so **any number of
standbys may join or leave in one era** — R14 admits them without bound, which is
the property R1–R4 rest on.

## 5. The planner: reduce-left batch evaluation

The leader never proposes an unsafe batch. Given a stream of operations it runs the
**reduce-left partitioner**: a fold carrying the tuple
`(taken ops, configuration so far, mass moved so far)`. For each incoming operation:

1. try the operation against the accumulated in-batch configuration;
2. if the growing batch stays legal (R13–R15 and every per-op boundary at its point
   in the sequence), **take** it: extend the taken list and the configuration;
3. if it would violate a rule, **pass only the prior list**: close the batch — the
   taken ops become one era's establishing operation — and retry the operation as
   the first op of the next batch.

The classic shapes fall out mechanically:

- *A ton of zero-weight joins, then a double*: every `JOIN` moves no mass, so the
  reduce keeps taking them; the `DOUBLE` cannot join a batch that has ops (R13), so
  it closes the joins and starts its own solitary era.
- *The reincarnation four* — `[DECREMENT(c), JOIN(d), INCREMENT(d), LEAVE(c)]` —
  **split nicely into two**: after `DECREMENT(c), JOIN(d)` the mass moved is `1`;
  adding `INCREMENT(d)` would move `2`, so the batch closes and the next era takes
  `INCREMENT(d), LEAVE(c)` (mass `1`). The canonical two-era form
  `(a:1,b:1,c:1) → (a:1,b:1,c:0,d:0) → (a:1,b:1,d:1)` is exactly what the
  partitioner produces, and each intermediate era is quorum-safe.
- *The reorder attempt* `(a:1,b:1,c:1) → (a:1,b:1,c:1,d:1) → (a:1,b:1,c:0,d:1)` is
  not expressible as legal ops in one era: the alphabet has no join-at-weight-one,
  and the batch that would reach it moves mass `2`, which R14 refuses. The
  partitioner splits the reordered stream into its own legal eras instead.

The what-if is the same arithmetic on a clone: everything is an immutable value and
every operation is a pure function `Configuration → Result<Configuration>`, so a
leader evaluates a proposed batch on a clone before proposing it for consensus,
thread-safely, with no lock and no mutation.

The planner's output is what travels on the wire. Under Fuse
(`docs/uvrr-fuse.md`) a schedule of at least two operations is packed —
one datagram per recipient, the shared ballot in the envelope header, each op
at its own consecutive slot. The planner certifies the sequence once; the
envelope is delivered atomically; the first op in the batch decides the whole
batch.

## 6. The reincarnation sequence

The leader computes the sequence from the **current committed configuration** and
the announced `(old, new)` pair; it never stores steps. Each step is a batch, each
batch commits one era, and recomputation is idempotent: a leader crash mid-sequence
leaves a legal intermediate era from which the next leader recomputes the remainder.

For a three-node unit cluster the replacement is two eras. The general
unit-decrement fallback is:

| Old identity's state | Remaining eras |
|---|---|
| weight `w ≥ 2` | `w−1` solitary `DECREMENT` eras, then `[DECREMENT(old), JOIN(new)]`, then `[INCREMENT(new), LEAVE(old)]` |
| weight `1` | `[DECREMENT(old), JOIN(new)]`, then `[INCREMENT(new), LEAVE(old)]` |
| weight `0` | `[JOIN(new), LEAVE(old)]`, then `[INCREMENT(new)]` |
| already evicted | `[JOIN(new)]`, then `[INCREMENT(new)]` |

A five-node unit cluster uses the weighted replacement below. The columns keep
identities fixed; zero includes an identity that is not a member. `JOIN(new)` uses
the old identity's succession position, and `LEAVE(old)` removes its zero-weight
membership.

| Stage | Batch | Old | New | Each of four survivors |
|---|---|---|---|---|
| Initial | — | 1 | 0 | 1 |
| Double | `DOUBLE` | 2 | 0 | 2 |
| Introduce | `JOIN(new), INCREMENT(new)` | 2 | 1 | 2 |
| Reduce old | `DECREMENT(old)` | 1 | 1 | 2 |
| Remove old | `DECREMENT(old), LEAVE(old)` | 0 | 1 | 2 |
| Increase new | `INCREMENT(new)` | 0 | 2 | 2 |
| Halve | `HALVE` | 0 | 1 | 1 |

These are six transitions and seven configurations. The four unchanged survivors
identify the doubled scale during recomputation, including after the old identity
has left: promotion to two and halving remain part of the sequence. Every row
recomputes exactly its uncommitted suffix.

This extends [Turner's weighted three-node example](https://github.com/DaveCTurner/paxos-membership/blob/raft-like-reconfiguration/paxos-reconf.tex)
to five nodes; the tests check this particular schedule's quorum arithmetic.
The finite schedule does not characterise the full space of legal reconfigurations.

With available weight `A > D` (unavailable weight) and a positive live leader,
adding phantom weight `A-D-1` produces total `2A-1`. The available identities and
the unavailable identities plus the leader are abstract majority quorums whose
intersection is exactly the leader; all preparing-quorum promises come from the
available side. Choosing a smaller preparing quorum bounds the required padding
to three vote units on at most two phantom identities in the `{0,1,2}` palette.
More generally, any two configurations with an available weighted majority and
a positive retained live leader admit a finite sequence of legal unit edits
preserving availability and adjacent quorum intersection. These are properties of
the identity-weight space; phantom membership is not a received acknowledgement.
The [constructive proofs and generated checks](../research/weighted-reachability/README.md)
state the exact predicates and cover arbitrary finite cluster sizes.

## 7. The worked schedules (the blog grids)

The canonical hot-swap schedules, as published in *Paxos Voting Weights*
(2017-03-16). Columns are servers `W, X, Y, Z` across three zones; `–` means not a
member. Each row is one committed era, each consecutive pair is era-safe, and the
partitioner reproduces each grid from its flat op stream — see
`tests/reconfiguration_plan.rs`.

**Grid 1 — the unit hot swap** (three zones, unit weights):

| W | X | Y | Z |
|---|---|---|---|
| 1 | 1 | 1 | – |
| 1 | 1 | 1 | 1 |
| 1 | 1 | – | 1 |

Op stream: `JOIN(Z)`, `INCREMENT(Z)`, `DECREMENT(Y)`, `LEAVE(Y)` →
eras `[JOIN(Z), INCREMENT(Z)]` then `[DECREMENT(Y), LEAVE(Y)]`.

**Grid 2 — the doubled-scale safe replacement** (the even-nodes optimisation;
weights run `0, 1, 2` only):

| W | X | Y | Z |
|---|---|---|---|
| 1 | 1 | 1 | – |
| 2 | 2 | 2 | – |
| 2 | 2 | 2 | 1 |
| 2 | 2 | 1 | 1 |
| 2 | 2 | – | 1 |
| 2 | 2 | – | 2 |
| 1 | 1 | – | 1 |

Op stream: `DOUBLE`, `JOIN(Z)`, `INCREMENT(Z)`, `DECREMENT(Y)`, `DECREMENT(Y)`,
`LEAVE(Y)`, `INCREMENT(Z)`, `HALVE` → eras
`[DOUBLE]`, `[JOIN(Z), INCREMENT(Z)]`, `[DECREMENT(Y)]`, `[DECREMENT(Y), LEAVE(Y)]`,
`[INCREMENT(Z)]`, `[HALVE]`. The final `HALVE` is integral because the joiner was
driven to weight 2 first — the doubled corner that makes the return to unit weights
possible at all.

## 8. The named mathematics

The user's statement — "the proofs are trivial, it's the property of the
intersections of integer sets" — has standard names:

| Rule | Named result |
|---|---|
| Two strict majorities of one configuration intersect | the **pigeonhole principle**: strict-majority threshold `floor(T/2) + 1` exceeds half the total, so two disjoint sets would exceed the total |
| Majorities intersect when the **per-node mass moved** is ≤ 1 | Turner's **Lemma 2** (general weighted-majority overlap), formalized as ladder rung 9 `WeightedGeneral.scaled_overlap` |
| `DOUBLE`/`HALVE` change no quorum family | **common-factor normalization**: the majority threshold `floor(T/2) + 1` scales with `T`, so `18/27 = 2/3` — the "zero op" |
| Even totals need one more than half, odd totals split exactly | `floor(T/2) + 1` is the strict-majority threshold: even `T` is "eager" (`2n → n+1`), odd `2n+1 → n+1` splits evenly |

The exhaustive check the tests use is `vrr::quorum::validate_transition` (the full
disjoint-pair search across the union node set); the planner's unit rule is the
cheap sufficient precondition a leader evaluates as a what-if, and every schedule
this document admits passes the exhaustive check.

## 9. State, snapshot, and the operation WAL

Cluster state is **folded history, never ambient mutation**:

- A `Configuration` is an immutable value; `apply` and the planner are pure
  functions. No lock, no shared mutable state: the what-if on a clone is
  thread-safe by construction.
- The durable record is a **snapshot** plus a **WAL of legal operations**. The
  snapshot is a validated object: it serializes (`serde` feature) and re-inflates
  only through a constructor that re-checks every invariant (domain {0,1,2},
  duplicate-free, non-empty unless era 0, total ≥ 1). The WAL carries
  `SystemOperation` values — already serde and wire types — and replay is the fold.
- A batch commits as one establishing operation (one era, one WAL entry), so the
  WAL is exactly the sequence of legal eras the planner produced.

## 10. No Crash-Recover, by design

There is no amnesia recovery protocol. A node that loses volatile state is a
**different node**: it bumps its identity, re-announces `(old, new)`, and the
leader drives the reincarnation sequence of §6 above. Same-identity recovery after
volatile-state loss is unrepresentable; classic VRR-2012 diskless quorum recovery
is literature about a design this protocol does not implement.

## 11. Test obligations

Every rule has a test that can fail. The matrix:

| Test | Confirms |
|---|---|
| many zero-weight joins in one era | R2, R4, R14 mass 0 |
| standbys coming and going in one era (mixed `JOIN`/`LEAVE` at 0) | R3, R14 |
| one unit change plus standbys in one era | R14 mass 1 |
| two unit changes in one era refused (`BatchMassMoved`) | R14 sharpness |
| the identity swap in one era refused (the `0`-net, mass-`2` batch) | R14, not net-total |
| the reincarnation four split into exactly two eras by the planner | §5, the canonical two-step |
| the reordered stream partitions into legal eras | the partitioner, mechanically |
| `DOUBLE`/`HALVE` solitary: refused with company, split off alone in a stream | R13 |
| `DOUBLE` with a `2` present refused; `INCREMENT` at `2` refused; `HALVE` with a `1` refused; `DECREMENT` at `0` refused; `LEAVE` at `w > 0` refused; weights never negative or above `2` after any legal sequence | R7–R12 boundaries |
| Grid 1 and Grid 2 reproduced row-by-row by the planner, every era passing `validate_transition` | §7, the blog grids |
| three-node two-era and five-node six-era replacement, every row folded, exact suffix recomputation, leader-crash continuation, and availability distinct from intersection safety | §6 |
| snapshot serde round-trip; tampered snapshot refused at inflation | §9 |
| snapshot + WAL fold ≡ folding the flat planned stream | §9 |

## 12. The abdication message

`ABDICATE` is the administration verb of the alphabet. It is a reconfiguration
message sent by an administrator to the leader; it is not a folded batch
operation and it never changes a weight. Its two fields are the CAS pair: the
view the sender believes the cluster is in, and the view the cluster should
move to. The CAS pair is in succession terms: the succession order over the
configuration's members names each view's primary, so the target view names
the successor the administrator wants.

**Obligations of the sender.** The message names both views and is addressed to
the current leader. The sender performs no protocol participation: the
validation function below is the only thing that turns an abdication into
protocol traffic.

**The validation function.** A pure function takes the abdication message and
the cluster membership and returns the standard protocol messages to send, or a
named refusal. It checks, in order:

1. the CAS: the cluster is in the view the message names;
2. the receiver is the primary of that view;
3. the delta rule: `0 < (target - current) <= N`, where `N` is the member count
   of the current configuration.

The delta rule bounds the bump from both sides. The primary schedule advances
one node per view along the succession order, so a legitimate relocation skips
the intervening members: moving the leader from `DC1:a` to `DC3:a` in a cluster
holding `a,b` per site is a bump of `+4` past `DC1:b`, `DC2:a`, `DC2:b`. The
upper bound `N` keeps that arithmetic honest: a bump larger than the total set —
view `1234` asked to become view `2^32` — would waste the succession space the
schedule depends on, so it is refused. A non-positive delta is refused: views do
not move backwards.

**The transition.** On a valid abdication the leader emits the standard
view-change messages for the target view — no new wire message exists; the
abdication is answered by exactly the protocol messages the ordinary era change
would run. The leader stops being the primary at the same instant: any further
message it sends is ignored, discarded by the standard membership and fence
checks of the new era. The successor named by the target era's schedule resumes
as primary.

**No new proof is owed.** The transition is composed entirely of standard
protocol messages, so the standard era-change argument governs it; the
obligations above are validation and test obligations, not proof obligations.

**Test obligations.** Each refusal is checked by one named test through the
public interface: the CAS failure, the non-primary receiver, the non-positive
delta, the delta above `N`, and the valid relocation path — the leader's
emission is the standard view-change set, and its own subsequent messages are
discarded.
