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
| `reincarnating` | wire phase: old identity pending eviction, new identity a weight-0 learner |

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
rules (simbo1905 blog, 2017-03-16: halve/double all weights, or change one node's
weight by one). Unit-weight special case, evicting `N2` and reincarnating it
as `N2′`:

| Step | Weights (N0, N1, N2, N2′) | Rule |
|---|---|---|
| D0 | (1, 1, 1, –) | baseline |
| D1 | (1, 1, 0, –) | exiting node weight 1→0 (subtract one) |
| D2 | (1, 1, 0, 0) | new node joins at weight 0 (learner; total unchanged) |
| D3 | (1, 1, 0, 1) | new node 0→1 (add one) |

**Era-safety invariant:** for every consecutive pair of configurations in the
sequence, the strict majorities of the two eras overlap (equivalently, each step obeys
the halve/double-all or ±1-unit rule, so consecutive majority families intersect).
Hence **every intermediate era is quorum-safe on its own**: a leader dying mid-sequence
leaves only safe membership eras, and a new leader can continue the sequence safely.

**Doubled-weights corner.** If the reincarnation happens while the cluster sits in the
doubled state of the blog's safe-replacement plan (survey Example B, state B2), the
sequence runs at the doubled scale and the return path to unit weights needs one extra
unit step before the halve, because halving is integral only when all weights are even:

| Step | Weights (N0, N1, N2, N2′) | Rule |
|---|---|---|
| C0 | (2, 2, 2, –) | doubled baseline |
| C1 | (2, 2, 1, –) | exiting node 2→1 |
| C2 | (2, 2, 0, –) | exiting node 1→0 |
| C3 | (2, 2, 0, 0) | new node joins at 0 |
| C4 | (2, 2, 0, 1) | new node 0→1 |
| C5 | (2, 2, 0, 2) | **corner**: new node 1→2 so the halve is integral |
| C6 | (1, 1, 0, 1) | halve all weights |

Every adjacent pair above obeys a scaling or unit rule, so every intermediate era is
quorum-safe by the same invariant. (The uniform distance-one bound for arbitrary
changes is sharp, but scaled steps preserve overlap — the checked
`WeightedGeneral.scaled_overlap` result covers each step.)

## 6. Membership-discard check

Messages **FROM** a node that is not in the current voting configuration — a weight-0
learner, or a superseded old identity — are **discarded** by the standard membership
check on ingress. Messages **TO** such a node are fine. This is good practice
independent of reincarnation; the crash-vector collector's
`old_reply_rejected` (rung 18) is the same primitive: once a later incarnation is
known for a node, replies from the old identity never regain eligibility, regardless
of redelivery.

## 7. Streaming order

The leader stays live for client requests throughout. It streams, in order:

1. the **first reconfiguration** (the forced sequence of §5),
2. then **client traffic**,
3. and **preemptively streams** to the reincarnated node **as a learner**
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
  weight-0 learner obeys and why the leader may safely stream to it. Its `generation`
  ghost is superseded as a carrier by the durable superblock incarnation (§4); the
  machinery stays.
- Rung 18 (`CrashVector`): `old_reply_rejected` is the membership-discard check for
  the old identity (§6).
- Rung 19 (`AcquisitionOrder`): the acquisition-order induction is what the
  reincarnated node's streamed-state acquisition must satisfy; its open retention
  premise becomes dischargeable — under reincarnation, retention of incarnation
  knowledge is the durable superblock identity itself.
- Rung 20 (`RecoveryAcquire`): the echoed `(incarnation, sequence)` request identity
  is the wire-level freshness contract the reincarnation message and learner streaming
  use; `crashed` bumping the incarnation is the formal shape of dirty ⇒ bump.
- Rungs 1–9 (eras, weights, weighted-general): the forced sequence is a path through
  the existing weighted-era space; each step's safety is the weighted-overlap
  argument.

The formalization rung for reincarnation itself (forced-sequence transition, per-era
safety, superblock classification) is planned as a later ladder rung and does not
exist yet.

## 10. Future work

**Speculative learner recovery:** a zero-weight learner may speculatively recover its
log non-votingly — acquiring state by streaming while never voting — so that the
0→1 promotion finds the node already caught up. Stated as future work, not claimed.

## Grounding sources

- <https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/> — voting
  weights; halve/double and ±1 unit rules; learners at weight 0.
- <https://simbo1905.wordpress.com/2016/12/16/upaxos-unbounded-paxos-reconfigurations/>
  — era-indexed reconfiguration, consecutive-configuration overlap, casting vote.
- <https://simbo1905.wordpress.com/2020/05/23/one-more-frown-please-upaxos-quorum-overlaps/>
  — the frown operator and the exact quorum-overlap chain
  `QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1`.
- <https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/>
  — uVRR motivation; TigerBeetle-style durability framing.
- <https://simbo1905.wordpress.com/2024/04/12/the-network-is-faster-than-the-disk/>
  — the deferred-flush economic rationale (§3).
