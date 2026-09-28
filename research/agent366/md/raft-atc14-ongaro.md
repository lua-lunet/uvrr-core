         In Search of an Understandable
              Consensus Algorithm
         Diego Ongaro and John Ousterhout, Stanford University
   https://www.usenix.org/conference/atc14/technical-sessions/presentation/ongaro




This paper is included in the Proceedings of USENIX ATC ’14:
        2014 USENIX Annual Technical Conference.
                     June 19–20, 2014 • Philadelphia, PA
                                 978-1-931971-10-2




                                           Open access to the Proceedings of
                                      USENIX ATC ’14: 2014 USENIX Annual Technical
                                          Conference is sponsored by USENIX.
          In Search of an Understandable Consensus Algorithm
                                        Diego Ongaro and John Ousterhout
                                              Stanford University
Abstract                                                        to understand than Paxos: after learning both algorithms,
   Raft is a consensus algorithm for managing a replicated      33 of these students were able to answer questions about
log. It produces a result equivalent to (multi-)Paxos, and      Raft better than questions about Paxos.
it is as efficient as Paxos, but its structure is different        Raft is similar in many ways to existing consensus al-
from Paxos; this makes Raft more understandable than            gorithms (most notably, Oki and Liskov’s Viewstamped
Paxos and also provides a better foundation for build-          Replication [27, 20]), but it has several novel features:
ing practical systems. In order to enhance understandabil-      • Strong leader: Raft uses a stronger form of leadership
ity, Raft separates the key elements of consensus, such as         than other consensus algorithms. For example, log en-
leader election, log replication, and safety, and it enforces      tries only flow from the leader to other servers. This
a stronger degree of coherency to reduce the number of             simplifies the management of the replicated log and
states that must be considered. Results from a user study          makes Raft easier to understand.
demonstrate that Raft is easier for students to learn than      • Leader election: Raft uses randomized timers to elect
Paxos. Raft also includes a new mechanism for changing             leaders. This adds only a small amount of mechanism
the cluster membership, which uses overlapping majori-             to the heartbeats already required for any consensus al-
ties to guarantee safety.                                          gorithm, while resolving conflicts simply and rapidly.
                                                                • Membership changes: Raft’s mechanism for changing
1 Introduction                                                     the set of servers in the cluster uses a new joint consen-
   Consensus algorithms allow a collection of machines             sus approach where the majorities of two different con-
to work as a coherent group that can survive the fail-             figurations overlap during transitions. This allows the
ures of some of its members. Because of this, they play a          cluster to continue operating normally during configu-
key role in building reliable large-scale software systems.        ration changes.
Paxos [13, 14] has dominated the discussion of consen-
                                                                   We believe that Raft is superior to Paxos and other con-
sus algorithms over the last decade: most implementations
                                                                sensus algorithms, both for educational purposes and as a
of consensus are based on Paxos or influenced by it, and
                                                                foundation for implementation. It is simpler and more un-
Paxos has become the primary vehicle used to teach stu-
                                                                derstandable than other algorithms; it is described com-
dents about consensus.
                                                                pletely enough to meet the needs of a practical system;
   Unfortunately, Paxos is quite difficult to understand, in    it has several open-source implementations and is used
spite of numerous attempts to make it more approachable.        by several companies; its safety properties have been for-
Furthermore, its architecture requires complex changes          mally specified and proven; and its efficiency is compara-
to support practical systems. As a result, both system          ble to other algorithms.
builders and students struggle with Paxos.
                                                                   The remainder of the paper introduces the replicated
   After struggling with Paxos ourselves, we set out to         state machine problem (Section 2), discusses the strengths
find a new consensus algorithm that could provide a bet-        and weaknesses of Paxos (Section 3), describes our gen-
ter foundation for system building and education. Our ap-       eral approach to understandability (Section 4), presents
proach was unusual in that our primary goal was under-          the Raft consensus algorithm (Sections 5–7), evaluates
standability: could we define a consensus algorithm for         Raft (Section 8), and discusses related work (Section 9).
practical systems and describe it in a way that is signifi-     A few elements of the Raft algorithm have been omitted
cantly easier to learn than Paxos? Furthermore, we wanted       here because of space limitations, but they are available in
the algorithm to facilitate the development of intuitions       an extended technical report [29]. The additional material
that are essential for system builders. It was important not    describes how clients interact with the system, and how
just for the algorithm to work, but for it to be obvious why    space in the Raft log can be reclaimed.
it works.
   The result of this work is a consensus algorithm called      2 Replicated state machines
Raft. In designing Raft we applied specific techniques to          Consensus algorithms typically arise in the context of
improve understandability, including decomposition (Raft        replicated state machines [33]. In this approach, state ma-
separates leader election, log replication, and safety) and     chines on a collection of servers compute identical copies
state space reduction (relative to Paxos, Raft reduces the      of the same state and can continue operating even if some
degree of nondeterminism and the ways servers can be in-        of the servers are down. Replicated state machines are
consistent with each other). A user study with 43 students      used to solve a variety of fault tolerance problems in dis-
at two universities shows that Raft is significantly easier     tributed systems. For example, large-scale systems that



USENIX Association                                                         2014 USENIX Annual Technical Conference 305
                                                                 • In the common case, a command can complete as soon
                                                                   as a majority of the cluster has responded to a single
                                                                   round of remote procedure calls; a minority of slow
                                                                   servers need not impact overall system performance.
                                                                 3 What’s wrong with Paxos?
                                                                    Over the last ten years, Leslie Lamport’s Paxos proto-
                                                                 col [13] has become almost synonymous with consensus:
                                                                 it is the protocol most commonly taught in courses, and
                                                                 most implementations of consensus use it as a starting
 Figure 1: Replicated state machine architecture. The con-       point. Paxos first defines a protocol capable of reaching
 sensus algorithm manages a replicated log containing state
                                                                 agreement on a single decision, such as a single replicated
 machine commands from clients. The state machines process
                                                                 log entry. We refer to this subset as single-decree Paxos.
 identical sequences of commands from the logs, so they pro-
 duce the same outputs.                                          Paxos then combines multiple instances of this protocol to
                                                                 facilitate a series of decisions such as a log (multi-Paxos).
have a single cluster leader, such as GFS [7], HDFS [34],        Paxos ensures both safety and liveness, and it supports
and RAMCloud [30], typically use a separate replicated           changes in cluster membership. Its correctness has been
state machine to manage leader election and store config-        proven, and it is efficient in the normal case.
uration information that must survive leader crashes. Ex-           Unfortunately, Paxos has two significant drawbacks.
amples of replicated state machines include Chubby [2]           The first drawback is that Paxos is exceptionally diffi-
and ZooKeeper [9].                                               cult to understand. The full explanation [13] is notori-
   Replicated state machines are typically implemented           ously opaque; few people succeed in understanding it, and
using a replicated log, as shown in Figure 1. Each server        only with great effort. As a result, there have been several
stores a log containing a series of commands, which its          attempts to explain Paxos in simpler terms [14, 18, 19].
state machine executes in order. Each log contains the           These explanations focus on the single-decree subset, yet
same commands in the same order, so each state ma-               they are still challenging. In an informal survey of atten-
chine processes the same sequence of commands. Since             dees at NSDI 2012, we found few people who were com-
the state machines are deterministic, each computes the          fortable with Paxos, even among seasoned researchers.
same state and the same sequence of outputs.                     We struggled with Paxos ourselves; we were not able to
   Keeping the replicated log consistent is the job of the       understand the complete protocol until after reading sev-
consensus algorithm. The consensus module on a server            eral simplified explanations and designing our own alter-
receives commands from clients and adds them to its log.         native protocol, a process that took almost a year.
It communicates with the consensus modules on other                 We hypothesize that Paxos’ opaqueness derives from
servers to ensure that every log eventually contains the         its choice of the single-decree subset as its foundation.
same requests in the same order, even if some servers fail.      Single-decree Paxos is dense and subtle: it is divided into
Once commands are properly replicated, each server’s             two stages that do not have simple intuitive explanations
state machine processes them in log order, and the out-          and cannot be understood independently. Because of this,
puts are returned to clients. As a result, the servers appear    it is difficult to develop intuitions about why the single-
to form a single, highly reliable state machine.                 decree protocol works. The composition rules for multi-
   Consensus algorithms for practical systems typically          Paxos add significant additional complexity and subtlety.
have the following properties:                                   We believe that the overall problem of reaching consensus
• They ensure safety (never returning an incorrect result)       on multiple decisions (i.e., a log instead of a single entry)
   under all non-Byzantine conditions, including network         can be decomposed in other ways that are more direct and
   delays, partitions, and packet loss, duplication, and re-     obvious.
   ordering.                                                        The second problem with Paxos is that it does not pro-
• They are fully functional (available) as long as any ma-       vide a good foundation for building practical implemen-
   jority of the servers are operational and can communi-        tations. One reason is that there is no widely agreed-
   cate with each other and with clients. Thus, a typical        upon algorithm for multi-Paxos. Lamport’s descriptions
   cluster of five servers can tolerate the failure of any two   are mostly about single-decree Paxos; he sketched possi-
   servers. Servers are assumed to fail by stopping; they        ble approaches to multi-Paxos, but many details are miss-
   may later recover from state on stable storage and re-        ing. There have been several attempts to flesh out and op-
   join the cluster.                                             timize Paxos, such as [24], [35], and [11], but these differ
• They do not depend on timing to ensure the consistency         from each other and from Lamport’s sketches. Systems
   of the logs: faulty clocks and extreme message delays         such as Chubby [4] have implemented Paxos-like algo-
   can, at worst, cause availability problems.                   rithms, but in most cases their details have not been pub-



 306 2014 USENIX Annual Technical Conference                                                            USENIX Association
lished.                                                         understandability: how hard is it to explain each alterna-
   Furthermore, the Paxos architecture is a poor one for        tive (for example, how complex is its state space, and does
building practical systems; this is another consequence of      it have subtle implications?), and how easy will it be for a
the single-decree decomposition. For example, there is lit-     reader to completely understand the approach and its im-
tle benefit to choosing a collection of log entries indepen-    plications?
dently and then melding them into a sequential log; this           We recognize that there is a high degree of subjectiv-
just adds complexity. It is simpler and more efficient to       ity in such analysis; nonetheless, we used two techniques
design a system around a log, where new entries are ap-         that are generally applicable. The first technique is the
pended sequentially in a constrained order. Another prob-       well-known approach of problem decomposition: wher-
lem is that Paxos uses a symmetric peer-to-peer approach        ever possible, we divided problems into separate pieces
at its core (though it eventually suggests a weak form of       that could be solved, explained, and understood relatively
leadership as a performance optimization). This makes           independently. For example, in Raft we separated leader
sense in a simplified world where only one decision will        election, log replication, safety, and membership changes.
be made, but few practical systems use this approach. If a         Our second approach was to simplify the state space
series of decisions must be made, it is simpler and faster      by reducing the number of states to consider, making the
to first elect a leader, then have the leader coordinate the    system more coherent and eliminating nondeterminism
decisions.                                                      where possible. Specifically, logs are not allowed to have
   As a result, practical systems bear little resemblance       holes, and Raft limits the ways in which logs can become
to Paxos. Each implementation begins with Paxos, dis-           inconsistent with each other. Although in most cases we
covers the difficulties in implementing it, and then de-        tried to eliminate nondeterminism, there are some situ-
velops a significantly different architecture. This is time-    ations where nondeterminism actually improves under-
consuming and error-prone, and the difficulties of under-       standability. In particular, randomized approaches intro-
standing Paxos exacerbate the problem. Paxos’ formula-          duce nondeterminism, but they tend to reduce the state
tion may be a good one for proving theorems about its cor-      space by handling all possible choices in a similar fashion
rectness, but real implementations are so different from        (“choose any; it doesn’t matter”). We used randomization
Paxos that the proofs have little value. The following com-     to simplify the Raft leader election algorithm.
ment from the Chubby implementers is typical:
                                                                5 The Raft consensus algorithm
     There are significant gaps between the description of
                                                                   Raft is an algorithm for managing a replicated log of
     the Paxos algorithm and the needs of a real-world
     system. . . . the final system will be based on an un-     the form described in Section 2. Figure 2 summarizes the
     proven protocol [4].                                       algorithm in condensed form for reference, and Figure 3
                                                                lists key properties of the algorithm; the elements of these
   Because of these problems, we concluded that Paxos
                                                                figures are discussed piecewise over the rest of this sec-
does not provide a good foundation either for system
                                                                tion.
building or for education. Given the importance of con-
                                                                   Raft implements consensus by first electing a distin-
sensus in large-scale software systems, we decided to see
                                                                guished leader, then giving the leader complete responsi-
if we could design an alternative consensus algorithm
                                                                bility for managing the replicated log. The leader accepts
with better properties than Paxos. Raft is the result of that
                                                                log entries from clients, replicates them on other servers,
experiment.
                                                                and tells servers when it is safe to apply log entries to
4 Designing for understandability                               their state machines. Having a leader simplifies the man-
   We had several goals in designing Raft: it must provide      agement of the replicated log. For example, the leader can
a complete and practical foundation for system building,        decide where to place new entries in the log without con-
so that it significantly reduces the amount of design work      sulting other servers, and data flows in a simple fashion
required of developers; it must be safe under all conditions    from the leader to other servers. A leader can fail or be-
and available under typical operating conditions; and it        come disconnected from the other servers, in which case
must be efficient for common operations. But our most           a new leader is elected.
important goal—and most difficult challenge—was un-                Given the leader approach, Raft decomposes the con-
derstandability. It must be possible for a large audience to    sensus problem into three relatively independent subprob-
understand the algorithm comfortably. In addition, it must      lems, which are discussed in the subsections that follow:
be possible to develop intuitions about the algorithm, so       • Leader election: a new leader must be chosen when an
that system builders can make the extensions that are in-          existing leader fails (Section 5.2).
evitable in real-world implementations.                         • Log replication: the leader must accept log entries
   There were numerous points in the design of Raft                from clients and replicate them across the cluster, forc-
where we had to choose among alternative approaches.               ing the other logs to agree with its own (Section 5.3).
In these situations we evaluated the alternatives based on      • Safety: the key safety property for Raft is the State Ma-



USENIX Association                                                         2014 USENIX Annual Technical Conference 307
                              State                                                       RequestVote RPC
 Persistent state on all servers:                                      Invoked by candidates to gather votes (§5.2).
 (Updated on stable storage before responding to RPCs)
                                                                       Arguments:
 currentTerm        latest term server has seen (initialized to 0
                    on first boot, increases monotonically)            term                candidate’s term
                                                                       candidateId         candidate requesting vote
 votedFor           candidateId that received vote in current
                                                                       lastLogIndex        index of candidate’s last log entry (§5.4)
                    term (or null if none)
                                                                       lastLogTerm         term of candidate’s last log entry (§5.4)
 log[]              log entries; each entry contains command
                    for state machine, and term when entry             Results:
                    was received by leader (first index is 1)          term                currentTerm, for candidate to update itself
                                                                       voteGranted         true means candidate received vote
 Volatile state on all servers:
 commitIndex       index of highest log entry known to be              Receiver implementation:
                   committed (initialized to 0, increases              1. Reply false if term < currentTerm (§5.1)
                   monotonically)                                      2. If votedFor is null or candidateId, and candidate’s log is at
 lastApplied       index of highest log entry applied to state            least as up-to-date as receiver’s log, grant vote (§5.2, §5.4)
                   machine (initialized to 0, increases
                   monotonically)
                                                                                          Rules for Servers
 Volatile state on leaders:
                                                                       All Servers:
 (Reinitialized after election)                                        • If commitIndex > lastApplied: increment lastApplied, apply
 nextIndex[]          for each server, index of the next log entry
                                                                         log[lastApplied] to state machine (§5.3)
                      to send to that server (initialized to leader    • If RPC request or response contains term T > currentTerm:
                      last log index + 1)                                set currentTerm = T, convert to follower (§5.1)
 matchIndex[]         for each server, index of highest log entry
                      known to be replicated on server                 Followers (§5.2):
                      (initialized to 0, increases monotonically)      • Respond to RPCs from candidates and leaders
                                                                       • If election timeout elapses without receiving AppendEntries
                  AppendEntries RPC                                      RPC from current leader or granting vote to candidate:
 Invoked by leader to replicate log entries (§5.3); also used as         convert to candidate
 heartbeat (§5.2).                                                     Candidates (§5.2):
 Arguments:                                                            • On conversion to candidate, start election:
 term                leader’s term                                       • Increment currentTerm
 leaderId            so follower can redirect clients                    • Vote for self
 prevLogIndex        index of log entry immediately preceding            • Reset election timer
                     new ones                                            • Send RequestVote RPCs to all other servers
 prevLogTerm         term of prevLogIndex entry                        • If votes received from majority of servers: become leader
 entries[]           log entries to store (empty for heartbeat;        • If AppendEntries RPC received from new leader: convert to
                     may send more than one for efficiency)              follower
 leaderCommit        leader’s commitIndex                              • If election timeout elapses: start new election

 Results:                                                              Leaders:
 term                currentTerm, for leader to update itself          • Upon election: send initial empty AppendEntries RPCs
 success             true if follower contained entry matching           (heartbeat) to each server; repeat during idle periods to
                     prevLogIndex and prevLogTerm                        prevent election timeouts (§5.2)
                                                                       • If command received from client: append entry to local log,
 Receiver implementation:                                                respond after entry applied to state machine (§5.3)
 1. Reply false if term < currentTerm (§5.1)                           • If last log index ≥ nextIndex for a follower: send
 2. Reply false if log doesn’t contain an entry at prevLogIndex          AppendEntries RPC with log entries starting at nextIndex
    whose term matches prevLogTerm (§5.3)                                • If successful: update nextIndex and matchIndex for
 3. If an existing entry conflicts with a new one (same index               follower (§5.3)
    but different terms), delete the existing entry and all that         • If AppendEntries fails because of log inconsistency:
    follow it (§5.3)                                                        decrement nextIndex and retry (§5.3)
 4. Append any new entries not already in the log                      • If there exists an N such that N > commitIndex, a majority
 5. If leaderCommit > commitIndex, set commitIndex =                     of matchIndex[i] ≥ N, and log[N].term == currentTerm:
    min(leaderCommit, index of last new entry)                           set commitIndex = N (§5.3, §5.4).
Figure 2: A condensed summary of the Raft consensus algorithm (excluding membership changes and log compaction). The server
behavior in the upper-left box is described as a set of rules that trigger independently and repeatedly. Section numbers such as §5.2
indicate where particular features are discussed. A formal specification [28] describes the algorithm more precisely.




308 2014 USENIX Annual Technical Conference                                                                       USENIX Association
 Election Safety: at most one leader can be elected in a
   given term. §5.2
 Leader Append-Only: a leader never overwrites or deletes
   entries in its log; it only appends new entries. §5.3
 Log Matching: if two logs contain an entry with the same
   index and term, then the logs are identical in all entries up
   through the given index. §5.3
 Leader Completeness: if a log entry is committed in a              Figure 4: Server states. Followers only respond to requests
   given term, then that entry will be present in the logs of       from other servers. If a follower receives no communication,
   the leaders for all higher-numbered terms. §5.4                  it becomes a candidate and initiates an election. A candidate
 State Machine Safety: if a server has applied a log entry at       that receives votes from a majority of the full cluster becomes
   a given index to its state machine, no other server will ever    the new leader. Leaders typically operate until they fail.
   apply a different log entry for the same index. §5.4.3
 Figure 3: Raft guarantees that each of these properties is true
 at all times. The section numbers indicate where each prop-
 erty is discussed.

  chine Safety Property in Figure 3: if any server has ap-
  plied a particular log entry to its state machine, then           Figure 5: Time is divided into terms, and each term begins
  no other server may apply a different command for the             with an election. After a successful election, a single leader
  same log index. Section 5.4 describes how Raft ensures            manages the cluster until the end of the term. Some elections
  this property; the solution involves an additional re-            fail, in which case the term ends without choosing a leader.
  striction on the election mechanism described in Sec-             The transitions between terms may be observed at different
  tion 5.2.                                                         times on different servers.
After presenting the consensus algorithm, this section dis-        act as a logical clock [12] in Raft, and they allow servers
cusses the issue of availability and the role of timing in the     to detect obsolete information such as stale leaders. Each
system.                                                            server stores a current term number, which increases
5.1 Raft basics                                                    monotonically over time. Current terms are exchanged
                                                                   whenever servers communicate; if one server’s current
   A Raft cluster contains several servers; five is a typical
                                                                   term is smaller than the other’s, then it updates its current
number, which allows the system to tolerate two failures.
                                                                   term to the larger value. If a candidate or leader discovers
At any given time each server is in one of three states:
                                                                   that its term is out of date, it immediately reverts to fol-
leader, follower, or candidate. In normal operation there
                                                                   lower state. If a server receives a request with a stale term
is exactly one leader and all of the other servers are fol-
                                                                   number, it rejects the request.
lowers. Followers are passive: they issue no requests on
                                                                      Raft servers communicate using remote procedure calls
their own but simply respond to requests from leaders
                                                                   (RPCs), and the consensus algorithm requires only two
and candidates. The leader handles all client requests (if
                                                                   types of RPCs. RequestVote RPCs are initiated by candi-
a client contacts a follower, the follower redirects it to the
                                                                   dates during elections (Section 5.2), and AppendEntries
leader). The third state, candidate, is used to elect a new
                                                                   RPCs are initiated by leaders to replicate log entries and
leader as described in Section 5.2. Figure 4 shows the
                                                                   to provide a form of heartbeat (Section 5.3). Servers retry
states and their transitions; the transitions are discussed
                                                                   RPCs if they do not receive a response in a timely manner,
below.
                                                                   and they issue RPCs in parallel for best performance.
   Raft divides time into terms of arbitrary length, as
shown in Figure 5. Terms are numbered with consecutive             5.2 Leader election
integers. Each term begins with an election, in which one             Raft uses a heartbeat mechanism to trigger leader elec-
or more candidates attempt to become leader as described           tion. When servers start up, they begin as followers. A
in Section 5.2. If a candidate wins the election, then it          server remains in follower state as long as it receives valid
serves as leader for the rest of the term. In some situations      RPCs from a leader or candidate. Leaders send periodic
an election will result in a split vote. In this case the term     heartbeats (AppendEntries RPCs that carry no log entries)
will end with no leader; a new term (with a new election)          to all followers in order to maintain their authority. If a
will begin shortly. Raft ensures that there is at most one         follower receives no communication over a period of time
leader in a given term.                                            called the election timeout, then it assumes there is no vi-
   Different servers may observe the transitions between           able leader and begins an election to choose a new leader.
terms at different times, and in some situations a server             To begin an election, a follower increments its current
may not observe an election or even entire terms. Terms            term and transitions to candidate state. It then votes for



USENIX Association                                                            2014 USENIX Annual Technical Conference 309
itself and issues RequestVote RPCs in parallel to each of
the other servers in the cluster. A candidate continues in
this state until one of three things happens: (a) it wins the
election, (b) another server establishes itself as leader, or
(c) a period of time goes by with no winner. These out-
comes are discussed separately in the paragraphs below.
   A candidate wins an election if it receives votes from
a majority of the servers in the full cluster for the same
term. Each server will vote for at most one candidate in a
given term, on a first-come-first-served basis (note: Sec-
tion 5.4 adds an additional restriction on votes). The ma-
jority rule ensures that at most one candidate can win the
election for a particular term (the Election Safety Prop-
erty in Figure 3). Once a candidate wins an election, it          Figure 6: Logs are composed of entries, which are numbered
becomes leader. It then sends heartbeat messages to all of        sequentially. Each entry contains the term in which it was
the other servers to establish its authority and prevent new      created (the number in each box) and a command for the state
                                                                  machine. An entry is considered committed if it is safe for that
elections.
                                                                  entry to be applied to state machines.
   While waiting for votes, a candidate may receive an
AppendEntries RPC from another server claiming to be             created subtle issues around availability (a lower-ranked
leader. If the leader’s term (included in its RPC) is at least   server might need to time out and become a candidate
as large as the candidate’s current term, then the candidate     again if a higher-ranked server fails, but if it does so too
recognizes the leader as legitimate and returns to follower      soon, it can reset progress towards electing a leader). We
state. If the term in the RPC is smaller than the candidate’s    made adjustments to the algorithm several times, but after
current term, then the candidate rejects the RPC and con-        each adjustment new corner cases appeared. Eventually
tinues in candidate state.                                       we concluded that the randomized retry approach is more
                                                                 obvious and understandable.
   The third possible outcome is that a candidate neither
wins nor loses the election: if many followers become            5.3 Log replication
candidates at the same time, votes could be split so that           Once a leader has been elected, it begins servicing
no candidate obtains a majority. When this happens, each         client requests. Each client request contains a command to
candidate will time out and start a new election by incre-       be executed by the replicated state machines. The leader
menting its term and initiating another round of Request-        appends the command to its log as a new entry, then is-
Vote RPCs. However, without extra measures split votes           sues AppendEntries RPCs in parallel to each of the other
could repeat indefinitely.                                       servers to replicate the entry. When the entry has been
   Raft uses randomized election timeouts to ensure that         safely replicated (as described below), the leader applies
split votes are rare and that they are resolved quickly. To      the entry to its state machine and returns the result of that
prevent split votes in the first place, election timeouts are    execution to the client. If followers crash or run slowly,
chosen randomly from a fixed interval (e.g., 150–300ms).         or if network packets are lost, the leader retries Append-
This spreads out the servers so that in most cases only a        Entries RPCs indefinitely (even after it has responded to
single server will time out; it wins the election and sends      the client) until all followers eventually store all log en-
heartbeats before any other servers time out. The same           tries.
mechanism is used to handle split votes. Each candidate             Logs are organized as shown in Figure 6. Each log en-
restarts its randomized election timeout at the start of an      try stores a state machine command along with the term
election, and it waits for that timeout to elapse before         number when the entry was received by the leader. The
starting the next election; this reduces the likelihood of       term numbers in log entries are used to detect inconsis-
another split vote in the new election. Section 8.3 shows        tencies between logs and to ensure some of the properties
that this approach elects a leader rapidly.                      in Figure 3. Each log entry also has an integer index iden-
   Elections are an example of how understandability             tifying its position in the log.
guided our choice between design alternatives. Initially            The leader decides when it is safe to apply a log en-
we planned to use a ranking system: each candidate was           try to the state machines; such an entry is called commit-
assigned a unique rank, which was used to select between         ted. Raft guarantees that committed entries are durable
competing candidates. If a candidate discovered another          and will eventually be executed by all of the available
candidate with higher rank, it would return to follower          state machines. A log entry is committed once the leader
state so that the higher ranking candidate could more eas-       that created the entry has replicated it on a majority of
ily win the next election. We found that this approach           the servers (e.g., entry 7 in Figure 6). This also commits



310 2014 USENIX Annual Technical Conference                                                                USENIX Association
all preceding entries in the leader’s log, including entries
created by previous leaders. Section 5.4 discusses some
subtleties when applying this rule after leader changes,
and it also shows that this definition of commitment is
safe. The leader keeps track of the highest index it knows
to be committed, and it includes that index in future
AppendEntries RPCs (including heartbeats) so that the
other servers eventually find out. Once a follower learns
that a log entry is committed, it applies the entry to its
local state machine (in log order).
   We designed the Raft log mechanism to maintain a high
level of coherency between the logs on different servers.         Figure 7: When the leader at the top comes to power, it is
Not only does this simplify the system’s behavior and             possible that any of scenarios (a–f) could occur in follower
make it more predictable, but it is an important component        logs. Each box represents one log entry; the number in the
of ensuring safety. Raft maintains the following proper-          box is its term. A follower may be missing entries (a–b), may
ties, which together constitute the Log Matching Property         have extra uncommitted entries (c–d), or both (e–f). For ex-
                                                                  ample, scenario (f) could occur if that server was the leader
in Figure 3:
                                                                  for term 2, added several entries to its log, then crashed before
• If two entries in different logs have the same index and        committing any of them; it restarted quickly, became leader
   term, then they store the same command.                        for term 3, and added a few more entries to its log; before any
• If two entries in different logs have the same index and        of the entries in either term 2 or term 3 were committed, the
   term, then the logs are identical in all preceding entries.    server crashed again and remained down for several terms.
   The first property follows from the fact that a leader            To bring a follower’s log into consistency with its own,
creates at most one entry with a given log index in a given      the leader must find the latest log entry where the two
term, and log entries never change their position in the         logs agree, delete any entries in the follower’s log after
log. The second property is guaranteed by a simple con-          that point, and send the follower all of the leader’s entries
sistency check performed by AppendEntries. When send-            after that point. All of these actions happen in response
ing an AppendEntries RPC, the leader includes the index          to the consistency check performed by AppendEntries
and term of the entry in its log that immediately precedes       RPCs. The leader maintains a nextIndex for each follower,
the new entries. If the follower does not find an entry in       which is the index of the next log entry the leader will
its log with the same index and term, then it refuses the        send to that follower. When a leader first comes to power,
new entries. The consistency check acts as an induction          it initializes all nextIndex values to the index just after the
step: the initial empty state of the logs satisfies the Log      last one in its log (11 in Figure 7). If a follower’s log is
Matching Property, and the consistency check preserves           inconsistent with the leader’s, the AppendEntries consis-
the Log Matching Property whenever logs are extended.            tency check will fail in the next AppendEntries RPC. Af-
As a result, whenever AppendEntries returns successfully,        ter a rejection, the leader decrements nextIndex and retries
the leader knows that the follower’s log is identical to its     the AppendEntries RPC. Eventually nextIndex will reach
own log up through the new entries.                              a point where the leader and follower logs match. When
   During normal operation, the logs of the leader and           this happens, AppendEntries will succeed, which removes
followers stay consistent, so the AppendEntries consis-          any conflicting entries in the follower’s log and appends
tency check never fails. However, leader crashes can leave       entries from the leader’s log (if any). Once AppendEntries
the logs inconsistent (the old leader may not have fully         succeeds, the follower’s log is consistent with the leader’s,
replicated all of the entries in its log). These inconsisten-    and it will remain that way for the rest of the term.
cies can compound over a series of leader and follower               The protocol can be optimized to reduce the number of
crashes. Figure 7 illustrates the ways in which followers’       rejected AppendEntries RPCs; see [29] for details.
logs may differ from that of a new leader. A follower may            With this mechanism, a leader does not need to take any
be missing entries that are present on the leader, it may        special actions to restore log consistency when it comes to
have extra entries that are not present on the leader, or        power. It just begins normal operation, and the logs auto-
both. Missing and extraneous entries in a log may span           matically converge in response to failures of the Append-
multiple terms.                                                  Entries consistency check. A leader never overwrites or
   In Raft, the leader handles inconsistencies by forcing        deletes entries in its own log (the Leader Append-Only
the followers’ logs to duplicate its own. This means that        Property in Figure 3).
conflicting entries in follower logs will be overwritten             This log replication mechanism exhibits the desirable
with entries from the leader’s log. Section 5.4 will show        consensus properties described in Section 2: Raft can ac-
that this is safe when coupled with one more restriction.        cept, replicate, and apply new log entries as long as a ma-



USENIX Association                                                           2014 USENIX Annual Technical Conference 311
jority of the servers are up; in the normal case a new entry
can be replicated with a single round of RPCs to a ma-
jority of the cluster; and a single slow follower will not
impact performance.
5.4 Safety
   The previous sections described how Raft elects lead-
ers and replicates log entries. However, the mechanisms
described so far are not quite sufficient to ensure that each
state machine executes exactly the same commands in the           Figure 8: A time sequence showing why a leader cannot de-
same order. For example, a follower might be unavailable          termine commitment using log entries from older terms. In
while the leader commits several log entries, then it could       (a) S1 is leader and partially replicates the log entry at index
be elected leader and overwrite these entries with new            2. In (b) S1 crashes; S5 is elected leader for term 3 with votes
                                                                  from S3, S4, and itself, and accepts a different entry at log
ones; as a result, different state machines might execute
                                                                  index 2. In (c) S5 crashes; S1 restarts, is elected leader, and
different command sequences.                                      continues replication. At this point, the log entry from term 2
   This section completes the Raft algorithm by adding a          has been replicated on a majority of the servers, but it is not
restriction on which servers may be elected leader. The           committed. If S1 crashes as in (d), S5 could be elected leader
restriction ensures that the leader for any given term con-       (with votes from S2, S3, and S4) and overwrite the entry with
tains all of the entries committed in previous terms (the         its own entry from term 3. However, if S1 replicates an en-
Leader Completeness Property from Figure 3). Given the            try from its current term on a majority of the servers before
election restriction, we then make the rules for commit-          crashing, as in (e), then this entry is committed (S5 cannot
ment more precise. Finally, we present a proof sketch for         win an election). At this point all preceding entries in the log
the Leader Completeness Property and show how it leads            are committed as well.
to correct behavior of the replicated state machine.             logs. If the logs have last entries with different terms, then
5.4.1 Election restriction                                       the log with the later term is more up-to-date. If the logs
   In any leader-based consensus algorithm, the leader           end with the same term, then whichever log is longer is
must eventually store all of the committed log entries. In       more up-to-date.
some consensus algorithms, such as Viewstamped Repli-            5.4.2 Committing entries from previous terms
cation [20], a leader can be elected even if it doesn’t             As described in Section 5.3, a leader knows that an en-
initially contain all of the committed entries. These al-        try from its current term is committed once that entry is
gorithms contain additional mechanisms to identify the           stored on a majority of the servers. If a leader crashes be-
missing entries and transmit them to the new leader, ei-         fore committing an entry, future leaders will attempt to
ther during the election process or shortly afterwards. Un-      finish replicating the entry. However, a leader cannot im-
fortunately, this results in considerable additional mecha-      mediately conclude that an entry from a previous term is
nism and complexity. Raft uses a simpler approach where          committed once it is stored on a majority of servers. Fig-
it guarantees that all the committed entries from previous       ure 8 illustrates a situation where an old log entry is stored
terms are present on each new leader from the moment of          on a majority of servers, yet can still be overwritten by a
its election, without the need to transfer those entries to      future leader.
the leader. This means that log entries only flow in one di-        To eliminate problems like the one in Figure 8, Raft
rection, from leaders to followers, and leaders never over-      never commits log entries from previous terms by count-
write existing entries in their logs.                            ing replicas. Only log entries from the leader’s current
   Raft uses the voting process to prevent a candidate from      term are committed by counting replicas; once an entry
winning an election unless its log contains all committed        from the current term has been committed in this way,
entries. A candidate must contact a majority of the cluster      then all prior entries are committed indirectly because
in order to be elected, which means that every committed         of the Log Matching Property. There are some situations
entry must be present in at least one of those servers. If the   where a leader could safely conclude that an older log en-
candidate’s log is at least as up-to-date as any other log       try is committed (for example, if that entry is stored on ev-
in that majority (where “up-to-date” is defined precisely        ery server), but Raft takes a more conservative approach
below), then it will hold all the committed entries. The         for simplicity.
RequestVote RPC implements this restriction: the RPC                Raft incurs this extra complexity in the commitment
includes information about the candidate’s log, and the          rules because log entries retain their original term num-
voter denies its vote if its own log is more up-to-date than     bers when a leader replicates entries from previous
that of the candidate.                                           terms. In other consensus algorithms, if a new leader re-
   Raft determines which of two logs is more up-to-date          replicates entries from prior “terms,” it must do so with
by comparing the index and term of the last entries in the       its new “term number.” Raft’s approach makes it easier



312 2014 USENIX Annual Technical Conference                                                                USENIX Association
                                                                    larger than the voter’s. Moreover, it was larger than T,
                                                                    since the voter’s last log term was at least T (it contains
                                                                    the committed entry from term T). The earlier leader that
                                                                    created leaderU ’s last log entry must have contained the
                                                                    committed entry in its log (by assumption). Then, by the
                                                                    Log Matching Property, leaderU ’s log must also contain
                                                                    the committed entry, which is a contradiction.
 Figure 9: If S1 (leader for term T) commits a new log entry           8. This completes the contradiction. Thus, the leaders
 from its term, and S5 is elected leader for a later term U, then   of all terms greater than T must contain all entries from
 there must be at least one server (S3) that accepted the log       term T that are committed in term T.
 entry and also voted for S5.
                                                                       9. The Log Matching Property guarantees that future
to reason about log entries, since they maintain the same           leaders will also contain entries that are committed indi-
term number over time and across logs. In addition, new             rectly, such as index 2 in Figure 8(d).
leaders in Raft send fewer log entries from previous terms             Given the Leader Completeness Property, it is easy to
than in other algorithms (other algorithms must send re-            prove the State Machine Safety Property from Figure 3
dundant log entries to renumber them before they can be             and that all state machines apply the same log entries in
committed).                                                         the same order (see [29]).
5.4.3 Safety argument                                               5.5 Follower and candidate crashes
   Given the complete Raft algorithm, we can now ar-                   Until this point we have focused on leader failures. Fol-
gue more precisely that the Leader Completeness Prop-               lower and candidate crashes are much simpler to han-
erty holds (this argument is based on the safety proof; see         dle than leader crashes, and they are both handled in the
Section 8.2). We assume that the Leader Completeness                same way. If a follower or candidate crashes, then fu-
Property does not hold, then we prove a contradiction.              ture RequestVote and AppendEntries RPCs sent to it will
Suppose the leader for term T (leaderT ) commits a log              fail. Raft handles these failures by retrying indefinitely;
entry from its term, but that log entry is not stored by the        if the crashed server restarts, then the RPC will complete
leader of some future term. Consider the smallest term U            successfully. If a server crashes after completing an RPC
> T whose leader (leaderU ) does not store the entry.               but before responding, then it will receive the same RPC
   1. The committed entry must have been absent from                again after it restarts. Raft RPCs are idempotent, so this
leaderU ’s log at the time of its election (leaders never           causes no harm. For example, if a follower receives an
delete or overwrite entries).                                       AppendEntries request that includes log entries already
   2. leaderT replicated the entry on a majority of the             present in its log, it ignores those entries in the new re-
cluster, and leaderU received votes from a majority of the          quest.
cluster. Thus, at least one server (“the voter”) both ac-           5.6 Timing and availability
cepted the entry from leaderT and voted for leaderU , as
                                                                       One of our requirements for Raft is that safety must
shown in Figure 9. The voter is key to reaching a contra-
                                                                    not depend on timing: the system must not produce incor-
diction.
                                                                    rect results just because some event happens more quickly
   3. The voter must have accepted the committed entry              or slowly than expected. However, availability (the ability
from leaderT before voting for leaderU ; otherwise it would         of the system to respond to clients in a timely manner)
have rejected the AppendEntries request from leaderT (its           must inevitably depend on timing. For example, if mes-
current term would have been higher than T).                        sage exchanges take longer than the typical time between
   4. The voter still stored the entry when it voted for            server crashes, candidates will not stay up long enough to
leaderU , since every intervening leader contained the en-          win an election; without a steady leader, Raft cannot make
try (by assumption), leaders never remove entries, and fol-         progress.
lowers only remove entries if they conflict with the leader.           Leader election is the aspect of Raft where timing is
   5. The voter granted its vote to leaderU , so leaderU ’s         most critical. Raft will be able to elect and maintain a
log must have been as up-to-date as the voter’s. This leads         steady leader as long as the system satisfies the follow-
to one of two contradictions.                                       ing timing requirement:
   6. First, if the voter and leaderU shared the same last
                                                                          broadcastTime ≪ electionTimeout ≪ MTBF
log term, then leaderU ’s log must have been at least as
long as the voter’s, so its log contained every entry in the        In this inequality broadcastTime is the average time it
voter’s log. This is a contradiction, since the voter con-          takes a server to send RPCs in parallel to every server
tained the committed entry and leaderU was assumed not              in the cluster and receive their responses; electionTime-
to.                                                                 out is the election timeout described in Section 5.2; and
   7. Otherwise, leaderU ’s last log term must have been            MTBF is the average time between failures for a single



USENIX Association                                                             2014 USENIX Annual Technical Conference 313
server. The broadcast time should be an order of mag-
nitude less than the election timeout so that leaders can
reliably send the heartbeat messages required to keep fol-
lowers from starting elections; given the randomized ap-
proach used for election timeouts, this inequality also
makes split votes unlikely. The election timeout should be
a few orders of magnitude less than MTBF so that the sys-
tem makes steady progress. When the leader crashes, the
system will be unavailable for roughly the election time-
out; we would like this to represent only a small fraction
of overall time.                                                  Figure 10: Switching directly from one configuration to an-
   The broadcast time and MTBF are properties of the un-          other is unsafe because different servers will switch at dif-
derlying system, while the election timeout is something          ferent times. In this example, the cluster grows from three
we must choose. Raft’s RPCs typically require the recip-          servers to five. Unfortunately, there is a point in time where
                                                                  two different leaders can be elected for the same term, one
ient to persist information to stable storage, so the broad-
                                                                  with a majority of the old configuration (Cold ) and another
cast time may range from 0.5ms to 20ms, depending on              with a majority of the new configuration (Cnew ).
storage technology. As a result, the election timeout is
likely to be somewhere between 10ms and 500ms. Typical              urations.
server MTBFs are several months or more, which easily            • Any server from either configuration may serve as
satisfies the timing requirement.                                   leader.
                                                                 • Agreement (for elections and entry commitment) re-
6 Cluster membership changes                                        quires separate majorities from both the old and new
   Up until now we have assumed that the cluster config-            configurations.
uration (the set of servers participating in the consensus       The joint consensus allows individual servers to transition
algorithm) is fixed. In practice, it will occasionally be nec-   between configurations at different times without com-
essary to change the configuration, for example to replace       promising safety. Furthermore, joint consensus allows the
servers when they fail or to change the degree of replica-       cluster to continue servicing client requests throughout
tion. Although this can be done by taking the entire cluster     the configuration change.
off-line, updating configuration files, and then restarting         Cluster configurations are stored and communicated
the cluster, this would leave the cluster unavailable dur-       using special entries in the replicated log; Figure 11 illus-
ing the changeover. In addition, if there are any manual         trates the configuration change process. When the leader
steps, they risk operator error. In order to avoid these is-     receives a request to change the configuration from Cold
sues, we decided to automate configuration changes and           to Cnew , it stores the configuration for joint consensus
incorporate them into the Raft consensus algorithm.              (Cold,new in the figure) as a log entry and replicates that
   For the configuration change mechanism to be safe,            entry using the mechanisms described previously. Once a
there must be no point during the transition where it            given server adds the new configuration entry to its log,
is possible for two leaders to be elected for the same           it uses that configuration for all future decisions (a server
term. Unfortunately, any approach where servers switch           always uses the latest configuration in its log, regardless
directly from the old configuration to the new configura-        of whether the entry is committed). This means that the
tion is unsafe. It isn’t possible to atomically switch all of    leader will use the rules of Cold,new to determine when the
the servers at once, so the cluster can potentially split into   log entry for Cold,new is committed. If the leader crashes,
two independent majorities during the transition (see Fig-       a new leader may be chosen under either Cold or Cold,new ,
ure 10).                                                         depending on whether the winning candidate has received
   In order to ensure safety, configuration changes must         Cold,new . In any case, Cnew cannot make unilateral deci-
use a two-phase approach. There are a variety of ways            sions during this period.
to implement the two phases. For example, some systems              Once Cold,new has been committed, neither Cold nor Cnew
(e.g., [20]) use the first phase to disable the old configura-   can make decisions without approval of the other, and the
tion so it cannot process client requests; then the second       Leader Completeness Property ensures that only servers
phase enables the new configuration. In Raft the cluster         with the Cold,new log entry can be elected as leader. It is
first switches to a transitional configuration we call joint     now safe for the leader to create a log entry describing
consensus; once the joint consensus has been committed,          Cnew and replicate it to the cluster. Again, this configura-
the system then transitions to the new configuration. The        tion will take effect on each server as soon as it is seen.
joint consensus combines both the old and new configu-           When the new configuration has been committed under
rations:                                                         the rules of Cnew , the old configuration is irrelevant and
• Log entries are replicated to all servers in both config-      servers not in the new configuration can be shut down. As



314 2014 USENIX Annual Technical Conference                                                               USENIX Association
                                                                     This does not affect normal elections, where each server
                                                                     waits at least a minimum election timeout before starting
                                                                     an election. However, it helps avoid disruptions from re-
                                                                     moved servers: if a leader is able to get heartbeats to its
                                                                     cluster, then it will not be deposed by larger term num-
                                                                     bers.
                                                                     7 Clients and log compaction
                                                                        This section has been omitted due to space limitations,
 Figure 11: Timeline for a configuration change. Dashed lines
 show configuration entries that have been created but not
                                                                     but the material is available in the extended version of this
 committed, and solid lines show the latest committed configu-       paper [29]. It describes how clients interact with Raft, in-
 ration entry. The leader first creates the Cold,new configuration   cluding how clients find the cluster leader and how Raft
 entry in its log and commits it to Cold,new (a majority of Cold     supports linearizable semantics [8]. The extended version
 and a majority of Cnew ). Then it creates the Cnew entry and        also describes how space in the replicated log can be re-
 commits it to a majority of Cnew . There is no point in time in     claimed using a snapshotting approach. These issues ap-
 which Cold and Cnew can both make decisions independently.          ply to all consensus-based systems, and Raft’s solutions
shown in Figure 11, there is no time when Cold and Cnew              are similar to other systems.
can both make unilateral decisions; this guarantees safety.          8 Implementation and evaluation
   There are three more issues to address for reconfigura-              We have implemented Raft as part of a replicated
tion. The first issue is that new servers may not initially          state machine that stores configuration information for
store any log entries. If they are added to the cluster in           RAMCloud [30] and assists in failover of the RAMCloud
this state, it could take quite a while for them to catch            coordinator. The Raft implementation contains roughly
up, during which time it might not be possible to com-               2000 lines of C++ code, not including tests, comments, or
mit new log entries. In order to avoid availability gaps,            blank lines. The source code is freely available [21]. There
Raft introduces an additional phase before the configu-              are also about 25 independent third-party open source im-
ration change, in which the new servers join the cluster             plementations [31] of Raft in various stages of develop-
as non-voting members (the leader replicates log entries             ment, based on drafts of this paper. Also, various compa-
to them, but they are not considered for majorities). Once           nies are deploying Raft-based systems [31].
the new servers have caught up with the rest of the cluster,
                                                                        The remainder of this section evaluates Raft using three
the reconfiguration can proceed as described above.
                                                                     criteria: understandability, correctness, and performance.
   The second issue is that the cluster leader may not be
part of the new configuration. In this case, the leader steps        8.1 Understandability
down (returns to follower state) once it has committed the              To measure Raft’s understandability relative to Paxos,
Cnew log entry. This means that there will be a period of            we conducted an experimental study using upper-level un-
time (while it is committing Cnew ) when the leader is man-          dergraduate and graduate students in an Advanced Oper-
aging a cluster that does not include itself; it replicates log      ating Systems course at Stanford University and a Dis-
entries but does not count itself in majorities. The leader          tributed Computing course at U.C. Berkeley. We recorded
transition occurs when Cnew is committed because this is             a video lecture of Raft and another of Paxos, and created
the first point when the new configuration can operate in-           corresponding quizzes. The Raft lecture covered the con-
dependently (it will always be possible to choose a leader           tent of this paper; the Paxos lecture covered enough ma-
from Cnew ). Before this point, it may be the case that only         terial to create an equivalent replicated state machine, in-
a server from Cold can be elected leader.                            cluding single-decree Paxos, multi-decree Paxos, recon-
   The third issue is that removed servers (those not in             figuration, and a few optimizations needed in practice
Cnew ) can disrupt the cluster. These servers will not re-           (such as leader election). The quizzes tested basic un-
ceive heartbeats, so they will time out and start new elec-          derstanding of the algorithms and also required students
tions. They will then send RequestVote RPCs with new                 to reason about corner cases. Each student watched one
term numbers, and this will cause the current leader to              video, took the corresponding quiz, watched the second
revert to follower state. A new leader will eventually be            video, and took the second quiz. About half of the par-
elected, but the removed servers will time out again and             ticipants did the Paxos portion first and the other half did
the process will repeat, resulting in poor availability.             the Raft portion first in order to account for both indi-
   To prevent this problem, servers disregard RequestVote            vidual differences in performance and experience gained
RPCs when they believe a current leader exists. Specif-              from the first portion of the study. We compared partici-
ically, if a server receives a RequestVote RPC within                pants’ scores on each quiz to determine whether partici-
the minimum election timeout of hearing from a cur-                  pants showed a better understanding of Raft.
rent leader, it does not update its term or grant its vote.             We tried to make the comparison between Paxos and



USENIX Association                                                              2014 USENIX Annual Technical Conference 315
                                                                                             20




                                                                    number of participants
                      60
                                                                                             15                               Paxos much easier
                      50                                                                                                      Paxos somewhat easier
                                                                                             10                               Roughly equal
                                                                                                                              Raft somewhat easier
                      40                                                                                                      Raft much easier


         Raft grade
                                                                                              5
                      30
                                                                                              0
                                                                                                  implement   explain
                      20
                                                                                     Figure 13: Using a 5-point scale, participants were asked
                      10                                                             (left) which algorithm they felt would be easier to implement
                                         Raft then Paxos                             in a functioning, correct, and efficient system, and (right)
                                         Paxos then Raft
                       0                                                             which would be easier to explain to a CS graduate student.
                           0   10   20   30    40    50    60
                                     Paxos grade                     reliable than participants’ quiz scores, and participants
 Figure 12: A scatter plot comparing 43 participants’ perfor-        may have been biased by knowledge of our hypothesis
 mance on the Raft and Paxos quizzes. Points above the diag-         that Raft is easier to understand.
 onal (33) represent participants who scored higher for Raft.           A detailed discussion of the Raft user study is available
Raft as fair as possible. The experiment favored Paxos in            at [28].
two ways: 15 of the 43 participants reported having some             8.2 Correctness
prior experience with Paxos, and the Paxos video is 14%                 We have developed a formal specification and a proof
longer than the Raft video. As summarized in Table 1, we             of safety for the consensus mechanism described in Sec-
have taken steps to mitigate potential sources of bias. All          tion 5. The formal specification [28] makes the informa-
of our materials are available for review [26, 28].                  tion summarized in Figure 2 completely precise using the
   On average, participants scored 4.9 points higher on the          TLA+ specification language [15]. It is about 400 lines
Raft quiz than on the Paxos quiz (out of a possible 60               long and serves as the subject of the proof. It is also use-
points, the mean Raft score was 25.7 and the mean Paxos              ful on its own for anyone implementing Raft. We have
score was 20.8); Figure 12 shows their individual scores.            mechanically proven the Log Completeness Property us-
A paired t-test states that, with 95% confidence, the true           ing the TLA proof system [6]. However, this proof relies
distribution of Raft scores has a mean at least 2.5 points           on invariants that have not been mechanically checked
larger than the true distribution of Paxos scores.                   (for example, we have not proven the type safety of the
   We also created a linear regression model that predicts           specification). Furthermore, we have written an informal
a new student’s quiz scores based on three factors: which            proof [28] of the State Machine Safety property which
quiz they took, their degree of prior Paxos experience, and          is complete (it relies on the specification alone) and rela-
the order in which they learned the algorithms. The model            tively precise (it is about 3500 words long).
predicts that the choice of quiz produces a 12.5-point dif-          8.3 Performance
ference in favor of Raft. This is significantly higher than             Raft’s performance is similar to other consensus algo-
the observed difference of 4.9 points, because many of the           rithms such as Paxos. The most important case for per-
actual students had prior Paxos experience, which helped             formance is when an established leader is replicating new
Paxos considerably, whereas it helped Raft slightly less.            log entries. Raft achieves this using the minimal number
Curiously, the model also predicts scores 6.3 points lower           of messages (a single round-trip from the leader to half the
on Raft for people that have already taken the Paxos quiz;           cluster). It is also possible to further improve Raft’s per-
although we don’t know why, this does appear to be sta-              formance. For example, it easily supports batching and
tistically significant.                                              pipelining requests for higher throughput and lower la-
   We also surveyed participants after their quizzes to see          tency. Various optimizations have been proposed in the
which algorithm they felt would be easier to implement               literature for other algorithms; many of these could be ap-
or explain; these results are shown in Figure 13. An over-           plied to Raft, but we leave this to future work.
whelming majority of participants reported Raft would be                We used our Raft implementation to measure the per-
easier to implement and explain (33 of 41 for each ques-             formance of Raft’s leader election algorithm and answer
tion). However, these self-reported feelings may be less             two questions. First, does the election process converge
 Concern                Steps taken to mitigate bias                                                Materials for review [26, 28]
 Equal lecture quality  Same lecturer for both. Paxos lecture based on and improved from exist- videos
                        ing materials used in several universities. Paxos lecture is 14% longer.
 Equal quiz difficulty Questions grouped in difficulty and paired across exams.                     quizzes
 Fair grading           Used rubric. Graded in random order, alternating between quizzes.           rubric
   Table 1: Concerns of possible bias against Paxos in the study, steps taken to counter each, and additional materials available.



316 2014 USENIX Annual Technical Conference                                                                                  USENIX Association
                     100%
                                                                                    average to elect a leader (the longest trial took 152ms).



cumulative percent
                     80%                                                            However, lowering the timeouts beyond this point violates
                     60%                                                            Raft’s timing requirement: leaders have difficulty broad-
                                                              150-150ms             casting heartbeats before other servers start new elections.
                     40%                                      150-151ms
                                                              150-155ms             This can cause unnecessary leader changes and lower
                     20%                                      150-175ms
                                                              150-200ms             overall system availability. We recommend using a con-
                                                              150-300ms             servative election timeout such as 150–300ms; such time-
                      0%
                            100         1000         10000           100000         outs are unlikely to cause unnecessary leader changes and
                     100%                                                           will still provide good availability.



cumulative percent
                     80%
                                                                                    9 Related work
                     60%
                                                                12-24ms
                                                                                       There have been numerous publications related to con-
                     40%                                        25-50ms             sensus algorithms, many of which fall into one of the fol-
                                                               50-100ms
                     20%                                      100-200ms             lowing categories:
                                                              150-300ms
                      0%                                                            • Lamport’s original description of Paxos [13], and at-
                            0     100    200      300      400       500      600      tempts to explain it more clearly [14, 18, 19].
                                          time without leader (ms)
                                                                                    • Elaborations of Paxos, which fill in missing details and
       Figure 14: The time to detect and replace a crashed leader.
       The top graph varies the amount of randomness in election
                                                                                       modify the algorithm to provide a better foundation for
       timeouts, and the bottom graph scales the minimum election                      implementation [24, 35, 11].
       timeout. Each line represents 1000 trials (except for 100 tri-               • Systems that implement consensus algorithms, such as
       als for “150–150ms”) and corresponds to a particular choice                     Chubby [2, 4], ZooKeeper [9, 10], and Spanner [5]. The
       of election timeouts; for example, “150–155ms” means that                       algorithms for Chubby and Spanner have not been pub-
       election timeouts were chosen randomly and uniformly be-                        lished in detail, though both claim to be based on Paxos.
       tween 150ms and 155ms. The measurements were taken on a                         ZooKeeper’s algorithm has been published in more de-
       cluster of five servers with a broadcast time of roughly 15ms.                  tail, but it is quite different from Paxos.
       Results for a cluster of nine servers are similar.                           • Performance optimizations that can be applied to
quickly? Second, what is the minimum downtime that can                                 Paxos [16, 17, 3, 23, 1, 25].
be achieved after leader crashes?                                                   • Oki and Liskov’s Viewstamped Replication (VR), an
   To measure leader election, we repeatedly crashed the                               alternative approach to consensus developed around the
leader of a cluster of five servers and timed how long it                              same time as Paxos. The original description [27] was
took to detect the crash and elect a new leader (see Fig-                              intertwined with a protocol for distributed transactions,
ure 14). To generate a worst-case scenario, the servers in                             but the core consensus protocol has been separated in
each trial had different log lengths, so some candidates                               a recent update [20]. VR uses a leader-based approach
were not eligible to become leader. Furthermore, to en-                                with many similarities to Raft.
courage split votes, our test script triggered a synchro-                              The greatest difference between Raft and Paxos is
nized broadcast of heartbeat RPCs from the leader before                            Raft’s strong leadership: Raft uses leader election as an
terminating its process (this approximates the behavior                             essential part of the consensus protocol, and it concen-
of the leader replicating a new log entry prior to crash-                           trates as much functionality as possible in the leader. This
ing). The leader was crashed uniformly randomly within                              approach results in a simpler algorithm that is easier to
its heartbeat interval, which was half of the minimum                               understand. For example, in Paxos, leader election is or-
election timeout for all tests. Thus, the smallest possible                         thogonal to the basic consensus protocol: it serves only as
downtime was about half of the minimum election time-                               a performance optimization and is not required for achiev-
out.                                                                                ing consensus. However, this results in additional mecha-
   The top graph in Figure 14 shows that a small amount                             nism: Paxos includes both a two-phase protocol for basic
of randomization in the election timeout is enough to                               consensus and a separate mechanism for leader election.
avoid split votes in elections. In the absence of random-                           In contrast, Raft incorporates leader election directly into
ness, leader election consistently took longer than 10 sec-                         the consensus algorithm and uses it as the first of the two
onds in our tests due to many split votes. Adding just 5ms                          phases of consensus. This results in less mechanism than
of randomness helps significantly, resulting in a median                            in Paxos.
downtime of 287ms. Using more randomness improves                                      Like Raft, VR and ZooKeeper are leader-based and
worst-case behavior: with 50ms of randomness the worst-                             therefore share many of Raft’s advantages over Paxos.
case completion time (over 1000 trials) was 513ms.                                  However, Raft has less mechanism that VR or ZooKeeper
   The bottom graph in Figure 14 shows that downtime                                because it minimizes the functionality in non-leaders. For
can be reduced by reducing the election timeout. With                               example, log entries in Raft flow in only one direction:
an election timeout of 12–24ms, it takes only 35ms on                               outward from the leader in AppendEntries RPCs. In VR



     USENIX Association                                                                        2014 USENIX Annual Technical Conference 317
log entries flow in both directions (leaders can receive           niques not only improved the understandability of Raft
log entries during the election process); this results in          but also made it easier to convince ourselves of its cor-
additional mechanism and complexity. The published de-             rectness.
scription of ZooKeeper also transfers log entries both to
and from the leader, but the implementation is apparently
                                                                   11 Acknowledgments
more like Raft [32].                                                  The user study would not have been possible with-
                                                                   out the support of Ali Ghodsi, David Mazières, and the
   Raft has fewer message types than any other algorithm
                                                                   students of CS 294-91 at Berkeley and CS 240 at Stan-
for consensus-based log replication that we are aware of.
                                                                   ford. Scott Klemmer helped us design the user study,
For example, VR and ZooKeeper each define 10 differ-
                                                                   and Nelson Ray advised us on statistical analysis. The
ent message types, while Raft has only 4 message types
                                                                   Paxos slides for the user study borrowed heavily from
(two RPC requests and their responses). Raft’s messages
                                                                   a slide deck originally created by Lorenzo Alvisi. Spe-
are a bit more dense than the other algorithms’, but they
                                                                   cial thanks go to David Mazières and Ezra Hoch for
are simpler collectively. In addition, VR and ZooKeeper
                                                                   finding subtle bugs in Raft. Many people provided help-
are described in terms of transmitting entire logs during
                                                                   ful feedback on the paper and user study materials,
leader changes; additional message types will be required
                                                                   including Ed Bugnion, Michael Chan, Hugues Evrard,
to optimize these mechanisms so that they are practical.
                                                                   Daniel Giffin, Arjun Gopalan, Jon Howell, Vimalkumar
   Several different approaches for cluster member-
                                                                   Jeyakumar, Ankita Kejriwal, Aleksandar Kracun, Amit
ship changes have been proposed or implemented in
                                                                   Levy, Joel Martin, Satoshi Matsushita, Oleg Pesok, David
other work, including Lamport’s original proposal [13],
                                                                   Ramos, Robbert van Renesse, Mendel Rosenblum, Nico-
VR [20], and SMART [22]. We chose the joint consensus
                                                                   las Schiper, Deian Stefan, Andrew Stone, Ryan Stutsman,
approach for Raft because it leverages the rest of the con-
                                                                   David Terei, Stephen Yang, Matei Zaharia, 24 anony-
sensus protocol, so that very little additional mechanism
                                                                   mous conference reviewers (with duplicates), and espe-
is required for membership changes. Lamport’s α -based
                                                                   cially our shepherd Eddie Kohler. Werner Vogels tweeted
approach was not an option for Raft because it assumes
                                                                   a link to an earlier draft, which gave Raft significant ex-
consensus can be reached without a leader. In comparison
                                                                   posure. This work was supported by the Gigascale Sys-
to VR and SMART, Raft’s reconfiguration algorithm has
                                                                   tems Research Center and the Multiscale Systems Cen-
the advantage that membership changes can occur with-
                                                                   ter, two of six research centers funded under the Fo-
out limiting the processing of normal requests; in con-
                                                                   cus Center Research Program, a Semiconductor Research
trast, VR stops all normal processing during configura-
                                                                   Corporation program, by STARnet, a Semiconductor Re-
tion changes, and SMART imposes an α -like limit on the
                                                                   search Corporation program sponsored by MARCO and
number of outstanding requests. Raft’s approach also adds
                                                                   DARPA, by the National Science Foundation under Grant
less mechanism than either VR or SMART.
                                                                   No. 0963859, and by grants from Facebook, Google, Mel-
10 Conclusion                                                      lanox, NEC, NetApp, SAP, and Samsung. Diego Ongaro
   Algorithms are often designed with correctness, effi-           is supported by The Junglee Corporation Stanford Gradu-
ciency, and/or conciseness as the primary goals. Although          ate Fellowship.
these are all worthy goals, we believe that understandabil-        References
ity is just as important. None of the other goals can be            [1] B OLOSKY, W. J., B RADSHAW, D., H AAGENS , R. B.,
achieved until developers render the algorithm into a prac-             K USTERS , N. P., AND L I , P. Paxos replicated state
tical implementation, which will inevitably deviate from                machines as the basis of a high-performance data store.
and expand upon the published form. Unless developers                   In Proc. NSDI’11, USENIX Conference on Networked
have a deep understanding of the algorithm and can cre-                 Systems Design and Implementation (2011), USENIX,
ate intuitions about it, it will be difficult for them to retain        pp. 141–154.
its desirable properties in their implementation.                   [2] B URROWS , M. The Chubby lock service for loosely-
   In this paper we addressed the issue of distributed con-             coupled distributed systems. In Proc. OSDI’06, Sympo-
sensus, where a widely accepted but impenetrable algo-                  sium on Operating Systems Design and Implementation
rithm, Paxos, has challenged students and developers for                (2006), USENIX, pp. 335–350.
many years. We developed a new algorithm, Raft, which               [3] C AMARGOS , L. J., S CHMIDT, R. M., AND P EDONE , F.
we have shown to be more understandable than Paxos.                     Multicoordinated Paxos. In Proc. PODC’07, ACM Sym-
We also believe that Raft provides a better foundation                  posium on Principles of Distributed Computing (2007),
for system building. Using understandability as the pri-                ACM, pp. 316–317.
mary design goal changed the way we approached the de-              [4] C HANDRA , T. D., G RIESEMER , R., AND R EDSTONE , J.
sign of Raft; as the design progressed we found ourselves               Paxos made live: an engineering perspective. In Proc.
reusing a few techniques repeatedly, such as decomposing                PODC’07, ACM Symposium on Principles of Distributed
the problem and simplifying the state space. These tech-                Computing (2007), ACM, pp. 398–407.



318 2014 USENIX Annual Technical Conference                                                              USENIX Association
 [5] C ORBETT, J. C., D EAN , J., E PSTEIN , M., F IKES , A.,   [20] L ISKOV, B., AND C OWLING , J. Viewstamped replica-
     F ROST, C., F URMAN , J. J., G HEMAWAT, S., G UBAREV,           tion revisited. Tech. Rep. MIT-CSAIL-TR-2012-021, MIT,
     A., H EISER , C., H OCHSCHILD , P., H SIEH , W., K AN -         July 2012.
     THAK , S., K OGAN , E., L I , H., L LOYD , A., M ELNIK ,   [21] LogCabin source code.         http://github.com/
     S., M WAURA , D., NAGLE , D., Q UINLAN , S., R AO , R.,         logcabin/logcabin.
     ROLIG , L., S AITO , Y., S ZYMANIAK , M., TAYLOR , C.,
                                                                [22] L ORCH , J. R., A DYA , A., B OLOSKY, W. J., C HAIKEN ,
     WANG , R., AND W OODFORD , D. Spanner: Google’s
                                                                     R., D OUCEUR , J. R., AND H OWELL , J. The SMART
     globally-distributed database. In Proc. OSDI’12, USENIX
                                                                     way to migrate replicated stateful services. In Proc. Eu-
     Conference on Operating Systems Design and Implemen-
                                                                     roSys’06, ACM SIGOPS/EuroSys European Conference on
     tation (2012), USENIX, pp. 251–264.
                                                                     Computer Systems (2006), ACM, pp. 103–115.
 [6] C OUSINEAU , D., D OLIGEZ , D., L AMPORT, L., M ERZ ,
                                                                [23] M AO , Y., J UNQUEIRA , F. P., AND M ARZULLO , K.
     S., R ICKETTS , D., AND VANZETTO , H. TLA+ proofs.
                                                                     Mencius: building efficient replicated state machines for
     In Proc. FM’12, Symposium on Formal Methods (2012),
                                                                     WANs. In Proc. OSDI’08, USENIX Conference on
     D. Giannakopoulou and D. Méry, Eds., vol. 7436 of Lec-
                                                                     Operating Systems Design and Implementation (2008),
     ture Notes in Computer Science, Springer, pp. 147–154.
                                                                     USENIX, pp. 369–384.
 [7] G HEMAWAT, S., G OBIOFF , H., AND L EUNG , S.-T. The       [24] M AZI ÈRES , D. Paxos made practical.           http:
     Google file system. In Proc. SOSP’03, ACM Symposium             //www.scs.stanford.edu/˜dm/home/
     on Operating Systems Principles (2003), ACM, pp. 29–43.         papers/paxos.pdf, Jan. 2007.
 [8] H ERLIHY, M. P., AND W ING , J. M. Linearizability: a      [25] M ORARU , I., A NDERSEN , D. G., AND K AMINSKY, M.
     correctness condition for concurrent objects. ACM Trans-        There is more consensus in egalitarian parliaments. In
     actions on Programming Languages and Systems 12 (July           Proc. SOSP’13, ACM Symposium on Operating System
     1990), 463–492.                                                 Principles (2013), ACM.
 [9] H UNT, P., KONAR , M., J UNQUEIRA , F. P., AND R EED ,     [26] Raft user study. http://ramcloud.stanford.
     B. ZooKeeper: wait-free coordination for internet-scale         edu/˜ongaro/userstudy/.
     systems. In Proc ATC’10, USENIX Annual Technical Con-      [27] O KI , B. M., AND L ISKOV, B. H.            Viewstamped
     ference (2010), USENIX, pp. 145–158.                            replication: A new primary copy method to support
[10] J UNQUEIRA , F. P., R EED , B. C., AND S ERAFINI , M.           highly-available distributed systems. In Proc. PODC’88,
     Zab: High-performance broadcast for primary-backup sys-         ACM Symposium on Principles of Distributed Computing
     tems. In Proc. DSN’11, IEEE/IFIP Int’l Conf. on Depend-         (1988), ACM, pp. 8–17.
     able Systems & Networks (2011), IEEE Computer Society,     [28] O NGARO , D. Consensus: Bridging Theory and Practice.
     pp. 245–256.                                                    PhD thesis, Stanford University, 2014 (work in progress).
[11] K IRSCH , J., AND A MIR , Y. Paxos for system builders.         http://ramcloud.stanford.edu/˜ongaro/
     Tech. Rep. CNDS-2008-2, Johns Hopkins University,               thesis.pdf.
     2008.                                                      [29] O NGARO , D., AND O USTERHOUT, J. In search of an un-
[12] L AMPORT, L. Time, clocks, and the ordering of events in        derstandable consensus algorithm (extended version).
     a distributed system. Commununications of the ACM 21, 7         http://ramcloud.stanford.edu/raft.pdf.
     (July 1978), 558–565.                                      [30] O USTERHOUT, J., AGRAWAL , P., E RICKSON , D.,
                                                                     KOZYRAKIS , C., L EVERICH , J., M AZI ÈRES , D., M I -
[13] L AMPORT, L. The part-time parliament. ACM Transac-
                                                                     TRA , S., N ARAYANAN , A., O NGARO , D., PARULKAR ,
     tions on Computer Systems 16, 2 (May 1998), 133–169.
                                                                     G., ROSENBLUM , M., RUMBLE , S. M., S TRATMANN ,
[14] L AMPORT, L. Paxos made simple. ACM SIGACT News                 E., AND S TUTSMAN , R. The case for RAMCloud. Com-
     32, 4 (Dec. 2001), 18–25.                                       munications of the ACM 54 (July 2011), 121–130.
[15] L AMPORT, L. Specifying Systems, The TLA+ Language         [31] Raft consensus algorithm website.
     and Tools for Hardware and Software Engineers. Addison-         http://raftconsensus.github.io.
     Wesley, 2002.                                              [32] R EED , B. Personal communications, May 17, 2013.
[16] L AMPORT, L. Generalized consensus and Paxos. Tech.        [33] S CHNEIDER , F. B. Implementing fault-tolerant services
     Rep. MSR-TR-2005-33, Microsoft Research, 2005.                  using the state machine approach: a tutorial. ACM Com-
[17] L AMPORT, L. Fast paxos. Distributed Computing 19, 2            puting Surveys 22, 4 (Dec. 1990), 299–319.
     (2006), 79–103.                                            [34] S HVACHKO , K., K UANG , H., R ADIA , S., AND
                                                                     C HANSLER , R. The Hadoop distributed file system.
[18] L AMPSON , B. W. How to build a highly available system
                                                                     In Proc. MSST’10, Symposium on Mass Storage Sys-
     using consensus. In Distributed Algorithms, O. Baboaglu
                                                                     tems and Technologies (2010), IEEE Computer Society,
     and K. Marzullo, Eds. Springer-Verlag, 1996, pp. 1–17.
                                                                     pp. 1–10.
[19] L AMPSON , B. W. The ABCD’s of Paxos. In Proc.             [35] VAN R ENESSE , R. Paxos made moderately complex.
     PODC’01, ACM Symposium on Principles of Distributed             Tech. rep., Cornell University, 2012.
     Computing (2001), ACM, pp. 13–13.



USENIX Association                                                         2014 USENIX Annual Technical Conference 319
