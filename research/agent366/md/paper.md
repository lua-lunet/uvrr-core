1

Diskless Viewstamped Replication
with Unbounded Crash-Stop Reincarnation
Simon Massey

Abstract—Small replicated metadata should not require a disk
ﬂush on the critical path. Unbounded Viewstamped Replication
Revisited is a protocol design for embedding strong consistency
with volatile normal-path replication and crash-stop reincarnation. A cleanly stopped process restarts with its identity and
ﬂushed state. A crashed process permanently retires its old
identity and rejoins under a fresh identity, obtaining state and
voting authority through reconﬁguration. Its organising mechanism is leader overlap: where the leader is the sole intersection
of the active decision quorum and the next prepare quorum, it
withholds its own promise so that preparation can proceed while
the active quorum remains usable. Five-voter replacement uses
six committed batches, including exact doubling and halving. A
constructive Rust solver also computes paths between arbitrary
weighted-majority conﬁgurations with an available majority at
each endpoint. The Lean development proves weighted reachability and a phantom-identity casting-vote construction, alongside
agreement and identity exclusion for a three-node schedule and
its ﬁve-voter extension. The addendum published with the code
separates the proof contracts and executable checks.
Index Terms—Crash-stop, leader overlap, reconﬁguration,
strong consistency, Viewstamped Replication.

I. I NTRODUCTION
Oki and Liskov introduced Viewstamped Replication in
1988 [1]. Liskov and Cowling’s 2012 Viewstamped Replication Revisited (VRR) eliminated disk writes during normal processing and view changes [2]. Replication therefore retained
the state needed for agreement without adding a local disk
ﬂush to each update. The remaining difﬁculty was how to
bring a replica back after loss of that state.
Michael et al. exposed a safety failure in VRR’s recovery
mechanism in 2016 [3, Sec. 4.2 and Appendix A.1]. A replica
can send a view-change commitment, crash, and return in an
older view having forgotten the commitment. It can then help
commit an operation that a later leader loses when delayed
view-change messages arrive. Their counterexample breaks
linearizability, including when channels preserve message order. The authors provide a virtual stable storage construction
to support diskless crash recovery. Their result makes the
problem precise: restoring a replica’s data must also preserve
the commitments made by its identity.
Unbounded Viewstamped Replication Revisited (uVRR) repairs this failure by making a crash ﬁnal for the protocol
identity. The crashed identity never resumes execution. The
process that replaces it joins under a fresh identity, receives
state from the cluster, and acquires voting authority through
committed membership changes. Messages sent before the
crash remain messages of the old identity; the new process
Author draft. Correspondence: simon.massey@stenographer.cloud.

cannot add fresh votes to that identity’s history. A cleanly
stopped process can instead restart under its existing identity
because it has preserved its protocol state. Startup fencing
and clean shutdown use durable writes, keeping those writes
outside normal replication. Loss of volatile state therefore
leads to replacement of a member: crash recovery becomes
crash-stop reincarnation.
Turner supplies the construction that connects this lifecycle
to continued replication [9]. Where the leader is the sole
intersection of an active decision quorum and the next prepare
quorum, it can withhold its own promise while collecting the
others. The active quorum remains usable during that preparation; the leader completes the handover with its own local
promise. We use this leader overlap to organise reincarnation.
For three unit-weight voters, replacement takes two committed
batches. For ﬁve, the weighted schedule has six, including
exact doubling and halving. The addendum published with the
code [20] gives the proof ladder, a general conﬁguration-path
constructor and an abstract casting-vote completion using fresh
identities. Turner’s agreement argument supplies the basis for
composing safe adjacent conﬁguration boundaries.
Together, these ideas retain VRR’s diskless normal path
while giving failed members a route back through fresh identities. The intended setting is agreement over small amounts of
metadata embedded in a distributed application [10]. Such a
service can coordinate membership or advisory-lock metadata
without putting synchronous persistence on every update’s
critical path. Its normal replication need not wait for a disk
ﬂush or batch updates to amortise one.
Section II develops the lifecycle and leader-overlap construction. Section III starts with three-voter Crash-StopReincarnation and extends the argument to ﬁve voters, stating
the history invariants needed for agreement, exclusion of the
evicted voter and identity nonreuse. Section VII develops the
metadata applications; the accompanying addendum gives the
mathematical working and veriﬁcation evidence. The name
“unbounded” follows Turner’s treatment of pipelining through
reconﬁguration: agreement does not depend on a ﬁxed bound
on outstanding slots.
II. M ETHOD
A. Processes, state and identity
The agreement model is asynchronous and non-Byzantine.
Messages may be delayed, and an old message retains the
identity that issued it. Each incarnation has a logical identity
distinct from its physical host. The lifecycle must ensure
that an identity never produces new protocol actions after a
replacement incarnation takes over.

RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

2

n1 (primary of v)
commit o

n2
at v (quoru

n3

m {n1 , n

2 })

D OV IEW C HA NG E

n1 crashes:
volatile state
R E Clost
OV E R

v+1 (volatile comm

itment)

Y x (§4

.3: no dis
k record)
R EC OV ERY x

YRESPO

R E C OV E R

N S E v, x

R EC OV ERY R ES PO NS

E v, x

n1 re-enters as n1 , status normal:
the v+1 commitment is forgotten
bad vote: helps fence/commit in v+1
with

stale state

Fig. 1. The diskless-recovery amnesia of VRR, after Michael et al. [3, Sec. 4.2 and Appendix A.1]. A replica sends a view-change commitment, crashes, and
returns in an older view having forgotten the commitment; it can then help commit an operation that a later leader loses when delayed view-change messages
arrive. The counterexample breaks linearizability, including when channels preserve message order. Three voters, f =1; acceptance replies and unrelated trafﬁc
omitted.

A node identity is universally unique and never recycled,
and it is durable before any startup emission. This is the
obligation Paxos places on ballots [5], applied to node identity:
a bump assigns a value that no life of any node has used or
will use. The boot marker carries the identity as the durable
pair {system identiﬁer, counter}, the packing of a unique
per-processor identiﬁer with an increasing counter that Disk
Paxos describes for ballot numbers [6]. The identity write
is amortised into the single small boot-fence write of the
following paragraph, so no ﬂush is added to any critical path.
A clean stop ﬁrst quiesces protocol processing, then ﬂushes
the state required for restart, and ﬁnally records a durable
stopped marker. On startup, a valid stopped marker permits
the process to reload that state and enter Restarting with the
same identity. Before sending protocol messages it durably
records that it is running again. This boot fence prevents a
later crash from being mistaken for another clean restart. A
restarting process retains its promises and accepted state, so
ordinary timeout processing is permitted by the design.
Without a valid clean-stop marker, startup enters Joining
under a fresh identity. The old identity is crash-stopped.
Cached state may reduce transfer work, but it provides no
voting authority. The joiner asks the leader to evict the old
incarnation and admit the new one; the leader can send missing
state while coordinating the change. A joiner does not vote or
initiate a view change. Its authority comes from the committed
conﬁguration, never from the fact that it shares a host with a
former voter.
The implementation design uses replicated superblock markers and majority interpretation of valid marker copies. This
paper abstracts that mechanism as correct clean-stop classiﬁcation and durable identity fencing. The Lean result concerns
the resulting lifecycle; it does not model the storage protocol

itself.
B. Quorums across eras
An era determines the quorum families for a sufﬁx of log
slots. Write QIe for its phase-one family and QII
e for its phasetwo family. The history model requires the following withinera and adjacent-era intersections:
q ∩ r ̸= ∅

I
(q ∈ QII
e , r ∈ Qe ),

(1)

q ∩ r ̸= ∅

(q ∈ QIe ,

(2)

r ∈ QII
e+1 ).

The direction in (2) matters: the earlier phase-one quorum
must meet the later decision quorum. These conditions are
part of Turner’s era-based agreement argument [9].
Our concrete schedule uses the same strict weightedmajority family for both phases. If we (n) is a nonnegative
integer weight on a ﬁnite support, a quorum q satisﬁes
∑
∑
2
we (n) >
we (n).
(3)
n∈q

n

The weighted proof establishes the requisite intersections
when successive conﬁgurations differ by at most one unit of
total absolute weight. Membership records for zero-weight
nodes may change in the same batch without altering (3).
A crash itself does not change any weight; only committed
conﬁguration commands do.
III. T HREE - NODE C RASH -S TOP -R EINCARNATION
Suppose a leader ℓ uses ballot b with an active decision
quorum q. To prepare ballot b′ > b for the next era, it chooses
a prospective phase-one quorum q ′ with
q ∈ QII
e,

q ′ ∈ QIe+1 ,

RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

q ∩ q ′ = {ℓ}.

(4)

3

n1 (primary of v)
commit o

n2
at v (quoru

n3

m {n1 , n

2 })

D OV IEW C HA NG E

v+1 (volatile comm

itment)

n1 crashes: ﬁnal for the
protocol identity — n1 never resumes
process reincarnates as n′1 :
fresh identity, weight 0, non-voting
m / state
ness strea

wit

n′1 caught up at the current era:
still no vote — the quorum read ran
outside the voting protocol
INCREM
committed

y
the standb
transfer to
ered
join gossip answ

′

E N T: n1

1
walks 0 →

n′1 votes for the ﬁrst time —
nothing of n1 ’s history can be re-voted
Fig. 2. Crash-stop reincarnation on the same preﬁx. The crashed identity never resumes; the replacement joins as a weight-0 standby, and its quorum read and
state transfer complete while it cannot vote, so the amnesia’s bad vote is unrepresentable rather than ﬁltered. Promotion to voting weight is itself a committed
membership change. Time runs downwards without a scale; acceptance replies omitted.

This is Turner’s casting-vote condition [9]. The leader ﬁrst and every intermediate era is quorum-safe on its own; a
sends prepare requests only to q ′ \ {ℓ}. Their promises do leader dying mid-sequence leaves only safe eras that a new
not disable an acceptor in q. The leader withholds its own leader can continue. Table I instantiates the sequence at
promise and may continue proposing at b, provided no other unit weight for the three-voter cluster A, B, C with B the
event invalidates the old ballot’s guards.
leader and C the crashed identity that returns as D: the start
After receiving those promises, the leader chooses the sufﬁx conﬁguration is (1, 1, 1, ˘); era 1 drives the leader’s batch
boundary k and makes its own promise locally. Phase one DECREMENT(C) + JOIN(D), reaching (1, 1, 0, 0); era 2
is then complete without another network exchange for the drives INCREMENT(D) + LEAVE(C), reaching (1, 1, ˘, 1).
leader’s response. Proposals at b′ must still obey the normal In the table a – is not a weight: it marks an identity that is
rule for preserving values reported by phase one. Figure 3 not a member of the conﬁguration in that era, whereas 0 is
shows the message order.
the weight of a standby member that the cluster still holds in
Decision quorums follow the era of the slot, not simply its order — which the reincarnated node is between the two
the era associated with a ballot. Consequently, continuing to eras while it is streamed preemptively. The old identity C is
decide later-era slots at b also requires a quorum valid for evicted by reincarnation, never a recovered amnesiac. If the
those slots. A value accepted under a leader that has failed sequence runs in the doubled scale of a weight-doubling plan,
is committed at the same slot by the new leader’s per-slot the return halve is integral only when all weights are even,
install; it is not a new value at the slot and not a violation. In so the corner sequence inserts one extra unit increment on
the concrete pivot below, the active quorum is a majority in the new identity before the halve. The leader streams the ﬁrst
both adjoining eras.
reconﬁguration, then client trafﬁc, and preemptively streams
Table I is the three-node schedule: n0 fails and is replaced to the reincarnated node as a weight-0 standby; messages to
by its fresh identity n3 , while n1 , n2 survive. A join inserts a it are ﬁne and messages from it are discarded by the standard
zero-weight identity; an increment grants its voting weight. A membership check. If the leader crashes, the cluster must reach
leave removes the old identity after its weight reaches zero.
a stable leader before the reincarnated node forces its old
Each boundary preserves cross-era majority intersection identity’s eviction.
by a unit change or exact scaling. Figure 3 illustrates the
Fig. 4 shows this forced sequence as a space-time chart
promotion boundary.
drawn in the grammar of Turner’s message diagrams [9]:
The leader stays live for client requests and drives a forced four lifelines A, B (the leader), C, and D, time ﬂowing
weight sequence from the voting-weights rules [17]: the ex- down; a numbered diagonal vector per step; the bold cross
iting node’s weight goes 1 → 0 (a Decrement; the Leave where C crashes and is fenced, with its dead segment dashed
follows at weight zero), the new identity joins at weight 0 below; the open circle where D’s lifeline begins; the dashed
(a Join), and the new identity goes 0 → 1. Each step is era-1 separator carrying DECREMENT(C) + JOIN(D), with
a unit-weight change, so the strict majorities of consecutive D joining at weight 0 as a standby and receiving the
conﬁgurations overlap by the unit-change overlap argument dashed preemptive stream; the era-2 separator carrying
RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

4

survivor n2

leader ℓ = n1
prepare b ′

replacement n3
; own pro

mise with
held

tb

roposals a

continue p

fo
promises
posals at

further pro

′

rb

b

choose sufﬁx k; promise locally
phase one complete
proposals
at

b′

Fig. 3. Three-voter leader overlap at E1 → E2 : q = {n1 , n2 } and q ′ = {n1 , n3 }. Each side outside the leader is a one-node minority. Original schematic
of Turner’s casting-vote construction [9]. Time runs downwards without a scale; acceptance replies, state transfer and unrelated trafﬁc are omitted.

TABLE I
R EINCARNATION FORCED WEIGHT SEQUENCE .

Era

A

B

C

D

start
era 1: DECREMENT(C) + JOIN(D)
era 2: INCREMENT(D) + LEAVE(C)

1
1
1

1
1
1

1
0
–

–
0
1

INCREMENT(D) + LEAVE(C), with D promoted to weight
1 and C leaving at weight 0; and the forming quorum {A, B}
promised in both eras, so no casting-vote pivot is exercised
on this path. The chart is rendered from the tracked source
reincarnation.svg.
The addendum published with the code [20] provides a
Lean theorem that composes an era-based agreement theorem
with replacement weights and an abstract identity lifecycle.
The work builds upon Turner’s P2–P7 [9]: promises preserve
their fences, reports identify prior acceptances, proposals preserve the selected history, and chosen values have quorum
evidence. A well-founded total ballot order and the ballot/slot
era conditions complete the contract. The addendum states
each hypothesis and the exercise that connects operational
executions to it.
Theorem 1 (Three-voter Crash-Stop-Reincarnation). For a
history using Table I, under the stated order, era and history
conditions, any two values chosen for the same slot are equal.
The ﬁrst batch reduces the failed identity to zero voting weight
and admits the fresh identity at zero; the second grants the
fresh identity unit weight. The retired identity remains without
voting authority when subsequent conﬁgurations preserve its
exclusion. Along the stipulated identity lifecycle, a superseded
identity never becomes current again. In a unit-weight threevoter conﬁguration, any leader has a casting vote between
two equal one-node minorities; at the promotion boundary, the
surviving leader is the singleton overlap shown in Figure 3.
Proof. The totals in Table I are 3, 2, 3, so the majority thresholds are 2, 2, 2. Each boundary changes one weight by one;

Fig. 4. The reincarnation forced sequence.

the integral weighted-overlap argument gives the within-era
and adjacent-era intersections. Turner’s agreement theorem
applies to those intersections and P2–P7. The two commands
establish the stated weights, and identity fencing supplies

RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

5

a message. A node whose durable state is lost returns blank;
a node that reincarnates under a fresh identity holds no roster
entry at all. Neither can guess an era, and the one-era window
drops what it cannot span. The rejoin path closes this with a
gossip layer outside the voting protocol; the addendum [20]
Stage
n0 n1 n2 n3 n4 n5
gives its full construction, its worked scenarios and the kernelInitial
1
1
1
1
1
0
checked safety conditions. Three properties carry the design,
Double
2
2
2
2
2
0
Join, increment
2
2
2
2
2
1
none of which touches a quorum rule.
Decrement old
1
2
2
2
2
1
a) The join gossip: A node outside the cluster never
Decrement, leave
0
2
2
2
2
1
assumes the cluster will come to it. It broadcasts a join
Increment new
0
2
2
2
2
2
request to every node it knows, carrying its frontiers, under
Halve
0
1
1
1
1
1
the deployment’s authenticated transport. Any node that hears
the request answers with the (era, membership) pair it has
nonreuse. At the promotion boundary, {n1 , n2 } and {n1 , n3 } committed locally; era advances only when a committed batch
meet only at leader n1 ; their nonleader members form the two is applied, so no honest response names an era the cluster has
minorities. The leader can collect the replacement’s promise not committed. The joiner adopts the maximum reported era
while withholding its own. The addendum gives the complete with that response’s membership, ﬂushes the pair to durable
three-node working, including the abstract witness at the ﬁrst storage whenever the adoption changes, and keeps its own
retransmission loop: a request lost in ﬂight is re-sent naming
boundary and the state-acquisition obligation.
the current frontiers, and the resend loop is the gossip’s own
reliability rather than a separate discovery path. The joiner
IV. F IVE - NODE C RASH -S TOP -R EINCARNATION
never assumes it has heard the latest era, so it always sends
For ﬁve original voters, n0 is replaced by n5 while to all nodes it knows.
n1 , . . . , n4 survive. Table II gives the complete six-batch
b) The gossip-witness list: Every node that hears a join
weighted schedule, including exact doubling and halving.
request records the sender on an operator visible witness list;
Both schedules are exercised by the Rust tests, and the the leader alone acts on the list. It replays the contiguous sufﬁx
addendum [20] gives the arithmetic for both schedules and of its journal from the advertised frontier, in order, each entry
the general construction of available paths and abstract leader through the ordinary accept path — there is no witness-speciﬁc
overlaps.
accept path — and then streams every subsequent phase-2
With all ﬁve unit voters responding, a leader has the casting and commit to the witness. The witness never votes: quorum
pair {ℓ, a, b} and {ℓ, c, d}. After one voter fails, obtaining membership is decided by the conﬁguration alone. Because
the corresponding responding split requires more care. The the list rides at every node, a successor leader resumes the
mechanical transformations in Table II organise the weighted stream on election without waiting for a re-announcement, so
replacement while preserving the agreement invariant; the failover does not interrupt keep-up. When a committed batch
addendum supplies their safety argument and the distinction makes the witness a member, every node removes the entry;
between abstract witnesses and protocol receipts.
delivery as both voter and witness in the window between
Theorem 2 (Five-voter Crash-Stop-Reincarnation). For a the commit and the removal is covered by deduplication and
history using Table II, under the same order, era and P2– merely wastes IO. A node registered as a witness in the startup
P7 conditions, any two values chosen for the same slot are conﬁguration is never purged: it is an append-only audit tape of
equal. After the fourth batch the old identity has zero weight; everything the cluster committed, and a disaster-recovery sink
after the sixth the replacement and all four survivors have in another region costs exactly the stream that reincarnation
unit weight. Preserving exclusion in subsequent conﬁgurations already needs, under every leader in turn.
c) Why the stream is safe: The stream telescopes a
and enforcing the identity lifecycle retain the old identity’s
promise:
per Lamport, a promise at ballot b covers every
retirement.
lower ballot, so a phase-2 accepted at view v raises the
Proof. Doubling and exact halving preserve the majority fam- witness’s promise ﬂoor to v exactly as if a prepare at v
ily. Each of the four intervening boundaries changes one had arrived ﬁrst. The witness forfeits nothing by raising the
unit of voting mass. The same local intersection argument ﬂoor — its promises are counted in no quorum — and the
therefore applies six times. Induction over these boundaries leader’s right to send at v was earned from members by
establishes the era intersection invariant, and Turner’s theorem the ordinary phase one. The addendum states this as ﬁve
gives agreement. Evaluation of the commands gives the stated construction rules, H1–H5, and proves the consequence in
ﬁnal weights.
Lean: the adopted era never exceeds the committed era; once
the leader answers, it equals it; a journal extended by the inV. R EJOIN GOSSIP AND WITNESSES
order contiguous sufﬁx reconstructs the leader’s preﬁx; the
The replacement schedules of Sections III and IV return a promoted witness’s committed preﬁx is indistinguishable from
failed member to voting weight through committed batches. a member that caught up by ordinary state transfer; and the
They presume the replacement can reach a fence: it must telescoped promise is itself an equivalence over the witness’s
know an era the cluster has committed before its engine sees accept history — promised at the induced ﬂoor, the witness
TABLE II
F IVE - NODE C RASH -S TOP -R EINCARNATION : SIX COMMITTED BATCHES .
Z ERO DENOTES VOTING WEIGHT, INCLUDING IDENTITIES ABSENT FROM
MEMBERSHIP.

RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

6

forbids every later acceptance below it, carries the promise
state of a member that processed the phase-1 at that view, and
promotes with its ﬂoor equal to the leader’s view. A node that
rejoins through this path arrives at its ﬁrst fence at the cluster’s
current era, having seen a commit, which makes the one-era
window gate unreachable by construction rather than widened.
VI. E VIDENCE
The addendum [20] maps these arguments to Lean 4 [21],
bounded exhaustive enumeration, Rust property tests and
protocol tests. It includes the existing specialised machine
theorem’s precise scope, the general lemmas used above,
and the reproduction commands. Conﬁguration reachability,
abstract casting witnesses and the operational history contract
are stated separately so each result can be checked at its own
layer.
VII. D ISCUSSION
A. Applications

described implementation uses a shared network drive for
agreed projections.
e) External coordination services: ZooKeeper and etcd
are established ways to supply coordination separately from
an application. ZooKeeper offers linearizable state changes
and per-client ordering, while local reads have weaker semantics [12]. etcd documents strict serializability for its key-value
operations and distinguishes default linearizable reads from
optional serializable reads [13]. An application must choose
the operation semantics it actually needs.
A separately operated service also brings deployment and
resource costs. For scale, etcd’s version 3.6 hardware guide
gives a small-cluster example with dedicated two-vCPU, 8
GB AWS machines [14]. etcd can itself be embedded in a
Go application [15]. The proposed combination here is small
application-owned state, volatile normal-path replication, and
replacement through fresh identities and overlapping quorums.
B. The host’s obligations

Discoverability, addressability, identity, and durability are
An application of strong consistency is the replication of ﬁrst-class obligations of the host. Each is stated over happenssmall state by consensus: the conﬁguration authority Lamport, before relations of the host’s input and output to the network,
Malkhi and Zhou describe [18], and Vertical Paxos’s auxiliary to the boot fence, and to the durable state: no protocol message
conﬁguration master [19], are the precedents. This design leaves before the boot fence records that the process runs, and
embeds such an authority in the application — for small no announcement names a new identity before that identity is
metadata, with ordinary replication performed in memory.
ﬂushed. A cold stop of all replicas followed by a cold start
a) Short-lived advisory locks and leader leases: A repli- resumes in the same era, because the era is durable in the
cated state machine can order acquisition, renewal and expiry marker. These relations rest on the behaviour of the storage
decisions over a small record containing an owner, generation layer: a single ﬂushed ﬂag is not reliable enough on real ﬁle
and expiry policy. An expiry command for an old generation systems [7], [8], which is why the marker is a quorum of
must not revoke a newer grant. If a client uses ownership to copies rather than a ﬂag.
control an external resource, that resource may also need to
reject stale generations. A real-time leader lease additionally
C. Scope
needs explicit clock and timing assumptions.
a) Implementation correspondence: The theorems asb) A membership authority for another algorithm: An
application whose bulk algorithm uses gossip or another sume protocol-history invariants. Their composition with the
weaker consistency model may still need one agreed version of era-changing operational model is left as an exercise to the
cluster membership, ownership assignments or conﬁguration reader. The addendum [20] makes that exercise veriﬁable by
changes. An embedded replicated state machine can supply stating its mapping, base case and preservation obligations,
that authority, while the bulk algorithm disseminates or con- including clean-stop classiﬁcation, identity fencing and state
transfer before voting.
sumes its decisions.
b) Scope and availability: Tables I and II replace one
c) The conﬁguration as replicated state: In the Paxos
voter
among three and ﬁve respectively. The addendum confamily the conﬁguration itself is replicated consensus state:
structs
paths between arbitrary weighted-majority endpoints
Lamport, Malkhi and Zhou deﬁne a reconﬁgurable state mawith
available
majorities. Agreement follows from the history
chine whose conﬁguration is a state component changed only
contract;
progress
additionally uses available recipients and
by an executed reconﬁguration command [18], and Vertical
a
stable
leader.
Fresh
identities permit repeated replacement
Paxos instead commits each new conﬁguration to an auxiliary
while
each
committed
boundary
preserves the invariant.
conﬁguration master, which determines the set of acceptors of
c)
Meaning
of
strong
consistency:
The mechanised result
every conﬁguration [19]. This design keeps that commitment
is
per-slot
agreement.
A
complete
service-level
linearizabilinside the application: reconﬁguration batches commit in the
ity
argument
also
needs
ordered
execution,
appropriate
read
same slot stream as client values.
semantics
and
a
mapping
between
client
operations
and
log
d) The CORFU precedent: CORFU provides a concrete
entries.
Lease
guarantees
require
the
additional
conditions
precedent for compact conﬁguration metadata alongside a
larger storage system [11]. Its reconﬁguration is patterned stated above.
after Vertical Paxos; Section 2.5 describes storage units acting
VIII. T HE DEMONSTRATION : LUNET- LOCKS
as acceptors in a Paxos-like protocol. Section 2.6 estimates
The demonstration is lunet-locks, a runnable, ﬁxedthat adding 0.1% writes keeps projection metadata below 25
MB for a 1 TB cluster under an adversarial workload. The membership advisory-lock service. It orders lock requests with
RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

7

vrr-core, the sans-IO Viewstamped Replication Revisited
core that accompanies this formalisation: one total function
maps a tick, an input message, and a state to a new state
and the messages to send, and the host owns time, transport,
storage, packetisation, threading, and naming. Clients speak
newline-delimited JSON over TCP; replicas exchange the
core’s binary datagrams over raw UDP; the service process
is LuaJIT and libuv (the Lunet runtime, version-pinned and
hash-veriﬁed), and a small Rust adapter drives the core and
owns the lock state machine.

by this service. What the demonstration implements is the
classic recovery boundary whose amnesia exposure motivates
reincarnation, hardened with durable-nonce fencing: the fresh
nonce is exactly the freshness ingress that the reincarnation
design’s stale-episode rule realises, and a restarting node
can never complete recovery with a stale identity stamp.
Converting the eviction sequence into this service is the next
integration step.
C. The lock and lease API

The client protocol has three operations, one JSON object
per line over TCP. Every request carries a message identiﬁer,
The adapter instantiates the concrete core replica over a stable client identiﬁer, an increasing request number, and a
a segmented log with weighted-majority quorums, running lock identiﬁer; retries reuse the exact envelope, and replication
volatile stability: protocol state lives in quorum memory, an deduplicates by (client, request number), replaying the prior
empty journal is provisioned at boot, and the only bytes the reply. get returns the live lease on a lock, or null. set grants
service ever fsyncs locally are the recovery nonce ﬁle. The or renews a lease — a lease identiﬁer, a holder identiﬁer, and
adapter retains the tick clock (a monotonic value clamped an expiry — and a lease is live only when its expiry is later
against wall-clock regression), exactly-once reply correlation than the execution time; an expired candidate is rejected, an
keyed by the client’s message identiﬁer, and guard checks absent or expired incumbent is free, and a live incumbent may
that re-validate every replicated operation entry — in Prepare, be renewed or replaced only by the same holder. release
DoViewChange, StartView, RecoveryResponse, and NewState removes a live lease only when both holder and lease identiﬁer
messages — against the lock service before the entry reaches match, and is idempotent for a missing or expired lease. Lease
the core. The Teal/Lunet wrapper owns sockets, TCP framing, expiry is what makes the diskless design safe to operate: a
peer source validation, and forwarding; a UDP peer envelope crashed holder’s locks free themselves when the lease lapses,
carrying a membership ﬁngerprint guards the ﬁxed mem- without any recovery path trusting the holder’s lost state.
bership at the transport boundary, so differently conﬁgured
clusters cannot merge. An append-only lock-event journal, a D. Test and hardening status
REST/WebSocket feed, and a console are observability only:
The service is exercised from several directions: a pure-Teal
the journal never participates in replication or recovery, and a
test
suite and an FFI-boundary test suite for the lock service
journal error disables journaling without affecting the service
and
cluster conﬁguration; a Cerulean format gate and Cyan
path.
type checks; the Rust adapter’s checks, tests, and release build,
which depend on the exact pinned vrr-core git revision
B. The rejoin path implemented
fetched over HTTPS; a three-process smoke test covering acEvery boot enters fenced recovery stamped with a fresh quire, read, contention, release, reacquisition, expiry takeover,
durable nonce. The nonce ﬁle is updated atomically — write, and a one-replica restart with a live quorum; and a thirtyfsync, rename, parent-directory sync — and the new value second, three-datacentre lease-failover simulation that kills the
is used as the recovery input’s tick, so per-node monotonic- observed lease holder every three seconds and veriﬁes takeover
ity holds across the two clock sources. Recovery needs a within ﬁve seconds, exiting nonzero on a conﬂicting holder.
live quorum: a restarted node rejoins from quorum memory, The same simulation runs against a stable Docker cluster. CI
installing the latest fenced view’s history and re-applying runs on Linux for regular changes and a full platform matrix
its committed sufﬁx. A rolling single-node restart therefore with packaged releases for tagged versions.
recovers from the surviving members, while a simultaneous
It is a working implementation under active hardening
full-cluster loss forfeits whatever the quorum held, and an and testing, not a ﬁnished demonstration of the reincarnation
operator re-bootstrapping a lost cluster must do so only after protocol.
all outstanding leases can no longer be valid.
A node outside the cluster — a lost durable state or a fresh
IX. E XPERIMENTAL RESULTS : DESIGNS , NOT
identity — rejoins through the gossip path of Section V: the
MEASUREMENTS
join gossip to every known node, the witness list acted on by WARNING: THIS WORK HAS NOT BEEN DONE. The experthe leader, and the streamed keep-up to the ﬁrst fence. The wire
iments below are designs; all numbers and charts are placeholders
carries the gossip as its own message type, and the explicit- (marked “dummy”).
The claim under test is diskless low latency: rejoin through
message tests pin the whole exchange; the section’s safety
argument is the contract those tests hold the implementation reincarnation is faster than paying a forced disk ﬂush at the
reincarnation boundary. The decision rule for the campaign: if
to.
Membership is ﬁxed for one process lifetime, so the reincar- the measured rejoin latency of E1 is slower than the forcednation eviction of Section II — the forced weight sequence that ﬂush baseline of E2 at the same sample count, the diskless
replaces the crashed identity with a new one — is not exercised claim collapses — if rejoining is slower than doing disk
A. Components

RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

standby rejoin (weight 0)
voting again (weight 1)

xxxx

xxxx

rejoin-serving latency (ms)

rejoin-serving latency (ms)

8

diskless default
naive single write
double-ring write

xxxx

dummy

xxxx

xxxx

dummy

xxxx

xxxx

xxxx

xxxx

xxxx

kill iteration 1 . . . k

Fig. 5. E1: node-kill rejoin latency.

xxxx

xxxx

xxxx

xxxx

node restart event

Fig. 6. E2: disk modes at the reincarnation boundary.

ﬂushes, nothing has been achieved. The system under test
for all four designs is the lunet-locks demonstration of
Section VIII. Every number below is a placeholder of the form
X.XX, showing the signiﬁcant ﬁgures and decimal places the
run will report — latencies in milliseconds with two decimals
for the local rig and one decimal at tens of milliseconds, counts
as integers — to be ﬁlled at run time from the lab book, one
ﬁgure per lab-book row.

Naive single write (the latency baseline). One 4 KiB
block write followed by fsync at the reincarnation
boundary. Per the FAST’18 ﬁnding [4], stock consensus
systems lose committed data under a single storage fault
despite fsync; the variant is the latency baseline only,
never a safety claim.
• Double-ring write. A TigerBeetle-style double write at
the reincarnation boundary: one 4 KiB block of data
plus a checksum header to the ﬁrst WAL ring, then the
A. E1: node-kill rejoin latency
header and checksum with no payload to the second WAL
ring, the two rings spaced apart in separate 4 KiB-sectorThe design kills one voting node that is not the leader, roaligned zones so the two checksum copies sit in different
tated across all non-leader voting nodes of the three-datacentre
erasure blocks. Both ﬂush variants carry fake data only,
lunet-locks cluster, for k = XXX kill/rejoin cycles,
with
no read-back: they measure write-and-ﬂush latency
with continuous competing lock trafﬁc at the lease cadence
and
never
reconstruct state from what was written.
throughout. Each iteration begins only after the previous rejoin
has completed serving and a soak interval has elapsed, so Each variant runs E1’s procedure exactly, k = XXX iterations
no measurement is contaminated by the previous one. For per variant, cold counts. The metrics are the kill-to-rejoineach iteration the harness waits for steady state, kills the serving distribution per variant (percentiles as in E1) and,
node’s process — Crash-Stop: volatile state lost, the node for the ﬂush variants, the reincarnation-boundary ﬂush latency
is by construction a different node on reopen — and the itself per iteration. The planned report is the rejoin-latency
iteration ends when the reincarnated node has rejoined as delta: diskless median X.XX ms, naive-ﬂush median XX.X ms,
a weight-0 standby, been walked back to voting weight by double-ring median XX.X ms, with tail ﬁgures XX.X ms at
the leader’s forced sequence, and is again a voting, serving the ninth decile. The delta is the utility test: if reincarnation is
replica. The metric is the kill-to-rejoin-serving latency, one slower than the forced ﬂush, the diskless design has achieved
sample per iteration: p50 X.XX ms, p90 X.XX ms, p99 nothing. Fig. 6 shows the designed chart: node restart event
XX.X ms, max XX.X ms, mean X.XX ms, and the count of on the horizontal axis, rejoin-serving latency on the vertical
eras and view changes consumed per rejoin, X. Fig. 5 shows axis, one series per build (diskless default, naive single write,
the designed chart: kill iteration on the horizontal axis, rejoin- double-ring write). No data points exist yet; the chart is
serving latency on the vertical axis, one series per milestone of watermarked “dummy”.
the forced sequence (standby rejoin at weight 0, voting again
at weight 1). No data points exist yet; the chart is watermarked C. E3: Maelstrom brute force at three and ﬁve nodes
“dummy”.
Does reincarnation hold consistency under brute-force fault
•

injection? Jepsen’s Maelstrom drives the cluster under its fault
B. E2: disk modes at the reincarnation boundary
proﬁle with reincarnation as the failure model: node crashes
What does the diskless default save, in latency, against with volatile-state loss — the crashed node reincarnates rather
paying a forced ﬂush at the reincarnation boundary? The de- than crash-recovers — plus partitions, with a Maelstrom
sign builds custom lunet-locks binaries, identical in every adapter fronting the lock service as a compare-and-set register
respect other than the durability behaviour at the reincarnation and competing lock clients at the lease cadence. The design
boundary — the point at which a reincarnating node durably runs the fault proﬁle at three nodes, then at ﬁve, XXX runs
records state before rejoining serving:
per cluster size. Three checks decide each run: Maelstrom’s
• Diskless (the default). No durable writes at the reincarna- linearizability check passes over the register — no committed
tion boundary; quorum memory is the only protocol state, operation is lost, and no node rejoins under its old identity to
and the recovery nonce ﬁle remains as shipped.
overwrite a newer decision; every killed node reincarnates and
RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

9

latency (ms)

suspicion ﬂoor (phi)

uVRR (diskless)
local cluster
cloud machines, three regions

xxxx

dummy

forced ﬂush (4 KiB + fsync)

ﬂoor +2 × RTT: X.XX ms

dummy

forced-ﬂush arm
xxxx

2× RTT (SVC + SV)

ﬂoor + ﬂush +2 × RTT + ﬂush: XX.X m

latency (ms)

xxxx

experiment (E1 rejoin, E2 ﬂush delta)
xxxx

xxxx

xxxx

Fig. 8. E5: the failover budget. Stacked segments per arm; the forced-ﬂush
arm prices the classic diskful barrier on identical hardware in the same run,
via the custom test build’s out-of-algorithm ﬂush.

Fig. 7. E4: cloud repetition of E1 and E2.

before the new primary answers S TART V IEW — so the rig
prices the diskful barrier without any uVRR code being made
rejoins serving, with no run ending in a permanently lost node; unsafe or rewritten to resemble the compared systems. The
and no read ever reports two holders, expiry being judged only reported numbers are the per-segment medians and the totals:
in the leader’s clock. All ﬁgures X.XX (medians, milliseconds) suspicion ﬂoor X.XX ms; per-RTT X.XX ms; total failover,
and XXX (run counts). A passing campaign supports the case ours, X.XX ms; total failover, forced-ﬂush arm, XX.X ms;
that reincarnation eliminates the diskless-recovery weakness ﬂush cost per boundary X.XX ms, expressed also in units of
[3]: consistency under crash with successful rejoin, by con- the same-run RTT, X.X RTTs. The last ratio is the hardwarestruction rather than by recovery.
agnostic headline: it should survive moving between loopback,
local virtual machines, and cloud machines, where the absolute
latencies will not. Fig. 8 shows the designed stacked budget
D. E4: cloud repetition
chart, one bar per arm; no data points exist yet; the chart is
E1 and E2 are repeated on commodity cloud virtual mawatermarked “dummy”.
chines, the six-node cluster laid out over three regions, two
machines each — the correlated-infrastructure setting that
weighted conﬁgurations address. Variables, procedure, metrics, F. E6: suspicion calibration under jitter — what phi actually
and sample counts are E1’s and E2’s unchanged; only the saw
environment differs. The planned report records XXX kills
The campaign owes an explanation of the detection ﬂoor,
per conﬁguration and zone-crossing medians of XX.X ms (E1) because the ﬁrst rig runs showed the phi accumulator ﬁring
and XX.X ms (E2), with XX availability incidents. Fig. 7 early: its estimate sat far below the true round-trip time, and
shows the designed chart: experiment on the horizontal axis under reconﬁguration jitter its variance exploded. The runtime
(E1 rejoin, E2 ﬂush delta), latency on the vertical axis, one is LuaJIT plus FFI driving a Rust core with small messages,
series per deployment (local cluster, cloud machines). No data so the noise budget is genuinely small; the open question is
points exist yet; the chart is watermarked “dummy”.
whether the residual variance is garbage collection, scheduler
jitter, or the estimator’s window shape, and E6 decides it
by attribution rather than by guess. The rig samples, per
E. E5: the failover budget — N round trips, no disk ﬂush
heartbeat, the measured RTT, the phi estimate, the estimator
The design claim is that view failover costs N network
window’s mean and variance, the LuaJIT allocation counter
round trips and no forced disk ﬂush, where the classic
(collectgarbage("count")) before and after each windiskful alternatives pay at least one ﬂush on the fence path.
dow, and the Rust-side tick gap distribution; reconﬁguration
The budget is measured, not assumed: the view change
epochs are marked on the same axis. The designed report is the
is decomposed into its wire segments and each segment
phi estimate against the measured RTT distribution with GCtimed. For the three-node cluster the segments are: suspiattributed pauses marked: median estimate X.XX ms against
cion (the phi-accrued suspect interval, ﬂoored by the host’s
median RTT X.XX ms, ninth-decile estimate XX.X ms against
election knob), S TART V IEW C HANGE dissemination (1 RTT),
ninth-decile RTT XX.X ms, GC-attributed share of outlier winD OV IEW C HANGE collection at the next view’s primary
dows XX.X percent, scheduler-attributed share XX.X percent,
(pipelined with dissemination: 0 extra RTT when the messages
unexplained share XX.X percent. The decision rule: if the
cross), and S TART V IEW installation (1 RTT): the designed
GC share explains the outlier windows, the estimator gets a
expectation is failover = suspicion ﬂoor + 2 RTT + 0 ﬂushes.
variance-capped window and a stated ﬂoor; if scheduler jitter
The counterfactual arm pays the same suspicion ﬂoor and
dominates, the ﬂoor moves to the measured jitter percentile
round trips plus a forced ﬂush before fencing (the VSR-1988and the paper says which. Fig. 9 shows the designed chart; no
class stable-storage barrier) and a further ﬂush before the new
data points exist yet; the chart is watermarked “dummy”.
primary may serve installed state: suspicion ﬂoor + ﬂush +
2 RTT + ﬂush. Both arms are measured on the same hardware,
the same run. The counterfactual is built by the forced-write rig G. Cross-checks: the ﬁgures must be self-consistent
of the campaign plan: a custom test build of the demonstration
Every placeholder above carries an implicit order of magperforms one 4 KiB write and fsync outside the algorithm — nitude, and the campaign checks the consistency before any
before the fenced node emits its view-change trafﬁc and again number is quoted. The rules the lab book enforces: no
RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

latency (ms)

10

xxxx

xxxx

measured RTT
phi estimate
GC-attributed outlier

dummy

replacement instantiation from the protocol’s history invariants.
The application discussion places this design alongside coordination services and CORFU, using their stated semantics and
resource context.
R EFERENCES

xxxx

[1] B. M. Oki and B. H. Liskov, “Viewstamped replication: A new primary
copy method to support highly-available distributed systems,” in ACM
heartbeat window
PODC, 1988, pp. 8–17.
xxxx
xxxx
xxxx
xxxx
[2] B. Liskov and J. Cowling, “Viewstamped replication revisited,” MIT
CSAIL, Tech. Rep. MIT-CSAIL-TR-2012-021, 2012. [Online]. AvailFig. 9. E6: phi estimate against measured RTT, GC-attributed outliers marked.
able: https://hdl.handle.net/1721.1/71763
[3] E. Michael, D. R. K. Ports, N. Kr. Sharma, and A. Szekeres, “Providing
stable storage for the diskless crash-recovery failure model,” University
ﬁgure
rig
lower bound
check
of Washington, Tech. Rep. UW-CSE-16-08-02, Aug. 25, 2016. [Online].
Available: https://syslab.cs.washington.edu/papers/diskless-tr16.pdf
E1 rejoin p50
local
X.XX ms (2 RTT + ﬂoor) ≥ bound
[4] R. Alagappan, A. Ganesan, E. Lee, A. Albarghouthi, V. Chidambaram,
E1 rejoin p50
cloud XX.X ms (2 RTT + ﬂoor) ≥ bound
A. C. Arpaci-Dusseau, and R. H. Arpaci-Dusseau, “Protocol-aware
E2 ﬂush delta
local
X.XX ms (device fsync)
≥ bound
recovery for consensus-based storage,” in USENIX FAST, 2018. [OnE2 ﬂush delta
cloud XX.X ms (device fsync)
≥ bound
line]. Available: https://www.usenix.org/system/ﬁles/conference/fast18/
E5 failover total local
X.XX ms
= ﬂoor +2 RTT fast18-alagappan.pdf
E5 failover total cloud XX.X ms
= ﬂoor +2 RTT
[5] L. Lamport, “Paxos made simple,” ACM SIGACT News, vol. 32, no. 4,
E6 phi median
local
X.XX ms (RTT median)
≈ RTT
pp. 51–58, 2001.
[6] E. Gafni and L. Lamport, “Disk Paxos,” Distributed Computing, vol. 16,
TABLE III
no. 1, pp. 1–20, 2003. [Online]. Available: https://lamport.azurewebsites.
C ONSISTENCY CHECKS : EACH FIGURE ’ S RIG , ITS PHYSICAL LOWER
net/pubs/disk-paxos-disc.pdf
BOUND , AND THE ARITHMETIC IDENTITY IT MUST SATISFY. X.XX MARKS
[7] T. S. Pillai, V. Chidambaram, R. Alagappan, A. C. Arpaci-Dusseau,
THE PLACEHOLDER SHOWING THE SIGNIFICANT FIGURES THE RUN WILL
and R. H. Arpaci-Dusseau, “All ﬁle systems are not created equal: On
REPORT; NO FIGURE ENTERS THE PAPER WITHOUT ITS ROW COMPLETED
the complexity of crafting crash consistent applications,” in USENIX
IN THE LAB BOOK .
OSDI, 2014. [Online]. Available: https://www.usenix.org/system/ﬁles/
conference/osdi14/osdi14-paper-pillai.pdf
[8] V. Chidambaram, T. S. Pillai, A. C. Arpaci-Dusseau, and R.
H. Arpaci-Dusseau, “Optimistic crash consistency: Application-aware
loopback-harness ﬁgure may be quoted beside a cloud claim
fully-extensible crash consistency,” in ACM SOSP, 2013, doi:
(loopback round trips are tens of microseconds; intra-zone
10.1145/2517349.2522726.
[9] D. Turner, “Unbounded pipelining in dynamically reconﬁgurable Paxos
cloud round trips are not); every failover ﬁgure must be at
clusters,” revision 1A9DBA37, Aug. 14, 2017. [Online]. Available:
least the suspicion ﬂoor plus two measured round trips of the
https://github.com/DaveCTurner/paxos-membership. The proof artifact
same rig — a failover quoted an order of magnitude faster
includes the consulted PDF and its digest.
than that rig’s RTT is a measurement error, not a result; every [10] S. Massey, “Viewstamped replication revisited,” Aug. 12, 2026.
[Online]. Available: https://simbo1905.wordpress.com/2026/08/12/
ﬂush ﬁgure must be at least the device class’s write latency —
viewstamped-replication-revisited/
a ﬂush quoted an order of magnitude below the block device’s [11] M. Balakrishnan, D. Malkhi, J. D. Davis, V. Prabhakaran, M. Wei, and
T. Wobber, “CORFU: A Distributed Shared Log,” ACM Trans. Comput.
measured fsync is the same class of error; and ratios may
Syst., vol. 31, no. 4, Art. 10, Dec. 2013, doi: 10.1145/2535930.
only be formed within one rig and one run. Table III lists the [12] P. Hunt, M. Konar, F. P. Junqueira, and B. Reed, “ZooKeeper:
checks; the lab-book template carries one row per ﬁgure and
Wait-free coordination for Internet-scale systems,” in USENIX ATC,
2010. [Online]. Available: https://www.usenix.org/events/usenix10/tech/
no ﬁgure may enter the paper with its row blank.
full_papers/Hunt.pdf
[13] etcd authors, “etcd API guarantees,” documentation v3.6, accessed
Sep. 11, 2026. [Online]. Available: https://etcd.io/docs/v3.6/learning/
X. R ELATED W ORK
api_guarantees/
Viewstamped Replication introduced the organisation of [14] etcd authors, “Hardware recommendations,” documentation v3.6, accessed Sep. 11, 2026. [Online]. Available: https://etcd.io/docs/v3.6/
replication around views and a primary [1]; VRR revisits that
op-guide/hardware/
organisation and separates normal operation, view change, re[15] etcd authors, “Embedding etcd in a Go application,” documentation v3.6,
covery and reconﬁguration [2]. Michael et al. identify the failaccessed Sep. 11, 2026. [Online]. Available: https://etcd.io/docs/v3.6/
dev-guide/golang_embed_pkg/
ure of diskless recovery to preserve view-change commitments
and construct virtual stable storage [3]. uVRR takes a different [16] simbo1905, “The network is faster than the disk,” 12 April 2024,
accessed 7 September 2026. https://simbo1905.wordpress.com/2024/04/
route: it makes a crash ﬁnal for the old identity, then brings
12/the-network-is-faster-than-the-disk/
the process back as a new member through reconﬁguration. [17] simbo1905, “Paxos Voting Weights,” 16 March 2017, accessed
7 September 2026. https://simbo1905.wordpress.com/2017/03/16/
The replacement schedule connects that lifecycle to agreement
paxos-voting-weights/
across conﬁgurations.
[18] L. Lamport, D. Malkhi, and L. Zhou, “Reconﬁguring a state machine,” ACM SIGACT News, vol. 41, no. 1, pp. 63–73, 2010, doi:
Turner’s Unbounded Pipelining in Dynamically Reconﬁg10.1145/1753171.1753191.
urable Paxos Clusters supplies the central quorum geome[19] L. Lamport, D. Malkhi, and L. Zhou, “Vertical Paxos and
try, era-based agreement argument and casting-vote construcprimary-backup replication,” in Proc. 28th ACM Symp. Principles of Distributed Computing (PODC), 2009, pp. 312–313, doi:
tion [9]. Withholding the leader’s own promise is Turner’s
10.1145/1582716.1582783.
mechanism. The present work applies it to reincarnation,
[20] S. Massey, “uVRR code and proof addendum,” 2026. [Online]. Available:
proves weighted conﬁguration reachability and an abstract
https://github.com/lua-lunet/uvrr-core

casting-vote completion, and proves agreement for a concrete
RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

11

[21] L. de Moura and S. Ullrich, “The Lean 4 theorem prover and programming language,” in Automated Deduction – CADE 28, 2021, pp. 625–
635, doi: 10.1007/978-3-030-79876-5_37.

RESEARCH CHECKPOINT — 28 SEPTEMBER 2026 — 20260928-f884ac7-draft

