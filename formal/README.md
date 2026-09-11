# vrr-core TLA+ model

`VrrCore.tla` is an executable safety model of the protocol-visible state and
transitions implemented by this crate. It is deliberately a model of this
codebase's contract, not a copy of a paper model.

The correspondence is:

| TLA+ | Rust |
|---|---|
| `status`, `currentView`, `retainedView`, `committed`, `applied` | `progress::Progress` |
| `logs` and `Len(logs[r])` | `Journal` and `Progress::accepted()` |
| `Propose`, `ReceivePrepare`, `CommitNext`, `ReceiveCommit` | `replica/normal.rs` |
| `EnterViewChange` through `ReceiveStartView` | `replica/view_change.rs` |
| `Crash` | `Replica::reopen` — the boot rule fences every reopen |
| `ApplyNext` | `Input::Applied` |
| `messages` | published `Effect::Send` values plus adversarial transport |
| `historicalCommitted` | none — a verification ghost recording every committed (slot, entry) fact |

The model makes loss, reordering, delay, and duplication implicit: messages
remain in a set forever, and a receive action is never required to occur. A
message can therefore be ignored, received later than newer messages, or
received repeatedly.

The model abstracts bounded suffix transfer to an atomic full-history install.
This is sound for the checked safety properties because transfer can only make
the already-selected history constructible; it cannot select a different
history. The `plan`/`publish`/confirmation handshake is likewise one atomic
TLA+ step: the Rust publication gate is responsible for refining that step so
no effect becomes visible before its supporting state. Reconfiguration is not
part of `VrrCore.tla`; it has a separate design model below because era-qualified
views change the type and meaning of almost every protocol variable.

The model does not claim liveness, correctness of suffix chunk assembly,
durability-barrier ordering, or exactly-once emission of an `Effect::Apply`
before the host acknowledges it with `Input::Applied`. Those are separate
implementation contracts exercised by the Rust suites; `applied` here records
only completed application inputs.

Two finite models are checked:

- `VrrCore.cfg`: normal operation and view change with three replicas, two
  commands, and views 0 and 1.

Run both with the repository's no-volume Docker path:

```sh
make tla
```

This uses ordinary `docker build` and `docker run`; it does not require
BuildKit or a bind mount. The default is explicitly `linux/arm64`, the built
image architecture is checked before either model runs, and TLC uses two
workers to limit sustained thermal load. These can be overridden with
`TLA_PLATFORM`, `TLA_ARCH`, and `TLA_WORKERS`. For a local Java 11+
installation and a downloaded `tla2tools.jar`:

```sh
make tla-local TLA2TOOLS_JAR=/absolute/path/to/tla2tools.jar
```

The principal checked theorem is prefix agreement: any two replicas agree at
every slot both consider committed. The other invariants enforce the frontier
order, application-prefix agreement, the current/retained view distinction,
and the two historical obligations over the ghost set: no
committed slot is ever repopulated with a different entry, and every entry
once committed remains present in at least one current replica history.

## Era-transition design model

`VrrCoreEras.tla` verifies the intended one-transition uVRR design. It is not a
description of unfinished Rust. The implementation is expected to converge on
the proved transition system; disagreement is a design or implementation
finding, not a reason to weaken the model.

The model uses explicit `[era |-> e, idx |-> v]` view identifiers with
lexicographic order. Configuration is selected by the `Scenario` constant, and
the startup gate exhaustively enumerates subset pairs for the following
obligations, each as its own named `Assert` in `Init`, so a refused scenario
reports the exact obligation it violated:

- same-era view/commit intersection in both eras;
- both directions of cross-era view/commit intersection;
- view-family self-intersection in both eras;
- fence intersection in both eras; and
- both directions of cross-era fence intersection (a quorum gathered in one
  era must meet the other era's fence family).

The action correspondence is deliberately to the design statement:

| Action | Status | Design obligation |
|---|---|---|
| `ProposeCommand`, `ReceivePrepare`, `CommitNext`, `ReceiveCommit` | base, era-generalized | normal replication; slot routing by `EraOfSlot` |
| `EnterViewChange` through `ReceiveStartView` | base, era-generalized | view fencing, retained-history provenance, establishing-entry certification |
| `Crash` | base, era-generalized | the crash event fences the identity and forgets the volatile protocol state; the fenced replica sends nothing (uVRR's Crash-Stop-Self-Evict reincarnation replaces the classic recovery exchange) |
| `ProposeReconfig` | new | one establishing operation and one consecutive era boundary |
| `SendPlannedViewChange`, `AnswerPlannedViewChange` | new | planned evidence that does not fence its sender |
| `CastPlannedVote` | new | the leader remains `Normal`, installs its own retained log, and changes `currentView` and `retainedView` atomically |

The abstract `PlannedOk` message is a projection of planned view-change
evidence: it retains provenance, history, and frontier fields, but the casting
transition consumes only voter identity. The planned/ordinary distinction is
carried by the message type itself; the M4 mutation deliberately erases that
distinction. The normative cast (`CastPlannedVote`) installs the leader's own
log and quantifies over no message evidence, so it does not generate one
identical successor per irrelevant `PlannedOk`; the unserialized-cast defect
has its own action (`CastPlannedVoteFromEvidence`) quantified over a chosen
planned report.

### Abstraction boundary

The eras model makes seven state-space cuts:

1. Exactly one consecutive transition, from era 0 to era 1. All checked quorum
   obligations are pairwise over that boundary.
2. One fixed membership and succession order. Only weights differ by era; zero-
   weight membership changes and order changes remain future work.
3. Quorum families are scenario constants checked at startup. Dynamic folding
   and closure over the operation alphabet belong to the exhaustive gate.
4. Full histories replace chunked suffix transfer after history selection.
5. The planned request is broadcast. This over-approximates routing to the
   selected old-era view quorum by permitting more replies, never fewer.
6. Application and replay state are omitted because they feed no quorum. A
   recovered node becomes eligible at the earliest safe transition, which is
   the adversarial direction for quorum safety.
7. Messages are never removed. Loss, delay, duplication, and reordering are
   choices to ignore or repeatedly consume an old message.
8. A crash always fences and forgets the volatile protocol state. Whether the
   durable records (journal and committed frontier) also survive is fixed per
   configuration by the `CrashRetainsDurable` constant rather than explored
   nondeterministically. uVRR replaces the classic quorum-recovery exchange
   with Crash-Stop-Self-Evict reincarnation (a dirty node reopens under a new
   incarnation and rejoins as a weight-0 standby — a **standby** is
   TigerBeetle's term for its non-voting cluster members: standby nodes have a
   zero voting weight so cannot form part of any quorum nor actively
   participate in the VSR algorithm), so the model performs no
   recovery exchange; the fenced replica sends nothing, which the
   `RecoveringSendsNothing` property checks at the transition where a message
   is added. Quorum counting is over the recorded evidence itself: a message
   sent while the sender was entitled to send it remains valid under
   arbitrary delay, loss, reorder, and duplication, so the model does not
   recheck a responder's current status or view when acknowledgements,
   planned votes, or ordinary reports are counted.

The model also made one previously implicit proof obligation explicit. A
follower may join an era-1 fence without locally knowing the establishing entry
committed, but a primary may not install that view until the maximum committed
frontier in its selected ordinary reports covers the establishing slot. Merely
preserving the entry is insufficient to prove that every `Normal` era-1 replica
knows the configuration is established.

### Configurations and measured runs

| Configuration | Method | Result |
|---|---|---|
| `VrrCoreEras.cfg` | exhaustive, 3 nodes, no crash | green: 837,204 distinct / 3,358,487 generated, depth 32, 3m24s |
| `VrrCoreErasWitnessNonStop.cfg` | expected failure | `WNonStop` reached in 207 distinct states |
| `VrrCoreErasWitnessOverlap.cfg` | expected failure | `WOverlapStreams` reached in 125 distinct states |
| `VrrCoreErasCrash.cfg` | fixed simulation preflight | green: 10,000 depth-40 traces, 900,119 checked states, 45s |
| `VrrCoreErasEven.cfg` | exhaustive, 4 nodes, no crash | green: 290,474 distinct / 1,571,856 generated, depth 30, 1m51s |
| `VrrCoreErasDeep.cfg` | fixed simulation | green: 100,000 depth-60 traces, seed 1, 26,793,084 checked states, 1h56m |

The even-split scenario uses commit/view thresholds 2/3 in era 0 and 3/4 after
incrementing `n0`. The next-era view threshold excludes `{n0,n1}`: its weight 3
cannot form a view quorum disjoint from the legal old-era commit set `{n2,n3}`.
The startup gate checks both cross-era directions before E3 explores states.

The two witness invariants are intentionally false. Their counterexample traces
are existence proofs that the casting transition is reachable without any
stop-the-world view change and that an era-1 slot can commit while the primary
still occupies an era-0 view.

### Defect regression

`Defect` is `"none"` in every normative model. Each mutation configuration
enables exactly one weakened perimeter, one falsified evidence transfer, or
one illegal scenario, and must exit nonzero on the named check:

| Config | Mutation | Expected result |
|---|---|---|
| M1 | drop the transferred tail entry while reporting its committed frontier | `FrontiersOrdered` |
| M2 | adopt a higher view without installing its selected history | `CommittedLogsAgree` |
| M4 | interpret planned evidence as ordinary fence/report provenance | `CommittedLogsAgree` |
| M5 | let the cast install a responder history instead of the leader's retained log | `CommittedLogsAgree` |
| M7 | six-node increment with disjoint old-view/new-commit sets | startup gate `gate: r1-era1`, zero states |
| M8 | era-0 and era-1 view pivots disjoint across the fence families | startup gate `gate: fr-cross-fence0-recovery1`, zero states |

M1, M2, M4, and M5 are breadth-first. M7 and M8 are refused by the named
startup gate assertions before any state is explored. Removing only the pivot-existence
guard is kept as the separate `VrrCoreErasNoPivot.cfg` experiment: it is expected to remain green
because the pivot is a pipelining availability condition, not a safety axiom.
That experiment completed exhaustively with 837,204 distinct states, depth 32,
in 3m29s and found no safety violation.

All Docker runs remain volume-free and BuildKit-free. The image, daemon, kernel,
Debian package architecture, and JVM `os.arch` are checked as ARM64/aarch64; TLC
1.7.4's banner incorrectly prints an x86_64 JVM label even when the JVM itself
reports `aarch64`. Worker count remains capped at two for thermal control.

## Reincarnation design model

`VrrCoreReincarnation.tla` verifies the Crash-Stop-Self-Evict forced sequence in
isolation from the era-transition model, under leader replacement. Identities
crash-stop: an identity marked dirty is dead under that identity forever and
sends nothing afterwards. The victim crashes, is fenced (bumped to weight 0,
recorded in `bumped`, never to vote again), and the node reopens under the new
identity `d` at weight 0. Two committed eras follow the reincarnation
announcement: era 1 batches `DECREMENT(Victim)+JOIN(d)` and era 2 batches
`INCREMENT(d)+LEAVE(Victim)`, matching `src/replica/reincarnation.rs`
`forced_steps`.

Leadership is a variable. The initial leader is `a`; a view change hands the
role to a live positive-weight member that has not been fenced, and the leader
may itself die. Replacing a live leader is rationed by `MaxViews`; replacing a
dead leader is never rationed, because that is the view-change protocol's
liveness obligation, and it is bounded anyway since each such change consumes a
leader death and deaths are bounded by `MaxDead`. Acknowledgements are gathered
per leader and per era: an ack answers one announcement, is addressed to the
announcing leader, is stamped with the announced era, and is counted only by
that leader while that era is current. A successor leader therefore announces
and gathers its own acks; stale acks addressed to an earlier leader or era are
never counted, and a recorded ack stays valid regardless of the sender's later
status (the sender's marks are not rechecked at the counting site).

The quorum rule is the design rule: support is the current leader plus counted
acks from identities whose current weight is positive, and a commit requires a
live leader and a strict majority over the current weight table
(`2*SumWt(s) > SumWt(Members)`). Replies from weight-0 standbys are discarded at
ingress (`StandbyDiscard`) and never enter the monotone message set. Both commit
actions `Assert` the per-node mass bound (at most mass 1 per node per era); the
bound is asserted, not assumed. Each commit records its quorum in `quorums[e]`
and its committing leader in `leaders[e]`. The commit actions assign their
effects from current state rather than accumulate, so after a leader crash the
remaining era is recomputed idempotently by the successor through the same
guarded actions — the §8 leader-crash rule of `docs/uvrr-reincarnation.md`.

A stall is a legitimate terminal state, not a defect: when the live voters no
longer hold a strict majority, no commit is enabled and the sequence waits.
Deadlock checking is therefore off in every configuration (`CHECK_DEADLOCK
FALSE`), and completion is checked separately as the liveness property
`Completes == <>(era = 2)` under `FairSpec == Spec /\ WF_vars(Next)`. Because
every state-changing action increases a bounded monotone quantity (marks,
messages, views, eras), a fair behaviour reaches a terminal state, so the
property holds exactly when every terminal state has committed era 2.

| Element | Obligation |
|---|---|
| `Crash` | crash-stop: a dead identity stays dead and sends nothing; the victim's crash opens the scenario, further deaths are bounded by `MaxDead`; a leader death raises `crashed` |
| `ViewChange` | a live, positive-weight, unfenced member takes the leader role; rationed by `MaxViews` only while the leader is alive |
| `Bump`, `Announce`, `Ack`, `StandbyDiscard` | fencing, per-era reincarnation announcement by the current leader, per-leader per-era acks from live voters, standby ingress discard |
| `LeaderCommitEra1` | crossing batch `DECREMENT(Victim)+JOIN(d)`, live leader, strict-majority quorum of the leader's own acks for the current era, mass ≤ 1 asserted, leader recorded |
| `LeaderCommitEra2` | promotion batch `INCREMENT(d)+LEAVE(Victim)`, same discipline, idempotent recompute by whichever live leader holds the role |
| `FrownChain` | every committed quorum is a strict majority over current weights; consecutive committed eras overlap |
| `EvictedNeverVoter` | every bumped identity stays at weight 0 |
| `NoStandbyVote` | every quorum member other than the committing leader has on file an ack addressed to that leader for the era the commit was gathered in, and holds positive weight |
| `MassRule` | each committed era moved at most mass 1 per node (mass is the weight delta the era produces) |
| `RebornAfterFence` | the reborn identity may hold weight only once the victim identity has been fenced |
| `SequenceCompletes` | a committed forced sequence is never abandoned before its final era commits |
| `LeaderIsVoter` | a live leader has positive weight and is not fenced; a dead leader holds the role only until replaced and commits nothing |
| `Completes` | liveness under weak fairness: the sequence reaches era 2 while the live voters keep a strict majority |

Scenario constants: `Members` (original voting identities), `Victim` (the
member that crashes and is fenced), `StartWt` (initial weight of each original
member; the reborn identity always starts at 0), `CrashFirst` (when TRUE, era 1
may commit only before a leader crash and era 2 only after one, forcing the
crash-between-eras path onto a successor leader), `MaxViews` (bound on view
changes that replace a live leader), and `MaxDead` (bound on dead identities,
the victim included). The `Defect` constant weakens exactly one guard per
mutation config (`"none"` is the intact design): mass is the weight delta an era
produces on each node.

### Configurations and measured runs

| Configuration | Method | Result |
|---|---|---|
| `VrrCoreReincarnation3.cfg` | exhaustive, 3 nodes (a:1, b:1, c:1), fixed leader, no further deaths | green: 28 distinct / 221 generated, depth 14, 0.7s |
| `VrrCoreReincarnation2.cfg` | exhaustive, 2 nodes (a:1, b:1), b crashes and reincarnates as d | green: 35 distinct / 162 generated, depth 11, 0.8s |
| `VrrCoreReincarnationDouble.cfg` | exhaustive, 3 nodes at double scale (a:2, b:2, c:2) | green: 28 distinct / 221 generated, depth 14, 0.7s |
| `VrrCoreReincarnationFive.cfg` | exhaustive, 5 nodes (a:1, b:1, c:1, e:1, f:1), f crashes and reincarnates as d, fixed leader | green: 3,406 distinct / 53,078 generated, depth 20, 1.1s |
| `VrrCoreReincarnationCrash.cfg` | exhaustive, 5 nodes, leader crash forced between the two eras (`CrashFirst`), one view change, two deaths | green: 1,253,444 distinct / 26,308,632 generated, depth 31, 1m48s |
| `VrrCoreReincarnationFiveViews.cfg` | exhaustive, 5 nodes, `MaxViews = 1`, `MaxDead = 2`: one replacement of a live leader, unrationed replacement of a dead one, the victim plus one further death, all invariants | green: 17,394,566 distinct / 371,140,753 generated, depth 32, 27m34s |
| `VrrCoreReincarnationFiveCompletes.cfg` | liveness under `FairSpec`, `PROPERTY Completes`, `MaxViews = 0`, `MaxDead = 2`: the leader may die and is replaced, all invariants | green: 126,301 distinct / 2,318,131 generated, depth 26, 19.0s; `Completes` checked over the complete state graph |
| `VrrCoreReincarnationFiveStall.cfg` | expected failure: as FiveCompletes with `MaxDead = 3` | red by `Completes`: stall reached at 59,666 distinct / 426,526 generated, 3.8s; the trace ends stuttering at `era = 0` with two live voters holding weight 2 of 4 |
| `VrrCoreReincarnationM1.cfg` | mutation: era 1 skips the decrement, era 2 leaves the victim at weight 2 | red by `MassRule`: the leave moves 2 units, counterexample records `moved[2][c] = 2` |
| `VrrCoreReincarnationM2.cfg` | mutation: one-era swap, the unfenced victim evicted and d promoted in one era | red by `MassRule`: the swap evicts at full weight, counterexample records `moved[1][c] = 2` |
| `VrrCoreReincarnationFiveM1.cfg` | mutation at 5 nodes (a:1, b:1, c:1, e:1, f:1): era 1 skips the decrement, era 2 leaves the victim at weight 2 | red by `MassRule`: the leave moves 2 units, counterexample records `moved[2][f] = 2` |
| `VrrCoreReincarnationFiveM2.cfg` | mutation at 5 nodes: one-era swap, the unfenced victim evicted and d promoted in one era | red by `MassRule`: the swap evicts at full weight, counterexample records `moved[1][f] = 2` |
| `VrrCoreReincarnationM3.cfg` | mutation: the sequence runs without the fence (no bump) | red by `RebornAfterFence`: d holds weight 1 while `bumped = {}` |
| `VrrCoreReincarnationM4.cfg` | mutation: the sequence aborts after era 1, era 2 never proposed | red by `SequenceCompletes`: `aborted = TRUE` with `quorums[2] = {}` |
| `VrrCoreReincarnationM5.cfg` | mutation: weight-0 identities' replies counted in quorums | red by `NoStandbyVote`: the weight-0 identity d is counted in `quorums[1]` |

The 2-node run is the load-bearing case: after b is fenced, the total member
weight is 1, so the crossing batch commits under the leader's casting vote
alone (E1). The double-scale run checks the majority arithmetic at weight 2,
where the leader alone (weight 2 over total 4 after the fence) is no longer a
strict majority and the second member's ack is required. The fixed-leader runs
(`MaxViews = 0`, `MaxDead = 1`) admit only the victim's death, so the leader
never changes and every era is committed by `a`.

The crash-between-eras run needs five voters: at three, the victim's fence and
the leader's death leave one live voter of a remaining weight 2, which is no
strict majority, so era 2 cannot commit under crash-stop at that scale. At five
the leader `a` commits era 1, dies, the successor gathers its own era-1 acks
from the three remaining voters and commits era 2. `FiveViews` is the main
safety check: any interleaving of a live leader's replacement, a leader death
with its unrationed replacement, and the victim's fence during the two rounds
preserves every safety invariant, stale acks to the earlier leader or era
included. Its state graph is dominated by the monotone message set: each
leader tenure per era adds an announcement round and up to sixteen ack
subsets, so the product over tenures grows by more than an order of magnitude
per additional rationed view change; `MaxViews = 1` is the largest bound the
search completes at this scale. `FiveCompletes` checks completion with view
changes that replace only a dead leader: three live voters remain in every
terminal state, a strict majority over the post-fence weight 4, so the
sequence finishes despite the leader's death. `FiveStall` is the intentionally
failing witness: with `MaxDead = 3` two voters can be left holding weight 2 of
4, no strict majority exists, and TLC reports `Completes` violated with a trace
that ends stuttering in the stalled state. The witness is the existence proof
that the model stalls rather than committing without a majority.

The seven mutation runs use one worker (a multi-worker search stops at a
nondeterministic point after the first violation, so only the violated
invariant is compared). All runs used the hash-pinned TLC 2.19 jar
(`sha256 936a2620…`), 2g heap; green and liveness searches with 2 workers,
mutations with 1. Evidence logs and digests are in
`formal/uvrr-lean/evidence/tlc/`.

`InitialLeader = "a"` and `Reborn = "d"` are fixed definitions, so the fifth
voter is `f` (`a` starts in the leader role, `d` remains outside the voter set
and is never eligible for it) and the recursion/majority machinery generalizes
to any member count. The five-node config was also offered to the symbolic
checker Apalache, which rejected the model outright; that outcome is recorded
in `research/apalache-smoke.md`.
