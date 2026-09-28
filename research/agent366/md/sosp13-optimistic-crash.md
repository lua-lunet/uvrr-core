                                         Optimistic Crash Consistency
                             Vijay Chidambaram, Thanumalayan Sankaranarayana Pillai,
                                Andrea C. Arpaci-Dusseau, Remzi H. Arpaci-Dusseau
                                         Department of Computer Sciences
                                          University of Wisconsin, Madison
                                  {vijayc, madthanu, dusseau, remzi}@cs.wisc.edu


Abstract                                                                      the disk has received the request, not that the data has
                                                                              been written to the disk surface persistently.
We introduce optimistic crash consistency, a new ap-                             Out-of-order write completion, in particular, greatly
proach to crash consistency in journaling file systems.                       complicates known techniques for recovering from sys-
Using an array of novel techniques, we demonstrate how                        tem crashes. For example, modern journaling file sys-
to build an optimistic commit protocol that correctly                         tems such as Linux ext3, XFS, and NTFS all carefully
recovers from crashes and delivers high performance.                          orchestrate a sequence of updates to ensure that writes to
We implement this optimistic approach within a Linux                          main file-system structures and the journal reach disk in a
ext4 variant which we call OptFS. We introduce two                            particular order [22]; copy-on-write file systems such as
new file-system primitives, osync() and dsync(), that                         LFS, btrfs, and ZFS also require ordering when updating
decouple ordering of writes from their durability. We                         certain structures. Without ordering, most file systems
show through experiments that OptFS improves perfor-                          cannot ensure that state can be recovered after a crash [6].
mance for many workloads, sometimes by an order of
                                                                                 Write ordering is achieved in modern drives via ex-
magnitude; we confirm its correctness through a series
                                                                              pensive cache flush operations [30]; such flushes cause
of robustness tests, showing it recovers to a consistent
                                                                              all buffered dirty data in the drive to be written to the
state after crashes. Finally, we show that osync() and
                                                                              surface (i.e., persisted) immediately. To ensure A is writ-
dsync() are useful in atomic file system and database
                                                                              ten before B, a client issues the write to A, and then a
update scenarios, both improving performance and meet-
                                                                              cache flush; when the flush returns, the client can safely
ing application-level consistency demands.
                                                                              assume that A reached the disk; the write to B can then
                                                                              be safely issued, knowing it will be persisted after A.
1 Introduction                                                                   Unfortunately, cache flushing is expensive, sometimes
Modern storage devices present a seemingly innocuous                          prohibitively so. Flushes make I/O scheduling less effi-
interface to clients. To read a block, one simply issues a                    cient, as the disk has fewer requests to choose from. A
low-level read command and specifies the address of the                       flush also unnecessarily forces all previous writes to disk,
block (or set of blocks) to read; when the disk finishes                      whereas the requirements of the client may be less strin-
the read, it is transferred into memory and any awaiting                      gent. In addition, during a large cache flush, disk reads
clients notified of the completion. A similar process is                      may exhibit extremely long latencies as they wait for
followed for writes.                                                          pending writes to complete [26]. Finally, flushing con-
   Unfortunately, the introduction of write buffering [28]                    flates ordering and durability; if a client simply wishes
in modern disks greatly complicates this apparently sim-                      to order one write before another, forcing the first write
ple process. With write buffering enabled, disk writes                        to disk is an expensive manner in which to achieve such
may complete out of order, as a smart disk scheduler may                      an end. In short, the classic approach of flushing is pes-
reorder requests for performance [13, 24, 38]; further, the                   simistic; it assumes a crash will occur and goes to great
notification received after a write issue implies only that                   lengths to ensure that the disk is never in an inconsis-
                                                                              tent state via flush commands. The poor performance
                                                                              that results from pessimism has led some systems to dis-
Permission to make digital or hard copies of part or all of this work for     able flushing, apparently sacrificing correctness for per-
personal or classroom use is granted without fee provided that copies         formance; for example, the Linux ext3 default configu-
are not made or distributed for profit or commercial advantage and that       ration did not flush caches for many years [8].
copies bear this notice and the full citation on the first page. Copyrights
for third-party components of this work must be honored. For all other           Disabling flushes does not necessarily lead to file sys-
uses, contact the Owner/Author.                                               tem inconsistency, but rather introduces it as a possibil-
                                                                              ity. We refer to such an approach as probabilistic crash
Copyright is held by the Owner/Author(s).                                     consistency, in which a crash might lead to file system in-
SOSP’13, Nov. 3–6, 2013, Farmington, Pennsylvania, USA.
ACM 978-1-4503-2388-8/13/11.                                                  consistency, depending on many factors, including work-
http://dx.doi.org/10.1145/2517349.2522726                                     load, system and disk parameters, and the exact timing of
the crash or power loss. In this paper, one of our first con-   enables application-level consistency at high perfor-
tributions is the careful study of probabilistic crash con-     mance. OptFS introduces two new file-system primi-
sistency, wherein we show which exact factors affect the        tives: osync(), which ensures ordering between writes
odds that a crash will leave the file system inconsistent       but only eventual durability, and dsync(), which en-
(§3). We show that for some workloads, the probabilistic        sures immediate durability as well as ordering.
approach rarely leaves the file system inconsistent.               We show how these primitives provide a useful base
   Unfortunately, a probabilistic approach is insufficient      on which to build higher-level application consistency
for many applications, where certainty in crash recov-          semantics (§7). Specifically, we show how a document
ery is desired. Indeed, we also show, for some work-            editing application can use osync() to implement the
loads, that the chances of inconsistency are high; to           atomic update of a file (via a create and then atomic re-
realize higher-level application-level consistency (i.e.,       name), and how the SQLite database management sys-
something a DBMS might desire), the file system must            tem can use file-system provided ordering to implement
provide something more than probability and chance.             ordered transactions with eventual durability. We show
Thus, in this paper, we introduce optimistic crash con-         that these primitives are sufficient for realizing useful
sistency, a new approach to building a crash-consistent         application-level consistency at high performance.
journaling file system (§4). This optimistic approach              Of course, the optimistic approach, while useful in
takes advantage of the fact that in many cases, ordering        many scenarios, is not a panacea. If an application
can be achieved through other means and that crashes            requires immediate, synchronous durability (instead of
are rare events (similar to optimistic concurrency con-         eventual, asynchronous durability with consistent order-
trol [12, 16]). However, realizing consistency in an opti-      ing), an expensive cache flush is still required. In this
mistic fashion is not without challenge; we thus develop        case, applications can use dsync() to request durabil-
a range of novel techniques, including a new extension of       ity (as well as ordering). However, by decoupling the
the transactional checksum [23] to detect data/metadata         durability of writes from their ordering, OptFS provides
inconsistency, delayed reuse of blocks to avoid incorrect       a useful middle ground, thus realizing high performance
dangling pointers, and a selective data journaling tech-        and meaningful crash consistency for many applications.
nique to handle block overwrite correctly. The combina-
tion of these techniques leads to both high performance         2 Pessimistic Crash Consistency
and deterministic consistency; in the rare event that a         To understand the optimistic approach to journaling, we
crash does occur, optimistic crash consistency either           first describe standard pessimistic crash consistency in
avoids inconsistency by design or ensures that enough           journaling file systems. To do so, we describe the nec-
information is present on the disk to detect and discard        essary disk support (i.e., cache-flushing commands) and
improper updates during recovery.                               details on how such crash consistency operates. We then
   We demonstrate the power of optimistic crash consis-         demonstrate the negative performance impact of cache
tency through the design, implementation, and analysis          flushing during pessimistic journaling.
of the optimistic file system (OptFS). OptFS builds upon
the principles of optimistic crash consistency to imple-        2.1    Disk Interface
ment optimistic journaling, which ensures that the file         For the purposes of this discussion, we assume the pres-
system is kept consistent despite crashes. Optimistic           ence of a disk-level cache flush command. In the ATA
journaling is realized as a set of modifications to the         family of drives, this is referred to as the “flush cache”
Linux ext4 file system, but also requires a slight change       command; in SCSI drives, it is known as “synchronize
in the disk interface to provide what we refer to as asyn-      cache”. Both operations have similar semantics, forcing
chronous durability notification, i.e., a notification when     all pending dirty writes in the disk to be written to the
a write is persisted in addition to when the write has sim-     surface. Note that a flush can be issued as a separate re-
ply been received by the disk. We describe the details of       quest, or as part of a write to a given block D; in the latter
our implementation (§5) and study its performance (§6),         case, pending writes are flushed before the write to D.
showing that for a range of workloads, OptFS signifi-              Some finer-grained controls also exist. For exam-
cantly outperforms classic Linux ext4 with pessimistic          ple, “force unit access” (FUA) commands read or write
journaling, and showing that OptFS performs almost              around the cache entirely. FUAs are often used in tan-
identically to Linux ext4 with probabilistic journaling         dem with flush commands; for example, to write A be-
while ensuring crash consistency.                               fore B, but to also ensure that both A and B are durable, a
   Central to the performance benefits of OptFS is              client might write A, then write B with both cache flush-
the separation of ordering and durability. By allow-            ing and FUA enabled; this ensures that when B reaches
ing applications to order writes without incurring a            the drive, A (and other dirty data) will be forced to disk;
disk flush, and request durability when needed, OptFS           subsequently, B will be forced to disk due to the FUA.
2.2    Pessimistic Journaling                                    tem crash (or power loss) would leave that transaction
Given the disk interface described above, we now de-             in a seemingly committed state but with garbage con-
scribe how a journaling file system safely commits data          tents. By computing a checksum over the entire transac-
to disk in order to maintain consistency in the event of a       tion and placing its value in JC , the writes to JM and JC
system crash. We base our discussion on ordered-mode             can be issued together, improving performance; with the
Linux ext3 and ext4 [34, 35], though much of what we             checksum present, crash recovery can avoid replay of im-
say is applicable to other journaling file systems such as       properly committed transactions. With this optimization,
SGI XFS [32], Windows NTFS [27], and IBM JFS [3].                the ordering is D → JM |JC → M (where the bar over the
In ordered mode, file-system metadata is journaled to            journal updates indicates their protection via checksum).
maintain its consistency; data is not journaled, as writing         Interestingly, these two optimizations do not combine,
each data block twice reduces performance substantially.         i.e., D|JM |JC → M is not correct; if the file system is-
   When an application updates file-system state, either         sues D, JM , and JC together, it is possible that JM and
metadata, user data, or (often) both need to be updated in       JC reach the disk first. In this case, the metadata com-
a persistent manner. For example, when a user appends            mits before the data; if a crash occurs before the data
a block to a file, a new data block (D) must be written to       is written, an inode (or indirect block) in the commit-
disk (at some point); in addition, various pieces of meta-       ted transaction could end up pointing to garbage data.
data (M) must be updated as well, including the file’s           Oddly, ext4 allows this situation with the “right” set of
inode and a bitmap marking the block as allocated.               mount options.
   We refer to the atomic update of metadata to the jour-           We should note that one other important ordering ex-
nal as a transaction. Before committing a transaction T x        ists among updates, specifically the order between trans-
to the journal, the file system first writes any data blocks     actions; journaling file systems assume transactions are
(D) associated with the transaction to their final desti-        committed to disk in order (i.e., T xi → T xi+1 ) [35]. Not
nations; writing data before transaction commit ensures          following this ordering could lead to odd results during
that committed metadata does not point to garbage. Af-           crash recovery. For example, a block B could have been
ter these data writes complete, the file system uses the         freed in T xi , and then reused in T xi+1 ; in this case, a
journal to log metadata updates; we refer to these journal       crash after T xi+1 committed but before T xi did would
writes as JM . After these writes are persisted, the file sys-   lead to a state where B is allocated to two files.
tem issues a write to a commit block (JC ); when the disk           Finally, and most importantly, we draw attention to the
persists JC , the transaction T x is said to be committed.       pessimistic nature of this approach. Whenever ordering
Finally, after the commit, the file system is free to update     is required, an expensive cache flush is issued, thus forc-
the metadata blocks in place (M); if a crash occurs during       ing all pending writes to disk, when perhaps only a sub-
this checkpointing process, the file system can recover          set of them needed to be flushed. In addition, the flushes
simply by scanning the journal and replaying committed           are issued even though the writes may have gone to disk
transactions. Details can be found elsewhere [22, 35].           in the correct order anyhow, depending on scheduling,
   We thus have the following set of ordered writes that         workload, and other details; flushes induce extra work
must take place: D before JM before JC before M, or              that may not be necessary. Finally, and perhaps most
more simply: D → JM → JC → M. Note that D, JM ,                  harmful, is the fact that the burden of flushes is added de-
and M can represent more than a single block (in larger          spite the fact that crashes are rare, thus exacting a heavy
transactions), whereas JC is always a single sector (for         cost in anticipation of an extremely occasional event.
the sake of write atomicity). To achieve this ordering,
                                                                 2.3    Flushing Performance Impact
the file system issues a cache flush wherever order is re-
quired (i.e., where there is a → symbol).                        To better understand the performance impact of cache
   Optimizations to this protocol have been suggested in         flushing during pessimistic journaling, we now perform
the literature, some of which have been realized in Linux        a simple experiment. Specifically, we run the Varmail
ext4. For example, some have noted that the ordering             benchmark atop Linux ext4, both with and without cache
between data and journaled metadata (D → JM ) is super-          flushing; with cache flushing enabled, we also enable
fluous; removing that ordering can sometimes improve             transactional checksums to see their performance impact.
performance (D|JM → JC → M) [22].                                Varmail is a good choice here as it simulates an email
   Others have suggested a “transactional check-                 server and includes many small synchronous updates to
sum” [23] which can be used to remove the ordering be-           disk, thus stressing the journaling machinery described
tween the journal metadata and journal commit (JM and            above. The experimental setup for this benchmark and
JC ). In the normal case, the file system cannot issue JM        configuration is described in more detail in Section 6.
and JC together, because the drive might reorder them; in           From Figure 1, we can observe the following. First,
that case, JC might hit the disk first, at which point a sys-    transactional checksums increase performance slightly,
                                                                 Ext4




       Throughput (IO/s)
                           5000                                (No Flush)
                                                                                                 W
                           4000                                                     Pinc =                            W = t2 - t1
                                                                                           t   workload
                           3000
                           2000                  Ext4                                 1          2        5           3         4        6
                                  Ext4      (Flush+Checksum)
                                  (Flush)                                                                     t1
                           1000                                                                                                     t2
                             0
                                                                                                                   t workload
Figure 1: The Cost of Flushing. The figure shows the per-
formance of Filebench Varmail on different ext4 configurations.             Figure 2: The Probability of Inconsistency (Pinc ). An
Performance increases 5X when flushes are disabled.                         example of a window of vulnerability is shown. Blocks 1
                                                                            through 6 were meant to be written in strict order to disk. How-
showing how removing a single ordering point (via a                         ever, block 5 (dark gray) is written early. Once 5 is committed,
checksum) can help. Second, and most importantly, there                     a window of vulnerability exists until blocks 3 and 4 (light gray)
is a vast performance improvement when cache flushing                       are committed; a crash during this time will lead to observable
is turned off, in this case nearly a factor of five. Given                  reordering. The probability of inconsistency is calculated by
                                                                            dividing the time spent in such a window (i.e., W = t2 − t1 ) by
this large performance difference, in some installations,
                                                                            the total runtime of the workload (i.e., tworkload ).
cache flushing is disabled, which leads to the follow-
ing question: what kind of crash consistency is provided                    ten occurs long after the transaction has been committed,
when flushing is disabled? Surprisingly, the answer is                      and thus ordering is preserved without a flush.
not “none”, as we now describe.                                                We refer to this arrangement as probabilistic consis-
                                                                            tency. In such a configuration, typical operation may or
3 Probabilistic Crash Consistency                                           may not result in much reordering, and thus the disk is
                                                                            only sometimes in an inconsistent state. A crash may not
Given the potential performance gains, practitioners
                                                                            lead to inconsistency despite a lack of enforcement by
sometimes forgo the safety provided by correct im-
                                                                            the file system via flush commands. Despite probabilistic
plementations that issue flushes and choose to disable
                                                                            crash consistency offering no guarantees on consistency
flushes [8]. In this fast mode, a risk of file-system incon-
                                                                            after a crash, many practitioners are drawn to it due to
sistency is introduced; if a crash occurs at an untimely
                                                                            large performance gains from turning off flushing.
point in the update sequence, and blocks have been re-
ordered across ordering points, crash recovery as run by                    3.1    Quantifying Probabilistic Consistency
the file system will result in an inconsistent file system.                 Unfortunately, probabilistic consistency is not well un-
   In some cases, practitioners observed that skipping                      derstood. To shed light on this issue, we quantify how
flush commands sometimes did not lead to observable in-                     often inconsistency arises without flushing via simula-
consistency, despite the presence of (occasional) crashes.                  tion. To do so, we disable flushing in Linux ext4 and
Such commentary led to a debate within the Linux com-                       extract block-level traces underneath ext4 across a range
munity as to underlying causes. Long-time kernel devel-                     of workloads. We analyze the traces carefully to deter-
oper Theodore Ts’o hypothesized why such consistency                        mine the chances of an inconsistency occurring due to
was often achieved despite the lack of ordering enforce-                    a crash. Our analysis is done via a simulator built atop
ment by the file system [33]:                                               DiskSim [4], which enables us to model complex disk
     I suspect the real reason why we get away with                         behaviors (e.g., scheduling, caching).
     it so much with ext3 is that the journal is usu-                           The main output of our simulations is a determination
     ally contiguous on disk, hence, when you write                         of when a window of vulnerability (W ) arises, and for
     to the journal, it’s highly unlikely that commit                       how long such windows last. Such a window occurs due
     block will be written and the blocks before the                        to reordering. For example, if A should be written to
     commit block have not. ... The most important                          disk before B, but B is written at time t1 and A written at
     reason, though, is that the blocks which are                           t2 , the state of the system is vulnerable to inconsistency
     dirty don’t get flushed out to disk right away!                        in the time period between, W = t2 − t1 . If the system
                                                                            crashes during this window, the file system will be left
   What the Ts’o Hypothesis refers to specifically is two                   in an inconsistent state; conversely, once the latter block
orderings: JM → JC and JC → M. In the first case, Ts’o                      (A) is written, there is no longer any concern.
notes that the disk is likely to commit JC to disk after                        Given a workload and a disk model, it is thus possible
JM even without an intervening flush (note that this is                     to quantify the probability of inconsistency (Pinc ) by di-
without the presence of transactional checksums) due to                     viding the total time spent in windows of vulnerability by
layout and scheduling; disks are simply unlikely to re-                     the total run time of the workload (Pinc = ∪ Wi /tworkload );
order two writes that are contiguous. In the second case,                   Figure 2 shows an example. Note that when a workload
Ts’o notes that JC → M often holds without a flush due                      is run on a file system with cache-flushing enabled, Pinc
to time; the checkpoint traffic that commits M to disk of-                  is always zero.
                                                                                       P (inconsistency)                                                    Total I/O Time (s)
                       60      Early Checkpoint                                                            50%
                               Transaction Misorder                                                                         P (inconsistency)         300
                       50      Mixed




   P (inconsistency)
                                                                                                                                                      200
                               Early Commit                                                                25%                            Time
                       40
                                                                                                                                                      100
                       30
                                                                                                           0%                                         0
                                                                                                                 1 2   4      8                  16
                       20
                                                                                                                           Queue Size
                       10
                                                                                     Figure 4: The Effect of Queue Size.            The figure shows
                       0                                                             Pinc (left y-axis) and total I/O completion time (right y-axis) as
                            Seq     Rand    Create    Web      File  Varmail MySQL
                            Write   Write    Files   Server   Server
                                                                                     the queue size of the simulated disk varies (x-axis). For this
Figure 3: Workload Study of Pinc . The figure shows Pinc                             experiment, we use the Varmail workload.
for six workloads. The first two workloads are sequential and
                                                                                     before D) has resulted. The graph breaks down Pinc into
random writes to a 1 GB file. Fileserver, Webserver, and Var-
mail are part of the Filebench benchmark suite [17]. Fileserver
                                                                                     these fine-grained reordering categories, grouped into
performs a sequence of creates, deletes, appends, reads, and                         the following relevant cases: early commit (e.g., JC →
writes. Webserver emulates a multi-threaded web host server,                         JM |D), early checkpoint (e.g., M → D|JM |JC ), transac-
performing sequences of open-read-close on multiple files plus                       tion misorder (e.g., T xi → T xi−1 ), and mixed (e.g., where
a log file append. Varmail emulates a multi-threaded mail                            more than one category could be attributed).
server, performing a sequence of create-append-sync, read-                              Our experiments show that early commit before data,
append-sync, reads, and deletes in a single directory. MySQL                         (JC → D), is the largest contributor to Pinc , accounting
represents the OLTP benchmark from Sysbench [1]. Each bar                            for over 90% of inconsistency across all workloads, and
is broken down into the percent contribution of the different
                                                                                     100% in some cases (Fileserver, random writes). This
types of misordering. Standard deviations are shown as well.
                                                                                     is not surprising, as in cases where transactions are being
3.2                    Factors affecting Pinc                                        forced to disk (e.g., due to calls to fsync()), data writes
                                                                                     (D) are issued just before transaction writes (JM and JC );
We now explore Pinc more systematically. Specifically,                               slight re-orderings by the disk will result in JC being
we determine sensitivity to workload and disk param-                                 persisted first. Also, for some workloads (MySQL, Var-
eters such as queue size and placement of the journal                                mail), all categories might contribute; though rare, early
relative to file-system structures. We use the validated                             checkpoints and transaction misordering can arise. Thus,
Seagate Cheetah 15k.5 disk model [4] provided with                                   any approach to provide reliable consistency mecha-
DiskSim for our experiments.                                                         nisms must consider all possible causes, not just one.
3.2.1                   Workload                                                     3.2.2                   Queue Size
We first show how the workload can impact Pinc . For this                            For the remaining studies, we focus on Varmail, as it ex-
experiment, we use 6 different workloads described in                                hibits the most interesting and varied probability of in-
the caption of Figure 3. From the figure, we make the                                consistency. First, we show how disk scheduler queue
following observations. Most importantly, Pinc is work-                              depth matters. Figure 4 plots the results of our experi-
load dependent. For example, if a workload is mostly                                 ment. The left y-axis plots Pinc as we vary the number
read oriented, there is little chance of inconsistency, as                           of outstanding requests to the disk; the right y-axis plots
file-system state is not updated frequently (e.g., Web-                              performance (overall time for all I/Os to complete).
server). Second, for write-heavy workloads, the nature                                  From the figure, we observe the following three re-
of the writes is important; workloads that write randomly                            sults. First, when there is no reordering done by the disk
or force writes to disk via fsync() lead to a fairly high                            (i.e., queue size is 1), there is no chance of inconsistency,
chance of a crash leaving the file system inconsistent                               as writes are committed in order; we would find the same
(e.g., random writes, MySQL, Varmail). Third, there can                              result if we used FIFO disk scheduling (instead of SPTF).
be high variance in Pinc ; small events that change the or-                          Second, even with small queues (e.g., 8), a great deal of
der of persistence of writes can lead to large differences                           inconsistency can arise; one block committed too early
in chances of inconsistency. Finally, even under extreme                             to disk can result in very large windows of vulnerability.
circumstances, Pinc never reaches 100% (the graph is cut                             Finally, we observe that a modest amount of reordering
off at 60%); there are many points in the lifetime of a                              does indeed make a noticeable performance difference;
workload when a crash will not lead to inconsistency.                                in-disk SPTF scheduling improves performance by about
   Beyond the overall Pinc shown in the graph, we also                               30% with a queue size of 8 or more.
break the probability further by the type of reordering
that leads to a window of vulnerability. Specifically, as-                           3.2.3                   Journal Layout
suming the following commit ordering (D|JM → JC →                                    We now study how distance between the main file-
M), we determine when a particular reordering (e.g., JC                              system structures and the journal affects Pinc . Figure 5
  P (inconsistency)                                                           Total I/O Time (s)
                      50%
                                                                        300                        4 Optimistic Crash Consistency
                                                         Time
                                                                        200                        Given that journaling with probabilistic consistency of-
                      25%
                                                                        100                        ten gives consistent results even in the presence of sys-
                                     P (inconsistency)
                      0%                                                 0                         tem crashes, we note a new opportunity. The goal of
                            0   20    40    60    80     100    120   140
                                                                                                   optimistic crash consistency, as realized in an optimistic
                            Distance Between Data and Journal (GB)
                                                                                                   journaling system, is to commit transactions to persis-
Figure 5: The Effect of Distance. The figure shows Pinc                                            tent storage in a manner that maintains consistency to the
(left y-axis) and total I/O completion time (right y-axis) as the                                  same extent as pessimistic journaling, but with nearly the
distance (in GB) between the data region and the journal of the                                    same performance as with probabilistic consistency. Op-
simulated disk is increased (x-axis). For this experiment, we                                      timistic journaling requires minimal changes to current
use the Varmail workload, with queue size set to 8.
                                                                                                   disk interfaces and the journaling layer; in particular, our
                                                                                                   approach does not require changes to file-system struc-
plots the results of varying the location of Varmail’s data                                        tures outside of the journal (e.g., backpointers [6]).
and metadata structures (which are usually located in one                                             To describe optimistic crash consistency and journal-
disk area) from close to the journal (left) to far away.                                           ing, we begin by describing the intuition behind opti-
   From the figure, we observe distance makes a signif-                                            mistic techniques. Optimistic crash consistency is based
icant difference in Pinc . Recall that one of the major                                            on two main ideas. First, checksums can remove the need
causes of reordering is early commit (i.e., JC written be-                                         for ordering writes. Optimistic crash consistency elim-
fore D); by separating the location of data and the jour-                                          inates the need for ordering during transaction commit
nal, it becomes increasingly unlikely for such reordering                                          by generalizing metadata transactional checksums [23]
to occur. Secondly, we also observe that increased dis-                                            to include data blocks. During recovery, transactions are
tance is not a panacea; inconsistency (10%) still arises                                           discarded upon checksum mismatch.
for Varmail. Finally, increased distance from the jour-                                               Second, asynchronous durability notifications are used
nal can affect performance somewhat; there is a 14% de-                                            to delay checkpointing a transaction until it has been
crease in performance when moving Varmail’s data and                                               committed durably. Fortunately, this delay does not af-
metadata from right next to the journal to 140 GB away.                                            fect application performance, as applications block until
   We also studied a number of other factors that might                                            the transaction is committed, not until it is checkpointed.
affect Pinc , including the placement of the journal as it re-                                     Additional techniques are required for correctness in sce-
lates to track boundaries on the disk, and other potential                                         narios such as block reuse and overwrite.
factors. In general, these parameters did not significantly                                           We first propose an additional notification that disk
affect Pinc and thus are not included.                                                             drives should expose. We then explain how optimistic
                                                                                                   journaling provides different properties to preserve the
                                                                                                   consistency semantics of ordered journaling. We show
3.3                    Summary
                                                                                                   that these properties can be achieved using a combination
The classic approach to journaling is overly pessimistic,                                          of optimistic techniques. We also describe an additional
forcing writes to persistent storage often when only or-                                           optimistic technique which enables optimistic journaling
dering is desired. As a result, users have sometimes                                               to provide consistency equivalent to data journaling.
turned to probabilistic journaling, taking their chances
                                                                                                   4.1    Asynchronous Durability Notification
with consistency in order to gain more performance. We
have carefully studied which factors affect the consis-                                            The current interface to the disk for ensuring that write
tency of the probabilistic approach, and shown that for                                            operations are performed in a specified order is pes-
some workloads, it works fairly well; unfortunately, for                                           simistic: the upper-level file system tells the lower-level
other workloads with a high number of random-write                                                 disk when it must flush its cache (or certain blocks) and
I/Os, or where the application itself forces traffic to disk,                                      the disk must then promptly do so. However, the actual
the probability of inconsistency becomes high. As de-                                              ordering and durability of writes to the platter does not
vices become more sophisticated, and can handle a large                                            matter, unless there is a crash. Therefore, the current in-
number of outstanding requests, the odds that a crash will                                         terface is overly constraining and limits I/O performance.
cause inconsistency increases. Thus, to advance beyond                                                Rather than requiring the disk to obey ordering and
the probabilistic approach, a system must include ma-                                              durability commands from the layer above, we propose
chinery to either avoid situations that lead to inconsis-                                          that the disk be freed to perform reads and writes in
tency, or be able to detect and recover when such oc-                                              the order that optimizes its scheduling and performance.
currences arise. We now describe one such approach:                                                Thus, the performance of the disk is optimized for the
optimistic crash consistency.                                                                      common case in which there is no crash.
                                                                                                 In-Memory
   Given that the file system must still be able to guaran-
                                                                                                    M:1                      M:2                 M:3
tee consistency and durability in the event of a crash, we
                                                                  On-Disk Journal
propose a minimal extension to the disk interface. With
                                                                                Jc:0               Jc:1                       Jc:2                 Jc:3
an asynchronous durability notification the disk informs             JM:0
                                                                             Ck(D:0,M:0)
                                                                                         JM:1
                                                                                                Ck(D:1,M:1)
                                                                                                              JM:2
                                                                                                                           Ck(D:2,M:2)
                                                                                                                                         JM:3
                                                                                                                                                Ck(D:3,M:3)

the upper-level client that a specific write request has          In-Place Checkpoint

completed and is now guaranteed to be durable. Thus
                                                                      D:0       M:0       D:1                        D:2                  D:3
there will be two notifications from the disk: first when
the disk has received the write and later when the write         Figure 6: Optimistic Journaling. The figure shows four
has been persisted. Some interfaces, such as Forced Unit         transactions in progress, involving writes to main memory, the
Access (FUA), provide a single, synchronous durability           on-disk journal, and to in-place checkpoints on disk. A rect-
notification: the drive receives the request and indicates       angle block indicates that the file system has been notified that
completion when the request has been persisted [14, 37].         the write has been durably completed. Cloud-shaped blocks in-
Tagged Queuing allows a limited number of requests to            dicate that the write has been initiated, but the file system has
be outstanding at a given point of time [15, 37]. Unfor-         not yet been notified of its completion and it may or may not
tunately, many drives do not implement tagged queuing            be durable. Circles indicate dirty blocks in main memory that
                                                                 cannot be written until a previous write is durable; a dashed
and FUA reliably [18]. Furthermore, a request tagged
                                                                 line indicates the write it is dependent on. Finally, the solid
with FUA also implies urgency, prompting some im-                arrow indicates that the meta-data may refer to the data block.
plementations to force the request to disk immediately.
While a correct implementation of tagged queuing and             blocks are not reused across transactions (i.e., they are
FUA may suffice for optimistic crash consistency, we             not freed and re-allocated to different files or overwrit-
feel that an interface that decouples request acknowl-           ten); we will remove this assumption later (§4.3.5).
edgement from persistence enables higher levels of I/O              In Figure 6, four transactions are in progress: T x: 0,
concurrency and thus provides a better foundation on             T x: 1, and T x: 2, and T x: 3. At this point, the file sys-
which to build OptFS.                                            tem has received notification that T x: 0 is durable (i.e.,
                                                                 D: 0, JM : 0, and JC : 0) and so it is in the process of check-
4.2    Optimistic Consistency Properties                         pointing the metadata M: 0 to its in-place location on disk
As described in Section 2.2, ordered journaling mode             (note that M: 0 may point to data D: 0). If there is a crash
involves the following writes for each transaction: data         at this point, the recovery mechanism will properly re-
blocks, D, to in-place locations on disk; metadata blocks        play T x: 0 and re-initiate the checkpoint of M: 0. Since
to the journal, JM ; a commit block to the journal, JC ; and     T x: 0 is durable, the application that initiated these writes
finally, a checkpoint of the metadata to its in-place lo-        can be notified that the writes have completed (e.g., if it
cation, M. We refer to writes belonging to a particular          called fsync()). Note that the journal entries for T x: 0
transaction i with the notation : i; for example, the jour-      can finally be freed once the file system has been notified
naled metadata of transaction i is denoted JM : i.               that the in-place checkpoint write of M: 0 is durable.
   Ordered journaling mode ensures several properties.              The file system has also started transactions T x: 1
First, metadata written in transaction T x: i + 1 cannot be      through T x: 3; many of the corresponding disk writes
observed unless metadata from transaction T x: i is also         have been initiated, while others are being held in mem-
observed. Second, it is not possible for metadata to point       ory based on unresolved dependencies. Specifically, the
to invalid data. These properties are maintained by the          writes for D: 1, JM : 1, and JC : 1 have been initiated; how-
recovery process and how writes are ordered. If a crash          ever, D: 1 is not yet durable, and therefore the metadata
occurs after the transaction is properly committed (i.e.,        (M: 1), which may refer to it, cannot be checkpointed. If
D, JM , and JC are all durably written), but before M is         M: 1 were checkpointed at this point and a crash occurred
written, then the recovery process can replay the transac-       (with M: 1 being persisted and D: 1 not), M: 1 could be
tion so that M is written to its in-place location. If a crash   left pointing to garbage values for D: 1. If a crash occurs
occurs before the transaction is completed, then ordered         now, before D: 1 is durable, checksums added to the com-
journaling ensures that no in-place metadata related to          mit block of T x: 1 will indicate a mismatch with D: 1; the
this transaction was updated.                                    recovery process will not replay T x: 1, as desired.
   Optimistic journaling allows the disk to perform writes          T x: 2 is allowed to proceed in parallel with T x: 1; in
in any order it chooses, but ensures that in the case of a       this case, the file system has not yet been notified that the
crash, the necessary consistency properties are upheld for       journal commit block JC : 2 has completed; again, since
ordered transactions. To give the reader some intuition          the transaction is not yet durable, metadata M: 2 cannot
for why particular properties are sufficient for ordered         be checkpointed. If a crash occurs when JC : 2 is not yet
journaling semantics, we walk through the example in             durable, then the recovery process will detect a mismatch
Figure 6. For simplicity, we begin by assuming that data         between the data blocks and the checksums and not re-
play T x: 2. Note that D: 2 may be durable at this point         a specific transaction has occurred. Specifically, check-
with no negative consequences because no metadata is             sums can optimistically “enforce” two orderings: that
allowed to refer to it yet, and thus it is not reachable.        the journal commit block (JC ) persists only after meta-
   Finally, T x: 3 is also in progress. Even if the file sys-    data to the journal (JM ) and after data blocks to their in-
tem is notified that D: 3, JM : 3, and JC : 3 are all durable,   place location (D). This technique for ensuring metadata
the checkpoint of M: 3 cannot yet be initiated because es-       is durably written to the journal in its entirety has been
sential writes in T x: 1 and T x: 2 are not durable (namely,     referred to as transactional checksumming [23]; in this
D: 1 and JC : 2). T x: 3 cannot be made durable until all        approach, a checksum is calculated over JM and placed
previous transactions are guaranteed to be durable; there-       in JC . If a crash occurs during the commit process, the
fore, its metadata M: 3 cannot be checkpointed.                  recovery procedure can reliably detect the mismatch be-
                                                                 tween JM and the checksum in JC and not replay that
4.3     Optimistic Techniques                                    transaction (or any transactions following). To identify
The behavior of optimistic journaling described above            this particular instance of transactional checksumming
can be ensured with a set of optimistic techniques: in-          we refer to it as metadata transactional checksumming.
order journal recovery and release, checksums, back-                A similar, but more involved, version of data trans-
ground writes after notification, reuse after notification,      actional checksumming can be used to ensure that data
and selective data journaling. We now describe each.             blocks D are written in their entirety as part of the trans-
4.3.1   In-Order Journal Recovery                                action. Collecting these data blocks and calculating their
                                                                 checksums as they are dirtied in main memory is more
The most basic technique for preserving the correct or-          involved than performing the checksums over JM , but the
dering of writes after a crash occurs during the journal         basic idea is the same. With the data checksums and their
recovery process itself. The recovery process reads the          on-disk block addresses stored in JC , the journal recov-
journal to observe which transactions were made durable          ery process can abort transactions upon mismatch. Thus,
and it simply discards or ignores any write operations           data transactional checksums enable optimistic journal-
that occurred out of the desired ordering.                       ing to ensure that metadata is not checkpointed if the
   The correction that optimistic journaling applies is to       corresponding data was not durably written.
ensure that if any part of a transaction T x: i was not cor-
rectly or completely made durable, then neither transac-         4.3.4   Background Write after Notification
tion T x: i nor any following transaction T x: j where j > i
                                                                 An important optimistic technique ensures that the
is left durable. Thus, journal recovery must proceed in-
                                                                 checkpoint of the metadata (M) occurs after the preced-
order, sequentially scanning the journal and performing
                                                                 ing writes to the data and the journal (i.e., D, JM , and
checkpoints in-order, stopping at the first transaction that
                                                                 JC ). While pessimistic journaling guaranteed this behav-
is not complete upon disk. The in-order recovery process
                                                                 ior with a flush after JC , optimistic journaling explicitly
will use the checksums described below to determine if
                                                                 postpones the checkpoint write of metadata M until it
a transaction is written correctly and completely.
                                                                 has been notified that all previous transactions have been
4.3.2   In-Order Journal Release                                 durably completed. Note that it is not sufficient for M to
Given that completed, durable journal transactions define        occur after only JC ; D and JM must precede M as well
the write operations that are durable on disk, optimistic        since optimistic journaling must ensure that the entire
journaling must ensure that journal transactions are not         transaction is valid and can be replayed if any of the in-
freed (or overwritten) until all corresponding checkpoint        place metadata M is written. Similarly, M: i + 1 must be
writes (of metadata) are confirmed as durable.                   postponed until all transactions T x: i have been durably
   Thus, optimistic journaling must wait until it has been       committed to ensure that M: i + 1 is not durable if M: i
notified by the disk that the checkpoint writes corre-           cannot be replayed. We note that M: i + 1 does not need
sponding to this transaction are durable. At this point,         to wait for M: i to complete, but must instead wait for the
optimistic journaling knows that the transaction need not        responsible transaction T x: i to be durable.
be replayed if the system crashes; therefore, the trans-            Checkpointing is one of the few points in the opti-
action can be released. To preserve the property that            mistic journaling protocol where the file system must
T x: i + 1 is made durable only if T x: i is durable, trans-     wait to issue a particular write until a specific set of
actions must be freed in order.                                  writes have completed. However, this particular wait-
                                                                 ing is not likely to impact performance because check-
4.3.3   Checksums                                                pointing occurs in the background. Subsequent writes
Checksums are a well-known technique for detecting               by applications will be placed in later transactions and
data corruption and lost writes [20, 31]. A checksum can         these journal updates can be written independently of any
also be used to detect whether or not a write related to         other outstanding writes; journal writes do not need to
wait for previous checkpoints or transactions. Therefore,        allocated to another file until the file system has been no-
even applications waiting for journal writes (e.g., by call-     tified by the disk that JMA′ : i has been durably written; at
ing fsync()) will not observe the checkpoint latency.            this point, the data block DA is “durably free.” When the
   For this reason, waiting for the asynchronous durabil-        file MB must be allocated a new data block, the optimistic
ity notification before a background checkpoint is funda-        file system allocates a “durably-free” data block that is
mentally more powerful than the pessimistic approach of          known to not be referenced by any other files; finding a
sending an ordering command to the disk (i.e., a cache           durably-free data block is straight-forward given the pro-
flush). With a traditional ordering command, the disk            posed asynchronous durability notification from disks.
is not able to postpone checkpoint writes across multi-              Performing reuse only after notification is unlikely to
ple transactions. On the other hand, the asynchronous            cause the file system to wait or to harm performance.
durability notification command does not artificially con-       Unless the file system is nearly 100% full, there should
strain the ordering of writes and gives more flexibility to      always exist a list of data blocks that are known to be
the disk so that it can best cache and schedule writes; the      durably free; under normal operating conditions, the file
command also provides the needed information to the file         system is unlikely to need to wait to be informed by the
system so that it can allow independent writes to proceed        disk that a particular data block is available.
while appropriately holding back dependent writes.
                                                                 4.3.6   Selective Data Journaling
4.3.5   Reuse after Notification
                                                                 Our final optimistic technique selectively journals data
The preceding techniques were sufficient for handling            blocks that have been overwritten. This technique allows
the cases where blocks were not reused across transac-           optimistic journaling to provide data journaling consis-
tions. The difficulty occurs with ordered journaling be-         tency semantics instead of ordered journaling semantics.
cause data writes are performed to their in-place loca-             A special case of an update dependency occurs when
tions, potentially overwriting data on disk that is still ref-   a data block is overwritten in a file and the metadata for
erenced by durable metadata from previous transactions.          that file (e.g., size) must be updated consistently. Opti-
Therefore, additional optimistic techniques are needed           mistic journaling could handle this using reuse after noti-
to ensure that durable metadata from earlier transactions        fication: a new durably-free data block is allocated to the
never points to incorrect data blocks changed in later           file and written to disk (DB : j), and then the new meta-
transactions. This is a security issue: if user A deletes        data is journaled (JMB : j). The drawback of this approach
their file, and then the deleted block becomes part of user      is that the file system takes on the behavior of a copy-on-
B’s file, a crash should not lead to user A being able to        write file system and loses some of the locality benefits of
view user B’s data. This is also one of the update depen-        an update-in-place file system [21]; since optimistic jour-
dency rules required for Soft Updates [25], but optimistic       naling forces a durably-free data block to be allocated,
journaling enforces this rule with a different technique:        a file that was originally allocated contiguously and pro-
reuse after notification.                                        vided high sequential read and write throughput may lose
   To understand the motivation for this technique, con-         its locality for certain random-update workloads.
sider the steps when a data block DA is freed from one              If update-in-place is desired for performance, a dif-
file MA and allocated to another file, MB and rewritten          ferent technique can be used: selective data journaling.
with the contents DB . Depending on how writes are re-           Data journaling places both metadata and data in the
ordered to disk, a durable version of MA may point to the        journal and both are then updated in-place at checkpoint
erroneous content of DB .                                        time. The attractive property of data journaling is that in-
   This problem can be fixed with transactions as follows.       place data blocks are not overwritten until the transaction
First, the freeing of DA and update to MA , denoted MA′ ,        is checkpointed; therefore, data blocks can be reused if
is written as part of a transaction JMA′ : i; the allocation     their metadata is also updated in the same transaction.
of DB to MB is written in a later transaction as DB : i + 1      The disadvantage of data journaling is that every data
and JMB : i + 1. Pessimistic journaling ensures that JMA′ : i    block is written twice (once in the journal, JD , and once
occurs before DB : i + 1 with a traditional flush between        in its checkpointed in-place location, D) and therefore of-
every transaction. The optimistic techniques introduced          ten has worse performance than ordered journaling [22].
so far are not sufficient to provide this guarantee because         Selective data journaling allows ordered journaling to
the writes to DB : i + 1 in their in-place locations cannot      be used for the common case and data journaling only
be recovered or rolled back if MA′ is lost (even if there is     when data blocks are repeatedly overwritten within the
a checksum mismatch and transactions T x: i or T x: i + 1        same file and the file needs to maintain its original layout
are found to be incomplete).                                     on disk. In selective data journaling, the checkpoint of
   Optimistic journaling guarantees that JMA′ : i occurs be-     both D and M simply waits for durable notification of all
fore DB : i + 1 by ensuring that data block DA is not re-        the journal writes (JD , JM , and JC ).
                                                 In-Memory       D:3     M:3
                                                                                              sue flushes to force the issue. To separate these cases, we
                                                                                              believe two calls should be provided; an “ordering” sync,
 On-Disk Journal
                                                                                              osync(), guarantees ordering between writes, while a
                          Jc:1                        Jc:2                         Jc:3
    JD:1    JM:1
                       Ck(D:1,M:1)
                                   JD:2   JM:2
                                                   Ck(D:2,M:2)
                                                                  JD:3   JM:3
                                                                                Ck(D:3,M:3)   “durability” sync, dsync(), ensures when it returns that
 In-Place Checkpoint
                                                                                              pending writes have been persisted.
                                                                                                 We now define and compare the guarantees given by
      D:1      M:1                  D:2      M:2
                                                                                              osync() and dsync(). Assume the user makes a se-
                                                                                              ries of writes W1 ,W2 , ...,Wn . If no osync() or dsync()
Figure 7: Optimistic Journaling: Selective Data Jour-                                         calls are made, there is no guarantee as to file-system
naling. The figure shows that selective data journaling may                                   state after a crash: any or all of the updates may be lost,
be used when transactions involve overwriting in-place data.                                  and updates may be applied out of order, i.e., W2 may be
Data blocks are now placed in the journal and checkpointed                                    applied without W1 .
after the transaction is committed.                                                              Now consider when every write is followed by
   Figure 7 shows an example of how selective data jour-                                      dsync(), i.e., W1 , d1 ,W2 , d2 , ...,Wn , dn . If a crash hap-
naling can be used to support overwrite, in particular the                                    pens after di , the file system will recover to a state with
case where blocks are reused from previous transactions                                       W1 ,W2 , ...,Wi applied.
without clearing the original references to those blocks.                                        If every write was followed by osync(), i.e.,
In this example, data blocks for three files have been                                        W1 , o1 ,W2 , o2 , ...,Wn , on , and a crash happens after oi , the
overwritten in three separate transactions.                                                   file system will recover to a state with W1 ,W2 , ...,Wi−k
   The first transaction illustrates how optimistic order-                                    applied, where the last k writes had not been made
ing ensures that durable metadata does not point to                                           durable before the crash. We term this eventual durabil-
garbage data. After the file system has been notified of                                      ity. Thus osync() provides prefix semantics [36], en-
the durability of T x: 1 (specifically, of JD : 1, JM : 1, and                                suring that users always see a consistent version of the
JC : 1), it may checkpoint both D: 1 and M: 1 to their in-                                    file system, though the data may be stale. Prior research
place locations. Because the file system can write M: 1                                       indicates that this is useful in many domains [7].
without waiting for a durable notification of D: 1, in the
case of crash it is possible for M: 1 to refer to garbage val-                                5 Implementation of OptFS
ues in D: 1; however, the recovery process will identify                                      We have implemented the Optimistic File System
this situation due to the checksum mismatch and replay                                        (OptFS) inside Linux 3.2, based on the principles out-
T x: 1 with the correct values for D: 1.                                                      lined before (§4), as a variant of the ext4 file system, with
   The second and third transactions illustrate how opti-                                     additional changes to the JBD2 journaling layer and vir-
mistic ordering ensures that later writes are visible only                                    tual memory subsystem.
if all earlier writes are visible as well. Specifically, D: 2
and M: 2 have been checkpointed, but only because both                                        5.1     Asynchronous Durability Notifications
T x: 2 and T x: 1 are both durable; therefore, a client can-                                  Since current disks do not implement the proposed asyn-
not see new contents for the second file without seeing                                       chronous durability notification interface, OptFS uses an
new contents for the first file. Furthermore, neither D: 3                                    approximation: durability timeouts. Durability timeouts
nor M: 3 (or any later transactions) can be checkpointed                                      represent the maximum time interval that the disk can
yet because not all blocks of its transaction are known to                                    delay committing a write request to the non-volatile plat-
be durable. Thus, selective data journaling provides the                                      ter. When a write for block A is received at time T by
desired consistency semantics while allowing overwrites.                                      the disk, the block must be made durable by time T +
                                                                                              TD . The value of TD is specific to each disk, and must be
4.4         Durability vs. Consistency                                                        exported by the disk to higher levels in the storage stack.
Optimistic journaling uses an array of novel techniques                                          Upon expiration of the time interval TD , OptFS consid-
to ensure that writes to disk are properly ordered, or that                                   ers the block to be durable; this is equivalent to the disk
enough information exists on disk to recover from an un-                                      notifying OptFS after TD seconds. Note that to account
timely crash when writes are issued out of order; the                                         for other delays in the I/O subsystem, TD is measured
result is file-system consistency and proper ordering of                                      from the time the write is acknowledged by the disk, and
writes, but without guarantees of durability. However,                                        not from the time the write is issued by the file system.
some applications may wish to force writes to stable stor-                                       This approximation is limiting; TD might overestimate
age for the sake of durability, not ordering. In this case,                                   the durability interval, leading to performance problems
something more than optimistic ordering is needed; the                                        and memory pressure; TD might underestimate the dura-
file system must either wait for such writes to be per-                                       bility interval, comprising consistency. OptFS errs to-
sisted (via an asynchronous durability notification) or is-                                   wards safety and sets TD to be 30 seconds.
5.2    Handling Data Blocks                                     fore freeing up journal blocks. Under memory pressure,
OptFS does not journal all data blocks: newly allocated         OptFS may need to free memory buffers of checkpoint
data blocks are only checksummed; their contents are not        blocks that have been issued to the disk and are waiting
stored in the journal. This complicates journal recov-          for durability timeouts. In such cases, OptFS issues a
ery as data-block contents may change after the check-          disk flush, ensuring the durability of checkpoint blocks
sum was recorded. Due to selective data journaling, a           that have been acknowledged by the disk. This allows
data block that is not journaled (as it is newly allocated)     OptFS to clean journal blocks belonging to some check-
in one transaction might be overwritten in the following        pointed transactions and free associated memory buffers.
transaction and therefore journaled. For example, con-          Checksums: OptFS checksums data blocks using the
sider data block D with content A belonging to T x1 . The       same CRC32 algorithm used for metadata. A tag is cre-
checksum A will be recorded in T x1 . D is overwritten by       ated for each data block which stores the block number
T x2 , with content B. Though this sequence is valid, the       and its checksum. Data tags are stored in the descriptor
checksum A in T x1 will not match the content B in D.           blocks along with tags for metadata checksums.
   This necessitates individual block checksums, since
                                                                Background Write after Notification: OptFS uses the
checksum mismatch of a single block is not a problem
                                                                VM subsystem to perform background writes. Check-
if the block belongs to a later valid transaction. In con-
                                                                point metadata blocks are marked as dirty and the
trast, since the frozen state of metadata blocks are stored
                                                                expiry field of each block is set to be T + TD (the disk
in the journal, checksumming over the entire set is suffi-
                                                                acknowledged the commit block at T ). T will reflect the
cient for metadata transactional checksums. We explain
                                                                time that the entire transaction has been acknowledged
how OptFS handles this during recovery shortly.
                                                                because the commit is not issued until the disk acknowl-
   OptFS does not immediately write out checksummed
                                                                edges data and metadata writes. The blocks are then
data blocks; they are collected in memory and written in
                                                                handed off to the VM subsystem.
large batches upon transaction commit. This increases
performance in some workloads.                                     During periodic background writes, the VM subsys-
                                                                tem checks if each dirty block has expired: if so,
5.3    Optimistic Techniques                                    the block is written out; otherwise the VM subsystem
We now describe the implementation of the optimistic            rechecks the block on its next periodic write-out cycle.
journaling techniques. We also describe how OptFS re-
                                                                Reuse after Notification: Upon transaction commit,
verts to more traditional mechanisms in some circum-
                                                                OptFS adds deleted data blocks to a global in-memory
stances (e.g., when the journal runs out of space).
                                                                list of blocks that will be freed after the durability time-
In-Order Journal Recovery: OptFS recovers transac-
                                                                out, TD . A background thread periodically frees blocks
tions in the order they were committed to the journal. A
                                                                with expired durability timeouts. Upon file-system un-
transaction can be replayed only if all its data blocks be-
                                                                mount, all list blocks are freed.
long to valid transactions, and the checksum computed
over metadata blocks matches that in the commit block.             When the file system runs out of space, if the reuse list
   OptFS performs recovery in two passes: the first pass        contains blocks, a disk flush is issued; this ensures the
linearly scans the journal, compiling a list of data blocks     durability of transactions which freed the blocks in the
with checksum mismatches and the first journal transac-         list. These blocks are then set to be free. We expect that
tion that contained each block. If a later valid transaction    these “safety” flushes will be used infrequently.
matches the block checksum, the block is deleted from           Selective Data Journaling: Upon a block write, the
the list. At the end of the scan, the earliest transaction in   block allocation information (which is reflected in New
the list is noted. The next pass performs journal recovery      state of the buffer head) is used to determine whether
until the faulting transaction, thus ensuring consistency.      the block was newly allocated. If the write is an over-
   OptFS journal recovery might take longer than ext4           write, the block is journaled as if it was metadata.
recovery since OptFS might need to read data blocks off
non-contiguous disk locations while ext4 only needs to
read the contiguous journal to perform recovery.                6 Evaluation
In-Order Journal Release: When the virtual memory
(VM) subsystem informs OptFS that checkpoint blocks             We now evaluate our prototype implementation of OptFS
have been acknowledged at time T , OptFS sets the trans-        on two axes: reliability and performance. Experiments
action cleanup time as T +TD , after which it is freed from     were performed on an Intel Core i3-2100 CPU with 16
the journal.                                                    GB of memory, a Hitachi DeskStar 7K1000.B 1 TB
   When the journal is running out of space, it may not         drive, and running Linux 3.2. The experiments were re-
be optimal to wait for the durability timeout interval be-      peatable; numbers reported are the average over 10 runs.
                                                                                            4        Ext4 (Flush)
                                         Crash points                                                                      301.68 Op/s
                                                                                                     Ext4 (No Flush)
      Workload     Delayed Blocks    Total Consistent                                       3        OptFS                                2839.68
                        Data           50         50                                                                                       Op/s




                                                                   Normalized Performance
       Append          JM , JC         50         50                                        2
                   Multiple blocks    100        100                                             106 MB/s     106 MB/s
                                                                                            1
                        Data           50         50
       Overwrite       JM , JC         50         50
                                                                                            0
                   Multiple blocks    100        100                                             Sequential   Sequential    Random        Create
                                                                                                   Write      Overwrite      Write         Files
Table 1: Reliability Evaluation. The table shows the to-                                    20
                                                                                                                                         21.80 Tx/s
tal number of simulated crashpoints, and the number of crash-                               16
points resulting in a consistent state after remount.                                       12                               825.81
                                                                                                                              Op/s
6.1    Reliability                                                                          8
                                                                                                  1176.28      9671.90
To verify OptFS’s consistency guarantees, we build and                                      4      Op/s         Op/s

apply a crash-robustness framework to it under two syn-                                     0
                                                                                                 Fileserver   Webserver      Varmail      MySQL
thetic workloads. The first appends blocks to the end of a
file; the second overwrites blocks of an existing file; both    Figure 8: Performance Comparison. Performance is
issue osync() calls frequently to induce more ordering          shown normalized to ext4 ordered mode with flushes. The abso-
points and thus stress OptFS machinery.                         lute performance of ordered mode with flushes is shown above
   Crash simulation is performed by taking a workload           each workload. Sequential writes are to 80 GB files. 200K ran-
trace, reordering some requests (either by delaying the         dom writes are performed over a 10 GB file, with an fsync()
                                                                every 1K writes. The overwrite benchmark sequentially writes
write of a single data block, journal commit or metadata
                                                                over a 32 GB file. Createfiles uses 64 threads to create 1M
blocks, or multiple blocks chosen at random), creating a        files. Fileserver emulates file-server activity, using 50 threads
file-system image that contains a subset of those writes        to perform a sequence of creates, deletes, appends, reads, and
up until a particular crash point, and then recovering the      writes. Webserver emulates a multi-threaded web host server,
file system from that image.                                    performing sequences of open-read-close on multiple files plus
   Table 1 shows the results of 400 different crash sce-        a log file append, with 100 threads. Varmail emulates a multi-
narios. OptFS recovers correctly in each case to a file         threaded mail server, performing a sequence of create-append-
system with a consistent prefix of updates (§4.4).              sync, read-append-sync, reads, and deletes in a single direc-
                                                                tory. Each workload was run for 660 seconds. MySQL OLTP
6.2    Performance                                              benchmark performs 200K queries over a table with 1M rows.
We now analyze the performance of OptFS under a num-            even with flushes disabled, does not perform as well as
ber of micro- and macro-benchmarks. Figure 8 illustrates        OptFS since OptFS delays writing dirty blocks, issuing
OptFS performance under these workloads; details of the         them in large batches periodically or on commit; in con-
workloads are found in the caption.                             trast, the background threads in ext4 issue writes in small
   Micro-benchmarks: OptFS sequential-write perfor-             batches so as to not affect foreground activity.
mance is similar to ordered mode; however, sequential              Finally, we run the MySQL OLTP benchmark from
overwrites cause bandwidth to drop to half of that of or-       Sysbench [1] to investigate the performance on database
dered mode as OptFS writes every block twice. Ran-              workloads. OptFS performs 10x better than ordered
dom writes on OptFS are 3x faster than on ext4 ordered          mode with flushes, and 40% worse than ordered mode
mode, as OptFS converts random overwrites into sequen-          without flushes (due to the many in-place overwrites of
tial journal writes (due to selective data journaling).         MySQL, which result in selective data journaling).
   On the Createfiles benchmark, OptFS performs 2x bet-            Summary: OptFS significantly outperforms ordered
ter than ext4 ordered mode, as ext4 writes dirty blocks         mode with flushes on most workloads, providing the
in the background for a number of reasons (e.g., hitting        same level of consistency at considerably lower cost.
the threshold for the amount of dirty in-memory data),          On many workloads, OptFS performs as well as ordered
while OptFS consolidates the writes and issues them             mode without flushes, which offers no consistency guar-
upon commit. When we modified ext4 to stop the back-            antees. OptFS may not be suited for workloads which
ground writes, its performance was similar to OptFS.            consist mainly of sequential overwrites.
   Macro-benchmarks: We run the Filebench File-
server, Webserver, and Varmail workloads [17]. OptFS
                                                                6.3                         Resource consumption
performs similarly to ext4 ordered mode without flushes         Table 2 compares the resource consumption of OptFS
for Fileserver and Webserver.          Varmail’s frequent       and ext4 for a 660 second run of Varmail. OptFS con-
fsync() calls cause a significant number of flushes,            sumes 10% more CPU than ext4 ordered mode without
leading to OptFS performing 7x better than ext4. ext4,          flushes. Some of this overhead in our prototype can be
       500                                                                       File system              CPU %     Memory (MB)
       400
                                         ext4 without flushes          ext4 ordered mode with flushes      3.39       486.75
                                               OptFS                  ext4 ordered mode without flushes    14.86      516.03
Tx/s
       300
       200                                                                          OptFS                  25.32       749.4
       100                                 ext4 with flushes        Table 2: Resource Consumption. The table shows the av-
         0                                                          erage resource consumption by OptFS and ext4 ordered mode
             32 64   128                                     512
                                                                    for a 660 second run of Filebench Varmail. OptFS incurs addi-
                            Journal Size (MB)
                                                                    tional overhead due to techniques like delayed checkpointing.
Figure 9: Performance with Small Journals. The fig-
ure shows the variation in OptFS performance on the MySQL
                                                                                                 gedit                SQLite
OLTP benchmark as the journal size is varied. When OptFS                               ext4 w/o ext4 w/ OptFS ext4 w/o ext4 w/ OptFS
runs out of journal space, it issues flushes to safely checkpoint                        flush flush            flush flush
transactions. Note that even in this stress scenario, OptFS per-     Total crashpoints     50      50    50      100     100    100
formance is 5x better than ext4 ordered mode with flushes.             Inconsistent         7      0      0       73      0      0
                                                                         Old state         26      21    36       8      50      76
attributed to CRC32 data checksumming and the back-                     New state          17      29    14       19     50      24
ground thread which frees data blocks with expired dura-             Time per op (ms) 1.12        39.4 0.84 23.38        152    15.3
bility timeouts, though further investigation is required.
                                                                    Table 3: Case Study: Gedit and SQLite.                 The table
   OptFS delays checkpointing and holds metadata in                 shows the number of simulated crashpoints that resulted in a
memory longer, thereby increasing memory consump-                   consistent or inconsistent application state after remounting. It
tion. Moreover, OptFS delays data writes until trans-               also shows the time required for an application operation.
action commit time, increasing performance for some
workloads (e.g., Filebench Createfiles), but at the cost               To study the effectiveness of OptFS for this usage, we
of additional memory load.                                          modified gedit to use osync() instead of fsync(), thus
                                                                    ensuring order is preserved. We then take a block-level
6.4          Journal size                                           I/O trace when running under both ext4 (with and with-
When OptFS runs out of journal space, it issues flushes in          out flushes) and OptFS, and simulate a large number of
order to checkpoint transactions and free journal blocks.           crash points and I/O re-orderings in order to determine
To investigate the performance of OptFS in such a sit-              what happens during a crash. Specifically, we create
uation, we reduce the journal size and run the MySQL                a disk image that corresponds to a particular subset of
OLTP benchmark. The results are shown in Figure 9.                  writes taking place before a crash; we then mount the
Note that due to selective journaling, OptFS performance            image, run recovery, and test for correctness.
will be lower than that of ext4 without flushes, even                  Table 3 shows our results. With OptFS, the saved file-
with large journals. We find that OptFS performs well               name always points to either the old or new versions of
with reasonably sized journals of 512 MB and greater;               the data in their entirety; atomic update is achieved. In
only with smaller journal sizes does performance de-                a fair number of crashes, old contents are recovered, as
grade near the level of ext4 with flushes.                          OptFS delays writing updates; this is the basic trade-off
                                                                    OptFS makes, increasing performance but delaying dura-
7 Case Studies                                                      bility. With ext4 (without flush), a significant number
We now show how to use OptFS ordering guarantees to                 of crashpoints resulted in inconsistencies, including un-
provide meaningful application-level crash consistency.             mountable file systems and corrupt data. As expected,
Specifically, we study atomic updates in a text editor              ext4 (with flush) did better, resulting in new or old con-
(gedit) and logging within a database (SQLite).                     tents exactly as dictated by the fsync() boundary.
                                                                       The last row of Table 3 compares the performance of
7.1          Atomic Update within Gedit                             atomic updates in OptFS and ext4. OptFS delivers per-
Many applications atomically update a file with the fol-            formance similar to ext4 without flushes, roughly 40x
lowing sequence: first, create a new version of the file            faster per operation than ext4 with flushes.
under a temporary name; second, call fsync() on the
file to force it to disk; third, rename the file to the desired
                                                                    7.2    Temporary Logging in SQLite
file name, replacing the original atomically with the new           We now investigate the use of osync() in a database
contents (some applications issue another fsync() to                management system, SQLite. To implement ACID trans-
the parent directory, to persist the name change). The              actions, SQLite first creates a temporary log file, writes
gedit text editor, which we study here, performs this se-           some information to it, and calls fsync(). SQLite then
quence to update a file, ensuring that either the old or new        updates the database file in place, calls fsync() on the
contents are in place in their entirety, but never a mix.           database file, and finally deletes the log file. After a
crash, if a log file is present, SQLite uses the log file     plication developers: Featherstitch requires developers
for recovery (if necessary); the database is guaranteed to    to explicitly encapsulate sets of file-system operations
be recovered to either the pre- or post-transaction state.    into units called patchgroups and define dependencies
   Although SQLite transactions provide durability by         between them. Since osync() builds upon the famil-
default, its developers assert that many situations do not    iar semantics of fsync(), we believe it will be easier
require it, and that “sync” can be replaced with pure or-     for application developers to use.
dering points in such cases. The following is an excerpt         Other work on “rethinking the sync” [19] has a sim-
from the SQLite documentation [29]:                           ilar flavor to our work. In that work, the authors clev-
                                                              erly note that disk writes only need to become durable
     As long as all writes that occur before the sync
                                                              when some external entity can observe said durability;
     are completed before any write that happens
                                                              thus, by delaying persistence until such externalization
     after the sync, no database corruption will oc-
                                                              occurs, huge gains in performance can be realized. Our
     cur. [...] the database will at least continue
                                                              work is complimentary, in that it reduces the number of
     to be consistent, and that is what most people
                                                              such durability events, instead enforcing a weaker (and
     care about (emphasis ours).
                                                              higher performance) ordering among writes, but avoid-
   We now conduct the same consistency and perfor-            ing the complexity of implementing dependency tracking
mance tests for SQLite. With a small set of tables            within the OS. In cases where durability is required (i.e.,
(≈30KB in size), we create a transaction to move records      applications use dsync() and not osync()), optimistic
from one half the tables to the other half. After a simu-     journaling does not provide much gain; thus, Nightingale
lated disk image that corresponds to a particular crash-      et al.’s work still can be of benefit therein.
point is mounted, if consistent, SQLite should convert           More recently, Chidambaram et al. implement
the database to either the pre-transaction (old) or post-     NoFS [6], which removes the need for any ordering to
transaction (new) state. The results in Table 3 are similar   disk at all, thus providing excellent performance. How-
to the gedit case-study: OptFS always results in a consis-    ever, a lack of ordered writes means certain kinds of
tent state, while ext4, without flushes, does not. OptFS      crashes can lead to a recovered file system that is con-
performs 10x better than ext4 with flushes.                   sistent, but that contains data from partially completed
                                                              operations. As a result, NoFS cannot implement atomic
8 Related Work                                                actions, such as rename(). Atomic rename is critical
A number of approaches to building higher performing          in the following update sequence: create temporary file;
file systems relate to our work on OptFS and optimistic       write temporary file with the entire contents of the old
crash consistency. For example, Soft Updates [11] shows       file, plus updates; persist temporary file via fsync();
how to carefully order disk updates so as to never leave      atomically rename temporary file over old file. At the
an on-disk structure in an inconsistent form. In contrast     end of this sequence, the file should exist in either the old
with OptFS, FreeBSD Soft Updates issues flushes to im-        state or the new state with all the updates. If rename()
plement fsync() and ordering (although the original           is not atomic, or operations can be re-ordered, the en-
work modified the SCSI driver to avoid issuing flushes).      tire file could be lost due to an inopportune crash (some-
   Given the presence of asynchronous durability noti-        thing that has been observed in deployment [9]). OptFS,
fications, Soft Updates could be modified to take ad-         in contrast, realizes many of the benefits of the NoFS
vantage of such signals. We believe doing so would            approach but still delivers meaningful semantics upon
be more challenging than modifying journaling file sys-       which higher-level application semantics can be built.
tems; while journaling works at the abstraction level of         Some real systems provide different flavors of
metadata and data, Soft Updates works directly with file-     fsync() to applications. For example, on Mac OS X,
system structures, significantly increasing its complexity.   fsync() only ensures that writes have been issued to
   Our work is similar to that of Frost et al.’s work on      the disk from main memory; as the man page states, if
Featherstitch [10], which provides a generalized frame-       an application desires durability, it should call fcntl()
work to order file-system updates, in either a soft-          with the F FULLSYNC flag. While the latter is identical
updating or journal-based approach. Our work instead          to dsync(), the former is not equivalent to osync();
focuses on delayed ordering for journal commits; some         importantly, simply flushing a write to the disk cache
of our techniques could increase the journal performance      does not provide any higher-level ordering guarantees
observed in their work.                                       and thus does not serve as a suitable primitive atop which
   Featherstitch provides similar primitives for ordering     to build application consistency; in contrast, as we have
and durability: pg depend() is similar to osync(),            shown, osync() is quite apt for this usage scenario.
while pg sync() is similar to dsync(). The main dif-             Prior work in distributed systems aims to measure
ference lies in the amount of work required from ap-          eventual consistency (among replicas) and understand
                                        Consistency
                                        Performance               9 Conclusion
                                        Availability
                                        Durability                We present optimistic crash consistency, a new approach
                                        TX support                to crash consistency in journaling file systems that uses
                                        Flush-free                a range of novel techniques to obtain both a high level
  Technique                             Optimistic
                                                   √√             of consistency and excellent performance in the com-
  File-system check                     L H L L √×                mon case. We introduce two new file-system primitives,
  Metadata journaling                   MM H H √× ×               osync() and dsync(), which decouple ordering from
  Data journaling                       HMH H √×√    ×            durability, and show that they can provide both high per-
  Soft Updates                          MM H H √×                 formance and meaningful semantics for application de-
  Copy-on-write                         HMH H      ×√
                                                   √ ×            velopers. We believe that such decoupling holds the key
  Backpointer-Based Consistency         MH H L √ ×√√              to resolving the constant tension between consistency
  Optimistic Crash Consistency          H H H H∗                  and performance in file systems.
Table 4: Consistency Techniques. The table compares                  The source code for OptFS can be obtained at:
various approaches to providing consistency in file systems.      http://www.cs.wisc.edu/adsl/Software/optfs.
Legend: L – Low, M – Medium, H – High. H∗ indicates that op-      We hope that this will encourage adoption of the
timistic crash consistency can provide immediate durability, if   optimistic approach to consistency.
required, using dsync(). Note that only optimistic crash con-
sistency provides support for transactions, immediate durabil-
ity on demand, and high performance while eliminating flushes     Acknowledgments
in the common case.                                                  We thank James Mickens (our shepherd), the anony-
why it works in practice, similar to how probabilistic            mous reviewers, and Eddie Kohler for their insightful
crash consistency seeks to understand how file systems            comments. We thank Mark Hill, Michael Swift, Ric
maintain crash consistency without flushes. Yu et al. pro-        Wheeler, Joo-young Hwang, Sankaralingam Panneersel-
vide metrics to measure consistency among replicas and            vam, Mohit Saxena, Asim Kadav, Arun Kumar, and the
show that such quantification allows replicated services          members of the ADSL lab for their feedback. We thank
to make useful trade-offs [39]. Bailis et al. bound data          Sivasubramanian Ramasubramanian for helping us run
staleness in eventually consistent distributed systems and        the probabilistic crash consistency experiments. This
explain why eventual consistency works in practice [2].           material is based upon work supported by the NSF un-
   Finally, we compare optimistic crash consistency with          der CNS-1319405 and CNS-1218405 as well as dona-
other approaches to providing crash consistency in file           tions from EMC, Facebook, Fusion-io, Google, Huawei,
systems. We have discussed some of them, such as                  Microsoft, NetApp, Sony, and VMware. Any opin-
Soft Updates and Backpointer-Based Consistency, in de-            ions, findings, and conclusions, or recommendations ex-
tail. Table 4 broadly compares various consistency tech-          pressed herein are those of the authors and do not neces-
niques on aspects such as durability, availability, and           sarily reflect the views of the NSF or other institutions.
performance. Transactional support indicates support
for multi-block atomic operations such as rename().
Flush-free indicates that the technique does not issue
flushes in the common case.
   We observe that there are four approaches which
are optimistic: the file-system check [5], Soft Updates,
Backpointer-Based Consistency, and Optimistic Crash
Consistency. Soft Updates issues flushes to ensure or-
dering, and hence is not flush-free. While the file-system
check and backpointer-based consistency are completely
flush-free, they cannot force writes to disk, and hence
have eventual durability. Pessimistic approaches like
journaling and copy-on-write employ flushes to provide
transactional support and high levels of consistency and
durability; however, they cannot implement ordering
without flushes, and hence offer reduced performance on
workloads with many ordering points. Due to osync()
and dsync(), optimistic crash consistency can persist
writes on demand (leading to immediate durability), and
yet remain flush-free when only ordering is required.
References                                                               [20] Swapnil Patil, Anand Kashyap, Gopalan Sivathanu, and Erez
                                                                              Zadok. I3 FS: An In-kernel Integrity Checker and Intrusion de-
 [1] Alexey Kopytov. SysBench: a system performance bench-                    tection File System. In LISA ’04, pages 69–79, Atlanta, GA,
     mark. http://sysbench.sourceforge.net/index.                             November 2004.
     html, 2004.                                                         [21] Zachary N. J. Peterson. Data Placement for Copy-on-write Using
 [2] Peter Bailis, Shivaram Venkataraman, Michael J. Franklin,                Virtual Contiguity. Master’s thesis, U.C. Santa Cruz, 2002.
     Joseph M. Hellerstein, and Ion Stoica. Probabilistically Bounded    [22] Vijayan Prabhakaran, Andrea C. Arpaci-Dusseau, and Remzi H.
     Staleness for Practical Partial Quorums. PVLDB, 5(8):776–787,            Arpaci-Dusseau. Analysis and Evolution of Journaling File Sys-
     2012.                                                                    tems. In USENIX ’05, pages 105–120, Anaheim, CA, April 2005.
 [3] Steve Best. JFS Overview. http://jfs.sourceforge.                   [23] Vijayan Prabhakaran, Lakshmi N. Bairavasundaram, Nitin
     net/project/pub/jfs.pdf, 2000.                                           Agrawal, Haryadi S. Gunawi, Andrea C. Arpaci-Dusseau, and
 [4] John S. Bucy, Jiri Schindler, Steven W. Schlosser, and Gregory R.        Remzi H. Arpaci-Dusseau. IRON File Systems. In SOSP ’05,
     Ganger. The DiskSim Simulation Environment Version 4.0 Ref-              pages 206–220, Brighton, UK, October 2005.
     erence Manual. Technical Report CMU-PDL-08-101, Carnegie            [24] Margo Seltzer, Peter Chen, and John Ousterhout. Disk Schedul-
     Mellon University, May 2008.                                             ing Revisited. In USENIX Winter ’90, pages 313–324, Washing-
 [5] Remy Card, Theodore Ts’o, and Stephen Tweedie. Design and                ton, DC, January 1990.
     Implementation of the Second Extended Filesystem. In First          [25] Margo I. Seltzer, Gregory R. Ganger, M. Kirk McKusick,
     Dutch International Symposium on Linux, Amsterdam, Nether-               Keith A. Smith, Craig A. N. Soules, and Christopher A. Stein.
     lands, December 1994.                                                    Journaling Versus Soft Updates: Asynchronous Meta-data Pro-
 [6] Vijay Chidambaram, Tushar Sharma, Andrea C. Arpaci-Dusseau,              tection in File Systems. In USENIX ’00, pages 71–84, San Diego,
     and Remzi H. Arpaci-Dusseau. Consistency Without Ordering.               CA, June 2000.
     In FAST ’12, pages 101–116, San Jose, CA, February 2012.            [26] Dick Sites. How Fast Is My Disk? Systems Seminar at the Uni-
 [7] James Cipar, Greg Ganger, Kimberly Keeton, Charles B Mor-                versity of Wisconsin-Madison, January 2013. http://www.
     rey III, Craig AN Soules, and Alistair Veitch. LazyBase: trading         cs.wisc.edu/event/how-fast-my-disk.
     freshness for performance in a scalable database. In EuroSys ’12,   [27] David A. Solomon. Inside Windows NT. Microsoft Programming
     pages 169–182, Bern, Switzerland, April 2012.                            Series. Microsoft Press, 2nd edition, May 1998.
 [8] Jonathan Corbet. Barriers and Journaling Filesystems. http://       [28] Jon A. Solworth and Cyril U. Orji. Write-Only Disk Caches. In
     lwn.net/Articles/283161, May 2008.                                       SIGMOD ’90, pages 123–132, Atlantic City, NJ, May 1990.
 [9] Jonathan Corbet. That massive filesystem thread. http://            [29] SQLite Team.    How To Corrupt An SQLite Database
     lwn.net/Articles/326471/, March 2009.                                    File. http://www.sqlite.org/howtocorrupt.html,
                                                                              2011.
[10] Christopher Frost, Mike Mammarella, Eddie Kohler, Andrew
     de los Reyes, Shant Hovsepian, Andrew Matsuoka, and Lei             [30] Marting Steigerwald. Imposing Order. Linux Magazine, May
     Zhang. Generalized File System Dependencies. In SOSP ’07,                2007.
     pages 307–320, Stevenson, WA, October 2007.                         [31] Christopher A. Stein, John H. Howard, and Margo I. Seltzer.
[11] Gregory R. Ganger and Yale N. Patt. Metadata Update Perfor-              Unifying File System Protection. In USENIX ’01, pages 79–90,
     mance in File Systems. In OSDI ’94, pages 49–60, Monterey,               Boston, MA, June 2001.
     CA, November 1994.                                                  [32] Adan Sweeney, Doug Doucette, Wei Hu, Curtis Anderson, Mike
[12] Maurice Herlihy. Apologizing Versus Asking Permission: Op-               Nishimoto, and Geoff Peck. Scalability in the XFS File System.
     timistic Concurrency Control for Abstract Data Types. ACM                In USENIX 1996, San Diego, CA, January 1996.
     Transactions on Database Systems (TODS), 15(1):96–124, 1990.        [33] Theodore Tso. Re: [PATCH 0/4] (RESEND) ext3[34] barrier
[13] D. M. Jacobson and J. Wilkes. Disk Scheduling Algorithms                 changes. Linux Kernel Mailing List. http://article.
     Based on Rotational Position. Technical Report HPL-CSP-91-7,             gmane.org/gmane.comp.file-systems.ext4/
     Hewlett Packard Laboratories, 1991.                                      6662, May 2008.
[14] KnowledgeTek. Serial ATA Specification Rev. 3.0 Gold.               [34] Theodore Ts’o and Stephen Tweedie. Future Directions for the
     http://www.knowledgetek.com/datastorage/                                 Ext2/3 Filesystem. In FREENIX ’02, Monterey, CA, June 2002.
     courses/SATA_3.0-8.14.09(CD).pdf, 2009.                             [35] Stephen C. Tweedie. Journaling the Linux ext2fs File System. In
[15] Charles M. Kozierok. Overview and History of the SCSI Inter-             The Fourth Annual Linux Expo, Durham, North Carolina, May
     face. http://www.pcguide.com/ref/hdd/if/scsi/                            1998.
     over-c.html, 2001.                                                  [36] Yang Wang, Manos Kapritsos, Zuocheng Ren, Prince Mahajan,
[16] Hsiang-Tsung Kung and John T Robinson. On Optimistic Meth-               Jeevitha Kirubanandam, Lorenzo Alvisi, and Mike Dahlin. Ro-
     ods for Concurrency Control. ACM Transactions on Database                bustness in the Salus Scalable Block Store. In NSDI ’13, pages
     Systems (TODS), 6(2):213–226, 1981.                                      357–370, Lombard, IL, April 2013.

[17] Richard McDougall and Jim Mauro. Filebench. http://                 [37] Ralph O. Weber.   SCSI Architecture Model - 3 (SAM-
     sourceforge.net/apps/mediawiki/filebench/                                3). http://www.t10.org/ftp/t10/drafts/sam3/
     index.php?title=Filebench, 2005.                                         sam3r14.pdf, September 2004.

[18] Marshall Kirk McKusick. Disks from the Perspective of a File        [38] B. L. Worthington, G. R. Ganger, and Y. N. Patt. Scheduling
     System. Communications of the ACM, 55(11), 2012.                         Algorithms for Modern Disk Drives. In SIGMETRICS ’94, pages
                                                                              241–251, Nashville, TN, May 1994.
[19] Edmund B. Nightingale, Kaushik Veeraraghavan, Peter M Chen,
                                                                         [39] Haifeng Yu and Amin Vahdat. Design and Evaluation of a Con-
     and Jason Flinn. Rethink the Sync. In OSDI ’06, pages 1–16,
                                                                              tinuous Consistency Model for Replicated Services. In OSDI ’00,
     Seattle, WA, November 2006.
                                                                              San Diego, CA, October 2000.
