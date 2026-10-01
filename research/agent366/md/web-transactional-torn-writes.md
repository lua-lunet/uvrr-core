<!-- source: https://transactional.blog/blog/2025-torn-writes -->

[↤ Decomposing Transactional Systems](/blog/2025-decomposing-transactional-systems.html) [↑ Blog ↑](/) [Modern Hardware for Future Databases ↦](/blog/2024-modern-database-hardware.html)

---

# Torn Write Detection and Protection

* [Detection Only](#_detection_only)
* [Sector-Sized Pages](#_sector_sized_pages)
* [Log Full Pages](#_log_full_pages)
* [Log Page on First Write](#_log_page_on_first_write)
* [Double-Write Buffer](#_double_write_buffer)
* [Copy on Write](#_copy_on_write)
* [Copy on First Write](#_copy_on_first_write)
* [Atomic (Multi-)Block Writes](#_atomic_multi_block_writes)
* [Comparison](#_comparison)

Disks have a unit of data at which they promise atomicity: the sector. Historically, this was 512 bytes. Disks also accept write requests for multiple contiguous sectors at a time. In the event of a power failure, this can lead to only a subset of the sectors being written: a *torn write*. Databases operate on 4KB or larger pages, and thus are vulnerable to torn writes. A database page affected by a torn write is called a *torn page*. Various databases have different opinions and strategies on detecting torn pages during recovery, and how to restore the page to a correct, full page image.

For a more explanation of torn writes or torn pages, see content elsewhere such as [blog posts](https://www.joshodgers.com/tag/torn-write/), [product documentation](https://learn.microsoft.com/en-us/troubleshoot/sql/database-engine/database-file-operations/logging-data-storage-algorithms#torn-page-detection), [videos](https://www.youtube.com/watch?v=OtxCzIHOMk4), or chat with your favorite LLM offering.

## Detection Only

Identifying that a page is invalid because it had been subject to a torn write can be done in cheaper ways than treating it as a corruption caught by checksums. One can include in written sectors a counter, which is required to be incremented by any amount on each write. If a page is read, and the counters in each sector aren’t all the same value, then a torn page has been identified, and the page should be treated as corrupt or invalid.

SQL Server has an [8KB page size](https://learn.microsoft.com/en-us/sql/relational-databases/pages-and-extents-architecture-guide?view=sql-server-ver16#pages). [Torn page detection](https://learn.microsoft.com/en-us/troubleshoot/sql/database-engine/database-file-operations/logging-data-storage-algorithms#torn-page-detection) is a feature that can be enabled or disabled in SQL Server, which takes the first two bits of each 512 byte sector, concatenates them into one 32bit value, and stores it in the page header. The first two bits are then replaced with a 2-bit counter that is incremented on each page write. Torn pages have counter values that don’t match across sectors. Pages that have failed the torn page detection check are logged in a [suspect pages table](https://learn.microsoft.com/en-us/sql/relational-databases/system-tables/suspect-pages-transact-sql?view=sql-server-ver16), and [page restore](https://learn.microsoft.com/en-us/sql/relational-databases/backup-restore/restore-pages-sql-server?view=sql-server-ver16) seems to be the most recommended solution for repairing the pages. Torn page detection has been largely superseded by page-level checksums (`PAGE_VERIFY=CHECKSUM`), which is also a valid way to view torn pages: it’s just another type of potential data corruption.

Oracle has an 8KB page size as well. The structure of data pages was only published as part of the [Block Browser and Editor documentation](https://www.orafaq.com/papers/dissassembling_the_data_block.pdf). Oracle RDBMS can detect torn writes by comparing the System Change Number in the header of a page with a copy of it stored in the tail of the page.[1] (See pages 10-12 of the linked PDF.) After a torn page is detected, an administrator must perform *media recovery* using [DBMS\_REPAIR](https://docs.oracle.com/en/database/oracle/oracle-database/19/admin/repairing-corrupted-data.html) or [RMAN](https://docs.oracle.com/en/database/oracle/oracle-database/19/bradv/rman-block-media-recovery.html) to fix the corrupt blocks. [`DB_BLOCK_CHECKSUM=TYPICAL`](https://docs.oracle.com/en/database/oracle/oracle-database/23/refrn/DB_BLOCK_CHECKSUM.html) will cause torn pages to be detected as checksum failures, but the head and tail SCNs can still catch torn pages even with the checksumming disabled. [1]:Note that this assumes that sectors will be written and persisted by the firmware sequentially, in order, and that’s not actually a promised property. Howard Chu, of LMDB fame, has made a couple of arguments [[1]](https://www.openldap.org/lists/openldap-devel/201410/msg00004.html) [[2]](https://lists.openldap.org/hyperkitty/list/openldap-devel@openldap.org/thread/YUUKXVYXA347IWW3UKRS6NJHBU4FEE6M/) over time that relying on such behavior is safe in practice.

There exists an argument that enterprise SSDs which include supercapacitors for power-loss safety, the same ones that are well-discussed for allowing `fsync()` to complete asynchronously, are also immune from torn writes. The ATA/SATA/SCSI/NVME protocols themselves allow write commands to span multiple logical blocks, so a multi-block write command is either received or not. Even in the event of a power loss, the drive is expected to have enough power left to flush its volatile cache, and the argument is that cache write-back includes finishing any in-flight requests. The lack of the Force Unit Access flag on those writes means there’s no explicit contract that they must have been fully persisted. This argument assumes perfectly working firmware, which is not a generally recommended assumption.[2] However, in such real-world situations, torn writes leading to torn pages have been sufficiently uncommon in practice that widely deployed databases like Oracle and SQL Server haven’t had to add automatic mitigation for torn pages. Particularly when the database is sold along with validated hardware to run the database on, there’s a reasonable argument that the cost of detecting and recovering from torn writes isn’t justified given the validated low risk. [2]: I’m in the process of working through SSD failure analysis papers, but [Understanding the Robustness of SSDs under Power Fault](https://6826.csail.mit.edu/2017/papers/fast13-final80.pdf) includes testing of a 2009 SSD with capacitors that exhibited torn writes under power failure. I’ll update this once I’ve concluded my paper series reading.

## Sector-Sized Pages

Storage device specifications require disks to make writes of an individual sector atomic. If the sector size of the disk and the database page size match, then torn writes become impossible! Historically, disks have used 512 byte sectors. Database page sizes are a balance between branching factor, risk of needing overflow pages to maintain a minimum branching factor, and write amplification for a small update. 512 bytes generally isn’t enough space to be able to ensure a sufficient branching factor within a single page, so 512 bytes of atomicity isn’t enough to be useful.

These days, some NVMe SSDs do natively support a 4096 byte physical sector size[3], even among those made for the consumer market. SSDs with Phison controllers are highly likely to support 4096 byte logical block addressing, Western Digital and SanDisk are likely to have support, and some SK Hynix and Kingston drives do as well. However, cloud support for this is questionable, as AWS’s i4i line was confirmed to not support 4096 byte logical blocks.[4] One may check NVMe drives' support by using `nvme-cli id-ns` and change it using `nvme-cli format` [as described elsewhere](https://www.bjonnh.net/article/20210721_nvme4k/). [3]: And I’d love to hear reports on if other instance types or other cloud vendors' instances do or don’t support 4096 byte sectors. [4]: Note that the [NVMe NVM Command Set Specification](https://nvmexpress.org/wp-content/uploads/NVMe-NVM-Command-Set-Specification-1.0a-2021.07.26-Ratified.pdf)'s 2.1.4 Atomic Operation specifies that Atomic Write Unit Power Fail is defined in terms of the number of logical blocks, so configuring a 4KB logical block side does require 4KB writes to be atomic as per the NVMe spec.

The story in the cloud is a bit better when using the persistent disk offerings. AWS advertises [torn write prevention](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/storage-twp.html) which permits configuring EBS volumes to support database page-sized atomic writes. Azure has [their own version](https://learn.microsoft.com/en-us/azure/virtual-machines/disks-torn-write-prevention) on managed disks. GCP had [experimented with similar support](https://price2meet.com/gcp/docs/compute_docs_tutorials_16kb-mysql-best-practices.pdf), which they’ve since retracted from their own docs. Alibaba appears to include it as a [write optimization feature](https://www.alibabacloud.com/help/en/rds/apsaradb-rds-for-mysql/optimized-write) as part of their AsparaDB RDS for MySQL offering. As IOPS and throughput are limited on cloud persistent disks, it’s a good feature to take advantage of if it’s available.

## Log Full Pages

A simple way to be able to fix torn pages is to write the entire updated page into the write-ahead log, for each page that is updated. If a failure causes a torn write, the recovery process will re-apply the pages from the WAL to the B-Tree, thus fixing the torn page. If a failure affects the WAL, then the transaction never committed, and thus can be instead rolled back during recovery.

The downsides to this simplicity are notable. This incurs a 2x write amplification, as every page is now being written twice. Space amplification is significant, as now the entire page is being written instead of just the tuple modification. But, simplicity is often a worthwhile benefit.

SQLite defaults to a 4KB page size[5] and follows this approach to torn page protection (which they call ["powersafe overwrite"](https://sqlite.org/psow.html)). In [WAL mode](https://sqlite.org/wal.html), updates to pages cause the full page to be placed into the WAL. During a checkpoint, performed at COMMIT or after 1000 pages are written, all pages are applied from the WAL back to the B-Tree. Checkpointing can thus only be done when there are no open transactions, and long-running write transactions can cause the WAL to grow significantly. [5]: Until 2016, [the default was 1KB pages](https://sqlite.org/pgszchng2016.html).

## Log Page on First Write

Logging page deltas is preferable over full page contents for the reduced write and space amplification. To permit any page to be reconstructed from the write-ahead log, a full copy of the page can be saved only once right before it is first modified. Then, if a torn write later causes a torn page, the older version of the page can be fetched from the write-ahead log, and all deltas applied to produce the correct page contents.

As checkpointing removes previous write-ahead log files, pages will need to be copied to the write-ahead log on their first modification following each checkpoint. But copying the page to the write-ahead log only once per checkpoint still lowers the expected costs drastically. The normal state will trend towards ~1x write amplification, and most of the WAL can remain descriptions of page changes rather than full page images which helps space amplification.

Postgres has an 8KB page size[6] and utilizes this torn write protection technique, which it calls [full page writes](https://wiki.postgresql.org/wiki/Full_page_writes). The page images are compressed before being written into the write-ahead log. During recovery, postgres unconditionally[7] applies the save full page images and re-applies all logged changes to the pages to avoid the random IO on the write-ahead log. The impact of `full_page_writes` has been studied in detail across blog posts such as [On the impact of full page writes](https://www.enterprisedb.com/blog/impact-full-page-writes) or talks like [Full page Writes in PostgreSQL](https://www.slideshare.net/slideshow/full-page-writes-in-postgresql-pgconfeu-2022/253854027). For a tale of an interaction with full page writes causing issues, see [did u ever read so hard u accidentally wrote?](https://blog.danslimmon.com/2025/03/14/did-u-ever-read-so-hard-u-accidentally-wrote/) [6]: The page size can be changed at compile time, with [4KB being the lowest probably safe value](https://www.postgresql.org/message-id/200611281746.32245.peter_e%40gmx.net). EBS charges for IOPS in units of rounding up to the nearest 16KB, so I’m surprised we don’t see more 16KB page size postgres being deployed in the cloud, but Andres Freund noted he’s seen a measurable latency difference when he’s looked at page sizing before. [7]: This has tangential advantages of pre-populating the buffer pool during recovery, thus lessening the cold cache latency effect of a restarted postgres instance. Postgres replication is also WAL-based, and thus benefits similarly.

## Double-Write Buffer

Rather than relying on the write-ahead log, it’s possible to move the torn write protection responsibility entirely to the B-Tree. By first writing all B-Tree pages to a dedicated scratch space on disk, one can ensure that any torn page has a full and correct copy of the page in the scratch space to recover from instead.

Double-write buffering takes an opposite set of trade-offs as logging pages into the write-ahead log. There’s no extra work involved with the write-ahead log, and instead, all responsibility is moved to the B-Tree page writes themselves. The cost though is the database must write every B-Tree page twice, and fsync the double-write buffer before writing to the B-Tree. In the best case, the working set fits in the page cache, and B-Tree pages will only *need* to be persisted once per checkpoint. In the worst case, the workload is largely out of memory, and the double-write buffer causing double the writes could be very noticeable.

MySQL’s InnoDB has a 16KB page size[8] and is the most well-known user of the [double write buffer](https://dev.mysql.com/doc/refman/5.7/en/innodb-doublewrite-buffer.html) strategy, with the other users being the MySQL fork XtraDB and CedarDB. Using a double write buffer has the advantage of not blocking commits on writing full page images into the WAL. It maintains the notable downside of 2x the write latency and 2x write amplification. [8]: The InnoDB page size can be changed at database initialization time. Percona once benchmarked the effect of [using a 4KB InnoDB page size](https://www.percona.com/blog/small-innodb_page_size-performance-boost-ssd/).

I’ve had a few conversations with Sunny Bains, who worked on InnoDB including the double write buffer implementation, and so to forward some historical context and wisdom on the subject:

* When looking at [benchmarks implying the double-write buffer is a significant slowdown](https://dev.mysql.com/doc/refman/5.7/en/innodb-doublewrite-buffer.html), be aware that the double-write buffer was a source of mutex contention under high write throughput for single page flushes. This was addressed as part of MySQL 8.0.23 in 2021 by [WL#5655](https://dev.mysql.com/worklog/task/?id=5655) and see bug tracker discussion like [Bug #81376](https://bugs.mysql.com/bug.php?id=81376). [The New InnoDB Double Write Buffer in Action](http://dimitrik.free.fr/blog/posts/mysql-80-perf-new-dblwr.html) shows the impact of the improvement.
* The last time the double-write buffer code was being overhauled, Intel Optane was coming out and it was thought that one would be able to place the double-write buffer on a separate Optane drive, thus utilizing its infinite IOPS and fast fsync()s to minimize the overhead of the double-write buffer. Instead, Intel killed Optane, and running MySQL on EBS became popular, where IOPS on EBS are instead highly constrained.
* Some prominent MySQL users had production environments where MySQL was already replicated, and replacing a replica was a sufficiently fast "fix" for torn pages. Thus, `innodb_doublewrite=DETECT_ONLY` was added to only check pages for torn writes during recovery but not do the full double-write buffer process to permit automatically fixing them.

## Copy on Write

A torn write can only turn into a torn page if it occurs while overwriting a page. The torn write must affect a page being updated in-place in the B-Tree.[9] This provides a clear way around the issue of torn writes: never update a page in-place. When one wishes to modify a B-Tree page, allocate a new page in the file, place the updated copy of the page there, and then adjust the parent to point to the newly "updated" child. Adjusting the parent is also an update, and so the copying will recurse up to the root of the B-Tree. Copy-on-Write B-Trees first write all pages except the root, and then a successful write of a new B-Tree root marks them all as committed updates to the file[10]. This technique is also known as *shadow paging*. [9]: This is also why this entire post is about B-Trees. LSMs are structured entirely as append-only write-once files. The lack of in-place page updates in an LSM means they’re immune to torn pages, by design. [10]: Copy-on-Write B-Trees can thus perform atomic multi-page updates without the use of a write-ahead log, and thus very frequently skip implementing one.

For a better illustration of how Copy-on-Write B-Trees work, see [how the append-only btree works](https://www.bzero.se/ldapd/btree.html) or [Copy-on-Write, wandering trees, and data recovery](https://www.klennet.com/notes/2024-06-16-wandering-trees.aspx). Note that there are two [different kinds of copy-on-write for a b-tree: CoW-Random and CoW-Sequential](https://smalldatum.blogspot.com/2015/08/different-kinds-of-copy-on-write-for-b.html).

A major advantage of Copy-on-Write B-Trees is their simplicity. There are no torn pages to worry about, no separate write-ahead log, and any set of pages can be updated atomically in the tree. The downside is everything else. Copying from leaf to root means higher write amplification for single-page updates. Each update requires two `fsync()` latencies before it can be considered durable, rather than one with a write-ahead log. There’s no clear way to allow multiple updates across the tree concurrently, as any update will eventually contend on updating the root node, which includes pointers to all of its children.

[LMDB](http://www.lmdb.tech/doc/) is one of the most widely known Copy-on-Write B-Tree implementations. More details on LMDB internals can be found in [its whitepaper](https://www.openldap.org/pub/hyc/mdb-paper.pdf), [talks](https://www.youtube.com/watch?v=tEa5sAh-kVk), or [third-party blog posts](https://github.com/erigontech/erigon/wiki/LMDB-freelist-illustrated-guide).

## Copy on First Write

Once can extend the Copy on Write technique to mitigate the throughput and latency issues mentioned above: use copy-on-write only for the first update of a page within each checkpoint interval. This is analogous to Log Page on First Write, except the page is maintained in the B-Tree instead of being copied into the WAL. If a torn write affects a page that was updated in-place, all of the deltas in the write-ahead log can be applied to the copied-from, older versioned page to produce the correct page contents.

Copy on First Write blends together the advantages of Log on First Write and Double Write Buffer. In the usual case, it has only a 1x write amplification and no penalty to write latency. After a checkpoint, new writes do need to do extra work of copying data in the B-Tree, but those writes do not block the commit into the WAL. The notable disadvantage is that old pages must be maintained for the duration until the next checkpoint completes, thus causing there to be space amplification on the order of the working set between checkpoints.

[OrioleDB](https://www.orioledb.com/) is the only database I’m aware of that uses this strategy (and reading its code is how I learned about it). They have [well-illustrated documents](https://www.orioledb.com/docs/architecture/overview#copy-on-write-checkpoints) that better describe its behavior. Reading about [free space management](https://www.orioledb.com/docs/architecture/fsm) is also insightful, as copied-from pages can only be recycled into free space once a checkpoint is completed, which makes tracking free blocks more complicated.

## Atomic (Multi-)Block Writes

Linux has recently begun introducing support for [atomic writes for torn write protection](https://lwn.net/Articles/963742/). [pwritev2(2)![Linux](https://www.kernel.org/theme/images/logos/favicon.png)](https://man7.org/linux/man-pages/man2/pwritev2.2.html) has gained a `RWF_ATOMIC` flag, used to indicate that the submitted writes should be performed atomically. Linux 6.11 marked the introduction of `RWF_ATOMIC` and [statx(2)![Linux](https://www.kernel.org/theme/images/logos/favicon.png)](https://man7.org/linux/man-pages/man2/statx.2.html) being extended to allow checking for support. XFS and ext4 gained support for `RWF_ATOMIC` with 6.13. Work is slated to merge into 6.15 to allow [atomic writes of data larger than the filesystem block size](https://patchwork.kernel.org/project/xfs/cover/20250102140411.14617-1-john.g.garry@oracle.com/).

To utilize this feature, first invoke [statx(2)![Linux](https://www.kernel.org/theme/images/logos/favicon.png)](https://man7.org/linux/man-pages/man2/statx.2.html), and check the returned fields:

* `stx_atomic_write_unit_min`: The minimum size in bytes supported an atomic write. Guaranteed to be a power of 2.
* `stx_atomic_write_unit_max`: The maximum size in bytes supported an atomic write. Guaranteed to be a power of 2.
* `stx_atomic_write_segments_max`: The number of writes, within the min and max above, which can be submitted as part of `pwritev2()` that will be persisted together, atomically.

The values returned will include the result of querying the underlying storage hardware for what it supports.

Writes should then be performed via [pwritev2(2)![Linux](https://www.kernel.org/theme/images/logos/favicon.png)](https://man7.org/linux/man-pages/man2/pwritev2.2.html) with `RWF_ATOMIC` specified in the flags, on a file descriptor opened with `O_DIRECT`, and the writes must match an alignment the same as the size of the write. Invocations of `pwritev2()` must voluntarily stay within the limits of the min and the max atomic write unit. If the vector count exceeds max write segments, `pwritev2()` will return `-EINVAL`, but if the size of a write exceeds the max atomic write unit, it is currently filesystem dependant as to if the `pwritev2()` will return an error or silently complete with lower guarantees.[11] There is no promise made on a specified level of write amplification or number of write latencies involved. [11]: As of 6.13, ext4 will return an error, and XFS won’t. XFS [will be fixed](https://patchwork.kernel.org/project/xfs/patch/20250102140411.14617-6-john.g.garry@oracle.com/), and then checking `statx()` first won’t be mandatory. However, from side conversation with an ext4 dev, you aren’t *supposed* to rely on `pwritev2()` return codes, and you should check `statx()` first.

The currently in-progress extension of `RWF_ATOMIC` to multi-block atomicity in Linux 6.15+ will have differing behavior based on filesystem. For ext4, a write with `RWF_ATOMIC` for larger than the hardware’s number of atomically writable blocks will fail. For XFS, there will be a software fallback following an approach similar to copy-on-write: the data will first be written to a new extent, and then the extent tree will be modified to remap the overwritten offset of the file to point to the new data. Relying on this likely means the equivalent of two `fsync()` latencies, and increased overhead of extent metadata and offset lookup cost, so using one of the "on first write" torn write protection strategies described above will still yield better performance for the database.

[AlloyDB Omni](https://cloud.google.com/alloydb/omni) has [recently announced support](https://cloud.google.com/alloydb/omni/16.3.0/docs/improve-database-performance-using-io-acceleration#torn-write-protection) for `RWF_ATOMIC`. They advertise that enabling `alloydb_omni_atomic` means that one may disable `full_page_writes`, to avoid the overhead of the "Log Page on First Write" approach described above. Note that this still currently requires a storage device that supports atomically writing 8KB.

## Comparison

Our comparison is conducted along three dimensions:

* **Write Latency**: What is the minimum number of `fsync()` latencies required for a single row update to be made durable?
* **Write Amplification**: A single B-Tree leaf page update produces how many additional page-sized writes?
* **Space Amplification**: How much extra space, as compared to the total size of the database, does this approach require?

| Write Latency | Write Amplification | Space Amplification |
| Detection Only | 1x | 1x | 1x |
| Sector-Sized Pages | 1x | 1x | 1x |
| Log Full Pages | 1x | 2x | 1x-100x[1] |
| Log Page on First Write | ~1x[2] |  |  |
| Double-Write Buffer | 2x | 2x | 1x |
| Copy on Write | 2x | O(height of B-Tree) | O(2x) |
| Copy on First Write | ~1x | ~1x | O(2x) |
| Atomic Block Writes | 1x | 1x | 1x |
| Atomic Multi-Block Writes | 2x | 1x + ~1[3] | ~1x[4] |

[1]: The WAL size would increase by a factor of `size of page / average size of update`, and the only bound on the additional space used is determined by the checkpointing interval or policy.  
 [2]: ~1x, meaning sometimes it might be more than 1x, but most operations will experience 1x.  
 [3]: Or whatever approximate constant the total write amplification of an extent update in the average filesystem works out to be.  
 [4]: There will be some extra space overhead for XFS in storing extent metadata for every 8KB or 16KB page rather than at intervals of MB of data.

---

[↤ Decomposing Transactional Systems](/blog/2025-decomposing-transactional-systems.html) [↑ Blog ↑](/) [Modern Hardware for Future Databases ↦](/blog/2024-modern-database-hardware.html)

See discussion of this page on [Reddit](https://www.reddit.com/search/?q=url%3Atransactional.blog/blog/2025-torn-writes), [Hacker News](https://hn.algolia.com/?query=transactional.blog/blog/2025-torn-writes), and [Lobsters](https://lobste.rs/search?q=domain%3Atransactional.blog+title%3A%22Torn Write Detection and Protection%22&what=stories).
