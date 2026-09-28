1

Diskless Viewstamped Replication with Unbounded
Crash-Stop Reincarnation
— Addendum: proof ladder, weighted
reconﬁguration and the operator solver
Simon Massey

Abstract—This document is an addendum to Diskless Viewstamped Replication with Unbounded Crash-Stop Reincarnation;
the parent paper’s abstract and keywords are not repeated here.
The addendum contributes what the paper does not contain: the
fuse transport with its telescoping theorem and the Lamport
induction it instantiates; the phantom-slot construction for phase
one of a ﬁve-node reincarnation; and the notational appendices
for weighted reconﬁguration, the casting-vote completion with
fresh identities, and the operator solver. The parent paper
supplies the method, the ﬁve-voter schedule and the agreement
result; nothing here modiﬁes them. A companion addendum,
Experimental results, reports the measured ﬁndings.

I. T HREE VOTERS : BOTH TRANSITIONS , THEN F USE
This proof ladder starts with the smallest replacement. Let
the identities be (o, ℓ, a, r): the retired process, surviving
leader, other survivor and fresh replacement. The complete
two-transition schedule is
(1, 1, 1, 0) −→ (0, 1, 1, 0) −→ (0, 1, 1, 1).
The commands that install a conﬁguration are decided under
the conﬁguration governing their slots. For clarity, the calculations below check every row, including the ﬁnal row governing
subsequent commands; this also covers either side of each
boundary.
A. The base case and the second transition
∑
∑For w : N → N, write T (w) = n∈N w(n) and Mw (R) =
n∈R w(n). A strict majority satisﬁes 2Mw (R) > T (w). At
the ﬁrst boundary any old quorum contains two of o, ℓ, a, while
every intermediate quorum contains both ℓ, a. Hence they meet.
At the second boundary every intermediate quorum contains
ℓ, a, and any ﬁnal quorum contains two of ℓ, a, r. Hence they
meet too. This proves the cross-era intersection obligation for
both transitions, not just the ﬁnal one.
For the response set R = {ℓ, a}, the masses are 2, 2, 2
and the totals are 3, 2, 3. Thus exactly the same two responding identities are a majority throughout. Adding the replacement’s acknowledgement cannot invalidate that fact. Abstract
singleton-intersection witnesses are {o, ℓ}∩{ℓ, a} = {ℓ} at the
ﬁrst boundary and {ℓ, a} ∩ {ℓ, r} = {ℓ} at the second. These
geometric witnesses and the responding quorum serve different
purposes: the fused decision certiﬁcate uses {ℓ, a} and does
Author draft. Correspondence: simon.massey@stenographer.cloud.

not ask the retired identity to reply. The existing module
ReincarnationThree.lean records both intersections
and both casting witnesses.
B. What a fused acknowledgement certiﬁes
Fuse carries a shared header, a count, and the ordered
commands for consecutive slots. FuseOk carries the individual
slot acknowledgements; CommitBatch carries the individual
commits. None of these needs a range encoding. Logical slots,
conﬁguration boundaries and their quorum families remain
distinct.
Let F = (b, s, [c0 , . . . , ck−1 ]). An acknowledgement of F
certiﬁes that the sender accepted every speciﬁed command at
ballot b and its speciﬁed slot. The leader matches the ballot,
ﬁrst slot, count and slot contents to its pending envelope,
counts each identity once, and checks its own eligibility when
adding its own acknowledgement. Let R be that set of acknowledgers. Then one collection of replies gives acceptance
evidence at every packed slot. It gives decision evidence at
slot i precisely when R ∈ QII
i .
Theorem 1 (Fuse telescoping). Suppose the certiﬁed schedule
preserves the invariant R ∈ QII
i at each step, the ﬁrst slot has
R ∈ QII
,
and
every
member
of
R acknowledges all the packed
0
slots. Every packed slot has a phase-two decision certiﬁcate.
If the expanded history satisﬁes Turner’s protocol invariants,
the ordinary era-based agreement theorem applies to each
certiﬁcate.
Proof. Induct on the slot index. The base is the ﬁrst quorum.
II
Preservation gives R ∈ QII
i+1 from R ∈ Qi . The fused
acknowledgements supply every acceptance required by that
quorum at slot i + 1. Thus every slot has both a quorum and
its acceptance evidence. Expanding the envelope into those
logical events leaves the evidence unchanged; the existing
agreement theorem supplies uniqueness under its history assumptions. In-order application then advances the committed
preﬁx through the schedule.
For three voters, the masses above discharge preservation
for both steps. The leader can collect one FuseOk from
a, complete its own all-slot acceptance, and issue the two
commits in one CommitBatch. This is one successful request–
reply round followed by commit notiﬁcation, provided the
envelope’s acceptance preconditions are already established.

RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

2

II. F IVE VOTERS AND ARBITRARY SOLVER SCHEDULES
A. The ﬁrst and second transitions
For the complete six-transition weighted replacement, let
R contain any three surviving original voters and exclude o.
The initial total is 5 and M (R) = 3. Doubling gives total
10 and mass 6: the ﬁrst boundary preserves every quorum
exactly. Introducing the replacement’s unit vote gives total 11
and mass at least 6. The strict inequality 12 > 11 proves the
second boundary’s responding-quorum condition explicitly.
The remaining rows have totals 10, 9, 10, 5 and response masses at least 6, 6, 6, 3. Each inequality stays
strict. These numbers cover the full schedule with doubling and halving, not the shorter ﬁve-voter specialisation
in ReincarnationSafety.lean. Any larger initial responding quorum excluding the retired identity contains such
a triple and therefore also qualiﬁes. Theorem 1 now telescopes
the same replies across all six transitions. The ordinary crossera intersections follow from exact scaling at the ends and
the four unit changes between them. There is no need to list
another ﬁve-voter replacement table here.
B. The solver’s induction invariant
For arbitrary endpoints, use a response set R that is a
strict majority at both endpoints. Deﬁne its margin µw (R) =
2Mw (R) − T (w). The constructive path ﬁrst decreases coordinates outside R, then increases coordinates inside R, moving
only towards their targets. These edits increase the margin. It
then decreases coordinates inside R and increases coordinates
outside R towards their targets. Throughout these latter phases
the margin is at least the strictly positive ﬁnal margin. Thus
µw (R) > 0 everywhere. Each unit edit also preserves crossera intersection; exact scaling preserves quorum families. Zeroweight membership edits have no effect on either proof.
This supplies a useful runtime certiﬁcate: compute the path
with R as the availability set, certify the preserved margin,
and use the same fused replies at every slot. The existing
operator solver and its general reachability proof provide the
construction; a Fuse integration must retain its response-set
certiﬁcate. Merely knowing that some available majority exists
in each row does not establish that every subset which replied
ﬁrst qualiﬁes in all rows. If only a larger available set L was
certiﬁed, the leader can use per-slot quorum accounting on the
packed acknowledgements, or wait for a response set which
satisﬁes the common certiﬁcate. Neither requires another wire
round when those responses arrive in the original round.
The necessity is visible in the proposed equal-total swap
(1, 2, 1, 2) → (2, 1, 2, 1). The response set {B, D} has mass 4
initially and 2 ﬁnally, with total 6 at both endpoints. The same
replies cannot be counted as a ﬁnal majority. Even expanding
the swap into legal unit edits does not alter that endpoint
calculation. This is a negative control for the certiﬁcate, not an
obstacle to solving the reconﬁguration using a suitable common response set or successive certiﬁcates. Phantom identities
remain available for the overlap construction in Appendix B;
an identity which sends no acknowledgement contributes no
acceptance evidence.

III. T HE ATOMICITY AND AGREEMENT OBLIGATIONS
The useful state-space reduction is local: one receiver either
refuses the envelope or acknowledges its entire accepted
sequence. It removes network loss between the commands
inside that envelope. Different recipients may still receive
different envelopes, and a leader may change while replies
or commits are in ﬂight. The existing agreement proof covers
such histories when its invariants hold.
An indivisible datagram does not itself make execution of
the receiver’s handler indivisible. The reﬁnement obligation is
therefore to validate the whole fold before exposing its effects,
acknowledge only after all accepts are recorded, and ensure
that interruption before acknowledgement leaves a state admitted by the ordinary clean-start or crash-stop-reincarnation
model. Checksums establish neither a transaction over memory
nor a journal transaction. Under an atomic handler model,
expansion is a ﬁnite sequence of ordinary accept actions
with no intervening local action. Under a ﬁner model, its
unacknowledged preﬁxes must also reﬁne ordinary accept
histories.
The induction for local acceptance needs its own invariant:
the next slot is consecutive; every command is legal in the
accumulated conﬁguration; the promise still permits its ballot;
and the slot’s era permits that proposal. A certiﬁed sequence
plus preservation of these guards proves that accepting the ﬁrst
permits accepting the rest. A shared header can hoist repeated
ﬁelds out of the wire encoding; it cannot discharge a changing
era guard. In particular, Turner’s multi-era argument includes
phase-one and value selection obligations. A Phase2-only
envelope must inherit those obligations from established state
or prove the corresponding reﬁnement; the quorum telescoping
theorem alone does not perform phase one.
Concretely, the current ordinary accept path in src/
replica/normal.rs requires the entry era to equal the
ballot era or its successor. If a shared ballot has era e and
expanded entries are assigned eras e, e + 1, . . . , e + k against
that one ballot, this guard implies k ≤ 1; Fuse.same_
ballot_era_guard checks that arithmetic in Lean. That
is the degenerate reading, not the envelope’s. Under the atomic
handler model above, the fold evaluates each payload against
the in-memory conﬁguration the preceding payloads established: payload i+1’s era guard is judged in the era payload
i’s commit folded, exactly as though the payloads had arrived
as separate datagrams with no intervening network. The entry
eras are therefore per-payload facts of the fold, discharged
in order, and the era the header carries disciplines the ﬁrst
payload alone. Phase-one authority for every packed slot is
the header ballot itself: Lamport’s leader executes phase one
once for all future instances at the same proposal number [1],
which is the induction the telescoping theorem records.
The proof dependencies are consequently:
legal schedule + preserved response quorum
+ all-slot acceptance certiﬁcate
=⇒ decision certiﬁcates for all slots;
certiﬁcates + era history invariants
=⇒ agreement under reconﬁguration.

RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

3

This is the precise sense in which proving a base case and a
preservation rule replaces a separate proof for every schedule
length. It does not assume the inductive step merely because
the ﬁrst case succeeds.

resurrection of the crashed identity into the phantom slot, with
the quorum arithmetic stated at each step. The later phases —
the forced weight sequence that grants the fresh identity its
vote and retires the old one — are named where they arise
and are not written here.

A. Prior art and executable evidence

Deﬁnition 1 (Phantom slot). A phantom slot is a member
Lamport states the same induction as “A newly chosen record of the committed conﬁguration held at voting weight
leader executes phase 1 for inﬁnitely many instances of the zero: a preallocated future slot for a resurrection. A phantom
consensus algorithm . . . Using the same proposal number for contributes no mass to any quorum; messages from it are
all instances” [1], where the ballot b of a standard message discarded by the membership check, while messages to it are
is deﬁned to apply to all future slots until the leader is delivered.1
interrupted. The fuse telescoping theorem is the same inductive
Let N = {n1 , . . . , n6 } carry the committed weights
logic in that the fuse header ballot number is both a Phase 1 (1, 1, 1, 1, 1, 0): ﬁve unit voters n , . . . , n and n a phantom
1
5
6
and Phase 2 for all commands in the fused batch. As the slot, preallocated so that a resurrection ﬁnds its record ready,
messages are processed as a slab they cannot be interrupted. and drawn from the start so that the ﬁgures can name it.
Turner supplies the era-history conditions and the weighted The total is T = 5 and the old view’s quorum family is the
reconﬁguration setting [5]. The present telescoping argument strict-majority family of the weights: R qualiﬁes exactly when
combines those obligations with a common response set; it 2M (R) > 5, that is when M (R) ≥ 3 — a majority of ﬁve
w
w
does not attribute a datagram-atomicity theorem to either is three. The phantom’s zero weight decides no membership:
source.
Mw (R ∪ {n6 }) = Mw (R) for every R.
The standard-library Python checker research/
The leader is n3 , and it declares its overlap as in Figure 4:
weighted-reachability/check_fuse.py enumer- an active decision quorum q = {n , n , n } of the old view
1
2
3
ates all identity subsets, including zero-weight identities. and a prospective phase-one quorum q ′ = {n , n , n } for the
3
4
5
It checks both three-voter boundaries (64 qualifying cross- next, meeting the casting-vote condition q ∩ q ′ = {n } of (4).
3
quorum pairs), all six ﬁve-voter boundaries (5,760 pairs), While it collects the promises of q ′ \ {n }, the leader services
3
and every initial quorum excluding the retired identity the current view: its beacon proposal and its own vote are
(respectively 2 and 10 sets, counting inclusion of the carried forward across the preparation.2
replacement). It also checks 29,690 ordered endpoint/responseThen n5 crashes. A crash changes no weight: only commitset combinations for all four identity proﬁles in {0, 1, 2}4 , ted conﬁguration commands do. The committed conﬁguration
covering 99,352 unit edges of the monotone construction. The stays (1, 1, 1, 1, 1, 0), so the old view’s quorum family is
equal-total swap is a required negative control. Its output is unchanged by the crash; what changes is availability. Four
research/weighted-reachability/generated/
unit voters survive against one unavailable: the available mass
fuse-summary.json.
is A = 4, the unavailable mass is D = 1, and A > D.
These are exhaustive statements about their ﬁnite domains.
The crashed process restarts dirty and bumps its own
The inductive argument supplies arbitrary ﬁnite schedule identity: it reincarnates as n = n +1, the own-id-increments
6
5
length. Lean checks the formal implication and its stated hy- rule of the reincarnation wire (Reincarnation.bump in
potheses; property tests check the implementation of packing, the Lean development), and announces the pair (n , n ) to
5
6
validation and commit application. TLA+ can model envelope all; only the leader responds. The fresh identity lands in
delivery as one receiver action after the reﬁnement obligation the preallocated phantom slot — which is why the slot was
is discharged. Maelstrom remains useful for implementation preallocated — and lights it up as a live standby of weight
faults involving replies, client histories, recovery and network- zero. Until a committed membership change assigns it weight,
ing. None of these tools needs to re-prove a ﬁnite arithmetic n votes nowhere.
6
domain already exhausted by an independent checker, but each
Proposition 1 (Phase-one quorum arithmetic). Throughout
must test the layer for which it supplies evidence.
phase
one the conﬁgured total stays T = 5 and the maTo replay the arithmetic from the repository root, run
jority
threshold
stays 3: the crash removes availability, not
python3 on research/weighted-reachability/
weight,
and
the
phantom
adds zero. Explicitly: the old view’s
check_fuse.py. The Lean module is UVRR/Fuse.lean;
quorum
family
is
{R
⊆
N : 2Mw (R) > 5} computed on
its proof generation and kernel-check evidence are recorded
w
=
(1,
1,
1,
1,
1,
0);
the
live
mass is 5 − 1 + 0 = 4, a majority
beside the proof ladder.
of ﬁve minus the crashed vote plus the phantom’s zero weight,
and 2 · 4 > 5; any three of the four survivors form a quorum,
IV. A PHANTOM SLOT: PHASE ONE OF A FIVE - NODE
since 2 · 3 = 6 > 5; and the declared overlap pair still meets
REINCARNATION
in exactly {n3 }.
The telescoping certiﬁcate of Section I is stated against quo1 The standby of the reincarnation design: zero voting weight, streamed the
rum families, not against the lifecycle that changes them. This
leader’s
trafﬁc, and voting only after a committed increment grants it weight.
section records the ﬁrst phase of a ﬁve-node reincarnation in
2“Beacon proposal” is the operator’s term for the leader’s continued
uVRR: the initial conﬁguration with its preallocated phantom proposal trafﬁc at the current ballot while its own promise is withheld; the
slot, the serving leader’s declared overlap, the crash, and the declared overlap is that proposal together with the leader’s vote.
RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

4

Proof. Each clause is arithmetic on the displayed w. The
family clause restates the strict-majority deﬁnition; a crash
edits no weight, so the family is unaffected. The phantom
clause is w(n6 ) = 0. The live-mass clause sums the four
surviving unit weights; the strict inequality 2M > T holds
at M = 4 and at M = 3. The overlap clause is set
intersection: {n1 , n2 , n3 } ∩ {n3 , n4 , n5 } = {n3 }. Any other
prepare quorum drawn from the survivors still meets every
old decision quorum, because two subsets each of mass at
least 3 out of a total of 5 must share a member.

the phantom-slot joiner, because no message can now arrive
from an era it has not already adopted or cannot reach by the
ordinary guards.

B. The witness list
The join request names the joiner’s frontiers (blank for a
reincarnated node). Every node that hears the request records
the sender as a witness — present on an operator-visible list,
absent from the voting conﬁguration. The leader alone acts on
the list:
1) replays the contiguous sufﬁx of its journal from the
Remark 1 (The later phases, named). Phase two is the
advertised frontier, in order, each entry subject to the orleader’s forced weight sequence over the six-slot membership
dinary accept path’s ﬁrst-slot discipline; then the commit
of Table I — the doubling, the join and increment that grant n6
frontier, which never exceeds the highest replayed slot
its vote, the decrements that retire n5 , and the halve back to
(commit information rides the stream exactly as VRRunit weights — with the leader’s memo-stream keeping the
2012 §4.1 step 6 rides it on P REPARE);
standby current throughout. Phase three is the completion:
2) copies the witness on all subsequent phase-2 trafﬁc and
n5 leaves the membership and the cluster returns to ﬁve unit
commits. The ﬁrst commit the witness folds starts its
voters, now with n6 among them. Neither is written here.
leader-timeout; it behaves as a passive sink that happens
to be precisely up to date;
V. W ITNESS ACQUISITION : THE JOINER LEARNS THE ERA
3) every node removes the witness entry when a committed
BEFORE IT PARTICIPATES
batch makes the witness a member: delivery as both
The phantom-slot analysis of §I exposed the failure shape
voter and witness in the window between the commit
this section removes: a node whose engine starts uninformed
and the removal is covered by ingress deduplication by
receives trafﬁc from an era it cannot classify, and the one(view, slot) and merely wastes IO.
era recovery gate drops what it cannot span. The ﬁx is not a Because the list rides at every node from the moment the
wider gate. A node that is not a member of the cluster has gossip lands, a successor leader resumes the stream on election
no business guessing an era at all; it learns the era ﬁrst, from without waiting for a re-announcement: failover does not
the cluster, and only then lets its engine see a message. The interrupt keep-up. The guard’s retransmission loop remains
whole mechanism is a wrapper — the ﬁnd-the-cluster guard load-bearing as the gossip’s own reliability — a request lost
— around the unmodiﬁed node engine, plus a witness list held in ﬂight is re-sent, naming the current frontiers, and a node
at every node and acted on by the leader. Neither touches a that missed the original learns of the joiner from the resend.
quorum rule.
A witness that is never promoted is an append-only audit tape
of everything the cluster committed; a statically registered
A. The ﬁnd-the-cluster guard
witness is never purged, so the disaster-recovery sink costs
A server that wants to join is known to the operator and exactly the stream that reincarnation already needs, under
mutually authenticated (a pre-shared key is sufﬁcient; the every leader in turn.
channel assumption is environmental, not protocol). It holds
some ﬁles describing which nodes were recently in the cluster. C. Safety conditions and kernel-checked content
It broadcasts a join request to all of them; it may miss the
The construction is safe under ﬁve hypotheses, each of
current leader, but every member answers with the pair (era, which is a construction rule rather than an assumption about
membership) as committed locally. Era advances only when a timing:
committed batch is applied, so no honest response can name
H1
Respondents report only committed (era, memberan era the cluster has not committed. The guard:
ship) pairs. Holds because era advances on commit
1) adopts the maximum reported era and the membership
application.
that response carried — the true era is that or something
H2
The guard adopts the maximum reported era with
ahead of it, never behind;
that response’s membership.
2) ﬂushes the adopted (era, membership) to durable storH3
The replay is the contiguous sufﬁx from the adverage whenever the adoption changes, so a crash during
tised frontier, folded in order by the ordinary accept
acquisition restarts from the best known pair rather than
path. There is no witness-speciﬁc accept path.
from blank;
H4
Messages from non-members are discarded at
3) runs its own retransmission loop until acquisition comingress; the witness votes nowhere and its promises
pletes — the ﬁrst responder may be stale or partitioned,
are counted nowhere.
and the leader may move while the responses are in
H5
The leader sends phase-2 only for views whose
ﬂight.
phase-1 completed on a quorum of members.
The guard then hands the engine a valid (era, membership)
Under H1–H5 the following are kernel-checked in
pair. The engine never executes the drop path that stranded UVRR/Witness.lean (zero external dependencies):
RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

5

n1

n2

leader n3

n4

n5

n6

initial conﬁguration (1, 1, 1, 1, 1, 0); n3 declares its
leader overlap, q ∩ q ′ = {n3 }, and services the
current view: beacon proposal and own vote carried
forward
(n

n5 crashes; a crash changes no weight

5, n
6)

resurrection announced as n6 = n5 +1; the
preallocated slot lights up at weight 0
phase one complete: live mass 4, the old view’s
family unchanged, n6 a live standby of weight 0
Fig. 1. Phase one of a ﬁve-node reincarnation with a preallocated phantom slot. Six membership slots n1 , . . . , n6 ; the phantom n6 is drawn grey and dashed
from the start at weight 0 and lights up when the crashed n5 resurrects into it. The leader n3 (ﬁlled) declares its overlap — beacon proposal and own vote
carried forward, dashed — and services the current view throughout. A crash changes no weight: the conﬁgured total stays 5 and the majority threshold stays
3; the live mass is 5 − 1 + 0 = 4. Time runs downwards without a scale.

W-era-bound
The adopted era never exceeds the committed era in no quorum (H4), and the leader’s right to send at
(adopt_le_of_honest).
v was earned from members (H5). The equivalence is
W-era-exact
Once the leader’s response arrives, the mechanised as telescoped_promise_equivalence in
adopted
era
equals
the
committed
era UVRR/Witness.lean: over the witness’s accept history —
(adopt_eq_of_leader).
the (view, accepted-preﬁx) pairs the stream delivered, views
W-replay
A journal holding exactly the frontier preﬁx, ex- strictly increasing — the induced promise ﬂoor is the highest
tended by the in-order contiguous sufﬁx, reconstructs accepted view, and with the induced promise in force no later
the leader’s preﬁx: log.k ⊕ replay(log, k, n) = log.n acceptance lands below it (the P23 fence over the stream);
(replay_reconstructs). A blank reincarnation the induced promise state is the promise state of a member
is k = 0 (replay_from_blank).
that processed phase-1 at the ﬂoor view, the history’s last
W-promotion
The witness’s committed preﬁx at any commit- entry being the greatest acceptance reported with it; and at
ted frontier equals the leader’s committed preﬁx promotion the witness’s ﬂoor equals the leader’s view and its
— hence equals any member’s, whose report is committed preﬁx equals the leader’s, so quorum certiﬁcates
a preﬁx of the primary by the ﬁxed-view in- that count it rest on the same evidence as for any member.
variant (promotion_committed_prefix with H5 stays a construction rule, carried as the named hypothesis
NormalLog.reachable_inv). Promotion there- LeaderEarned at the same layer of obligation as the acceptfore admits a voter whose state is indistinguishable path era guard of §III; the two negative controls refuse a
from a member that caught up by ordinary state witness that accepts below its highest accepted view and a
transfer, and quorum certiﬁcates that count it rest on promotion below the leader’s preﬁx. Authentication, the network and the witness’s timeout are environment; the durable
the same evidence as for any member.
W-non-interference
No
quorum contains the witness (witness_not_in_quorum):
adoption record is an instance of the termination obligations’
quorum membership is decided by the conﬁguration, marker storage.
and the witness is not in it.
VI. I NTRODUCTION
Two fault controls pin the hypotheses. invented_era_breaks_bound:
Oki and Liskov introduced Viewstamped Replication in
a report above the committed era breaks W-era-bound, so
1988
[2]. Liskov and Cowling’s 2012 Viewstamped ReplicaH1 is load-bearing — this is the failure that stranded the
tion
Revisited
(VRR) eliminated disk writes during normal prophantom-slot joiner, and the guard exists precisely to make it
cessing
and
view
changes [3]. Replication therefore retained
unrepresentable at the engine. gapped_replay_fails: a
the
state
needed
for
agreement without adding a local disk
replay that skips an entry cannot reconstruct the preﬁx, so the
ﬂush
to
each
update.
The remaining difﬁculty was how to
slot discipline of H3 is load-bearing.
bring a replica back after loss of that state.
Michael et al. exposed a safety failure in VRR’s recovery
D. The implicit promise, and honest scope
mechanism in 2016 [4, Sec. 4.2 and Appendix A.1]. A replica
A leader that streams phase-2 at view v to a witness can send a view-change commitment, crash, and return in an
is telescoping a promise: per Lamport, a promise at bal- older view having forgotten the commitment. It can then help
lot b covers all lower ballots, so accepting at v raises commit an operation that a later leader loses when delayed
the witness’s promise ﬂoor to v exactly as if a P RE - view-change messages arrive. Their counterexample breaks
PARE (v) had been received ﬁrst. The witness forfeits noth- linearizability, including when channels preserve message oring by raising the ﬂoor: its prior promises were counted der. The authors provide a virtual stable storage construction
RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

6

to support diskless crash recovery. Their result makes the
problem precise: restoring a replica’s data must also preserve
the commitments made by its identity.
Unbounded Viewstamped Replication Revisited (uVRR) repairs this failure by making a crash ﬁnal for the protocol
identity. The crashed identity never resumes execution. The
process that replaces it joins under a fresh identity, receives
state from the cluster, and acquires voting authority through
committed membership changes. Messages sent before the
crash remain messages of the old identity; the new process
cannot add fresh votes to that identity’s history. A cleanly
stopped process can instead restart under its existing identity
because it has preserved its protocol state. Startup fencing
and clean shutdown use durable writes, keeping those writes
outside normal replication. Loss of volatile state therefore
leads to replacement of a member: crash recovery becomes
crash-stop reincarnation.
Turner supplies the construction that connects this lifecycle
to continued replication [5]. Where the leader is the sole
intersection of an active decision quorum and the next prepare
quorum, it can withhold its own promise while collecting
the others. The active quorum remains usable during that
preparation; the leader completes the handover with its own
local promise. We use this leader overlap to organise reincarnation. For ﬁve unit-weight voters, the runtime schedule
has six committed batches, including exact doubling and
halving. The simpliﬁed two-batch instantiation retained for the
concrete lifecycle theorem is stated separately in Appendix A.
Appendix B supplies a general conﬁguration-path constructor
and an abstract casting-vote completion using fresh identities.
Turner’s agreement argument supplies the basis for composing
safe adjacent conﬁguration boundaries.
Together, these ideas retain VRR’s diskless normal path
while giving failed members a route back through fresh identities. The intended setting is agreement over small amounts
of metadata embedded in a distributed application [6]. Such a
service can coordinate membership or advisory-lock metadata
without putting synchronous persistence on every update’s
critical path. Its normal replication need not wait for a disk
ﬂush or batch updates to amortise one.
Section VII develops the lifecycle and leader-overlap construction. Section VIII gives the ﬁve-voter schedule and its
Lean proof of per-slot agreement from the protocol’s history
invariants, together with the replacement weights, exclusion of
the evicted voter and identity nonreuse. Section IX develops
the metadata applications, and the appendix sets out the
mathematical hypotheses and veriﬁcation evidence. The name
“unbounded” follows Turner’s treatment of pipelining through
reconﬁguration: agreement does not depend on a ﬁxed bound
on outstanding slots.

that an identity never produces new protocol actions after a
replacement incarnation takes over.
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

(q ∈ QIe , r ∈ QII
e+1 ).

(2)

The direction in (2) matters: the earlier phase-one quorum
must meet the later decision quorum. These conditions are
part of Turner’s era-based agreement argument [5].
Our concrete schedule uses the same strict weightedmajority family for both phases. If we (n) is a nonnegative
integer weight on a ﬁnite support, a quorum q satisﬁes
∑
∑
we (n) >
we (n).
(3)
2
n∈q

n

The weighted proof establishes the requisite intersections
when successive conﬁgurations differ by at most one unit of
total absolute weight. Membership records for zero-weight
nodes may change in the same batch without altering (3).
A crash itself does not change any weight; only committed
conﬁguration commands do.

VII. M ETHOD
A. Processes, state and identity

C. The leader as the overlap

The agreement model is asynchronous and non-Byzantine.
Messages may be delayed, and an old message retains the
identity that issued it. Each incarnation has a logical identity
distinct from its physical host. The lifecycle must ensure

Suppose a leader ℓ uses ballot b with an active decision
quorum q. To prepare ballot b′ > b for the next era, it chooses
a prospective phase-one quorum q ′ with
q ∈ QII
e,

q ′ ∈ QIe+1 ,

RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

q ∩ q ′ = {ℓ}.

(4)

7

{n2 , n3 }

n1 (primary of v)
commit o
at v (quo
ru

{n4 , n5 }

m {n1 , n

2 , n3 })
D OV IE W C HA NG E
v+1 (volatile comm
itment)

n1 crashes:
volatile R
state
E C Olost
VE

RY x (§
4

f

OV E
+1 R E C

.3: no dis

P
RY R E S

O N S E in

k record)

R EC OV ERY x
ary
rrent prim

cl. cu

n1 re-enters as n1 , status normal:
the v+1 commitment is forgotten
bad vote:

n1 ’s stale self joins a v+1 quorum

Fig. 2. The diskless-recovery amnesia of VRR at ﬁve voters (f =2), after Michael et al. [4, Sec. 4.2 and Appendix A.1]. The forgotten commitment now
spans a three-voter quorum: the recovering node’s stale self can complete a v+1 quorum that a later leader cannot reconcile. Group arrows represent messages
to each member; time runs downwards without a scale.

{n2 , n3 }

n1 (primary of v)
commit o
a

t v (quoru

{n4 , n5 }

m {n1 , n

2 , n3 })
D OV IE W C HA NG E
v+1 (volatile comm

itment)

n1 crashes: ﬁnal for the
protocol identity — n1 never resumes
process reincarnates as n6 :
fresh identity, weight 0, non-voting
transfe
am / state

re
witness st
n6 caught up at the current era:
still no vote — the quorum read ran
outside the voting protocol
d

committe

NT
INCREME

andby

r to the st

s 0→1
: n6 walk

n6 votes for the ﬁrst time —
nothing of n1 ’s history can be re-voted
Fig. 3. Crash-stop reincarnation at ﬁve voters on the same preﬁx. The replacement n6 acquires state while it cannot vote; promotion is the committed
membership change. Group arrows represent messages to each member; acceptance replies omitted.

This is Turner’s casting-vote condition [5]. The leader ﬁrst
sends prepare requests only to q ′ \ {ℓ}. Their promises do
not disable an acceptor in q. The leader withholds its own
promise and may continue proposing at b, provided no other
event invalidates the old ballot’s guards.
After receiving those promises, the leader chooses the sufﬁx
boundary k and makes its own promise locally. Phase one
is then complete without another network exchange for the
leader’s response. Proposals at b′ must still obey the normal
rule for preserving values reported by phase one. Figure 4
shows the message order.
Decision quorums follow the era of the slot, not simply
the era associated with a ballot. Consequently, continuing to

decide later-era slots at b also requires a quorum valid for
those slots. In the concrete pivot below, the active quorum is
a majority in both adjoining eras.
D. Replacing one of ﬁve voters
Let n0 be the failed incarnation, n1 , . . . , n4 the other original voters, and n5 its fresh replacement. Table I gives all six
committed boundaries of the ﬁve-voter weighted replacement.
A join inserts the replacement at zero weight before its ﬁrst
increment; a leave removes the old identity after its ﬁnal
decrement. The three-voter unit-weight case uses two batches.
These are distinct schedules, both covered by the Rust tests.

RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

8

{n1 , n2 }

leader ℓ = n3
prepare b ′

{n4 , n5 }
; own pro
mise with

held

continue

at b
proposals
for
promises
posals at

further pro

b′

b

choose sufﬁx k; promise locally
phase one complete
proposals
at

b′

Fig. 4. Leader overlap in the simpliﬁed Appendix A schedule at E1 → E2 : q = {n1 , n2 , n3 } and q ′ = {n3 , n4 , n5 }. Original schematic of Turner’s
casting-vote construction [5], specialised to the replacement schedule. Time runs downwards without a scale. Group arrows represent messages to each member;
acceptance replies, state transfer and unrelated trafﬁc are omitted.

TABLE I
F ULL FIVE - VOTER REPLACEMENT: SIX COMMITTED BATCHES . Z ERO
DENOTES VOTING WEIGHT, INCLUDING IDENTITIES ABSENT FROM
MEMBERSHIP.

Stage

n0

n1

n2

n3

n4

n5

Initial
Double
Join, increment
Decrement old
Decrement, leave
Increment new
Halve

1
2
2
1
0
0
0

1
2
2
2
2
2
1

1
2
2
2
2
2
1

1
2
2
2
2
2
1

1
2
2
2
2
2
1

0
0
1
1
1
2
1

Each boundary preserves cross-era majority intersection by
a unit change or exact scaling. Figure 1 depicts the pivot of the
simpliﬁed agreement instantiation retained in Appendix A; it is
not a diagram of all six weighted boundaries. A failed witness
for a particular pair of quorum sets does not establish failure of
reconﬁguration reachability. Appendix B constructs paths with
an available majority and shows how fresh logical identities
supply an abstract singleton leader intersection. Membership
weight and receipt of a protocol message remain distinct
quantities.
VIII. R ESULTS
A. The agreement result
The ﬁnal Lean theorem composes an era-based agreement
theorem with the concrete replacement weights and an abstract
identity lifecycle. Its contract assumes a well-founded total
ballot order, a ballot-to-era map monotone in ballot order,
the required ballot/slot era bound, and the history invariants
corresponding to Turner’s P2–P7 [5]. Appendix A states the
assumptions and reproduction procedure. Those invariants
constrain promises, reports of prior acceptances, phase-one
justiﬁcation and value selection, acceptance of proposals, and
quorum evidence for a chosen value. The theorem derives
quorum intersection from the weight schedule.

Theorem 2 (Five-voter replacement). For a history whose
phase-one and phase-two quorum families follow the simpliﬁed Table II, assuming the ballot-order, era and history
conditions above, any two values chosen for the same slot
are equal. In E1 , both the failed identity and its replacement
have zero weight; in E2 , the failed identity has zero weight
and its replacement has unit weight. From completion of the
crossing era onwards, the old identity has no voting weight in
any continuation of the forced conﬁguration run. Separately,
along an identity run satisfying the formal lifecycle transitions,
an identity that has been superseded by a bump is never again
the current identity.
Proof. The strict-majority construction and one-unit boundaries establish the within-era and adjacent-era quorum intersections (1)–(2), together with quorum nonemptiness. Instantiating the era-based agreement theorem with those families establishes equality of chosen values under the supplied
history invariants. Evaluation of the concrete conﬁgurations
establishes the stated weights. The eviction invariant preserves zero weight for the old identity along the forced run.
The identity lifecycle transitions preserve monotonic identity
growth, excluding a superseded identity from becoming current again.
The leader-overlap lemmas establish that preparing members outside the active quorum preserves that quorum’s guards,
and that adding the leader’s promise completes the prospective
phase-one quorum.
B. Evidence and reproduction
The Lean 4 project is archived at repository commit 352b0bf [12], extended here by adding the existing eviction invariant to the ﬁnal conjunction. It
uses Lean 4 version 4.33.1 [13] without Mathlib. The
source directory is formal/uvrr-lean/UVRR/. The
weighted intersection, era agreement, casting-vote and simpliﬁed ﬁve-voter modules supply the components of Theorem 2. Its ﬁnal statement is ReincarnationSafety.

RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

9

five_safe, in ReincarnationSafety.lean, import- gives a small-cluster example with dedicated two-vCPU, 8
ing ReincarnationAgreement.lean.
GB AWS machines [10]. etcd can itself be embedded in a
From formal/uvrr-lean, run lake build and Go application [11]. The proposed combination here is small
python3 check_axioms.py. The checked baseline’s ax- application-owned state, volatile normal-path replication, and
iom audit covers 355 declarations and permits only Lean’s replacement through fresh identities and overlapping quorums.
standard logical axioms; it rejects proof holes and custom
axioms. The recorded rung-26 evidence also exercises a neg- B. Scope
ative control that tries to give the replacement voting weight
a) Implementation correspondence: The ﬁnal theorem
in E1 . Its failure checks that particular erroneous claim. The
assumes protocol-history invariants. Their composition with
repository contains the proof-generation transcript; generated
the era-changing operational model is left as an exercise to
proof terms are checked by Lean, and the generator is not part
the reader, as speciﬁed in Appendix A. The appendix also
of the trusted proof checker.
identiﬁes the corresponding implementation obligations: cleanTheorem 2 covers the simpliﬁed two-batch instantiation
stop classiﬁcation, identity fencing and state transfer before
with its ﬁnal conﬁguration retained thereafter. Appendix B
voting.
supplies the general weighted reachability result and the
b) Scope and availability: Table I replaces one unitruntime solver contract. The general weighted-intersection
weight voter among ﬁve. Appendix B constructs schedules
lemmas have broader scope than this instantiation.
for arbitrary weighted-majority endpoints with available majorities. A higher ballot, an unavailable leader or insufﬁcient
IX. D ISCUSSION
surviving voters can interrupt progress. Fresh identities permit
A. Applications
repeated replacement in the design; the theorem covers the
The intended deployment is an embedded authority for displayed schedule.
c) Meaning of strong consistency: The mechanised result
small metadata, with ordinary replication performed in memis per-slot agreement. A complete service-level linearizabilory.
a) Short-lived advisory locks and leader leases: A repli- ity argument also needs ordered execution, appropriate read
cated state machine can order acquisition, renewal and expiry semantics and a mapping between client operations and log
decisions over a small record containing an owner, generation entries. Lease guarantees require the additional conditions
and expiry policy. An expiry command for an old generation stated above.
must not revoke a newer grant. If a client uses ownership to
X. R ELATED W ORK
control an external resource, that resource may also need to
reject stale generations. A real-time leader lease additionally
Viewstamped Replication introduced the organisation of
needs explicit clock and timing assumptions.
replication around views and a primary [2]; VRR revisits that
b) A membership authority for another algorithm: An organisation and separates normal operation, view change, reapplication whose bulk algorithm uses gossip or another covery and reconﬁguration [3]. Michael et al. identify the failweaker consistency model may still need one agreed version of ure of diskless recovery to preserve view-change commitments
cluster membership, ownership assignments or conﬁguration and construct virtual stable storage [4]. uVRR takes a different
changes. An embedded replicated state machine can supply route: it makes a crash ﬁnal for the old identity, then brings
that authority, while the bulk algorithm disseminates or con- the process back as a new member through reconﬁguration.
sumes its decisions.
The replacement schedule connects that lifecycle to agreement
c) The CORFU precedent: CORFU provides a concrete across conﬁgurations.
precedent for compact conﬁguration metadata alongside a
Turner’s Unbounded Pipelining in Dynamically Reconﬁglarger storage system [7]. Its reconﬁguration is patterned after urable Paxos Clusters supplies the central quorum geomeVertical Paxos; Section 2.5 describes storage units acting try, era-based agreement argument and casting-vote construcas acceptors in a Paxos-like protocol. Section 2.6 estimates tion [5]. Withholding the leader’s own promise is Turner’s
that adding 0.1% writes keeps projection metadata below 25 mechanism. The present work applies it to reincarnation,
MB for a 1 TB cluster under an adversarial workload. The proves weighted conﬁguration reachability and an abstract
described implementation uses a shared network drive for casting-vote completion, and proves agreement for a concrete
agreed projections.
replacement instantiation from the protocol’s history invariants.
d) External coordination services: ZooKeeper and etcd The application discussion places this design alongside coordiare established ways to supply coordination separately from nation services and CORFU, using their stated semantics and
an application. ZooKeeper offers linearizable state changes resource context.
and per-client ordering, while local reads have weaker semantics [8]. etcd documents strict serializability for its key-value
A PPENDIX A
operations and distinguishes default linearizable reads from
P ROOF STRUCTURE , HYPOTHESES AND EXECUTABLE
optional serializable reads [9]. An application must choose
CHECKS
the operation semantics it actually needs.
This appendix separates the mathematical argument, its
A separately operated service also brings deployment and mechanised components, and the obligations supplied as hyresource costs. For scale, etcd’s version 3.6 hardware guide potheses. The term UPaxos below refers to Turner’s paper [5].
RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

10

TABLE II
S IMPLIFIED TWO - BATCH AGREEMENT INSTANTIATION . A ZERO RECORDS
VOTING WEIGHT, NOT WHETHER A ZERO - WEIGHT MEMBERSHIP RECORD
IS PRESENT.
Era

n0

n1

n2

n3

n4

n5

E0
E1
E2

1
0
0

1
1
1

1
1
1

1
1
1

1
1
1

0
0
1

The weighted-intersection and rescaling results follow from
UPaxos Appendix A.
A. Weighted overlap, doubling and halving
Let N be a ﬁnite set of identities, with
∑nonnegative integral
weight
functions
w,
v.
Write
W
=
n∈N w(n), V =
∑
∑
v(n)
and
w(q)
=
w(n).
A
strict
majority has
n∈N
n∈N ∩q
2w(q) > W .
Lemma 1 (Scaled overlap, UPaxos). If positive integers a, b
satisfy
∑
|aw(n) − bv(n)| ≤ 1,
(5)

3) P2–P3: a free promise for a ballot excludes lower-ballot
acceptances for that slot.
4) P4: a promise reporting a prior acceptance reports the
greatest acceptance below its ballot.
5) P5: a proposal is justiﬁed by a phase-one quorum at
its ballot era; if that quorum reports prior values, the
proposal selects the value at a maximal reported ballot.
6) P6: every acceptance has a corresponding proposal.
7) P7: a chosen value has acceptance evidence from a
phase-two quorum at the slot era, and e(i) ≤ e(b) + 1.
These are history predicates. The ﬁxed-conﬁguration operational components in rungs 13–16 do not constitute a completed composition with the changing era sequence.
a) Exercise (composition): Construct the mapping from
the era-changing operational execution to this history and
establish the listed predicates, including preservation of the
promise fence, maximal-history selection, and installation
across conﬁguration boundaries. This composition is left as an
exercise to the reader. The host must additionally implement
clean-stop classiﬁcation, fresh identity allocation and state
transfer before voting.

n∈N

then every strict majority under w intersects every strict
majority under v.
Proof. Positive scaling preserves strict majority. Suppose majorities q, r are disjoint. Summing the nodewise disjointness
bound gives
∑
2aw(q) + 2bv(r) ≤ aW + bV +
|aw(n) − bv(n)|. (6)
n∈N

Since all quantities are integral, the two strict majority inequalities make the left side at least aW + bV + 2. Equations (5) and (6) contradict that bound. The Lean declarations are WeightedGeneral.disjoint_bound and
WeightedGeneral.scaled_overlap.
Taking a = b = 1 gives the one-unit change rule. Exact rescaling makes the distance zero: doubling all weights
preserves the majority family, as does exact halving when
all weights are even. Thus these transformations preserve the
required intersections. Arbitrary rounding during halving is
not covered by this argument. Lean provides majority_
scale and scaled_equal_overlap in the same module.
Appendix B applies these local closure results to arbitrary
ﬁnite weighted paths; the concrete lifecycle conjunction below
retains its stated two-batch instantiation.
B. Hypotheses of the agreement theorem

C. Composition steps and conclusion
First, apply Lemma 1 to each adjacent pair in Table II,
and self-overlap within each era. The case split over era
0, era 1, and later eras is ReincarnationAgreement.
sequence_p1; it derives P1 for the sequence. Second,
instantiate Turner’s Theorem 10 with P1 and the contract above to obtain per-slot agreement. Third, evaluate
the four intermediate and ﬁnal weights in Replacement.
Fourth, apply ReincarnationFive.five_evicted_
never_voting: after the crossing era, the old identity
has zero voting authority throughout any continuation of
the forced run. Finally, apply Reincarnation.amnesia_
unreachable to exclude reuse of the pre-bump identity in
the identity transition system. These are the four conjuncts
now exposed by Safe.
Zero voting authority is a conﬁguration property. Delayed
old messages retain their old identity. The separate identity and
conﬁguration runs are abstract components; the exercise above
supplies their relationship to a single executable protocol. The
concrete leader-overlap witness is the E1 /E2 pivot in Figure 4.
D. Commands and observed checks
From formal/uvrr-lean, the checks are:
lake build
python3 check_axioms.py
python3 check_mutations.py
showboat verify \
ladder/26-reincarnation-safety.md

For each slot i and ballot b, let e(i) and e(b) be their
eras. The contract of ReincarnationSafety.five_ The extended build completes 25 jobs. The axiom audit
safe supplies:
covers 412 declarations; the ﬁnal theorem depends only on
1) A well-founded strict total ballot order, with e(b) mono- propext, Classical.choice and Quot.sound. The
tone in that order. A proposed value satisﬁes e(b) ≤ e(i). mutation suite checks rejection of an existential quorum
2) Both quorum families equal the strict-majority family checker, an unsafe decision family, an omitted next-slot guard,
of the displayed E0 , E1 , E2 sequence, with E2 retained append after a view-change fence, and cross-view prepare
afterwards.
receipt. It also checks detection of sorryAx, even when the
RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

11

TABLE III
R EPORTED ISOLATED L EANSTRAL REPAIR OUTCOMES . FAILURE MEANS
NO ACCEPTED REPAIR WITHIN THE STATED BUDGET.

Mutation and supplied context

Outcome

M1: delete ﬁnal proof; ﬁle only
M2: M1 plus four lemma signatures
M3: wrong conjunction projection
M4: delete sequence P1 proof; signatures
supplied
M5: false claim c1 (5) = 1

Fail
Pass, round 4
Fail
Pass, round 2;
repeat failed
Fail, as required

compiler returns success. Rung 26 checks that promoting the
replacement prematurely in E1 is rejected.
The Leanstral driver has two ofﬂine regressions, run from
the repository root with python3 applied to tests/test_
leanstral_driver.py. A mock API forces two concurrent proof loops to ﬁnish writing their candidates before either
can copy its result, exposing a shared-ﬁle race deterministically. A second assertion checks that the prompt preserves
project imports. Both failed on the original driver and pass
with private per-invocation scratch directories and an importpreserving prompt.

Proof. Decrease unavailable coordinates towards their targets,
then increase available coordinates towards theirs. Each operation increases the available-minus-unavailable margin. Next
decrease available coordinates towards their targets, then increase unavailable coordinates towards theirs. In these latter
phases the margin is at least the positive target margin. Every coordinate moves monotonically between its endpoints;
in
∑ particular the retained leader remains positive. Exactly
n |s(n)−t(n)| unit edits occur, the minimum possible using
unit edits. Consecutive vectors differ in one unit, so Lemma 1
discharges cross-era intersection. Zero-weight joins and leaves
preserve quorum families; exact doubling and halving provide
additional shortcuts.

The Lean module WeightedReachability.lean
proves zero_one_two_connected using an available
leader of weight one as an intermediate proﬁle. It also proves
UnitStep.overlap, linking actual identity-indexed unit
steps to the weighted quorum theorem. The shorter four-phase
construction above is implemented in Rust and independently
checked in Python; the Lean path is an existence proof, not a
proof that the Rust program is the shortest-path implementation.
When membership identities or succession order differ, the
Rust constructor uses the available one-voter intermediate. It
E. Reported adversarial repair experiment
removes zero-weight records, transfers authority to an availTable III records the externally supplied Fable 5.1 report, not able target voter if necessary, inserts the requested identities at
a fresh experiment executed for this revision. The report used zero weight in target order, and restores available target mass
Leanstral 1.5, a ﬁve-round budget per loop, isolated directories, before unavailable target mass. The transfer has a two-voter
and byte-identical statement checks on passing repairs. Its stage before the former anchor loses its vote. Each membership
initial shared-directory batch was discarded because candidates edit is a separate boundary, keeping each endpoint union
crossed between jobs. The raw repair transcripts were not within the implementation’s 16-identity enumeration cap even
supplied with the report, so the table should be read with that when the source and target memberships are disjoint. This cap
provenance. The contrast between M1 and M2 suggests that bounds implementation cost; the mathematical result is ﬁnitesupplying library signatures helped composition in these runs. size general. The one-voter intermediate retains availability but
M5 supplies a useful false-statement control.
reduces tolerance of additional failures.
A PPENDIX B
C ONSTRUCTIVE WEIGHTED RECONFIGURATION AND
OPERATOR SOLVER

B. Abstract casting votes with fresh identities
Fix an available leader of weight l > 0 and an available
preparing quorum P of mass p, containing the leader. Let
T = A + D. Add fresh identities outside P with total weight

This second appendix gives the general construction. Its universe is a ﬁnite set of logical identities, including identities at
δ = max(0, 2p − 2l + 1 − T ).
(7)
weight zero. A conﬁguration is a function w : N → {0, 1, 2}.
Let L ⊆ N be the identities available to answer protocol
Then P and (N \ P ) ∪ {leader} are strict majorities in the
messages, A = w(L), and D = w(N \ L). Availability means
enlarged universe and their intersection is exactly the leader.
A > D. Correctness concerns intersection of quorum families;
Indeed 2p > T + δ and 2(T + δ − p + l) > T + δ. The ﬁrst
tolerance of a further host or datacentre failure is a separate
inequality follows from the original majority and l ≥ 1; the
property of each conﬁguration.
second is precisely the lower bound deﬁning δ. Every preﬁx
of the added mass keeps P a majority. Only identities in P
A. Reachability and the operation schedule
are required to answer preparation; fresh abstract identities are
Theorem 3 (Available-majority reachability). For ﬁnite weight not counted as received promises.
proﬁles s, t with the same availability set, an available strict
Choose P by starting with the leader and adding available
majority at each endpoint, and a retained leader of positive identities until its mass ﬁrst exceeds T /2. Since each weight
weight at both endpoints, there is a ﬁnite path of single-identity is at most two, p ≤ ⌊T /2⌋ + 2, whence δ ≤ 3. Two
increments and decrements from s to t. Every intermediate fresh identities, with weights at most two each, sufﬁce. The
proﬁle stays in {0, 1, 2}, retains that leader and an available bound is sharp: available weights (1, 2, 2) with the weightstrict majority, and consecutive strict-majority quorum fami- one leader and unavailable weight one require three added
lies intersect.
units. Lean proves the preﬁx selection and this bound in
RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

12

cap_two_phantom_construction, the arithmetic minimality for a chosen P , and the set-intersection statement. This
is an abstract quorum witness, not an assertion that absent
hardware responds to messages. The ordinary prepare and
acceptance history obligations still govern its protocol use.

view changes and client commits. Datacentre-loss assertions
describe the restored uniform conﬁguration, not every intermediate schedule.
The independent Python checker and generated ﬁgures
are in research/weighted-reachability/. Its
exhaustive run covers 310,007 symmetry representatives
through ten identities (14,541,815 labelled proﬁles) and
C. Equal total mass and datacentre resilience
774,159 endpoint-pair representatives through six identities
Equal total mass alone does not imply cross-era intersection.
(3,939,756 labelled pairs), with zero failed checks. These
For (1, 2, 1, 2) → (2, 1, 2, 1) on (A, B, C, D), {B, D} is an
ﬁnite counts are computational evidence; the general
old majority and {A, C} a disjoint new majority. The solver
claim rests on the proof above and the separately stated
instead returns
Lean theorems. Reproduce the Rust checks with cargo
test -test reconfiguration_solver -test
(1, 2, 1, 2) → (2, 2, 1, 2) → (2, 2, 2, 2)
reconfiguration_stepwise, and the proof checks
→ (2, 1, 2, 2) → (2, 1, 2, 1).
with lake build and python3 check_axioms.py in
Each of these four boundaries is safe. For datacentres holding the Lean project. The Leanstral driver’s check command also
(A, B) and (C, D) at weights (1, 1, 2, 2), loss of the second accepts the module without proof holes or compiled-decision
leaves mass two out of six; loss of the ﬁrst leaves mass four. shortcuts.
This asymmetry can be deliberate. Four unit voters split two
R EFERENCES
per datacentre can use ﬂexible phase-two quorums of two with
phase-one quorums of three: local decisions and local leader
[1] L. Lamport, “Paxos made simple,” 2001. [Online]. Available:
https://www.microsoft.com/en-us/research/wp-content/uploads/2014/03/
election have different availability conditions. The solver’s
paxos-simple.pdf
guarantee is for strict weighted majorities; a ﬂexible quorum
[2] B. M. Oki and B. H. Liskov, “Viewstamped replication: A new primary
policy must pass the core’s role-speciﬁc intersection gates
copy method to support highly-available distributed systems,” in ACM
PODC, 1988, pp. 8–17.
separately.
D. Executable interface and validation
The dependency-free Rust module src/solver.rs exposes solve and solve_replacement. Results are ordered EraStep batches with the resulting conﬁgurations; the
target era is ignored and eras advance from the committed
source. A request without an available majority at either endpoint returns the available and total masses. The replacement
entrypoint prefers the complete standard schedule when its
intermediate conﬁgurations remain available and its endpoint
matches; otherwise it invokes the general constructor. The
three-unit-voter schedule has two batches and the ﬁve-unitvoter schedule has all six batches in Table I. Replanning uses
the current committed conﬁguration.
The command-line binary reconfiguration-solver
accepts ordered id:weight lists, a target list or -replace
old:new, and an available-identity list. Each output row
states the batch, resulting weights and available mass. State
acquisition precedes promotion. If a different leader is needed,
the existing next_view_selecting and administrative
view-change input select a later ballot directly; the ordinary
quorum-backed view-change exchange still establishes leadership. A local era increment does not constitute that exchange.
Rust property tests sample arbitrary six-identity source
and target weights in {0, 1, 2} subject to available majorities, replay every batch, and check every boundary with
the exhaustive quorum gate. Deterministic tests cover the
equal-mass counterexample, all 36 original-leader/failed-host
combinations for two nodes in each of three datacentres, exact
standard schedule lengths, and disjoint 16-member endpoints.
The protocol test drives all 36 unit-weight cases through
leader timeout when required, reincarnation announcements,

[3] B. Liskov and J. Cowling, “Viewstamped replication revisited,” MIT
CSAIL, Tech. Rep. MIT-CSAIL-TR-2012-021, 2012. [Online]. Available: https://hdl.handle.net/1721.1/71763
[4] E. Michael, D. R. K. Ports, N. Kr. Sharma, and A. Szekeres, “Providing
stable storage for the diskless crash-recovery failure model,” University
of Washington, Tech. Rep. UW-CSE-16-08-02, Aug. 25, 2016. [Online].
Available: https://syslab.cs.washington.edu/papers/diskless-tr16.pdf
[5] D. Turner, “Unbounded pipelining in dynamically reconﬁgurable Paxos
clusters,” revision 1A9DBA37, Aug. 14, 2017. [Online]. Available:
https://github.com/DaveCTurner/paxos-membership. The proof artifact
includes the consulted PDF and its digest.
[6] S. Massey, “Viewstamped replication revisited,” Aug. 12, 2026.
[Online]. Available: https://simbo1905.wordpress.com/2026/08/12/
viewstamped-replication-revisited/
[7] M. Balakrishnan, D. Malkhi, J. D. Davis, V. Prabhakaran, M. Wei, and
T. Wobber, “CORFU: A Distributed Shared Log,” ACM Trans. Comput.
Syst., vol. 31, no. 4, Art. 10, Dec. 2013, doi: 10.1145/2535930.
[8] P. Hunt, M. Konar, F. P. Junqueira, and B. Reed, “ZooKeeper:
Wait-free coordination for Internet-scale systems,” in USENIX ATC,
2010. [Online]. Available: https://www.usenix.org/events/usenix10/tech/
full_papers/Hunt.pdf
[9] etcd authors, “etcd API guarantees,” documentation v3.6, accessed
Sep. 11, 2026. [Online]. Available: https://etcd.io/docs/v3.6/learning/
api_guarantees/
[10] etcd authors, “Hardware recommendations,” documentation v3.6, accessed Sep. 11, 2026. [Online]. Available: https://etcd.io/docs/v3.6/
op-guide/hardware/
[11] etcd authors, “Embedding etcd in a Go application,” documentation v3.6,
accessed Sep. 11, 2026. [Online]. Available: https://etcd.io/docs/v3.6/
dev-guide/golang_embed_pkg/
[12] S. Massey, “uVRR proof artifact,” commit 352b0bf, 2026. [Online].
Available: https://github.com/lua-lunet/uvrr-core/tree/352b0bf/formal/
uvrr-lean
[13] L. de Moura and S. Ullrich, “The Lean 4 theorem prover and programming language,” in Automated Deduction – CADE 28, 2021, pp. 625–
635, doi: 10.1007/978-3-030-79876-5_37.

RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

13

Fig. 5. Sharp phantom-mass example, generated by the Python checker. The preparing quorum uses available identities; its abstract companion includes fresh
identities. Both have weight ﬁve out of nine and intersect only at the leader. Fresh identities do not supply message receipts.

RESEARCH CHECKPOINT — 11 SEPTEMBER 2026 — 20260928-bef2d42

