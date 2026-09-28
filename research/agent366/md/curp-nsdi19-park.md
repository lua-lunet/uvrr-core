   Exploiting Commutativity For
     Practical Fast Replication
   Seo Jin Park and John Ousterhout, Stanford University
    https://www.usenix.org/conference/nsdi19/presentation/park




This paper is included in the Proceedings of the
16th USENIX Symposium on Networked Systems
     Design and Implementation (NSDI ’19).
          February 26–28, 2019 • Boston, MA, USA
                     ISBN 978-1-931971-49-2



                                  Open access to the Proceedings of the
                             16th USENIX Symposium on Networked Systems
                                  Design and Implementation (NSDI ’19)
                                            is sponsored by
                     Exploiting Commutativity For Practical Fast Replication
                                       Seo Jin Park                    John Ousterhout
                                    Stanford University               Stanford University


                          Abstract                                     Consistent Unordered Replication Protocol (CURP) re-
Traditional approaches to replication require client requests       duces the overhead for replication by taking advantage of the
to be ordered before making them durable by copying them to         fact that most operations are commutative, so their order of ex-
replicas. As a result, clients must wait for two round-trip times   ecution doesn’t matter. CURP supplements a system’s exist-
(RTTs) before updates complete. In this paper, we show that         ing replication mechanism with a lightweight form of replica-
this entanglement of ordering and durability is unnecessary         tion without ordering based on witnesses. A client replicates
for strong consistency. The Consistent Unordered Replica-           each operation to one or more witnesses in parallel with send-
tion Protocol (CURP) allows clients to replicate requests that      ing the request to the primary server; the primary can then ex-
have not yet been ordered, as long as they are commutative.         ecute the operation and return to the client without waiting for
This strategy allows most operations to complete in 1 RTT           normal replication, which happens asynchronously. This al-
(the same as an unreplicated system). We implemented                lows operations to complete in 1 RTT, as long as all witnessed-
CURP in the Redis and RAMCloud storage systems. In                  but-not-yet-replicated operations are commutative. Non-
RAMCloud, CURP improved write latency by ∼2x (14 µs                 commutative operations still require 2 RTTs. If the primary
→ 7.1 µs) and write throughput by 4x. Compared to un-               crashes, information from witnesses is combined with that
replicated RAMCloud, CURP’s latency overhead for 3-way              from the normal replicas to re-create a consistent server state.
replication is just 1 µs (6.1 µs vs 7.1 µs). CURP transformed          CURP can be easily applied to most existing systems
a non-durable Redis cache into a consistent and durable             using primary-backup replication. Changes required by
storage system with only a small performance overhead.              CURP are not intrusive, and it works with any kind of backup
                                                                    mechanism (e.g. state machine replication [31], file writes to
1   Introduction                                                    network replicated drives [1], or scattered replication [26]).
   Fault-tolerant systems rely on replication to mask individ-      This is important since most high-performance systems
ual failures. To ensure that an operation is durable, it cannot     optimize their backup mechanisms, and we don’t want to lose
be considered complete until it has been properly replicated.       those optimizations (e.g. CURP can be used with RAMCloud
Replication introduces a significant overhead because it            without sacrificing its fast crash recovery [26]).
requires round-trip communication to one or more additional            To show its performance benefits and applicability, we
servers. Within a datacenter, replication can easily double         implemented CURP in two NoSQL storage systems: Re-
the latency for operations in comparison to an unreplicated         dis [30] and RAMCloud [27]. Redis is generally used as
system; in geo-replicated environments the cost of replication      a non-durable cache due to its very expensive durability
can be even greater.                                                mechanism. By applying CURP to Redis, we were able to
   In principle, the cost of replication could be reduced or        provide durability and consistency with similar performance
eliminated if replication could be overlapped with the execu-       to the non-durable Redis. For RAMCloud, CURP reduced
tion of the operation. In practice, however, this is difficult to   write latency by half (only a 1 µs penalty relative to RAM-
do. Executing an operation typically establishes an ordering        Cloud without replication) and increased throughput by 3.8x
between that operation and other concurrent operations, and         without compromising consistency.
the order must survive crashes if the system is to provide             Overall, CURP is the first replication protocol that com-
consistent behavior. If replication happens in parallel with        pletes linearizable deterministic update operations within
execution, different replicas may record different orders for       1 RTT without special networking. Instead of relying
the operations, which can result in inconsistent behavior           on special network devices or properties for fast replica-
after crashes. As a result, most systems perform ordering           tion [21, 28, 22, 12, 3], CURP exploits commutativity, and it
before replication: a client first sends an operation to a server   can be used for any system where commutativity of client re-
that orders the operation (and usually executes it as well);        quests can be checked just from operation parameters (CURP
then that server issues replication requests to other servers,      cannot use state-dependent commutativity). Even when
ensuring a consistent ordering among replicas. As a result,         compared to Speculative Paxos or NOPaxos (which require
the minimum latency for an operation is two round-trip              a special network topology and special network switches),
times (RTTs). This problem affects all systems that provide         CURP is faster since client request packets do not need to de-
consistency and replication, including both primary-backup          tour to get ordered by a networking device (NOPaxos has an
approaches and consensus approaches.                                overhead of 16 µs, but CURP only increased latency by 1 µs).



USENIX Association                             16th USENIX Symposium on Networked Systems Design and Implementation              47
2    Separating Durability from Ordering
   Replication protocols supporting concurrent clients have
combined the job of ordering client requests consistently
among replicas and the job of ensuring the durability of
operations. This entanglement causes update operations to
take 2 RTTs.
   Replication protocols must typically guarantee the
following two properties:
  • Consistent Ordering: if a replica completes operation a
     before b, no client in the system should see the effects of      Figure 1: CURP clients directly replicate to witnesses. Witnesses
     b without the effects of a.                                      only guarantee durability without ordering. Backups hold data that
  • Durability: once its completion has been externalized             includes ordering information. Witnesses are temporary storage to ensure
     to an application, an executed operation must survive            durability until operations are replicated to backups.
     crashes.                                                        operations in 2 RTTs, CURP achieves durability without
   To achieve both consistent ordering and durability, current       ordering and uses the commutativity of operations to defer
replication protocols need 2 RTTs. For example, in master-           agreement on operation order.
backup (a.k.a. primary-backup) replication, client requests             To achieve durability in 1 RTT, CURP clients directly
are always routed to a master replica, which serializes              record their requests in temporary storage, called a witness,
requests from different clients. As part of executing an             without serializing them through masters. As shown in Fig-
operation, the master replicates either the client request itself    ure 1, witnesses do not carry ordering information, so clients
or the result of the execution to backup replicas; then the          can directly record operations into witnesses in parallel with
master responds back to clients. This entire process takes 2         sending operations to masters so that all requests will finish in
RTTs total: 1 from clients to masters and another RTT for            1 RTT. In addition to the unordered replication to witnesses,
masters to replicate data to backups in parallel.                    masters still replicate ordered data to backups, but do so
   Consensus protocols with strong leaders (e.g. Multi-              asynchronously after sending the execution results back to the
Paxos [17] or Raft [25]) also require 2 RTTs for update              clients. Since clients directly make their operations durable
operations. Clients route their requests to the current leader       through witnesses, masters can reply to clients as soon as
replica, which serializes the requests into its operation log.       they execute the operations without waiting for permanent
To ensure durability and consistent ordering of the client           replication to backups. If a master crashes, the client requests
requests, the leader replicates its operation log to a majority      recorded in witnesses are replayed to recover any operations
of replicas, and then it executes the operation and replies          that were not replicated to backups. A client can then
back to clients with the results. In consequence, consensus          complete an update operation and reveal the result returned
protocols with strong leaders also require 2 RTTs for updates:       from the master if it successfully recorded the request in
1 RTT from clients to leaders and another RTT for leaders to         witnesses (optimistic fast path: 1 RTT), or after waiting for
replicate the operation log to other replicas.                       the master to replicate to backups (slow path: 2 RTT).
   Fast Paxos [19] and Generalized Paxos [18] reduced the               CURP’s approach introduces two threats to consistency:
latency of replicated updates from 2 RTTs to 1.5 RTT by              ordering and duplication. The first problem is that the order
allowing clients to optimistically replicate requests with           in which requests are replayed after a server crash may not
presumed ordering. Although their leaders don’t serialize            match the order in which the master processed those requests.
client requests by themselves, leaders must still wait for a         CURP uses commutativity to solve this problem: all of the
majority of replicas to durably agree on the ordering of the         unsynced requests (those that a client considers complete,
requests before executing them. This extra waiting adds 0.5          but which have not been replicated to backups) must be com-
RTT overhead. (See §B.3 for a detailed explanation on why            mutative. Given this restriction, the order of replay will have
they cannot achieve 1 RTT.)                                          no visible impact on system behavior. Specifically, a witness
   Network-Ordered Paxos [21] and Speculative Paxos [28]             only accepts and saves an operation if it is commutative with
achieve near 1 RTT latency for updates by using special net-         every other operation currently stored by that witness (e.g.,
working to ensure that all replicas receive requests in the same     writes to different objects). In addition, a master will only
order. However, since they require special networking hard-          execute client operations speculatively (by responding before
ware, it is difficult to deploy them in practice. Also, they can’t   replication is complete), if that operation is commutative with
achieve the minimum possible latency since client requests           every other unsynced operation. If either a witness or master
detour to a common root-layer switch (or a middlebox).               finds that a new operation is not commutative, the client must
   The key idea of CURP is to separate durability and                ask the master to sync with backups. This adds an extra RTT
consistent ordering, so update operations can be done in 1           of latency, but it flushes all of the speculative operations.
RTT in the normal case. Instead of replicating totally ordered          The second problem introduced by CURP is duplication.



48   16th USENIX Symposium on Networked Systems Design and Implementation                                            USENIX Association
When a master crashes, it may have completed the replication
of one or more operations that are recorded by witnesses. Any
completed operations will be re-executed during replay from
witnesses. Thus there must be a mechanism to detect and
filter out these re-executions. The problem of re-executions is
not unique to CURP, and it can happen in distributed systems
for a variety of other reasons. There exist mechanisms to
filter out duplicate executions, such as RIFL [20], and they
can be applied to CURP as well.
    We can apply the idea of separating ordering and durability              Figure 2: CURP architecture for f = 3 fault tolerance.
to both consensus-based replicated state machines (RSM) and
                                                                    fying their specialized backup mechanisms. For example,
primary-backup, but this paper focuses on primary-backup
                                                                    CURP can be applied to a system which uses file writes to
since it is more critical for application performance. Fault-
                                                                    network replicated drives as a backup mechanism, where
tolerant large-scale high-performance systems are mostly
                                                                    the use of witnesses will improve latency while retaining its
configured with a single cluster coordinator replicated by
                                                                    special backup mechanism. However, when designing new
consensus and many data servers using primary-backup (e.g.
                                                                    systems, witnesses may be combined with backups for extra
Chubby [6], ZooKeeper [15], Raft [25] are used for cluster
                                                                    performance benefits. (See §B.1 for details.)
coordinators in GFS [13], HDFS [32], and RAMCloud [27]).
                                                                       CURP makes no assumptions about the network. It
The cluster coordinators are used to prevent split-brains for
                                                                    operates correctly even with networks that are asynchronous
data servers, and operations to the cluster coordinators (e.g.
                                                                    (no bound on message delay) and unreliable (messages
change of master node during recovery) are infrequent and
                                                                    can be dropped). Thus, it can achieve 1 RTT updates on
less latency sensitive. On the other hand, operations to data
                                                                    replicated systems in any environment, unlike other alter-
servers (e.g. insert, replace, etc) directly impact application
                                                                    native solutions. (For example, Speculative Paxos [28] and
performance, so the rest of this paper will focus on the CURP
                                                                    Network-Ordered Paxos [21] require special networking
protocol for primary-backup, which is the main replication
                                                                    hardware and cannot be used for geo-replication.)
technique for data servers. In §B.2, we sketch how the same
technique can be applied for consensus.                             3.2 Normal Operation
                                                                    3.2.1 Client
3   CURP Protocol                                                      Client interaction with masters is generally the same as it
   CURP is a new replication protocol that allows clients           would be without CURP. Clients send update RPC requests
to complete linearizable updates within 1 RTT. Masters in           to masters. If a client cannot receive a response, it retries the
CURP speculatively execute and respond to clients before            update RPC. If the master crashes, the client may retry the
the replication to backups has completed. To ensure the             RPC with a different server.
durability of the speculatively completed updates, clients             For 1 RTT updates, masters return to clients before replica-
multicast update operations to witnesses. To preserve               tion to backups. To ensure durability, clients directly record
linearizability, witnesses and masters enforce commutativity        their requests to witnesses concurrently while waiting for
among operations that are not fully replicated to backups.          responses from masters. Once all f witnesses have accepted
3.1 Architecture and Model                                          the requests, clients are assured that the requests will survive
   CURP provides the same guarantee as current primary-             master crashes, so clients complete the operations with the
backup protocols: it provides linearizability to client requests    results returned from masters.
in spite of failures. CURP assumes a fail-stop model and does          If a client cannot record in all f witnesses (due to failures or
not handle byzantine faults. As in typical primary-backup           rejections by witnesses), the client cannot complete an update
replications, it uses a total of f + 1 replicas composed of 1       operation in 1 RTT. To ensure the durability of the operation,
master and f backups, where f is the number of replicas that        the client must wait for replication to backups by sending
can fail without loss of availability. In addition to that, it      a sync RPC to the master. Upon receiving sync RPCs, the
uses f witnesses to ensure durability of updates even before        master ensures the operation is replicated to backups before
replications to backups are completed. As shown in Figure 2,        returning to the client. This waiting for sync increases the
witnesses may fail independently and may be co-hosted               operation latency to 2 RTTs in most cases and up to 3 RTT in
with backups. CURP remains available (i.e. immediately              the worst case where the master hasn’t started syncing until it
recoverable) despite up to f failures, but will still be strongly   receives a sync RPC from a client. If there is no response to
consistent even if all replicas fail.                               the sync RPC (indicating the master might have crashed), the
   Throughout the paper, we assume that witnesses are               client restarts the entire process; it resends the update RPC to
separate from backups. This allows CURP to be applied to            a new master and tries to record the RPC request in witnesses
a wide range of existing replicated systems without modi-           of the new master.



USENIX Association                             16th USENIX Symposium on Networked Systems Design and Implementation                   49
3.2.2 Witness
   Witnesses support 3 basic operations: they record opera-
tions in response to client requests, hold the operations until
explicitly told to drop by masters, and provide the saved             Figure 3: Sequence of executed operations in the crashed master.
operations during recovery.                                          Unlike traditional primary-backup replication, masters
   Once a witness accepts a record RPC for an operation, it       in CURP generally respond back to clients before syncing
guarantees the durability of the operation until told that the    to backups, so that clients can receive the results of update
operation is safe to drop. To be safe from power failures,        RPCs within 1 RTT. We call this speculative execution since
witnesses store their data in non-volatile memory (such as        the execution may be lost if masters crash. Also, we call
flash-backed DRAM). This is feasible since a witness needs        the operations that were speculatively executed but not yet
only a small amount of space to temporarily hold recent client    replicated to backups unsynced operations. As shown in
requests. Similar techniques are used in strongly-consistent      Figure 3, all unsynced operations are contiguous at the tail of
low-latency storage systems, such as RAMCloud [27].               the masters’ execution history.
   A witness accepts a new record RPC from a client only             To prevent inconsistency, a master must sync before
if the new operation is commutative with all operations that      responding if the operation is not commutative with any
are currently saved in the witness. If the new request doesn’t    existing unsynced operations. If a master responds for a non-
commute with one of the existing requests, the witness must       commutative operation before syncing, the result returned to
reject the record RPC since the witness has no way to order       the client may become inconsistent if the master crashes. This
the two noncommutative operations consistent with the             is because the later operation might complete and its result
execution order in masters. For example, if a witness already     could be externalized (because it was recorded to witnesses)
accepted “x ← 1”, it cannot accept “x ← 5”.                       while the earlier operation might not survive the crash
   Witnesses must be able to determine whether operations are     (because, for example, its client crashed before recording it
commutative or not just from the operation parameters. For        to witnesses). For example, if a master speculatively executes
example, in key-value stores, witnesses can exploit the fact      “x ← 2” and “read x”, the returned read value, 2, will not be
that operations on different keys are commutative. In some        valid if the master crashes and loses “x ← 2”. To prevent such
cases, it is difficult to determine whether two operations com-   unsafe dependencies, masters enforce commutativity among
mute each other. SQL UPDATE is an example; it is impos-           unsynced operations; this ensures that all results returned to
sible to determine the commutativity of “UPDATE T SET             clients will be valid as long as they are recorded in witnesses.
rate = 40 WHERE level = 3” and “UPDATE T SET                         If an operation is synced because of a conflict, the master
rate = rate + 10 WHERE dept = SDE” just from                      tags its result as “synced” in the response; so, even if the
the requests themselves. To determine the commutativity of        witnesses rejected the operation, the client doesn’t need to
the two updates, we must run them with real data. Thus, wit-      send a sync RPC and can complete the operation in 2 RTTs.
nesses cannot be used for operations whose commutativity          3.3 Recovery
depends on the system state. In addition to the case explained,      CURP recovers from a master’s crash in two phases: (1)
determining commutativity can be more subtle for complex          restoration from backups and (2) replay from witnesses.
systems, such as DBMS with triggers and views.                    First, the new master restores data from one of the backups,
   Each of f witnesses operates independently; witnesses          using the same mechanism it would have used in the absence
need not agree on either ordering or durability of operations.    of CURP.
In an asynchronous network, record RPCs may arrive at                Once all data from backups have been restored, the new
witnesses in different order, which can cause witnesses to        master replays the requests recorded in witnesses. The new
accept and reject different sets of operations. However, this     master picks any available witness. If none of the f witnesses
does not endanger consistency. First, as mentioned in §3.2.1,     are reachable, the new master must wait. After picking
a client can proceed without waiting for sync to backups          the witness to recover from, the new master first asks it to
only if all f witnesses accepted its record RPCs. Second,         stop accepting more operations; this prevents clients from
requests in each witness are required to be commutative           erroneously completing update operations after recording
independently, and only one witness is selected and used          them in a stale witness whose requests will not be retried
during recovery (described in §3.3).                              anymore. After making the selected witness immutable, the
3.2.3 Master                                                      new master retrieves the requests recorded in the witness.
   The role of masters in CURP is similar to their role in        Since all requests in a single witness are guaranteed to be
traditional primary-backup replications. Masters in CURP          commutative, the new master can execute them in any order.
receive, serialize, and execute all update RPC requests from      After replaying all requests recorded in the selected witness,
clients. If an executed operation updates the system state, the   the new master finalizes the recovery by syncing to backups
master synchronizes (syncs) its current state with backups by     and resetting witnesses for the new master (or assigning a new
replicating the updated value or the log of ordered operations.   set of witnesses). Then the new master can start accepting



50   16th USENIX Symposium on Networked Systems Design and Implementation                                        USENIX Association
client requests again.                                             This endangers consistency since requests recorded in the old
   Some of the requests in the selected witness may have been      witnesses will not be replayed during recovery.
executed and replicated to backups before the master crashed,         To prevent clients from completing an unsynced update op-
so the replay of such requests will result in re-execution of      eration with just recording to old witnesses, CURP maintains
already executed operations. Duplicate executions of the           a monotonically increasing integer, WitnessListVersion, for
requests can violate linearizability [20].                         each master. A master’s WitnessListVersion is incremented
   To avoid duplicate executions of the requests that are          every time the witness configuration for the master is updated,
already replicated to backups, CURP relies on exactly-once         and the master is notified of the new version along with the
semantics provided by RIFL [20], which detects already             new witness list. Clients obtain the WitnessListVersion when
executed client requests and avoids their re-execution. Such       they fetch the witness list from the configuration manager. On
mechanisms for exactly-once semantics are already neces-           all update requests, clients include the WitnessListVersion,
sary to achieve linearizability for distributed systems [20],      so that masters can detect and return errors if the clients used
so CURP does not introduce a new requirement. In RIFL,             wrong witnesses; if they receive errors, the clients fetch new
clients assign a unique ID to each RPC; servers save the IDs       witness lists and retry the updates. This ensures that clients’
and results of completed requests and use them to detect and       update operations can never complete without syncing to
answer duplicate requests. The IDs and results are durably         backups or recording to current witnesses.
preserved with updated objects in an atomic fashion. (If a            Third, for load balancing, a master can split its data into
system replicates client requests to backups instead of just       two partitions and migrate a partition to a different master.
updated values, providing atomic durability becomes trivial        Migrations usually happen in two steps: a prepare step
since each request already contains its ID and its result can be   of copying data while servicing requests and a final step
obtained from its replay during recovery.)                         which stops servicing (to ensure that all recent operations are
   This recovery protocol together with the normal operation       copied) and changes configuration. To simplify the protocol
protocol described in §3.2 guarantee linearizability of client     changes from the base primary-backup protocol, CURP
operations even with server failures. An informal proof of         masters sync to backups and reset witnesses before the final
correctness can be found in appendix §A.                           step of migration, so witnesses are completely ruled out of
3.4 Garbage Collection                                             migration protocols. After the migration is completed, some
   To limit memory usage in witnesses and reduce possible          clients may send updates on the migrated partition to the old
rejections due to commutativity violations, witnesses must         master and old witnesses; the old master will reject and tell
discard requests as soon as possible. Witnesses can drop the       the client to fetch the new master information (this is the same
recorded client requests after masters make their outcomes         as without CURP); then the client will fetch the new master
durable in backups. In CURP, masters send garbage collec-          and its witness information and retry the update. Meanwhile,
tion RPCs for the synced updates to their witnesses. The           the requests on the migrated partition can be accidentally
garbage collection RPCs are batched: each RPC lists several        recorded in the old witness, but this does not cause safety
operations that are now durable (using RPC IDs provided by         issues; masters will ignore such requests during the replay
RIFL [20]).                                                        phase of recovery by the filtering mechanism used to reject
3.5 Reconfigurations                                               requests on not owned partitions during normal operations.
   This section discusses three cases of reconfiguration:          3.6 Read Operations
recovery of a crashed backup, recovery of a crashed witness,          CURP handles read operations in a fashion similar to that
and data migration for load balancing. First, CURP doesn’t         of primary-backup replication. Since such operations don’t
change the way to handle backup failures, so a system can          modify system state, clients can directly read from masters,
just recover a failed backup as it would without CURP.             and neither clients nor masters replicate read-only operations
   Second, if a witness crashes or becomes non-responsive,         to witnesses or backups.
the system configuration manager (the owner of all cluster            However, even for read operations, a master must check
configurations) decommissions the crashed witness and              whether a read operation commutes with all currently
assigns a new witness for the master; then it notifies the         unsynced operations as discussed in §3.2.3. If the read
master of the new witness list. When the master receives the       operation conflicts with some unsynced update operations,
notification, it syncs to backups to ensure f -fault tolerance     the master must sync the unsynced updates to backups before
and responds back to the configuration manager that it is now      responding for the read.
safe to recover from the new witness. After this point, clients    3.7 Consistent Reads from Backups
can use f witnesses again to record operations. However,              In primary-backup replication, clients normally issue
CURP does not push the new list of witnesses to clients. Since     all read operations to the master. However, some systems
clients cache the list of witnesses, clients may still use the     allow reading from backups because it reduces the load on
decommissioned witness (if it was temporarily disconnected,        masters and can provide better latency in a geo-replicated
the witness will continue to accept record RPCs from clients).     environment (clients can read from a backup in the same



USENIX Association                            16th USENIX Symposium on Networked Systems Design and Implementation              51
                                                                                   after another read returned the new value. The first issue
                                                                                   is prevented by checking a witness before reading from a
                                                                                   backup. Since clients can complete an update operation only
                                                                                   if it is synced to all backups or recorded in all witnesses, a
                                                                                   reader will either see a noncommutative update request in the
                                                                                   witness being checked or find the new value from the backup;
                                                                                   thus, it is impossible for a read after an update to return the
                                                                                   old value. For the second issue, since both a master and
                                                                                   backups delay reads of a new value until it is fully replicated
                                                                                   to all backups, it is impossible to read an older value after
                                                                                   another client reads the new value.

 Figure 4: Three cases of reading the value of x from a backup replica
 while another client is changing the value of x from 0 to 1: (a) client R first   4   Implementation on NoSQL Storage
 confirms that a nearby witness has no request that is not commutative with            This section describes how to implement CURP on low-
 “read x,” so the client directly reads the value of x from a nearby backup.
 (b) Just after client W completes “x ← 1”, client R starts another read.
                                                                                   latency NoSQL storage systems that use primary-backup
 Client R finds that there is a non-commutative request saved in a nearby          replications. With the emergence of large-scale Web ser-
 witness, so it must read from a remote master to guarantee consistency.           vices, NoSQL storage systems became very popular (e.g.
 (c) After syncing “x ← 1” to the backup, the master garbage collected             Redis [30], RAMCloud [27], DynamoDB [33] and Mon-
 the update request from witnesses and acknowledged the full sync to
 backups. Now, client R sees no non-commutative requests in the witness            goDB [7]), and they range from simple key-value stores to
 and can complete read operation by reading from the nearby backup.                more fully featured stores supporting secondary indexing and
region to avoid wide-area RTTs). However, naively reading                          multi-object transactions; so, improving their performance
from backups can violate linearizability since updates in                          using CURP is an important problem with a broad impact.
CURP can complete before syncing to backups.                                           The most important piece missing from §3 to implement
    To avoid reading stale values, clients in CURP use a nearby                    CURP is how to efficiently detect commutativity violations.
witness (possibly colocated with a backup) to check whether                        Fortunately for NoSQL systems, CURP can use primary
the value read from a nearby backup is up to date. To perform                      keys to efficiently check the commutativity of operations.
a consistent read, a client must first ask a witness whether the                   NoSQL systems store data as a collection of objects, which
read operation commutes with the operations currently saved                        are identified by primary keys. Most update operations in
in the witness (as shown in Figure 4). If it commutes, the client                  NoSQL specify the affected object with its primary key (or a
is assured that the value read from a backup will be up to date.                   list of primary keys), and update operations are commutative
If it doesn’t commute (i.e. the witness retains a write request                    if they modify disjoint sets of objects. The rest of this section
on the key being read), the value read from a backup might be                      describes an implementation of CURP that exploits this
stale. In this case, the client must read from the master.                         efficient commutativity check.
    In addition, we assume that the underlying primary-backup                      4.1 Life of A Witness
replication mechanism prevents backups from returning new                              Witnesses have two modes of operation: normal and
values that are not yet fully synced to all backups. Such mech-                    recovery. In each mode, witnesses service a subset of
anism is neccessary even before applying CURP since return-                        operations listed in Figure 5. When it receives a start RPC,
ing a new value prematurely can cause inconsistency; even if                       a witness starts its life for a master in normal mode, in
a value is replicated to some of backups, the value may get lost                   which the witness is allowed to mutate its collection of saved
if the master crashes and a new master recovers from a backup                      requests. In normal mode, the witness services record RPCs
that didn’t receive the new value. A simple solution for this                      for client requests targeted to the master for which the witness
problem is that backups don’t allow reading values that are not                    was configured by start; by accepting only requests for the
yet fully replicated to all backups. For backups to track which                    correct master, CURP prevents clients from recording to
values are fully replicated and ok to be read, a master can pig-                   incorrect witnesses. Also, witnesses drop their saved client
gyback the acknowlegements for successful previous syncs                           requests as they receive gc RPCs from masters.
when it sends sync requests to backups. When a client tries                            A witness irreversibly switches to a recovery mode once
to read a value that is not known to be yet fully replicated, the                  it receives a getRecoveryData RPC. In recovery mode,
backup can wait for full replication or ask the client to retry.                   mutations on the saved requests are prohibited; witnesses
    Thanks to the safety mechanisms discussed above, CURP                          reject all record RPCs and only service getRecoveryData
still guarantees linearizability. With a concurrent update,                        or end. As a recovery is completed and the witness becomes
reading from backups could violate linearizability in two                          useless, the cluster coordinator may send end to free up the
ways: (1) a read sees the old value after the completion                           resources, so that the witness server can start another life for
of the update operation and (2) a read sees the old value                          a different master.



52    16th USENIX Symposium on Networked Systems Design and Implementation                                                   USENIX Association
 C LIENT TO W ITNESS:                                                       4.4 Improving Throughput of Masters
 record(masterID, list of keyHash, rpcId, request) → {ACCEPTED or
                                                                               Masters in primary-backup replication are usually the bot-
 REJECTED}
    Saves the client request (with rpcId) of an update on keyHashes.        tlenecks of systems since they drive replication to backups.
    Returns whether the witness could accomodate and save the request.      Since masters in CURP can respond to clients before syncing
 M ASTER TO W ITNESS:                                                       to backups, they can delay syncs until the next batch without
 gc(list of {keyHash, rpcId}) → list of request                             impacting latency. This batching of syncs improves masters’
    Drops the saved requests with the given keyHashes and rpcIds. Returns
    stale requests that haven’t been garbage collected for a long time.
                                                                            throughput in two ways.
 getRecoveryData() → list of request                                           First, by batching replication RPCs, CURP reduces the
    Returns all requests saved for a particular crashed master.             number of RPCs a master must handle per client request.
 C LUSTER C OORDINATOR TO W ITNESS:                                         With 3-way primary-backup replication, a master must
 start(masterId) → {SUCCESS or FAIL}
                                                                            process 4 RPCs per client request (1 update RPC and 3
    Start a witness instance for the given master, and return SUCCESS. If
    the server fails to create the instance, FAIL is returned.              replication RPCs). If the master batches replication and syncs
 end() → NULL                                                               every 10 client requests, it handles 1.3 RPCs on average. On
    This witness is decommissioned. Destruct itself.                        NoSQL storage systems, sending and receiving RPCs takes a
                    Figure 5: The APIs of Witnesses.                        significant portion of the total processing time since NoSQL
4.2 Data Structure of Witnesses                                             operations are not compute-heavy.
   Witnesses are designed to minimize the CPU cycles spent                     Second, CURP eliminates wasted resources and other inef-
for handling record RPCs. For client requests mutating a                    ficiencies that arise when masters wait for syncs. For example,
single object, recording to a witness is similar to inserting               in the RAMCloud [27] storage system, request handlers use
in a set-associative cache; a record operation finds a set of               a polling loop to wait for completion of backup syncs. The
slots using a hash of the object’s primary key and writes                   syncs complete too quickly to context-switch to a different
the given request to an available slot in the set. To enforce               activity, but the polling still wastes more than half of the CPU
commutativity, the witness searches the occupied slots in                   cycles of the polling thread. With CURP, a master can com-
the set and rejects if there is another request with the same               plete a request without waiting for syncing and move on to the
primary key (for performance, we compare 64-bit hashes of                   next request immediately, which results in higher throughput.
primary keys instead of full keys). If there is no slot available              The batch size of syncs is limited in CURP to reduce
in the set for the key, the record operation is rejected as well.           witness rejections. Delaying syncs increases the chance of
   For client requests mutating multiple objects, witnesses                 finding non-commutative operations in witnesses and mas-
perform the commutativity and space check for every affected                ters, causing extra rejections in witnesses and more blocking
object; to accept an update affecting n objects, a witness must             syncs in masters. A simple way to limit the batching would be
ensure that (1) no existing client request mutates any of the               for masters to issue a sync immediately after responding to a
n objects and (2) there is an available slot in each set for all n          client if there is no outstanding sync; this strategy gives a rea-
objects. If the update is commutative and space is available,               sonable throughput improvement since at most one CPU core
the witness writes the update request n times as if recording               will be used for syncing, and it also reduces witness rejections
n different requests on each object.                                        by syncing aggresively. However, to find the optimal batch
4.3 Commutativity Checks in Masters                                         size, an experiment with a system and real workload is neces-
   Every NoSQL update operation changes the values of one                   sary since each workload has a different sensitivity to larger
or more objects. To enforce commutativity, a master can                     batch sizes. For example, workloads which randomly access
check if the objects touched (either updated or just read) by               large numbers of keys uniformly can use a very large batch
an operation are unsynced at the time of its execution. If an               size without increasing the chance of commutativity conflicts.
operation touches any unsynced value, it is not commutative                 4.5 Garbage Collection
and the master must sync all unsynced operations to backups                    As discussed in §3.4, masters send garbage collection RPCs
before responding back to the client.                                       for synced updates to their witnesses. Right after syncing to
   If the object values are stored in a log, masters can                    backups, masters send gc RPCs (in Figure 5), so the witnesses
determine if an object value is synced or not by comparing its              can discard data for the operations that were just synced.
position in the log against the last synced position.                          To identify client requests for removal, CURP uses 64-bit
   If the object values are not stored in a log, masters can use            key hashes and RPC IDs assigned by RIFL [20]. Upon
monotonically increasing timestamps. Whenever a master                      receiving a gc RPC, a witness locates the sets of slots using
updates the value of an object, it tags the new value with a                the keyHashes and resets the slots whose occupying requests
current timestamp. Also, the master keeps the timestamp of                  have the matching RPC IDs. Witnesses ignore keyHashes
when last backup sync started. By comparing the timestamp                   and rpcIds that are not found since the record RPCs might
of an object against the timestamp of the last backup sync,                 have been rejected. For client requests that mutate multiple
a master can tell whether the value of the object has been                  objects, gc RPCs include multiple hkeyHash, rpcIdsi pairs
synced to backups.                                                          for all affected objects, so that witnesses can clear all slots



USENIX Association                                     16th USENIX Symposium on Networked Systems Design and Implementation                53
occupied by the request.                                                             RAMCloud cluster           Redis cluster
                                                                        CPU      Xeon X3470 (4x2.93 GHz) Xeon D-1548 (8x2.0 GHz)
   Although the described garbage collection can clean up               RAM       24 GB DDR3 at 800 MHz         64 GB DDR4
most records, some slots may be left uncollected: if a client           Flash    2x Samsung 850 PRO SSDs     Toshiba NVMe flash
crashes before sending the update request to the master,                NIC
                                                                                    Mellanox ConnectX-2     Mellanox ConnectX-3
or if the record RPC is delayed significantly and arrives                        InfiniBand HCA (PCIe 2.0) 10 Gbps NIC (PCIe 3.0)
                                                                        Switch   Mellanox SX6036 (2 level)       HPE 45XGc
after the master finished garbage collection for the update.
                                                                        OS         Linux 3.16.0-4-amd64    Linux 3.13.0-100-generic
Uncollected garbage will cause witnesses to indefinitely
                                                                           Table 1: The server hardware configuration for benchmarks.
reject requests with the same keys.
   Witnesses detect such uncollected records and ask masters        safety issues with respect to zombies.
to retry garbage collection for them. When it rejects a record,     4.8 Modifications to RIFL
a witness recognizes the existing record as uncollected                In order to work with CURP, the garbage collection
garbage if there have been many garbage collections since           mechanism of RIFL described in [20] must be modified. See
the record was written (three is a good number if a master          §C.1 for details.
performs only one gc RPC at a time). Witnesses notify               5    Evaluation
masters of the requests that are suspected as uncollected              We evaluated CURP by implementing it in the RAMCloud
garbage through the response messages of gc RPCs; then the          and Redis storage systems, which have very different backup
masters retry the requests (most likely filtered by RIFL), sync     mechanisms. First, using the RAMCloud implementation, we
to backups, and thus include them in the next gc requests.          show that CURP improves the performance of consistently
4.6 Recovery Steps                                                  replicated systems. Second, with the Redis implementation,
   To recover a crashed master, CURP first restores data            we demonstrate that CURP can make strong consistency
from backups and then replays requests from a witness.              affordable in a system where it had previously been too
To fetch the requests to replay, the new master sends a             expensive for practical use.
getRecoveryData RPC (in Figure 5), which has two effects:           5.1 RAMCloud Performance Improvements
(1) it irreversibly sets the witness into recovery mode, so that       RAMCloud [27] is a large-scale low latency distributed
the data in the witness will never change, (2) it provides the      key-value store, which primarily focuses on reducing latency.
entire list of client requests saved in the witness.                Small read operations take 5 µs, and small writes take
   With the provided requests, the new master replays all of        14 µs. By default, RAMCloud replicates each new write to 3
them. Since operations already recovered from backups will          backups, which asynchronously flush data into local drives.
be filtered out by RIFL [20], the replay step finishes very         Although replicated data are stored in slow disk (for cost sav-
quickly. In total, CURP increases recovery time by the exe-         ing), RAMCloud features a technique to allow fast recovery
cution time for a few requests plus 2 RTT (1 RTT for getRe-         from a master crash (it recovers within a few seconds) [26].
coveryData and another RTT for backup sync after replay).              With the RAMCloud implementation of CURP, we
4.7 Zombies                                                         answered the following questions:
   For a fault-tolerant system to be consistent, it must neutral-     • How does CURP improve RAMCloud’s latency and
ize zombies. A zombie is a server that has been determined               throughput?
to have crashed, so some other server has taken over its              • How many resources do witness servers consume?
functions, but the server has not actually crashed (e.g., it may      • Will CURP be performant under highly-skewed work-
have suffered temporary network connectivity problems).                  loads with hot keys?
Clients may continue to communicate with zombies; reads or             Our evaluations using the RAMCloud implementation
updates accepted by a zombie may be inconsistent with the           were conducted on a cluster of machines with the specifica-
state of the replacement server.                                    tions shown in Table 1. All measurements used InfiniBand
   CURP assumes that the underlying system already has              networking and RAMCloud’s fastest transport, which by-
mechanisms to neutralize zombies (e.g., by asking backups           passes the kernel and communicates directly with InfiniBand
to reject replication requests from a crashed master [27]).         NICs. Our CURP implementation kept RAMCloud’s fast
The witness mechanism provides additional safeguards.               crash recovery [26], which recovers from master crashes
If a zombie responds to a client request without waiting            within a few seconds using data stored on backup disks.
for replication, then the client must communicate with all          Servers were configured to replicate data to 1–3 different
witnesses before completing the request. If it succeeds before      backups (and 1–3 witnesses for CURP results), indicated as
the witness data has been replayed during recovery, then            a replication factor f . The log cleaner of RAMCloud did not
the update will be reflected in the new master. If the client       run in any measurements; in a production system, the log
contacts a witness after its data has been replayed, the witness    cleaner can reduce the throughput.
will reject the request; the client will then discover that the        For RAMCloud, CURP moved backup syncs out of
old master has crashed and reissue its request to the new           the critical path of write operations. This decoupling not
master. Thus, the witness mechanism does not create new             only improved latency but also improved the throughput of



54   16th USENIX Symposium on Networked Systems Design and Implementation                                          USENIX Association
                                               0                                                                                                                  YCSB-A (50% read, 50% write)                                                 YCSB-B (95% read, 5% write)
                                        1x10                                                                                                         1000
                                                                                               Original (f = 3)                                                                                                                   1000




                                                                                                                              Throughput (k ops/s)                                                         Throughput (k ops/s)
                                                                                                CURP (f = 3)
                                        1x10-1
                                                                                                                                                     800
                                                                                                CURP (f = 2)                                                                                                                      800
                                                                                                CURP (f = 1)                                         600




 Fraction of Writes
                                                                                                                                                                                                                                  600
                                        1x10-2                                                  Unreplicated
                                                                                                                                                     400                                                                          400              Unreplicated
                                                                                                                                                                                                                                                   CURP (f=1)
                                        1x10-3                                                                                                       200             Unreplicated         CURP (f=3)                              200              CURP (f=3)
                                                                                                                                                                     CURP (f=1)           Original                                                 Original (f=3)
                                                                                                                                                       0                                                                            0
                                        1x10-4                                                                                                              0.5     0.6    0.7      0.8     0.9
                                                                                                                                                                   Zipfian Skew Parameter (θ)
                                                                                                                                                                                                       1                                 0.5     0.6     0.7        0.8
                                                                                                                                                                                                                                                Zipfian Skew Parameter (θ)
                                                                                                                                                                                                                                                                          0.9    1


                                                                                                                                             Figure 8: Throughput of a single RAMCloud server for YCSB-A and
                                        1x10-5
                                                                                                                                             YCSB-B workloads with CURP at different Zipfian skewness levels.
                                        1x10-6                                                                                               Each experiment was run 5 times, and median values are displayed with
                                                   5 6 7       10          20       30                     100          200                  errorlines for min and max.
                                                                                Latency (µs)
                      Figure 6: Complementary cumulative distribution of latency for 100B
                                                                                                                              throughput by 10%. In all configurations except the original
                      random RAMCloud writes with CURP. Writes were issued sequentially                                       RAMCloud, masters are bottlenecked by a dispatch thread
                      by a single client to a single server, which batches 50 writes between                                  which handles network communications for both incoming
                      syncs. A point (x,y) indicates that y of the 1M measured writes took at                                 and outgoing RPCs. Sending witness gc RPCs burdens the
                      least x µs to complete. f refers to fault tolerance level (i.e. number of
                      backups and witnesses). “Original” refers to the base RAMCloud system
                                                                                                                              already bottlenecked dispatch thread and reduces throughput.
                      before adopting CURP. “Unreplicated” refers to RAMCloud without any                                        We also measured the latency and throughput of RAM-
                      replication. The median latency for synchronous, CURP ( f = 3), and                                     Cloud read operations before and after applying CURP, and
                      unreplicated writes were 14 µs, 7.1 µs, and 6.1 µs respectively.
                                                                                                                              there were no differences.




Write Throughput (k write per second)
                                        900                                                                                   5.2 Resource Consumption by Witness Servers
                                        800                                                                                      Each witness server implemented in RAMCloud can
                                        700                                                                                   handle 1270k record requests per second with occasional
                                                                                                     Unreplicated
                                        600                                                          CURP (f = 1)             garbage collection requests (1 every 50 writes) from master
                                        500                                                          Async (f = 3)            servers. A witness server runs on a single thread and con-
                                                                                                     CURP (f = 2)
                                        400                                                          CURP (f = 3)             sumes 1 hyper-thread core at max throughput. Considering
                                        300                                                          Original (f = 3)
                                                                                                                              that each RAMCloud master server uses 8 hyper-thread
                                        200                                                                                   cores to achieve 728k writes per second, adding 1 witness
                                        100
                                                                                                                              increases the total CPU resources consumed by RAMCloud
                                           0
                                               0           5            10         15        20              25         30    by 7%. However, CURP reduces the number of distinct
                                                                    Client Count (number of clients)                          backup operations performed by masters, because it enables
                      Figure 7: The aggregate throughput for one server serving 100B                                          batching; this offsets most of the cost of the witness requests
                      RAMCloud writes with CURP, as a function of the number of clients.
                                                                                                                              (both backup and witness operations are so simple that most
                      Each client repeatedly issued random writes back to back to a single
                      server, which batches 50 writes before syncs. Each experiment was                                       of their cost is the fixed cost of handling an RPC; a batched
                      run 15 times, and median values are displayed. “Original” refers to the                                 replication request costs about the same as a simple one).
                      base RAMCloud system before adding CURP. “Unreplicated” refers to                                          The second resource overhead is memory usage. Each
                      RAMCloud without any replication. In “Async” RAMCloud, masters
                      return to clients before backup syncs, and clients complete writes without                              witness server allocates 4096 request storage slots for each as-
                      replication to witnesses or backups.                                                                    sociated master, and each storage slot is 2KB. With additional
RAMCloud writes.                                                                                                              metadata, the total memory overhead per master-witness pair
   Figure 6 shows the latency of RAMCloud write operations                                                                    is around 9MB.
before and after applying CURP. CURP cuts the median write                                                                       The third issue is network traffic amplification. In CURP,
latencies in half. Even the tail latencies are improved overall.                                                              each update request is replicated both to witnesses and
When compared to unreplicated RAMCloud, each additional                                                                       backups. With 3-way replication, CURP increases network
replica with CURP adds 0.3 µs to median latency.                                                                              bandwidth use for update operations by 75% (in the original
   Figure 7 shows the single server throughput of write                                                                       RAMCloud, a client request is transferred over the network
operations with and without CURP by varying the number                                                                        to a master and 3 backups).
of clients. The server batches 50 writes before starting a                                                                    5.3 Impact of Highly-Skewed Workloads
sync. By batching backup syncs, CURP improves throughput                                                                         CURP may lose its performance benefits when used
by about 4x. When compared to unreplicated RAMCloud,                                                                          with highly-skewed workloads with hot keys; in CURP, an
adding an additional CURP replica drops throughput by ∼6%.                                                                    unsynced update on a key causes conflicts on all following
   To illustrate the overhead of CURP on throughput (e.g.                                                                     updates or reads on the same key until the sync completes. To
sending gc RPCs to witnesses), we measured RAMCloud                                                                           measure the impact of hot keys, we measured RAMCloud’s
with asynchronous replication to 3 backups, which is identical                                                                performance with CURP using a highly-skewed Zipfian
to CURP ( f =3) except that it does not record information on                                                                 distribution [14] with 1M objects. Specifically, we used two
witnesses. Achieving strong consistency with CURP reduces                                                                     different workloads similar to YCSB-A and YCSB-B [9];



USENIX Association                                                                                   16th USENIX Symposium on Networked Systems Design and Implementation                                                                                                       55
                        YCSB-A (50% read, 50% write) @ 250 kops                                   YCSB-B (95% read, 5% write) @ 700 kops
                       25                                                                    12                                            5.4 Making Redis Consistent and Durable



Average Latency (µs)                                                  Average Latency (µs)
                       20                                                                    10                                               Redis [30] is another low-latency in-memory key-value
                                                                                              8
                       15            Original (f=3)
                                     CURP (f=3)
                                                                                                                                           store, where values are data structures, such as lists, sets, etc.
                                                                                              6
                       10
                                     CURP (f=1)
                                     Unreplicated
                                                                                                                                           For Redis, the only way to achieve durability and consistency
                                                                                              4           Original (f=3)
                       5                                                                      2
                                                                                                          CURP (f=3)
                                                                                                          CURP (f=1)
                                                                                                                                           after crashes is to log client requests to an append-only file
                                                                                                          Unreplicated
                       0                                                                      0                                            and invoke fsync before responding to clients. However,
                            0.5     0.6       0.7     0.8   0.9   1                               0.5    0.6     0.7       0.8   0.9   1
                                    Zipfian Skew Parameter (θ)                                          Zipfian Skew Parameter (θ)         fsyncs can take several milliseconds, which is a 10–100x
                  Figure 9: Average RAMCloud client request latency for YCSB-A and                                                         performance penalty. As a result, most Redis applications do
                  YCSB-B workloads with CURP at different Zipfian skewness levels. 10                                                      not use synchronous mode; they use Redis as a cache with no
                  clients issued requests to maintain a certain throughput level (250 kops
                                                                                                                                           durability guarantees. Redis also offers replication to multi-
                  for YCSB-A and 700 kops for YCSB-B). Each experiment was run 5
                  times, and median values are displayed with errorlines for min and max.                                                  ple servers, but the replication mechanism is asynchronous,
                  Latency values are averaged over both read and write operations.                                                         so updates can be lost after crashes; as a result, this feature is
since RAMCloud is a key-value store and doesn’t support                                                                                    not widely used either.
100B field writes in 1k objects, we modified the YCSB                                                                                         For this experiment, we used CURP to hide the cost of
benchmark to read and write 100B objects with 30B keys.                                                                                    Redis’ logging mechanism: we modified Redis to record
   Figure 8 shows the impact of workload skew (defined                                                                                     operations on witnesses, so that operations can return
in [14]) on the throughput of a single server. For YCSB-A                                                                                  without waiting for log syncs. Log data is then written
(write-heavy workload), the server throughput with CURP                                                                                    asynchronously in the background. The result is a system
is similar to an unreplicated server when skew is low, but                                                                                 with durability and consistency, but with performance
it drops as the workload gets more heavily skewed. For                                                                                     equivalent to a system lacking both of these properties. In
YCSB-B, since most operations are reads, the throughput is                                                                                 this experiment the log data is not replicated, but the same
less affected by skew. CURP’s throughput benefit degrades                                                                                  mechanism could be used to replicate the log data as well.
starting at a Zipfian parameter θ = 0.8 (about 3% of accesses                                                                                 With the Redis implementation of CURP, we answered the
are on hot keys) and almost disappears at θ = 0.99.                                                                                        following questions:
                                                                                                                                             • Can CURP transform a fast in-memory cache into a
   Figure 9 shows the impact of skew on CURP’s latency;                                                                                         strongly-consistent durable storage system without
unlike the throughput benefits, CURP retains its latency                                                                                        degrading performance?
benefits even with extremely skewed workloads. We mea-                                                                                       • How wide a range of operations can CURP support?
sured latencies under load since an unloaded system will not
                                                                                                                                              Measurements of the Redis implementation were con-
experience conflicts even with extremely skewed workloads.
                                                                                                                                           ducted on a cluster of machines in CloudLab [29], whose
For YCSB-A, the latency of CURP increases starting at
                                                                                                                                           specifications are in Table 1. All measurements were col-
θ = 0.85, but CURP still reduces latency by 42% even at
                                                                                                                                           lected using 10 Gbps networking and NVMe SSDs for Redis
θ = 0.99. For YCSB-B, only 5% of operations are writes, so
                                                                                                                                           backup files. Linux fsync on the NVMe SSDs takes around
the latency improvements are not as dramatic as YCSB-A.
                                                                                                                                           50–100 µs; systems with SATA3 SSDs will perform worse
   Figure 10 shows the latency distributions of reads and                                                                                  with the fsync-always option.
writes separately at θ = 0.95 under the same loaded con-                                                                                      For the Redis implementation, we used Redis 3.2.8 for
ditions as Figure 9. For YCSB-A, CURP increases the tail                                                                                   servers and “C++ Client” [34] for clients. We modified “C++
latency for read operations slightly since reads occasionally                                                                              Client” to construct Redis requests more quickly.
conflict with unsynced writes on the same keys. CURP                                                                                          Figure 11 shows the performance of Redis before and after
reduces write latency by 2–4x: write latency with CURP                                                                                     adding CURP to its local logging mechanism; it graphs the
is almost as low as for unreplicated writes until the 50th                                                                                 cumulative distribution of latencies for Redis SET operations.
percentile, where conflicts begins to cause blocking on syncs.                                                                             After applying CURP (using 1 witness server), the median
Overall, the improvement of write latency by CURP more                                                                                     latency increased by 3 µs (12%). The additional cost is
than compensates for the degradation of read latency.                                                                                      caused primarily by the extra syscalls for send and recv on
   For YCSB-B, operation conflicts are more rare since                                                                                     the TCP socket used to communicate with the witness; each
all reads (which compose 95% of all operations) are com-                                                                                   syscall took around 2.5 µs.
mutative with each other. In this workload, CURP actually                                                                                     When a second witness server is added in Figure 11,
improved the overall read latency; this is because, by batching                                                                            latency increases significantly. This occurs because the
replication, CURP makes CPU cores more readily available                                                                                   Redis RPC system has relatively high tail latency. Even for
for incoming read requests (which is also why unreplicated                                                                                 the non-durable original Redis system, which makes only a
reads have lower latency). For YCSB-A, CURP doesn’t                                                                                        single RPC request per operation, latency degrades rapidly
improve read latency much since frequent conflicts limit                                                                                   above the 80th percentile. With two witnesses, CURP must
batching replication. In general, read-heavy workloads                                                                                     wait for three RPCs to finish (the original to the server,
experience fewer conflicts and are less affected by hot keys.                                                                              plus two witness RPCs). At least one of these is likely



56                                16th USENIX Symposium on Networked Systems Design and Implementation                                                                                USENIX Association
                       |———— YCSB-A @ 250 kops, Zipfian param (θ ): 0.95 ————-|                                                                                                                                      |———— YCSB-B @ 700 kops, Zipfian param (θ ): 0.95 ————-|
                                             READ (50%)                                                                        WRITE (50%)                                                                                     READ (95%)                                                                               WRITE (5%)
                           1                                                                        1                                                                                                            1                                                                                  1
                                                 Unreplicated                                                                                          Unreplicated                                                                 Unreplicated                                                                          Unreplicated
                                                 Original (f=3)                                                                                         CURP (f=3)                                                                   CURP (f=3)                                                                            CURP (f=3)




 Fraction of Reads                                                           Fraction of Writes                                                                                         Fraction of Reads                                                                    Fraction of Writes
                                                  CURP (f=3)                                                                                           Original (f=3)                                                               Original (f=3)                                                                        Original (f=3)

                      0.1                                                                          0.1                                                                                                       0.1                                                                                   0.1




                     0.01                                                                         0.01                                                                                                      0.01                                                                                  0.01
                               0        5   10      15      20     25   30                               0   20            40                                60   80        100   120                                0     5   10      15     20                   25   30                               0   10    20     30    40     50    60   70
                                             Latency (µs)                                                                                 Latency (µs)                                                                         Latency (µs)                                                                             Latency (µs)
              Figure 10: Complementary cumulative distribution of read and write latencies with CURP on a loaded server (250 kops for YCSB-A and 700 kops for
              YCSB-B). 10 clients issued read and write operations (using the read / write mix ratio of YCSB) for 1 min to a single server. The workloads used a Zipfian
              distribution with θ = 0.95, which means 16% of operations are on keys that were accessed within the last 100 executed operations.
                      1
                                                                                                                                                       200




                                                                                                                    Write Throughput (k write / sec)
                                                                                                                                                                                                                                                                  50                                          Original Redis (non-durable)
                     0.8                                                                                                                                                                                                                                                                                                CURP (1 Witness)




Fraction of Writes
                                                                                                                                                                                                                                                                                                                        CURP (2 Witness)
                                                                                                                                                       150                                                                                                        40



                                                                                                                                                                                                                                                   Latency (µs)
                     0.6
                                                                                                                                                                                                                                                                  30
                                                                                                                                                       100                                                                                                        20
                     0.4

                                                     Original Redis (non-durable)                                                                                                                                                                                 10
                     0.2                                        CURP (1 Witness)                                                                       50                          Original Redis (non-durable)
                                                             CURP (2 Witnesses)                                                                                                               CURP (1 Witness)                                                    0
                                                          Original Redis (durable)                                                                                                         CURP (2 Witnesses)                                                                              SET                    HMSET              INCR
                      0                                                                                                                                                                 Original Redis (durable)
                                   20       40        60          80    100                          120      140                                       0                                                                                                    Figure 13: Median latencies before and after
                                                         Latency (µs)                                                                                        0         10         20                        30            40    50           60
                                                                                                                                                                            Client Count (number of clients)
                                                                                                                                                                                                                                                             applying CURP on various Redis commands.
              Figure 11: Cumulative distribution of latency for                                                                                                                                                                                              All experiments select a random 30B key over
                                                                                                                                               Figure 12: The aggregate throughput for one                                                                   2M unique keys. SET used 100B random val-
              100B random Redis SET requests with CURP.
                                                                                                                                               server serving 100B Redis SET operations with                                                                 ues, and each HMSET operation sets 1 member
              Writes were issued sequentially by a single client
                                                                                                                                               CURP, as a function of the number of clients. Each                                                            with a 100B value. The member key was 1B.
              to a single Redis server. CURP used one or two
                                                                                                                                               client repeatedly issued random writes back to back                                                           Commands were issued sequentially by a sin-
              additional Redis servers as witnesses. “Original
                                                                                                                                               to a single server. “Original Redis (durable)” refers                                                         gle client to a single Redis server, with one or
              Redis (durable)” refers to the base Redis without
                                                                                                                                               to the base Redis without CURP, but configured to                                                             two additional Redis witness servers in CURP.
              CURP, configured to invoke fsync on a backup file
                                                                                                                                               invoke fsync before replying to clients.
              before replying to clients.
to experience high tail latency and slow down the overall                                                                                                                                               writes data to a member of a hashmap; and INCR, which
completion. We didn’t see a similar effect in RAMCloud                                                                                                                                                  increments an integer counter and returns its current value.
because its latency is consistent out to the 99th percentile:                                                                                                                                           For all three operations, latency overheads were small for
when issuing three concurrent RPCs, it is unlikely that any of                                                                                                                                          CURP with 1 witness. CURP with 2 witnesses increased
them will experience high latency.                                                                                                                                                                      latency about 10 µs because of tail latency issues. We believe
   Figure 12 shows the throughput of Redis SET operations                                                                                                                                               that the TCP transport library used by the C++ client is
for a single Redis server with varying numbers of clients.                                                                                                                                              inefficient for waiting for multiple responses concurrently,
Applying CURP reduced the throughput of Redis about 18%.                                                                                                                                                and we will continue to investigate this.
With a large number of clients, the original synchronous                                                                                                                                                6                Related work
form of Redis can offer throughput approaching non-durable                                                                                                                                                 Table 2 summarizes the performance of CURP and other
Redis. The reason for this is that Redis batches fsyncs in                                                                                                                                              fast replication protocols. The paragraphs below explain
synchronous mode: in each cycle through its event loop, it                                                                                                                                              these numbers in detail. We present analytical performance
processes all of the requests waiting on its incoming sockets,                                                                                                                                          instead of emprical results since empirical performance de-
issues a single fsync, then responds to all of those requests.                                                                                                                                          pends too much on implementation and underlying systems
The disadvantage of this approach is that it results in very                                                                                                                                            (e.g. CURP on RAMCloud and CURP on Redis have very
high latency for clients.                                                                                                                                                                               different absolute performance).
5.5 Applicability of CURP                                                                                                                                                                                  Generalized Paxos [18] allows clients to complete op-
   CURP can be applied to a variety of operations, not just                                                                                                                                             erations (i.e. receive execution results) in 1.5 RTTs and
write operations in key-value stores. Redis supports many                                                                                                                                               supersedes Fast Paxos [19]. Both protocols allow clients to
data structures, such as strings, hashmaps, lists, counters, and                                                                                                                                        send requests directly to replicas and reduce latency from 2
so on. All of these update operations (including ones that                                                                                                                                              RTTs to 1.5 RTT. Fast Paxos has a contention problem and
are non-idempotent or return read values) can benefit from                                                                                                                                              performs well only at low throughput. Generalized Paxos
CURP. Since each data structure is assigned to a specific key,                                                                                                                                          resolves the contention problem by using commutativity; it
CURP can execute many update operations on different keys                                                                                                                                               groups commutative requests from concurrent clients into
without blocking on syncs.                                                                                                                                                                              an unordered set, and it only orders between sets. Although
   Figure 13 shows the median latency with and without                                                                                                                                                  Generalized Paxos allows a leader replica to learn that oper-
CURP on three different Redis commands: SET, which                                                                                                                                                      ations are committed in 1 RTT, clients need to wait another
writes ASCII data to a string data structure; HMSET, which                                                                                                                                              half RTT to receive the execution results from the leader; so



USENIX Association                                                                                                  16th USENIX Symposium on Networked Systems Design and Implementation                                                                                                                                                          57
                             CURP     Gen.Paxos   EPaxos         NOPaxos          industry [10, 8, 5]. Systems using eventual consistency return
                     read     1 RTT    1.5 RTTs    2 RTTs        1 RTT + α

           WAN LAN
                                                                                  from updates before replication is complete, and replications
 Latency
                     write    1 RTT    1.5 RTTs    2 RTTs        1 RTT + α
                     read    ∼0 RTT    1.5 RTTs    ∼1 RTT        Not Avail.       happen asynchronously; since nearby replicas are stale,
                     write    1 RTT    1.5 RTTs    ∼1 RTT        Not Avail.       clients must read from far-away masters for consistency.
 load on             read    <1 RPC    ∼ n RPCs   ∼2 RPCs            1 RPC        Pileus [35] and Tuba [2] allowed applications to declare
  leader             write    1 RPC    ∼ n RPCs   ∼2 RPCs            1 RPC
                                                                                  their consistency and latency priorities, and they dynamically
 Table 2: Performance comparisons of replication protocols. “LAN”
 means intra-datacenter replications. “WAN” means geo-replication and
                                                                                  select replicas to read from.
 assumes that all clients have a local replica; clients in a datacenter without      Broadcast-broadcast (BB) protocols [4, 3, 12, 16] for total
 local replicas must send requests to a remote replica and experience the         order broadcasts [11] have similarities to CURP. Senders in
 WAN RTTs same as in “LAN”. NOPaxos’s RTT is longer than usual since              BB protocols broadcast a message to all destinations (repli-
 network packets must detour through a sequencer. All latency numbers
 omitted the time to make data persistent, which is same for all protocols        cated processes) plus a sequencer before ordering, followed
 (1 persistence time per request) and insignificant with the use of modern        by a second broadcast from the sequencer about the ordering
 fast storage technologies. “Load on leader” shows how many RPCs a                information. Some variants of BB protocols [3, 12] exploit
 leader (or master) processes per client request. “n” denotes the number of       the fact that broadcasts are mostly delivered in-order in small
 replicas.
                                                                                  LAN environments and let processes optimistically consume
its end-to-end latency becomes 1.5 RTTs, as opposed to 1                          messages without waiting for the ordering information from
RTT for CURP. (See §B.3 for a detailed explanation why they                       the sequencer. If the suspected order turned out to be different
cannot achieve 1 RTT.)                                                            from the order determined by the sequencer, the process must
   Egalitarian Paxos (EPaxos) [22] relies on commutativity                        rollback to correct the inconsistency. On the other hand, in
to allow multiple leaders to propose and execute operations                       CURP, replicas wait for the ordered replication from a master
concurrently. This approach improves throughput. In geo-                          instead of executing operations with a presumed ordering, so
replicated environments, EPaxos allows clients to choose a                        CURP doesn’t require rollbacks, which is expensive and diffi-
nearby replica as leader, so operations can complete in 1 wide-                   cult to implement. Furthermore, even if client requests arrive
area RTT. However, in LAN environments, EPaxos clients                            in a master and witnesses out of order, CURP still achieves 1
cannot hide the message delay to a leader, so operations take                     RTT as long as the reordered requests are commutative.
2 RTT. Also, since EPaxos does not have a strong leader, read
operations must run through full consensus and be written to                      7   Conclusion
replicated command logs; for read-heavy workloads, EPaxos                            In this paper we have uncovered an opportunity for intro-
will perform worse than traditional 2 RTT protocols with                          ducing concurrency into mechanisms for consistent replica-
read leases, such as Raft [25]. On the other hand, CURP can                       tion. By exploiting the commutativity of operations, replica-
directly execute read operations in masters or even in backups                    tion without ordering can be performed in parallel with send-
with the help of witnesses. Another limitation of EPaxos is                       ing requests to an execution server. This general approach can
that clients in a datacenter that doesn’t host a replica must use                 be applied to improve a variety of replication mechanisms,
a remote leader, increasing its latency to 2 wide-area RTTs.                      including primary-backup approaches and consensus proto-
                                                                                  cols with strong leaders. We presented Consistent Unordered
   Speculative Paxos [28] and Network-Ordered Paxos
                                                                                  Replication Protocol (CURP), which supplements standard
(NOPaxos) [21] reduce latency almost to 1 RTT by seri-
                                                                                  primary-backup replication mechanisms. CURP reduces the
alizing client requests within network. Both protocols use
                                                                                  latency to complete operations from 2 RTTs to 1 RTT while
SDNs to detour requests from all clients through a single
                                                                                  retaining strong consistency. We implemented CURP in
network device (a root layer switch or middlebox); so, they
                                                                                  RAMCloud and Redis to demonstrate its benefits.
can be deployed only in specialized environments (e.g. a
privately-owned datacenter). Also, due to detouring of                            Acknowledgements
packets, they actually add latency overhead over unreplicated                        We thank our shepherd, Manos Kapritsos, and our anony-
systems; Speculative Paxos (∼25 µs) or NOPaxos(∼16 µs)                            mous NSDI and OSDI reviewers for their feedback. Thanks
have higher latency overhead compared to CURP (∼1 µs).                            to Stephen Yang and Collin Lee for helping on improving
   TAPIR [37] and Janus [23] commit distributed transactions                      the clarity of this paper. This work was supported by the
in 1 wide-area RTT; before them, transaction commits took                         industrial affiliates of the Stanford Platform Lab and by the
2 RTTs: 1 for transaction prepares and 1 for geo-replicating                      Samsung Scholarship.
the data of prepare. They flattened out these serial steps by                     References
replicating data before the prepare is executed. They mod-                         [1] GlusterFS. https://www.gluster.org, 2017.
ified concurrency control protocols to fix inconsistencies in                          Accessed: 2017-09-22.
replications. They also require commutativity of workloads
for 1 RTT commits.                                                                 [2] A RDEKANI , M. S., AND T ERRY, D. B. A self-
   To avoid the performance penalty of consistent replica-                             configurable geo-replicated cloud storage system. In
tions, eventual consistency [36] has been widely adopted in                            11th USENIX Symposium on Operating Systems Design



58         16th USENIX Symposium on Networked Systems Design and Implementation                                             USENIX Association
     and Implementation (OSDI 14) (Broomfield, CO, 2014),                (Phoenix, AZ, USA, 2001), ICDCS ’01, IEEE Com-
     USENIX Association, pp. 367–381.                                    puter Society, pp. 333–341.
 [3] BALAKRISHNAN , M., B IRMAN , K., AND P HAN -                   [13] G HEMAWAT, S., G OBIOFF , H., AND L EUNG , S.-T.
     ISHAYEE , A. PLATO: Predictive latency-aware total or-              The Google file system. SIGOPS Oper. Syst. Rev. 37,
     dering. In Proceedings of the 25th IEEE Symposium on                5 (Oct. 2003), 29–43.
     Reliable Distributed Systems (Leeds, UK, 2006), SRDS
     ’06, IEEE Computer Society, pp. 175–188.                       [14] G RAY, J., S UNDARESAN , P., E NGLERT, S., BA -
                                                                         CLAWSKI , K., AND W EINBERGER , P. J. Quickly gen-
 [4] B IRMAN , K., S CHIPER , A., AND S TEPHENSON , P.                   erating billion-record synthetic databases. SIGMOD
     Lightweight causal and atomic group multicast. ACM                  Rec. 23, 2 (May 1994), 243–252.
     Trans. Comput. Syst. 9, 3 (Aug. 1991), 272–314.
                                                                    [15] H UNT, P., KONAR , M., J UNQUEIRA , F. P., AND R EED ,
 [5] B RONSON , N., A MSDEN , Z., C ABRERA , G.,                         B. ZooKeeper: Wait-free coordination for internet-
     C HAKKA , P., D IMOV, P., D ING , H., F ERRIS , J., G I -           scale systems. In Proceedings of the 2010 USENIX
     ARDULLO , A., K ULKARNI , S., L I , H., M ARCHUKOV,
                                                                         Conference on USENIX Annual Technical Conference
     M., P ETROV, D., P UZAR , L., S ONG , Y. J., AND                    (Boston, MA, 2010), USENIXATC’10, USENIX Asso-
     V ENKATARAMANI , V. TAO: Facebook’s distributed                     ciation, pp. 11–11.
     data store for the social graph. In Presented as part of the
     2013 USENIX Annual Technical Conference (USENIX                [16] K AASHOEK , M. F., AND TANENBAUM , A. S. Group
     ATC 13) (San Jose, CA, 2013), USENIX, pp. 49–60.                    communication in the amoeba distributed operating sys-
                                                                         tem. In [1991] Proceedings. 11th International Con-
 [6] B URROWS , M. The Chubby lock service for loosely-
                                                                         ference on Distributed Computing Systems (May 1991),
     coupled distributed systems. In Proceedings of the 7th
                                                                         pp. 222–230.
     Symposium on Operating Systems Design and Imple-
     mentation (Seattle, WA, 2006), OSDI ’06, USENIX As-            [17] L AMPORT, L. The part-time parliament. ACM Transac-
     sociation, pp. 335–350.                                             tions on Computer Systems 16, 2 (May 1998), 133–169.
 [7] C HODOROW, K., AND D IROLF, M. MongoDB: The                    [18] L AMPORT, L. Generalized consensus and Paxos. Tech.
     Definitive Guide, 1st ed. O’Reilly Media, Inc., 2010.               rep., March 2005.
 [8] C OOPER , B. F., R AMAKRISHNAN , R., S RIVASTAVA ,
                                                                    [19] L AMPORT, L. Fast Paxos. Distributed Computing 19
     U., S ILBERSTEIN , A., B OHANNON , P., JACOBSEN ,
                                                                         (October 2006), 79–103.
     H.-A., P UZ , N., W EAVER , D., AND Y ERNENI , R.
     PNUTS: Yahoo!’s hosted data serving platform. Proc.            [20] L EE , C., PARK , S. J., K EJRIWAL , A., M ATSUSHITA ,
     VLDB Endow. 1, 2 (Aug. 2008), 1277–1288.                            S., AND O USTERHOUT, J. Implementing linearizabil-
 [9] C OOPER , B. F., S ILBERSTEIN , A., TAM , E., R A -                 ity at large scale and low latency. In Proceedings of
     MAKRISHNAN , R., AND S EARS , R. Benchmarking
                                                                         the 25th Symposium on Operating Systems Principles
     cloud serving systems with YCSB. In Proceedings of the              (Monterey, CA, 2015), SOSP ’15, ACM, pp. 71–86.
     1st ACM Symposium on Cloud Computing (Indianapo-               [21] L I , J., M ICHAEL , E., S HARMA , N. K., S ZEKERES ,
     lis, IN, 2010), SoCC ’10, ACM, pp. 143–154.                         A., AND P ORTS , D. R. K. Just say no to Paxos over-
[10] D E C ANDIA , G., H ASTORUN , D., JAMPANI , M.,                     head: Replacing consensus with network ordering. In
     K AKULAPATI , G., L AKSHMAN , A., P ILCHIN , A.,                    Proceedings of the 12th USENIX Conference on Oper-
     S IVASUBRAMANIAN , S., VOSSHALL , P., AND VO -                      ating Systems Design and Implementation (Savannah,
     GELS , W. Dynamo: Amazon’s highly available key-                    GA, 2016), OSDI’16, USENIX Association, pp. 467–
     value store. In Proceedings of Twenty-first ACM                     483.
     SIGOPS Symposium on Operating Systems Principles
                                                                    [22] M ORARU , I., A NDERSEN , D. G., AND K AMINSKY,
     (Stevenson, WA, 2007), SOSP ’07, ACM, pp. 205–220.
                                                                         M. There is more consensus in egalitarian parlia-
[11] D ÉFAGO , X., S CHIPER , A., AND U RB ÁN , P. Total or-           ments. In Proceedings of the Twenty-Fourth ACM Sym-
     der broadcast and multicast algorithms: Taxonomy and                posium on Operating Systems Principles (Farminton,
     survey. ACM Comput. Surv. 36, 4 (Dec. 2004), 372–421.               PA, 2013), SOSP ’13, ACM, pp. 358–372.

[12] F ELBER , P., AND S CHIPER , A. Optimistic active              [23] M U , S., N ELSON , L., L LOYD , W., AND L I , J. Con-
     replication. In Proceedings of the The 21st Interna-                solidating concurrency control and consensus for com-
     tional Conference on Distributed Computing Systems                  mits under conflicts. In Proceedings of the 12th USENIX



USENIX Association                            16th USENIX Symposium on Networked Systems Design and Implementation           59
     Conference on Operating Systems Design and Imple-           [33] S IVASUBRAMANIAN , S. Amazon dynamoDB: A seam-
     mentation (Savannah, GA, 2016), OSDI’16, USENIX                  lessly scalable non-relational database service. In Pro-
     Association, pp. 517–532.                                        ceedings of the 2012 ACM SIGMOD International Con-
                                                                      ference on Management of Data (Scottsdale, AZ, 2012),
[24] O KI , B. M., AND L ISKOV, B. H. Viewstamped repli-              SIGMOD ’12, ACM, pp. 729–730.
     cation: A new primary copy method to support highly-
     available distributed systems. In Proceedings of the        [34] S PRENKER , L., AND H AMMOND , B.   Redis
     Seventh Annual ACM Symposium on Principles of Dis-               C++ Client.     https://github.com/mrpi/
     tributed Computing (Toronto, Ontario, Canada, 1988),             redis-cplusplus-client, 2011. Accessed:
     PODC ’88, ACM, pp. 8–17.                                         2017-04-20.

                                                                 [35] T ERRY, D. B., P RABHAKARAN , V., KOTLA , R., BAL -
[25] O NGARO , D., AND O USTERHOUT, J. In search
                                                                      AKRISHNAN , M., AGUILERA , M. K., AND A BU -
     of an understandable consensus algorithm. In 2014
                                                                      L IBDEH , H. Consistency-based service level agree-
     USENIX Annual Technical Conference (USENIX ATC
                                                                      ments for cloud storage. In Proceedings of the Twenty-
     14) (Philadelphia, PA, 2014), USENIX Association,
                                                                      Fourth ACM Symposium on Operating Systems Princi-
     pp. 305–319.
                                                                      ples (Farminton, PA, 2013), SOSP ’13, ACM, pp. 309–
                                                                      324.
[26] O NGARO , D., RUMBLE , S. M., S TUTSMAN , R.,
     O USTERHOUT, J., AND ROSENBLUM , M. Fast crash              [36] VOGELS , W. Eventually consistent. Commun. ACM 52,
     recovery in RAMCloud. In Proceedings of the Twenty-              1 (Jan. 2009), 40–44.
     Third ACM Symposium on Operating Systems Princi-
     ples (Cascais, Portugal, 2011), SOSP ’11, ACM, pp. 29–      [37] Z HANG , I., S HARMA , N. K., S ZEKERES , A., K R -
     41.                                                              ISHNAMURTHY, A., AND P ORTS , D. R. K. Build-
                                                                      ing consistent transactions with inconsistent replication.
[27] O USTERHOUT, J., G OPALAN , A., G UPTA , A., K EJRI -            In Proceedings of the 25th Symposium on Operating
     WAL , A., L EE , C., M ONTAZERI , B., O NGARO , D.,              Systems Principles (Monterey, CA, 2015), SOSP ’15,
     PARK , S. J., Q IN , H., ROSENBLUM , M., RUMBLE ,                ACM, pp. 263–278.
     S., S TUTSMAN , R., AND YANG , S. The RAMCloud
     storage system. ACM Trans. Comput. Syst. 33, 3 (Aug.        [38] Z HAO , W. Fast Paxos made easy: Theory and imple-
     2015), 7:1–7:55.                                                 mentation. International Journal of Distributed Systems
                                                                      and Technologies (IJDST) 6, 1 (2015), 15–33.
[28] P ORTS , D. R. K., L I , J., L IU , V., S HARMA , N. K.,
     AND K RISHNAMURTHY, A. Designing distributed sys-
     tems using approximate synchrony in data center net-
     works. In Proceedings of the 12th USENIX Confer-
     ence on Networked Systems Design and Implementation
     (Oakland, CA, 2015), NSDI’15, USENIX Association,
     pp. 43–57.

[29] R ICCI , R., E IDE , E., AND T EAM , C. Introducing
     CloudLab: Scientific infrastructure for advancing cloud
     architectures and applications. ; login:: the magazine of
     USENIX & SAGE 39, 6 (2014), 36–38.

[30] S ANFILIPPO , S., ET AL . Redis. https://redis.
     io/, 2015. Accessed: 2017-04-18.

[31] S CHNEIDER , F. B. Implementing fault-tolerant ser-
     vices using the state machine approach: A tutorial. ACM
     Comput. Surv. 22, 4 (Dec. 1990), 299–319.

[32] S HVACHKO , K., K UANG , H., R ADIA , S., AND
     C HANSLER , R. The Hadoop distributed file system. In
     2010 IEEE 26th Symposium on Mass Storage Systems
     and Technologies (MSST) (May 2010), pp. 1–10.



60   16th USENIX Symposium on Networked Systems Design and Implementation                                 USENIX Association
A    Informal Proof of Correctness                                erations that are replicated to backups. CURP may break
   With the normal operation behaviors described in §3.2,         the linearizability of the underlying system since masters in
the recovery protocol in §3.3 guarantees the following            CURP return before syncing to backups. So, we will reason
correctness properties.                                           about how CURP recovers from master crashes without
  • Durability: if a client completes an operation, it survives   breaking linearizability.
     server crashes.                                                 The definition of linearizability can be reworded as
  • Consistency: if a client completes an operation, its result   following: if the execution of an operation is observed by
     returned to an application remains consistent after server   the issuing client or other clients, no contrary observation
     crash recoveries.                                            can occur afterwards (i.e. it should not appear to revert or
  • Linearizability: an operation appears to be executed          be reordered). Since we only care about what happens after
     exactly once between start and completion.                   recovery, we prove the following proposition: if the execution
   Before presenting proofs, we reiterate some key behaviors      of an individual operation α is observed before crash, no
of the CURP protocol.                                             contrary observation can occur after recovery.
   (Rule 1) from §3.2.1, a client only completes an update           Case 1: the execution of α was observed by other depen-
operation if (1) it is recorded in all f witnesses or (2) it is   dent operations (e.g. reads). By (Rule 2), the master must
replicated to f backups.                                          have synced α to backups since dependent operations don’t
   (Rule 2) a completed unsynced operation must be individ-       commute with α. Since it was replicated to backups, α will
ually commutative with all preceding operations that are not      be linearizable as long as the underlying system is.
synced yet. This is the behavior described in §3.2.3; a master       Case 2: the execution was observed only by the completion
must sync before responding if the current operation is not       of α. α must be recovered because of the Durability property.
commutative with any other existing (preceding) unsynced          The only observation about α before crash was the returned
operations.                                                       execution result, and it must be still consistent even after
   Now, we present proof sketches for the properties.             recovery because of the Consistency property.
   Durability: recovery of a master only completes after             Case 3: no observation was made before crash. α may
recovery from 1 backup and 1 witness, and the completed           be lost if it didn’t reach to either the backup or witness used
operation must exist in the backup or the witness by (Rule 1);    for recovery. In CURP, the client keeps retrying until it can
thus, the completed operation must be recovered when the          complete α. Regardless of whether α was recovered or not,
recovery is completed.                                           RIFL ensures the retry will only execute α at-most once and
   Consistency: Consider an individual completed operation        return the result of the sole execution.                     
α and its consistency. To prove that α’s result doesn’t change
even after crash recovery, we will think about the operation      B    Extra Discussions
execution sequence before α, which we will call history of α      B.1 Why Are Witnesses Separate from Backups?
(or Hα ).                                                            By having witnesses separated from backups, CURP
   Case 1: the operation α has been synced to the backup          requires fewer changes to the existing systems and is more
used for recovery. This operation will be recovered from          applicable to many wildly different backup mechanisms.
the backup (phase 1) and any replay from witnesses (phase         Both of our two implementations leveraged this flexibility:
2) will be ignored (by RIFL). Since backup syncs preserve         in RAMCloud, a master keeps changing backups to which
the execution order of operations, the Hα didn’t change; so       it replicates (to spread data over the entire cluster), so
the post-recovery execution sequence should regenerate the        clients don’t know which backups are currently used by the
original execution result of α.                                   master; in Redis, operation logs are stored in local disks to
   Case 2: the operation α has not been synced to the backup      ensure durability, so there are no separate backup servers
used for recovery. α must have been recorded in all witnesses     to which CURP clients can record inputs. Thus, separating
by (Rule 1) and will be recovered during phase 2. We can          witnesses from backups improves CURPs applicability to
split the original execution history of α into two parts as in    many existing primary-backup systems.
Figure 3: hsyncedi followed by hunsyncedi. The 1st phase             On the other hand, when designing a new storage system,
of recovery will recover the exactly same execution history       combining witnesses and backups can bring extra perfor-
for the hsyncedi part. By (Rule 2), we know that losing any       mance benefits. When they are combined, clients directly
hunsyncedi part of history after crash will not change the        send requests to a master and backups, which now also serve
execution result of α. During phase 2 of recovery (from           as witnesses. The key change is masters now sync operation
a witness), we may replay some other operations before            orders (by listing IDs as in witness gc RPCs) instead of full
replaying α, but the result of α doesn’t change since all         client requests; then backups lookup the matching requests
operations recorded in the witness must be commutative.          from their witness storage and move them to backup logs.
   Linearizability: we assume that the underlying system          This approach will lower network bandwidth consumption.
before applying CURP guarantees linearizability for op-           Also, most witness gc RPCs can be eliminated; immediately



USENIX Association                           16th USENIX Symposium on Networked Systems Design and Implementation             61
after handling the sync, the requests in the witness storage          For correctness, the client requests replayed from witnesses
can be deleted as they are now safe in the backup log. (For        during recovery must be commutative and inclusive of all
safety, the recovery protocol must pick 1 witness/backup           completed operations that are not yet committed in a major-
combo and must not mix.) This saving of gc RPCs will               ity of replicas. By recording to a superquorum, all completed
improve masters’ throughput and will reduce the chance of          operations (but not yet committed) are guaranteed to exist in
commutativity conflicts.                                           a majority (d f /2e+1) of any quorum of f +1 witnesses, and
B.2 Extending CURP to Consensus Protocols                          any operations that don’t commute with the completed oper-
   This section illustrates how CURP can be extended to            ations cannot exist in more than b f /2c (less than majority of
reduce the latency of consensus protocols. CURP can be in-         any quorum). Thus, during recovery, all requests that appear
tegrated in most consensus protocols with strong leaders (e.g.     in a majority (d f /2e+1) from any quorum of f +1 witnesses
Raft [25], Viewstamped Replication [24]). In such protocols,       are guaranteed to be commutative and include all completed
clients send requests to the current leader, which serializes      operations; so, recovery can replay requests that appear in
the requests into its command log. The leader then replicates      more than d f /2e+1 witnesses out of any f +1 witnesses.
its command log to a majority of replicas before executing            When leadership changes (e.g. leader election in Raft [25]
the requests and replying back to clients with the results. This   or view change in Viewstamped Replication [24]), the new
process takes 2 RTTs, and CURP can reduce it to 1 RTT.             leader must recover from witnesses before accepting new op-
   As in primary-backup replication, CURP on consensus             erations. To do so, the new leader must collect saved requests
allows clients to replicate requests to witnesses in parallel      from at least f +1 witnesses. This collection can be included
with sending requests to the leader; the leader then specu-        in the existing data collection (e.g. Raft votes) that is required
latively executes the requests and responds to clients before      by most leadership change protocols. As mentioned in the
replicating the requests to a quorum of replicas. A client can     previous paragraph, the new leader should only replay client
complete an operation if it is accepted by a superquorum of        requests that are recorded in at least d f /2e + 1 witnesses to
witnesses or committed in a quorum of replicas.                    ensure commutativity.
   To mask f failures, consensus protocols use 2 f + 1 repli-         After leadership changes, the state machine of the old
cas, and systems stay available with f failed replicas. For the    leader could have diverged from other replicas due to
same guarantee, CURP also uses 2 f + 1 replicas, but each          speculatively executed operations that were not recovered
replica also has a witness component in addition to existing       from witnesses. To fix this, the old leader must reload from
components for consensus. Although CURP can proceed                a checkpoint that does not have speculative executions.
with f + 1 available replicas, it needs f + d f /2e + 1 replicas   However, we can avoid reloading from checkpoints if the
(for superquorum of witnesses) to use 1 RTT operations.            leadership change was not because of a crash or disconnect of
With less than f +d f /2e+1 replicas, clients must ask masters     the old leader; instead of requring old leader to reload from a
to commit operations in f +1 replicas before returning result      checkpoint, we can require the new leader to fetch and commit
(2 RTTs).                                                          all uncommitted operations in the old leader’s command log.
   Like masters in regular CURP, leader replicas execute oper-        The last problem introduced by speculative execution is
ations speculatively if they are commutative with existing un-     that clients may use old zombie leaders (which believe they
synced operations; for an incoming client request, the leader      are current leaders). Zombie leaders were not possible before
serializes it into its command log, executes it, and responds      CURP since an operation must be committed in a majority
to the client before committing it in a majority of replicas.      before being executed and at least one replica would reject
   For clients to complete an operation in 1 RTT, it must be       the operation. To prevent clients from completing operations
recorded in a superquorum of f + d f /2e + 1 witnesses. The        with an old (possibly disconnected) leader, they tag record
reason why CURP needs a superquorum instead of a simple            RPCs with a term number (e.g. a Raft term or a view-number
majority is to ensure commutativity of replays from witnesses      in Viewstamped Replication), which increments every time
during recovery. During recovery, only f + 1 out of 2 f + 1        when leadership changes. A witness checks the term number
replicas (each of which embeds a witness) might be available.      against the term used by its replica (recall that a witness is a
If a client could complete an operation after recording to         part of a consensus replica); if the record RPC has an old term
 f + 1 witnesses, the completed operation may exist in only        number, the witness rejects the request and tells the client to
1 witness out of available f + 1 witnesses during recovery         fetch new leader information.
(since intersection of two quorums is 1 replica). If the other f      CURP can use read leases like many consensus protocols
witnesses accepted other operations that are not commutative       so that read operations can be executed solely by leaders
with the completed operation (since each witness enforces          within 1 RTT without recording to witnesses. Optimizing
commutativity individually), recovery cannot distinguish           read operations using read leases is common for consensus
which one is the completed one; executing all appearing            protocols with strong leaders. A leader replica with a
in any f + 1 witnesses is also not safe since they are not         valid read lease can safely execute read operations without
commutative, so they must be replayed in a correct order.          committing the read operations through consensus. For the



62   16th USENIX Symposium on Networked Systems Design and Implementation                                     USENIX Association
                                                                     Number of records between conflicts
optimization, each replica grants the read lease to the current                                            1400
                                                                                                                       8-way associative
leader, promising not to agree on a leader change for a lease                                              1200        4-way associative
                                                                                                                       2-way associative
period. With valid leases from a majority of replicas, the                                                 1000        Direct mapping
leader knows that no operations can be committed from other                                                800
replicas, so it can safely execute read operations without
                                                                                                           600
consulting with other replicas. CURP does not interfere with
                                                                                                           400
this read lease mechanism.
                                                                                                           200
B.3 Why Do Fast / Generalized Paxos require 1.5 RTTs?
   There is a widespread misunderstanding that both Fast                                                     0
                                                                                                              500   1000   1500 2000 2500 3000 3500          4000   4500
Paxos and Generalized Paxos already achieve 1 RTT opera-                                                                      Number of slots in a witness
tions. The confusion probably stems from the fact that both          Figure 14: Simulation results for the expected number of recordings
Fast and Generalized Paxos allow Paxos learners to know              before a collision occurs in a witness’ cache, assuming a random
about acceptance of an operation in 1 RTT.                           distribution of keys. Each data point is the average of 10000 simulations.
                                                                     Introducing associativity reduces the chance of collisions significantly.
   However, 1 RTT is sufficient to know only that an operation
                                                                    the receipts of results, masters remove their completion
is committed but not enough to know the result: that requires
                                                                    records and start to ignore (not returning results) the duplicate
another 0.5 RTT. The abstract for Generalized Paxos says that
                                                                    requests. Since replays from witnesses happen in random
a server can execute the command in two message delays;
                                                                    orders, acknowledgements piggybacked on later requests can
however, it take an additional message delay for the result to
                                                                    make masters to ignore the replay of earlier requests. Thus,
reach a client, for a total of three message delays (1.5 RTT). It
                                                                    clients’ acknowledgments included in RPC requests must be
doesn’t help for the client to be a Paxos learner, because even
                                                                    ignored during recovery from witnesses.
learners don’t know the result after 1 RTT.
   For most operations, results are not trivial and clients must       Secondly, if a client crashes and its lease expires, masters
wait for the results from real executions before completing         remove all of the completion records for the client; then any
operations. Many writes, such as conditional writes or read-        requests from the expired client are ignored. This can be
modify-writes, have results that clients cannot know before         a problem in CURP since the replay of the expired client’s
executions. Blind writes (those that don’t return results)          requests will be ignored during witness-based recovery. To
could potentially complete in 1 RTT. However, truly blind           prevent this, masters must sync all operations to backups
writes are rarely feasible because they can return exceptions,      before expiring a client lease. In practice, the period of syncs
such as “table no longer on this server” or “permission             is much smaller than the grace period between the time of
denied”; clients must be aware of these exceptions.                 a client crash and the time of its lease expiration; so, most
                                                                    systems are safe automatically.
   As a result, Fast/Generalized Paxos are generally con-
sidered to have 1.5 RTT latency for clients to complete             C.2 Why Use Set-associative Cache for Witnesses?
operations. [21, 28, 38]                                               We initially used a direct-mapped cache instead of set-
                                                                    associative cache, but this resulted in a high rate of rejections
C    Implementation Details                                         because of conflicts (i.e. no slot is available for the mapped
C.1 Modifications to RIFL                                           set). Figure 14 shows the expected number of recordings
   RIFL [20] is a mechanism for detecting duplicate invoca-         before a conflict occurs on a witness slot. Using a direct
tions of RPCs. With RIFL, masters make a durable comple-            mapping and 4096 total slots, it is expected to have a false
tion record of each RPC that updates state, which includes          conflict after about 80 insertions. Thus, we switched to
the RPC result. The completion record survives crashes and          4-way associative cache, to reduce witness rejections. We
can be used to detect duplicate invocations of the RPC. When        didn’t need 8-way associativity (a bit slower than 4-way)
a duplicate is detected, the master skips the execution of the      since the number of requests in witnesses is already limited
RPC and returns the result from the completion record.              by commutativity. (Once a master hits a non-commutative
   RIFL has two mechanisms for garbage collecting com-              operation and syncs to backups, all saved requests in the
pletion records: (1) on RPC requests, clients piggyback             witness are garbage collected.)
acknowledgments of the results of their previous requests (so
servers can safely delete these completion records), and (2)        D                                       Additional Evaluations
clients maintain leases in a central server; if a client’s lease    D.1 RAMCloud’s Throughput by Batch Size
expires, masters can delete all completion records for that           Figure 15 shows the single-server throughput of write
client. Both of these must be modified to work with CURP.           operations with CURP while varying the aggressiveness of
   Since both garbage collection mechanisms assume that             syncs. After introducing CURP, RAMCloud can delay the
retries always come from the same client that made the              sync to backups after responding back to clients; delaying
original request, RIFL must be modified to accommodate              and batching sync to backups makes the server more efficient
retries from witnesses. Firstly, once clients acknowledge           and improves throughput about 4 times. Since RAMCloud



USENIX Association                             16th USENIX Symposium on Networked Systems Design and Implementation                                                   63
Write Throughput (k write per second)
                                    900
                                    800
                                    700
                                    600                                                               Unreplicated
                                                                                                      Async (f = 3)
                                    500                                                               CURP (f = 1)
                                                                                                      CURP (f = 2)
                                    400                                                               CURP (f = 3)
                                    300                                                               Original RAMCloud

                                    200
                                    100
                                                     0
                                                         0           10          20          30            40           50
                                                             Minimum Batch Size (number of writes before starting sync)
                     Figure 15: The aggregate throughput for one server serving 100B RAM-
                     Cloud writes with CURP, as a function of sync batch size. Each client
                     repeatedly issued random writes back to back to a single server. “Original
                     RAMCloud” refers to the base RAMCloud system before adding CURP.
                     “Unreplicated” refers to RAMCloud without any replication. Each
                     datapoint was measured 15 times, and median values are displayed.
                                                 500
                                                                       Original Redis (non-durable)
                                                                       CURP (1 Witness)
                                                 400                   CURP (2 Witnesses)
                                                                       Original Redis (durable)




                              Average Latency (µs)
                                                 300


                                                 200


                                                 100


                                                         0
                                                             0    20      40     60    80      100      120    140    160
                                                                         Write Throughput (k write per second)
                     Figure 16: Observed latency at a specific throughput level for one
                     server serving 100B Redis SET operations with CURP. “Original Redis
                     (durable)” refers to the base Redis without CURP, but configured to
                     invoke fsync before replying to clients. Original Redis processes requests
                     from multiple clients, fsyncs once per eventloop, and replies to all clients.
allows only one outstanding sync, syncs are naturally batched
for around 15 writes even at 1 minimum batch size.
D.2 Redis Latency vs. Throughput
   Figure 16 shows observed latency during the throughput
benchmark. Both CURP and non-durable Redis maintains la-
tency low until it reaches 80% of max throughput. The latency
of durable Redis increases almost linearly due to bathcing.
The original Redis is designed to provide maximum through-
put under high load and natively batches fsyncs; for each
event-loop cycle, Redis iterates through TCP sockets for all
clients and executes all requests from them; after the iteration,
Redis fsyncs once and responds to the clients. This batching
amortizes the cost of fsync, and throughput of durable Redis
approaches that of non-durable Redis as the number of clients
increases. However, this batching adds extra delay before
responding back to clients, so latency increases linearily.




64                                                   16th USENIX Symposium on Networked Systems Design and Implementation    USENIX Association
