# uVRR protocols: reconfiguration, reincarnation, rejoin, and the leader's instruments

The protocol documents in one place. The reconfiguration-rules chapter
states the command alphabet, its boundaries, and the era planner; the
reincarnation chapter states the Crash-Stop-Self-Evict protocol that uses
them; the rejoin chapter states the gossip and witness layer outside the
main protocol; the solver chapter states the operator's weighted
reconfiguration plans; the NOMINATE chapter states the leader-assignment
letter that rides them. Each chapter keeps its own section numbering: a
reference of the form "the reincarnation chapter §6" names a section within
that chapter.

## uVRR reconfiguration rules: the command alphabet, its boundaries, and the era planner

The rules of this document are mechanically confirmable. Each one is checked by
exactly one refusal in the fold (`src/configuration.rs`) or the era planner
(`src/reconfiguration.rs`), and each is exercised by a named test in
`tests/reconfiguration_plan.rs` or `tests/reincarnation.rs`. The safety argument
behind them is integer arithmetic over weighted majorities; its named forms are in
§8. This chapter states what the rules ARE; the reincarnation chapter states how
reincarnation uses them; `docs/architecture.md` records the decision.

### 1. The voting-weight domain: {0, 1, 2}

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

A weight-0 member is a **standby**, TigerBeetle's term for its non-voting cluster
members (older drafts of this document called it a learner). Standby nodes have a zero
voting weight so cannot form part of any quorum nor actively participate in the VSR
algorithm. TigerBeetle ships exactly this role: its constants cap `standbys_max` alongside
`replicas_max` and sum them into `members_max`, and its primary replicates to other
replicas *and* standbys (`send_message_to_other_replicas_and_standbys` in
`src/vsr/replica.zig`) while every quorum is sized on the replica count alone.

A nine-node deployment of three voting nodes across three data centres with six
standbys is a **three-node cluster** with six warm standbys, the quorum arithmetic
runs entirely over the voters. There is no bound on the number of standbys other
than the membership cap, and standbys may come and go.

### 2. Membership rules

| Rule | Statement |
|---|---|
| R2 | A node joins the cluster at voting weight zero (`Join` inserts a standby; there is no operation that joins at any other weight). |
| R3 | A node leaves the cluster only after its voting weight has been changed to zero (`Leave` is legal only at weight 0). |
| R4 | A zero-weight member never votes: it is not a voter in any quorum evaluation, so it is not counted against any quorum. |
| R5 | A leader does not count any response from a zero-weight member toward any vote, and every replica drops view-change-like messages from a zero-weight member. The one message a non-voting identity may send that is never dropped is the `Reincarnation` announcement (§4 of the reincarnation chapter), the entry ticket. The one response a departing identity's answer still counts toward is the non-stop transition's own planned quorum (§8.7.7): the pivot places the departing member inside `qI` and the construction solicits exactly that vote, so the §6 membership-discard (the reincarnation chapter) admits precisely that solicited `EvidenceKind::Planned` answer past ingress, every other message from the departed identity stays refused by name. |
| R6 | A leader sends prepare and commit to zero-weight standbys so they stay caught up: warm standbys, swappable in by uVRR. |

### 3. The operation alphabet

`DOUBLE`, `HALVE`, `INCREMENT(n)`, `DECREMENT(n)`, `JOIN(n, position)`,
`LEAVE(n)`, plus the two genesis operations `VOID` and `INIT` (genesis is not
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

### 4. The era rule: what may commit as one reconfiguration

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
standbys may join or leave in one era**, R14 admits them without bound, which is
the property R1–R4 rest on.

### 5. The planner: reduce-left batch evaluation

The leader never proposes an unsafe batch. Given a stream of operations it runs the
**reduce-left partitioner**: a fold carrying the tuple
`(taken ops, configuration so far, mass moved so far)`. For each incoming operation:

1. try the operation against the accumulated in-batch configuration;
2. if the growing batch stays legal (R13–R15 and every per-op boundary at its point
   in the sequence), **take** it: extend the taken list and the configuration;
3. if it would violate a rule, **pass only the prior list**: close the batch, the
   taken ops become one era's establishing operation, and retry the operation as
   the first op of the next batch.

The classic shapes fall out mechanically:

- *A ton of zero-weight joins, then a double*: every `JOIN` moves no mass, so the
  reduce keeps taking them; the `DOUBLE` cannot join a batch that has ops (R13), so
  it closes the joins and starts its own solitary era.
- *The reincarnation four*, `[DECREMENT(c), JOIN(d), INCREMENT(d), LEAVE(c)]`,
  **splits nicely into two**: after `DECREMENT(c), JOIN(d)` the mass moved is `1`;
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
(`docs/uvrr-fuse.md`) a schedule of at least two operations is packed:
one datagram per recipient, the shared ballot in the envelope header, each op
at its own consecutive slot. The planner certifies the sequence once; the
envelope is delivered atomically; the first op in the batch decides the whole
batch.

### 6. The reincarnation sequence

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
| Initial |, | 1 | 0 | 1 |
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

### 7. The worked schedules (the blog grids)

The canonical hot-swap schedules, as published in *Paxos Voting Weights*
(2017-03-16). Columns are servers `W, X, Y, Z` across three zones; `–` means not a
member. Each row is one committed era, each consecutive pair is era-safe, and the
partitioner reproduces each grid from its flat op stream, see
`tests/reconfiguration_plan.rs`.

**Grid 1, the unit hot swap** (three zones, unit weights):

| W | X | Y | Z |
|---|---|---|---|
| 1 | 1 | 1 | – |
| 1 | 1 | 1 | 1 |
| 1 | 1 | – | 1 |

Op stream: `JOIN(Z)`, `INCREMENT(Z)`, `DECREMENT(Y)`, `LEAVE(Y)` →
eras `[JOIN(Z), INCREMENT(Z)]` then `[DECREMENT(Y), LEAVE(Y)]`.

**Grid 2, the doubled-scale safe replacement** (the even-nodes optimisation;
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
driven to weight 2 first, the doubled corner that makes the return to unit weights
possible at all.

### 8. The named mathematics

The user's statement, "the proofs are trivial, it's the property of the
intersections of integer sets", has standard names:

| Rule | Named result |
|---|---|
| Two strict majorities of one configuration intersect | the **pigeonhole principle**: strict-majority threshold `floor(T/2) + 1` exceeds half the total, so two disjoint sets would exceed the total |
| Majorities intersect when the **per-node mass moved** is ≤ 1 | Turner's **Lemma 2** (general weighted-majority overlap), formalized as ladder rung 9 `WeightedGeneral.scaled_overlap` |
| `DOUBLE`/`HALVE` change no quorum family | **common-factor normalization**: the majority threshold `floor(T/2) + 1` scales with `T`, so `18/27 = 2/3`, the "zero op" |
| Even totals need one more than half, odd totals split exactly | `floor(T/2) + 1` is the strict-majority threshold: even `T` is "eager" (`2n → n+1`), odd `2n+1 → n+1` splits evenly |

The exhaustive check the tests use is `uvrr::quorum::validate_transition` (the full
disjoint-pair search across the union node set); the planner's unit rule is the
cheap sufficient precondition a leader evaluates as a what-if, and every schedule
this document admits passes the exhaustive check.

### 9. State, snapshot, and the operation WAL

Cluster state is **folded history, never ambient mutation**:

- A `Configuration` is an immutable value; `apply` and the planner are pure
  functions. No lock, no shared mutable state: the what-if on a clone is
  thread-safe by construction.
- The durable record is a **snapshot** plus a **WAL of legal operations**. The
  snapshot is a validated object: it serializes (`serde` feature) and re-inflates
  only through a constructor that re-checks every invariant (domain {0,1,2},
  duplicate-free, non-empty unless era 0, total ≥ 1). The WAL carries
  `SystemOperation` values, already serde and wire types, and replay is the fold.
- A batch commits as one establishing operation (one era, one WAL entry), so the
  WAL is exactly the sequence of legal eras the planner produced.

### 10. No Crash-Recover, by design

There is no amnesia recovery protocol. A node that loses volatile state is a
**different node**: it bumps its identity, re-announces `(old, new)`, and the
leader drives the reincarnation sequence of §6 above. Same-identity recovery after
volatile-state loss is unrepresentable; classic VRR-2012 diskless quorum recovery
is literature about a design this protocol does not implement.

### 11. Test obligations

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

## uVRR reincarnation: the Crash-Stop-Self-Evict protocol

### 1. Model statement

uVRR by definition does **not** allow Crash-Recover (CR). It enforces
**Crash-Stop-Self-Evict**, called **reincarnation**. A node whose volatile state was
lost is, by construction, a *different node*: it reopens under a new identity and its
old identity is evicted from the voting configuration. Same-identity recovery after
volatile-state loss is unrepresentable in the protocol. Classic VRR-2012 diskless
quorum recovery (its §4.3) is not implemented; there is no amnesia recovery protocol.
Classic VRR's published recovery failure (the DISC'17 Appendix B.1 amnesia class,
Michael et al.) is literature about the classic crash-recover class; uVRR eliminates
the class by construction rather than repairing it.

### 2. Durable identity contract: the four superblocks

Durable state is **four TigerBeetle-style superblocks**. Node restart reads all four.

**Marker semantics**:

- `flushed` = "my on-disk state is a self-consistent checkpoint of identity X".
  Written at **clean shutdown** (node stopped responding, flushed all writes, fsynced)
  and at the **bump** (after rewriting all four superblocks as the new identity).
- `unflushed` = running state, written when a node **starts** operating.

Startup classification:

1. All four read `flushed` → mark `unflushed`, continue normally. This is the
   ordinary CR-free path: no recovery protocol runs.
2. Any of the four reads `unflushed` → the node is **dirty**.

**Dirty path:** bump the incarnation (new identity), write new identity + `flushed`
to all four superblocks, then enter the wire phase (§4), the identity law of
`docs/uvrr-io-obligations.md`, the boot-gate chapter §5: the bumped pair is
flushed before the first
announcement, unconditionally, seated or not. The state machine is:

| State | Meaning |
|---|---|
| `flushed` | durable checkpoint of identity X; written at clean shutdown and after the bump |
| `unflushed` | running sentinel; written at start of operating |
| `dirty` | restart observed any-`unflushed`; eviction must begin |
| `bumped` | incarnation incremented; all four superblocks rewritten as (new identity, `flushed`) |
| `reincarnating` | wire phase: old identity pending eviction, new identity a weight-0 standby |

The identity is named by the durable pair `{systemIdentifier, crashCounter}`
(`docs/uvrr-io-obligations.md`, the boot-gate chapter §5): the
sysadmin-assigned `systemIdentifier` is burnt
into the marker before first boot; the crash counter is durable in the same
file; both are one-indexed and never read as zero; and a bumped value is
never revisited, universally unique per life, the ballot obligation of
Paxos Made Simple applied to node identity.

A **standby** is TigerBeetle's term for its non-voting cluster members (this document's
older drafts called it a learner). Standby nodes have a zero voting weight so cannot
form part of any quorum nor actively participate in the VSR algorithm.

**Read rule (higher-identity-wins):** any read of the superblocks may observe a higher
identity than the reader last knew; the reader adopts the higher identity.

**Continuation commitment:** once Crash-Stop-Eviction is initiated it MUST continue;
the forced sequence of §5 is never aborted mid-way.

### 3. Economic rationale: the network is faster than the disk

Reincarnation takes advantage of "The network is faster than the disk"
(<https://simbo1905.wordpress.com/2024/04/12/the-network-is-faster-than-the-disk/>).
For nano-state that fits in memory, disk flushes can be **deferred**: the forced
reconfiguration rounds of §5 are network round trips, which are cheaper than fsyncs.
The four-superblock mark is **one fsync amortized over the whole epoch**, not
per-operation durability. The dirty check is therefore paid once per restart, and the
correctness cost of same-identity amnesia is avoided entirely instead of being paid
per operation as a flush.

### 4. Wire message

A new **reincarnation** message carries the pair `(old identity, new identity)`,
sent by the bumped node to **all** nodes. **Only the leader responds**, and it
responds immediately, before it starts the cluster's forced reconfiguration (§5)
with an ack that carries the new era and the state the node missed: the
announcement names the slot the reincarnated node had committed in its past life
and what it had prepared, so the leader pushes exactly the missing range and the
node synchronises to the frontier at once. From the ack onward the leader memo-
streams all phase-1 and phase-2 messages to the reincarnated node even though it
is not yet in the cluster, so it stays up to date. Answering before the
reconfiguration is safe because the announcer is a non-member: its messages are
discarded by §6, and it does not vote until a committed reconfiguration has put

### 5. Forced weight sequence

The leader drives the forced reconfiguration sequence from the Paxos Voting Weights
rules (the reconfiguration-rules chapter: the operation alphabet `DOUBLE`, `HALVE`,
`INCREMENT`, `DECREMENT`, `JOIN`, `LEAVE`; each reconfiguration commits a legal batch,
one era). One reconfiguration commits a batch, and a batch either moves at most one
unit of per-node voting mass (standbys at weight 0 move none) or is one solitary
scaling op. Three-node unit-weight special case, evicting `N2` and reincarnating it as `N2′`,
**two eras**, each a batch:

| Era | Batch | Weights (N0, N1, N2, N2′) | Mass moved |
|---|---|---|---|
| D1 | `DECREMENT(N2), JOIN(N2′)` | (1, 1, 0, 0) | 1 |
| D2 | `INCREMENT(N2′), LEAVE(N2)` | (1, 1, –, 1) | 1 |

The old identity leaves only after its weight is zero, and the new identity joins at
zero, the two together in the crossing era move exactly one unit, and the eviction
era moves exactly one. **Era-safety invariant:** every consecutive pair of
configurations in the sequence satisfies the per-node mass rule (R14 of the
reconfiguration-rules chapter), so consecutive majority families intersect and **every intermediate era is
quorum-safe on its own**: a leader dying mid-sequence leaves only safe membership
eras, and a new leader recomputes the remaining eras from the configuration that
committed.

**Five-node replacement.** A unit-weight five-node cluster commits six batches:
`DOUBLE`; `JOIN(new), INCREMENT(new)`; `DECREMENT(old)`;
`DECREMENT(old), LEAVE(old)`; `INCREMENT(new)`; `HALVE`. The complete weight table
and availability conditions are in §6 of the reconfiguration-rules chapter.
Recomputation includes every remaining batch, including promotion and halving after
the old member leaves. Ordinary view changes separate these forced batches.
The broader identity-weight space also admits phantom completion of an abstract
leader overlap and available-majority paths between arbitrary admissible endpoints;
see the [weighted reachability proofs](../research/weighted-reachability/README.md).

**Doubled-weights corner.** If the reincarnation happens while the cluster sits in the
doubled state of the blog's safe-replacement plan, the sequence runs at the doubled
scale and the return to unit weights needs the extra unit step that makes the halve
integral:

| Era | Batch | Weights (N0, N1, N2, N2′) |
|---|---|---|
| C1 | `DECREMENT(N2)` | (2, 2, 1, –) |
| C2 | `DECREMENT(N2), JOIN(N2′)` | (2, 2, 0, 0) |
| C3 | `INCREMENT(N2′), LEAVE(N2)` | (2, 2, –, 1) |
| C4 | `INCREMENT(N2′)` | (2, 2, –, 2), **corner**: joiner at 2 so the halve is integral |
| C5 | `HALVE` | (1, 1, –, 1) |

Every era above is a legal batch of the reconfiguration-rules chapter, so every intermediate era is
quorum-safe by the same invariant. (Scaled steps preserve overlap by common-factor
normalization, the checked `WeightedGeneral.scaled_overlap` result covers each step.)

### 6. Membership-discard check

Messages **FROM** a node that is not in the current voting configuration, a weight-0
standby, or a superseded old identity, are **discarded** by the standard membership
check on ingress. Messages **TO** such a node are fine. This is good practice
independent of reincarnation: once a later incarnation is
known for a node, replies from the old identity never regain eligibility, regardless
of redelivery.

### 7. Streaming order

The leader stays live for client requests throughout. From its immediate ack (§4)
it **preemptively streams** to the reincarnated node **as a standby** (weight 0;
messages TO it fine, FROM it discarded via §6), all phase-1 and phase-2 messages,
so the node is current before the sequence reaches its promotion. It streams, in
order:

1. the ack with the missed range (the announcement names the committed slot and
   the prepared slot of the node's past life),
2. the **first reconfiguration** (the forced sequence of §5), with the memo
   stream running alongside,
3. then **client traffic**.

### 8. Leader-crash ordering

If the leader crashes mid-sequence, the cluster must reach a **stable leader** before
the reincarnated node forces its old-identity eviction. Because every intermediate era
is quorum-safe (§5), whichever safe era the crash lands in is a legal starting point
for the new leader, which resumes the sequence. The reincarnated node does not force
eviction of the old identity until a stable leader exists to drive it.

### 9. Relationship to the checked ladder

- Rungs 3, 4, 6, 9 (`Synod`, `Eras`, `CastingVote`, `WeightedGeneral`): the forced
  sequence is a path through the weighted-era space; each step's safety is the
  weighted-overlap argument, and the leader-overlap round is rung 6's casting vote.
- Rung 22 (`Reincarnation.lean`, `ReincarnationGeneral.lean`): the definitions and
  state machine above, with the four general theorems discharged at arbitrary scale:
  bumped-identity non-membership from eviction on (`evicted_never_voting`), quorum
  safety of every intermediate era (`forced_sequence_era_safe`), unreachability of
  the classic amnesia trace (`amnesia_unreachable`), and continuation commitment
  (`forced_run_committed`, `forced_run_terminal_flushed`).
- Rung 23 (`CastingVoteReincarnation.lean`): the casting vote on the reincarnation
  eras, the two-node leader-overlap case and the three-node degenerate negative.
- Rung 24 (`ReincarnationFive.lean`): the five-voter instance, computed majority
  families, era safety by the general discharge, the E1/E2 pivot at `n3`.
- Rung 25 (`ReincarnationAgreement.lean`): agreement across the forced sequence
  under every view schedule. The era family `E0, E1, E2, E2, …` satisfies P1 at
  arbitrary scale (`sequence_p1`), so rung 4's Theorem 10 instantiates: any two
  chosen ballots of one instance agree for every history satisfying P2–P7
  (`sequence_agreement`, `five_agreement`). View changes, within an era, across the
  two boundaries, the leader-overlap schedule included, are ballots of that history
  and change nothing. What a view change can do is take the majority away: with the
  victim dead, three survivors form one quorum that is a majority in every era and two
  survivors form none (`five_majority_boundary`); the sequence stalls until a majority
  is live, and no disagreement is reachable.

- Rung 26 (`ReincarnationSafety.lean`): the final composition, per-slot agreement across the replacement schedule at unit weights, is discharged as `five_safe` over eras 0-2, sealing the ladder's lifecycle premises.

The TLC counterpart is `formal/VrrCoreReincarnation.tla`: the two-era sequence with
variable leadership, view changes mid-sequence, crash-stop identities, and per-era
acknowledgement quorums, checked exhaustively at five nodes with the leader-loss and
majority-loss configurations.




### 10. Learner acquisition

**Speculative learner acquisition:** a zero-weight learner acquires state by
streaming while never voting, so that the 0→1 promotion finds the node already
caught up. The mechanism is the ordinary state transfer, gated by the learner
acquisition rule:

- **Serving:** the leader serves a `GetState` from a member of its current
  committed configuration, any weight, a weight-0 learner included, even
  when the sender is not a member of the configuration of the era it names:
  a learner behind the frontier can only name the eras its own table holds,
  and the era that admitted it is by definition not one of them. Serving is
  read-only retransmission; a node outside the current configuration (a
  foreign identity, a superseded old identity) is refused as before.
- **Acquisition:** a node still at its boot fence (`Restarting` or
  `Joining` at `current == retained`, the fenced entry state) that opened the fetch itself
  takes the answering chunk's committed frontier and folds the system
  operations it covers, the fold input is the chunk the suffix ruling
  already verified against the local journal. The node stays fenced: it
  adopts no view, its votes are never counted, and it serves nothing.
  The admitting era folds exactly there, which makes the leader's
  `StartView` evaluable and the ordinary install completes the catch-up.
- **Era-by-era catch-up:** a boot-fenced member admitted several eras
  past its boot table catches up era by era, one fold per stalled-ruling
  re-run: an offer more than one era past is retained when it NAMES the
  node (the era's establishing operation, a `Join`, the `Increment`
  that promotes it, or a batch carrying either), the acquisition's fold
  is capped at one era past the view it carries (the §8.7.3 era window),
  each ordinary tick re-runs the retained ruling, the walked view carries
  the next round's fetch, and the offer installs, the ordinary install,
  once its era is evaluable. The member never votes in an era it has not
  folded, and the walked view never adopts: the node stays at its boot fence
  (`Restarting`) until the retained offer installs.
- **Authority:** unchanged, a learner votes only after a committed
  `INCREMENT` grants it weight; while its weight is 0 its messages are
  discarded by the standard membership checks (§6).

A `ViewChange`-fenced node is not covered: its attempt's completing ruling
owns the commit frontier.

### Grounding sources

- <https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/>, voting
  weights; halve/double and ±1 unit rules; standbys at weight 0.
- <https://simbo1905.wordpress.com/2016/12/16/upaxos-unbounded-paxos-reconfigurations/>
 , era-indexed reconfiguration, consecutive-configuration overlap, casting vote.
- <https://simbo1905.wordpress.com/2020/05/23/one-more-frown-please-upaxos-quorum-overlaps/>
 , the frown operator and the exact quorum-overlap chain
  `QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1`.
- <https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/>
 , uVRR motivation; TigerBeetle-style durability framing.
- <https://simbo1905.wordpress.com/2024/04/12/the-network-is-faster-than-the-disk/>
 , the deferred-flush economic rationale (§3).

## uVRR rejoin gossip and witnesses

### 1. Model statement

Rejoining is a **gossip protocol outside the main uVRR protocol**. A node that
is not part of the cluster never assumes the cluster will come to it: it
gossips to find the cluster, and the cluster streams it until it is promoted.
The gossip layer exists so that the reincarnated node arrives at its first
fence at the cluster's current era, having seen a commit, which makes the
engine's one-era catch-up window (the reincarnation chapter §10) unreachable by
construction. The main protocol is unchanged: the gossip layer owns era
discovery and witness streaming; the engine still owns fencing, voting, and
the superblock lifecycle.

### 2. The join gossip (joiner side)

- A node outside the cluster, crash-restarted, reincarnated, or cold,
  gossips "I want to join" to **every node it knows about**, including its
  **frontiers**. The gossip runs under the deployment's authenticated
  transport (paxe with TLS 1.3 PSK), so every identity a sysadmin configured
  is known; the protocol does not depend on that beyond ordinary
  authentication.
- Any node that hears a join gossip responds with what it knows: the current
  era and the cluster configuration.
- The rejoining node adopts the max era it hears, takes the configuration
  from a responder, and **flushes the configuration to disk as JSON whenever
  it increments the era**. After a crash it comes back blank at era 0 and
  re-adopts from the responses, it never comes back at a remembered era.
- The rejoining node **never assumes it has heard the latest era**. It may
  straddle a partial network partition and hear an isolated node's newer
  configuration alongside a stale era. It therefore keeps its own resend
  timer and always sends all gossip messages to all nodes it has learned are
  in the cluster.
- When the gossip delivers a piggybacked commit, the node has seen leader
  activity and starts its phi leader-timeout baseline from it.

### 3. The gossip-witness list (all nodes, leader streams)

- **Every** node that hears a join gossip adds the sender to its
  **gossip-witness list**. Only the leader acts on the list: it replies
  and, based on the sender's frontiers, pushes all the phase-2 messages
  the sender needs to catch up, then a commit message.
- From then on the leader pushes **all phase-2s and all commits** to the
  gossip-witness list, as if those nodes were part of the cluster. The
  witness is a passive data sink outside the roster: it never votes, and its
  votes are refused by the standard membership checks
  (the reincarnation chapter §6).
- A joining node stays on the leader's witness list **until its membership is
  committed**: the drop on a committed JOIN (below) fires at the commit and
  never earlier, so a node is never served under two roles at once. During a
  join the leader owes the host the at-least-once obligation: every slot's
  traffic reaches each target at least once, and the last once may arrive
  more than once. The sender may deduplicate on the triple
  (slot, target, type). The view needs no ledger of its own: a leader that
  loses leadership crashes and returns under a new identity, so every resend
  ledger lives and dies inside one identity's leadership.
- Because the witness holds a live commit stream, it stays current
  **indefinitely**. Two worked scenarios show what this buys across a heal
  (§3.1).
- When a reconfiguration carrying a JOIN commits, **every** node scans its
  gossip-witness list and drops the promoted node. Sending to the node as
  both voter and witness is safe, duplicate delivery is idempotent, but
  wastes IO; dropping is a dedup optimisation, not a safety rule.
- Because every node keeps the list, **failover does not interrupt the
  stream**: the successor leader already carries the joiner in its own
  gossip-witness list and starts streaming on election. The joiner's
  resend timer covers a gossip lost in flight, it is the reliability
  mechanism of the gossip itself, not a failover re-discovery path.
- In this manner **any node outside a cluster can gossip to find the leader
  during failovers**: tracking the cluster through the witness stream
  replaces a lucky first message.

#### 3.1 Worked scenarios

**Three nodes: stream through the heal.** Take `n1, n2(l), n3` with `n1`
isolated and `n3` crashed. `n3` gossips its desire to join to `n2`, which
streams phase-2 messages to it, yet nothing commits, because no majority
responds. The network heals: the phase-2 messages hit `n1`, which acks and
requests retransmission of the messages it did not get. `n2` gains a majority
response for a high slot but still has a gap, so it cannot commit; then `n1`
catches up and responds to all slots, and `n2` commits in slot order and
sends the commit to both nodes. Because `n3` gossiped that it wanted to join,
`n2` computed the JOIN cluster commands as the next values to be chosen,
those were streamed too, so upon contiguous commits in slot order `n3` has
joined the cluster and leaves the leader's witness list.

**Five nodes: the isolated leader loses the quorum.** Take a five-node
cluster where the leader sits in a minority partition together with the
reincarnated node: the leader plays keep-up, streaming to the witness as
usual. The leader crashes. The network heals; the three surviving nodes form
a quorum and a new leader emerges. What happens to the reincarnated node?
Every surviving node heard its original gossip, so the new leader already
carries it in its gossip-witness list: on election it streams the messages
the surviving majority had committed. These may override the old leader's
uncommitted slots, and they add in and commit the new join attempt at
slots chosen by the new leader, the witness arrives at the new leader
already at the committed frontier and the join completes under the
configuration the quorum chose. The joiner's resend timer keeps gossiping
regardless: a node that missed the original gossip learns of the joiner
from a resend.

### 4. Witnesses as a first-class configuration role

- The startup cluster configuration carries a **witness list** in addition
  to the member list: regular members **plus witnesses**, where a witness
  cannot also be a member. Startup witnesses are the pre-provisioned form,
  e.g. out-of-region cold-backup nodes. Every node loads the list at
  startup and a statically registered witness is **never purged**: as the
  cluster fails over, each leader in turn streams to the full DR backup.
- The join method takes a **join type: `standard | witness`**.
- On a committed JOIN of a `standard` joiner, every node drops it from the
  gossip-witness list (§3): it is now a voting member and receives protocol
  traffic as one.
- On a committed **witness** join the list is **not** cleared. The purpose
  of the commit broadcast is so that **every node adds the witness to its own
  list**, so when failover to a new leader happens, the new leader forwards
  to the witnesses.
- A witness is therefore a durable role in the configuration, replicated by
  the ordinary commit path, not a leader-local memo.

### 5. Relationship to learner acquisition

The reincarnation chapter §10 specifies weight-0 learner acquisition inside the
engine: a learner streams while never voting, and the 0→1 promotion finds it
caught up. The gossip layer is the outer complement to that mechanism:

- The **"find the cluster" guard** wraps the engine on the host side. In
  blank mode it accepts any inbound era greater than its known era, sets its
  own era, takes the cluster membership, and flushes it to disk if it
  differs from what it last sent. The engine beneath the guard still runs
  the fenced acquisition path of §10.
- The **witness stream** is the leader-side send-set extension: one stream,
  all phase-2s and commits, serves weight-0 members mid-acquisition,
  persistent witnesses, and a future DR service alike. A DR sink in another
  region is just a witness whose apply log is an AOF tape. The catchup
  traffic is gossip-like, request and response pairs outside the voting
  protocol, and it inherits the at-least-once and dedup obligations of §3.

## Weighted reconfiguration solver

`uvrr::solver::solve(current, target, available)` returns committed-era batches
and their resulting configurations. It preserves a strict available majority
at every boundary and intersects consecutive strict-majority quorum families.
It uses only the existing operation alphabet and adds no dependencies.
The target's exact membership order and weights are honoured; its era is ignored.

For identical ordered identities, the solver decreases unavailable weights,
increases available weights, decreases available weights, then increases
unavailable weights. This takes the minimum number of unit edits. Exact global
doubling or halving takes one batch. For different identities or order, the
longest common prefix the two orders share, member and weight alike, stays
put: the current's tail drains and leaves, and the target's tail joins and
takes its weights, each live joiner promoted at its join, the dead joiners'
weights restored last. A prefix whose own mass cannot hold a majority falls
back to the available one-voter intermediate, which reserves one live voter
and transfers the vote to a live target member when necessary, complete
within the membership cap but reducing tolerance of additional failures.
The target's exact membership order and weights are honoured; its era is
ignored. Every step carries a `Nominate` rider that bumps the view into
the era the step establishes, keeping the leader constant through the plan
(the NOMINATE chapter); the serving view is an argument, a
snapshot supplied by the operator like the availability.

`solve_replacement(current, old, new, available)` retains the old member's weight
and position under a fresh identity. It prefers the full standard schedule when
its intermediate configurations remain available and its endpoint matches:
two batches for three unit voters, six including doubling and halving for five.
Otherwise it uses the general constructor. Replan a partially committed request
with `solve` from the current configuration to the original target.

Run the operator tool from the repository:

```sh
cargo run --features sysadmin_tool --bin uvrr-reconfig -- plan --current members.jsonl --replace 2:3 --available 0,1,3
cargo run --features sysadmin_tool --bin uvrr-reconfig -- apply --plan plan.jsonl --leader 10.0.0.1:9000
```

Acquire state before promoting a learner.
Select a live, positive-weight leader using a quorum-backed view change when
needed; `next_view_selecting` can select a later view directly without polling
through each skipped view. A returned plan does not itself send or acknowledge
messages, commit configurations, or establish leadership. Availability is a
snapshot supplied by the operator: re-evaluate it when failures change.

### Reconfiguration plans

A plan is the dumb-operator artefact: compute it once, submit it, and the leader
steps through it while the cluster keeps running normally. `uvrr::plan::Plan`
is the core's serde-free form, the initial membership (order is succession)
and one batch of operations per era, in commit order.

The plan travels as JSONL, one JSON object per line: a header line, then one
step line per era.

```
{"kind":"plan","version":1,"initial":[{"id":0,"weight":1},{"id":1,"weight":1},{"id":2,"weight":1}],"target":[{"id":0,"weight":1},{"id":1,"weight":1},{"id":3,"weight":1}]}
{"kind":"step","ops":[{"op":"decrement","node":2},{"op":"join","node":3,"position":2}]}
{"kind":"step","ops":[{"op":"increment","node":3},{"op":"leave","node":2}]}
```

`target` is the membership the steps reach. The operation vocabulary is exactly
the existing `SystemOperation` alphabet, `increment`, `decrement`, `double`,
`halve`, `join` (with `position`), `leave`, each naming `node` where the
operation has one. JSON is parsed once at the tool perimeter into the
serde-free `Plan`; the core never sees JSON. The codec refuses, on the way in,
an initial membership that is not a legal configuration, any step the fold
refuses, and a declared `target` that is not the configuration the steps reach.

The leader acceptance rule: the leader rejects any plan whose `initial`
configuration is not its current committed configuration, membership,
succession order and weights are all compared, and rejects any step the
configuration fold refuses (`uvrr::plan::Plan::validate_against`). A plan that
was legal when computed but has drifted is rejected, not committed; replan
from the current configuration to the original target.

An accepted plan is executed by the leader's plan-execution machine: one step
per era, each proposed through the ordinary reconfiguration gates, until the
last step commits and the machine clears. A step the gates refuse, the
cluster changed underneath the plan, aborts the machine with `PlanAborted`
and the operator re-plans from the configuration that committed. The plan
arrives on the leader's dedicated admin ingress, and the host polls that
ingress BEFORE the regular client queue on every selection
(`docs/architecture.md` §Host obligations): reconfigurations are rare, so the
poll is usually empty, but a plan never waits behind client traffic. The core
side is the `Input::SubmitPlan` input and the `Effect::AdminResponse` verdict
effect.

`plan` computes the schedule with the solver and writes plan JSONL to stdout
or `--out`. `apply` sends the plan as ONE UDP datagram to the leader's admin
port, prefixed by the header line `{"kind":"plan_submit","version":1}`; a
serialised submission larger than 60 000 bytes is refused locally. The
verdict is one JSON line, `{"kind":"plan_response","verdict":"accepted"}` or
`{"kind":"plan_response","verdict":"rejected","reason":"..."}`; `apply` waits
ten seconds for it, prints it, and exits 0 on `accepted`, 1 on `rejected`.
Availability is a snapshot supplied by the operator: re-evaluate it when
failures change.

For `(1,2,1,2) -> (2,1,2,1)`, total mass is unchanged but old quorum `{B,D}` and
new quorum `{A,C}` are disjoint. The solver increments A and C before decreasing
B and D, producing four safe boundaries. Failure tolerance is separate: weights
`(1,1,2,2)` split across two datacentres tolerate loss of the lighter datacentre,
but not the heavier one. The solver's guarantee is for `WeightedMajority`;
other policies must pass the core's role-specific gates independently.

Appendix 2 of `formal/uvrr-lean/paper/paper.tex` gives the proofs and test contract.
`research/weighted-reachability/` contains the independent Python enumeration,
charts and the phantom-identity casting-vote construction. The latter constructs
an abstract quorum witness without treating absent identities as received
promises. The Rust solver computes configuration paths; it does not currently
schedule these optional phantom witnesses or optimise datacentre resilience.

## NOMINATE leader assignment across reconfiguration

The leader of a view is `voters[view % voters]` under the view's era
configuration (`Configuration::primary`, §1.2, §8.4): the primary is a
ballot outcome, not a function of membership. A reconfiguration that
changes the positive-weight sequence moves the arithmetic mid-plan, so no
node can independently compute a stable leader through the transition.
This document states the NOMINATE letter that settles the assignment: a
view increment carried as a compare-and-swap at a committed slot, so the
leader the old arithmetic elected is the leader the new arithmetic
re-elects, at the very slot the arithmetic moves.

The solver chapter states the plans; this chapter
states the nomination that rides them. The safety licence for the jump is
the view-jump lemma, `formal/uvrr-lean/UVRR/ViewJump.lean`: a view may
jump by more than one during a reconfiguration and agreement is
unaffected, because no hypothesis of the agreement argument quantifies
over intermediate ballots; eras may not jump, the rule's adjacency
discipline; the only risk of a jump is exhaustion of the view space,
bounded code-side by the CAS delta.

### The case semantics at a moving slot

`SystemOperation::Nominate { from: View, offset: u32 }` is a letter of
the §8.7.2 alphabet. `from` names the view number the proposing leader
believes the cluster holds; `offset` names the increment `u`, strictly
positive. The command rides the committed log like every system
operation: a slot is committed, it happens everywhere, and every node
applies the same command at the same slot.

The CAS pair is the serving state and the view number. A node applying a
nomination at commit publishes the bumped view
`Ballot { era: the era the covering fold established, view: from + offset }`
as its current view, on the same published transition that carries the
folded table, if and only if it is `Normal`
and its published view number equals `from`. The bump advances the serving
view by a committed command and re-selects no history: the retained view,
the retained history's provenance, stays what it was, and a `Normal` node
serves at or past its retained view. The bumped view's era is
the era the nomination's own establishing run established, so the bump
enters the new arithmetic directly: the leader under the new
configuration is `record(era).primary(from + offset)`, the same node the
old arithmetic elected. The era component is never a claim the message
carries: it is the fold's own answer, and the §8.7.3 accept-time
relation already bounds it, an entry two eras past a node's view era is
refused at accept, so no committed nomination can name a two-era jump.

The nomination issues no `StartViewChange`, no fence, no evidence, no
install: the bump is not a view change, it is a commit. At the wrap the
bump IS the era-boundary crossing: the view's era must equal the
established era before the next establishing operation is proposed
(§8.7.8's gate, `PlanRefusal::EraTransitionOutstanding`), and the bump
enters it on the same published transition that carries the folded table,
so the wrap's boundary needs no view-change message at all. A node
not serving the named view, fenced, electing, replaying, or simply past
`from`, is outside the nomination's authority and does not bump; its own
path, the fence, the install, or the acquisition, sets its view. Install
transitions supersede the nomination outright: a `StartView` or NewState
install that covers a nomination carries its own view authority, and a
node whose election carried an old leader's nomination commits it as a
no-op, the cluster has moved on. Several nominations inside one covered
advance apply in slot order, each CAS predicated on the running number
the prior bumps of the same advance produced.

### The fold's seats

The nomination moves no mass and no membership. Folded in era it yields
the identical configuration, so inside a `SystemOperation::Batch` it is
a zero-mass rider under R14, and the seat it takes is the last
sub-operation of every establishing batch that carries no scaling
operation: the rider shares the step's own slot, so the bump lands
exactly when the step's era is established. A solitary `Nominate` entry
is refused by the fold with
`ConfigError::SolitaryNomination`, a nomination never establishes an era
of its own. A zero offset is refused with
`ConfigError::ZeroNominationOffset`, the jump strictly increases
(ViewJump's rule). A batch of one nomination is a legal zero-mass batch,
a rotation instrument the solver does not use.

R13's solitude reserves the scaling eras: `Double` and `Halve` preserve
the positive-weight sequence elementwise, a positive weight stays
positive and a zero stays zero, and they touch no order, so the wrap
never coincides with a scaling operation and the rider never needs a
scaling seat. `Join` and `Leave` at weight zero insert and remove
learners only, the voter subsequence is unchanged. The wrap is exactly a
sub-operation that moves a member across weight zero: an `Increment`
from 0, or a `Decrement` to 0.

### The solver's emission

`solve` and `solve_replacement` take the serving view, the view number
the leader holds when the plan begins, and name it in every emitted
nomination. The emission pass runs over the computed steps: walking with
the running view `v`, the view the parameter names plus the advances of
the steps already walked, and the previous step's configuration, every
step whose batch carries no scaling operation gains a
`Nominate { from: v, offset: u }` as its last sub-operation, `u` the
least positive offset with `primary(next, v + u) == primary(previous, v)`.
The rider's bump is the era entry the §8.7.8 gate demands: the
re-electing offset across a wrap that would move the leader, and the
count, the least offset that preserves the index, at a step that keeps
it, which is the same view the machinery's own §13.4 selector names.

A scaling step carries no rider: R13 keeps `Double` and `Halve`
solitary, and the scaling preserves the positive-weight sequence
elementwise, so the leader never moves at one. That step's era boundary
is crossed by the leader-preserving view change the host drives (§14.2),
the least view past the running one selecting the constant leader, the
same number `u` the rider would have named, so the chained views agree
whichever carries the boundary. The running view advances by `u` at
every step; the chained `from` values are the views the cluster actually
holds, each rider's bump landing at its step's commit. A step that
evicts the serving leader from the voter sequence cannot keep it: no
offset re-elects the evicted, the rider names the era-entering increment
alone, and the leadership passes to the arithmetic's choice; the next
step's emission continues from the leader the new arithmetic names.

The forced-reincarnation machine's own runtime recomputation
(`replica::forced_steps`) emits no nominations: its §6 schedule is
crash-restart idempotent and its leadership is the fence machinery's
business. `solve_replacement` prefers that schedule and the emission pass
wraps it, so the solver's plans carry the nominations and the machine's
ticks do not. The operator tool's `plan` takes the serving view as it
takes the availability snapshot.

The routes the emission wraps are the common-prefix routes: the longest
prefix the two orders share, member and weight alike, stays put; the
current's tail drains and leaves; and the target's tail joins and takes
its weights, each live joiner promoted at its join (the expansion's
shape: join at weight zero, increment, join, increment), the dead
joiners' weights restored last. The prefix-only intermediate must itself
be a legal, available configuration; a prefix whose mass cannot hold a
majority falls back to the anchor route, which reserves one live voter
and is always available. The prefix retention keeps the voter count at
two or more through every prefix the ladder drives, so the establishing
rounds always have a voter to acknowledge them: the anchor route's
drained intermediates, one voter and then one member, are the standing
question's and the host's business, not the plan's.

The standing question (§4, R4): a weight-zero member's acknowledgement
counts against no quorum and is not recorded, but its arrival still asks
the leader whether its own implicit vote and the recorded
acknowledgements already commit anything outstanding. A drained
intermediate's sole voter is its own commit quorum, so the zero-mass
steps at one-voter eras commit without another voter's vote; at any era
where the recorded votes hold no quorum the question changes nothing.

### The invariants

* **The constant leader through the wrap.** At every committed slot, every
  live member whose own committed configuration gives it positive weight
  computes the same leader: the configuration its committed history has
  established, evaluated at its current view number, names the node the
  plan started under, at every step of every scenario.
* **Era adjacency.** The bump's era is the establishing run's era, exactly
  one past the receiver's pre-fold row, and the accept-time §8.7.3
  relation enforces the bound before the commit ever sees the entry.
* **Weight-zero joins cannot vote.** The nomination moves no weights, and
  the §8.4 learner stays what it was: invisible to every majority and
  never the primary.

### The failure modes ruled out

* **Leader bounce.** The rider shares the wrap's establishing slot: there
  is no era between the old arithmetic and the bump in which the modulo
  rule would name the wrong leader, and no era after it in which the bump
  has not yet landed.
* **Era ambiguity.** The CAS predicate is the view number the sender
  believes the cluster holds, never a configuration-era claim; the bump's
  era component is the fold's own answer. No absolute era is committed at
  a slot a prior leader chose: the nomination's authority is the CAS, and
  a cluster that has moved on makes the commit a no-op.
* **Crash mid-transition.** A new leader committing an old leader's
  nomination finds the view moved, the CAS fails, and nothing bumps. An
  isolated node that bumped on its own is converged by later messages,
  the ordinary exchange; an overflowing `from + offset` is inert, the view
  space's exhaustion is the nomination's own dead end, never the node's.

### The force-feed test ladder

`tests/nominate_leader_assignment.rs` is the proof carrier. The driver,
per scenario: provision and bootstrap; force the cluster to the scenario's
serving view with the host-forced view change (§14.2), whose primary is
the scenario's constant leader; then for each step of the solver's plan,
the leader proposes the step as ONE establishing operation,
`SystemOperation::Batch(step.ops)`, through the ordinary §8.7.4 pipeline,
one entry at one slot, never a `Fuse` envelope, and the script feeds every
released message to its addressee until the network is empty; after each
step's quiesce the ladder asserts the constant leader at every live voter
(the node's own committed configuration, evaluated at its current view
number) and the cluster safety checker runs; and when the view lags the
established era after a step whose batch carried a scaling operation, the
script drives the era-boundary view change (§14.2) to the
leader-preserving view, the least view past the current one selecting the
constant leader under the era's voters, the same number the rider the
step would have carried names, and the plan continues.

The scenarios, in order:

1. **Expansion 3 to 5** at serving view 3, leader `n(0)`: `solve` from the
   genesis `(1, 1, 1)` to `(1, 1, 1, 1, 1)`. The common-prefix route keeps
   the genesis membership as the target's prefix and appends: join at
   weight zero, increment, second join, second increment, the promotions
   the wraps that carry the re-electing riders; four steps, four riders,
   the plan ends at view 15 with the unit-weight five.
2. **The three-node replacement** at serving view 3, leader `n(0)`:
   `solve` from `(n0, n1, n2)` to `(n0, n1, n3)`. The common-prefix
   route keeps the shared prefix, drains and leaves the old tail, joins
   and promotes the fresh one, the drain and the promotion the moving
   wraps; four steps, four riders, the plan ends at view 9 with the old
   identity evicted and the fresh one seated at unit weight.
3. **The five-node crash-reincarnation replace** at serving view 5,
   leader `n(0)`: crash `n(4)`, reopen its bumped life, and
   `solve_replacement` over the six-era forced schedule, `Double`, the
   join and promotion of the bumped identity, the drain of the old, its
   departure, the promotion, and `Halve`. The four unit-mass steps carry
   their riders, the moving wraps (the bumped identity's promotion,
   voters 5 to 6, and the old identity's drain to zero, voters 6 to 5)
   re-electing; the scaling eras' boundaries are the leader-preserving
   view changes; the plan ends at view 30 with the unit-weight five, the
   bumped identity in the old seat.

The Red rung is the ladder run against the un-nominated solver, and it
fails twice over: the assertion fails at the first wrap's committed slot,
the modulo rule names a different node at the unchanged view number (the
expansion and the replacement both name the drained member's successor at
view 3), and the plan cannot even cross its era boundaries, the §8.7.8
gate refuses the second establishing operation while the view lags the
established era (`EraTransitionOutstanding`, the face the
crash-reincarnation scenario reaches first, at its second step). The
emission pass and the commit-time bump make the same ladder green, the
constant leader at every committed slot of all three scenarios. The
bumped life's own seating is the §10 acquisition, the rejoin path's
business, and is not the ladder's assertion: the scenario asserts the
plan's commits and the leader's constancy at the serving voters.
