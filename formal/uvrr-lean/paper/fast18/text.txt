                   Protocol-Aware Recovery for
                    Consensus-Based Storage
   Ramnatthan Alagappan and Aishwarya Ganesan, University of Wisconsin—Madison;
Eric Lee, University of Texas at Austin; Aws Albarghouthi, University of Wisconsin—Madison;
     Vijay Chidambaram, University of Texas at Austin; Andrea C. Arpaci-Dusseau and
                Remzi H. Arpaci-Dusseau, University of Wisconsin - Madison
              https://www.usenix.org/conference/fast18/presentation/alagappan



              This paper is included in the Proceedings of the
         16th USENIX Conference on File and Storage Technologies.
                       February 12–15, 2018 • Oakland, CA, USA
                                  ISBN 978-1-931971-42-3




                                               Open access to the Proceedings of
                                                the 16th USENIX Conference on
                                                 File and Storage Technologies
                                                    is sponsored by USENIX.
                 Protocol-Aware Recovery for Consensus-Based Storage
             Ramnatthan Alagappan, Aishwarya Ganesan, Eric Lee† , Aws Albarghouthi,
          Vijay Chidambaram† , Andrea C. Arpaci-Dusseau, and Remzi H. Arpaci-Dusseau
                   University of Wisconsin – Madison              † University of Texas at Austin


                       Abstract                                    In this paper, we apply PAR to replicated state ma-
We introduce protocol-aware recovery (PAR), a new ap-           chine (RSM) systems. We focus on RSM systems for
proach that exploits protocol-specific knowledge to cor-        two reasons. First, correctly implementing recovery is
rectly recover from storage faults in distributed sys-          most challenging for RSM systems because of the strong
tems. We demonstrate the efficacy of PAR through                consistency and durability guarantees they provide [58];
the design and implementation of corruption-tolerant            a small misstep in recovery could violate the guaran-
replication (C TRL), a PAR mechanism specific to repli-         tees. Second, the reliability of RSM systems is crucial:
cated state machine (RSM) systems. We experimentally            many systems entrust RSM systems with their critical
show that the C TRL versions of two systems, LogCabin           data [45]. For example, Bigtable, GFS, and other sys-
and ZooKeeper, safely recover from storage faults and           tems [7,26] store their metadata on RSM systems such as
provide high availability, while the unmodified versions        Chubby [16] or ZooKeeper [4]. Hence, protecting RSM
can lose data or become unavailable. We also show that          systems from storage faults such as data corruption will
the C TRL versions have little performance overhead.            improve the reliability of many dependent systems.
                                                                   We first characterize the different approaches to han-
1    Introduction                                               dling storage faults by developing the RSM recovery
Failure recovery using redundancy is central to improved        taxonomy, through experimental and qualitative analy-
reliability of distributed systems [14, 22, 31, 35, 61, 67].    sis of practical systems and methods proposed by prior
Distributed systems recover from node crashes and net-          research (§2). Our analyses show that most approaches
work failures using copies of data and functionality on         employed by currently deployed systems do not use any
several nodes [6,47,55]. Similarly, bad or corrupted data       protocol-level knowledge to perform recovery, leading to
on one node should be recovered from redundant copies.          disastrous outcomes such as data loss and unavailability.
   In a static setting where all nodes always remain               Thus, to improve the resiliency of RSM systems to
reachable and where clients do not actively update data,        storage faults, we design a new protocol-aware recov-
recovering corrupted data from replicas is straightfor-         ery approach that we call corruption-tolerant replication
ward; in such a setting, a node could repair its state by       or C TRL (§3). C TRL constitutes two components: a lo-
simply fetching the data from any other node.                   cal storage layer and a distributed recovery protocol;
   In reality, however, a distributed system is a dynamic       while the storage layer reliably detects faults, the dis-
environment, constantly in a state of flux. In such             tributed protocol recovers faulty data from redundant
settings, orchestrating recovery correctly is surprisingly      copies. Both the components carefully exploit RSM-
hard. As a simple example, consider a quorum-based              specific knowledge to ensure safety (e.g., no data loss)
system, in which a piece of data is corrupted on one node.      and high availability.
When the node tries to recover its data, some nodes may            C TRL applies several novel techniques to achieve
fail and be unreachable, some nodes may have recently           safety and high availability. For example, a crash-
recovered from a failure and so lack the required data or       corruption disentanglement technique in the storage
hold a stale version. If enough care is not exercised, the      layer distinguishes corruptions caused by crashes from
node could “fix” its data from a stale node, overwriting        disk faults; without this technique, safety violations or
the new data, potentially leading to a data loss.               unavailability could result. Next, a global-commitment
   To correctly recover corrupted data from redundant           determination protocol in the distributed recovery sepa-
copies in a distributed system, we propose that a recovery      rates committed items from uncommitted ones; this sep-
approach should be protocol-aware. A protocol-aware             aration is critical: while recovering faulty committed
recovery (PAR) approach is carefully designed based on          items is necessary for safety, discarding uncommitted
how the distributed system performs updates to its repli-       items quickly is crucial for availability. Finally, a novel
cated data, elects the leader, etc. For instance, in the pre-   leader-initiated snapshotting mechanism enables identi-
vious example, a PAR mechanism would realize that a             cal snapshots across nodes to greatly simplify recovery.
faulty node has to query at least R (read quorum) other            We implement C TRL in two storage systems that are
nodes to safely and quickly recover its data.                   based on different consensus algorithms: LogCabin [43]



USENIX Association                                      16th USENIX Conference on File and Storage Technologies         15
(based on Raft [50]) and ZooKeeper [4] (based on               states by executing commands on a state machine (an in-
ZAB [39]) (§4). Through experiments, we show that              memory data structure on each node) [58]. Typically,
C TRL versions provide safety and high availability in the     clients interact with a single node (the leader) to exe-
presence of storage faults, while the original systems re-     cute operations on the state machine. Upon receiving
main unsafe or unavailable in many cases; we also show         a command, the leader durably writes the command to
that C TRL induces minimal performance overhead (§5).          an on-disk log and replicates it to the followers. When
                                                               a majority of nodes have durably persisted the command
2     Background and Motivation                                in their logs, the leader applies the command to its state
                                                               machine and returns the result to the client; at this point,
We first provide background on storage faults and RSM
                                                               the command is committed. The commands in the log
systems. We then present the taxonomy of different ap-
                                                               have to be applied to the state machine in-order. Losing
proaches to handling storage faults in RSM systems.
                                                               or overwriting committed commands violates the safety
2.1    Storage Faults in Distributed Systems                   property of the state machine. The replicated log is kept
Disks and flash devices exhibit a subtle and complex fail-     consistent across nodes by a consensus protocol such as
ure model: a few blocks of data could become inaccessi-        Paxos [41] or Raft [50].
ble or be silently corrupted [8, 9, 32, 59]. Although such        Because the log can grow indefinitely and exhaust disk
storage faults are rare compared to whole-machine fail-        space, periodically, a snapshot of the in-memory state
ures, in large-scale distributed systems, even rare failures   machine is written to disk and the log is garbage col-
become prevalent [60, 62]. Thus, it is critical to reliably    lected. When a node restarts after a crash, it restores
detect and recover from storage faults.                        the system state by reading the latest on-disk snapshot
   Storage faults occur due to several reasons: media er-      and the log. The node also recovers its critical metadata
rors [10], program/read disturbance [60], and bugs in          (e.g., log start index) from a structure called metainfo.
firmware [9], device drivers [66], and file systems [27,       Thus, each node maintains three critical persistent data
28]. Storage faults manifest in two ways: block errors         structures: the log, the snapshots, and the metainfo.
and corruption. Block errors (or latent sector errors)            These persistent data structures could be corrupted
arise when the device internally detects a problem with a      due to storage faults. Practical systems try to safely
block and throws an error upon access. Studies of both         recover the data and remain available under such fail-
flash [33, 60] and hard drives [10, 59] show that block er-    ures [15, 17]. However, as we will show, none of the
rors are common. Corruption could occur due to lost and        current approaches correctly recover from storage faults,
misdirected writes that may not be detected by the de-         motivating the need for a new approach.
vice. Studies [9, 51] and anecdotal evidence [36, 37, 57]
show the prevalence of data corruption in the real world.      2.3    RSM Recovery Taxonomy
   Many local file systems, on encountering a storage          To understand the different possible ways to handling
fault, simply propagate the fault to applications [11, 54,     storage faults in RSM systems, we analyze a broad range
64]. For example, ext4 silently returns corrupted data         of approaches. We perform this analysis by two means:
if the underlying device block is corrupted. In contrast,      first, we analyze practical systems including ZooKeeper,
a few file systems transform an underlying fault into a        LogCabin, etcd [25], and a Paxos-based system [24] us-
different one; for example, btrfs returns an error to appli-   ing a fault-injection framework we developed (§5); sec-
cations if the accessed block is corrupted on the device.      ond, we analyze techniques proposed by prior research
In either case, storage systems built atop local file sys-     or used in proprietary systems [15, 17].
tems should handle corrupted data and storage errors to           Through our analysis, we classify the approaches into
preserve end-to-end data integrity.                            two categories: protocol-oblivious and protocol-aware.
   One way to tackle storage faults is to use RAID-like        The oblivious approaches do not use any protocol-level
storage to maintain multiple copies of data on each node.      knowledge to perform recovery. Upon detecting a
However, many distributed deployments would like to            fault, these approaches take a recovery action locally
use inexpensive disks [22, 31]. Given that the data in         on the faulty node; such actions interact with the dis-
a distributed system is inherently replicated, it is waste-    tributed protocols in unsafe ways, leading to data loss.
ful to store multiple copies on each node. Hence, it is        The protocol-aware approaches use some RSM-specific
important for distributed systems to use the inherent re-      knowledge to recover; however, they do not use this
dundancy to recover from storage faults.                       knowledge correctly, leading to undesirable outcomes.
                                                               Our taxonomy is not complete in that there may be other
2.2    RSM-based Storage Systems                               techniques; however, to the best of our knowledge, we
Our goal is to harden RSM systems to storage faults.           have not observed other approaches apart from those in
In an RSM system, a set of nodes compute identical             our taxonomy.



16    16th USENIX Conference on File and Storage Technologies                                        USENIX Association
 S1   23            23   23              23        23   23
                                                                                                       Safety
                                                                                                       Availability
 S2 1 2 3         1 23 1 3             1 3       1 3 1 3                                               Performance
                                                                                                       No Intervention
 S3 1 2 3         1 23 1 23            1 23      1 2  1 2                                              No extra nodes
 S4 1 2 3              1 23            1 23      1 23   23
                                                                                                       Fast Recovery         (i)
                                                                                                                             (ii)
                                                                                                                             (iii)
                                                                                                                             (iv)
                                                                           Class       Approach        Low Complexity        (v)
 S5 1 2 3                                                                                                √√√√ √
                                                                                                                             (vi)
                       1 23                      1 23 1 3                                              ×
                                                                           Protocol
                                                                                       NoDetection
                                                                                                       √   √ √ na √          E E E E E E
       (i)          (ii)      (iii)      (iv)      (v)        (vi)                     Crash             ×
                                                                                                         √√√× √ na √         UCUCUU
                                                                           Oblivious   Truncate        ×√√ √×√               C L C L L L
Figure 1: Sample Scenarios. The figure shows sample scenarios in
which current approaches fail. Faulty entries are striped. Crashed and                 DeleteRebuild   × √√ ×√×√             C L C L L L
lagging nodes are shown as gray and empty boxes, respectively.                         MarkNonVoting   × ×
                                                                                                       √ √      ×√           UCUCUU
                                                                           Protocol
   To illustrate the problems, we use Figure 1. In all
                                                                                       Reconfigure
                                                                                                       √× √ ×××              UCUCUU
                                                                            Aware      Byzantine FT
                                                                                                       √√×√×√√×√na ×
                                                                                                                   √         UCUUUU
cases, log entries† 1, 2, and 3 are committed; losing these                            C TRL                                 C C C C C C
items will violate safety. Table 1 shows how each ap-
proach behaves in Figure 1’s scenarios. As shown in                          E- Return Corrupted, L- Data Loss, U- Unavailable, C- Correct
the table, all current approaches lead to safety violation               Table 1: Recovery Taxonomy.            The table shows how different
(e.g., data loss), low availability, or both. A recovery                 approaches behave in Figure 1 scenarios. While all approaches are
mechanism that effectively uses redundancy should be                     unsafe or unavailable, C TRL ensures safety and high availability.
safe and available in all cases. Table 1 also compares the
                                                                         (possibly faulty) portions of data and continue operat-
approaches along other axes such as performance, main-
                                                                         ing. The intuition behind Truncate is that if the faulty
tenance overhead (intervention and extra nodes), recov-
                                                                         data is discarded, the node can continue to operate (un-
ery time, and complexity. Although Figure 1 shows only
                                                                         like Crash), improving availability.
faults in the log, the taxonomy applies to other structures
including the snapshots and the metainfo.                                   However, we find that Truncate can cause a safety vi-
                                                                         olation (data loss). Consider the scenario shown in Fig-
NoDetection. The simplest reaction to storage faults is
                                                                         ure 2 in which entry 1 is corrupted on S1 ; S4 , S5 are lag-
none at all: to trust every layer in the storage stack to
                                                                         ging and do not have any entry. Assume S2 is the leader.
work reliably. For example, a few prototype Paxos-based
                                                                         When S1 reads its log, it detects the corruption; however,
systems [24] do not use checksums for their on-disk data;
                                                                         S1 truncates its log, losing the corrupted entry and all
similarly, LogCabin does not protect its snapshots with
                                                                         subsequent entries (Figure 2(ii)). Meanwhile, S2 (leader)
checksums. NoDetection trivially violates safety; cor-
                                                                         and S3 crash. S1 , S4 , and S5 form a majority and elect S1
rupted data can be obliviously served to clients. How-
                                                                         the leader. Now the system does not have any knowledge
ever, deployed systems do use checksums and other in-
                                                                         of committed entries 1, 2, and 3, resulting in a silent data
tegrity strategies for most of their on-disk data.
                                                                         loss. The system also commits new entries x, y, and z in
Crash. A better strategy is to use checksums and han-
                                                                         the place of 1, 2, and 3 (Figure 2(iii)). Finally, when S2
dle I/O errors, and crash the node on detecting a fault.
                                                                         and S3 recover, they follow S1 ’s log (Figure 2(iv)), com-
Crash may seem like a good strategy because it in-
                                                                         pletely removing entries 1, 2, and 3.
tends to prevent any damage that the faulty node may
inflict on the system. Our experiments show that the                        In summary, although the faulty node detects the cor-
Crash approach is common: LogCabin, ZooKeeper, and                       ruption, it truncates its log, losing the data locally. When
etcd crash sometimes when their logs are faulty. Also,                   this node forms a majority along with other nodes that
ZooKeeper crashes when its snapshots are corrupted.                      are lagging, data is silently lost, violating safety. We find
   Although Crash preserves safety, it suffers from se-                  this safety violation in ZooKeeper and LogCabin.
vere unavailability. Given that nodes could be unavail-                     Further, Truncate suffers from inefficient recovery.
able due to other failures, even a single storage fault re-              For instance, in Figure 1(i), S1 truncates its log after a
sults in unavailability, as shown in Figure 1(i). Similarly,             fault, losing entries 1, 2, and 3. Now to fix S1 ’s log,
a single fault even in different portions of data on a ma-               the leader needs to transfer all entries, increasing S1 ’s re-
jority (e.g., Figure 1(v)) renders the system unavailable.               covery time and wasting network bandwidth. ZooKeeper
Note that simply restarting the node does not help; stor-                and LogCabin suffer from this slow recovery problem.
age faults, unlike other faults, could be persistent: the                DeleteRebuild. Another commonly employed action is
node will encounter the same fault and crash again until                 to manually delete all data on the faulty node and restart
manual intervention, which is error-prone and may cause                  the node. Unfortunately, similar to Truncate, DeleteRe-
a data loss. Thus, it is desirable to recover automatically.             build can violate safety; specifically, a node whose data
Truncate. A more sophisticated action is to truncate                     is deleted could form a majority along with the lagging
                                                                         nodes, leading to a silent data loss. Surprisingly, admin-
   † A log entry contains a state-machine command and data.              istrators often use this approach hoping that the faulty



USENIX Association                                               16th USENIX Conference on File and Storage Technologies                     17
  S1 2 3                                           xy z                 xy z            Most approaches do not use any protocol-level knowl-
  S2 1 2 3                  1 23                   1 23                 xy z            edge to recover; for example, Truncate and DeleteRe-
  S3 1 2 3                  1 23                   1 23                 xy z            build take actions locally on the faulty node and so inter-
  S4                                               xy z                 xy z            act with the distributed protocol in unsafe ways, causing
  S5                                               xy z                 xy z            a global data loss. Although some approaches (e.g., Mar-
 (i) S4, S5 lagging; (ii) S1 truncates 1,2,3;    (iii) 1,2,3 lost; (iv) S2, S3 follow   kNonVoting) use some RSM-specific knowledge, they do
    1 faulty at S1         S2, S3 down        overwritten by x,y,z       S1’s log
                                                                                        not do so correctly, causing data loss or unavailability.
Figure 2: Safety Violation Example.          The figure shows the se-                   Thus, to ensure safety and high availability, a recovery
quence of events which exposes a safety violation in Truncate.                          approach should effectively use redundancy by exploit-
node will be “simply fixed” by fetching the data from                                   ing protocol-specific knowledge. Further, it is benefi-
other nodes [63, 65, 73]. DeleteRebuild also suffers from                               cial to avoid other problems such as manual intervention
the slow recovery problem similar to Truncate.                                          and slow recovery. Our protocol-aware approach, C TRL,
MarkNonVoting. In this approach, used by a Paxos-                                       aims to achieve these goals.
based system at Google [17], a faulty node deletes all its
data on a fault and marks itself as a non-voting mem-                                   3     Corruption-Tolerant Replication
ber; the node does not participate in elections until it
                                                                                        Designing a correct recovery mechanism needs a careful
observes one round of consensus and rebuilds its data
                                                                                        understanding of the underlying protocols of the system.
from other nodes. By marking a faulty node as non-
                                                                                        For example, the recovery mechanism should be cog-
voting, safety violations such as the one in Figure 2 are
                                                                                        nizant of how updates are performed on the replicated
avoided. However, MarkNonVoting can sometimes vio-
                                                                                        data and how the leader is elected. We base C TRL’s de-
late safety as noted by prior work [70]. The underlying
                                                                                        sign on the following important protocol-level observa-
reason for unsafety is that a corrupted node deletes all
                                                                                        tions common to most RSM systems.
its state including the promises† given to leaders. Once
                                                                                        Leader-based. A single node acts as the leader; all data
a faulty node has lost its promise given to a new leader,
                                                                                        updates flow only through the leader.
it could accept an entry from an old leader (after observ-
                                                                                        Epochs. RSM systems partition time into logical units
ing a round of consensus on an earlier entry). The new
                                                                                        called epochs. For any given epoch, only one leader is
leader, however, still believes that it has the promise from
                                                                                        guaranteed to exist. Every data item is associated with
the faulty node and so can overwrite the entry, previously
                                                                                        the epoch in which it was appended and its index in the
committed by the old leader.
                                                                                        log. Since the entries could only be proposed by the
   Further, this approach suffers from unavailability. For
                                                                                        leader and only one leader could exist for an epoch, an
example, when only a majority of nodes are alive, a sin-
                                                                                        hepoch, indexi pair uniquely identifies a log entry.
gle fault can cause unavailability because the faulty node
                                                                                        Leader Completeness. A node will not vote for a can-
cannot vote; other nodes cannot now elect a leader.
                                                                                        didate if it has more up-to-date data than the candidate.
Reconfigure. In this approach, a faulty node is removed
                                                                                        Since committed data is present at least in a majority of
and a new node is added. However, to change the con-
                                                                                        nodes and a majority vote is required to win the election,
figuration, a configuration entry needs to be committed
                                                                                        the leader is guaranteed to have all the committed data.
by a majority. Hence, the system remains unavailable in
                                                                                           C TRL exploits these protocol-level attributes common
many cases (for example, when a majority are alive but
                                                                                        to RSM systems to correctly recover from storage faults.
one node’s data is corrupted). Although Reconfigure is
                                                                                        C TRL divides the recovery responsibility between two
not used in practical systems to tackle storage faults, it
                                                                                        components: the local storage layer and the distributed
has been suggested by prior research [15, 44].
                                                                                        recovery protocol; while the storage layer reliably de-
BFT. An extreme approach is to use a Byzantine-fault-                                   tects faulty data on a node, the distributed protocol recov-
tolerant algorithm which should theoretically tolerate                                  ers the data from redundant copies. Both the components
storage faults. However, BFT is expensive to be used in                                 use RSM-specific knowledge to perform their functions.
practical storage systems; specifically, BFT can achieve
                                                                                           In this section, we first describe C TRL’s fault model
only half the throughput of what a crash-tolerant proto-
                                                                                        (§3.1) and safety and availability guarantees (§3.2). We
col can achieve [21]. Moreover, BFT requires 3 f + 1
                                                                                        then describe the local storage layer (§3.3). Finally, we
nodes to tolerate f faults [2], thus remaining unavailable
                                                                                        describe C TRL’s distributed recovery in two parts: first,
in most scenarios in Figure 1.
                                                                                        we show how faulty logs are recovered (§3.4) and then
Taxonomy Summary. None of the current approaches                                        we explain how faulty snapshots are recovered (§3.5).
effectively use redundancy to recover from storage faults.
     † In Paxos, a promise for a proposal numbered p is a guarantee given               3.1    Fault Model
by a follower (acceptor) to the leader (proposer) that it will not accept               Our fault model includes the standard failure assump-
a proposal numbered less than p in the future [41].                                     tions made by crash-tolerant RSM systems: nodes could



18      16th USENIX Conference on File and Storage Technologies                                                               USENIX Association
                    Fault Outcome                       Possible Causes                                                 entry               len
                                                                                                     log
                                                                                   (a) Typical
              corrupted data                 misdirected and lost writes in ext                                          cksum len         data       Update:
Data          inaccessible data              LSE, corruptions in ZFS and btrfs
                                                                                                  e1 e2     ...
                                                                                                                           data = <index, epoch,    write(log, ei)
                                             directory entry corrupted, fsck may     format                                command, ...>             fsync(log)
              missing files/directories


FS Metadata
                                             remove a faulty inode                                entry identifiers
                                             sanity check fails after inode cor-                                                                     Update:
                                                                                   (b) CLSTORE                    ...      e1 e2       ...
              unopenable files/directories                                                        id1 id2
                                             ruption, permission bits corrupted                                                                    pwrite(log, ei)
              files with more or fewer bytes i size field in the inode corrupted                 ei- ith log entry; idi - identifier of entry ei   pwrite(log, idi)
              file system read-only          journal corrupted; fsck not run          format     idi = <index(ei), epoch(ei), offset(ei), cksum>    fsync(log)
              file system unmountable        superblock corrupted; fsck not run                  idi also serves as persist record for ei
Table 2: Storage Fault Model. The table shows storage faults                       Figure 3: Log Format. (a) shows the format and update protocol
included in our model and possible causes that lead to a fault outcome.            of a typical RSM log; (b) shows the same for C LSTORE.
crash at any time and recover later, and nodes could be                            C LSTORE must correctly distinguish crashes from corrup-
unreachable due to network failures [21, 42, 50]. Our                              tions; safety can be violated otherwise. Third, C LSTORE
model adds another realistic failure scenario where per-                           must identify which pieces of data are faulty; only if
sistent data on the individual nodes could be corrupted                            C LSTORE identifies which pieces have been affected, can
or inaccessible. Table 2 shows a summary of our storage                            the distributed protocol recover those pieces.
fault model. Our model includes faults in both user data
and the file-system metadata blocks.                                               3.3.1 Persistent Structures Overview
   User data blocks in the files that implement the sys-                           As we discussed, RSM systems maintain three persis-
tem’s persistent structures could be affected by errors                            tent structures: the log, the snapshots, and the metainfo.
or corruption. A number of (possibly contiguous) data                              C LSTORE uses RSM-specific knowledge of how these
blocks could be faulty as shown by studies [12,59]. Also,                          structures are used and updated, to perform its functions.
a few bits/bytes of a block could be corrupted. Depend-                            For example, C LSTORE detects faults at a different granu-
ing on the local file system in use, corrupted data may be                         larity depending on the RSM data structure: faults in the
returned obliviously or transformed into errors.                                   log are detected at the granularity of individual entries,
   File-system metadata blocks can also be affected by                             while faults in the snapshot are detected at the granular-
faults; for example, the inode of a log file could be cor-                         ity of chunks. Similarly, C LSTORE uses the RSM-specific
rupted. Our fault model considers the following out-                               knowledge that a log entry is uniquely qualified by its
comes that can be caused by file-system metadata faults:                           hepoch, indexi pair to identify faulty log entries.
files/directories may go missing, files/directories may be                         Log. The log is a set of files containing a sequence of
unopenable, a file may appear with fewer or more bytes,                            entries. The format of a typical RSM log is shown in
the file system may be mounted read-only, and in the                               Figure 3(a). The log is updated synchronously in the crit-
worst case, the file system may be unmountable. Some                               ical path; hence, changes made to the log format should
file systems such as ZFS may mask most of the above                                not affect its update performance. C LSTORE uses a mod-
outcomes from applications [72]; however, our model                                ified format as shown in Figure 3(b) which achieves this
includes these faulty outcomes because they could real-                            goal. A corrupted log is recovered at the granularity of
istically occur on other file systems that provide weak                            individual entries.
protection against corruption (e.g., ext2/3/4). Through                            Snapshots. The in-memory state machine is periodi-
fault-injection tests, we have verified that the metadata                          cally written to a snapshot. Since snapshots can be huge,
fault outcomes shown in Table 2 do occur on ext4.                                  C LSTORE splits them into chunks; a faulty snapshot is re-
                                                                                   covered at the granularity of individual chunks.
3.2               Safety and Availability Guarantees                               Metainfo. The metainfo is special in that faulty metainfo
C TRL guarantees that if there exists at least one correct                         cannot be recovered from other nodes. This is because
copy of a committed data item, it will be recovered or the                         the metainfo contains information unique to a node (e.g.,
system will wait for that item to be fixed; committed data                         its current epoch); recovering metainfo obliviously from
will never be lost. In unlikely cases where all copies of                          other nodes could violate safety. C LSTORE uses this
a committed item are faulty, the system will correctly re-                         knowledge correctly and so maintains two copies of the
main unavailable. C TRL also guarantees that the system                            metainfo locally; if one copy is faulty, the other copy is
will make a decision about an uncommitted faulty item                              used. Fortunately, the metainfo is only a few tens of bytes
as early as possible, ensuring high availability.                                  in size and is updated infrequently; therefore, maintain-
                                                                                   ing two copies does not incur significant overheads.
3.3               CTRL Local Storage Layer
To reliably recover, the storage layer (C LSTORE) needs                            3.3.2  Detecting Faulty Data
to satisfy three key requirements. First, C LSTORE must                            C LSTORE uses well-known techniques for detection: in-
be able to reliably detect a storage fault. Second,                                accessible data is detected by catching return codes (e.g.,



USENIX Association                                                         16th USENIX Conference on File and Storage Technologies                              19
EIO) and corrupted data is detected by a checksum mis-          tion and not due to a crash. pi is checksummed and is
match. C LSTORE assumes that if an item and its check-          very small; it can be atomically written and thus cannot
sum agree, then the item is not faulty. In the log, each        be “corrupted” due to a crash. If pi is corrupted in addi-
entry is protected by a checksum; similarly, each chunk         tion to ei , we can conclude that it is a corruption and not
in a snapshot and the entire metainfo are checksummed.          a crash.
   C LSTORE also handles file-system metadata faults.               The above logic works when ei is ordered before pi .
Missing and unopenable files/directories are detected by        However, such ordering requires an (additional) expen-
handling error codes upon open. Log and metainfo                sive fsync in the critical path, affecting log-update per-
files with fewer or more bytes are detected easily be-          formance. For this reason, C LSTORE does not order
cause these files are preallocated and are of a fixed size;     ei before pi ; thus, the append protocol is t1 :write(ei ),
snapshot sizes are stored separately, and C LSTORE cross-       t2 :write(pi ), t3 :fsync().† Given this update sequence, as-
checks the stored size with the file-system reported size       sume a checksum mismatch occurs for ei . If pi is not
to detect discrepancies. A read-only/unmountable file           present, C LSTORE can conclude that it is a crash (before
system is equivalent to a missing data directory. In most       t2 ) and discard ei . Contrarily, if pi is present, there are
cases of file-system metadata faults, C LSTORE crashes          two possibilities: either ei could be affected by a corrup-
the nodes. Crashing reliably on a metadata fault pre-           tion after t3 or a crash could have occurred between t2
serves safety but compromises on availability. However,         and t3 in which pi hit the disk while ei was only partially
we believe this is an acceptable behavior because there         written. The second case is possible because file systems
are far more data blocks than metadata blocks; therefore,       can reorder writes between two fsync operations and ei
the probability of faults is significantly less for metadata    could span multiple sectors [3, 19, 52, 53]. C LSTORE can
than data blocks.                                               still conclude that it is a corruption if ei+1 or pi+1 is
                                                                present. However, if ei is the last entry, then we cannot
3.3.3 Disentangling Crashes and Corruption in Log               determine whether it was a crash or a corruption.∗
An interesting challenge arises when detecting corrup-              The inability to disentangle the last entry when its
tions in the log. A checksum mismatch for a log entry           persist record is present is not specific to C LSTORE, but
could occur due to two different situations. First, the         rather a fundamental limitation in log-based systems. For
system could have crashed in the middle of an update; in        instance, in ext4’s journal async commit mode (where a
this case, the entry would be partially written and hence       transaction is not ordered before its commit record), a
cause a mismatch. Second, the entry could be safely per-        corrupted last transaction is assumed to be caused due to
sisted but corrupted at a later point. Most log-based sys-      a crash, possibly losing data [38,69]. Even if crashes and
tems conflate these two cases: they treat a mismatch as         corruptions can be disentangled, there is little a single-
a crash [30]. On a mismatch, they discard the corrupted         machine system can do to recover the corrupted data.
entry and all subsequent entries, losing the data. Discard-     However, in a distributed system, redundant copies can
ing entries due to such conflation introduces the possibil-     be used to recover. Thus, when the last entry cannot
ity of a global data loss (as shown earlier in Figure 2).       be disentangled, C LSTORE safely marks the entry as cor-
   Note that if the mismatch were really due to a crash,        rupted and leaves it to the distributed recovery to fix or
it is safe to discard the partially written entry. It is safe   discard the entry based on the global commitment.
because the node would not have acknowledged to any                 The entanglement problem does not arise for snap-
external entity that it has written the entry. However, if      shots or metainfo. These files are first written to a tempo-
an entry is corrupted, the entry cannot be simply dis-          rary file and then atomically renamed. If a crash happens
carded since it could be globally committed. Further, if a      before the rename, the partially written temporary file is
mismatch can be correctly attributed to a crash, the faulty     discarded. Thus, the system will never see a corrupted
entry can be quickly discarded locally, avoiding the dis-       snapshot or metainfo due to a crash; if these structures
tributed recovery. Hence, it is important for the local         are corrupted, it is because of a storage corruption.
storage layer to distinguish the two cases.
                                                                3.3.4 Identifying Faulty Data
   To denote the completion of an operation, many sys-
                                                                Once a faulty item is detected, it has to be identified; only
tems write a commit record [13, 18]. Similarly, C LSTORE
                                                                if C LSTORE can identify a faulty item, the distributed
writes a persist record, pi , after writing an entry ei . For
                                                                layer can recover the item. For this purpose, C LSTORE
now, assume that ei is ordered before pi , i.e., the se-
                                                                redundantly stores an identifier of an item apart from the
quence of steps to append an entry ei is write(ei ), fsync(),
                                                                item itself; duplicating only the identifier instead of the
write(pi ), fsync(). On a checksum mismatch for ei , if pi
                                                                whole item obviates the (2×) storage and performance
is not present, we can conclude that the system crashed
during the update. Conversely, if pi is present, we can           † The final fsync is required for durability.

conclude that the mismatch was caused due to a corrup-            ∗ The proof of this claim is available [1].




20   16th USENIX Conference on File and Storage Technologies                                                      USENIX Association
overhead. However, storing the identifier near the item is       with a different epoch. This situation is possible because
less useful; a misdirected write can corrupt both the item       S5 could have been the leader for epoch 2 and crashed
and its identifier [9,10]. Hence, identifiers are physically     immediately after appending an entry. As discussed ear-
separated from the items they identify.                          lier, an entry is uniquely identified by its hepoch, indexi;
    The hepoch, indexi pair serves as the identifier for a       thus, when querying for faulty entries, a node needs to
log entry and is stored separately at the head of the log,       specify the epoch of the entry in addition to its index.
as shown in Figure 3(b). The offset of an entry is also          Thus, S5 informs the leader that its entry hepoch:2, in-
stored as part of the identifier to enable traversal of subse-   dex:3i is faulty. However, S1 does not have such an entry
quent entries on a fault. The identifier of a log entry also     in its log. If the leader does not have an entry that the fol-
conveniently serves as its persist record. Similarly, for        lower has, then the entry must be an uncommitted entry
a snapshot chunk, the hsnap-index, chunk#i pair serves           because the leader is guaranteed to have all committed
as the identifier; the snap-index and the snapshot size are      data; thus, the leader instructs S5 to truncate the faulty
stored in a separate file than the snapshot file. The iden-      entry and also replicates the correct entry.
tifiers have a nominal storage overhead (32 bytes for log           Although the naive approach guarantees safety, it has
entries and 12 bytes for snapshots), can be atomically           availability problems. The system will be unavailable
written, and are also protected by a checksum.                   in cases such as the ones shown in (b): a leader cannot
    It is highly unlikely an item and its identifier will both   be elected because the logs of the alive nodes are either
be faulty since they are physically separated [9, 10, 12,        faulty or lagging. Note that even a single storage fault
59]. In such unlikely and unfortunate cases, C LSTORE            can cause an unavailability as shown in (b)(i). It is possi-
crashes the node to preserve safety. Table 3 (second col-        ble for a carefully designed recovery protocol to provide
umn) summarizes C LSTORE’s key techniques.                       better availability in these cases. Specifically, since at
                                                                 least one intact copy of all committed entries exists, it is
3.4    CTRL Distributed Log Recovery                             possible to collectively reconstruct the log.
The local storage layer detects faulty data items and
passes on their identifiers to the distributed recovery
                                                                 3.4.1   Removing the Restriction Safely
layer. We now describe how the distributed layer recov-
ers the identified faulty items from redundant copies us-        To recover from scenarios such as those in Figure 4(b),
ing RSM-specific knowledge. We first describe how log            we remove the additional constraint on the election.
entries are recovered and subsequently describe snapshot         Specifically, any node that has a more up-to-date log can
recovery. As we discussed, metainfo files are recovered          now be elected the leader even if it has faulty entries.
locally and so we do not discuss them any further. We            This relaxation improves availability; however, two key
use Figure 4 to illustrate how log recovery works.               questions arise: first, when can the faulty leader proceed
Naive Approach: Leader Restriction. RSM systems                  to accept new commands? second, and more importantly,
do not allow a node with an incomplete log to become             is it safe to elect a faulty node as the leader?
the leader. A naive approach to recovering from storage             To accept a new command, the leader has to append
faults could be to impose an additional constraint on the        the command to its log, replicate it, and apply it to the
election: a node cannot be elected the leader if its log         state machine. However, before applying the new com-
contains a faulty entry. The intuition behind the naive          mand, all previous commands must be applied. Specif-
approach is as follows: since the leader is guaranteed to        ically, faulty commands cannot be skipped and later ap-
have all committed data and our new restriction ensures          plied when they are fixed; such out-of-order application
that the leader is not faulty, faulty log entries on other       would violate safety. Hence, it is required for the leader
nodes could be fixed using the corresponding entries on          to fix its faulty entries before it can accept new com-
the leader. Cases (a)(i) and (a)(ii) in Figure 4 show sce-       mands. Thus, for improved availability, the leader needs
narios where the naive approach could elect a leader. In         to fix its faulty entries as early as possible.
(a)(i), only S1 can become the leader because other nodes           The crucial part of the recovery to ensure safety is to
are either lagging or have at least one faulty entry. As-        fix the leader’s log using the redundant copies on the fol-
sume S1 is the leader also in case (a)(ii).                      lowers. In simple cases such as (b)(i) and (b)(ii), the
Fixing Followers’ Logs. When the leader has no faulty            leader S1 could fix its faulty entry hepoch:1, index:1i us-
entries, fixing the followers is straightforward. For ex-        ing the correct entries from the followers and proceed
ample, in case (a)(i), the followers inform S1 of their          to normal operation. However, in several scenarios, the
faulty entries; S1 then supplies the correct entries. How-       leader cannot immediately recover its faulty entries; for
ever, sometimes the leader might not have any knowl-             example, none of the reachable followers might have any
edge of an entry that a follower is querying for. For in-        knowledge of the entry to be recovered or the entry to be
stance, in case (a)(ii), S5 has a faulty entry at index 3 but    recovered could also be faulty on the followers.



USENIX Association                                       16th USENIX Conference on File and Storage Technologies            21
             1 2 3       1   2 3       1 2 3       1 2 3        1 2 3       1 2 3        1 2 3       1 2 3       1 2 3         Legend
                     L             L           L            L           L            L           L           L
       S1L a b c     a b c3             bc     bc      ab        ab                      ab      ab      ab                     correct entry,
       S2 a b                                                                                                            d
                     a b c3            ab    ab        ab        abc                     ab      ab      ab                    data:d, epoch: 1
       S3 a b        a b c3            ab    ab        ab        ab                      ab      ab      ab                  faulty entry,
       S4 b c        a b c3            abc     bc      ab        ab                      abc ab          ab                     epoch: 1
       S5 a c        a b 2             abc  a c        ab        ab                      abc ab          abc                 correct  entry,
                                                                                                                         de
           (a)(i)     (a)(ii)       (b)(i)  (b)(ii)    (c)(i)    (c)(ii)                 (d)(i) (d)(ii) (d)(iii)            data: d, epoch: e
          Leader restriction       Leader restriction  Discard faulty                    Waiting to determine             e     faulty entry,
                 works               unavailability uncommitted entries                       commitment                          epoch: e

     Figure 4: Distributed Log Recovery. The figure shows how C TRL’s log recovery operates. All entries are appended in epoch 1 unless explicitly
     mentioned. For entries appended in other epochs, the epoch number is shown in the superscript. Entries shown as striped boxes are faulty. A gray
     box around a node denotes that it is down or extremely slow. The leader is marked with L on the left. Log indexes are shown at the top.
     3.4.2     Determining Commitment                                           mulating three (a majority out of five) negative acknowl-
     The main insight to fix the leader’s faulty log safely and                 edgments; hence, the leader can conclude that the entry
     quickly is to distinguish uncommitted entries from possi-                  is uncommitted, discard it, and continue to normal oper-
     bly committed ones; while recovering the committed en-                     ation. In (d)(iii), S4 would respond that it has the entry
ab   tries is necessary for safety, uncommitted entries can be                  but is faulty in its log too. In this case, the leader has to
     safely discarded. Further, discarding uncommitted faulty                   wait for the response from S5 to determine commitment.
     entries immediately is crucial for availability. For in-                   In the unfortunate and unlikely case where all copies of
     stance, in case (c)(i), the faulty entry on S1 cannot be                   an entry are faulty, the system will remain unavailable.
     fixed since there are no copies of it; waiting to fix that
     entry results in indefinite unavailability. Sometimes, an                  3.4.3 The Complete Log Recovery Protocol
     entry could be partially replicated but remain uncommit-                   We now assemble the pieces of the log recovery protocol.
     ted; for example, in case (c)(ii), the faulty entry on S1 is               First, fixing faulty followers is straightforward; the com-
     partially replicated but is not committed. Although there                  mitted faulty entries on the followers can be eventually
     is a possibility of recovering this entry from the other                   fixed by the leader because the leader is guaranteed to
     node (S2 ), this is not necessary for safety; it is completely             have all committed data. Faulty entries on followers that
     safe for the leader to discard this uncommitted entry.                     the leader does not know about are uncommitted; hence,
        To determine the commitment of a faulty entry, the                      the leader instructs the followers to discard such entries.
     leader queries the followers. If a majority of the follow-                    The main challenge is thus fixing the leader’s log. The
     ers respond that they do not have the entry (negative ac-                  leader queries the followers to recover its entry hepoch:e,
     knowledgment), then the leader concludes that the entry                    index:ii. Three types of responses are possible:
     is uncommitted. In this case, the leader safely discards                   Response 1: have – a follower could respond that it has
     that and all subsequent entries; it is safe to discard the                 the entry hepoch:e, index:ii and is not faulty in its log.
     subsequent entries because entries are committed in or-                    Response 2: dontHave – a follower could respond that it
     der. Conversely, if the entry were committed, at least                     does not have the entry hepoch:e, index:ii.
     one node in this majority would have that entry and in-                    Response 3: haveFaulty – a follower could respond that
     form the leader of it; in this case, the leader can fix its                it has hepoch:e, index:ii but is faulty in its log too.
     faulty entry using that response.                                             Once the leader collects these responses from the fol-
     Waiting to Determine Commitment. Sometimes, it                             lowers, it takes the following possible actions:
     may be impossible for the leader to quickly determine                      Case 1: if it gets a have response from at least one fol-
     commitment. For instance, consider the cases in Fig-                       lower, it fixes the entry in its log.
     ure 4(d) in which S4 and S5 are down or slow. S1 queries                   Case 2: if it gets a dontHave response from a majority
     the followers to recover its entry hepoch:1, index:3i. S2                  of followers, it confirms that the entry is uncommitted,
     and S3 respond that they do not have such an entry (neg-                   discards that entry and all subsequent entries.
     ative acknowledgment). S4 and S5 do not respond be-                        Case 3: if it gets a haveFaulty response from a follower,
     cause they are down or slow. The leader, in this case,                     it waits for either Case 1 or Case 2 to happen.
     has to wait for either S4 or S5 to respond; discarding the                    Case 1 and Case 2 can happen in any order; both order-
     entry without waiting for S4 or S5 could violate safety.                   ings are safe. Specifically, if the leader decides to discard
     However, once S4 or S5 responds, the leader will make a                    the faulty entry (after collecting a majority dontHave re-
     decision immediately. In (d)(i), S4 or S5 would respond                    sponses), it is safe since the entry was uncommitted any-
     with the correct entry, fixing the leader. In (d)(ii), S4 or               ways. Conversely, there is no harm in accepting a correct
     S5 would respond that it does not have the entry, accu-                    entry (at least one have response) and replicating it. The



     22      16th USENIX Conference on File and Storage Technologies                                                         USENIX Association
first to happen out of these two cases will take prece-                           Local Storage             Distributed Recovery
dence over the other.                                             Log    granularity: entry;              global-commitment de-
                                                                         identifier:hepoch, indexi;       termination to fix leader,
   The leader proceeds to normal operation only after its
                                                                         crash-corruption disentanglement leader fixes followers
faulty data is discarded or recovered. However, C TRL
                                                               Snapshot granularity: chunk;                leader-initiated identical
discards uncommitted data as early as possible and min-                 identifier:hsnap-index, chunk#i;   snapshots,
imizes the recovery latency by recovering faulty data at                no entanglement                    chunk-based recovery
a fine granularity (as we show in §5.2), ensuring that the     Metainfo granularity: file;                 none (only internal re-
leader proceeds to normal operation quickly.                            identifier: n/a;                   dundancy)
                                                                        no entanglement
   The leader could crash or be partitioned while recov-
ering its log. On a leader failure, the followers will elect
                                                               Table 3: Techniques Summary.          The table shows a summary of
a new leader and make progress. The partial repair done
                                                               techniques employed by C TRL’s storage layer and distributed recovery.
by the failed leader is harmless: it could have either fixed
committed faulty entries or discarded uncommitted ones,        mance. For example, one naive way to realize identi-
both of which are safe.                                        cal snapshots is for the leader to produce the snapshot,
3.5    CTRL Distributed Snapshot Recovery                      insert it into the log as yet another entry, and repli-
Because the logs can grow indefinitely, periodically,          cate it. However, such an approach will affect update
the in-memory state machine is written to disk and the         performance since the snapshot could be huge and all
logs are garbage collected. Current systems including          client commands must wait while the snapshot com-
ZooKeeper and LogCabin do not handle faulty snapshots          mits [49]. Moreover, transferring the snapshot to the fol-
correctly (§2.3): they either crash or load corrupted snap-    lowers wastes network bandwidth.
shots obliviously. C TRL aims to recover faulty snapshots         C TRL takes a different approach to identical snapshots
from redundant copies. Snapshot recovery is different          that preserves common-case performance. The leader
from log recovery in that all data in a snapshot is com-       initiates the snapshot procedure by first deciding the in-
mitted and already applied to the state machine; hence,        dex at which a snapshot will be taken and informing the
faulty snapshots cannot be discarded in any case (unlike       followers of the index. Once a majority agree on the in-
uncommitted log entries which can be discarded safely).        dex, all nodes independently take a snapshot at the index.
                                                               When the leader learns that a majority (including itself)
3.5.1 Leader-Initiated Identical Snapshots                     have taken a snapshot at an index i, it garbage collects its
Current systems [43] have two properties with respect to       log up to i and instructs the followers to do the same.
snapshots. First, they allow new commands to be applied           C TRL implements the above procedure using the log.
to the state machine while a snapshot is in progress. Sec-     When the leader decides to take a snapshot, it inserts
ond, they take index-consistent snapshots: a snapshot Si       a special marker called snap into the log. When the
represents the state machine in which log entries exactly      snap marker commits, and thus when a node applies the
up to i have been applied. One of the mechanisms used          marker to the state machine, it takes a snapshot (i.e., the
in current systems to realize the above two properties is      snapshot corresponds to the state where commands ex-
to take snapshots in a fork-ed child process; while the        actly up to the marker have been applied). Within each
child can write an index-consistent image to the disk, the     node, we reuse the same mechanism used by the original
parent can keep applying new commands to its copy of           system (e.g., a fork-ed child) to allow new commands
the state machine. C TRL should enable snapshot recovery       to be applied while a snapshot is in progress. Notice
while preserving the above two properties.                     that the snapshot operation happens independently on all
   In current systems, every node runs the snapshot pro-       nodes but the operation will produce identical snapshots
cedure independently, taking snapshots at different log        because the marker will be seen at the same log index by
indexes. Because the snapshots are taken at different in-      all nodes when it is committed. When the leader learns
dexes, snapshot recovery can be complex: a faulty snap-        that a majority of nodes (including itself) have taken a
shot on one node cannot be simply fetched from other           snapshot at an index i, it appends another marker called
nodes. Further, snapshots cannot be recovered at the           gc for i; when the gc marker is committed and applied,
granularity of chunks because they will be byte-wise           the nodes garbage collect their log entries up to i.
non-identical; entire snapshots have to be transferred
across nodes, slowing down recovery.                           3.5.2 Recovering Snapshot Chunks
   This complexity can be significantly alleviated if the      With the identical-snapshot mechanism, snapshot recov-
nodes take the snapshot at the same index; identical snap-     ery becomes easier. Once a faulty snapshot is detected,
shots also enable chunk-based recovery.                        the local storage layer provides the distributed protocol
   However, coordinating a snapshot operation across           the snapshot index and the chunk that is faulty. The dis-
nodes can, in general, affect the common-case perfor-          tributed protocol recovers the faulty chunk from other



USENIX Association                                     16th USENIX Conference on File and Storage Technologies                    23
                                                                  Original                              CTRL                                                                                               Outcomes                                                                                                     Outcomes



                                       Total Test Cases                                                                                                                      Total Test Cases                                                                                          Total Test Cases
                                                                      Outcomes                         Outcomes                                                                                        Original   CTRL                                                                                              Original    CTRL



                                                                      Unavailable                      Unavailable
                                                                                                                                                                                                                                  Unavailable
                                                                                                                                                                                                Unavailable                                                                                               Unavailable
                                                                                                                                                                                                                                                                                                                                             Unavailable




ZooKeeper LogCabin System
                                                                                             Correct                          Correct                                                                                   Correct                 Correct                                                                            Correct                 Correct




                                                                                                                                        ZooKeeper LogCabin System                                                                                         ZooKeeper LogCabin System
                                                                                    Unsafe                           Unsafe                                                                                    Unsafe             Unsafe                                                                                  Unsafe
                                                          Original
                                                                                                                                                                                                                                                                                                                                             Unsafe
                            Recovery                                                                                                                                Experi-
                                                          Approach
                            Scenario                                                                                                                                 ment
                                          truncate                     0 2355 46 0 0 2401                                                                           Corrupt-
                            Possible 2401                                                                                                                                    5000 738 793 3469 0 0 5000
                                            crash                    2355 0 46 0 0 2401                                                                              ions
                                          truncate                                                                                                                                                                                                                                    5000 4194 141 665 0 0 5000
                              Not                                      0 1695 0 1695 0 0
                                     1695                                                                                                                           Errors 5000 2497 0 2503 0 0 5000
                            possible        crash                    1695 0 0 1695 0 0
                                          truncate                     0 2355 46 0 0 2401                                                                           Corrupt-
                            Possible 2401                                                                                                                                    5000 807 656 3537 0 0 5000
                                            crash                    2355 0 46 0 0 2401                                                                              ions                                                                                                             5000 1306 1806 1888 0 0 5000
                              Not         truncate                     0 1695 0 1695 0 0
                                     1695                                                                                                                           Errors 5000 2469 0 2531 0 0 5000
                            possible        crash                    1695 0 0 1695 0 0

                                                          (a) Targeted Corruptions                                                                     (b) Random Block Corruptions and Errors (c) Corruptions with Lagging Nodes
Table 4: Log Recovery. (a) shows results for targeted corruptions; we trigger two policies (truncate and crash) in the original systems. (b)
shows results for random block corruptions and errors. (c) shows results for random corruptions with crashed and lagging nodes.

nodes. First, the leader recovers its faulty chunks from                                                                                                                      log is a set of files, each of a fixed size and preallocated.
the followers and then fixes the faulty snapshots on fol-                                                                                                                     The header of each file is reserved for the log-entry iden-
lowers. Three cases arise during snapshot recovery.                                                                                                                           tifiers. The size of the reserved header is proportional to
   First, the log entries for a faulty snapshot may not be                                                                                                                    the file size. C LSTORE ensures that a log entry and its
garbage collected yet; in this case, the snapshot is recov-                                                                                                                   identifier are at least a few megabytes physically apart.
ered locally from the log (after fixing the log if needed).                                                                                                                   Both systems batch many log entries to improve update
   Second, if the log is garbage collected, then a faulty                                                                                                                     performance. With batching, C LSTORE performs crash-
snapshot has to be recovered from other nodes. However,                                                                                                                       corruption disentanglement as follows: the first faulty
if the log entries for a snapshot are garbage collected,                                                                                                                      entry without an identifier and its subsequent entries are
then at least a majority of the nodes must have taken the                                                                                                                     discarded; faulty entries preceding that point are marked
same snapshot. This is true because the gc marker is                                                                                                                          as corrupted and passed on to the distributed layer.
inserted only after a majority of nodes have taken the                                                                                                                            In both systems, the state machine is a data tree. We
snapshot. Thus, faulty garbage-collected snapshots are                                                                                                                        modified both the systems to take index-consistent iden-
recovered from those redundant copies.                                                                                                                                        tical snapshots: when a snap marker is applied, the state
   Third, sometimes, the leader may not know a snapshot                                                                                                                       machine (i.e., the tree) is serialized to the disk. The snap-
that a follower is querying for (for example, if a follower                                                                                                                   index and snapshot size are stored separately. C LSTORE
took a snapshot and went offline for a long time and the                                                                                                                      uses a chunk size of 4K, enabling fine-grained recovery.
leader replaced that snapshot with an advanced one); in                                                                                                                           In LogCabin, the metainfo contains the currentTerm
this case, the leader supplies the full advanced snapshot.                                                                                                                    and votedFor structures. Similarly, in ZooKeeper,
                                                                                                                                                                              structures such as acceptedEpoch and currentEpoch
3.6                            CTRL Summary                                                                                                                                   constitute the metainfo. C LSTORE stores redundant
The storage layer detects and identifies faulty data. Atop                                                                                                                    copies of metainfo and protects them using checksums.
the storage layer, the distributed protocol recovers the                                                                                                                          Log entries, snapshot chunks, and metainfo are pro-
faulty items from redundant copies. Both the layers ex-                                                                                                                       tected by a CRC32 checksum. C LSTORE detects inac-
ploit RSM-specific knowledge to correctly perform their                                                                                                                       cessible data items by catching errors (EIO); it then pop-
functions. A summary of C TRL’s local storage and dis-                                                                                                                        ulates the item’s in-memory buffer with zeros, causing
tributed recovery techniques is shown in Table 3.                                                                                                                             a checksum mismatch. Thus, C LSTORE deals with both
                                                                                                                                                                              corruptions and errors as checksum mismatches.
4                             Implementation
We implement C TRL in two different RSM systems, Log-                                                                                                                         4.2                             Distributed Recovery
Cabin (v1.0) and ZooKeeper (v3.4.8); while LogCabin is                                                                                                                        LogCabin. In Raft, terms are equivalent to epochs.
based on Raft, ZooKeeper is based on ZAB. Implement-                                                                                                                          Thus, a log entry is uniquely identified by its hterm, in-
ing C TRL’s storage layer and distributed recovery took                                                                                                                       dexi pair. To fix the followers, we modified the Append-
only a moderate developer effort; C TRL adds about 1500                                                                                                                       Entries RPC used by the leader to replicate entries [50].
lines of code to each of the base systems.                                                                                                                                    The followers inform the leader of their faulty log en-
                                                                                                                                                                              tries and snapshot chunks in the responses of this RPC;
4.1                            Local Storage Layer                                                                                                                            the leader sends the correct entries and chunks in a sub-
We implemented C LSTORE by modifying the storage en-                                                                                                                          sequent RPC. A follower starts applying commands to
gines of LogCabin and ZooKeeper. In both systems, the                                                                                                                         its state machine once its faulty data is fixed. To fix the



24                            16th USENIX Conference on File and Storage Technologies                                                                                                                                                                                                                                   USENIX Association
leader, we added a new RPC which the leader issues to                                                              Outcomes                                                           Outcomes



                                                                                        Total Test Cases                                                   Total Test Cases
                                                                                                               Original  CTRL                                                     Original  CTRL
the followers. The leader does not proceed to normal op-
                                                                                                                                  Correct

                                                                                                           Unavailable                                                        Unavailable
                                                                                                                                                                                                               Unavailable
eration until its faulty data is fixed. After a configurable
                                                                                                                                                Correct                                              Correct
                                                                                                                                                                                                               Unsafe
                                                                                                                         Unsafe                                                             Unsafe
                                                                  Zoo-
recovery timeout, the leader steps down if it is unable to                                                                        Unavailable

                                                                        Log-
recover its faulty data (for example, due to a partition),                                                                        Unsafe                                                                       Correct


                                                                              System
allowing other nodes to become the leader. Several en-
tries and chunks are batched in a single request/response,                             1000 297 257 446 0 0 1000                                          1000 405 36 559 434 0 566


                                                                 Keeper Cabin
avoiding multiple round trips.
ZooKeeper. In ZAB, the epoch and index are packed                                      1000 417 200 383 0 0 1000                                          1000 329 192 479 502 0 498
into the zxid which uniquely identifies a log entry [5].
Followers discover and connect to the leader in Phase 1.                               (a) Snapshot Recovery                                               (b) FS Metadata Faults
We modified Phase 1 to send information about the fol-
lowers’ faulty data. The followers are synchronized with         Table 5: Snapshot and FS Metadata Faults. (a) and (b) show how
the leader in Phase 2. We modified Phase 2 so that the           C TRL recovers from snapshot and FS metadata faults, respectively.

leader sends the correct data to the followers. The leader       on detection). For example, while ZooKeeper truncates
waits to hear from a majority during Phase 1 after which         when the tail of a transaction is corrupted, it crashes the
it sends a newEpoch message; we modified this message            node if the transaction header is corrupted. C TRL always
to send information about the leader’s faulty data. The          recovers the corrupted data from other replicas.
leader does not proceed to Phase 2 until its data is fixed.         Table 4(a) shows the results. When recovery is possi-
                                                                 ble, the original systems recover only in 46/2401 cases.
5     Evaluation                                                 In those 46 cases, no node or only one node is corrupted.
We evaluate the correctness and performance of C TRL             In the remaining 2355 cases, the original systems are
versions of LogCabin and ZooKeeper. We conducted our             either unsafe (for truncate) or unavailable (for crash).
performance experiments on a three-node cluster on a 1-          In contrast, C TRL correctly recovers in all 2401 cases.
Gb network; each node is a 40-core Intel Xeon CPU E5-            When a recovery is not possible (all copies corrupted),
2660 machine with 128 GB memory running Linux 3.13,              the original systems are either unsafe or unavailable in
with a 500-GB SSD and a 1-TB HDD managed by ext4.                all cases. C TRL, by design, correctly remains unavailable
                                                                 since continuing would violate safety.
5.1    Correctness                                               Random Block Corruptions and Errors. We initialize
To verify C TRL’s safety and availability guarantees, we         the cluster by replicating a few entries to all nodes. We
built a fault-injection framework that can inject stor-          first choose a random set of nodes. In each such node, we
age faults (targeted corruptions and random block cor-           then corrupt a randomly selected file-system block (from
ruptions and errors). The framework can also inject              the files implementing the log). We repeat this process,
crashes. By injecting crashes at different points in time,       producing 5000 test cases. We similarly inject block er-
the framework simulates lagging nodes. After injecting           rors. Since we inject a fault into a block, several entries
faults, we issue reads from clients to determine whether         and their checksums within the block will be faulty.
the target system remains available and preserves safety.           Table 4(b) shows the results. For block corruptions,
   We first exercise different log-recovery scenarios.           original LogCabin is unsafe or unavailable in about
Then, we test snapshot recovery, and finally file-system         30% ((738 + 793)/5000) of cases. Similarly, original
metadata fault recovery.                                         ZooKeeper is incorrect in about 30% of cases. On a
5.1.1 Log Recovery                                               block error, original LogCabin and ZooKeeper simply
Targeted Corruptions. We initialize the cluster with             crash the node, leading to unavailability in about 50% of
four log entries, replicated to all three nodes. We ex-          cases. In contrast, C TRL correctly recovers in all cases.
ercise all combinations of entry corruptions across the          Faults with Crashed and Lagging Nodes. In the previ-
three nodes ((24 )3 = 4096 combinations). Out of the             ous experiments, all entries were committed and present
4096 cases, a correct recovery is possible in 2401 cases         on all nodes. In this experiment, we inject crashes at
(at least one non-faulty copy of each entry exists). In the      different points on a random set of nodes while insert-
remaining 1695 cases, recovery is not possible because           ing entries. Thus, in the resultant log states, nodes could
one or more entries are corrupted on all the nodes. We           be lagging, entries could be uncommitted, and have dif-
inject targeted corruptions into two different sets of on-       ferent epochs on different nodes for the same log index.
disk structures. In the first set, on a corruption, the origi-   hS1 : [a1 , , ], S2 : [b2 , c3 , ], S3 : [b2 , , ]i is an example
nal systems invoke the truncate action (i.e., they truncate      state where S1 appends a at index 1 in epoch 1 (shown
faulty data and continue). In the second set, the origi-         in superscript) and crashes, S2 appends b at index 1 in
nal systems invoke the crash action (i.e., node crashes          epoch 2, replicates to S3 , then S2 , S3 crash and recover,



USENIX Association                                       16th USENIX Conference on File and Storage Technologies                                                                                                      25
                                                                                                                                                                                                               Throughput(Kops/sec)
                                                                                                                                                                      Original        CTRL




                                                                                                                                         Throughput(Kops/sec)
                                                                                                                                                                                                                                                                             0.96
                             Original          CTRL                                               Original       CTRL




Throughput(ops/sec)                                                   Throughput(ops/sec)
                                                                                                                                  0.92
                                                                                                                                                                                                                                          Original         CTRL
                                                                                                                                                                                                0.97    0.96
                      500
                                                         0.9    0.9                                                                                                                     0.99
                                                                                                                                                            4
                                                                                     1000
                                                                                                                                                                                                                              15
                                                                                                                                                                                                                                                                     0.97
                      400
                                                                                            800                                                             3                  0.99
                      300                        0.85                                                                     0.89
                                                                                                                                                                       0.98                                                                                  0.97
                                                                                            600                                                                                                                               10
                                        0.83
                                                                                                                                                            2
                      200
                              0.82                                                                                0.89                                                                                                                              0.98
                                                                                                          0.88
                                                                                            400
                      100                                                                   200   0.84                                                      1                                                                         5    1.04
                       0                                                                     0                                                              0                                                                         0
                              2      4          8       16     32                                 2      4        8      16      32                                   2        4       8       16      32                                 2         4       8       16      32
                                  Number of clients                                                   Number of clients                                                   Number of clients                                                   Number of clients

                        (a) LogCabin: Write (HDD)                                           (b) ZooKeeper: Write (HDD)                                          (c) LogCabin: Write (SSD)                                             (d) ZooKeeper: Write (SSD)
Figure 5: Common-Case Write Performance. (a) and (b) show the write throughput in original and C TRL versions of LogCabin and ZooKeeper
on an HDD. (c) and (d) show the same for SSD. The number on top of each bar shows the performance of C TRL normalized to that of original.
S2 appends c in epoch 3 and crashes. From each such                                                                                                             node if it is unable to locally construct the data from the
state, we corrupt different entries, generating 5000 test                                                                                                       snapshot and the log, leading to unavailability; unsafety
cases. For example, from the above state, we corrupt a                                                                                                          results because a faulty log is truncated in some cases.
on S1 and b, c on S2 . If S2 is elected the leader, S2 needs
to fix b from S3 (since b is committed), discard c (c is                                                                                                        5.1.3 File-system Metadata Faults
uncommitted and cannot be recovered), and also instruct                                                                                                         To test how C TRL recovers from file-system metadata
S1 to discard a (a is uncommitted) and replicate correct                                                                                                        faults, we corrupt file-system metadata structures (such
entry b. As shown in Table 4(c), C TRL correctly recov-                                                                                                         as inodes and directory blocks) resulting in unopenable
ers from all such cases, while the original versions are                                                                                                        files, missing files, and files with fewer or more bytes.
unsafe or unavailable in many cases.                                                                                                                            We inject such faults in a randomly chosen file on one or
Model Checking. We also model checked C TRL’s log                                                                                                               two nodes at a time, creating 1000 test cases. Table 5(b)
recovery since it involves many corner cases, using a                                                                                                           shows the results. In some cases, the faulty nodes in orig-
python-based model that we developed. We explored                                                                                                               inal versions crash because of a failed deserialization or
over 2.5M log states all of in which C TRL correctly re-                                                                                                        assertion. However, sometimes original LogCabin and
covered. Also, when key decisions are tweaked, the                                                                                                              ZooKeeper do not detect the fault and continue operat-
checker finds a violation immediately: for example, the                                                                                                         ing, violating safety in 36 and 192 cases, respectively. In
leader concludes that a faulty entry is uncommitted only                                                                                                        contrast, C TRL reliably crashes the node on a file-system
after gathering bN/2c + 1 dontHave responses; if this                                                                                                           metadata fault, preserving safety always.
number is reduced, then the checker finds a safety viola-
                                                                                                                                                                5.2           Performance
tion. We have also added the specification of C TRL’s log
                                                                                                                                                                We now compare the common-case performance of the
recovery to the TLA+ specification of Raft [23] and con-
                                                                                                                                                                C TRL versions against the original versions. In both Log-
firmed that it correctly recovers from corruptions, while
                                                                                                                                                                Cabin and ZooKeeper, reads are served from memory
the original specification violates safety.
                                                                                                                                                                and the read paths are not affected by C TRL. Hence,
5.1.2 Snapshot Recovery                                                                                                                                         we show only performance of write workloads. The
We trigger the nodes to take a snapshot, crashing them at                                                                                                       workload runs for 300 seconds, inserting entries each of
different points, producing three possible states for each                                                                                                      size 1K. Both systems batch writes to improve through-
node: l, t, and g, where l is a state where the node has                                                                                                        put. Snapshots are taken periodically during the updates.
only the log (it has not taken a snapshot), t is a snapshot                                                                                                     Numbers reported are the average over five runs.
for which garbage collection has not been performed yet,                                                                                                           Figure 5(a) and (b) show the throughput on an
and g is a snapshot which has been garbage collected. We                                                                                                        HDD for varying number of clients in LogCabin and
produce all possible combinations of states across three                                                                                                        ZooKeeper, respectively. C LSTORE physically separates
nodes. On each such state, we randomly pick a set of                                                                                                            the identifier from the entry; this separation induces a
nodes to inject faults, and corrupt a random combination                                                                                                        seek on disks in the update path. However, the seek cost
of snapshots and log entries, generating 1000 test cases.                                                                                                       is amortized when more requests are batched; C TRL has
For example, hS1 : t, S2 : g, S3 : li is a base state on which                                                                                                  an overhead of 8%-10% for 32 clients on disks. Fig-
we corrupt snapshot t and a few preceding log entries                                                                                                           ure 5(c) and (d) show throughput on an SSD; C TRL adds
on S1 and g on S2 . In such a state, if S1 becomes the                                                                                                          very minimal overhead on SSDs (4% in the worst case).
leader, it has to fix its log from S3 , then has to locally                                                                                                     Note that our workload performs only writes and there-
recover its t snapshot, after which it has to fix g on S2 .                                                                                                     fore shows C TRL’s overheads in the worst case; for more
S1 also needs to install the snapshot on S3 . As shown in                                                                                                       realistic workloads that predominantly perform reads,
Table 5(a), C TRL correctly recovers from all such cases.                                                                                                       the overheads should be even lower.
Original LogCabin is incorrect in about half of the cases                                                                                                       Fast Log Recovery. To show the potential reduction in
because it obliviously loads faulty snapshots sometimes                                                                                                         log-recovery time, we insert 30K log entries (each of size
and crashes sometimes. Original ZooKeeper crashes the                                                                                                           1K) and corrupt the first entry on one node. In origi-



26                          16th USENIX Conference on File and Storage Technologies                                                                                                                                                               USENIX Association
nal LogCabin, the faulty node detects the corruption but         C TRL can be available as long as a majority of nodes
truncates all entries; hence, the leader transfers all entries   are up and at least one non-faulty copy of a data item
to bring the node up-to-date. C TRL fixes only the faulty        exists even though portions of data on all nodes could
entry, reducing recovery time. The faulty node is fixed in       be corrupted. C TRL cannot tolerate arbitrary non-crash
1.24 seconds (32MB transferred) in the original system,          faults [40] (e.g., memory errors). However, C TRL can
while C TRL takes only 1.2 ms (7KB transferred). We see          augment the generic approaches: for example, a sys-
a similar reduction in log-recovery time in ZooKeeper.           tem can be hardened against memory faults using PASC
                                                                 while making it robust to storage faults using C TRL.
6    Related Work
Our analysis of how RSM-based systems react to storage           7    Conclusions
faults (§2.3) builds upon several fault-injection studies.       Recovering from storage faults in distributed systems is
Our design of C TRL (§3) builds upon several efforts on          surprisingly hard. We introduce protocol-aware recov-
tolerating practical faults in distributed systems.              ery (PAR), a new approach that exploits protocol-specific
Storage Faults. Several studies on storage faults [34,46,        knowledge of the underlying distributed system to cor-
48, 59, 60] motivated our work. Our previous work [29,           rectly recover from storage faults. We design C TRL, a
30] discovered fundamental reasons why distributed sys-          protocol-aware recovery approach for RSM systems. We
tems are not resilient to storage faults. However, the           experimentally show that C TRL correctly recovers from a
study did not uncover any safety or availability viola-          range of storage faults with little performance overhead.
tions reported in §2.3; this is because the fault model in          Our work is only a first step in hardening distributed
our previous study considers injecting only storage faults       systems to storage faults: while we have successfully ap-
(precisely, a single storage fault on a single node at a         plied the PAR approach to RSM systems, other classes of
time). In contrast, our fault model in this work considers       systems (e.g., primary-backup, Dynamo-style quorums)
crashes and network failures in addition to storage faults,      still remain to be analyzed. We believe the PAR approach
exposing previously unknown safety and availability vi-          can be applied to such classes as well. We hope our work
olations in RSM systems.                                         will lead to more work on building reliable distributed
Targeted Approaches. Prior research describes two ap-            storage systems that are robust to storage faults.
proaches [15, 17] to tackle storage faults in RSM sys-
tems. However, these approaches suffer from unavail-
ability. Furthermore, the MarkNonVoting approach [17]            Acknowledgments
can violate safety because important metainfo such as
promises can be lost on a storage fault [70]. C TRL avoids       We thank Mahesh Balakrishnan (our shepherd), the
such safety violations by storing two copies of metainfo         anonymous reviewers, and the members of ADSL
on each node. Approaches that improve the reliability of         for their excellent feedback. We also thank Cloud-
other specific systems have also been proposed [68, 71].         Lab [56] for providing a great environment to run our
Generic Approaches. Many generic approaches to han-              experiments. This material was supported by fund-
dling practical faults other than crashes have been pro-         ing from NSF grants CNS-1421033 and CNS-1218405,
posed. PASC [21] hardens systems to tolerate corrup-             DOE grant DE-SC0014935, and donations from EMC,
tions by maintaining two copies of the entire state on           Huawei, Microsoft, and VMware. Any opinions, find-
each node and assumes that both the copies will not be           ings, and conclusions or recommendations expressed in
faulty at the same time. This approach does not work             this material are those of the authors and may not reflect
well for storage faults; having two copies of on-disk state      the views of NSF, DOE, or other institutions.
incurs 2× space overhead. Furthermore, in most cases,
PASC crashes the node on a fault, causing unavailability.        References
XFT [42] is designed to tolerate non-crash faults. How-
                                                                  [1] Crash-Corruption Disentanglement Proof.
ever, it can tolerate only a total of b(N − 1)/2c crash and
                                                                      http://research.cs.wisc.edu/adsl/
non-crash faults. Similarly, UpRight [20] has an upper
                                                                      Publications/par/.
bound on the total faults to remain safe and available.
   C TRL differs from the generic approaches through its          [2] Ittai Abraham, Gregory Chockler, Idit Keidar, and
special focus on storage faults. This focus brings two                Dahlia Malkhi. Byzantine Disk Paxos: Optimal
main advantages. First, C TRL attributes faults at a fine             Resilience with Byzantine Shared Memory. Dis-
granularity: while the generic approaches consider a                  tributed Computing, 18(5):387–408, 2006.
node as faulty if any of its data is corrupted, C TRL con-
siders faults at the granularity of individual data items.        [3] Ramnatthan Alagappan, Aishwarya Ganesan, Yu-
Second, because of such fine-granular fault treatment,                vraj Patel, Thanumalayan Sankaranarayana Pillai,



USENIX Association                                       16th USENIX Conference on File and Storage Technologies        27
     Andrea C. Arpaci-Dusseau, and Remzi H. Arpaci-        [13] Mahesh Balakrishnan, Dahlia Malkhi, Vijayan
     Dusseau. Correlated Crash Vulnerabilities. In Pro-         Prabhakaran, Ted Wobber, Michael Wei, and
     ceedings of the 12th USENIX Conference on Op-              John D. Davis. CORFU: A Shared Log Design for
     erating Systems Design and Implementation (OSDI            Flash Clusters. In Proceedings of the 9th Sympo-
     ’16), Savannah, GA, November 2016.                         sium on Networked Systems Design and Implemen-
                                                                tation (NSDI ’12), San Jose, CA, April 2012.
 [4] Apache. ZooKeeper. https://zookeeper.
     apache.org/.                                          [14] Andrew D. Birrell, Roy Levin, Michael D.
                                                                Schroeder, and Roger M. Needham. Grapevine:
 [5] Apache.     ZooKeeper Guarantees, Prop-                    An Exercise in Distributed Computing. Commun.
     erties, and Definitions.      https://                     ACM, 25(4):260–274, April 1982.
     zookeeper.apache.org/doc/r3.2.
     2/zookeeperInternals.html#sc_                     [15] William J. Bolosky, Dexter Bradshaw, Randolph B.
     guaranteesPropertiesDefinitions.                       Haagens, Norbert P. Kusters, and Peng Li. Paxos
                                                            Replicated State Machines As the Basis of a High-
 [6] Apache Cassandra. Cassandra Replication. http:         performance Data Store. In Proceedings of the 8th
     //docs.datastax.com/en/cassandra/                      Symposium on Networked Systems Design and Im-
     2.0/cassandra/architecture/                            plementation (NSDI ’11), Boston, MA, April 2011.
     architectureDataDistributeReplication_
     c.html.                                           [16] Mike Burrows. The Chubby Lock Service for
                                                            Loosely-Coupled Distributed Systems. In Proceed-
 [7] Apache ZooKeeper.         Applications and Or-         ings of the 7th Symposium on Operating Systems
     ganizations using ZooKeeper.             https:        Design and Implementation (OSDI ’06), Seattle,
     //cwiki.apache.org/confluence/                         WA, November 2006.
     display/ZOOKEEPER/PoweredBy.
                                                       [17] Tushar D Chandra, Robert Griesemer, and Joshua
 [8] Remzi H. Arpaci-Dusseau and Andrea C. Arpaci-          Redstone. Paxos Made Live: An Engineering Per-
     Dusseau. Operating Systems: Three Easy Pieces.         spective. In Proceedings of the 26th ACM Sympo-
     Arpaci-Dusseau Books, 0.91 edition, May 2015.          sium on Principles of Distributed Computing, Port-
                                                            land, OR, August 2007.
 [9] Lakshmi N. Bairavasundaram, Andrea C. Arpaci-
     Dusseau, Remzi H. Arpaci-Dusseau, Garth R.        [18] Vijay Chidambaram, Thanumalayan Sankara-
     Goodson, and Bianca Schroeder. An Analysis of          narayana Pillai, Andrea C. Arpaci-Dusseau, and
     Data Corruption in the Storage Stack. In Proceed-      Remzi H. Arpaci-Dusseau. Optimistic Crash Con-
     ings of the 6th USENIX Symposium on File and           sistency. In Proceedings of the 24th ACM Sympo-
     Storage Technologies (FAST ’08), San Jose, CA,         sium on Operating Systems Principles (SOSP ’13),
     February 2008.                                         Farmington, PA, November 2013.
[10] Lakshmi N. Bairavasundaram, Garth R. Goodson,         [19] Vijay Chidambaram, Tushar Sharma, Andrea C.
     Shankar Pasupathy, and Jiri Schindler. An Analysis         Arpaci-Dusseau, and Remzi H. Arpaci-Dusseau.
     of Latent Sector Errors in Disk Drives. In Proceed-        Consistency Without Ordering. In Proceedings of
     ings of the 2007 ACM SIGMETRICS Conference on              the 10th USENIX Symposium on File and Storage
     Measurement and Modeling of Computer Systems               Technologies (FAST ’12), San Jose, CA, February
     (SIGMETRICS ’07), San Diego, CA, June 2007.                2012.
[11] Lakshmi N. Bairavasundaram, Meenali Rungta,           [20] Allen Clement, Manos Kapritsos, Sangmin Lee,
     Nitin Agrawal, Andrea C. Arpaci-Dusseau,                   Yang Wang, Lorenzo Alvisi, Mike Dahlin, and Tay-
     Remzi H. Arpaci-Dusseau, and Michael M. Swift.             lor Riche. Upright Cluster Services. In Proceedings
     Analyzing the Effects of Disk-Pointer Corruption.          of the 22nd ACM Symposium on Operating Systems
     In Proceedings of the International Conference             Principles (SOSP ’09), Big Sky, Montana, October
     on Dependable Systems and Networks (DSN ’08),              2009.
     Anchorage, Alaska, June 2008.
                                                           [21] Miguel Correia, Daniel Gómez Ferro, Flavio P. Jun-
[12] Lakshmi Narayanan Bairavasundaram. Character-              queira, and Marco Serafini. Practical Hardening of
     istics, Impact, and Tolerance of Partial Disk Fail-        Crash-Tolerant Systems. In 2012 USENIX Annual
     ures. PhD thesis, University of Wisconsin, Madi-           Technical Conference (USENIX ATC 12), Boston,
     son, 2008.                                                 MA, June 2012.



28   16th USENIX Conference on File and Storage Technologies                                  USENIX Association
[22] Jeff Dean. Building Large-Scale Internet Services.        In Proceedings of the 19th Annual Meeting of the
     http://static.googleusercontent.                          IEEE International Symposium on Modeling, Anal-
     com/media/research.google.com/en/                         ysis, and Simulation of Computer and Telecom-
     /people/jeff/SOCC2010-keynote-                            munication Systems (MASCOTS), Washington, DC,
     slides.pdf.                                               July 2011.
[23] Diego Ongaro. Raft TLA+ Specification. https:        [33] Kevin M Greenan, Darrell DE Long, Ethan L
     //github.com/ongardie/raft.tla.                           Miller, Thomas Schwarz, and Avani Wildani.
                                                               Building Flexible, Fault-Tolerant Flash-Based Stor-
[24] epaxos. epaxos source code. https://github.               age Systems. In The 5th Workshop on Hot Topics in
     com/efficient/epaxos.                                     System Dependability (HotDep ’09), Lisbon, Portu-
[25] etcd. etcd. https://coreos.com/etcd.                      gal, June 2009.

[26] Etcd.  Etcd: Production users. https:                [34] Laura M Grupp, Adrian M Caulfield, Joel Coburn,
     //coreos.com/etcd/docs/latest/                            Steven Swanson, Eitan Yaakobi, Paul H Siegel,
     production-users.html.                                    and Jack K Wolf. Characterizing Flash Memory:
                                                               Anomalies, Observations, and Applications. In
[27] Daniel Fryer, Dai Qin, Jack Sun, Kah Wai Lee, An-         Proceedings of the 42nd Annual IEEE/ACM In-
     gela Demke Brown, and Ashvin Goel. Checking               ternational Symposium on Microarchitecture (MI-
     the Integrity of Transactional Mechanisms. In Pro-        CRO’09), New York, New York, December 2009.
     ceedings of the 12th USENIX Symposium on File
     and Storage Technologies (FAST ’14), Santa Clara,    [35] James Hamilton. On Designing and Deploying
     CA, February 2014.                                        Internet-Scale Services. In Proceedings of the 21st
                                                               Annual Large Installation System Administration
[28] Daniel Fryer, Kuei Sun, Rahat Mahmood, TingHao            Conference (LISA ’07), Dallas, Texas, November
     Cheng, Shaun Benjamin, Ashvin Goel, and An-               2007.
     gela Demke Brown. Recon: Verifying File System
     Consistency at Runtime. In Proceedings of the 10th   [36] James Myers. Data Integrity in Solid State Drives.
     USENIX Symposium on File and Storage Technolo-            http://intel.ly/2cF0dTT.
     gies (FAST ’12), San Jose, CA, February 2012.        [37] John Goerzen.  Silent Data Corruption Is
[29] Aishwarya Ganesan, Ramnatthan Alagappan, An-              Real.    http://changelog.complete.
     drea C. Arpaci-Dusseau, and Remzi H. Arpaci-              org/archives/9769-silent-data-
     Dusseau. Redundancy Does Not Imply Fault Tol-             corruption-is-real.
     erance: Analysis of Distributed Storage Reac-        [38] Jonathan Corbet. Responding to ext4 journal
     tions to File-System Faults. ACM Trans. Storage,          corruption. https://lwn.net/Articles/
     13(3):20:1–20:33, September 2017.                         284037/.
[30] Aishwarya Ganesan, Ramnatthan Alagappan, An-         [39] Flavio P Junqueira, Benjamin C Reed, and Marco
     drea C. Arpaci-Dusseau, and Remzi H. Arpaci-              Serafini. Zab: High-Performance Broadcast for
     Dusseau. Redundancy Does Not Imply Fault Toler-           Primary-Backup Systems. In Proceedings of the
     ance: Analysis of Distributed Storage Reactions to        International Conference on Dependable Systems
     Single Errors and Corruptions. In Proceedings of          and Networks (DSN ’11), Hong Kong, China, June
     the 15th USENIX Conference on File and Storage            2011.
     Technologies (FAST ’17), Santa Clara, CA, Febru-
     ary 2017.                                            [40] Dmitrii Kuvaiskii, Rasha Faqeh, Pramod Bhato-
                                                               tia, Pascal Felber, and Christof Fetzer. HAFT:
[31] Sanjay Ghemawat, Howard Gobioff, and Shun-Tak             Hardware-assisted Fault Tolerance. In Proceedings
     Leung. The Google File System. In Proceedings             of the EuroSys Conference (EuroSys ’16), London,
     of the 19th ACM Symposium on Operating Systems            United Kingdom, April 2016.
     Principles (SOSP ’03), Bolton Landing, New York,
     October 2003.                                        [41] Leslie Lamport. Paxos Made Simple. ACM Sigact
                                                               News, 32(4):18–25, 2001.
[32] Matthias Grawinkel, Thorsten Schafer, Andre
     Brinkmann, Jens Hagemeyer, and Mario Por-            [42] Shengyun Liu, Paolo Viotti, Christian Cachin,
     rmann. Evaluation of Applied Intra-disk Redun-            Vivien Quéma, and Marko Vukolic. XFT: Practical
     dancy Schemes to Improve Single Disk Reliability.         Fault Tolerance Beyond Crashes. In Proceedings



USENIX Association                                16th USENIX Conference on File and Storage Technologies      29
     of the 12th USENIX Conference on Operating Sys-      [53] Thanumalayan Sankaranarayana Pillai, Vijay
     tems Design and Implementation (OSDI ’16), Sa-            Chidambaram, Ramnatthan Alagappan, Samer
     vannah, GA, November 2016.                                Al-Kiswany, Andrea C. Arpaci-Dusseau, and
                                                               Remzi H. Arpaci-Dusseau. All File Systems Are
[43] LogCabin. LogCabin. https://github.com/                   Not Created Equal: On the Complexity of Crafting
     logcabin/logcabin.                                        Crash-consistent Applications. In Proceedings of
[44] Jacob R Lorch, Atul Adya, William J Bolosky,              the 11th Symposium on Operating Systems Design
     Ronnie Chaiken, John R Douceur, and Jon Howell.           and Implementation (OSDI ’14), Broomfield, CO,
     The SMART Way to Migrate Replicated Stateful              October 2014.
     Services. In Proceedings of the EuroSys Confer-
                                                          [54] Vijayan Prabhakaran, Lakshmi N. Bairavasun-
     ence (EuroSys ’06), Leuven, Belgium, April 2006.
                                                               daram, Nitin Agrawal, Haryadi S. Gunawi, An-
[45] Parisa Jalili Marandi, Christos Gkantsidis, Flavio        drea C. Arpaci-Dusseau, and Remzi H. Arpaci-
     Junqueira, and Dushyanth Narayanan. Filo: Con-            Dusseau. IRON File Systems. In Proceedings of the
     solidated Consensus As a Cloud Service. In 2016           20th ACM Symposium on Operating Systems Prin-
     USENIX Annual Technical Conference (USENIX                ciples (SOSP ’05), Brighton, UK, October 2005.
     ATC 16), Denver, CO, June 2016.
                                                          [55] Redis. Redis Replication. http://redis.io/
[46] Justin Meza, Qiang Wu, Sanjev Kumar, and Onur             topics/replication.
     Mutlu. A Large-Scale Study of Flash Memory Fail-
     ures in the Field. In Proceedings of the 2015 ACM    [56] Robert Ricci, Eric Eide, and CloudLab Team. In-
     SIGMETRICS International Conference on Mea-               troducing CloudLab: Scientific infrastructure for
     surement and Modeling of Computer Systems (SIG-           advancing cloud architectures and applications.
     METRICS ’15), Portland, Oregon, June 2015.                USENIX ;login:, 39(6), 2014.

[47] MongoDB.        MongoDB   Replication.               [57] Robert Harris. Data corruption is worse than you
     https://docs.mongodb.org/manual/                          know. http://www.zdnet.com/article/
     replication/.                                             data-corruption-is-worse-than-
                                                               you-know/.
[48] Iyswarya Narayanan, Di Wang, Myeongjae Jeon,
     Bikash Sharma, Laura Caulfield, Anand Sivasubra-     [58] Fred B. Schneider. Implementing Fault-tolerant
     maniam, Ben Cutler, Jie Liu, Badriddine Khessib,          Services Using the State Machine Approach: A Tu-
     and Kushagra Vaid. SSD Failures in Datacenters:           torial. ACM Comput. Surv., 22(4):299–319, De-
     What? When? And Why? In Proceedings of                    cember 1990.
     the 9th ACM International on Systems and Stor-
     age Conference (SYSTOR ’16), Haifa, Israel, June     [59] Bianca Schroeder, Sotirios Damouras, and Phillipa
     2016.                                                     Gill. Understanding Latent Sector Errors and How
                                                               to Protect Against Them. In Proceedings of the 8th
[49] Diego Ongaro. Consensus: Bridging Theory and
                                                               USENIX Symposium on File and Storage Technolo-
     Practice. PhD thesis, Stanford University, 2014.
                                                               gies (FAST ’10), San Jose, CA, February 2010.
[50] Diego Ongaro and John Ousterhout. In Search of
     an Understandable Consensus Algorithm. In 2014       [60] Bianca Schroeder, Raghav Lagisetty, and Arif Mer-
     USENIX Annual Technical Conference (USENIX                chant. Flash Reliability in Production: The Ex-
     ATC 14), Philadelphia, PA, June 2014.                     pected and the Unexpected. In Proceedings of
                                                               the 14th USENIX Conference on File and Storage
[51] Bernd Panzer-Steindel. Data Integrity. CERN/IT,           Technologies (FAST ’16), Santa Clara, CA, Febru-
     2007.                                                     ary 2016.
[52] Thanumalayan Sankaranarayana Pillai, Ram-            [61] Michael D. Schroeder, Andrew D. Birrell, and
     natthan Alagappan, Lanyue Lu, Vijay Chi-                  Roger M. Needham. Experience with Grapevine:
     dambaram, Andrea C. Arpaci-Dusseau, and                   The Growth of a Distributed System. ACM Trans.
     Remzi H. Arpaci-Dusseau. Application Crash                Comput. Syst., 2(1):3–23, February 1984.
     Consistency and Performance with CCFS. In
     Proceedings of the 15th USENIX Conference on         [62] Thomas Schwarz, Ahmed Amer, Thomas Kroeger,
     File and Storage Technologies (FAST ’17), Santa           Ethan L. Miller, Darrell D. E. Long, and Jehan-
     Clara, CA, February 2017.                                 Franois Pris. RESAR: Reliable Storage at Exabyte



30   16th USENIX Conference on File and Storage Technologies                                USENIX Association
     Scale. In Proceedings of the 24th Annual Meet-           Alvisi, and Mike Dahlin. Robustness in the Salus
     ing of the IEEE International Symposium on Mod-          Scalable Block Store. In Proceedings of the 10th
     eling, Analysis, and Simulation of Computer and          Symposium on Networked Systems Design and Im-
     Telecommunication Systems (MASCOTS), London,             plementation (NSDI ’13), Lombard, IL, April 2013.
     United Kingdom, September 2016.
                                                          [72] Yupu Zhang, Abhishek Rajimwale, Andrea C.
[63] Romain Slootmaekers and Nicolas Trangez.                  Arpaci-Dusseau, and Remzi H. Arpaci-Dusseau.
     Arakoon: A Distributed Consistent Key-Value               End-to-end Data Integrity for File Systems: A
     Store. In SIGPLAN OCaml Users and Developers              ZFS Case Study. In Proceedings of the 8th
     Workshop, volume 62, 2012.                                USENIX Symposium on File and Storage Technolo-
                                                               gies (FAST ’10), San Jose, CA, February 2010.
[64] Stackoverflow. Can ext4 detect corrupted file
     contents?       http://stackoverflow.                [73] ZooKeeper Jira Issues. Unable to load database
     com/questions/31345097/can-ext4-                          on disk when restarting after node freeze.
     detect-corrupted-file-contents.                           https://issues.apache.org/jira/
                                                               browse/ZOOKEEPER-1546.
[65] Stackoverflow. ZooKeeper Clear State. http:
     //stackoverflow.com/questions/
     17038957/org-apache-hadoop-hbase-
     pleaseholdexception-master-is-
     initializing.

[66] Michael M. Swift, Brian N. Bershad, and Henry M.
     Levy. Improving the Reliability of Commodity
     Operating Systems. In Proceedings of the 19th
     ACM Symposium on Operating Systems Principles
     (SOSP ’03), Bolton Landing, New York, October
     2003.

[67] D. B. Terry, M. M. Theimer, Karin Petersen, A. J.
     Demers, M. J. Spreitzer, and C. H. Hauser. Man-
     aging Update Conflicts in Bayou, a Weakly Con-
     nected Replicated Storage System. In Proceedings
     of the 15th ACM Symposium on Operating Systems
     Principles (SOSP ’95), Copper Mountain Resort,
     CO, December 1995.

[68] Thanh Do, Tyler Harter, Yingchao Liu, Haryadi
     S. Gunawi, Andrea C. Arpaci-Dusseau, Remzi H.
     Arpaci-Dusseau. HARDFS: Hardening HDFS with
     Selective and Lightweight Versioning. In Pro-
     ceedings of the 11th Conference on File and Stor-
     age Technologies (FAST ’13), San Jose, California,
     February 2013.

[69] Theodore Ts’o. What to do when the journal
     checksum is incorrect. https://lwn.net/
     Articles/284038/.

[70] Robbert Van Renesse, Nicolas Schiper, and Fred B
     Schneider. Vive La Différence: Paxos vs. View-
     stamped Replication vs. Zab. IEEE Transactions
     on Dependable and Secure Computing, 12(4):472–
     484, 2015.

[71] Yang Wang, Manos Kapritsos, Zuocheng Ren,
     Prince Mahajan, Jeevitha Kirubanandam, Lorenzo



USENIX Association                                16th USENIX Conference on File and Storage Technologies   31
