UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37

1

Unbounded Pipelining in Dynamically
Reconfigurable Paxos Clusters
David C. Turner

Abstract—Consensus is an essential ingredient of a faulttolerant distributed system systems. When equipped with a
consensus algorithm a distributed system can act as a replicated
state machine (RSM), duplicating its state across a cluster
of redundant components to avoid the failure of any single
component leading to a system-wide failure. Paxos and Raft
are examples of algorithms for achieving distributed consensus.
Practical implementations of this kind of system must support
dynamic reconfiguration in order to be able to replace failed
components and perform other administrative tasks without
downtime. Paxos can achieve high performance by pipelining
(starting work on new requests before existing requests have completed) but typically bounds the length of the pipeline to ensure
consistency during reconfiguration. Raft also supports pipelining
and imposes no such bound on concurrent requests, preserving
consistency instead by restricting which reconfigurations may be
performed. This article shows how to extend Paxos to support
a more general form of reconfiguration which subsumes the
original bounded-pipeline approach as well as Raft-like fullyconcurrent reconfigurations and more besides.
Index Terms—Distributed algorithms, fault tolerance.

I. I NTRODUCTION
ELIABLE distributed systems must be able to tolerate
a fault in any individual component without suffering
a system-wide failure, and typically achieve this by ensuring
that there is redundancy between the components. A replicated
state machine (RSM) is a style of fault-tolerant distributed
system in which a deterministic state machine is replicated
across a set of distinct nodes [1]. Being deterministic, the
nodes’ states remain synchronised if they all start in the same
state and perform the same sequence of transitions.
In order to arrange for each node to perform the same
transitions the system may achieve consensus on (or choose)
a sequence of values which describe the transitions. The
sequence must be consistent across the whole system even in
the presence of failures, and as long as there are not too many
failures it must remain possible to continue to make progress.
This is known as the distributed consensus problem, for which
a number of solutions are known to exist, including Paxos [2]
and Raft [3]. They typically run on a cluster of 2f + 1 nodes,
where f is the number of faulty nodes that should be tolerated,
and consensus is achieved when a nonempty quorum of nodes
(e.g. at least f + 1 of them) agree. The collection of quorums
in use is known as the configuration of the cluster.

R

Manuscript received 14 August 2017.
The author is with the Operations and Planning Systems division of
Tracsis plc, Leeds LS2 9DF, United Kingdom (email: dct25-a1vwi@mythicbeasts.com)
c 2016-7 Tracsis plc. This work is licensed under the Creative Commons
Attribution-ShareAlike 4.0 International License. To view a copy of this
license, visit http://creativecommons.org/licenses/by-sa/4.0/ or send a letter
to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

It is normally necessary to be able to dynamically reconfigure a cluster by adding or removing nodes while it is
running, in order that parts of the system can be repaired
or replaced without needing to take the whole system offline. It is crucial that all participating nodes agree on the
cluster configuration, and this can be achieved by holding the
configuration within the RSM itself and using the consensus
algorithm to choose special reconfiguration commands when
a configuration change is desired.
In Paxos, each value is chosen using a conceptually-separate
instance of a two-phase consensus protocol known as Synod.
The full Paxos algorithm essentially runs an infinite sequence
of Synod instances in parallel, using uniformity of the instances to do so without requiring infinite time or resources. It
starts by running phase I of all instances at once and then runs
phase II of each instance in turn to yield the desired sequence
of chosen values. It normally continues to run phase II for
extended periods of time, but will return to phase I if certain
nodes become faulty, or if messages between certain pairs of
nodes cease to be delivered reliably for a period, or if the
configuration changes. Raft’s pattern of execution is similar.
Both algorithms can achieve high throughput by allowing
for pipelining [4] whereby work may begin on an instance
even before all previous instances have fully completed. It is
a little tricky to ensure that this preserves consistency when
the configuration is held in the RSM itself because a value
may only be proposed once a quorum of nodes are ready for
it, but there may be a configuration change in the pipeline
which would change the quorums so as to make a proposal
invalid. If this case is not handled carefully then it may lead
to inconsistency. Paxos implementations typically solve this
problem by limiting the length of the pipeline to some α > 0
and requiring that a configuration change chosen at instance i
does not take effect until at least instance i + α, which means
that when a proposal is made there can be no as-yet-undecided
configuration change in the pipeline that could affect it. In
contrast, Raft imposes no limit on the number of concurrentlyrunning instances and instead restricts the reconfigurations
that may occur to only allow ones that cannot result in
inconsistency. An operator may then perform a sequence of
these restricted reconfigurations in order to achieve an arbitrary
reconfiguration.
In Paxos, the choice of the pipeline length parameter α
must be made carefully. If it is too small then the system may
suffer from poor performance due to lack of parallelism, but if
it is too large then configuration changes can be unreasonably
expensive to complete. It is more elegant [5] to limit the
pipeline length only during reconfiguration and to allow the
limit to vary while the system is running, but it would be

UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37

better still if there were no need for a limit at all, even during
reconfiguration.
Here it is shown that the pipeline may indeed safely remain
unbounded even during a configuration change as long as the
reconfiguration satisfies certain conditions described in section
IV-A below. It is also shown that if these conditions are not
satisfied then a reconfiguration can still take place as long as
the pipeline length is temporarily limited.
The algorithm is presented here in its entirety for the sake
of consistency of notation and because it modifies the original
algorithm in ways that invalidate its consistency and liveness
proofs. We begin with a recap of the Synod algorithm in
section III and follow this by covering the full Paxos algorithm
in section IV, then in section V it is shown how this work
generalises and unifies the previously-known reconfiguration
processes supported in Paxos and Raft. Reworked proofs of
liveness and consistency are included in section IV-C and
appendices B and C and differences from the original are
highlighted throughout. The appendices are informal versions
of formal proofs performed using the Isabelle/HOL proof
assistant [6].
II. R ELATED W ORK
Lamport’s original presentation of Paxos [2] introduced the
bounded-pipeline technique for supporting reconfiguration in
which the pipeline length was defined to be 3. He later clarified
that the value ‘3’ was intended to stand for an arbitrary α > 0
in [7]. Later still Lamport and Massa [8] drew a distinction
between Static Paxos in which the configuration may not
change and Dynamic Paxos which supports reconfiguration
but requires a bounded pipeline. Dynamic Paxos was used as
a basis for Cheap Paxos in which the system automatically
reconfigures itself to achieve higher resilience to failures that
do not occur simultaneously, and which uses a heterogeneous
set of nodes to reduce the costs of operating a cluster.
Malkhi, Lamport and Zhou [9] proposed Stoppable Paxos,
an alternative method for reconfiguring clusters in which an
RSM may be stopped, reconfigured, and then restarted with
the new configuration. Stoppable Paxos improves on Dynamic
Paxos by removing the need for a pipeline limit when the
configuration is static and by allowing a different limit to be
selected for different reconfigurations.
Malkhi, Lamport and Zhou [10] then proposed Vertical
Paxos which keeps the RSM running throughout a reconfiguration, but requires a separate oracle to manage the configuration. The Egalitarian Paxos of Moraru, Andersen and
Kaminsky [11] uses a similar reconfiguration scheme. The
problem with needing an external oracle is that for full
resilience it must be possible to reconfigure the oracle itself,
which requires another oracle and so on ad infinitum.
Chandra, Griesemer and Redstone [12] noted that the details
of reconfiguration are “relatively minor” but “subtle” and do
not give any details on the reconfiguration scheme used in
their Chubby system. It seems likely that they also used the
bounded-pipeline approach. In contrast, Birman, Malkhi and
van Renesse [13] noted that allowing for α > 1 may require
an unacceptably complex implementation, leading many real-

2

world systems to disable pipelining by setting α = 1, or even
to disable reconfiguration entirely.
Bortnikov et al. [14] implemented a reconfigurable RSM
using static RSMs which may pass responsibility to each
other in a manner that is similar to that of Stoppable Paxos.
They improve the performance of their system compared with
Stoppable Paxos by allowing each RSM to speculatively start
executing commands before the transfer of responsibility has
been fully agreed.
Ongaro and Ousterhout [3] developed the Raft protocol
which supports an unbounded pipeline throughout the reconfiguration process without inconsistency by instead limiting
the reconfigurations that can be performed. They performed a
formal proof of the correctness of Raft without reconfiguration,
and an informal argument that consistency is preserved when
a single node is added or removed from the cluster. Raft uses
simple majorities of the set of nodes as its quorums.
Viewstamped Replication and Zab are two other well-known
consensus protocols. Viewstamped Replication supports reconfiguration as described by Liskov and Cowling [15] in a similar
fashion to Vertical Paxos. Reed and Junqueira [16] initially
presented Zab without reconfiguration and this feature was
subsequently added by Shraer, Reed, Malkhi and Junqueira
[17], using a limited-pipeline approach much as in Dynamic
Paxos.
An interesting alternative approach to reconfiguration was
proposed by Jahl and Meling [18] which uses eventual consistency rather than consensus to determine the configuration
of the system, and therefore supports reconfiguration as long
as it is still possible to communicate with a quorum of nodes
even if consensus cannot be achieved due to an inability to
elect a distinguished leader. In such a situation consensusbased approaches such as the one presented here would fail
to make any further progress, whereas one based on eventual
consistency could be reconfigured into one in which a leader
may be elected and thus in which further progress can be
made.
III. T HE S YNOD A LGORITHM
Synod [2] is an algorithm for achieving consensus on a
single value in a distributed system comprising a set of nodes
which can communicate by sending messages to each other.
The system is asynchronous but not Byzantine, in the sense
that messages may be delayed, reordered, duplicated and
dropped but not corrupted, and processes may run arbitrarily
slowly or even stop but may not deviate from their specifications.
Let B be a set of ballot identifiers with a wellfounded total
order ≺. Let A be a set of node identifiers and let V be the
set of values that may be chosen.
The Synod algorithm involves five kinds of message, in
two phases, as described below. Throughout, a ∈ A and
b, b′ ∈ B. Phase I starts with the broadcast of a prepare
message prepare(b) to which each node may respond with a
promise, either a free promise, written promised(a, b), or a
forced promise, written promised(a, b; b′ ), where a identifies
the responding node. Phase II starts with the broadcast of a

UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37

3

Fig. 1. Invariants preserved by the Synod algorithm

S1) For all b1 ≻ b2 ∈ B there are sets of quorums QI (b1 ) and QII (b2 ) ⊆ PA such that if proposed(b1 ) and chosen(b2 )
then QI (b1 ) ⌢ QII (b2 ).
S2) If promised(a, b) then ¬accepted(a, b′ ) for all b′ ≺ b.
S3) If promised(a, b; b′ ) then b′ ≺ b, accepted(a, b′ ), and b′ is the greatest such ballot in the sense that ¬accepted(a, b′′ )
for all b′′ having b′ ≺ b′′ ≺ b.
S4) If proposed(b) then there is a quorum q I ∈ QI (b) such that for every node a ∈ q I either promised(a, b) or else there

exists a b′ such that promised(a, b; b′ ); if also P , {b′ | ∃a ∈ q I .promised(a, b; b′ )} 6= ∅ then v(b) = v max(P ) .
S5) If accepted(a, b) then proposed(b).
S6) If chosen(b) then there is a quorum q II ∈ QII (b) such that accepted(a, b) for every a ∈ q II .

proposal, written proposed(b), to which each node a may
respond with an acceptance, written accepted(a, b). Once
phase II is complete a success message, written chosen(b),
is broadcast. It is convenient also to use these symbols
as predicates indicating whether the corresponding messages
have been sent.
There is a function v : B → V assigning a value to each
ballot, discussed in more detail in section III-A below.
The system satisfies a set of invariants listed in fig. 1, from
which it follows that consistency is guaranteed in the sense that
if chosen(b) and chosen(b′ ) then v(b) = v(b′ ) as shown by
theorem 8 in appendix B.
Each node operates as a state machine whose transitions are
caused by the receipts of messages. Each phase is considered
to be complete for a particular ballot when appropriate messages have been received from sufficiently many nodes, where
“sufficiently many” is defined in terms of sets of quorums of
nodes in the system’s configuration.
In more detail, a node may emit proposed(b) when it
considers phase I to be complete at ballot b, which is when
promises for b have been received from a quorum of nodes,
and similarly may emit chosen(b) when it considers phase
II to be complete at b, which is when acceptances of b have
been received from a quorum of nodes.
The phase-I and phase-II quorums are defined so as to
always contain at least one node in common, but may vary
depending on the ballot b and the phase as discussed in section
III-B below. In particular, the quorums need not all mutually
intersect, as discovered independently by Howard, Malkhi and
Spiegelman [19].
A. Implementing the value function
In an implementation of a RSM, the values chosen represent
the transitions that the state machines must perform. It is possible that these values may be expensive to transfer between
nodes because they could carry a large quantity of data.
In the original presentation of the Synod algorithm, the
values of ballots are carried along with their identifiers in the
messages promised(a, b; b′ , v(b′ )), proposed(b, v(b)) and
chosen(b, v(b)). This means that, in a unicast network of
2f +1 nodes, each value is included in 2f messages (at least f
proposals and at least f success messages) even in the absence
of faults, which is twice as many as necessary. Furthermore, a
simple method for detecting faults is to insist that each node

sends at least one complete message within a certain period
of time, but this method is unsatisfactory if messages can be
unboundedly large.
Observation O4 in [8] notes that these values can be
replaced in some cases by hashes, but this idea can be taken
a step further and the values can be completely elided from
the messages that take part in the Synod protocol, allowing
considerably more freedom in the implementation of the
function v without sacrificing consistency.
By allowing the values to be communicated using a separate
mechanism from the consensus messages themselves it is
possible to seek optimisations that rely on the fact that the
values may be large but need not move quickly whereas the
consensus messages are small but must be transported with
low latency to ensure the system has good performance.
Although it appears that the function v is fixed, in practice
it is allowed to change as the system runs. Treating it as
fixed simplifies the consistency proof and highlights that its
values need not be included in all messages, but means that
the system cannot be shown to satisfy any useful liveness
properties. To recover liveness, note that if the invariants of
fig. 1 are satisfied with a value function v then they continue to
be satisfied if v is replaced by another value function v ′ that
agrees with v on proposed ballots, i.e. where v(b) = v ′ (b)
if proposed(b) but not necessarily otherwise. Since only
owner(b) may propose b, if it has not yet itself proposed a
value for b it can deduce that ¬proposed(b) and therefore
freely modify v(b).
It is also important for liveness that the implementation
of v is resilient to the same failure modes as the rest of
the system. Since owner(b) is, in a sense, responsible for
the value of the ballot b, it is possible to think of v as an
insert-only set of pairs {hb, v(b)i | proposed(b)} which is
an example of a convergent replicated data type [20] and can
therefore be implemented simply and robustly in a distributed
system without needing to rely on a consensus algorithm.
Indeed the original presentation can be seen as containing
such an implementation, where the inclusion of values in all
messages ensures convergence occurs as quickly as possible,
and replicating this set across all 2f + 1 nodes ensures v itself
may be resilient to as many as 2f failures. Cheap Paxos [8]
is cheaper partly because it replicates v across just the f + 1
primary processors, with the f auxiliary processors storing just
the hashes of values to ensure integrity.
It is also worth comparing this approach to that of Vertical

UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37

Paxos [10] which takes great care to ensure that the system
state is completely transferred between nodes before they start
to participate fully in the cluster, with certain optimisations
in recognition of the fact that this state transfer could be
an expensive and time-consuming operation involving a very
large quantity of data. However if the implementation of v is
separated out then the quantity of data that must be transferred
as part of the consensus algorithm becomes small enough that
it needs no special treatment and a consensus-free technique
may be used to implement v more efficiently.
B. Per-phase quorums
The consistency property of the Synod algorithm relies on
the fact that the set of nodes involved in completing phase I
must always intersect the set of nodes involved in completing
phase II so that there is at least one node involved in both
phases. Write Q1 ⌢ Q2 iff every q1 ∈ Q1 and q2 ∈ Q2 have
q1 ∩ q2 6= ∅, and write QI (b) and QII (b) for the sets of phase-I
and phase-II quorums for ballot b respectively.
In the original presentation of the Synod algorithm any
(weighted) majority of the nodes could be used as a quorum,
so that QI (b1 ) = QII (b2 ) and hence QI (b1 ) ⌢ QII (b2 ) for all
b1 and b2 since all majorities intersect.
Theorem 8 shows that consistency can still be guaranteed
even if sometimes QI (b1 ) 6= QII (b2 ), as long as QI (b1 ) ⌢
QII (b2 ) when proposed(b1 ), chosen(b2 ) and b1 ≻ b2 , as
described in invariant S1. This weaker invariant is the key to
allowing more general reconfigurations to take place safely as
described in section IV-A below.
IV. T HE PAXOS ALGORITHM
Conceptually, Paxos is a sequence of distinct instances of
the Synod algorithm all running simultaneously. To achieve
this, the messages of the Synod algorithm above are indexed with the instance number i ∈ N: promisedi (a, b),
promisedi (a, b; b′ ), proposedi (b), acceptedi (a, b) and
choseni (b). There is another kind of message known as
a multi-promise, written promised≥i (a, b), which can be
thought of as standing for the infinite set of free promises
{promisedj (a, b) | j ≥ i}. Prepare messages prepare(b)
apply to all instances so are not indexed.
As in the Synod algorithm, phase I starts with a broadcast
of prepare(b) for some b to which each node a may respond
with a set of promises promisedi (a, b), promisedi (a, b; b′ )
and promised≥i (a, b) according to its past behaviour. Each
phase II instance i operates just as in the Synod algorithm,
starting with a broadcast of proposedi (b) to which each node
a may respond with an acceptance acceptedi (a, b) and once
acceptances have been received from a quorum of nodes it
follows that choseni (b) may be broadcast.
There is a function vi : B → V for each instance i giving
a value to each ballot, and theorem 10 shows that whenever
choseni (b) and choseni (b′ ) it follows that vi (b) = vi (b′ ).
The invariants listed in fig. 2 are roughly the same as
for many other presentations of Paxos with the addition of
constraints on the configurations associated with instances and
ballots as discussed below. Note that invariants P2, P3 and P4

4

limit the acceptances that can be sent as well as the promises,
so there is no need to define separate invariants concerning
the sending of acceptances.

A. Configuration changes
A fixed cluster configuration is a pair hQI , QII i of sets of
quorums satisfying QI ⌢ QII , where QI and QII are the sets
of quorums to use in phase I and phase II respectively.
Changes to the cluster configuration are modelled by a
sequence of configurations hQI0 , QII0 i, hQI1 , QII1 i, . . . that also
satisfy QIe ⌢ QIIe+1 for all e. The integer subscript is called
the era of a configuration. Intuitively the cluster is “in era
e” while instances are being chosen using hQIe , QIIe i. While a
change from era e to e + 1 is in progress some instances may
use the interim configuration hQIe , QIIe+1 i, and once the change
to era e + 1 is complete instances will use hQIe+1 , QIIe+1 i. This
intuition is captured more precisely in section IV-D below, and
since QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1 it follows that consistency
is preserved throughout by the observation of III-B above.
To achieve this there are also two nondecreasing integervalued functions, both written e(·), which respectively assign
an era e(i) to each instance i, and an era e(b) to each ballot
b. Intuitively e(b) records which quorums may be used in
phase I to decide that proposedi (b) can be sent, and e(i)
records which quorums may be used in phase II to decide
that choseni (b) can be sent. More precisely a node may emit
proposedi (b) only if it has received promises for ballot b in
instance i from a quorum of nodes in QIe(b) , which implies that
e(b) ≤ e(i), and similarly a node may emit choseni (b) only
if e(i) ≤ e(b) + 1 and it has received acceptedi (a, b) from a
quorum of nodes in QIIe(i) . These extra conditions on the eras
of ballots and instances in messages ensure that if choseni (b)
then e(b) ≤ e(i) ≤ e(b) + 1 and hence QIe(b) ⌢ QIIe(i) as
required to ensure consistency, as shown in lemma 9.

B. Dynamic configuration changes
As
in
Dynamic
Paxos,
the
configurations
hQI0 , QII0 i, hQI1 , QII1 i, . . . and era numbers e(i) are themselves
chosen by consensus. In contrast, era numbers e(b) for ballots
b are fixed in advance and not chosen by consensus.
In more detail, configurations are held within the RSM as
a finite sequence hQI0 , QII0 i, hQI1 , QII1 i, . . . , hQIemax , QIIemax i and
the era numbers of instances are held similarly as a nondecreasing sequence e(0), e(1), . . . , e(imax ). The transitions that
affect these sequences may append one or more elements, but
may not change any existing elements.
The sequences are always long enough to make progress,
in the sense that emax ≥ e(imax ) and if chosenj (b) for all
j < i then imax ≥ i.
Note that a node may emit a promise for b at instance i
only if e(b) ≤ e(min(i, imax )). This allows nodes to make
promises for an instance i even if e(i) is not yet known, i.e. if
i > imax . Invariant P7 requires that i ≤ imax , and hence e(i)
is known, before choseni (b).

UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37

5

Fig. 2. Invariants preserved by the Paxos algorithm

P1) There are configurations hQI0 , QII0 i, hQI1 , QII1 i, . . . where QIIe ⌢ QIe ⌢ QIIe+1 for each e.
P2) If promised≥i (a, b) then e(b) ≤ e(min(i, imax )) and ¬acceptedj (a, b′ ) for all j ≥ i and all b′ ≺ b.
P3) If promisedi (a, b) then e(b) ≤ e(min(i, imax )) and ¬acceptedi (a, b′ ) for all b′ ≺ b.
P4) If promisedi (a, b; b′ ) then e(b) ≤ e(min(i, imax )), b′ ≺ b, acceptedi (a, b′ ), and b′ is the greatest such ballot in the
sense that ¬acceptedi (a, b′′ ) for all b′′ having b′ ≺ b′′ ≺ b.
P5) If proposedi (b) then there is a quorum q ∈ QIe(b) such that for every a ∈ q one of the following holds:
• promised≥j (a, b) for some j ≤ i, or
• promisedi (a, b), or
′
′
• promisedi (a, b; b ) for some b .

Furthermore if P , {b′ | ∃a ∈ q.promisedi (a, b; b′ )} 6= ∅ then vi (b) = vi max(P ) .
P6) If acceptedi (a, b) then proposedi (b).
P7) If choseni (b) then i ≤ imax , e(i) ≤ e(b) + 1, and there is a quorum q ∈ QIIe(i) with acceptedi (a, b) for every a ∈ q.

C. Liveness
To be useful, a consensus algorithm must not only guarantee
consistency but also ensure that it cannot “get stuck”, i.e. it
is always possible to make progress by eventually choosing
a value for each instance. It is known to be impossible to
guarantee liveness in a deterministic asynchronous system [21]
but as with earlier presentations of Paxos [7] here liveness can
be shown under the assumption that a distinguished node ℓ is
eventually selected as the only one that may emit prepare(b)
messages.
The original liveness proof then proceeded by having ℓ emit
prepare(b) for some b that is chosen to be large enough that a
quorum of nodes may respond with promises. In contrast, here
there is an upper bound on suitable ballots since a proposed
ballot must not belong to an era which is too large, because
if e(b) > e(i) then no promise for ballot b at instance i may
be made. Therefore here ℓ must be able to choose a ballot
that is large enough to be accepted but which still belongs to
the correct era, or, more precisely, for each ballot b, each era
e ≥ e(b) and each node a there must be a ballot b′ ≻ b having
e(b′ ) = e and owner(b′ ) = a.
This means that ballot numbers cannot be simple integers
because this would imply that there exist eras containing
only finitely many ballots. Instead an implementation could,
for example, let B = N × N × A ordered lexicographically,
where e(he, n, ai) , e and owner(he, n, ai) , a. This is
the approach used in Egalitarian Paxos [11] in which eras
are known as epochs but this terminology is avoided here
to prevent confusion with the epochs (views, terms, . . . ) of
leader-election protocols (e.g. [22]) which track the current
leader rather than the current configuration.
With this in mind the proof of liveness runs much as in the
original presentation:
Theorem 1 (Liveness). Given that there is eventually a
nonfaulty distinguished node ℓ which is the only node that may
emit prepare messages, and sufficiently many other nonfaulty
nodes, and given that for every instance there is eventually at
least one value to propose, then eventually a value is chosen
for every instance.
Proof. The proof proceeds by induction over the instances,

so suppose that chosenj (bj ) for all j < i and show that
eventually choseni (bi ) as follows.
Firstly recall that the sequences of eras and configurations
are long enough, in the sense that i ≤ imax and e(imax ) ≤
emax , so that the values of e(i), QIe(i)−1 , QIe(i) and QIIe(i) are
known to ℓ.
The distinguished node ℓ first chooses a ballot bi having
e(bi ) ∈ {e(i) − 1, e(i)} and owner(bi ) = ℓ and such that
enough nodes can emit acceptedi (a, bi ) without breaking
any of their previously-made promises. By invariants P2, P3
and P4, all promises for such a ballot b′ at instance i must
have e(b′ ) ≤ e(i) so that such a bi does exist.
If ℓ has not yet received enough promises for bi at instance i
then it broadcasts prepare(bi ) and waits to receive promises
from a quorum of nodes in QIe(bi ) .
Then, if ℓ has not yet emitted proposedi (bi ) it selects
one of the values for instance i (which eventually exists), sets
vi (bi ) as appropriate, broadcasts proposedi (bi ), and waits
to receive acceptances in response. When acceptances have
been received from a quorum of nodes in QIIe(i) it follows that
choseni (bi ) as required.
D. Fully concurrent configuration changes
The discussion so far shows that, like other Paxos variants,
this algorithm satisfies consistency and liveness properties.
The benefit of this scheme compared with other variants
is that, under normal running conditions, it is possible to
perform a reconfiguration without needing to impose a limit
on the number of concurrently-running instances. This section
describes the details of this procedure.
In normal running there is an instance i0 and a ballot
b with e(b) = e(i0 ) = e(imax ) = emax and the distinguished
node ℓ = owner(b) has received a quorum of promises
q I ∈ QIe(b) for ballot b for all instances i ≥ i0 . In this state,
ℓ may emit proposedi (b) for any i ≥ i0 as long as vi (b) is
set appropriately. The node ℓ is known as the leader and its
proposals are normally accepted without undue delay by all
other nodes.
Suppose that, in normal running, an operator wishes
to change the cluster configuration to hQInew , QIInew i where
QIe(b) ⌢ QIInew ⌢ QInew . First she appends hQInew , QIInew i to

UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37

the sequence of configurations, setting hQIe(b)+1 , QIIe(b)+1 i =
hQInew , QIInew i and emax = e(b) + 1, then she picks a future
instance ic > imax at which the change should take effect and
appends values to the sequence of eras to set e(i) = e(i0 ) for
i0 ≤ i < ic and e(ic ) = e(i0 ) + 1. Since e(ic ) = e(i0 ) + 1 ≤
e(b)+1, values for instance ic and any future instances with the
same era may be proposed and chosen even though phase I has
not yet run for a ballot in this era, so this does not prevent any
further instances from running concurrently. There is therefore
no drawback to choosing as small a value for ic as possible,
so it makes sense to choose ic = imax + 1.
At this point, the system is no longer in normal running as
defined above because e(b) = e(imax )−1. If the operator were
to increase e(imax ) any further then there would be an instance
i with e(i) > e(b) + 1 and hence ¬choseni (b). A value can
still eventually be chosen for instance i due to theorem 1,
but not before the leader has selected a new ballot b′ in an
appropriate era, completed phase I for b′ , and then broadcast
new proposals for b′ . These steps may cause the pipeline to
stall if not completed quickly enough.
The system must therefore be returned to normal running
before any further reconfiguration can occur. To do this, the
leader chooses a new ballot b′ having e(b′ ) = e(b) + 1
and owner(b′ ) = ℓ and runs phase I for b′ in a way that
does not prevent any progress in era e(b) while it has not
completed. This is possible if ℓ has a casting vote in the
sense that there are quorums of nonfailed nodes q ∈ QIIe(b)
and q ′ ∈ QIe(b)+1 having q ∩ q ′ = {ℓ}. With a casting vote,
ℓ may broadcast prepare(b′ ) just to the nodes in q ′ \ {ℓ}
without preventing further progress in era e(b) since the nodes
in q can continue to accept proposals in this era throughout.
When it has received promises from all the other nodes in q ′ it
can send itself promised≥i′ (ℓ, b′ ) for some sufficiently large
i′ , which completes phase I at b′ and restores the system to
normal running in era e(b′ ) = e(b) + 1. A message from a
node to itself does not incur any network delays so the last
step occurs essentially instantaneously.
If ℓ does not have a casting vote, but there is some other
node ℓ′ which does, then ℓ should first abdicate its leadership
to ℓ′ and then the new leader should perform the new phase I
as described above. If there is no node with a casting vote at
all then the broadcast of prepare(b′ ) may prevent progress
until phase I is complete at b′ .
If any node fails during this process then it may be necessary
to retry some of the steps or possibly even elect a new leader.
Liveness and consistency continue to hold if nodes fail but
performance may be affected, for instance by meaning that ℓ
no longer has a casting vote. In general it is not possible to
prevent node failures from having a performance impact.
To give a concrete example of this process, fig. 3 shows the
flow of messages during a reconfiguration involving the nodes
a1 and a2 and the leader ℓ. The system starts in era e where
{ℓ, a1 } ∈ QIe and {ℓ, a2 } ∈ QIIe , and moves to era e + 1 where
{ℓ, a1 } ∈ QIe+1 and {ℓ, a2 } ∈ QIIe+1 too. Because of these
quorums, the leader only needs a response from node a1 to
complete phase I, and similarly only needs a response from
node a2 to complete phase II and choose a value. Notice that

6

a1

ℓ

a2

prepare(b)

proposedi (b)
proposedi+1 (b)
proposedi+2 (b)
proposedi+3 (b)

era e

proposedi+4 (b)

era e + 1

proposedi+5 (b)
proposedi+6 (b)

prepare(b′ )

proposedi+7 (b)
proposedi+8 (b)
proposedi+9 (b)
proposedi+10 (b)
proposedi+11 (b)
proposedi+12 (b′ )
proposedi+13 (b′ )

Fig. 3. Message flow during a reconfiguration.

this means the leader has a casting vote. The messages sent
from ℓ to the other nodes are labelled on the diagram, but the
successful responses (promises and acceptances from a1 and
a2 respectively) are left unlabelled for clarity. Initially, imax =
i + 3 and e(i) = e(i + 1) = e(i + 2) = e(i + 3) = e = emax ,
and all instances before i have already been chosen.
The leader starts by completing phase I at a sufficiently
large ballot b, where e(b) = e, which starts a period of normal
running in era e and means that ℓ may emit proposedj (b)
for any j ≥ i. It receives three client requests, causing it to
propose values for instances i, i + 1 and i + 2 in turn. On
the receipt of each proposal the node a2 responds with an
acceptance, which when ultimately received by ℓ allows it to
decide that each value is chosen because {ℓ, a2 } ∈ QIIe .
A configuration change is proposed at instance i + 3 which,
when chosen, appends the next configuration hQIe+1 , QIIe+1 i to
the configuration sequence, sets e(i + 4) = e(i + 5) = . . . =
e + 1, and increases emax and imax accordingly. This takes the
system out of normal running because now e(imax ) = e + 1 6=
e(b). In order to bring the system back into normal running,
the leader must choose a ballot b′ having e(b′ ) = emax and
complete a phase I at b′ , so it sends prepare(b′ ) to a1 .
While instance i+3 was being chosen, the leader continued
to service client requests by sending out proposedi+4 (b),
proposedi+5 (b) and proposedi+6 (b), which a2 accepts in

UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37

due course. Although these instances come after the configuration change at instance i + 3, when the leader receives
acceptedi+4 (a2 , b) it may still safely deduce choseni+4 (b)
since e(i + 4) = e + 1 ≤ e(b) + 1 and {ℓ, a2 } ∈ QIIe+1 , and
similarly for instances i + 5 and i + 6. It is important to notice
that the leader is now using QIIe+1 and not QIIe to determine
when ballots are chosen. The horizontal dashed line shows the
point in time at which the leader moves from era e to era e+1.

7

a1

ℓ

prepare(b)

While the leader is waiting for the response from a1 , client
requests continue to arrive, yielding proposals for subsequent
instances i + 7, i + 8, . . . . Notice that the leader is still
using ballot b for these requests as it has not yet completed
phase I at b′ , but that it is still safe to deduce choseni+7 (b),
choseni+8 (b), . . . because e(i + 7) = e(i + 8) = e(b) + 1.
Importantly, there is no limit to how many requests the leader
can handle in this way, so the system will continue to be able
to process client requests even if the phase I messages are
arbitrarily delayed.
At last the response from a1 is received just after the sending
of proposedi+11 (b) and before a proposal has been made for
instance i + 12. The leader can then send itself the message
promised≥i+12 (ℓ, b′ ) which completes phase I at ballot b′
since {ℓ, a1 } ∈ QIe+1 . This restores the system to normal
running, and allows ℓ to make proposals proposedj (b′ ) for
all j ≥ i + 12. It is imporant to notice that the leader did
not need to predict how long it would take to complete phase
I at b′ in advance, nor how many requests it might have to
handle during this time, in order to ensure that it can continue
to process these requests without delay.
Fig. 4 shows an equivalent reconfiguration performed in a
Dynamic Paxos cluster with the pipeline length α = 2. As
there are frequently two proposals being processed concurrently it seems likely that better performance could be achieved
by selecting a higher value. A higher value still could have
avoided the pause between instances i + 5 and i + 6 caused by
the extra delay in completing phase I at ballot b′ . On the other
hand if the pipeline is too long then configuration changes can
be expensive to complete. It is, in general, difficult to select
an appropriate value for α up-front as the best choice may
depend on changeable system conditions, and no matter what
value is selected it is possible that a configuration change may
cause a pause if a phase-I message is unexpectedly delayed or
a burst of client requests are received.
In contrast, fig. 5 shows an equivalent reconfiguration
performed in Stoppable Paxos. This variant of Paxos allows
for an unlimited number of proposals to run in parallel
within each configuration, and permits out-of-order execution,
so in this illustration the stopping command is proposed at
instance i+3 before proposals are made for the two preceding
instances. By selecting instance i + 3 for the reconfiguration,
the operator is limiting the system to service at most two more
client requests before the reconfiguration completes. As in the
illustration of Dynamic Paxos above, the operator’s choice is
too conservative so the remaining two free instances are used
up before phase I is completed at ballot b′ , which causes the
system to temporarily suspend its processing of client requests.

a2

proposedi (b)
proposedi+1 (b)

proposedi+2 (b)
proposedi+3 (b)

proposedi+4 (b)
proposedi+5 (b)

′

prepare(b )

era e
era e + 1

proposedi+6 (b′ )
proposedi+7 (b′ )

Fig. 4. Message flow for Dynamic Paxos with α = 2.

V. E XAMPLES
This section contains some examples of configuration
changes that satisfy the conditions described above. All the
configurations described here have equal sets of quorums in
phase I and phase II of each era, so for the sake of simplicity
throughout this section define Qe , QIe = QIIe . Implementations can ensure Qe ⌢ Qe for each e by, for instance,
arranging for each quorum in Qe to comprise a majority subset
of some finite set of nodes. Slightly more generally, let a
weight function be a function w : A → N that only takes
finitely many nonzero values. This can be used to define a
configuration hM (w), M (w)i by weighted majority:
(
)
X
X
M (w) , q
2w(a) >
w(a) .
a∈q

a∈A

Corollary 5 in appendix A demonstrates the well-known result
that M (w) ⌢ M (w) for any weight function w. Indeed, if w
and w′ are weight functions that differ by a constant factor
in the sense that that there are positive integers k and k ′ with
kw(a) = k ′ w′ (a) for all a, then clearly M (w) = M (w′ ) and
hence M (w) ⌢ M (w′ ).
Raft’s quorums are simple unweighted majorities of a finite
set of nodes, which can be emulated with weight functions
that only take values in {0, 1}. It only supports adding or

UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37

a1

ℓ

a2

8

functions starting at era e:
node
we
we+1
we+2

prepare(b)

proposedi (b)
proposedi+3 (b)
proposedi+1 (b)
proposedi+2 (b)

era e
era e + 1
prepare(b′ )

proposedi+4 (b′ )
proposedi+5 (b′ )
proposedi+6 (b′ )

Fig. 5. Message flow for Stoppable Paxos.

removing a single node from this set which amounts to
changing the weight of a single node by ±1. For instance,
if A = {a1 , a2 , . . .} then define weight functions
(
1 ifa ∈ {a1 , . . . , an }
1...n
w
(a) ,
0 otherwise
and
observe
that
M (w1...3 ) ⌢ M (w1...4 )
and
1...4
1...5
M (w
) ⌢ M (w
) but M (w1...3 ) 6⌢ M (w1...5 ) because
{a1 , a2 } ∈ M (w1...3 ) and {a3 , a4 , a5 } ∈ M (w1...5 ) do
not intersect. This justifies the restriction against adding or
removing more than one node at once.
In fact there is no need to restrict attention just to weight
functions taking values in {0, 1} as shown by lemma 3 which
is reminiscent of the amoeba analogy in [8]: any two integervalued weight functions whose total absolute difference is at
most one can be used to define consecutive configurations.
This extra generality is important for deployments where nodes
may share infrastructure (e.g. power distribution or network
connectivity) because such nodes may suffer correlated failures, and reducing this correlation by adding more independent
infrastructure may be costly. In more detail, a naı̈ve approach
to swapping a node aold for a replacement anew in a threenode cluster using unweighted majorities would be to perform
the configuration changes given by this sequence of weight

aold
1
1
0

anew
0
1
1

a1
1
1
1

a2
1
1
1

However to be resilient to infrastructure failures this requires
all four nodes and their underlying infrastructures to be
completely independent since a correlated failure of any two
nodes would prevent further progress. It also has no node with
a casting vote in era e + 1. On the other hand the following
sequence achieves the same overall change but allows aold and
anew to share infrastructure without extra risk, and both a1 and
a2 have casting votes throughout:
node
we
we+1
we+2
we+3
we+4
we+5
we+6

aold
1
2
2
1
0
0
0

anew
0
0
1
1
1
2
1

a1
1
2
2
2
2
2
1

a2
1
2
2
2
2
2
1

This is important as in many operating environments it may be
too expensive or complicated to arrange for four independent
infrastructures particularly if the fourth is only required to
ensure consistency in relatively rare periods of maintenance.
For instance at time of writing only one Amazon Web Services
region (us-east-1) has four independent zones, whereas
four of them have three: ap-southeast-2, eu-west-1,
sa-east-1 and us-west-2. Similarly, only one Google
Cloud Platform region (us-central1) has four zones and
all the others have three zones.
Early versions of Raft supported more general reconfigurations using a technique known as joint configurations. To
change from configuration Qe to an unrelated configuration
Q′ (i.e. Qe 6⌢ Q′ ⌢ Q′ ) it is possible to set Qe+2 = Q′ and
set Qe+1 to be the joint configuration of Qe and Q′ :
Qe+1 = {q ∪ q ′ | q ∈ Qe , q ′ ∈ Q′ }
as this satisfies that Qe ⌢ Qe+1 ⌢ Qe+1 ⌢ Q′ .
If w(a) = 0 for all a then w is said to be weightless
and M (w) = ∅. Clearly if there is an instance i such that
Qe(i) = ∅ then no value can ever be chosen for i. On the other
hand Q ⌢ ∅ for all Q so changing to or from a weightless
configuration is always permitted.
There is no requirement for the eras of consecutive instances
to differ by at most one so an era may be skipped at instance
i + 1 by setting e(i + 1) = e(i) + 2. This recovers the ability
to perform arbitrary configuration changes in a single step as
in Stoppable Paxos. In more detail, if the system is currently
using configuration Qe and an operator wishes to change to
an unrelated configuration Q′ then she can set Qe+2 = Q′
and pick an appropriate Qe+1 (such as ∅) which satisfies that
Qe ⌢ Qe+1 ⌢ Qe+1 ⌢ Q′ . However, re-running phase I can
only be delayed as described in section IV-D above if the era
increases by 1 and in this situation the era increases by 2, so
a new phase I must be completed before phase II of any new

UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37

instances can be started. To prevent this causing the pipeline
to stall, the operator chooses a sufficiently large α > 0 and
sets e(i + α) = e(i) + 2 and e(j) = e(i) for i < j ≤ i + α,
effectively delaying the configuration change for α instances
in the hope that this is long enough to have completed the
new phase I. The pattern of communication in this situation
is very similar to that shown in fig. 5 where α was chosen to
be 3.

9

P
′ ′
Let dA , k ′ w′ (a0 ) − kw(a0 ) so that
a∈A k w (a) =
P
a∈A kw(a) + dA and |dA | ≤ 1. Also let
(
dA a 0 ∈ q ′
dq ′ ,
0
otherwise,
P
P
so that a∈q′ k ′ w′ (a) = a∈q′ kw(a) + dq′ . Then
X
2kw(a) + dA + k + k ′
a∈A

VI. C ONCLUSION
The approach described here improves on Dynamic Paxos
[8] by supporting changing the pipeline length parameter
α and running with an unlimited-length pipeline while a
configuration change is not in progress. In that sense, it can
be compared to that of Stoppable Paxos [9] which allows for
an unlimited number of proposals to run in parallel within
each configuration, but requires a temporary arbitrary limit on
concurrency while a reconfiguration takes place.
In both Dynamic and Stoppable Paxos, if the selected
limit is either too small or too large then it may affect the
system’s performance, and the best choice of limit depends
on a prediction of the system’s future performance. The
approach described here avoids the need to make any such
prediction or select any such limit and responds to changing
system conditions without needing further tuning. Once a
reconfiguration is chosen, it can complete after a single roundtrip to a quorum of nodes and the system can continue to serve
all clients while this round-trip is in progress, no matter how
long it takes.
This is achieved by using Raft-style reconfigurations [3]
which can be performed with an unlimited pipeline throughout.
Unlike in Raft, here a configuration change only takes effect
once it is chosen, which avoids the need to back-track to
an earlier state if a leader fails during reconfiguration. It
generalises the simple majorities used in Raft to integerweighted majorities which can reduce the costs of dealing with
correlated failures during maintenance.
It achieves equivalent goals to those of Vertical Paxos [10]
except that here there is no requirement for a separate oracle
to manage the configuration of the system.
It is also noted that there is no need for every message to
include the corresponding value, or even a hash of the value,
which may allow for even cheaper implementations of Cheap
Paxos [8] and can simplify the transfer of state [10] required
when new nodes are commissioned.
A PPENDIX A
′

Lemma 2. If w, w : A → N are
functions and k, k ′
P weight
′ ′
are positive integers such that a∈A |k w (a) − kw(a)| ≤ 1
then M (w) ⌢ M (w′ ).
Proof. Since kw(a) and k ′ w′ (a) are integers for all a, there
must be a node a0 such that kw(a) = k ′ w′ (a) for all a 6= a0 .
Let q ∈ M (w) and q ′ ∈ M (w′ ). By the definition
of M ,
P
′
and
since
w
and
w
take
only
integer
values,
2w(a)
≥
P a∈q′
P
P
′
w
(a)
+
1.
2w
(a)
≥
w(a)
+
1
and
′
a∈A
a∈q
a∈A

=k

X

!

w(a) + 1

a∈A

≤

X

2kw(a) +

a∈q

=

X

2kw(a) +

X

X

a∈A

′

!

w (a) + 1

a∈A

2k ′ w′ (a)

X

2kw(a) + 2dq′

a∈q ′

2kw(a) +

a∈q∪q ′

≤

X

a∈q ′

a∈q

=

X

+k

′

2kw(a) +

X

2kw(a) + 2dq′

a∈q∩q ′

X

2kw(a) + 2dq′

a∈q∩q ′

P
so that a∈q∩q′ 2kw(a) ≥ dA +k+k ′ −2dq′ = k+k ′ ±dA ≥ 1
and hence q ∩ q ′ 6= ∅ as desired.
Lemma 3. If w, w′ : A → N are weight functions such that
P
′
′
a∈A |w (a) − w(a)| ≤ 1 then M (w) ⌢ M (w ).

Proof. By lemma 2 with k = k ′ = 1.

Lemma 4. If w, w′ : A → N are weight functions and k, k ′
are positive integers such that k ′ w′ (a) = kw(a) for all a then
M (w) ⌢ M (w′ ).
P
Proof. By lemma 2, since a∈A |k ′ w′ (a) − kw(a)| = 0.
Corollary 5. If w : A → N is a weight function then
M (w) ⌢ M (w).
Proof. By lemma 4 with w′ = w and k ′ = k.
A PPENDIX B
C ONSISTENCY OF THE S YNOD ALGORITHM
Lemma 6. If accepted(a, b2 ), promised(a, b1 ; b3 ) and
b2 ≺ b1 then b2  b3 .
Proof. From invariant S3 it follows that b3 is the largest ballot
such that b3 ≺ b1 and accepted(a, b3 ), but b2 is also such a
ballot and therefore b2  b3 as required.
Lemma 7. If chosen(b2 ), proposed(b1 ) and b2 ≺ b1 then
v(b1 ) = v(b2 ).
Proof. Suppose for a contradiction that v(b1 ) 6= v(b2 ) and
since ≺ is wellfounded suppose without loss of generality that
b1 is the minimal such ballot. Since chosen(b2 ) by invariant
S6 there is a quorum q II ∈ QII (b2 ) such that accepted(a, b2 )
for every a ∈ q II . By invariant S2 it cannot be that
promised(a, b1 ) for any a ∈ q II . Also since proposed(b1 )
by invariant S4 there is a quorum q I ∈ QI (b1 ) such that either
promised(a, b1 ) or ∃b′ .promised(a, b1 ; b′ ) for all a ∈ q I .
Let P , {b′ | ∃a ∈ q I .promised(a, b1 ; b′ )}. By invariant S1,

UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37

QI (b1 ) ⌢ QII (b2 ) and hence q I ∩q II 6= ∅ so it follows that P 6=
∅, which means that v(b1 ) = v(max(P )) by invariant S4.
Let amax ∈ q I be such that promised(amax , b1 ; max(P )).
By invariant S3 it follows that max(P ) ≺ b1 and also that
accepted(amax , max(P )) and hence proposed(max(P ))
by invariant S5. Furthermore by lemma 6 it follows that
b2  max(P ) and since b1 was assumed to be the smallest
counterexample it must be that v(max(P )) = v(b2 ). Hence
v(b1 ) = v(b2 ) which is a contradiction as required.
Theorem 8. If chosen(b1 ) and chosen(b2 ) then v(b1 ) =
v(b2 ).
Proof. Without loss of generality assume that b2 ≺ b1 . By
invariant S6 there is a quorum q ∈ QII (b1 ) such that
accepted(a, b1 ) for every node a ∈ q and therefore
proposed(b1 ) by invariant S5. Therefore by lemma 7 it
follows that v(b1 ) = v(b2 ) as required.
A PPENDIX C
C ONSISTENCY OF THE PAXOS ALGORITHM
Lemma 9. For b1 ≻ b2 ∈ B, if proposedi (b1 ) and
choseni (b2 ) then QIe(b1 ) ⌢ QIIe(i) .
Proof. proposedi (b1 ) implies that promised≥i′ (a, b1 ) or
promisedi′ (a, b1 ) or promisedi′ (a, b1 ; b′ ) for some node
a and some i′ ≤ i with e(b1 ) ≤ e(min(i′ , imax )). Therefore e(b1 ) ≤ e(i) since e is nondecreasing. It follows that
e(i) ≤ e(b2 ) + 1 ≤ e(b1 ) + 1 ≤ e(i) + 1 since choseni (b2 )
so that e(i) ∈ {e(b1 ), e(b1 ) + 1} and hence QIe(b1 ) ⌢ QIIe(i) by
invariant P1.
Theorem 10. If choseni (b1 ) and choseni (b2 ) then
vi (b1 ) = vi (b2 ).
Proof. If choseni (b1 ) then the Paxos invariants imply the
Synod invariants for instance i. In more detail, let
QI (b)
QII (b)
promised(a, b)
promised(a, b; b′ )
proposed(b)
accepted(a, b)
chosen(b)
v(b)

, QIe(b)
, QIIe(i)
, promisedi (a, b)
∨ ∃i′ ≤ i.promised≥i′ (a, b)
, promisedi (a, b; b′ )
, proposedi (b)
, acceptedi (a, b)
, choseni (b) and
, vi (b).

Synod’s invariant S1 follows from lemma 9 and the remaining
invariants are simple to show so by theorem 8 it follows that
vi (b1 ) = vi (b2 ) as required.
ACKNOWLEDGMENT
The author would like to thank Leslie Lamport, Dahlia
Malkhi and Leander Nikolaus Jehl for their encouragement
and comments on earlier drafts of this paper. The author is
also very grateful to Tracsis plc for supporting this work.

10

R EFERENCES
[1] B. W. Lampson, “How to build a highly available system using consensus,” in Proceedings of the 10th International Workshop on Distributed
Algorithms, ser. WDAG ’96. London, UK, UK: Springer-Verlag, 1996,
pp. 1–17.
[2] L. Lamport, “The part-time parliament,” ACM TRANSACTIONS ON
COMPUTER SYSTEMS, vol. 16, no. 2, pp. 133–169, 1998.
[3] D. Ongaro and J. Ousterhout, “In search of an understandable consensus
algorithm,” in Proceedings of the 2014 USENIX Conference on USENIX
Annual Technical Conference, ser. USENIX ATC’14. Berkeley, CA,
USA: USENIX Association, 2014, pp. 305–320.
[4] J. R. Lorch, A. Adya, W. J. Bolosky, R. Chaiken, J. R. Douceur, and
J. Howell, “The smart way to migrate replicated stateful services,”
in Proceedings of the 2006 EuroSys Conference. Leuven, Belgium:
Association for Computing Machinery, Inc., April 2006, p. 103115.
[5] L. Lamport, D. Malkhi, and L. Zhou, “Reconfiguring a state machine,”
Microsoft Research, Tech. Rep. MSR-TR-2008-198, February 2008.
[6] T. Nipkow, M. Wenzel, and L. C. Paulson, Isabelle/HOL: A Proof
Assistant for Higher-order Logic. Berlin, Heidelberg: Springer-Verlag,
2002.
[7] L. Lamport, “Paxos made simple,” ACM SIGACT News, vol. 32, no. 4,
pp. 18–25, 2001.
[8] L. Lamport and M. Massa, “Cheap paxos,” 2004.
[9] L. Lamport, D. Malkhi, and L. Zhou, “Stoppable paxos,” Microsoft
Research, Tech. Rep. MSR-TR-2008-197, April 2008.
[10] ——, “Vertical paxos and primary-backup replication,” in Proceedings
of the 28th ACM Symposium on Principles of Distributed Computing,
ser. PODC ’09. New York, NY, USA: ACM, 2009, pp. 312–313.
[11] I. Moraru, D. G. Andersen, and M. Kaminsky, “There is more consensus
in egalitarian parliaments,” in Proceedings of the Twenty-Fourth ACM
Symposium on Operating Systems Principles, ser. SOSP ’13. New York,
NY, USA: ACM, 2013, pp. 358–372.
[12] T. D. Chandra, R. Griesemer, and J. Redstone, “Paxos made live: An
engineering perspective,” in Proceedings of the Twenty-sixth Annual
ACM Symposium on Principles of Distributed Computing, ser. PODC
’07. New York, NY, USA: ACM, 2007, pp. 398–407.
[13] K. Birman, D. Malkhi, and R. van Renesse, “Virtually synchronous
methodology for dynamic service replication,” Tech. Rep., 2010.
[14] V. Bortnikov, G. Chockler, A. Roytman, S. Shachor, I. Shnayderman,
and D. Perelman, Reconfigurable state machine replication from nonreconfigurable building blocks. ACM, 2012, pp. 93–94.
[15] B. Liskov and J. Cowling, “Viewstamped replication revisited,” MIT,
Tech. Rep. MIT-CSAIL-TR-2012-021, Jul. 2012.
[16] B. Reed and F. P. Junqueira, “A simple totally ordered broadcast protocol,” in Proceedings of the 2Nd Workshop on Large-Scale Distributed
Systems and Middleware, ser. LADIS ’08. New York, NY, USA: ACM,
2008, pp. 2:1–2:6.
[17] A. Shraer, B. Reed, D. Malkhi, and F. P. Junqueira, “Dynamic reconfiguration of primary/backup clusters,” in Presented as part of the 2012
USENIX Annual Technical Conference (USENIX ATC 12). Boston,
MA: USENIX, 2012, pp. 425–437.
[18] L. Jehl and H. Meling, Distributed Computing and Networking: 15th
International Conference, ICDCN 2014, Coimbatore, India, January 47, 2014. Proceedings. Berlin, Heidelberg: Springer Berlin Heidelberg,
2014, ch. Asynchronous Reconfiguration for Paxos State Machines, pp.
119–133.
[19] H. Howard, D. Malkhi, and A. Spiegelman, “Flexible Paxos: Quorum
intersection revisited,” ArXiv e-prints, Aug. 2016.
[20] M. Shapiro, N. Preguiça, C. Baquero, and M. Zawirski, Stabilization,
Safety, and Security of Distributed Systems: 13th International Symposium, SSS 2011, Grenoble, France, October 10-12, 2011. Proceedings.
Berlin, Heidelberg: Springer Berlin Heidelberg, 2011, ch. Conflict-Free
Replicated Data Types, pp. 386–400.
[21] M. J. Fischer, N. A. Lynch, and M. S. Paterson, “Impossibility of
distributed consensus with one faulty process,” J. ACM, vol. 32, no. 2,
pp. 374–382, Apr. 1985.
[22] D. Malkhi, F. Oprea, and L. Zhou, “Omega meets paxos: Leader election
and stability without eventual timely links,” in 19th Intl. Symposium
on Distributed Computing (DISC 05), no. MSR-TR-2005-93. Cracow, Poland: European Association for Theoretical Computer Science,
September 2005, p. 25.

