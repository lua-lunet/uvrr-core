# Ousterhout–Ongaro strong-consistency bibliography

Compiled 2026-09-28 (agent366). Verified fact first, then the works.

## Provenance fact-check

- **John Ousterhout is "of RAFT fame".** He is second author on *In Search of an
  Understandable Consensus Algorithm* (USENIX ATC 2014, Best Paper Award) and
  was **primary advisor** of Diego Ongaro's Stanford PhD dissertation
  (*Consensus: Bridging Theory and Practice*, 2014,
  <https://purl.stanford.edu/qr033xr6097>). LWN describes him as "of Tcl/Tk and
  Raft fame". So: stop bashing Raft if you want him as a reader — he is Raft's
  co-author, not merely its supervisor.
- **Same person across both threads the user cares about**: RAMCloud (2009–2015)
  → Raft (2014, with Ongaro) → CURP (NSDI'19) → EPaxos Revisited (NSDI'21) →
  Homa (SIGCOMM'18, ATC'21, arXiv 2022, talks 2022–2026).
- The 2026-09-17 AI Engineer talk transcript is at
  `research/agent366/ousterhout/homa-talk-transcript.txt` (whisper large-v3 STT
  of yt-dlp audio). He ends it by explicitly soliciting collaborators:
  "I retired from Stanford… so I can spend 100% of my time hacking on HOMA.
  I'd be delighted to work with you… feel free to contact me. My email is on
  the slide."

## The works (abstracts verbatim)

### RAFT LINE

**raft-2014** — D. Ongaro, J. Ousterhout. *In Search of an Understandable
Consensus Algorithm*. USENIX ATC 2014, pp. 305–319. (Best Paper.)
<https://www.usenix.org/system/files/conference/atc14/atc14-paper-ongaro.pdf>
Local: `../md/raft-atc14-ongaro.md`.

> Raft is a consensus algorithm for managing a replicated log. It produces a
> result equivalent to (multi-)Paxos, and it is as efficient as Paxos, but its
> structure is different from Paxos; this makes Raft more understandable than
> Paxos and also provides a better foundation for building practical systems.
> In order to enhance understandability, Raft separates the key elements of
> consensus, such as leader election, log replication, and safety, and it
> enforces a stronger degree of coherency to reduce the number of states that
> must be considered. Results from a user study demonstrate that Raft is
> easier for students to learn than Paxos. Raft also includes a new mechanism
> for changing the cluster membership, which uses overlapping majorities to
> guarantee safety.

Note: the paper itself states "Raft is similar in many ways to existing
consensus algorithms (most notably, Oki and Liskov's Viewstamped
Replication)". The Raft→VR lineage is in his own abstract section.

**ongaro-dissertation-2014** — D. Ongaro. *Consensus: Bridging Theory and
Practice*. Stanford PhD dissertation, 2014. <https://purl.stanford.edu/qr033xr6097>

> Distributed consensus is fundamental to building fault-tolerant systems. It
> allows a collection of machines to work as a coherent group that can survive
> the failures of some of its members. Unfortunately, the most common
> consensus algorithm, Paxos, is widely regarded as difficult to understand
> and implement correctly. This dissertation presents a new consensus
> algorithm called Raft, which was designed for understandability. Raft first
> elects a server as leader, then concentrates all decision-making onto the
> leader. These two basic steps are relatively independent and form a better
> structure than Paxos, whose components are hard to separate. Raft elects a
> leader using voting and randomized timeouts. The election guarantees that
> the leader already stores all the information it needs, so data only flows
> outwards from the leader to other servers. Compared to other leader-based
> algorithms, this reduces mechanism and simplifies the behavior. Once a
> leader is elected, it manages a replicated log. Raft leverages a simple
> invariant on how logs grow to reduce the algorithm's state space and
> accomplish this task with minimal mechanism. Raft is also more suitable than
> previous algorithms for real-world implementations. It performs well enough
> for practical deployments, and it addresses all aspects of building a
> complete system, including how to manage client interactions, how to change
> the cluster membership, and how to compact the log when it grows too large.
> To change the cluster membership, Raft allows adding or removing one server
> at a time (complex changes can be composed from these basic steps), and the
> cluster continues servicing requests throughout the change. We believe that
> Raft is superior to Paxos and other consensus algorithms, both for
> educational purposes and as a foundation for implementation. Results from a
> user study demonstrate that Raft is easier for students to learn than Paxos.
> The algorithm has been formally specified and proven, its leader election
> algorithm works well in a variety of environments, and its performance is
> equivalent to Multi-Paxos. Many implementations of Raft are now available,
> and several companies are deploying Raft.

**logcabin** — D. Ongaro. *LogCabin: a coordination service built with Raft*.
(Ongaro's post-Raft system; described in the dissertation and his talks.)
<https://github.com/logcabin/logcabin>

### RAMCLOUD LINE (the "durability without disk" antecedent)

**case-for-ramcloud-2009** — J. Ousterhout, P. Agrawal, D. Erickson,
C. Kozyrakis, J. Leverich, D. Mazières, S. Mitra, A. Narayanan, G. Parulkar,
M. Rosenblum, S. M. Rumble, E. Stratmann, R. Stutsman. *The Case for
RAMClouds: Scalable High-Performance Storage Entirely in DRAM*. SIGOPS OSR
43(4), Dec 2009; also CACM 54(7), 2011.
<https://web.stanford.edu/~ouster/cgi-bin/papers/ramcloud.pdf>

> Disk-oriented approaches to online storage are becoming increasingly
> problematic: they do not scale gracefully to meet the needs of large-scale
> Web applications, and improvements in disk capacity have far out-stripped
> improvements in access latency and bandwidth. This paper argues for a new
> approach to datacenter storage called RAMCloud, where information is kept
> entirely in DRAM and large-scale systems are created by aggregating the main
> memories of thousands of commodity servers. We believe that RAMClouds can
> provide durable and available storage with 100-1000x the throughput of
> disk-based systems and 100-1000x lower access latency. The combination of
> low latency and large scale will enable a new breed of data-intensive
> applications.

**ramcloud-tocs-2015** — J. Ousterhout, A. Gopalan, A. Gupta, A. Kejriwal,
C. Lee, B. Montazeri, **D. Ongaro**, S. J. Park, H. Qin, M. Rosenblum,
S. Rumble, R. Stutsman, S. Yang. *The RAMCloud Storage System*. ACM TOCS
33(3), Art. 7, Aug 2015. doi 10.1145/2806887.
<https://web.stanford.edu/~ouster/cgi-bin/papers/ramcloud-tocs.pdf>

> RAMCloud is a storage system that provides low-latency access to large-scale
> datasets. To achieve low latency, RAMCloud stores all data in DRAM at all
> times. To support large capacities (1PB or more), it aggregates the memories
> of thousands of servers into a single coherent key-value store. RAMCloud
> ensures the durability of DRAM-based data by keeping backup copies on
> secondary storage. It uses a uniform log-structured mechanism to manage both
> DRAM and secondary storage, which results in high performance and efficient
> memory usage. RAMCloud uses a polling-based approach to communication,
> bypassing the kernel to communicate directly with NICs; with this approach,
> client applications can read small objects from any RAMCloud storage server
> in less than 5μs, durable writes of small objects take about 13.5μs.
> RAMCloud does not keep multiple copies of data online; instead, it provides
> high availability by recovering from crashes very quickly (1 to 2 seconds).
> RAMCloud's crash recovery mechanism harnesses the resources of the entire
> cluster working concurrently so that recovery performance scales with
> cluster size.

**curp-2019** — S. J. Park, J. Ousterhout. *Exploiting Commutativity For
Practical Fast Replication* (CURP). NSDI 2019, pp. 47–64.
<https://www.usenix.org/conference/nsdi19/presentation/park>

> Traditional approaches to replication require client requests to be ordered
> before making them durable by copying them to replicas. As a result, clients
> must wait for two round-trip times (RTTs) before updates complete. In this
> paper, we show that this entanglement of ordering and durability is
> unnecessary for strong consistency. Consistent Unordered Replication
> Protocol (CURP) allows clients to replicate requests that have not yet been
> ordered, as long as they are commutative. This strategy allows most
> operations to complete in 1 RTT (the same as an unreplicated system). We
> implemented CURP in the Redis and RAMCloud storage systems. In RAMCloud,
> CURP improved write latency by ~2x (14us -> 7.1us) and write throughput by
> 4x. Compared to unreplicated RAMCloud, CURP's latency overhead for 3-way
> replication is just 1us (6.1us vs 7.1us). CURP transformed a non-durable
> Redis cache into a consistent and durable storage system with only a small
> performance overhead.

Note: CURP is the closest Ousterhout-line work to uVRR's territory — strong
consistency with durability decoupled from ordering, measured in microseconds.

**epaxos-revisited-2021** — S. Tollman, S. J. Park, J. Ousterhout. *EPaxos
Revisited*. NSDI 2021, pp. 613–632.
<https://www.usenix.org/conference/nsdi21/presentation/tollman>

> This paper re-evaluates the performance of the EPaxos consensus protocol for
> geo-replication and proposes an enhancement that uses synchronized clocks to
> reduce operation latency. The benchmarking approach used for the original
> EPaxos evaluation does not trigger or measure the full impact of conflict
> behavior on system performance. Our re-evaluation confirms the original
> claim that EPaxos provides optimal median commit latency in a WAN, but it
> shows much worse tail latency than previously reported (more than 4x worse
> than Multi-Paxos). Furthermore, performance is highly sensitive to
> application workloads, particularly at the tail.
>
> In addition, we show how synchronized clocks can be used to reduce conflicts
> in geo-replication. By imposing intentional delays on message processing, we
> can achieve roughly in-order deliveries to multiple replicas. When applied
> to EPaxos, this technique reduced conflicts by at least 50% without
> introducing additional overhead, decreasing mean latency by up to 7.5%.

### HOMA LINE (transport; "the end of TCP")

**homa-2018** — B. Montazeri, Y. Li, M. Alizadeh, J. Ousterhout. *Homa:
A Receiver-Driven Low-Latency Transport Protocol Using Network Priorities*.
ACM SIGCOMM 2018. doi 10.1145/3230543.3230564; complete version arXiv:1803.09615.
<https://people.csail.mit.edu/alizadeh/papers/homa-sigcomm18.pdf>

> Homa is a new transport protocol for datacenter networks. It provides
> exceptionally low latency, especially for workloads with a high volume of
> very short messages, and it also supports large messages and high network
> utilization. Homa uses in-network priority queues to ensure low latency for
> short messages; priority allocation is managed dynamically by each receiver
> and integrated with a receiver-driven flow control mechanism. Homa also uses
> controlled overcommitment of receiver downlinks to ensure efficient
> bandwidth utilization at high load. Our implementation of Homa delivers 99th
> percentile round-trip times less than 15 µs for short messages on a 10 Gbps
> network running at 80% load. These latencies are almost 100x lower than the
> best published measurements of an implementation. In simulations, Homa's
> latency is roughly equal to pFabric and significantly better than pHost,
> PIAS, and NDP for almost all message sizes and workloads. Homa can also
> sustain higher network loads than pFabric, pHost, or PIAS.

**homa-linux-2021** — J. Ousterhout. *A Linux Kernel Implementation of the
Homa Transport Protocol*. USENIX ATC 2021, pp. 773–787.
<https://www.usenix.org/conference/atc21/presentation/ousterhout>

> Homa/Linux is a Linux kernel module that implements the Homa transport
> protocol. Measurements of Homa/Linux reconfirm Homa's superior performance
> compared to TCP and DCTCP. In a cluster benchmark with 40 nodes, Homa/Linux
> provided lower latency than both TCP and DCTCP for all message sizes; for
> short messages, Homa's 99th percentile tail latency was 7–83x lower than TCP
> and DCTCP. The benchmarks also show that Homa has eliminated network
> congestion as a significant performance limitation. Both tail latency and
> throughput are now limited by software overheads, particularly software
> congestion caused by imperfect load balancing of the protocol stack across
> cores. Another factor of 5–10x in performance can be achieved if software
> overheads can be eliminated in the future.

**replace-tcp-2022** — J. Ousterhout. *It's Time to Replace TCP in the
Datacenter*. arXiv:2210.00714 (v2, Jan 2023); published in CACM 66(4), 2023.
<https://arxiv.org/abs/2210.00714>

> In spite of its long and successful history, TCP is a poor transport
> protocol for modern datacenters. Every significant element of TCP, from its
> stream orientation to its expectation of in-order packet delivery, is wrong
> for the datacenter. It is time to recognize that TCP's problems are too
> fundamental and interrelated to be fixed; the only way to harness the full
> performance potential of modern networks is to introduce a new transport
> protocol into the datacenter. Homa demonstrates that it is possible to
> create a transport protocol that avoids all of TCP's problems. Although Homa
> is not API-compatible with TCP, it should be possible to bring it into
> widespread usage by integrating it with RPC frameworks.

**homa-talk-2026** — J. Ousterhout. *Homa: The End of TCP for AI Clusters*.
AI Engineer talk, uploaded 2026-09-17 (18:48).
<https://www.youtube.com/watch?v=eZ8WWZzoaR0>
Transcript (whisper large-v3, local): `homa-talk-transcript.txt`;
timestamped segments: `homa-talk-segments.json`.

Gist from the transcript: AI workloads are shifting from large,
throughput-dominated transfers to smaller, latency-critical ones; TCP and
RDMA both mishandle that shift; Homa is message-based (not byte-stream),
receiver-driven (grants), and uses switch priority queues to approximate SRPT;
measured tail latency for short messages is >10x better than TCP. Hardware
requirement reality-check (user asked "Homa needs special hardware if I am
correct"): Homa needs only the priority queues "already present in modern
switches" — no special hardware beyond commodity datacenter switches;
its kernel module runs on stock Linux.

## Adjacent works already in the agent366 corpus (for the graph)

VR-1988, VR-2012, Oki dissertation, PMS-2001, Disk Paxos 2003, Vertical Paxos
2009, DISC-2017, Reconfiguring a State Machine 2010, FQI 2016, Turner
paxos-reconf (branch raft-like-reconfiguration), OSDI-2014, SOSP-2013,
LAMPSON-1979, FAST-18 PAR, UW diskless TR16, CORFU TOCS, ZooKeeper ATC'10,
Lean 4 CADE'21, the five simbo1905 blog posts, etcd/TigerBeetle/MySQL/Jepsen
web corpus — see `../CORPUS.md`.
