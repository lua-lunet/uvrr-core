           ZooKeeper: Wait-free coordination for Internet-scale systems

           Patrick Hunt and Mahadev Konar                         Flavio P. Junqueira and Benjamin Reed
                     Yahoo! Grid                                              Yahoo! Research
            {phunt,mahadev}@yahoo-inc.com                               {fpj,breed}@yahoo-inc.com




                       Abstract                                    that implement mutually exclusive access to critical re-
                                                                   sources.
In this paper, we describe ZooKeeper, a service for co-
                                                                      One approach to coordination is to develop services
ordinating processes of distributed applications. Since
                                                                   for each of the different coordination needs. For exam-
ZooKeeper is part of critical infrastructure, ZooKeeper
                                                                   ple, Amazon Simple Queue Service [3] focuses specif-
aims to provide a simple and high performance kernel
                                                                   ically on queuing. Other services have been devel-
for building more complex coordination primitives at the
                                                                   oped specifically for leader election [25] and configura-
client. It incorporates elements from group messaging,
                                                                   tion [27]. Services that implement more powerful prim-
shared registers, and distributed lock services in a repli-
                                                                   itives can be used to implement less powerful ones. For
cated, centralized service. The interface exposed by Zoo-
                                                                   example, Chubby [6] is a locking service with strong
Keeper has the wait-free aspects of shared registers with
                                                                   synchronization guarantees. Locks can then be used to
an event-driven mechanism similar to cache invalidations
                                                                   implement leader election, group membership, etc.
of distributed file systems to provide a simple, yet pow-
erful coordination service.                                           When designing our coordination service, we moved
                                                                   away from implementing specific primitives on the
   The ZooKeeper interface enables a high-performance
                                                                   server side, and instead we opted for exposing an API
service implementation. In addition to the wait-free
                                                                   that enables application developers to implement their
property, ZooKeeper provides a per client guarantee of
                                                                   own primitives. Such a choice led to the implementa-
FIFO execution of requests and linearizability for all re-
                                                                   tion of a coordination kernel that enables new primitives
quests that change the ZooKeeper state. These design de-
                                                                   without requiring changes to the service core. This ap-
cisions enable the implementation of a high performance
                                                                   proach enables multiple forms of coordination adapted to
processing pipeline with read requests being satisfied by
                                                                   the requirements of applications, instead of constraining
local servers. We show for the target workloads, 2:1
                                                                   developers to a fixed set of primitives.
to 100:1 read to write ratio, that ZooKeeper can handle
tens to hundreds of thousands of transactions per second.             When designing the API of ZooKeeper, we moved
This performance allows ZooKeeper to be used exten-                away from blocking primitives, such as locks. Blocking
sively by client applications.                                     primitives for a coordination service can cause, among
                                                                   other problems, slow or faulty clients to impact nega-
                                                                   tively the performance of faster clients. The implemen-
1   Introduction                                                   tation of the service itself becomes more complicated
                                                                   if processing requests depends on responses and fail-
Large-scale distributed applications require different             ure detection of other clients. Our system, Zookeeper,
forms of coordination. Configuration is one of the most            hence implements an API that manipulates simple wait-
basic forms of coordination. In its simplest form, con-            free data objects organized hierarchically as in file sys-
figuration is just a list of operational parameters for the        tems. In fact, the ZooKeeper API resembles the one of
system processes, whereas more sophisticated systems               any other file system, and looking at just the API signa-
have dynamic configuration parameters. Group member-               tures, ZooKeeper seems to be Chubby without the lock
ship and leader election are also common in distributed            methods, open, and close. Implementing wait-free data
systems: often processes need to know which other pro-             objects, however, differentiates ZooKeeper significantly
cesses are alive and what those processes are in charge            from systems based on blocking primitives such as locks.
of. Locks constitute a powerful coordination primitive                Although the wait-free property is important for per-

                                                              1
formance and fault tolerance, it is not sufficient for co-        tion of ZooKeeper. With ZooKeeper, we are able to im-
ordination. We have also to provide order guarantees for          plement all coordination primitives that our applications
operations. In particular, we have found that guarantee-          require, even though only writes are linearizable. To val-
ing both FIFO client ordering of all operations and lin-          idate our approach we show how we implement some
earizable writes enables an efficient implementation of           coordination primitives with ZooKeeper.
the service and it is sufficient to implement coordination        To summarize, in this paper our main contributions are:
primitives of interest to our applications. In fact, we can       Coordination kernel: We propose a wait-free coordi-
implement consensus for any number of processes with                   nation service with relaxed consistency guarantees
our API, and according to the hierarchy of Herlihy, Zoo-               for use in distributed systems. In particular, we de-
Keeper implements a universal object [14].                             scribe our design and implementation of a coordi-
   The ZooKeeper service comprises an ensemble of                      nation kernel, which we have used in many criti-
servers that use replication to achieve high availability              cal applications to implement various coordination
and performance. Its high performance enables appli-                   techniques.
cations comprising a large number of processes to use             Coordination recipes: We show how ZooKeeper can
such a coordination kernel to manage all aspects of co-                be used to build higher level coordination primi-
ordination. We were able to implement ZooKeeper us-                    tives, even blocking and strongly consistent primi-
ing a simple pipelined architecture that allows us to have             tives, that are often used in distributed applications.
hundreds or thousands of requests outstanding while still         Experience with Coordination: We share some of the
achieving low latency. Such a pipeline naturally enables               ways that we use ZooKeeper and evaluate its per-
the execution of operations from a single client in FIFO               formance.
order. Guaranteeing FIFO client order enables clients to
submit operations asynchronously. With asynchronous
operations, a client is able to have multiple outstanding         2     The ZooKeeper service
operations at a time. This feature is desirable, for exam-
ple, when a new client becomes a leader and it has to ma-         Clients submit requests to ZooKeeper through a client
nipulate metadata and update it accordingly. Without the          API using a ZooKeeper client library. In addition to ex-
possibility of multiple outstanding operations, the time          posing the ZooKeeper service interface through the client
of initialization can be of the order of seconds instead of       API, the client library also manages the network connec-
sub-second.                                                       tions between the client and ZooKeeper servers.
                                                                     In this section, we first provide a high-level view of the
   To guarantee that update operations satisfy lineariz-
                                                                  ZooKeeper service. We then discuss the API that clients
ability, we implement a leader-based atomic broadcast
                                                                  use to interact with ZooKeeper.
protocol [23], called Zab [24]. A typical workload
of a ZooKeeper application, however, is dominated by
read operations and it becomes desirable to scale read            Terminology. In this paper, we use client to denote a
throughput. In ZooKeeper, servers process read opera-             user of the ZooKeeper service, server to denote a process
tions locally, and we do not use Zab to totally order them.       providing the ZooKeeper service, and znode to denote
   Caching data on the client side is an important tech-          an in-memory data node in the ZooKeeper data, which
nique to increase the performance of reads. For example,          is organized in a hierarchical namespace referred to as
it is useful for a process to cache the identifier of the         the data tree. We also use the terms update and write to
current leader instead of probing ZooKeeper every time            refer to any operation that modifies the state of the data
it needs to know the leader. ZooKeeper uses a watch               tree. Clients establish a session when they connect to
mechanism to enable clients to cache data without man-            ZooKeeper and obtain a session handle through which
aging the client cache directly. With this mechanism,             they issue requests.
a client can watch for an update to a given data object,
and receive a notification upon an update. Chubby man-
                                                                  2.1    Service overview
ages the client cache directly. It blocks updates to in-
validate the caches of all clients caching the data being         ZooKeeper provides to its clients the abstraction of a set
changed. Under this design, if any of these clients is            of data nodes (znodes), organized according to a hierar-
slow or faulty, the update is delayed. Chubby uses leases         chical name space. The znodes in this hierarchy are data
to prevent a faulty client from blocking the system indef-        objects that clients manipulate through the ZooKeeper
initely. Leases, however, only bound the impact of slow           API. Hierarchical name spaces are commonly used in file
or faulty clients, whereas ZooKeeper watches avoid the            systems. It is a desirable way of organizing data objects,
problem altogether.                                               since users are used to this abstraction and it enables bet-
   In this paper we discuss our design and implementa-            ter organization of application meta-data. To refer to a


                                                              2
given znode, we use the standard UNIX notation for file           chical keys. The hierarchal namespace is useful for al-
system paths. For example, we use /A/B/C to denote                locating subtrees for the namespace of different applica-
the path to znode C, where C has B as its parent and B            tions and for setting access rights to those subtrees. We
has A as its parent. All znodes store data, and all znodes,       also exploit the concept of directories on the client side to
except for ephemeral znodes, can have children.                   build higher level primitives as we will see in section 2.4.
                                                                     Unlike files in file systems, znodes are not designed
                                                                  for general data storage. Instead, znodes map to abstrac-
                              /
                                                                  tions of the client application, typically corresponding
                                                                  to meta-data used for coordination purposes. To illus-
             /app1                             /app2
                                                                  trate, in Figure 1 we have two subtrees, one for Applica-
                                                                  tion 1 (/app1) and another for Application 2 (/app2).
                                                                  The subtree for Application 1 implements a simple group
                                                                  membership protocol: each client process pi creates a
                                                                  znode p i under /app1, which persists as long as the
      /app1/p_1   /app1/p_2   /app1/p_3                           process is running.
                                                                     Although znodes have not been designed for general
Figure 1: Illustration of ZooKeeper hierarchical name             data storage, ZooKeeper does allow clients to store some
space.                                                            information that can be used for meta-data or configu-
                                                                  ration in a distributed computation. For example, in a
                                                                  leader-based application, it is useful for an application
   There are two types of znodes that a client can create:        server that is just starting to learn which other server is
Regular: Clients manipulate regular znodes by creating            currently the leader. To accomplish this goal, we can
      and deleting them explicitly;                               have the current leader write this information in a known
Ephemeral: Clients create such znodes, and they ei-               location in the znode space. Znodes also have associated
      ther delete them explicitly, or let the system remove       meta-data with time stamps and version counters, which
      them automatically when the session that creates            allow clients to track changes to znodes and execute con-
      them terminates (deliberately or due to a failure).         ditional updates based on the version of the znode.
   Additionally, when creating a new znode, a client can
set a sequential flag. Nodes created with the sequen-             Sessions. A client connects to ZooKeeper and initiates
tial flag set have the value of a monotonically increas-          a session. Sessions have an associated timeout. Zoo-
ing counter appended to its name. If n is the new znode           Keeper considers a client faulty if it does not receive any-
and p is the parent znode, then the sequence value of n           thing from its session for more than that timeout. A ses-
is never smaller than the value in the name of any other          sion ends when clients explicitly close a session handle
sequential znode ever created under p.                            or ZooKeeper detects that a clients is faulty. Within a ses-
   ZooKeeper implements watches to allow clients to               sion, a client observes a succession of state changes that
receive timely notifications of changes without requir-           reflect the execution of its operations. Sessions enable a
ing polling. When a client issues a read operation                client to move transparently from one server to another
with a watch flag set, the operation completes as nor-            within a ZooKeeper ensemble, and hence persist across
mal except that the server promises to notify the client          ZooKeeper servers.
when the information returned has changed. Watches
are one-time triggers associated with a session; they
are unregistered once triggered or the session closes.            2.2    Client API
Watches indicate that a change has happened, but do               We present below a relevant subset of the ZooKeeper
not provide the change. For example, if a client is-              API, and discuss the semantics of each request.
sues a getData(‘‘/foo’’, true) before “/foo”                      create(path, data, flags): Creates a znode
is changed twice, the client will get one watch event                  with path name path, stores data[] in it, and
telling the client that data for “/foo” has changed. Ses-              returns the name of the new znode. flags en-
sion events, such as connection loss events, are also sent             ables a client to select the type of znode: regular,
to watch callbacks so that clients know that watch events              ephemeral, and set the sequential flag;
may be delayed.                                                   delete(path, version): Deletes the znode
                                                                       path if that znode is at the expected version;
Data model. The data model of ZooKeeper is essen-                 exists(path, watch): Returns true if the znode
tially a file system with a simplified API and only full               with path name path exists, and returns false oth-
data reads and writes, or a key/value table with hierar-               erwise. The watch flag enables a client to set a


                                                              3
      watch on the znode;                                          client to have multiple outstanding operations, and con-
getData(path, watch): Returns the data and                         sequently we can choose to guarantee no specific order
      meta-data, such as version information, associated           for outstanding operations of the same client or to guar-
      with the znode. The watch flag works in the same             antee FIFO order. We choose the latter for our property.
      way as it does for exists(), except that Zoo-                It is important to observe that all results that hold for
      Keeper does not set the watch if the znode does not          linearizable objects also hold for A-linearizable objects
      exist;                                                       because a system that satisfies A-linearizability also sat-
setData(path, data, version): Writes                               isfies linearizability. Because only update requests are A-
      data[] to znode path if the version number is                linearizable, ZooKeeper processes read requests locally
      the current version of the znode;                            at each replica. This allows the service to scale linearly
getChildren(path, watch): Returns the set of                       as servers are added to the system.
      names of the children of a znode;                               To see how these two guarantees interact, consider the
sync(path): Waits for all updates pending at the start             following scenario. A system comprising a number of
      of the operation to propagate to the server that the         processes elects a leader to command worker processes.
      client is connected to. The path is currently ignored.       When a new leader takes charge of the system, it must
   All methods have both a synchronous and an asyn-                change a large number of configuration parameters and
chronous version available through the API. An applica-            notify the other processes once it finishes. We then have
tion uses the synchronous API when it needs to execute             two important requirements:
a single ZooKeeper operation and it has no concurrent                 • As the new leader starts making changes, we do not
tasks to execute, so it makes the necessary ZooKeeper                    want other processes to start using the configuration
call and blocks. The asynchronous API, however, en-                      that is being changed;
ables an application to have both multiple outstanding                • If the new leader dies before the configuration has
ZooKeeper operations and other tasks executed in par-                    been fully updated, we do not want the processes to
allel. The ZooKeeper client guarantees that the corre-                   use this partial configuration.
sponding callbacks for each operation are invoked in or-              Observe that distributed locks, such as the locks pro-
der.                                                               vided by Chubby, would help with the first requirement
   Note that ZooKeeper does not use handles to access              but are insufficient for the second. With ZooKeeper,
znodes. Each request instead includes the full path of             the new leader can designate a path as the ready znode;
the znode being operated on. Not only does this choice             other processes will only use the configuration when that
simplifies the API (no open() or close() methods),                 znode exists. The new leader makes the configuration
but it also eliminates extra state that the server would           change by deleting ready, updating the various configu-
need to maintain.                                                  ration znodes, and creating ready. All of these changes
   Each of the update methods take an expected ver-                can be pipelined and issued asynchronously to quickly
sion number, which enables the implementation of con-              update the configuration state. Although the latency of a
ditional updates. If the actual version number of the zn-          change operation is of the order of 2 milliseconds, a new
ode does not match the expected version number the up-             leader that must update 5000 different znodes will take
date fails with an unexpected version error. If the version        10 seconds if the requests are issued one after the other;
number is −1, it does not perform version checking.                by issuing the requests asynchronously the requests will
                                                                   take less than a second. Because of the ordering guaran-
2.3    ZooKeeper guarantees                                        tees, if a process sees the ready znode, it must also see
                                                                   all the configuration changes made by the new leader. If
ZooKeeper has two basic ordering guarantees:                       the new leader dies before the ready znode is created, the
Linearizable writes: all requests that update the state            other processes know that the configuration has not been
      of ZooKeeper are serializable and respect prece-             finalized and do not use it.
      dence;                                                          The above scheme still has a problem: what happens
FIFO client order: all requests from a given client are            if a process sees that ready exists before the new leader
      executed in the order that they were sent by the             starts to make a change and then starts reading the con-
      client.                                                      figuration while the change is in progress. This problem
   Note that our definition of linearizability is different        is solved by the ordering guarantee for the notifications:
from the one originally proposed by Herlihy [15], and              if a client is watching for a change, the client will see
we call it A-linearizability (asynchronous linearizabil-           the notification event before it sees the new state of the
ity). In the original definition of linearizability by Her-        system after the change is made. Consequently, if the
lihy, a client is only able to have one outstanding opera-         process that reads the ready znode requests to be notified
tion at a time (a client is one thread). In ours, we allow a       of changes to that znode, it will see a notification inform-

                                                               4
ing the client of the change before it can read any of the        the most recent information. For example, if a process
new configuration.                                                watching zc is notified of a change to zc and before it
   Another problem can arise when clients have their own          can issue a read for zc there are three more changes to
communication channels in addition to ZooKeeper. For              zc , the process does not receive three more notification
example, consider two clients A and B that have a shared          events. This does not affect the behavior of the process,
configuration in ZooKeeper and communicate through a              since those three events would have simply notified the
shared communication channel. If A changes the shared             process of something it already knows: the information
configuration in ZooKeeper and tells B of the change              it has for zc is stale.
through the shared communication channel, B would ex-
pect to see the change when it re-reads the configuration.
If B’s ZooKeeper replica is slightly behind A’s, it may
                                                                  Rendezvous Sometimes in distributed systems, it is
not see the new configuration. Using the above guar-
                                                                  not always clear a priori what the final system config-
antees B can make sure that it sees the most up-to-date
                                                                  uration will look like. For example, a client may want to
information by issuing a write before re-reading the con-
                                                                  start a master process and several worker processes, but
figuration. To handle this scenario more efficiently Zoo-
                                                                  the starting processes is done by a scheduler, so the client
Keeper provides the sync request: when followed by
                                                                  does not know ahead of time information such as ad-
a read, constitutes a slow read. sync causes a server
                                                                  dresses and ports that it can give the worker processes to
to apply all pending write requests before processing the
                                                                  connect to the master. We handle this scenario with Zoo-
read without the overhead of a full write. This primitive
                                                                  Keeper using a rendezvous znode, zr , which is an node
is similar in idea to the flush primitive of ISIS [5].
                                                                  created by the client. The client passes the full pathname
   ZooKeeper also has the following two liveness and
                                                                  of zr as a startup parameter of the master and worker
durability guarantees: if a majority of ZooKeeper servers
                                                                  processes. When the master starts it fills in zr with in-
are active and communicating the service will be avail-
                                                                  formation about addresses and ports it is using. When
able; and if the ZooKeeper service responds successfully
                                                                  workers start, they read zr with watch set to true. If zr
to a change request, that change persists across any num-
                                                                  has not been filled in yet, the worker waits to be notified
ber of failures as long as a quorum of servers is eventu-
                                                                  when zr is updated. If zr is an ephemeral node, master
ally able to recover.
                                                                  and worker processes can watch for zr to be deleted and
                                                                  clean themselves up when the client ends.
2.4    Examples of primitives
In this section, we show how to use the ZooKeeper API
to implement more powerful primitives. The ZooKeeper              Group Membership We take advantage of ephemeral
service knows nothing about these more powerful primi-            nodes to implement group membership. Specifically, we
tives since they are entirely implemented at the client us-       use the fact that ephemeral nodes allow us to see the state
ing the ZooKeeper client API. Some common primitives              of the session that created the node. We start by designat-
such as group membership and configuration manage-                ing a znode, zg to represent the group. When a process
ment are also wait-free. For others, such as rendezvous,          member of the group starts, it creates an ephemeral child
clients need to wait for an event. Even though ZooKeeper          znode under zg . If each process has a unique name or
is wait-free, we can implement efficient blocking primi-          identifier, then that name is used as the name of the child
tives with ZooKeeper. ZooKeeper’s ordering guarantees             znode; otherwise, the process creates the znode with the
allow efficient reasoning about system state, and watches         SEQUENTIAL flag to obtain a unique name assignment.
allow for efficient waiting.                                      Processes may put process information in the data of the
                                                                  child znode, addresses and ports used by the process, for
                                                                  example.
Configuration Management ZooKeeper can be used
to implement dynamic configuration in a distributed ap-              After the child znode is created under zg the process
plication. In its simplest form configuration is stored in        starts normally. It does not need to do anything else. If
a znode, zc . Processes start up with the full pathname           the process fails or ends, the znode that represents it un-
of zc . Starting processes obtain their configuration by          der zg is automatically removed.
reading zc with the watch flag set to true. If the config-           Processes can obtain group information by simply list-
uration in zc is ever updated, the processes are notified         ing the children of zg . If a process wants to monitor
and read the new configuration, again setting the watch           changes in group membership, the process can set the
flag to true.                                                     watch flag to true and refresh the group information (al-
   Note that in this scheme, as in most others that use           ways setting the watch flag to true) when change notifi-
watches, watches are used to make sure that a process has         cations are received.

                                                              5
Simple Locks Although ZooKeeper is not a lock ser-                EPHEMERAL flag on creation, processes that crash will
vice, it can be used to implement locks. Applications             automatically cleanup any lock requests or release any
using ZooKeeper usually use synchronization primitives            locks that they may have.
tailored to their needs, such as those shown above. Here             In summary, this locking scheme has the following ad-
we show how to implement locks with ZooKeeper to                  vantages:
show that it can implement a wide variety of general syn-           1. The removal of a znode only causes one client to
chronization primitives.                                                wake up, since each znode is watched by exactly
   The simplest lock implementation uses “lock files”.                  one other client, so we do not have the herd effect;
The lock is represented by a znode. To acquire a lock,              2. There is no polling or timeouts;
a client tries to create the designated znode with the              3. Because of the way we have implemented locking,
EPHEMERAL flag. If the create succeeds, the client                      we can see by browsing the ZooKeeper data the
holds the lock. Otherwise, the client can read the zn-                  amount of lock contention, break locks, and debug
ode with the watch flag set to be notified if the current               locking problems.
leader dies. A client releases the lock when it dies or ex-
plicitly deletes the znode. Other clients that are waiting        Read/Write Locks To implement read/write locks we
for a lock try again to acquire a lock once they observe          change the lock procedure slightly and have separate
the znode being deleted.                                          read lock and write lock procedures. The unlock pro-
   While this simple locking protocol works, it does have         cedure is the same as the global lock case.
some problems. First, it suffers from the herd effect. If
there are many clients waiting to acquire a lock, they will       Write Lock
all vie for the lock when it is released even though only         1 n = create(l + “/write-”, EPHEMERAL|SEQUENTIAL)
                                                                  2 C = getChildren(l, false)
one client can acquire the lock. Second, it only imple-           3 if n is lowest znode in C, exit
ments exclusive locking. The following two primitives             4 p = znode in C ordered just before n
                                                                  5 if exists(p, true) wait for event
show how both of these problems can be overcome.                  6 goto 2

                                                                  Read Lock
Simple Locks without Herd Effect We define a lock                 1 n = create(l + “/read-”, EPHEMERAL|SEQUENTIAL)
                                                                  2 C = getChildren(l, false)
znode l to implement such locks. Intuitively we line up           3 if no write znodes lower than n in C, exit
all the clients requesting the lock and each client obtains       4 p = write znode in C ordered just before n
                                                                  5 if exists(p, true) wait for event
the lock in order of request arrival. Thus, clients wishing       6 goto 3
to obtain the lock do the following:
                                                                     This lock procedure varies slightly from the previous
Lock                                                              locks. Write locks differ only in naming. Since read
1 n = create(l + “/lock-”, EPHEMERAL|SEQUENTIAL)
2 C = getChildren(l, false)                                       locks may be shared, lines 3 and 4 vary slightly because
3 if n is lowest znode in C, exit                                 only earlier write lock znodes prevent the client from ob-
4 p = znode in C ordered just before n
5 if exists(p, true) wait for watch event                         taining a read lock. It may appear that we have a “herd
6 goto 2                                                          effect” when there are several clients waiting for a read
Unlock                                                            lock and get notified when the “write-” znode with the
1 delete(n)                                                       lower sequence number is deleted; in fact, this is a de-
   The use of the SEQUENTIAL flag in line 1 of Lock               sired behavior, all those read clients should be released
orders the client’s attempt to acquire the lock with re-          since they may now have the lock.
spect to all other attempts. If the client’s znode has the
lowest sequence number at line 3, the client holds the            Double Barrier Double barriers enable clients to syn-
lock. Otherwise, the client waits for deletion of the zn-         chronize the beginning and the end of a computation.
ode that either has the lock or will receive the lock be-         When enough processes, defined by the barrier thresh-
fore this client’s znode. By only watching the znode              old, have joined the barrier, processes start their compu-
that precedes the client’s znode, we avoid the herd effect        tation and leave the barrier once they have finished. We
by only waking up one process when a lock is released             represent a barrier in ZooKeeper with a znode, referred
or a lock request is abandoned. Once the znode being              to as b. Every process p registers with b – by creating
watched by the client goes away, the client must check            a znode as a child of b – on entry, and unregisters – re-
if it now holds the lock. (The previous lock request may          moves the child – when it is ready to leave. Processes
have been abandoned and there is a znode with a lower             can enter the barrier when the number of child znodes
sequence number still waiting for or holding the lock.)           of b exceeds the barrier threshold. Processes can leave
   Releasing a lock is as simple as deleting the zn-              the barrier when all of the processes have removed their
ode n that represents the lock request. By using the              children. We use watches to efficiently wait for enter and


                                                              6
exit conditions to be satisfied. To enter, processes watch                                  Katta Katta [17] is a distributed indexer that uses Zoo-
for the existence of a ready child of b that will be cre-                                   Keeper for coordination, and it is an example of a non-
ated by the process that causes the number of children to                                   Yahoo! application. Katta divides the work of indexing
exceed the barrier threshold. To leave, processes watch                                     using shards. A master server assigns shards to slaves
for a particular child to disappear and only check the exit                                 and tracks progress. Slaves can fail, so the master must
condition once that znode has been removed.                                                 redistribute load as slaves come and go. The master can
                                                                                            also fail, so other servers must be ready to take over in
                                                                                            case of failure. Katta uses ZooKeeper to track the status
3   ZooKeeper Applications
                                                                                            of slave servers and the master (group membership),
We now describe some applications that use ZooKeeper,                                       and to handle master failover (leader election). Katta
and explain briefly how they use it. We show the primi-                                     also uses ZooKeeper to track and propagate the assign-
tives of each example in bold.                                                              ments of shards to slaves (configuration management).

The Fetching Service Crawling is an important part of                                       Yahoo! Message Broker Yahoo! Message Broker
a search engine, and Yahoo! crawls billions of Web doc-                                     (YMB) is a distributed publish-subscribe system. The
uments. The Fetching Service (FS) is part of the Yahoo!                                     system manages thousands of topics that clients can pub-
crawler and it is currently in production. Essentially, it                                  lish messages to and receive messages from. The topics
has master processes that command page-fetching pro-                                        are distributed among a set of servers to provide scala-
cesses. The master provides the fetchers with configura-                                    bility. Each topic is replicated using a primary-backup
tion, and the fetchers write back informing of their status                                 scheme that ensures messages are replicated to two ma-
and health. The main advantages of using ZooKeeper                                          chines to ensure reliable message delivery. The servers
for FS are recovering from failures of masters, guaran-                                     that makeup YMB use a shared-nothing distributed ar-
teeing availability despite failures, and decoupling the                                    chitecture which makes coordination essential for correct
clients from the servers, allowing them to direct their re-                                 operation. YMB uses ZooKeeper to manage the distribu-
quest to healthy servers by just reading their status from                                  tion of topics (configuration metadata), deal with fail-
ZooKeeper. Thus, FS uses ZooKeeper mainly to man-                                           ures of machines in the system (failure detection and
age configuration metadata, although it also uses Zoo-                                      group membership), and control system operation.
Keeper to elect masters (leader election).
                                                                                                                                       broker domain

                             2000
                                                                            read
                                                                            write
                                                                                                  shutdown       nodes            migration_prohibited       topics     broker_disabled

                             1500




      Number of operations
                                                                                             <hostname> <hostname>     ..... < h o s t n a m e >   <topic>   <topic>    .... < t o p i c >
                             1000
                                                                                                            load
                                                                                                         # of topics
                                                                                                                                                      primary         backup
                             500
                                                                                                                                                    hostname



                               0
                                    0h   6h   12h 18h 24h 30h 36h 42h 48h 54h 60h 66h       Figure 3: The layout of Yahoo! Message Broker (YMB)
                                                         Time in seconds
                                                                                            structures in ZooKeeper
Figure 2: Workload for one ZK server with the Fetching
Service. Each point represents a one-second sample.
                                                                                               Figure 3 shows part of the znode data layout for YMB.
   Figure 2 shows the read and write traffic for a Zoo-                                     Each broker domain has a znode called nodes that has
Keeper server used by FS through a period of three days.                                    an ephemeral znode for each of the active servers that
To generate this graph, we count the number of opera-                                       compose the YMB service. Each YMB server creates
tions for every second during the period, and each point                                    an ephemeral znode under nodes with load and sta-
corresponds to the number of operations in that second.                                     tus information providing both group membership and
We observe that the read traffic is much higher compared                                    status information through ZooKeeper. Nodes such as
to the write traffic. During periods in which the rate is                                   shutdown and migration prohibited are mon-
higher than 1, 000 operations per second, the read:write                                    itored by all of the servers that make up the service and
ratio varies between 10:1 and 100:1. The read operations                                    allow centralized control of YMB. The topics direc-
in this workload are getData(), getChildren(),                                              tory has a child znode for each topic managed by YMB.
and exists(), in increasing order of prevalence.                                            These topic znodes have child znodes that indicate the


                                                                                        7
primary and backup server for each topic along with the                           message proposals consisting of state changes from the
subscribers of that topic. The primary and backup                                 leader and agree upon state changes.
server znodes not only allow servers to discover the
servers in charge of a topic, but they also manage leader
election and server crashes.
                                                                                  4.1   Request Processor

                                ZooKeeper Service
                                                                                  Since the messaging layer is atomic, we guarantee that
                                                                                  the local replicas never diverge, although at any point in
                   Request
 Write
 Request          Processor
                              txn                                  Response
                                                                                  time some servers may have applied more transactions
                                                      Replicated
                                                      Database                    than others. Unlike the requests sent from clients, the
                                     Atomic
                                    Broadcast   txn                               transactions are idempotent. When the leader receives
                                                                                  a write request, it calculates what the state of the sys-
                                                      Read
                                                                                  tem will be when the write is applied and transforms it
                                                      Request
                                                                                  into a transaction that captures this new state. The fu-
                                                                                  ture state must be calculated because there may be out-
    Figure 4: The components of the ZooKeeper service.
                                                                                  standing transactions that have not yet been applied to
                                                                                  the database. For example, if a client does a conditional
                                                                                  setData and the version number in the request matches
4     ZooKeeper Implementation                                                    the future version number of the znode being updated,
                                                                                  the service generates a setDataTXN that contains the
ZooKeeper provides high availability by replicating the                           new data, the new version number, and updated time
ZooKeeper data on each server that composes the ser-                              stamps. If an error occurs, such as mismatched version
vice. We assume that servers fail by crashing, and such                           numbers or the znode to be updated does not exist, an
faulty servers may later recover. Figure 4 shows the high-                        errorTXN is generated instead.
level components of the ZooKeeper service. Upon re-
ceiving a request, a server prepares it for execution (re-
quest processor). If such a request requires coordina-                            4.2   Atomic Broadcast
tion among the servers (write requests), then they use an                         All requests that update ZooKeeper state are forwarded
agreement protocol (an implementation of atomic broad-                            to the leader. The leader executes the request and
cast), and finally servers commit changes to the Zoo-                             broadcasts the change to the ZooKeeper state through
Keeper database fully replicated across all servers of the                        Zab [24], an atomic broadcast protocol. The server that
ensemble. In the case of read requests, a server simply                           receives the client request responds to the client when it
reads the state of the local database and generates a re-                         delivers the corresponding state change. Zab uses by de-
sponse to the request.                                                            fault simple majority quorums to decide on a proposal,
   The replicated database is an in-memory database con-                          so Zab and thus ZooKeeper can only work if a majority
taining the entire data tree. Each znode in the tree stores a                     of servers are correct (i.e., with 2f + 1 server we can
maximum of 1MB of data by default, but this maximum                               tolerate f failures).
value is a configuration parameter that can be changed in                            To achieve high throughput, ZooKeeper tries to keep
specific cases. For recoverability, we efficiently log up-                        the request processing pipeline full. It may have thou-
dates to disk, and we force writes to be on the disk media                        sands of requests in different parts of the processing
before they are applied to the in-memory database. In                             pipeline. Because state changes depend on the appli-
fact, as Chubby [8], we keep a replay log (a write-ahead                          cation of previous state changes, Zab provides stronger
log, in our case) of committed operations and generate                            order guarantees than regular atomic broadcast. More
periodic snapshots of the in-memory database.                                     specifically, Zab guarantees that changes broadcast by a
   Every ZooKeeper server services clients. Clients con-                          leader are delivered in the order they were sent and all
nect to exactly one server to submit its requests. As we                          changes from previous leaders are delivered to an estab-
noted earlier, read requests are serviced from the local                          lished leader before it broadcasts its own changes.
replica of each server database. Requests that change the                            There are a few implementation details that simplify
state of the service, write requests, are processed by an                         our implementation and give us excellent performance.
agreement protocol.                                                               We use TCP for our transport so message order is main-
   As part of the agreement protocol write requests are                           tained by the network, which allows us to simplify our
forwarded to a single server, called the leader1 . The                            implementation. We use the leader chosen by Zab as
rest of the ZooKeeper servers, called followers, receive                          the ZooKeeper leader, so that the same process that cre-
    1 Details of leaders and followers, as part of the agreement protocol,        ates transactions also proposes them. We use the log to
are out of the scope of this paper.                                               keep track of proposals as the write-ahead log for the in-


                                                                              8
memory database, so that we do not have to write mes-              sponds to that update. Servers process writes in order
sages twice to disk.                                               and do not process other writes or reads concurrently.
   During normal operation Zab does deliver all mes-               This ensures strict succession of notifications. Note that
sages in order and exactly once, but since Zab does not            servers handle notifications locally. Only the server that
persistently record the id of every message delivered,             a client is connected to tracks and triggers notifications
Zab may redeliver a message during recovery. Because               for that client.
we use idempotent transactions, multiple delivery is ac-              Read requests are handled locally at each server. Each
ceptable as long as they are delivered in order. In fact,          read request is processed and tagged with a zxid that cor-
ZooKeeper requires Zab to redeliver at least all messages          responds to the last transaction seen by the server. This
that were delivered after the start of the last snapshot.          zxid defines the partial order of the read requests with re-
                                                                   spect to the write requests. By processing reads locally,
4.3    Replicated Database                                         we obtain excellent read performance because it is just an
                                                                   in-memory operation on the local server, and there is no
Each replica has a copy in memory of the ZooKeeper                 disk activity or agreement protocol to run. This design
state. When a ZooKeeper server recovers from a crash, it           choice is key to achieving our goal of excellent perfor-
needs to recover this internal state. Replaying all deliv-         mance with read-dominant workloads.
ered messages to recover state would take prohibitively               One drawback of using fast reads is not guaranteeing
long after running the server for a while, so ZooKeeper            precedence order for read operations. That is, a read op-
uses periodic snapshots and only requires redelivery of            eration may return a stale value, even though a more
messages since the start of the snapshot. We call Zoo-             recent update to the same znode has been committed.
Keeper snapshots fuzzy snapshots since we do not lock              Not all of our applications require precedence order, but
the ZooKeeper state to take the snapshot; instead, we do           for applications that do require it, we have implemented
a depth first scan of the tree atomically reading each zn-         sync. This primitive executes asynchronously and is
ode’s data and meta-data and writing them to disk. Since           ordered by the leader after all pending writes to its lo-
the resulting fuzzy snapshot may have applied some sub-            cal replica. To guarantee that a given read operation re-
set of the state changes delivered during the generation of        turns the latest updated value, a client calls sync fol-
the snapshot, the result may not correspond to the state           lowed by the read operation. The FIFO order guarantee
of ZooKeeper at any point in time. However, since state            of client operations together with the global guarantee of
changes are idempotent, we can apply them twice as long            sync enables the result of the read operation to reflect
as we apply the state changes in order.                            any changes that happened before the sync was issued.
   For example, assume that in a ZooKeeper data tree two           In our implementation, we do not need to atomically
nodes /foo and /goo have values f1 and g1 respec-
                                                                   broadcast sync as we use a leader-based algorithm, and
tively and both are at version 1 when the fuzzy snap-
shot begins, and the following stream of state changes             we simply place the sync operation at the end of the
arrive having the form htransactionType, path,                     queue of requests between the leader and the server ex-
value, new-versioni:                                               ecuting the call to sync. In order for this to work, the
                                                                   follower must be sure that the leader is still the leader.
hSetDataTXN, /foo, f2, 2i                                          If there are pending transactions that commit, then the
hSetDataTXN, /goo, g2, 2i                                          server does not suspect the leader. If the pending queue
hSetDataTXN, /foo, f3, 3i
                                                                   is empty, the leader needs to issue a null transaction to
                                                                   commit and orders the sync after that transaction. This
   After processing these state changes, /foo and /goo             has the nice property that when the leader is under load,
have values f3 and g2 with versions 3 and 2 respec-                no extra broadcast traffic is generated. In our implemen-
tively. However, the fuzzy snapshot may have recorded              tation, timeouts are set such that leaders realize they are
that /foo and /goo have values f3 and g1 with ver-                 not leaders before followers abandon them, so we do not
sions 3 and 1 respectively, which was not a valid state            issue the null transaction.
of the ZooKeeper data tree. If the server crashes and                 ZooKeeper servers process requests from clients in
recovers with this snapshot and Zab redelivers the state           FIFO order. Responses include the zxid that the response
changes, the resulting state corresponds to the state of the       is relative to. Even heartbeat messages during intervals
service before the crash.                                          of no activity include the last zxid seen by the server that
                                                                   the client is connected to. If the client connects to a new
                                                                   server, that new server ensures that its view of the Zoo-
4.4    Client-Server Interactions
                                                                   Keeper data is at least as recent as the view of the client
When a server processes a write request, it also sends out         by checking the last zxid of the client against its last zxid.
and clears notifications relative to any watch that corre-         If the client has a more recent view than the server, the


                                                               9
server does not reestablish the session with the client un-             go to the leader, but does not get broadcast.) Clients
til the server has caught up. The client is guaranteed to               send counts of the number of completed operations ev-
be able to find another server that has a recent view of the            ery 300ms and we sample every 6s. To prevent memory
system since the client only sees changes that have been                overflows, servers throttle the number of concurrent re-
replicated to a majority of the ZooKeeper servers. This                 quests in the system. ZooKeeper uses request throttling
behavior is important to guarantee durability.                          to keep servers from being overwhelmed. For these ex-
    To detect client session failures, ZooKeeper uses time-             periments, we configured the ZooKeeper servers to have
outs. The leader determines that there has been a failure               a maximum of 2, 000 total requests in process.
if no other server receives anything from a client ses-
                                                                                                                               Throughput of saturated system
sion within the session timeout. If the client sends re-                                             90000
quests frequently enough, then there is no need to send                                                           3 servers
                                                                                                     80000        5 servers
                                                                                                                  7 servers
any other message. Otherwise, the client sends heartbeat                                             70000        9 servers
                                                                                                                 13 servers




                                                                             Operations per second
messages during periods of low activity. If the client                                               60000

cannot communicate with a server to send a request or                                                50000

heartbeat, it connects to a different ZooKeeper server to                                            40000

re-establish its session. To prevent the session from tim-                                           30000

                                                                                                     20000
ing out, the ZooKeeper client library sends a heartbeat
                                                                                                     10000
after the session has been idle for s/3 ms and switch to a
                                                                                                        0
new server if it has not heard from a server for 2s/3 ms,                                                    0           20          40             60          80   100
                                                                                                                                Percentage of read requests
where s is the session timeout in milliseconds.

                                                                        Figure 5: The throughput performance of a saturated sys-
5     Evaluation                                                        tem as the ratio of reads to writes vary.

We performed all of our evaluation on a cluster of 50
servers. Each server has one Xeon dual-core 2.1GHz                                                     Servers                100% Reads              0% Reads
                                                                                                         13                         460k                    8k
processor, 4GB of RAM, gigabit ethernet, and two SATA
                                                                                                          9                         296k                   12k
hard drives. We split the following discussion into two
                                                                                                          7                         257k                   14k
parts: throughput and latency of requests.                                                                5                         165k                   18k
                                                                                                          3                          87k                   21k
5.1     Throughput                                                      Table 1: The throughput performance of the extremes of
To evaluate our system, we benchmark throughput when                    a saturated system.
the system is saturated and the changes in throughput
                                                                           In Figure 5, we show throughput as we vary the ratio
for various injected failures. We varied the number of
                                                                        of read to write requests, and each curve corresponds to
servers that make up the ZooKeeper service, but always
                                                                        a different number of servers providing the ZooKeeper
kept the number of clients the same. To simulate a large
                                                                        service. Table 1 shows the numbers at the extremes of
number of clients, we used 35 machines to simulate 250
                                                                        the read loads. Read throughput is higher than write
simultaneous clients.
                                                                        throughput because reads do not use atomic broadcast.
   We have a Java implementation of the ZooKeeper
                                                                        The graph also shows that the number of servers also has
server, and both Java and C clients2 . For these experi-
                                                                        a negative impact on the performance of the broadcast
ments, we used the Java server configured to log to one
                                                                        protocol. From these graphs, we observe that the number
dedicated disk and take snapshots on another. Our bench-
                                                                        of servers in the system does not only impact the num-
mark client uses the asynchronous Java client API, and
                                                                        ber of failures that the service can handle, but also the
each client has at least 100 requests outstanding. Each
                                                                        workload the service can handle. Note that the curve for
request consists of a read or write of 1K of data. We
                                                                        three servers crosses the others around 60%. This situ-
do not show benchmarks for other operations since the
                                                                        ation is not exclusive of the three-server configuration,
performance of all the operations that modify state are
                                                                        and happens for all configurations due to the parallelism
approximately the same, and the performance of non-
                                                                        local reads enable. It is not observable for other config-
state modifying operations, excluding sync, are approx-
                                                                        urations in the figure, however, because we have capped
imately the same. (The performance of sync approxi-
                                                                        the maximum y-axis throughput for readability.
mates that of a light-weight write, since the request must
                                                                           There are two reasons for write requests taking longer
    2 The implementation is publicly available at http://hadoop.        than read requests. First, write requests must go through
apache.org/zookeeper.                                                   atomic broadcast, which requires some extra processing


                                                                   10
and adds latency to requests. The other reason for longer                                                                                                                  Atomic Broadcast Throughput
                                                                                                                                                  70000
processing of write requests is that servers must ensure
                                                                                                                                                  60000
that transactions are logged to non-volatile store before
sending acknowledgments back to the leader. In prin-                                                                                              50000




                                                                                                                            Requests per second
ciple, this requirement is excessive, but for our produc-                                                                                         40000

tion systems we trade performance for reliability since                                                                                           30000

ZooKeeper constitutes application ground truth. We use                                                                                            20000
more servers to tolerate more faults. We increase write
                                                                                                                                                  10000
throughput by partitioning the ZooKeeper data into mul-
                                                                                                                                                      0
tiple ZooKeeper ensembles. This performance trade off                                                                                                     2       4          6            8          10          12       14
                                                                                                                                                                                 Size of ensemble
between replication and partitioning has been previously
observed by Gray et al. [12].
                                                                                                               Figure 7: Average throughput of the atomic broadcast
                                          Throughput of saturated system (all requests to leader)
                                                                                                               component in isolation. Error bars denote the minimum
                              90000
                                                                                      3 servers
                                                                                                               and maximum values.
                                                                                      5 servers
                              80000                                                   7 servers
                                                                                      9 servers
                              70000                                                  13 servers
                                                                                                               versions all require CPU. The contention for CPU low-

      Operations per second
                              60000

                              50000                                                                            ers ZooKeeper throughput to substantially less than the
                              40000                                                                            atomic broadcast component in isolation. Because Zoo-
                              30000                                                                            Keeper is a critical production component, up to now our
                              20000                                                                            development focus for ZooKeeper has been correctness
                              10000                                                                            and robustness. There are plenty of opportunities for im-
                                 0                                                                             proving performance significantly by eliminating things
                                      0        20           40             60             80        100
                                                       Percentage of read requests                             like extra copies, multiple serializations of the same ob-
                                                                                                               ject, more efficient internal data structures, etc.
Figure 6: Throughput of a saturated system, varying the
                                                                                                                                                                             Time series with failures
ratio of reads to writes when all clients connect to the                                                                                          70000
                                                                                                                                                                                                          Throughput
leader.                                                                                                                                           60000


                                                                                                                                                  50000




                                                                                                                    Operations per second
   ZooKeeper is able to achieve such high throughput by
                                                                                                                                                  40000                                              4c
distributing load across the servers that makeup the ser-                                                                                                                                                             6
                                                                                                                                                  30000                2                        4b
vice. We can distribute the load because of our relaxed                                                                                                       1                           4a
consistency guarantees. Chubby clients instead direct all                                                                                         20000

requests to the leader. Figure 6 shows what happens if                                                                                            10000
we do not take advantage of this relaxation and forced                                                                                                                              3                        5
                                                                                                                                                     0
the clients to only connect to the leader. As expected the                                                                                                0       50        100       150           200          250      300
                                                                                                                                                                           Seconds since start of series
throughput is much lower for read-dominant workloads,
but even for write-dominant workloads the throughput is
                                                                                                                                                    Figure 8: Throughput upon failures.
lower. The extra CPU and network load caused by ser-
vicing clients impacts the ability of the leader to coor-                                                         To show the behavior of the system over time as fail-
dinate the broadcast of the proposals, which in turn ad-                                                       ures are injected we ran a ZooKeeper service made up
versely impacts the overall write performance.                                                                 of 5 machines. We ran the same saturation benchmark
   The atomic broadcast protocol does most of the work                                                         as before, but this time we kept the write percentage at
of the system and thus limits the performance of Zoo-                                                          a constant 30%, which is a conservative ratio of our ex-
Keeper more than any other component. Figure 7 shows                                                           pected workloads. Periodically we killed some of the
the throughput of the atomic broadcast component. To                                                           server processes. Figure 8 shows the system throughput
benchmark its performance we simulate clients by gen-                                                          as it changes over time. The events marked in the figure
erating the transactions directly at the leader, so there is                                                   are the following:
no client connections or client requests and replies. At                                                         1. Failure and recovery of a follower;
maximum throughput the atomic broadcast component                                                                2. Failure and recovery of a different follower;
becomes CPU bound. In theory the performance of Fig-                                                             3. Failure of the leader;
ure 7 would match the performance of ZooKeeper with                                                              4. Failure of two followers (a, b) in the first two marks,
100% writes. However, the ZooKeeper client commu-                                                                    and recovery at the third mark (c);
nication, ACL checks, and request to transaction con-                                                            5. Failure of the leader.


                                                                                                          11
  6. Recovery of the leader.                                                                          # of clients
                                                                              # of barriers     50       100       200
   There are a few important observations from this
                                                                                   200         9.4       19.8      41.0
graph. First, if followers fail and recover quickly, then
                                                                                   400         16.4      34.1      62.0
ZooKeeper is able to sustain a high throughput despite                             800         28.9      55.9     112.1
the failure. The failure of a single follower does not pre-                       1600         54.0     102.7 234.4
vent servers from forming a quorum, and only reduces
throughput roughly by the share of read requests that the
server was processing before failing. Second, our leader           Table 3: Barrier experiment with time in seconds. Each
election algorithm is able to recover fast enough to pre-          point is the average of the time for each client to finish
vent throughput from dropping substantially. In our ob-            over five runs.
servations, ZooKeeper takes less than 200ms to elect a             5.3    Performance of barriers
new leader. Thus, although servers stop serving requests
for a fraction of second, we do not observe a throughput           In this experiment, we execute a number of barriers se-
of zero due to our sampling period, which is on the order          quentially to assess the performance of primitives imple-
of seconds. Third, even if followers take more time to re-         mented with ZooKeeper. For a given number of barriers
cover, ZooKeeper is able to raise throughput again once            b, each client first enters all b barriers, and then it leaves
they start processing requests. One reason that we do              all b barriers in succession. As we use the double-barrier
not recover to the full throughput level after events 1, 2,        algorithm of Section 2.4, a client first waits for all other
and 4 is that the clients only switch followers when their         clients to execute the enter() procedure before mov-
connection to the follower is broken. Thus, after event 4          ing to next call (similarly for leave()).
the clients do not redistribute themselves until the leader           We report the results of our experiments in Table 3.
fails at events 3 and 5. In practice such imbalances work          In this experiment, we have 50, 100, and 200 clients
themselves out over time as clients come and go.                   entering a number b of barriers in succession, b ∈
                                                                   {200, 400, 800, 1600}. Although an application can have
                                                                   thousands of ZooKeeper clients, quite often a much
                                                                   smaller subset participates in each coordination oper-
5.2    Latency of requests                                         ation as clients are often grouped according to the
                                                                   specifics of the application.
To assess the latency of requests, we created a bench-
                                                                      Two interesting observations from this experiment are
mark modeled after the Chubby benchmark [6]. We cre-
                                                                   that the time to process all barriers increase roughly lin-
ate a worker process that simply sends a create, waits
                                                                   early with the number of barriers, showing that concur-
for it to finish, sends an asynchronous delete of the new
                                                                   rent access to the same part of the data tree did not pro-
node, and then starts the next create. We vary the number
                                                                   duce any unexpected delay, and that latency increases
of workers accordingly, and for each run, we have each
                                                                   proportionally to the number of clients. This is a con-
worker create 50,000 nodes. We calculate the throughput
                                                                   sequence of not saturating the ZooKeeper service. In
by dividing the number of create requests completed by
                                                                   fact, we observe that even with clients proceeding in
the total time it took for all the workers to complete.
                                                                   lock-step, the throughput of barrier operations (enter and
                                                                   leave) is between 1,950 and 3,100 operations per second
                            Number of servers                      in all cases. In ZooKeeper operations, this corresponds
         Workers        3        5       7      9                  to throughput values between 10,700 and 17,000 opera-
            1         776      748     758    711                  tions per second. As in our implementation we have a
           10        2074     1832 1572 1540
                                                                   ratio of reads to writes of 4:1 (80% of read operations),
           20        2740     2336 1934 1890
                                                                   the throughput our benchmark code uses is much lower
      Table 2: Create requests processed per second.               compared to the raw throughput ZooKeeper can achieve
                                                                   (over 40,000 according to Figure 5). This is due to clients
   Table 2 show the results of our benchmark. The cre-             waiting on other clients.
ate requests include 1K of data, rather than 5 bytes in
the Chubby benchmark, to better coincide with our ex-              6     Related work
pected use. Even with these larger requests, the through-
put of ZooKeeper is more than 3 times higher than the              ZooKeeper has the goal of providing a service that mit-
published throughput of Chubby. The throughput of the              igates the problem of coordinating processes in dis-
single ZooKeeper worker benchmark indicates that the               tributed applications. To achieve this goal, its design uses
average request latency is 1.2ms for three servers and             ideas from previous coordination services, fault tolerant
1.4ms for 9 servers.                                               systems, distributed algorithms, and file systems.


                                                              12
   We are not the first to propose a system for the coor-               Boxwood [21] is a system that uses distributed lock
dination of distributed applications. Some early systems             servers. Boxwood provides higher-level abstractions to
propose a distributed lock service for transactional ap-             applications, and it relies upon a distributed lock service
plications [13], and for sharing information in clusters             based on Paxos. Like Boxwood, ZooKeeper is a com-
of computers [19]. More recently, Chubby proposes a                  ponent used to build distributed systems. ZooKeeper,
system to manage advisory locks for distributed appli-               however, has high-performance requirements and is used
cations [6]. Chubby shares several of the goals of Zoo-              more extensively in client applications. ZooKeeper ex-
Keeper. It also has a file-system-like interface, and it uses        poses lower-level primitives that applications use to im-
an agreement protocol to guarantee the consistency of the            plement higher-level primitives.
replicas. However, ZooKeeper is not a lock service. It                  ZooKeeper resembles a small file system, but it only
can be used by clients to implement locks, but there are             provides a small subset of the file system operations
no lock operations in its API. Unlike Chubby, ZooKeeper              and adds functionality not present in most file systems
allows clients to connect to any ZooKeeper server, not               such as ordering guarantees and conditional writes. Zoo-
just the leader. ZooKeeper clients can use their local               Keeper watches, however, are similar in spirit to the
replicas to serve data and manage watches since its con-             cache callbacks of AFS [16].
sistency model is much more relaxed than Chubby. This                   Sinfonia [2] introduces mini-transactions, a new
enables ZooKeeper to provide higher performance than                 paradigm for building scalable distributed systems. Sin-
Chubby, allowing applications to make more extensive                 fonia has been designed to store application data,
use of ZooKeeper.                                                    whereas ZooKeeper stores application metadata. Zoo-
   There have been fault-tolerant systems proposed in                Keeper keeps its state fully replicated and in memory for
the literature with the goal of mitigating the problem of            high performance and consistent latency. Our use of file
building fault-tolerant distributed applications. One early          system like operations and ordering enables functionality
system is ISIS [5]. The ISIS system transforms abstract              similar to mini-transactions. The znode is a convenient
type specifications into fault-tolerant distributed objects,         abstraction upon which we add watches, a functionality
thus making fault-tolerance mechanisms transparent to                missing in Sinfonia. Dynamo [11] allows clients to get
users. Horus [30] and Ensemble [31] are systems that                 and put relatively small (less than 1M) amounts of data in
evolved from ISIS. ZooKeeper embraces the notion of                  a distributed key-value store. Unlike ZooKeeper, the key
virtual synchrony of ISIS. Finally, Totem guarantees total           space in Dynamo is not hierarchal. Dynamo also does
order of message delivery in an architecture that exploits           not provide strong durability and consistency guarantees
hardware broadcasts of local area networks [22]. Zoo-                for writes, but instead resolves conflicts on reads.
Keeper works with a wide variety of network topologies                  DepSpace [4] uses a tuple space to provide a Byzan-
which motivated us to rely on TCP connections between                tine fault-tolerant service. Like ZooKeeper DepSpace
server processes and not assume any special topology or              uses a simple server interface to implement strong syn-
hardware features. We also do not expose any of the en-              chronization primitives at the client. While DepSpace’s
semble communication used internally in ZooKeeper.                   performance is much lower than ZooKeeper, it provides
   One important technique for building fault-tolerant               stronger fault tolerance and confidentiality guarantees.
services is state-machine replication [26], and Paxos [20]
is an algorithm that enables efficient implementations               7   Conclusions
of replicated state-machines for asynchronous systems.
We use an algorithm that shares some of the character-               ZooKeeper takes a wait-free approach to the problem of
istics of Paxos, but that combines transaction logging               coordinating processes in distributed systems, by expos-
needed for consensus with write-ahead logging needed                 ing wait-free objects to clients. We have found Zoo-
for data tree recovery to enable an efficient implementa-            Keeper to be useful for several applications inside and
tion. There have been proposals of protocols for practical           outside Yahoo!. ZooKeeper achieves throughput val-
implementations of Byzantine-tolerant replicated state-              ues of hundreds of thousands of operations per second
machines [7, 10, 18, 1, 28]. ZooKeeper does not assume               for read-dominant workloads by using fast reads with
that servers can be Byzantine, but we do employ mech-                watches, both of which served by local replicas. Al-
anisms such as checksums and sanity checks to catch                  though our consistency guarantees for reads and watches
non-malicious Byzantine faults. Clement et al. dis-                  appear to be weak, we have shown with our use cases that
cuss an approach to make ZooKeeper fully Byzantine                   this combination allows us to implement efficient and
fault-tolerant without modifying the current server code             sophisticated coordination protocols at the client even
base [9]. To date, we have not observed faults in produc-            though reads are not precedence-ordered and the imple-
tion that would have been prevented using a fully Byzan-             mentation of data objects is wait-free. The wait-free
tine fault-tolerant protocol. [29].                                  property has proved to be essential for high performance.

                                                                13
   Although we have described only a few applications,                              SOSP ’07: Proceedings of the 21st ACM symposium on Operat-
there are many others using ZooKeeper. We believe such                              ing systems principles, New York, NY, USA, 2007. ACM Press.
a success is due to its simple interface and the powerful                      [12] J. Gray, P. Helland, P. O’Neil, and D. Shasha. The dangers of
                                                                                    replication and a solution. In Proceedings of SIGMOD ’96, pages
abstractions that one can implement through this inter-                             173–182, New York, NY, USA, 1996. ACM.
face. Further, because of the high-throughput of Zoo-                          [13] A. Hastings. Distributed lock management in a transaction pro-
Keeper, applications can make extensive use of it, not                              cessing environment. In Proceedings of IEEE 9th Symposium on
only course-grained locking.                                                        Reliable Distributed Systems, Oct. 1990.
                                                                               [14] M. Herlihy. Wait-free synchronization. ACM Transactions on
                                                                                    Programming Languages and Systems, 13(1), 1991.
Acknowledgements                                                               [15] M. Herlihy and J. Wing. Linearizability: A correctness condi-
                                                                                    tion for concurrent objects. ACM Transactions on Programming
We would like to thank Andrew Kornev and Runping Qi                                 Languages and Systems, 12(3), July 1990.
for their contributions to ZooKeeper; Zeke Huang and                           [16] J. H. Howard, M. L. Kazar, S. G. Menees, D. A. Nichols,
Mark Marchukov for valuable feedback; Brian Cooper                                  M. Satyanarayanan, R. N. Sidebotham, and M. J. West. Scale
                                                                                    and performance in a distributed file system. ACM Trans. Com-
and Laurence Ramontianu for their early contributions                               put. Syst., 6(1), 1988.
to ZooKeeper; Brian Bershad and Geoff Voelker made                             [17] Katta. Katta - distribute lucene indexes in a grid. http://
important comments on the presentation.                                             katta.wiki.sourceforge.net/, 2008.
                                                                               [18] R. Kotla, L. Alvisi, M. Dahlin, A. Clement, and E. Wong.
                                                                                    Zyzzyva: speculative byzantine fault tolerance. SIGOPS Oper.
References                                                                          Syst. Rev., 41(6):45–58, 2007.
 [1] M. Abd-El-Malek, G. R. Ganger, G. R. Goodson, M. K. Reiter,               [19] N. P. Kronenberg, H. M. Levy, and W. D. Strecker. Vaxclus-
     and J. J. Wylie. Fault-scalable byzantine fault-tolerant services.             ters (extended abstract): a closely-coupled distributed system.
     In SOSP ’05: Proceedings of the twentieth ACM symposium on                     SIGOPS Oper. Syst. Rev., 19(5), 1985.
     Operating systems principles, pages 59–74, New York, NY, USA,             [20] L. Lamport. The part-time parliament. ACM Transactions on
     2005. ACM.                                                                     Computer Systems, 16(2), May 1998.
 [2] M. Aguilera, A. Merchant, M. Shah, A. Veitch, and C. Karamano-            [21] J. MacCormick, N. Murphy, M. Najork, C. A. Thekkath, and
     lis. Sinfonia: A new paradigm for building scalable distributed                L. Zhou. Boxwood: Abstractions as the foundation for storage
     systems. In SOSP ’07: Proceedings of the 21st ACM symposium                    infrastructure. In Proceedings of the 6th ACM/USENIX Sympo-
     on Operating systems principles, New York, NY, 2007.                           sium on Operating Systems Design and Implementation (OSDI),
                                                                                    2004.
 [3] Amazon. Amazon simple queue service.             http://aws.
     amazon.com/sqs/, 2008.                                                    [22] L. Moser, P. Melliar-Smith, D. Agarwal, R. Budhia, C. Lingley-
                                                                                    Papadopoulos, and T. Archambault. The totem system. In Pro-
 [4] A. N. Bessani, E. P. Alchieri, M. Correia, and J. da Silva Fraga.              ceedings of the 25th International Symposium on Fault-Tolerant
     Depspace: A byzantine fault-tolerant coordination service. In                  Computing, June 1995.
     Proceedings of the 3rd ACM SIGOPS/EuroSys European Systems
                                                                               [23] S. Mullender, editor. Distributed Systems, 2nd edition. ACM
     Conference - EuroSys 2008, Apr. 2008.
                                                                                    Press, New York, NY, USA, 1993.
 [5] K. P. Birman. Replication and fault-tolerance in the ISIS system.         [24] B. Reed and F. P. Junqueira. A simple totally ordered broad-
     In SOSP ’85: Proceedings of the 10th ACM symposium on Oper-                    cast protocol. In LADIS ’08: Proceedings of the 2nd Workshop
     ating systems principles, New York, USA, 1985. ACM Press.                      on Large-Scale Distributed Systems and Middleware, pages 1–6,
 [6] M. Burrows. The Chubby lock service for loosely-coupled dis-                   New York, NY, USA, 2008. ACM.
     tributed systems. In Proceedings of the 7th ACM/USENIX Sympo-             [25] N. Schiper and S. Toueg. A robust and lightweight stable leader
     sium on Operating Systems Design and Implementation (OSDI),                    election service for dynamic systems. In DSN, 2008.
     2006.                                                                     [26] F. B. Schneider. Implementing fault-tolerant services using the
 [7] M. Castro and B. Liskov. Practical byzantine fault tolerance and               state machine approach: A tutorial. ACM Computing Surveys,
     proactive recovery. ACM Transactions on Computer Systems,                      22(4), 1990.
     20(4), 2002.                                                              [27] A. Sherman, P. A. Lisiecki, A. Berkheimer, and J. Wein. ACMS:
 [8] T. Chandra, R. Griesemer, and J. Redstone. Paxos made live: An                 The Akamai configuration management system. In NSDI, 2005.
     engineering perspective. In Proceedings of the 26th annual ACM            [28] A. Singh, P. Fonseca, P. Kuznetsov, R. Rodrigues, and P. Ma-
     symposium on Principles of distributed computing (PODC), Aug.                  niatis. Zeno: eventually consistent byzantine-fault tolerance.
     2007.                                                                          In NSDI’09: Proceedings of the 6th USENIX symposium on
 [9] A. Clement, M. Kapritsos, S. Lee, Y. Wang, L. Alvisi, M. Dahlin,               Networked systems design and implementation, pages 169–184,
     and T. Riche. UpRight cluster services. In Proceedings of the 22               Berkeley, CA, USA, 2009. USENIX Association.
     nd ACM Symposium on Operating Systems Principles (SOSP),                  [29] Y. J. Song, F. Junqueira, and B. Reed.            BFT for the
     Oct. 2009.                                                                     skeptics. http://www.net.t-labs.tu-berlin.de/
                                                                                    ˜petr/BFTW3/abstracts/talk-abstract.pdf.
[10] J. Cowling, D. Myers, B. Liskov, R. Rodrigues, and L. Shira. Hq
     replication: A hybrid quorum protocol for byzantine fault toler-          [30] R. van Renesse and K. Birman. Horus, a flexible group com-
     ance. In SOSP ’07: Proceedings of the 21st ACM symposium on                    munication systems. Communications of the ACM, 39(16), Apr.
     Operating systems principles, New York, NY, USA, 2007.                         1996.
                                                                               [31] R. van Renesse, K. Birman, M. Hayden, A. Vaysburd, and
[11] G. DeCandia, D. Hastorun, M. Jampani, G. Kakulapati, A. Lak-
                                                                                    D. Karr. Building adaptive systems using ensemble. Software
     shman, A. Pilchin, S. Sivasubramanian, P. Vosshall, and W. Vo-
                                                                                    - Practice and Experience, 28(5), July 1998.
     gels. Dynamo: Amazons highly available key-value store. In


                                                                          14
