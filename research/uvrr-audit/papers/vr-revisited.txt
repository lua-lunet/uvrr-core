Computer Science and Artificial Intelligence Laboratory
Technical Report

MIT-CSAIL-TR-2012-021                                              July 23, 2012




Viewstamped Replication Revisited
Barbara Liskov and James Cowling




m a ss a c h u se t t s i n st i t u t e o f t e c h n o l o g y, c a m b ri d g e , m a 02139 u s a — w w w. c s a il . m i t . e d u
                                  Viewstamped Replication Revisited

                                   Barbara Liskov and James Cowling
                        MIT Computer Science and Artificial Intelligence Laboratory
                              liskov@csail.mit.edu, cowling@csail.mit.edu



Abstract                                                                change, e.g., to replace a faulty node with a differ-
                                                                        ent machine. A reconfiguration can also change the
This paper presents an updated version of Viewstamped                   group size so that the number of failures the group
Replication, a replication technique that handles failures              can handle can increase or decrease.
in which nodes crash. It describes how client requests are
handled, how the group reorganizes when a replica fails,
and how a failed replica is able to rejoin the group. The
                                                                     • The paper presents the protocol independently of
paper also describes a number of important optimizations
                                                                       any applications that use VR. The original papers
and presents a protocol for handling reconfigurations that
                                                                       explained the protocol as part of a database [11, 10]
can change both the group membership and the number
                                                                       or file system [8], which made it difficult to separate
of failures the group is able to handle.
                                                                       the protocol from other details having to do with its
                                                                       use in an application.
1     Introduction
This paper presents an updated version of Viewstamped                 VR was originally developed in the 1980s, at about
Replication [11, 10, 8] (referred to as VR from now on).           the same time as Paxos [5, 6], but without knowledge of
VR works in an asynchronous network like the Internet,             that work. It differs from Paxos in that it is a replication
and handles failures in which nodes fail by crashing. It           protocol rather than a consensus protocol: it uses con-
supports a replicated service that runs on a number of             sensus, with a protocol very similar to Paxos, as part of
replica nodes. The service maintains a state, and makes            supporting a replicated state machine. Another point is
that state accessible to a set of client machines. VR              that unlike Paxos, VR did not require disk I/O during the
provides state machine replication [4, 13]: clients can            consensus protocol used to execute state machine opera-
run general operations to observe and modify the service           tions.
state. Thus the approach is suitable for implementing                 Some information about the history of VR can be
replicated services such as a lock manager or a file sys-          found in [7]. That paper presents a similar but less com-
tem.                                                               plete description of VR. It also explains how later work
   The presentation here differs from the earlier papers           on Byzantine fault tolerance was based on VR and how
on VR in several ways:                                             those protocols are related to the VR protocols.

    • The protocol described here improves on the orig-               The remainder of the paper is organized as follows.
      inal: it is simpler and has better performance.              Section 2 provides some background material and Sec-
      Some improvements were inspired by later work on             tion 3 gives an overview of the approach. The VR pro-
      Byzantine fault tolerance [2, 1].                            tocol is described in Section 4. Section 5 describes a
                                                                   number of details of the implementation that ensure good
    • The protocol does not require any use of disk; in-           performance, while Section 6 discusses a number of op-
      stead it uses replicated state to provide persistence.       timizations that can further improve performance. Sec-
                                                                   tion 7 describes our reconfiguration protocol. Section 8
    • The paper presents a reconfiguration protocol that           provides a discussion of the correctness of VR and we
      allows the membership of the replica group to                conclude in Section 9.


                                                               1
2     Background

This section begins with a discussion of our assumptions
about the environment in which VR runs. Section 2.2
discusses the number of replicas required to provide cor-
rect behavior, and Section 2.3 describes how to configure
a system to use VR.


2.1    Assumptions
VR handles crash failures: we assume that the only way             Figure 1: VR Architecture; the figure shows the configu-
nodes fail is by crashing, so that a machine is either func-       ration when f = 1.
tioning correctly or completely stopped. VR does not
handle Byzantine failures, in which nodes can fail arbi-
trarily, perhaps due to an attack by a malicious party.
                                                                   happened in the previous step. In a group of 2f + 1 repli-
   VR is intended to work in an asynchronous network,
                                                                   cas, f + 1 is the smallest quorum size that will work.
like the Internet, in which the non-arrival of a message
indicates nothing about the state of its sender. Messages             In general a group need not be exactly of size 2f + 1;
might be lost, delivered late or out of order, and delivered       if it isn’t, the threshold is the largest f such that 2f + 1
more than once; however, we assume that if sent repeat-            is less than or equal to the group size, K, and a quorum
edly a message will eventually be delivered. In this paper         is of size K − f . However, for a particular threshold f
we assume the network is not under attack by a malicious           there is no benefit in having a group of size larger than
party who might spoof messages. If such attacks are a              2f + 1: a larger group requires larger quorums to ensure
concern, they can be withstood by using cryptography to            intersection, but does not tolerate more failures. There-
obtain secure channels.                                            fore in the protocol descriptions in this paper we assume
                                                                   the group size is exactly 2f + 1.

2.2    Replica Groups
VR ensures reliability and availability when no more
than a threshold of f replicas are faulty. It does this by         2.3    Architecture
using replica groups of size 2f + 1; this is the minimal
number of replicas in an asynchronous network under the            The architecture for running VR is presented in Figure 1.
crash failure model. The rationale for needing this many           The figure shows some client machines that are using
replicas is as follows:                                            VR, which is running on 3 replicas; thus f = 1 in this
   We have to be able to carry out a request without wait-         example. Client machines run the user code on top of
ing for f replicas to participate, since these replicas may        the VR proxy. The user code communicates with VR
be crashed and unable to reply. However, the f repli-              by making operation calls to the proxy. The proxy then
cas we didn’t hear from might merely be slow to reply,             communicates with the replicas to cause the operation to
e.g., because of network congestion. f of the replicas we          be carried out and returns the result to the client when the
did hear from may thus subsequently fail. Therefore we             operation has completed.
need to run the protocol with enough replicas to ensure
that even if these f fail, there is a least one replica that          The replicas run code for the service that is being repli-
knows about the request. This implies that each step of            cated using VR, e.g., a file system. The replicas also run
the protocol must be processed by f + 1 replicas. These            the VR code. The VR code accepts requests from client
f + 1 together with the f that may not respond give us             proxies, carries out the protocol, and when the request is
the smallest group size of 2f + 1.                                 ready to be executed, causes this to happen by making
   A group of f + 1 replicas is often referred to as a quo-        an up-call to the service code at the replica. The service
rum and correctness of the protocol depends on the quo-            code executes the call and returns the result to the VR
rum intersection property: the quorum of replicas that             code, which sends it in a message to the client proxy that
processes a particular step of the protocol must have a            made the request.
non-empty intersection with the group of replicas avail-              Of the 2f + 1 replicas, only f + 1 need to run the ser-
able to handle the next step, since this way we can ensure         vice code. This point is discussed further in Section 6.1.
that at each next step at least one participant knows what


                                                               2
3     Overview                                                     4 The VR Protocol

State machine replication requires that replicas start in          This section describes how VR works under the assump-
the same initial state, and that operations be determinis-         tion that the group of replicas is fixed. We discuss some
tic. Given these assumptions, it is easy to see that repli-        ways to improve performance of the protocols in Sec-
cas will end up in the same state if they execute the same         tion 5 and optimizations in Section 6. The reconfigu-
sequence of operations. The challenge for the replication          ration protocol, which allows the group of replicas to
protocol is to ensure that operations execute in the same          change, is described in Section 7.
order at all replicas in spite of concurrent requests from            Figure 2 shows the state of the VR layer at a replica.
clients and in spite of failures.                                  The identity of the primary isn’t recorded in the state but
   VR uses a primary replica to order client requests; the         rather is computed from the view-number and the con-
other replicas are backups that simply accept the order            figuration. The replicas are numbered based on their
selected by the primary. Using a primary provides an               IP addresses: the replica with the smallest IP address is
easy solution to the ordering requirement, but it also in-         replica 1. The primary is chosen round-robin, starting
troduces a problem: what happens if the primary fails?             with replica 1, as the system moves to new views. The
VR’s solution to this problem is to allow different repli-         status indicates what sub-protocol the replica is engaged
cas to assume the role of primary over time. The system            in.
moves through a sequence of views. In each view one of                The client-side proxy also has state. It records the
the replicas is selected to be the primary. The backups            configuration and what it believes is the current view-
monitor the primary, and if it appears to be faulty, they          number, which allows it to know which replica is cur-
carry out a view change protocol to select a new primary.          rently the primary. Each message sent to the client in-
   To work correctly across a view change the state of the         forms it of the current view-number; this allows the client
system in the next view must reflect all client operations         to track the primary.
that were executed in earlier views, in the previously se-            In addition the client records its own client-id and a
lected order. We support this requirement by having the            current request-number. A client is allowed to have just
primary wait until at least f + 1 replicas (including it-          one outstanding request at a time. Each request is given a
self) know about a client request before executing it, and         number by the client and later requests must have larger
by initializing the state of a new view by consulting at           numbers than earlier ones; we discuss how clients ensure
least f + 1 replicas. Thus each request is known to a              this if they fail and recover in Section 4.5. The request
quorum and the new view starts from a quorum.                      number is used by the replicas to avoid running requests
   VR also provides a way for nodes that have failed to            more than once; it is also used by the client to discard
recover and then continue processing. This is important            duplicate responses to its requests.
since otherwise the number of failed nodes would even-
tually exceed the threshold. Correct recovery requires             4.1    Normal Operation
that the recovering replica rejoin the protocol only af-
ter it knows a state at least as recent as its state when it       This section describes how VR works when the primary
failed, so that it can respond correctly if it is needed for       isn’t faulty. Replicas participate in processing of client
a quorum. Clearly this requirement could be satisfied by           requests only when their status is normal. This constraint
having each replica record what it knows on disk prior             is critical for correctness as discussed in Section 8.
to each communication. However we do not require the                  The protocol description assumes all participating
use of disk for this purpose (and neither did the original         replicas are in the same view. Every message sent from
version of VR).                                                    one replica to another contains the sender’s current view-
   Thus, VR uses three sub-protocols that work together            number. Replicas only process normal protocol mes-
to ensure correctness:                                             sages containing a view-number that matches the view-
                                                                   number they know. If the sender is behind, the receiver
    • Normal case processing of user requests.                     drops the message. If the sender is ahead, the replica
                                                                   performs a state transfer: it requests information it is
    • View changes to select a new primary.                        missing from the other replicas and uses this information
                                                                   to bring itself up to date before processing the message.
    • Recovery of a failed replica so that it can rejoin the       State transfer is discussed further in Section 5.2.
      group.                                                          The request processing protocol works as follows. The
                                                                   description ignores a number of minor details, such as
These sub-protocols are described in detail in the next            re-sending protocol messages that haven’t received re-
section.                                                           sponses.


                                                               3
• The configuration. This is a sorted array containing
  the IP addresses of each of the 2f + 1 replicas.
• The replica number. This is the index into the con-
  figuration where this replica’s IP address is stored.

• The current view-number, initially 0.
• The current status, either normal, view-change, or
  recovering.
                                                                   Figure 3: Normal case processing in VR for a configura-
• The op-number assigned to the most recently re-                  tion with f = 1.
  ceived request, initially 0.

• The log. This is an array containing op-number                        number, adds the request to the end of its log, up-
  entries. The entries contain the requests that have                   dates the client’s information in the client-table, and
  been received so far in their assigned order.                         sends a hP REPARE O K v, n, ii message to the pri-
                                                                        mary to indicate that this operation and all earlier
• The commit-number is the op-number of the most
                                                                        ones have prepared locally.
  recently committed operation.
• The client-table. This records for each client the                5. The primary waits for f P REPARE O K messages
  number of its most recent request, plus, if the re-                  from different backups; at this point it considers
  quest has been executed, the result sent for that re-                the operation (and all earlier ones) to be commit-
  quest.                                                               ted. Then, after it has executed all earlier operations
                                                                       (those assigned smaller op-numbers), the primary
            Figure 2: VR state at a replica.                           executes the operation by making an up-call to the
                                                                       service code, and increments its commit-number.
                                                                       Then it sends a hR EPLY v, s, xi message to the
1. The client sends a hR EQUEST op, c, si message to                   client; here v is the view-number, s is the number
   the primary, where op is the operation (with its ar-                the client provided in the request, and x is the result
   guments) the client wants to run, c is the client-id,               of the up-call. The primary also updates the client’s
   and s is the request-number assigned to the request.                entry in the client-table to contain the result.
2. When the primary receives the request, it compares               6. Normally the primary informs backups about the
   the request-number in the request with the informa-                 commit when it sends the next P REPARE message;
   tion in the client table. If the request-number s isn’t             this is the purpose of the commit-number in the
   bigger than the information in the table it drops the               P REPARE message. However, if the primary does
   request, but it will re-send the response if the re-                not receive a new client request in a timely way, it
   quest is the most recent one from this client and it                instead informs the backups of the latest commit by
   has already been executed.                                          sending them a hC OMMIT v, ki message, where k
3. The primary advances op-number, adds the request                    is commit-number (note that in this case commit-
   to the end of the log, and updates the information                  number = op-number).
   for this client in the client-table to contain the new
                                                                    7. When a backup learns of a commit, it waits un-
   request number, s. Then it sends a hP REPARE v, m,
                                                                       til it has the request in its log (which may require
   n, ki message to the other replicas, where v is the
                                                                       state transfer) and until it has executed all earlier
   current view-number, m is the message it received
                                                                       operations. Then it executes the operation by per-
   from the client, n is the op-number it assigned to
                                                                       forming the up-call to the service code, increments
   the request, and k is the commit-number.
                                                                       its commit-number, updates the client’s entry in the
4. Backups process P REPARE messages in order: a                       client-table, but does not send the reply to the client.
   backup won’t accept a prepare with op-number n
   until it has entries for all earlier requests in its log.          Figure 3 shows the phases of the normal processing
   When a backup i receives a P REPARE message, it                 protocol.
   waits until it has entries in its log for all earlier re-          If a client doesn’t receive a timely response to a re-
   quests (doing state transfer if necessary to get the            quest, it re-sends the request to all replicas. This way
   missing information). Then it increments its op-                if the group has moved to a later view, its message will


                                                               4
reach the new primary. Backups ignore client requests;             request that was preparing when the view change oc-
only the primary processes them.                                   curred makes it into the new view. For example, opera-
    The protocol could be modified to allow backups to             tion 25 might have been preparing when the view change
process P REPARE messages out of order in Step 3. How-             happened, but none of the replicas that knew about it par-
ever there is no great benefit in doing things this way, and       ticipated in the view change protocol and as a result the
it complicates the view change protocol. Therefore back-           new primary knows nothing about operation 25. In this
ups process P REPARE messages in op-number order.                  case, the new primary might assign this number to a dif-
    The protocol does not require any writing to disk. For         ferent operation.
example, replicas do not need to write the log to disk                If two operations are assigned the same op-number,
when they add the operation to the log. This point is              how can we ensure that the right one is executed at that
discussed further in Section 4.3.                                  point in the order? The solution to this dilemma is to
    The protocol as described above has backups execut-            use the view-number: two operations can be assigned the
ing operations quickly: information about commits prop-            same number only when there has been a view change
agates rapidly, and backups execute operations as soon as          and in this case the one assigned a number in the later
they can. A somewhat lazier approach could be used, but            view prevails.
it is important that backups not lag very far behind. The             The view change protocol works as follows. Again the
reason is that when there is a view change, the replica            presentation ignores minor details having to do with fil-
that becomes the new primary will be unable to execute             tering of duplicate messages and with re-sending of mes-
new client requests until it is up to date. By executing           sages that appear to have been lost.
operations speedily, we ensure that when a replica takes
over as primary it is able to respond to new client re-             1. A replica i that notices the need for a view change
quests with low delay.                                                 advances its view-number, sets its status to view-
                                                                       change, and sends a hS TART V IEW C HANGE v, ii
4.2    View Changes                                                    message to the all other replicas, where v iden-
                                                                       tifies the new view. A replica notices the need
View changes are used to mask failures of the primary.                 for a view change either based on its own timer,
   Backups monitor the primary: they expect to hear                    or because it receives a S TART V IEW C HANGE or
from it regularly. Normally the primary is sending P RE -              D OV IEW C HANGE message for a view with a larger
PARE messages, but if it is idle (due to no requests) it               number than its own view-number.
sends C OMMIT messages instead. If a timeout expires
without a communication from the primary, the replicas              2. When replica i receives S TART V IEW C HANGE mes-
carry out a view change to switch to a new primary.                    sages for its view-number from f other replicas, it
   The correctness condition for view changes is that ev-              sends a hD OV IEW C HANGE v, l, v’, n, k, ii message
ery operation that has been executed by means of an up-                to the node that will be the primary in the new view.
call to the service code at one of the replicas must survive           Here v is its view-number, l is its log, v 0 is the view
into the new view in the same order selected for it at the             number of the latest view in which its status was
time it was executed. This up-call is performed at the old             normal, n is the op-number, and k is the commit-
primary first, and therefore the replicas carrying out the             number.
view change may not know whether the up-call occurred.
However, up-calls occur only for committed operations.              3. When the new primary receives f + 1
This means that the old primary must have received at                  D OV IEW C HANGE messages from different
least f P REPARE O K messages from other replicas, and                 replicas (including itself), it sets its view-number
this in turn implies that the operation is recorded in the             to that in the messages and selects as the new log
logs of at least f + 1 replicas (the old primary and the f             the one contained in the message with the largest
backups that sent the P REPARE O K messages).                          v 0 ; if several messages have the same v 0 it selects
   Therefore the view change protocol obtains informa-                 the one among them with the largest n. It sets its
tion from the logs of at least f + 1 replicas. This is                 op-number to that of the topmost entry in the new
sufficient to ensure that all committed operations will be             log, sets its commit-number to the largest such
known, since each must be recorded in at least one of                  number it received in the D OV IEW C HANGE mes-
these logs; here we are relying on the quorum intersec-                sages, changes its status to normal, and informs the
tion property. Operations that had not committed might                 other replicas of the completion of the view change
also survive, but this is not a problem: it is beneficial to           by sending hS TART V IEW v, l, n, ki messages to
have as many operations survive as possible.                           the other replicas, where l is the new log, n is the
   However, it’s impossible to guarantee that every client             op-number, and k is the commit-number.


                                                               5
  4. The new primary starts accepting client requests. It              However, running the protocol this way is unattrac-
     also executes (in order) any committed operations              tive since it adds a delay to normal case processing: the
     that it hadn’t executed previously, updates its client         primary would need to write to disk before sending the
     table, and sends the replies to the clients.                   P REPARE message, and the other replicas would need to
                                                                    write to disk before sending the P REPARE O K response.
  5. When other replicas receive the S TART V IEW mes-              Furthermore, it is unnecessary to do the disk write be-
     sage, they replace their log with the one in the mes-          cause the state is also stored at the other replicas and
     sage, set their op-number to that of the latest entry          can be retrieved from them, using a recovery protocol.
     in the log, set their view-number to the view num-             Retrieving state will be successful provided replicas are
     ber in the message, change their status to normal,             failure independent, i.e., highly unlikely to fail at the
     and update the information in their client-table. If           same time. If all replicas were to fail simultaneously,
     there are non-committed operations in the log, they            state will be lost if the information on disk isn’t up to
     send a hP REPARE O K v, n, ii message to the primary;          date; with failure independence a simultaneous failure is
     here n is the op-number. Then they execute all op-             unlikely. If nodes are all in the same data center, the use
     erations known to be committed that they haven’t               of UPS’s (uninterruptible power supplies) or non-volatile
     executed previously, advance their commit-number,              memory can provide failure independence if the problem
     and update the information in their client-table.              is a power failure. Placing replicas at different geograph-
                                                                    ical locations can additionally avoid loss of information
   In this protocol we solve the problem of more than one
                                                                    when there is a local problem like a fire.
request being assigned the same op-number by taking the
                                                                       This section describes a recovery protocol that doesn’t
log for the next view from latest previous active view and
                                                                    require disk I/O during either normal processing or dur-
ignoring logs from earlier view. VR as originally defined
                                                                    ing a view change. The original VR specification used
used a slightly different approach: it assigned each oper-
                                                                    a protocol that wrote to disk during the view change but
ation a viewstamp. A viewstamp is a pair hview-number,
                                                                    did not require writing to disk during normal case pro-
op-numberi, with the natural order: the view-number is
                                                                    cessing.
considered first, and then the op-number for two view-
                                                                       When a node comes back up after a crash it sets its sta-
stamps with the same view-number. At any op-number,
                                                                    tus to recovering and carries out the recovery protocol.
VR retained the request with the higher viewstamp. VR
                                                                    While a replica’s status is recovering it does not partici-
got its name from these viewstamps.
                                                                    pate in either the request processing protocol or the view
   A view change may not succeed, e.g., because the new
                                                                    change protocol. To carry out the recovery protocol, the
primary fails. In this case the replicas will start a further
                                                                    node needs to know the configuration. It can learn this by
view change, with yet another primary.
                                                                    waiting to receive messages from other group members
   The protocol as described is expensive because the log
                                                                    and then fetching the configuration from one of them;
is big, and therefore messages can be large. The ap-
                                                                    alternatively this information could be stored on disk.
proach we use to reduce the expense of view changes
                                                                       The recovery protocol is as follows:
is described in Section 5.
                                                                     1. The recovering replica, i, sends a hR ECOVERY i, xi
                                                                        message to all other replicas, where x is a nonce.
4.3    Recovery
                                                                     2. A replica j replies to a R ECOVERY message only
When a replica recovers after a crash it cannot partici-
                                                                        when its status is normal. In this case the replica
pate in request processing and view changes until it has
                                                                        sends a hR ECOVERY R ESPONSE v, x, l, n, k, ji mes-
a state at least as recent as when it failed. If it could
                                                                        sage to the recovering replica, where v is its view-
participate sooner than this, the system can fail. For ex-
                                                                        number and x is the nonce in the R ECOVERY mes-
ample, if it forgets that it prepared some operation, this
                                                                        sage. If j is the primary of its view, l is its log, n is
operation might then be known to fewer than a quorum
                                                                        its op-number, and k is the commit-number; other-
of replicas even though it committed, which could cause
                                                                        wise these values are nil.
the operation to be forgotten in a view change.
   If nodes record their state on disk before sending mes-           3. The recovering replica waits to receive at least f +
sages, a node will be able to rejoin the system as soon                 1 R ECOVERY R ESPONSE messages from different
as it has reinitialized its state by reading from disk. The             replicas, all containing the nonce it sent in its R E -
reason is that in this case a recovering node hasn’t forgot-            COVERY message, including one from the primary
ten anything it did before the crash (assuming the disk is              of the latest view it learns of in these messages.
intact). Instead it is the same as a node that has been un-             Then it updates its state using the information from
able to communicate for some period of time: its state is               the primary, changes its status to normal, and the
old but it hasn’t forgotten anything it did before.                     recovery protocol is complete.

                                                                6
The protocol is expensive because logs are big and there-           request it sent before it failed is still in transit (since that
fore the messages are big. A way to reduce this expense             request will have as its request number the number the
is discussed in Section 5.                                          client learns plus 1).
   If the group is doing a view change at the time of re-
covery, and the recovering replica, i, would be the pri-
mary of the new view, that view change cannot complete,             5 Pragmatics
since i will not respond to the D OV IEW C HANGE mes-
sages. This will cause the group to do a further view               The description of the protocols presented in the previ-
change, and i will be able to recover once this view                ous section ignores a number of important issues that
change occurs.                                                      must be resolved in a practical system. In this section
   The protocol uses the nonce to ensure that the recov-            we discuss how to provide good performance for node
ering replica accepts only R ECOVERY R ESPONSE mes-                 recovery, state transfer, and view changes. In all three
sages that are for this recovery and not an earlier one. It         cases, the key issue is efficient log management.
can produce the nonce by reading its clock; this will pro-
duce a unique nonce assuming clocks always advance.
                                                                    5.1    Efficient Recovery
Alternatively, it could maintain a counter on disk and ad-
vance this counter on each recovery.                                When a replica recovers from a crash it needs to recover
                                                                    its log. The question is how to do this efficiently. Send-
                                                                    ing it the entire log, as described in Section 4.3, isn’t a
4.4    Non-deterministic Operations
                                                                    practical way to proceed, since the log can get very large
State machine replication requires that if replicas start in        in a long-lived system.
the same state and execute the same sequence of opera-                 A way to reduce expense is to keep a prefix of the log
tions, they will end up with the same state. However, ap-           on disk. The log can be pushed to disk in the background;
plications frequently have non-deterministic operations.            there is no need to do this while running the protocol.
For example, file reads and writes are non-deterministic            When the replica recovers it can read the log from disk
if they require setting “time-last-read” and “time-last-            and then fetch the suffix from the other replicas. This re-
modified”. If these values are obtained by having each              duces the cost of recovery protocol substantially. How-
replica read its clock independently, the states at the             ever the replica will then need to execute all the requests
replicas will diverge.                                              in the log (or at least those that modify the state), which
   We can avoid divergence due to non-determinism by                can take a very long time if the log is big.
having the primary predict the value. It can do this by                Therefore a better approach is to take advantage of the
using local information, e.g., it reads its clock when a            application state at the recovering replica: if this state is
file operation arrives. Or it can carry out a pre-step in the       on disk, the replica doesn’t need to fetch the prefix of
protocol in which it requests values from the backups,              the log that has already been applied to the application
waits for f responses, and then computes the predicted              state and it needn’t execute the requests in that prefix ei-
value as a deterministic function of their responses and            ther. Note that this does not mean that the application is
its own. The predicted value is stored in the log along             writing to disk in the foreground; background writes are
with the client request and propagated to the other repli-          sufficient here too.
cas. When the operation is executed, the predicted value               For this approach to work, we need to know exactly
is used.                                                            what log prefix is captured on disk, both so that we ob-
   Use of predicted values can require changes to the ap-           tain all the requests after that point, and so that we avoid
plication code. There may need to be an up-call to obtain           rerunning operations that ran before the node failed. (Re-
a predicted value from the application prior to running             running operations can cause the application state to be
the protocol. Also, the application needs to use the pre-           incorrect unless the operations are idempotent.)
dicted value when it executes the request.                             Our solution to this problem uses checkpoints and
                                                                    is based on our later work on Byzantine-fault toler-
4.5    Client Recovery                                              ance [2, 1]. Every O operations the replication code
                                                                    makes an upcall to the application, requesting it to take
If a client crashes and recovers it must start up with a            a checkpoint; here O is a system parameter, on the or-
request-number larger than what it had before it failed.            der of 100 or 1000. To take a checkpoint the applica-
It fetches its latest number from the replicas and adds 2           tion must record a snapshot of its state on disk; addition-
to this value to be sure the new request-number is big              ally it records a checkpoint number, which is simply the
enough. Adding 2 ensures that its next request will have            op-number of the latest operation included in the check-
a unique number even in the odd case where the latest               point. When it executes operations after the checkpoint,


                                                                7
it must not modify the snapshot, but this can be accom-             entries after this from its log.
plished by using copy-on-write. These copied pages then                To get the state the replica sends a hG ET S TATE v, n’,
become what needs to be saved to make the next snapshot             ii message to one of the other replicas, where v is its
and thus checkpoints need not be very expensive.                    view-number and n0 is its op-number.
   When a node recovers, it first obtains the application              A replica responds to a G ET S TATE message only if its
state from another replica. To make this efficient, the             status is normal and it is currently in view v. In this case
application maintains a Merkle tree [9] over the pages              it sends a hN EW S TATE v, l, n, ki message, where v is its
in the snapshot. The recovering node uses the Merkle                view-number, l is its log after n0 , n is its op-number, and
tree to determine which pages to fetch; it only needs to            k is its commit-number.
fetch those that differ from their counterpart at the other
                                                                       When replica i receives the N EW S TATE message, it
replica. It’s possible that the node it is fetching from may
                                                                    appends the log in the message to its log and updates its
take another checkpoint while the recovery is in process;
                                                                    state using the other information in the message.
in this case the recovering node re-walks the Merkle tree
to pick up any additional changes.                                     Because of garbage collecting the log, it’s possible for
   In rare situations where a node has been out of service          there to be a gap between the last operation known to the
for a very long time, it may be infeasible to transfer the          slow replica and what the responder knows. Should a
new state over the network. In this case it is possible             gap occur, the slow replica first brings itself almost up to
to clone the disk of an active replica, install it at the re-       date using application state (like a recovering node would
covering node, and use this as a basis for computing the            do) to get to a recent checkpoint, and then completes the
Merkle tree.                                                        job by obtaining the log forward from the point. In the
   After the recovering node has all the application state          process of getting the checkpoint, it moves to the view in
as of the latest checkpoint at the node it is fetching from,        which that checkpoint was taken.
it can run the recovery protocol. When it runs the pro-
tocol it informs the other nodes of the current value of
its state by including the number of the checkpoint in its          5.3    View Changes
R ECOVERY message. The primary then sends it the log
from that point on.                                                 To complete a view change, the primary of the new view
   As mentioned checkpoints also speed up recovery                  must obtain an up-to-date log, and we would like the pro-
since the recovering replica only needs to execute re-              tocol to be efficient: we want to have small messages,
quests in the portion of the log not covered by the check-          and we want to avoid adding steps to the protocol.
point. Furthermore checkpoints allow the system to                     The protocol described in Section 4.2 has a small num-
garbage collect the log, since only the operations after            ber of steps, but big messages. We can make these mes-
the latest checkpoint are needed. Keeping a larger log              sages smaller, but if we do, there is always a chance that
than the minimum is a good idea however. For exam-                  more messages will be required.
ple, when a recovering node runs the recovery protocol,
                                                                       A reasonable way to get good behavior most of the
the primary might have just taken a checkpoint, and if
                                                                    time is for replicas to include a suffix of their log in their
it immediately discarded the log prefix reflected in that
                                                                    D OV IEW C HANGE messages. The amount sent can be
checkpoint, it would be unable to bring the recovering
                                                                    small since the most likely case is that the new primary
replica up to date without transferring application state.
                                                                    is up to date. Therefore sending the latest log entry, or
A large enough suffix of the log should be retained to
                                                                    perhaps the latest two entries, should be sufficient. Oc-
avoid this problem.
                                                                    casionally, this information won’t be enough; in this case
                                                                    the primary can ask for more information, and it might
5.2    State Transfer                                               even need to first use application state to bring itself up
                                                                    to date.
State transfer is used by a node that has gotten behind
(but hasn’t crashed) to bring itself up-to-date. There
are two cases, depending on whether the slow node has
learned that it is missing requests in its current view, or         6 Optimizations
has heard about a later view. In the former case it only
needs to learn about requests after its op-number. In the           This section describes several optimizations that can be
latter it needs to learn about requests after the latest com-       used to improve the performance of the protocol. Some
mitted request in its log, since requests after that might          optimizations were proposed in the paper on Harp [8];
have been reordered in the view change, so in this case it          others are based on later work on the PBFT replication
sets its op-number to its commit-number and removes all             protocol, which handles Byzantine failures [2, 1].


                                                                8
6.1     Witnesses                                                     To prevent a primary returning results based on stale
                                                                   data, Harp used leases [3]. The primary processes reads
Harp proposed the use of witnesses to avoid having all
                                                                   unilaterally only if it holds valid leases from f other
replicas actively run the service. The group of 2f + 1
                                                                   replicas, and a new view will start only after leases at
replicas includes f +1 active replicas, which store the ap-
                                                                   f + 1 participants in the view change protocol expire.
plication state and execute operations, and f witnesses,
                                                                   This ensures that the new view starts after the old primary
which do not. The primary is always an active replica.
                                                                   has stopped replying to read requests, assuming clocks
Witnesses are needed for view changes and recovery.
                                                                   rates are loosely synchronized.
They aren’t involved in the normal case protocol as long
                                                                      In addition to reducing message traffic and delay for
as the f + 1 active replicas are processing operations.
                                                                   processing reads, this approach has another benefit: read
They fill in for active replicas when they aren’t respond-
                                                                   requests need not run through the protocol. Thus load on
ing; however, even in this case witnesses do not execute
                                                                   the system can be reduced substantially, especially for
operations. Thus most of the time witnesses can be doing
                                                                   workloads that are primarily read-only, which is often
other work; only the active replicas run the service code
                                                                   the case.
and store the service state.

                                                                   6.3.2   Reads at Backups
6.2     Batching
                                                                   Leases aren’t needed if it is acceptable for the result of a
PBFT proposed the use of batching to reduce the over-              read to be based on stale information. In this case it also
head of running the protocol. Rather than running the              works to execute read requests at backups.
protocol each time a request arrives, the primary collects            To support causality, there must be a way for a client
a bunch of requests and then runs the protocol for all of          to inform a backup of its previous operation. One way to
them at once.                                                      do this is for the client to maintain a last-request-number
   The use of batching can be limited to when the primary          in its state. When the client does a write, the primary
is heavily loaded. This way it has little impact on latency.       returns the op-number that it assigned to that request, and
When the primary isn’t busy it processes requests as soon          the client stores this in last-request-number. When the
as they arrive. When it is busy, it batches, but requests          client sends a read request it includes this number, and
will be arriving frequently and therefore it needn’t wait          the replica responds only if it has executed operations
very long to collect the next batch. Batching can provide          at least this far; in its response it includes its commit-
significant gains in throughput when the system is heav-           number, which the client stores in last-request-number.
ily loaded by amortizing the cost of the protocol over a              The “reads at backups” provides a form of load balanc-
large number of client requests.                                   ing; effectively it allows the backups to be used as caches
                                                                   that are up to date enough to satisfy the causality require-
6.3     Fast Reads                                                 ments. However unlike the first approach (“reads at the
                                                                   primary”) it does not provide external consistency. The
This section discusses two ways to reduce the latency for          first approach provides causality even in a setting where
handling of read requests; both techniques additionally            there are many storage repositories. In this setting, the
improve overall throughput.                                        second approach requires a different way of capturing
                                                                   causality, e.g., with vector timestamps [12] or Lamport
6.3.1   Reads at the Primary                                       clocks [4].
Harp proposed a way to improve performance for reads,
by having the primary execute them without consulting              7 Reconfiguration
the other replicas. This communication is not needed be-
cause read operations don’t modify state and therefore             This section describes a reconfiguration protocol that al-
need not survive into the next view.                               lows the members of the group of replicas to change over
   However, having the primary perform read requests               time. Reconfiguration is needed to replace failed nodes
unilaterally could lead to the read returning a result based       that cannot be recovered, as well as for other reasons,
on an old state. This can happen if the request goes to            e.g., to use more powerful computers as replicas, or to
an old primary that is not aware that a view change has            position the replicas in different data centers.
occurred. For example, suppose a network partition has                The reconfiguration protocol can also be used to
isolated the old primary from other replicas. Meanwhile            change the threshold, f , the number of failures the group
a view change has happened and the new primary has                 is intended to handle: the new group can be bigger or
executed more operations, so that the state at the old pri-        smaller than the old one. Changing the threshold is use-
mary is stale.                                                     ful to allow the system to adapt to changing circum-


                                                               9
  • The epoch-number, initially 0.                                  2f 0 + 1 is less than or equal to the size of new-config.
                                                                       If the primary accepts the request, it processes it in
  • The old-configuration, initially empty.                         the usual way, by carrying out the normal case proto-
                                                                    col, but with two differences: First, the primary immedi-
 Figure 4: Additional state needed for reconfiguration.             ately stops accepting other client requests; the reconfigu-
                                                                    ration request is the last request processed in the current
stances, e.g., if experience indicates that more or fewer           epoch. Second, executing the request does not cause an
failures are happening than expected.                               up-call to the service code; instead, a reconfiguration af-
   The approach to handling reconfigurations is as fol-             fects only the VR state.
lows. A reconfiguration is triggered by a special client               The processing of the request happens as follows:
request. This request is run through the normal case pro-
                                                                     1. The primary adds the request to its log, sends a
tocol by the old group. When the request commits, the
                                                                        P REPARE message to the backups, and stops accept-
system moves to a new epoch, in which responsibility for
                                                                        ing client requests.
processing client requests shifts to the new group. How-
ever, the new group cannot process client requests until             2. The backups handle the P REPARE in the usual way:
its replicas are up to date: the new replicas must know all             they add the request to their log, but only when they
operations that committed in the previous epoch. To get                 are up to date. Then they send P REPARE O K re-
up to date they transfer state from the old replicas, which             sponses to the primary.
do not shut down until the state transfer is complete.
                                                                     3. When the primary receives f of these responses
                                                                        from different replicas, it increments its epoch-
7.1    Reconfiguration Details                                          number, sends C OMMIT messages to the other old
To handle reconfigurations we add some information to                   replicas, and sends hS TART E POCH e, n, old-config,
the replica state, as shown in Figure 4. In addition there              new-configi messages to replicas that are being
is another status, transitioning. A replica sets its status             added to the system, i.e., those that are members
to transitioning at the beginning of the next epoch. New                of the new group but not of the old group. Here e
replicas use the old-configuration for state transfer at the            is the new epoch-number and n is the op-number.
start of an epoch; this way new nodes know where to                     Then it executes all client requests ordered before
get the state. Replicas that are members of the replica                 the R ECONFIGURATION request that it hadn’t exe-
group for the new epoch change their status to normal                   cuted previously and sets its status to transitioning.
when they have received the complete log up to the start
of the epoch; replicas that are being replaced shut down               Now we explain how the two groups move to the new
once they know their state has been transferred to the new          epochs. First we explain the processing at replicas that
group.                                                              are members of the new group; these replicas may be
   Every message now contains an epoch-number. Repli-               members of the old group, or they may be added as part
cas only process messages (from either clients or other             of the reconfiguration. Then we explain processing at
replicas) that match the epoch they are in. If they receive         replicas that are being replaced, i.e., they are members
a message with a later epoch number they move to that               of the old group but not of the new group.
epoch as discussed below. If they receive a message with
an earlier epoch number they discard the message but in-            7.1.1   Processing in the New Group
form the sender about the later epoch.
                                                                    Replicas that are members of the replica group for the
   Reconfigurations are requested by a client, c,
                                                                    new epoch handle reconfiguration as follows:
e.g., the administrator’s node, which sends a
hR ECONFIGURATION e, c, s, new-configi message                       1. When a replica learns of the new epoch (e.g., be-
to the current primary. Here e is the current epoch-                    cause it receives a S TART E POCH or C OMMIT mes-
number known to c, s is c0 s request-number, and                        sage), it initializes its state to record the old and new
new-config provides the IP addresses of all members of                  configurations, the new epoch-number, and the op-
the new group. The primary will accept the request only                 number, sets its view-number to 0, and sets its status
if s is large enough (based on the client-table) and e                  to transitioning.
is the current epoch-number. Additionally the primary
discards the request if new-config contains fewer than               2. If the replica is missing requests from its log, it
3 IP addresses (since this is the minimum group size                    brings its state up to date by sending state transfer
needed for VR). The new threshold is determined by the                  messages to the old replicas and also to other new
size of new-config: it is the largest value f 0 such that               replicas. This allows it to get a complete state up to

                                                               10
      the op-number, and thus learn of all client requests          Thus a replica will not accept a normal case or view
      up to the reconfiguration request.                            change message that contains an old epoch-number; in-
                                                                    stead it informs the sender about the new epoch.
 3. Once a replica in the new group is up to date with                 Additionally, in the view change protocol the new pri-
    respect to the start of the epoch, it sets its status to        mary needs to recognize that a reconfiguration is in pro-
    normal and starts to carry out normal processing; it            cess, so that it stops accepting new client requests. To
    executes any requests in the log that it hasn’t already         handle this case the new primary checks the topmost re-
    executed and, if it is the primary of the new group, it         quest in the log; if it is a R ECONFIGURATION request,
    starts accepting new requests. Additionally, it sends           it won’t accept any additional client requests. Further-
    hE POCH S TARTED e, ii messages to the replicas that            more, if the request is committed, it sends S TART E POCH
    are being replaced.                                             messages to the new replicas.
Replicas in the new group select the primary in the usual              The recovery protocol also needs to change. An old
way, by using a deterministic function of the configura-            replica that is attempting to recover while a reconfigura-
tion for the new epoch and the current view number.                 tion is underway may be informed about the next epoch.
   Replicas in the new group might receive (duplicate)              If the replica isn’t a member of the new replica group
S TART E POCH messages after they have completed state              it shuts down; otherwise, it continues with recovery by
transfer. In this case they send an E POCH S TARTED re-             communicating with replicas in the new group. (Here
sponse to the sender.                                               we are assuming that new replicas are warm when they
                                                                    start up as discussed in Section 7.5.)
                                                                       In both the view change and recovery protocols, R E -
7.1.2   Processing at Replicas being Replaced
                                                                    CONFIGURATION requests that are in the log, but not in
 1. When a replica being replaced learns of the new                 the topmost entry, are ignored, because in this case the
    epoch (e.g., by receiving a C OMMIT message for                 reconfiguration has already happened.
    the reconfiguration request), it changes its epoch-
    number to that of the new epoch and sets its sta-
    tus to transitioning. If the replica doesn’t yet have
                                                                    7.3    Shutting down Old Replicas
    the reconfiguration request in its log it obtains it            The protocol described above allows replicas to recog-
    by performing state transfer from other old repli-              nize when they are no longer needed so that they can shut
    cas. Then it stores the current configuration in old-           down. However, we also provide a way for the adminis-
    configuration and stores the new configuration in               trator who requested the reconfiguration to learn when it
    configuration.                                                  has completed. This way machines being replaced can
                                                                    be shut down quickly, even when, for example, they are
 2. Replicas being replaced respond to state transfer re-
                                                                    unable to communicate with other replicas because of a
    quests from replicas in the new group until they re-
                                                                    long-lasting network partition.
    ceive f 0 + 1 E POCH S TARTED messages from the
                                                                       Receiving a reply to the R ECONFIGURATION request
    new replicas, where f 0 is the threshold of the new
                                                                    doesn’t contain the necessary information since that only
    group. At this point the replica being replaced shuts
                                                                    tells the administrator that the request has committed,
    down.
                                                                    whereas the administrator needs to know that enough
 3. If a replica being replaced doesn’t receive the                 new nodes have completed state transfer. To provide
    E POCH S TARTED messages in a timely way, it sends              the needed information, we provide another operation,
    S TART E POCH messages to the new replicas (or                  hC HECK E POCH e, c, si; the administrator calls this op-
    to the subset of those replicas it hasn’t already               eration after getting the reply to the R ECONFIGURATION
    heard from). New replicas respond to these mes-                 request. Here c is the client machine being used by the
    sages either by moving to the epoch, or if they                 administrator, s is c0 s request-number, and e is the new
    are already active in the next epoch, they send the             epoch. The operation runs through the normal case pro-
    E POCH S TARTED message to the old replica.                     tocol in the new group, and therefore when the adminis-
                                                                    trator gets the reply this indicates the reconfiguration is
                                                                    complete.
7.2     Other Protocol Changes
                                                                       It’s important that the administrator wait for the recon-
To support reconfigurations, we need to modify the view             figuration to complete before shutting down the nodes
change and recovery protocols so that they work while a             being replaced. The reason is that if one of these nodes
reconfiguration is underway.                                        were shut down prematurely, this can lead to more than f
   The most important change is that a replica will not             failures in the old group before the state has been trans-
accept messages for an epoch earlier than what it knows.            ferred to the new group, and the new group would then

                                                               11
be unable to process client requests.                              condition implies that any request that had been executed
                                                                   retains its place in the order.
                                                                      Clearly this condition holds in the first view. Assum-
7.4    Locating the Group                                          ing it holds in view v, the protocol will ensure that it also
Since the group can move, a new client needs a way to              holds in the next view, v 0 . The reasoning is as follows:
find the current configuration. This requires an out-of-              Normal case processing ensures that any operation o
band mechanism, e.g., the current configuration can be             that committed in view v is known to at least f + 1 repli-
obtained by communicating with a web site run by the               cas, each of which also knows all operations ordered be-
administrator.                                                     fore o, including (by assumption) all operations commit-
   Old clients can also use this mechanism to find the new         ted in views before v. The view change protocol starts
group. However, to make it easy for current clients to             the new view with the most recent log received from
find the group, old replicas that receive a client request         f + 1 replicas. Since none of these replicas accepts P RE -
with an old epoch number inform the client about the               PARE messages from the old primary after sending the
reconfiguration by sending it a hN EW E POCH e, v, new-            D OV IEW C HANGE message, the most recent log contains
configi message.                                                   the latest operation committed in view v (and all earlier
                                                                   operations). Therefore all operations committed in views
                                                                   before v 0 are present in the log that starts view v 0 , in their
7.5    Discussion                                                  previously assigned order.
The most important practical issue with this reconfigura-             It’s worth noting that it is crucial that replicas stop ac-
tion protocol is the following: while the system is mov-           cepting P REPARE messages from earlier views once they
ing from one epoch to the next it does not accept any new          start the view change protocol (this happens because they
client requests. The old group stops accepting client re-          change their status as soon as they learn about the view
quests the moment the primary of the old group receives            change). Without this constraint the system could get
the R ECONFIGURATION request; the new group can start              into a state in which there are two active primaries: the
processing client requests only when at least f 0 + 1 new          old one, which hasn’t failed but is merely slow or not
replicas have completed state transfer.                            well connected to the network, and the new one. If a
   Since client requests will be delayed until the move            replica sent a P REPARE O K message to the old primary
is complete, we would like the move to happen quickly.             after sending its log to the new one, the old primary
However, the problem is that state transfer can take a             might commit an operation that the new primary doesn’t
long time, even with our approach of checkpoints and               learn about in the D OV IEW C HANGE messages.
Merkle trees, if the application state is large.                      Liveness. The protocol executes client requests pro-
   The way to reduce the delay is for the new nodes to             vided at least f + 1 non-failed replicas, including the
be warmed up by doing state transfer before the recon-             current primary, are able to communicate. If the primary
figuration. While this state transfer is happening the old         fails, requests cannot be executed in the current view.
group continues to process client requests. The R ECON -           However if replicas are unable to execute the client re-
FIGURATION request is sent only when the new nodes                 quest in the current view, they will move to a new one.
are almost up to date. As a result, the delay until the new           Replicas monitor the primary and start a view change
nodes can start handling client requests will be short.            by sending the S TART V IEW C HANGE messages if the
                                                                   primary is unresponsive. When other replicas receive
                                                                   the S TART V IEW C HANGE messages they will also ad-
8     Correctness                                                  vance their view-number and send S TART V IEW C HANGE
                                                                   messages. As a result, replicas will receive enough
In this section we provide an informal discussion of the           S TART V IEW C HANGE messages so that they can send
correctness of the protocol. Section 8.1 discusses the cor-        D OV IEW C HANGE messages, and thus the new primary
rectness of the view change protocol ignoring node re-             will receive enough D OV IEW C HANGE messages to en-
covery; correctness of the recovery protocol is discussed          able it to start the next view. And once this happens it will
in Section 8.2. Section 8.3 discusses correctness of the           be able to carry out client requests. Additionally, clients
reconfiguration protocol.                                          send their requests to all replicas if they don’t hear from
                                                                   the primary, and thus cause requests to be executed in a
                                                                   later view if necessary.
8.1    Correctness of View Changes
                                                                      More generally liveness depends on properly setting
Safety. The correctness condition for view changes is              the timeouts used to determine whether to start a view
that every committed operation survives into all subse-            change so as to avoid unnecessary view changes and thus
quent views in the same position in the serial order. This         allow useful work to get done.


                                                              12
8.2    Correctness of the Recovery Protocol                             We could avoid the round of S TART V IEW C HANGE
                                                                     messages by having replicas write the new view num-
Safety. The recovery protocol is correct because it guar-            ber to disk before sending the DoViewChange messages;
antees that when a recovering replica changes its status             this is the approach used in the original version of VR.
to normal it does so in a state at least as recent as what it           Liveness. It’s interesting to note that the recovery pro-
knew when it failed.                                                 tocol requires f + 2 replicas to communicate! Neverthe-
   When a replica recovers it doesn’t know what view it              less the protocol is live, assuming no more that f replicas
was in when it failed. However, when it receives f +                 fail simultaneously. The reason is that a replica is con-
1 responses to its R ECOVERY message, it is certain to               sidered failed until it has recovered its state. Therefore
learn of a view at least as recent as the one that existed           while a replica is recovering, there must be at least f + 1
when it sent its last P REPARE O K, D OV IEW C HANGE, or             other replicas that are not failed, and thus the recovering
R ECOVERY R ESPONSE message. Furthermore it gets its                 replica will receive at least f +1 responses to its R ECOV-
state from the primary of the latest view it hears about,            ERY message.
which ensures it learns the latest state of that view. In
effect, the protocol uses the volatile state at f +1 replicas
as stable state.                                                     8.3    Correctness of Reconfiguration
   One important point is that the nonce is needed be-
                                                                     Safety. Reconfiguration is correct because it preserves
cause otherwise a recovering replica might combine re-
                                                                     all committed requests in the order selected for them.
sponses to an earlier R ECOVERY message with those to a
                                                                     The primary of the old group stops accepting client re-
later one; in this case it would not necessarily learn about
                                                                     quests as soon as it receives a R ECONFIGURATION re-
the latest state.
                                                                     quest. This means that the R ECONFIGURATION request
   Another important point is that the key to correct re-            is the last committed request in that epoch. Furthermore
covery is the combination of the view change and recov-              new replicas do not become active until they have com-
ery protocols. In particular the view change protocol has            pleted state transfer. Thus they learn about all requests
two message exchanges (for the S TART V IEW C HANGE                  that committed in the previous epoch, in the order se-
and D OV IEW C HANGE messages). These ensure that                    lected for them, and these requests are ordered before
when a view change happens, at least f + 1 replicas al-              any client request processed in the new epoch.
ready know that the view change is in progress. There-
                                                                        An interesting point is that it’s possible for the pri-
fore if a view change was in progress when the replica
                                                                     maries in both the old and new groups to be active si-
failed, it is certain to recover in that view or a later one.
                                                                     multaneously. This can happen if the primary of the old
   It’s worth noting that having two message exchanges               group fails after the reconfiguration request commits. In
is necessary. If there were only one exchange, i.e., just an         this case it’s possible for a view change to occur in the
exchange of D OV IEW C HANGE messages, the following                 old group, and the primary selected by this view change
scenario is possible (here we consider a group containing            might re-run the normal-case protocol for the reconfigu-
three replicas, currently in view v with primary r1):                ration request. Meanwhile the new group might be ac-
                                                                     tively processing new client requests. However the old
  1. Before it failed the recovering replica, r3, had
                                                                     group will not accept any new client requests (because
     decided to start a view change and had sent a
                                                                     the primary of the new view checks whether the topmost
     D OV IEW C HANGE message to r2, which will be the
                                                                     request in the log is a reconfiguration request and if so
     primary of view v +1, but this message has been de-
                                                                     it won’t accept client requests). Therefore processing in
     layed in the network.
                                                                     the old group cannot interfere with the ordering of re-
  2. Replica r3 recovers in view v after receiving R E -             quests that are handled in the new epoch.
     COVERY R ESPONSE messages from both r1 and r2.                     It’s worth noting that it is important that the new epoch
     Then it starts sending P REPARE O K messages to r1              start in view 0 rather than using the view in which the old
     in response to r1’s P REPARE messages, but these                epoch ended up. The reason is that S TART E POCH mes-
     P REPARE messages do not arrive at r2.                          sages could be sent from old epoch replicas with different
                                                                     view numbers; this can happen if there is a view change
  3. Replica r3’s D OV IEW C HANGE message arrives at                in the old group and the new primary re-runs the recon-
     r2, which starts view v + 1.                                    figuration request. If the new group accepted the view
                                                                     number from the old group, we could end up with two
Step 3 is erroneous because the requests that committed              primaries in the new group, which would be incorrect.
after r3 recovered are not included in the log for view                 Liveness. The system is live because (1) the base pro-
v +1. The round of S TART V IEW C HANGE messages pre-                tocol is live, which ensures that the R ECONFIGURATION
vents this scenario.                                                 request will eventually be executed in the old group; (2)

                                                                13
new replicas will eventually move to the next epoch; and             only f + 1 of the 2f + 1 replicas. The paper also de-
(3) old replicas do not shut down until new replicas are             scribes ways to reduce latency of reads and writes, and to
ready to process client requests.                                    improve throughput when the system is heavily loaded.
   New replicas are certain to learn of the new epoch                   Today there is increasing use of replication proto-
once a R ECONFIGURATION request has committed be-                    cols that handle crash failures in modern large-scale dis-
cause after this point old group members actively com-               tributed systems. Our hope is that this paper will be a
municate with both old and new ones to ensure that other             help to those who are developing the next generation of
replicas know about the reconfiguration. Most important              reliable distributed systems.
here are the S TART E POCH messages that old nodes send
to new ones if they don’t receive E POCH S TARTED mes-               References
sages in a timely way. These messages ensure that even
                                                                      [1] C ASTRO , M. Practical Byzantine Fault Tolerance. Technical Re-
if the old primary fails to send S TART E POCH messages                   port MIT-LCS-TR-817, Laboratory for Computer Science, MIT,
to the new replicas, or if it sends these messages but they               Cambridge, Jan. 2000. Ph.D. thesis.
fail to arrive, nevertheless the new replicas will learn of           [2] C ASTRO , M., AND L ISKOV, B. Practical Byzantine Fault Tol-
the new epoch.                                                            erance and Proactive Recovery. ACM Transactions on Computer
   Old replicas do not shut down too early because they                   Systems 20, 4 (Nov. 2002), 398–461.
wait for f 0 + 1 E POCH S TARTED messages before shut-                [3] G RAY, C., AND C HERITON , D. Leases: An efficient fault-
ting down. This way we know that enough new replicas                      tolerant mechanism for distributed file cache consistency. In Pro-
                                                                          ceedings of the Twelfth ACM Symposium on Operating Systems
have their state to ensure that the group as a whole can                  Principles (1989), ACM, pp. 202–210.
process client requests assuming no more than a thresh-
                                                                      [4] L AMPORT, L. Time, Clocks, and the Ordering of Events in a
old of failures in the new group.                                         Distributed System. Comm. of the ACM 21, 7 (July 1978), 558–
   Old replicas might shut down before some new repli-                    565.
cas have finished state transfer. However, this can happen            [5] L AMPORT, L. The Part-Time Parliament. Research Report 49,
only after at least f 0 +1 new replicas have their state, and             Digital Equipment Corporation Systems Research Center, Palo
the other new replicas will then be able to get up to date                Alto, CA, Sept. 1989.
by doing state transfer from the new replicas.                        [6] L AMPORT, L. The Part-Time Parliament. ACM Transactions on
                                                                          Computer Systems 10, 2 (May 1998).
   A final point is that old replicas shut down by the ad-
                                                                      [7] L ISKOV, B. From viewstamped replication to byzantine fault
ministrator do not cause a problem, assuming the admin-
                                                                          tolerance. In Replication: Theory and Practice (2010), no. 5959
istrator doesn’t do this until after executing a C HECK -                 in Lecture Notes in Computer Science.
E POCH request in the new epoch. Waiting until this point             [8] L ISKOV, B., G HEMAWAT, S., G RUBER , R., J OHNSON , P.,
ensures that at least f 0 + 1 replicas in the new group have              S HRIRA , L., AND W ILLIAMS , M. Replication in the Harp File
their state; therefore after this point the old replicas are              System. In Proceedings of the Thirteenth ACM Symposium on
no longer needed.                                                         Operating System Principles (Pacific Grove, California, 1991),
                                                                          pp. 226–238.
                                                                      [9] M ERKLE , R. C. A Digital Signature Based on a Conventional
                                                                          Encryption Function. In Advances in Cryptology - Crypto’87,
9   Conclusions                                                           C. Pomerance, Ed., no. 293 in Lecture Notes in Computer Sci-
                                                                          ence. Springer-Verlag, 1987, pp. 369–378.
This paper has presented an improved version of View-                [10] O KI , B., AND L ISKOV, B. Viewstamped Replication: A New
stamped Replication, a protocol used to build replicated                  Primary Copy Method to Support Highly-Available Distributed
systems that are able to tolerate crash failures. The pro-                Systems. In Proc. of ACM Symposium on Principles of Dis-
                                                                          tributed Computing (1988), pp. 8–17.
tocol does not require any disk writes as client requests
are processed or even during view changes, yet it allows             [11] O KI , B. M. Viewstamped Replication for Highly Available Dis-
                                                                          tributed Systems. Technical Report MIT-LCS-TR-423, Labora-
nodes to recover from failures and rejoin the group.                      tory for Computer Science, MIT, Cambridge, MA, May 1988.
   The paper also presents a protocol to allow for recon-                 Ph.D. thesis.
figurations that change the members of the replica group,            [12] PARKER , D. S., P OPEK , G. J., RUDISIN , G., S TOUGHTON , A.,
and even the failure threshold. A reconfiguration tech-                   WALKER , B., WALTON , E., C HOW, J., E DWARDS , D., K ISER ,
nique is necessary for the protocol to be deployed in                     S., AND K LINE , C. Detection of mutual inconsistency in dis-
                                                                          tributed systems. IEEE Transactions on Software Engineering
practice since the systems of interest are typically long                 SE-9, 3 (May 1983), 240–247.
lived.
                                                                     [13] S CHNEIDER , F. Implementing Fault-Tolerant Services using the
   In addition, the paper describes a number of optimiza-                 State Machine Approach: a Tutorial. ACM Computing Surveys
tions that make the protocol efficient. The state transfer                22, 4 (Dec. 1990), 299–319.
technique uses application state to speed up recovery and
allows us to keep the log small, by discarding prefixes.
The use of witnesses allows service state to be stored at


                                                                14
