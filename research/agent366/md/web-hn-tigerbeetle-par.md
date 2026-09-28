<!-- source: https://news.ycombinator.com/item?id=36683353 -->

|  |  |  |  |
| --- | --- | --- | --- |
| |  |  |  | | --- | --- | --- | |  | **[Hacker News](news)**[new](newest) | [past](front) | [comments](newcomments) | <ask> | <show> | <jobs> | <submit> | [login](login?goto=item%3Fid%3D36683353) | |
|
| |  |  |  | | --- | --- | --- | |  |  | [jorangreef](user?id=jorangreef) [on July 11, 2023](item?id=36683353)  | [parent](item?id=36682155) | [context](item?id=36680535#36683353) | [favorite](fave?id=36683353&auth=f76f24ece782ff839725b39473d532609563fbdb) | on: [We put a distributed database in the browser and m...](item?id=36680535 "We put a distributed database in the browser and made a game of it")   Thanks for the question! Joran from TigerBeetle here. The research in question is the 2018 paper from UW-Madison, “Protocol-Aware Recovery for Consensus-Based Storage” (PAR) [0] by Ram Alagappan, Aishwarya Ganesan, as well as Remzi and Andrea Arpaci-Dusseau (who you may recognize as authors of OSTEP). PAR won best paper at FAST '18 for showing that a single disk sector fault, in the write-ahead log (WAL) of a single replica, could propagate through the distributed RAFT or MultiPaxos consensus protocol, to cause global cluster data loss. This was counter-intuitive at the time, because PAR showed that the redundancy of these consensus and replication protocols did not in fact always imply fault-tolerance, as had previously been assumed. The reason is, and we cover this in depth in our recent QCon London talk [1], but it was assumed that checksums alone would be sufficient to detect and recover from storage faults. However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot. What you tend to find is that the WALs for many of these protocols will truncate the WAL at the first sign of a checksum mismatch, conflating the mismatch with power loss when it might be bitort, and thereby truncating committed transactions, and undermining quorum votes in the Raft or MultiPaxos implementations. RAID solutions don't always help here, either. See "Parity Lost and Parity Regained" [2] for more details. ZRAID is better here, and ZFS is a huge inspiration, but with local redundancy under ZFS you're still not leveraging the global redundancy of the consensus protocol as well as you could be. To summarize PAR: There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly. Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum. However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss. On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory. But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware. Thanks for the question and happy to answer more! [0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan) [1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY> [2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained) [3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf> |     |  |  |  |  | | --- | --- | --- | --- | | |  |  |  | | --- | --- | --- | |  |  | [petrosagg](user?id=petrosagg) [on July 11, 2023](item?id=36684025)  [[–]](javascript:void(0))   Thank you for the detailed response! > However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot. The PAR paper states that "although Crash preserves safety, it suffers from severe unavailability". I assume that when TigerBeetle loads state from RAM into a CPU cache/register it operates under the NoDetection consistency model or the Crash consistency model if ECC RAM automatically resets the CPU on read errors. At the same time it doesn't suffer from severe unavailability so what gives? The answer is probably that ECC RAM is just reliable enough that the NoDetection/Crash models are fine in practice. I can believe that off-the-shelf checksum and redundancy options offered by filesystems like ext4 and ZFS or systems like RAID don't hit the required error probabilities but why does the argument stop there? Couldn't a distributed database generate error correcting data on every write in the application layer so that the probability becomes low enough such that NoDetection/Crash become a non-issue for storage, just like RAM? Is there some other fundamental difference between reading and write data from RAM versus a disk? | | | |  |  |  | | --- | --- | --- | |  |  | [jorangreef](user?id=jorangreef) [on July 11, 2023](item?id=36684200)  | [parent](#36684025) [[–]](javascript:void(0))   Huge pleasure, thanks again for the question! The crux of the problem: How do you solve misdirected read/write I/O? Where the firmware writes/reads to/from the wrong disk sector (but with a valid checksum)? PAR shows how both global consensus protocol and local storage engine need to be modified for this, with foundational design changes at the protocol-level, if a distributed system is to not only preserve correctness, but also optimize for high availability. Bear in mind that PAR is not only actually correct, but it's also more efficient than simply dialing up local redundancy, because it lets you recover from the global redundancy that you have via replication in the consensus protocol. The paper is great, but will especially reward a few passes of reading. The examples they give take time, but are great to work through slowly to gain a deeper understanding. And/or, you can read the Zig code of PAR in TB! :) Here's a great place to start, one of our favorite pieces of code in TigerBeetle: [https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b...](https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b177a1a7daf1b9ff9b3db4ee048a5/src/vsr/journal.zig#L1150-L1216) | | | |  |  |  | | --- | --- | --- | |  |  | [yencabulator](user?id=yencabulator) [on July 12, 2023](item?id=36699917)  | [root](#36684025) | [parent](#36684200) [[–]](javascript:void(0))   > The crux of the problem: How do you solve misdirected read/write I/O? Where the firmware writes/reads to/from the wrong disk sector (but with a valid checksum)? Can't you make the expected location of the data part of the checksum? Concretely, - switch from checksums to hashes - use something like Blake3 as keyed hash with the WAL offset as key. Now, you can't accidentally read WAL block #5 instead of #7, as it's recorded hash won't match H(data, key=7). Similar more old school technique: storing the expected role & id of a block inside the block can make storage more robust. | | | |  |  |  | | --- | --- | --- | |  |  | [jorangreef](user?id=jorangreef) [on July 18, 2023](item?id=36779564)  | [root](#36684025) | [parent](#36699917) [[–]](javascript:void(0))   > Can't you make the expected location of the data part of the checksum? Yes, and in fact we do this already in TigerBeetle (specifically towards solving misdirected I/O, along with hash chaining). Coincidentally, we used to use Blake3 but have since moved to AEGIS for hardware acceleration. However, and this begins to hint at the problem, but redundancy alone is not sufficient. For misdirected I/O, we are already encoding more into the checksum... And, PAR goes beyond this. For example, how do you disentangle corruption in the middle of the committed write ahead log, from a torn write at the end of the WAL due to power loss? For this, to solve this correctly (to decide whether to repair a committed operation or truncate an uncommitted operation respectively, for correctness and high availability), you really do need two WALs... and integration with (or awareness of) the invariants of the global consensus protocol—as the paper motivates. This is a foundational design change. | | |
| |  | | --- | |  |    [Guidelines](newsguidelines.html) | [FAQ](newsfaq.html) | [Lists](lists) | [API](https://github.com/HackerNews/API) | [Security](security.html) | [Legal](https://www.ycombinator.com/legal/) | [Apply to YC](https://www.ycombinator.com/apply/) | [Contact](mailto:hn@ycombinator.com)   Search: |

|  |  |  |
| --- | --- | --- |
|  | **[Hacker News](news)**[new](newest) | [past](front) | [comments](newcomments) | <ask> | <show> | <jobs> | <submit> | [login](login?goto=item%3Fid%3D36683353) |

![](y18.svg)

|  |  |  |
| --- | --- | --- |
|  |  | [jorangreef](user?id=jorangreef) [on July 11, 2023](item?id=36683353)  | [parent](item?id=36682155) | [context](item?id=36680535#36683353) | [favorite](fave?id=36683353&auth=f76f24ece782ff839725b39473d532609563fbdb) | on: [We put a distributed database in the browser and m...](item?id=36680535 "We put a distributed database in the browser and made a game of it")   Thanks for the question! Joran from TigerBeetle here. The research in question is the 2018 paper from UW-Madison, “Protocol-Aware Recovery for Consensus-Based Storage” (PAR) [0] by Ram Alagappan, Aishwarya Ganesan, as well as Remzi and Andrea Arpaci-Dusseau (who you may recognize as authors of OSTEP). PAR won best paper at FAST '18 for showing that a single disk sector fault, in the write-ahead log (WAL) of a single replica, could propagate through the distributed RAFT or MultiPaxos consensus protocol, to cause global cluster data loss. This was counter-intuitive at the time, because PAR showed that the redundancy of these consensus and replication protocols did not in fact always imply fault-tolerance, as had previously been assumed. The reason is, and we cover this in depth in our recent QCon London talk [1], but it was assumed that checksums alone would be sufficient to detect and recover from storage faults. However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot. What you tend to find is that the WALs for many of these protocols will truncate the WAL at the first sign of a checksum mismatch, conflating the mismatch with power loss when it might be bitort, and thereby truncating committed transactions, and undermining quorum votes in the Raft or MultiPaxos implementations. RAID solutions don't always help here, either. See "Parity Lost and Parity Regained" [2] for more details. ZRAID is better here, and ZFS is a huge inspiration, but with local redundancy under ZFS you're still not leveraging the global redundancy of the consensus protocol as well as you could be. To summarize PAR: There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly. Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum. However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss. On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory. But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware. Thanks for the question and happy to answer more! [0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan) [1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY> [2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained) [3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf> |

The research in question is the 2018 paper from UW-Madison, “Protocol-Aware Recovery for Consensus-Based Storage” (PAR) [0] by Ram Alagappan, Aishwarya Ganesan, as well as Remzi and Andrea Arpaci-Dusseau (who you may recognize as authors of OSTEP).

PAR won best paper at FAST '18 for showing that a single disk sector fault, in the write-ahead log (WAL) of a single replica, could propagate through the distributed RAFT or MultiPaxos consensus protocol, to cause global cluster data loss.

This was counter-intuitive at the time, because PAR showed that the redundancy of these consensus and replication protocols did not in fact always imply fault-tolerance, as had previously been assumed.

The reason is, and we cover this in depth in our recent QCon London talk [1], but it was assumed that checksums alone would be sufficient to detect and recover from storage faults.

However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot.

What you tend to find is that the WALs for many of these protocols will truncate the WAL at the first sign of a checksum mismatch, conflating the mismatch with power loss when it might be bitort, and thereby truncating committed transactions, and undermining quorum votes in the Raft or MultiPaxos implementations.

RAID solutions don't always help here, either. See "Parity Lost and Parity Regained" [2] for more details. ZRAID is better here, and ZFS is a huge inspiration, but with local redundancy under ZFS you're still not leveraging the global redundancy of the consensus protocol as well as you could be.

To summarize PAR:

There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly.

Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum.

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

PAR won best paper at FAST '18 for showing that a single disk sector fault, in the write-ahead log (WAL) of a single replica, could propagate through the distributed RAFT or MultiPaxos consensus protocol, to cause global cluster data loss.

This was counter-intuitive at the time, because PAR showed that the redundancy of these consensus and replication protocols did not in fact always imply fault-tolerance, as had previously been assumed.

The reason is, and we cover this in depth in our recent QCon London talk [1], but it was assumed that checksums alone would be sufficient to detect and recover from storage faults.

However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot.

What you tend to find is that the WALs for many of these protocols will truncate the WAL at the first sign of a checksum mismatch, conflating the mismatch with power loss when it might be bitort, and thereby truncating committed transactions, and undermining quorum votes in the Raft or MultiPaxos implementations.

RAID solutions don't always help here, either. See "Parity Lost and Parity Regained" [2] for more details. ZRAID is better here, and ZFS is a huge inspiration, but with local redundancy under ZFS you're still not leveraging the global redundancy of the consensus protocol as well as you could be.

To summarize PAR:

There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly.

Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum.

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

This was counter-intuitive at the time, because PAR showed that the redundancy of these consensus and replication protocols did not in fact always imply fault-tolerance, as had previously been assumed.

The reason is, and we cover this in depth in our recent QCon London talk [1], but it was assumed that checksums alone would be sufficient to detect and recover from storage faults.

However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot.

What you tend to find is that the WALs for many of these protocols will truncate the WAL at the first sign of a checksum mismatch, conflating the mismatch with power loss when it might be bitort, and thereby truncating committed transactions, and undermining quorum votes in the Raft or MultiPaxos implementations.

RAID solutions don't always help here, either. See "Parity Lost and Parity Regained" [2] for more details. ZRAID is better here, and ZFS is a huge inspiration, but with local redundancy under ZFS you're still not leveraging the global redundancy of the consensus protocol as well as you could be.

To summarize PAR:

There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly.

Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum.

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

The reason is, and we cover this in depth in our recent QCon London talk [1], but it was assumed that checksums alone would be sufficient to detect and recover from storage faults.

However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot.

What you tend to find is that the WALs for many of these protocols will truncate the WAL at the first sign of a checksum mismatch, conflating the mismatch with power loss when it might be bitort, and thereby truncating committed transactions, and undermining quorum votes in the Raft or MultiPaxos implementations.

RAID solutions don't always help here, either. See "Parity Lost and Parity Regained" [2] for more details. ZRAID is better here, and ZFS is a huge inspiration, but with local redundancy under ZFS you're still not leveraging the global redundancy of the consensus protocol as well as you could be.

To summarize PAR:

There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly.

Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum.

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot.

What you tend to find is that the WALs for many of these protocols will truncate the WAL at the first sign of a checksum mismatch, conflating the mismatch with power loss when it might be bitort, and thereby truncating committed transactions, and undermining quorum votes in the Raft or MultiPaxos implementations.

RAID solutions don't always help here, either. See "Parity Lost and Parity Regained" [2] for more details. ZRAID is better here, and ZFS is a huge inspiration, but with local redundancy under ZFS you're still not leveraging the global redundancy of the consensus protocol as well as you could be.

To summarize PAR:

There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly.

Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum.

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

What you tend to find is that the WALs for many of these protocols will truncate the WAL at the first sign of a checksum mismatch, conflating the mismatch with power loss when it might be bitort, and thereby truncating committed transactions, and undermining quorum votes in the Raft or MultiPaxos implementations.

RAID solutions don't always help here, either. See "Parity Lost and Parity Regained" [2] for more details. ZRAID is better here, and ZFS is a huge inspiration, but with local redundancy under ZFS you're still not leveraging the global redundancy of the consensus protocol as well as you could be.

To summarize PAR:

There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly.

Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum.

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

RAID solutions don't always help here, either. See "Parity Lost and Parity Regained" [2] for more details. ZRAID is better here, and ZFS is a huge inspiration, but with local redundancy under ZFS you're still not leveraging the global redundancy of the consensus protocol as well as you could be.

To summarize PAR:

There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly.

Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum.

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

To summarize PAR:

There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly.

Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum.

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

There are fundamental design changes to both the global consensus protocol and the local storage engine that would need to be made, if the storage fault model of PAR (and TigerBeetle) is to be solved correctly.

Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum.

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

Furthermore, few simulators even test for these kinds of storage faults. For example, misdirected I/O, where the disk writes or reads to or from the wrong location of disk, which may yet have a valid checksum.

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

However, this is important, because disks fail in the real world. A single disk has on the order of a 0.5-1% chance of corruption in a 2 year period [3]. For example, a 5 node cluster has a 2.5-5% chance of a single disk sector fault, which again in terms of PAR can lead to global cluster data loss.

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

On the other hand, memory (or even CPU) faults, assuming ECC are not in the same order of magnitude probability, and therefore TigerBeetle's memory fault model is to require ECC memory.

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

But, again, to be crystal clear, checksums alone are not sufficient to solve the consensus corruption issue. The fix requires protocol changes at the design level, for the consensus protocol to be made storage fault-aware.

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

Thanks for the question and happy to answer more!

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

[0] “Protocol-Aware Recovery for Consensus-Based Storage” [https://www.usenix.org/conference/fast18/presentation/alagap...](https://www.usenix.org/conference/fast18/presentation/alagappan)

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

[1] “A New Era for Database Design” (we also dive into the research surrounding Fsyncgate, looking into the latent correctness issues that remain) <https://www.youtube.com/watch?v=_jfOk4L7CiY>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

[2] “Parity Lost and Parity Regained” [https://www.usenix.org/conference/fast-08/parity-lost-and-pa...](https://www.usenix.org/conference/fast-08/parity-lost-and-parity-regained)

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

[3] “An Analysis of Data Corruption in the Storage Stack” <https://www.cs.toronto.edu/~bianca/papers/fast08.pdf>

|  |  |  |  |
| --- | --- | --- | --- |
| |  |  |  | | --- | --- | --- | |  |  | [petrosagg](user?id=petrosagg) [on July 11, 2023](item?id=36684025)  [[–]](javascript:void(0))   Thank you for the detailed response! > However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot. The PAR paper states that "although Crash preserves safety, it suffers from severe unavailability". I assume that when TigerBeetle loads state from RAM into a CPU cache/register it operates under the NoDetection consistency model or the Crash consistency model if ECC RAM automatically resets the CPU on read errors. At the same time it doesn't suffer from severe unavailability so what gives? The answer is probably that ECC RAM is just reliable enough that the NoDetection/Crash models are fine in practice. I can believe that off-the-shelf checksum and redundancy options offered by filesystems like ext4 and ZFS or systems like RAID don't hit the required error probabilities but why does the argument stop there? Couldn't a distributed database generate error correcting data on every write in the application layer so that the probability becomes low enough such that NoDetection/Crash become a non-issue for storage, just like RAM? Is there some other fundamental difference between reading and write data from RAM versus a disk? | |
| |  |  |  | | --- | --- | --- | |  |  | [jorangreef](user?id=jorangreef) [on July 11, 2023](item?id=36684200)  | [parent](#36684025) [[–]](javascript:void(0))   Huge pleasure, thanks again for the question! The crux of the problem: How do you solve misdirected read/write I/O? Where the firmware writes/reads to/from the wrong disk sector (but with a valid checksum)? PAR shows how both global consensus protocol and local storage engine need to be modified for this, with foundational design changes at the protocol-level, if a distributed system is to not only preserve correctness, but also optimize for high availability. Bear in mind that PAR is not only actually correct, but it's also more efficient than simply dialing up local redundancy, because it lets you recover from the global redundancy that you have via replication in the consensus protocol. The paper is great, but will especially reward a few passes of reading. The examples they give take time, but are great to work through slowly to gain a deeper understanding. And/or, you can read the Zig code of PAR in TB! :) Here's a great place to start, one of our favorite pieces of code in TigerBeetle: [https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b...](https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b177a1a7daf1b9ff9b3db4ee048a5/src/vsr/journal.zig#L1150-L1216) | |
| |  |  |  | | --- | --- | --- | |  |  | [yencabulator](user?id=yencabulator) [on July 12, 2023](item?id=36699917)  | [root](#36684025) | [parent](#36684200) [[–]](javascript:void(0))   > The crux of the problem: How do you solve misdirected read/write I/O? Where the firmware writes/reads to/from the wrong disk sector (but with a valid checksum)? Can't you make the expected location of the data part of the checksum? Concretely, - switch from checksums to hashes - use something like Blake3 as keyed hash with the WAL offset as key. Now, you can't accidentally read WAL block #5 instead of #7, as it's recorded hash won't match H(data, key=7). Similar more old school technique: storing the expected role & id of a block inside the block can make storage more robust. | |
| |  |  |  | | --- | --- | --- | |  |  | [jorangreef](user?id=jorangreef) [on July 18, 2023](item?id=36779564)  | [root](#36684025) | [parent](#36699917) [[–]](javascript:void(0))   > Can't you make the expected location of the data part of the checksum? Yes, and in fact we do this already in TigerBeetle (specifically towards solving misdirected I/O, along with hash chaining). Coincidentally, we used to use Blake3 but have since moved to AEGIS for hardware acceleration. However, and this begins to hint at the problem, but redundancy alone is not sufficient. For misdirected I/O, we are already encoding more into the checksum... And, PAR goes beyond this. For example, how do you disentangle corruption in the middle of the committed write ahead log, from a torn write at the end of the WAL due to power loss? For this, to solve this correctly (to decide whether to repair a committed operation or truncate an uncommitted operation respectively, for correctness and high availability), you really do need two WALs... and integration with (or awareness of) the invariants of the global consensus protocol—as the paper motivates. This is a foundational design change. | |

|  |  |  |
| --- | --- | --- |
|  |  | [petrosagg](user?id=petrosagg) [on July 11, 2023](item?id=36684025)  [[–]](javascript:void(0))   Thank you for the detailed response! > However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot. The PAR paper states that "although Crash preserves safety, it suffers from severe unavailability". I assume that when TigerBeetle loads state from RAM into a CPU cache/register it operates under the NoDetection consistency model or the Crash consistency model if ECC RAM automatically resets the CPU on read errors. At the same time it doesn't suffer from severe unavailability so what gives? The answer is probably that ECC RAM is just reliable enough that the NoDetection/Crash models are fine in practice. I can believe that off-the-shelf checksum and redundancy options offered by filesystems like ext4 and ZFS or systems like RAID don't hit the required error probabilities but why does the argument stop there? Couldn't a distributed database generate error correcting data on every write in the application layer so that the probability becomes low enough such that NoDetection/Crash become a non-issue for storage, just like RAM? Is there some other fundamental difference between reading and write data from RAM versus a disk? |

![](s.gif)

> However, while checksums can be used under the “Crash Consistency Model” to solve consistency through power loss, PAR showed that checksums are not sufficient to be able to distinguish between a torn write at the end of the (uncommitted) WAL caused by power loss, and a torn write in the middle of the (committed) WAL caused by bitrot.

The PAR paper states that "although Crash preserves safety, it suffers from severe unavailability". I assume that when TigerBeetle loads state from RAM into a CPU cache/register it operates under the NoDetection consistency model or the Crash consistency model if ECC RAM automatically resets the CPU on read errors. At the same time it doesn't suffer from severe unavailability so what gives?

The answer is probably that ECC RAM is just reliable enough that the NoDetection/Crash models are fine in practice.

I can believe that off-the-shelf checksum and redundancy options offered by filesystems like ext4 and ZFS or systems like RAID don't hit the required error probabilities but why does the argument stop there? Couldn't a distributed database generate error correcting data on every write in the application layer so that the probability becomes low enough such that NoDetection/Crash become a non-issue for storage, just like RAM? Is there some other fundamental difference between reading and write data from RAM versus a disk?

The PAR paper states that "although Crash preserves safety, it suffers from severe unavailability". I assume that when TigerBeetle loads state from RAM into a CPU cache/register it operates under the NoDetection consistency model or the Crash consistency model if ECC RAM automatically resets the CPU on read errors. At the same time it doesn't suffer from severe unavailability so what gives?

The answer is probably that ECC RAM is just reliable enough that the NoDetection/Crash models are fine in practice.

I can believe that off-the-shelf checksum and redundancy options offered by filesystems like ext4 and ZFS or systems like RAID don't hit the required error probabilities but why does the argument stop there? Couldn't a distributed database generate error correcting data on every write in the application layer so that the probability becomes low enough such that NoDetection/Crash become a non-issue for storage, just like RAM? Is there some other fundamental difference between reading and write data from RAM versus a disk?

The answer is probably that ECC RAM is just reliable enough that the NoDetection/Crash models are fine in practice.

I can believe that off-the-shelf checksum and redundancy options offered by filesystems like ext4 and ZFS or systems like RAID don't hit the required error probabilities but why does the argument stop there? Couldn't a distributed database generate error correcting data on every write in the application layer so that the probability becomes low enough such that NoDetection/Crash become a non-issue for storage, just like RAM? Is there some other fundamental difference between reading and write data from RAM versus a disk?

I can believe that off-the-shelf checksum and redundancy options offered by filesystems like ext4 and ZFS or systems like RAID don't hit the required error probabilities but why does the argument stop there? Couldn't a distributed database generate error correcting data on every write in the application layer so that the probability becomes low enough such that NoDetection/Crash become a non-issue for storage, just like RAM? Is there some other fundamental difference between reading and write data from RAM versus a disk?

|  |  |  |
| --- | --- | --- |
|  |  | [jorangreef](user?id=jorangreef) [on July 11, 2023](item?id=36684200)  | [parent](#36684025) [[–]](javascript:void(0))   Huge pleasure, thanks again for the question! The crux of the problem: How do you solve misdirected read/write I/O? Where the firmware writes/reads to/from the wrong disk sector (but with a valid checksum)? PAR shows how both global consensus protocol and local storage engine need to be modified for this, with foundational design changes at the protocol-level, if a distributed system is to not only preserve correctness, but also optimize for high availability. Bear in mind that PAR is not only actually correct, but it's also more efficient than simply dialing up local redundancy, because it lets you recover from the global redundancy that you have via replication in the consensus protocol. The paper is great, but will especially reward a few passes of reading. The examples they give take time, but are great to work through slowly to gain a deeper understanding. And/or, you can read the Zig code of PAR in TB! :) Here's a great place to start, one of our favorite pieces of code in TigerBeetle: [https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b...](https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b177a1a7daf1b9ff9b3db4ee048a5/src/vsr/journal.zig#L1150-L1216) |

![](s.gif)

The crux of the problem: How do you solve misdirected read/write I/O? Where the firmware writes/reads to/from the wrong disk sector (but with a valid checksum)?

PAR shows how both global consensus protocol and local storage engine need to be modified for this, with foundational design changes at the protocol-level, if a distributed system is to not only preserve correctness, but also optimize for high availability.

Bear in mind that PAR is not only actually correct, but it's also more efficient than simply dialing up local redundancy, because it lets you recover from the global redundancy that you have via replication in the consensus protocol.

The paper is great, but will especially reward a few passes of reading. The examples they give take time, but are great to work through slowly to gain a deeper understanding.

And/or, you can read the Zig code of PAR in TB! :)

Here's a great place to start, one of our favorite pieces of code in TigerBeetle: [https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b...](https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b177a1a7daf1b9ff9b3db4ee048a5/src/vsr/journal.zig#L1150-L1216)

PAR shows how both global consensus protocol and local storage engine need to be modified for this, with foundational design changes at the protocol-level, if a distributed system is to not only preserve correctness, but also optimize for high availability.

Bear in mind that PAR is not only actually correct, but it's also more efficient than simply dialing up local redundancy, because it lets you recover from the global redundancy that you have via replication in the consensus protocol.

The paper is great, but will especially reward a few passes of reading. The examples they give take time, but are great to work through slowly to gain a deeper understanding.

And/or, you can read the Zig code of PAR in TB! :)

Here's a great place to start, one of our favorite pieces of code in TigerBeetle: [https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b...](https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b177a1a7daf1b9ff9b3db4ee048a5/src/vsr/journal.zig#L1150-L1216)

Bear in mind that PAR is not only actually correct, but it's also more efficient than simply dialing up local redundancy, because it lets you recover from the global redundancy that you have via replication in the consensus protocol.

The paper is great, but will especially reward a few passes of reading. The examples they give take time, but are great to work through slowly to gain a deeper understanding.

And/or, you can read the Zig code of PAR in TB! :)

Here's a great place to start, one of our favorite pieces of code in TigerBeetle: [https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b...](https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b177a1a7daf1b9ff9b3db4ee048a5/src/vsr/journal.zig#L1150-L1216)

The paper is great, but will especially reward a few passes of reading. The examples they give take time, but are great to work through slowly to gain a deeper understanding.

And/or, you can read the Zig code of PAR in TB! :)

Here's a great place to start, one of our favorite pieces of code in TigerBeetle: [https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b...](https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b177a1a7daf1b9ff9b3db4ee048a5/src/vsr/journal.zig#L1150-L1216)

And/or, you can read the Zig code of PAR in TB! :)

Here's a great place to start, one of our favorite pieces of code in TigerBeetle: [https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b...](https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b177a1a7daf1b9ff9b3db4ee048a5/src/vsr/journal.zig#L1150-L1216)

Here's a great place to start, one of our favorite pieces of code in TigerBeetle: [https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b...](https://github.com/tigerbeetle/tigerbeetle/blob/4aca8a22627b177a1a7daf1b9ff9b3db4ee048a5/src/vsr/journal.zig#L1150-L1216)

|  |  |  |
| --- | --- | --- |
|  |  | [yencabulator](user?id=yencabulator) [on July 12, 2023](item?id=36699917)  | [root](#36684025) | [parent](#36684200) [[–]](javascript:void(0))   > The crux of the problem: How do you solve misdirected read/write I/O? Where the firmware writes/reads to/from the wrong disk sector (but with a valid checksum)? Can't you make the expected location of the data part of the checksum? Concretely, - switch from checksums to hashes - use something like Blake3 as keyed hash with the WAL offset as key. Now, you can't accidentally read WAL block #5 instead of #7, as it's recorded hash won't match H(data, key=7). Similar more old school technique: storing the expected role & id of a block inside the block can make storage more robust. |

![](s.gif)

Can't you make the expected location of the data part of the checksum?

Concretely,

- switch from checksums to hashes

- use something like Blake3 as keyed hash with the WAL offset as key.

Now, you can't accidentally read WAL block #5 instead of #7, as it's recorded hash won't match H(data, key=7).

Similar more old school technique: storing the expected role & id of a block inside the block can make storage more robust.

Concretely,

- switch from checksums to hashes

- use something like Blake3 as keyed hash with the WAL offset as key.

Now, you can't accidentally read WAL block #5 instead of #7, as it's recorded hash won't match H(data, key=7).

Similar more old school technique: storing the expected role & id of a block inside the block can make storage more robust.

- switch from checksums to hashes

- use something like Blake3 as keyed hash with the WAL offset as key.

Now, you can't accidentally read WAL block #5 instead of #7, as it's recorded hash won't match H(data, key=7).

Similar more old school technique: storing the expected role & id of a block inside the block can make storage more robust.

- use something like Blake3 as keyed hash with the WAL offset as key.

Now, you can't accidentally read WAL block #5 instead of #7, as it's recorded hash won't match H(data, key=7).

Similar more old school technique: storing the expected role & id of a block inside the block can make storage more robust.

Now, you can't accidentally read WAL block #5 instead of #7, as it's recorded hash won't match H(data, key=7).

Similar more old school technique: storing the expected role & id of a block inside the block can make storage more robust.

Similar more old school technique: storing the expected role & id of a block inside the block can make storage more robust.

|  |  |  |
| --- | --- | --- |
|  |  | [jorangreef](user?id=jorangreef) [on July 18, 2023](item?id=36779564)  | [root](#36684025) | [parent](#36699917) [[–]](javascript:void(0))   > Can't you make the expected location of the data part of the checksum? Yes, and in fact we do this already in TigerBeetle (specifically towards solving misdirected I/O, along with hash chaining). Coincidentally, we used to use Blake3 but have since moved to AEGIS for hardware acceleration. However, and this begins to hint at the problem, but redundancy alone is not sufficient. For misdirected I/O, we are already encoding more into the checksum... And, PAR goes beyond this. For example, how do you disentangle corruption in the middle of the committed write ahead log, from a torn write at the end of the WAL due to power loss? For this, to solve this correctly (to decide whether to repair a committed operation or truncate an uncommitted operation respectively, for correctness and high availability), you really do need two WALs... and integration with (or awareness of) the invariants of the global consensus protocol—as the paper motivates. This is a foundational design change. |

![](s.gif)

Yes, and in fact we do this already in TigerBeetle (specifically towards solving misdirected I/O, along with hash chaining). Coincidentally, we used to use Blake3 but have since moved to AEGIS for hardware acceleration.

However, and this begins to hint at the problem, but redundancy alone is not sufficient. For misdirected I/O, we are already encoding more into the checksum...

And, PAR goes beyond this. For example, how do you disentangle corruption in the middle of the committed write ahead log, from a torn write at the end of the WAL due to power loss? For this, to solve this correctly (to decide whether to repair a committed operation or truncate an uncommitted operation respectively, for correctness and high availability), you really do need two WALs... and integration with (or awareness of) the invariants of the global consensus protocol—as the paper motivates.

This is a foundational design change.

However, and this begins to hint at the problem, but redundancy alone is not sufficient. For misdirected I/O, we are already encoding more into the checksum...

And, PAR goes beyond this. For example, how do you disentangle corruption in the middle of the committed write ahead log, from a torn write at the end of the WAL due to power loss? For this, to solve this correctly (to decide whether to repair a committed operation or truncate an uncommitted operation respectively, for correctness and high availability), you really do need two WALs... and integration with (or awareness of) the invariants of the global consensus protocol—as the paper motivates.

This is a foundational design change.

And, PAR goes beyond this. For example, how do you disentangle corruption in the middle of the committed write ahead log, from a torn write at the end of the WAL due to power loss? For this, to solve this correctly (to decide whether to repair a committed operation or truncate an uncommitted operation respectively, for correctness and high availability), you really do need two WALs... and integration with (or awareness of) the invariants of the global consensus protocol—as the paper motivates.

This is a foundational design change.

This is a foundational design change.

![](s.gif)

|  |
| --- |
|  |
