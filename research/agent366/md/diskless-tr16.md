 Providing Stable Storage for the Diskless Crash-Recovery
                      Failure Model
          Ellis Michael     Dan R. K. Ports      Naveen Kr. Sharma Adriana Szekeres
                    {emichael, drkp, naveenks, aaasz}@cs.washington.edu
                                     University of Washington

                                      Technical Report UW-CSE-16-08-02
                                                  August 25, 2016


Abstract                                                         model with stable storage, e.g., a disk attached to each
                                                                 process whose contents are never lost. In practice, this
Many classic protocols in the fault tolerant distributed
                                                                 assumption does not always hold. Because writing data
computing literature assume a Crash-Fail model in
                                                                 synchronously to disk incurs a high performance cost,
which processes either are up, or have crashed and
                                                                 many systems now eschew persistent disk writes in
are permanently down. While this model is useful, it
                                                                 favor of replicated in-memory storage. Furthermore,
does not fully capture the difficulties many real systems
                                                                 disks can become corrupted or totally fail, and real-
must contend with. In particular, real-world systems
                                                                 world systems must cope with these challenges.
are long-lived and must have a recovery mechanism
so that crashed processes can rejoin the system and re-             To address this need, we introduce a Diskless Crash-
store its fault-tolerance. When processes are assumed            Recovery model: processes lose their state on failures
to have access to stable storage that is persistent across       but can use a recovery protocol to regenerate their state
failures, the Crash-Recovery model is trivial. However,          upon recovery. We formally define this model and com-
because disk failures are common and because having              pare it to traditional Crash-Stop and Crash-Recovery
a disk on a protocol’s critical path is often performance        models.
concern, diskless recovery protocols are needed. While              The central question this paper answers is: How and
such protocols do exist in the state machine replication         when should an algorithm for Crash-Stop or Crash-
literature, several well-known protocols have flawed             Recovery models be transformed into one for Diskless
recovery mechanisms. We examine these errors to elu-             Crash-Recovery? It is well known that a correct al-
cidate the problem of diskless recovery and present our          gorithm for the asynchronous Crash-Stop model can
own protocol for providing virtual stable storage, trans-        be converted to an algorithm for the Crash-Recovery
forming any protocol in the Crash-Recovery with stable           model simply by recording each state transition to sta-
storage model into a protocol in the Diskless Crash-             ble storage. Is a similar transformation possible to a
Recovery model.                                                  Diskless Crash-Recovery model? Intuitively, we should
                                                                 be able to construct a diskless algorithm by replacing
1    Introduction                                                writes to stable storage with writes to a quorum of pro-
Distributed algorithms are needed to build reliable ser-         cesses so that data remains available upon recovery.
vices out of unreliable processes. To effectively support           Several algorithms have attempted to provide ad-hoc
long-lived systems, these algorithms must be both ro-            solutions to this problem in the context of specific pro-
bust to process and communication failures and able              tocols; none has been general purpose. In particular,
to help processes recover from crashes. That is, algo-           diskless recovery is of great interest for state machine
rithms must provide provable guarantees in both the              replication, which demands a reliable system that must
Crash-Stop and Crash-Recovery models.                            be both long-lived and tolerant of node and disk fail-
   Prior algorithms generally assume a Crash-Recovery            ures. To our knowledge, no current algorithm correctly

                                                             1
handles recovery in the diskless model. We examine                  node stops executing the algorithm forever and stops
three prior algorithms and demonstrate how they vio-                communicating with other nodes. We say a process
late the safety conditions of state machine replication             is UP if it is executing protocol steps and performing
under certain failure conditions.                                   actions. If it crashes, it transitions to the DOWN state.
   This paper introduces the first general purpose algo-            Once DOWN, a process no longer accepts messages that
rithm for transforming Crash-Stop or Crash-Recovery                 are sent to it and forever remains in the DOWN state.
algorithms to the Diskless Crash-Recovery model. It
                                                                    2.2   Crash-Recovery Model with Local
provides a virtual stable storage abstraction with the
                                                                          Stable Storage
same interface as a disk but constructed using dis-
tributed volatile storage. This algorithm provides dura-            In the Crash-Recovery model (CR), a process can re-
bility (i.e., any data written to the virtual stable storage        cover after crashing and resume executing the algo-
is readable later) and liveness (i.e., reads and writes             rithm. As in the CS model, a process in this model is
eventually complete). We prove that our algorithm guar-             assumed to be either UP or DOWN, but it can also transi-
antees safety in the Diskless Crash-Recovery model in               tion between the two states an infinite number of times.
all cases and guarantees termination given reasonable               A process that is DOWN can recover and transition back
assumptions about failure patterns.                                 to the UP state.
   The most closely related work to ours is Aguilera                   We want this model to capture the distinction be-
et al.’s study of fault tolerance in an earlier Crash-              tween the volatile state of the system and stable storage.
Recovery model without stable storage [1]. That work                As a result, the automaton does not recover in the same
concludes that consensus is solvable in their model only            state it was in before the crash. Rather, it can access sta-
when at least one process never crashes – an unrealistic            ble storage that it must manage explicitly. We abstract
assumption for long-lived systems, as the authors them-             access to stable storage using an additional internal au-
selves admit. Our work provides a solution even when                tomaton action, WRITE(X), that persists X on stable
no process remains always up. The key differentiator is             storage. If the action completes successfully, X will al-
that our work permits nodes to run a recovery protocol              ways be seen by the automaton irrespective of the num-
after crashes rather than being forced to immediately               ber of crashes. If the process crashes while WRITE(X)
resume the normal-case protocol.                                    is pending, X may or may not be persisted.
                                                                       When a process restarts in this model, it can recover
2     Models and Definitions                                        state by using another internal action, READ, that re-
This section formally defines three models: the classic             turns the set of all values X that were persisted using
Crash-Stop model, a Crash-Recovery model that uses                  the WRITE(X) action in the past. Our choice of a set of
stable storage, and the Diskless Crash-Recovery model               values as the interface to READ is somewhat arbitrary.
without stable storage.                                             We could instead provide a log interface or a single
   In each case, we consider a distributed system of a              atomically-updated value; these options are equivalent
fixed set of n processes, with IDs 1, . . . , n. Each process       to one another, in that each can trivially be constructed
is modeled as an I/O automaton [11] that takes input                from the others.
or an internal action, produces output, and transitions             2.3   Diskless Crash-Recovery:
between states. A distributed execution happens in dis-                   Crash-Recovery without Local Stable
crete timesteps, during each of which, one (or more)                      Storage
process(es) takes a step. Processes communicate with
each other by sending messages through a complete,                  We focus on the Diskless Crash-Recovery (DCR)
asynchronous network. This means that messages can                  model, where nodes have no access to stable storage.
be lost, duplicated, or reordered arbitrarily – but not             As in the CR model, a process can recover after crash-
modified – by the network. However, we assume the                   ing and resume execution. However, unlike CR, DCR
network cannot duplicate or drop messages an infinite               has no READ or WRITE actions.
number of times (i.e., the network is fair-lossy).                     Upon recovery, a process loses all state except its
                                                                    unique process ID in t1, 2, ..., nu. However, processes
2.1    Crash-Stop Model                                             can read from a local clock to get a number guaranteed
The Crash-Stop (CS) model assumes that processes can                to be larger than any previous number read by any previ-
fail by crashing and that crashes are permanent; a failed           ous incarnation of that process. Importantly, this makes

                                                                2
it possible to distinguish between different incarnations                 handle messages that the network drops or delays.
of the same processes.1                                                   Towards Diskless Crash Recovery. This paper
    A recovering process must bring itself to a state that                poses the question, How can a protocol from the
lets it resume its computation without violating any                      Crash-Stop or Crash-Recovery model be correctly
guarantee of the algorithm. To do so, a process that re-                  transformed to one that works in the Diskless Crash-
covers runs a distinct recovery protocol. This protocol                   Recovery model? Specifically, can we implement a vir-
can communicate with other processes to recover pre-                      tual stable storage abstraction in the DCR model that
vious state. Once the recovery protocol terminates, the                   provides the same READ/WRITE() interface defined
automaton changes its internal status to operational and                  in CR? In the DCR model, a process can persist state
resumes execution of its normal protocol.                                 only by replicating on other processes in the system.
    We describe a process that is UP as RECOVERING if                     Hence, any READ/WRITE(X) action must communi-
it is running its recovery protocol and OPERATIONAL                       cate with other processes in the system to provide the
otherwise. This distinction makes it possible to state                    same guarantees even in the presence of concurrent fail-
failure bounds in terms of the number of operational                      ures. We begin by defining the notion of virtual stable
processes, e.g., that no more than half of the processes                  storage:
can be either down or recovering at any moment. Our
definition matches the design of existing protocols (e.g.,                Virtual Stable Storage. We say that an algorithm
Viewstamped Replication [10]).                                            correctly implements virtual stable storage in the DCR
    One important property that protocols in the DCR                      model if it provides READ and WRITE(X) primitives
model should satisfy is recovery termination, which                       with the following properties:
requires a recovering process to eventually complete
recovery and become OPERATIONAL, as long as it does                       • Persistence: A successful READ outputs the set of
not crash again. We are thus not interested in trivial                      all objects the process successfully persisted through
solutions, i.e., those where recovering processes declare                   a prior invocation of WRITE(X).
themselves to be permanently DOWN and never again                         • Termination: Every READ and WRITE(X) at a pro-
participate in the normal protocol.                                         cess eventually succeeds unless the process crashes.

3     Model Transformations                                               4    State Machine Replication in
Many algorithms solve problems in the CS failure                               Diskless Crash-Recovery
model. When can these algorithms be applied to solv-
                                                                          Providing stable storage in the DCR model seems su-
ing the same problems in the CR or DCR models?
                                                                          perficially straightforward – simply replace each write
Crash-Stop to Crash-Recovery with Stable Storage.                         to disk with a write to a majority of replicas. How-
Transforming CS protocols to the CR model is triv-                        ever, there are surprisingly subtle challenges; if handled
ial when local stable storage is available. This can be                   naı̈vely, delayed messages from prior incarnations of a
done by recording every state transition to disk before                   process can lead to the illusion that a write has been
performing output actions. Equivalently, the automa-                      persisted when, in fact, it could still be lost.
ton can log every message and input action received to                       As evidence of its subtlety, several protocols have
stable storage and replay these in order on recovery.                     attempted, and failed, to provide diskless crash recov-
   Any asynchronous CS algorithm transformed in this                      ery in the context of a specific problem, state machine
way provides the same guarantees in the CR model.                         replication. State machine replication (SMR) – a classic
Because the automaton recovers in the same state it                       problem that lies at the core of many critical deployed
was in before crashing, the only difference is that it                    distributed systems [2, 7, 9, 10, 13, 15] – highlights the
does not process any messages that it received while                      relevance of diskless recovery: SMR deployments are
it was down. Asynchronous algorithms are inherently                       long-lived and must be able to handle node crashes and
robust to these omission failures [3] because they can                    recoveries, including ones where stable storage is lost.
    1 The algorithm we present in Section 5 could be easily adapted
                                                                             We analyze 3 state machine replication protocols for
to work when processes can only generate a unique number instead
                                                                          the DCR model: Viewstamped Replication [10], Paxos
of a monotonically increasing one. We assume a monotonic clock            Made Live [2], and JPaxos [6]. In each case, we show
only for simplicity of exposition.                                        that these protocols do not completely recover the state

                                                                      3
of a failed node under certain scenarios, a situation that       is suspected to have failed, a view change protocol re-
can lead to user-visible violations of protocol correct-         places the leader with a new one. Replicas increment
ness. To our knowledge, no prior work has provided a             their view numbers, stop processing requests in the old
correct recovery protocol for this problem.                      view, then send the new leader a VIEW- CHANGE mes-
   This section provides a detailed case study of cor-           sage with their log of operations. The new leader be-
rectness problems in Viewstamped Replication and an              gins processing only when it receives VIEW- CHANGE
overview of the equivalent problem in Paxos Made Live            messages from a majority of replicas, ensuring that it
and JPaxos. A more complete description, including               knows about all operations successfully completed in
detailed traces of the failure in all three protocols, is        prior views. These two protocols are equivalent to the
available in Appendix A.                                         two phases of Paxos.
4.1   Definitions                                                View Change Invariant. In VR, an important invari-
The SMR approach models a replicated service as a                ant is that each replica’s view number increases mono-
state machine. Clients submit requests to the service,           tonically: once a replica sends a VIEW- CHANGE mes-
which runs a consensus protocol (e.g., Multi-Paxos or            sage for view v, it never returns to a lower view. This is
Viewstamped Replication) to establish a global order             important for correctness because it implies that repli-
of requests; the replicas then execute that request and          cas cannot commit to new operations in prior views
respond to the client. A correct SMR protocol provides           once they have sent a VIEW- CHANGE message to the
linearizability [4]: clients receive responses as though         new leader. Without this invariant, a new leader cannot
their requests had been executed by a single system in           be guaranteed to know about all operations completed
a serial order; if operation A receives a response before        by previous leaders and would thus violate linearizabil-
B is invoked, A must precede B in that serial order.             ity.
   SMR is well known to be equivalent to repeated in-            Recovery in VR. Ensuring that nodes in VR can re-
stances of consensus, or atomic broadcast. We choose             cover from failures requires providing a recovery proce-
to frame the problem as SMR rather than consensus be-            dure. This procedure must ensure that the view change
cause the former implies a long-lived system consistent          invariant continues to hold, i.e., that each replica recov-
with our DCR model. In particular, single-instance con-          ers in a view number at least as high as the view number
sensus admits some solutions (such as requiring some             in any VIEW- CHANGE message it has ever sent.
fraction of nodes to be up for the duration of the proto-           The original version of VR [13] achieved this invari-
col [1]) that are not reasonable in the long-lived SMR           ant through stable storage. This protocol logged view
model.                                                           numbers to stable storage during view changes, but it
4.2   Viewstamped Replication                                    eschewed the use of stable storage in normal operations
                                                                 because writing every operation to disk is slow. It used
Viewstamped Replication (VR) is a classic SMR algo-              a recovery protocol to recover all other state, including
rithm. The original presentation by Oki and Liskov in            the set of committed operations.
1988 [13] used stable storage for certain parts of the              A later version, “VR Revisited” [10], claimed to pro-
replica state. VR always guarantees sequence agree-              vide a completely diskless mode of operation. It used a
ment and integrity, and it guarantees liveness as long as        recovery protocol and an extension to the view change
there is sufficient network synchrony and no more than           protocol to, in essence, replace each write to disk with
 f out of 2 f ` 1 replicas are failed. A subsequent ver-         communication with a quorum of nodes. We show be-
sion [10] revised the protocol to eliminate the need for         low that this protocol is insufficient to ensure continued
stable storage; we demonstrate that this version does            correctness of the system.
not correctly handle all loss-of-state failures.
                                                                    VR Revisited’s recovery protocol is straightforward:
   VR is a leader-based algorithm: the system moves
                                                                 the recovering replica sends a RECOVERY message to
through a series of numbered views, in which one node
                                                                 all other replicas.2 If not recovering or in the mid-
is designated as the leader. VR uses 2 protocols. Dur-
                                                                 dle of a view change, every other node replies with
ing normal case execution, the leader assigns sequence
                                                                 a RECOVERY- RESPONSE containing its view number;
numbers to incoming client requests, sends PREPARE
messages to replicas, and executes the operation once               2 This message contains a unique nonce to distinguish responses

it has received replies from a majority. When the leader         from different recoveries if a node recovers more than once.


                                                             4
the leader also includes its log of operations. Once the        PREPARE - OK messages to the leader (say it is A). This
recovering replica has received a quorum of responses           operation can succeed without C learning of the oper-
with the same view, including one from that view’s              ations, because A and B together form a quorum. If C
leader, it updates its state with the information in the        suspects that A has failed and initiates a view change,
log.                                                            it can proceed using only its own log and the delayed
   VR Revisited adds another phase to the view change           VIEW- CHANGE message from B. Neither log contains
protocol. When nodes determine a view change is                 o, so this operation will not be visible to future clients.
necessary, they increment their view number, stop                  Appendix A provides a complete trace of a lineariz-
processing requests in the old view, and send a                 ability violation in the VR Revisited protocol.
START- VIEW- CHANGE message to all other replicas.
Only when replicas receive START- VIEW- CHANGE
                                                                4.3   Other Protocols: Paxos Made Live and
messages from a quorum of replicas do they send their
                                                                      JPaxos
VIEW- CHANGE message to the new leader and proceed              Viewstamped Replication is not the only protocol that
as in the original protocol. The additional phase is in-        attempts to provide diskless recovery for SMR. The
tended to serve as the equivalent of a disk write, ensur-       same type of correctness problem exists in two other
ing that replicas do not commit to a new view until a           protocols, Paxos Made Live [2] and JPaxos [6], and
majority of them becomes aware of the intended view             we briefly review their recovery approaches here; a full
change. Together with the recovery protocol, the added          description is in Appendix A.
phase aims to prevent violation of the View Change In-
variant by ensuring that a crashed replica recovers in a        Paxos Made Live [2] is Google’s implementation of
view at least as high as any VIEW- CHANGE message it            the Multi-Paxos protocol. To handle corrupted disks,
has sent.                                                       it lets a replica rejoin the system without its previous
   By itself, however, this approach is not suffi-              state and runs an (unspecified) recovery protocol to re-
cient. The problem of persistence has only shifted a            store the application state. The recovering replica must
layer: rather than a node “forgetting” that it sent a           then wait to observe a full instance of successful con-
VIEW- CHANGE message on a crash, it can forget that
                                                                sensus before participating. This step successfully pre-
it sent a START- VIEW- CHANGE. That is, consider the            vents the replica from accepting multiple values for the
following case with three nodes:                                same instance (e.g., one before and one after the crash).
                                                                However, it does not prevent the replica from sending
 Node A initiates a view change by sending                     different promises (i.e., view change commitments) to
  START- VIEW- CHANGE v ` 1, but this message is de-            potential new leaders, which can lead to a new leader
  layed.                                                        deciding a new value for a prior successful instance of
 A    crashes     and     recovers;    it    receives          consensus.
  RECOVERY- RESPONSE s from B and C. Be-
  cause they have not seen the START- VIEW- CHANGE                 JPaxos [6], a hybrid of Multi-Paxos and VR, pro-
  message, A recovers to view v.                                vides a variety of deployment options, including a disk-
 B receives A’s START- VIEW- CHANGE and sends                  less one. Nodes in JPaxos maintain an epoch vector that
  START- VIEW- CHANGE v ` 1. Because it has a quo-              tracks which nodes have crashed and recovered to dis-
  rum of these messages (from A and itself), it sends           card lost promises made by prior incarnations of recov-
  VIEW- CHANGE v ` 1, but this message is delayed.              ered nodes. However, like VR and PML, this approach
 B      crashes,     recovers,      and      receives          encounters the same problem at a different level: cer-
  RECOVERY- RESPONSE s from nodes A and C                       tain failures during node recovery can cause the system
  during recovery. These nodes are in view v and have           to lose state and violate safety properties.
  no knowledge of the view change, so B recovers to
  view v.                                                       5     Providing Stable Storage in
B has now sent a VIEW- CHANGE message but has lost
                                                                      Diskless Crash-Recovery
all knowledge of that fact, violating the View Change           In this section we present a protocol that provides the
Invariant.                                                      virtual stable storage abstraction in the Diskless Crash-
   This leads to a violation of linearizability. B can          Recovery model along with its correctness proof. The
now accept a new operation o in view v by sending               protocol implementation is shown in Algorithm 1.

                                                            5
Algorithm 1 Single reader, single writer non-atomic set in Diskless Crash-Recovery
        Permanent Local State:                                    27: function S END -M ESSAGE(m, j)
           n P N`             Ź Number of processes               28:    m. f Ð i                          Ź Sender
           i P r1, . . . , ns     Ź Process number                29:    m.v Ð v
                                                                  30:    Send m to process j
        Volatile Local State:                                     31: end function
            v Ð rK for i P r1, . . . , nss Ź Crash vector
                                                                  32: function D ISCARD -O LD -R EPLIES
            op Ð f alse                  Ź Operational flag
                                                                  33:    while Dm P R where
            R Ð tu                      Ź Acquire reply set
                                                                  34:           m.vrm. f s ă vrm. f s do
            wÐK                    Ź Value being written
            S Ð tKu                           Ź Local set         35:        Remove m from R
                                                                  36:        S END -M ESSAGE(xACQUIRE, Sy, m. f )
   1: upon S YSTEM -I NITIALIZE                                   37:    end while
   2:    op Ð true                                                38:    while Dm, m1 P R where
   3: end upon
                                                                  39:           m. f “ m1 . f ^ m ‰ m1 do
                                                                  40:        Remove m from R
   4: upon R ECOVER
                                                                  41:    end while
   5:    vris Ð R EAD -C LOCK                                     42: end function
   6:    ACQUIRE -Q UORUM(K)
                                                                  43: upon receiving x ACQUIREy, m
   7:    op Ð true
                                                                  44: guard: op
   8: end upon
                                                                  45:    v Ð v \ m.v
   9: procedure W RITE(val)                                       46:    S Ð S Y m.S
  10: guard: op                                                   47:    m1 Ð xACQUIRE - REP, Sy
  11:    ACQUIRE -Q UORUM(val)                                    48:    S END -M ESSAGE(m1 , m. f )
  12: end procedure                                               49: end upon

  13: function ACQUIRE -Q UORUM (val)                             50: upon receiving x ACQUIRE - REPy, m
  14:    w Ð val                                                  51: guard: m.vris ě vris ^ w P m.S
  15:    R Ð tu                                                   52:    v Ð v \ m.v
  16:    Add val to S                                             53:    S Ð S Y m.S
  17:    m Ð xACQUIRE, Sy                                         54:    Add m to R
  18:    for all j P r1, . . . , ns do                            55:    D ISCARD -O LD -R EPLIES
  19:        S END -M ESSAGE(m, j)                                56: end upon
  20:    end for
  21:    Wait until |R| ą n{2
  22: end function

  23: procedure R EAD
  24: guard: op
  25:    return S ´ tKu
  26: end procedure




                                                              6
5.1   Overview                                                    cedure is not satisfied, that procedure is inactive and
                                                                  fails upon invocation.
The protocol we propose implements a single reader,
single writer, fault-tolerant non-atomic set that provides           There is a set of n processes, Π, and every process
the READ/WRITE interface and virtual stable storage               has a unique ID in t1, 2, ..., nu. Each process maintains
properties presented in Section 3. Adapted from the def-          a crash vector of length n, with one entry for each pro-
inition of regular registers [8], a non-atomic set offers         cess in system. Entry i in this vector (called i’s crash
weaker guarantees than a linearizable one. It guarantees          ID) tracks the latest incarnation of process i. When a
that a READ returns a set containing all previously suc-          process recovers, it gets a new value from its local,
cessfully written values, but it makes no other guaran-           monotonic clock and updates its crash ID in its own
tees. Importantly, the process is allowed only to write           vector. When the recovery procedure ends, the process
values to the set, never to remove them. We choose                becomes OPERATIONAL and signals this through the
these weaker guarantees for simplicity, so we can focus           op flag. A process’s crash vector is updated whenever
on the main problem – how to correctly obtain diskless            a process learns about a newer incarnation of another
persistence in the presence of crashes and recoveries.            process. Crash vectors are partially ordered, and a join
   Stronger properties can be implemented on top of               operation, denoted \, is defined over vectors, where
this basic interface. For instance, this protocol could           pv1 \ v2 qris “ maxpv1 ris, v2 risq.
easily be adapted to provide a multi-reader, multi-writer
non-atomic set. However, to guarantee that a READ                    The crash vector has two purposes: (1) to match re-
returns all previously written values, a process would            quests with their replies (e.g., to ignore old replies), and
have to read from a (simple, not necessarily consistent)          (2) to detect whether a process has crashed and recov-
quorum instead of its local set. A multi-reader, multi-           ered. Initially, each process’s crash vector is rK, . . . , Ks,
writer set would be a powerful abstraction upon which             where K is some value smaller than any value ever re-
to readily build other data structures (e.g., the shared          turned by any clock, and thus smaller than the crash ID
log of a replicated state machine). Also, it is worth             of any process that has ever crashed and begun recov-
noting that concurrent instances of this single reader,           ery.
single writer set can be run to give all processes access
                                                                     The single main function of our algorithm,
to virtual stable storage.
                                                                  ACQUIRE -Q UORUM, handles both writing values and
   While the virtual stable storage protocol is always
                                                                  recovering. ACQUIRE -Q UORUM ensures the persis-
safe, the termination of WRITE and recovery are guar-
                                                                  tence of both the process’s current crash vector (in par-
anteed only under certain assumptions. Section 5.3
                                                                  ticular, the process’s own crash ID in the vector) as well
specifies these assumptions and proves protocol cor-
                                                                  as the value to be written (K, a unique value which ev-
rectness.
                                                                  ery process’s local set contains, in the case of recovery).
5.2   Protocol Description                                        Additionally, it updates the process’s local set to con-
                                                                  tain all previously written values. This guarantees that
We now describe the protocol, which is presented as               upon recovery, if the set owner calls READ, it will get
pseudo-code in Algorithm 1. We present the algorithm              those values.
using a modified I/O automaton notation. In our proto-
col, procedures are input actions that can be invoked                ACQUIRE -Q UORUM provides these guarantees by
at any time (e.g., in a higher level protocol) by the set’s       collecting replies from a quorum of processes and en-
owner, the designated reader/writer process for which             suring that those replies are consistent per Section 5.3.
this instance of the protocol is providing virtual sta-           It uses crash vectors to detect when any process that
ble storage; functions are private methods; and upon              previously replied could have crashed and thus “forgot-
clauses specify how processes handle external events              ten” about the written value and the process’s crash ID.
(i.e., messages, the global startup event, and the transi-        The D ISCARD -O LD -R EPLIES function detects and re-
tion from DOWN to RECOVERING).                                    moves these replies from the reply set and then resends
   We use guards to prevent actions from being acti-              the original message. While recovering, processes do
vated under certain conditions. If the guard of a mes-            not send replies for WRITEs or other processes’ recov-
sage handler is not satisfied, the message is dropped,            eries; all ACQUIRE - REP messages are sent by OPERA -
and the action is not executed. If the guard of a pro-            TIONAL processes.


                                                              7
5.3   Protocol Correctness                                      a quorum promise showing that val was written and
Prior to discussing the correctness of Algorithm 1, we          that every participant had a crash vector greater than or
first define basic terms.                                       equal to its own vector when it sent the ACQUIRE mes-
                                                                sages. This implies that whenever a process finishes
Definition 1. A quorum, Q, is a set of processes such           recovery and sets its op flag to true, it must have re-
that:                                                           ceived a quorum promise showing that the participants
                                                                in its recovery had that process’s latest crash ID in their
       Q P Q “ tQ | Q P 2Π and |Q| ą |Π|{2u                     crash vectors.

   Note that @Q1 , Q2 P Q, Q1 X Q2 ‰ H (Quorum In-              Definition 7. We say that a process participates in
tersection).                                                    a quorum promise for X when it sends an ACQUIRE -
                                                                REP message that will eventually belong to a quorum
Definition 2. We say that property X is stable if the           promise that some other process receives.
following two guarantees hold:
                                                                   Unlike having a stable property, that a process par-
1. If a process, p, has property X, then as long as p           ticipated in a quorum promise holds across failures and
   does not crash, p will still have property X.                recoveries. That is, we say that a process, not a specific
2. If a process, p, has property X and p sends an AC -          incarnation of that process, participated in a quorum
   QUIRE or ACQUIRE - REP message to p1 , then upon             promise. Also note that only OPERATIONAL processes
   receiving the message, p1 will have property X.              ever participate in a quorum promise, guaranteed by the
                                                                guard on the ACQUIRE message handler.
   Note that a process having a crash vector, v, that is
greater than or equal to some value is a stable property.       Theorem 1 (Persistence of Quorum Knowledge). If at
Also, a process having some element in its local set, S,        time t, some quorum, Q, knows stable property X, then
is a stable property.                                           for all times t 1 ě t, Q knows X.
Definition 3. If a process, p, has some stable property,        Proof. We prove by (strong) induction on t 1 that the
X, we say p knows X. If a message, m, from some                 following invariant, I, holds for all t 1 ě t. For all p
process, p, indicates that at the time p sent m, p knew         in Q: (1) p is OPERATIONAL and knows X, (2) p is
X, then we say m shows X (for p).                               RECOVERING , or (3) p is DOWN . In the base case at
                                                                time t, Q knows X by assumption, so I holds.
Definition 4. We say that a quorum Q knows some                    Now, assuming I holds at all times t 1 ´ 1 ě t, we
stable property X if, for all processes p P Q, one of           show that I holds at time t 1 . Because X is stable, the
the following holds: (1) p is DOWN, (2) p is OPERA -            only step any process, p, in Q could take to falsify I
TIONAL, and p knows X, or (3) p is RECOVERING and               is finishing recovery. Either recovery began after time
is guaranteed to know X upon finishing recovery.                t, or at or before time t. If the latter, then because Q
Definition 5. A set of ACQUIRE - REP messages, R, is            knew X, p is guaranteed to know X now that it has
consistent if:                                                  finished recovering. If this recovery began after time t,
                                                                then p must have received some set of ACQUIRE - REP
          @s1 , s2 P R : s1 .vrs2 . f s ď s2 .vrs2 . f s        messages from a quorum, all of which were sent after
                                                                time t. By quorum intersection, one of these messages
Definition 6. We say that a consistent set of ACQUIRE -         must have come from some process in Q. Call this pro-
REP messages constitutes a quorum promise showing               cess q. Since q’s ACQUIRE - REP message, m, was sent
stable property X if the set of senders of those messages       after time t and before t 1 , by the induction hypothesis, q
is a quorum, and each message shows X.                          must have known X when it sent m. Since X is a stable
                                                                property, p now knows X upon finishing recovery.
  The D ISCARD -O LD -R EPLIES function (line 32)                  Since I holds for all times t 1 ě t, this implies the
guarantees the consistency of the reply set by discard-         theorem.
ing any inconsistent messages; it also guarantees that
there is at most one message from each process in               Theorem 2 (Acquisition of Quorum Knowledge). If
the reply set. Therefore, the termination of ACQUIRE -          process p receives a quorum promise showing stable
Q UORUM (line 13) means that the process has received           property X from quorum Q, then Q knows X.

                                                            8
Proof. We again prove this theorem by (strong) induc-                  R, so all of the processes in Q are either DOWN, RECOV-
tion, showing that the following invariant, I, holds for               ERING (and guaranteed to know X upon recovery), or
all times, t:                                                          are OPERATIONAL and know X. Therefore, Q knows X,
                                                                       and this completes the proof that property 1 of I holds
1. If p receives a quorum promise showing stable prop-                 at time t.
   erty X from quorum Q, then Q knows X.
2. If p ever participated in a quorum promise for X at or                 Now, let’s deal with property 2 of I. Suppose, for
   before time t, and p is OPERATIONAL, then p knows                   the sake of contradiction, that at or before time t, p
   X.                                                                  participated in quorum promise R showing X (i.e. sent
   In the base case at t “ 0, I holds vacuously since                  some message showing X that is, or will be, in a quo-
no process could have yet received or participated in a                rum promise) received by q. Further suppose that p is
quorum promise. Now, assuming I holds at time t ´1 ě                   OPERATIONAL , and p doesn’t know X. Let Q be the

0, we will show that I holds at time t.                                the set of participants in R (i.e. those processes which
                                                                       already have or will at some point participate in the
   We prove our invariant in two parts. First, let’s dis-              quorum promise, R).
patch with property 1 of I. If p has received a quorum                    Since X is a stable property, p must have crashed
promise, R, from quorum Q showing X, then because R                    and recovered since participating in R. Consider p’s
is consistent, we know that at the time they participated              most recent recovery, and let the quorum promise it
in R no process in Q had participated in the recovery3                 received showing that the senders knew p’s new crash
of any later incarnation of any other process in Q than                ID in the crash vector (or a greater one) be R1 . Let the
the one that participated in R. If they had, then by the               set of participants in R1 be Q1 . By quorum intersection,
induction hypothesis (which we can apply as their par-                 there exists some process in Q X Q1 . Consider one such
ticipation happened before time t), we would know that                 process, r.
such a process would have known the recovered pro-                        We know that r couldn’t have already participated
cess’s new crash ID in the crash vector when it partici-               in R when it participated in R1 ; otherwise by induction,
pated in R, and R would not have been consistent.                      when r participated in R1 , it would have known X, so
   Given that fact, we will use a secondary induction                  our incarnation of p at time t would know X. There-
to show that for all times, t 1 , all of the processes in Q            fore, r participated in R1 before participating in R. r
either: (1) haven’t yet participated R, (2) are DOWN, (3)              couldn’t have participated in R before time t, however;
are RECOVERING, or (4) are OPERATIONAL and know                        otherwise, by property 2 of I, it would have known p’s
X.                                                                     latest crash ID when participating in R, violating the
   In the base case at t 1 “ 0, no process in Q has yet                consistency of R.
participated in R. Now, for the inductive step, note that                 However, we know that p has received a quorum
the only step any process, q, could take that would fal-               promise for its new crash ID at or before time t. So by
sify our invariant is transitioning from RECOVERING to                 property 1 of I, which we have already shown holds at
OPERATIONAL after having participated in R. We know                    time t, Q1 knows p’s new crash ID. And, by Theorem 1,
that if q finished recovering, it must have received a                 if Q1 knows p’s new crash ID at some time less than or
quorum promise showing that the senders knew its new                   equal to t, then for all times greater than or equal to t,
ID in the crash vector. By quorum intersection, at least               Q1 knows p’s new crash ID. In particular, this means
one of these came from some process in Q; call this pro-               that for all times greater than or equal to time t, if r
cess r. We already know r couldn’t have participated                   is up, it knows p’s new crash ID. Therefore, r can’t
in q’s recovery before participating in R. So by the in-               participate in R at or after time t, contradicting the fact
duction hypothesis, r knew X at the time it participated               that r participates in R and completing the proof that
in q’s recovery. And since X is stable, q knows X, com-                property 2 holds at time t.
pleting this secondary induction.
   Finally, we know that since p has received R, at time                  Finally, as stated above, if W RITE terminates at time
t all of the processes in Q have already participated in               t, we know that the process received promise from a
   3 That is, participated in the quorum promise needed by a re-       quorum, Q, showing that val was written. So by Theo-
covering process, showing that the senders knew the recovering         rem 2, we know this means Q knows val, and by The-
process’s new ID (or a greater one) in the crash vector.               orem 1, we know that Q will continue to know val. If

                                                                   9
that process ever crashes after time t and subsequently                     in a Crash-Recovery failure model without stable stor-
recovers, we know that upon finishing recovery, it must                     age. The authors proved that consensus is impossible
know val since it must have received an ACQUIRE -                           to solve in this model unless the number of always-up
REPLY from some process in Q which knew val. There-                         processes exceeds the upper bound on processes that
fore, once a W RITE terminates, any subsequent R EAD                        are either eventually-always-down or unstable. Their
will at least return the written value, showing the safety                  failure model, however, differs significantly from ours:
property of virtual stable storage.                                         it does not make a distinction between recovering and
                                                                            operational processes. As a result, it cannot allow pro-
Termination It would be nice to guarantee the ter-
                                                                            cesses to recover their state, and hence at least one
mination of WRITE and recovery in all circumstances.
                                                                            process must remain always up. Instead, we show that
However, this is clearly impossible. In the DCR model,
                                                                            it is possible to provide stable storage in the Diskless
processes can experience “amnesia.” Consider some
                                                                            Crash-Recovery model as long as there is a quorum of
process, p, trying to recover or write a value. If every
                                                                            processes that remain operational for a sufficient time.
time any other process receives a message from p, it
                                                                            This allows any process to crash and fail at some point
replies, and then crashes and recovers (before any other
                                                                            during the execution of the protocol, which is expected
process receives a message from p), then it is obvi-
                                                                            to happen in long-lived systems.
ously impossible for p to build up distributed knowl-
                                                                                An interesting trade-off between stable storage and
edge, much less a quorum promise.
                                                                            a majority of correct process is discussed by Martin et
   Our single reader, single writer set protocol does,
                                                                            al. [12]. They consider a particular problem in the DCR
however, guarantee termination of both WRITE and
                                                                            model, eventual leader election, that has weaker require-
recovery if there is some quorum of processes, all of
                                                                            ments than emulating stable storage in this model. In
which remain OPERATIONAL for a sufficient period of
                                                                            particular, this problem can be solved even when pro-
time (and the writing or recovering process itself does
                                                                            cesses do not always recover the state that they lost.
not crash). This is easy to see since a writing or re-
covering process will eventually get an ACQUIRE - REP                       7    Conclusion
from each of these OPERATIONAL processes, and those
                                                                            This paper defines the Diskless Crash-Recovery model
replies must be consistent (since no process could have
                                                                            (DCR), which supports long-lived distributed services
a crash ID for another process in its vector higher than
                                                                            where nodes may crash and rejoin the system without
the crash ID of the latest incarnation of that process).
                                                                            their volatile state. Building a correct recovery proto-
   Furthermore we note that even though our protocol
                                                                            col for this environment is surprisingly subtle; to our
is always safe, if ever a majority of processes is DOWN
                                                                            knowledge, it has never been addressed in general. All
any given time, then no process can ever receive replies
                                                                            previous protocols for the specific case of state machine
from a quorum again, so no process will ever be able
                                                                            replication in this model violate safety under certain
to recover or write again.4
                                                                            failure scenarios. We present the first general procedure
6     Related work                                                          for transforming Crash-Stop or Crash-Recovery algo-
                                                                            rithms that rely on stable storage to the DCR model.
Early work on the Crash-Recovery failure model pro-                         Our algorithm, which uses a crash vector to achieve
posed protocols to solve the consensus problem [3, 14].                     global knowledge about recovery progress, remains cor-
These papers assume stable storage, and do not focus                        rect in all cases and terminates as long as a majority of
on state-loss failures, either assuming that processes do                   nodes remain operational.
not lose state on crash [14] or that every state transi-
tion is persisted to stable storage [3]. (Others persist
only critical state to stable storage [5].) As a result, the
main focus of this work is omission failures, i.e., the
messages that processes do not receive while down.
   Aguilera et al. [1] were the first to study consensus
   4 In fact, if there is ever a majority of processes that are DOWN

or RECOVERING (where there does not exist a set of messages
currently in the network that will allow any of them to recover),
then no process will ever be able to recover or write again.


                                                                       10
References                                                   [8] L. Lamport.    On interprocess communication,
                                                                 1986.
[1] M. K. Aguilera, M. Kawazoe, A. Wei, and C. S.
    Toueg. Failure Detection and Consensus in the            [9] L. Lamport. Paxos made simple. ACM SIGACT
    Crash-Recovery Model. In Proc. of DISC, 1998.                News 32, 2001.
[2] T. D. Chandra, R. Griesemer, and J. Redstone.           [10] B. Liskov and J. Cowling. Viewstamped replica-
    Paxos made live: an engineering perspective. In              tion revisited. Technical report, 2012.
    Proc. of PODC, 2007.
                                                            [11] N. A. Lynch and M. R. Tuttle. An Introduction to
[3] D. Dolev, R. Friedman, I. Keidar, and D. Malkhi.             Input/Output Automata. Technical report, MIT
    Failure detectors in omission failure environ-               Laboratory for Computer Science, Cambridge,
    ments. Technical report, Department of Computer              Mass., 1988.
    Science, Cornell University, 1997.
                                                            [12] C. Martin, M. Larrea, and E. Jimenez. Imple-
[4] M. Herlihy and J. M. Wing. Linearizability: A                menting the Omega Failure Detector in the Crash-
    Correctness Condition for Concurrent Objects.                Recovery Failure Model. J. Comput. Syst. Sci.,
    ACM Trans. Program. Lang. Syst., 1990.                       2009.
[5] M. Hurfin, A. Mostéfaoui, and M. Raynal. Con-          [13] B. M. Oki and B. H. Liskov. Viewstamped repli-
    sensus in Asynchronous Systems Where Pro-                    cation: A new primary copy method to support
    cesses Can Crash and Recover. In Proc. of the                highly-available distributed systems. In Proc. of
    17th Symposium on Reliable Distributed Systems,              PODC, 1988.
    SRDS, 1998.
                                                            [14] R. Oliveira, R. Guerraoui, and A. Schiper. Con-
[6] J. Kończak, N. Santos, T. Żurkowski, P. T. Wo-             sensus in the Crash Recover Model. Technical
    jciechowski, and A. Schiper. JPaxos: State ma-               report, Départment d’Informatique, École Poly-
    chine replication based on the Paxos protocol.               technique Fédérale, Lausanne, Switzerland, 1997.
    Technical report, 2011.
                                                            [15] F. B. Schneider. Implementing Fault-Tolerant Ser-
[7] L. Lamport. Time, Clocks, and the Ordering of                vices Using the State Machine Approach: A Tuto-
    Events in a Distributed System. Communications               rial. ACM Computing Surveys, 1990.
    of the ACM, 1978.




                                                       11
A     Description of Safety Violations in                          even if replicas crash and recover.
      Existing SMR Solutions                                       Diskless Recovery. The original version of VR used
As mentioned in Section 4, three prior systems have                stable storage for logging view numbers during view
proposed protocols for state machine replication in the            changes (and only for this purpose). VR Revisited elim-
Diskless Crash-Recovery model. This appendix shows                 inates this use of stable storage, attempting to emulate
safety violating traces for these protocols.                       it with a write to a quorum.
                                                                      To achieve this goal, VR Revisited uses two addi-
A.1    Viewstamp Replication Revisited                             tions to the protocol, described in Section 4. First, it
Viewstamped Replication Revisited [10] is a recent ver-            introduces a recovery protocol, where a recovering
sion of the classic VR protocol [13] that specifically             node contacts all other replicas and waits for a re-
targets the DCR model.                                             sponse from a quorum of replicas with matching view
Basic VR Protocol. VR is a leader-based algorithm;                 number. Second, it adds another phase to the view
the system moves through a series of views in which                change protocol: nodes stop processing requests when
one node is designated as the leader. That node is re-             they notice the need for a view change and send a
                                                                   START- VIEW- CHANGE message to all other replicas;
sponsible for assigning an order to operations. When
the leader receives a request from a client, it sends              they only send the DO - VIEW- CHANGE message upon
a PREPARE message containing the request and se-                   receiving START- VIEW- CHANGE messages from a quo-
quence number to the other replicas. Upon receiving                rum of replicas.
a PREPARE, a replica verifies that it is in the same view          Failure Trace. Simply replacing a log to stable stor-
as the leader and then records the request in its log              age with a write to a quorum is not sufficient to ensure
and responds to the leader with a PREPARE - OK. Once               correct recovery. In Figure 1, we show a trace with 3
the leader receives PREPARE - OK messages from a ma-               processes, in which a new leader (NL) mistakenly over-
jority of replicas, it executes the request, responds to           writes the decision of a previous leader (OL).
the client, and sends a COMMIT message to the other
replicas. The other replicas then execute the request.              Initially, NL suspects OL of failing, and sends a
   VR uses a view change protocol to mask failures                   START- VIEW- CHANGE message to node 1 to switch
of the leader. If a replica suspects the leader of hav-              to view 1.
ing failed, it notifies the other replicas. These replicas          NL crashes immediately after sending this message
stop processing requests, increment their view number,               – before node 1 receives it – then immediately ini-
stop processing requests in the old view, and send a                 tiates recovery. It sends RECOVERY messages and
DO - VIEW- CHANGE message to the leader of the new                   receives RECOVERY- RESPONSE messages from OL
view. This request includes the log of operations pre-               and 1, both of which are in view 0 – so NL recovers
viously executed or prepared by that replica. Once                   in view 0.
the new leader receives DO - VIEW- CHANGE messages                  Node 1 receives the START- VIEW- CHANGE message
from a majority of replicas, it selects the longest log,             sent by the previous incarnation of NL
and sends a START- VIEW message to notify the other                 Node 1 sends a START- VIEW- CHANGE message
replicas that they can resume normal operation. This                 to NL for view 1. Because node 1 has a quo-
protocol ensures that all successfully completed opera-              rum of START- VIEW- CHANGE messages for view
tions persist across view changes: each such operation               1 (its own and the one from NL), it also sends a
must have completed at a majority of replicas, and a                 DO - VIEW- CHANGE message to NL. Both messages
majority of replicas send logs to their leader, so the               are delayed by the network.
new leader will have learned about that operation from              Node 1 crashes and immediately recovers, sending
at least one replica.                                                RECOVERY messages to and receiving responses
   As described in Section 4, a key invariant in VR is               from OL and NL – both of which are in view 0.
that once a node has sent a DO - VIEW- CHANGE mes-                  NL       receives     START- VIEW- CHANGE        and
sage to the leader of a view, it must never accept new               DO - VIEW- CHANGE messages from node 1. It
operations in prior views – otherwise these operations               has a quorum of START- VIEW- CHANGE messages,
could be lost after the view change. A recovery pro-                 so it sends a DO - VIEW- CHANGE message for view
tocol must ensure that this property continues to hold               1. This is enough for it to complete the view change.

                                                             A.1
               OL                  NL                 1               sumably it is an application-level state transfer from
                                   X                                  a quorum of correct processes as in VR. In order to
                                                                      ensure consistency, the recovering process is not al-
      Time                                            X               lowed to participate in the protocol until it observes
                                                                      a completed instance of successful consensus after
                                                                      its recovery, i.e., until it learns that at least a quo-
                                                                      rum have agreed on a value for a new consensus in-
                                                                      stance. This mechamism suffers from a similar prob-
     Recovered process in View 0        Start-View-Change
     Recovery response                  Do-View-Change
                                                                      lem to the one in VR. Although it protects against los-
  X Crash                               Start-View for View 1         ing ACKNOWLEDGEMENTs (i.e., PREPARE - OK mes-
  OL Old Leader                                                       sages in VR), it does not protect against losing
  NL New Leader
                                                                      PROMISEs made to potential new leaders (i.e., VR’s
Figure 1: Trace showing safety violation in View-                     DO - VIEW- CHANGE messages).
stamped Replication Revisited [10]
                                                                      Failure Trace. Figure 3 shows a trace with 5 pro-
                                                                      cesses that leads to a new leader mistakenly deciding a
  It sends a START- VIEW message.                                     new value for a prior successful instance of consensus,
 Until the START- VIEW message is received, nodes                    overwriting the decision of the previous leader:
  OL and 1 are still in view 0 and do not believe a
  view change is in progress. Thus, they can commit                    Initially, node OL is the leader.
  new operations, which the new leader NL will not                     Node NL suspects the leader of having failed, so
  know about, leaving the system in an inconsistent                     sends a PROPOSE message proposing itself as the
  state..                                                               next leader.

                                                                               OL       NL       1        2     3         4         5
   In this trace, messages are reordered inside the net-
                                                                                                  X
work. In particular, messages are reordered across fail-
ures, i.e., messages from a prior incarnation of a pro-                                                         X

cess are sometimes delivered after messages from a
                                                                        Time                                              X
later one. A network with FIFO communication chan-
nels would not allow the violation described above:                                                        X

recovering nodes will receive a reply to their recov-                                             X
ery message only after their previous messages (e.g.,
                                                                                                           X
START- VIEW- CHANGE message) have been received.
                                                                                                                          X
However, message reordering is not required for this
failure case: we have found a a trace with 7 nodes that                                                         X

leads to the same behavior, even with FIFO communi-                                              X

cation channels, as shown in Figure 2.                                                                    X

                                                                                                                X
A.2          Paxos Made Live
Paxos Made Live is Google’s production implemen-
tation of a Paxos [2]. It is based on the well-known
Multi-Paxos optimization which chains together multi-
ple instances of Paxos [9] and is effectively equivalent
to VR. This system primarily uses stable storage to sup-                          Recovered process in View 0
port crash recovery, but because disks can become cor-                            Recovery response
                                                                                X Crash
                                                                                                                    Start-View-Change
                                                                                                                    Do-View-Change
rupted or otherwise fail, the authors propose a version                        OL Old Leader
                                                                                                                    Start-View for View 1
                                                                               NL New Leader
that allows recovery without disks.
Recovery Protocol. On recovery, a process first uses                  Figure 2: Trace showing safety violation in View-
a catch-up mechanism to bring itself up-to-date. The                  stamped Replication Revisited [10], assuming FIFO
specific mechanism it uses is not described, but pre-                 channels


                                                                A.2
          OL           NL            1           2            3               attempts to maintain a vector clock of how many times
         C1
                                    X                                         each node has crashed and recovered. Each node up-
  Time                                                                        dates this vector when it sends a recovery response to
         C2
                                                                              another node, and includes a copy of its vector in the
                                                                              PROPOSE - OK messages that it uses to commit to a new

                    C2                                                        view (i.e., the equivalent of the DO - VIEW- CHANGE
                                                                              message). It then uses these vectors to discard all
                                      Propose & Promise
                                                                              PROPOSE - OK messages that do not come from a con-
      Recovered process
      Recovery response               Accept & Acknowledgement                sistent quorum.
   X Crash                         Cn Consensus instance n
                                                                                 JPaxos’s epoch-based recovery protocol has one crit-
   OL Old Leader
   NL New Leader                                                              ical difference from ours. It treats recovery as complete
Figure 3: Trace showing safety violation in Paxos Made                        once it has received RECOVERY- ANSWER messages
Live [2]                                                                      from a quorum of replicas. In contrast, our protocol
                                                                              requires recovery responses from a consistent quorum
                                                                              of replicas. That is, it discards recovery responses from
 Node 1 receives NL’s proposal, and sends a                                  the quorum if it learned that the replica that sent the
  PROMISE to NL, promising that it will not accept                            response itself crashed and recovered.
  any further operations from OL.                                                This subtle change has important implications for
 Node 1 crashes and immediately recovers.                                    correctness. Without this, JPaxos does not correctly
 OL selects a new value for the next instance, C1, and                       handle recovery of nodes when the nodes they are re-
  sends ACCEPT and receives ACKNOWLEDGEMENT                                   covering from can themselves crash and recover. When
  messages from nodes 2 and 3. The operation is com-                          this happens, JPaxos can allow a node to recover in a
  mitted, and node 1 completes recovery because it                            way that simultaneously 1) causes the node to lose the
  has now observed an instance of consensus.                                  promise that it made in a previous PREPARE - OK, yet 2)
 OL selects a value for instance C2, and sends                               does not allow other nodes to detect that the promise
  ACCEPT s and receives ACKNOWLEDGEMENTs from                                 might have been lost. We give such an example below.
  nodes 1 and 3.
 Node 3 then receives NL’s PROPOSE and sends NL                                       OL        NL        1           2         3
  a PROMISE.                                                                                 (0,0,0,0,0)   X
 NL has now received a quorum of PROMISE mes-                                  Time                                   X
  sages: from itself, node 3, and the previous incarna-
  tion of node 1. None of these observed consensus                                                                               X
  instance C2, so NL can now start instance 2 of con-
                                                                                             (0,0,0,1,1)
  sensus and overwrite the previous value with its own
  ACCEPT messages.
                                                                                    Recovered process
A.3      JPaxos                                                                     Recovery response
                                                                                 X Crash
JPaxos [6] is a state machine replication protocol based                         OL Old Leader                 Recovery & RecoveryAnswer
                                                                                 NL New Leader                 Prepare & PrepareOK
on Paxos. It is a hybrid between Multi-Paxos and VR,
replacing promises with views. The protocol is pre-                            Figure 4: Trace showing safety violation in JPaxos
sented in several deployment options, including one
that does not use stable storage, i.e., supports the Disk-
                                                                              Failure Trace. Figure 4 shows a trace in which a
less Crash-Recovery model.5
                                                                              process (node 1 in the figure) commits to a new view
Recovery Protocol. JPaxos provides a protocol for                             via a PREPARE - OK message, then crashes and recovers.
the DCR model called epoch-based recovery. This pro-                          When it recovers, it recovers in the previous view with-
tocol is similar in spirit to our use of crash vectors: it                    out having learned of its promise; however, the new
   5 The protocol is actually presented as using stable storage on            leader (NL) does not learn about node 1’s crash and re-
process initialization, but only to store a monotonic value on write.         covery, and therefore does not discard its PREPARE - OK
The monotonic counter available in our DCR model fills this need.             message.

                                                                        A.3
 The system starts in a view where OL is the leader,
  and all nodes have epoch vector p0, 0, 0, 0, 0q.
 Node NL suspects OL of having failed, so sends out
  a PREPARE message proposing itself as the leader of
  the next view.
 Node 1 receives NL’s PREPARE message and sends
  a PREPARE - OK.
 Node 1 crashes and immediately recovers. It sends
  a RECOVERY message to node 2, and receives a
  RECOVERY- ANSWER . Node 2’s epoch vector is now
  p0, 0, 1, 0, 0q
 Node 2 crashes and immediately recovers. It
  sends a RECOVERY message and receives
  RECOVERY- ANSWER s from nodes OL, NL,
  and 3. All of these have epoch vector p0, 0, 0, 1, 0q,
  so 2 now has this vector as well – in other words, it
  has lost its knowledge that 1 crashed.
 Node 1 sends a RECOVERY message to node 3,
  and receives a reply. Node 3’s epoch vector is now
  p0, 0, 1, 1, 0q.
 Node 3 crashes and immediately recovers, commu-
  nicating with nodes OL, NL, and 2 during recovery.
  After recovery, its epoch vector is p0, 0, 0, 1, 1q.
 NL sends a PREPARE message to 3, and receives
  a PREPARE - OK responses. It now has a quorum of
  PREPARE - OK responses from itself, 1, and 3, so it
  can start a new view.
 Node 1 sends a RECOVERY message to node OL,
  and receives a response. It is now fully recovered in
  the original view.
 OL can now commit operations (via the quorum of
  itself, 1, and 2) which will not appear in NL’s new
  view.

   Our protocol avoids this problem by checking for a
consistent quorum on recovery. When node 1 receives
a recovery response from OL, that response will have
crash vector p0, 0, 0, 1, 1q – and so node 1 will discard
the earlier recovery responses it received from 2 and 3.
It does so because it has learned that those nodes have
crashed and recovered, and therefore their updates to
the crash vector may not be stable.




                                                            A.4
