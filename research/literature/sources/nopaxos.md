      Just say NO to Paxos Overhead:
Replacing Consensus with Network Ordering
Jialin Li, Ellis Michael, Naveen Kr. Sharma, Adriana Szekeres, and Dan R. K. Ports,
                               University of Washington
        https://www.usenix.org/conference/osdi16/technical-sessions/presentation/li




       This paper is included in the Proceedings of the
    12th USENIX Symposium on Operating Systems Design
               and Implementation (OSDI ’16).
                    November 2–4, 2016 • Savannah, GA, USA
                                  ISBN 978-1-931971-33-1



                                             Open access to the Proceedings of the
                                         12th USENIX Symposium on Operating Systems
                                                 Design and Implementation
                                                   is sponsored by USENIX.
                               Just Say NO to Paxos Overhead:
                         Replacing Consensus with Network Ordering
       Jialin Li     Ellis Michael        Naveen Kr. Sharma Adriana Szekeres                   Dan R. K. Ports
                                            University of Washington
                   {lijl, emichael, naveenks, aaasz, drkp}@cs.washington.edu


                        Abstract                                the network and the application can eliminate nearly all
                                                                replication overhead. Our key insight is that the commu-
Distributed applications use replication, implemented by        nication layer should provide a new ordered unreliable
protocols like Paxos, to ensure data availability and trans-    multicast (OUM) primitive – where all receivers are guar-
parently mask server failures. This paper presents a new        anteed to process multicast messages in the same order,
approach to achieving replication in the data center with-      but messages may be lost. This model is weak enough to
out the performance cost of traditional methods. Our work       be implemented efficiently, yet strong enough to dramati-
carefully divides replication responsibility between the        cally reduce the costs of a replication protocol.
network and protocol layers. The network orders requests           The ordered unreliable multicast model enables our
but does not ensure reliable delivery – using a new primi-      new replication protocol, Network-Ordered Paxos. In nor-
tive we call ordered unreliable multicast (OUM). Imple-         mal cases, NOPaxos avoids coordination entirely by re-
menting this primitive can be achieved with near-zero-cost      lying on the network to deliver messages in the same
in the data center. Our new replication protocol, Network-      order. It requires application-level coordination only to
Ordered Paxos (NOPaxos), exploits network ordering to           handle dropped packets, a fundamentally simpler prob-
provide strongly consistent replication without coordi-         lem than ordering requests. The resulting protocol is sim-
nation. The resulting system not only outperforms both          ple, achieves near-optimal throughput and latency, and
latency- and throughput-optimized protocols on their re-        remains robust to network-level failures.
spective metrics, but also yields throughput within 2%             We describe several ways to build the OUM communi-
and latency within 16 µs of an unreplicated system – pro-       cations layer, all of which offer net performance benefits
viding replication without the performance cost.                when combined with NOPaxos. In particular, we achieve
1   Introduction                                                an essentially zero-overhead implementation by relying
                                                                on the network fabric itself to sequence requests, using
Server failures are a fact of life for data center applica-     software-defined networking technologies and the ad-
tions. To guarantee that critical services always remain        vanced packet processing capabilities of next-generation
available, today’s applications rely on fault-tolerance tech-   data center network hardware [10, 49, 59]. We achieve
niques like state machine replication. These systems use        similar throughput benefits (albeit with a smaller latency
application-level consensus protocols such as Paxos to          improvement) using an endpoint-based implementation
ensure the consistency of replicas’ states. Unfortunately,      that requires no specialized hardware or network design.
these protocols require expensive coordination on every            By relying on the OUM primitive, NOPaxos avoids
request, imposing a substantial latency penalty and limit-      all coordination except in rare cases, eliminating nearly
ing system scalability. This paper demonstrates that repli-     all the performance overhead of traditional replication
cation in the data center need not impose such a cost by        protocols. It provides throughput within 2% and latency
introducing a new replication protocol with performance         within 16 µs of an unreplicated system, demonstrating
within 2% of an unreplicated system.                            that there need not be a tradeoff between enforcing strong
   It is well known that the communication model funda-         consistency and providing maximum performance.
mentally affects the difficulty of consensus. Completely           This paper makes four specific contributions:
asynchronous and unordered networks require the full
complexity of Paxos; if a network could provide a totally       1. We define the ordered unreliable multicast model for
ordered atomic broadcast primitive, ensuring replica con-          data center networks and argue that it strikes an ef-
sistency would become a trivial matter. Yet this idea has          fective balance between providing semantics strong
yielded few gains in practice since traditional ordered-           enough to be useful to application-level protocols yet
multicast systems are themselves equivalent to consensus;          weak enough to be implemented efficiently.
they simply move the same coordination expense to a
different layer.                                                2. We demonstrate how to implement this network model
   We show that a new division of responsibility between           in the data center by presenting three implementations:



USENIX Association                  12th USENIX Symposium on Operating Systems Design and Implementation             467
    (1) an implementation in P4 [9] for programmable             one replica is the designated leader and assigns an order
    switches, (2) a middlebox-style prototype using a Cav-       to requests. Its normal operation proceeds in four phases:
    ium Octeon network processor, and (3) a software-            clients submit requests to the leader; the leader assigns a
    based implementation that requires no specialized            sequence number and notifies the other replicas; a major-
    hardware but imposes slightly higher latency.                ity of other replicas acknowledge; and the leader executes
                                                                 the request and notifies the client.
3. We introduce NOPaxos, an algorithm which provides
                                                                    These protocols are designed for an asynchronous net-
   state machine replication on an ordered, unreliable net-
                                                                 work, where there are no guarantees that packets will be
   work. Because NOPaxos relies on the OUM primitive,
                                                                 received in a timely manner, in any particular order, or
   it avoids the need to coordinate on every incoming re-
                                                                 even delivered at all. As a result, the application-level
   quest to ensure a total ordering of requests. Instead, it
                                                                 protocol assumes responsibility for both ordering and
   uses application-level coordination only when requests
                                                                 reliability.
   are lost in the network or after certain failures of server
   or network components.                                        The case for ordering without reliable delivery. If
                                                                 the network itself provided stronger guarantees, the full
4. We evaluate NOPaxos on a testbed using our Open-              complexity of Paxos-style replication would be unneces-
   Flow/Cavium prototype and demonstrate that it outper-         sary. At one extreme, an atomic broadcast primitive (i.e., a
   forms classic leader-based Paxos by 54% in latency            virtually synchronous model) [6, 27] ensures both reliable
   and 4.7× in throughput. It simultaneously provides            delivery and consistent ordering, which makes replication
   42% better latency and 24% better throughput than             trivial. Unfortunately, implementing atomic broadcast is a
   latency- and throughput-optimized protocols respec-           problem equivalent to consensus [14] and incurs the same
   tively, circumventing a classic tradeoff.                     costs, merely in a different layer.
                                                                    This paper envisions a middle ground: an ordered but
2     Separating Ordering from Reliable De-                      unreliable network. We show that a new division of re-
      livery in State Machine Replication                        sponsibility – providing ordering in the network layer but
We consider the problem of state machine replication [56].       leaving reliability to the replication protocol – leads to
Replication, used throughout data center applications,           a more efficient whole. What makes this possible is that
keeps key services consistent and available despite the in-      an ordered unreliable multicast primitive can be imple-
evitability of failures. For example, Google’s Chubby [11]       mented efficiently and easily in the network, yet funda-
and Apache ZooKeeper [24] use replication to build a             mentally simplifies the task of the replication layer.
highly available lock service that is widely used to coor-          We note that achieving reliable delivery despite the
dinate access to shared resources and configuration infor-       range of possible failures is a formidable task, and the
mation. It is also used in many storage services to prevent      end-to-end principle suggests that it is best left to the appli-
system outages or data loss [8, 16, 54].                         cation [15, 55]. However, ordering without a guarantee of
   Correctness for state machine replication requires a sys-     reliability permits a straightforward, efficient implemen-
tem to behave as a linearizable [23] entity. Assuming that       tation: assigning sequence numbers to messages and then
the application code at the replicas is deterministic, estab-    discarding those that arrive out of sequence number order.
lishing a single totally ordered set of operations ensures       We show in §3 that this approach can be implemented at
that all replicas remain in a consistent state. We divide        almost no cost in data center network hardware.
this into two separate properties:                                  At the same time, providing an ordering guarantee sim-
                                                                 plifies the replication layer dramatically. Rather than agree
1. Ordering: If some replica processes request a before          on which request should be executed next, it needs to en-
   b, no replica processes b before a.                           sure only all-or-nothing delivery of each message. We
2. Reliable Delivery: Every request submitted by a client        show that this enables a simpler replication protocol that
   is either processed by all replicas or none.                  can execute operations without inter-replica coordination
                                                                 in the common case when messages are not lost, yet can
   Our research examines the question: Can the respon-           recover quickly from lost messages.
sibility for either of these properties be moved from the           Prior work has considered an asynchronous network
application layer into the network?                              that provides ordering and reliability in the common case
State of the art. Traditional state machine replica-             but does not guarantee either. Fast Paxos [36] and related
tion uses consensus protocol – e.g., Paxos [33, 34] or           systems [29, 45, 50] provide agreement in one fewer mes-
Viewstamped Replication [42, 48] – to achieve agree-             sage delay when requests usually arrive at replicas in the
ment on operation order. Most deployments of Paxos-              same order, but they require more replicas and/or larger
based replicated systems use the Multi-Paxos optimiza-           quorum sizes. Speculative Paxos [53] takes this further by
tion [34] (equivalent to Viewstamped Replication), where         having replicas speculatively execute operations without



468    12th USENIX Symposium on Operating Systems Design and Implementation                              USENIX Association
                                          Paxos [33, 34, 48]   Fast Paxos [36]     Paxos+batching     Speculative Paxos [53]      NOPaxos
                    Network ordering             No              Best-effort              No                 Best-effort             Yes
                              Latency            4                   3                    4+                     2                    2
                Messages at bottleneck           2n                  2n                 2 + 2n
                                                                                             b                   2                    2
                         Quorum size            > n/2              > 2n/3                > n/2                 > 3n/4                > n/2
    Reordering/Dropped packet penalty           low               medium                 low                   high                  low

                                          Table 1: Comparison of NOPaxos to prior systems.

coordination, eliminating another message delay and a
throughput bottleneck at the cost of significantly reduced                     Client App           Client App          Replica App
performance (including application-level rollback) when                        libnopaxos     . . . libnopaxos          libnopaxos
                                                                                                                         libnopaxos
                                                                                 libOUM               libOUM               libnopaxos
                                                                                                                            libOUM
the network violates its best-effort ordering property. Our
approach avoids these problems by strengthening network
semantics. Table 1 summarizes the properties of these
protocols.
                                                                                 controller          sequencer                   network
3     Ordered Unreliable Multicast
We have argued for a separation of concerns between or-                                 Figure 1: Architecture of NOPaxos.
dering and reliable delivery. Towards this end, we seek to
design an ordered but unreliable network. In this section,               • Ordered Multicast: The network supports a multicast
we precisely define the properties that this network pro-                  operation such that if two messages, m and m0 , are
vides, and show how it can be realized efficiently using                   multicast to a set of processes, R, then all processes
in-network processing.                                                     in R that receive m and m0 receive them in the same
   We are not the first to argue for a network with ordered                order.
delivery semantics. Prior work has observed that some
                                                                         • Multicast Drop Detection: If some message, m, is
networks often deliver requests to replicas in the same
                                                                           multicast to some set of processes, R, then either: (1)
order [50,58], that data center networks can be engineered
                                                                           every process in R receives m or a notification that
to support a multicast primitive that has this property [53],
                                                                           there was a dropped message before receiving the
and that it is possible to use this fact to design protocols
                                                                           next multicast, or (2) no process in R receives m or
that are more efficient in the common case [29, 36, 53].
                                                                           a dropped message notification for m.1
We contribute by demonstrating that it is possible to build
a network with ordering guarantees rather than proba-                      The asynchrony and unreliability properties are stan-
bilistic or best-effort properties. As we show in §5, doing             dard in network design. Ordered multicast is not: exist-
so can support simpler and more efficient protocols.                    ing multicast mechanisms do not exhibit this property,
   Figure 1 shows the architecture of an OUM/NOPaxos                    although Mostly-Ordered Multicast provides it on a best-
deployment. All components reside in a single data cen-                 effort basis [53]. Importantly, our model requires that any
ter. OUM is implemented by components in the net-                       pair of multicast messages successfully sent to the same
work along with a library, libOUM, that runs on senders                 group are always delivered in the same order to all re-
and receivers. NOPaxos is a replication system that uses                ceivers – unless one of the messages is not received. In
libOUM; clients use libOUM to send messages, and repli-                 this case, however, the receiver is notified.
cas use libOUM to receive clients’ messages.
                                                                        3.2      OUM Sessions and the libOUM API
3.1     Ordered Unreliable Multicast Properties                         Our OUM primitive is implemented using a combination
We begin by describing the basic primitive provided by                  of a network-layer sequencer and a communication library
our networking layer: ordered unreliable multicast. More                called libOUM. libOUM’s API is a refinement of the
specifically, our model is an asynchronous, unreliable                  OUM model described above. An OUM group is a set of
network that supports ordered multicast with multicast                  receivers and is identified by an IP address. We explain
drop detection. These properties are defined as follows:                group membership changes in §5.2.5.
                                                                           libOUM introduces an additional concept, sessions.
• Asynchrony: There is no bound on the latency of mes-                  For each OUM group, there are one or more sessions,
  sage delivery.                                                        which are intervals during which the OUM guarantees
                                                                           1 This second case can be thought of as a sender omission, whereas
• Unreliability: The network does not guarantee that                    the first case can be thought of as a receiver omission, with the added
  any message will ever be delivered to any recipient.                  drop notification guarantee.




USENIX Association                       12th USENIX Symposium on Operating Systems Design and Implementation                                469
    libOUM Sender Interface                                     libOUM library can ensure ordering by discarding mes-
    • send(addr destination, byte[] message) — send a mes-      sages that are received out of order and detect and report
      sage to the given OUM group                               dropped messages by noticing gaps in the sequence num-
                                                                ber.
    libOUM Receiver Interface
    • getMessage() — returns the next message, a DROP -
                                                                   Achieving this design poses three challenges. First, the
      NOTIFICATION , or a SESSION - TERMINATED error            network must serialize all requests through the sequencer;
    • listen(int sessionNum, int messageNum) — resets           we use software-defined networking (SDN) to provide this
      libOUM to begin listening in OUM session sessionNum for   network serialization (§4.1). Second, we must implement
      message messageNum                                        a sequencer capable of high throughput and low latency.
                                                                We present three such implementations in §4.2: a zero-
               Figure 2: The libOUM interface.                  additional-latency implementation for programmable data
hold. Conceptually, the stream of messages being sent           center switches, a middlebox-like prototype using a net-
to a particular group is divided into consecutive OUM           work processor, and a pure-software implementation. Fi-
sessions. From the beginning of an OUM session to the           nally, the system must remain robust to failures of network
time it terminates, all OUM guarantees apply. However,          components, including the sequencer (§4.3).
OUM sessions are not guaranteed to terminate at the same        4.1   Network Serialization
point in the message stream for each multicast receiver:
an arbitrary number of messages at the end of an OUM            The first aspect of our design is network serialization,
session could be dropped without notification, and this         where all OUM packets for a particular group are routed
number might differ for each multicast receiver. Thus,          through a sequencer on the common path. Network seri-
each multicast recipient receives a prefix of the messages      alization was previously used to implement a best-effort
assigned to each OUM session, where some messages are           multicast [53]; we adapt that design here.
replaced with drop notifications.                                  Our design targets a data center that uses software-
   Sessions are generally long-lived. However, rare, excep-     defined networking, as is common today. Data center
tional network events (sequencer failures) can terminate        networks are engineered networks constructed with a par-
them. In this case, the application is notified of session      ticular topology – generally some variant of a multi-rooted
termination and then must ensure that it is in a consistent     tree. A traditional design calls for a three-level tree topol-
state with the other receivers before listening for the next    ogy where many top-of-rack switches, each connecting
session. In this respect, OUM sessions resemble TCP con-        to a few dozen server, are interconnected via aggregation
nections: they guarantee ordering within their lifetime,        switches that themselves connect through core switches.
but failures may cause them to end.                             More sophisticated topologies, such as fat-tree or Clos
   Applications access OUM sessions via the libOUM              networks [1,22,43,46] extend this basic design to support
interface (Figure 2). The receiver interface provides a         large numbers of physical machines using many commod-
getMessage() function, which returns either a message           ity switches and often provide full bisection bandwidth.
or a DROP - NOTIFICATION during an OUM session. When            Figure 3 shows the testbed we use, implementing a fat-
an OUM session terminates, getMessage() returns a               tree network [1].
special value, SESSION - TERMINATED, until the user of             Software-defined networking additionally allows the
libOUM starts the next OUM session. To begin listen-            data center network to be managed by a central controller.
ing to the next OUM session and receiving its messages          This controller can install custom forwarding, filtering,
and DROP - NOTIFICATIONs, the receiver calls listen(int         and rewriting rules in switches. The current generation of
newSessionNum, 0). To start an OUM session at a par-            SDN switches, e.g., OpenFlow [44], allow these rules to
ticular position in the message stream, the receiver can        be installed at a per-flow granularity, matching on a fixed
call listen(int sessionNum, int messageNum). Users              set of packet headers.
of libOUM must ensure that all OUM receivers begin lis-            To implement network serialization, we assign each
tening to the new session in a consistent state.                OUM group a distinct address in the data center network
                                                                that senders can use to address messages to the group. The
4     OUM Design and Implementation                             SDN controller installs forwarding rules for this address
We implement OUM in the context of a single data center         that route messages through the sequencer, then to group
network. The basic design is straightforward: the net-          members.
work routes all packets destined for a given OUM group             To do this, the controller must select a sequencer for
through a single sequencer, a low-latency device that           each group. In the most efficient design, switches them-
serves one purpose: to add a sequence number to each            selves are used as sequencers (§4.2.1). In this case, the
packet before forwarding it to its destination. Since all       controller selects a switch that is a common ancestor of all
packets have been marked with a sequence number, the            destination nodes in the tree hierarchy to avoid increasing



470    12th USENIX Symposium on Operating Systems Design and Implementation                            USENIX Association
path lengths, e.g., a root switch or an aggregation switch               Cavium
                                                                        Processor
if all receivers are in the same subtree. For load balancing,
different OUM groups are assigned different sequencers,
                                                                      Root Layer
e.g., using hash-based partitioning.                                (Arista 7150S)
   Figure 3 shows an example of network serialization for-                          10 Gbps
warding paths in a 12-switch, 3-level fat tree network. Se-          Aggr. Layer
quencers are implemented as network processors (§4.2.2)               (HP 6600)
connected to root switches. Messages from a client ma-                              1 Gbps

chine are first forwarded upward to the designated se-                  ToR Layer
quencer – here, attached to the leftmost root switch – then             (HP 6600)
                                                                                    1 Gbps
distributed downward to all recipients.
   Network serialization could create longer paths than
traditional IP multicast because all traffic must be routed
to the sequencer, but this effect is minimal in practice. We
                                                                Figure 3: Testbed network topology. Green lines indicate the
quantified this latency penalty using packet-level network
                                                                upward path from a client to the sequencer, and orange lines
simulation. The simulated network contained 2,560 end-          indicate the downward path from the sequencer to receivers.
hosts and 119 switches configured in a 3-level fat tree
network, with background traffic modeled on Microsoft           switch itself as the sequencer, incurring no latency cost
data centers [4]. Each client sent multicast messages to        (§4.2.1). As this hardware is not yet available, we describe
a random group of 5 receivers. In 88% of cases, net-            a prototype that uses a network processor to implement
work serialization added no additional latency for the          a middlebox-like sequencer (§4.2.2). Finally, we discuss
message to be received by a quorum of 3 receivers; the          using an end-host as a sequencer (§4.2.3).
99th-percentile was less than 5 µs of added latency. This
                                                                4.2.1     In-Switch Sequencing
minimal increase in latency is due to the fact that the se-
quencer is a least- common-ancestor switch of the replica       Ideally, switches themselves could serve as sequencers.
group, and most packets have to traverse that switch any-       The benefit of doing so is latency: packets could be se-
way to reach a majority of the group.                           quenced by one of the switches through which they al-
                                                                ready transit, rather than having to be redirected to a
4.2   Implementing the Sequencer
                                                                dedicated device. Moreover, switching hardware is highly
The sequencer plays a simple but critical role: assigning a     optimized for low-latency packet processing, unlike end-
sequence number to each message destined for a particular       hosts.
OUM group, and writing that sequence number into the               Using a switch as a sequencer is made possible by the
packet header. This establishes a total order over packets      increasing ability of data center switches to perform flexi-
and is the key element that elevates our design from a          ble, per-packet computations. An emerging class of switch
best-effort ordering property to an ordering guarantee.         architectures – such as Reconfigurable Match Tables [10],
Even if packets are dropped (e.g., due to congestion or         Intel’s FlexPipe [49], and Cavium’s XPliant [59] – allow
link failures) or reordered (e.g., due to multipath effects)    the switch’s behavior to be controlled on a per-packet
in the network, receivers can use the sequence numbers          granularity, supporting the parsing and matching of ar-
to ensure that they process packets in order and deliver        bitrary packet fields, rewriting of packet contents, and
drop notifications for missing packets.                         maintaining of small amounts of state between packets.
   Sequencers maintain one counter per OUM group. For           Exposed through high-level languages like P4 [9], this
every packet destined for that group, they increment the        increased flexibility lets us consider network switches
counter and write it into a designated field in the packet      as not simply forwarding elements, but as devices with
header. The counter must be incremented by 1 on each            computational ability.
packet (as opposed to a timestamp, which monotonically             We implemented our switch sequencing functionality
increases but may have gaps). This counter lets libOUM          in the P4 language, which allows it to be compiled and
return DROP - NOTIFICATIONs when it notices gaps in the         deployed to upcoming programmable switches as well as
sequence numbers of incoming messages. Sequencers               software switches. Our implementation uses the recon-
also maintain and write into each packet the OUM ses-           figurable parser capabilities of these switches to define a
sion number that is used to handle sequencer failures; we       custom packet header that includes the OUM sequence
describe its use in §4.3.                                       and session numbers. It uses stateful memory (register
   Our sequencer design is general; we discuss three pos-       arrays) to store the current sequence number for every
sible implementations here. The most efficient one targets      OUM group and increments it on each packet. Complete
upcoming programmable network switches, using the               NOPaxos P4 code is available in [39].



USENIX Association                  12th USENIX Symposium on Operating Systems Design and Implementation                471
   Programmable switches capable of this processing are         latency, it allows the OUM abstraction to be implemented
not yet commercially available, although we expect them         without any specialized hardware. Nevertheless, using a
to be within the next year. Therefore, we cannot evaluate       dedicated host for network-level sequencing can still pro-
their performance, but there is reason to believe they can      vide throughput, if not latency, benefits as we demonstrate
execute this processing with no measurable increase in la-      in §6. We implemented a simple Linux program that uses
tency. As evidence, Intel’s FlexPipe chips (now available,      raw sockets to access packet headers.
e.g., in the Arista 7150 switch) can modify packets to in-
                                                                4.2.4    Sequencer Scalability
clude the egress timestamp with zero latency cost [2, 49].
   We note that a network switch provides orders-of-            Since all OUM packets for a particular group go through
magnitude lower latency and greater reliability [21] than       the sequencer, a valid concern is whether the sequencer
an end-host. Today’s fastest cut-through switches can con-      will become the performance bottleneck. Switches and
sistently process packets in approximately 300 ns [2],          network processors are designed to process packets at line
while a typical Linux server has median latency in the 10–      rate and thus will not become the bottleneck for a single
100 µs range and 99.9th-percentile latency over 5 ms [40].      OUM group (group receivers are already limited by the
This trend seems unlikely to change: even with high-            link bandwidth). Previous work [28] has demonstrated
performance server operating systems [3,52], NIC latency        that an end-host sequencer using RDMA can process
remains an important factor [20]. At the same time, the         close to 100 million requests per second, many more
limited computational model of the switch requires a care-      than any single OUM group can process. We note that
ful partitioning of functionality between the network and       different OUM groups need not share a sequencer, and
application. The OUM model offers such a design.                therefore deployment of multiple OUM groups can scale
                                                                horizontally.
4.2.2    Hardware Middlebox Prototype Sequencing
                                                                4.3     Fault Tolerance
Because available switches do not provide the necessary
flexibility to run P4 programs, we implemented a proto-         Designating a sequencer and placing it on the common
type using existing OpenFlow switches and a network             path for all messages to a particular group introduces an
processor.                                                      obvious challenge: what if it fails or becomes unreach-
   This prototype is part of the testbed that we use to eval-   able? If link failures or failures of other switches render
uate our OUM model and its uses for distributed protocols.      the sequencer unreachable, local rerouting mechanisms
This testbed simulates the 12-switch, 3-layer fat-tree net-     may be able to identify an alternate path [43]. However, if
work configuration depicted in Figure 3. We implemented         the sequencer itself fails, or local rerouting is not possible,
it on three physical switches by using VLANs and ap-            replacing the sequencer becomes necessary.
propriate OpenFlow forwarding rules to emulate separate            In our design, the network controller monitors the se-
virtual switches: two HP 6600 switches implement the            quencer’s availability. If it fails or no longer has a path
ToR and aggregation tiers, and one Arista 7050S switch          to all OUM group members, the controller selects a dif-
implements the core tier.                                       ferent switch. It reconfigures the network to use this new
   We implemented the sequencer as a form of middlebox          sequencer by updating routes in other switches. During
using a Cavium Octeon II CN68XX network processor.              the reconfiguration period, multicast messages may not
This device contains 32 MIPS64 cores and supports 10            be delivered. However, failures of root switches happen
Gb/s Ethernet I/O. Users can customize network func-            infrequently [21], and rerouting can be completed within
tionality by loading C binaries that match, route, drop         a few milliseconds [43], so this should not significantly
or modify packets going through the processor. Onboard          affect system availability.
DRAM maintains per-group state. We attached the mid-               We must also ensure that the ordering guarantee of
dlebox to the root switches and installed OpenFlow rules        multicast messages is robust to sequencer failures. This
to redirect OUM packets to the middlebox.                       requires the continuous, ordered assignment of sequence
   This implementation does not provide latency as low          numbers even when the network controller fails over to a
as the switch-based sequencer; routing traffic through          new sequencer.
the network processor adds latency. We measured this               To address this, we introduce a unique, monotonically
latency to be 8 µs in the median case and 16 µs in the          increasing session number, incremented each time se-
99th percentile. This remains considerably lower than           quencer failover occurs. When the controller detects a
implementing packet processing in an end-host.                  sequencer failure, it updates the forwarding rules and
                                                                contacts the new sequencer to set its local session num-
4.2.3    End-host Sequencing
                                                                ber to the appropriate value. As a result, the total or-
Finally, we also implemented the sequencing function-           der of messages follows the lexicographical order of the
ality on a conventional server. While this incurs higher        hsession-number, sequence-numberi tuple, and clients



472     12th USENIX Symposium on Operating Systems Design and Implementation                           USENIX Association
can still discard packets received out of order.                  5.2    Protocol
    Once libOUM receives a message with a session num-            Overview. NOPaxos is built on top of the guarantees
ber higher than the receiver is listening for, it realizes that   of the OUM network primitive. During a single OUM
a new sequencer is active and stops delivering messages           session, REQUESTs broadcast to the replicas are totally
from the old session. However, libOUM does not know if            ordered but can be dropped. As a result, the replicas have
it missed any packets from the old sequencer. As a result,        to agree only on which REQUESTs to execute and which
it cannot deliver DROP - NOTIFICATIONs during a session           to permanently ignore, a simpler task than agreeing on
change. Instead, it delivers a SESSION - TERMINATED no-           the order of requests. Conceptually, this is equivalent to
tification, exposing this uncertainty to the application.         running multiple rounds of binary consensus. However,
NOPaxos, for example, resolves this by executing a view           NOPaxos must explicitly run this consensus only when
change (§5.2.3) so that replicas agree on exactly which           DROP - NOTIFICATIONs are received. To switch OUM ses-
requests were received in the old session.                        sions (in the case of sequencer failure), the replicas must
    The network controller must ensure that session num-          agree on the contents of their shared log before they start
bers for any given group monotonically increase, even             listening to the new session.
across controller failures. Many design options are avail-           To these ends, NOPaxos uses a view-based approach:
able, for example using timestamps as session numbers, or         each view has a single OUM session-num and a single
recording session numbers in stable or replicated storage.        replica acting as leader. The leader executes requests and
Our implementation uses a Paxos-replicated controller             drives the agreement to skip a dropped request. That is,
group, since SDN controller replication is already com-           it decides which of the sequencer’s REQUESTs to ignore
mon in practice [26, 31]. We note that our replication            and treat as NO - OPs. The view ID is a tuple hleader-num,
protocol, NOPaxos (§5), is completely decoupled from              session-numi. Here, leader-num is incremented each time
controller replication, and the controller updates only on        a new leader is needed; the current leader of any view is
sequencer failures, not for every NOPaxos request.                leader-num (mod n); and session-num is the latest ses-
                                                                  sion ID from libOUM. View IDs in NOPaxos are partially
5     NOPaxos                                                     ordered.2 However, the IDs of all views that successfully
NOPaxos, or Network-Ordered Paxos, is a new replication           start will be comparable.
protocol which leverages the Ordered Unreliable Multi-               In the normal case, the replicas receive a REQUEST
cast sessions provided by the network layer.                      from libOUM. The replicas then reply to the client, the
                                                                  leader replying with the result of the REQUEST, so the
5.1   Model                                                       client’s REQUEST is processed in only a single round-trip.
                                                                  NOPaxos uses a single round-trip in the normal case be-
NOPaxos replicas communicate over an asynchronous
                                                                  cause, like many speculative protocols, the client checks
network that provides OUM sessions (via libOUM).
                                                                  the durability of requests. However, unlike most specula-
NOPaxos requires the network to provide ordered but
                                                                  tive protocols, NOPaxos clients have a guarantee regard-
unreliable delivery of multicast messages within a ses-
                                                                  ing ordering of operations; they need only check that the
sion. In the normal case, these messages are deliv-
                                                                  operation was received.
ered sequentially and are not dropped; however, it re-
                                                                     When replicas receive a DROP - NOTIFICATION from
mains robust to dropped packets (presented as DROP -
                                                                  libOUM, they first try to recover the missing REQUEST
NOTIFICATION through libOUM). NOPaxos is also robust
                                                                  from each other. Failing that, the leader initiates a round of
to SESSION - TERMINATED notifications that occur if the
                                                                  agreement to commit a NO - OP into the corresponding slot
sequencer fails. These network anomalies do not affect
                                                                  in the log. Finally, NOPaxos uses a view change protocol
NOPaxos’s safety guarantees, and we discuss how they
                                                                  to handle leader failures and OUM session termination
affect NOPaxos’s performance in §6.
                                                                  while maintaining consistency.
   NOPaxos assumes a crash failure model. It uses 2 f + 1
replicas, where f replicas are allowed to fail. However, in       Outline. NOPaxos consists of four subprotocols:
the presence of more than f failures, the system still guar-
antees safety. Furthermore, NOPaxos guarantees safety             • Normal Operations (§5.2.1): NOPaxos processes
even in an asynchronous network with no bound on mes-               client REQUESTs in a single round-trip in the normal
sage latency (provided the OUM guarantees continue to               case.
hold).
                                                                  • Gap Agreement (§5.2.2): NOPaxos ensures correctness
   NOPaxos provides linearizability of client requests. It
                                                                    in the face of DROP - NOTIFICATIONs by having the
provides at-most-once semantics using the standard mech-
anism of maintaining a table of the most recent request              2 That is, v ≤ v iff both v ’s leader-num and session-num are less
                                                                                 1   2          1
from each client [42].                                            than or equal to v2 ’s.




USENIX Association                   12th USENIX Symposium on Operating Systems Design and Implementation                         473
  Replica:                                                        5.2.2    Gap Agreement
   • replica-num — replica number
                                                                  NOPaxos replicas always process operations in order.
   • status — one of Normal or ViewChange
                                                                  When a replica receives a DROP - NOTIFICATION from
   • view-id = hleader-num, session-numi — the view number, a
     tuple of the current leader number and OUM session number,   libOUM (and increments its session-msg-num), it must
     partially ordered, initially h0, 0i                          either recover the contents of the missing request or pre-
   • session-msg-num — the number of messages (REQUESTs or        vent it from succeeding before moving on to subsequent
     DROP - NOTIFICATION s) received in this OUM session          requests. Non-leader replicas do this by contacting the
   • log — client REQUESTs and NO - OPs in sequential order       leader for a copy of the request. If the leader itself re-
   • sync-point — the latest synchronization point                ceives a DROP - NOTIFICATION, it coordinates to commit
                                                                  a NO - OP operation in place of that request:
          Figure 4: Local state of NOPaxos replicas.

   replicas reach agreement on which sequence numbers             1. If the leader receives a DROP - NOTIFICATION, it in-
   should be permanently dropped.                                    serts a NO - OP into its log and sends a hGAP - COMMIT,
                                                                     log-sloti to the other replicas, where log-slot is the slot
                                                                     into which the NO - OP was inserted.
• View Change (§5.2.3): NOPaxos ensures correctness in
  the face of leader failures or OUM session termination
                                                                  2. When a non-leader replica receives the GAP - COMMIT
  using a variation of a standard view change protocol.
                                                                     and has filled all log slots up to the one specified by the
                                                                     leader, it inserts a NO - OP into its log at the specified lo-
• Synchronization (§5.2.4): Periodically, the leader syn-            cation3 (possibly overwriting a REQUEST) and replies
  chronizes the logs of all replicas.                                to the leader with a hGAP - COMMIT- REP, log-sloti.

   Figure 4 illustrates the state maintained at each              3. The leader waits for f GAP - COMMIT- REPs (retrying if
NOPaxos replica. Replicas tag all messages sent to each              necessary).
other with their current view-id, and while in the Nor-
mal Operations, Gap Agreement, and Synchronization
                                                                     Clients need not be notified explicitly when a NO - OP
subprotocols, replicas ignore all messages from different
                                                                  has been committed in place of one of their requests.
views. Only in the View Change protocol do replicas with
                                                                  They simply retry their request after failing to receive a
different view-ids communicate.
                                                                  quorum of responses. Note that the retried operation will
5.2.1    Normal Operations                                        be considered a new request and will have a new slot in the
                                                                  replicas’ logs. Replicas identify duplicate client requests
In the normal case when replicas receive REQUESTs in-             by checking if they have processed another request with
stead of DROP - NOTIFICATIONs, client requests are com-           the same client-id and request-id, as is commonly done
mitted and executed in a single phase. Clients broad-             in other protocols.
cast hREQUEST, op, request-idi to all replicas through               This protocol ensures correctness because clients do
libOUM, where op is the operation they want to execute,           not consider an operation completed until they receive
and request-id is a unique id used to match requests and          a response from the leader, so the leader can propose a
their responses.                                                  NO - OP regardless of whether the other replicas received
   When each replica receives the client’s REQUEST, it            the REQUEST. However, before proceeding to the next
increments session-msg-num and appends op to the log. If          sequence number, the leader must ensure that a majority
the replica is the leader of the current view, it executes the    of replicas have learned of its decision to commit a NO -
op (or looks up the previous result if it is a duplicate of a     OP . When combined with the view change protocol, this
completed request). Each replica then replies to the client       ensures that the decision persists even if the leader fails.
with hREPLY, view-id, log-slot-num, request-id, resulti,             As an optimization, the leader can first try to contact
where log-slot-num is the index of op in the log. If the          the other replicas to obtain a copy of the REQUEST and ini-
replica is the leader, it includes the result of the operation;   tiate the gap commit protocol only if no replicas respond
NULL otherwise.                                                   before a timeout. While not necessary for correctness, this
   The client waits for REPLYs to the REQUEST with                reduces the number of NO - OPs.
matching view-ids and log-slot-nums from f + 1 repli-
                                                                     3 If the replica had not already filled log-slot in its log or received a
cas, where one of those replicas is the leader of the view.
                                                                  DROP - NOTIFICATION for that slot when it inserted the NO - OP, it ignores
This indicates that the request will remain persistent even       the next REQUEST or DROP - NOTIFICATION from libOUM (and incre-
across view changes. If the client does not receive the           ments session-msg-num), maintaining consistency between its position
required REPLYs within a timeout, it retries the request.         in the OUM session and its log.




474     12th USENIX Symposium on Operating Systems Design and Implementation                                      USENIX Association
5.2.3   View Change                                                     Normal.4 For each slot in the log, the merged result
                                                                        is a NO - OP if any log has a NO - OP. Otherwise, the
During each view, a NOPaxos group has a particular                      result is a REQUEST if at least one has a REQUEST.
leader and OUM session number. NOPaxos must perform                     It then updates its log to the merged one.
view changes to ensure progress in two cases: (1) when
the leader is suspected of having failed (e.g, by failing to       • The leader sets its view-id to the one from the VIEW-
                                                                     CHANGE messages and its session-msg-num to the
respond to pings), or (2) when a replica detects the end
of an OUM session. To successfully replace the leader                highest out of all the messages used to form the
or move to a new OUM session, NOPaxos runs a view                    merged log.
change protocol. This protocol ensures that all successful         • It then sends hSTART- VIEW, view-id,
operations from the old view are carried over into the new           session-msg-num, logi to all replicas (includ-
view and that all replicas start the new view in a consistent        ing itself).
state.
                                                                3. When a replica receives a START- VIEW message with
   NOPaxos’s view change protocol resembles that used
                                                                   a view-id greater than or equal to its current view-id,
in Viewstamped Replication [42]. The principal differ-
                                                                   it first updates its view-id, log, and session-msg-num
ence is that NOPaxos views serve two purposes, and so
                                                                   to the new values. It then calls listen(session-num,
NOPaxos view IDs are therefore a tuple of hleader-num,
                                                                   session-msg-num) in libOUM. The replica sends RE -
session-numi rather than a simple integer. A view change
                                                                   PLY s to clients for all new REQUESTs added to its log
can increment either one. However, NOPaxos ensures
                                                                   (executing them if the replica is the new leader). Fi-
that each replica’s leader-num and session-num never go
                                                                   nally, the replica sets its status to Normal and begins
backwards. This maintains a total order over all views
                                                                   receiving messages from libOUM again.5
that successfully start.
                                                                5.2.4     Synchronization
1. A replica initiates a view change when: (1) it sus-          During any view, only the leader executes operations and
   pects that the leader in its current view has failed;        provides results. Thus, all successful client REQUESTs are
   (2) it receives a SESSION - TERMINATED notification          committed on a stable log at the leader, which contains
   from libOUM; or (3) it receives a VIEW- CHANGE or            only persistent client REQUESTs. In contrast, non-leader
   VIEW- CHANGE - REQ message from another replica              replicas might have speculative operations throughout
   with a higher leader-num or session-num. In all cases,       their logs. If the leader crashes, the view change protocol
   the replica appropriately increments the leader-num          ensures that the new leader first recreates the stable log
   and/or session-num in its view-id and sets its status        of successful operations. However, it must then execute
   to ViewChange. If the replica incremented its session-       all operations before it can process new ones. While this
   num, it resets its session-msg-num to 0.                     protocol is correct, it is clearly inefficient.
                                                                   Therefore, as an optimization, NOPaxos periodically
   It then sends hVIEW- CHANGE - REQ, view-idi to the           executes a synchronization protocol in the background.
   other replicas and hVIEW- CHANGE, view-id, v0 ,              This protocol ensures that all other replicas learn which
   session-msg-num, logi to the leader of the new view,         operations have successfully completed and which the
   where v0 is the view ID of the last view in which its        leader has replaced with NO - OPs. That is, synchronization
   status was Normal. While in ViewChange status, the           ensures that all replicas’ logs are stable up to their sync-
   replica ignores all replica-to-replica messages (except      point and that they can safely execute all REQUESTs up
   START- VIEW , VIEW- CHANGE , and VIEW- CHANGE -              to this point in the background.
   REQ ).                                                          For brevity, we omit the details of this protocol. See
                                                                [39] for the full specification.
   If the replica ever times out waiting for the view change
   to complete, it simply rebroadcasts the VIEW- CHANGE         5.2.5     Recovery and Reconfiguration
   and VIEW- CHANGE - REQ messages.                             While the NOPaxos protocol as presented above assumes
                                                                a crash failure model and a fixed replica group, it can
2. When the leader for the new view receives f + 1 VIEW-           4 While view-ids are only partially ordered, because individual repli-

   CHANGE messages (including one from itself) with             cas’ view-ids only increase and views require a quorum of replicas to
                                                                start, all views that successfully start are comparable – so identifying
   matching view-ids, it performs the following steps:          the view with the highest number is in fact meaningful. For a full proof
                                                                of this fact, see [39].
                                                                   5 Replicas also send an acknowledgment to the leader’s START- VIEW
   • The leader merges the logs from the most recent            message, and the leader periodically resends the START- VIEW to those
     (largest) view in which the replicas had status            replicas from whom it has yet to receive an acknowledgment.




USENIX Association                  12th USENIX Symposium on Operating Systems Design and Implementation                           475
also facilitate recovery and reconfiguration using adapta-     transfer application state from another replica, rather than
tions of standard mechanisms (e.g. Viewstamped Repli-          application-level rollback.
cation [42]). While the recovery mechanism is a direct            Finally, unlike many replication protocols, NOPaxos
equivalent of the Viewstamped Replication protocol, the        replicas send and receive a constant number of messages
reconfiguration protocol additionally requires a member-       for each REQUEST in the normal case, irrespective of the
ship change in the OUM group. The OUM membership is            total number of replicas. This means that NOPaxos can be
changed by contacting the controller and having it install     deployed with an increasing number of replicas without
new forwarding rules for the new members, as well as a         the typical performance degradation, allowing for greater
new session-num in the sequencer (terminating the old          fault-tolerance. §6.3 demonstrates that NOPaxos achieves
session). The protocol then ensures all members of the         the same throughput regardless of the number of replicas.
new configuration start in a consistent state.
                                                               5.4   Correctness
5.3   Benefits of NOPaxos
                                                               NOPaxos guarantees linearizability: that operations sub-
NOPaxos achieves the theoretical minimum latency and           mitted by multiple concurrent clients appear to be exe-
maximum throughput: it can execute operations in one           cuted by a single, correct machine. In a sense, correct-
round-trip from the client to the replicas and does not        ness in NOPaxos is a much simpler property than in
require replicas to coordinate on each request. By relying     other systems, such as Paxos and Viewstamped Repli-
on the network to stamp requests with sequence numbers,        cation [33, 48], because the replicas need not agree on the
it requires replies only from a simple majority of repli-      order of the REQUESTs they execute. Since the REQUEST
cas and uses a cheaper and rollback-free mechanism to          order is already provided by the guarantees of OUM ses-
correctly account for network anomalies.                       sions, the replicas must only agree on which REQUESTs
    The OUM session guarantees mean that the replicas          to execute and which REQUESTs to drop.
already agree on the ordering of all operations. As a con-        Below, we sketch the proof of correctness for the
sequence, clients need not wait for a superquorum of           NOPaxos protocol. For a full, detailed proof, see [39].
replicas to reply, as in Fast Paxos and Speculative Paxos      Additionally, see [39] for a TLA+ specification of the
(and as is required by any protocol that provides fewer        NOPaxos protocol.
message delays than Paxos in an asynchronous, unordered
                                                               Definitions. We say that a REQUEST or NO - OP is com-
network [37]). In NOPaxos, a simple majority of replicas
                                                               mitted in a log slot if it is processed by f + 1 replicas with
suffices to guarantee the durability of a REQUEST in the
                                                               matching view-ids, including the leader of that view. We
replicas’ shared log.
                                                               say that a REQUEST is successful if it is committed and
    Additionally, the OUM guarantees enable NOPaxos
                                                               the client receives the f + 1 suitable REPLYs. We say a
to avoid expensive mechanisms needed to detect when
                                                               log is stable in view v if it will be a prefix of the log of
replicas are not in the same state, such as using hashing to
                                                               every replica in views higher than v.
detect conflicting logs from replicas. To keep the replicas’
logs consistent, the leader need only coordinate with the      Sketch of Proof. During a view, a leader’s log grows
other replicas when it receives DROP - NOTIFICATIONs.          monotonically (i.e., entries are only appended and never
Committing a NO - OP takes but a single round-trip and         overwritten). Also, leaders execute only the first of du-
requires no expensive reconciliation protocol.                 plicate REQUESTs. Therefore, to prove linearizability it
    NOPaxos also avoids rollback, which is usually neces-      is sufficient to show that: (1) every successful operation
sary in speculative protocols. It does so not by coordinat-    was appended to a stable log at the leader and that the
ing on every operation, as in non-speculative protocols,       resulting log is also stable, and (2) replicas always start a
but by having only the leader execute operations. Non-         view listening to the correct session-msg-num in an OUM
leader replicas do not execute requests during normal          session (i.e., the message corresponding to the number of
operations (except, as an optimization, when the synchro-      REQUESTs or NO - OPs committed in that OUM session).
nization protocol indicates it is safe to do so), so they         First, note that any REQUEST or NO - OP that is com-
need not rollback. The leader executes operations spec-        mitted in a log slot will stay in that log slot for all future
ulatively, without coordinating with the other replicas,       views: it takes f + 1 replicas to commit a view and f + 1
but clients do not accept a leader’s response unless it is     replicas to complete a view change, so, by quorum inter-
supported by matching responses from f other replicas.         section, at least one replica initiating the view change will
The only rare case when a replica will execute an opera-       have received the REQUEST or NO - OP. Also, because it
tion that is not eventually committed is if a functioning      takes the leader to commit a REQUEST or NO - OP and its
leader is incorrectly replaced through a view change, los-     log grows monotonically, only a single REQUEST or NO -
ing some operations it executed. Because this case is rare,    OP is ever committed in the same slot during a view. There-
it is reasonable to handle it by having the ousted leader      fore, any log consisting of only committed REQUESTs and



476   12th USENIX Symposium on Operating Systems Design and Implementation                            USENIX Association
NO - OPs is stable.                                                               1200
                                                                                                                                            Paxos
   Next, every view that starts (i.e., f + 1 replicas receive                     1000
                                                                                                                                       Fast Paxos
                                                                                                                                         Batching
the START- VIEW and enter Normal status) trivially starts                          800
                                                                                                                                       SpecPaxos
                                                                                                                                        NOPaxos


                                                                   Latency (µs)
with a log containing only committed REQUESTs and NO -                                                                                Unreplicated
                                                                                   600
OP s. Replicas send REPLY s to a REQUEST only after all
log slots before the REQUEST’s slot have been filled with                          400

REQUEST s or NO - OP s; further, a replica inserts a NO - OP                       200
only if the leader already inserted that NO - OP. Therefore,
                                                                                        0
if a REQUEST is committed, all previous REQUESTs and                                     0K   50K      100K   150K   200K    250K      300K    350K    400K
NO - OPs in the leader’s log were already committed.                                                           Throughput (ops/sec)

   This means that any REQUEST that is successful in             Figure 5: Latency vs. throughput comparison for testbed deploy-
a view must have been appended to a stable log at the            ment of NOPaxos and other protocols.
leader, and the resulting log must also be stable, showing
(1). To see that (2) is true, notice that the last entry in                       700                             NOPaxos
                                                                                              NOPaxos + End-host Sequencer
the combined log formed during a view change and the                              600                         Unreplicated
session-msg-num are taken from the same replica(s) and                            500


                                                                   Latency (µs)
therefore must be consistent.                                                     400
   NOPaxos also guarantees liveness given a sufficient                            300
amount of time during which the following properties                              200
hold: the network over which the replicas communicate                             100
is fair-lossy; there is some bound on the relative pro-                             0
cessing speeds of replicas; there is a quorum of repli-                              0K          50K          100K       150K           200K         250K
                                                                                                              Throughput (ops/sec)
cas that stays up; there is a replica that stays up that no
replica suspects of having failed; all replicas correctly sus-   Figure 6: Comparison of running NOPaxos with the prototype
pect crashed nodes of having failed; no replica receives         Cavium sequencer and an end-host sequencer.
a DROP - NOTIFICATION or SESSION - TERMINATED from               6.1              Latency vs. Throughput
libOUM; and clients’ REQUESTs eventually get delivered
through libOUM.                                                  To compare the latency and throughput of NOPaxos and
                                                                 the other four protocols, we ran each system with an
6   Evaluation                                                   increasing number of concurrent closed-loop clients. Fig-
                                                                 ure 5 shows results of this experiment. NOPaxos achieves
We implemented the NOPaxos protocol in approximately             a much higher maximum throughput than Paxos and Fast
5,000 lines of C++ code. We ran our experiments using            Paxos (370% increases in both cases) without any addi-
the 3-level fat-tree network testbed shown in Figure 3.          tional latency. The leaders in both Paxos and Fast Paxos
All clients and replicas ran on servers with 2.5 GHz Intel       send and receive more messages than the other replicas,
Xeon E5-2680 processors and 64GB of RAM. All exper-              and the leaders’ message processing quickly becomes the
iments used five replicas (thereby tolerating two replica        bottleneck of these systems. NOPaxos has no such inef-
failures).                                                       ficiency. NOPaxos also achieves higher throughput than
   To evaluate the performance of NOPaxos, we com-               Speculative Paxos (24% increase) because Speculative
pared it to four other replication protocols: Paxos, Fast        Paxos requires replicas to compute hashes of their logs
Paxos, Paxos with batching, and Speculative Paxos; we            for each client request.
also evaluated it against an unreplicated system that pro-          Figure 5 also shows that NOPaxos has lower latency
vides no fault tolerance. Like NOPaxos, the clients in           (111 µs) than Paxos (240 µs) and Fast Paxos (193 µs) be-
both Speculative Paxos and Fast Paxos multicast their            cause NOPaxos requires fewer message delays in the nor-
requests to the replicas through a root serialization switch     mal case. Speculative Paxos also has higher latency than
to minimize message reordering. Requests from NOPaxos            NOPaxos because clients must wait for a superquorum of
clients, however, are also routed through the Cavium pro-        replica replies instead of NOPaxos’s simple quorum.
cessor to be stamped with the sequencer’s OUM session               Batching improves Paxos’s throughput by reducing the
number and current request sequence number. For the              number of messages sent by the leader. Paxos with batch-
batching variant of Paxos, we used a sliding-window tech-        ing is able to reach a maximum throughput equivalent
nique where the system adaptively adjusts the batch size,        to Speculative Paxos. However, batching also increases
keeping at least one batch in progress at all times; this        the latency of Paxos (385 µs at low load and 907 µs
approach reduces latency at low load while still providing       at maximum throughput). NOPaxos attains both higher
throughput benefits at high load [13].                           throughput and lower latency than Paxos with batching.



USENIX Association                   12th USENIX Symposium on Operating Systems Design and Implementation                                                     477
                         350K
                                         Paxos        Batching              NOPaxos                                                      Paxos           Batching             NOPaxos
                                    Fast Paxos       SpecPaxos            Unreplicated                                              Fast Paxos          SpecPaxos           Unreplicated
                         300K




  Throughput (ops/sec)                                                                          Throughput (ops/sec)
                         250K

                         200K                                                                                          100K

                         150K

                         100K

                         50K

                          0K                                                                                           10K
                           0.001%                0.01%                0.1%               1%                                   3                   5                         7                 9
                                                    Simulated drop rate                                                                               Number of Replicas

Figure 7: Maximum throughput with simulated packet dropping.                                  Figure 8: Maximum throughput with increasing number of repli-
                                                                                              cas.
   NOPaxos is able to attain throughput within 2% of an
unreplicated system and latency within 16 µs. However,                                                                 300K
we note that our middlebox prototype adds around 8 µs


                                                                                                Throughput (ops/sec)
                                                                                                                       250K
to NOPaxos’s latency. We envision that implementing                                                                    200K

the sequencer in a switch could bring NOPaxos’s latency                                                                150K

even closer to the unreplicated system. This demonstrates                                                              100K

that NOPaxos can achieve close to optimal performance                                                                  50K

while providing fault-tolerance and strong consistency.                                                                 0K
                                                                                                                                  -0.25     0    0.25      0.5      0.75        1    1.25   1.5
   We also evaluated the performance of NOPaxos when                                                                                                       Time (s)
using an end-host as the sequencer instead of the network
                                                                                               Figure 9: NOPaxos throughput during a sequencer failover.
processor. Figure 6 shows that NOPaxos still achieves
impressive throughput when using an end-host sequencer,                                       Fast Paxos suffer throughput degradation proportional
though at a cost of 36% more latency due to the additional                                    to the number of replicas because the leaders in those
message delay required.                                                                       protocols have to process more messages from the ad-
6.2                       Resilience to Network Anomalies                                     ditional replicas. Replicas in NOPaxos and Speculative
                                                                                              Paxos, however, process a constant number of messages,
To test the performance of NOPaxos in an unreliable                                           so those protocols maintain their throughput when more
network, we randomly dropped a fraction of all packets.                                       replicas are added.
Figure 7 shows the maximum throughput of the five pro-
tocols and the unreplicated system with an increasing                                         6.4                       Sequencer Failover
packet drop rate. Paxos’s and Fast Paxos’s throughput                                         NOPaxos relies on the sequencer to order client requests.
do not decrease significantly, while Paxos with batching                                      We measured the throughput of NOPaxos during a se-
shows a larger drop in throughput due to frequent state                                       quencer failover (Figure 9). We ran NOPaxos at peak
transfers. However, the throughput of Speculative Paxos                                       throughput for approximately 7 seconds. We then sim-
drops substantially after 0.5% packet dropping, demon-                                        ulated a sequencer failure by sending the controller a
strating NOPaxos’s largest advantage over Speculative                                         notification message. The controller modified the routing
Paxos. When 1% of packets are dropped, Speculative                                            rules in the network and installed a new session number
Paxos’s maximum throughput falls to that of Paxos. As                                         in the sequencer (as described in §3). The throughput of
discussed in §5.3, Speculative Paxos performs an expen-                                       the system drops to zero during the failover and takes
sive reconciliation protocol when messages are dropped                                        approximately 110 ms to resume normal operations and
and replica states diverge. NOPaxos is much more re-                                          approximately 270 ms to resume processing operations
silient to packet drops and reorderings. It achieves higher                                   at peak throughput. Most of this delay is caused by the
throughput than Paxos with batching and much higher                                           route update rather than the NOPaxos view change.
throughput than Speculative Paxos at high drop rates.                                         6.5                       Application Performance
Even with a 1% message drop rate, NOPaxos’s throughput
does not drop significantly. Indeed, NOPaxos maintains                                        To further demonstrate the benefits of the NOPaxos pro-
throughput roughly equivalent to an unreplicated system,                                      tocol, we evaluated the performance of a distributed, in-
demonstrating its strong resilience to network anomalies.                                     memory key-value store. The key-value store uses two-
                                                                                              phase commit and optimistic concurrency control to sup-
6.3                       Scalability                                                         port serializable transactions, and each shard runs atop our
To test NOPaxos’s scalability, we measured the maximum                                        replication framework. Clients issue GET and PUT requests
throughput of the five protocols running on increasing                                        within transactions. We benchmarked the key-value store
number of replicas. Figure 8 shows that both Paxos and                                        using a workload based on the Retwis Twitter clone [38].



478                        12th USENIX Symposium on Operating Systems Design and Implementation                                                                            USENIX Association
                                14000
                                                                                                                responsibility between an ordered but unreliable commu-
                                                                                                                nications layer and an application-level reliability layer. In



    Max Throughput (txns/sec)
                                12000
                                10000                                                                           particular, the choice to leave reliability to the application
                                8000                                                                            is inspired by the end-to-end argument [15, 55].
                                6000
                                4000
                                                                                                                Network-level processing NOPaxos takes advantage
                                2000
                                                                                                                of flexible network processing to implement the OUM
                                    0                                                                           model. Many designs have been proposed for flexible
                                        Pa
                                         xo
                                                 Fa
                                                  stP
                                                             Ba
                                                              tc
                                                              hi
                                                                       Sp
                                                                        ec
                                                                                     N
                                                                                     O
                                                                                         Pa
                                                                                                  U
                                                                                                  nr
                                                                                                      ep
                                                                                                                processing, including fully flexible, software-based de-
                                             s        ax          ng        Pa            xo           lic
                                                        os                   xo  s            s            at
                                                                                                           ed
                                                                                                                signs like Click [30] and others based on network proces-
                                                                                                                sors [57] or FPGA platforms [47]. At the other extreme,
Figure 10: Maximum throughput achieved by a replicated trans-                                                   existing software defined networking mechanisms like
actional key-value store within 10 ms SLO.                                                                      OpenFlow [44] can easily achieve line-rate performance
   Figure 10 shows the maximum throughput of the key-                                                           in commodity hardware implementations but lack the flex-
value store with a 10ms SLO. NOPaxos outperforms all                                                            ibility to implement our multicast primitive. We use the P4
other variants on this metric: it attains more than 4 times                                                     language [9], which supports several high-performance
the throughput of Paxos, and outperforms the best prior                                                         hardware designs like Reconfigurable Match Tables [10].
protocol, Speculative Paxos, by 45%. Its throughput is                                                             These processing elements have generally been used for
also within 4% that of an unreplicated system.                                                                  classic networking tasks like congestion control or queue
                                                                                                                management. A notable exception is SwitchKV [41],
7                               Related Work                                                                    which uses OpenFlow switches for content-based routing
Our work draws on techniques from consensus protocol                                                            and load balancing in key-value stores.
design as well as network-level processing mechanisms.                                                          ...and their intersection Recent work on Specula-
Consensus protocols Many protocols have been pro-                                                               tive Paxos and Mostly-Ordered Multicast proposes co-
posed for the equivalent problems of consensus, state                                                           designing network primitives and consensus protocols
machine replication, and atomic broadcast. Most closely                                                         to achieve faster performance. Our work takes the next
related is a line of work on achieving better performance                                                       step in this direction. While Speculative Paxos assumes
when requests typically arrive at replicas in the same order,                                                   only a best-effort ordering property, NOPaxos requires
including Fast Paxos [36], Speculative Paxos [53], and                                                          an ordering guarantee. Achieving this guarantee requires
Optimistic Atomic Broadcast [29, 50, 51]; Zyzzyva [32]                                                          more sophisticated network support made possible with
applies a similar idea in the context of Byzantine fault                                                        a programmable data plane (Speculative Paxos’s Mostly-
tolerant replication. These protocols can reduce consen-                                                        Ordered Multicast requires only OpenFlow support).
sus latency in a manner similar to NOPaxos. However,                                                            However, as discussed in §5.3, NOPaxos achieves a sim-
because requests are not guaranteed to arrive in the same                                                       pler and more robust protocol as a result, avoiding the
order, they incur extra complexity and require superma-                                                         need for superquorums and speculation.
jority quorum sizes to complete a request (either 2/3 or                                                            A concurrent effort, NetPaxos [18], also explores ways
3/4 of replicas rather than a simple majority). This differ-                                                    to use the network layer to improve the performance of
ence is fundamental: the possibility of conflicting orders                                                      a replication protocol. That work proposes moving the
requires either an extra message round or a larger quorum                                                       Paxos logic into switches, with one switch serving as
size [37].                                                                                                      a coordinator and others as Paxos acceptors. This logic
   Another line of work aims to reduce latency and im-                                                          can also be implemented using P4 [17]. However, as the
prove throughput by avoiding coordination for opera-                                                            authors note, this approach requires the switches to im-
tions that are commutative or otherwise need not be or-                                                         plement substantial parts of the logic, including storing
dered [12, 35, 45, 60]; this requires application support to                                                    potentially large amounts of state (the results of each
identify commutative operations. NOPaxos avoids coordi-                                                         consensus instance). Our work takes a more practical ap-
nation for all operations.                                                                                      proach by splitting the responsibility between the OUM
   Ordered Unreliable Multicast is related to a long line                                                       network model, which can be readily implemented, and
of work on totally ordered broadcast primitives, usually                                                        the NOPaxos consensus protocol.
in the context of group communication systems [6, 7].                                                               Other related work uses hardware acceleration to speed
Years ago, a great debate raged in the SOSP community                                                           communication between nodes in a distributed system.
about the effectiveness and limits of this causal and totally                                                   FaRM [19] uses RDMA to bypass the kernel and mini-
ordered communication support (CATOCS) [5, 15]. Our                                                             mize CPU involvement in remote memory accesses. Con-
work draws inspiration from both sides of this debate, but                                                      sensus in a Box [25] implements a standard atomic broad-
occupies a new point in the design space by splitting the                                                       cast protocol entirely on FPGAs. NOPaxos provides more



USENIX Association                                                      12th USENIX Symposium on Operating Systems Design and Implementation                             479
flexible deployment options. However, its protocol could
be integrated with RDMA or other kernel-bypass network-
ing for faster replica performance.
8     Conclusions
We presented a new approach to high-performance, fault-
tolerant replication, one based on dividing the respon-
sibility for consistency between the network layer and
the replication protocol. In our approach, the network
is responsible for ordering, while the replication proto-
col ensures reliable delivery. “Splitting the atom” in this
way yields dramatic performance gains: network-level
ordering, while readily achievable, supports NOPaxos, a
simpler replication protocol that avoids coordination in
most cases. The resulting system outperforms state-of-
the-art replication protocols on latency, throughput, and
application-level metrics, demonstrating the power of this
approach. More significantly, it achieves both throughput
and latency equivalent to an unreplicated system, proving
that replication does not have to come with a performance
cost.
Acknowledgments
We thank Irene Zhang, the anonymous reviewers, and our
shepherd Dawn Song for their helpful feedback. This re-
search was supported by the National Science Foundation
under awards CNS-1518702 and CNS-1615102 and by
gifts from Google and VMware.




480    12th USENIX Symposium on Operating Systems Design and Implementation   USENIX Association
References                                                          7th USENIX Symposium on Operating Systems De-
 [1] M. Al-Fares, A. Loukissas, and A. Vahdat. A scal-              sign and Implementation (OSDI ’06), Seattle, WA,
     able, commodity data center network architecture.              USA, Nov. 2006.
     In Proceedings of ACM 2008, New York, NY, USA,            [12] L. Camargos, R. Schmidt, and F. Pedone. Multi-
     Aug. 2008.                                                     coordinated Paxos. Technical report, University of
                                                                    Lugano Faculty of Informatics, 2007/02, Jan. 2007.
 [2] Arista Networks. 7150 series ultra low latency
     switch. https://www.arista.com/assets/data/               [13] M. Castro and B. Liskov. Practical Byzantine fault
     pdf/Datasheets/7150S_Datasheet.pdf.                            tolerance. In Proceedings of the 3rd USENIX Sym-
                                                                    posium on Operating Systems Design and Imple-
 [3] A. Belay, G. Prekas, A. Klimovic, S. Grossman,
                                                                    mentation (OSDI ’99), New Orleans, LA, USA, Feb.
     C. Kozyrakis, and E. Bugnion. IX: A protected dat-
                                                                    1999.
     aplane operating system for high throughput and
     low latency. In Proceedings of the 11th USENIX            [14] T. D. Chandra, V. Hadzilacos, and S. Toueg. The
     Symposium on Operating Systems Design and Im-                  weakest failure detector for solving consensus.
     plementation (OSDI ’14), Broomfield, CO, USA,                  J. ACM, 43(4), July 1996.
     Oct. 2014. USENIX.                                        [15] D. R. Cheriton and D. Skeen. Understanding the
 [4] T. Benson, A. Akella, and D. A. Maltz. Network                 limitations of causally and totally ordered commu-
     traffic characteristics of data centers in the wild. In        nication. In Proceedings of the 13th ACM Sympo-
     Proceedings of the 10th ACM SIGCOMM Confer-                    sium on Operating Systems Principles (SOSP ’93),
     ence on Internet Measurement, IMC ’10, Melbourne,              Asheville, NC, USA, Dec. 1993. ACM.
     Australia, 2010. ACM.                                     [16] J. C. Corbett, J. Dean, M. Epstein, A. Fikes, C. Frost,
 [5] K. Birman. A response to Cheriton and Skeen’s crit-            J. Furman, S. Ghemawat, A. Gubarev, C. Heiser,
     icism of causal and totally ordered communication.             P. Hochschild, W. Hsieh, S. Kanthak, E. Kogan,
     ACM SIGOPS Operating Systems Review, 28(1), Jan.               H. Li, A. Lloyd, S. Melnik, D. Mwaura, D. Nagle,
     1994.                                                          S. Quinlan, R. Rao, L. Rolig, Y. Saito, M. Szyma-
                                                                    niak, C. Taylor, R. Wang, and D. Woodford. Span-
 [6] K. P. Birman and T. A. Joseph. Exploiting virtual              ner: Google’s globally-distributed database. In Pro-
     synchrony in distributed systems. In Proceedings               ceedings of the 10th USENIX Symposium on Oper-
     of the 11th ACM Symposium on Operating Systems                 ating Systems Design and Implementation (OSDI
     Principles (SOSP ’87), Austin, TX, USA, Oct. 1987.            ’12), Hollywood, CA, USA, Oct. 2012.
 [7] K. P. Birman and T. A. Joseph. Reliable communica-        [17] H. T. Dang, P. Bressana, H. Wang, K. S. Lee,
     tion in the presence of failures. ACM Trans. Comput.           H. Weatherspoon, M. Canini, F. Pedone, and
     Syst., 5(1), Jan. 1987.                                        R. Soulé. Network hardware-accelerated consensus.
 [8] W. J. Bolosky, D. Bradshaw, R. B. Haagens, N. P.               Technical Report USI-INF-TR-2016-03, Università
     Kusters, and P. Li. Paxos replicated state machines            della Svizzera italiana, May 2016.
     as the basis of a high-performance data store. In         [18] H. T. Dang, D. Sciascia, M. Canini, F. Pedone, and
     Proceedings of the 8th USENIX Symposium on Net-                R. Soulé. NetPaxos: Consensus at network speed. In
     worked Systems Design and Implementation (NSDI                 Proceedings of the 1st ACM SIGCOMM Symposium
    ’11), Boston, MA, USA, Apr. 2011. USENIX.                       on Software Defined Networking Research, SOSR
 [9] P. Bosshart, D. Daly, G. Gibb, M. Izzard, N. McKe-             ’15, New York, NY, USA, 2015. ACM.
     own, J. Rexford, C. Schlesinger, D. Talayco, A. Vah-      [19] A. Dragojević, D. Narayanan, M. Castro, and
     dat, G. Varghese, and D. Walker. P4: Program-                  O. Hodson. FaRM: Fast remote memory. In 11th
     ming protocol-independent packet processors. SIG-              USENIX Symposium on Networked Systems Design
     COMM Comput. Commun. Rev., 44(3), July 2014.                   and Implementation (NSDI 14), Seattle, WA, Apr.
[10] P. Bosshart, G. Gibb, H.-S. Kim, G. Vargh-                     2014. USENIX Association.
     ese, N. McKeown, M. Izzard, F. Mujica, and                [20] M. Flajslik and M. Rosenblum. Network interface
     M. Horowitz. Forwarding metamorphosis: Fast pro-               design for low latency request-response protocols.
     grammable match-action processing in hardware for              In Proceedings of the 2013 USENIX Annual Tech-
     SDN. In Proceedings of ACM SIGCOMM 2013.                       nical Conference, San Jose, CA, USA, June 2013.
     ACM, 2013.                                                     USENIX.
[11] M. Burrows. The Chubby lock service for loosely-          [21] P. Gill, N. Jain, and N. Nagappan. Understanding
     coupled distributed systems. In Proceedings of the             network failures in data centers: Measurement, anal-



USENIX Association                 12th USENIX Symposium on Operating Systems Design and Implementation               481
      ysis, and implications. In Proceedings of ACM SIG-           networks. In Proceedings of the 9th USENIX Sympo-
      COMM 2011, Toronto, ON, Canada, Aug. 2011.                   sium on Operating Systems Design and Implementa-
[22] A. Greenberg, J. R. Hamilton, N. Jain, S. Kandula,            tion (OSDI ’10), Vancouver, BC, Canada, Oct. 2010.
     C. Kim, P. Lahiri, D. A. Maltz, P. Patel, and S. Sen-         USENIX.
     gupta. VL2: A scalable and flexible data center          [32] R. Kotla, L. Alvisi, M. Dahlin, A. Clement, and
     network. In Proceedings of ACM SIGCOMM 2009,                  E. Wong. Zyzzyva: Speculative Byzantine fault
     Barcelona, Spain, Aug. 2009.                                  tolerance. In Proceedings of the 21th ACM Sympo-
[23] M. P. Herlihy and J. M. Wing. Linearizabiliy: A               sium on Operating Systems Principles (SOSP ’07),
     correctness condition for concurrent objects. ACM             Stevenson, WA, USA, Oct. 2007.
     Transactions on Programming Languages and Sys-           [33] L. Lamport. The part-time parliament. ACM Trans-
     tems, 12(3), July 1990.                                       actions on Computer Systems, 16(2), May 1998.
[24] P. Hunt, M. Konar, F. P. Junqueira, and B. Reed.         [34] L. Lamport. Paxos made simple. ACM SIGACT
     ZooKeeper: Wait-free coordination for Internet-               News, 32(4), Dec. 2001.
     scale systems. In Proceedings of the 2010 USENIX
                                                              [35] L. Lamport. Generalized consensus and Paxos.
     Annual Technical Conference, Boston, MA, USA,
                                                                   Technical Report MSR-TR-2005-33, Microsoft Re-
     June 2010.
                                                                   search, Mar. 2005.
[25] Z. István, D. Sidler, G. Alonso, and M. Vukolic.
                                                              [36] L. Lamport. Fast Paxos. Distributed Computing,
     Consensus in a box: Inexpensive coordination in
                                                                   19(2), Oct. 2006.
     hardware. In 13th USENIX Symposium on Net-
     worked Systems Design and Implementation (NSDI           [37] L. Lamport. Lower bounds for asynchronous con-
     16), Santa Clara, CA, Mar. 2016. USENIX Associa-              sensus. Distributed Computing, 19(2), Oct. 2006.
     tion.                                                    [38] C. Leau.     Spring Data Redis – Retwis-J,
[26] S. Jain, A. Kumar, S. Mandal, J. Ong, L. Poutievski,          2013.    http://docs.spring.io/spring-data/
     A. Singh, S. Venkata, J. Wanderer, J. Zhou, M. Zhu,           data-keyvalue/examples/retwisj/current/.
     J. Zolla, U. Hölzle, S. Stuart, and A. Vahdat. B4: Ex-   [39] J. Li, E. Michael, N. K. Sharma, A. Szekeres, and
     perience with a globally-deployed software defined            D. R. K. Ports. Just say NO to Paxos overhead: Re-
     WAN. In Proceedings of ACM SIGCOMM 2013,                      placing consensus with network ordering [extended
     Hong Kong, China, Aug. 2013. ACM.                             version]. Technical Report UW-CSE-16-09-02, Uni-
[27] F. P. Junqueira, B. C. Reed, and M. Serafini. Zab:            versity of Washington CSE, Seattle, WA, USA, Nov.
     High-performance broadcast for primary-backup                 2016.
     systems. In Proceedings of the 2011 IEEE/IFIP            [40] J. Li, N. K. Sharma, D. R. K. Ports, and S. D. Grib-
     41st International Conference on Dependable Sys-              ble. Tales of the tail: Hardware, OS, and application-
     tems&Networks, DSN ’11, Washington, DC, USA,                  level sources of tail latency. In Proceedings of the
     2011. IEEE Computer Society.                                  5th Symposium on Cloud Computing (SOCC ’14),
[28] A. Kalia, M. Kaminsky, and D. G. Andersen. De-                Seattle, WA, USA, Nov. 2014. ACM.
     sign guidelines for high performance RDMA sys-           [41] X. Li, R. Sethi, M. Kaminsky, D. G. Andersen, and
     tems. In 2016 USENIX Annual Technical Confer-                 M. J. Freedman. Be fast, cheap and in control with
     ence (USENIX ATC 16), Denver, CO, June 2016.                  SwitchKV. In 13th USENIX Symposium on Net-
     USENIX Association.                                           worked Systems Design and Implementation (NSDI
[29] B. Kemme, F. Pedone, G. Alonso, and A. Schiper.               16), Santa Clara, CA, Mar. 2016. USENIX Associa-
     Processing transactions over optimistic atomic                tion.
     broadcast protocols. In Proceedings of the 13th          [42] B. Liskov and J. Cowling. Viewstamped replica-
     International Symposium on Distributed Computing              tion revisited. Technical Report MIT-CSAIL-TR-
     (DISC ’99), Bratislava, Slovakia, Sept. 1999.                 2012-021, MIT Computer Science and Artificial In-
[30] E. Kohler, R. Morris, B. Chen, J. Jannotti, and M. F.         telligence Laboratory, Cambridge, MA, USA, July
     Kaashoek. The Click modular router. ACM Trans-                2012.
     actions on Computer Systems, 18(3), Aug. 2000.           [43] V. Liu, D. Halperin, A. Krishnamurthy, and T. An-
[31] T. Koponen, M. Casado, N. Gude, J. Stribling,                 derson. F10: A fault-tolerant engineered network. In
     L. Poutievski, M. Zhu, R. Ramanathan, Y. Iwata,               Proceedings of the 10th USENIX Symposium on Net-
     H. Inoue, T. Hama, and S. Shenker. Onix: A dis-               worked Systems Design and Implementation (NSDI
     tributed control platform for large-scale production         ’13), Lombard, IL, USA, Apr. 2013.



482   12th USENIX Symposium on Operating Systems Design and Implementation                         USENIX Association
[44] N. McKeown, T. Anderson, H. Balakrishnan,                    rakis: The operating system is the control plane. In
     G. Parulkar, L. Peterson, J. Rexford, S. Shenker,            Proceedings of the 11th USENIX Symposium on Op-
     and J. Turner. OpenFlow: enabling innovation in              erating Systems Design and Implementation (OSDI
     campus networks. ACM SIGCOMM Computer Com-                  ’14), Broomfield, CO, USA, Oct. 2014. USENIX.
     munication Review, 38(2), Apr. 2008.
                                                            [53] D. R. K. Ports, J. Li, V. Liu, N. K. Sharma, and A. Kr-
[45] I. Moraru, D. G. Andersen, and M. Kaminsky. There           ishnamurthy. Designing distributed systems using
     is more consensus in egalitarian parliaments. In            approximate synchrony in data center networks. In
     Proc. of SOSP, 2013.                                        Proc. of NSDI, 2015.
[46] R. N. Mysore, A. Pamboris, N. Farrington,              [54] J. Rao, E. J. Shekita, and S. Tata. Using Paxos to
     N. Huang, P. Miri, S. Radhakrishnan, V. Subra-              build a scalable, consistent, and highly available
     manya, and A. Vahdat. PortLand: A scalable fault-           datastore. Proc. of VLDB, 4(4), Apr. 2011.
     tolerant layer 2 data center network fabric. In
     Proceedings of ACM SIGCOMM 2009, Barcelona,            [55] J. H. Saltzer, D. P. Reed, and D. D. Clark. End-to-
     Spain, Aug. 2009.                                           end arguments in system design. ACM Transactions
                                                                 on Computer Systems, 2(4), Nov. 1984.
[47] J. Naous, D. Erickson, G. A. Covington, G. Appen-
                                                            [56] F. B. Schneider. Implementing fault-tolerant ser-
     zeller, and N. McKeown. Implementing an Open-
                                                                 vices using the state machine approach: a tutorial.
     Flow switch on the NetFPGA platform. In Proceed-
                                                                 ACM Computing Surveys, 22(4), Dec. 1990.
     ings of the 4th ACM/IEEE Symposium on Architec-
     tures for Networking and Communications Systems,       [57] T. Spalink, S. Karlin, L. Peterson, and Y. Gottlieb.
     ANCS ’08, New York, NY, USA, 2008. ACM.                     Building a robust software-based router using net-
[48] B. M. Oki and B. H. Liskov. Viewstamped replica-            work processors. In Proceedings of the 19th ACM
     tion: A new primary copy method to support highly-          Symposium on Operating Systems Principles (SOSP
     available distributed systems. In Proceedings of the       ’01), Banff, Canada, Oct. 2001. ACM.
     7th ACM Symposium on Principles of Distributed         [58] P. Urbán, X. Défago, and A. Schiper. Chasing the
     Computing (PODC ’88), Toronto, Ontario, Canada,             FLP impossibility result in a LAN: or, how robust
     Aug. 1988.                                                  can a fault tolerant server be? In Proceedings of
[49] R. Ozdag. Intel R Ethernet switch FM6000 series-            the 20th IEEE Symposium on Reliable Distributed
     software defined networking.                                Systems (SRDS ’01), New Orleans, LA USA, Oct.
[50] F. Pedone and A. Schiper. Optimistic atomic broad-          2001.
     cast. In Proceedings of the 12th International Sym-    [59] XPliant       Ethernet            switch         prod-
     posium on Distributed Computing (DISC ’98), An-             uct     family.            www.cavium.com/
     dros, Greece, Sept. 1998.                                   XPliant-Ethernet-Switch-Product-Family.
[51] F. Pedone and A. Schiper. Optimistic atomic broad-          html.
     cast: A pragmatic viewpoint. Theor. Comput. Sci.,      [60] I. Zhang, N. K. Sharma, A. Szekeres, A. Krishna-
     291(1), Jan. 2003.                                          murthy, and D. R. K. Ports. Building consistent
[52] S. Peter, J. Li, I. Zhang, D. R. K. Ports, D. Woos,         transactions with inconsistent replication. In Proc.
     A. Krishnamurthy, T. Anderson, and T. Roscoe. Ar-           of SOSP, 2015.




USENIX Association                12th USENIX Symposium on Operating Systems Design and Implementation             483
