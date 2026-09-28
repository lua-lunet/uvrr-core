       All File Systems Are Not Created Equal:
            On the Complexity of Crafting
            Crash-Consistent Applications
Thanumalayan Sankaranarayana Pillai, Vijay Chidambaram, Ramnatthan Alagappan,
                Samer Al-Kiswany, Andrea C. Arpaci-Dusseau,
         and Remzi H. Arpaci-Dusseau, University of Wisconsin–Madison

        https://www.usenix.org/conference/osdi14/technical-sessions/presentation/pillai

          This paper is included in the Proceedings of the
                   11th USENIX Symposium on
          Operating Systems Design and Implementation.
                          October 6–8, 2014 • Broomfield, CO
                                      978-1-931971-16-4



                                               Open access to the Proceedings of the
                                           11th USENIX Symposium on Operating Systems
                                                    Design and Implementation
                                                     is sponsored by USENIX.
              All File Systems Are Not Created Equal:
     On the Complexity of Crafting Crash-Consistent Applications
    Thanumalayan Sankaranarayana Pillai    Vijay Chidambaram    Ramnatthan Alagappan
     Samer Al-Kiswany        Andrea C. Arpaci-Dusseau        Remzi H. Arpaci-Dusseau
                          University of Wisconsin-Madison

Abstract                                                           but achieving this goal atop modern file systems is chal-
We present the first comprehensive study of application-           lenging for two fundamental reasons.
level crash-consistency protocols built atop modern file              The first challenge is that the exact guarantees pro-
systems. We find that applications use complex update              vided by file systems are unclear and underspecified.
protocols to persist state, and that the correctness of            Applications communicate with file systems through the
these protocols is highly dependent on subtle behaviors            POSIX system-call interface [48], and ideally, a well-
of the underlying file system, which we term persistence           written application using this interface would be crash-
properties. We develop a tool named B OB that empir-               consistent on any file system that implements POSIX.
ically tests persistence properties, and use it to demon-          Unfortunately, while the POSIX standard specifies the
strate that these properties vary widely among six pop-            effect of a system call in memory, specifications of how
ular Linux file systems. We build a framework named                disk state is mutated in the event of a crash are widely
A LICE that analyzes application update protocols and              misunderstood and debated [1]. As a result, each file sys-
finds crash vulnerabilities, i.e., update protocol code that       tem persists application data slightly differently, leaving
requires specific persistence properties to hold for cor-          developers guessing.
rectness. Using A LICE, we analyze eleven widely-used                 To add to this complexity, most file systems provide
systems (including databases, key-value stores, version            a multitude of configuration options that subtly affect
control systems, distributed systems, and virtualization           their behavior; for example, Linux ext3 provides numer-
software) and find a total of 60 vulnerabilities, many of          ous journaling modes, each with different performance
which lead to severe consequences. We also show that               and robustness properties [51]. While these configura-
A LICE can be used to evaluate the effect of new file-             tions are useful, they complicate reasoning about exact
system designs on application-level consistency.                   file system behavior in the presence of crashes.
                                                                      The second challenge is that building a high-
1    Introduction                                                  performance application-level crash-consistency proto-
Crash recovery is a fundamental problem in systems                 col is not straightforward. Maintaining application con-
research [8, 21, 34, 38], particularly in database man-            sistency would be relatively simple (though not trivial)
agement systems, key-value stores, and file systems.               if all state were mutated synchronously. However, such
Crash recovery is hard to get right; as evidence, con-             an approach is prohibitively slow, and thus most appli-
sider the ten-year gap between the release of commercial           cations implement complex update protocols to remain
database products (e.g., System R [7, 8] and DB2 [34])             crash-consistent while still achieving high performance.
and the development of a working crash recovery algo-              Similar to early file system and database schemes, it is
rithm (ARIES [33]). Even after ARIES was invented, an-             difficult to ensure that applications recover correctly af-
other five years passed before the algorithm was proven            ter a crash [41, 47]. The protocols must handle a wide
correct [24, 29].                                                  range of corner cases, which are executed rarely, rela-
   The file-systems community has developed a standard             tively untested, and (perhaps unsurprisingly) error-prone.
set of techniques to provide file-system metadata consis-             In this paper, we address these two challenges directly,
tency in the face of crashes [4]: logging [5, 9, 21, 37, 45,       by answering two important questions. The first question
51], copy-on-write [22, 30, 38, 44], soft updates [18], and        is: what are the behaviors exhibited by modern file sys-
other similar approaches [10, 16]. While bugs remain in            tems that are relevant to building crash-consistent appli-
the file systems that implement these methods [28], the            cations? We label these behaviors persistence properties
core techniques are heavily tested and well understood.            (§2). They break down into two global categories: the
   Many important applications, including databases                atomicity of operations (e.g., does the file system ensure
such as SQLite [43] and key-value stores such as Lev-              that rename() is atomic [32]?), and the ordering of oper-
elDB [20], are currently implemented on top of these               ations (e.g., does the file system ensure that file creations
file systems instead of directly on raw disks. Such data-          are persisted in the same order they were issued?).
management applications must also be crash consistent,                To analyze file system persistence properties, we de-


                                                               1
USENIX Association              11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14) 433
                                                                               Player [52]. These applications represent software from
                                                                               different domains and at varying levels of maturity. The
                                                                               study focuses on file-system behavior that affects users,
                                                                               rather than on strictly verifying application correctness.
                                                                               We hence consider typical usage scenarios, sometimes
Figure 1: Git Crash Vulnerability. The figure shows part of                    checking for additional consistency guarantees beyond
the Git update protocol. The arrows represent ordering dependencies:           those promised in the application documentation. Our
if the appends are not persisted before the rename, any further commits        study takes a pessimistic view of file-system behavior;
to the repository fail. We find that, whether the protocol is vulnerable
                                                                               for example, we even consider the case where renames
or not varies even between configurations of the same file system.
                                                                               are not atomic on a system crash.
velop a simple tool, known as the Block Order Breaker
(B OB). B OB collects block-level traces underneath a file                        Overall, we find that application-level consistency in
system and re-orders them to explore possible on-disk                          these applications is highly sensitive to the specific per-
crash states that may arise. With this simple approach,                        sistence properties of the underlying file system. In gen-
B OB can find which persistence properties do not hold                         eral, if application correctness depends on a specific file-
for a given system. We use B OB to study six Linux file                        system persistence property, we say the application con-
systems (ext2, ext3, ext4, reiserfs, btrfs, and xfs) in var-                   tains a crash vulnerability; running the application on a
ious configurations. We find that persistence properties                       different file system could result in incorrect behavior.
vary widely among the tested file systems. For example,                        We find a total of 60 vulnerabilities across the applica-
appends to file A are persisted before a later rename of                       tions we studied; several vulnerabilities have severe con-
file B in the ordered journaling mode of ext3, but not in                      sequences such as data loss or application unavailability.
the same mode of ext4, unless a special option is enabled.                     Using A LICE, we also show that many of these vulner-
                                                                               abilities (roughly half) manifest on current file systems
   The second question is: do modern applications im-
                                                                               such as Linux ext3, ext4, and btrfs.
plement crash consistency protocols correctly? An-
swering this question requires understanding update pro-                          We find that many applications implicitly expect or-
tocols, no easy task since update protocols are com-                           dering among system calls (e.g., that writes, even to dif-
plex [47] and spread across multiple files in the source                       ferent files, are persisted in order); when such ordering
code. To analyze applications, we develop A LICE, a                            is not maintained, 7 of the 11 tested applications have
novel framework that enables us to systematically study                        trouble properly recovering from a crash. We also find
application-level crash consistency (§3). A LICE takes ad-                     that 10 of the 11 applications expect atomicity of file-
vantage of the fact that, no matter how complex the ap-                        system updates. In some cases, such a requirement is
plication source code, the update protocol boils down to                       reasonable (e.g., a single 512-byte write or file rename
a sequence of file-system related system calls. By an-                         operation are guaranteed to be atomic by many current
alyzing permutations of the system-call trace of work-                         file systems when running on a hard-disk drive); in other
loads, A LICE produces protocol diagrams: rich annotated                       situations (e.g., with file appends), it is less so. We also
graphs of update protocols that abstract away low-level                        note that some of these atomicity assumptions are not fu-
details to clearly present the underlying logic. A LICE                        ture proof; for example, new storage technology may be
determines the exact persistence properties assumed by                         atomic only at a smaller granularity than 512-bytes (e.g.,
applications as well as flaws in their design.                                 eight-byte PCM [12]). Finally, for 7 of the 11 applica-
   Figure 1 shows an example of A LICE in action. The                          tions, durability guarantees users likely expect are not
figure shows a part of the update protocol of Git [26].                        met, often due to directory operations not being flushed.
A LICE detected that the appends need to be persisted be-                         A LICE also enables us to determine whether new file-
fore the rename; if not, any future commits to the repos-                      system designs will help or harm application protocols.
itory fail. This behavior varies widely among file sys-                        We use A LICE to show the benefits of an ext3 variant
tems: a number of file-system features such as delayed                         we propose (ext3-fast), which retains much of the pos-
allocation and journaling mode determine whether file                          itive ordering and atomicity properties of ext3 in data
systems exhibit this behavior. Some common configura-                          journaling mode, without the high cost. Such verifica-
tions like ext3 ordered mode persist these operations in                       tion would have been useful in the past; when delayed
order, providing a false sense of security to the developer.                   allocation was introduced in Linux ext4, it broke several
   We use A LICE to study and analyze the up-                                  applications, resulting in bug reports, extensive mailing-
date protocols of eleven important applications: Lev-                          list discussions, widespread data loss, and finally, file-
elDB [20], GDBM [19], LMDB [46], SQLite [43],                                  system changes [14]. With A LICE, testing the impact of
PostgreSQL [49], HSQLDB [23], Git [26], Mercu-                                 changing persistence properties can become part of the
rial [31]), HDFS [40], ZooKeeper [3], and VMWare                               file-system design process.

                                                                           2
434 11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14)                                   USENIX Association
                       
                                    refer to this as size-atomicity. A lack of atomicity could
                                                                           also be realized with only part of a write reaching disk, as
                                    shown in State B. We refer to this as content-atomicity.
                                                                            If the file system persists the calls out of order, another
                       
                                                                                outcome is possible (State C). In this case, the second
                                         write reaches the disk first, and as a result only the second
Figure 2: Crash States. The figure shows the initial, final, and                file is updated. Various combinations of these states are
some of the intermediate crash states possible for the workload de-             also possible.
scribed in Section 2.1 . X represents garbage data in the files. Interme-          As we will see when we study application update pro-
diate states #A and #B represent different kinds of atomicity violations,
                                                                                tocols, modern applications expect different atomicity
while intermediate state #C represents an ordering violation.
                                                                                and ordering properties from underlying file systems. We
2      Persistence Properties                                                   now study such properties in detail.
In this section, we study the persistence properties of
modern file systems. These properties determine which                           2.2    Study and Results
possible post-crash file system states are possible for a                       We study the persistence properties of six Linux file sys-
given file system; as we will see, different file systems                       tems: ext2, ext3, ext4, btrfs, xfs, and reiserfs. A large
provide subtly different guarantees, making the chal-                           number of applications have been written targeting these
lenge of building correct application protocols atop such                       file systems. Many of these file systems also provide
systems more vexing.                                                            multiple configurations that make different trade-offs be-
   We begin with an example, and then describe our                              tween performance and consistency: for instance, the
methodology: to explore possible on-disk states by re-                          data journaling mode of ext3 provides the highest level of
ordering the I/O block stream, and then examine pos-                            consistency, but often results in poor performance [35].
sible resulting states. Our testing is not complete, but                        Between file systems and their various configurations, it
finds persistence properties that do not hold for a file-                       is challenging to know or reason about which persistence
system implementation. We then discuss our findings for                         properties are provided. Therefore, we examine different
six widely-used Linux file systems: ext2 [6], ext3 [51],                        configurations of the file systems we study (a total of 16).
ext4 [50], btrfs [30], xfs [45], and reiserfs [37].                                To study persistence properties, we built a tool, known
   Application-level crash consistency depends strongly                         as the Block Order Breaker (B OB), to empirically find
upon these persistence properties, yet there are currently                      cases where various persistence properties do not hold
no standards. We believe that defining and studying per-                        for a given file system. B OB first runs a simple user-
sistence properties is the first step towards standardizing                     supplied workload designed to stress the persistence
them across file systems.                                                       property tested (e.g., a number of writes of a specific size
2.1      An Example                                                             to test overwrite atomicity). B OB collects the block I/O
                                                                                generated by the workload, and then re-orders the col-
All application update protocols boil down to a sequence
                                                                                lected blocks, selectively writing some of them to disk to
of I/O-related system calls which modify on-disk state.
                                                                                generate a new legal disk state (disk barriers are obeyed).
Two broad properties of system calls affect how they are
                                                                                In this manner, B OB generates a number of unique disk
persisted. The first is atomicity: does the update from the
                                                                                images corresponding to possible on-disk states after a
call happen all at once, or are there possible intermediate
                                                                                system crash. B OB then runs file-system recovery on
states that might arise due to an untimely crash? The
                                                                                each resulting disk image, and checks whether various
second is ordering: can this system call be persisted after
                                                                                persistence properties hold (e.g., if writes were atomic).
a later system call? We now explain these properties with
                                                                                If B OB finds even a single disk image where the checker
an example.
                                                                                fails, then we know that the property does not hold on
We consider the following pseudo-code snippet:
                                                                                the file system. Proving the converse (that a property
    write(f1, "pp");
    write(f2, "qq");                                                            holds in all situations) is not possible using B OB; cur-
In this example, the application first appends the string                       rently, only simple block re-orderings and all prefixes of
pp to file descriptor f1 and then appends the string qq to                      the block trace are tested.
file descriptor f2. Note that we will sometimes refer to                           Note that different system calls (e.g., writev(),
such a write() as an append() for simplicity.                                   write()) lead to the same file-system output. We group
   Figure 2 shows a few possible crash states that can                          such calls together into a generic file-system update we
result. If the append is not atomic, for example, it would                      term an operation. We have found that grouping all op-
be possible for the size of the file to be updated without                      erations into three major categories is sufficient for our
the new data reflected to disk; in this case, the files could                   purposes here: file overwrite, file append, and directory
contain garbage, as shown in State A in the diagram. We                         operations (including rename, link, unlink, mkdir, etc.).


                                                                            3
USENIX Association                        11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14) 435
Persistence Property                         File system                      entire block is persisted before adding a pointer to it.
                               ext2
                               ext2-sync
                                                                                 Current file systems do not provide atomic multi-block
                               ext3-writeback
                               ext3-ordered
                               ext3-datajournal
                                                                              appends; appends can be broken down into multiple op-
                               ext4-writeback
                               ext4-ordered
                               ext4-nodelalloc
                                                                              erations. However, most file systems seemingly guaran-
                               ext4-datajournal
                               btrfs
                               xfs                                            tee that some prefix of the data written (e.g., the first 10
                               xfs-wsync
                               reiserfs-nolog
                               reiserfs-writeback                             blocks of a larger append) will be appended atomically.
                                                                                 Directory operations such as rename() and link()
                               reiserfs-ordered
                               reiserfs-datajournal
Atomicity                                                                     are seemingly atomic on all file systems that use tech-
Single sector overwrite
                                                                              niques like journaling or copy-on-write for consistency.
Single sector append        × ×  ×          ×
Single block overwrite      ×××× ×××   ×× ×××
                                                                              2.2.2 Ordering
Single block append         ×××  ×        ××
Multi-block append/writes   ×××××××××××× ××××                                 We observe that ext3, ext4, and reiserfs in data journal-
Multi-block prefix append   ×××  ×        ××                                  ing mode, and ext2 in sync mode, persist all tested op-
Directory op                ××            ×                                   erations in order. Note that these modes often result in
Ordering
                                                                              poor performance on many workloads [35].
Overwrite → Any op          × ×× ×××   ×× ×××
[Append, rename]→ Any op × ×     ×        ××                                     The append operation has interesting special cases. On
O TRUNC Append → Any op × ×      ×        ××                                  file systems with the delayed allocation feature, it may
Append → Append (same file) × ×  ×        ××                                  be persisted after other operations. A special exception
Append → Any op             × ×  ××  ×× ××                                    to this rule is when a file is appended, and then renamed.
Dir op → Any op             ×        ×    ×
                                                                              Since this idiom is commonly used to atomically update
Table 1: Persistence Properties.               The table shows atomic-        files [14], many file systems recognize it and allocate
ity and ordering persistence properties that we empirically determined        blocks immediately. A similar special case is append-
for different configurations of file systems. X → Y indicates that X is       ing to files that have been opened with O TRUNC. Even
persisted before Y. [X,Y] → Z indicates that Y follows X in program
                                                                              with delayed allocation, successive appends to the same
order, and both become durable before Z. A × indicates that we have a
reproducible test case where the property fails in that file system.
                                                                              file are persisted in order. Linux ext2 and btrfs freely re-
                                                                              order directory operations (especially operations on dif-
   Table 1 lists the results of our study. The table shows,                   ferent directories [11]) to increase performance.
for each file system (and specific configuration) whether
a particular persistence property has been found to not                       2.3    Summary
hold; such cases are marked with an ×.                                        From Table 1, we observe that persistence properties
   The size and alignment of an overwrite or append af-                       vary widely among file systems, and even among differ-
fects its atomicity. Hence, we show results for single sec-                   ent configurations of the same file system. The order of
tor, single block, and multi-block overwrite and append                       persistence of system calls depends upon small details
operations. For ordering, we show whether given prop-                         like whether the calls are to the same file or whether the
erties hold assuming different orderings of overwrite, ap-                    file was renamed. From the viewpoint of an application
pend, and directory operations; the append operation has                      developer, it is risky to assume that any particular prop-
some interesting special cases relating to delayed alloca-                    erty will be supported by all file systems.
tion (as found in Linux ext4) – we show these separately.
                                                                              3     The Application-Level Intelligent
2.2.1 Atomicity
We observe that all tested file systems seemingly pro-                              Crash Explorer (ALICE)
vide atomic single-sector overwrites: in some cases (e.g.,                    We have now seen that file systems provide different per-
ext3-ordered), this property arises because the underly-                      sistence properties. However, some important questions
ing disk provides atomic sector writes. Note that if such                     remain: How do current applications update their on-disk
file systems are run on top of new technologies (such as                      structures? What do they assume about the underlying
PCM) that provide only byte-level atomicity [12], single-                     file systems? Do such update protocols have vulnerabil-
sector overwrites will not be atomic.                                         ities? To address these questions, we developed A LICE
   Providing atomic appends requires the update of two                        (Application-Level Intelligent Crash Explorer). A LICE
locations (file inode, data block) atomically. Doing so re-                   constructs different on-disk file states that may result due
quires file-system machinery, and is not provided by ext2                     to a crash, and then verifies application correctness on
or writeback configurations of ext3, ext4, and reiserfs.                      each created state.
   Overwriting an entire block atomically requires data                          Unlike other approaches [53, 54] that simply test an
journaling or copy-on-write techniques; atomically ap-                        application atop a given storage stack, A LICE finds the
pending an entire block can be done using ordered mode                        generic persistence properties required for application
journaling, since the file system only needs to ensure the                    correctness, without being restricted to only a specified

                                                                          4
436 11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14)                                  USENIX Association
                                                                                                             # Checker
                                                                                # Workload
                                                                                                       db = gdbm.open(’mydb’)
                                                                                # Opening database     c = len(db.keys())
                                                                                db = gdbm.open(’mydb’) if alice.printed(’Done’):
                                                                                # Inserting key-value ..assert c == 1
                                                                                db[’x’] = ’foo’       else:
                                                                                db.sync()             ..assert c == 0 or c == 1
                                                                                print ’Done’          if c == 1:
                                                                                                      ..assert db[’x’] == ’foo’

                                                                                Listing 1: Workload and Checker. Simplified form of python
                                                                                workload and checker for GDBM (a key-value store).

                                                                                   The exact crash states possible for a workload varies
                                                                                with the file system. For example, depending on the file
                                                                                system, appending to a file can result in the file con-
                                                                                taining either a prefix of the data persisted, with ran-
                                                                                dom data intermixed with file data, or various combi-
                                                                                nations thereof. A LICE uses file-system Abstract Per-
                                                                                sistence Models (APMs) to define the exact crash states
Figure 3: A LICE Overview.               The figure shows how A LICE            possible in a given file system. By default, A LICE uses an
converts user inputs into crash states and finally into crash vulnerabil-       APM with few restrictions on the possible crash states, so
ities. Black boxes are user inputs. Grey boxes are optional inputs.             as to find generic persistence properties required for ap-
file system. A LICE associates discovered vulnerabili-                          plication correctness. However, A LICE can be restricted
ties directly with source lines, and targets specific states                    to find vulnerabilities occurring only on a specific file
that are prone to reveal crash vulnerabilities in different                     system, by supplying the APM of that file system.
source lines. A LICE achieves this by constructing file                            Listing 1 shows example workload and checker scripts
states directly from the system-call trace of an applica-                       for GDBM, a key-value store, written in Python. We dis-
tion workload. The states to be explored and verified can                       cuss how APMs are specified in the next subsection.
be described purely in terms of system calls: the actual
storage stack is not involved. A LICE can also be used to
                                                                                3.2     Crash States and APMs
                                                                                Figure 3 shows an overview of the steps A LICE follows
abstractly test the safety of new file systems.
                                                                                to find crash vulnerabilities. The user-supplied workload
   We first describe how A LICE is used (§3.1). We then
                                                                                is first run, and a system-call trace obtained; the trace
describe how A LICE calculates states possible during
                                                                                represents an execution of the application’s update pro-
a system crash, using an Abstract Persistence Model
                                                                                tocol. The trace is converted into a sequence of logical
(APM) (§3.2). Next, we describe how these states are
                                                                                operations by A LICE. The sequence of logical opera-
selectively explored so as to discover application require-
                                                                                tions, along with an APM, is used to calculate the dif-
ments in terms of persistence properties (§3.3), and how
                                                                                ferent crash states that are possible from the initial state.
discovered vulnerabilities are reported associated with
                                                                                These steps are now explained in detail.
source code lines (§3.4). Finally, we describe our im-
plementation (§3.5) and its limitations (§3.6).                                 3.2.1   Logical Operations
                                                                                A LICE first converts the trace of system calls in the ap-
3.1     Usage                                                                   plication workload to logical operations. Logical opera-
                                                                                tions abstract away details such as current read and write
A LICE is simple to use. The user first supplies A LICE
                                                                                offsets, file descriptors, and transform a large set of sys-
with an initial snapshot of the files used by the applica-
                                                                                tem calls and other I/O producing behavior into a small
tion (typically an entire directory), and a workload script
                                                                                set of file-system operations. For example, write(),
that exercises the application (such as performing a trans-
                                                                                pwrite(), writev(), pwritev(), and mmap()-writes
action). The user also supplies a checker script corre-
                                                                                are all translated into overwrite or append logical op-
sponding to the workload that verifies whether invariants
                                                                                erations. Logical operations also associate a conceptual
of the workload are maintained (such as atomicity of the
                                                                                inode to each file or directory involved.
transaction). A LICE runs the checker atop different crash
states, i.e., the state of files after rebooting from a system                  3.2.2 Abstract Persistence Models
crash that can occur during the workload. A LICE then                           An APM specifies all constraints on the atomicity and or-
produces a logical representation of the update protocol                        dering of logical operations for a given file system, thus
executed during the workload, vulnerabilities in the pro-                       defining which crash states are possible.
tocol and their associated source lines, and persistence                           APMs represent crash states as consisting of two log-
properties required for correctness.                                            ical entities: file inodes containing data and a file size,


                                                                            5
USENIX Association                     11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14) 437
  Logical Operation      Micro-operations                                     open(path="/x2VC") = 10
      overwrite          N × write block(data)                                Micro-ops: None
       append                   change file size                              Ordered after: None
                              {
                          N × write block(random)
                                write block(data)
                                                                              pwrite(fd=10, offset=0, size=1024)
                                                                              Micro-ops: #1 write block(inode=8, offset=0, size=512)
       truncate                 change file size
                              {
                          N × write block(random)
                                write block(zeroes)
                                                                              Micro-ops: #2 write block(inode=8, offset=512, size=512)
                                                                              Ordered after: None
                                                                              fsync(10)
          link           create dir entry
                                                                              Micro-ops: None
         unlink          delete dir entry + truncate if last link
                                                                              Ordered after: None
        rename           delete dir entry(dest) + truncate if last link
                         create dir entry(dest)                               pwrite(fd=10, offset=1024, size=1024)
                         delete dir entry(source)                             Micro-ops: #3 write block(inode=8, offset=1024, size=512)
         print           stdout                                               Micro-ops: #4 write block(inode=8, offset=1536, size=512)
                      (a) Atomicity Constraints.                              Ordered after: #1, #2
 Description      Constraint                                                  link(oldpath="/x2VC", newpath="/file")
  sync-ops        [any-opi (A) ... fsyncj (A)]→any-opk ∀ i < j < k            Micro-ops: #5 create dir entry(dir=2, entry=‘file’, inode=8)
   stdout         stdouti ()→any-opj ∀ i < j                                  Ordered after: #1, #2
                      (b) Ordering Constraints.                               write(fd=1, data="Writes recorded", size=15)
Table 2: Default APM Constraints.                   (a) shows atomicity       Micro-ops: #6 stdout(”Writes recorded”)
constraints; N indicates a logical operation being divided into many          Ordered after: #1, #2
micro-ops. (b) shows ordering constraints. Xi is the ith operation, and       Listing 2: Annotated Update Protocol Example. Micro-
any-op(A) is an operation on the file or directory A.                         operations generated for each system call are shown along with their
and directories containing directory entries. Each logical                    dependencies. The inode number of x2VC is 8, and for the root
operation operates on one or more of these entities. An                       directory is 2. Some details of listed system calls have been omitted.
infinite number of instances of each logical entity exist,                    writes to A that precede the sync. Similar ordering also
and they are never allocated or de-allocated, but rather                      applies to stdout, and additionally, all operations follow-
simply changed. Additionally, each crash state also in-                       ing an stdout must be ordered after it.
cludes any output printed to the terminal during the time                        A LICE can also model the behavior of real file systems
of the crash as a separate entity.                                            when configured with other APMs. As an example, for
   To capture intermediate crash states, APMs break log-                      the ext3 file system under the data=journal mode, the
ical operations into micro-operations, i.e., the smallest                     ordering constraint is simply that each micro-op depends
atomic modification that can be performed upon each                           on all previous micro-ops. Atomicity constraints for ext3
logical entity. There are five micro-ops:                                     are mostly simple: all operations are atomic, except file
 • write block: A write of size block to a file. Two spe-                     writes and truncates, which are split at block-granularity.
   cial arguments to write block are zeroes and random:                       Atomic renames are imposed by a circular ordering de-
   zeroes indicates the file system initializing a newly                      pendency between the micro-ops of each rename.
   allocated block to zero; random indicates an unini-                        3.2.3 Constructing crash states.
   tialized block. Writes beyond the end of a file cause                      As explained, using the APM, A LICE can translate the
   data to be stored without changing file size.                              system-call trace into micro-ops and calculate ordering
 • change file size: Changes the size of a file inode.                        dependencies amongst them. Listing 2 shows an example
 • create dir entry: Creates a directory entry in a direc-                    system-call trace, and the resulting micro-ops and order-
   tory, and associates a file inode or directory with it.                    ing constraints. A LICE also represents the initial snapshot
 • delete dir entry: Deletes a directory entry.                               of files used by the application as logical entities.
 • stdout: Adds messages to the terminal output.                                 A LICE then selects different sets of the translated
                                                                              micro-ops that obey the ordering constraints. A new
   The APM specifies atomicity constraints by defining
                                                                              crash state is constructed by sequentially applying the
how logical operations are translated into micro-ops. The
                                                                              micro-ops in a selected set to the initial state (represented
APM specifies ordering constraints by defining which
                                                                              as logical entities). For each crash state, A LICE then con-
micro-ops can reach the disk before other micro-ops.
                                                                              verts the logical entities back into actual files, and sup-
   In most cases, we utilize a default APM to find the
                                                                              plies them to the checker. The user-supplied checker thus
greatest number of vulnerabilities in application update
                                                                              verifies the crash state.
protocols. The atomicity constraints followed by this de-
fault file system are shown in Table 2(a), which specifi-                     3.3     Finding Application Requirements
cally shows how each logical operation is broken down                         By default, A LICE targets specific crash states that con-
into micro-ops. The ordering constraints imposed by the                       cern the ordering and atomicity of each individual system
default APM are quite simple, as seen in Table 2(b): all                      call. The explored states thus relate to basic persistence
micro-ops followed by a sync on a file A are ordered after                    properties like those discussed in Section 2, making it


                                                                          6
438 11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14)                                         USENIX Association
straightforward to determine application requirements.              3.5    Implementation
We now briefly describe the crash states explored.                  A LICE consists of around 4000 lines of Python code, and
   Atomicity across System Calls. The application up-               also traces memory-mapped writes in addition to system
date protocol may require multiple system calls to be               calls. It employs a number of optimizations.
persisted together atomically. This property is easy to                First, A LICE caches crash states, and constructs a new
check: if the protocol has N system calls, A LICE con-              crash state by incrementally applying micro-operations
structs one crash state for each prefix (i.e., the first X          onto a cached crash state. We also found that the time
system calls, ∀ 1 < X < N ) applied. In the sequence                required to check a crash state was much higher than
of crash states generated in this manner, the first crash           the time required to incrementally construct a crash state.
state to have an application invariant violated indicates           Hence, A LICE constructs crash states sequentially, but in-
the start of an atomic group. The invariant will hold               vokes checkers concurrently in multiple threads.
once again in crash states where all the system calls in               Different micro-op sequences can lead to the same
the atomic group are applied. If A LICE determines that a           crash state. For example, different micro-op sequences
system call X is part of an atomic group, it does not test          may write to different parts of a file, but if the file is un-
whether the protocol is vulnerable to X being persisted             linked at the end of sequence, the resulting disk state is
out of order, or being partially persisted.                         the same. Therefore, A LICE hashes crash states and only
   System-Call Atomicity. The protocol may require a                checks the crash state if it is new.
single system call to be persisted atomically. A LICE tests            We found that many applications write to debug logs
this for each system call by applying all previous sys-             and other files that do not affect application invariants.
tem calls to the crash state, and then generating crash             A LICE filters out system calls involved with these files.
states corresponding to different intermediate states of
the system call and checking if application invariants are          3.6    Limitations
violated. The intermediate states for file-system oper-             A LICE is not complete, in that there may be vulnerabil-
ations depend upon the APM, as shown (for example)                  ities that are not detected by A LICE. It also requires the
in Table 2. Some interesting cases include how ALICE                user to write application workloads and checkers; we be-
handles appends and how it explores the atomicity of                lieve workload automation is orthogonal to the goal of
writes. For appends, we introduce intermediate states               A LICE, and various model-checking techniques can be
where blocks are filled with random data; this models               used to augment A LICE. For workloads that use multiple
the update of the size of a file reaching disk before the           threads to interact with the file system, A LICE serializes
data has been written. We split overwrites and appends              system calls in the order they were issued; in most cases,
in two ways: into block-sized micro-operations, and into            this does not affect vulnerabilities as the application uses
three parts regardless of size. Though not exhaustive, we           some form of locking to synchronize between threads.
have found our exploration of append and write atomic-              A LICE currently does not handle file attributes; it would
ity useful in finding application vulnerabilities.                  be straight-forward to extend A LICE to do so.
   Ordering Dependency among System Calls. The
protocol requires system call A to be persisted before B            4     Application Vulnerabilities
if a crash state with B applied (and not A) violates appli-         We study 11 widely used applications to find whether
cation invariants. A LICE tests this for each pair of system        file-system behavior significantly affects application
calls in the update protocol by applying every system call          users, which file-system behaviors are thus important,
from the beginning of the protocol until B except for A.            and whether testing using A LICE is worthwhile in gen-
                                                                    eral. One of A LICE’s unique advantages, of being able
3.4    Static Vulnerabilities                                       to find targeted vulnerabilities under an abstract file sys-
ALICE must be careful in how it associates problems                 tem and reporting them in terms of a persistence prop-
found in a system-call trace with source code. For exam-            erty violated, is thus integral to the study. The ap-
ple, consider an application issuing ten writes in a loop.          plications each represent different domains, and range
The update protocol would then contain ten write()                  in maturity from a few years-old to decades-old. We
system calls. If each write is required to be atomic for            study three key-value stores (LevelDB [20], GDBM [19],
application correctness, A LICE detects that each system            LMDB [46]), three relational databases (SQLite [43],
call is involved in a vulnerability; we term these as dy-           PostgreSQL [49], HSQLDB [23]), two version control
namic vulnerabilities. However, the cause of all these              systems (Git [26], Mercurial [31]), two distributed sys-
vulnerabilities is a single source line. A LICE uses stack          tems (HDFS [40], ZooKeeper [3]), and a virtualization
trace information to correlate all 10 system calls to the           software (VMWare Player [52]). We study two versions
line, and reports it as a single static vulnerability. In the       of LevelDB (1.10, 1.15), since they vary considerably in
rest of this paper, we only discuss static vulnerabilities.         their update-protocol implementation.


                                                                7
USENIX Association              11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14) 439
   Aiming towards the stated goal of the study, we try to           Version Control Systems. Git’s crash guarantees are
consider typical user expectations and deployment sce-           fuzzy; mailing-list discussions suggest that Git expects a
narios for applications, rather than only the guarantees         fully-ordered file system [27]. Mercurial does not pro-
listed in their documentation. Indeed, for some applica-         vide any guarantees, but does provide a plethora of man-
tions (Git, Mercurial), we could not find any documented         ual recovery techniques. Our workloads add two files to
guarantees. We also consider file-system behaviors that          the repository and then commit them. The checker uses
may not be common now, but may become prevalent in               commands like git-log, git-fsck, and git-commit
the future (especially with new classes of I/O devices).         to verify repository state, checking the integrity of the
Moreover, the number of vulnerabilities we report (in            repository and the durability of the workload commands.
each application) only relates to the number of source           The checkers remove any leftover lock files, and perform
code lines depending on file-system behavior. Note that,         recovery techniques that do not discard committed data
due to these reasons, the study is not suitable for com-         or require previous backups.
paring the correctness between different applications, or           Virtualization and Distributed Systems.              The
towards strictly verifying application correctness.              VMWare Player workload issues writes and flushes from
   We first describe the workloads and checkers used             within the guest; the checker repairs the virtual disk
in detecting vulnerabilities (§4.1). We then present an          and verifies that flushed writes are durable. HDFS is
overview of the protocols and vulnerabilities found in           configured with replicated metadata and restore enabled.
different applications (§4.2). We discuss the importance         HDFS and ZooKeeper workloads create a new directory
of the discovered vulnerabilities (§4.3), interesting pat-       hierarchy; the checker tests that files created before the
terns observable among the vulnerabilities (§4.4), and           crash exist. In ZooKeeper, the checker also verifies that
whether vulnerabilities are exposed on current file sys-         quota and ACL modifications are consistent.
tems (§4.5). We also evaluate whether A LICE can vali-              If A LICE finds a vulnerability related to a system call,
date new file-system designs (§4.6).                             it does not search for other vulnerabilities related to the
                                                                 same call. If the system call is involved in multiple, log-
4.1   Workloads and Checkers                                     ically separate vulnerabilities, this has the effect of hid-
Most applications have configuration options that change         ing some of the vulnerabilities. Most tested applications,
the update protocol or application crash guarantees. Our         however, have distinct, independent sets of failures (e.g.,
workloads test a total of 34 such configuration options          dirstate and repository corruption in Mercurial, consis-
across the 11 applications. Our checkers are conceptu-           tency and durability violation in other applications). We
ally simple: they do read operations to verify workload          use different checkers for each type of failure, and report
invariants for that particular configuration, and then try       vulnerabilities for each checker separately.
writes to the datastore. However, some applications have            Summary. If application invariants for the tested con-
complex invariants, and recovery procedures that they            figuration are explicitly and conspicuously documented,
expect users to carry out (such as removing a leftover           we consider violating those invariants as failure; other-
lock file). Our checkers are hence complex (e.g., about          wise, our checkers consider violating a lay user’s expec-
500 LOC for Git), invoking all recovery procedures we            tations as failure. We are careful about any recovery
are aware of that are expected of normal users.                  procedures that need to be followed on a system crash.
   We now discuss the workloads and checkers for                 Space constraints here limit exact descriptions of the
each application class. Where applicable, we also                checkers; we provide more details in our webpage [2].
present the guarantees we believe each application makes
to users, information garnered from documentation,               4.2    Overview
mailing-list discussions, interaction with developers, and       We now discuss the logical protocols of the applications
other relevant sources.                                          examined. Figure 4 visually represents the update proto-
   Key-value Stores and Relational Databases. Each               cols, showing the logical operations in the protocol (or-
workload tests different parts of the protocol, typically        ganized as modules) and discovered vulnerabilities.
opening a database, and inserting enough data to trigger
checkpoints. The checkers check for atomicity, ordering,         4.2.1 Databases and Key-Value Stores
and durability of transactions. We note here that GDBM           Most databases use a variant of write-ahead logging.
does not provide any crash guarantees, though we be-             First, the new data is written to a log. Then, the log is
lieve lay users will be affected by any loss of integrity.       checkpointed or compacted, i.e., the actual database is
Similarly, SQLite does not provide durability under the          usually overwritten, and the log is deleted.
default journal-mode (we became aware of this only af-              Figure 4(A) shows the protocol used by LevelDB-
ter interacting with developers), but its documentation          1.15. LevelDB adds inserted key-value pairs to the log
seems misleading. We enable checksums on LevelDB.                until it reaches a threshold, and then switches to a new

                                                             8
440 11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14)                     USENIX Association
                        creat(x.ldb)                                                                         creat(tmp)                          write(pg xlog)
                                                                                                0
                   N x append(x.ldb)                                write(mdb file)                                                         fdatasync(pg xlog)
                                                                                                1           append(tmp)
                      fdatasync(x.ldb)                             append(mdb file)                                                              write(pg clog)
                                                                                                2           fsync(tmp)
                                                                fdatasync(mdb file)
            ?x    {    creat(new.log)
                                                                   [write(mdb file)]
                                                                                                3       [
                                                                                                     unlink(props)         ]                fdatasync(pg clog)
                                                                                                                                                    ....
                      creat(mani-new)                                                           4 rename(tmp, props)                        [write(pg control)      ]
              [N x append(mani-new)]                         file sync range(mdb file)
                                                                                                        (D)(i) HSQLDB
                                                                                                                                             fsync(pg control)
                      fsync(parent-dir)                                  (B) LMDB                        update props                            (E) Postgres
                  fdatasync(mani-new)                                                                                                             checkpoint
                         creat(tmp)                                                                     [append(log)]
 ?x
            {          append(tmp)
                      fdatasync(tmp)
              [rename(tmp, current)]
                      unlink(mani-old)
                                                                         creat(db)
                                                                   N x append(db)
                                                                         fsync(db)
                                                                   N x append(db)
                                                                                                 )*
                                                                                               i(4
                                                                                                        N x fsync(log)
                                                                                                           creat(stmp)
                                                                                                        append(stmp)
                                                                                                           fsync(stmp)
                                                                                                                                                 creat(journal)
                                                                                                                                            N x append(journal)
                                                                                                                                                 fsync(journal)
                                                                                                                                             fsync(parent-dir)
                       unlink(old.log)                                                                 (i) update props                          write(journal)
                                                                     N x write(db)             i(4         unlink(log)   (i)4
            (A)(i) LevelDB compaction                                                            )*                                              fsync(journal)
                                                                     ? x fsync(db)             (i)3     unlink(script)
                      creat(new.log)                                                                                                                write(db)
              [N x append(new.log)]
                                                                     stdout(done)
                                                                                                [rename(stmp, script)]                              fsync(db)
                                                                   (C) GDBM create                     (i) update props
              ? x fdatasync(new.log)                                                                                            (i)3            unlink(journal)
                                                                       and insert
                      stdout(done)                                                                     (D)(ii) HSQLDB                            stdout(done)
              (A)(ii) LevelDB insert                                                                     shutdown
                                                                                                                                                    (F) SQLite



              0            mkdir(o/x)                                                                          creat(tmp)
                                                                                                                                                    mkdir(v)
              1         creat(o/x/tmp y)                                 creat(tmp)
                                                                     append(tmp)
                                                                                                              append(tmp)              ?x   {    creat(v/log)
              2 N x append(o/x/tmp y)                                                                          fsync(tmp)
                                                                                                                                                append(v/log)
              3    fsync(o/x/tmp y)                            [rename(tmp, dirstate)]                 [rename(tmp, x.vmdk)]                     trunc(v/log)
                      link(o/x/tmp y, o/x/y)              (H)(i) Mercurial update dirstate                   write(x-split1)
              4
              5        unlink(o/x/tmp y)                                    ...
                                                                     creat(journal)
                                                                                                Nx     {fsync region(x-split1)
                                                                                                                                                append(v/log)
                                                                                                                                                [ write(v/log)  ]
                   (G)(i) Git store object                                creat(filelog)               (I) VMWare write-flush                   ? x write(v/log)


                                                               {
                                                                                                                                                                 ’’
                   creat(index.lock)
                                                          Nx
                                                                     [
                                                                     append(journal)       ]                     ....
                                                                                                                                                ? x write(v/log)
                  N x (i) store object     (i)                      N x append(filelog)                      creat(tmp)                      fdatasync(v/log)
                                               0,                           ...
                  append(index.lock)             (i)
                                                  4                  [
                                                                     append(journal) ]                      append(tmp)                          stdout(done)
            [rename(index.lock, index)]                             [append(manifest)]           [rename(tmp, seen txid)]                    (K) ZooKeeper
                  stdout(finished add)
                                                                     [append(journal)]                       creat(ckpt)
                  N x (i) store object                              append(changelog)                       append(ckpt)
                  creat(branch.lock)           (i)0
                                                    4              rename(journal, undo)                     fsync(ckpt)
                                                ,(i)                        ...
(i)0,(i)4
                 append(branch.lock)                                                                       creat(md5.tmp)                    Legend
                                                                        creat(tmp)
                 append(branch.lock)                                                                  N x append(md5.tmp)                    Safe flush, rename
                                                                          append(tmp)
                append(logs/branch)
                  append(logs/HEAD)
                                                                [rename(tmp,
                                                                          ...
                                                                              fncache)]
                                                                                                           fsync(md5.tmp)                    Other ordering

     rename(branch.lock, x/branch)
                                                                      update dirstate
                                                                                                    rename(md5.tmp, md5)               []    Atomicity
                                                                            ...                       rename(ckpt, fsimage)
              stdout(finished commit)                                                                          ....
                                                                (H)(ii) Mercurial commit
                                                                                                        (J) HDFS update
              (G)(ii) Git add commit



Figure 4: Protocol Diagrams. The diagram shows the modularized update protocol for all applications. For applications with more than
one configuration (or versions), only a single configuration is shown (SQLite: Rollback, LevelDB: 1.15). Uninteresting parts of the protocol and
a few vulnerabilities (similar to those already shown) are omitted. Repeated operations in a protocol are shown as ‘N ×’ next to the operation,
and portions of the protocol executed conditionally are shown as ‘? ×’. Blue-colored text simply highlights such annotations and sync calls.
Ordering and durability dependencies are indicated with arrows, and dependencies between modules are indicated by the numbers on the arrows,
corresponding to line numbers in modules. Durability dependency arrows end in an stdout micro-op; additionally, the two dependencies marked
with * in HSQLDB are also durability dependencies. Dotted arrows correspond to safe rename or safe file flush vulnerabilities discussed in
Section 4.4. Operations inside brackets must be persisted together atomically.



                                                                                           9

USENIX Association                                    11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14) 441
log; during the switch, a background thread starts com-             file maintains consistency (though VMWare does use
pacting the old log file. Figure 4(A)(i) shows the com-             update-via-rename for some small files). Both HDFS and
paction; Figure 4(A)(ii) shows the appends to the log file.         ZooKeeper use write-ahead logging. Figure 4(K) shows
During compaction, LevelDB first writes data to a new               the ZooKeeper logging module. We find that ZooKeeper
ldb file, updates pointers to point to the new file (by ap-         does not explicitly persist directory entries of log files,
pending to a manifest), and then deletes the old log file.          which can lead to lost data. ZooKeeper also requires
   In LevelDB, we find vulnerabilities occurring while              some log writes to be atomic.
appending to the log file. A crash can result in the ap-
pended portion of the file containing garbage; LevelDB’s            4.3    Vulnerabilities Found
recovery code does not properly handle this situation,              A LICE finds 60 static vulnerabilities in total, correspond-
and the user gets an error if trying to access the inserted         ing to 156 dynamic vulnerabilities. Altogether, applica-
key-value pair (which should not exist in the database).            tions failed in more than 4000 crash states. Table 3(a)
We also find some vulnerabilities occurring during com-             shows the vulnerabilities classified by the affected per-
paction. For example, LevelDB does not explicitly per-              sistence property, and 3(b) shows the vulnerabilities clas-
sist the directory entries of ldb files; a crash might cause        sified by failure consequence. Table 3(b) also separates
the files to vanish, resulting in unavailability.                   out those vulnerabilities related only to user expectations
   Some databases follow protocols that are radically dif-          and not to documented guarantees, with an asterik (∗ );
ferent from write-ahead logging. For example, LMDB                  many of these correspond to applications for which we
uses shadow-paging (copy-on-write). LMDB requires                   could not find any documentation of guarantees.
that the final pointer update (106 bytes) in the copy-on-              The different journal-mode configurations provided by
write tree to be atomic. HSQLDB uses a combination of               SQLite use different protocols, and the different versions
write-ahead logging and update-via-rename, on the same              of LevelDB differ on whether their protocols are de-
files, to maintain consistency. The update-via-rename               signed around the mmap() interface. Tables 3(a) and 3(b)
is performed by first separately unlinking the destina-             hence show these configurations of SQLite and LevelDB
tion file, and then renaming; out-of-order persistence of           separately. All other configurations (in all applications)
rename(), unlink(), or log creation causes problems.                do not change the basic protocol, but vary on the appli-
4.2.2 Version Control Systems                                       cation invariants; among different configurations of the
Git and Mercurial maintain meta-information about their             same update protocol, all vulnerabilities are revealed in
repository in the form of logs. The Git protocol is il-             the safest configuration. Table 3 and the rest of the paper
lustrated in Figure 4(G). Git stores information in the             only show vulnerabilities we find in the safest configura-
form of object files, which are never modified; they are            tion, i.e., we do not count separately the same vulnerabil-
created as temporary files, and then linked to their per-           ities from different configurations of the same protocol.
manent file names. Git also maintains pointers in sepa-                We find many vulnerabilities have severe conse-
rate files, which point to both the meta-information log            quences such as silent errors or data loss. Seven applica-
and the object files, and are updated using update-via-             tions are affected by data loss, while two (both LevelDB
rename. Mercurial, on the other hand, uses a journal to             versions and HSQLDB) are affected by silent errors. The
maintain consistency, using update-via-rename only for              cannot open failures include failure to start the server in
a few unimportant pieces of information.                            HDFS and ZooKeeper, while the failed reads and writes
   We find many ordering dependencies in the Git proto-             include basic commands (e.g., git-log, git-commit)
col, as shown in Figure 4(G). This result is not surpris-           failing in Git and Mercurial. A few cannot open fail-
ing, since mailing-list discussions suggest Git developers          ures and failed reads and writes might be solvable by
expect total ordering from the file system. We also find            application experts, but we believe lay users would have
a Git vulnerability involving atomicity across multiple             difficulty recovering from such failures (our checkers in-
system calls; a pointer file being updated (via an append)          voke standard recovery techniques). We also checked
has to be persisted atomically with another file getting            if any discovered vulnerabilities are previously known,
updated (via an update-via-rename). In Mercurial, we                or considered inconsequential. The single PostgreSQL
find many ordering vulnerabilities for the same reason,             vulnerability is documented; it can be solved with non-
not being designed to tolerate out-of-order persistence.            standard (although simple) recovery techniques. The sin-
                                                                    gle LMDB vulnerability is discussed in a mailing list,
4.2.3 Virtualization and Distributed Systems                        though there is no available workaround. All these pre-
VMWare Player’s protocol is simple. VMWare main-                    viously known vulnerabilities are separated out in Ta-
tains a static, constant mapping between blocks in the              ble 3(b) († ). The five dirstate fail vulnerabilities in Mer-
virtual disk, and in the VMDK file (even for dynami-                curial are shown separately, since they are less harmful
cally allocated VMDK files); directly overwriting the VMDK          than other vulnerabilities (though frustrating to the lay

                                                               10
442 11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14)                         USENIX Association
                             Types                                                                                                                    Application ext3-w ext3-o ext3-j ext4-o btrfs




                                                                    Unique static vulnerabilities
Application       Atomicity Ordering Durability
              Across-syscalls atomicity                                                                       Silent errors                           Leveldb1.10    3      1      1      2     4
                                                                                                                                                      Leveldb1.15    2      1      1      2     3
              Appends and truncates                                                                           Data loss                               LMDB
              Single-block overwrites                                                                                                                 GDBM           3      3      2      3     4
              Renames and unlinks                                                                             Cannot open                             HSQLDB                                    4



                                          Safe file flush
                                                                                                                                                      Sqlite-Roll    1      1      1      1     1
              Safe file flush
                                                                                                              Failed reads and writes
                                                                                                                                                      Sqlite-WAL
              Safe renames                                                                          Application                 Other                 PostgreSQL
                                                            Other
                                                                                                                                                      Git            2      2      2      2     5
              Other                                                                                 Leveldb1.10 1 1 5 4
                                                                                                                                                      Mercurial      4      3      3      6     8
                                                                                                    Leveldb1.15 2    2 2
Leveldb1.10 1‡ 1          1 2 1 3            1                          10                                                                            VMWare
Leveldb1.15 1 1           1 1   2                                       6                           LMDB                   read-only open†            HDFS                                      1
LMDB                  1                                                 1                           GDBM          2∗ 3∗                               ZooKeeper      1      1             1     1
GDBM        1             1         1                        2          5                           HSQLDB 2 3 5                                      Total         16     12     10     17    31
HSQLDB         1          2 1       3        2               1          10                          Sqlite-Roll   1∗                                          (c) Under Current File Systems.
Sqlite-Roll                                                  1          1                           Sqlite-WAL                                                             Ordering                DO AGCA
Sqlite-WAL                                                               0                          PostgreSQL       1†                               ext3-w Dir ops and file-sizes ordered among  4K ×
PostgreSQL            1                                                 1                           Git           1∗ 3∗ 5∗       3#∗
                                                                                                                                                             themselves, before sync operations.
Git         1             1 2 1 3                            1          9                                                                             ext3-o Dir ops, appends, truncates ordered  4K 
                                                                                                    Mercurial     2∗ 1∗ 6∗ 5 dirstate fail∗                  among themselves. Overwrites be-
Mercurial   2 1           1    1 4                           2          10
VMWare                    1                                             1                           VMWare           1∗                                      fore non-overwrites, all before sync.
                                                                                                    HDFS             2∗                               ext3-j All operations are ordered.            4K 
HDFS                      1      1                                      2
                                                                                                                                                      ext4-o Safe rename, safe file flush, dir ops  4K 
ZooKeeper             1          1           2                          4                           ZooKeeper     2∗ 2∗
                                                                                                                                                             ordered among themselves
Total       6 4       3 9 6 3 18             5               7          60                          Total       5 12 25 17        9                   btrfs Safe rename, safe file flush            4K 
                      (a) Types.                                                                          (b) Failure Consequences.                                (d) APMs considered.
Table 3: Vulnerabilities.           (a) shows the discovered static vulnerabilities categorized by the type of persistence property. The number of
unique vulnerabilities for an application can be different from the sum of the categorized vulnerabilities, since the same source code lines can
exhibit different behavior. ‡ The atomicity vulnerability in Leveldb1.10 corresponds to multiple mmap() writes. (b) shows the number of static
vulnerabilities resulting in each type of failure. † Previously known failures, documented or discussed in mailing lists. ∗ Vulnerabilities relating
to unclear documentation or typical user expectations beyond application guarantees. # There are 2 fsck-only and 1 reflog-only errors in Git. (c)
shows the number of vulnerabilities that occur on current file systems (all applications are vulnerable under future file systems). (d) shows APMs
used for calculating Table (c). Legend: DO: directory operations atomicity. AG: granularity of size-atomicity. CA: Content-Atomicity.
user). Git’s fsck-only and reflog-only errors are poten-                                                                            behavior guaranteed by SQLite (specifically, that dura-
tially dangerous, but do not seem to affect normal usage.                                                                           bility cannot be achieved under rollback journaling);
   We interacted with the developers of eight applica-                                                                              we believe the documentation is misleading.
tions, reporting a subset of the vulnerabilities we find.                                                                              Of the five acted-on vulnerabilities, three relate to not
Our interactions convince us that the vulnerabilities will                                                                          explicitly issuing an fsync() on the parent directory af-
affect users if they are exposed. The other applications                                                                            ter creating and calling fsync() on a file. However, not
(GDBM, Git, and Mercurial) were not designed to pro-                                                                                issuing such an fsync() is perhaps more safe in mod-
vide crash guarantees, although we believe their users                                                                              ern file systems than out-of-order persistence of directory
will be affected by the vulnerabilities found should an                                                                             operations. We believe the developers’ interest in fixing
untimely crash occur. Thus, the vulnerabilities will not                                                                            this problem arises from the Linux documentation ex-
surprise a developer of these applications, and we did not                                                                          plicitly recommending an fsync() after creating a file.
report them. We also did not report vulnerabilities con-                                                                               Summary. A LICE detects 60 vulnerabilities in total,
cerning partial renames (usually dismissed since they are                                                                           with 5 resulting in silent failures, 12 in loss of durability,
not commonly exposed), or documented vulnerabilities.                                                                               25 leading to inaccessible applications, and 17 returning
                                                                                                                                    errors while accessing certain data. A LICE is also able to
   Developers have acted on five of the vulnerabilities                                                                             detect previously known vulnerabilities.
we find: one (LevelDB-1.10) is now fixed, another
(LevelDB-1.15) was fixed parallel to our discovery, and                                                                             4.4    Common Patterns
three (HDFS, and two in ZooKeeper) are under consider-                                                                              We now examine vulnerabilities related with different
ation. We have found that developers often dismiss other                                                                            persistence properties. Since durability vulnerabilities
vulnerabilities which do not (or are widely believed to                                                                             show a separate pattern, we consider them separately.
not) get exposed in current file systems, especially relat-
ing to out-of-order persistence of directory operations.                                                                            4.4.1 Atomicity across System Calls
The fact that only certain operating systems allow an                                                                               Four applications (including both versions of LevelDB)
fsync() on a directory is frequently referred to; both                                                                              require atomicity across system calls. For three applica-
HDFS and ZooKeeper respondents lament that such an                                                                                  tions, the consequences seem minor: inaccessibility dur-
fsync() is not easily achievable with Java. The devel-                                                                              ing database creation in GDBM, dirstate corruption in
opers suggest the SQLite vulnerability is actually not a                                                                            Mercurial, and an erratic reflog in Git. LevelDB’s vul-

                                                                                                                            11
USENIX Association                                     11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14) 443
nerability has a non-minor consequence, but was fixed                  Safe renames. On file systems with delayed alloca-
immediately after introducing LevelDB-1.15 (when Lev-               tion, a common heuristic to prevent data loss is to persist
elDB started using read()-write() instead of mmap()).               all data (including appends and truncates) of a file before
   In general, we observe that this class of vulnerabilities        subsequent renames of the file [14]. We find that this
seems to affect applications less than other classes. This          heuristic only matches (and thus fixes) three discovered
result may arise because these vulnerabilities are easily           vulnerabilities, one each in Git, Mercurial, and LevelDB-
tested: they are exposed independent of the file system             1.10. A related heuristic, where updating existing files
(i.e, via process crashes), and are easier to reproduce.            by opening them with O TRUNC flushes the updated data
                                                                    while issuing a close(), does not affect any of the vul-
4.4.2 Atomicity within System Calls                                 nerabilities we discovered. Also, the effect of the heuris-
Append atomicity. Surprisingly, three applications re-              tics varies with minor details: if the safe-rename heuris-
quire appends to be content-atomic: the appended por-               tic does not persist file truncates, only two vulnerabilities
tion should contain actual data. The failure consequences           will be fixed; if the O TRUNC heuristic also acts on new
are severe, such as corrupted reads (HSQLDB), failed                files, an additional vulnerability will be fixed.
reads (LevelDB-1.15) and repository corruption (Mercu-                 Safe file flush. An fsync() on a file does not guaran-
rial). Filling the appended portion with zeros instead of           tee that the file’s directory entry is also persisted. Most
garbage still causes failure; only the current implemen-            file systems, however, persist directory entries that the
tation of delayed allocation (where file size does not in-          file is dependent on (e,g., directory entries of the file and
crease until actual content is persisted) works. Most ap-           its parent). We found that this behavior is required by
pends seemingly do not need to be block-atomic; only                three applications for maintaining basic consistency.
Mercurial is affected, and the affected append also re-
quires content-atomicity.                                           4.4.4 Durability
   Overwrite atomicity. LMDB, PostgreSQL, and                       We find vulnerabilities in seven applications resulting in
ZooKeeper require small writes (< 200 bytes) to be                  durability loss. Of these, only two applications (GDBM
atomic. Both the LMDB and PostgreSQL vulnerabilities                and Mercurial) are affected because an fsync() is not
are previously known.                                               called on a file. Six applications require fsync() calls
   We do not find any multi-block overwrite vulnerabil-             on directories: three are affected by safe file flush dis-
ities, and even single-block overwrite requirements are             cussed previously, while four (HSQLDB, SQLite, Git,
typically documented. This finding is in stark contrast             and Mercurial) require other fsync() calls on directo-
with append atomicity; some of the difference can be                ries. As a special case, with HSQLDB, previously com-
attributed to the default APM (overwrites are content-              mitted data is lost, rather than data that was being com-
atomic), and to some workloads simply not using over-               mitted during the time of the workload. In all, only
writes. However, the major cause seems to be the basic              four out of the twelve vulnerabilities are exposed when
mechanism behind application update protocols: mod-                 full ordering is promised: many applications do issue an
ifications are first logged, in some form, via appends;             fsync() call before durability is essential, but do not
logged data is then used to overwrite the actual data. Ap-          fsync() all the required information.
plications have careful mechanisms to detect and repair
failures in the actual data, but overlook the presence of           4.4.5 Summary
garbage content in the log.                                         We believe our study offers several insights for file-
   Directory operation atomicity. Given that most file              system designers. Future file systems should consider
systems provide atomic directory operations (§2.2), one             providing ordering between system calls, and atomic-
would expect that most applications would be vulnera-               ity within a system call in specific cases. Vulnerabili-
ble to such operations not being atomic. However, we                ties involving atomicity of multiple system calls seem to
do not find this to be the case for certain classes of ap-          have minor consequences. Requiring applications to sep-
plications. Databases and key-value stores do not em-               arately flush the directory entry of a created and flushed
ploy atomic renames extensively; consequently, we ob-               file can often result in application failures. For durability,
serve non-atomic renames affecting only three of these              most applications seem to explicitly flush some, but not
applications (GDBM, HSQLDB, LevelDB). Non-atomic                    all, of the required information; thus, providing ordering
unlinks seemingly affect only HSQLDB (which uses un-                among system calls can also help durability.
links for logically performing renames), and we did not
find any application affected by non-atomic truncates.
                                                                    4.5    Impact on Current File Systems
                                                                    Our study thus far has utilized an abstract (and weak) file
4.4.3 Ordering between System Calls                                 system model (i.e., APM) in order to discover the broad-
Applications are extremely vulnerable to system calls be-           est number of vulnerabilities. We now utilize specific
ing persisted out of order; we find 27 vulnerabilities.             file-system APMs to understand how modern protocols


                                                               12
444 11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14)                          USENIX Association
would function atop a range of modern file systems and             dering vulnerabilities (of the studied applications) are not
configurations. Specifically, we focus on Linux ext3 (in-          exposed under ext3-fast. The design was not meant to fix
cluding writeback, ordered, and data-journaling mode),             durability or atomicity across system calls vulnerabili-
Linux ext4, and btrfs. The considered APMs are based               ties, so those vulnerabilities are still reported by A LICE.
on our understanding of file systems from Section 2.                  We estimate the performance gain of ext3-fast using
   Table 3(c) shows the vulnerabilities reported by                the following experiment: we first write 250 MB to
A LICE, while 3(d) shows the considered APMs. We                   file A, then append a byte to file B and fsync() B.
make a number of observations based on Table 3(c).                 When both files are on the same ext3-ordered file system,
First, a significant number of vulnerabilities are exposed         fsync() takes about four seconds. If the files belong to
on all examined file systems. Second, ext3 with jour-              different partitions on the same disk, mimicking the be-
naled data is the safest: the only vulnerabilities exposed         havior of ext3-fast, the fsync() takes only 40 ms. The
relate to atomicity across system calls, and a few dura-           first case is 100 times slower because 250 MB of data is
bility vulnerabilities. Third, a large number of vulner-           ordered before the single byte that needs to be persistent.
abilities are exposed on btrfs as it aggressively persists            Summary. The ext3-fast file system (derived from in-
operations out of order [11]. Fourth, some applications            ferences provided by A LICE) seems interesting for appli-
show no vulnerabilities on any considered APM; thus,               cation safety, though further investigation is required into
the flaws we found in such applications do not manifest            the validity of its design. We believe that the ease of use
on today’s file systems (but may do so on future systems).         offered by A LICE will allow it to be incorporated into the
   Summary. Application vulnerabilities are exposed on             design process of new file systems.
many current file systems. The vulnerabilities exposed
vary based on the file system, and thus testing applica-           4.7    Discussion
tions on only a few file systems does not work.                    We now consider why crash vulnerabilities occur com-
                                                                   monly even among widely used applications. We find
4.6    Evaluating New File-System Designs                          that application update protocols are complex and hard to
File-system modifications for improving performance                isolate and understand. Many protocols are layered and
have introduced wide-spread data loss in the past [14],            spread over multiple files. Modules are also associated
because of changes to the file-system persistence proper-          with other complex functionality (e.g., ensuring thread
ties. A LICE can be used to test whether such modifica-            isolation). This complexity leads to issues that are obvi-
tions break correctness of existing applications. We now           ous with a bird’s eye view of the protocol: for example,
describe how we use A LICE to evaluate a hypothetical              HSQLDB’s protocol has 3 consecutive fsync() calls to
variant of ext3 (data-journaling mode), ext3-fast.                 the same file (increasing latency). A LICE helps solve this
   Our study shows that ext3 (data-journaling mode) is             problem by making it easy to obtain logical representa-
the safest file system; however, it offers poor perfor-            tions of update protocols as shown in Figure 4.
mance for many workloads [35]. Specifically, fsync()                  Another factor contributing to crash vulnerabilities is
latency is extremely high as ext3 persists all previous op-        poorly written, untested recovery code. In LevelDB, we
erations on fsync(). One way to reduce fsync() la-                 find vulnerabilities that should be prevented by correct
tency would be to modify ext3 to persist only the synced           implementations of the documented update protocols.
file. However, other file systems (e.g,. btrfs) that have          Some recovery code is non-optimal: potentially recover-
attempted to reduce fsync() latency [13] have resulted             able data is lost in several applications (e.g., HSQLDB,
in increased vulnerabilities. Our study suggests a way to          Git). Mercurial and LevelDB provide utilities to verify
reduce latency without exposing more vulnerabilities.              or recover application data; we find these utilities hard to
   Based on our study, we hypothesize that data that is            configure and error-prone. For example, an user invok-
not synced need not be persisted before explicitly synced          ing LevelDB’s recovery command can unintentionally
data for correctness; such data must only be persisted             end up further corrupting the datastore, and be affected
in-order amongst itself. We design ext3-fast to reflect            by (seemingly) unrelated configuration options (para-
this: fsync() on a file A persists only A, while other             noid checksums). We believe these problems are a direct
dirty data and files are still persisted in-order.                 consequence of the recovery code being infrequently ex-
   We modeled ext3-fast in A LICE by slightly changing             ecuted and insufficiently tested. With A LICE, recovery
the APM of ext3 data journaling mode, so that synced               code can be tested on many corner cases.
directories, files, and their data, depend only on previous           Convincing developers about crash vulnerabilities is
syncs and operations necessary for the file to exist (i.e.,        sometimes hard: there is a general mistrust surrounding
safe file flush is obeyed). The operations on a synced file        such bug reports. Usually, developers are suspicious that
are also ordered among themselves.                                 the underlying storage stack might not respect fsync()
   We test our hypothesis with A LICE; the observed or-            calls [36], or that the drive might be corrupt. We hence


                                                              13
USENIX Association             11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14) 445
believe that most vulnerabilities that occur in the wild are        system during a single workload execution. Thus, their
associated with an incorrect root cause, or go unreported.          work is more suited for finding those vulnerabilities that
A LICE can be used to easily reproduce vulnerabilities.             are commonly exposed under a given file system.
   Unclear documentation of application guarantees con-                Woodpecker [15] can be used to find crash vulnerabil-
tributes to the confusion about crash vulnerabilities. Dur-         ities when supplied with suspicious source code patterns
ing discussions with developers about durability vulner-            to guide symbolic execution. Our work is fundamentally
abilities, we found that SQLite, which proclaims itself             different to this approach, as A LICE does not require prior
as fully ACID-complaint, does not provide durability                knowledge of patterns in checked applications.
(even optionally) with the default storage engine, though              Our work is influenced by SQLite’s internal testing
the documentation suggests it does. Similarly, GDBM’s               tool [43]. The tool works at an internal wrapper layer
GDBM SYNC flag does not ensure durability. Users can                within SQLite, and is not helpful for generic testing.
employ A LICE to determine guarantees directly from the                R ACE P RO [25], a testing tool for concurrency bugs,
code, bypassing the problem of bad documentation.                   records system calls and replays them by splitting them
                                                                    into small operations, but does not test crash consistency.
5    Related Work                                                      OptFS [9], Featherstitch [16], and transactional file
Our previous workshop paper [47] identifies the prob-               systems [17, 39, 42], discuss new file-system interfaces
lem of application-level consistency depending upon file-           that will affect vulnerabilities. Our study can help inform
system behavior, but is limited to two applications and             the design of new interfaces by providing clear insights
does not use automated testing frameworks. Since we                 into what is missing in today’s interfaces.
use A LICE to obtain results, our current study includes a          6    Conclusion
greater number and variety of applications.
                                                                    In this paper, we show how application-level consistency
   This paper adapts ideas from past work on dynamic
                                                                    is dangerously dependent upon file system persistence
program analysis and model checking. E XPLODE [53]
                                                                    properties, i.e., how file systems persist system calls.
has a similar flavor to our work: the authors use in-
                                                                    We develop B OB, a tool to test persistence properties
situ model checking to find crash vulnerabilities on dif-
                                                                    and show that such properties vary widely among file
ferent storage stacks. A LICE differs from E XPLODE in
                                                                    systems. We build A LICE, a framework that analyzes
four significant ways. First, E XPLODE requires the target
                                                                    application-level protocols and detects crash vulnerabil-
storage stack to be fully implemented; A LICE only re-
                                                                    ities. We analyze 11 applications, and find 60 vulner-
quires a model of the target storage stack, and can there-
                                                                    abilities, some of which result in severe consequences
fore be used to evaluate application-level consistency on
                                                                    like corruption or data loss. We present several insights
top of proposed storage stacks, while they are still at
                                                                    derived from our study. The A LICE tool, and detailed de-
the design stage. Second, E XPLODE requires the user to
                                                                    scriptions of the vulnerabilities found in our study, can
carefully annotate complex file systems using choose()
                                                                    be obtained from our webpage [2].
calls; A LICE requires the user to only specify a high-
level APM. Third, E XPLODE reconstructs crash states by
tracking I/O as it moves from the application to the stor-          Acknowledgments
age. Although it is possible to use E XPLODE to deter-              We thank Lorenzo Alvisi (our shepherd) and the anony-
mine the root cause of a vulnerability, we believe it is            mous reviewers for their insightful comments. We
easier to do so using A LICE since A LICE checks for vi-            thank members of ADSL, application developers and
olation of specific persistence properties. Fourth, E X -           users, and file system developers, for valuable discus-
PLODE stops at finding crash vulnerabilities; by helping            sions. This material is based upon work supported by
produce protocol diagrams, A LICE contributes to under-             the NSF under CNS-1421033, CNS-1319405, and CNS-
standing the protocol itself. Like B OB, E XPLODE can be            1218405 as well as donations from EMC, Facebook,
used to test persistence properties; however, while B OB            Fusion-io, Google, Huawei, Microsoft, NetApp, Sam-
only re-orders block I/O, E XPLODE can test re-orderings            sung, Sony, and VMware. Vijay Chidambaram and
caused at different layers in the storage stack.                    Samer Al-Kiswany are supported by the Microsoft Re-
   Zheng et al. [54] find crash vulnerabilities in                  search PhD Fellowship and the NSERC Postdoctoral Fel-
databases. They contribute a standard set of workloads              lowship, respectively. Any opinions, findings, and con-
that stress databases (particularly, with multiple threads),        clusions, or recommendations expressed herein are those
and check ACID properties; the workloads and checkers               of the authors and do not necessarily reflect the views of
can be used with A LICE. Unlike our work, Zheng et al.              the NSF or other institutions.
do not systematically explore vulnerabilities of each sys-
tem call; they are limited by the re-orderings and non-
atomicity exhibited by a particular (implemented) file


                                                               14
446 11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14)                        USENIX Association
References                                                                   [22] Dave Hitz, James Lau, and Michael Malcolm. File System De-
 [1] Necessary step(s) to synchronize filename operations on disk.                sign for an NFS File Server Appliance. In Proceedings of the
     http://austingroupbugs.net/view.php?id=672.                                  USENIX Winter Technical Conference (USENIX Winter ’94), San
                                                                                  Francisco, California, January 1994.
 [2] Tool and Results:           Application Crash Vulnerabilities.
     http://research.cs.wisc.edu/adsl/Software/alice/.                       [23] HyperSQL. HSQLDB. http://www.hsqldb.org/.
 [3] Apache. Apache Zookeeper. http://zookeeper.apache.                      [24] Dean Kuo. Model and verification of a data manager based on
     org/.                                                                        aries. ACM Trans. Database Syst., 21(4):427–479, December
                                                                                  1996.
 [4] Remzi H. Arpaci-Dusseau and Andrea C. Arpaci-Dusseau. Op-               [25] Oren Laadan, Nicolas Viennot, Chia-Che Tsai, Chris Blinn, Jun-
     erating Systems: Three Easy Pieces. Arpaci-Dusseau Books, 0.8                feng Yang, , and Jason Nieh. Pervasive Detection of Process
     edition, 2014.                                                               Races in Deployed Systems. In Proceedings of the 23rd ACM
 [5] Steve Best. JFS Overview. http://jfs.sourceforge.                            Symposium on Operating Systems Principles (SOSP ’11), Cas-
     net/project/pub/jfs.pdf, 2000.                                               cais, Portugal, October 2011.
 [6] Remy Card, Theodore Ts’o, and Stephen Tweedie. Design and               [26] Linus Torvalds. Git. http://git-scm.com/, 2005.
     Implementation of the Second Extended Filesystem. In First
     Dutch International Symposium on Linux, Amsterdam, Nether-              [27] Linus Torvalds. Git Mailing List. Re: what’s the current wisdom
     lands, December 1994.                                                        on git over NFS/CIFS? http://marc.info/?l=git&m=
                                                                                  124839484917965&w=2, 2009.
 [7] Donald D Chamberlin, Morton M Astrahan, Michael W Blasgen,
     James N Gray, W Frank King, Bruce G Lindsay, Raymond Lo-                [28] Lanyue Lu, Andrea C. Arpaci-Dusseau, Remzi H. Arpaci-
     rie, James W Mehl, Thomas G Price, Franco Putzolu, et al. A                  Dusseau, and Shan Lu. A Study of Linux File System Evolu-
     history and evaluation of system r. Communications of the ACM,               tion. In Proceedings of the 11th USENIX Symposium on File and
     24(10):632–646, 1981.                                                        Storage Technologies (FAST ’13), San Jose, California, February
                                                                                  2013.
 [8] Donald D Chamberlin, Arthur M Gilbert, and Robert A Yost. A             [29] Cris Pedregal Martin and Krithi Ramamritham. Toward formaliz-
     history of system r and sql/data system. In VLDB, pages 456–464,             ing recovery of (advanced) transactions. In Advanced Transaction
     1981.                                                                        Models and Architectures, pages 213–234. Springer, 1997.
 [9] Vijay Chidambaram, Thanumalayan Sankaranarayana Pillai, An-
     drea C. Arpaci-Dusseau, and Remzi H. Arpaci-Dusseau. Opti-              [30] Chris Mason.        The Btrfs Filesystem.        oss.oracle.
     mistic Crash Consistency. In Proceedings of the 24th ACM Sym-                com/projects/btrfs/dist/documentation/
     posium on Operating Systems Principles (SOSP ’13), Nemacolin                 btrfs-ukuug.pdf, September 2007.
     Woodlands Resort, Farmington, Pennsylvania, October 2013.               [31] Matt Mackall. Mercurial. http://mercurial.selenic.
[10] Vijay Chidambaram, Tushar Sharma, Andrea C. Arpaci-Dusseau,                  com/, 2005.
     and Remzi H. Arpaci-Dusseau. Consistency Without Ordering. In           [32] Marshall K. McKusick, William N. Joy, Sam J. Leffler, and
     Proceedings of the 10th USENIX Symposium on File and Storage                 Robert S. Fabry. A Fast File System for UNIX. ACM Trans-
     Technologies (FAST ’12), pages 101–116, San Jose, California,                actions on Computer Systems, 2(3):181–197, August 1984.
     February 2012.                                                          [33] C. Mohan, D. Haderle, B. Lindsay, H. Pirahesh, and P. Schwarz.
[11] Chris Mason. Btrfs Mailing List. Re: Ordering of direc-                      ARIES: A Transaction Recovery Method Supporting Fine-
     tory operations maintained across system crashes in Btrfs?                   Granularity Locking and Partial Rollbacks Using Write-Ahead
     http://www.spinics.net/lists/linux-btrfs/                                    Logging. ACM Transactions on Database Systems, 17(1):94–
     msg32215.html, 2014.                                                         162, March 1992.
[12] Jeremy Condit, Edmund B. Nightingale, Christopher Frost, Engin          [34] C Mohan, Bruce Lindsay, and Ron Obermarck. Transaction
     Ipek, Benjamin Lee, Doug Burger, and Derrick Coetzee. Better                 management in the r* distributed database management system.
     I/O Through Byte-addressable, Persistent Memory. In Proceed-                 ACM Transactions on Database Systems (TODS), 11(4):378–
     ings of the 22nd ACM Symposium on Operating Systems Princi-                  396, 1986.
     ples (SOSP ’09), Big Sky, Montana, October 2009.                        [35] Vijayan Prabhakaran, Andrea C. Arpaci-Dusseau, and Remzi H.
[13] Jonathan Corbet. Solving the Ext3 Latency Problem. http:                     Arpaci-Dusseau. Analysis and Evolution of Journaling File Sys-
     //lwn.net/Articles/328363/, 2009.                                            tems. In Proceedings of the USENIX Annual Technical Confer-
                                                                                  ence (USENIX ’05), pages 105–120, Anaheim, California, April
[14] Jonathan Corbet. That massive filesystem thread. http://                     2005.
     lwn.net/Articles/326471/, March 2009.
                                                                             [36] Abhishek Rajimwale, Vijay Chidambaram, Deepak Ramamurthi,
[15] Heming Cui, Gang Hu, Jingyue Wu, and Junfeng Yang. Veri-                     Andrea C. Arpaci-Dusseau, and Remzi H. Arpaci-Dusseau. Co-
     fying systems rules using rule-directed symbolic execution. In               erced Cache Eviction and Discreet-Mode Journaling: Dealing
     Proceedings of the eighteenth international conference on Archi-             with Misbehaving Disks. In Proceedings of the International
     tectural support for programming languages and operating sys-                Conference on Dependable Systems and Networks (DSN ’11),
     tems, pages 329–342. ACM, 2013.                                              Hong Kong, China, June 2011.
[16] Christopher Frost, Mike Mammarella, Eddie Kohler, Andrew                [37] Hans Reiser. ReiserFS. www.namesys.com, 2004.
     de los Reyes, Shant Hovsepian, Andrew Matsuoka, and Lei
     Zhang. Generalized File System Dependencies. In Proceed-                [38] Mendel Rosenblum and John Ousterhout. The Design and Imple-
     ings of the 21st ACM Symposium on Operating Systems Princi-                  mentation of a Log-Structured File System. ACM Transactions
     ples (SOSP ’07), pages 307–320, Stevenson, Washington, Octo-                 on Computer Systems, 10(1):26–52, February 1992.
     ber 2007.                                                               [39] Frank Schmuck and Jim Wylie. Experience with transactions in
[17] Bill Gallagher, Dean Jacobs, and Anno Langen. A High-                        quicksilver. In ACM SIGOPS Operating Systems Review, vol-
     performance, Transactional Filestore for Application Servers. In             ume 25, pages 239–253. ACM, 1991.
     Proceedings of the 2005 ACM SIGMOD International Confer-                [40] Konstantin Shvachko, Hairong Kuang, Sanjay Radia, and Robert
     ence on Management of Data (SIGMOD ’05), pages 868–872,                      Chansler. The Hadoop Distributed File System. In Proceedings
     Baltimore, Maryland, June 2005.                                              of the 26th IEEE Symposium on Mass Storage Systems and Tech-
[18] Gregory R. Ganger and Yale N. Patt. Metadata Update Perfor-                  nologies (MSST ’10), Incline Village, Nevada, May 2010.
     mance in File Systems. In Proceedings of the 1st Symposium              [41] Stewart Smith. Eat My Data: How everybody gets file I/O wrong.
     on Operating Systems Design and Implementation (OSDI ’94),                   In OSCON, Portland, Oregon, July 2008.
     pages 49–60, Monterey, California, November 1994.                       [42] R. P. Spillane, S. Gaikwad, E. Zadok, C. P. Wright, and
[19] GNU. GNU Database Manager (GDBM). http://www.gnu.                            M. Chinni. Enabling transactional file access via lightweight ker-
     org.ua/software/gdbm/gdbm.html, 1979.                                        nel extensions. In Proceedings of the Seventh USENIX Confer-
[20] Google.      LevelDB.      https://code.google.com/p/                        ence on File and Storage Technologies (FAST ’09), pages 29–42,
     leveldb/, 2011.                                                              San Francisco, CA, February 2009. USENIX Association.
[21] Robert Hagmann. Reimplementing the Cedar File System Using              [43] SQLite. SQLite transactional SQL database engine. http://
     Logging and Group Commit. In Proceedings of the 11th ACM                     www.sqlite.org/.
     Symposium on Operating Systems Principles (SOSP ’87), Austin,           [44] Sun Microsystems. ZFS: The last word in file systems. www.
     Texas, November 1987.                                                        sun.com/2004-0914/feature/, 2006.


                                                                        15
USENIX Association                   11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14) 447
[45] Adan Sweeney, Doug Doucette, Wei Hu, Curtis Anderson, Mike
     Nishimoto, and Geoff Peck. Scalability in the XFS File Sys-
     tem. In Proceedings of the USENIX Annual Technical Conference
     (USENIX ’96), San Diego, California, January 1996.
[46] Symas. Lightning Memory-Mapped Database (LMDB). http:
     //symas.com/mdb/, 2011.
[47] Thanumalayan Sankaranarayana Pillai, Vijay Chidambaram, Joo-
     young Hwang, Andrea C. Arpaci-Dusseau, Remzi H. Arpaci-
     Dusseau. Towards Efficient, Portable Application-Level Con-
     sistency. In Proceedings of the 9th Workshop on Hot Topics in
     Dependable Systems (HotDep ’13), Farmington, PA, November
     2013.
[48] The Open Group. POSIX.1-2008 IEEE Std 1003.1. http://
     pubs.opengroup.org/onlinepubs/9699919799/,
     2013.
[49] The PostgreSQL Global Development Group. PostgreSQL.
     http://www.postgresql.org/.
[50] Theodore Ts’o and Stephen Tweedie. Future Directions for the
     Ext2/3 Filesystem. In Proceedings of the USENIX Annual Tech-
     nical Conference (FREENIX Track), Monterey, California, June
     2002.
[51] Stephen C. Tweedie. Journaling the Linux ext2fs File System. In
     The Fourth Annual Linux Expo, Durham, North Carolina, May
     1998.
[52] VMWare. VMWare Player. http://www.vmware.com/
     products/player.
[53] Junfeng Yang, Can Sar, and Dawson Engler. EXPLODE: A
     Lightweight, General System for Finding Serious Storage Sys-
     tem Errors. In Proceedings of the 7th Symposium on Operating
     Systems Design and Implementation (OSDI ’06), Seattle, Wash-
     ington, November 2006.
[54] Mai Zheng, Joseph Tucek, Dachuan Huang, Feng Qin, Mark Lil-
     libridge, Elizabeth S Yang, Bill W Zhao, and Shashank Singh.
     Torturing Databases for Fun and Profit. In 11th USENIX Sym-
     posium on Operating Systems Design and Implementation (OSDI
     ’14), Broomfield, CO, October 2014.




                                                                       16

448 11th USENIX Symposium on Operating Systems Design and Implementation (OSDI ’14)   USENIX Association
