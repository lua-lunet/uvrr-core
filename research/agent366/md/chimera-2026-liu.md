                                                        C HIMERA: Protocol-Aware Recovery for
                                                             Confidential BFT Consensus
                                                    Tong Liu∗ , Xiaoqing Wen† , Ziwei Zhou‡ , Si Liu§ , Jianyu Niu¶ , Cong Wang¶ , Yinqian Zhang∗
                                              ∗ Southern University of Science and Technology, † University of British Columbia, ‡ East China Normal University,
                                                                             § Texas A&M University, ¶ City University of Hong Kong



                                            Abstract—Trusted Execution Environments (TEEs) have en-           due to the confidentiality guarantees of TEEs, the secrecy
                                         abled confidential Byzantine Fault-Tolerant (BFT) consensus          of computations (e.g., blockchain transactions) is preserved
                                         systems with confidentiality and improved scalability. How-          against untrusted nodes, enabling wider applicability across
                                         ever, TEEs do not provide state continuity: during recovery,
                                                                                                              sensitive use cases.




arXiv:2606.09101v2 [cs.DC] 11 Jun 2026
                                         a compromised host can roll back a crashed enclave to a
                                         stale persistent state, significantly threatening both safety and       Nonetheless, a well-known Achilles’ heel of confidential
                                         availability. Existing defenses face a fundamental tradeoff: they    BFT lies in the lack of state continuity in TEEs. State conti-
                                         either impose substantial overhead on critical consensus paths,      nuity allows a TEE to preserve a consistent, tamper-resistant
                                         reducing throughput and increasing latency, or incur prolonged       state across crashes and restarts. In fault-tolerant protocols, a
                                         recovery delays, hurting availability.
                                            We present the first systematic taxonomy of rollback-resilient    persisted state enables a crashed node to recover and rejoin
                                         recovery for confidential BFT consensus, distilling prior ap-        the system. However, an adversary that controls the host can
                                         proaches into four categories. We further expose their inher-        provide stale state to the TEE during recovery, causing a
                                         ent limitations. Guided by this detailed analysis, we design         rollback attack [4], [8].
                                         C HIMERA, a protocol-aware recovery framework that breaks this          Recent years have seen substantial efforts on achieving roll-
                                         tradeoff. Our key insight is that rollback protection in consensus
                                         systems should not be uniform. Different types of persistent         back resilience in such settings [12], [13], [14], [15], [4], [16],
                                         states differ fundamentally in their state distributions, update     [17], [9], [18], [19]. We categorize these approaches into four
                                         behaviors, and representations. C HIMERA separates persistent        categories: Trusted Counter (TC), Diskless Crash Recovery
                                         state into metadata and logs according to these protocol-level       (DCR), Rollback Fault Tolerance (RFT), and Reconfiguration
                                         properties and applies distinct recovery mechanisms to each type.    (RC). Yet each category exemplifies a fundamental trade-
                                         We formally model C HIMERA in Maude and verify its safety and
                                         liveness properties. We implement it on Braft and ZooKeeper          off between performance and availability: they either impose
                                         using Intel TDX, and evaluate it in both LAN and WAN settings.       high overhead on critical consensus paths or incur prolonged
                                         Results show that C HIMERA achieves higher throughput, lower         recovery delays. (See §III for a detailed discussion.)
                                         recovery latency, and better availability than state-of-the-art         In this paper, we aim to answer the question: How can
                                         rollback-resilient baselines.                                        we design rollback-resilient recovery for confidential BFT
                                                                                                              consensus that maintains high performance during normal
                                                               I. I NTRODUCTION
                                                                                                              operation and minimizes downtime during recovery?
                                            Confidential Byzantine Fault Tolerant (BFT) consensus,               Our key insight is that the above tradeoff stems from a com-
                                         which uses Trusted Execution Environments (TEEs) to im-              mon design choice: existing defenses protect persistent state
                                         prove system scalability and confidentiality, has recently           uniformly—applying the same protection to all state—even
                                         gained significant traction. By leveraging TEEs, confidential        though different types of state differ in how they are updated,
                                         BFT consensus enables a set of nodes to agree on an ever-            stored, and recovered by the protocol. By classifying persistent
                                         growing, consistent log of transactions while preserving client      states according to their protocol-level characteristics, we
                                         confidentiality in the presence of Byzantine behavior (i.e., arbi-   can design recovery mechanisms tailored to each type. For
                                         trary protocol deviation) among nodes. Due to the promising          leader-based consensus, these states can be divided into two
                                         consistency, confidentiality, and fault tolerance properties, it     categories: metadata and logs. Metadata is a set of node-local
                                         has been used in many decentralized or cloud services, in-           control states that records each node’s view of the consensus
                                         cluding datastores [1], [2], [3], blockchains [4], [5], [6], cloud   process, while logs are replicated histories, with replications
                                         computing [7], [8], [9], and self-expiring data objects [10].        maintained according to consensus rules.
                                            More specifically, confidential BFT consensus offers two             Guided by this distinction, we propose C HIMERA, a
                                         key advantages over classic BFT protocols. First, due to the in-     protocol-aware recovery framework for confidential BFT con-
                                         tegrity guarantees of TEEs, parties running inside TEEs cannot       sensus that customizes recovery for metadata and logs. For
                                         equivocate their messages (e.g., votes). Therefore, confidential     metadata, C HIMERA uses a TC-based mechanism to securely
                                         BFT consensus achieves Byzantine fault tolerance by running          and timely recover metadata from local storage. For logs,
                                         Crash Fault Tolerant (CFT) protocols, such as Raft [11],             C HIMERA uses protocol-guided cluster recovery that leverages
                                         inside TEEs. This design improves scalability by allowing a          consensus replication rules to rebuild a safe log from other
                                         smaller node size and less computational overhead. Second,           replicas. This design balances performance and availability: it
confines TC overhead to infrequent metadata updates, avoiding                                  II. BACKGROUND
TC updates on the high-frequency log replication path; it
                                                                         A. TEEs and Their State Continuity
also reduces recovery downtime by avoiding epoch skips and
allowing crashed leaders to resume without waiting for new                  Trusted Execution Environments (TEEs) are hardware-
leader election.                                                         supported execution environments that protect sensitive code
   Applying such protocol-aware recovery, however, is nontriv-           and data. They achieve this protection through mechanisms
ial. First, for metadata, unexpected crashes may occur between           such as memory encryption, hardware-enforced isolation, and
the counter increment and metadata sealing. Without proper               remote attestation. Representative platforms include enclave-
handling, a recovering node may fail to restore its metadata if          based TEEs such as Intel SGX [24], VM-based TEEs such as
the counter and persistent states are mismatched. Achilles [18]          Intel TDX [23] and AMD SEV [25], and other hardware iso-
proposes skipping several epochs to avoid double voting, but             lation architectures such as ARM TrustZone [26]. Compared
this can render the node temporarily unavailable. We analyze             with enclave-based TEEs, VM-based TEEs allow existing
the update process of metadata and design a binding update               applications to benefit from TEE protection with minimal code
solution to address this issue (§VI-B). Second, in log recovery,         changes and low additional overhead. We therefore build our
a crashed leader may leave an entry replicated to only a subset          implementation on VM-based TEEs.
of nodes, failing to reach quorum. To support leader recovery,              TEEs have been adopted in a wide range of stateful ap-
the recovered leader must confirm its leadership and identify            plications, including blockchains [27], [28], [5], [29], [30],
any unfinished log entries; otherwise, it may re-propose en-             [31], trusted storage [32], [1], [2], [3], authentication rate
tries at the same indices, resulting in conflicting entries at           limiting [14], and cloud computing services [7], [8], [9], [19],
the same log position (§VI-C). Third, log synchronization                [13]. However, despite their popularity, TEEs still lack state
from the network is time-consuming. To mitigate this cost,               continuity guarantees: rollback attacks can revert a TEE to a
C HIMERA uses metadata to identify the log range that needs              prior state, undermining the security of applications built atop
to be synchronized. This approach reduces unnecessary log                them.
transmission during recovery and enables recovering nodes to             State Continuity. State continuity of TEEs mandates that
rejoin the protocol with minimal delay (§VI-C).                          when a stateful TEE application resumes execution from an
   We prototype C HIMERA by extending two open-                          interruption (e.g., reboots or system crashes), it must resume
source industrial CFT consensus platforms: Braft [20],                   in the same state as before [15]. This property is critical
a high-performance implementation of Raft [11], and                      for applying TEEs in distributed systems, since nodes’ ability
ZooKeeper [21], a widely adopted coordination service built              to recover from crashes directly determines system reliabil-
on the Zab [22] consensus protocol. We select Raft and                   ity. [33], [34], [35]. Existing TEE platforms provide sealing
Zab because they are the most widely used CFT consensus                  functionality, allowing TEE applications to encrypt and store
protocols for building confidential BFT consensus [3], [7],              their state to untrusted persistent storage. The sealing key can
[8], [4], [1], [2]. Both prototypes are implemented on top of            be configured to be accessible to all enclaves with the same
Virtual Machine (VM)-based TEE, i.e., Intel TDX [23]. We                 MRENCLAVE or MRSIGNER [13].
formally model C HIMERA in Maude and verify its safety                   Rollback Attacks. The sealing functionality can ensure the
and liveness properties. We conduct extensive experiments on             integrity of retrieved data, but does not provide freshness
a public cloud platform to evaluate and compare C HIMERA                 guarantees [36]. An adversary controlling the OS can roll back
with four counterparts, particularly their performance during            a TEE application to a previous state by providing it with stale
recovery over LAN and WAN.                                               data, resulting in a rollback attack [37]. These attacks break
                                                                         state continuity and undermine the security guarantees of TEEs
Contributions. Our main contributions are as follows:
                                                                         for various stateful applications, particularly confidential BFT
• We provide the first taxonomy of rollback-resilient solutions          consensus systems. For instance, if a node is rolled back after
  for TEEs in distributed systems, and organize them into four           casting a vote, it may re-enter a previous state and cast the vote
  categories. We further analyze why applying uniform rollback           again. Such repeated voting can lead to equivocation, violating
  protection is insufficient for persistent states with differing        the safety properties of the protocol.
  protocol-level characteristics in these systems.
• We propose C HIMERA, a protocol-aware recovery frame-                  B. Confidential BFT Consensus
 work for confidential BFT consensus. It tailors recovery                   Confidential BFT consensus ports CFT consensus proto-
 mechanisms to metadata and logs according to their charac-              cols into TEEs to provide BFT services. Examples include
 teristics. To ensure its correctness (i.e., safety and liveness),       SVR3 [3] adopted by Signal [38], CCF [8] deployed on the
 we perform a thorough security analysis and conduct a formal            Azure cloud platform [39], Engraft [4], Hyperledger Fab-
 modeling and verification of C HIMERA.                                  ric [40], and SecureKeeper [2]. Among them, SVR3, CCF,
• We implement C HIMERA atop Braft and ZooKeeper with                    and Engraft are built on Raft [11], whereas SecureKeeper
 Intel TDX, and integrate optimizations to reduce recovery               uses Zab [22]. Raft and Zab are well-established leader-based
 communication. Our extensive evaluation demonstrates that               CFT consensus protocols. Both provide similar protocol-level
 C HIMERA delivers superior performance.                                 semantics, which are leveraged in our design.



                                                                     2
• Epoch. The leader-based consensus protocol partitions time                          TABLE I: Rollback-resilient solutions.
 into logical units called epochs (referred to as terms in Raft).
                                                                                         Recovery       Recovery        Performance     Availability
 For each epoch, at most one node, called the leader, is elected          Approach
                                                                                          Pattern      Completeness      Overhead       Degradation
 and agreed upon by a quorum. When the leadership needs to
                                                                          TC              Local           High              High           Low
 change, the epoch is incremented to reflect the transition.
                                                                          DCR           Distributed      Medium             Low            High
• Leader Election. When nodes detect that the leader is un-               RFT             Local          High∗              High           Low
 available, they initiate a leader election to preserve liveness.         RC            Distributed       Low               Low            High
 Specifically, each node casts at most one vote per epoch                 C HIMERA       Hybrid           High              Low            Low
 and persists this voting behavior to prevent double voting.            ∗ RFT enables a node to recover its state, but cannot prevent state rollback.

 A candidate node becomes a leader when it receives votes
 from the majority of nodes. A non-leader node is called a               increment but before sealing makes recovery impossible. In
 follower. In this paper, we refer to the persisted epoch and            contrast, the store-then-inc pattern [15], [14] favors availabil-
 voting behavior as metadata.                                            ity over safety, since a crash during the interval can result in
• Log. The log is a sequence of client transactions maintained           rollback. Further discussion is provided in Appendix A1.
 by nodes, where each entry contains one or a batch of                  • Detection-only recovery. TC can determine the freshness of
 transactions. Each entry is uniquely identified by an epoch             a persisted state by comparing counter values, but it cannot
 number and a monotonically increasing index. Nodes commit               identify which state is safe for recovery; in other words, it
 entries in a contiguous sequence sorted by index without                provides only rollback detection, not full recovery.
 gaps. Unlike metadata, logs are replicated across nodes
                                                                        Diskless Crash Recovery (DCR). DCR enables a node to
 following consensus rules.
                                                                        recover its state from the in-memory state of other nodes in the
                 III. R ECOVERY TAXONOMY                                cluster [44], [45], [46], [47]. Specifically, DCR is effective at
                                                                        recovering redundant data, such as logs, because the protocol’s
   We present the first systematic analysis of TEE recovery             replication rules allow recovery to locate a node with a
mechanisms in distributed systems, categorizing them into               sufficiently safe copy. It relies only on naive log replication
four groups, as shown in Table I. This taxonomy provides a              and introduces no additional overhead on the consensus path.
direct comparison of the approaches, as we present next. More           However, directly applying DCR to confidential BFT recovery
importantly, it shows why rollback recovery for confidential            exposes the following shortcomings.
BFT must be protocol-aware rather than uniform.
Trusted Counter (TC). TC is a tamper-resistant counter                  • Epoch skipping for safety. DCR cannot recover node-local
whose value, once incremented, cannot be reverted to a                   metadata such as epochs and voting behavior. To prevent
previous value [41]. It can be implemented in hardware (e.g.,            rollback-induced double voting, a recovering node must first
TPM [41] or SGX monotonic counters [42]) or in software                  infer a safe upper-bound epoch based on the protocol’s
(e.g., distributed trusted KV [12], [13]) (Appendix A1). It is           metadata update rules. The node then recovers directly into
the most popular and general approach for addressing TEEs’               that epoch, skipping earlier epochs. As a result, it may remain
rollback attacks [12], [13], [43], [14], [15], [4].                      unavailable until the cluster reaches the same epoch.
   This approach performs two operations for each state up-             • Protocol-specific recovery rules. The safe epoch inferred
date: (1) incrementing the counter value, and (2) sealing the            by DCR depends on the protocol’s metadata update rules.
updated state together with the counter value in persistent              Different protocols may require skipping varying numbers of
storage. During recovery, a node retrieves the sealed states             epochs, and those without bounded epoch growth may fail to
and validates its freshness by comparing the stored counter              provide a usable safe epoch. Consequently, both the recovery
value against the current one. Although this design is relatively        rules and the unavailability period are protocol-specific.
simple and allows the system to recover the exact pre-crash             • No leader recovery. DCR requires an active leader. If the
state, it suffers from the following limitations:                         leader crashes, recovering nodes can only resume as followers
• High read/write overhead. Each state update requires a                  and must wait for a new leader to be elected. This dependency
 counter write, introducing substantial overhead. Updating                prolongs the period of unavailability after leader failures.
 a hardware-backed counter, such as a TPM counter, takes                Rollback Fault Tolerance (RFT). RFT addresses rollback
 roughly 97 ms, while reading it for state verification takes           attacks by adjusting read/write quorum sizes according to the
 about 35 ms [14]. Software-based counters typically incur              number of potentially rolled-back nodes. RR [17] modifies
 one or two extra communication rounds [12], [13], [43]. Such           read/write quorum sizes to guarantee intersection with up-to-
 overhead makes TC unsuitable for a frequently updated state.           date nodes, whereas FlexiBFT [16] increases the overall node
• Inc-store consistency dilemma. Since counter increment and            size to 3f + 1, aligning with classical BFT requirements. In
 state sealing cannot be performed atomically, two usage                RFT, a recovering node can quickly retrieve its state—which
 patterns have emerged. The inc-then-store pattern [12], [13],          is not protected against rollbacks—and rejoin the protocol
 [43], [4] increments the counter before sealing the state,             without affecting system availability. This design choice brings
 preserving safety but risking availability: a crash after the          the following limitations:



                                                                    3
• Scalability limitation. Increasing node and quorum sizes                and confidentiality cannot be breached (introduced shortly).
 undermines log commit performance, particularly in large                 The other nodes that faithfully follow the protocol and re-
 deployments. Evaluation results show that when f is small,               main operational (i.e., participating in consensus) are correct
 the performance gap between the 2f +1 and 3f +1 configura-               nodes. The rationale behind this assumption is provided in
 tions atop Braft (i.e., Braft-DR and Braft-RFT, respectively)            Appendix A2.
 is around 10%. However, this gap grows with larger f = 20,                  The adversary gains full control over the OS of the corrupted
 reaching roughly 20% (§IX-B). Increasing the node size also              node: it can manipulate network messages between TEEs
 conflicts with prior TEE-aided designs that aim for smaller              and arbitrarily start, stop, and invoke TEEs. Moreover, the
 node sizes [48], [49].                                                   adversary can provide the TEE with stale data to rollback
Reconfiguration (RC). RC enables the system to dynamically                the state [12], [14], [15], [53], [54], [4]. We do not consider
modify its node set by adding or removing nodes. It is a more             cloning attacks [12], [13] or micro-architectural side-channel
general approach than simple recovery, as a recovering node               attacks [55], [56], [57], as they are orthogonal to this work.
can rejoin the system as a newly added participant without                Cloning attacks can be mitigated using TPM PCR [14],
having to recover any prior state. Notable confidential BFT               while side-channel attacks can be addressed via software-level
protocols, CCF [8] and Recipe [50] adopt this design. Since               countermeasures, particularly in cryptographic libraries such
RC does not affect the log replication, no extra overhead is              as OpenSSL and Intel SGX SSL [58], [59].
introduced. Its drawbacks appear during recovery:                         Network Model. We adopt the partially synchronous network
• High-cost membership changes. RC typically requires com-                model [60], which is commonly used in consensus [61], [62],
 plex recovery designs. To ensure safety, most protocols                  [63], [64]. In this model, there is an established bound ∆
 require nodes to reach consensus on the order of membership              and an undefined Global Stabilization Time (GST). After the
 changes, so that all nodes apply them consistently. This                 GST point, the delivery of any message transmitted between
 coordination can delay concurrent log commits. For example,              two correct nodes within the ∆ limit is guaranteed. That is, the
 RC in Raft requires two rounds of consensus to perform a                 system behaves synchronously following the GST. Liveness
 membership change safely.                                                is guaranteed after GST.
• Expensive synchronization. A rejoining node must syn-
  chronize the entire application state from scratch [8]. For             B. Problem Statement
  applications with large state, such as blockchains, this process           In confidential BFT consensus, each node runs a customized
  can involve transferring hundreds of gigabytes of data and              leader-based CFT protocol (Appendix A3) inside its TEE to
  may take several hours to days [51], [52].                              commit and execute client transactions. The protocol proceeds
Summary. Existing recovery approaches in confidential BFT                 in a sequence of epochs, each representing a logical leader
consensus follow a one-size-fits-all design: each approach                term. Ideally (i.e., without rollback issues), one node is elected
tries to protect or recover all persistent states with the same           as the leader in each epoch, and a majority of nodes agree
mechanism. As a result, the system pays the worst-case cost               on this choice (i.e., election safety). The leader batches client
for the least suitable state type, rather than exploiting protocol-       transactions, proposes them as log entries, and replicates each
level knowledge and the characteristics of metadata and logs.             entry to the logs of all nodes pi within the TEE. Each log
This observation motivates a protocol-aware recovery design               entry is tagged with an epoch and log index (ep, idx). Here,
that customizes and strengthens recovery for each state type.             ep denotes the epoch in which the entry is proposed, and
                                                                          idx denotes its position in the log. The leader appends each
                  IV. P ROBLEM S TATEMENT
                                                                          entry to its local log and persists it. It then replicates the
A. System Model                                                           entry to all followers. Each follower appends and persists
   Following the model of prior confidential BFT consen-                  the entry before sending an acknowledgment. Upon receiving
sus [4], [8], we consider a distributed system maintained by              acknowledgments from a majority of followers, the leader
n = 2f +1 nodes {p1 , p2 , ... , pn }, each equipped with a TEE.          marks the entry as committed.
Confidential BFT consensus runs entirely inside each node’s                  As in prior work [65], confidential BFT consensus provides
TEE. We assume a Public Key Infrastructure (PKI): each node               two fundamental guarantees: safety and liveness. Safety re-
pi has a public/private key pair, denoted by (pki , ski ), in which       quires the service to be linearizable: no two correct nodes
the private key is accessible only within the node’s TEE. A               commit different entries at the same log index idx. Liveness
message m signed with ski is denoted by mσi . We assume                   ensures that every transaction submitted by a client is eventu-
that a finite set of clients sends transactions to nodes’ TEEs            ally committed.
for confidential BFT service over encrypted and authenticated             Recovery under TEE Rollbacks. The integrity guarantees
channels (e.g., TLS).                                                     of TEEs allow confidential BFT consensus to maintain safety
Threat model. We assume an adversary A that can corrupt                   and liveness during normal execution [65]. However, recovery
at most f nodes at any time and any number of clients.                    in the presence of TEE rollbacks introduces additional chal-
Following prior study [4], [8], corrupted nodes are Byzantine,            lenges. In leader-based consensus protocols such as Raft [11],
i.e., behaving arbitrarily, with the exception that TEE integrity         safety and liveness can be refined into three critical conditions



                                                                      4
during recovery. Specifically, to preserve safety, the recovery                                     Consensus Node               Peers
procedure must satisfy the following two properties:
                                                                          Trusted Region
                                                                                                               Log
Definition 1 (Election Safety). For any epoch ep and any two
distinct nodes pi and pj , it is impossible for both pi and pj            Attack Surface          Disk       Metadata
to be elected leaders in ep.                                              Update Dataflow
                                                                                                 Trusted     Consensus
Definition 2 (Leader Completeness). For any log entry en                                         Counter      Engine
                                                                         Recovery Dataflow
committed at index idx in epoch ep, every leader elected in a
later epoch ep′ > ep must contain en at index idx in its log.                                Fig. 1: Architecture of C HIMERA.
   Election safety guarantees a unique leader for each epoch,
while leader completeness ensures that all committed log en-           metadata preserves election safety, but entering a higher epoch
tries are preserved across subsequent leaders. During recovery,        may temporarily reduce availability. This trade-off makes
the system must maintain correct commitment progress and               metadata well-suited for TC-based local freshness protection,
restore all committed entries.                                         which enables precise, node-local recovery. Second, metadata
   For liveness, the recovery procedure must ensure that an            is scalar and can be stored in a register-like form. In particular,
uncorrupted recovering node can eventually resume participa-           the epoch increases monotonically during leader transitions,
tion, as formalized below.                                             which aligns naturally with the TC design. Moreover, the low
                                                                       update frequency of metadata ensures that introducing TC
Definition 3 (Recovery Liveness). For any uncorrupted node             incurs minimal additional overhead.
pi , if pi starts recovery and remains uncorrupted, then pi            Log. Each node maintains a log that consists of ordered
eventually completes recovery and resumes participation in             entries. An entry is considered committed once it has been
the protocol.                                                          replicated to a majority of nodes. Leader completeness ensures
                                                                       that all committed entries are preserved across subsequent
                  V. C HIMERA OVERVIEW
                                                                       leaders. To recover safely, we only need to make sure that
   We present an overview of C HIMERA, a protocol-aware re-            the commit process is correct and that committed entries are
covery framework for confidential BFT consensus. C HIMERA              durable. Each node that replicates an entry must ensure the
first characterizes the persistent state maintained by leader-         entry’s durability before it is considered committed. By doing
based consensus protocols (§V-A). It then applies a recovery           so, all committed entries remain available for future recovery
strategy specialized for each state type (§V-B). This frame-           and are preserved by subsequent leaders.
work can achieve rollback-resilient recovery with both high               Logs differ from metadata in distribution and form. Unlike
performance and high availability, while preserving election           metadata, which is node-local, logs are replicated across
safety, leader completeness, and recovery liveness.                    multiple nodes in a structured way. Therefore, we can leverage
                                                                       the replicas to reconstruct a safe log during recovery. More-
A. Characterizing System State
                                                                       over, logs are large and stored on disk, making it infeasible
   In a leader-based CFT consensus, three types of system state        to maintain a direct mapping to a trusted counter. Version
require external persistence: metadata, logs, and snapshots.           numbers can be used to detect inconsistencies, but they do
Snapshots are primarily used to accelerate log synchronization;        not enable full recovery of log contents.
therefore, their recovery is not essential for preserving safety
guarantees and does not need rollback-resilient mechanisms. In         B. Protocol-Aware State Recovery
this work, we focus on the two safety-critical states—metadata            We leverage protocol-level knowledge, i.e., the inherent
and logs—as summarized in Table II.                                    characteristics of logs and metadata, to carefully tailor recov-
Metadata. Metadata denotes a set of safety-critical control            ery strategies, as illustrated in Fig. 1.
variables that govern leader election and epoch transitions.           Metadata Recovery. C HIMERA protects metadata using a
Metadata is updated infrequently and generally changes during          trusted counter (TC), leveraging the counter’s node-local na-
leader transitions. It typically includes the current epoch and        ture to enable precise recovery. The TC enables a recovering
the node’s voting record within that epoch. For example, in            node to verify the freshness of sealed metadata locally, ensur-
Raft, metadata consists of currentTerm (the epoch identifier)          ing election safety without coordinating with other replicas.
and votedFor (the candidate voted for in that epoch). The epoch        This approach allows precise local recovery while avoiding un-
identifier allows nodes to recognize larger epochs and reject          necessary loss of availability, capturing the trade-off between
stale leadership attempts. The voting record prevents double           freshness guarantees and state continuity.
voting, ensuring election safety.                                         Raw TC alone, however, is insufficient: counter increments
   Two properties make metadata distinct for recovery. First,          and metadata sealing are not atomic, so a mismatch may
metadata is node-local: it records a node’s own view of con-           indicate either a rollback or a crash during the update window.
sensus. As a result, other replicas cannot reliably reconstruct        To address this, C HIMERA proposes a binding-update design,
the recovering node’s exact epoch or vote. Therefore, recovery         motivated by the similarity between metadata and the TC. The
must balance safety and availability: ensuring freshness of            binding-update design ties the epoch to the counter value to



                                                                   5
TABLE II: The characteristics of metadata, log, and snapshot.             2) TC interfaces. We use two TC operations:
              Safety      Recovery       System        Update             • ctr ← IncTC(k): Atomically increment the counter identi-
 State Type                                                                fied by key k by one and return the new value ctr.
              Critical   Requirement   Redundancy     Frequency

 Metadata        ✓         Precise          ✗           Low
                                                                          • ctr ← ReadTC(k): Return the current value ctr of the
 Log             ✓         Loose            ✓           High               counter identified by key k without modifying it.
 Snapshot        ✗         Loose            ✗           Low                  Each node maintains two trusted counters for different
                                                                          recovery purposes:
ensure consistent metadata recovery. When the counter and                 • T Cmd records metadata updates. Its value is bound to
epoch match, the node can safely restore the sealed metadata.              the epoch in sealed metadata, enabling rollback detection
Otherwise, the counter serves as a safe epoch anchor, enabling             and safe recovery when the sealed states and the counter
recovery without conservative epoch skipping.                              mismatch.
Log Recovery. For logs, C HIMERA relies on protocol-guided                • T Crole records a node’s role within the current epoch. It is
cluster recovery rather than per-entry TC protection. Consen-              initialized to 0 and incremented whenever the node changes
sus replication ensures that committed entries are preserved               its role between follower and leader. Thus, when the counter
on a quorum of replicas. During recovery, a node can query a               value is odd, the node is a leader; when it is even, the node
quorum of replicas to reconstruct a log that safely includes               is a follower.
all committed entries. To guarantee safety, each node that                Initialization. Each node boots from a hard-coded gene-
replicates an entry must ensure the entry’s durability before             sis configuration and initializes with default metadata (i.e.,
it is considered committed.                                               ep = 1) and an empty log. The configuration information,
   C HIMERA also tailors recovery to the node’s role to improve           including key pairs, is stored on local disks in an encrypted
availability. A recovering follower can use metadata to avoid             and authenticated form.
unnecessary log transfer during catch-up. A recovering leader,
                                                                             In this work, we consider a static configuration: a recovering
in contrast, can resume service after reconstructing a safe
                                                                          node can retrieve the configuration to obtain its own key
log without waiting for a new leader election. By leveraging
                                                                          pairs and the public keys of other nodes for communication.
both protocol-level guarantees and node role information,
                                                                          Dynamic reconfiguration is discussed in §XI.
C HIMERA enables safe and efficient log recovery while pre-
                                                                             The trusted counters are initialized to match the starting
serving availability.
                                                                          system state: T Cmd is set to 1 to align with the initial epoch,
                     VI. C HIMERA D ESIGN                                 while T Crole is set to 0 to represent the initial follower role.
                                                                          These initializations ensure that metadata freshness and node
A. Data Structures and Interfaces                                         role tracking start from a consistent, well-defined state.
Data Structures. C HIMERA handles the update and recovery
of metadata and logs separately.
1) Metadata. Metadata captures epoch and leader-election                  B. Metadata Recovery Mechanism
information, such as the current epoch ep and the node’s                     1) Metadata Update: C HIMERA protects metadata with
voting record. We denote the metadata of node pi as mdi .                 TC, since metadata is node-local and requires more precise
Each metadata update corresponds to an operation op (e.g.,                recovery to gain higher availability. For each metadata update,
sending an election vote), which is executed only after the               the node first increments T Cmd and then seals the metadata
updated metadata has been sealed.                                         together with the counter value, following an inc-then-store
2) Log. Each node pi maintains a local log logi , which consists          scheme [14] (Fig. 2a).
of a sequence of entries enj indexed by j. A log entry is                    Using a TC provides a local freshness check for metadata,
identified by its epoch and index; we refer to this pair (ep, j)          but a raw TC is insufficient: if a crash occurs after the counter
as the entry’s epoch-index tag. We compare epoch-index tags               is incremented but before the updated metadata is sealed,
lexicographically: (ep1 , idx1 ) > (ep2 , idx2 ) iff ep1 > ep2 , or       recovery observes a counter mismatch and cannot distinguish
ep1 = ep2 and idx1 > idx2 .                                               between a benign crash and a rollback. C HIMERA resolves
Interfaces. C HIMERA relies on sealing interfaces for protect-            this ambiguity by binding the scalar epoch to T Cmd . The key
ing states stored in external storage, and on TC interfaces for           rule is to advance both values in lockstep: every metadata
maintaining freshness across crashes.                                     update increases the epoch by exactly one and performs
1) Sealing interfaces. TEEs provide two sealing interfaces:               one IncT C(md). This design turns T Cmd into a record of
• Seal(data, h): Encrypt data with the TEE-internal sealing               epoch transitions. When the counter value matches the sealed
  key and then store the associated encrypted data at the                 metadata, the node can safely restore the full metadata from
  location identified by handle h in external storage.                    persistent storage. In the event of a mismatch, C HIMERA uses
• data ← UnSeal(h): Retrieve the sealed data from an                      the counter value as the recovered epoch. To maintain safety,
  external storage with the handle h, decrypt it, and return the          the node disables voting in this epoch due to uncertainty about
  associated plaintext data.                                              its previous votes.



                                                                      6
   When a node receives a request with a higher target epoch
ep∗ than its current epoch ep, it executes the following loop                 Update process       Recovery process       Potential crash point
until it reaches ep∗ :                                                                                E3      (3) Execute update
① Invoke IncTC(md) to advance the trusted counter T Cmd                     Trusted     (1) IncTC("md")                   (2) Seal(md || ctr)
by one step and obtain the new value ctr.                                   Counter                        Metadata
                                                                                          E1                               E2
② Update the metadata: increment md.epoch by 1 and set
md.vote according to the request type. For all intermediate                              (a) Update process of metadata
epochs, set md.vote to non-voting. When md.epoch reaches                                                       (3) Compare Counter
the target ep∗ , set md.vote to the selected candidate if the                Trusted    (1) ReadTC("md")                     (2) UnSeal()
                                                                             Counter                       Metadata
request is a vote request; otherwise, it remains non-voting.
③ Persist metadata: call Seal(md|ctr, hmd ) to store the
                                                                                        (b) Recovery process of metadata
updated metadata along with the counter value.
④ Send vote reply: for vote requests, the node sends the
reply when md.epoch reaches ep∗ . The reply is sent after the                       Fig. 2: Recovery design of metadata.
corresponding metadata has been sealed to ensure consistency.
   A multi-epoch jump is executed as a sequence of single-             • E2 Crash. At E2, T Cmd has advanced by one epoch,
epoch updates. All intermediate epochs are marked non-voting.           but the corresponding metadata has not yet been sealed.
The final epoch records a candidate only if the triggering              This creates a mismatch between the counter and the sealed
request is a granted vote. This binding update keeps md.epoch           metadata. This mismatch is indistinguishable from an actual
synchronized with T Cmd and prevents the node from casting              rollback, rendering detection unreliable. The binding update
a different vote in any previous epoch.                                 design resolves this issue by using the counter value as the
   2) Metadata Recovery: As shown in Fig. 2b, metadata                  recovered epoch. Since no sealed metadata is available, the
recovery follows the update scheme. The node first checks               node must disable voting in that epoch to prevent casting or
the freshness of the sealed metadata using T Cmd . If the               resending votes with incomplete metadata.
sealed counter value matches the current counter value, the            • E3 Crash. At E3, both T Cmd and the sealed metadata
node recovers from the sealed state. Otherwise, it recovers             have been updated. If no rollback occurred, the counter and
to a safe epoch without risking double voting. The binding-             sealed metadata match, allowing the node to safely restore the
update design binds T Cmd to the current epoch, maintaining             sealed metadata. With the complete metadata, any previously
a correspondence between the counter value and epoch. This              recorded voting behavior can also be safely replayed. If a
correspondence provides a reliable anchor for safe recovery.            rollback occurred, a mismatch between the counter and the
The recovery process then proceeds as follows:                          metadata will be observed. Recovery then uses the counter
① Load sealed metadata: the node invokes UnSeal(hmd ) to                value to restore the epoch and marks the node as non-voting.
retrieve md′ together with the sealed counter value ctr′ .
② Read current counter: the node calls ReadTC(md) to obtain            C. Log Recovery Mechanism
the current counter value ctr for freshness verification.                 A recovering node begins log recovery once metadata
③ Check counter match: if ctr = ctr′ , the sealed metadata is          recovery has established a safe epoch. Unlike metadata, logs
fresh. The node sets md ← md′ and finishes recovery.                   are replicated according to consensus rules, so C HIMERA can
④ Handle mismatch: if ctr ̸= ctr′ , the sealed metadata may            recover them from other replicas. The role counter T Crole
be outdated. Then the node starts metadata repair with ctr.            allows a recovering node to determine its previous role before
⑤ Repair metadata: the node reconstructs metadata by setting           the crash, and C HIMERA uses different recovery paths for the
md.epoch ← ctr and md.vote to non-voting. The node then                two cases. Alg. 1 summarizes both paths.
resumes execution using this repaired metadata.                           1) Log Recovery for Followers: To ensure correctness,
   3) Crash Window Analysis: The inc-then-store update is              the commit process must be properly executed: every entry
not atomic. Crashes can occur in three intervals, as illustrated       considered committed must be durably stored on a quorum of
in Fig. 2a: before incrementing T Cmd (E1); after incrementing         replicas. A recovering follower, therefore, needs to catch up
T Cmd but before sealing the metadata (E2); and after sealing          with a safe leader. Generic DCR-style recovery may collect
the metadata but before sending the vote reply or completing           f +1 replies to identify the latest leader and ensure that entries
the triggering operation (E3). Our analysis shows that each            still being replicated are not lost. In C HIMERA, metadata
crash falls into one of two cases. In the match case, the sealed       recovery already restores a safe epoch for the follower. There-
metadata can be safely restored. In the mismatch case, the             fore, a recovering follower can synchronize its log from any
node repairs to a non-voting epoch.                                    leader whose epoch is no lower than its own.
• E1 Crash. At E1, the node has prepared an update, but                   The follower recovery path leverages this observation to
  neither the epoch nor the T Cmd has changed. Since nothing           reduce message overhead. First, the follower unseals its local
  has been written to persistent metadata, there is no mismatch.       log from persistent storage and determines its last-entry tag. It
  Recovery can safely restore the previous metadata, and no            then broadcasts a F OLLOWER R ECOVER message containing
  actions or effects have been produced.                               its recovered epoch and last-entry tag (Alg. 1, L6–L12). Only



                                                                   7
Algorithm 1 Log recovery at node pi                                     follower can then complete the recovery process.
 1: resp ← 0 // the number of R EPLY R ECOVER messages received
 2: tagmax ← ⊥ // the most up-to-date last-entry tag observed              2) Log Recovery for the Leader: The leader recovery path
 3: src ← ⊥ // the ID of the node selected as the log source            is designed to minimize the system’s unavailability following
 4: done ← false // whether follower recovery has completed             a leader crash. If the crashed leader waits for a timeout and
 5:                                                                     a subsequent election, the system remains unavailable during
 6: upon ⟨L OG R ECOVER, hlog ⟩:
                                                                        that interval. In C HIMERA, the crashed leader can resume
 7:   logi ← UnSeal(hlog )
 8:   epi ← recovered metadata epoch of pi                              service after recovery.
 9:   lasti ← last-entry tag of logi                                       The leader first checks T Crole to confirm that it crashed
10:   roleCtri ← ReadTC(role)
11:   if roleCtri is even then
                                                                        during a leader phase. It then advances the epoch and broad-
12:      broadcast ⟨F OLLOWER R ECOVER, i, epi , lasti , non⟩           casts a L EADER R ECOVER message (Alg. 1, L13–L16). A node
13:   else                                                              replies only if it can vote in that epoch. Before sending its
14:      epi ← epi + 1                                                  reply, the node catches up to the new epoch and records that
15:      broadcast ⟨L EADER R ECOVER, i, epi , lasti , non⟩             it has voted for the leader of that epoch. In this way, every
16:   end if
17:
                                                                        follower that contributes to the recovery quorum will not vote
18: upon receiving ⟨F OLLOWER R ECOVER, j, epj , lastj , non⟩:          for another candidate in the same epoch (Alg. 1, L28–L38).
19:   if rolei ̸= leader or epi < epj then return end if                   The recovering leader collects f +1 replies and selects the
20:   suffix ← log entries after lastj
21:   send ⟨R EPLY L OG, epi , suffix, i, non⟩ to pj                    log with the most up-to-date last-entry tag. This quorum
22:                                                                     is sufficient to preserve all committed entries. By quorum
23: upon receiving ⟨R EPLY L OG, ep, suffix, j, non⟩:                   intersection, any entry committed before the crash must appear
24:   if non is invalid or done or ep < epi then return end if          in at least one of the replies. Selecting the log with the latest
25:   append valid suffix to logi                                       last-entry tag ensures that all committed entries are included,
26:   done ← true
27:
                                                                        preventing any loss (Alg. 1, L40–L48).
28: upon receiving ⟨L EADER R ECOVER, j, epj , lastj , non⟩:               If the recovering leader resumes in the old epoch without
29:   if epi > epj or (epi = epj and votei = disabled) then             advancing, entries that were not yet committed could cause
30:      return
31:   end if                                                            inconsistencies. Depending on which followers respond, such
32:   if epi ≤ epj then                                                 entries may or may not be included in the f +1 selected log. If
33:      // use the binding update of metadata                          the leader resumes in the old epoch after losing these entries,
34:      epi ← epj                                                      it could propose conflicting entries at the same index under
35:      votei ← disabled                                               the same epoch-index tag. Moving to a new epoch eliminates
36:   end if
37:   lasti ← last-entry tag of logi                                    this ambiguity. After synchronization, the leader extends the
38:   send ⟨R EPLY R ECOVER, epi , lasti , i, non⟩ to pj                recovered log in the new epoch, and any unrecoverable,
39:                                                                     uncommitted suffix from the old epoch is safely discarded.
40: upon receiving ⟨R EPLY R ECOVER, ep, last, j, non⟩:
41:   if non is invalid then return end if                                 3) Log Recovery Acceleration: C HIMERA further reduces
42:   resp ← resp + 1                                                   recovery and normal-case overhead through two optimizations
43:   if last is more up-to-date than tagmax then                       enabled by protocol-guided log recovery.
44:      tagmax ← last; src ← j
45:   end if                                                            Unblocking Log Recovery. If the leader remains unchanged,
46:   if resp = f + 1 then                                              it tracks the next entry index for each follower. For follower
47:      synchronize log with psrc and resume proposing                 pi , this index is denoted as idxi . After unsealing its log
48:   end if
                                                                        from persistent storage, pi can complete recovery once it has
                                                                        caught up to idxi , without synchronizing with the leader’s
                                                                        latest log. This mechanism accelerates recovery by eliminating
the leader whose epoch is no lower than the follower’s epoch            unnecessary synchronization.
responds. The reply contains the log entries after the follower’s       Background Log Persistence. C HIMERA moves log persis-
last-entry tag. This allows the follower to reuse its locally           tence off the commit critical path to reduce synchronous
persisted entries and fetch only the missing suffix (Alg. 1,            I/O during normal-case replication. Under high load, a node
L18–L26). If the tags do not match, the follower replaces any           keeps newly appended log entries in memory and flushes
inconsistent entries with the corresponding entries from the            them to disk in the background. This design is safe because
leader’s log.                                                           log recovery relies on the cluster rather than the persistent
   The follower accepts the first valid leader reply that passes        storage as the sole source. By contrast, some systems require
the epoch check. It appends the corresponding suffix and                every committed log entry to be durably written to local
completes recovery. Any subsequent replies are ignored. If              storage. Such systems cannot use background persistence.
no valid leader reply is received, a standard leader election           Losing unflushed entries in these cases would compromise
eventually produces a leader under partial synchrony. The               safety.



                                                                    8
    VII. C ORRECTNESS A NALYSIS AND V ERIFICATION                     • I6 (Leader append-only). While acting as a leader, a node
                                                                       only appends at indices strictly greater than its current lastIdx
A. Correctness Analysis                                                and never rewrites an earlier entry of its own log.
   TEE integrity and the underlying CFT consensus proto-              • I7 (Up-to-date vote rule). A valid vote is generated
col together guarantee consensus safety and liveness during            only if the candidate’s last-entry tag (lastEp, lastIdx) is
normal execution [65]; our goal is therefore to show that              lexicographically greater than or equal to the voter’s own.
C HIMERA’s recovery procedure preserves the three properties          Invariants I1–I2 follow from the metadata update/recovery
of §IV-B in the presence of rollback attempts on sealed               procedure of §VI-B; I3–I4 follow from the leader-recovery
storage. We collect primitive assumptions inherited from the          steps of §VI-C; I5–I7 are inherited from the underlying leader-
TEE and protocol-level invariants maintained by §VI-B–                based CFT consensus (e.g., Raft [11]).
§VI-C, state three formal theorems capturing those properties,        Theorem 1 (Election Safety). In any epoch ep, at most one
introduce two supporting lemmas, and prove the theorems in            node can act as a leader.
dependency order.
                                                                      Theorem 2 (Leader Completeness). If an entry is committed
   By the threat model in §IV-A, trusted hardware provides the
                                                                      in epoch ep, every leader that assumes leadership in any later
following primitive guarantees:
                                                                      epoch ep′ > ep contains that entry in its log.
• A1 (Counter monotonicity). For any counter we use, the
 value returned by ReadTC is monotonically non-decreasing             Theorem 3 (Recovery Liveness). Every recovering, uncor-
 across crashes, and IncTC returns a value strictly greater           rupted node eventually completes its recovery procedure.
 than any value previously returned for the same counter.               We now state two supporting lemmas and then prove
• A2 (Seal authenticity). A successful UnSeal returns a               Theorems 1–3 in dependency order.
 plaintext that was produced inside an enclave with the
 matching measurement at some earlier time.                           Lemma 1 (Recovered Epoch Monotonicity). For any correct
                                                                      node, the epoch after any crash recovery is no smaller than
• A3 (Enclave integrity). Code running inside an enclave              its epoch immediately before the crash.
 executes as specified; the adversary can delay or drop enclave
 messages, but cannot tamper with authenticated messages.             Proof. Let pi be a correct node whose pre-crash epoch is ep.
Each node maintains two trusted counters: T Cmd for metadata          By I1, ep equals the value ctr returned by pi ’s last successful
updates and T Crole for role tracking.                                IncTC(md) (with ep = 1 if pi has never completed a metadata
Protocol Invariants. The procedures in §VI-B–§VI-C main-              update). By A1, ReadTC(md) at recovery returns some v ≥
tain the following invariants at every correct node.                  ctr = ep. Recovery then proceeds as follows: in the match
                                                                      case, the restored epoch equals the sealed ctr′ = v (which
• I1 (Counter–epoch binding). After every successful meta-            by A2 and I1 is authentic) and thus equals v ≥ ep; otherwise
 data update, the sealed metadata satisfies md.epoch =                (mismatch or UnSeal failure), I2 sets the epoch to v ≥ ep.
 ctr, where ctr is the value returned by the corresponding
 IncTC(md); the in-memory epoch always equals md.epoch.               Lemma 2 (Vote Uniqueness per Epoch). In each epoch, a
• I2 (Mismatch repair). During recovery, if ReadTC(md)                node can grant a valid vote to at most one node.
 differs from the sealed counter value, the node sets its epoch       Proof. In C HIMERA, a valid vote is either a normal election
 to ReadTC(md) and disables voting in that epoch.                     vote or a recovery vote carried by a R EPLY R ECOVER message.
• I3 (Fresh epoch for leader recovery). A recovering leader           Fix an epoch ep and a node pi . In a normal election, pi can
 uses T Crole to identify that it crashed in a leader phase.          vote for ep only when it first advances its epoch to ep. I1
 Before resuming, it advances metadata to a new epoch and             and A1 ensure that pi has at most one voting opportunity
 obtains at least f +1 R EPLY R ECOVER responses from nodes           in ep. Upon recovery, pi either restores the sealed vote or
 that were eligible to vote in the new epoch.                         disables voting for the recovered epoch by I2. Thus, pi
• I4 (Safe log selection in leader recovery). Before a                cannot cast another vote in ep. In leader recovery, sending
 recovering leader proposes in a fresh epoch, it collects             a R EPLY R ECOVER counts as a recovery vote. By I3, pi sends
 R EPLY R ECOVER responses from at least f +1 nodes and               this message only if it is still eligible to vote in ep, and it then
 adopts the responder’s log with the most up-to-date last-entry       becomes non-voting. Therefore, pi can grant a valid vote to
 tag (ep, lastIdx).                                                   at most one node in epoch ep.
• I5 (Log prefix consistency). A follower appends a suf-              Proof of Theorem 1. Assume, for contradiction, that two dis-
 fix from a leader only if the suffix’s predecessor tag               tinct nodes pa ̸= pb act as leaders in the same epoch ep. Let
 (prevEp, prevIdx) matches the follower’s local entry at              Qa and Qb be their vote quorums, where a recovery quorum of
 prevIdx. If the tags do not match, the follower truncates            R EPLY R ECOVER messages is treated as a vote quorum. Then
 the conflicting suffix and retries from an earlier matched           |Qa | ≥ f + 1 and |Qb | ≥ f + 1. Since n = 2f + 1, we have
 tag. This rule ensures that log synchronization preserves a          Qa ∩ Qb ̸= ∅. Let pc ∈ Qa ∩ Qb . Then pc grants a valid vote
 common prefix between the leader and follower.                       to both pa and pb in epoch ep, contradicting Lemma 2.



                                                                  9
  The following technical lemma underlies the proof of                    B. Formal Verification.
Leader Completeness.                                                         We develop a Maude [66] specification of C HIMERA atop
Lemma 3 (Leader Append-Only Preservation). If LT becomes                  Braft and model check it against both safety and liveness
the leader with entry en at index k in its log, then LT preserves         guarantees expressed in linear temporal logic (see Appendix C
en at index k throughout its leadership. Moreover, any new                for details). We chose Maude as it is a well-established
entry proposed by LT is appended after index k.                           formal specification language and analysis framework that has
                                                                          been successfully applied to a broad range of distributed and
Proof. When LT becomes the leader, its last log index is at               networked systems [67], [68], [69], [70]. Within the explored
least k. By I6, LT only appends new entries after its current             bound, the model checker reports no counterexample to the
last log index and never rewrites earlier entries while it remains        three recovery properties and protocol liveness.
the leader. Therefore, en remains at index k, and every new
                                                                                             VIII. I MPLEMENTATION
entry proposed by LT is appended after index k.
                                                                             We implement C HIMERA on top of Braft [20] and
Proof of Theorem 2. Let en be committed at index k in epoch               ZooKeeper [71], which use the leader-based CFT consensus
ep by leader L. By the commit rule, en is replicated to a                 protocols Raft and Zab, respectively.1 We refer to these
quorum Qc with |Qc | ≥ f + 1, each storing en at (ep, k).                 implementations as C HIMERA-B and C HIMERA-Z. For TEE
   We prove by strong induction on T > ep that every leader               support, we port C HIMERA to VM–based TEEs, i.e., Intel
LT in epoch T holds en at index k throughout its tenure.                  TDX, and apply additional optimizations to mitigate the per-
                                                                          formance overhead of TEEs. (The rationale for choosing Braft,
   Base case (T = ep + 1). Consider the first leader Lep+1
                                                                          ZooKeeper, and TDX is discussed in §I.)
after en is committed. By the quorum intersection argument,
                                                                             We adopt the software-based counter TIKS [4] for metadata
Lep+1 adopts a log containing the committed prefix including
                                                                          recovery. Moreover, we use Narrator-Pro [43] as the trusted
en at k. Lemma 3 then ensures en remains at k.
                                                                          counter to safely update the counter within a single round
   Induction hypothesis. Assume that every leader LT ′ for
                                                                          of communication. For sealing functionality, we invoke Intel
ep < T ′ ≤ T contains en at k throughout its tenure.
                                                                          SDK [72] to retrieve the measurement (mr td) of TDX, and
   Induction step (T → T + 1). Consider leader LT +1 .                    derive a stable sealing key from it. This key is used to encrypt
   - If LT +1 assumes leadership via a normal election, let Qv            and decrypt data for the persistent storage outside TEEs.
be its vote quorum. Since |Qv |, |Qc | ≥ f + 1, they intersect at         Implementation atop Braft. We list required modifications
some r. By induction hypothesis and I5, r’s log contains en               on Braft to realize C HIMERA:
at k. By I7, r votes only if LT +1 ’s last-entry tag ⪰ r’s, so I5         • Metadata. Raft’s metadata consists of two fields: current-
ensures LT +1 ’s log contains en.                                          Term and votedFor (§V-A). In Braft, these fields are combined
   - If LT +1 assumes leadership via recovery, let Qr be the               into a single on-disk structure called raftMeta. In C HIMERA,
recovery-quorum with Qr ∩ Qc ̸= ∅ and r in it; by I4, LT +1                we attach a counter value to each update of raftMeta to support
adopts the log with largest last-entry tag ≥ r’s log, which I5             precise recovery.
ensures preserves the committed prefix with en at k.
                                                                          • Log. In C HIMERA-B, new log entries are first buffered in
   Thus, by induction, every leader in epoch T + 1 contains
                                                                           memory. Their persistence is handled by a background thread
en at index k before proposing, completing the proof.
                                                                           that runs during idle periods. This design reduces critical-
                                                                           path latency and allows higher throughput compared to Braft
Proof of Theorem 3. Consider a correct node pi beginning
                                                                           (§IX-B). For recovery, we reuse Braft’s existing replicator
recovery.
                                                                           component to transmit log entries to recovering nodes.
   (1) Metadata recovery. pi invokes UnSeal and
                                                                          Implementation atop ZooKeeper. Modifications on
ReadTC(md), both O(1) TEE operations. On match, it
                                                                          ZooKeeper, including the log, are similar to those on Braft.
restores md′ (I1); on mismatch or UnSeal failure, it applies
                                                                          Thus, we only list the differences:
I2. In either case, this phase terminates.
                                                                          • Metadata. Unlike Raft’s single-phase election, the Zab pro-
   (2) Log recovery. If pi is a follower, after GST a current              tocol uses a two-phase process: discovery and synchroniza-
or newly elected leader is reachable within bounded time. The              tion. In this process, a prospective leader is first nominated
leader replies to F OLLOWER R ECOVER with the missing suffix,              and is only promoted after receiving acknowledgments from
which pi appends to complete recovery. If pi is a recovering               a majority of followers [22]. Specifically, after Fast Leader
leader, it broadcasts L EADER R ECOVER; after GST, either f +              Election (FLE), the newly elected leader enters the discovery
1 nodes in the fresh epoch respond with R EPLY R ECOVER, in                phase. It proposes a fresh epoch ep∗ , which is greater than
which case I4 lets pi adopt the most up-to-date log and resume             any previously observed. Each follower persists this value as
proposing, or another leader is eventually established and pi              acceptedEpoch and returns an acknowledgment to the leader.
recovers via the follower path. A nonce in recovery messages
                                                                           In Zab, followers do not need to record which node they
prevents responses from being replayed.
                                                                           voted for. Election safety is ensured instead by two metadata
   Each phase completes in bounded time after GST, so pi
eventually rejoins the system.                                              1 Source code is at https://github.com/Artifacts2026/CHIMERA.




                                                                     10
 fields: acceptedEpoch and currentEpoch. Together, these fields         bandwidth to 1 Gbps per node and enforcing a 40 ms round-
 guarantee that at most one leader is recognized by the                 trip latency with ±4 ms jitter. Note that WAN evaluation is
 majority. Because the two fields are updated under different           performed in emulation because TEE-enabled instances are
 conditions, we use separate counters to protect them inde-             restricted to specific cloud regions; this reflects deployment
 pendently.                                                             constraints rather than a design flaw.
                                                                        Parameters and Metrics. We vary the fault-tolerance pa-
                      IX. E VALUATION                                   rameter f ∈ {1, 2, 4, 10, 20}. The leader processes client
   We evaluate the performance of C HIMERA-B (built on                  requests in batches of 256, with each log entry containing a
Braft [20]) and C HIMERA-Z (built on ZooKeeper [71]). Since             256 B payload. To balance memory consumption and network
ZooKeeper comprises additional components beyond consen-                utilization, the system bounds the number of in-flight log
sus and its performance is influenced by multiple factors,              entries at 5120. In addition to throughput and latency, we
we focus our main evaluation on C HIMERA-B, especially in               measure recovery time, defined as the interval from when a
comparison with its counterparts, while providing an over-              faulty node restarts until it fully recovers and resumes service.
head profiling of C HIMERA-Z. We consider four recovery                 B. Fault-Free Performance
taxonomies (§III) and adopt them on Braft as baselines. For
consistency, all protocols running inside TDX perform disk                 This section evaluates C HIMERA-B’s performance of log
I/O through sealing.                                                    commitment in the fault-free scenarios with no failures or
                                                                        leader changes. We compare C HIMERA-B with Braft-TC,
• Braft-TC adopts a software-based counter to protect state
                                                                        Braft-RFT, and Braft-DR. As Braft-DCR and Braft-RC in-
  updates as Engraft [4]. Each counter increment requires two
                                                                        troduce no additional overhead in normal-case operations
  rounds of broadcast to advance the state.
                                                                        compared to Braft-DR, their results are not presented in detail.
• Braft-RFT follows FlexiBFT [73] by increasing the node                Due to space constraints, we present the throughput–latency
 size from 2f + 1 to 3f + 1 and employs a TC only at the                scalability of C HIMERA-B in Appendix B.
 leader to prevent equivocating proposals.                                 1) Performance in WAN: We evaluate throughput and
• Braft-RC adopts the original reconfiguration design of Raft,          latency under a WAN deployment while varying the fault
 implemented through joint consensus [11].                              threshold f (Fig. 3a and 3b). In this setting, communication
• Braft-DCR follows Achilles [18] to ask a recovering node              overhead and bandwidth limits dominate performance.
 to skip two epochs for safety. In Braft, the Pre-Vote mecha-              During each transaction, Braft-TC requires two additional
 nism bounds the growth of epoch, making DCR feasible.                  communication rounds to interact with its TC for securely
In addition to the four baselines, we introduce Braft-Direct            recording log changes. According to Raft’s replication seman-
Recovery (Braft-DR) to evaluate the overhead of rollback                tics, this results in a total of five communication rounds to
resilient solutions. In Braft-DR, a node restores its state             complete a transaction, incurring substantial latency. Braft-
directly from sealed data on untrusted storage.                         RFT, by contrast, adopts a 3f + 1 configuration, which, for
   We evaluate C HIMERA-B against all baselines under both              the same fault-tolerance level f , entails more nodes and thus
fault-free and faulty scenarios to answer three questions:              higher processing overhead. Moreover, an additional counter
Q1: How does C HIMERA perform with varying nodes in                     update during leader persistence incurs one more communica-
 WAN and LAN compared to its counterparts? (§IX-B)                      tion round, which further degrades WAN performance.
                                                                           Both C HIMERA-B and Braft-DR operate with 2f + 1 nodes
Q2: What is the performance of C HIMERA’s recovery proto-               and incur no additional normal-case overhead, which explains
 col, where do the primary bottlenecks lie, and how does it             their relatively higher throughput in this environment. Never-
 compare to prior approaches? (§IX-C)                                   theless, C HIMERA-B’s recovery mechanism does not rely on
Q3: How much overhead do TEE-related operations intro-                  the completeness of on-disk logs, enabling it to safely defer
 duce, and how effective are our optimizations? (§IX-D)                 disk writes to background operations and thus reduce normal-
                                                                        case commit latency. This design yields an average throughput
A. Experimental Setup                                                   improvement of approximately 10% over Braft-DR; however,
  We conducted all experiments on a public cloud platform               as communication dominates in the WAN scenario, the advan-
using up to 61 Intel TDX-enabled instances, with one instance           tage from reduced I/O latency is less pronounced.
per node. Each node ran on a dedicated virtual machine                     2) Performance in LAN: We also evaluate the throughput
provisioned with 4 vCPUs and 16 GB of RAM, running Linux                and latency of C HIMERA in a LAN deployment to minimize
kernel 5.10 LTS (64-bit).                                               the effect of network communication (Fig. 3c and 3d). As
  We evaluate C HIMERA under two deployment scenarios:                  the network communication cost is negligible in a LAN
Local Area Network (LAN) and Wide Area Network (WAN).                   environment, local processing and I/O overhead become the
Both are configured using the Linux tc tool for traffic shaping.        dominant factors affecting performance.
In the LAN setting, the per-node bandwidth is limited to                   C HIMERA-B exhibits a pronounced throughput advantage
10 Gbps, and the inter-node RTT stays below 1 ms. In the                over all other variants, particularly at low fault tolerance levels,
WAN setting, we emulate wide-area conditions by limiting                due to its normal-case optimization that defers durable disk



                                                                   11
Throughput (kTPS)                                                                                                      Throughput (wTPS)
                                                Chimera-B                                Chimera-B      Braft-TC                                                                  Chimera-B                             Chimera-B
                    80                                                      400                                                                                                                               80
                                                Braft-DR                                 Braft-DR       Braft-RFT                                                                 Braft-DR                              Braft-DR




                                                             Latency (ms)                                                                                                                      Latency (ms)
                                                                                                                                           30
                    60                          Braft-TC                                                                                                                          Braft-TC                              Braft-TC
                                                                                                                                                                                                              60
                                                Braft-RFT                                                                                                                         Braft-RFT                             Braft-RFT
                                                                            300                                                            20
                    40
                                                                            200                                                                                                                               40

                    20                                                                                                                     10
                                                                            100                                                                                                                               20
                     0
                         1     2        4       10      20                        1       2       4      10     20                                    1              2        4   10      20                       1        2        4    10                  20
                              Number of faults                                           Number of faults                                                           Number of faults                                    Number of faults

                         (a) Normal case, WAN                                     (b) Normal case, WAN                                               (c) Normal case, LAN                                          (d) Normal case, LAN
                                               Fig. 3: Throughput and latency comparisons with varying nodes in WAN and LAN.
                                      TABLE III: Recovery overhead.
                                                                                                                                                                                  Shutdown        Restart
                                                                                                                                                                                    Recovery Completed



                                                                                                                                           Throughput (kTPS)
    Cost (s)                 Braft-DR       Braft-TC   Braft-RC                       Braft-DCR       C HIMERA-B




                                                                                                                                                                                                                                           Available Node #
    Prep.                      N/A            0.01       0.01                        N/A                   0.01                                            150                                                                            20
    Sync.                      6.15           6.31      117.01                      24.53               6.42 (∗ 18)
                                                                                                                                                           100
    Quie.                      N/A            N/A        N/A                      2 epochs                 N/A                                                                                                                            15
    Total                      6.15           6.32      117.02                24.53 + 2 epochs        6.43 (∗ 18.01)                                           50
                                                                                                                                                                00        2       4        6                  8        10       12       1410
writes to background operations (as explained above). When
f = 1, this optimization leads to a 68% improvement over
                                                                                                                                                                                       Time Window (s)
Braft-DR that follows the original Braft I/O semantics of                                                                                                Fig. 4: The fault recovery process of C HIMERA-B.
synchronous persistence.
   Braft-TC incurs further penalties from its additional com-                                                                         the node must reconstruct its state from scratch, which takes
munication and trusted counter operations, though in the                                                                              about 110–120 seconds for 5 GB of data. For data-intensive
LAN setting, these penalties are partially masked by the low                                                                          applications such as blockchains, where the state can reach
RTT. Braft-RFT, with its 3f + 1 configuration, experiences                                                                            several terabytes (e.g., Bitcoin [74]), the recovery time under
a more significant throughput drop as fault increases, due                                                                            RC can extend to hours or even days.
to the larger quorum size and corresponding processing and                                                                            Braft-DCR’s recovery requires disk loading (≈ 6s) and
message handling overhead. Overall, C HIMERA-B shows su-                                                                              network synchronization (≈ 18s). By contrast, C HIMERA
perior performance when network delays are not the primary                                                                            leverages its optimization to safely recover without network
bottleneck and storage-layer optimizations directly translate                                                                         catch-up, which takes about 6.42s. Without the optimization,
into substantial end-to-end performance gains.                                                                                        this stage takes about 18s.
                                                                                                                           • Quiescence. This stage ensures safety during rejoining.
C. Performance under Faults
                                                                                                                            Braft-DCR requires a node to skip two epochs, which in
   We evaluate C HIMERA-B against Braft-DR, Braft-TC,                                                                       practical deployments can range from tens of seconds to
Braft-RC, and Braft-DCR to measure rollback-resilient re-                                                                   hours or even days.
covery overhead under faults. Braft-RFT is omitted, as its                                                                 Throughput under Recovering Faults. We evaluate the
recovery is identical to Braft-DR. We deploy 21 nodes in a                                                                 system-level impact of recovery in terms of system throughput
LAN deployment and simulate failures by shutting down and                                                                  and available nodes, focusing on C HIMERA-B. Since Braft-
restarting 10 nodes while continuously issuing client requests.                                                            DR, Braft-DCR (without quiescence stage), and Braft-TC
Each node is equipped with 210 MB/s sequential read/write                                                                  exhibit recovery latencies comparable to C HIMERA-B, their
throughput and preloaded with 5 GB of log entries to emulate                                                               effects are effectively captured by C HIMERA-B and are omit-
a large-scale deployment.                                                                                                  ted here. For completeness, we defer the results for Braft-RC
Single-Node Recovery Latency. Table III presents the time                                                                  to Appendix B due to space constraints.
for a recovering node to rejoin the protocol. To enable fair                                                                  Fig. 4 shows the throughput variation and the number
comparison, we divide the recovery process into three stages:                                                              of available nodes (i.e., who participate in consensus) of
preparation, synchronization, and quiescence, as below.                                                                    C HIMERA-B during faulty nodes’ recovery. At the shutdown
• Preparation. This stage includes the steps required to ini-                                                              point (i.e., 2.2 seconds), the throughput briefly drops to zero
 tialize state synchronization. In Braft-TC and C HIMERA-B,                                                                as the leader handles connection failures. Once stabilized,
 the node reads the trusted counter, which takes about 10 ms.                                                              the throughput surpasses the pre-failure baseline. This occurs
 In Braft-RC, the recovering node performs reconfiguration                                                                 because, although the quorum size is unchanged, the leader
 through two consensus rounds, also taking around 10 ms.                                                                   no longer replicates log entries to the failed nodes. Thus, the
 The latency of RC depends on inter-node message delays.                                                                   leader’s available bandwidth is redistributed to the remaining
• Synchronization. This stage involves loading metadata and                                                                nodes, reducing contention and increasing throughput.
 log entries, with the latter dominating the cost. In Braft-RC,                                                               During recovery, faulty nodes reload their logs from disk,



                                                                                                                     12
                                                                            120
                                                                                                                    the Zab protocol [22]. Similarly, Brandenburger et al. [40]



Throughput (wTPS)
                                           Chimera-B




                                                        Throughput (kTPS)
                    40                                                                            Chimera-Z
                                           NoTEE                                                  NoTEE             integrate Intel SGX into Hyperledger Fabric [76] to secure
                                           SyncWrite                         80                   SyncWrite
                    30
                                           NoEncrypt                                              NoEncrypt
                                                                                                                    smart contract execution. CCF [8] uses enclaves to maintain a
                    20
                                                                                                                    distributed key-value store and runs Raft [11] to achieve low
                                                                             40
                                                                                                                    latency and tolerate a minority of Byzantine faults.
                    10
                                                                                                                       However, most systems do not consider TEEs’ rollback
                         1     2     4     10      20                             1    2     4     10     20
                              Number of faults                                        Number of faults              attacks. Engraft [4] first identified this threat and introduced
                                                                                                                    TIKS, i.e., software-based counters, to enforce trusted counters
                    (a) C HIMERA-B breakdown                                (b) C HIMERA-Z breakdown
                                                                                                                    for rollback protection. This approach corresponds to TC (as
                         Fig. 5: Overhead profiling of TEE-related execution.                                       introduced in §III), which represents the most general solution.
                                                                                                                    More details of the trusted counter are introduced shortly.
without affecting ongoing throughput. However, once recovery                                                        Later versions of CCF [9] address the issue by reconfiguration,
completes, these nodes lag behind because the leader continues                                                      referred to as RC. However, these solutions degrade either
to serve client requests. To rejoin replication, they must first                                                    performance or availability (§III).
catch up, consuming bandwidth and temporarily reducing                                                              TEE-Assisted BFT Consensus. Unlike confidential BFT con-
throughput until synchronization finishes.                                                                          sensus that ports whole consensus protocols into TEEs, TEE-
                                                                                                                    Assisted BFT consensus [77], [78], [79], [80], [73], [81],
D. Overhead Profiling                                                                                               [18] usually utilizes TEEs to provide some trusted functions,
   To better understand the overhead of TEE-related opera-                                                          such as the append-only log and monotonic counter, to mini-
tions, we compare variants of C HIMERA-B and C HIMERA-Z.                                                            mize Trusted Computing Base (TCB). These trusted functions
• NoTEE. It runs outside Intel TDX, serving as a baseline to                                                        can prevent Byzantine nodes from equivocating messages,
  measure TEE-related overhead.                                                                                     resulting in better scalability in terms of smaller quorum size
                                                                                                                    and shorter transaction latency. Recently, FlexiBFT [73] and
• SyncWrite. It uses synchronous disk writes instead of asyn-
                                                                                                                    Achilles [18] identified TEEs’ rollback issues in TEE-Assisted
 chronous persistence, isolating the cost of log persistence.
                                                                                                                    BFT consensus and proposed RFT and NVR as solutions,
• NoEncrypt. It disables memory encryption atop SyncWrite,                                                          respectively. However, RFT relaxes the tolerance, while NVR
 revealing the overhead of cryptographic operations.                                                                weakens the system’s tolerance. See more details in §III.
C HIMERA-B. Fig. 5a shows the throughput of C HIMERA-B
and its variants as the number of faults (f ) increases in a LAN                                                              XI. C ONCLUSION AND F UTURE W ORK
setting, with all other parameters identical to the fault-free                                                         We systematically analyze existing TEE rollback-resilient
scenarios. Compared to NoTEE variant, C HIMERA-B shows                                                              solutions, establishing a taxonomy to assess their suitability
an 8–10% slowdown, capturing the inherent cost of TEE                                                               for confidential BFT consensus. Building on the insights, we
execution. Relative to SyncWrite variant, persistence alone                                                         propose C HIMERA, a hybrid recovery framework that tailors
adds roughly 15–20% overhead, independent of encryption.                                                            recovery strategies for persistent state. We prove C HIMERA’s
Finally, the difference between the SyncWrite and NoEncrypt                                                         correctness and complement our proofs with formal verifi-
variants quantifies the cost of cryptographic sealing, resulting                                                    cation of its Braft design. We implement proof-of-concept
in an additional 10–15% performance degradation.                                                                    prototypes atop Raft and ZooKeeper using Intel TDX, and
C HIMERA-Z. Fig. 5b presents the maximum throughput of                                                              our extensive evaluation demonstrates that C HIMERA delivers
C HIMERA-Z and its variants. While C HIMERA-Z exhibits                                                              superior performance.
a similar trend, the performance gaps are smaller. This is                                                             Next, we discuss our approach’s limitations and potential
because the additional components and coordination overhead                                                         extensions. First, C HIMERA focuses on rollback-resilient re-
in ZooKeeper limit peak throughput, masking much of the                                                             covery and does not currently support dynamic reconfigura-
relative impact of TEE execution and persistence operations.                                                        tion. Integrating reconfiguration with recovery is challenging
                                                                                                                    because configuration updates are themselves stored in the
                                          X. R ELATED W ORK                                                         replicated log. During recovery, a node must know the cur-
   We discuss prior work on confidential computing, TEE-                                                            rent configuration to safely recover the log, while the latest
assisted BFT consensus, and trusted counters.                                                                       configuration may only exist inside the log being recovered.
Confidential BFT Service. Confidential BFT services have                                                            We leave the integration of reconfiguration into C HIMERA as
recently attracted significant attention from industry and                                                          future work. Second, although we focus on confidential BFT
academia, driven by the explosive growth of cloud and de-                                                           consensus, the core insight of C HIMERA, i.e., using protocol-
centralized applications. Notable industrial examples include                                                       level semantics to design tailored TEE recovery, extends to
SVR3 [3] that ports Raft into TEEs to secure private key                                                            other confidential computing systems, including confidential
management, and Azure that provides Confidential Ledger                                                             MapReduce frameworks [82], federated learning [83], and
service [75] atop CCF. Meanwhile, in academia, Secure-                                                              encrypted databases [36]. More broadly, separating critical
Keeper [2] is among the first to use TEEs to protect metadata                                                       metadata from bulk state and customizing recovery accord-
confidentiality in cloud settings with minimal changes to                                                           ingly may benefit a wider range of stateful TEE applications.



                                                                                                               13
                      E THICS C ONSIDERATIONS                                             [22] F. P. Junqueira, B. C. Reed, and M. Serafini, “Zab: High-performance
                                                                                               broadcast for primary-backup systems,” in Proc. of DSN, 2011.
   This work studies rollback-resilient recovery for confidential                         [23] “Intel Trust Domain Extensions,” https://www.intel.com/content/
BFT consensus systems. Our experiments are conducted on                                        dam/develop/external/us/en/documents/tdx-whitepaper-final9-17.pdf,
controlled cloud testbeds using synthetic workloads and do                                     retrieved September 2025.
                                                                                          [24] M. Hoekstra, R. Lal, P. Pappachan, V. Phegade, and J. Del Cuvillo,
not involve human subjects, personal data, or attacks on third-                                “Using innovative instructions to create trustworthy software solutions,”
party systems. The evaluated vulnerabilities are analyzed under                                in Proc. of HASP, 2013.
an abstract threat model, and the artifacts are intended solely                           [25] D. Kaplan, J. Powell, and T. Woller, “AMD SEV-SNP: Strengthening
                                                                                               VM isolation with integrity protection and more,” AMD, Tech. Rep.,
for research and reproducibility.                                                              2020. [Online]. Available: https://www.amd.com/system/files/TechDocs/
                                                                                               SEV-SNP-strengthening-vm-isolation-with-integrity-protection.pdf
                               R EFERENCES                                                [26] “Building a secure system using TrustZone technology,” https://
 [1] A. Jeffery, J. Maffre, H. Howard, and R. Mortier, “LSKV: A confidential                   documentation-service.arm.com/static/5f212796500e883ab8e74531, re-
     distributed datastore to protect critical data in the cloud,” arXiv preprint,             trieved September 2025.
     2024.                                                                                [27] R. Cheng, F. Zhang, J. Kos, W. He, N. Hynes, N. Johnson, A. Juels,
 [2] S. Brenner, C. Wulf, D. Goltzsche, N. Weichbrodt, M. Lorenz, C. Fetzer,                   A. Miller, and D. Song, “Ekiden: A platform for confidentiality-
     P. Pietzuch, and R. Kapitza, “SecureKeeper: Confidential ZooKeeper                        preserving, trustworthy, and performant smart contracts,” in Proc. of
     using Intel SGX,” in Proc. of Middleware, 2016.                                           EuroS&P, 2019.
 [3] G. Connell, V. Fang, R. Schmidt, E. Dauterman, and R. A. Popa, “Secret               [28] J. Lind, O. Naor, I. Eyal, F. Kelbert, E. G. Sirer, and P. Pietzuch,
     key recovery in a global-scale end-to-end encryption system,” in Proc.                    “Teechain: A secure payment network with asynchronous blockchain
     of OSDI, 2024.                                                                            access,” in Proc. of SOSP, 2019.
 [4] W. Wang, S. Deng, J. Niu, M. K. Reiter, and Y. Zhang, “Engraft:                      [29] X. Wen, Q. Feng, H. Lyu, J. Niu, Y. Zhang, and C. Feng, “TeeRollup:
     Enclave-guarded raft on Byzantine faulty nodes,” in Proc. of CCS, 2022.                   Efficient rollup design using heterogeneous TEE,” in IEEE Transactions
 [5] Y. Yan, C. Wei, X. Guo, X. Lu, X. Zheng, Q. Liu, C. Zhou, X. Song,                        on Computers, 2025.
     B. Zhao, H. Zhang et al., “Confidentiality support over financial grade              [30] X. Wen, Q. Feng, J. Niu, Y. Zhang, and C. Feng, “Mercury: Practical
     consortium blockchain,” in Proc. of ACM SIGMOD, 2020.                                     cross-chain exchange via trusted hardware,” IEEE Transactions on
 [6] “The oasis blockchain platform,” https://assets.website-files.com/                        Dependable and Secure Computing, vol. 23, no. 2, pp. 2949–2961, 2026.
     5f59478e350b91447863f593/628ba74a9aee37587419cf65 20200623%                          [31] S. Xie, D. Kang, H. Lyu, J. Niu, and M. Sadoghi, “Fides: Scalable
     20The%20Oasis%20Blockchain%20Platform.pdf, retrieved September                            censorship-resistant DAG consensus via trusted components.”
     2025.                                                                                [32] A. Oprea and M. K. Reiter, “Integrity checking in cryptographic file
 [7] M. Russinovich, E. Ashton, C. Avanessians, M. Castro, A. Chamayou,                        systems with constant trusted storage.” in Proc. of USENIX Security,
     S. Clebsch, M. Costa, C. Fournet, M. Kerner, S. Krishna et al., “CCF:                     2007.
     A framework for building confidential verifiable replicated services,”               [33] S. Ghemawat, H. Gobioff, and S.-T. Leung, “The google file system,”
     Microsoft Research and Microsoft Azure, Tech. Rep., 2019.                                 in Proc. of SOSP, 2003.
 [8] H. Howard, F. Alder, E. Ashton, A. Chamayou, S. Clebsch, M. Costa,                   [34] J. Hamilton, “On designing and deploying internet-scale services,” in
     A. Delignat-Lavaud, C. Fournet, A. Jeffery, M. Kerner, F. Kounelis,                       Proc. of LISA, 2007.
     M. A. Kuppe, J. Maffre, M. Russinovich, and C. M. Wintersteiger, “Con-               [35] M. D. Schroeder, A. D. Birrell, and R. M. Needham, “Experience with
     fidential consortium framework: Secure multiparty applications with                       grapevine: the growth of a distributed system,” ACM Trans. Comput.
     confidentiality, integrity, and high availability,” Proc. VLDB Endow.,                    Syst., vol. 2, no. 1, p. 3–23, 1984.
     vol. 17, no. 2, p. 225–240, 2023.                                                    [36] C. Priebe, K. Vaswani, and M. Costa, “EnclaveDB: A secure database
 [9] H. Howard, M. A. Kuppe, E. Ashton, A. Chamayou, and N. Crooks,                            using SGX,” in Proc. of S&P. IEEE, 2018.
     “Smart casual verification of the confidential consortium framework,”
                                                                                          [37] A. Wilde, T. N. Gruel, C. Soriente, and G. Karame, “The forking way:
     in Proc. of NSDI, 2025.
                                                                                               When tees meet consensus,” arXiv preprint, 2024.
[10] M. Gao, H. Dang, and E.-C. Chang, “TEEKAP: Self-expiring data
                                                                                          [38] “Signal       Secure      Value      Recovery,”     https://signal.org/blog/
     capsule using Trusted Execution Environment,” in Proc. of ACSAC,
                                                                                               secure-value-recovery, retrieved September 2025.
     2021.
[11] D. Ongaro and J. Ousterhout, “In search of an understandable consensus               [39] M.        Azure.,        “Confidential       consortium        framework,”
     algorithm,” in Proc. of ATC, 2014.                                                        https://www.microsoft.com/en-us/research/project/
[12] S. Matetic, M. Ahmed, K. Kostiainen, A. Dhar, D. Sommer, A. Gervais,                      confidential-consortium-framework/, retrieved September 2025.
     A. Juels, and S. Capkun, “ROTE: Rollback protection for trusted                      [40] M. Brandenburger, C. Cachin, R. Kapitza, and A. Sorniotti, “Trusted
     execution,” in Proc. of USENIX Security, 2017.                                            computing meets blockchain: Rollback attacks and a solution for Hy-
[13] J. Niu, W. Peng, X. Zhang, and Y. Zhang, “Narrator: Secure and practical                  perledger Fabric,” in Proc. of SRDS, 2019.
     state continuity for trusted execution in the cloud,” in Proc. of CCS,               [41] L. F. G. Sarmenta, M. van Dijk, C. W. O’Donnell, J. Rhodes, and
     2022.                                                                                     S. Devadas, “Virtual monotonic counters and count-limited objects using
[14] R. Strackx and F. Piessens, “Ariadne: A minimal approach to state                         a TPM without a trusted OS,” in Proc. of STC, 2006.
     continuity,” in Proc. of USENIX Security, 2016.                                      [42] “Trusted       time     and      monotonic      counters      with      intel
[15] B. Parno, J. R. Lorch, J. R. Douceur, J. Mickens, and J. M. McCune,                       software       guard      extensions     platform      services,”     https:
     “Memoir: Practical state continuity for protected modules,” in Proc. of                   //www.intel.com/content/www/us/en/content-details/671564/
     S&P, 2011.                                                                                trusted-time-and-/monotonic-counters-with-intel-software-/
[16] S. Gupta, S. Rahnama, S. Pandey, N. Crooks, and M. Sadoghi, “Dis-                         guard-extensions-platform-services.html, retrieved September 2025.
     secting BFT consensus: In trusted components we trust!” in Proc. of                  [43] W. Peng, X. Li, J. Niu, X. Zhang, and Y. Zhang, “Ensuring state
     EuroSys, 2023.                                                                            continuity for confidential computing: A blockchain-based approach,”
[17] B. Dinis, P. Druschel, and R. Rodrigues, “RR: A fault model for efficient                 IEEE Trans. Dependable Secure Comput., vol. 21, no. 6, pp. 5635–5649,
     TEE replication,” in Proc. of NDSS, 2023.                                                 2024.
[18] J. Niu, X. Wen, G. Wu, S. Liu, J. Yu, and Y. Zhang, “Achilles: Efficient             [44] E. Michael, D. R. K. Ports, N. K. Sharma, and A. Szekeres, “Providing
     TEE-assisted BFT consensus via rollback resilient recovery,” in Proc.                     stable storage for the diskless crash-recovery failure model,” University
     of EuroSys, 2025.                                                                         of Washington, Tech. Rep. UW-CSE-16-08-02, 2016.
[19] S. Angel, A. Basu, W. Cui, T. Jaeger, S. Lau, S. Setty, and S. Singana-              [45] B. Liskov and J. Cowling, “Viewstamped replication revisited,” MIT
     malla, “Nimble: Rollback protection for confidential cloud services,” in                  CSAIL, Tech. Rep. MIT-CSAIL-TR-2012-021, 2012.
     Proc. of OSDI, 2023.                                                                 [46] T. D. Chandra, R. Griesemer, and J. Redstone, “Paxos made live: An
[20] “Braft,” https://github.com/baidu/braft, retrieved September 2025.                        engineering perspective,” in Proc. of PODC, 2007.
[21] P. Hunt, M. Konar, F. P. Junqueira, and B. Reed, “ZooKeeper: Wait-free               [47] J. Kończak, N. Santos, T. Żurkowski, P. T. Wojciechowski, and
     coordination for internet-scale systems,” in Proc. of ATC, 2010.                          A. Schiper, “Jpaxos: State machine replication based on the paxos




                                                                                     14
     protocol,” EPFL, Tech. Rep. 167765, 2011. [Online]. Available:                      [73] F. Gai, A. Farahbakhsh, J. Niu, C. Feng, I. Beschastnikh, and H. Duan,
     https://infoscience.epfl.ch/handle/20.500.14299/69874                                    “Dissecting the performance of chained-BFT,” in Proc. of ICDCS, 2021.
[48] A. Bessani, M. Correia, T. Distler, R. Kapitza, P. Esteves-Verissimo, and           [74] S. Nakamoto, “Bitcoin: A peer-to-peer electronic cash system,” Working
     J. Yu, “Vivisecting the dissection: On the role of trusted components in                 Paper, 2008.
     BFT protocols,” arXiv preprint arXiv:2312.05714, 2023.                              [75] “Microsoft azure confidential ledger,” https://learn.microsoft.com/en-us/
[49] A. Clement, F. Junqueira, A. Kate, and R. Rodrigues, “On the (limited)                   azure/confidential-ledger/overview, retrieved September 2025.
     power of non-equivocation,” in Proc. of PODC, 2012.                                 [76] E. Androulaki, A. Barger, V. Bortnikov, C. Cachin, K. Christidis, A. D.
[50] D. Giantsidi, E. Giortamis, J. Pritzi, M. Bailleu, M. Kapritsos, and                     Caro, D. Enyeart, C. Ferris, G. Laventman, Y. Manevich, S. Mu-
     P. Bhatotia, “Recipe: Hardware-accelerated replication protocols,” arXiv                 ralidharan, C. Murthy, B. Nguyen, M. Sethi, G. Singh, K. Smith,
     preprint, 2025.                                                                          A. Sorniotti, C. Stathakopoulou, M. Vukolic, S. W. Cocco, and J. Yellick,
[51] J.-Y. Kim, J. Lee, Y. Koo, S. Park, and S.-M. Moon, “Ethanos: efficient                  “Hyperledger Fabric: A distributed operating system for permissioned
     bootstrapping for full nodes on account-based blockchain,” in Proc. of                   blockchains,” in Proc. of EuroSys, 2018.
     EuroSys, 2021.                                                                      [77] J. Behl, T. Distler, and R. Kapitza, “Hybrids on Steroids: SGX-based
[52] H. Feng, Y. Hu, Y. Kou, R. Li, J. Zhu, L. Wu, and Y. Zhou, “Sli-                         high performance BFT,” in Proc. of EuroSys, 2017.
     mArchive: A lightweight architecture for ethereum archive nodes,” in                [78] J. Liu, W. Li, G. O. Karame, and N. Asokan, “Scalable Byzantine
     Proc. of USENIX ATC, 2024.                                                               consensus via hardware-assisted secret sharing,” IEEE Transactions on
[53] M. van Dijk, J. Rhodes, L. F. G. Sarmenta, and S. Devadas, “Offline un-                  Computers, vol. 68, pp. 139–151, 2019.
     trusted storage with immediate detection of forking and replay attacks,”            [79] J. Zhang, J. Gao, K. Wang, Z. Wu, Y. Li, Z. Guan, and Z. Chen,
     in Proc. of STC, 2007.                                                                   “TBFT: Efficient Byzantine fault tolerance using Trusted Execution
[54] R. Strackx, B. Jacobs, and F. Piessens, “ICE: A passive, high-speed,                     Environment,” in Proc. of ICC, 2022.
     state-continuity scheme,” in Proc. of ACSAC, 2014.                                  [80] J. Decouchant, D. Kozhaya, V. Rahli, and J. Yu, “Damysus: Streamlined
[55] M. Schwarz, M. Lipp, D. Moghimi, J. Van Bulck, J. Stecklina,                             BFT consensus leveraging trusted components,” in Proc. of EuroSys,
     T. Prescher, and D. Gruss, “ZombieLoad: Cross-privilege-boundary data                    2022.
     sampling,” in Proc. of CCS, 2019.                                                   [81] ——, “Oneshot: View-adapting streamlined BFT protocols with Trusted
[56] S. Van Schaik, A. Milburn, S. Österlund, P. Frigo, G. Maisuradze,                       Execution Environments,” in Proc. of IPDPS, 2024.
     K. Razavi, H. Bos, and C. Giuffrida, “RIDL: Rogue in-flight data load,”             [82] F. Schuster, M. Costa, C. Fournet, C. Gkantsidis, M. Peinado, G. Mainar-
     in Proc. of S&P. IEEE, 2019.                                                             Ruiz, and M. Russinovich, “VC3: Trustworthy data analytics in the cloud
[57] G. Chen, S. Chen, Y. Xiao, Y. Zhang, Z. Lin, and T. H. Lai, “Sgxpectre:                  using sgx,” in Proc. of S&P, 2015.
     Stealing Intel secrets from SGX enclaves via speculative execution,” in             [83] D. L. Quoc and C. Fetzer, “SecFL: Confidential federated learning using
     Proc. of EuroS&P, 2019.                                                                  TEEs,” arXiv preprint, 2021.
[58] M.-W. Shih, S. Lee, T. Kim, and M. Peinado, “T-sgx: Eradicating                     [84] A. Martin, C. Lian, F. Gregor, R. Krahn, V. Schiavoni, P. Felber,
     controlled-channel attacks against enclave programs,” in Proc. of NDSS,                  and C. Fetzer, “ADAM-CS: Advanced asynchronous monotonic counter
     2017.                                                                                    service,” in Proc. of DSN, 2021.
[59] O. Oleksenko, B. Trach, R. Krahn, M. Silberstein, and C. Fetzer, “Varys:            [85] W. Wang, J. Niu, M. K. Reiter, and Y. Zhang, “Formally verifying a
     Protecting SGX enclaves from practical Side-Channel attacks,” in Proc.                   rollback-prevention protocol for TEEs,” in Proc. of FORTE, 2024.
     of USENIX ATC, 2018.                                                                [86] G. Kaptchuk, I. Miers, and M. Green, “Giving state to the stateless:
[60] C. Dwork, N. Lynch, and L. Stockmeyer, “Consensus in the presence                        Augmenting trustworthy computation with ledgers,” in Proc. of NDSS,
     of partial synchrony,” J. ACM, vol. 35, no. 2, pp. 288–323, 1988.                        2019.
[61] M. Castro and B. Liskov, “Practical Byzantine fault tolerance,” in Proc.            [87] G. A. Agha, ACTORS - a model of concurrent computation in distributed
     of OSDI, 1999.                                                                           systems, ser. MIT Press series in artificial intelligence. MIT Press, 1990.
[62] M. Yin, D. Malkhi, M. K. Reiter, G. G. Gueta, and I. Abraham,                       [88] S. Liu, P. C. Ölveczky, M. Zhang, Q. Wang, and J. Meseguer, “Automatic
     “HotStuff: BFT consensus with linearity and responsiveness,” in Proc.                    analysis of consistency properties of distributed transaction systems in
     of PODC, 2019.                                                                           maude,” in TACAS 2019, ser. LNCS, vol. 11428. Springer, 2019, pp.
[63] M. M. Jalalzai, J. Niu, C. Feng, and F. Gai, “Fast-HotStuff: A fast and                  40–57.
     robust BFT protocol for blockchains,” IEEE Trans. Dependable Secure                 [89] L. Ouyang, X. Sun, R. Tang, Y. Huang, M. Jivrajani, X. Ma, and T. Xu,
     Comput., vol. 21, no. 4, pp. 2478–2493, 2024.                                            “Multi-grained specifications for distributed system model checking and
[64] F. Gai, J. Niu, I. Beschastnikh, C. Feng, and S. Wang, “Scaling                          verification,” in Proceedings of the Twentieth European Conference on
     blockchain consensus via a robust shared mempool,” in Proc. of ICDE,                     Computer Systems, ser. EuroSys ’25. ACM, 2025, p. 379–395.
     2023.                                                                               [90] S. Liu, M. R. Rahman, S. Skeirik, I. Gupta, and J. Meseguer, “Formal
[65] M. Castro and B. Liskov, “Practical Byzantine fault tolerance,” in Proc.                 modeling and analysis of cassandra in maude,” in ICFEM 2014, ser.
     of OSDI, 1999.                                                                           LNCS, vol. 8829. Springer, 2014, pp. 332–347.
[66] M. Clavel, F. Durán, S. Eker, P. Lincoln, N. Martı́-Oliet, J. Meseguer, and        [91] J. Schvimer, A. J. J. Davis, and M. Hirschhorn, “extreme modelling in
     C. Talcott, All About Maude: A High-Performance Logical Framework:                       practice,” Proc. VLDB Endow., vol. 13, no. 9, pp. 1346–1358, 2020.
     How to Specify, Program and Verify Systems in Rewriting Logic.                      [92] S. Ghasemirad, S. Liu, C. Sprenger, L. Multazzu, and D. Basin, “Veriso:
     Springer, 2007.                                                                          Verifiable isolation guarantees for database transactions,” Proc. VLDB
[67] R. Bobba, J. Grov, I. Gupta, S. Liu, J. Meseguer, P. C. Ölveczky, and                   Endow., vol. 18, no. 5, p. 1362–1375, Aug. 2025.
     S. Skeirik, “Survivability: design, formal modeling, and validation of
     cloud storage systems using maude,” Assured cloud computing, pp. 10–                                                A PPENDIX
     48, 2018.
[68] S. Liu, H. Duan, L. Heimes, M. Bearzi, J. Vieli, D. Basin, and A. Perrig,           A. Supplementary Background
     “A formal framework for end-to-end dns resolution,” in SIGCOMM ’23.
     ACM, 2023, p. 932–949.                                                                 1) Trusted Counter and Extensions: Trusted counter is
[69] L. Chuat, M. Legner, D. A. Basin, D. Hausheer, S. Hitz, P. Müller, and             the most representative and common method for rollback
     A. Perrig, The Complete Guide to SCION - From Design Principles
     to Formal Verification, ser. Information Security and Cryptography.                 prevention. Generally, there are two types of counters:
     Springer, 2022.                                                                     hardware-based and software-based. The first includes SGX
[70] S. Liu, J. Meseguer, P. C. Ölveczky, M. Zhang, and D. A. Basin, “Bridg-            counter [42], TPM counter [84], and TPM NVRAM [14], [54],
     ing the semantic gap between qualitative and quantitative models of
     distributed systems,” Proc. ACM Program. Lang., vol. 6, no. OOPSLA2,                [15]. Hardware-based counters usually have poor performance,
     pp. 315–344, 2022.                                                                  i.e., long latency (e.g., tens of milliseconds) for read and write
[71] “Apache ZooKeeper,” https://zookeeper.apache.org/, retrieved Septem-                operations and limited write cycles [12], [13].
     ber 2025.
[72] “SGX data center attestation primitives,” https://github.com/intel/                    The second is virtual counters, which can be implemented
     SGXDataCenterAttestationPrimitives, retrieved September 2025.                       by a single-write multiple-read register [13], [43], [85] or an



                                                                                    15
append-only ledger [86], [19]. The former includes ROTE [12]             protocol can guarantee safety and liveness properties without
and Narrator [13], which adopt a two-phase broadcast proto-              rollback attacks.
col. The latter can be realized by blockchain [86] or CFT
consensus as adopted in Nimble [19]. However, using them                 B. Additional Evaluation
in confidential BFT consensus protocols introduces several
communication steps. In this paper, we use Narrator-Pro, i.e., a         Throughput vs. Latency. Fig. 6 illustrates the latency of
single-write multiple-read register [43], as the software-based          the C HIMERA-B and its counterparts with increasing through-
counters. Despite their high costs, we employ counters only              put until system saturation in both LAN and WAN deploy-
for infrequently updated metadata, thereby avoiding protocol             ments. With 10 faulty nodes, C HIMERA reaches maximum
overhead for log commitment.                                             throughputs of 135.8 kTPS in LAN and 22.6 kTPS in WAN,
Inc-store consistency dilemma. There are two fundamental                 closely matching or surpassing the best performance among all
operations when using a trusted counter: incrementing the                counterparts. Notably, in LAN, C HIMERA-B even outperforms
counter and sealing the state. Because these operations cannot           Braft-DR, as asynchronous disk writes reduce persistence bot-
be executed atomically, their sequence leads to two distinct             tlenecks. Overall, the results confirm that C HIMERA introduces
patterns.                                                                negligible overhead to log consensus and can even enhance
                                                                         performance in certain settings.
• Inc-then-store pattern [12], [13], [43], [4]. In this approach,
 the counter is incremented before the state is sealed. This
                                                                                        120                                                         500
 guarantees that no previously sealed state can be replayed,                            100
                                                                                                    Chimera-B                                                   Chimera-B
                                                                                                    Braft-DR                                                    Braft-DR




                                                                         Latency (ms)                                                Latency (ms)
                                                                                                                                                    400
 since the counter always moves forward. However, if a                                  80          Braft-TC                                                    Braft-TC
 crash occurs after the counter has been incremented but                                60
                                                                                                    Braft-RFT                                       300         Braft-RFT

 before the state is sealed, the counter and state become                               40                                                          200

 permanently inconsistent, making recovery impossible. From                             20
                                                                                                                                                    100
 the recovering node’s perspective, a mismatch between the                               0
                                                                                          0.0      2.5   5.0    7.5   10.0   12.5                         0      5     10   15      20
 counter value and the binding counter in the sealed state is                                       Throughput (wTPS)                                           Throughput (kTPS)
 indistinguishable from either a benign crash or a deliberate                                   (a) f = 10, LAN                                               (b) f = 10, WAN
 rollback attack.
                                                                         Fig. 6: Throughput vs. Latency of C HIMERA-B and its coun-
• Store-then-inc pattern [15], [14]. The state is sealed first,          terparts.
  and then the counter is incremented. This avoids unrecover-
  able crashes, since the sealed state always exists even if the
  counter increment fails. However, this introduces a rollback           Throughput of Braft-RC under Recovering Faults. Fig. 7
  window: an adversary can seal multiple different states under          shows the throughput variation during the recovery of faulty
  the same counter value and later replay an old one. After a            nodes in Braft-RC. The throughput behavior at the shutdown
  reboot, the node cannot determine which state with the same            point is the same as that observed in C HIMERA-B. These
  counter value is most recent, allowing its state to be rolled          effects follow the same reasoning as discussed earlier and are
  back.                                                                  not elaborated here.
                                                                            A key difference in Braft-RC is that recovery is realized
   2) Restricted Faults: C HIMERA assumes at most f nodes
                                                                         through reconfiguration. Newly added nodes do not reload logs
may fail concurrently. Without this assumption, the system
                                                                         from disk but instead start synchronizing directly from the
may gradually lose liveness as no recovering leader can
                                                                         leader. During this synchronization phase, replication traffic
recover its log from collecting f +1 replies. Yet this limitation
                                                                         competes with ongoing client requests, which slows down
is not unique to our work. Diskless CFT protocols without
                                                                         replication processing and temporarily reduces throughput
stable storage, such as VR [45] and variants of Paxos [46],
                                                                         until the synchronization completes.
[47], also share this constraint (no more than f crashed nodes
concurrently). Moreover, all BFT protocols have a security
threshold f . An adversary compromising more than f nodes                                                             Shutdown        Restart
would disrupt system correctness. This also holds true for                                                              Recovery Completed



                                                                     Throughput (kTPS)                                                                                              Available Node #
C HIMERA.                                                                                                                                                                        20
   3) Customized CFT Consensus Protocols: Except for roll-
                                                                                        100
back issues, Wang et al. [4] also identify several safety and
liveness violations of directly porting the Raft protocol into                            50                                                                                     15
TEEs. To address these violations, Wang et al. propose several
countermeasures, including file encryption, network encryp-                                   00         25       50         75     100 125 150 175                              10
tion and authentication, and malicious leader detection. In this                                                         Time Window (s)
paper, we focus on rollback-resilient recovery and so assume
a customized CFT protocol with the above countermeasures                                        Fig. 7: The fault recovery process of Braft-RC.
running within TEEs. In other words, the customized CFT



                                                                    16
            TABLE IV: Model checking results.                                properties:
 Property   Metric    3 Nodes   3 Nodes w/ 2 reboots     5 Nodes
                                                                             • P1. Election Safety: at most one leader can be elected in
                                                                              any epoch.
            #States      /         2,313,191 [45]      2,200,414[26]
 P1                                                                          • P2. Leader Completeness: once a log entry is committed
             Time      557s            1,174s              4,209s
                                                                              by a leader, any subsequent leader contains this entry.
            #States      /         2,691,520 [45]      2,652,353 [26]
 P2
             Time     1,294s           2,883s              9,814s            • P3. Recovery Liveness: a node undergoing reboot eventually
            #States      /         2,691,520 [45]      2,652,353 [26]
                                                                              completes its recovery procedure.
 P3
             Time     1,145s           3,274s             10,213s            • P4. Protocol Liveness: every client request is eventually
            #States      /         2,691,520 [45]      2,652,353 [26]         committed as a log entry.
 P4
             Time     1,146s           3,369s             10,469s               Table IV shows the model checking results for three cases:
                                                                             three nodes with one reboot, three nodes with two reboots,
                                                                             and five nodes. Note that the state space grows rapidly with
C. Formal Modeling and Analysis                                              additional nodes, making exhaustive verification infeasible
   Our formal specification of C HIMERA-B consists of approx-                within a reasonable time, a well-recognized challenge in the
imately 910 LoC in Maude.2 Our modeling follows Agha’s                       formal verification of distributed protocols [88], [89], [90],
actors paradigm [87]. Specifically, nodes are modeled as                     [91], [92]. To address this, we utilize Maude’s bounded search,
actors, and their communication is captured through message                  which explores the state space with depth limits (shown in
passing. Upon receiving a message, a node may update its                     [square brackets] in Table IV). This enables model checking
local state and possibly generate new messages. The overall                  up to a specified bound. Within the explored bounds, Maude
system evolves through such message-triggered transitions.                   reports no counterexamples to the four properties.
   We verify C HIMERA-B under TEE rollbacks using linear-                     2 The Maude specification is available at https://github.com/Artifacts2026/
temporal-logic (LTL) model checking, focusing on four key                    CHIMERA/MaudeSpec.




                                                                        17
