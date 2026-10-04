                 Dynamic Reconfiguration of Primary/Backup Clusters

    Alexander Shraer      Benjamin Reed                          Dahlia Malkhi                          Flavio Junqueira
               Yahoo! Research                                 Microsoft Research                       Yahoo! Research
       {shralex, breed}@yahoo-inc.com                         dalia@microsoft.com                      fpj@yahoo-inc.com

                       Abstract                                   the road to elasticity has been error prone and hazardous:
                                                                  Presently, servers cannot be added to or removed from
   Dynamically changing (reconfiguring) the member-
                                                                  a running ZooKeeper cluster and similarly no other con-
ship of a replicated distributed system while preserving
                                                                  figuration parameter (such as server roles, network ad-
data consistency and system availability is a challenging
                                                                  dresses and ports, or the quorum system) can be changed
problem. In this paper, we show that reconfiguration can
                                                                  dynamically. A cluster can be taken down, reconfigured,
be simplified by taking advantage of certain properties
                                                                  and restarted, but (as we explain further in Section 2) this
commonly provided by Primary/Backup systems. We
                                                                  process is manually intensive, error prone and hard to ex-
describe a new reconfiguration protocol, recently imple-
                                                                  ecute correctly even for expert ZooKeeper users. Data
mented in Apache Zookeeper. It fully automates configu-
                                                                  corruption and split-brain1 caused by misconfiguration
ration changes and minimizes any interruption in service
                                                                  of Zookeeper has happened in production2 . In fact, con-
to clients while maintaining data consistency. By lever-
                                                                  figuration errors are a primary cause of failures in pro-
aging the properties already provided by Zookeeper our
                                                                  duction systems [22]. Furthermore, service interruptions
protocol is considerably simpler than state of the art.
                                                                  are currently inevitable during reconfigurations. These
                                                                  negative side-effects cause operators to avoid reconfigu-
1   Introduction                                                  rations as much as possible. In fact, operators often pre-
                                                                  fer to over-provision a Zookeeper cluster than to recon-
The ability to reconfigure systems is critical to cope with       figure it with changing load. Over-provisioning (such as
the dynamics of deployed applications. Servers per-               adding many more replicas) wastes resources and adds
manently crash or become obsolete, user load fluctu-              to the management overhead.
ates over time, new features impose different constraints;           Our work provides a reconfiguration capability using
these are all reasons to reconfigure an application to use        ZooKeeper as our primary case-study. Our experience
a different group of servers, and to shift roles and bal-         with ZooKeeper in production over the past years has
ance within a service. We refer to this ability of a system       lead us to the following requirements: first, ZooKeeper
to dynamically adapt to a changing set of machines or             is a mature product that we do not want to destabilize; a
processes as elasticity.                                          solution to the dynamic reconfiguration problem should
   Cloud computing has intensified the need for elastic           not require major changes, such as limiting concurrency
long lived distributed systems. For example, some appli-          or introducing additional system components. Second, as
cations such as sports and shopping are seasonal with             many Zookeeper-based systems are online, service dis-
heavy workload bursts during championship games or                ruptions during a reconfiguration should be minimized
peak shopping days. Such workloads mean that elastic-             and happen only in rare circumstances. Third, even if
ity is not a matter of slowly growing a cluster; it may           there are failures during reconfiguration, data integrity,
mean that a cluster grows by an order of magnitude only           consistency or service availability must not be compro-
to shrink by the same order of magnitude shortly after.           mised, for instance, split-brain or loss of service due to
   Unfortunately, at the back-end of today’s cloud ser-           partial configuration propagation should never be possi-
vices, one frequently finds a coordination service which          ble. Finally, we must support a vast number of clients
itself is not elastic, such as ZooKeeper [12]. Companies          who seamlessly migrate between configurations.
such as Facebook, LinkedIn, Netflix, Twitter, Yahoo!,                We use the Zookeeper service itself for reconfigura-
and many others, use Zookeeper to track failures and              tion, but we ruled out several straw-man approaches.
configuration changes of distributed applications; appli-         First, we could have used an external coordination ser-
cation developers just need to react to events sent to               1 In a split-brain scenario, servers form multiple groups, each inde-
them by the coordination service. However, Zookeeper              pendently processing client requests, hence causing contradictory state
users have been asking repeatedly since 2008 to facil-            changes to occur.
itate reconfiguration of the service itself, and thus far,           2 http://search-hadoop.com/m/ek5ej2dOQsB




                                                              1
vice, such as another ZooKeeper cluster, to coordinate             clients. As the service configuration changes, clients
the reconfiguration, but this would simply push the re-            should stay connected to the service. Literature rarely
configuration problems to another system and add extra             mentions the client side of reconfiguration, usually stat-
management complexity. Another naı̈ve solution would               ing the need for a name-service (such as DNS), which
be to store configuration information as a replicated ob-          is of course necessary. However, its also crucial to
ject in Zookeeper. When a ZooKeeper server instance                re-balance client connections across new configuration
comes up, it looks at its replica of the state to obtain the       servers and at the same time prevent unnecessary client
configuration from the designated object. While this so-           migration which may overload servers, severely de-
lution is simple and elegant, it is prone to inconsisten-          grading performance. We propose a probabilistic load-
cies. Some replicas may be behind others, which means              balancing scheme to move as few clients as possible
they could have different configuration states. In a fixed         and still maintain an even distribution of clients across
configuration, a consistent view of the system can be ob-          servers. When clients detect a change, they each ap-
tained by contacting a quorum of the servers. A reconfig-          ply a migration policy in a distributed fashion to decide
uration, however, changes the set of servers and therefore         whether to move to a new server, and if so, which server
guaranteeing a consistent view requires additional care.           they should move to.
Consequently, reading the configuration from an object                In summary, this paper makes the following contribu-
in Zookeeper may lead to unavailability or, even worse,            tions:
corrupt data and split-brain.
                                                                       • An observation that primary order allows for simple
   Indeed, dynamically reconfiguring a replicated dis-
                                                                         and efficient dynamic reconfiguration.
tributed system while preserving data consistency and
                                                                       • A new reconfiguration protocol for Primary/Backup
system availability is a challenging problem. We
                                                                         replication systems preserving primary order. Un-
found, however, that high-level properties provided by
                                                                         like all previous reconfiguration protocols, our new
Zookeeper simplify this task. Specifically, ZooKeeper
                                                                         algorithm does not limit concurrency, does not re-
employs a primary/backup replication scheme where a
                                                                         quire client operations to be stopped during recon-
single dynamically elected primary executes all opera-
                                                                         figurations, and does not incur a complicated man-
tions that change the state of the service and broadcasts
                                                                         agement overhead or any added complexity to nor-
state-updates to backups. This method of operation re-
                                                                         mal client operation.
quires that replicas apply state changes according to the
                                                                       • A decentralized, client-driven protocol that re-
order of primaries over time, guaranteeing a property
                                                                         balances client connections across servers in the
called primary order [13]. Interestingly, this property is
                                                                         presence of service reconfiguration. The protocol
preserved by many other primary/backup systems, such
                                                                         achieves a proven uniform distribution of clients
as Chubby [5], GFS [8], Boxwood [19], PacificA [21]
                                                                         across servers while minimizing client migration.
and Chain-Replication [20] (see Section 6). These sys-
                                                                       • Implementation of our reconfiguration and load-
tems, however, resort to an external service for reconfig-
                                                                         balancing protocols in Zookeeper (being con-
uration. In this work we show that leveraging primary
                                                                         tributed to Zookeeper codebase) and analysis of
order simplifies reconfiguration. By exploiting primary
                                                                         their performance.
order we are able to implement reconfiguration without
using an external service and with minimal changes to
ZooKeeper (in fact, reconfigurations are pipelined with            2     Background
other operations and treated similarly) while guarantee-
ing minimal disruption to the operation of a running sys-          This section provides the necessary background on
tem. We believe that our methods may be applied to effi-           ZooKeeper, its way of implementing the primary/backup
ciently reconfigure any Primary/Backup system satisfy-             approach, and the challenges of reconfiguration.
ing primary order.                                                 Zookeeper. Zookeeper totally orders all writes to its
   Previous reconfiguration approaches, such as the one            database. In addition, to enable some of the most com-
proposed by Lamport [15], may violate primary order,               mon use-cases, it executes requests of every client in
cause service disruption during reconfiguration, as well           FIFO order. Zookeeper uses a primary/backup scheme
as impose a bound on the concurrent processing of all              in which the primary executes all write operations and
operations due to uncertainty created by the ability to            broadcasts state changes to the backups using an atomic
reconfigure (see Section 2). Similar to our approach,              broadcast protocol called Zab [13]. ZooKeeper replicas
FRAPPE [4] imposes no such bounds, but requires roll-              process read requests locally. Figure 1 shows a write op-
back support and complex management of speculative                 eration received by a primary. The primary executes the
execution paths, not needed in our solution.                       write and broadcasts a state change that corresponds to
   Our reconfiguration protocol also encompasses the               the result of the execution to the backups. Zab uses quo-

                                                               2
                  Backup        Primary        Backup
                       3
                                                                                     of state-updates previously acknowledged by the
                                              3
                            c             c                                          backup, as well as the epoch eaccept .
                                                                                When b collects a quorum of ACK messages, it adopts a
                                     2
                                       w                                        history H received with the highest eaccept value, break-
                                                 w 1
                                                                                ing ties by preferring a longer H.
                                                                                   Steady-state: For every client request op, the primary
                                                                                b applies op to its update history H and sends an ACCEPT
                                                                                message to the backups containing e and the adopted his-
Figure 1: The processing of a write request by a primary. 1.
                                                                                tory H; in practice, only a delta-increment of H is sent
a backup receives the request, w; 2. the backup forwards w to
the primary; 3. the primary broadcasts the new state change, c,                 each time. When a backup receives an ACCEPT message,
that resulted from the execution of w.                                          if e ≥ eprepare , it adopts H and sets both eprepare and
                                                                                eaccept to e. It then sends an acknowledgment back to b.
                                                                                Once a quorum of followers have acknowledged the AC -
rums to commit state changes. As long as a quorum of                            CEPT message, and hence the history prefix, b commits
servers are available, Zab can broadcast messages and                           it by sending a COMMIT message to the backups.
ZooKeeper remains available.
                                                                                Primary order. Because the primary server broadcasts
Primary/Backup replication a la Zab. Zab is very sim-                           state changes, Zab must ensure that they are received
ilar to Paxos [15], with one crucial difference – the agree-                    in order. Specifically, if state change c is received by
ment is reached on full history prefixes rather than on in-                     a backup from a primary, all changes that precede c from
dividual operations. This difference allows Zab to pre-                         that primary must also have been received by the backup.
serve primary order, which may be violated by Paxos                             Zab refers to this ordering guarantee as local primary or-
(as shown in [13]). We now present an overview of the                           der. The local primary order property, however, is not
protocol executed by the primary. Note that the pro-                            sufficient to guarantee order when primaries can crash.
tocol in this section is abstract and excludes many de-                         It is also necessary that a new primary replacing a previ-
tails irrelevant to this paper. The protocol has two parts,                     ous primary guarantees that once it broadcasts new up-
each involving an interaction with a quorum: A startup                          dates, it has received all changes of previous primaries
procedure, which is performed only once, and through                            that have been delivered or that will be delivered. The
which a new leader3 determines the latest state of the                          new primary must guarantee that no state changes from
system4 ; and a steady-state procedure for committing up-                       previous primaries succeed its own state changes in the
dates, which is executed in a loop.                                             order of delivered state changes. Zab refers to this order-
   Zab refers to the period of time that a leader is ac-                        ing guarantee as global primary order.
tive as an epoch. Because there is only one leader active                          The term primary order refers to an ordering that satis-
at a time, these epochs form a sequence, and each new                           fies both local and global primary orders. While the dis-
epoch can be assigned a monotonically increasing inte-                          cussion above has been in the context of ZooKeeper and
ger called the epoch number. Specifically, each backup                          Zab, any primary/backup system in which a primary exe-
maintains two epoch identifiers: the highest epoch that it                      cutes operations and broadcasts state changes to backups
received from any primary in a startup phase, eprepare , and                    will need primary order. The importance of this property
the highest epoch of a primary whose history it adopted                         has already been highlighted in [13, 3]. Here, we further
in steady-state, eaccept .                                                      exploit this property to simplify system reconfiguration.
   Startup: A candidate leader b chooses a unique epoch                         Configurations in Zookeeper. A ZooKeeper deploy-
e and sends a PREPARE message to the backups. A                                 ment currently uses a static configuration S for both
backup receiving a PREPARE message acts as follows:                             clients and servers, which comprises a set of servers,
   • If e ≥ eprepare , it records the newly seen epoch by                       with network address information, and a quorum system.
     setting eprepare to e and then responds with an ACK                        Each server can be defined as a participant, in which case
     message back to the candidate.                                             it participates in Zab as a primary or as backup, or an ob-
                                                                                server, which means that it does not participate in Zab
   • The ACK includes a history prefix H consisting                             and only learns of state updates once they are commit-
   3 For the sake of readers familiar with Zookeeper and its terminol-          ted. For consistent operation each server needs to have
ogy, in the context of Zookeeper and Zab we use the term “leader” for           the same configuration S, and clients need to have a con-
“primary” and “follower” for “backup” (with no difference in mean-              figuration that includes some subset of S.
ing).
   4 Zab contains a preparatory step that optimistically chooses a                 Performing changes to a ZooKeeper configuration is
candidate-leader that already has the up-to-date history, eliminating the       currently a tricky task. Suppose, for example, that we are
need to copy the latest history from one of the backups during startup.         to add three new servers to a cluster of two servers. The


                                                                            3
two original members of the cluster hold the latest state,         stated that we already committed slot number 80 using
so we want one of them to be elected leader of the new             the current configuration; this could lead to inconsistency
cluster. If one of the three new servers is elected leader,        (a split brain scenario). We must therefore delay the con-
the data stored by the two original members will be lost.          sensus decisions on a slot until we know the configura-
(This could happen if the three new servers start up, form         tion in which it should be executed, i.e., after all previous
a quorum, and elect a leader before the two older servers          slots have been decided. As a remedy, Lamport proposed
start up.) Currently, membership changes are done using            to execute the configuration change α slots in the future,
a “rolling restart” – a procedure whereby servers are shut         which then allows the consensus algorithms on slots n
down and restarted in a particular order so that any quo-          through n + α − 1 to execute simultaneously with slot
rum of the currently running servers includes at least one         n. In this manner, we can maintain a ‘pipeline’ of opera-
server with the latest state. To preserve this invariant,          tions, albeit bounded by α.
some reconfigurations (in particular, the ones in which               Thus, standard SMR reconfiguration approaches limit
quorums from the old and the new configurations do not             the concurrent processing of all operations, because of
intersect) require restarting servers multiple times. Ser-         the uncertainty introduced by the ability to reconfigure.
vice interruptions are unavoidable, as all servers must be         We use a different approach that overcomes this limita-
restarted at least once. Rolling restart is manually in-           tion by exploiting primary order. Our reconfiguration al-
tensive, error prone, and hard to execute correctly even           gorithm speculatively executes any number of operations
for expert ZooKeeper users (especially if failures hap-            concurrently.
pen during reconfiguration). Furthermore, this procedure
gives no insight on how clients can discover or react to           3     Primary/Backup Reconfiguration
membership changes.
   The protocol we propose in this paper overcomes such            We start with a high level description of our reconfig-
problems and enables dynamic changes to the configura-             uration protocol. In general, in order for the system to
tion without restarting servers or interrupting the service.       correctly move from a configuration S to a configuration
                                                                   S 0 we must take the following steps [3], illustrated in
Reconfiguring a state-machine. Primary/backup repli-
                                                                   Figure 2:
cation is a special instance of a more general problem,
state-machine replication (SMR). With SMR, all repli-                  1. persist information about S 0 on stable storage at a
cas start from the same state and process the same se-                    quorum of S (more precisely, a consensus decision
quence of operations. Agreement on each operation in                      must be reached in S regarding the “move” to S 0 );
the sequence is reached using a consensus protocol such
as Paxos [15]. Similarly to our algorithm, most existing               2. deactivate S, that is, make sure that no further oper-
SMR approaches use the state-machine itself to change                     ations can be committed in S;
system configuration, that is, the reconfiguration is in-
terjected as any other operation in the sequence of state-             3. identify and transfer all committed (and potentially
machine commands [16]. The details of implementing                        committed) state from S to S 0 , persisting it on stable
this in a real system are complex, as pointed out in a                    storage at a quorum of S 0 (a consensus decision in
keynote describing the implementation of Paxos devel-                     S 0 regarding its initial state).
oped at Google [6]. One of the core difficulties is that               4. activate S 0 , so that it can independently operate and
a reconfiguration is very different from other SMR com-                   process client operations.
mands, in that it changes the consensus algorithm used
to agree on the subsequent operations in the sequence.                                  Step 2: Deactivate
                                                                                        current configuration
   To better understand the issue, notice that in SMR
there is no dependency among operations and thus sep-                                        A       B        C                          D   E
arate consensus decisions are made for the different               Step 1: Write new
“slots” in the history sequence. Thus, if operations 1             configuration
                                                                   to stable storage
through 100 are proposed by some server, it is possi-
                                                                                                                  Step 3: Transfer
ble that first operation 1 is committed, then 80, then 20,                                                        state
and so on. It is also possible that an operation proposed
by a different server is chosen for slot number 2. Sup-                                          A        B       C        D         E
pose now that a server proposes reconfiguration for slot
                                                                   Step 4: Activate new
50. If the proposal achieves a consensus decision, it is           configuration including
                                                                    D and E
most natural to expect that it changes the set of servers
that need to execute the consensus algorithm on subse-
quent slots (51 and onward). Unfortunately, above we               Figure 2: The generic approach to reconfiguration: adding
                                                                   servers D and E to a cluster of three servers A, B and C.
                                                               4
   Note that steps 1 and 2 are necessary to avoid split            steps 2–4 in order to transition to S 0 . Unlike the origi-
brain. Steps 3 and 4 make sure that no state is lost when          nal primary, the new candidate primary needs to perform
moving to S 0 . The division into four steps is logical and        a startup-phase in S 0 and discover the potential actions
somewhat arbitrary – some of these steps are often exe-            of a previous primary in S 0 as well. This presented an
cuted together.                                                    interesting challenge in the Zab realm, since a primary
   In a primary/backup system many of the steps above              in Zab usually has the most up-to-date prefix of com-
can be simplified by taking advantage of properties al-            mands, and enforces it on the backups. However, a new
ready provided by the system. In such systems, the pri-            primary elected from S might have a staler state com-
mary is the only one executing operations, producing               pared to servers in S 0 . We must therefore make sure that
state-updates which are relative to its current state. Thus,       no committed updates are lost without introducing sig-
each state-update only makes sense in the context of all           nificant changes to Zab. Below (in Section 3.1), we de-
previous updates. For this reason, such systems reach              scribe the solution we chose for this pragmatic issue and
agreement on the prefix of updates and not on individual           the Activation Property it induces.
operations. In other words, a new update can be com-                  We now dive into the details of our protocol. Due to
mitted only after all previous updates commit. This does           space limitations, we omit the formal proofs here and
not, however, limit concurrency: a primary can execute             focus on the intuition behind our algorithm.
and send out any number of state-updates speculatively
to the backups, however updates are always committed in
order and an uncommitted suffix of updates may later be            3.1    Stable primary
revoked from a backup’s log if the primary fails without
persisting the update to a sufficient number of replicas           We start by discussing the simpler case, where the pri-
(a quorum). Reconfiguration fits this framework well –             mary P of the current configuration S does not fail and
we interject a configuration update operation, cop, in the         continues to lead the next configuration. Figure 3 depicts
stream of normal state-updates, which causes a reconfig-           the flow of the protocol.
uration after previously scheduled updates are committed           pre-step: In order to overlap state-transfer with normal
(in state-machine terminology, α = 1). Thus, a reconfig-           activity, backups in S 0 connect to the current primary,
uration is persisted to stable storage in the old configura-       who initializes their state by transferring its currently
tion S just like any other operation in S (this corresponds        committed prefix of updates H. With Zab, such state-
to step 1 above). At the same time, there is no need to            transfer happens automatically once backups connect to
explicitly deactivate S – step 2 follows from the specu-           the primary, and they continue receiving from P all sub-
lative nature of the execution. Just like with any other           sequent commands (e.g., op1 and op2 in Figure 3), mak-
state-update, the primary may execute any number of                ing the transition to S 0 smooth.
subsequent operations, speculatively assuming that cop             step 1: The primary p schedules cop, the reconfigura-
commits. Primary order then makes sure that such op-               tion command, at the tail of the normal stream of up-
erations are committed only after the entire prefix up to          dates. It sends an ACCEPT message containing cop to all
the operation (including the configuration change cop) is          the backups connected to it (a backup may belong to S
committed, i.e., they can only be committed in the new             and/or to S 0 ) and waits for acknowledgments. Consensus
configuration as required by step 2.                               on the next configuration is reached once a quorum of S
   Since the primary is the only one executing operations,         acknowledges cop.
its local log includes all state changes that may have been
                                                                   step 2: The primary does not stall operations it receives
committed; hence, in step 3 there is no need to copy
                                                                   after cop. Instead, they are executed immediately and
state from other servers. Moreover, we start state transfer
                                                                   scheduled after cop. In principle, all updates following
ahead of time, to avoid delaying the primary’s pipeline.
                                                                   cop are the responsibility of S 0 .
When processing the reconfiguration operation cop, the
primary only makes sure that state transfer is complete,           step 3: Transfer of commands has already been initiated
namely that a quorum of S 0 has persisted all operations           in the pre-step; now, p waits for acknowledgement for
scheduled up to and including cop. Finally, in step 4, the         cop and the history of commands which precede it from
primary activates S 0 .                                            a quorum of S 0 .
   If the primary of S fails during reconfiguration, a can-        step 4: Once cop is acknowledged by both S and S 0 ,
didate primary in S must discover possible decisions               the primary commits cop and activates S 0 by sending an
made in step 1. If a new configuration S 0 is discovered           ACTIVATE message to backups. Similarly to an ACCEPT ,
at this stage, the candidate primary must first take steps         ACTIVATE includes the primary’s epoch e and processed
to commit the stream of commands up to (and includ-                by a backup only if e is greater or equal to this backup’s
ing) the operation proposing S 0 , and then it must repeat         eprepare .


                                                               5
                                                                            Quorums from previous and new
                   New configuration          Primary sends                                                       Primary activates
                                                                            configurations acknowledge Op1,
                    obtains history          new configuration                                                    new configuration
                                                                                      Op2, and COP
       P
                                                                             COMMIT                                  COMMIT COP
                                       Op1      Op2      COP    ACK Op1,2     Op1,2
                      H                                                                 ACK COP                       ACTIVATE


      B1                                                                     COMMIT
                                                         COP ACK Op                                                  COMMIT COP
                          H               Op1     Op2                         Op1,2     ACK COP                       ACTIVATE
                                                                   1,2


      B2
             H?               ACK                         COP                  COMMIT                                        COMMIT COP
                                          Op1     Op2                           Op1,2         ACK Op1,2 ACK COP               ACTIVATE


      B3
                                                                             COMMIT                                      COMMIT COP
                                                Op1     Op2    COP            Op1,2             ACK Op1,2 ACK COP
              H?               ACK                                                                                        ACTIVATE


      B4

            Current config S = {P, B1, B2}
            New configuration S' = {P, B3, B4}

                                     Figure 3: Reconfiguration with a stable primary P .


As mentioned earlier, in order to be compatible with                        In the following example, updates u1 through u4 are
Zookeeper’s existing mechanism for recovery from                         sent by the primary speculatively, before any of them
leader failure, we guarantee an additional property:                     commits, while u5 is scheduled after all previous up-
                                                                         dates are committed and the activation message for the
Activation Property. before ACTIVATE is received by a                    last proposed configuration (S 00 ) is sent out.
     quorum of S 0 , all updates that may have been com-
                                                                                                                         ACTIVATE(S’’) is sent
     mitted are persisted to stable storage by a quorum                    last active
     of S.                                                               configuration:
                                                                                          S                                           S’’
                                                                                          u1 cop(S’) u3 cop’(S’’) u4                      u5
To guarantee it, we make a change in step 2:                                 updates:
                                                                                          1        3       4         5         6            7
                                                                            required
step 2’: an update scheduled after cop and before the
                                                                            quorums:
                                                                                          S       S, S’ S, S’     S, S’’ S, S’’           S’’
activation message for S 0 is sent can be committed by
a primary in S 0 only once a quorum of both S and S 0                               Figure 4: Cascading reconfigurations
acknowledge the update (of course, we also require all                      Notice that for a given update, only the last active and
preceding updates to be committed). Updates scheduled                    the last proposed configuration (at the time this update is
after the ACTIVATE message for S 0 is sent, need only be                 scheduled) are involved in the protocol steps for that up-
persisted to stable storage by a quorum of S 0 in order to               date. Once there is a sufficient window of time between
be committed.                                                            reconfigurations that allows state-transfer to the last pro-
                                                                         posed configuration to complete, the primary activates
   Since the current primary is stable, it becomes the pri-              that configuration. We note that currently the described
mary of S 0 , and it may skip the startup-phase of a new                 extension of the protocol to support multiple concurrent
primary (described in Section 2), since in this case it                  reconfigurations is not being integrated into Zookeeper;
knows that no updates were committed in S 0 .                            for simplicity, a reconfiguration request is rejected if an-
Cascading reconfigurations. Even before ACTIVATE is                      other reconfiguration is currently in progress. (The issu-
sent for a configuration S 0 , another reconfiguration oper-             ing client may resubmit the reconfiguration request after
ation cop0 proposing a configuration S 00 may be sched-                  the current reconfiguration operation completes.)
uled by the primary (see Figure 4 below). For exam-
                                                                         3.2     Primary failure or replacement
ple, if we reconfigure to remove a faulty member, and
meanwhile detect another failure, we can evict the addi-                 Until now, we assumed that the primary does not fail dur-
tional member without ever going through the interme-                    ing the transition to S 0 and continues as the primary of
diate step. We streamline cascading reconfigurations by                  S 0 . It remains to ensure that when it is removed or fails,
skipping the activation of S 0 .                                         safety is still guaranteed. First, consider the case that


                                                                     6
the current primary in S needs to be replaced. There are              isting implementation of leader recovery in Zookeeper.
many reasons why we may want to replace a primary,                    Recall that the stream of updates by the previous primary
e.g., the current primary may not be in S 0 , its new role            may continue past cop, and so backups in S 0 may have a
in S 0 might not allow it to continue leading, or even if             longer history of commands than b. In Zookeeper, con-
the IP address or port it uses for communication with the             necting to b would cause them to truncate their history.
backups needs to change as part of the reconfiguration.               This is exactly why we chose to preserve the Activation
   Our framework easily accommodates this variation:                  Property. If b succeeds to connect to a quorum of S 0
The old primary can still execute operations scheduled                without learning of the activation of S 0 , we know that
after cop and send them out to connected backups but                  all updates that may have been committed are stored at
it does not commit these operations, as these logically               a quorum of S. Thus, b will find all such updates once
belong in S 0 . It is the responsibility of a new primary             completing the startup-phase in S; in fact, in Zookeeper
elected in S 0 to commit these operations. As an opti-                the candidate b is chosen (by preliminary selection) as
mization, we explicitly include in an ACTIVATE message                the most up-to-date backup in S (that can communicate
the identity of a designated, initial primary for S 0 (this is        with a quorum of S), so it will already have the full pre-
one of the backups in S 0 , which has acknowledged the                fix and no actual transfer of updates is needed during the
longest prefix of operations, including at least cop). As             startup-phase.
before, this primary does not need to execute the startup-               Finally, note that b might discover more than a single
phase in S 0 since we know that no primary previously                 future reconfiguration while performing its startup-phase
existed in S 0 . Obviously, if that default primary fails to          in S. For example, it may see that both S 0 and S 00 were
form a quorum, we fall-back to the normal primary elec-               proposed. b may in this case skip S 0 and run the startup-
tion in S 0 .                                                         phase in S and S 00 , after which it activates S 00 .
   Likewise, the case of a primary failure after S 0 has
been activated is handled as a normal Zab leader re-                  3.3    Progress guarantees
election.                                                             As in [2], the fault model represents a dynamic inter-
   An attempted reconfiguration might not even reach a                play between the execution of reconfiguration operations
quorum of backups in S, in which case it may disappear                and the “adversary”: The triggering of a reconfiguration
from the system like any other failed command.                        event from S to S 0 marks a first transition. Until this
   We are left with the interesting case when a primary-              event, a quorum of S is required to remain alive in order
candidate b in S discovers a pending attempt for a con-               for progress to be guaranteed. After it, both a quorum of
sensus on cop(S 0 ) by the previous primary. This can                 S and of S 0 are required to remain alive. The completion
mean either that cop was already decided, or simply that              of a reconfiguration is generally not known to the partic-
some backup in the quorum of b heard cop from p. As                   ipants in the system. In our protocol, it occurs when the
for any other command in the prefix b learns, it must first           following conditions are met: (a) a quorum of S 0 receives
commit cop in S (achieving the consensus decision re-                 and processes the ACTIVATE message for S 0 , and (b) all
quired in step 1). However, executing cop requires addi-              operations scheduled before S 0 is activated by a primary
tional work, and b must follow the reconfiguration steps              are committed. The former condition indicates that S 0
to implement it.                                                      can independently process new operations, while the lat-
   The only deviation from the original primary’s proto-              ter indicates that all previous operations, including those
col is that b must follow the startup-phase of a new pri-             scheduled while the reconfiguration was in progress, are
mary (Section 2) in both S and S 0 . In order to do so,               committed (it is required due to the Activation Property
b connects to the servers in S 0 . When connecting to a               and step 2’). Neither conditions are externally visible to
server b0 in S 0 , b finds out whether b0 knows of the acti-          a client or operator submitting the reconfiguration com-
vation of S 0 (or a later configuration). If S 0 has been acti-       mand. However, there is an easy way to make sure that
vated, servers in S 0 may know of newer updates unknown               both condition are met: after the reconfiguration com-
to b, hence b should not attempt to perform state transfer            pletes at the client, it can submit a no-op update opera-
(otherwise it may cause newer updates to be truncated).               tion; once it commits, we know that both conditions (a)
Instead, b restarts primary re-election in S 0 (and in partic-        and (b) are satisfied (the no-op update can be automati-
ular connects to an already elected primary in S 0 if such            cally submitted by the client-side library). An alternative
primary exists). Otherwise, b implicitly initiates state-             way to achieve this is to introduce another round to the
transfer to b0 (much like its predecessor did). This in-              reconfiguration protocol (which, for simplicity and com-
cludes at least all updates up to cop but may also include            patibility with Zab, we decided to avoid). Either way,
updates scheduled by the previous primary after cop.                  once (a) and (b) are satisfied, the fault model transitions
   This leads us to a subtle issue resulting from our de-             for the second time: only a quorum of S 0 is required to
sire to introduce as few changes as possible to the ex-               survive from now on.

                                                                  7
                                                                                                          X
            S                     S'                                      E(l(i, S 0 )) = l(i, S) +                l(j, S) ∗ P r(j → i)
                                                                                                        j∈S∧j6=i
                                                                                             X
                                                                              − l(i, S)                 P r(i → j)
                                                                                          j∈S 0 ∧j6=i
    10          10     10
                                                                         We solve for the probabilities assuming that the load
                                                                      was uniform across all servers in S and requiring that the
Figure 5: A balanced service (10 clients are connected to each        expected load remains uniform in S 0 (in the example of
server) about to move to a new configuration S 0 .                    Figure 5, we require that E(l(i, S 0 )) = 6). Intuitively,
                                                                      the probability of a client switching to a different server
4    Reconfiguring the Clients                                        depends on whether the cluster size increases or shrinks,
                                                                      and by how much. We have two cases to consider:
Once servers are able to reconfigure themselves we are                Case 1: |S| < |S 0 | Since the number of servers is in-
left with two problems at the client. First, clients need             creasing, load must move off from all servers. For a
to learn about new servers to be able to connect to                   server i ∈ M we get: E(l(i, S 0 )) = l(i, S) − l(i, S) ∗
them. This is especially important if servers that a client           P r(i → N ). We can substitute l(i, S) = C/|S| since
was using have been removed from the configuration or                 load was balanced in S, and E(l(i, S 0 )) = C/|S 0 | since
failed. Second, we need to rebalance the load on the                  this is what we would like to achieve. This gives:
servers. ZooKeeper clients use long-lived connections                 Rule 1. If |S| < |S 0 | and a client is connected to M ,
and only change the server they are connected to if it has            then with probability 1 − |S|/|S 0 | the client disconnects
failed. This means that new servers added to a config-                from its server and then connects to a random server in
uration will not take on new load until new clients start             N . That is, the choice among the servers in N is made
or other servers fail. We can solve the first problem us-             uniformly at random.
ing DNS and by having clients subscribe to configuration
                                                                         Notice that clients connected to servers in O should
changes (see Section 5) in Zookeeper. For lack of space
                                                                      move only to N as servers in M have too many clients to
here we concentrate on the second problem.
                                                                      begin with.
   Figure 5 shows a balanced service with configuration               Rule 2. If |S| < |S 0 | and a client is connected to O, then
S that is about to move to S 0 . There are 30 clients in              the client moves to a random server in N .
the system and each of the three servers in S serves 10
of the clients. When we change to S 0 we would like                   Case 2: |S| ≥ |S 0 | Since the number of servers decreases
to make sure the new system is also load-balanced. In                 or stays the same, the load on each server in S 0 will be
this example this means that each server should service 6             greater or equal to the load on each server in S. Thus, a
clients. We would also like to move as few clients as pos-            server in M will not need to decrease load:
sible since session reestablishment puts load on both the             Rule 3. If |S| ≥ |S 0 | and a client is connected to a server
clients and the servers and increases latency for client re-          in M , it should remain connected.
quests issued while the reestablishment is in process. A
                                                                         The total collective load in S 0 on all servers in M is
final goal is to accomplish the load balance using only
                                                                      the load on M in S plus the expected number of clients
logic at the clients so as not to burden the servers.
                                                                      that move to M from O:
   We denote by M the set of servers that are in both con-                 |M |C     |M |C      |O|C
figurations, S ∩ S 0 . Machines that are in the old configu-                   0
                                                                                  =          +        ∗ P r(i → M |i ∈ O)
                                                                            |S |       |S|       |S|
ration S but not in the new configuration we will label O,            We thus get our last rule:
that is, O = S \ M . Machines that are in the new config-
uration S 0 but not in the old configuration are labeled N ,          Rule 4. If |S| ≥ |S 0 | and a client is connected to a server
that is, N = S 0 \ M . Denote the total number of clients             in O, it moves to a random server in M with probability
                                                                      |M |(|S|−|S 0 |)
by C. The number of clients connected to server i in S                    |S 0 ||O|    ; otherwise, moves to a random server in N .
is denoted by l(i, S).                                                By having each client independently apply these rules,
                                       0                              we achieve uniform load in a distributed fashion.
   In general, for a server i ∈ S , the expected number
of clients that connect to i in S 0 , E(l(i, S’)) is the number       5     Implementation and Evaluation
of clients connected to it in S plus the number of clients
migrating from other servers in S to i (we denote a move              We implemented our server and client-side protocols in
from server j to server i by j → i and a move to any of               Apache Zookeeper. To this end we updated the server-
the servers in a set G by j → G) minus the number of                  side library of ZooKeeper (written in Java) as well as
clients migrating from i to other servers in S 0 :                    the two client libraries (written in Java and in C). We


                                                                  8
added a reconfig command to the API that changes the              chosen follower is removed, and (6) the follower is added
configuration, a config command that retrieves the cur-           back to the ensemble.
rent configuration and additionally allows users to sub-                                     70000
                                                                                                                                      1                      2                       3                       4                        5                      6
scribe for configuration changes and finally the update-                                     60000




                                                                      Throughput (ops/sec)
server-list command that triggers the client migration al-                                   50000

gorithm described in Section 4. We support two recon-                                        40000                                                                                                                                           15% writes
figuration modes. The first is incremental – it allows                                                                                                                                                                                       30% writes
                                                                                             30000

adding and removing servers to the current configuration.                                    20000                                                                                                                                           50% writes
The second type of reconfiguration is non-incremental,                                       10000

which means that the user specifies the new configura-                                           0
                                                                                                                                                                                                                                             100% writes

tion. This method allows changing the quorum system                                                  00:00   00:09   00:18   00:27   00:36   00:45   00:54   01:03   01:12   01:21   01:30   01:39   01:48   01:57   02:06   02:15   02:24   02:33   02:45   02:54   03:03

dynamically. We allow adding and removing servers as                                                                                                                 Time (mm:ss)

well as changing server roles. We also support dynami-                           Figure 6: Throughput during configuration changes.
cally changing the different network addresses and ports             Unsurprisingly, removing the leader has the most sig-
used by the system.                                               nificant effect on throughput. In Zookeeper, any leader
   In the remainder of this section we evaluate the impact        change (e.g., due to the failure of the previous leader)
of reconfigurations on Zookeeper clients. We focus on             always renders the system temporarily unavailable, and
the effect on throughput and latency of normal operations         a reconfiguration removing the leader is no different in
as well as on load balancing.                                     that respect. Note that in Zookeeper, each follower is
   We performed our evaluation on a cluster of 50                 connected only to one leader. Thus, when the leader
servers. Each server has one Xeon dual-core 2.13GHz               changes, followers disconnect from the old leader and
processor, 4GB of RAM, gigabit ethernet, and two SATA             only after a new leader is established can submit fur-
hard drives. The servers run RHEL 5.3 using the ext3 file         ther operations. While this explains why write opera-
system. We use the 1.6 version of Sun’s JVM.                      tions cannot be executed in the transition period (and the
   We used the Java server configured to log to one dedi-         throughput drop for a 100% write workload), the rea-
cated disk and take snapshots on another. Our benchmark           sons for disabling any read activity during leader elec-
client uses the asynchronous Java client API, and each            tion (which causes the throughput drop for read intensive
client is allowed up to 100 outstanding requests. Each            workloads) are more subtle. One of the reasons is that
request consists of a read or write of 1K of data (typ-           Zookeeper guarantees that all operations complete in the
ical operation size). We focus on read and write opera-           order they were invoked. Thus, even asynchronous in-
tions as the performance of all the operations that modify        vocations by the same thread have a well defined order
the state is approximately the same, and the performance          known in advance to the programmer. Keeping this in
of non state modifying operations is approximately the            mind, consider a read operation that follows a write by
same. When measuring throughput, clients send counts              the same client (not necessarily to the same data item).
of the number of completed operations every 300ms and             The read will only be able to complete after the write,
we sample every 3s. Finally, note that state-transfer is          whereas writes await the establishment of a new leader5 .
always performed ahead of time and a reconfig opera-                 The throughput quickly returns to normal after a leader
tion simply completes it, thus our measurements do not            crash or removal. Notice that read intensive workloads
depend on the size of the Zookeeper database.                     are more sensitive to removal and addition of follow-
                                                                  ers. This is due to the effect of client migration to other
Throughput. We first measure the effect of dynamic                followers for load balancing (we explore load-balancing
reconfigurations on throughput of normal operations. To           further in Section 5.1). Still, the change in through-
this end, we used 250 simultaneous clients executing on           put with such reconfigurations is insignificant compared
35 machines, up to 11 of which are dedicated to run               to normal fluctuations of system throughput. The rea-
Zookeeper servers (typical installations have 3-7 servers,        son is the in-order completion property of Zookeeper
so 11 is larger than a typical setting). Figure 6 shows the       mentioned above; writes, which are broadcasted by the
throughput in a saturated state as it changes over time.          leader to followers, determine the throughput of the sys-
We show measurements for workloads with 100%, 50%,                tem. More precisely, the network interface of the leader
30% and 15% write operations. The ensemble is initially           is the bottleneck. Zookeeper uses a single IP address
composed of 7 servers. The following reconfiguration              for leader-follower communication. The throughput of
events are marked on the figure: (1) a randomly chosen                5 In Zookeeper 3.4, each operation is blocked until every operation
follower is removed; (2) the follower is added back to the        (not necessarily by the same client) previously submitted to the same
ensemble; (3) the leader is removed; (4) former leader is         follower completes; this is not necessary to guarantee the in-order com-
added back to the ensemble as a follower; (5) a randomly          pletion semantics and may therefore change in the future.


                                                              9
the system therefore depends on the number of servers                                        move four randomly chosen followers which is immedi-
connected to the leader, not the number of followers in                                      ately followed by a second write batch. If we use the
the ensemble. Note, however, that removing or adding                                         reconfiguration procedure described in Section 3, we get
a server from the cluster using the reconfig command                                         an average latency again of 10.8ms. However, if we stall
does not necessarily change the number of connections.                                       the request pipeline during the reconfiguration, the aver-
Although a removal excludes a server from participat-                                        age latency increases to 15.2ms.
ing in Zab voting it does not necessarily disconnect the                                        With three replicas, our average write latency is
follower from the leader; an administrator might want                                        10.5ms. The client then requests to add back four repli-
to first allow clients to gracefully migrate to other fol-                                   cas, followed by another write batch. Using our approach
lowers and only then disconnect a removed follower or                                        write latency is at 11.4ms and jumps to 18.1ms if we stall
shut it down. In addition, removing a follower is some-                                      the pipeline.
times necessary as an intermediate step when changing
its role in the protocol (for example, in some situations                                    Leader removal. Finally, we investigate the effect of
when converting an observer to a follower). Figure 7 il-                                     reconfigurations removing the leader. Note that a server
lustrates this point. It shows two executions, with 30%                                      can never be added to a cluster as leader as we always
writes, 250 clients and 11 servers initially in the clus-                                    prioritize the current leader. Figure 8 shows the advan-
ter. There are two reconfiguration events, each removes                                      tage of designating a new leader when removing the cur-
multiple servers from the cluster. In one execution, the                                     rent one, and thus avoiding leader election. It depicts
removed servers are turned off while in the other (simi-                                     the average time to recover from a leader crash versus
larly to Figure 6) removed followers maintain their con-                                     the average time to regain system availability following
nections to the leader. The graph shows that discon-                                         the removal of the leader. The average is taken on 10
necting the servers indeed increases system throughput.                                      executions. We can see that designating a default leader
This shows, that over-provisioning a cluster by adding                                       saves up to 1sec, depending on the cluster size. As cluster
more replicas (even if those replicas are observers) can                                     size increases, leader election takes longer while using a
be detrimental to Zookeeper throughput. A better strat-                                      default leader takes constant time regardless of the clus-
egy is to reconfigure the system dynamically with chang-                                     ter size. Nevertheless, as the figure shows, cluster size
ing load.                                                                                    always affects total leader recovery time, as it includes
                                                                                             synchronizing state with a quorum of followers.
                                   (a) remove and shut‐down
                           34000
                                   running avg. of (a) since last reconfiguration                                                       2500




                                                                                                time (ms) until service re‐instatated
                                   (b) remove w/o shut‐down
                           32000
                                   running avg. of (b) since last reconfiguration
                                                                                                                                        2000




    Throughput (ops/sec)
                                                1                                   2
                           30000


                           28000                                                                                                        1500

                           26000
                                                                                                                                        1000
                           24000                                                                                                                                              leader crashes
                                                                                                                                                                              leader removed
                           22000                                                                                                        500


                           20000
                                                                                                                                           0
                                   00:00
                                   00:12
                                   00:24
                                   00:36
                                   00:48
                                   01:00
                                   01:12
                                   01:24
                                   01:36
                                   01:48
                                   02:00
                                   02:12
                                   02:24
                                   02:36
                                   02:48
                                   03:00
                                   03:12
                                   03:24
                                   03:36
                                   03:48
                                                                                                                                               3       5                  7              9
                                   04:00
                                   04:12
                                   04:24
                                   04:36
                                   04:48
                                   05:00
                                   05:12
                                   05:24
                                   05:36
                                   05:48
                                   06:00
                                   06:12
                                   06:24
                                   06:36
                                   06:48
                                   07:00
                                                                                                                                                     Number of replicas
                                                             Time (mm:ss)

                                                                                              Figure 8: Unavailability following leader removal or crash.
Figure 7: Throughput during configuration changes. Initially
there are 11 servers in the cluster. The workload includes 30%                               5.1                                          Load Balancing
writes. Configuration changes: (1) four followers are removed,                               In this section, we would like to evaluate our ap-
(2) two additional followers are removed.                                                    proach for load balancing clients as part of configura-
Latency. Next, we focus on the effect of reconfigura-                                        tion changes. To this end, we experiment with a clus-
tion on the latency of other requests. We measured the                                       ter of nine servers and 1000 clients. Clients subscribe
average latency of write operations performed by a single                                    to configuration changes using the config command and
client connected to Zookeeper; the writes are submitted                                      update their list of servers using the update-server-list
in batches of 100 operations, after all previously submit-                                   command when notified of a change. In order to avoid
ted writes complete. Initially, the cluster contains seven                                   mass migration of clients at the same time, each client
replicas and writes have an average latency of 10.8ms6 .                                     waits for a random period of time between 0 and 5sec.
   We then measured the impact of removing replicas on                                       The graphs presented below include four reconfiguration
latency. A client submits a reconfiguration request to re-                                   events: (1) remove one random server; (2) remove two
    6 the average latencies presented here are taken over 150 executions                     random servers; (3) remove one random server and add
or the described experiment and lie within 0.3ms of the real average                         the three previously removed servers, and (4) add the
with 95% confidence                                                                          server removed in step 3.


                                                                                        10
   We evaluate load balancing by measuring the mini-                                  them to randomly chosen servers in the new configu-
mum and maximum number of clients connected to any                                    ration. This, however, creates excessive migration and
of the servers and compare it to the average (number of                               unnecessary loss of throughput. Ideally, we would like
clients divided by the current number of servers). When                               the number of migrating clients to be proportional to the
the client connections are balanced across the servers, the                           change in membership. If only a single server is removed
minimum and maximum are close to the average, i.e.,                                   (or added), only clients that were (or should be) con-
there are no overloaded or under-utilized servers.                                    nected to that server should need to migrate.

Baseline. Our first baseline is the current implementa-                               Consistent Hashing. A natural way to achieve such
tion of load balancing in Zookeeper. The only measure                                 limited migration, which we use as a second baseline,
of load is currently the number of clients connected to                               is to associate each client with a server using consis-
each server, and Zookeeper is trying to keep the num-                                 tent hashing [14]. Client and server identifiers are ran-
ber of connections the same for all servers. To this end,                             domly mapped to points in an m-bit space, which can
each client creates a random permutation of the list of                               be seen as circular (i.e., 0 follows 2m − 1). Each client
servers and connects to the first server on its list. If that                         is then associated with the server that immediately fol-
server fails, it moves on to the next server on the list and                          lows it in the circle. If a server is removed, only the
so on (in round robin). This approach works reasonably                                clients that are associated with it will need to migrate by
well when system membership is fixed, and can easily                                  connecting to the next server on the circle. Similarly,
accommodate server removals. It does not, however, pro-                               if a new server is added a client migrates to it only if
vide means for incorporating a new server added to the                                the new server was inserted between the client and the
cluster. In order to account for additions in this scheme,                            server to which it is currently connected. In order to im-
we replace the client’s list with a new list of servers. The                          prove load balancing, each server is sometimes hashed k
client maintains its connection unless its current server                             times (usually k is chosen to be in the order of log(N ),
is not in the new list. Figure 9 shows that load is bal-                              where N is the number of servers). To evaluated the
anced well as long as we perform removals (steps 1 and                                approach, we implemented it in Zookeeper. Figure 10
2), however when servers are added in steps 3 and 4 the                               shows measurements for k = 1, k = 5 and k = 20.
newly added servers are under-utilized. In the beginning                              We used MD5 hashing to create random identifiers for
of step 3 there are six servers in the system, thus approx-                           clients and servers (m = 128). We can see that higher
imately 166 clients are connected to every server. When                               values of k achieve better load balancing. Note, however,
we remove a server and add three new ones in step 3,                                  that load-balancing in consistent hashing is uniform only
the clients connected to the removed server migrate to                                with “high probability”, which depends on N and k. In
a random server in the new configuration. Thus, every                                 the case of Zookeeper, where 3-7 servers (N ) are usu-
server out of the eight servers in the new configuration                              ally used, the values of N and k are not high enough to
gets an expected 21 additional clients (the newly added                               achieve reasonable load balancing.
servers will only have these clients, as no other clients
disconnect from their servers). In step 4 we add back the                             Probabilistic Load Balancing. Finally, Figure 11
last server, however no clients migrate to this server. Al-                           shows measurements of load-balancing with the ap-
though all clients find out about the change and update                               proach we have implemented in Zookeeper as outlined
their lists, no client disconnects from its server as it is                           in Section 4. Unlike consistent hashing, in this approach
still part of the system.                                                             every client makes a probabilistic decision whether and
                                  250
                                         average
                                                                                      where to migrate, such that the expected number of
                                         minimum                                      clients per server is the same for every server. As we can
                                  200




   number of clients per server
                                         maximum                                      see from the figure the difference in number of clients
                                  150                                                 between the server with the most clients and the least
                                                                                      clients is very small. Using our simple case-based prob-
                                  100
                                                                                      abilistic load balancing we are able to achieve very close
                                                                     3
                                   50
                                                                             4
                                                                                      to optimal load-balance using logic entirely at the client.
                                           1            2

                                    0
                                                                                      6   Related Work
                                                      Time (mm:ss)                    Primary order is commonly guaranteed by Pri-
                                        Figure 9: Baseline load balancing.            mary/Backup replication systems, e.g., Chubby [5],
                                                                                      GFS [8], Boxwood [19], PacificA [21], chain replica-
  To mitigate the problem illustrated in Figure 9 we                                  tion [20], Harp [17] and Echo [11]. Although Paxos
could of course disconnect all clients and re-connect                                 does not guarantee primary order [13], some systems


                                                                                 11
                                450                                                                                                                                                                                                                                                                      250
                                                                                 average                                                                                                                                                                                                                                           average                                                                                                                                                                                                250
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                average
                                400
                                                                                 minimum                                                                                                                                                                                                                                           minimum
                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                minimum




                                                                                                                                                                                                                                                                          number of clients per server
                                                                                                                                                                                                                                                                                                                                   maximum




 number of clients per server
                                350                                                                                                                                                                                                                                                                      200                                                                                                                                                                                                                              200




                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           number of clients per server
                                                                                 maximum                                                                                                                                                                                                                                                                                                                                                                                                                                                                        maximum
                                300
                                                                                                                                                                                                                                                                                                         150                                                                                                                                                                                                                              150
                                250

                                200                                                                                                                                                      3
                                                                                         1                                                                                                                                                                                                               100                                                                                                                                                                                                                              100
                                150

                                100
                                                                                                                                     2                                                                                                                                                                    50                                                                                                                                                                                                                               50
                                50                                                                                                                                                                                                       4                                                                                                   1                                             2                                  3                                                                                                                                         1                                               2                                               3                                           4
                                                                                                                                                                                                                                                                                                                                                                                                                                                                   4

                                  0                                                                                                                                                                                                                                                                       0                                                                                                                                                                                                                                 0
                                                        00:00    00:05   00:09   00:13       00:17   00:22   00:26   00:30   00:34    00:39   00:43   00:47   00:51   00:55    01:00   01:04   01:08   01:12   01:17   01:21   01:25    01:29    01:33    01:38                                                   00:00   00:05   00:11    00:18   00:24   00:28   00:33   00:38   00:43   00:48   00:52   00:57   01:02   01:07   01:12   01:17   01:22   01:27   01:32   01:37   01:42   01:47   01:51                                        00:00   00:05   00:09   00:13   00:18   00:22   00:26   00:31   00:35       00:39   00:44   00:48   00:52   00:57   01:01   01:05   01:10   01:14   01:18   01:23   01:27   01:31   01:36

                                                                                                                                          Time (mm:ss)                                                                                                                                                                                                                             Time (mm:ss)                                                                                                                                                                                                                              Time (mm:ss)


                                                                                         Figure 10: Load balancing using consistent hashing, with k = 1 (left), k = 5 (middle), and k = 20 (right).

                                                                250
                                                                                                             average                                                                                                                                                                                                                                                                                               tion 2). In addition, SMART uses configuration-specific
                                                                200
                                                                                                             minimum                                                                                                                                                                                                                                                                                               replicas: if the cluster consists of replicas A, B, and C




                                 number of clients per server
                                                                                                             maximum                                                                                                                                                                                                                                                                                               and we are replacing C with D, SMART runs two repli-
                                                                150                                                                                                                                                                                                                                                                                                                                                cas of A and two of B, one in the new configuration
                                                                100
                                                                                                                                                                                                                                                                                                                                                                                                                   and one in the old, each running its own instance of the
                                                                                                                                                                                                                                                                                                                                                                                                                   replication protocol. An important design consideration
                                                                 50
                                                                                                             1                                                                2                                                              3                                                                            4
                                                                                                                                                                                                                                                                                                                                                                                                                   in our work has been to introduce minimal changes to
                                                                   0
                                                                                                                                                                                                                                                                                                                                                                                                                   Zookeeper, as it is used in production by many com-
                                                                            00:00        00:04       00:09    00:13     00:18        00:22    00:26     00:30     00:35       00:39    00:43     00:48     00:52       00:56    01:01      01:05         01:09    01:13              01:18                01:22      01:27    01:31       01:35    01:40     01:44                                                 mercial companies. Dynamically creating additional
                                                                                                                                                                                Time (mm:ss)                                                                                                                                                                                                                       Zookeeper replicas just for the purpose of reconfigura-
                                                                                                                                                                                                                                                                                                                                                                                                                   tion adds an implementation and management overhead
                      Figure 11: Load balancing using our method (Section 4).                                                                                                                                                                                                                                                                                                                                      that would not be acceptable to Zookeeper users. Un-
                                                                                                                                                                                                                                                                                                                                                                                                                   like SMART, we do not limit concurrency or require any
implementing Paxos (such as Chubby and Boxwood)                                                                                                                                                                                                                                                                                                                                                                    additional resources to reconfigure.
have one outstanding decree at-a-time, which in fact                                                                                                                                                                                                                                                                                                                                                                  FRAPPE [4] proposes a different solution. Each server
achieves primary-order. This is done primarily to                                                                                                                                                                                                                                                                                                                                                                  in FRAPPE works with a set of possible configurations,
simplify implementation and recovery [19]. Unlike                                                                                                                                                                                                                                                                                                                                                                  similarly to RAMBO. If a reconfiguration is proposed for
such approaches, we do not limit the the concurrent                                                                                                                                                                                                                                                                                                                                                                history slot n, any number of operations can be proposed
processing of operations.                                                                                                                                                                                                                                                                                                                                                                                          after n, however their completion is speculative – users
   Unlike systems such as RAMBO [9], Boxwood [19],                                                                                                                                                                                                                                                                                                                                                                 are aware that even though these operations commit they
GFS [8], Chubby [5], chain replication [20] and Paci-                                                                                                                                                                                                                                                                                                                                                              may later be rolled back if a different operation is chosen
ficA [21] that use an external reconfiguration service,                                                                                                                                                                                                                                                                                                                                                            for slot n. This requires servers to maintain a speculative
we use the system itself as the reconfiguration engine,                                                                                                                                                                                                                                                                                                                                                            execution tree, each branch corresponding to an assump-
exploiting the primary order property to streamline re-                                                                                                                                                                                                                                                                                                                                                            tion on the decision on some reconfiguration for a par-
configurations with other operations. Zookeeper is of-                                                                                                                                                                                                                                                                                                                                                             ticular history slot. In case the reconfiguration is chosen
ten used by other systems for the exact same purpose,                                                                                                                                                                                                                                                                                                                                                              for slot n and once state transfer is complete, the spec-
and thus relying on another system for reconfiguring                                                                                                                                                                                                                                                                                                                                                               ulative operations become permanently committed and
Zookeeper would simply push the problem further as                                                                                                                                                                                                                                                                                                                                                                 the corresponding tree-branch is merged into the “trunk”.
well as introduce additional management overhead. An                                                                                                                                                                                                                                                                                                                                                               Otherwise, the branch is simply abandoned. Similarly to
additional difference from RAMBO is that in our design,                                                                                                                                                                                                                                                                                                                                                            SMART and FRAPPE, we do not require any intersec-
every backup has a single “active” configuration in which                                                                                                                                                                                                                                                                                                                                                          tion between the memberships of consecutive configura-
it operates, unlike in RAMBO where servers maintain a                                                                                                                                                                                                                                                                                                                                                              tions. The algorithm presented in this paper processes
set of possible configurations, and operate in all of them                                                                                                                                                                                                                                                                                                                                                         updates speculatively, similar to FRAPPE. However, our
simultaneously. Finally, RAMBO and several other re-                                                                                                                                                                                                                                                                                                                                                               algorithm does not require servers to work with or ex-
configurable systems (see [1] for a survey), are designed                                                                                                                                                                                                                                                                                                                                                          plicitly manage multiple configurations and it does not
for reconfiguring read/write storage, whereas Zookeeper                                                                                                                                                                                                                                                                                                                                                            expose speculative operation completions to the clients.
provides developers with arbitrary functionality, i.e., a                                                                                                                                                                                                                                                                                                                                                             Group communication systems that provide virtual
universal object via consensus [10]; the read/write recon-                                                                                                                                                                                                                                                                                                                                                         synchrony [7, 3] are perhaps closer to Zookeeper than
figuration problem is conceptually different [2] than the                                                                                                                                                                                                                                                                                                                                                          Paxos-style replicated state machines. In such systems, a
one we address in this paper.                                                                                                                                                                                                                                                                                                                                                                                      group of processes may exchange messages with others
   SMART [18] is perhaps the most practical implemen-                                                                                                                                                                                                                                                                                                                                                              in the group, and the membership of the group (called a
tation of Paxos [15] SMR published in detail. SMART                                                                                                                                                                                                                                                                                                                                                                view) may change. Virtual synchrony guarantees that all
uses Lamport’s α parameter to bound the number of                                                                                                                                                                                                                                                                                                                                                                  processes transferring from one view to the next agree on
operations that may be executed concurrently (see Sec-                                                                                                                                                                                                                                                                                                                                                             the set of messages received in the previous view. Note


                                                                                                                                                                                                                                                                                                                                                                                               12
that they do not necessarily agree on the order of mes-            comments and thorough reviews of this work. Finally,
sages, and processes that did not participate in the pre-          we would like to thank the anonymous reviewers and our
vious view do not have to deliver these messages. Still,           shepherd, Christopher Small, for their comments.
virtual synchrony is similar to primary order in the sense
that it does not allow messages sent in different con-             References
figurations to interleave just as primary order does not            [1] AGUILERA , M. K., K EIDAR , I., M ALKHI , D., M ARTIN , J.-P.,
allow messages sent by different leaders to interleave.                 AND S HRAER , A. Reconfiguring replicated atomic storage: A
                                                                        tutorial. Bulletin of the EATCS 102 (2010), 84–108.
Unlike state-machine replication systems, which remain
                                                                    [2] AGUILERA , M. K., K EIDAR , I., M ALKHI , D., AND S HRAER ,
available as long as a quorum of the processes are alive,               A. Dynamic atomic storage without consensus. J. ACM 58, 2
group communication systems must react to every fail-                   (2011), 7.
ure by removing the faulty process from the view. While             [3] B IRMAN , K., M ALKHI , D., AND VAN R ENESSE , R. Virtually
                                                                        synchronous methodology for dynamic service replication. Tech.
this reconfiguration is in progress, client operations are              Rep. 151, MSR, Nov. 2010.
not processed. Other systems, such as Harp [17] and                 [4] B ORTNIKOV, V., C HOCKLER , G., P ERELMAN , D., ROYTMAN ,
                                                                        A., S HACHOR , S., AND S HNAYDERMAN , I. Frappé : Fast repli-
Echo [11] follow similar methodology, stopping all client               cation platform for elastic services. In ACM LADIS (2011).
operations during reconfigurations. Conversely, our de-             [5] B URROWS , M. The chubby lock service for loosely-coupled dis-
sign (similarly to state-machine replication systems) tol-              tributed systems. In OSDI (2006), pp. 335–350.
erates failures as long as a quorum of the replicas remains         [6] C HANDRA , T. D., G RIESEMER , R., AND R EDSTONE , J. Paxos
                                                                        made live: an engineering perspective. In PODC (2007), pp. 398–
available, and allows executing client operations while                 407.
reconfiguration and state-transfer are in progress.                 [7] C HOCKLER , G., K EIDAR , I., AND V ITENBERG , R. Group com-
                                                                        munication specifications: a comprehensive study. ACM Comput.
7   Conclusions                                                         Surv. 33, 4 (2001), 427–469.
                                                                    [8] G HEMAWAT, S., G OBIOFF , H., AND L EUNG , S.-T. The google
Reconfiguration is hard in general. It becomes espe-                    file system. In SOSP (2003), pp. 29–43.
cially hard when reconfiguring the configuration service.           [9] G ILBERT, S., LYNCH , N. A., AND S HVARTSMAN , A. A.
                                                                        Rambo: a robust, reconfigurable atomic memory service for dy-
While intuitively it seems simple, care must be taken to                namic networks. Distributed Computing 23, 4 (2010), 225–272.
address all failure cases and execution orderings.                 [10] H ERLIHY, M. Wait-free synchronization. ACM Trans. Program.
   Our reconfiguration protocol builds on properties of                 Lang. Syst. 13, 1 (1991), 124–149.
Primary/Backup systems to achieve high performance                 [11] H ISGEN , A., B IRRELL , A., J ERIAN , C., M ANN , T.,
                                                                        S CHROEDER , M., AND S WART, G. Granularity and semantic
reconfigurations without imposing a bound on concur-                    level of replication in the echo distributed file system. In Pro-
rent processing of operations or stalling them, and with-               ceedings of the IEEE Workshop on the Management of Replicated
                                                                        Data (November 1990).
out the high management price of previous proposals.               [12] H UNT, P., KONAR , M., J UNQUEIRA , F. P., AND R EED , B.
   The load balancing algorithm for distributing clients                Zookeeper: Wait-free coordination for internet-scale systems. In
                                                                        USENIX Annual Technology Conference (2010) (2010).
across servers in a new configuration involves decisions
                                                                   [13] J UNQUEIRA , F. P., R EED , B. C., AND S ERAFINI , M. Zab:
made locally at the client in a completely distributed                  High-performance broadcast for primary-backup systems. In
fashion. We guarantee uniform expected load while                       DSN (2011), pp. 245–256.
moving a minimum number of clients between servers.                [14] K ARGER , D. R., L EHMAN , E., L EIGHTON , F. T., PANIGRAHY,
                                                                        R., L EVINE , M. S., AND L EWIN , D. Consistent hashing and
   We implemented our protocols in an existing open-                    random trees: Distributed caching protocols for relieving hot
source primary/backup system, and are currently work-                   spots on the world wide web. In STOC (1997), pp. 654–663.
ing on integrating it into production. This involved sim-          [15] L AMPORT, L. The part-time parliament. ACM Trans. Comput.
                                                                        Syst. 16, 2 (1998), 133–169.
ple changes, mostly to the commit and recovery opera-              [16] L AMPORT, L., M ALKHI , D., AND Z HOU , L. Reconfiguring a
tions of Zookeeper. Our evaluation shows that there are                 state machine. SIGACT News 41, 1 (2010), 63–73.
minimal disruptions in both throughput and latency using           [17] L ISKOV, B., G HEMAWAT, S., G RUBER , R., J OHNSON , P.,
                                                                        S HRIRA , L., AND W ILLIAMS , M. Replication in the harp file
our approach.                                                           system. In SOSP (1991), pp. 226–238.
   While the methods described in this paper were im-              [18] L ORCH , J. R., A DYA , A., B OLOSKY, W. J., C HAIKEN , R.,
plemented in the context of ZooKeeper, the primary or-                  D OUCEUR , J. R., AND H OWELL , J. The smart way to migrate
                                                                        replicated stateful services. In EuroSys (2006), pp. 103–115.
der property we have taken advantage of is commonly
                                                                   [19] M AC C ORMICK , J., M URPHY, N., NAJORK , M., T HEKKATH ,
provided by Primary/Backup systems.                                     C. A., AND Z HOU , L. Boxwood: Abstractions as the foundation
                                                                        for storage infrastructure. In OSDI (2004), pp. 105–120.
                                                                   [20] VAN R ENESSE , R., AND S CHNEIDER , F. B. Chain replication
Acknowledgments                                                         for supporting high throughput and availability. In OSDI (2004),
                                                                        pp. 91–104.
                                                                   [21] W EI L IN , M AO YANG , L. Z., AND Z HOU , L. Pacifica: Replica-
We would like to thank Marshall McMullen for his                        tion in log-based distributed storage systems. Tech. Rep. MSR-
valuable contributions to this project. We thank the                    TR-2008-25, MSR, Feb. 2008.
Zookeeper open source community and in particular to               [22] Y IN , Z., M A , X., Z HENG , J., Z HOU , Y., BAIRAVASUNDARAM ,
                                                                        L. N., AND PASUPATHY, S. An empirical study on configuration
Vishal Kher, Mahadev Konar, Rakesh Radhakrishnan                        errors in commercial and open source systems. In SOSP (2011),
and Raghu Shastry for their support, helpful discussions,               pp. 159–172.



                                                              13
