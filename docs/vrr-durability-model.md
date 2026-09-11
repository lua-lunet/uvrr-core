# VRR-2012 Protocol State and SANS-I/O Host Contract

**Status:** design analysis. This document specifies protocol-visible state and host obligations. It does not specify a storage engine, file format, retention policy, or threading implementation.

## Abstract

This document defines the state, persistence, recovery, and concurrency boundary for a SANS-I/O implementation of Viewstamped Replication. `VSR-1988` denotes Brian M. Oki and Barbara H. Liskov's paper [*Viewstamped Replication: A New Primary Copy Method to Support Highly-Available Distributed Systems*](https://www.cs.princeton.edu/courses/archive/fall11/cos518/papers/viewstamped.pdf), presented at PODC in August 1988.[^oki-dissertation] `VRR-2012` denotes Barbara Liskov and James Cowling's later paper, [*Viewstamped Replication Revisited*](https://dspace.mit.edu/entities/publication/80846d94-fcd3-40e6-87fb-8d91fe99a5d1), published in 2012. `vrr-core` implements VRR-2012 only. VSR-1988 is discussed solely to explain the origin and purpose of the VRR-2012 view-change fence.

[^oki-dissertation]: The fuller contemporary treatment is Brian Masao Oki's MIT dissertation and technical report, [*Viewstamped Replication for Highly Available Distributed Systems*](https://publications.csail.mit.edu/lcs/pubs/pdf/MIT-LCS-TR-423.pdf), supervised by Professor Barbara H. Liskov, submitted in May 1988, and issued as MIT/LCS/TR-423 in August 1988.

VRR-2012 is **diskless** in the precise sense used here: no normal-operation or view-change message has a protocol-required forced local-storage barrier. It uses the volatile state of a quorum of replicas as stable protocol state. A host may nevertheless persist protocol progress, accepted operations, application state, or any combination. Those are distinct logical concerns even when one host transaction records them together.

The core exposes only state and operations required by VRR-2012. Physical layout, retained history, write batching, flush frequency, file lifecycle, and garbage collection are exclusively host policy.

## 1. Terminology

### 1.1 Protocol versions

| Name | Definition |
|---|---|
| `VSR-1988` | Brian M. Oki and Barbara H. Liskov, *Viewstamped Replication: A New Primary Copy Method to Support Highly-Available Distributed Systems*, ACM PODC, 1988. |
| `VRR-2012` | Barbara Liskov and James Cowling, *Viewstamped Replication Revisited*, MIT-CSAIL-TR-2012-021, 2012. |
| `vrr-core` | The Rust implementation under analysis. It is intended to implement VRR-2012. |

The phrases “revised VR” and “the revised protocol” are not used without the explicit identifier `VRR-2012`.

### 1.2 View and configuration generation

For a fixed ordered membership of size `N`, VRR-2012 defines a monotonically increasing **view number** `v` and selects the primary deterministically:

```text
primary(v) = v mod N
```

This document uses **view** in the VRR-2012 sense: a primary-succession number. A change of membership, voting weights, or legal quorum families creates a new **configuration generation**, normally denoted `g`. Section 8.7 uses the synonymous term **era**, denoted `e`, where required to state the Unbounded VSR construction and its view-number encoding precisely.

`Replica.view` is therefore the current VRR view number. It is not a configuration-generation identifier.

### 1.3 Slots and frontiers

| Term | Definition |
|---|---|
| `accepted` | Highest slot for which the replica has accepted an operation in its current logical history. |
| `committed` | Highest slot known to be fixed by a normal-operation quorum. |
| `applied` | Highest committed slot incorporated into the local application state. |

Two view values are required:

| Value | Definition | Used for |
|---|---|---|
| `current_view` | The greatest view the replica has entered. The replica is fenced from normal-operation messages in all lower views. | Exact-view validation of normal messages; `primary(current_view)`; initiation and coordination of the current view-change attempt. |
| `retained_view` | The view at which the replica's currently retained logical history was most recently selected and installed. It identifies that history's provenance. | Ranking `DoViewChange` reports when selecting the history for a new view. |

The status-dependent relation is:

```text
Active (`Status::Normal`):
                        current_view = retained_view
ViewChange:             current_view >= retained_view
Restarting/Joining/Replaying:   current_view is not an authority to participate
```

A replica can enter `current_view = v+1` on a timeout or `StartViewChange(v+1)` while still reporting the history retained from view `v`. It has entered the later view as a fence, but no new view state has yet been selected and installed. Therefore `current_view` alone does not identify the provenance of the reported log.

VRR-2012 carries `retained_view` in `DoViewChange`. The new primary ranks candidate histories first by this value and then by the accepted frontier. A longer history retained from an earlier view must not displace history retained from a later view merely because it has a larger slot count. `retained_view` is not needed for normal request processing; it is view-change evidence.

The invariant is:

```text
applied <= committed <= accepted
```

The existing code represents these values as `executed_slot`, `commit_slot`, and `slot` respectively.

## 2. Baseline failure and quorum model

This section states the simple-majority model implemented by the present core. Section 8 generalises the proof obligations to quorum families, voting weights, and even-sized configurations.

Let:

```text
N = 2f + 1       replica count
Q = f + 1        quorum size
```

VRR-2012 assumes crash failures rather than Byzantine failures. Messages may be delayed, reordered, duplicated, or lost. Safety requires:

1. no more than `f` replicas are failed or recovering at one time;
2. a recovering replica counts as failed;
3. a replica which has lost volatile state does not participate in normal operation or view change until recovery establishes sufficiently recent state;
4. every protocol message identifies its view;
5. a replica does not process normal-operation messages from an earlier view.

For three replicas, at most one replica may be failed or recovering. For five replicas, at most two may be failed or recovering. Increasing the group size widens the tolerated failure set; it does not change the protocol.

Liveness additionally requires eventual communication among a quorum, an available primary in some later view, and timeouts which eventually permit useful work.

## 3. Protocol-visible state

The algorithm interacts with the following state classes.

| State class | Values | Algorithmic purpose | Host strategy boundary |
|---|---|---|---|
| Configuration | generation, ordered node identities, voting weights, and legal quorum families | Defines voting authority and `primary(v)` | Supplied by the host in the current core; planned replicated Unbounded VSR support is specified in §8.7 |
| Progress | current view, retained view, status, `accepted`, `committed`, `applied` | Fences older views and identifies the provenance of retained protocol history | Candidate for a host-supplied progress strategy |
| Accepted history | logical slot-to-operation history required for normal operation, view change, recovery, and state transfer | Preserves operations which a later view may have to select | Candidate for a host-supplied journal strategy |
| Application state | lock/lease state or other replicated service state | Result of applying committed operations | Host-owned application upcall boundary |
| Quorum evidence | prepare acknowledgements, start-view-change senders, view-change reports | Proves one in-flight protocol transition | Core-local and transient; no storage strategy |
| Event clock | one `u64` value sampled at the start of every input | Orders host observations | Host-owned; the core never reads a system clock |
| Transfer state | source cursor, destination assembly, encoding state | Carries logical history between replicas | Per-transfer state; never shared protocol progress |

No core API determines how much accepted history a host retains. The host may retain exactly the currently required logical history or substantially more. Retention and physical reclamation are outside the protocol interface.

## 4. Logical journal contract

The term **journal** denotes the host strategy through which the core observes and records accepted protocol history. It does not imply a file, WAL, database table, mutable array, or indefinite retention.

The algorithm requires the following semantic capabilities:

| Capability | Required semantics |
|---|---|
| Identify the accepted frontier | Return the highest slot in the current logical accepted history. |
| Read protocol history | Return operations required for comparison, view-change reports, recovery, application, or state transfer. |
| Record acceptance | Make a newly accepted `(slot, operation)` part of the current logical history under the current view. |
| Record view selection | Make the history selected by the VRR-2012 view-change rule the current logical history for the new view. |

These operations describe logical protocol facts. They do not prescribe physical modification. Recording a view selection changes which logical history the replica presents to the protocol; it says nothing about the physical representation or retention of any data.

The journal contract contains no physical deletion, truncation, file rotation, segment management, size threshold, age threshold, or minimum-retention operation. None is a VRR-2012 state transition. A host may implement any such policy without exposing it to the core, provided the journal continues to satisfy protocol reads or reports that the requested history is unavailable.

If requested history is not locally available, the host reports that fact. Recovery or state transfer must then obtain an adequate state from another replica or use a host-specific application-state transfer facility. The core does not prescribe why the history is unavailable or how the host manages retained data.

## 5. Progress contract

`Progress` is a compact protocol record, conceptually:

```text
Progress {
    view,
    status,
    retained_view,
    accepted,
    committed,
    applied,
    fault
}
```

Not every field must be persisted. The selected durability profile determines which values survive a local crash.

The following cross-strategy invariants are mandatory:

1. published `accepted` equals the accepted frontier reported by the current logical journal history;
2. published `committed` does not exceed `accepted`;
3. published `applied` does not exceed `committed`;
4. a durable frontier must not claim state which the host cannot recover under the same declared durability profile;
5. after an indeterminate persistence result, `fault` is sticky until explicit recovery establishes a coherent state.

`status` is process control as well as protocol state. The entry state is decided by the superblock marker read at boot, never assumed: a node that completed a controlled shutdown restarts as `Restarting`; a node whose markers prove no controlled shutdown resurrects under a bumped identity as `Joining`.

### 5.1 The marker transition machine — safe crash detection without flushes on the hot path

uVRR performs **no disk flushes on the normal path**. A node ordered to stop instantaneously refuses to send messages, so any disk flush sits outside the protocol's hot path: the client-visible protocol never waits on a disk. This is the fundamental construction over VRR-2012: VRR's recovery protocol exists to repair amnesiac voters, and its cost lands either on the hot path (forced flushes, quorum recovery exchanges) or on the safety edge (an amnesiac voter voting). uVRR removes the class: there is no crash-recover, only a clean-stop restart and a crash-stop-rejoin.

The superblock markers are an ordered transition system — four states, three transitions. The quorum question at boot is *did the transition complete?*, answered by 2-of-4 copies holding the state to the right of the transition (the store's open threshold; the working quorum also resolves the identity, higher-identity-wins, and repairs the remaining copies to 3-of-4 or better):

```text
Running ──stop──> Stopping ──drain──> Stopped ──boot, 2-of-4──> Restarting
                  (4x write)  flush     (4x write)              (4x write)
                              WALs + grids
Running ──crash──> (markers unchanged) ──boot, no 2-of-4 Stopped──> Joining
                                                       (bump, 4x write)
```

- **Stopping→Stopped is the clean-shutdown protocol.** On the stop command the node first writes `Stopping` 4x, then drains — flushes both WALs and the grids — then writes `Stopped` 4x. The drain happens strictly *between* the two marker states, so a `Stopped` marker vouches for the WAL under it: reading 2-of-4 `Stopped` proves the transition completed, which proves the drain completed, which proves **there is no amnesiac risk**. A stop that dies partway still reads as clean on the surviving quorum, correctly, because the flush had already completed before the first `Stopped` write.
- **Stopped→Restarting is the boot of a controlled shutdown.** The node keeps its identity, writes `Restarting` 4x, and runs the Restarting protocol — identical to the original recovery protocol, renamed because the node completed a controlled shutdown. It is a member with complete state: it ticks the full protocol, and suspects a silent primary like any backup. No `Started` state is written: no safety logic looks for `Started`, it looks for `Stopped` — the extra superblock write buys no safety and is elided.
- **Anything-else→Joining is the resurrection.** No 2-of-4 `Stopped` — a crash, a torn marker set, or death mid-join — means this identity is dead. The node bumps its identity, writes `Joining` 4x, and broadcasts `evict(old), join(new, [frontier])`. It is not a cluster member: it neither votes nor drives view change, and every node drops its messages by the membership checks. The leader alone answers: it retransmits what the frontier shows the node lacks and keeps streaming `Prepare`/`Commit` while the forced sequence commits — the old identity zeroed, the new zeroed, then voted in — and the new membership announces on the commit.

A node dying mid-join reads no `Stopped` quorum and resurrects again. A running node's markers hold `Restarting` (or `Joining`) from its boot write, so a crash leaves exactly the no-controlled-shutdown evidence the next boot needs. The marker writes themselves are durable-on-write (flushed), which is the only disk traffic outside the stop path.

## 6. Functional core model

The SANS-I/O transition is:

```text
TimedInput {
    at: u64,
    event: Input,
}

delta : TimedInput x State x JournalView
     -> CandidateState x PersistenceIntent x Effects
```

The host samples `TimedInput.at` when it begins dispatching the event. A peer datagram therefore carries the time at which its receive event began; a timeout carries the time at which the host dispatched that timeout. The value is host observation metadata, not a timestamp received from a peer.

The core performs no clock reads. It neither selects a clock source nor calls an operating-system time API. This preserves deterministic replay and keeps clock behaviour within the SANS-I/O host contract.

`Input` is one of:

- a proposed operation;
- peer message;
- timer event;
- recovery request;
- application completion.

`PersistenceIntent` contains only logical protocol changes:

```text
PersistenceIntent {
    progress_change?,
    journal_change?
}
```

`Effects` contains:

- peer messages;
- application upcalls;
- timer requests.

The host may record progress, journal changes, and application changes in one wider transaction. That composition does not merge their logical semantics.

### 6.1 Event-clock contract and the freshness carrier

`TimedInput.at` is an unsigned 64-bit tick supplied by the host. Nanosecond-resolution time is preferred because normal process scheduling makes accidental reuse unlikely. The core treats the value as opaque and does not convert units.

Classic VRR-2012 diskless recovery carries freshness in a recovery nonce: the host tick of each recovery event, with a bounded nonce set per attempt and a delayed response counted iff its echoed nonce is still remembered. That carrier exists because a classic diskless restart keeps its identity and has no durable freshness record. uVRR does not perform that exchange: the freshness carrier is the durable four-superblock incarnation — a dirty node bumps its incarnation (Crash-Stop-Self-Evict), so freshness survives the crash as durable identity rather than as a nonce set. The tick remains the host's observation metadata (S4) and the `(incarnation, sequence)` request identity of the acquisition certificates; the classic-VRR nonce rules above are retained here as literature about the classic design they govern.

## 7. Transition publication and durability

For each state-changing input, the host must serialize the following interval:

```text
1. read the current published state and journal view;
2. compute CandidateState, PersistenceIntent, and Effects;
3. validate all state invariants;
4. submit the persistence intent to the selected host strategies;
5. complete the durability action required by the host profile;
6. atomically publish CandidateState;
7. release Effects whose preconditions are now satisfied.
```

The core must not emit an acknowledgement or view-change report before the state supporting that message has reached the stability level assumed by the selected proof:

| Stability level | Meaning at effect release |
|---|---|
| `Volatile` | State is installed in process memory. Safety relies on VRR-2012 quorum memory and recovery. |
| `Deferred` | The host has accepted a write for later durability. Until a barrier completes, safety remains the `Volatile` case. |
| `Forced` | The host confirms a host-selected local crash/power-loss barrier. This may improve recovery but does not change the VRR-2012 protocol sequence. |
| `ExternalTransaction` | The host confirms its wider transaction, potentially including application state. |

Calling a mutation “journalled” does not establish durability. The relevant property is the barrier completed before the dependent effect becomes observable.

An indeterminate persistence result places the node in a sticky faulted state. The process must not guess whether the transition committed.

## 8. Quorum invariants

### 8.1 Baseline majority invariant

A primary appends an operation at slot `n` and sends `Prepare(v,n,operation)`. A backup installs the operation before returning `PrepareOk(v,n)`. An operation commits after it is represented at `Q` replicas, counting the primary.

Let `C(n)` be the quorum which committed slot `n`. Let `D(v+1)` be the quorum of `DoViewChange` reports used to construct view `v+1`. Since both contain `f + 1` replicas in a universe of `2f + 1`:

```text
|C(n)| + |D(v+1)| = 2f + 2 > 2f + 1
therefore C(n) intersect D(v+1) is non-empty.
```

The VRR-2012 log-selection rule uses this intersection to preserve every committed operation in the same slot in all later views.

### 8.2 Quorum families, not quorum counts

A quorum policy defines a family of legal node sets. Let:

```text
C_g = family of normal-operation commit quorums in configuration generation g
V_g = family of view-change quorums in configuration generation g
R_g = family of recovery quorums in configuration generation g
```

A concrete recovery quorum contains only responses from replicas whose state survived independently. The recovering replica does not count its own lost or unqualified state toward `R_g`.

In the notation of the reconfiguration safety argument, `C_g` plays the `QII_g` role and `V_g` plays the `QI_g` role for the limited purpose of history selection. These are names for proof obligations.

For quorum families `A` and `B`, write:

```text
A ⌢ B  iff  for every a in A and every b in B, a intersect b is non-empty
```

The distinction between a family and one concrete quorum is essential. Counts alone do not establish an overlap when node identities, weights, or configurations differ.

Within a fixed configuration, VRR history safety requires:

```text
C_g ⌢ V_g
```

Every view-change quorum must therefore observe at least one replica from every possible quorum which could have committed an operation.

### 8.3 The additional diskless-VRR overlap

VRR-2012 uses volatile quorum knowledge in place of a forced stable-storage view fence. Consequently, history selection is not the only quorum obligation. A recovery quorum must intersect every quorum which could have become fenced into a later view:

```text
F_g ⌢ R_g
```

where `F_g` is the family of `StartViewChange` fence quorums. If one policy uses the same family for view-change fencing, `DoViewChange` collection, and recovery, so that `F_g = V_g = R_g`, then it must additionally satisfy:

```text
V_g ⌢ V_g
```

This self-intersection is not implied by `QI ⌢ QII` alone. It is required by this diskless VRR construction because a recovering replica must encounter the volatile evidence that an earlier view was fenced. A quorum policy is therefore not automatically a valid VRR-2012 policy merely because `QI ⌢ QII` holds.

This section describes classic VRR-2012 diskless recovery (its §4.3) as literature, not uVRR: uVRR replaces the diskless recovery overlap with Crash-Stop-Self-Evict reincarnation. A crashed node whose superblocks record an unflushed session reopens under a new incarnation, the leader evicts the old identity through the forced weight sequence (exiting 1 to 0, joining at 0, then 0 to 1), and the new identity rejoins as a weight-0 standby — a **standby** is TigerBeetle's term for its non-voting cluster members (older drafts of this document called it a learner): standby nodes have a zero voting weight so cannot form part of any quorum nor actively participate in the VSR algorithm — so no recovery quorum meets a fence family, and the obligation above governs the classic design only.

### 8.4 Weighted quorums

Assign each voting replica a non-negative integer weight `w_i`. For total weight `W` and threshold `T`, define:

```text
Q(T) = { S : sum(w_i for i in S) >= T }
```

For two threshold families over the same weighted membership, the following is a simple sufficient condition for universal intersection:

```text
T_a + T_b > W
```

Exact validation is over the legal quorum families; the threshold inequality is sufficient but need not be necessary for every indivisible weight assignment.

A strict weighted-majority family uses:

```text
T_majority = floor(W / 2) + 1
```

It self-intersects and is therefore valid for `C_g`, `V_g`, `F_g`, and `R_g`. Weight zero grants no voting authority. A zero-weight member may receive state and serve as a standby, but it must become adequately caught up before a later configuration gives it positive weight.

The following transformations have precise quorum effects when each configuration uses strict weighted majorities:

| Transformation | Effect |
|---|---|
| Multiply every weight by the same positive integer | Leaves the legal quorum family unchanged. |
| Divide every weight by a common positive divisor | Leaves the legal quorum family unchanged. |
| Increase or decrease one replica's weight by one | Consecutive weighted-majority families universally intersect. |
| Join or leave a zero-weight replica | Leaves the voting quorum family unchanged. |

The one-unit rule is an intersection lemma, not a complete reconfiguration protocol. Promotion still requires state transfer, and removal still requires activation of the new configuration to fence messages authorized only by the old configuration.

### 8.5 Even-sized configurations

For `N = 2k` unit-weight replicas, use:

```text
V_g threshold = k + 1
C_g threshold = k
```

Then:

```text
(k + 1) + k > 2k       therefore C_g ⌢ V_g
2(k + 1) > 2k          therefore V_g ⌢ V_g
```

The primary counts itself in `C_g`, so a four-replica primary needs one follower acknowledgement to commit in the steady state. A view change still requires three replicas, counting the participating replica itself. This improves steady-state latency and availability under some partitions; it does not improve the failure tolerance of electing a primary.

### 8.6 Reconfiguration overlap

For adjacent configuration generations, the reconfiguration safety chain is:

```text
C_g ⌢ V_g ⌢ C_(g+1) ⌢ V_(g+1)
```

This notation denotes exactly three pairwise requirements:

```text
C_g       ⌢ V_g
V_g       ⌢ C_(g+1)
C_(g+1)   ⌢ V_(g+1)
```

It does not imply any other pairwise intersection by transitivity. In particular, it does not imply `V_g ⌢ V_(g+1)`.

The chain transfers to VRR as a necessary history-preservation condition: the view-change evidence associated with the transition must overlap both the old commit family and the new commit family. It is not by itself a complete diskless-VRR reconfiguration proof. Such a protocol must additionally specify:

1. which committed operation activates generation `g+1`;
2. which configuration authorizes each message while the transition is incomplete;
3. how old-generation voters become fenced;
4. how recovery intersects volatile view-fence knowledge across the boundary; and
5. when a zero-weight standby is sufficiently caught up to receive positive weight.

The *leader casting vote* is a pipelining optimisation rather than a safety condition. Its prepare/accept routing sequence is not itself a VRR view-change sequence. Section 8.7 defines the corresponding Unbounded VSR transition which this project intends to support; implementation remains conditional on a complete model and safety proof of that VSR-specific transition.

### 8.7 Planned extension: Unbounded VSR reconfiguration by voting weights

`vrr-core` will support **Unbounded VSR**, a non-stop reconfiguration protocol corresponding to David Turner's unbounded-pipelining reconfiguration result (reference 5) but stated in VSR operations, views, and quorum evidence. This section is the normative design target. It is not a claim that the current implementation already supports reconfiguration.

#### 8.7.1 Configuration

A configuration is:

```text
Configuration = <order, W>
```

where `order` is an ordered list of distinct node identifiers and:

```text
W : node_id -> non-negative integer
T(W) = sum(W(n))
Q(W) = floor(T(W) / 2) + 1
```

A set `S` is a quorum exactly when:

```text
sum(W(n) for n in S) >= Q(W)
```

Era `e` indexes the configuration sequence. `config(e)` is obtained by folding committed reconfiguration operations through the operation which establishes era `e`. Node order determines primary rotation; weights determine voting authority.

#### 8.7.2 Reconfiguration operations

The replicated operation set is:

```text
VOID
INIT(order)                 -- every initial member has weight 1
INCREMENT(node_id)
DECREMENT(node_id)
DOUBLE
HALVE
JOIN(node_id, position)      -- insert with weight 0
LEAVE(node_id)             -- permitted only at weight 0
```

The preconditions are:

| Operation | Preconditions |
|---|---|
| `VOID` | Operation number 1, view 0, era 0, with an otherwise empty log. |
| `INIT(order)` | Operation number 2, view 0, era 0, immediately preceded by `VOID`; not proposable after any view change. |
| `INCREMENT(n)` | `n` is a member. |
| `DECREMENT(n)` | `W(n) >= 1`. |
| `DOUBLE` | None beyond the common invariants. |
| `HALVE` | Every `W(n)` is even. |
| `JOIN(n,p)` | `n` is not in `order`; `p` is a valid insertion position. The new weight is zero. |
| `LEAVE(n)` | `n` is a member and `W(n) = 0`. |
| Every operation | The result has non-empty `order` and `T(W) >= 1`. |

`VOID` and `INIT` make initial configuration construction part of the replicated history rather than unrecorded ambient state. A newly added zero-weight member has no voting authority. State transfer must make it adequately current before a later committed `INCREMENT` grants authority.

#### 8.7.3 View-number construction

The view number encodes both configuration era and the primary-selection index:

```text
view = (era << k) | index
```

The low `k` bits contain `index`; the remaining bits contain `era`. The primary is:

```text
primary(view) = config(era(view)).order[
    index(view) mod length(config(era(view)).order)
]
```

A replica may propose a view only if its accepted history contains the committed reconfiguration operation establishing that view's era. View-number gaps are legal. The implementation must use checked encoding and must reject an era or index which cannot be represented; wraparound is forbidden.

Turner's relation between ballot era and slot era becomes the following VSR relation between the current view and the configuration authorizing an operation slot:

```text
era(view) + 1 >= era(slot) >= era(view)
```

The `+1` case is overlap mode: the primary still operates in an era-`e` view while committing operations using the era-`e+1` replication quorum.

#### 8.7.4 Safety requirements

Let `QI_e` be the view-change quorum family and `QII_e` the normal-operation commit quorum family for era `e`. For every era:

```text
R1: QI_e ⌢ QII_e
R2: QI_e ⌢ QII_(e+1)
```

Together with `R1` instantiated at `e+1`, these are the three consecutive pairwise intersections:

```text
QII_e ⌢ QI_e ⌢ QII_(e+1) ⌢ QI_(e+1)
```

The classic diskless-VRR fence/recovery overlap in §8.3 governs classic VRR-2012 diskless recovery, which uVRR replaces with Crash-Stop-Self-Evict reincarnation; it does not bind uVRR.

#### 8.7.5 Closure of weighted-majority configurations

For `Q(W) = floor(T/2) + 1`, each permitted operation preserves the required overlap between consecutive strict weighted-majority configurations. The identity used below is:

```text
floor(T/2) + floor((T+1)/2) = T
```

**`INCREMENT`, `T -> T+1`.** Assume an old-era quorum `A` and a new-era quorum `B` are disjoint. Measured under the new weights:

```text
w_(e+1)(A) + w_(e+1)(B) <= T + 1
```

But:

```text
w_(e+1)(A) >= w_e(A) >= floor(T/2) + 1
w_(e+1)(B) >= floor((T+1)/2) + 1
```

Their sum is at least `T+2`, a contradiction.

**`DECREMENT`, `T -> T-1`.** For disjoint old-era quorum `A` and new-era quorum `B`:

```text
w_(e+1)(A) >= w_e(A) - 1 >= floor(T/2)
w_(e+1)(B) >= floor((T-1)/2) + 1
```

Disjointness would require their sum to be at most `T-1`. The lower bounds instead imply `T <= T-1`, a contradiction.

**`DOUBLE`.** The new threshold is `T+1`, and:

```text
2w(S) >= T+1
iff w(S) >= ceil((T+1)/2)
iff w(S) >= floor(T/2)+1
```

The legal quorum family is unchanged. `HALVE` is the inverse when every weight is even.

**`JOIN` and `LEAVE` at weight zero.** Total voting weight is unchanged and the affected member contributes zero to every sum. The voting quorum family is unchanged.

These lemmas establish overlap closure. They do not establish that a particular leader has the pivot required for the non-stop path.

#### 8.7.6 Pivot condition

Let `L` be the primary of view `v` in era `e`. `L` may use non-stop reconfiguration only if it can select concrete quorums `qI` and `qII` such that:

```text
qI intersect qII = {L}
qI  is legal under config(e)
qII is legal under config(e)
qII is legal under config(e+1)
```

For unweighted threshold quorums this corresponds to `|qI| + |qII| = N+1`. With weights, `L` must be pivotal. A low-weight primary may have no legal split. Failure to construct the pivot is not a protocol fault: the core falls back to the ordinary stop-the-world view-change path.

The notation distinguishes quorum families (`QI`, `QII`) from the concrete quorums selected for one transition (`qI`, `qII`).

#### 8.7.7 Non-stop transition

The non-stop transition is:

1. `L` sends `Prepare` for the reconfiguration operation to `qII` under view `v`.
2. When acknowledgements from `qII` commit the operation, era `e+1` is established. `L` may continue streaming further operations to the same `qII` under `v`; these slots are authorized by `config(e+1)`.
3. `L` selects `v' > v` such that `era(v') = e+1` and `primary(v') = L` under the new ordered membership. View-number gaps are permitted.
4. `L` sends `PlannedViewChange(v')` only to `qI - {L}`. This message solicits planned view-change evidence. It is never sent to `qII - {L}` and does not itself make a recipient stop accepting otherwise valid `Prepare` messages in view `v`.
5. Each recipient records the planned target separately from `current_view` and returns its `DoViewChange` evidence for `v'`. After `L` has all responses from `qI - {L}`, it casts the final vote locally. Completion of `qI` and `L`'s switch to `v'` are one serialized node transition. `L` retains its own accepted history, which is at least as complete as every history it generated and sent during overlap mode.
6. `L` broadcasts `StartView(v', suffix, accepted, committed)` to every member of `config(e+1)`. The bounded-suffix and missing-range rules in §13.1 apply.
7. `L` sends subsequent operations under `v'` to a legal `QII_(e+1)` quorum.

The operation stream need not pause during steps 1–7: while planned view-change evidence is collected, `L` continues committing operations through `qII` in view `v`. `PlannedViewChange` is not `StartViewChange`: it neither establishes the ordinary diskless view fence nor changes `current_view` at a recipient. Its safety depends on the disjoint concrete quorum routing and on `L` supplying the casting vote as the serialized transition point.

#### 8.7.8 Primary failure during overlap mode

If `L` fails after sending `PlannedViewChange(v')` but before completing its local cast and `StartView`, members of `qI - {L}` have recorded the planned target `v'`, while members of `qII - {L}` may remain in view `v`. No planned quorum completed because `L` withheld the pivotal vote.

A survivor may propose only an era whose establishing reconfiguration operation is present in its accepted history:

- a replica without the era-`e+1` operation proposes in era `e` and performs ordinary view change using `QI_e`, which intersects both `QII_e` and `QII_(e+1)` by `R1` and `R2`;
- a replica with the era-`e+1` operation may propose in era `e+1`; ordinary `R1` at `e+1` makes its view-change quorum intersect operations accepted by `QII_(e+1)`.

The planned target is not a completed view fence and is discarded or superseded by the ordinary view-change result. Recovery and history selection use the standard VRR paths; there is no special recovery algorithm for a failed overlap-mode primary.

## 9. View-change protocol and persistence

### 9.1 VRR-2012

A timeout initiates a view change; it does not authorize a new primary by itself:

```text
replica R enters view v+1
    -> R changes status to view-changing
    -> R broadcasts StartViewChange(v+1)

after R observes StartViewChange(v+1) from f other replicas
    -> a quorum, including R, is known to have entered v+1
    -> R sends DoViewChange(v+1, R's logical state) to primary(v+1)

after primary(v+1) receives a DoViewChange quorum
    -> select protocol state according to the VRR-2012 rule
    -> broadcast StartView(v+1, selected state)
```

The two quorums establish different facts:

| Quorum | Fact established |
|---|---|
| `StartViewChange` quorum | A quorum is fenced from all earlier views. |
| `DoViewChange` quorum | The designated new primary has sufficient state evidence to select the new view safely. |

If `primary(v+1)` is unavailable, view `v+1` cannot complete. Replicas eventually advance to a later view. No cooperation from the old primary is required.

### 9.2 Why VRR-2012 has two exchanges

The `DoViewChange` quorum alone is insufficient if view numbers are volatile. VRR-2012 §8.2 gives the counterexample:

1. replica `R` sends `DoViewChange(v+1,state_old)`; the message is delayed;
2. `R` crashes and loses its knowledge of `v+1`;
3. `R` recovers into `v` and helps the old primary commit later operations;
4. the delayed report reaches `primary(v+1)`;
5. the new primary counts a report whose state predates those later commits and whose sender no longer honours the implied fence.

Messages accumulated at different times do not necessarily prove that a quorum is simultaneously fenced.

The preliminary `StartViewChange` quorum prevents this execution. Before any `DoViewChange` is sent, `Q` replicas know that view `v` has ended. Under the failure bound, recovery must intersect that knowledge and cannot legitimately return the sender to `v`.

### 9.3 Historical contrast: VSR-1988

VSR-1988 establishes the fence using local stable storage:

```text
replica R enters view v+1
    -> R durably records highest_view_entered >= v+1
    -> R sends DoViewChange(v+1, R's logical state)
```

After restarting, `R` cannot participate in any lower view. Therefore the `StartViewChange` exchange may be omitted. The new primary still requires a `DoViewChange` quorum and must still select state safely.

“Persist the view” means that each sender records its own monotonic view fence on its own stable storage before its report is released. It does not mean distributing the view record or permitting the new primary to start without a quorum.

Persisting only the view fence does not recover accepted operations, commitment, application state, or application completions. A replica which recovers only that fence remains unable to participate until the remaining state has been restored or recovered.

This mechanism is not a selectable `vrr-core` protocol mode. `vrr-core` always retains the VRR-2012 `StartViewChange` exchange. A host may persist progress for recovery, but that persistence does not permit the core to omit or abbreviate the VRR-2012 view-change protocol.

### 9.4 Three- and five-replica groups

The VRR-2012 quorum-memory failure envelope is determined by group size:

- with three replicas, quorum memory tolerates one failed or recovering replica;
- with five replicas, quorum memory tolerates two failed or recovering replicas;
- survival of loss beyond `f` volatile replica states requires additional durable recovery state, not only a view number.

The narrower operational envelope of a three-replica group may make local persistence attractive as a host recovery policy. It does not make VRR-2012 unsafe within its stated failure model, and it does not change the view-change sequence implemented by `vrr-core`.

### 9.5 `StartViewChange` is not a pre-vote

`StartViewChange` is not a pre-vote. Sending it means that the sender has entered the proposed view and ceased normal participation in earlier views. Receipt may cause another replica to do the same. The exchange is therefore deliberately disruptive once it reaches a quorum; that disruption establishes the volatile fence required by the recovery proof.

A read-only poll performed before entering a view would be an additional liveness optimisation, not part of VRR-2012. It could reduce disruption when one replica's timeout fires while a quorum still receives useful primary traffic. It could not prove that no primary exists, because an asynchronous system cannot distinguish failure from delay. Such a poll would create no view fence and could replace neither VRR-2012 view-change exchange. It is outside the present core specification.

## 10. Higher-view messages

A normal-operation message from a higher view proves that the receiver is stale. It does not prove which logical history or commit frontier the receiver must install.

The safe transition is:

```text
higher-view evidence
    -> cease participation in lower views
    -> enter the specified state-transfer or view-change path
    -> install state only from protocol-qualified evidence
```

VRR-2012 states that a replica receiving a normal message from a later view performs state transfer before processing it. The current `vrr-core` implementation instead ignores higher-view `Prepare` and `Commit` messages because their guards require exact view equality. Only `StartViewChange`, qualifying `DoViewChange`, and `StartView` paths advance `Replica.view`.

A rejection or NACK containing the higher view can accelerate convergence. It is a liveness optimisation, not part of the quorum-intersection safety argument.

## 11. Application boundary

`vrr-core` orders opaque operations. It does not model clients, sockets, retries, deduplication, forwarding, or replies.

An operation is:

```text
Operation {
    id:      OperationId,   -- opaque correlation token { msb: u64, lsb: u64 }
    payload: bytes
}
```

`OperationId` is an opaque 128-bit correlation token. The core never compares identifiers for duplicate detection and assigns no retry semantics. Repeating an identifier is not suppressed; the host protocol decides whether a repetition is a retry, a duplication, or another valid invocation. Transport, leader forwarding, connection tracking, and response formatting are host extensions.

### 11.1 Application upcalls and completion

Commitment and application are separate state transitions. When a replica locally learns that an operation is committed, it emits an ordered application upcall containing `(slot, OperationId, payload)`. Every replica applies the operation to its local host state, so every replica emits the same ordered application sequence.

The host reports `Applied { slot }` as a later serialized input. The completion carries no result: application results never enter consensus state.

A host may retain an ephemeral `OperationId -> pending request` association. After successful application and completion publication, the host returns its result only when such an association still exists; otherwise it discards the result. On process failure the pending connections and associations disappear, while the operation may nevertheless commit and apply. An I/O failure therefore means the caller cannot know the write outcome and must reconnect and query according to the host protocol.

Recovery re-emits the application upcalls of committed-but-unapplied slots: the §6.1 fast-forward emits the newly committed slots' upcalls as the frontier advances, and the completion's replay walk does not re-emit a slot the fast-forward already emitted in the same process life. A crash discards that emission memory, so replay after a crash re-emits from the durable `applied` frontier. Application durability, replay handling, and side-effect semantics remain host responsibilities.

### 11.2 Application participating in a host transaction

```text
establish committed n
    -> stage progress and journal intent
    -> apply operation n in the host transaction
    -> stage applied n
    -> commit the host transaction
    -> publish candidate state
    -> release peer effects
```

### 11.3 Application not participating in the same transaction

```text
establish committed n
    -> publish committed n after the selected stability action
    -> emit the application upcall for n
    -> receive Applied { slot: n } as a later serialized input
    -> publish applied n
```

A crash may cause an application upcall to be issued again. Exactly-once external side effects require cooperation from the application through idempotence, an operation-identity transaction, or durable deduplication state. The consensus core cannot manufacture exactly-once effects across an external application boundary.

### 11.4 Contrast with the VRR-2012 client table

The VRR-2012 paper describes a per-client table recording each client's latest request number and result, with at most one outstanding request per client, so that the primary can suppress duplicate requests and replay cached results (<https://dspace.mit.edu/server/api/core/bitstreams/9f8c52b3-ea46-4fde-9dc9-354ed6d9c7d9/content>). That design is a client-proxy convenience for a specific request/response service shape. This generic core does not adopt it: correlation identifiers live inside log entries and therefore survive view change and recovery as ordinary protocol history, while duplicate policy, result caching, and reply delivery belong to the host. Section 9.2 of this document concerns why the view-change protocol has two exchanges; it is unrelated to request deduplication.

## 12. Concurrency contract for the C ABI

The host provides exactly one state-changing transition owner per node. This may be:

- a reentrant node-level write lock in a synchronous embedding;
- an actor or serial executor;
- a single libuv event-loop owner with queued continuations.

The required invariant is:

> No second state-changing transition may read the node state between the first transition's initial state read and its persistence/publication decision.

An asynchronous host retains logical ownership rather than holding an operating-system mutex across a yield:

```text
Idle
  -> Planning
  -> AwaitingStability
  -> Publishing
  -> ReleasingEffects
  -> Idle

Indeterminate persistence result -> Faulted
```

Inputs received during `AwaitingStability` are queued or rejected with backpressure. They may not start another transition from the same published state.

Read-only observation may load one immutable `Progress` value through an atomic reference. A reader which then reads journal history requires a host-provided stable read handle or equivalent consistency guarantee. The atomic frontier alone does not define journal concurrency.

## 13. Suggested optimisations

The following mechanisms may reduce messages, latency, or storage traffic. None changes the safety predicates stated above. Each optimisation must have a fallback whose correctness does not depend on the hint succeeding.

### 13.1 Bounded view-change suffix

The baseline `DoViewChange` description in VRR-2012 §4.2 carries the replica's log. Section 5.3 explicitly permits an implementation to carry only a small suffix—suggesting the latest one or two entries—and to request more information when that suffix is insufficient.

The amount sent initially is therefore policy, not a new consensus rule. A suitable host-aware policy is:

```text
1. reserve space for the complete DoViewChange envelope and fixed evidence;
2. read the newest accepted entries from the host journal;
3. add entries in reverse selection order until the configured transport
   budget, normally one datagram payload, would be exceeded;
4. encode them in ascending slot order with the commit frontier;
5. if the new primary cannot prove and construct the selected history from
   its collected evidence, request the missing range before StartView.
```

Small application operations will commonly fill otherwise unused packet capacity and repair the likely one- or two-entry lag without another round trip. Large operations may leave room for no suffix at all. Correctness is unchanged because the new primary may not install the new view until it has obtained sufficient history.

The suffix is `DoViewChange` evidence, not shared node state and not a new mutable replay buffer. The sender obtains a stable journal read handle, constructs one per-message value, emits it, and discards it. Concurrent transfers have independent cursors and buffers.

### 13.2 Transport coalescing and speculative replay

A host may coalesce multiple independently valid logical messages into one transport packet when its framing preserves their order. For example, it may place already-authorized retransmissions of recent normal-operation messages before the view-change frame. Duplicate delivery is harmless because protocol handlers must be idempotent.

This is only a performance hint:

- a replayed `Prepare` or `Commit` must have been authorized by the core; the host may not synthesize protocol messages from journal contents;
- a replica already fenced into a later view does not resume participation in an earlier view merely to send replay traffic;
- receivers may reject old-view normal messages after observing the view-change fence;
- replay traffic does not count as `DoViewChange` evidence and cannot replace the missing-range fallback; and
- packet size, MTU discovery, coalescing, fragmentation, and retransmission remain host transport policy.

The protocol-level optimisation should therefore be expressed primarily as a bounded suffix attached to `DoViewChange`. Transport-level replay is opportunistic and may be ignored or dropped without affecting safety.

### 13.3 Commit-frontier piggybacking

A primary may include its current commit frontier on a subsequent `Prepare`, allowing that message to serve the role of a separate `Commit` heartbeat for preceding slots. An idle primary still emits the host-scheduled heartbeat required for backups to learn commitment and detect failure. Piggybacking changes message count, not commit semantics.

### 13.4 Higher-view convergence hint

A rejection or response may carry the receiver's higher view so that a stale sender begins state transfer promptly. The value is evidence that the sender is stale, not evidence identifying which history it should install. Installation still follows the qualified view-change or state-transfer path in §10.

### 13.5 Read-only preflight before view change

A host may perform a non-fencing liveness probe before dispatching a view-change timeout. This can suppress unnecessary disruption when a quorum still observes useful primary traffic. The probe cannot establish failure in an asynchronous system and cannot replace either VRR-2012 view-change exchange.

### 13.6 Batching persistence and application

The host may batch journal writes, defer a flush, or apply several committed operations in one application transaction. The published `committed` and `applied` frontiers advance only when the corresponding transition has completed under the selected durability profile. Batching does not weaken the serialized transition-publication interval or manufacture exactly-once external effects.

### 13.7 Quorum-policy optimisations

Weighted quorums and the even-sized configuration described in §8 reduce steady-state acknowledgements or permit controlled membership evolution. They are selected quorum strategies with explicit intersection proofs, not transport hints. The planned Unbounded VSR generation-transition protocol is specified in §8.7.

## 14. Current implementation assessment

### 14.1 Present mechanisms

The current code contains:

- both VRR-2012 view-change exchanges;
- the self-inclusive `StartViewChange` quorum condition;
- a `DoViewChange` quorum and selected-state installation at the new primary;
- separate accepted, committed, and executed frontiers;
- explicit `Restarting`, `Joining`, and `Replaying` statuses;
- host strategies for the journal (`Journal`/`JournalView`, with the segmented in-memory implementation) and an explicit stability-completion boundary (`Stability`) gating dependent effects;
- the provisioning/reopen lifecycle: `provision` establishes the genesis configuration and joins fenced `Joining`, `reopen` restarts after possible state loss and starts fenced `Restarting`;
- the marker transition machine (§5.1): `Stopping→Stopped` proves the drain, 2-of-4 `Stopped` at boot proves the clean stop, `Restarting` ticks the full protocol, and a missing stopped quorum resurrects as `Joining` under a bumped identity;
- the host-forced view change (`Input::AdminForceView`, §14.2): an ordinary fence/evidence/install pipeline driven from the host's say-so, never a state install from it — a general administrative lever, not a boot obligation;
- the host-supplied `u64` event tick on every input, with recovery nonces derived from it (§6.1);
- the higher-view normal-message state-transfer behaviour specified by VRR-2012.

### 14.2 Missing contracts

The current code does not provide:

- a normative C ABI transition-ownership contract — no FFI module exists at present; the C ABI is planned work.

The pre-rewrite `Replica::new` created an empty normal replica in view zero; used after loss of volatile state and fed normal input before recovery, it admitted an amnesiac voter and violated the failure model. That constructor no longer exists. `provision` and `reopen` both start fenced `Restarting` and become normal only after local restoration establishes adequate state, so the amnesiac-voter path is unrepresentable.

## 15. Minimal proposal

1. Retain the complete VRR-2012 two-exchange view-change protocol regardless of whether a host also persists progress.
2. Use `view` only for VRR primary succession and `configuration generation` for membership or quorum-policy changes.
3. Expose a host strategy for `Progress` and a separate host strategy for logical accepted history.
4. Keep the application upcall/completion boundary independent; allow the host to compose it into a wider transaction.
5. Expose one stability-completion point which gates publication and dependent effects.
6. Permit host-selected in-memory strategies for tests and intentionally volatile deployments.
7. Define separate provisioning and reopen/recovery lifecycles.
8. Publish immutable progress for observation; prohibit mutation through diagnostics, transport, timers, or transfer side channels.
9. Keep quorum evidence and transfer state process-local and explicitly scoped.
10. Leave physical storage, retained-history policy, batching, flush cadence, and reclamation entirely to the host.
11. Make quorum policy pluggable by semantic role, validate family intersections rather than counts, and retain strict majority as the initial policy.
12. Support the Unbounded VSR extension specified in §8.7; keep its pivot construction, planned transition, fallback, and crash proof distinct from the ordinary VRR-2012 view-change path.
13. Require a host-supplied event-start `u64` on every input; derive recovery nonces from it and perform no clock reads inside the core.

## 16. References

1. Brian M. Oki and Barbara H. Liskov, [*Viewstamped Replication: A New Primary Copy Method to Support Highly-Available Distributed Systems*](https://www.cs.princeton.edu/courses/archive/fall11/cos518/papers/viewstamped.pdf), ACM PODC, 15–17 August 1988. This is the publication denoted `VSR-1988`; its canonical DOI is [10.1145/62546.62549](https://doi.org/10.1145/62546.62549), and it is reference [10] in VRR-2012.
2. Brian Masao Oki, [*Viewstamped Replication for Highly Available Distributed Systems*](https://publications.csail.mit.edu/lcs/pubs/pdf/MIT-LCS-TR-423.pdf), dissertation supervised by Professor Barbara H. Liskov and submitted to MIT on 20 May 1988; issued as MIT/LCS/TR-423, August 1988. This is the fuller supporting treatment and reference [11] in VRR-2012.
3. Barbara Liskov and James Cowling, [*Viewstamped Replication Revisited*](https://dspace.mit.edu/entities/publication/80846d94-fcd3-40e6-87fb-8d91fe99a5d1), MIT-CSAIL-TR-2012-021, 2012. The disk-free recovery argument and the `StartViewChange` versus stable-view-record alternative are in §§4.3 and 8.2.
4. Heidi Howard, Dahlia Malkhi, and Alexander Spiegelman, [*Flexible quorum intersection revisited*](https://arxiv.org/pdf/1608.06696), 2016.
5. David Turner, *Unbounded pipelining in dynamically reconfigurable clusters*, Tracsis technical report, 2016.
6. Simon Birch, “Unbounded reconfigurations”, 2016.
7. Simon Birch, “Voting weights”, 2017.
8. Simon Birch, “The even-nodes optimisation”, 2016.
9. Simon Birch, “One more frown please! (quorum overlaps)”, 2020.
10. Allen Ling and Simon Birch, [unbounded-reconfiguration progress discussion](https://gist.github.com/allenling/99bf0e965fa7e0b208f461446fcc97e1), GitHub Gist, 2020.

## Amendment A1 — §8.7.3 view-number construction is superseded

See `docs/architecture.md` decision **W1**.

The packed encoding of §8.7.3:

```text
view = (era << k) | index
```

is **superseded**. The normative representation is an explicit pair:

```rust
struct ViewId { era: u32, view: u32 }
```

carried in a 20-byte big-endian header `(tag: u32, era: u32, view: u32, slot: u64)`.
There is no bit packing anywhere in the wire format.

Reasons, in order of weight:

1. `k` is fixed at genesis. A packed field bounds era and primary-selection index
   simultaneously and irrevocably; a cluster that outlives its `k` has no migration path
   that is not itself a reconfiguration protocol.
2. Routing by era must be explicit. Overlap mode (§8.7.3, `era(view) + 1 >= era(slot) >=
   era(view)`) requires a host to dispatch on era. A packed view forces the host to decode
   a protocol number to make a transport decision. Era becomes a first-class header field
   instead.
3. Checked encoding and the wraparound prohibition of §8.7.3 are discharged by
   construction rather than by validation: two independent `u32` fields cannot alias.

Everything else in §8.7.3 stands unchanged: primary selection remains
`config(era).order[index mod len(order)]`, view-number gaps remain legal, and a replica
may still propose a view only if its accepted history contains the committed
reconfiguration operation establishing that view's era. The original reasoning in §8.7.3
is retained above, unedited, because the packing analysis is the reason this amendment
knows what it is giving up.
