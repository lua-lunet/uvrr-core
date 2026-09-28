<!-- source: https://dev.mysql.com/doc/refman/8.0/en/innodb-disk-io.html -->

[Skip to Main Content](#main)

The world's most popular open source database

[Developer Zone](/)  [Downloads](https://www.mysql.com/downloads/)  [MySQL.com](https://www.mysql.com/)

Section Menu:

[Documentation Home](/doc/)

---

MySQL 8.0 Reference Manual

Related Documentation

[MySQL 8.0 Release Notes](/doc/relnotes/mysql/8.0/en/)  
 [MySQL 8.0 Source Code Documentation](/doc/dev/mysql-server/latest/)

Download this Manual

[PDF (US Ltr)](https://downloads.mysql.com/docs/refman-8.0-en.pdf) - 43.2Mb  
 [PDF (A4)](https://downloads.mysql.com/docs/refman-8.0-en.a4.pdf) - 43.3Mb  
 [Man Pages (TGZ)](https://downloads.mysql.com/docs/refman-8.0-en.man-gpl.tar.gz) - 295.2Kb  
 [Man Pages (Zip)](https://downloads.mysql.com/docs/refman-8.0-en.man-gpl.zip) - 400.4Kb  
 [Info (Gzip)](https://downloads.mysql.com/docs/mysql-8.0.info.gz) - 4.3Mb  
 [Info (Zip)](https://downloads.mysql.com/docs/mysql-8.0.info.zip) - 4.3Mb

Excerpts from this Manual

[MySQL Globalization](/doc/mysql-g11n-excerpt/8.0/en/)  
 [MySQL Information Schema](/doc/mysql-infoschema-excerpt/8.0/en/)  
 [MySQL Installation Guide](/doc/mysql-installation-excerpt/8.0/en/)  
 [Security in MySQL](/doc/mysql-security-excerpt/8.0/en/)  
 [Starting and Stopping MySQL](/doc/mysql-startstop-excerpt/8.0/en/)  
 [MySQL and Linux/Unix](/doc/mysql-linuxunix-excerpt/8.0/en/)  
 [MySQL and Windows](/doc/mysql-windows-excerpt/8.0/en/)  
 [MySQL and macOS](/doc/mysql-macos-excerpt/8.0/en/)  
 [MySQL and Solaris](/doc/mysql-solaris-excerpt/8.0/en/)  
 [Building MySQL from Source](/doc/mysql-sourcebuild-excerpt/8.0/en/)  
 [MySQL Restrictions and Limitations](/doc/mysql-reslimits-excerpt/8.0/en/)  
 [MySQL Partitioning](/doc/mysql-partitioning-excerpt/8.0/en/)  
 [MySQL Tutorial](/doc/mysql-tutorial-excerpt/8.0/en/)  
 [MySQL Performance Schema](/doc/mysql-perfschema-excerpt/8.0/en/)  
 [MySQL Replication](/doc/mysql-replication-excerpt/8.0/en/)  
 [Using the MySQL Yum Repository](/doc/mysql-repo-excerpt/8.0/en/)  
 [MySQL NDB Cluster 8.0](/doc/mysql-cluster-excerpt/8.0/en/)

version 8.0

[26.7](/doc/refman/26.7/en/innodb-disk-io.html)   
  [9.7  current](/doc/refman/9.7/en/innodb-disk-io.html)   
  [8.4](/doc/refman/8.4/en/innodb-disk-io.html)   

[8.0  Japanese](/doc/refman/8.0/ja/innodb-disk-io.html)

[MySQL 8.0 Reference Manual](/doc/refman/8.0/en/)  /  ...  /   [The InnoDB Storage Engine](innodb-storage-engine.html)  /  [InnoDB Disk I/O and File Space Management](innodb-disk-management.html)  /   InnoDB Disk I/O

### 17.11.1 InnoDB Disk I/O

`InnoDB` uses asynchronous disk I/O where possible, by creating a number of threads to handle I/O operations, while permitting other database operations to proceed while the I/O is still in progress. On Linux and Windows platforms, `InnoDB` uses the available OS and library functions to perform “native” asynchronous I/O. On other platforms, `InnoDB` still uses I/O threads, but the threads may actually wait for I/O requests to complete; this technique is known as “simulated” asynchronous I/O.

If `InnoDB` can determine there is a high probability that data might be needed soon, it performs read-ahead operations to bring that data into the buffer pool so that it is available in memory. Making a few large read requests for contiguous data can be more efficient than making several small, spread-out requests. There are two read-ahead heuristics in `InnoDB`:

* In sequential read-ahead, if `InnoDB` notices that the access pattern to a segment in the tablespace is sequential, it posts in advance a batch of reads of database pages to the I/O system.
* In random read-ahead, if `InnoDB` notices that some area in a tablespace seems to be in the process of being fully read into the buffer pool, it posts the remaining reads to the I/O system.

For information about configuring read-ahead heuristics, see [Section 17.8.3.4, “Configuring InnoDB Buffer Pool Prefetching (Read-Ahead)”](innodb-performance-read_ahead.html "17.8.3.4 Configuring InnoDB Buffer Pool Prefetching (Read-Ahead)").

#### Doublewrite Buffer

`InnoDB` uses a novel file flush technique involving a structure called the [doublewrite buffer](glossary.html#glos_doublewrite_buffer "doublewrite buffer"), which is enabled by default in most cases ([`innodb_doublewrite=ON`](innodb-parameters.html#sysvar_innodb_doublewrite)). It adds safety to recovery following an unexpected exit or power outage, and improves performance on most varieties of Unix by reducing the need for `fsync()` operations.

Before writing pages to a data file, `InnoDB` first writes them to a storage area called the doublewrite buffer. Only after the write and the flush to the doublewrite buffer has completed does `InnoDB` write the pages to their proper positions in the data file. If there is an operating system, storage subsystem, or unexpected [**mysqld**](mysqld.html "6.3.1 mysqld — The MySQL Server") process exit in the middle of a page write (causing a [torn page](glossary.html#glos_torn_page "torn page") condition), `InnoDB` can later find a good copy of the page from the doublewrite buffer during recovery.

For more information about the doublewrite buffer, see [Section 17.6.4, “Doublewrite Buffer”](innodb-doublewrite-buffer.html "17.6.4 Doublewrite Buffer").

[PREV](innodb-disk-management.html "Previous: InnoDB Disk I/O and File Space Management")    [HOME](index.html "Start")    [UP](innodb-disk-management.html "Up: InnoDB Disk I/O and File Space Management")   [NEXT](innodb-file-space.html "Next: File Space Management")

Related Documentation

[MySQL 8.0 Release Notes](/doc/relnotes/mysql/8.0/en/)  
 [MySQL 8.0 Source Code Documentation](/doc/dev/mysql-server/latest/)

Download this Manual

[PDF (US Ltr)](https://downloads.mysql.com/docs/refman-8.0-en.pdf) - 43.2Mb  
 [PDF (A4)](https://downloads.mysql.com/docs/refman-8.0-en.a4.pdf) - 43.3Mb  
 [Man Pages (TGZ)](https://downloads.mysql.com/docs/refman-8.0-en.man-gpl.tar.gz) - 295.2Kb  
 [Man Pages (Zip)](https://downloads.mysql.com/docs/refman-8.0-en.man-gpl.zip) - 400.4Kb  
 [Info (Gzip)](https://downloads.mysql.com/docs/mysql-8.0.info.gz) - 4.3Mb  
 [Info (Zip)](https://downloads.mysql.com/docs/mysql-8.0.info.zip) - 4.3Mb

Excerpts from this Manual

[MySQL Globalization](/doc/mysql-g11n-excerpt/8.0/en/)  
 [MySQL Information Schema](/doc/mysql-infoschema-excerpt/8.0/en/)  
 [MySQL Installation Guide](/doc/mysql-installation-excerpt/8.0/en/)  
 [Security in MySQL](/doc/mysql-security-excerpt/8.0/en/)  
 [Starting and Stopping MySQL](/doc/mysql-startstop-excerpt/8.0/en/)  
 [MySQL and Linux/Unix](/doc/mysql-linuxunix-excerpt/8.0/en/)  
 [MySQL and Windows](/doc/mysql-windows-excerpt/8.0/en/)  
 [MySQL and macOS](/doc/mysql-macos-excerpt/8.0/en/)  
 [MySQL and Solaris](/doc/mysql-solaris-excerpt/8.0/en/)  
 [Building MySQL from Source](/doc/mysql-sourcebuild-excerpt/8.0/en/)  
 [MySQL Restrictions and Limitations](/doc/mysql-reslimits-excerpt/8.0/en/)  
 [MySQL Partitioning](/doc/mysql-partitioning-excerpt/8.0/en/)  
 [MySQL Tutorial](/doc/mysql-tutorial-excerpt/8.0/en/)  
 [MySQL Performance Schema](/doc/mysql-perfschema-excerpt/8.0/en/)  
 [MySQL Replication](/doc/mysql-replication-excerpt/8.0/en/)  
 [Using the MySQL Yum Repository](/doc/mysql-repo-excerpt/8.0/en/)  
 [MySQL NDB Cluster 8.0](/doc/mysql-cluster-excerpt/8.0/en/)
