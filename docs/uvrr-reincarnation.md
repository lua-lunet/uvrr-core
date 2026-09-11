# uVRR reincarnation: the Crash-Stop-Self-Evict protocol

## 1. Model statement

uVRR by definition does **not** allow Crash-Recover (CR). It enforces
**Crash-Stop-Self-Evict**, called **reincarnation**. A node whose volatile state was
lost is, by construction, a *different node*: it reopens under a new identity and its
old identity is evicted from the voting configuration. Same-identity recovery after
volatile-state loss is unrepresentable in the protocol. Classic VRR-2012 diskless
quorum recovery (its §4.3) is not implemented; there is no amnesia recovery protocol.
Classic VRR's published recovery failure (the DISC'17 Appendix B.1 amnesia class,
Michael et al.) is literature about the classic crash-recover class; uVRR eliminates
the class by construction rather than repairing it.

## 2. Durable identity contract: the four superblocks

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
to all four superblocks, then enter the wire phase (§4). The state machine is:

| State | Meaning |
|---|---|
| `flushed` | durable checkpoint of identity X; written at clean shutdown and after the bump |
| `unflushed` | running sentinel; written at start of operating |
| `dirty` | restart observed any-`unflushed`; eviction must begin |
| `bumped` | incarnation incremented; all four superblocks rewritten as (new identity, `flushed`) |
| `reincarnating` | wire phase: old identity pending eviction, new identity a weight-0 standby |

A **standby** is TigerBeetle's term for its non-voting cluster members (this document's
older drafts called it a learner). Standby nodes have a zero voting weight so cannot
form part of any quorum nor actively participate in the VSR algorithm.

**Read rule (higher-identity-wins):** any read of the superblocks may observe a higher
identity than the reader last knew; the reader adopts the higher identity.

**Continuation commitment:** once Crash-Stop-Eviction is initiated it MUST continue;
the forced sequence of §5 is never aborted mid-way.

## 3. Economic rationale: the network is faster than the disk

Reincarnation takes advantage of "The network is faster than the disk"
(<https://simbo1905.wordpress.com/2024/04/12/the-network-is-faster-than-the-disk/>).
For nano-state that fits in memory, disk flushes can be **deferred**: the forced
reconfiguration rounds of §5 are network round trips, which are cheaper than fsyncs.
The four-superblock mark is **one fsync amortized over the whole epoch**, not
per-operation durability. The dirty check is therefore paid once per restart, and the
correctness cost of same-identity amnesia is avoided entirely instead of being paid
per operation as a flush.

## 4. Wire message

A new **reincarnation** message carries the pair `(old identity, new identity)`, sent
by the bumped node to the leader. The leader drives the forced sequence of §5. The
identity pair (old, new) is the freshness carrier where it meets rung 17's fence
machinery: it **supersedes** the `generation` ghost field of
`UVRR/RecoveryFence.lean` as the environment freshness abstraction, instantiated by
the durable superblock incarnation. The RecoveryFence machinery is unaffected;
rung 17 requires no modification.

## 5. Forced weight sequence

The leader drives the forced reconfiguration sequence from the Paxos Voting Weights
rules (`docs/uvrr-reconfiguration-rules.md`: the operation alphabet `DOUBLE`, `HALVE`,
`INCREMENT`, `DECREMENT`, `JOIN`, `LEAVE`; each reconfiguration commits a legal batch,
one era). One reconfiguration commits a batch, and a batch either moves at most one
unit of per-node voting mass (standbys at weight 0 move none) or is one solitary
scaling op. Unit-weight special case, evicting `N2` and reincarnating it as `N2′` —
**two eras**, each a batch:

| Era | Batch | Weights (N0, N1, N2, N2′) | Mass moved |
|---|---|---|---|
| D1 | `DECREMENT(N2), JOIN(N2′)` | (1, 1, 0, 0) | 1 |
| D2 | `INCREMENT(N2′), LEAVE(N2)` | (1, 1, –, 1) | 1 |

The old identity leaves only after its weight is zero, and the new identity joins at
zero — the two together in the crossing era move exactly one unit, and the eviction
era moves exactly one. **Era-safety invariant:** every consecutive pair of
configurations in the sequence satisfies the per-node mass rule (R14 of the rules
doc), so consecutive majority families intersect and **every intermediate era is
quorum-safe on its own**: a leader dying mid-sequence leaves only safe membership
eras, and a new leader recomputes the remaining eras from the configuration that
committed.

**Doubled-weights corner.** If the reincarnation happens while the cluster sits in the
doubled state of the blog's safe-replacement plan, the sequence runs at the doubled
scale and the return to unit weights needs the extra unit step that makes the halve
integral:

| Era | Batch | Weights (N0, N1, N2, N2′) |
|---|---|---|
| C1 | `DECREMENT(N2)` | (2, 2, 1, –) |
| C2 | `DECREMENT(N2), JOIN(N2′)` | (2, 2, 0, 0) |
| C3 | `INCREMENT(N2′), LEAVE(N2)` | (2, 2, –, 1) |
| C4 | `INCREMENT(N2′)` | (2, 2, –, 2) — **corner**: joiner at 2 so the halve is integral |
| C5 | `HALVE` | (1, 1, –, 1) |

Every era above is a legal batch of the rules document, so every intermediate era is
quorum-safe by the same invariant. (Scaled steps preserve overlap by common-factor
normalization — the checked `WeightedGeneral.scaled_overlap` result covers each step.)

## 6. Membership-discard check

Messages **FROM** a node that is not in the current voting configuration — a weight-0
standby, or a superseded old identity — are **discarded** by the standard membership
check on ingress. Messages **TO** such a node are fine. Two exceptions, each the
message that makes or keeps a membership: the `Reincarnation` announcement is the
bumped node's entry ticket (§4), and the non-stop reconfiguration's solicited planned
evidence (§8.7.7) is the vote the construction solicited — the pivot places a
departing member inside `qI` precisely so its answer completes the planned quorum, so
the discard treating that one answer as hostile input would conflate the transition
with the departure. The exception is scoped to exactly that message: a `DoViewChange`
carrying `EvidenceKind::Planned` for the armed machine's transition view, from a
member the machine's pivot names in `qI`; every other guard of the planned-evidence
path re-fires at the counting site, and a non-member's ordinary traffic — and its
state-transfer requests — stay refused by name. This is good practice
independent of reincarnation; the crash-vector collector's
`old_reply_rejected` (rung 18) is the same primitive: once a later incarnation is
known for a node, replies from the old identity never regain eligibility, regardless
of redelivery.

## 7. Streaming order

The leader stays live for client requests throughout. It streams, in order:

1. the **first reconfiguration** (the forced sequence of §5),
2. then **client traffic**,
3. and **preemptively streams** to the reincarnated node **as a standby**
   (weight 0; messages TO it fine, FROM it discarded via §6).

## 8. Leader-crash ordering

If the leader crashes mid-sequence, the cluster must reach a **stable leader** before
the reincarnated node forces its old-identity eviction. Because every intermediate era
is quorum-safe (§5), whichever safe era the crash lands in is a legal starting point
for the new leader, which resumes the sequence. The reincarnated node does not force
eviction of the old identity until a stable leader exists to drive it.

## 9. Relationship to the checked ladder

- Rung 17 (`RecoveryFence`): the recovering replica cannot reply; fresh-episode
  evidence only; stale evidence rejected. This is precisely the ingress rule the
  weight-0 standby obeys and why the leader may safely stream to it. Its `generation`
  ghost is superseded as a carrier by the durable superblock incarnation (§4); the
  machinery stays.
- Rung 18 (`CrashVector`): `old_reply_rejected` is the membership-discard check for
  the old identity (§6).
- Rung 19 (`AcquisitionOrder`): the acquisition-order induction is what the
  reincarnated node's streamed-state acquisition must satisfy; its open retention
  premise becomes dischargeable — under reincarnation, retention of incarnation
  knowledge is the durable superblock identity itself.
- Rung 20 (`RecoveryAcquire`): the echoed `(incarnation, sequence)` request identity
  is the wire-level freshness contract the reincarnation message and standby streaming
  use; `crashed` bumping the incarnation is the formal shape of dirty ⇒ bump.
- Rungs 1–9 (eras, weights, weighted-general): the forced sequence is a path through
  the existing weighted-era space; each step's safety is the weighted-overlap
  argument.

The reincarnation formalization is ladder rung 22 (`formal/uvrr-lean/UVRR/Reincarnation.lean`):
the definitions and state machine above, with kernel-checked structural lemmas and finite
instances (the unit-scale forced sequence's two consecutive eras are quorum-safe; the
startup classification is exhaustive; the bumped identity is never a voter again; the
forced sequence is monotone and cannot abort). The four general theorems — bumped-identity
non-membership in every view ≥ eviction, quorum safety of every intermediate era at
arbitrary scale, unreachability of the classic amnesia trace, and continuation commitment
in general — are that rung's stated proof obligations, for later rungs.

## 10. Learner acquisition

**Speculative learner recovery:** a zero-weight learner acquires state by
streaming while never voting, so that the 0→1 promotion finds the node already
caught up. The mechanism is the ordinary state transfer, gated by the learner
acquisition rule:

- **Serving:** the leader serves a `GetState` from a member of its current
  committed configuration — any weight, a weight-0 learner included — even
  when the sender is not a member of the configuration of the era it names:
  a learner behind the frontier can only name the eras its own table holds,
  and the era that admitted it is by definition not one of them. Serving is
  read-only retransmission; a node outside the current configuration (a
  foreign identity, a superseded old identity) is refused as before.
- **Acquisition:** a node that has adopted nothing — no view installed
  since it opened — that opened the fetch itself takes the answering
  chunk's committed frontier and folds the system operations it covers —
  the fold input is the chunk the suffix ruling already verified against
  the local journal. Two states are this state: the boot fence
  (a fenced entry state — `Restarting` on reopen, `Joining` on provision — at
  `current == retained`), and the
  reincarnated not-yet-adopted state the forced walk leaves behind
  (`ViewChange` under the higher-view signal's fence, `retained` still
  the boot view whose configuration cannot yet name the node). The node
  stays fenced: it adopts no view, its votes are never counted, and a
  boot-fenced node serves nothing. The admitting era folds exactly
  there, which makes the leader's `StartView` evaluable and the ordinary
  install completes the catch-up.
- **Era-by-era catch-up:** a fenced member admitted several eras past
  its table catches up era by era, one fold per stalled-ruling re-run:
  an offer more than one era past is retained when it NAMES the node
  (the era's establishing operation — a `Join`, the `Increment` that
  promotes it, or a batch carrying either), the acquisition's fold is
  capped at one era past the view it carries (the §8.7.3 era window),
  each ordinary tick re-runs the retained ruling, the walked view
  carries the next round's fetch, and the offer installs — the ordinary
  install — once its era is evaluable. The member never votes in an era
  it has not folded, and the walked view never adopts. A reincarnated
  identity catches up the same way to the live view before it can lead
  it: the designations that succession hands it wait for the folds, and
  the designation completes when the live view's evidence arrives at a
  table that can evaluate it.
- **Authority:** unchanged — a learner votes only after a committed
  `INCREMENT` grants it weight; while its weight is 0 its messages are
  discarded by the standard membership checks (§6).

A `ViewChange`-fenced node whose retained view's configuration admits it
is not covered: its attempt's completing ruling owns the commit frontier.
Only the reincarnated not-yet-adopted state — the retained configuration
cannot yet name the node — folds under the fence.

## Grounding sources

- <https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/> — voting
  weights; halve/double and ±1 unit rules; standbys at weight 0.
- <https://simbo1905.wordpress.com/2016/12/16/upaxos-unbounded-paxos-reconfigurations/>
  — era-indexed reconfiguration, consecutive-configuration overlap, casting vote.
- <https://simbo1905.wordpress.com/2020/05/23/one-more-frown-please-upaxos-quorum-overlaps/>
  — the frown operator and the exact quorum-overlap chain
  `QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1`.
- <https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/>
  — uVRR motivation; TigerBeetle-style durability framing.
- <https://simbo1905.wordpress.com/2024/04/12/the-network-is-faster-than-the-disk/>
  — the deferred-flush economic rationale (§3).
