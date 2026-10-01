<!-- source: https://dev.mysql.com/doc/refman/8.4/en/innodb-doublewrite-buffer.html -->

[Skip to Main Content](#main)

The world's most popular open source database

[Developer Zone](/)  [Downloads](https://www.mysql.com/downloads/)  [MySQL.com](https://www.mysql.com/)

Section Menu:

[Documentation Home](/doc/)

---

MySQL 8.4 Reference Manual

Related Documentation

[MySQL 8.4 Release Notes](/doc/relnotes/mysql/8.4/en/)

Download this Manual

[PDF (US Ltr)](https://downloads.mysql.com/docs/refman-8.4-en.pdf) - 40.2Mb  
 [PDF (A4)](https://downloads.mysql.com/docs/refman-8.4-en.a4.pdf) - 40.3Mb  
 [Man Pages (TGZ)](https://downloads.mysql.com/docs/refman-8.4-en.man-gpl.tar.gz) - 261.9Kb  
 [Man Pages (Zip)](https://downloads.mysql.com/docs/refman-8.4-en.man-gpl.zip) - 367.5Kb  
 [Info (Gzip)](https://downloads.mysql.com/docs/mysql-8.4.info.gz) - 4.0Mb  
 [Info (Zip)](https://downloads.mysql.com/docs/mysql-8.4.info.zip) - 4.0Mb

version 8.4

[26.7](/doc/refman/26.7/en/innodb-doublewrite-buffer.html)   
  [9.7  current](/doc/refman/9.7/en/innodb-doublewrite-buffer.html)   
  [8.0](/doc/refman/8.0/en/innodb-doublewrite-buffer.html)   

[8.0  Japanese](/doc/refman/8.0/ja/innodb-doublewrite-buffer.html)

[MySQL 8.4 Reference Manual](/doc/refman/8.4/en/)  /  ...  /   [The InnoDB Storage Engine](innodb-storage-engine.html)  /  [InnoDB On-Disk Structures](innodb-on-disk-structures.html)  /   Doublewrite Buffer

### 17.6.4 Doublewrite Buffer

The doublewrite buffer is a storage area where `InnoDB` writes pages flushed from the buffer pool before writing the pages to their proper positions in the `InnoDB` data files. If there is an operating system, storage subsystem, or unexpected [**mysqld**](mysqld.html "6.3.1 mysqld — The MySQL Server") process exit in the middle of a page write, `InnoDB` can find a good copy of the page from the doublewrite buffer during crash recovery.

Although data is written twice, the doublewrite buffer does not require twice as much I/O overhead or twice as many I/O operations. Data is written to the doublewrite buffer in a large sequential chunk, with a single `fsync()` call to the operating system (except in the case that [`innodb_flush_method`](innodb-parameters.html#sysvar_innodb_flush_method) is set to `O_DIRECT_NO_FSYNC`).

The doublewrite buffer storage area is located in doublewrite files.

The following variables are provided for doublewrite buffer configuration:

* The [`innodb_doublewrite`](innodb-parameters.html#sysvar_innodb_doublewrite) variable controls whether the doublewrite buffer is enabled. It is enabled by default in most cases. To disable the doublewrite buffer, set [`innodb_doublewrite`](innodb-parameters.html#sysvar_innodb_doublewrite) to `OFF`. Consider disabling the doublewrite buffer if you are more concerned with performance than data integrity, as may be the case when performing benchmarks, for example.

  [`innodb_doublewrite`](innodb-parameters.html#sysvar_innodb_doublewrite) supports `DETECT_AND_RECOVER` and `DETECT_ONLY` settings.

  The `DETECT_AND_RECOVER` setting is the same as the `ON` setting. With this setting, the doublewrite buffer is fully enabled, with database page content written to the doublewrite buffer where it is accessed during recovery to fix incomplete page writes.

  With the `DETECT_ONLY` setting, only metadata is written to the doublewrite buffer. Database page content is not written to the doublewrite buffer, and recovery does not use the doublewrite buffer to fix incomplete page writes. This lightweight setting is intended for detecting incomplete page writes only.

  MySQL supports dynamic changes to the [`innodb_doublewrite`](innodb-parameters.html#sysvar_innodb_doublewrite) setting that enables the doublewrite buffer, between `ON`, `DETECT_AND_RECOVER`, and `DETECT_ONLY`. MySQL does not support dynamic changes between a setting that enables the doublewrite buffer and `OFF` or vice versa.

  If the doublewrite buffer is located on a Fusion-io device that supports atomic writes, the doublewrite buffer is automatically disabled and data file writes are performed using Fusion-io atomic writes instead. However, be aware that the [`innodb_doublewrite`](innodb-parameters.html#sysvar_innodb_doublewrite) setting is global. When the doublewrite buffer is disabled, it is disabled for all data files including those that do not reside on Fusion-io hardware. This feature is only supported on Fusion-io hardware and is only enabled for Fusion-io NVMFS on Linux. To take full advantage of this feature, an [`innodb_flush_method`](innodb-parameters.html#sysvar_innodb_flush_method) setting of `O_DIRECT` is recommended.
* The [`innodb_doublewrite_dir`](innodb-parameters.html#sysvar_innodb_doublewrite_dir) variable defines the directory where `InnoDB` creates doublewrite files. If no directory is specified, doublewrite files are created in the [`innodb_data_home_dir`](innodb-parameters.html#sysvar_innodb_data_home_dir) directory, which defaults to the data directory if unspecified.

  A hash symbol '#' is automatically prefixed to the specified directory name to avoid conflicts with schema names. However, if a '.', '#'. or '/' prefix is specified explicitly in the directory name, the hash symbol '#' is not prefixed to the directory name.

  Ideally, the doublewrite directory should be placed on the fastest storage media available.
* The [`innodb_doublewrite_files`](innodb-parameters.html#sysvar_innodb_doublewrite_files) variable defines the number of doublewrite files, which defaults to 2. By default, two doublewrite files are created for each buffer pool instance: A flush list doublewrite file and an LRU list doublewrite file.

  The flush list doublewrite file is for pages flushed from the buffer pool flush list. The default size of a flush list doublewrite file is the `InnoDB` page size \* doublewrite page bytes.

  The LRU list doublewrite file is for pages flushed from the buffer pool LRU list. It also contains slots for single page flushes. The default size of an LRU list doublewrite file is the `InnoDB` page size \* (doublewrite pages + (512 / the number of buffer pool instances)) where 512 is the total number of slots reserved for single page flushes.

  At a minimum, there are two doublewrite files. The maximum number of doublewrite files is two times the number of buffer pool instances. (The number of buffer pool instances is controlled by the [`innodb_buffer_pool_instances`](innodb-parameters.html#sysvar_innodb_buffer_pool_instances) variable.)

  Doublewrite file names have the following format: `#ib_page_size_file_number.dblwr` (or `.bdblwr` with the `DETECT_ONLY` setting). For example, the following doublewrite files are created for a MySQL instance with an `InnoDB` pages size of 16KB and a single buffer pool:

  ```
  #ib_16384_0.dblwr #ib_16384_1.dblwr
  ```

  The [`innodb_doublewrite_files`](innodb-parameters.html#sysvar_innodb_doublewrite_files) variable is intended for advanced performance tuning. The default setting should be suitable for most users.
* The [`innodb_doublewrite_pages`](innodb-parameters.html#sysvar_innodb_doublewrite_pages) variable controls the maximum number of doublewrite pages per thread. This variable is intended for advanced performance tuning. The default value should be suitable for most users.

`InnoDB` automatically encrypts doublewrite file pages that belong to encrypted tablespaces (see [Section 17.13, “InnoDB Data-at-Rest Encryption”](innodb-data-encryption.html "17.13 InnoDB Data-at-Rest Encryption")). Likewise, doublewrite file pages belonging to page-compressed tablespaces are compressed. As a result, doublewrite files can contain different page types including unencrypted and uncompressed pages, encrypted pages, compressed pages, and pages that are both encrypted and compressed.

[PREV](innodb-tablespace-autoextend-size.html "Previous: Tablespace AUTOEXTEND_SIZE Configuration")    [HOME](index.html "Start")    [UP](innodb-on-disk-structures.html "Up: InnoDB On-Disk Structures")   [NEXT](innodb-redo-log.html "Next: Redo Log")

Related Documentation

[MySQL 8.4 Release Notes](/doc/relnotes/mysql/8.4/en/)

Download this Manual

[PDF (US Ltr)](https://downloads.mysql.com/docs/refman-8.4-en.pdf) - 40.2Mb  
 [PDF (A4)](https://downloads.mysql.com/docs/refman-8.4-en.a4.pdf) - 40.3Mb  
 [Man Pages (TGZ)](https://downloads.mysql.com/docs/refman-8.4-en.man-gpl.tar.gz) - 261.9Kb  
 [Man Pages (Zip)](https://downloads.mysql.com/docs/refman-8.4-en.man-gpl.zip) - 367.5Kb  
 [Info (Gzip)](https://downloads.mysql.com/docs/mysql-8.4.info.gz) - 4.0Mb  
 [Info (Zip)](https://downloads.mysql.com/docs/mysql-8.4.info.zip) - 4.0Mb
