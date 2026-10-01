<!-- source: https://www.percona.com/blog/a-tale-of-two-databases-how-postgresql-and-mysql-handle-torn-pages -->

[Download the New 2026 State of Open Source Database Management Report](https://www.percona.com/resource/state-of-open-source-db-management/)

[![](https://www.percona.com/wp-content/uploads/2026/02/Logo.svg)](/)

[See all results →](#)

[Downloads](/downloads/)

[Blog](/blog/)

[Contact Us](/contact-us/)

# A Tale of Two Databases: How PostgreSQL and MySQL Handle Torn Pages

July 3, 2025

Author

 [Pep Pla](https://www.percona.com/blog/author/pep-pla/) 

[Pep Pla](https://www.percona.com/blog/author/pep-pla/)

[Insight for DBAs](https://www.percona.com/blog/category/insight-for-dbas/)

[MySQL](https://www.percona.com/blog/category/mysql/)

[PostgreSQL](https://www.percona.com/blog/category/postgresql/)

Share this Post:

![](https://www.percona.com/wp-content/uploads/2026/04/Percona_Texture_109.jpg)

Welcome to this first installment of the blog series, which explores how PostgreSQL and MySQL deal with different aspects of relational databases. This post is about how to handle torn pages. As a long-time open source database administrator, I have always been fascinated by the differences in how these two databases handle various challenges and how DBAs who know one of these technologies often lack understanding of the other. This series aims to provide insight into how these work and to help bridge the gap between the two communities.

Even if you are a seasoned PostgreSQL or MySQL DBA, you will learn something new and interesting from these posts. Sometimes, the topics may be a bit challenging to digest, but I will try to make them as funny and engaging as possible.

## What is a torn page?

Did I already say that some of the topics could be a bit hard to digest? Well, if there are topics that are hard to digest, this is probably one of them: Torn pages. But as Shakespeare wrote:

> *“An’t please your honour,  
> We are but men; and what so many may do,  
> Not being torn a-pieces, we have done:  
> An army cannot rule ’em.”*

Well, maybe using quotes from the Immortal Bard will not make this topic easier to digest, but it brings a bit of distinction and culture to this otherwise completely technical blog post. And we all need a bit of culture in our lives, right?

But let’s stop digressing and get back to the topic. If a page is the basic unit of storage in a database, a torn page is a page that has been partially written to disk. This happens for multiple reasons; the first is that database pages are bigger than the physical sectors of the disk. Additionally, the filesystem usually does not provide atomic writes. So, what happens when the server crashes in the middle of writing a database page? The page will be written partially, leaving the database in an inconsistent state.

## How do we handle a torn page?

Well, we can’t really avoid them, but we can detect them and, what is more important, we can recover from them. How? By using redundancy. For example, I used the word “page” six times in the last paragraph of the previous section. This is redundancy.

*Don’t tell me that you went back to count the number of times I used the word page.*

How do we use redundancy to recover from torn pages? We write the page twice, once in a temporary location and then in the final location. Later, if we find that both are different during the recovery process, we just use the correct one.

But how do we know which one is correct? We will discuss this later because it is one of the differences between PostgreSQL and MySQL.

## Let’s synchronize our watches… I mean disks

Before we go into the details of how PostgreSQL and MySQL handle torn pages, let’s talk briefly about how writing is usually performed by those little liars, also known as operating systems.

When an application tells the operating system to write a page to disk, the OS usually writes that page to a cache in memory and later flushes it to disk. This is done to improve performance, but it also means that when the OS says that a page has been written, it is more like an acknowledgment that the page has to be written rather than an actual confirmation that the page has been written to disk.

But this goes against the ACID principle of durability, as the call to the write function can return before the data is actually on disk. To fix this mess, the operating system provides another function to synchronize pending writes to disk; this is what the sync () system call does.

As the fsync man page says (after translation to a human-understandable language):

fsync() transfers (“flushes”) all modified data of the file, referred to as a parameter, to the disk device (or other permanent storage device) so that all changed information can be retrieved even if the system crashes or is rebooted. This includes writing through or flushing a disk cache if present. The call blocks until the device reports that the transfer has been completed.

To make it simple, we do not only need to write the data to disk; we also need to flush it to disk, so no data loss is guaranteed in case of a crash or power failure.

The normal flow of data writing is as follows:

1. **The database writes to a memory buffer.**
2. **The database writes the memory buffer to a file on the disk.**
3. **The database calls fsync to flush a specific file on the disk.**

Each step takes longer to complete than the previous one. But the good news is that we can perform each step multiple times before performing the following one. If we size the memory buffer properly, we can write multiple times before writing the buffer to disk. If we size some files properly, we can write multiple buffers before having to flush them.

Obviously, this is oversimplified, and multiple factors impact how often we write into the buffer, how often we write the buffer to disk, and how often we flush the file. But I hope you get the idea.

## Another brick in the WAL

Ok, the title of this blog post mentions torn pages and two databases. So far, we have only talked about generic stuff that applies to any database or, even worse, to any application that writes data to disk. Now I could tell you that the wait is over, and we are finally going to talk about how PostgreSQL and MySQL handle torn pages, but I am afraid that I have to disappoint you again. I hope that after all this ultra-extended introduction, everything will make sense, all the pieces will fall into place, everything will be crystal clear, and you will be able to get rid of my redundant writing style and move to something different.

Databases usually store data in multiple files. This means that any change has to be written into the corresponding buffer (specific for that file), then written to the file on disk and later flushed. All this writing and flushing usually means overspending resources. So, ideally, we want to write and flush as much data as possible in each operation.

But if we wait too long, the system may crash, and data will be lost. How could we avoid this? How to do it may sound a bit counterintuitive, but the way to avoid data loss and increase efficiency is to write more data than necessary. This is what the Write Ahead Log (WAL) in PostgreSQL or the Redo Log in MySQL do.

Instead of writing changes into multiple files, we **write them into a single file**. This way, we ensure durability, and we have more time (more data) to write into each of the data files. Sometimes, it can even happen that we do not need to write the data in the final destination because it has been overwritten multiple times, and we only have to write the final version.

If you remember, we said that a page is the basic unit of storage in a database. However, for the WAL, we optimize the amount of data written by writing only what has changed instead of the whole page. This allows us to write more information (data related to multiple pages) in each write operation.

## It is a checkpoint, Charlie!

We will not leave the American Sector before discussing checkpoints. (*Don’t worry if you are too young to understand this joke.*)

We’ve seen that WAL or Redo Logs are great for increasing write efficiency, but the way they are structured is very inefficient for reading—or, to be more precise, finding specific data for a certain table in the database. This means that sooner or later, we will have to write the data into the final destination, where reads are more efficient.

While a page contains data that hasn’t been written to the final destination, we call it a dirty page. And who writes the dirty pages on the disk? In PostgreSQL, two processes assume this task: the checkpointer and the background writer. In MySQL, we have multiple threads that do this job based on the number of free pages in the buffer pool (if a page is dirty, it cannot be evicted from the buffer pool without being written to disk first), the amount of WAL written, the time since the last checkpoint and other factors.

**But what is a checkpoint?** Although PostgreSQL and MySQL use the same name, they are different concepts for each database. In PostgreSQL, a [checkpoint](https://www.percona.com/blog/evaluating-checkpointing-in-postgresql/) is a point when all dirty pages are written to disk; in other words, there are no dirty pages in the database memory. In MySQL, a checkpoint is a point where all the dirty pages generated before that point are written to disk. The difference here is that the point in time for MySQL is not the time the checkpoint happens; it is just a point in time, and all the dirty pages generated after that point will not be written.

I know this is not my best explanation, and I probably lost you, but let me try to recover you by oversimplifying it: In PostgreSQL, after a checkpoint, there are no dirty pages. In MySQL, after a checkpoint, there are still dirty pages.

(*FYI, what PostgreSQL does is called a Sharp Checkpoint, while what MySQL does is a Fuzzy Checkpoint.)*

The great thing about checkpoints is that they provide a consistent snapshot of the database at the moment of the checkpoint. If there is a crash, we only have to apply the changes from the WAL or Redo since the last checkpoint, and this is the same for both PostgreSQL and MySQL. The process of applying changes during the database boot after a crash is called recovery.

The problem with checkpoints is that if they have to write a lot of dirty pages, they can generate a spike of IO that can impact the database’s performance. To avoid this, dirty pages are also written between checkpoints to avoid the impact of write spikes.

Summarizing, data is written during the checkpoint, by the background writing process in the form of pages, or when we write the WAL in the form of WAL records. And what happens if the system crashes while these write operations are happening?

## A torn page is born

Correct, if the database crashes while we are writing, it is possible that we have data partially written to disk, and we have one or more torn pages.

*Very good, Pep*. After 1786 words, we’re at the same point where we started. If a page is half-written during a crash, we have a torn page. But how do we recover from it? You already told us that we can recover from torn pages by writing the page twice and using the correct one. Please, let’s move on!

Your prayers have been heard. We will finally talk about how PostgreSQL and MySQL recover from torn pages.

## How does PostgreSQL handle torn pages?

If you remember, after a checkpoint, the database has a consistent snapshot of the data. This means that there are no torn pages after a checkpoint. They can only happen when the database crashes during the background writing or if the crash occurs during the checkpoint itself.

The method used by PostgreSQL to recover from torn pages is based on something called Full Page Writes (FPW). The first time a page is modified after a checkpoint, a full copy of the page is written to the WAL. That copy of the page will be used during recovery regardless of whether the page was torn or not. After the FPW, any modification to that page must only write what has been changed to the WAL.

Nice, but what happens if the torn page is the one written to the WAL? Well, in that case, the page’s checksum validation will fail during recovery, and we will know that the transaction that modified the page was not successfully committed, so we may discard that page. Commit only happens when the entry has been correctly written to the WAL.

Ahhh! But what happens if the torn page happens during the checkpoint? In that case, the checkpoint will not be properly written in the WAL files, and we will need to use the previous checkpoint to perform the recovery. This is not a problem, as the previous checkpoint will have a consistent snapshot of the database, so we can apply the changes from the WAL since that checkpoint.

**Concept to remember**: PostgreSQL writes the redundant page to the WAL, and this is done the first time the page is modified after a checkpoint.

## How does MySQL (InnoDB) handle torn pages?

MySQL’s (InnoDB) approach to recovering from torn pages is different, but it also relies on redundancy. InnoDB uses a doublewrite buffer, which, for recent MySQL versions, is a separate area (files) on the disk where modified pages are first written before being written to their final location. This means that every time MySQL has to write a page on the disk, it writes it first in the doublewrite buffer and then in the data file itself.

During recovery, we check the checksums of the pages in the doublewrite buffer and at the final destination. If both are correct and the same, no torn pages have happened. If the doublewrite version is wrong, it can be discarded, and the page will be fixed with the change records in the redo log, as the page in the final destination is correct (but old). In any other case, the doublewrite buffer version is used to restore the page to the final location.

To improve performance, MySQL does not write a single page in the doublewrite buffer but a whole block of pages and then performs a unique fsync call.

**Remember:** MySQL writes the redundant page to the doublewrite buffer whenever a page needs to be written to disk.

### Final thoughts

PostgreSQL uses the WAL file to write a full copy of the page when it is modified after a checkpoint. This means that the transaction will have a slightly higher overhead. Frequent checkpoints may also have a performance impact, as they increase the number of full-page writes. Tuning the checkpoint frequency can significantly improve PostgreSQL’s performance.

MySQL, on the other hand, uses a doublewrite buffer to write multiple pages in one shot before writing them to their final location. This makes tuning checkpoint frequency less critical, but this method, combined with the fuzzy checkpoint strategy, can lead to longer recovery times in the case of a crash.

Thank you for reading this blog post, and I hope you found it interesting and informative on how to handle torn pages. If you have any questions or comments, please get in touch with me. And if you are a PostgreSQL or MySQL DBA, I hope you learned something new today.

0 0 votes

Article Rating

Subscribe

3 Comments

Oldest

Newest Most Voted

[![Frederic Descamps](https://secure.gravatar.com/avatar/ac815ccd563ad2e3cf5e7b179bab485d6922fd1cf1e1ea00807ef7054e31025e?s=64&d=mm&r=g)](https://www.percona.com/blog/author/lefred/)

[Frederic Descamps](http://www.lefred.be)

1 year ago

Nice writeup Pep!  
 I would like to add that during the last MySQL & HeatWave Summit, we also announced that, on kernels that support it, we have disabled double write for InnoDB, with a 50% performance gain expected. See <https://youtu.be/yUdDO04udYM?si=CjoOgK_4b1JA-nQi&t=1815>  
 Cheers

0

Reply

![Markus](https://secure.gravatar.com/avatar/43672fd3915e207cedd2e248bf2d3ea3aacf4ec11e574b386cc2d5ce7503a13d?s=64&d=mm&r=g)

Markus

1 year ago

You wrote that PG uses these FPW to recover torn pages.

Afaik, it is not possible to recover torn pages only.  
 One have to recover the whole database to a PITR before the torn page happend.  
 No?

0

Reply

[![Pep Pla](https://secure.gravatar.com/avatar/310e4e32e48f52c6f25cb90de519827c1f8bd48e46f2b97dcf42b16d2a4162d8?s=64&d=mm&r=g)](https://www.percona.com/blog/author/pep-pla/)

Author

[Pep Pla](https://www.percona.com/blog/author/pep-pla/)

1 year ago

Reply to   [Markus](#comment-23410)

Recovery requires the WAL files. The FPWs are written in the WAL files, but they are not enough for recovery.

Also, changes that occur between checkpoints are written to the WAL and used for recovery/PITR.

0

Reply

Resources

RELATED POSTS

[![](https://www.percona.com/wp-content/uploads/2026/04/Percona_Texture_17.jpg)

MySQL

September 21, 2026

|

Catalin Besleaga

###### DISTANCE() and VECTOR\_DISTANCE(): Vector Similarity in Percona Server for MySQL 9.7](https://www.percona.com/blog/vector-distance-vector-similarity-search-in-percona-server-for-mysql/)

[![](https://www.percona.com/wp-content/uploads/2026/04/Percona_Texture_234.jpg)

MySQL

|

September 17, 2026

|

Anil Joshi

###### Group Replication Beyond a Single Cluster: DC-DR with Percona (PS MySQL) Operator](https://www.percona.com/blog/group-replication-beyond-a-single-cluster-dc-dr-with-percona-ps-mysql-operator/)

[![](https://www.percona.com/wp-content/uploads/2026/09/goat-pxc.png)

Percona

|

September 17, 2026

|

Kedar Vaijanapurkar

###### Too many GCache Page Files in MySQL Data Directory](https://www.percona.com/blog/too-many-gcache-page-files-in-mysql-data-directory/)

## Far Enough.

![](https://www.percona.com/wp-content/uploads/2026/02/Polygon-3.svg)

Said no pioneer ever.

[Get Started](https://www.percona.com/contact-us/)

![](https://www.percona.com/wp-content/uploads/2026/02/Layer_2.svg)

Open source database software from experts who stand with you in production. Forever free from lock-in and other corporate BS.

© 2026 Percona All Rights Reserved

Database Technology

 [PostgreSQL](https://www.percona.com/postgresql/)  [MySQL](https://www.percona.com/mysql/)  [MariaDB](https://www.percona.com/mariadb-support/)  [MongoDB](https://www.percona.com/mongodb/)  [Valkey / Redis](https://www.percona.com/valkey-redis/)  [Databases on Kubernetes](https://www.percona.com/cloud-native/)  [Monitoring & Management (PMM)](https://www.percona.com/monitoring/)  [Percona Toolkit](https://www.percona.com/toolkit/)

Services

 [Expert Support](https://www.percona.com/services/expert-support/)  [ExpertOps](https://www.percona.com/services/expertops/)  [Expert Consulting](https://www.percona.com/services/expert-consulting-and-services/)  [Training](https://www.percona.com/training/)  [Solution Bundles](https://www.percona.com/solution-bundles/)  [Subscription Policies](/subscription-policies/)

Use Cases

 [Cost Optimization](https://www.percona.com/use-cases/cost-optimization/)  [Performance and Reliability](https://www.percona.com/use-cases/performance-and-reliability/)  [Sovereignty, Security, and Compliance](https://www.percona.com/use-cases/sovereignty-security-and-compliance/)  [AI and Future Readiness](https://www.percona.com/use-cases/ai-and-future-readiness/)

Company

 [About Percona](https://www.percona.com/about/)  [Partners](https://www.percona.com/partners/)  [Careers](https://www.percona.com/careers/)  [Events](https://www.percona.com/events/)  [Press Releases](https://www.percona.com/press/)  [News](https://www.percona.com/news/)

[Terms of Use](/platform-terms-of-service/) 

|

 [Privacy](/privacy-policy/) 

|

 [Copyright](/copyright-policy/) 

|

 [Legal](/legal/) 

|

 [Security Center](https://trust.percona.com/)

MySQL, PostgreSQL, InnoDB, MariaDB, MongoDB and Kubernetes are trademarks for their respective owners.

© 2026 Percona All Rights Reserved

wpDiscuz
