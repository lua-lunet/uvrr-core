UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37                                                                      1




                     Unbounded Pipelining in Dynamically
                        Reconfigurable Paxos Clusters
                                                                    David C. Turner



   Abstract—Consensus is an essential ingredient of a fault-                        It is normally necessary to be able to dynamically recon-
tolerant distributed system systems. When equipped with a                        figure a cluster by adding or removing nodes while it is
consensus algorithm a distributed system can act as a replicated                 running, in order that parts of the system can be repaired
state machine (RSM), duplicating its state across a cluster
of redundant components to avoid the failure of any single                       or replaced without needing to take the whole system of-
component leading to a system-wide failure. Paxos and Raft                       fline. It is crucial that all participating nodes agree on the
are examples of algorithms for achieving distributed consensus.                  cluster configuration, and this can be achieved by holding the
Practical implementations of this kind of system must support                    configuration within the RSM itself and using the consensus
dynamic reconfiguration in order to be able to replace failed                    algorithm to choose special reconfiguration commands when
components and perform other administrative tasks without
downtime. Paxos can achieve high performance by pipelining                       a configuration change is desired.
(starting work on new requests before existing requests have com-                   In Paxos, each value is chosen using a conceptually-separate
pleted) but typically bounds the length of the pipeline to ensure                instance of a two-phase consensus protocol known as Synod.
consistency during reconfiguration. Raft also supports pipelining                The full Paxos algorithm essentially runs an infinite sequence
and imposes no such bound on concurrent requests, preserving                     of Synod instances in parallel, using uniformity of the in-
consistency instead by restricting which reconfigurations may be
performed. This article shows how to extend Paxos to support                     stances to do so without requiring infinite time or resources. It
a more general form of reconfiguration which subsumes the                        starts by running phase I of all instances at once and then runs
original bounded-pipeline approach as well as Raft-like fully-                   phase II of each instance in turn to yield the desired sequence
concurrent reconfigurations and more besides.                                    of chosen values. It normally continues to run phase II for
   Index Terms—Distributed algorithms, fault tolerance.                          extended periods of time, but will return to phase I if certain
                                                                                 nodes become faulty, or if messages between certain pairs of
                       I. I NTRODUCTION                                          nodes cease to be delivered reliably for a period, or if the
                                                                                 configuration changes. Raft’s pattern of execution is similar.
R     ELIABLE distributed systems must be able to tolerate
      a fault in any individual component without suffering
a system-wide failure, and typically achieve this by ensuring
                                                                                    Both algorithms can achieve high throughput by allowing
                                                                                 for pipelining [4] whereby work may begin on an instance
that there is redundancy between the components. A replicated                    even before all previous instances have fully completed. It is
state machine (RSM) is a style of fault-tolerant distributed                     a little tricky to ensure that this preserves consistency when
system in which a deterministic state machine is replicated                      the configuration is held in the RSM itself because a value
across a set of distinct nodes [1]. Being deterministic, the                     may only be proposed once a quorum of nodes are ready for
nodes’ states remain synchronised if they all start in the same                  it, but there may be a configuration change in the pipeline
state and perform the same sequence of transitions.                              which would change the quorums so as to make a proposal
   In order to arrange for each node to perform the same                         invalid. If this case is not handled carefully then it may lead
transitions the system may achieve consensus on (or choose)                      to inconsistency. Paxos implementations typically solve this
a sequence of values which describe the transitions. The                         problem by limiting the length of the pipeline to some α > 0
sequence must be consistent across the whole system even in                      and requiring that a configuration change chosen at instance i
the presence of failures, and as long as there are not too many                  does not take effect until at least instance i + α, which means
failures it must remain possible to continue to make progress.                   that when a proposal is made there can be no as-yet-undecided
This is known as the distributed consensus problem, for which                    configuration change in the pipeline that could affect it. In
a number of solutions are known to exist, including Paxos [2]                    contrast, Raft imposes no limit on the number of concurrently-
and Raft [3]. They typically run on a cluster of 2f + 1 nodes,                   running instances and instead restricts the reconfigurations
where f is the number of faulty nodes that should be tolerated,                  that may occur to only allow ones that cannot result in
and consensus is achieved when a nonempty quorum of nodes                        inconsistency. An operator may then perform a sequence of
(e.g. at least f + 1 of them) agree. The collection of quorums                   these restricted reconfigurations in order to achieve an arbitrary
in use is known as the configuration of the cluster.                             reconfiguration.
                                                                                    In Paxos, the choice of the pipeline length parameter α
   Manuscript received 14 August 2017.                                           must be made carefully. If it is too small then the system may
   The author is with the Operations and Planning Systems division of
Tracsis plc, Leeds LS2 9DF, United Kingdom (email: dct25-a1vwi@mythic-           suffer from poor performance due to lack of parallelism, but if
beasts.com)                                                                      it is too large then configuration changes can be unreasonably
    c 2016-7 Tracsis plc. This work is licensed under the Creative Commons       expensive to complete. It is more elegant [5] to limit the
Attribution-ShareAlike 4.0 International License. To view a copy of this
license, visit http://creativecommons.org/licenses/by-sa/4.0/ or send a letter   pipeline length only during reconfiguration and to allow the
to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.                  limit to vary while the system is running, but it would be
UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37                                                        2



better still if there were no need for a limit at all, even during   world systems to disable pipelining by setting α = 1, or even
reconfiguration.                                                     to disable reconfiguration entirely.
   Here it is shown that the pipeline may indeed safely remain          Bortnikov et al. [14] implemented a reconfigurable RSM
unbounded even during a configuration change as long as the          using static RSMs which may pass responsibility to each
reconfiguration satisfies certain conditions described in section    other in a manner that is similar to that of Stoppable Paxos.
IV-A below. It is also shown that if these conditions are not        They improve the performance of their system compared with
satisfied then a reconfiguration can still take place as long as     Stoppable Paxos by allowing each RSM to speculatively start
the pipeline length is temporarily limited.                          executing commands before the transfer of responsibility has
   The algorithm is presented here in its entirety for the sake      been fully agreed.
of consistency of notation and because it modifies the original         Ongaro and Ousterhout [3] developed the Raft protocol
algorithm in ways that invalidate its consistency and liveness       which supports an unbounded pipeline throughout the recon-
proofs. We begin with a recap of the Synod algorithm in              figuration process without inconsistency by instead limiting
section III and follow this by covering the full Paxos algorithm     the reconfigurations that can be performed. They performed a
in section IV, then in section V it is shown how this work           formal proof of the correctness of Raft without reconfiguration,
generalises and unifies the previously-known reconfiguration         and an informal argument that consistency is preserved when
processes supported in Paxos and Raft. Reworked proofs of            a single node is added or removed from the cluster. Raft uses
liveness and consistency are included in section IV-C and            simple majorities of the set of nodes as its quorums.
appendices B and C and differences from the original are                Viewstamped Replication and Zab are two other well-known
highlighted throughout. The appendices are informal versions         consensus protocols. Viewstamped Replication supports recon-
of formal proofs performed using the Isabelle/HOL proof              figuration as described by Liskov and Cowling [15] in a similar
assistant [6].                                                       fashion to Vertical Paxos. Reed and Junqueira [16] initially
                                                                     presented Zab without reconfiguration and this feature was
                     II. R ELATED W ORK                              subsequently added by Shraer, Reed, Malkhi and Junqueira
                                                                     [17], using a limited-pipeline approach much as in Dynamic
   Lamport’s original presentation of Paxos [2] introduced the       Paxos.
bounded-pipeline technique for supporting reconfiguration in            An interesting alternative approach to reconfiguration was
which the pipeline length was defined to be 3. He later clarified    proposed by Jahl and Meling [18] which uses eventual con-
that the value ‘3’ was intended to stand for an arbitrary α > 0      sistency rather than consensus to determine the configuration
in [7]. Later still Lamport and Massa [8] drew a distinction         of the system, and therefore supports reconfiguration as long
between Static Paxos in which the configuration may not              as it is still possible to communicate with a quorum of nodes
change and Dynamic Paxos which supports reconfiguration              even if consensus cannot be achieved due to an inability to
but requires a bounded pipeline. Dynamic Paxos was used as           elect a distinguished leader. In such a situation consensus-
a basis for Cheap Paxos in which the system automatically            based approaches such as the one presented here would fail
reconfigures itself to achieve higher resilience to failures that    to make any further progress, whereas one based on eventual
do not occur simultaneously, and which uses a heterogeneous          consistency could be reconfigured into one in which a leader
set of nodes to reduce the costs of operating a cluster.             may be elected and thus in which further progress can be
   Malkhi, Lamport and Zhou [9] proposed Stoppable Paxos,            made.
an alternative method for reconfiguring clusters in which an
RSM may be stopped, reconfigured, and then restarted with
                                                                                    III. T HE S YNOD A LGORITHM
the new configuration. Stoppable Paxos improves on Dynamic
Paxos by removing the need for a pipeline limit when the                Synod [2] is an algorithm for achieving consensus on a
configuration is static and by allowing a different limit to be      single value in a distributed system comprising a set of nodes
selected for different reconfigurations.                             which can communicate by sending messages to each other.
   Malkhi, Lamport and Zhou [10] then proposed Vertical              The system is asynchronous but not Byzantine, in the sense
Paxos which keeps the RSM running throughout a reconfig-             that messages may be delayed, reordered, duplicated and
uration, but requires a separate oracle to manage the con-           dropped but not corrupted, and processes may run arbitrarily
figuration. The Egalitarian Paxos of Moraru, Andersen and            slowly or even stop but may not deviate from their specifica-
Kaminsky [11] uses a similar reconfiguration scheme. The             tions.
problem with needing an external oracle is that for full                Let B be a set of ballot identifiers with a wellfounded total
resilience it must be possible to reconfigure the oracle itself,     order ≺. Let A be a set of node identifiers and let V be the
which requires another oracle and so on ad infinitum.                set of values that may be chosen.
   Chandra, Griesemer and Redstone [12] noted that the details          The Synod algorithm involves five kinds of message, in
of reconfiguration are “relatively minor” but “subtle” and do        two phases, as described below. Throughout, a ∈ A and
not give any details on the reconfiguration scheme used in           b, b′ ∈ B. Phase I starts with the broadcast of a prepare
their Chubby system. It seems likely that they also used the         message prepare(b) to which each node may respond with a
bounded-pipeline approach. In contrast, Birman, Malkhi and           promise, either a free promise, written promised(a, b), or a
van Renesse [13] noted that allowing for α > 1 may require           forced promise, written promised(a, b; b′ ), where a identifies
an unacceptably complex implementation, leading many real-           the responding node. Phase II starts with the broadcast of a
UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37                                                          3




Fig. 1. Invariants preserved by the Synod algorithm

 S1) For all b1 ≻ b2 ∈ B there are sets of quorums QI (b1 ) and QII (b2 ) ⊆ PA such that if proposed(b1 ) and chosen(b2 )
     then QI (b1 ) ⌢ QII (b2 ).
 S2) If promised(a, b) then ¬accepted(a, b′ ) for all b′ ≺ b.
 S3) If promised(a, b; b′ ) then b′ ≺ b, accepted(a, b′ ), and b′ is the greatest such ballot in the sense that ¬accepted(a, b′′ )
     for all b′′ having b′ ≺ b′′ ≺ b.
 S4) If proposed(b) then there is a quorum q I ∈ QI (b) such that for every node a ∈ q I either promised(a, b) or else there
     exists a b′ such that promised(a, b; b′ ); if also P , {b′ | ∃a ∈ q I .promised(a, b; b′ )} 6= ∅ then v(b) = v max(P ) .
                                                                                                                             

 S5) If accepted(a, b) then proposed(b).
 S6) If chosen(b) then there is a quorum q II ∈ QII (b) such that accepted(a, b) for every a ∈ q II .


proposal, written proposed(b), to which each node a may              sends at least one complete message within a certain period
respond with an acceptance, written accepted(a, b). Once             of time, but this method is unsatisfactory if messages can be
phase II is complete a success message, written chosen(b),           unboundedly large.
is broadcast. It is convenient also to use these symbols                Observation O4 in [8] notes that these values can be
as predicates indicating whether the corresponding messages          replaced in some cases by hashes, but this idea can be taken
have been sent.                                                      a step further and the values can be completely elided from
   There is a function v : B → V assigning a value to each           the messages that take part in the Synod protocol, allowing
ballot, discussed in more detail in section III-A below.             considerably more freedom in the implementation of the
   The system satisfies a set of invariants listed in fig. 1, from   function v without sacrificing consistency.
which it follows that consistency is guaranteed in the sense that       By allowing the values to be communicated using a separate
if chosen(b) and chosen(b′ ) then v(b) = v(b′ ) as shown by          mechanism from the consensus messages themselves it is
theorem 8 in appendix B.                                             possible to seek optimisations that rely on the fact that the
   Each node operates as a state machine whose transitions are       values may be large but need not move quickly whereas the
caused by the receipts of messages. Each phase is considered         consensus messages are small but must be transported with
to be complete for a particular ballot when appropriate mes-         low latency to ensure the system has good performance.
sages have been received from sufficiently many nodes, where            Although it appears that the function v is fixed, in practice
“sufficiently many” is defined in terms of sets of quorums of        it is allowed to change as the system runs. Treating it as
nodes in the system’s configuration.                                 fixed simplifies the consistency proof and highlights that its
   In more detail, a node may emit proposed(b) when it               values need not be included in all messages, but means that
considers phase I to be complete at ballot b, which is when          the system cannot be shown to satisfy any useful liveness
promises for b have been received from a quorum of nodes,            properties. To recover liveness, note that if the invariants of
and similarly may emit chosen(b) when it considers phase             fig. 1 are satisfied with a value function v then they continue to
II to be complete at b, which is when acceptances of b have          be satisfied if v is replaced by another value function v ′ that
been received from a quorum of nodes.                                agrees with v on proposed ballots, i.e. where v(b) = v ′ (b)
   The phase-I and phase-II quorums are defined so as to             if proposed(b) but not necessarily otherwise. Since only
always contain at least one node in common, but may vary             owner(b) may propose b, if it has not yet itself proposed a
depending on the ballot b and the phase as discussed in section      value for b it can deduce that ¬proposed(b) and therefore
III-B below. In particular, the quorums need not all mutually        freely modify v(b).
intersect, as discovered independently by Howard, Malkhi and            It is also important for liveness that the implementation
Spiegelman [19].                                                     of v is resilient to the same failure modes as the rest of
                                                                     the system. Since owner(b) is, in a sense, responsible for
                                                                     the value of the ballot b, it is possible to think of v as an
A. Implementing the value function                                   insert-only set of pairs {hb, v(b)i | proposed(b)} which is
   In an implementation of a RSM, the values chosen represent        an example of a convergent replicated data type [20] and can
the transitions that the state machines must perform. It is pos-     therefore be implemented simply and robustly in a distributed
sible that these values may be expensive to transfer between         system without needing to rely on a consensus algorithm.
nodes because they could carry a large quantity of data.             Indeed the original presentation can be seen as containing
   In the original presentation of the Synod algorithm, the          such an implementation, where the inclusion of values in all
values of ballots are carried along with their identifiers in the    messages ensures convergence occurs as quickly as possible,
messages promised(a, b; b′ , v(b′ )), proposed(b, v(b)) and          and replicating this set across all 2f + 1 nodes ensures v itself
chosen(b, v(b)). This means that, in a unicast network of            may be resilient to as many as 2f failures. Cheap Paxos [8]
2f +1 nodes, each value is included in 2f messages (at least f       is cheaper partly because it replicates v across just the f + 1
proposals and at least f success messages) even in the absence       primary processors, with the f auxiliary processors storing just
of faults, which is twice as many as necessary. Furthermore, a       the hashes of values to ensure integrity.
simple method for detecting faults is to insist that each node          It is also worth comparing this approach to that of Vertical
UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37                                                                       4



Paxos [10] which takes great care to ensure that the system           limit the acceptances that can be sent as well as the promises,
state is completely transferred between nodes before they start       so there is no need to define separate invariants concerning
to participate fully in the cluster, with certain optimisations       the sending of acceptances.
in recognition of the fact that this state transfer could be
an expensive and time-consuming operation involving a very
large quantity of data. However if the implementation of v is         A. Configuration changes
separated out then the quantity of data that must be transferred
as part of the consensus algorithm becomes small enough that             A fixed cluster configuration is a pair hQI , QII i of sets of
it needs no special treatment and a consensus-free technique          quorums satisfying QI ⌢ QII , where QI and QII are the sets
may be used to implement v more efficiently.                          of quorums to use in phase I and phase II respectively.
                                                                         Changes to the cluster configuration are modelled by a
                                                                      sequence of configurations hQI0 , QII0 i, hQI1 , QII1 i, . . . that also
B. Per-phase quorums
                                                                      satisfy QIe ⌢ QIIe+1 for all e. The integer subscript is called
   The consistency property of the Synod algorithm relies on          the era of a configuration. Intuitively the cluster is “in era
the fact that the set of nodes involved in completing phase I         e” while instances are being chosen using hQIe , QIIe i. While a
must always intersect the set of nodes involved in completing         change from era e to e + 1 is in progress some instances may
phase II so that there is at least one node involved in both          use the interim configuration hQIe , QIIe+1 i, and once the change
phases. Write Q1 ⌢ Q2 iff every q1 ∈ Q1 and q2 ∈ Q2 have              to era e + 1 is complete instances will use hQIe+1 , QIIe+1 i. This
q1 ∩ q2 6= ∅, and write QI (b) and QII (b) for the sets of phase-I    intuition is captured more precisely in section IV-D below, and
and phase-II quorums for ballot b respectively.                       since QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1 it follows that consistency
   In the original presentation of the Synod algorithm any            is preserved throughout by the observation of III-B above.
(weighted) majority of the nodes could be used as a quorum,              To achieve this there are also two nondecreasing integer-
so that QI (b1 ) = QII (b2 ) and hence QI (b1 ) ⌢ QII (b2 ) for all   valued functions, both written e(·), which respectively assign
b1 and b2 since all majorities intersect.                             an era e(i) to each instance i, and an era e(b) to each ballot
   Theorem 8 shows that consistency can still be guaranteed           b. Intuitively e(b) records which quorums may be used in
even if sometimes QI (b1 ) 6= QII (b2 ), as long as QI (b1 ) ⌢        phase I to decide that proposedi (b) can be sent, and e(i)
QII (b2 ) when proposed(b1 ), chosen(b2 ) and b1 ≻ b2 , as            records which quorums may be used in phase II to decide
described in invariant S1. This weaker invariant is the key to        that choseni (b) can be sent. More precisely a node may emit
allowing more general reconfigurations to take place safely as        proposedi (b) only if it has received promises for ballot b in
described in section IV-A below.                                      instance i from a quorum of nodes in QIe(b) , which implies that
                                                                      e(b) ≤ e(i), and similarly a node may emit choseni (b) only
                 IV. T HE PAXOS ALGORITHM                             if e(i) ≤ e(b) + 1 and it has received acceptedi (a, b) from a
   Conceptually, Paxos is a sequence of distinct instances of         quorum of nodes in QIIe(i) . These extra conditions on the eras
the Synod algorithm all running simultaneously. To achieve            of ballots and instances in messages ensure that if choseni (b)
this, the messages of the Synod algorithm above are in-               then e(b) ≤ e(i) ≤ e(b) + 1 and hence QIe(b) ⌢ QIIe(i) as
dexed with the instance number i ∈ N: promisedi (a, b),               required to ensure consistency, as shown in lemma 9.
promisedi (a, b; b′ ), proposedi (b), acceptedi (a, b) and
choseni (b). There is another kind of message known as
a multi-promise, written promised≥i (a, b), which can be              B. Dynamic configuration changes
thought of as standing for the infinite set of free promises
{promisedj (a, b) | j ≥ i}. Prepare messages prepare(b)                  As         in      Dynamic           Paxos,      the      configurations
apply to all instances so are not indexed.                            hQI0 , QII0 i, hQI1 , QII1 i, . . . and era numbers e(i) are themselves
   As in the Synod algorithm, phase I starts with a broadcast         chosen by consensus. In contrast, era numbers e(b) for ballots
of prepare(b) for some b to which each node a may respond             b are fixed in advance and not chosen by consensus.
with a set of promises promisedi (a, b), promisedi (a, b; b′ )           In more detail, configurations are held within the RSM as
and promised≥i (a, b) according to its past behaviour. Each           a finite sequence hQI0 , QII0 i, hQI1 , QII1 i, . . . , hQIemax , QIIemax i and
phase II instance i operates just as in the Synod algorithm,          the era numbers of instances are held similarly as a nonde-
starting with a broadcast of proposedi (b) to which each node         creasing sequence e(0), e(1), . . . , e(imax ). The transitions that
a may respond with an acceptance acceptedi (a, b) and once            affect these sequences may append one or more elements, but
acceptances have been received from a quorum of nodes it              may not change any existing elements.
follows that choseni (b) may be broadcast.                               The sequences are always long enough to make progress,
   There is a function vi : B → V for each instance i giving          in the sense that emax ≥ e(imax ) and if chosenj (b) for all
a value to each ballot, and theorem 10 shows that whenever            j < i then imax ≥ i.
choseni (b) and choseni (b′ ) it follows that vi (b) = vi (b′ ).         Note that a node may emit a promise for b at instance i
   The invariants listed in fig. 2 are roughly the same as            only if e(b) ≤ e(min(i, imax )). This allows nodes to make
for many other presentations of Paxos with the addition of            promises for an instance i even if e(i) is not yet known, i.e. if
constraints on the configurations associated with instances and       i > imax . Invariant P7 requires that i ≤ imax , and hence e(i)
ballots as discussed below. Note that invariants P2, P3 and P4        is known, before choseni (b).
UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37                                                          5




Fig. 2. Invariants preserved by the Paxos algorithm

 P1) There are configurations hQI0 , QII0 i, hQI1 , QII1 i, . . . where QIIe ⌢ QIe ⌢ QIIe+1 for each e.
 P2) If promised≥i (a, b) then e(b) ≤ e(min(i, imax )) and ¬acceptedj (a, b′ ) for all j ≥ i and all b′ ≺ b.
 P3) If promisedi (a, b) then e(b) ≤ e(min(i, imax )) and ¬acceptedi (a, b′ ) for all b′ ≺ b.
 P4) If promisedi (a, b; b′ ) then e(b) ≤ e(min(i, imax )), b′ ≺ b, acceptedi (a, b′ ), and b′ is the greatest such ballot in the
     sense that ¬acceptedi (a, b′′ ) for all b′′ having b′ ≺ b′′ ≺ b.
 P5) If proposedi (b) then there is a quorum q ∈ QIe(b) such that for every a ∈ q one of the following holds:
       • promised≥j (a, b) for some j ≤ i, or
       • promisedi (a, b), or
                             ′                ′
       • promisedi (a, b; b ) for some b .
     Furthermore if P , {b′ | ∃a ∈ q.promisedi (a, b; b′ )} 6= ∅ then vi (b) = vi max(P ) .
                                                                                                     

 P6) If acceptedi (a, b) then proposedi (b).
 P7) If choseni (b) then i ≤ imax , e(i) ≤ e(b) + 1, and there is a quorum q ∈ QIIe(i) with acceptedi (a, b) for every a ∈ q.


C. Liveness                                                        so suppose that chosenj (bj ) for all j < i and show that
   To be useful, a consensus algorithm must not only guarantee     eventually choseni (bi ) as follows.
consistency but also ensure that it cannot “get stuck”, i.e. it       Firstly recall that the sequences of eras and configurations
is always possible to make progress by eventually choosing         are long enough, in the sense that i ≤ imax and e(imax ) ≤
a value for each instance. It is known to be impossible to         emax , so that the values of e(i), QIe(i)−1 , QIe(i) and QIIe(i) are
guarantee liveness in a deterministic asynchronous system [21]     known to ℓ.
but as with earlier presentations of Paxos [7] here liveness can      The distinguished node ℓ first chooses a ballot bi having
be shown under the assumption that a distinguished node ℓ is       e(bi ) ∈ {e(i) − 1, e(i)} and owner(bi ) = ℓ and such that
eventually selected as the only one that may emit prepare(b)       enough nodes can emit acceptedi (a, bi ) without breaking
messages.                                                          any of their previously-made promises. By invariants P2, P3
                                                                   and P4, all promises for such a ballot b′ at instance i must
   The original liveness proof then proceeded by having ℓ emit
                                                                   have e(b′ ) ≤ e(i) so that such a bi does exist.
prepare(b) for some b that is chosen to be large enough that a
                                                                      If ℓ has not yet received enough promises for bi at instance i
quorum of nodes may respond with promises. In contrast, here
                                                                   then it broadcasts prepare(bi ) and waits to receive promises
there is an upper bound on suitable ballots since a proposed
                                                                   from a quorum of nodes in QIe(bi ) .
ballot must not belong to an era which is too large, because
if e(b) > e(i) then no promise for ballot b at instance i may         Then, if ℓ has not yet emitted proposedi (bi ) it selects
be made. Therefore here ℓ must be able to choose a ballot          one of the values for instance i (which eventually exists), sets
that is large enough to be accepted but which still belongs to     vi (bi ) as appropriate, broadcasts proposedi (bi ), and waits
the correct era, or, more precisely, for each ballot b, each era   to receive acceptances in response. When acceptances have
e ≥ e(b) and each node a there must be a ballot b′ ≻ b having      been received from a quorum of nodes in QIIe(i) it follows that
e(b′ ) = e and owner(b′ ) = a.                                     choseni (bi ) as required.
   This means that ballot numbers cannot be simple integers
because this would imply that there exist eras containing          D. Fully concurrent configuration changes
only finitely many ballots. Instead an implementation could,           The discussion so far shows that, like other Paxos variants,
for example, let B = N × N × A ordered lexicographically,          this algorithm satisfies consistency and liveness properties.
where e(he, n, ai) , e and owner(he, n, ai) , a. This is           The benefit of this scheme compared with other variants
the approach used in Egalitarian Paxos [11] in which eras          is that, under normal running conditions, it is possible to
are known as epochs but this terminology is avoided here           perform a reconfiguration without needing to impose a limit
to prevent confusion with the epochs (views, terms, . . . ) of     on the number of concurrently-running instances. This section
leader-election protocols (e.g. [22]) which track the current      describes the details of this procedure.
leader rather than the current configuration.                          In normal running there is an instance i0 and a ballot
   With this in mind the proof of liveness runs much as in the     b with e(b) = e(i0 ) = e(imax ) = emax and the distinguished
original presentation:                                             node ℓ = owner(b) has received a quorum of promises
                                                                   q I ∈ QIe(b) for ballot b for all instances i ≥ i0 . In this state,
Theorem 1 (Liveness). Given that there is eventually a
                                                                   ℓ may emit proposedi (b) for any i ≥ i0 as long as vi (b) is
nonfaulty distinguished node ℓ which is the only node that may
                                                                   set appropriately. The node ℓ is known as the leader and its
emit prepare messages, and sufficiently many other nonfaulty
                                                                   proposals are normally accepted without undue delay by all
nodes, and given that for every instance there is eventually at
                                                                   other nodes.
least one value to propose, then eventually a value is chosen
                                                                       Suppose that, in normal running, an operator wishes
for every instance.
                                                                   to change the cluster configuration to hQInew , QIInew i where
Proof. The proof proceeds by induction over the instances,         QIe(b) ⌢ QIInew ⌢ QInew . First she appends hQInew , QIInew i to
UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37                                                           6



the sequence of configurations, setting hQIe(b)+1 , QIIe(b)+1 i =                           a1        ℓ       a2
hQInew , QIInew i and emax = e(b) + 1, then she picks a future
instance ic > imax at which the change should take effect and
appends values to the sequence of eras to set e(i) = e(i0 ) for
                                                                              prepare(b)
i0 ≤ i < ic and e(ic ) = e(i0 ) + 1. Since e(ic ) = e(i0 ) + 1 ≤
e(b)+1, values for instance ic and any future instances with the
same era may be proposed and chosen even though phase I has
not yet run for a ballot in this era, so this does not prevent any
further instances from running concurrently. There is therefore
no drawback to choosing as small a value for ic as possible,                                                     proposedi (b)
so it makes sense to choose ic = imax + 1.                                                                       proposedi+1 (b)
    At this point, the system is no longer in normal running as                                                  proposedi+2 (b)
defined above because e(b) = e(imax )−1. If the operator were
                                                                                                                 proposedi+3 (b)
to increase e(imax ) any further then there would be an instance
i with e(i) > e(b) + 1 and hence ¬choseni (b). A value can                            era e                      proposedi+4 (b)
still eventually be chosen for instance i due to theorem 1,                      era e + 1                       proposedi+5 (b)
but not before the leader has selected a new ballot b′ in an                                                     proposedi+6 (b)
appropriate era, completed phase I for b′ , and then broadcast                                                   proposedi+7 (b)
new proposals for b′ . These steps may cause the pipeline to                  prepare(b′ )
                                                                                                                 proposedi+8 (b)
stall if not completed quickly enough.
                                                                                                                 proposedi+9 (b)
    The system must therefore be returned to normal running
before any further reconfiguration can occur. To do this, the                                                    proposedi+10 (b)
leader chooses a new ballot b′ having e(b′ ) = e(b) + 1                                                          proposedi+11 (b)
and owner(b′ ) = ℓ and runs phase I for b′ in a way that                                                         proposedi+12 (b′ )
does not prevent any progress in era e(b) while it has not
                                                                                                                 proposedi+13 (b′ )
completed. This is possible if ℓ has a casting vote in the
sense that there are quorums of nonfailed nodes q ∈ QIIe(b)
and q ′ ∈ QIe(b)+1 having q ∩ q ′ = {ℓ}. With a casting vote,
ℓ may broadcast prepare(b′ ) just to the nodes in q ′ \ {ℓ}
without preventing further progress in era e(b) since the nodes      Fig. 3. Message flow during a reconfiguration.
in q can continue to accept proposals in this era throughout.
When it has received promises from all the other nodes in q ′ it
can send itself promised≥i′ (ℓ, b′ ) for some sufficiently large     this means the leader has a casting vote. The messages sent
i′ , which completes phase I at b′ and restores the system to        from ℓ to the other nodes are labelled on the diagram, but the
normal running in era e(b′ ) = e(b) + 1. A message from a            successful responses (promises and acceptances from a1 and
node to itself does not incur any network delays so the last         a2 respectively) are left unlabelled for clarity. Initially, imax =
step occurs essentially instantaneously.                             i + 3 and e(i) = e(i + 1) = e(i + 2) = e(i + 3) = e = emax ,
    If ℓ does not have a casting vote, but there is some other       and all instances before i have already been chosen.
node ℓ′ which does, then ℓ should first abdicate its leadership         The leader starts by completing phase I at a sufficiently
to ℓ′ and then the new leader should perform the new phase I         large ballot b, where e(b) = e, which starts a period of normal
as described above. If there is no node with a casting vote at       running in era e and means that ℓ may emit proposedj (b)
all then the broadcast of prepare(b′ ) may prevent progress          for any j ≥ i. It receives three client requests, causing it to
until phase I is complete at b′ .                                    propose values for instances i, i + 1 and i + 2 in turn. On
    If any node fails during this process then it may be necessary   the receipt of each proposal the node a2 responds with an
to retry some of the steps or possibly even elect a new leader.      acceptance, which when ultimately received by ℓ allows it to
Liveness and consistency continue to hold if nodes fail but          decide that each value is chosen because {ℓ, a2 } ∈ QIIe .
performance may be affected, for instance by meaning that ℓ             A configuration change is proposed at instance i + 3 which,
no longer has a casting vote. In general it is not possible to       when chosen, appends the next configuration hQIe+1 , QIIe+1 i to
prevent node failures from having a performance impact.              the configuration sequence, sets e(i + 4) = e(i + 5) = . . . =
    To give a concrete example of this process, fig. 3 shows the     e + 1, and increases emax and imax accordingly. This takes the
flow of messages during a reconfiguration involving the nodes        system out of normal running because now e(imax ) = e + 1 6=
a1 and a2 and the leader ℓ. The system starts in era e where         e(b). In order to bring the system back into normal running,
{ℓ, a1 } ∈ QIe and {ℓ, a2 } ∈ QIIe , and moves to era e + 1 where    the leader must choose a ballot b′ having e(b′ ) = emax and
{ℓ, a1 } ∈ QIe+1 and {ℓ, a2 } ∈ QIIe+1 too. Because of these         complete a phase I at b′ , so it sends prepare(b′ ) to a1 .
quorums, the leader only needs a response from node a1 to               While instance i+3 was being chosen, the leader continued
complete phase I, and similarly only needs a response from           to service client requests by sending out proposedi+4 (b),
node a2 to complete phase II and choose a value. Notice that         proposedi+5 (b) and proposedi+6 (b), which a2 accepts in
UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37                                                         7



due course. Although these instances come after the config-                                  a1      ℓ      a2
uration change at instance i + 3, when the leader receives
acceptedi+4 (a2 , b) it may still safely deduce choseni+4 (b)
since e(i + 4) = e + 1 ≤ e(b) + 1 and {ℓ, a2 } ∈ QIIe+1 , and
similarly for instances i + 5 and i + 6. It is important to notice            prepare(b)
that the leader is now using QIIe+1 and not QIIe to determine
when ballots are chosen. The horizontal dashed line shows the
point in time at which the leader moves from era e to era e+1.
   While the leader is waiting for the response from a1 , client
                                                                                                              proposedi (b)
requests continue to arrive, yielding proposals for subsequent
instances i + 7, i + 8, . . . . Notice that the leader is still                                               proposedi+1 (b)
using ballot b for these requests as it has not yet completed
phase I at b′ , but that it is still safe to deduce choseni+7 (b),
choseni+8 (b), . . . because e(i + 7) = e(i + 8) = e(b) + 1.                                                  proposedi+2 (b)
Importantly, there is no limit to how many requests the leader                                                proposedi+3 (b)
can handle in this way, so the system will continue to be able
to process client requests even if the phase I messages are
arbitrarily delayed.                                                                                          proposedi+4 (b)
   At last the response from a1 is received just after the sending            prepare(b )′                    proposedi+5 (b)
of proposedi+11 (b) and before a proposal has been made for                          era e
instance i + 12. The leader can then send itself the message
                                                                                 era e + 1
promised≥i+12 (ℓ, b′ ) which completes phase I at ballot b′
since {ℓ, a1 } ∈ QIe+1 . This restores the system to normal
running, and allows ℓ to make proposals proposedj (b′ ) for
all j ≥ i + 12. It is imporant to notice that the leader did                                                  proposedi+6 (b′ )
not need to predict how long it would take to complete phase
                                                                                                              proposedi+7 (b′ )
I at b′ in advance, nor how many requests it might have to
handle during this time, in order to ensure that it can continue
to process these requests without delay.
                                                                     Fig. 4. Message flow for Dynamic Paxos with α = 2.
   Fig. 4 shows an equivalent reconfiguration performed in a
Dynamic Paxos cluster with the pipeline length α = 2. As
there are frequently two proposals being processed concur-                                        V. E XAMPLES
rently it seems likely that better performance could be achieved
by selecting a higher value. A higher value still could have            This section contains some examples of configuration
avoided the pause between instances i + 5 and i + 6 caused by        changes that satisfy the conditions described above. All the
the extra delay in completing phase I at ballot b′ . On the other    configurations described here have equal sets of quorums in
hand if the pipeline is too long then configuration changes can      phase I and phase II of each era, so for the sake of simplicity
be expensive to complete. It is, in general, difficult to select     throughout this section define Qe , QIe = QIIe . Implementa-
an appropriate value for α up-front as the best choice may           tions can ensure Qe ⌢ Qe for each e by, for instance,
depend on changeable system conditions, and no matter what           arranging for each quorum in Qe to comprise a majority subset
value is selected it is possible that a configuration change may     of some finite set of nodes. Slightly more generally, let a
cause a pause if a phase-I message is unexpectedly delayed or        weight function be a function w : A → N that only takes
a burst of client requests are received.                             finitely many nonzero values. This can be used to define a
                                                                     configuration hM (w), M (w)i by weighted majority:
   In contrast, fig. 5 shows an equivalent reconfiguration                               (                            )
performed in Stoppable Paxos. This variant of Paxos allows                                     X             X
                                                                                M (w) , q         2w(a) >        w(a) .
for an unlimited number of proposals to run in parallel
                                                                                                   a∈q           a∈A
within each configuration, and permits out-of-order execution,
so in this illustration the stopping command is proposed at          Corollary 5 in appendix A demonstrates the well-known result
instance i+3 before proposals are made for the two preceding         that M (w) ⌢ M (w) for any weight function w. Indeed, if w
instances. By selecting instance i + 3 for the reconfiguration,      and w′ are weight functions that differ by a constant factor
the operator is limiting the system to service at most two more      in the sense that that there are positive integers k and k ′ with
client requests before the reconfiguration completes. As in the      kw(a) = k ′ w′ (a) for all a, then clearly M (w) = M (w′ ) and
illustration of Dynamic Paxos above, the operator’s choice is        hence M (w) ⌢ M (w′ ).
too conservative so the remaining two free instances are used           Raft’s quorums are simple unweighted majorities of a finite
up before phase I is completed at ballot b′ , which causes the       set of nodes, which can be emulated with weight functions
system to temporarily suspend its processing of client requests.     that only take values in {0, 1}. It only supports adding or
UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37                                                          8



                       a1        ℓ          a2                         functions starting at era e:
                                                                                        node     aold   anew   a1   a2
                                                                                          we      1      0     1    1
          prepare(b)                                                                    we+1      1      1     1    1
                                                                                        we+2      0      1     1    1
                                                                       However to be resilient to infrastructure failures this requires
                                                                       all four nodes and their underlying infrastructures to be
                                                                       completely independent since a correlated failure of any two
                                             proposedi (b)             nodes would prevent further progress. It also has no node with
                                             proposedi+3 (b)
                                                                       a casting vote in era e + 1. On the other hand the following
                                                                       sequence achieves the same overall change but allows aold and
                                             proposedi+1 (b)
                                                                       anew to share infrastructure without extra risk, and both a1 and
                                             proposedi+2 (b)           a2 have casting votes throughout:
                 era e                                                                  node     aold   anew   a1   a2
             era e + 1                                                                    we      1      0     1    1
                                                                                        we+1      2      0     2    2
         prepare(b′ )
                                                                                        we+2      2      1     2    2
                                                                                        we+3      1      1     2    2
                                                                                        we+4      0      1     2    2
                                                                                        we+5      0      2     2    2
                                             proposedi+4 (b′ )                          we+6      0      1     1    1
                                             proposedi+5 (b′ )
                                                                       This is important as in many operating environments it may be
                                             proposedi+6 (b′ )         too expensive or complicated to arrange for four independent
                                                                       infrastructures particularly if the fourth is only required to
                                                                       ensure consistency in relatively rare periods of maintenance.
                                                                       For instance at time of writing only one Amazon Web Services
                                                                       region (us-east-1) has four independent zones, whereas
                                                                       four of them have three: ap-southeast-2, eu-west-1,
Fig. 5. Message flow for Stoppable Paxos.                              sa-east-1 and us-west-2. Similarly, only one Google
                                                                       Cloud Platform region (us-central1) has four zones and
                                                                       all the others have three zones.
removing a single node from this set which amounts to                     Early versions of Raft supported more general reconfigu-
changing the weight of a single node by ±1. For instance,              rations using a technique known as joint configurations. To
if A = {a1 , a2 , . . .} then define weight functions                  change from configuration Qe to an unrelated configuration
                             (                                         Q′ (i.e. Qe 6⌢ Q′ ⌢ Q′ ) it is possible to set Qe+2 = Q′ and
               1...n           1 ifa ∈ {a1 , . . . , an }              set Qe+1 to be the joint configuration of Qe and Q′ :
             w         (a) ,
                               0 otherwise
                                                                                     Qe+1 = {q ∪ q ′ | q ∈ Qe , q ′ ∈ Q′ }
and        observe      that      M (w1...3 ) ⌢ M (w1...4 )      and   as this satisfies that Qe ⌢ Qe+1 ⌢ Qe+1 ⌢ Q′ .
       1...4          1...5
M (w         ) ⌢ M (w       ) but M (w1...3 ) 6⌢ M (w1...5 ) because      If w(a) = 0 for all a then w is said to be weightless
{a1 , a2 } ∈ M (w1...3 ) and {a3 , a4 , a5 } ∈ M (w1...5 ) do          and M (w) = ∅. Clearly if there is an instance i such that
not intersect. This justifies the restriction against adding or        Qe(i) = ∅ then no value can ever be chosen for i. On the other
removing more than one node at once.                                   hand Q ⌢ ∅ for all Q so changing to or from a weightless
   In fact there is no need to restrict attention just to weight       configuration is always permitted.
functions taking values in {0, 1} as shown by lemma 3 which               There is no requirement for the eras of consecutive instances
is reminiscent of the amoeba analogy in [8]: any two integer-          to differ by at most one so an era may be skipped at instance
valued weight functions whose total absolute difference is at          i + 1 by setting e(i + 1) = e(i) + 2. This recovers the ability
most one can be used to define consecutive configurations.             to perform arbitrary configuration changes in a single step as
This extra generality is important for deployments where nodes         in Stoppable Paxos. In more detail, if the system is currently
may share infrastructure (e.g. power distribution or network           using configuration Qe and an operator wishes to change to
connectivity) because such nodes may suffer correlated fail-           an unrelated configuration Q′ then she can set Qe+2 = Q′
ures, and reducing this correlation by adding more independent         and pick an appropriate Qe+1 (such as ∅) which satisfies that
infrastructure may be costly. In more detail, a naı̈ve approach        Qe ⌢ Qe+1 ⌢ Qe+1 ⌢ Q′ . However, re-running phase I can
to swapping a node aold for a replacement anew in a three-             only be delayed as described in section IV-D above if the era
node cluster using unweighted majorities would be to perform           increases by 1 and in this situation the era increases by 2, so
the configuration changes given by this sequence of weight             a new phase I must be completed before phase II of any new
UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37                                                                9



                                                                   Let dA , k ′ w′ (a0 ) − kw(a0 ) so that          ′ ′
                                                                                                             P
instances can be started. To prevent this causing the pipeline     P                                           a∈A k w (a) =
to stall, the operator chooses a sufficiently large α > 0 and        a∈A kw(a) + dA and |dA | ≤ 1. Also let
sets e(i + α) = e(i) + 2 and e(j) = e(i) for i < j ≤ i + α,                                  (
effectively delaying the configuration change for α instances                                  dA a 0 ∈ q ′
                                                                                      dq ′ ,
in the hope that this is long enough to have completed the                                     0  otherwise,
new phase I. The pattern of communication in this situation
                                                                   so that a∈q′ k ′ w′ (a) = a∈q′ kw(a) + dq′ . Then
                                                                          P                  P
is very similar to that shown in fig. 5 where α was chosen to
be 3.
                                                                     X
                                                                         2kw(a) + dA + k + k ′
                                                                     a∈A
                                                                                                           !                            !
                        VI. C ONCLUSION                                                     X
                                                                                                                    ′
                                                                                                                          X
                                                                                                                                 ′
                                                                                  =k              w(a) + 1     +k               w (a) + 1
   The approach described here improves on Dynamic Paxos                                    a∈A                           a∈A
                                                                                      X                 X
[8] by supporting changing the pipeline length parameter                          ≤         2kw(a) +           2k ′ w′ (a)
α and running with an unlimited-length pipeline while a                               a∈q              a∈q ′
configuration change is not in progress. In that sense, it can                        X                 X
                                                                                  =         2kw(a) +           2kw(a) + 2dq′
be compared to that of Stoppable Paxos [9] which allows for
                                                                                      a∈q              a∈q ′
an unlimited number of proposals to run in parallel within                             X                       X
each configuration, but requires a temporary arbitrary limit on                   =             2kw(a) +                2kw(a) + 2dq′
concurrency while a reconfiguration takes place.                                      a∈q∪q ′
                                                                                      X                  X
                                                                                                             a∈q∩q ′

   In both Dynamic and Stoppable Paxos, if the selected                           ≤         2kw(a) +             2kw(a) + 2dq′
limit is either too small or too large then it may affect the                         a∈A              a∈q∩q ′
system’s performance, and the best choice of limit depends
                                                                   so that a∈q∩q′ 2kw(a) ≥ dA +k+k ′ −2dq′ = k+k ′ ±dA ≥ 1
                                                                          P
on a prediction of the system’s future performance. The
                                                                   and hence q ∩ q ′ 6= ∅ as desired.
approach described here avoids the need to make any such
prediction or select any such limit and responds to changing       Lemma 3. If w, w′ : A → N are weight functions such that
                                                                           ′                                  ′
                                                                   P
system conditions without needing further tuning. Once a             a∈A |w (a) − w(a)| ≤ 1 then M (w) ⌢ M (w ).
reconfiguration is chosen, it can complete after a single round-
                                                                   Proof. By lemma 2 with k = k ′ = 1.
trip to a quorum of nodes and the system can continue to serve
all clients while this round-trip is in progress, no matter how    Lemma 4. If w, w′ : A → N are weight functions and k, k ′
long it takes.                                                     are positive integers such that k ′ w′ (a) = kw(a) for all a then
   This is achieved by using Raft-style reconfigurations [3]       M (w) ⌢ M (w′ ).
which can be performed with an unlimited pipeline throughout.
                                                                   Proof. By lemma 2, since a∈A |k ′ w′ (a) − kw(a)| = 0.
                                                                                               P
Unlike in Raft, here a configuration change only takes effect
once it is chosen, which avoids the need to back-track to          Corollary 5. If w : A → N is a weight function then
an earlier state if a leader fails during reconfiguration. It
                                                                                             M (w) ⌢ M (w).
generalises the simple majorities used in Raft to integer-
weighted majorities which can reduce the costs of dealing with     Proof. By lemma 4 with w′ = w and k ′ = k.
correlated failures during maintenance.
   It achieves equivalent goals to those of Vertical Paxos [10]                         A PPENDIX B
except that here there is no requirement for a separate oracle             C ONSISTENCY OF THE S YNOD ALGORITHM
to manage the configuration of the system.                         Lemma 6. If accepted(a, b2 ), promised(a, b1 ; b3 ) and
   It is also noted that there is no need for every message to     b2 ≺ b1 then b2  b3 .
include the corresponding value, or even a hash of the value,
which may allow for even cheaper implementations of Cheap          Proof. From invariant S3 it follows that b3 is the largest ballot
Paxos [8] and can simplify the transfer of state [10] required     such that b3 ≺ b1 and accepted(a, b3 ), but b2 is also such a
when new nodes are commissioned.                                   ballot and therefore b2  b3 as required.
                                                                   Lemma 7. If chosen(b2 ), proposed(b1 ) and b2 ≺ b1 then
                          A PPENDIX A                              v(b1 ) = v(b2 ).
                    ′                                              Proof. Suppose for a contradiction that v(b1 ) 6= v(b2 ) and
Lemma 2. If w, w : A → N are   P weight functions and k, k ′
                                      ′ ′                          since ≺ is wellfounded suppose without loss of generality that
are positive integers such that a∈A |k w (a) − kw(a)| ≤ 1
                                                                   b1 is the minimal such ballot. Since chosen(b2 ) by invariant
then M (w) ⌢ M (w′ ).
                                                                   S6 there is a quorum q II ∈ QII (b2 ) such that accepted(a, b2 )
Proof. Since kw(a) and k ′ w′ (a) are integers for all a, there    for every a ∈ q II . By invariant S2 it cannot be that
must be a node a0 such that kw(a) = k ′ w′ (a) for all a 6= a0 .   promised(a, b1 ) for any a ∈ q II . Also since proposed(b1 )
Let q ∈ M (w) and q ′ ∈ M (w′ ). By the definition       of M ,    by invariant S4 there is a quorum q I ∈ QI (b1 ) such that either
                   ′
                                                                   promised(a, b1 ) or ∃b′ .promised(a, b1 ; b′ ) for all a ∈ q I .
                                                 P
and since w and  w   take only  integer  values,
                                              P a∈q′   2w(a) ≥
                                   ′
                                                                   Let P , {b′ | ∃a ∈ q I .promised(a, b1 ; b′ )}. By invariant S1,
P                       P
  a∈A  w(a) +  1  and     a∈q ′ 2w   (a)  ≥      a∈A w (a) + 1.
UNBOUNDED PIPELINING IN DYNAMICALLY RECONFIGURABLE PAXOS CLUSTERS — 1A9DBA37                                                                      10



QI (b1 ) ⌢ QII (b2 ) and hence q I ∩q II 6= ∅ so it follows that P 6=                                R EFERENCES
∅, which means that v(b1 ) = v(max(P )) by invariant S4.                 [1] B. W. Lampson, “How to build a highly available system using consen-
Let amax ∈ q I be such that promised(amax , b1 ; max(P )).                   sus,” in Proceedings of the 10th International Workshop on Distributed
By invariant S3 it follows that max(P ) ≺ b1 and also that                   Algorithms, ser. WDAG ’96. London, UK, UK: Springer-Verlag, 1996,
                                                                             pp. 1–17.
accepted(amax , max(P )) and hence proposed(max(P ))                     [2] L. Lamport, “The part-time parliament,” ACM TRANSACTIONS ON
by invariant S5. Furthermore by lemma 6 it follows that                      COMPUTER SYSTEMS, vol. 16, no. 2, pp. 133–169, 1998.
b2  max(P ) and since b1 was assumed to be the smallest                 [3] D. Ongaro and J. Ousterhout, “In search of an understandable consensus
                                                                             algorithm,” in Proceedings of the 2014 USENIX Conference on USENIX
counterexample it must be that v(max(P )) = v(b2 ). Hence                    Annual Technical Conference, ser. USENIX ATC’14. Berkeley, CA,
v(b1 ) = v(b2 ) which is a contradiction as required.                        USA: USENIX Association, 2014, pp. 305–320.
                                                                         [4] J. R. Lorch, A. Adya, W. J. Bolosky, R. Chaiken, J. R. Douceur, and
                                                                             J. Howell, “The smart way to migrate replicated stateful services,”
Theorem 8. If chosen(b1 ) and chosen(b2 ) then v(b1 ) =                      in Proceedings of the 2006 EuroSys Conference. Leuven, Belgium:
v(b2 ).                                                                      Association for Computing Machinery, Inc., April 2006, p. 103115.
                                                                         [5] L. Lamport, D. Malkhi, and L. Zhou, “Reconfiguring a state machine,”
Proof. Without loss of generality assume that b2 ≺ b1 . By                   Microsoft Research, Tech. Rep. MSR-TR-2008-198, February 2008.
invariant S6 there is a quorum q ∈ QII (b1 ) such that                   [6] T. Nipkow, M. Wenzel, and L. C. Paulson, Isabelle/HOL: A Proof
                                                                             Assistant for Higher-order Logic. Berlin, Heidelberg: Springer-Verlag,
accepted(a, b1 ) for every node a ∈ q and therefore                          2002.
proposed(b1 ) by invariant S5. Therefore by lemma 7 it                   [7] L. Lamport, “Paxos made simple,” ACM SIGACT News, vol. 32, no. 4,
follows that v(b1 ) = v(b2 ) as required.                                    pp. 18–25, 2001.
                                                                         [8] L. Lamport and M. Massa, “Cheap paxos,” 2004.
                                                                         [9] L. Lamport, D. Malkhi, and L. Zhou, “Stoppable paxos,” Microsoft
                                                                             Research, Tech. Rep. MSR-TR-2008-197, April 2008.
                      A PPENDIX C                                       [10] ——, “Vertical paxos and primary-backup replication,” in Proceedings
         C ONSISTENCY OF THE PAXOS ALGORITHM                                 of the 28th ACM Symposium on Principles of Distributed Computing,
                                                                             ser. PODC ’09. New York, NY, USA: ACM, 2009, pp. 312–313.
Lemma 9. For b1 ≻ b2 ∈ B, if proposedi (b1 ) and                        [11] I. Moraru, D. G. Andersen, and M. Kaminsky, “There is more consensus
                                                                             in egalitarian parliaments,” in Proceedings of the Twenty-Fourth ACM
choseni (b2 ) then QIe(b1 ) ⌢ QIIe(i) .                                      Symposium on Operating Systems Principles, ser. SOSP ’13. New York,
                                                                             NY, USA: ACM, 2013, pp. 358–372.
Proof. proposedi (b1 ) implies that promised≥i′ (a, b1 ) or             [12] T. D. Chandra, R. Griesemer, and J. Redstone, “Paxos made live: An
promisedi′ (a, b1 ) or promisedi′ (a, b1 ; b′ ) for some node                engineering perspective,” in Proceedings of the Twenty-sixth Annual
a and some i′ ≤ i with e(b1 ) ≤ e(min(i′ , imax )). There-                   ACM Symposium on Principles of Distributed Computing, ser. PODC
                                                                             ’07. New York, NY, USA: ACM, 2007, pp. 398–407.
fore e(b1 ) ≤ e(i) since e is nondecreasing. It follows that            [13] K. Birman, D. Malkhi, and R. van Renesse, “Virtually synchronous
e(i) ≤ e(b2 ) + 1 ≤ e(b1 ) + 1 ≤ e(i) + 1 since choseni (b2 )                methodology for dynamic service replication,” Tech. Rep., 2010.
so that e(i) ∈ {e(b1 ), e(b1 ) + 1} and hence QIe(b1 ) ⌢ QIIe(i) by     [14] V. Bortnikov, G. Chockler, A. Roytman, S. Shachor, I. Shnayderman,
                                                                             and D. Perelman, Reconfigurable state machine replication from non-
invariant P1.                                                                reconfigurable building blocks. ACM, 2012, pp. 93–94.
                                                                        [15] B. Liskov and J. Cowling, “Viewstamped replication revisited,” MIT,
Theorem 10. If choseni (b1 ) and choseni (b2 ) then                          Tech. Rep. MIT-CSAIL-TR-2012-021, Jul. 2012.
                                                                        [16] B. Reed and F. P. Junqueira, “A simple totally ordered broadcast pro-
vi (b1 ) = vi (b2 ).                                                         tocol,” in Proceedings of the 2Nd Workshop on Large-Scale Distributed
                                                                             Systems and Middleware, ser. LADIS ’08. New York, NY, USA: ACM,
Proof. If choseni (b1 ) then the Paxos invariants imply the                  2008, pp. 2:1–2:6.
Synod invariants for instance i. In more detail, let                    [17] A. Shraer, B. Reed, D. Malkhi, and F. P. Junqueira, “Dynamic recon-
                                                                             figuration of primary/backup clusters,” in Presented as part of the 2012
                QI (b)       , QIe(b)                                        USENIX Annual Technical Conference (USENIX ATC 12). Boston,
                                                                             MA: USENIX, 2012, pp. 425–437.
                QII (b)      , QIIe(i)                                  [18] L. Jehl and H. Meling, Distributed Computing and Networking: 15th
        promised(a, b)       , promisedi (a, b)                              International Conference, ICDCN 2014, Coimbatore, India, January 4-
                                                                             7, 2014. Proceedings. Berlin, Heidelberg: Springer Berlin Heidelberg,
                              ∨ ∃i′ ≤ i.promised≥i′ (a, b)                   2014, ch. Asynchronous Reconfiguration for Paxos State Machines, pp.
    promised(a, b; b′ )      , promisedi (a, b; b′ )                         119–133.
                                                                        [19] H. Howard, D. Malkhi, and A. Spiegelman, “Flexible Paxos: Quorum
       proposed(b)           , proposedi (b)                                 intersection revisited,” ArXiv e-prints, Aug. 2016.
      accepted(a, b)         , acceptedi (a, b)                         [20] M. Shapiro, N. Preguiça, C. Baquero, and M. Zawirski, Stabilization,
                                                                             Safety, and Security of Distributed Systems: 13th International Sympo-
          chosen(b)          , choseni (b) and                               sium, SSS 2011, Grenoble, France, October 10-12, 2011. Proceedings.
                 v(b)        , vi (b).                                       Berlin, Heidelberg: Springer Berlin Heidelberg, 2011, ch. Conflict-Free
                                                                             Replicated Data Types, pp. 386–400.
Synod’s invariant S1 follows from lemma 9 and the remaining             [21] M. J. Fischer, N. A. Lynch, and M. S. Paterson, “Impossibility of
                                                                             distributed consensus with one faulty process,” J. ACM, vol. 32, no. 2,
invariants are simple to show so by theorem 8 it follows that                pp. 374–382, Apr. 1985.
vi (b1 ) = vi (b2 ) as required.                                        [22] D. Malkhi, F. Oprea, and L. Zhou, “Omega meets paxos: Leader election
                                                                             and stability without eventual timely links,” in 19th Intl. Symposium
                                                                             on Distributed Computing (DISC 05), no. MSR-TR-2005-93. Cra-
                      ACKNOWLEDGMENT                                         cow, Poland: European Association for Theoretical Computer Science,
                                                                             September 2005, p. 25.
   The author would like to thank Leslie Lamport, Dahlia
Malkhi and Leander Nikolaus Jehl for their encouragement
and comments on earlier drafts of this paper. The author is
also very grateful to Tracsis plc for supporting this work.
