                   CORFU: A Shared Log Design for Flash Clusters
                          Mahesh Balakrishnan§, Dahlia Malkhi§, Vijayan Prabhakaran§
                                 Ted Wobber§, Michael Wei‡, John D. Davis§
              §                                                       ‡
                  Microsoft Research Silicon Valley                       University of California, San Diego


Abstract                                                              and geo-distribution [16]; and even a primary data store
                                                                      that leverages fast appends on underlying media. Flash
CORFU1 organizes a cluster of flash devices as a single,              is an ideal medium for implementing a scalable shared
shared log that can be accessed concurrently by multiple              log, supporting fast, contention-free random reads to the
clients over the network. The CORFU shared log makes                  body of the log and fast sequential writes to its tail.
it easy to build distributed applications that require                   One simple option for implementing a flash-based
strong consistency at high speeds, such as databases,                 shared log is to outfit a high-end server with an expensive
transactional key-value stores, replicated state machines,            PCI-e SSD (e.g., Fusion-io [2]), replicating it to handle
and metadata services. CORFU can be viewed as a dis-                  failures and scale read throughput. However, the result-
tributed SSD, providing advantages over conventional                  ing log is limited in append throughput by the bandwidth
SSDs such as distributed wear-leveling, network lo-                   of a single server. In addition, interposing bulky, general-
cality, fault tolerance, incremental scalability and geo-             purpose servers between the network and flash can cre-
distribution. A single CORFU instance can support up to               ate performance bottlenecks, leaving bandwidth under-
200K appends/sec, while reads scale linearly with clus-               utilized, and can also offset the power benefits of flash.
ter size. Importantly, CORFU is designed to work di-                  In contrast, clusters of small flash units have been shown
rectly over network-attached flash devices, slashing cost,            to be balanced, power-efficient and incrementally scal-
power consumption and latency by eliminating storage                  able [5]. Required is a distributed implementation of a
servers.                                                              shared log that can operate over such clusters.
                                                                         Accordingly, we present CORFU, a shared log ab-
                                                                      straction implemented over a cluster of flash units. In
1     Introduction                                                    CORFU, each position in the shared log is mapped to a
                                                                      set of flash pages on different flash units. This map is
Traditionally, system designers have been forced to                   maintained – consistently and compactly – at the clients.
choose between performance and safety when building                   To read a particular position in the shared log, a client
large-scale storage systems. Flash storage has the poten-             uses its local copy of this map to determine a correspond-
tial to dramatically alter this trade-off, providing persis-          ing physical flash page, and then directly issues a read
tence as well as high throughput and low latency. The                 to the flash unit storing that page. To append data, a
advent of commodity flash drives creates new opportu-                 client first determines the next available position in the
nities in the data center, enabling new designs that are              shared log – using a sequencer node as an optimization
impractical on disk or RAM infrastructure.                            for avoiding contention with other appending clients –
   In this paper, we posit that flash storage opens the door          and then writes data directly to the set of physical flash
to shared log designs within data centers, where hun-                 pages mapped to that position.
dreds of client machines append to the tail of a single
                                                                         CORFU’s client-centric design has two objectives.
log and read from its body concurrently. A shared log
                                                                      First, it ensures that the append throughput of the log is
is a powerful and versatile primitive for ensuring strong
                                                                      not a function of the bandwidth of any single flash unit.
consistency in the presence of failures and asynchrony.
                                                                      Instead, clients can append data to the log as fast as the
It can play many roles in a distributed system: a con-
                                                                      sequencer can assign them 64-bit tokens, i.e., new posi-
sensus engine for consistent replication; a transaction ar-
                                                                      tions in the log. Our current user-space sequencer runs
bitrator [13, 22, 25] for isolation and atomicity; an exe-
                                                                      at 200K tokens/s; assuming 4KB entries and two-way
cution history for replica creation, consistent snapshots,
                                                                      replication, this is sufficient to saturate the write band-
    1 CORFU stands for Clusters of Raw Flash Units, and also for an   width of a cluster of 50 Intel X25-V [1] drives, which in
island near Paxos in Greece.                                          turn can support a million random 4KB reads/sec. Es-
sentially, CORFU’s design decouples ordering from I/O,        ering from holes within a millisecond, and from crashed
extracting parallelism from the cluster for appends while     drives within 30 ms. Finally, we show that CORFU-Store
providing single-copy semantics for the shared log.           provides atomic operations at the speed of the log (40K
   Second, placing functionality at the clients reduces the   10-key multi-gets/s and 20K 10-key multi-puts/s, with
complexity, cost, latency and power consumption of the        4KB values), and that CORFU-SMR runs at 70K 512-
flash units. In fact, CORFU can operate over SSDs that        byte commands/s with 10 state machine replicas.
are attached directly to the network, eliminating general-       To summarize the contributions of this paper: we pro-
purpose storage servers from the critical path. In a par-     pose the first complete design and implementation of a
allel effort outside the scope of this paper, we have pro-    shared log abstraction over a flash cluster. This is also
totyped a network-attached flash unit on an FPGA plat-        the first distributed, shared log design where maximum
form; when used with the CORFU stack, this custom             append throughput is not a function of any single node’s
hardware provides the same throughput as a server-based       I/O bandwidth. We describe low-latency fault-tolerance
flash unit while using an order of magnitude less power       mechanisms for recovering from flash unit crashes and
and providing 33% lower latency on reads. Over a clus-        holes in the log. We present designs for a strongly con-
ter of such flash units, CORFU’s logging design acts as       sistent key-value store and a state machine replication
a distributed SSD, implementing functionality found in-       library that use CORFU. Finally, we evaluate CORFU
side conventional SSDs – such as wear-leveling – at clus-     throughput, latency, fault-tolerance and application per-
ter scale.                                                    formance on a 32-drive cluster of server-attached SSDs
   To realize these benefits of a client-centric design,      as well as an FPGA-based network-attached SSD.
CORFU needs to handle failures efficiently. When flash
units fail, clients must move consistently to a new map
from log positions to flash pages. CORFU achieves this        2    Motivation
via a reconfiguration mechanism (patterned after Vertical
Paxos [17]) capable of restoring availability within tens     As stated earlier, the key insight in this paper is that flash
of milliseconds on drive failures. A challenging failure      storage is an ideal medium for shared log designs. The
mode peculiar to a client-centric design involves ‘holes’     primary argument is that flash provides fast, contention-
in the log; a client can obtain a log position from the se-   free random reads, which enables designs where hun-
quencer for an append and then crash without complet-         dreds of clients can concurrently access a shared log.
ing the write to that position. To handle such situations,    However, the CORFU design of a shared, distributed log
CORFU provides a fast hole-filling primitive that allows      makes sense for other reasons, as well. We first offer a
other clients to complete an unfinished append (or mark       quick primer on the properties of flash storage, and then
the log position as junk) within a millisecond.               expand on the rationale for a shared, distributed log.
   CORFU’s target applications are infrastructure layers         Flash is read and written in increments of pages (typ-
that use the shared log to implement high-level inter-        ically of size 4KB). Before a page can be overwritten, it
faces. In this paper, we present two such applications.       must be erased; erasures can only occur at the granular-
CORFU-Store is a key-value store that supports atomic         ity of multi-page blocks (of size 256KB). Significantly,
multi-key puts and gets, fast consistent checkpointing        flash wears out or ages; as a page is erased and overwrit-
and low-latency geo-distribution; these are properties        ten, it becomes less reliable and is eventually unusable.
that are difficult to achieve on conventional partitioned     From a performance standpoint, overwriting a randomly
key-value stores. CORFU-SMR is a state machine repli-         selected flash page requires the surrounding block to un-
cation library where replicas propose commands by ap-         dergo an erase operation in the critical path, resulting in
pending to the log and execute commands by playing            poor random write speeds.
the log. In addition to these, we are currently building         At the level of a single drive, these problems are
a database, a virtual disk layer, and a reliable multicast    masked from applications by the Flash Translation Layer
mechanism over CORFU.                                         (FTL) within an SSD. SSDs implement a logical address
   We evaluate a CORFU implementation on a cluster of         space over raw flash, mapping logical addresses to phys-
32 Intel X25-V drives attached to servers, showing that it    ical flash pages. Since flash chips within an SSD cannot
saturates the aggregate storage bandwidth of the cluster      be easily replaced, the primary goal of the FTL is wear-
at speeds of 400K 4KB reads per second and nearly 200K        leveling: ensuring that all flash chips in the drive age, and
4KB appends per second over the network. We also              expire, in unison. FTLs also speed up random writes by
evaluate CORFU running over an FPGA-based network-            maintaining a free pool of extra, pre-erased blocks; how-
attached flash unit, showing that it performs end-to-end      ever, sequential writes are still significantly faster. Ad-
reads under 0.5 ms and cross-rack mirrored appends un-        ditionally, the OS can trim logical addresses on an SSD,
der 1 ms. We show that CORFU is capable of recov-             allowing the FTL to reclaim and reuse physical pages.
   As a result of these properties, the best data structure      tations, leveraging flash storage to provide better perfor-
for a single flash device is a log; it is always best to write   mance.
sequentially to flash. One reason is performance; ran-               The case for a distributed log: Existing flash-based
dom writes are slower than sequential writes both on raw         storage systems scale capacity and throughput by par-
flash and SSDs (as explained above). Depending on the            titioning data across multiple, independent logs, each of
design of the FTL, random writes can also cause signif-          which resides on a single flash drive. In a partitioned sys-
icantly greater wear-out than sequential writes. Accord-         tem, a total order no longer exists on all updates, making
ingly, almost all single-machine filesystems or databases        it difficult to support operations such as consistent snap-
designed for flash storage implement a log-structured de-        shots and atomic updates across partitions. Strongly con-
sign within each device; for example, FAWN [5] orga-             sistent operations are usually limited in size and scope
nizes each of its individual drives as a log.                    to a single partition. The high throughput/size ratio of
   CORFU extends this theme by organizing an entire              flash results in smaller drives, exacerbating this prob-
cluster of flash drives as a single log. This log is dis-        lem. Even if all the data involved in an atomic update
tributed across multiple drives and shared by multiple           miraculously resides on the same fine-grained partition,
clients. We now explain the rationale for these two de-          the throughput of updates on that data is limited by the
sign decisions.                                                  I/O capacity of the primary server of the partition.
   The case for a shared log: We stated earlier that a               Specific to flash storage, other problems arise when
shared log is a powerful building block for distributed          a system is partitioned into individual per-SSD logs. In
applications that require strong consistency. We now ex-         particular, the age distribution of the drives in the clus-
pand on this point by describing the different ways in           ter is tightly coupled to the observed workload; skewed
which applications can use a fast, flash-based shared log.       workloads can age drives at different rates, resulting in
By ‘shared’, we mean that multiple clients can read and          unpredictable reliability and performance. For example,
append to the log concurrently over the network, regard-         a range of key-value pairs can become slower and less re-
less of how the log is implemented.                              liable than the rest of the key-value store if one of them
   Historically, shared log designs have appeared in a           frequently overwritten. Additionally, striping data across
diverse array of systems. QuickSilver [13, 22] and               SSDs of different ages can bottleneck performance at the
Camelot [25] used shared logs for failure atomicity and          oldest SSD in the stripe. Further, administrative policies
node recovery. LBRM [14] uses shared logs for recov-             may require all drives to be replaced together, in which
ery from multicast packet loss. Shared logs are also             case even wear-out is preferred. Conversely, specific pat-
used in distributed storage systems for consistent remote        terns of uneven wear-out may be preferred, to allow the
mirroring [16]. In such systems, CORFU can replace               oldest subset of drives to be replaced periodically.
disk-based shared logs, providing higher throughput and              A distributed log solves these problems by spread-
lower latency.                                                   ing writes across the cluster in controlled fashion, de-
   However, a flash-based shared log also enables new            coupling the age distribution of drives from the ob-
applications that are infeasible on disk-based infrastruc-       served workload; in effect, it implements distributed
ture. For instance, Hyder [7] is a recently proposed high-       wear-leveling. In addition, the entire cluster still func-
performance database designed around a flash-based               tions as a single log; as we show later, this makes it
shared log, where servers speculatively execute transac-         possible to implement strongly consistent operations at
tions by appending them to the shared log and then us-           cluster scale.
ing log order to decide commit/abort status. In fact, Hy-            Partitioning is ultimately necessary for achieving
der was the original motivating application for CORFU            scale; we do not contend this point. However, modern
and is currently being implemented over our code base.           systems choose to create very fine-grained partitions, at
While the original Hyder paper included a brief design           the level of a single drive or a single array attached to a
for a flash-based shared log, this was never implemented;        server. CORFU instead allows an entire cluster to act as
later in this paper, we describe how – and why – CORFU           a single, coarse-grained partition.
departs significantly from the Hyder proposal.
   Interestingly, a shared log can also be used as a con-
sensus engine, providing functionality identical to con-         3    Design and Implementation
sensus protocols such as Paxos (a fact hinted at by the
name of our system). Used in this manner, CORFU                  The setting for CORFU is a data center with a large num-
provides a fast, fault-tolerant service for imposing and         ber of application servers (which we call clients) and a
durably storing a total order on events in a distributed         cluster of flash units (see Figure 1). Our goal is to pro-
system. From this perspective, CORFU can be used as              vide applications running on the clients with a shared log
a drop-in replacement for disk-based Paxos implemen-             abstraction implemented over the flash cluster.
                                                                      append(b)        Append an entry b and return
       Application                   Database
                                     Key-Value Store                                   the log position ` it occupies
                                     Replicated State Machine         read(`)          Return entry at log position `
      Corfu Library                  Metadata Service                 trim(`)          Indicate that no valid data exists
 Clients access flash units          Virtual Disk
                                                                                       at log position `
                                     …
 directly over the network                                            fill(`)          Fill log position ` with junk
 via the Corfu library
                                  Append
                                                                   Figure 2: API exposed by CORFU to applications
                 Read


       0 1 2 3 4 5 6 7 8 -             -   -   -   -   -   -           available logical position on the log for new data.
  Each log position is mapped                                     • A replication protocol to write a log entry consis-
  to flash pages in the cluster
                                                                    tently on multiple flash pages.

                                                                   These three functions – combined with the ability of
                                                                clients to read and write directly to the address space of
         Cluster of Network-Attached Flash Units                each flash unit – are sufficient to support a shared log
                                                                abstraction. To read data at a specific log position, the
Figure 1: CORFU presents applications running on                client-side library uses the mapping function to find the
clients with the abstraction of a shared log, implemented       appropriate flash page, and then directly issues a read to
over a cluster of flash units by a client-side library.         the device where the flash page is located. To append
                                                                data, a client finds the tail position of the log, maps it
                                                                to a set of flash pages, and then initiates the replication
   Our design for this shared log abstraction is driven by a    protocol that issues writes to the appropriate devices.
single imperative: to keep flash units as simple, inexpen-         Accordingly, the primary challenges in CORFU re-
sive and power-efficient as possible. We achieve this goal      volve around implementing these three functions in an
by placing all CORFU functionality at the clients and           efficient and fault-tolerant manner. Crucially, these
treating flash units as passive storage devices. CORFU          functions have to provide single-copy semantics for the
clients read and write directly to the address space of         shared log even when flash units fail and clients crash.
each flash unit, coordinating with each other to ensure            In this section, we first describe the assumptions made
single-copy semantics for the shared log. Individual flash      by CORFU about each flash unit. We then describe
units do not initiate communication, are unaware of other       CORFU’s implementation of the three functions de-
flash units, and do not participate actively in replication     scribed above.
protocols. CORFU does require specific functionality
from flash units, which we discuss shortly.
   Accordingly, CORFU is implemented as a client-side           3.1      Flash Unit Requirements
library that exposes a simple API to applications, shown        The most basic requirement of a flash unit is that it sup-
in Figure 2. The append interface adds an entry to the          port reads and writes on an address space of fixed-size
log and returns its position. The read interface accepts        pages. We use the term ‘flash page’ to refer to a page
a position in the log and returns the entry at that posi-       on this address space; however, the flash unit is free to
tion. If no entry exists at that position, an error code is     expose a logical address space where logical pages are
returned. The application can perform garbage collection        mapped internally to physical flash pages, as a conven-
using trim, which indicates to CORFU that no valid data         tional SSD does. The flash unit is expected to detect and
exists at a specific log position. Lastly, the application      re-map bad blocks in this address space.
can fill a position with junk, ensuring that it cannot be          To provide single-copy semantics for the shared
updated in future with a valid value.                           log, CORFU requires ‘write-once’ semantics on the
   CORFU’s task of implementing a shared log abstrac-           flash unit’s address space. Reads on pages that have
tion with this API over a cluster of flash units – each of      not yet been written should return an error code (er-
which exposes a separate address space – involves three         ror unwritten). Writes on pages that have already
functions:                                                      been written should also return an error code (er-
  • A mapping function from logical positions in the            ror overwritten). In addition to reads and writes, flash
    log to flash pages on the cluster of flash units.           units are also required to expose a trim command, allow-
                                                                ing clients to indicate that the flash page is not in use
  • A tail-finding mechanism for finding the next               anymore.
                                                                       round-robin: in the example in Figure 3, log position 0
                                                   •F0    0:20K        is mapped to F0 : 0, position 1 is mapped to F1 : 0, po-
 Example Projection               0 – 40K         •F1    0:20K        sition 2 back to F0 : 1, and so on. Any function can be
 Range [0 – 40K) is                                                    used as long as it is deterministic given a list of extents
 mapped to F0 and F1.
                                                   •F2    0:20K        and a log position. The example above maps each log
 Range [40K – 80K) is                                                  position to a single flash page; for replication, each ex-
 mapped to F2 and F3.
                               40K – 80K           •F3    0:20K
                                                                       tent is associated with a replica set of flash units rather
                                                                       than just one unit. For example, for two-way replica-
                                                                 40K
                                                                       tion the extent F0 : 0 : 20K would be replaced by
   0         1   2     3   4   5        .          .     40K      +1   F0 /F00 : 0 : 20K and the extent F1 : 0 : 20K would
                                                                       be replaced by F1 /F1 : 0 : 20K.
      F0         F1                          F2            F3             Accordingly, to map a log position to a set of flash
                                                                       pages, the client first consults its projection to determine
      0          1                           40K           40K+1
                                                                       the right list of extents for that position; in Figure 3, posi-
      2          3                           40K+2         40K+3       tion 45K in the log maps to extents on units F2 to F3 . It
                                                                       then computes the log position relative to the start of the
      ...        ...                         ...           ...
                                                                       range; in the example, this is 5K. Using this relative log
      40K-2      40K-1                       80K-2         80K-1       position, it applies the deterministic function on the list
                                                                       of extents to determine the flash pages to use. With the
                                                                       round-robin function and the example projection above,
Figure 3: Example projection that maps different ranges                the resulting page would be F2 : 2500.
of the shared log onto flash unit extents.
                                                                          By mapping log positions to flash pages, a projection
                                                                       essentially provides a logical address space implemented
   In addition, flash units are required to support a ‘seal’           over a cluster of flash units. Clients can read or write to
command. Each incoming message to a flash unit is                      positions in this address space by using the projection to
tagged with an epoch number. When a particular epoch                   determine the flash pages to access. Since CORFU or-
number is sealed at a flash unit, it must reject all subse-            ganizes this address space as a log (using a tail-finding
quent messages sent with an epoch equal or lower to the                mechanism which we describe shortly), clients end up
sealed epoch. In addition, the flash unit is expected to               writing only to the last range of positions in the projec-
send back an acknowledgment for the seal command to                    tion ([40K, 80K) in the example); we call this the active
the sealing entity, including the highest page offset that             range in the projection.
has been written on its address space thus far.
   These requirements – write-once semantics and seal-
ing – are sufficient to ensure CORFU correctness. They                 3.2.1   Changing the mapping
are also enough to ensure efficient appends and reads.                 In a sense, projections are similar to classical views. All
However, for efficient garbage collection on general-                  operations on flash units – reads, writes, trims – are is-
purpose workloads (in terms of network/storage band-                   sued by clients within the context of a single projection.
width and flash erase cycles), CORFU requires that the                 When some event occurs that necessitates a change in the
flash unit expose an infinite address space. We explain                mapping – for example, when a flash unit fails, or when
this last requirement in detail when we discuss garbage                the tail of the log moves past the current active range –
collection and the implementation of flash units.                      a new projection has to be installed on all clients in the
                                                                       system. In effect, each client observes a totally ordered
3.2         Mapping in CORFU                                           sequence of projections as the position-to-page mapping
                                                                       evolves over time; we call a projection’s position in this
Each CORFU client maintains a local, read-only replica                 sequence its epoch. When an operation executes in the
of a data structure called a projection that carves the                context of a projection, all the messages it generates are
log into disjoint ranges. Each such range is mapped to                 tagged with the projection’s epoch.
a list of extents within the address spaces of individual                 As with conventional view change protocols, all par-
flash units. Figure 3 shows an example projection, where               ticipants – in this case, the CORFU clients – must move
range [0, 40K) is mapped to extents on units F0 and F1 ,               consistently to a new projection when a change occurs.
while [40K, 80K) is mapped to extents on F2 and F3 .                   The new projection should correctly reflect all activity
   Within each range in the log, positions are mapped to               that was successfully completed in any previous projec-
flash pages in the corresponding list of extents via a sim-            tion; i.e., reads must reflect writes and trims that com-
ple, deterministic function. The default function used is              pleted in older projections. Further, any activity in-flight
during the view change must be aborted and retried in the
new projection.                                                               •F0/F1   0:20K                 •F0/F1   0:20K
   To achieve these properties, CORFU uses a simple,               0 – 40K    •F2/F3   0:20K      0 – 40K    •F2/F3   0:20K
auxiliary-driven reconfiguration protocol. The auxiliary                      •F4/F5   0:20K                 •F4/F5   0:20K
is a durably stored sequence of projections in the system,                    •F6/F7   0:20K                 •F7/F8   0:20K
                                                                  40K – 80K                      40K – 80K
where the position of the projection in the sequence is
equivalent to its epoch. Clients can read the ith entry                        (A)                            (C)
in the auxiliary (getting an error if no such entry exists),
or write the ith entry (getting an error if an entry already
exists at that position). The auxiliary can be implemented                  •F0/F1     0:20K               •F0/F1 0:20K
in multiple ways: on a conventional disk volume, as a              0 – 40K  •F2/F3     0:20K      0 – 40K  •F2/F3 0:20K
Paxos state machine, or even as a CORFU instance with                       •F4/F5      0:5K               •F4/F5 0:20K
a static, never-changing projection.                              40K – 50K •F7         0:5K     40K – 80K •F7/F8   0:20K
   Auxiliary-driven reconfiguration involves two distinct                   •F4/F5     5K:20K              •F9/F10 0:20K
steps:                                                            50K – 80K •F7/F8     5K:20K      80K -   •F11/F12 0:20K
   1. Sealing the current projection: When a client Cr                                             120K

decides to reconfigure the system from the current pro-                        (B)                            (D)
jection Pi to a new projection Pi+1 , it first seals Pi ; this
involves sending a seal command to a subset of the flash
units in Pi . A flash unit in Pi has to be sealed only if
                                                                 Figure 4: Sequence of projections: When F6 fails in (A)
a log position mapped to one of its flash pages by Pi is
                                                                 with the log tail at 50K, clients move to (B) in order to
no longer mapped to the same page by Pi+1 . In prac-
                                                                 replace F6 with F8 for new appends. Once old data on F6
tice, this means that only a small subset of flash units
                                                                 is rebuilt, F8 is used in (C) for reads on old data as well.
have to be sealed, depending on the reason for reconfig-
                                                                 When the log tail goes past 80K, clients add capacity by
uration. Sealing ensures that flash units will reject in-
                                                                 moving to (D).
flight messages – writes as well as reads – sent to them
in the context of the sealed projection. When clients re-
ceive these rejections, they realize that the current pro-       to F4 /F5 and F6 /F7 . When F6 fails with the log tail at
jection has been sealed, and wait for a new projection to        position 50K, CORFU moves to projection (B) immedi-
be installed; if this does not happen, they time out and         ately, replacing F6 with F8 for new appends beyond the
initiate reconfiguration on their own. The reconfiguring         current tail of the log, while servicing reads in the log
client Cr receives back acknowledgements to the seal             range [40K, 50K) with the remaining mirror F7 .
command from the flash units, which include the high-               Once F6 is completely rebuilt on F8 (by copying
est offsets written on those flash units thus far. Using         entries from F7 ), the system moves to projection (C),
this information, it can determine the highest log position      where F8 is now used to service all reads in the range
reached in the sealed projection; this is useful in certain      [40K, 80K). Eventually, the log tail moves past 80K,
reconfiguration scenarios.                                       and the system again reconfigures, adding a new range
   2. Writing the new projection at the auxiliary:               in projection (D) to service reads and writes past 80K.
Once the reconfiguring client Cr has successfully sealed
the current projection Pi , it attempts to write the new
                                                                 3.3     Finding the tail in CORFU
projection Pi+1 at the (i + 1)th position in the auxiliary.
If some other client has already written to that position,       Thus far, we described the machinery used by CORFU
client Cr aborts its own reconfiguration, reads the exist-       to map log positions to sets of flash pages. This allows
ing projection at position (i + 1) in the auxiliary, and         clients to read or write any position in a logical address
uses it as its new current projection. As a result, mul-         space. To treat this address space as an appendable log,
tiple clients can initiate reconfiguration simultaneously,       clients must be able to find the tail of the log and write to
but only one of them succeeds in proposing the new pro-          it.
jection.                                                             One possible solution is to allow clients to contend
   Projections – and the ability to move consistently be-        for positions. In this case, when a CORFU instance is
tween them – offer a versatile mechanism for CORFU               started, every client that wishes to append data will try
to deal with dynamism. Figure 4 shows an example se-             to concurrently write to position 0. One client will win,
quence of projections. In Figure 4(A), range [0, 40K)            while the rest fail; these clients then try again on posi-
in the log is mapped to the two flash unit mirrored pairs        tion 1, and so on. This approach provides log semantics
F0 /F1 and F2 /F3 , while range [40K, 80K) is mapped             if only one write is allowed to ‘win’ on each position;
i.e., complete successfully with the guarantee that any         3.4    Replication in CORFU
subsequent read on the position returns the value writ-
ten, until the position is trimmed. We call this property       Once a client reserves a new position in the log via the
safety-under-contention.                                        sequencer, it maps this position to a replica set of flash
   In the absence of replication, this property is satisfied    pages in the cluster using the current projection. At this
trivially by the flash unit’s write-once semantics. When        point, it has to write data at these flash pages over the
each position is replicated on multiple flash pages, it is      network. The protocol used to write to the set of flash
still possible to provide this property; in fact, the repli-    pages has to provide two properties. First, it has to pro-
cation protocol used in CORFU (that we describe next)           vide the safety-under-contention property described ear-
does so. However, it is clear that such an approach will        lier: when multiple clients write to the replica set for
result in poor performance when there are hundreds of           a log position, reading clients should observe a single
clients concurrently attempting appends to the log.             value. Second, it has to provide durability: written data
   To eliminate such contention at the tail of the log,         must be visible to reads only after it has sufficient fault
CORFU uses a dedicated sequencer that assigns clients           tolerance (i.e., reached f + 1 replicas). Given the rela-
‘tokens’, corresponding to empty log positions. The se-         tively high cost of flash, we require a solution that toler-
quencer can be thought of as a simple networked counter.        ates f failures with just f + 1 replicas; as a result, data
To append data, a client first goes to the sequencer, which     must be visible to reads only after it reaches all replicas.
returns its current value and increments itself. The client        One approach is to have the client write in parallel to
has now reserved a position in the log and can write to it      the flash units in the set, and wait for all of them to re-
without contention from other clients.                          spond before acknowledging the completion of the ap-
   Importantly, the sequencer does not represent a single       pend to the application. Unfortunately, when appending
point of failure; it is merely an optimization to reduce        clients contend for a log position, different values can be
contention in the system and is not required for either         written on different replicas, making it difficult to sat-
safety or progress. For fast recovery from sequencer fail-      isfy the safety-under-contention property. Also, satisfy-
ure, we store the identity of the current sequencer in the      ing the durability property with parallel writes requires
projection and use reconfiguration to change sequencers.        the reading client to access all replicas to determine if a
The counter of the new sequencer is determined using the        write has completed or not.
highest page written on each flash unit, which is returned         Instead, CORFU uses a simple chaining protocol (es-
by the flash unit in response to the seal command during        sentially, a client-driven variant of Chain Replication
reconfiguration.                                                [28]) to achieve the safety-under-contention and durabil-
   However, CORFU’s sequencer-based approach does               ity properties. When a client wants to write to a replica
introduce a new failure mode, since ‘holes’ can appear          set of flash pages, it updates them in a deterministic or-
in the log when clients obtain tokens and then fail to use      der, waiting for each flash unit to respond before mov-
them immediately due to crashes or slowdowns. Holes             ing to the next one. The write is successfully completed
can cripple applications that consume the log in strict or-     when the last flash unit in the chain is updated. As a
der, such as state machine replication or transaction pro-      result, if two clients attempt to concurrently update the
cessing, since no progress can be made until the status of      same replica set of flash pages, one of them will arrive
the hole is resolved. Given that a large system is likely       second at the first unit of the chain and receive an er-
to have a few malfunctioning clients at any given time,         ror overwrite. This ensures safety-under-contention.
holes can severely disrupt application performance. A              To read from the replica set, clients have two options.
simple solution is to have other clients fill holes aggres-     If they are unaware of whether the log position was suc-
sively with a reserved ‘junk’ value. To prevent aggres-         cessfully written to or not (for example, when replay-
sive hole-filling from burning up flash cycles and net-         ing the log after a power failure), they are required to go
work bandwidth, flash units can be junk-aware, simply           to the last unit of the chain. If the last unit has not yet
updating internal meta-data to mark an address as filled        been updated, it will return an error unwritten. This en-
with junk instead of writing an actual value to the flash.      sures the durability property. Alternatively, if the read-
   Note that filling holes reintroduces contention for log      ing client knows already that the log position has been
positions: if a client is merely late in using a reserved to-   successfully written to (for example, via an out-of-band
ken and has not really crashed, it could end up competing       notification by the writing client), it can go to any replica
with another client trying to fill the position with junk,      in the chain for better read performance.
which is equivalent to two clients concurrently writing            By efficiently handling contention, chained appends
to the same position. In other words, the sequencer is an       allow a client to rapidly fill holes left in the log by other
optimization that removes contention for log positions in       clients that crashed midway through an append. To fill
the common case, but does not eliminate it entirely.            holes, the client starts by checking the first unit of the
chain to determine if a valid value exists in the prefix         3.5    Garbage Collection
of the chain. If such a value exists, the client walks
down the chain to find the first unwritten replica, and          CORFU provides the abstraction of an infinitely grow-
then ‘completes’ the append by copying over the value            ing log to applications. The application does not have
to the remaining unwritten replicas in chain order. Al-          to move data around in the address space of the log to
ternatively, if the first unit of the chain is unwritten, the    free up space. All it is required to do is use the trim
client writes the junk value to all the replicas in chain or-    interface to inform CORFU when individual log posi-
der. CORFU exposes this fast hole filling functionality          tions are no longer in use. As a result, CORFU makes it
to applications via a fill interface. Applications can use       easy for developers to build applications over the shared
this primitive as aggressively as required, depending on         log without worrying about garbage collection strategies.
their sensitivity to holes in the shared log.                    An implication of this approach is that as the application
   Reconfiguration and Replication: How does this                appends to the log and trims positions selectively, the ad-
replication protocol interact with the reconfiguration           dress space of the log can become increasingly sparse.
mechanism described in Section 3.2.1? We first define               Accordingly, CORFU has to efficiently support a
a chain property: the replica chain for a log position in a      sparse address space for the shared log. The solution is
projection has a written prefix storing a single value and       a two-level mapping. As described before, CORFU uses
an unwritten suffix. A completed write corresponds to a          projections to map from a single infinite address space to
chain with a full-length prefix and a zero-length suffix.        individual extents on each flash unit. Each flash-unit then
   When the system reconfigures from one projection to           maps a sparse 64-bit address space, broken into extents,
another, the replica chain for a position can change from        onto the physical set of pages. The flash unit has to main-
one ordered set of flash pages to another. Trivially, the        tain a hash-map from 64-bit addresses to the physical ad-
new replica chain is required to satisfy the chain prop-         dress space of the flash. In fact, this is a relatively minor
erty. Further, we impose two conditions on the transition:       departure from the functionality implemented by mod-
if the old chain had a prefix of non-zero length, the new        ern SSDs, which often employ page-level maps from a
chain must have one as well with the same value. If the          fixed-size logical address space to a slightly larger phys-
old chain had a zero-length suffix, the new chain must           ical address space.
have one too. Lastly, to prevent split-brain scenarios, we          Crucially, a projection is a range-to-range mapping,
require that at least one flash unit in the old replica set be   and hence it is quite concise. Despite this, it is possible
sealed in the old projection’s epoch.                            that an adversarial workload can result in bloated projec-
                                                                 tions; for instance, if each range in the projection has a
   To meet these requirements, our current implementa-
                                                                 single valid entry that is never trimmed, the mapping for
tion follows the protocol described earlier in Figure 4 for
                                                                 that range has to be retained in the projection for perpe-
flash unit failures. The system first reconfigures to a new
                                                                 tuity.
chain without the failed replica. It then prepares a new
replica by copying over the completed value on each po-             Even for such adversarial workloads, it is easy to
sition, filling any holes it encounters in the process. Once     bound the size of the projection tightly by introducing
the new replica is ready, the system reconfigures again to       a small amount of proactive data movement across flash
add it to the end of the replica chain.                          units in order to merge consecutive ranges in the projec-
                                                                 tion. For instance, we estimate that adding 0.1% writes
   Relationship to Paxos: Each chain of flash units in
                                                                 to the system can keep the projection under 25 MB on
CORFU can be viewed as a single consensus engine,
                                                                 a 1 TB cluster for an adversarial workload. In practice,
providing (as Paxos does) an ordered sequence of state
                                                                 we do not expect projections to exceed 10s of KBs for
machine replication (SMR) commands. In conventional
                                                                 conventional workloads; this is borne out by our expe-
SMR implemented using Paxos, the throughput of the
                                                                 rience building applications over CORFU. In any case,
system is typically scaled by partitioning the system
                                                                 handling multi-MB projections is not difficult, since they
across multiple instances, effectively splitting the single
                                                                 are static data structures that can be indexed efficiently.
stream of totally ordered commands into many unrelated
                                                                 Additionally, new projections can be written as deltas to
streams. Conversely, CORFU scales SMR throughput by
                                                                 the auxiliary.
partitioning over time, not space; the consensus deci-
sion on each successive log entry is handled by a dif-
ferent replica chain. Stitching these multiple command           3.6    Flash Unit Implementations
streams together is the job of the projection, which is
determined by a separate, auxiliary-driven consensus en-         Flash units can be viewed as conventional SSDs with net-
gine, as described earlier. In a separate report, we ex-         work interfaces, supporting reads, writes and trims on an
amine the foundational differences between Paxos and             address space. Recall that flash units have to meet three
CORFU in more detail [19].                                       major requirements: write-once semantics, a seal capa-
bility, and an infinite address space.                          FPGA+SSD: an FPGA with a SATA SSD attached to
   Of these, the seal capability is simple: the flash unit      it, prototyped using the Beehive [27] architecture on the
maintains an epoch number cur sealed epoch and rejects          BEE3 hardware platform [10]. The FPGA includes a net-
all messages tagged with equal or lower epochs, sending         work interface to talk to clients and SATA ports to con-
back an error badepoch error code. When a seal com-             nect to the SSD. A variant under development runs over
mand arrives with a new epoch to seal, the flash unit           raw flash chips, implementing FTL functionality on the
first flushes all ongoing operations and then updates its       FPGA itself.
cur sealed epoch. It then responds to the seal command
with cur highest offset, the highest address written in the
address space it exposes; this is required for reconfigura-     4    CORFU Applications
tion, as explained previously.
   The infinite address space is essentially just a hash-       We are currently prototyping several applications over
map from 64-bit virtual addresses to the physical address       CORFU. Two such applications that we have im-
space of the flash. With respect to write-once semantics,       plemented are CORFU-Store, a key-value store, and
an address is considered unwritten if it does not exist in      CORFU-SMR, an implementation of State Machine
the hash-map. When an address is written to the first           Replication [23].
time, an entry is created in the hash-map pointing the vir-        CORFU-Store: This is a key-value store that sup-
tual address to a valid physical address. Each hash-map         ports a number of properties that are difficult to achieve
entry also has a bit indicating whether the address has         on partitioned stores, including atomic multi-key puts
been trimmed or not, which is set by the trim command.          and gets, distributed snapshots, geo-distribution and dis-
   Accordingly, a read succeeds if the address exists in        tributed rollback/replay. In CORFU-Store, a map-service
the hash-map and does not have the trim bit set. It returns     (which can be replicated) maintains a mapping from
error trimmed if the trim bit is set, and error unwritten if    keys to shared log offsets. To atomically put multiple
the address does not exist in the hash-map. A write to an       keys, clients must first append the key-value pairs to the
address that exists in the hash-map returns error trimmed       CORFU log, append a commit record and then send the
if the trim bit is set and error overwritten if it is not. If   commit record to the map-service. To atomically get
the address is not in the hash-map, the write succeeds.         multiple keys, clients must query the map-service for
   To eventually remove trimmed addresses from the              the latest key-offset mappings, and then perform CORFU
hash-map, the flash unit also maintains a watermark be-         reads on the returned offsets. This protocol ensures lin-
fore which no unwritten addresses exist, and removes            earizability for single-key puts and gets, as well as atom-
trimmed addresses from the hash-map that are lower than         icity for multi-key puts and gets.
the watermark. Reads and writes to addresses before                CORFU-Store’s shared log design makes it easy to
the watermark that are not in the hash-map return error-        take consistent point-in-time snapshots across the entire
trimmed immediately. If the address is in the hash-map,         key space, simply by playing the shared log up to some
reads succeed while writes return error overwritten.            position. The key-value store can be geo-distributed
   The flash unit also efficiently supports fill operations     asynchronously by copying the log, ensuring that the
that write junk by treating such writes differently from        mirror is always at some prior snapshot of the system,
first-class writes. Junk writes are initially treated as        with bounded lag. Additionally, CORFU-Store ensures
conventional writes, either succeeding or returning er-         even wear-out across the cluster despite highly skewed
ror overwritten or error trimmed as appropriate. How-           write workloads.
ever, instead of writing to the flash or SSD, the flash unit       CORFU-SMR: Earlier, we noted that CORFU pro-
points the hash-map entry to a special address reserved         vides the same functionality as Paxos. Accordingly,
for junk; this ensures that flash cycles and capacity is not    CORFU is ideal for implementing replicated state ma-
wasted. Also, once the hash-map is updated, the entry is        chines. Each SMR server simply plays the log forward
immediately trimmed in the scope of the same operation.         to receive the next command to execute. It proposes new
This removes the need for clients to explicitly track and       commands into the state machine by appending them to
trim junk in the log.                                           the log. The ability to do fast queries on the body of the
   Currently, we have built two flash unit instantiations:      log means that the log entries can also include the data
Server+SSD: this consists of conventional SATA SSDs             being executed over, obviating the need for each SMR
attached to servers with network interfaces.            The     server to store state persistently.
server accepts CORFU commands over the network                     In our current implementation, each SMR server plays
from clients and implements the functionality described         the log forward by issuing reads to the log. This ap-
above, issuing reads and writes to the fixed-size address       proach can stress the shared log: with N SMR servers
space of the SSD as required.                                   running at T commands/sec, the CORFU log will see
N ∗ T reads/sec. However, we retained this design since                         3000
we did not bottleneck at the shared log in our experi-                                         Server:TCP,Flash
ments; instead, we were limited by the ability of each                          2500           Server:TCP,RAM
                                                                                               Server:UDP,RAM
SMR server to receive and process commands. In the fu-                                         FPGA:UDP,Flash
ture, we expect to explore different approaches to playing                      2000


                                                                 Latency (ms)
the log forward, perhaps by having dedicated machines
multicast the contents of the log out to all SMR servers.                       1500
   Other Applications: As mentioned previously, we
                                                                                1000
are implementing the Hyder database [7] over CORFU.
One application in the works is a shared block device
                                                                                 500
that exposes a conventional fixed-size address space; this
can then be used to expose multiple, independent virtual                           0
disks to client VMs [20]. Another application is reliable                              Reads   Appends        Fills
multicast, where senders append to a single, channel-
specific log before multicasting. A receiver can detect
lost packets by observing gaps in the sequence of log po-     Figure 5: Latency for CORFU operations on different
sitions, and retrieve them from the log.                      flash unit configurations.

                                                              around 15W; in contrast, one of our servers consumes
5    Evaluation                                               250W. We also experimented with low-power Atom-
                                                              based servers, but found them incapable of serving SSD
We evaluate CORFU on a cluster of 32 Intel X25V               reads over the network at 1 Gbps.
drives. Our experiment setup consists of two racks;
each rack contains 8 servers (with 2 drives attached to
                                                              5.1               End-to-end Latency
each server) and 11 clients. Each machine has a 1
Gbps link. Together, the two drives on a server pro-          We first summarize the end-to-end latency characteris-
vide around 40,000 4KB read IOPS; accessed over the           tics of CORFU in Figure 5. We show the latency for
network, each server bottlenecks on the Gigabit link and      read, append and fill operations issued by clients for four
gives us around 30,000 4KB read IOPS. Each server runs        CORFU configurations. The left-most bar for each oper-
two processes, one per SSD, which act as individual flash     ation type (Server:TCP,Flash) shows the latency of the
units in the distributed system. Currently, the top-of-rack   server-attached flash unit where clients access the flash
switches of the two racks are connected to a central 10       unit over TCP/IP when data is durably stored on the
Gbps switch; our experiments do not generate more than        SSD; this represents the configuration of our 32-drive de-
8 Gbps of inter-rack traffic. We run two client processes     ployment. To illustrate the impact of flash latencies on
on each of the client machines, for a total of 44 client      this number, we then show (Server:TCP,RAM), in which
processes.                                                    the flash unit reads and writes to RAM instead of the
   In all our experiments, we run CORFU with two-way          SSD. Third, (Server:UDP,RAM) presents the impact of
replication, where appends are mirrored on drives in ei-      the network stack by replacing TCP with UDP between
ther rack. Reads go from the client to the replica in the     clients and the flash unit. Lastly, (FPGA:UDP,Flash)
local rack. Accordingly, the total read throughput possi-     shows end-to-end latency for the FPGA+SSD flash unit,
ble on our hardware is equal to 2 GB/sec (16 servers X 1      with the clients communicating with the unit over UDP.
Gbps each) or 500K/sec 4KB reads. Append throughput              Against these four configurations we evaluate the la-
is half that number, since appends are mirrored.              tency of three operation types. Reads from the client in-
   Unless otherwise mentioned, our throughput numbers         volve a simple request over the network to the flash unit.
are obtained by running all 44 client processes against       Appends involve a token acquisition from the sequencer,
the entire cluster of 32 drives. We measure throughput at     and then a chained append over two flash unit replicas.
the clients over a 60-second period during each run.          Fills involve an initial read on the head of the chain to
   In addition to this primary deployment of server-          check for incomplete appends, and then a chained ap-
attached SSDs, we also provide some results over the          pend to two flash unit replicas.
prototype FPGA+SSD flash unit. In this case, the FPGA            In this context, Figure 5 makes a number of impor-
has two SSDs attached to it and emulates a pair of flash      tant points. First, the latency of the FPGA unit is very
units, and a single CORFU client accesses it over the         low for all three operations, providing sub-millisecond
network. The FPGA runs at 1 Gbps or roughly 30K               appends and fills while satisfying reads within half a mil-
4KB reads/sec. When running at full speed, it consumes        lisecond. This justifies our emphasis on a client-centric
                     100                                         100                                             100
                      75       Server Reads                       75            Server Appends                    75         Server Fills
   %                  50                                     %    50                                         %    50
                      25                                          25                                              25
                       0                                           0                                               0
                           0 1 2 3 4 5 6 7 8 9 10                      0 1 2 3 4 5 6 7 8 9 10                          0 1 2 3 4 5 6 7 8 9 10
                                    Latency (ms)                                   Latency (ms)                              Latency (ms)
                     100                                         100                                             100
                      75           FPGA Reads                     75            FPGA Appends                      75         FPGA Fills
   %                  50                                     %    50                                         %    50
                      25                                          25                                              25
                       0                                           0                                               0
                           0 1 2 3 4 5 6 7 8 9 10                      0 1 2 3 4 5 6 7 8 9 10                          0 1 2 3 4 5 6 7 8 9 10
                                    Latency (ms)                                   Latency (ms)                              Latency (ms)


                                        Figure 6: Latency distributions for CORFU operations on 4KB entries.


                     500K                                         2                   in’ address spaces that have been written to completely.
                                     Append Throughput                                Emulating SSD writes also allows us to test the CORFU
                                       Read Throughput                                design at speeds exceeding the write bandwidth of our
                     375K                                         1.5                 commodity SSDs.



   4KB Entries/Sec
                                                                                         Figure 7 shows how log throughput for 4KB appends
                                                                       GB/sec
                     250K                                         1                   and reads scales with the number of drives in the sys-
                                                                                      tem. As we add drives to the system, both append and
                                                                                      read throughput scale up proportionally. Ultimately, ap-
                     125K                                         0.5                 pend throughput hits a bottleneck at around 180K ap-
                                                                                      pends/sec; this is the maximum speed of our sequencer
                       0K                                         0
                                                                                      implementation, which is a user-space application run-
                               4     8 12 16 20 24 28 32                              ning over TCP/IP. Since we were near the append limit of
                                                                                      our hardware, we did not further optimize our sequencer
                                     Number of Flash Units
                                                                                      implementation.
                                                                                         At such high append throughputs, CORFU can wear
 Figure 7: Throughput for random reads and appends.                                   out 1 TB of MLC flash in around 4 months. We believe
                                                                                      that replacing a $3K cluster of flash drives every four
                                                                                      months is acceptable in settings that require strong con-
design; eliminating the server from the critical path ap-                             sistency at high speeds, especially since we see CORFU
pears to have a large impact on latency. Second, the la-                              as a critical component of larger systems (as a consensus
tency to fill a hole in the log is very low; on the FPGA                              engine or a metadata service, for example).
unit, fills complete within 650 microseconds. CORFU’s
ability to fill holes rapidly is key to realizing the bene-
fits of a client-centric design, since hole-inducing client                           5.3    Reconfiguration
crashes can be very frequent in large-scale systems. In
                                                                                      Reconfiguration is used extensively in CORFU, to re-
addition, the chained replication scheme that allows fast
                                                                                      place failed drives, to add capacity to the system, and
fills in CORFU does not impact append latency drasti-
                                                                                      to add, remove, or relocate replicas. This makes recon-
cally; on the FPGA unit, appends complete within 750
                                                                                      figuration latency a crucial metric for our system.
microseconds.
                                                                                         Recall that reconfiguration latency has two compo-
                                                                                      nents: sealing the current configuration, which contacts
5.2                  Throughput                                                       a subset of the cluster, and writing the new configuration
                                                                                      to the auxiliary. In our experiments, we conservatively
We now focus on throughput scalability; these experi-                                 seal all drives, to provide an upper bound on reconfigu-
ments are run on our 32-drive cluster of server-attached                              ration time; in practice, only a subset needs to be sealed.
SSDs. To avoid burning out the SSDs in the throughput                                 Our auxiliary is implemented as a networked file share.
experiments, we emulate writes to the SSDs; however,                                     Figure 8 (Left) shows throughput behavior at an ap-
all reads are served from the SSDs, which have ‘burnt-                                pending and reading client when a flash unit fails. When
                          25K                                                     50                                                     10
                                                                                           Sealing Latency                                        Sealing Latency
                                                                                             Total Latency
                          20K                                                     40                                                      8




   Throughput (Ops/Sec)                                   % of Reconfigurations
                                                                                                                          Latency (ms)
                          15K                                                     30                                                      6
                                Reads
                              Appends
                          10K  Failure                                            20                                                      4

                          5K                                                      10                                                      2

                          0K                                                       0                                                      0
                                0   5 10 15 20 25 30 35                                0   10     20   30      40   50                        4    8 12 16 20 24 28 32
                                      Time (seconds)                                            Latency (ms)                                      Number of Flash Units


Figure 8: Reconfiguration performance on 32-drive cluster. Left: Appending client waits on failed drive for 1 second
before reconfiguring, while reading client continues to read from alive replica. Middle: Distribution of sealing and
total reconfiguration latency for 32 drives. Right: Scalability of sealing with number of drives.


the error occurs 25 seconds into the experiment, the ap-                                           second. The line plots the total number of CORFU log
pending client’s throughput flat-lines and it waits for a                                          appends resulting as a result of the multi-put operations;
1-second timeout period before reconfiguring to a pro-                                             a multi-put involving k keys generates k +1 log appends,
jection that does not have the failed unit. The reading                                            one for each updated key and a final append for the com-
client, on the other hand, continues reading from the                                              mit record. For small multi-puts involving one or two
other replica; when reconfiguration occurs, it receives an                                         keys, we are bottlenecked by the ability of the CORFU-
error from the replica indicating that it has a stale, sealed                                      Store map-service to handle and process incoming com-
projection; it then experiences a minor blip in throughput                                         mit records; our current implementation bottlenecks at
as it retrieves the latest projection from the auxiliary. The                                      around 50K single-key commit records per second. As
graph for sequencer failure looks identical to this graph.                                         we add more keys to each multi-put, the number of log
   Figure 8 (Middle) shows the distribution of the latency                                         appends generated increases and the CORFU log bot-
between the start of reconfiguration and its successful                                            tlenecks at around 180K appends/sec. Beyond 4 keys
completion on a 32-drive cluster. The median latency                                               per multi-put, the log bottleneck determines the number
for the sealing step is around 5 ms, while writing to the                                          of multi-puts we push through; for example, we obtain
                                                                                                   180K
auxiliary takes 25 ms more. The slow auxiliary write is                                               6 =30K multi-puts involving 5 keys.
due to overheads in serializing the projection as human-                                              Figure 9 (Right) shows performance for atomic multi-
readable XML and writing it to the network share; im-                                              get operations. As we increase the number of keys ac-
plementing the auxiliary as a static CORFU instance and                                            cessed by each multi-get, the overall log throughput stays
writing binary data would give us sub-millisecond auxil-                                           constant while multi-get throughput decreases. For 4
iary writes (but make the system less easy to administer).                                         keys per multi-get, we get a little over 100K multi-gets
   Figure 8 (Right) shows how the median sealing latency                                           per second, resulting in a load of over 400K reads/s on
scales with the number of drives in the system. Sealing                                            the CORFU log.
involves the client contacting all flash units in parallel
and waiting for responses. The auxiliary write step in                                                 Figure 10 shows the throughput of CORFU-SMR,
reconfiguration is insensitive to system size.                                                     the state machine replication library implemented over
                                                                                                   CORFU. In this application, each client acts as a state
                                                                                                   machine replica, proposing new commands by append-
5.4                       Applications                                                             ing them to the log and executing commands by playing
                                                                                                   the log. Each command consists of a 512-byte payload
We now demonstrate the performance of CORFU-Store                                                  and 64 bytes of metadata; accordingly, 7 commands can
for atomic multi-key operations. Figure 9 (Left) shows                                             fit into a single CORFU log entry with batching. In the
the performance of multi-put operations in CORFU-                                                  experiment, each client generates 7K commands/sec; as
Store. On the x-axis, we vary the number of keys up-                                               we add clients on the x-axis, the total rate of commands
dated atomically in each multi-put operation. The bars                                             injected into the system increases by 7K. On the y-axis,
in the graph plot the number of multi-puts executed per                                            we plot the average rate (with error bars signifying the
                                                                                                                                        100                                      250
                                  500           Multi-Get Tput             500                                                                          Multi-Put Tput




    Thousands of Multi-Gets/Sec                                                                           Thousands of Multi-Puts/Sec
                                                Log Read Tput                                                                                         Log Append Tput




                                                                                 Thousands of Reads/Sec                                                                                Thousands of Appends/Sec
                                                                                                                                         80                                      200
                                  400                                      400
                                                                                                                                         60                                      150
                                  300                                      300

                                  200                                      200                                                           40                                      100

                                  100                                      100                                                           20                                      50

                                    0                                      0                                                              0                                      0
                                        1   2   3   4   5    6     7   8                                                                      1   2    3   4   5   6     7   8
                                            # of Keys in Multi-Get                                                                                # of Keys in Multi-Put


 Figure 9: Example CORFU Application: CORFU-Store supports atomic multi-gets and multi-puts at cluster scale.

                                  90K                                                                                clusters could efficiently support parallelizable work-
                                                        SMR Throughput                                               loads, our contribution lies in extending the benefits of
                                  80K
                                  70K
                                                                                                                     such an architecture to strongly consistent applications.
                                                                                                                        In contrast to the FAWN and CORFU architecture are


    Commands/Sec
                                  60K
                                                                                                                     PCI-e drives such as Fusion-io [2], which offer up to 10
                                  50K
                                                                                                                     Gbps of random I/O. Accessing such drives from remote
                                  40K                                                                                clients requires a high-end, expensive storage server to
                                  30K                                                                                act as a conduit between the network and the drive. The
                                  20K                                                                                server is a single point of failure, acts as a bottleneck
                                  10K                                                                                for reads and writes, is a power hog, costs as much as
                                   0K                                                                                the drive, and is not incrementally scalable. Storage ap-
                                        1 2 3 4 5 6 7 8 9 10 11 12 13 14                                             pliances based on PCI-e drives [3, 4] can solve the first
                                                                                                                     two issues via replication but not the others. CORFU of-
                                                        Replicas
                                                                                                                     fers similar performance for an order of magnitude less
                                                                                                                     power consumption and less than half the cost (our 32-
Figure 10: Example CORFU Application: CORFU-                                                                         drive X25-V cluster cost $3K), while offering all the ben-
SMR supports high-speed state machine replication.                                                                   efits of a distributed solution.
                                                                                                                        Replicated Data Stores: A traditional design parti-
min and max across replicas) at which commands are                                                                   tions a virtual block drive or a file system onto a col-
executed by each replica. While the system has 10 or                                                                 lection of independent replica sets. Internally, each ob-
less CORFU-SMR replicas, the rate at which commands                                                                  ject (a file or a block) is replicated using primary-backup
are executed in the system matches the rate at which they                                                            mirroring (e.g., [18]) or quorum replication (e.g., [11]).
are injected. Beyond 10 replicas, we overload the abil-                                                              Mapping each object onto a replica set is done by a cen-
ity of a client to execute incoming commands, and hence                                                              tralized configuration manager (usually implemented as
throughput flat-lines; this means that clients are now lag-                                                          a replicated state machine). As we explained earlier,
ging behind while playing the state machine.                                                                         CORFU partitions data across time rather than space. In
                                                                                                                     addition, CORFU can act as the configuration manager
                                                                                                                     in such systems, adding a fast source of consistency that
6                        Related Work                                                                                other applications can use for resource and lock manage-
                                                                                                                     ment. This vital role is is currently played by systems
Flash in the Data Center: CORFU’s architecture bor-                                                                  like Chubby [9] and ZooKeeper [15]. Another type of
rows heavily from the FAWN system [5], which first ar-                                                               effort related to SMR concentrates on high throughput
gued for clusters of low-power flash units. CORFU’s                                                                  by allowing reads to proceed from any replica [8, 26].
network-attached flash units are a natural extension of                                                              CORFU faces a similar challenge of learning which ap-
the FAWN argument that low-speed CPUs are more                                                                       pends have committed in order to read from any replica.
power-efficient; our FPGA implementation consisted of                                                                   Log-structured Filesystems: All contemporary de-
a ring of 100 MHz cores. While FAWN showed that such                                                                 signs for flash storage borrow heavily from a long line of
log-structured filesystems starting with LFS [21]. Mod-        References
ern SSDs are testament to the fact that flash is ideally
                                                                [1] Intel x25-v datasheet. http://www.intel.com/design/
suited for logging designs, eliminating the traditional             flash/nand/value/technicaldocuments.htm.
problem with such systems of garbage collection inter-          [2] Fusion-io. http://www.fusionio.com/, 2011.
fering with foreground activity [24]. Zebra [12] and            [3] Texas memory systems. http://www.ramsan.com/, 2011.
                                                                [4] Violin memory. http://www.violin-memory.com/, 2011.
xFS [6] extended LFS to a distributed setting, striping
                                                                [5] D. G. Andersen, J. Franklin, M. Kaminsky, A. Phanishayee,
logs across a collection of storage servers. Like these             L. Tan, and V. Vasudevan. FAWN: A Fast Array of Wimpy Nodes.
systems, CORFU stripes updates across drives in a log-              In SOSP 2009.
structured manner. Unlike them, it implements a single          [6] T. Anderson, M. Dahlin, J. Neefe, D. Patterson, D. Roselli, and
                                                                    R. Wang. Serverless Network File Systems. In ACM SIGOPS
shared log that can be concurrently accessed by multiple            Operating Systems Review, volume 29, pages 109–126. ACM,
clients while providing single-copy semantics.                      1995.
   Comparison with Hyder: As mentioned, Hyder was               [7] P. Bernstein, C. Reid, and S. Das. Hyder – A Transactional
our original motivating application, and is currently be-           Record Manager for Shared Flash. In CIDR 2011, pages 9–20,
                                                                    2011.
ing implemented over CORFU. The Hyder paper [7] in-             [8] W. Bolosky, D. Bradshaw, R. Haagens, N. Kusters, and P. Li.
cluded a design outline of a flash-based shared log (we             Paxos Replicated State Machines as the Basis of a High-
call this Hyder-Log) that was simulated but not imple-              performance Data Store. In NSDI 2011.
                                                                [9] M. Burrows. The Chubby lock service for loosely-coupled dis-
mented. In fact, the design requirements for CORFU
                                                                    tributed systems. In OSDI 2006.
emerged from discussions with the Hyder authors on             [10] J. Davis, C. P. Thacker, and C. Chang. BEE3: Revitalizing com-
adding fault-tolerance and scalability to their proposal.           puter architecture research. In MSR-TR-2009-45.
   CORFU differs from the Hyder-Log design in multi-           [11] S. Frølund, A. Merchant, Y. Saito, S. Spence, and A. Veitch. FAB:
                                                                    Enterprise Storage Systems on a Shoestring. In HotOS 2003.
ple ways. First, CORFU’s use of an off-path sequencer          [12] J. Hartman and J. Ousterhout. The zebra striped network file
ensures that no single flash unit has to process all ap-            system. ACM TOCS, 13(3):274–310, 1995.
pend I/O in the system; in contrast, Hyder-Log funnels         [13] R. Haskin, Y. Malachi, and G. Chan. Recovery management in
all appends through a primary unit, and accordingly ap-             QuickSilver. ACM TOCS, 6(1):82–108, 1988.
                                                               [14] H. Holbrook, S. Singhal, and D. Cheriton. Log-based receiver-
pend throughput is bounded by the speed of a single unit.           reliable multicast for distributed interactive simulation. ACM
Second, Hyder-Log parallelizes appends by striping in-              SIGCOMM CCR, 25(4):328–341, 1995.
dividual entries across units; this forces applications to     [15] P. Hunt, M. Konar, F. Junqueira, and B. Reed. Zookeeper: wait-
use large entry sizes that are multiples of 4KB (typical            free coordination for internet-scale systems. In USENIX ATC,
                                                                    pages 11–11. USENIX Association, 2010.
flash page granularity). To achieve parallelism without        [16] M. Ji, A. Veitch, J. Wilkes, et al. Seneca: remote mirroring done
imposing large entry sizes, CORFU uses projections to               write. In USENIX 2003 Annual Technical Conference, 2003.
map log positions to units in round-robin fashion.             [17] L. Lamport, D. Malkhi, and L. Zhou. Vertical paxos and primary-
                                                                    backup replication. In PODC 2009, pages 312–313. ACM, 2009.
   Finally, our experience with CORFU identified holes         [18] E. Lee and C. Thekkath. Petal: Distributed virtual disks. ACM
in the log as the most common mode of failure and source            SIGOPS Operating Systems Review, 30(5):84–92, 1996.
of delays; accordingly, we focused on providing a sub-         [19] D. Malkhi, M. Balakrishnan, J. D. Davis, V. Prabhakaran, and
millisecond hole-filling primitive that ensures high, sta-          T. Wobber. From Paxos to CORFU: A Flash-Speed Shared Log.
                                                                    ACM SIGOPS Operating Systems Review, 46(1):47–51, 2012.
ble log throughput despite client crashes. Hyder-Log suf-      [20] D. Meyer, G. Aggarwal, B. Cully, G. Lefebvre, M. Feeley,
fers from holes as well; it resorts to a heavy recovery pro-        N. Hutchinson, and A. Warfield. Parallax: Virtual Disks for Vir-
tocol that involves sealing the system and forcing clients          tual Machines. In Eurosys 2008.
to move to a new view every time a hole has to be filled.      [21] M. Rosenblum and J. Ousterhout. The design and implementation
                                                                    of a log-structured file system. ACM TOCS, 10(1), Feb. 1992.
As a result of these differences, CORFU departs exten-         [22] F. Schmuck and J. Wylie. Experience with transactions in Quick-
sively in its design from the Hyder-Log proposal.                   Silver. In ACM SIGOPS OSR, volume 25. ACM, 1991.
                                                               [23] F. B. Schneider. Implementing fault-tolerant services using
                                                                    the state machine approach: A tutorial. ACM Comput. Surv.,
                                                                    22(4):299–319, 1990.
7    Conclusion                                                [24] M. Seltzer, K. Smith, H. Balakrishnan, J. Chang, S. McMains,
                                                                    and V. Padmanabhan. File system logging versus clustering: A
                                                                    performance comparison. In USENIX ATC 1995.
New storage designs are required to unlock the full po-
                                                               [25] A. Spector, R. Pausch, and G. Bruell. Camelot: A flexible, dis-
tential of flash storage. In this paper, we presented               tributed transaction processing system. In Compcon Spring’88.
the CORFU system, which organizes a cluster of flash           [26] J. Terrace and M. Freedman. Object storage on CRAQ: High-
drives as a single, shared log. CORFU offers single-copy            throughput chain replication for read-mostly workloads. In
                                                                    Usenix ATC 2009.
semantics at cluster-scale speeds, providing a scalable        [27] C. P. Thacker. Beehive: A many-core computer for FPGAs. Un-
source of atomicity and durability for distributed sys-             published Manuscript.
tems. CORFU’s novel client-centric design eliminates           [28] R. van Renesse and F. B. Schneider. Chain replication for sup-
storage servers in favor of simple, efficient and inexpen-          porting high throughput and availability. In OSDI 2004.
sive flash chips that attach directly to the network.
