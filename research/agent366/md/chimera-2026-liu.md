C HIMERA: Protocol-Aware Recovery for
Confidential BFT Consensus
Tong Liu∗ , Xiaoqing Wen† , Ziwei Zhou‡ , Si Liu§ , Jianyu Niu¶ , Cong Wang¶ , Yinqian Zhang∗
∗ Southern University of Science and Technology, † University of British Columbia, ‡ East China Normal University,

arXiv:2606.09101v2 [cs.DC] 11 Jun 2026

§ Texas A&M University, ¶ City University of Hong Kong

Abstract—Trusted Execution Environments (TEEs) have enabled confidential Byzantine Fault-Tolerant (BFT) consensus
systems with confidentiality and improved scalability. However, TEEs do not provide state continuity: during recovery,
a compromised host can roll back a crashed enclave to a
stale persistent state, significantly threatening both safety and
availability. Existing defenses face a fundamental tradeoff: they
either impose substantial overhead on critical consensus paths,
reducing throughput and increasing latency, or incur prolonged
recovery delays, hurting availability.
We present the first systematic taxonomy of rollback-resilient
recovery for confidential BFT consensus, distilling prior approaches into four categories. We further expose their inherent limitations. Guided by this detailed analysis, we design
C HIMERA, a protocol-aware recovery framework that breaks this
tradeoff. Our key insight is that rollback protection in consensus
systems should not be uniform. Different types of persistent
states differ fundamentally in their state distributions, update
behaviors, and representations. C HIMERA separates persistent
state into metadata and logs according to these protocol-level
properties and applies distinct recovery mechanisms to each type.
We formally model C HIMERA in Maude and verify its safety and
liveness properties. We implement it on Braft and ZooKeeper
using Intel TDX, and evaluate it in both LAN and WAN settings.
Results show that C HIMERA achieves higher throughput, lower
recovery latency, and better availability than state-of-the-art
rollback-resilient baselines.

I. I NTRODUCTION
Confidential Byzantine Fault Tolerant (BFT) consensus,
which uses Trusted Execution Environments (TEEs) to improve system scalability and confidentiality, has recently
gained significant traction. By leveraging TEEs, confidential
BFT consensus enables a set of nodes to agree on an evergrowing, consistent log of transactions while preserving client
confidentiality in the presence of Byzantine behavior (i.e., arbitrary protocol deviation) among nodes. Due to the promising
consistency, confidentiality, and fault tolerance properties, it
has been used in many decentralized or cloud services, including datastores [1], [2], [3], blockchains [4], [5], [6], cloud
computing [7], [8], [9], and self-expiring data objects [10].
More specifically, confidential BFT consensus offers two
key advantages over classic BFT protocols. First, due to the integrity guarantees of TEEs, parties running inside TEEs cannot
equivocate their messages (e.g., votes). Therefore, confidential
BFT consensus achieves Byzantine fault tolerance by running
Crash Fault Tolerant (CFT) protocols, such as Raft [11],
inside TEEs. This design improves scalability by allowing a
smaller node size and less computational overhead. Second,

due to the confidentiality guarantees of TEEs, the secrecy
of computations (e.g., blockchain transactions) is preserved
against untrusted nodes, enabling wider applicability across
sensitive use cases.
Nonetheless, a well-known Achilles’ heel of confidential
BFT lies in the lack of state continuity in TEEs. State continuity allows a TEE to preserve a consistent, tamper-resistant
state across crashes and restarts. In fault-tolerant protocols, a
persisted state enables a crashed node to recover and rejoin
the system. However, an adversary that controls the host can
provide stale state to the TEE during recovery, causing a
rollback attack [4], [8].
Recent years have seen substantial efforts on achieving rollback resilience in such settings [12], [13], [14], [15], [4], [16],
[17], [9], [18], [19]. We categorize these approaches into four
categories: Trusted Counter (TC), Diskless Crash Recovery
(DCR), Rollback Fault Tolerance (RFT), and Reconfiguration
(RC). Yet each category exemplifies a fundamental tradeoff between performance and availability: they either impose
high overhead on critical consensus paths or incur prolonged
recovery delays. (See §III for a detailed discussion.)
In this paper, we aim to answer the question: How can
we design rollback-resilient recovery for confidential BFT
consensus that maintains high performance during normal
operation and minimizes downtime during recovery?
Our key insight is that the above tradeoff stems from a common design choice: existing defenses protect persistent state
uniformly—applying the same protection to all state—even
though different types of state differ in how they are updated,
stored, and recovered by the protocol. By classifying persistent
states according to their protocol-level characteristics, we
can design recovery mechanisms tailored to each type. For
leader-based consensus, these states can be divided into two
categories: metadata and logs. Metadata is a set of node-local
control states that records each node’s view of the consensus
process, while logs are replicated histories, with replications
maintained according to consensus rules.
Guided by this distinction, we propose C HIMERA, a
protocol-aware recovery framework for confidential BFT consensus that customizes recovery for metadata and logs. For
metadata, C HIMERA uses a TC-based mechanism to securely
and timely recover metadata from local storage. For logs,
C HIMERA uses protocol-guided cluster recovery that leverages
consensus replication rules to rebuild a safe log from other
replicas. This design balances performance and availability: it

confines TC overhead to infrequent metadata updates, avoiding
TC updates on the high-frequency log replication path; it
also reduces recovery downtime by avoiding epoch skips and
allowing crashed leaders to resume without waiting for new
leader election.
Applying such protocol-aware recovery, however, is nontrivial. First, for metadata, unexpected crashes may occur between
the counter increment and metadata sealing. Without proper
handling, a recovering node may fail to restore its metadata if
the counter and persistent states are mismatched. Achilles [18]
proposes skipping several epochs to avoid double voting, but
this can render the node temporarily unavailable. We analyze
the update process of metadata and design a binding update
solution to address this issue (§VI-B). Second, in log recovery,
a crashed leader may leave an entry replicated to only a subset
of nodes, failing to reach quorum. To support leader recovery,
the recovered leader must confirm its leadership and identify
any unfinished log entries; otherwise, it may re-propose entries at the same indices, resulting in conflicting entries at
the same log position (§VI-C). Third, log synchronization
from the network is time-consuming. To mitigate this cost,
C HIMERA uses metadata to identify the log range that needs
to be synchronized. This approach reduces unnecessary log
transmission during recovery and enables recovering nodes to
rejoin the protocol with minimal delay (§VI-C).
We prototype C HIMERA by extending two opensource industrial CFT consensus platforms: Braft [20],
a high-performance implementation of Raft [11], and
ZooKeeper [21], a widely adopted coordination service built
on the Zab [22] consensus protocol. We select Raft and
Zab because they are the most widely used CFT consensus
protocols for building confidential BFT consensus [3], [7],
[8], [4], [1], [2]. Both prototypes are implemented on top of
Virtual Machine (VM)-based TEE, i.e., Intel TDX [23]. We
formally model C HIMERA in Maude and verify its safety
and liveness properties. We conduct extensive experiments on
a public cloud platform to evaluate and compare C HIMERA
with four counterparts, particularly their performance during
recovery over LAN and WAN.
Contributions. Our main contributions are as follows:
• We provide the first taxonomy of rollback-resilient solutions
for TEEs in distributed systems, and organize them into four
categories. We further analyze why applying uniform rollback
protection is insufficient for persistent states with differing
protocol-level characteristics in these systems.

II. BACKGROUND
A. TEEs and Their State Continuity
Trusted Execution Environments (TEEs) are hardwaresupported execution environments that protect sensitive code
and data. They achieve this protection through mechanisms
such as memory encryption, hardware-enforced isolation, and
remote attestation. Representative platforms include enclavebased TEEs such as Intel SGX [24], VM-based TEEs such as
Intel TDX [23] and AMD SEV [25], and other hardware isolation architectures such as ARM TrustZone [26]. Compared
with enclave-based TEEs, VM-based TEEs allow existing
applications to benefit from TEE protection with minimal code
changes and low additional overhead. We therefore build our
implementation on VM-based TEEs.
TEEs have been adopted in a wide range of stateful applications, including blockchains [27], [28], [5], [29], [30],
[31], trusted storage [32], [1], [2], [3], authentication rate
limiting [14], and cloud computing services [7], [8], [9], [19],
[13]. However, despite their popularity, TEEs still lack state
continuity guarantees: rollback attacks can revert a TEE to a
prior state, undermining the security of applications built atop
them.
State Continuity. State continuity of TEEs mandates that
when a stateful TEE application resumes execution from an
interruption (e.g., reboots or system crashes), it must resume
in the same state as before [15]. This property is critical
for applying TEEs in distributed systems, since nodes’ ability
to recover from crashes directly determines system reliability. [33], [34], [35]. Existing TEE platforms provide sealing
functionality, allowing TEE applications to encrypt and store
their state to untrusted persistent storage. The sealing key can
be configured to be accessible to all enclaves with the same
MRENCLAVE or MRSIGNER [13].
Rollback Attacks. The sealing functionality can ensure the
integrity of retrieved data, but does not provide freshness
guarantees [36]. An adversary controlling the OS can roll back
a TEE application to a previous state by providing it with stale
data, resulting in a rollback attack [37]. These attacks break
state continuity and undermine the security guarantees of TEEs
for various stateful applications, particularly confidential BFT
consensus systems. For instance, if a node is rolled back after
casting a vote, it may re-enter a previous state and cast the vote
again. Such repeated voting can lead to equivocation, violating
the safety properties of the protocol.
B. Confidential BFT Consensus

• We propose C HIMERA, a protocol-aware recovery framework for confidential BFT consensus. It tailors recovery
mechanisms to metadata and logs according to their characteristics. To ensure its correctness (i.e., safety and liveness),
we perform a thorough security analysis and conduct a formal
modeling and verification of C HIMERA.

Confidential BFT consensus ports CFT consensus protocols into TEEs to provide BFT services. Examples include
SVR3 [3] adopted by Signal [38], CCF [8] deployed on the
Azure cloud platform [39], Engraft [4], Hyperledger Fabric [40], and SecureKeeper [2]. Among them, SVR3, CCF,
and Engraft are built on Raft [11], whereas SecureKeeper
uses Zab [22]. Raft and Zab are well-established leader-based
CFT consensus protocols. Both provide similar protocol-level
semantics, which are leveraged in our design.

• We implement C HIMERA atop Braft and ZooKeeper with
Intel TDX, and integrate optimizations to reduce recovery
communication. Our extensive evaluation demonstrates that
C HIMERA delivers superior performance.

2

TABLE I: Rollback-resilient solutions.

• Epoch. The leader-based consensus protocol partitions time
into logical units called epochs (referred to as terms in Raft).
For each epoch, at most one node, called the leader, is elected
and agreed upon by a quorum. When the leadership needs to
change, the epoch is incremented to reflect the transition.
• Leader Election. When nodes detect that the leader is unavailable, they initiate a leader election to preserve liveness.
Specifically, each node casts at most one vote per epoch
and persists this voting behavior to prevent double voting.
A candidate node becomes a leader when it receives votes
from the majority of nodes. A non-leader node is called a
follower. In this paper, we refer to the persisted epoch and
voting behavior as metadata.

Approach

Recovery
Pattern

Recovery
Completeness

Performance
Overhead

Availability
Degradation

TC
DCR
RFT
RC
C HIMERA

Local
Distributed
Local
Distributed
Hybrid

High
Medium
High∗
Low
High

High
Low
High
Low
Low

Low
High
Low
High
Low

∗ RFT enables a node to recover its state, but cannot prevent state rollback.

increment but before sealing makes recovery impossible. In
contrast, the store-then-inc pattern [15], [14] favors availability over safety, since a crash during the interval can result in
rollback. Further discussion is provided in Appendix A1.

• Log. The log is a sequence of client transactions maintained
by nodes, where each entry contains one or a batch of
transactions. Each entry is uniquely identified by an epoch
number and a monotonically increasing index. Nodes commit
entries in a contiguous sequence sorted by index without
gaps. Unlike metadata, logs are replicated across nodes
following consensus rules.

• Detection-only recovery. TC can determine the freshness of
a persisted state by comparing counter values, but it cannot
identify which state is safe for recovery; in other words, it
provides only rollback detection, not full recovery.
Diskless Crash Recovery (DCR). DCR enables a node to
recover its state from the in-memory state of other nodes in the
cluster [44], [45], [46], [47]. Specifically, DCR is effective at
recovering redundant data, such as logs, because the protocol’s
replication rules allow recovery to locate a node with a
sufficiently safe copy. It relies only on naive log replication
and introduces no additional overhead on the consensus path.
However, directly applying DCR to confidential BFT recovery
exposes the following shortcomings.
• Epoch skipping for safety. DCR cannot recover node-local
metadata such as epochs and voting behavior. To prevent
rollback-induced double voting, a recovering node must first
infer a safe upper-bound epoch based on the protocol’s
metadata update rules. The node then recovers directly into
that epoch, skipping earlier epochs. As a result, it may remain
unavailable until the cluster reaches the same epoch.

III. R ECOVERY TAXONOMY
We present the first systematic analysis of TEE recovery
mechanisms in distributed systems, categorizing them into
four groups, as shown in Table I. This taxonomy provides a
direct comparison of the approaches, as we present next. More
importantly, it shows why rollback recovery for confidential
BFT must be protocol-aware rather than uniform.
Trusted Counter (TC). TC is a tamper-resistant counter
whose value, once incremented, cannot be reverted to a
previous value [41]. It can be implemented in hardware (e.g.,
TPM [41] or SGX monotonic counters [42]) or in software
(e.g., distributed trusted KV [12], [13]) (Appendix A1). It is
the most popular and general approach for addressing TEEs’
rollback attacks [12], [13], [43], [14], [15], [4].
This approach performs two operations for each state update: (1) incrementing the counter value, and (2) sealing the
updated state together with the counter value in persistent
storage. During recovery, a node retrieves the sealed states
and validates its freshness by comparing the stored counter
value against the current one. Although this design is relatively
simple and allows the system to recover the exact pre-crash
state, it suffers from the following limitations:
• High read/write overhead. Each state update requires a
counter write, introducing substantial overhead. Updating
a hardware-backed counter, such as a TPM counter, takes
roughly 97 ms, while reading it for state verification takes
about 35 ms [14]. Software-based counters typically incur
one or two extra communication rounds [12], [13], [43]. Such
overhead makes TC unsuitable for a frequently updated state.

• Protocol-specific recovery rules. The safe epoch inferred
by DCR depends on the protocol’s metadata update rules.
Different protocols may require skipping varying numbers of
epochs, and those without bounded epoch growth may fail to
provide a usable safe epoch. Consequently, both the recovery
rules and the unavailability period are protocol-specific.
• No leader recovery. DCR requires an active leader. If the
leader crashes, recovering nodes can only resume as followers
and must wait for a new leader to be elected. This dependency
prolongs the period of unavailability after leader failures.
Rollback Fault Tolerance (RFT). RFT addresses rollback
attacks by adjusting read/write quorum sizes according to the
number of potentially rolled-back nodes. RR [17] modifies
read/write quorum sizes to guarantee intersection with up-todate nodes, whereas FlexiBFT [16] increases the overall node
size to 3f + 1, aligning with classical BFT requirements. In
RFT, a recovering node can quickly retrieve its state—which
is not protected against rollbacks—and rejoin the protocol
without affecting system availability. This design choice brings
the following limitations:

• Inc-store consistency dilemma. Since counter increment and
state sealing cannot be performed atomically, two usage
patterns have emerged. The inc-then-store pattern [12], [13],
[43], [4] increments the counter before sealing the state,
preserving safety but risking availability: a crash after the

3

• Scalability limitation. Increasing node and quorum sizes
undermines log commit performance, particularly in large
deployments. Evaluation results show that when f is small,
the performance gap between the 2f +1 and 3f +1 configurations atop Braft (i.e., Braft-DR and Braft-RFT, respectively)
is around 10%. However, this gap grows with larger f = 20,
reaching roughly 20% (§IX-B). Increasing the node size also
conflicts with prior TEE-aided designs that aim for smaller
node sizes [48], [49].
Reconfiguration (RC). RC enables the system to dynamically
modify its node set by adding or removing nodes. It is a more
general approach than simple recovery, as a recovering node
can rejoin the system as a newly added participant without
having to recover any prior state. Notable confidential BFT
protocols, CCF [8] and Recipe [50] adopt this design. Since
RC does not affect the log replication, no extra overhead is
introduced. Its drawbacks appear during recovery:
• High-cost membership changes. RC typically requires complex recovery designs. To ensure safety, most protocols
require nodes to reach consensus on the order of membership
changes, so that all nodes apply them consistently. This
coordination can delay concurrent log commits. For example,
RC in Raft requires two rounds of consensus to perform a
membership change safely.

and confidentiality cannot be breached (introduced shortly).
The other nodes that faithfully follow the protocol and remain operational (i.e., participating in consensus) are correct
nodes. The rationale behind this assumption is provided in
Appendix A2.
The adversary gains full control over the OS of the corrupted
node: it can manipulate network messages between TEEs
and arbitrarily start, stop, and invoke TEEs. Moreover, the
adversary can provide the TEE with stale data to rollback
the state [12], [14], [15], [53], [54], [4]. We do not consider
cloning attacks [12], [13] or micro-architectural side-channel
attacks [55], [56], [57], as they are orthogonal to this work.
Cloning attacks can be mitigated using TPM PCR [14],
while side-channel attacks can be addressed via software-level
countermeasures, particularly in cryptographic libraries such
as OpenSSL and Intel SGX SSL [58], [59].
Network Model. We adopt the partially synchronous network
model [60], which is commonly used in consensus [61], [62],
[63], [64]. In this model, there is an established bound ∆
and an undefined Global Stabilization Time (GST). After the
GST point, the delivery of any message transmitted between
two correct nodes within the ∆ limit is guaranteed. That is, the
system behaves synchronously following the GST. Liveness
is guaranteed after GST.

• Expensive synchronization. A rejoining node must synchronize the entire application state from scratch [8]. For
applications with large state, such as blockchains, this process
can involve transferring hundreds of gigabytes of data and
may take several hours to days [51], [52].
Summary. Existing recovery approaches in confidential BFT
consensus follow a one-size-fits-all design: each approach
tries to protect or recover all persistent states with the same
mechanism. As a result, the system pays the worst-case cost
for the least suitable state type, rather than exploiting protocollevel knowledge and the characteristics of metadata and logs.
This observation motivates a protocol-aware recovery design
that customizes and strengthens recovery for each state type.

B. Problem Statement
In confidential BFT consensus, each node runs a customized
leader-based CFT protocol (Appendix A3) inside its TEE to
commit and execute client transactions. The protocol proceeds
in a sequence of epochs, each representing a logical leader
term. Ideally (i.e., without rollback issues), one node is elected
as the leader in each epoch, and a majority of nodes agree
on this choice (i.e., election safety). The leader batches client
transactions, proposes them as log entries, and replicates each
entry to the logs of all nodes pi within the TEE. Each log
entry is tagged with an epoch and log index (ep, idx). Here,
ep denotes the epoch in which the entry is proposed, and
idx denotes its position in the log. The leader appends each
entry to its local log and persists it. It then replicates the
entry to all followers. Each follower appends and persists
the entry before sending an acknowledgment. Upon receiving
acknowledgments from a majority of followers, the leader
marks the entry as committed.
As in prior work [65], confidential BFT consensus provides
two fundamental guarantees: safety and liveness. Safety requires the service to be linearizable: no two correct nodes
commit different entries at the same log index idx. Liveness
ensures that every transaction submitted by a client is eventually committed.
Recovery under TEE Rollbacks. The integrity guarantees
of TEEs allow confidential BFT consensus to maintain safety
and liveness during normal execution [65]. However, recovery
in the presence of TEE rollbacks introduces additional challenges. In leader-based consensus protocols such as Raft [11],
safety and liveness can be refined into three critical conditions

IV. P ROBLEM S TATEMENT
A. System Model
Following the model of prior confidential BFT consensus [4], [8], we consider a distributed system maintained by
n = 2f +1 nodes {p1 , p2 , ... , pn }, each equipped with a TEE.
Confidential BFT consensus runs entirely inside each node’s
TEE. We assume a Public Key Infrastructure (PKI): each node
pi has a public/private key pair, denoted by (pki , ski ), in which
the private key is accessible only within the node’s TEE. A
message m signed with ski is denoted by mσi . We assume
that a finite set of clients sends transactions to nodes’ TEEs
for confidential BFT service over encrypted and authenticated
channels (e.g., TLS).
Threat model. We assume an adversary A that can corrupt
at most f nodes at any time and any number of clients.
Following prior study [4], [8], corrupted nodes are Byzantine,
i.e., behaving arbitrarily, with the exception that TEE integrity

4

during recovery. Specifically, to preserve safety, the recovery
procedure must satisfy the following two properties:

Consensus Node
Trusted Region

Definition 1 (Election Safety). For any epoch ep and any two
distinct nodes pi and pj , it is impossible for both pi and pj
to be elected leaders in ep.
Definition 2 (Leader Completeness). For any log entry en
committed at index idx in epoch ep, every leader elected in a
later epoch ep′ > ep must contain en at index idx in its log.

Log

Attack Surface

Disk

Metadata

Update Dataflow

Trusted
Counter

Consensus
Engine

Recovery Dataflow

Peers

Fig. 1: Architecture of C HIMERA.

Election safety guarantees a unique leader for each epoch,
while leader completeness ensures that all committed log entries are preserved across subsequent leaders. During recovery,
the system must maintain correct commitment progress and
restore all committed entries.
For liveness, the recovery procedure must ensure that an
uncorrupted recovering node can eventually resume participation, as formalized below.

metadata preserves election safety, but entering a higher epoch
may temporarily reduce availability. This trade-off makes
metadata well-suited for TC-based local freshness protection,
which enables precise, node-local recovery. Second, metadata
is scalar and can be stored in a register-like form. In particular,
the epoch increases monotonically during leader transitions,
which aligns naturally with the TC design. Moreover, the low
update frequency of metadata ensures that introducing TC
incurs minimal additional overhead.
Log. Each node maintains a log that consists of ordered
entries. An entry is considered committed once it has been
replicated to a majority of nodes. Leader completeness ensures
that all committed entries are preserved across subsequent
leaders. To recover safely, we only need to make sure that
the commit process is correct and that committed entries are
durable. Each node that replicates an entry must ensure the
entry’s durability before it is considered committed. By doing
so, all committed entries remain available for future recovery
and are preserved by subsequent leaders.
Logs differ from metadata in distribution and form. Unlike
metadata, which is node-local, logs are replicated across
multiple nodes in a structured way. Therefore, we can leverage
the replicas to reconstruct a safe log during recovery. Moreover, logs are large and stored on disk, making it infeasible
to maintain a direct mapping to a trusted counter. Version
numbers can be used to detect inconsistencies, but they do
not enable full recovery of log contents.

Definition 3 (Recovery Liveness). For any uncorrupted node
pi , if pi starts recovery and remains uncorrupted, then pi
eventually completes recovery and resumes participation in
the protocol.
V. C HIMERA OVERVIEW
We present an overview of C HIMERA, a protocol-aware recovery framework for confidential BFT consensus. C HIMERA
first characterizes the persistent state maintained by leaderbased consensus protocols (§V-A). It then applies a recovery
strategy specialized for each state type (§V-B). This framework can achieve rollback-resilient recovery with both high
performance and high availability, while preserving election
safety, leader completeness, and recovery liveness.
A. Characterizing System State
In a leader-based CFT consensus, three types of system state
require external persistence: metadata, logs, and snapshots.
Snapshots are primarily used to accelerate log synchronization;
therefore, their recovery is not essential for preserving safety
guarantees and does not need rollback-resilient mechanisms. In
this work, we focus on the two safety-critical states—metadata
and logs—as summarized in Table II.
Metadata. Metadata denotes a set of safety-critical control
variables that govern leader election and epoch transitions.
Metadata is updated infrequently and generally changes during
leader transitions. It typically includes the current epoch and
the node’s voting record within that epoch. For example, in
Raft, metadata consists of currentTerm (the epoch identifier)
and votedFor (the candidate voted for in that epoch). The epoch
identifier allows nodes to recognize larger epochs and reject
stale leadership attempts. The voting record prevents double
voting, ensuring election safety.
Two properties make metadata distinct for recovery. First,
metadata is node-local: it records a node’s own view of consensus. As a result, other replicas cannot reliably reconstruct
the recovering node’s exact epoch or vote. Therefore, recovery
must balance safety and availability: ensuring freshness of

B. Protocol-Aware State Recovery
We leverage protocol-level knowledge, i.e., the inherent
characteristics of logs and metadata, to carefully tailor recovery strategies, as illustrated in Fig. 1.
Metadata Recovery. C HIMERA protects metadata using a
trusted counter (TC), leveraging the counter’s node-local nature to enable precise recovery. The TC enables a recovering
node to verify the freshness of sealed metadata locally, ensuring election safety without coordinating with other replicas.
This approach allows precise local recovery while avoiding unnecessary loss of availability, capturing the trade-off between
freshness guarantees and state continuity.
Raw TC alone, however, is insufficient: counter increments
and metadata sealing are not atomic, so a mismatch may
indicate either a rollback or a crash during the update window.
To address this, C HIMERA proposes a binding-update design,
motivated by the similarity between metadata and the TC. The
binding-update design ties the epoch to the counter value to

5

TABLE II: The characteristics of metadata, log, and snapshot.
State Type

Safety
Critical

Recovery
Requirement

System
Redundancy

Update
Frequency

Metadata
Log
Snapshot

✓
✓
✗

Precise
Loose
Loose

✗
✓
✗

Low
High
Low

2) TC interfaces. We use two TC operations:
• ctr ← IncTC(k): Atomically increment the counter identified by key k by one and return the new value ctr.
• ctr ← ReadTC(k): Return the current value ctr of the
counter identified by key k without modifying it.
Each node maintains two trusted counters for different
recovery purposes:
• T Cmd records metadata updates. Its value is bound to
the epoch in sealed metadata, enabling rollback detection
and safe recovery when the sealed states and the counter
mismatch.

ensure consistent metadata recovery. When the counter and
epoch match, the node can safely restore the sealed metadata.
Otherwise, the counter serves as a safe epoch anchor, enabling
recovery without conservative epoch skipping.
Log Recovery. For logs, C HIMERA relies on protocol-guided
cluster recovery rather than per-entry TC protection. Consensus replication ensures that committed entries are preserved
on a quorum of replicas. During recovery, a node can query a
quorum of replicas to reconstruct a log that safely includes
all committed entries. To guarantee safety, each node that
replicates an entry must ensure the entry’s durability before
it is considered committed.
C HIMERA also tailors recovery to the node’s role to improve
availability. A recovering follower can use metadata to avoid
unnecessary log transfer during catch-up. A recovering leader,
in contrast, can resume service after reconstructing a safe
log without waiting for a new leader election. By leveraging
both protocol-level guarantees and node role information,
C HIMERA enables safe and efficient log recovery while preserving availability.

• T Crole records a node’s role within the current epoch. It is
initialized to 0 and incremented whenever the node changes
its role between follower and leader. Thus, when the counter
value is odd, the node is a leader; when it is even, the node
is a follower.
Initialization. Each node boots from a hard-coded genesis configuration and initializes with default metadata (i.e.,
ep = 1) and an empty log. The configuration information,
including key pairs, is stored on local disks in an encrypted
and authenticated form.
In this work, we consider a static configuration: a recovering
node can retrieve the configuration to obtain its own key
pairs and the public keys of other nodes for communication.
Dynamic reconfiguration is discussed in §XI.
The trusted counters are initialized to match the starting
system state: T Cmd is set to 1 to align with the initial epoch,
while T Crole is set to 0 to represent the initial follower role.
These initializations ensure that metadata freshness and node
role tracking start from a consistent, well-defined state.

VI. C HIMERA D ESIGN
A. Data Structures and Interfaces
Data Structures. C HIMERA handles the update and recovery
of metadata and logs separately.
1) Metadata. Metadata captures epoch and leader-election
information, such as the current epoch ep and the node’s
voting record. We denote the metadata of node pi as mdi .
Each metadata update corresponds to an operation op (e.g.,
sending an election vote), which is executed only after the
updated metadata has been sealed.
2) Log. Each node pi maintains a local log logi , which consists
of a sequence of entries enj indexed by j. A log entry is
identified by its epoch and index; we refer to this pair (ep, j)
as the entry’s epoch-index tag. We compare epoch-index tags
lexicographically: (ep1 , idx1 ) > (ep2 , idx2 ) iff ep1 > ep2 , or
ep1 = ep2 and idx1 > idx2 .
Interfaces. C HIMERA relies on sealing interfaces for protecting states stored in external storage, and on TC interfaces for
maintaining freshness across crashes.
1) Sealing interfaces. TEEs provide two sealing interfaces:
• Seal(data, h): Encrypt data with the TEE-internal sealing
key and then store the associated encrypted data at the
location identified by handle h in external storage.
• data ← UnSeal(h): Retrieve the sealed data from an
external storage with the handle h, decrypt it, and return the
associated plaintext data.

B. Metadata Recovery Mechanism
1) Metadata Update: C HIMERA protects metadata with
TC, since metadata is node-local and requires more precise
recovery to gain higher availability. For each metadata update,
the node first increments T Cmd and then seals the metadata
together with the counter value, following an inc-then-store
scheme [14] (Fig. 2a).
Using a TC provides a local freshness check for metadata,
but a raw TC is insufficient: if a crash occurs after the counter
is incremented but before the updated metadata is sealed,
recovery observes a counter mismatch and cannot distinguish
between a benign crash and a rollback. C HIMERA resolves
this ambiguity by binding the scalar epoch to T Cmd . The key
rule is to advance both values in lockstep: every metadata
update increases the epoch by exactly one and performs
one IncT C(md). This design turns T Cmd into a record of
epoch transitions. When the counter value matches the sealed
metadata, the node can safely restore the full metadata from
persistent storage. In the event of a mismatch, C HIMERA uses
the counter value as the recovered epoch. To maintain safety,
the node disables voting in this epoch due to uncertainty about
its previous votes.

6

When a node receives a request with a higher target epoch
ep∗ than its current epoch ep, it executes the following loop
until it reaches ep∗ :
① Invoke IncTC(md) to advance the trusted counter T Cmd
by one step and obtain the new value ctr.
② Update the metadata: increment md.epoch by 1 and set
md.vote according to the request type. For all intermediate
epochs, set md.vote to non-voting. When md.epoch reaches
the target ep∗ , set md.vote to the selected candidate if the
request is a vote request; otherwise, it remains non-voting.
③ Persist metadata: call Seal(md|ctr, hmd ) to store the
updated metadata along with the counter value.
④ Send vote reply: for vote requests, the node sends the
reply when md.epoch reaches ep∗ . The reply is sent after the
corresponding metadata has been sealed to ensure consistency.
A multi-epoch jump is executed as a sequence of singleepoch updates. All intermediate epochs are marked non-voting.
The final epoch records a candidate only if the triggering
request is a granted vote. This binding update keeps md.epoch
synchronized with T Cmd and prevents the node from casting
a different vote in any previous epoch.
2) Metadata Recovery: As shown in Fig. 2b, metadata
recovery follows the update scheme. The node first checks
the freshness of the sealed metadata using T Cmd . If the
sealed counter value matches the current counter value, the
node recovers from the sealed state. Otherwise, it recovers
to a safe epoch without risking double voting. The bindingupdate design binds T Cmd to the current epoch, maintaining
a correspondence between the counter value and epoch. This
correspondence provides a reliable anchor for safe recovery.
The recovery process then proceeds as follows:
① Load sealed metadata: the node invokes UnSeal(hmd ) to
retrieve md′ together with the sealed counter value ctr′ .
② Read current counter: the node calls ReadTC(md) to obtain
the current counter value ctr for freshness verification.
③ Check counter match: if ctr = ctr′ , the sealed metadata is
fresh. The node sets md ← md′ and finishes recovery.
④ Handle mismatch: if ctr ̸= ctr′ , the sealed metadata may
be outdated. Then the node starts metadata repair with ctr.
⑤ Repair metadata: the node reconstructs metadata by setting
md.epoch ← ctr and md.vote to non-voting. The node then
resumes execution using this repaired metadata.
3) Crash Window Analysis: The inc-then-store update is
not atomic. Crashes can occur in three intervals, as illustrated
in Fig. 2a: before incrementing T Cmd (E1); after incrementing
T Cmd but before sealing the metadata (E2); and after sealing
the metadata but before sending the vote reply or completing
the triggering operation (E3). Our analysis shows that each
crash falls into one of two cases. In the match case, the sealed
metadata can be safely restored. In the mismatch case, the
node repairs to a non-voting epoch.
• E1 Crash. At E1, the node has prepared an update, but
neither the epoch nor the T Cmd has changed. Since nothing
has been written to persistent metadata, there is no mismatch.
Recovery can safely restore the previous metadata, and no
actions or effects have been produced.

Update process

Recovery process
E3

Trusted
Counter

(1) IncTC("md")

E1

Potential crash point

(3) Execute update

Metadata

(2) Seal(md || ctr)

E2

(a) Update process of metadata
Trusted
Counter

(3) Compare Counter
(1) ReadTC("md")

Metadata

(2) UnSeal()

(b) Recovery process of metadata

Fig. 2: Recovery design of metadata.
• E2 Crash. At E2, T Cmd has advanced by one epoch,
but the corresponding metadata has not yet been sealed.
This creates a mismatch between the counter and the sealed
metadata. This mismatch is indistinguishable from an actual
rollback, rendering detection unreliable. The binding update
design resolves this issue by using the counter value as the
recovered epoch. Since no sealed metadata is available, the
node must disable voting in that epoch to prevent casting or
resending votes with incomplete metadata.
• E3 Crash. At E3, both T Cmd and the sealed metadata
have been updated. If no rollback occurred, the counter and
sealed metadata match, allowing the node to safely restore the
sealed metadata. With the complete metadata, any previously
recorded voting behavior can also be safely replayed. If a
rollback occurred, a mismatch between the counter and the
metadata will be observed. Recovery then uses the counter
value to restore the epoch and marks the node as non-voting.
C. Log Recovery Mechanism
A recovering node begins log recovery once metadata
recovery has established a safe epoch. Unlike metadata, logs
are replicated according to consensus rules, so C HIMERA can
recover them from other replicas. The role counter T Crole
allows a recovering node to determine its previous role before
the crash, and C HIMERA uses different recovery paths for the
two cases. Alg. 1 summarizes both paths.
1) Log Recovery for Followers: To ensure correctness,
the commit process must be properly executed: every entry
considered committed must be durably stored on a quorum of
replicas. A recovering follower, therefore, needs to catch up
with a safe leader. Generic DCR-style recovery may collect
f +1 replies to identify the latest leader and ensure that entries
still being replicated are not lost. In C HIMERA, metadata
recovery already restores a safe epoch for the follower. Therefore, a recovering follower can synchronize its log from any
leader whose epoch is no lower than its own.
The follower recovery path leverages this observation to
reduce message overhead. First, the follower unseals its local
log from persistent storage and determines its last-entry tag. It
then broadcasts a F OLLOWER R ECOVER message containing
its recovered epoch and last-entry tag (Alg. 1, L6–L12). Only

7

Algorithm 1 Log recovery at node pi

follower can then complete the recovery process.

1: resp ← 0 // the number of R EPLY R ECOVER messages received
2: tagmax ← ⊥ // the most up-to-date last-entry tag observed
3: src ← ⊥ // the ID of the node selected as the log source
4: done ← false // whether follower recovery has completed
5:
6: upon ⟨L OG R ECOVER, hlog ⟩:
7:
logi ← UnSeal(hlog )
8:
epi ← recovered metadata epoch of pi
9:
lasti ← last-entry tag of logi
10:
roleCtri ← ReadTC(role)
11:
if roleCtri is even then
12:
broadcast ⟨F OLLOWER R ECOVER, i, epi , lasti , non⟩
13:
else
14:
epi ← epi + 1
15:
broadcast ⟨L EADER R ECOVER, i, epi , lasti , non⟩
16:
end if
17:
18: upon receiving ⟨F OLLOWER R ECOVER, j, epj , lastj , non⟩:
19:
if rolei ̸= leader or epi < epj then return end if
20:
suffix ← log entries after lastj
21:
send ⟨R EPLY L OG, epi , suffix, i, non⟩ to pj
22:
23: upon receiving ⟨R EPLY L OG, ep, suffix, j, non⟩:
24:
if non is invalid or done or ep < epi then return end if
25:
append valid suffix to logi
26:
done ← true
27:
28: upon receiving ⟨L EADER R ECOVER, j, epj , lastj , non⟩:
29:
if epi > epj or (epi = epj and votei = disabled) then
30:
return
31:
end if
32:
if epi ≤ epj then
33:
// use the binding update of metadata
34:
epi ← epj
35:
votei ← disabled
36:
end if
37:
lasti ← last-entry tag of logi
38:
send ⟨R EPLY R ECOVER, epi , lasti , i, non⟩ to pj
39:
40: upon receiving ⟨R EPLY R ECOVER, ep, last, j, non⟩:
41:
if non is invalid then return end if
42:
resp ← resp + 1
43:
if last is more up-to-date than tagmax then
44:
tagmax ← last; src ← j
45:
end if
46:
if resp = f + 1 then
47:
synchronize log with psrc and resume proposing
48:
end if

2) Log Recovery for the Leader: The leader recovery path
is designed to minimize the system’s unavailability following
a leader crash. If the crashed leader waits for a timeout and
a subsequent election, the system remains unavailable during
that interval. In C HIMERA, the crashed leader can resume
service after recovery.
The leader first checks T Crole to confirm that it crashed
during a leader phase. It then advances the epoch and broadcasts a L EADER R ECOVER message (Alg. 1, L13–L16). A node
replies only if it can vote in that epoch. Before sending its
reply, the node catches up to the new epoch and records that
it has voted for the leader of that epoch. In this way, every
follower that contributes to the recovery quorum will not vote
for another candidate in the same epoch (Alg. 1, L28–L38).
The recovering leader collects f +1 replies and selects the
log with the most up-to-date last-entry tag. This quorum
is sufficient to preserve all committed entries. By quorum
intersection, any entry committed before the crash must appear
in at least one of the replies. Selecting the log with the latest
last-entry tag ensures that all committed entries are included,
preventing any loss (Alg. 1, L40–L48).
If the recovering leader resumes in the old epoch without
advancing, entries that were not yet committed could cause
inconsistencies. Depending on which followers respond, such
entries may or may not be included in the f +1 selected log. If
the leader resumes in the old epoch after losing these entries,
it could propose conflicting entries at the same index under
the same epoch-index tag. Moving to a new epoch eliminates
this ambiguity. After synchronization, the leader extends the
recovered log in the new epoch, and any unrecoverable,
uncommitted suffix from the old epoch is safely discarded.
3) Log Recovery Acceleration: C HIMERA further reduces
recovery and normal-case overhead through two optimizations
enabled by protocol-guided log recovery.
Unblocking Log Recovery. If the leader remains unchanged,
it tracks the next entry index for each follower. For follower
pi , this index is denoted as idxi . After unsealing its log
from persistent storage, pi can complete recovery once it has
caught up to idxi , without synchronizing with the leader’s
latest log. This mechanism accelerates recovery by eliminating
unnecessary synchronization.

the leader whose epoch is no lower than the follower’s epoch
responds. The reply contains the log entries after the follower’s
last-entry tag. This allows the follower to reuse its locally
persisted entries and fetch only the missing suffix (Alg. 1,
L18–L26). If the tags do not match, the follower replaces any
inconsistent entries with the corresponding entries from the
leader’s log.
The follower accepts the first valid leader reply that passes
the epoch check. It appends the corresponding suffix and
completes recovery. Any subsequent replies are ignored. If
no valid leader reply is received, a standard leader election
eventually produces a leader under partial synchrony. The

Background Log Persistence. C HIMERA moves log persistence off the commit critical path to reduce synchronous
I/O during normal-case replication. Under high load, a node
keeps newly appended log entries in memory and flushes
them to disk in the background. This design is safe because
log recovery relies on the cluster rather than the persistent
storage as the sole source. By contrast, some systems require
every committed log entry to be durably written to local
storage. Such systems cannot use background persistence.
Losing unflushed entries in these cases would compromise
safety.

8

• I6 (Leader append-only). While acting as a leader, a node
only appends at indices strictly greater than its current lastIdx
and never rewrites an earlier entry of its own log.

VII. C ORRECTNESS A NALYSIS AND V ERIFICATION
A. Correctness Analysis

• I7 (Up-to-date vote rule). A valid vote is generated
only if the candidate’s last-entry tag (lastEp, lastIdx) is
lexicographically greater than or equal to the voter’s own.
Invariants I1–I2 follow from the metadata update/recovery
procedure of §VI-B; I3–I4 follow from the leader-recovery
steps of §VI-C; I5–I7 are inherited from the underlying leaderbased CFT consensus (e.g., Raft [11]).

TEE integrity and the underlying CFT consensus protocol together guarantee consensus safety and liveness during
normal execution [65]; our goal is therefore to show that
C HIMERA’s recovery procedure preserves the three properties
of §IV-B in the presence of rollback attempts on sealed
storage. We collect primitive assumptions inherited from the
TEE and protocol-level invariants maintained by §VI-B–
§VI-C, state three formal theorems capturing those properties,
introduce two supporting lemmas, and prove the theorems in
dependency order.
By the threat model in §IV-A, trusted hardware provides the
following primitive guarantees:
• A1 (Counter monotonicity). For any counter we use, the
value returned by ReadTC is monotonically non-decreasing
across crashes, and IncTC returns a value strictly greater
than any value previously returned for the same counter.

Theorem 1 (Election Safety). In any epoch ep, at most one
node can act as a leader.
Theorem 2 (Leader Completeness). If an entry is committed
in epoch ep, every leader that assumes leadership in any later
epoch ep′ > ep contains that entry in its log.
Theorem 3 (Recovery Liveness). Every recovering, uncorrupted node eventually completes its recovery procedure.
We now state two supporting lemmas and then prove
Theorems 1–3 in dependency order.

• A2 (Seal authenticity). A successful UnSeal returns a
plaintext that was produced inside an enclave with the
matching measurement at some earlier time.

Lemma 1 (Recovered Epoch Monotonicity). For any correct
node, the epoch after any crash recovery is no smaller than
its epoch immediately before the crash.

• A3 (Enclave integrity). Code running inside an enclave
executes as specified; the adversary can delay or drop enclave
messages, but cannot tamper with authenticated messages.
Each node maintains two trusted counters: T Cmd for metadata
updates and T Crole for role tracking.
Protocol Invariants. The procedures in §VI-B–§VI-C maintain the following invariants at every correct node.
• I1 (Counter–epoch binding). After every successful metadata update, the sealed metadata satisfies md.epoch =
ctr, where ctr is the value returned by the corresponding
IncTC(md); the in-memory epoch always equals md.epoch.

Proof. Let pi be a correct node whose pre-crash epoch is ep.
By I1, ep equals the value ctr returned by pi ’s last successful
IncTC(md) (with ep = 1 if pi has never completed a metadata
update). By A1, ReadTC(md) at recovery returns some v ≥
ctr = ep. Recovery then proceeds as follows: in the match
case, the restored epoch equals the sealed ctr′ = v (which
by A2 and I1 is authentic) and thus equals v ≥ ep; otherwise
(mismatch or UnSeal failure), I2 sets the epoch to v ≥ ep.
Lemma 2 (Vote Uniqueness per Epoch). In each epoch, a
node can grant a valid vote to at most one node.

• I2 (Mismatch repair). During recovery, if ReadTC(md)
differs from the sealed counter value, the node sets its epoch
to ReadTC(md) and disables voting in that epoch.

• I4 (Safe log selection in leader recovery). Before a
recovering leader proposes in a fresh epoch, it collects
R EPLY R ECOVER responses from at least f +1 nodes and
adopts the responder’s log with the most up-to-date last-entry
tag (ep, lastIdx).

Proof. In C HIMERA, a valid vote is either a normal election
vote or a recovery vote carried by a R EPLY R ECOVER message.
Fix an epoch ep and a node pi . In a normal election, pi can
vote for ep only when it first advances its epoch to ep. I1
and A1 ensure that pi has at most one voting opportunity
in ep. Upon recovery, pi either restores the sealed vote or
disables voting for the recovered epoch by I2. Thus, pi
cannot cast another vote in ep. In leader recovery, sending
a R EPLY R ECOVER counts as a recovery vote. By I3, pi sends
this message only if it is still eligible to vote in ep, and it then
becomes non-voting. Therefore, pi can grant a valid vote to
at most one node in epoch ep.

• I5 (Log prefix consistency). A follower appends a suffix from a leader only if the suffix’s predecessor tag
(prevEp, prevIdx) matches the follower’s local entry at
prevIdx. If the tags do not match, the follower truncates
the conflicting suffix and retries from an earlier matched
tag. This rule ensures that log synchronization preserves a
common prefix between the leader and follower.

Proof of Theorem 1. Assume, for contradiction, that two distinct nodes pa ̸= pb act as leaders in the same epoch ep. Let
Qa and Qb be their vote quorums, where a recovery quorum of
R EPLY R ECOVER messages is treated as a vote quorum. Then
|Qa | ≥ f + 1 and |Qb | ≥ f + 1. Since n = 2f + 1, we have
Qa ∩ Qb ̸= ∅. Let pc ∈ Qa ∩ Qb . Then pc grants a valid vote
to both pa and pb in epoch ep, contradicting Lemma 2.

• I3 (Fresh epoch for leader recovery). A recovering leader
uses T Crole to identify that it crashed in a leader phase.
Before resuming, it advances metadata to a new epoch and
obtains at least f +1 R EPLY R ECOVER responses from nodes
that were eligible to vote in the new epoch.

9

The following technical lemma underlies the proof of
Leader Completeness.

B. Formal Verification.
We develop a Maude [66] specification of C HIMERA atop
Braft and model check it against both safety and liveness
guarantees expressed in linear temporal logic (see Appendix C
for details). We chose Maude as it is a well-established
formal specification language and analysis framework that has
been successfully applied to a broad range of distributed and
networked systems [67], [68], [69], [70]. Within the explored
bound, the model checker reports no counterexample to the
three recovery properties and protocol liveness.

Lemma 3 (Leader Append-Only Preservation). If LT becomes
the leader with entry en at index k in its log, then LT preserves
en at index k throughout its leadership. Moreover, any new
entry proposed by LT is appended after index k.
Proof. When LT becomes the leader, its last log index is at
least k. By I6, LT only appends new entries after its current
last log index and never rewrites earlier entries while it remains
the leader. Therefore, en remains at index k, and every new
entry proposed by LT is appended after index k.

VIII. I MPLEMENTATION
We implement C HIMERA on top of Braft [20] and
ZooKeeper [71], which use the leader-based CFT consensus
protocols Raft and Zab, respectively.1 We refer to these
implementations as C HIMERA-B and C HIMERA-Z. For TEE
support, we port C HIMERA to VM–based TEEs, i.e., Intel
TDX, and apply additional optimizations to mitigate the performance overhead of TEEs. (The rationale for choosing Braft,
ZooKeeper, and TDX is discussed in §I.)
We adopt the software-based counter TIKS [4] for metadata
recovery. Moreover, we use Narrator-Pro [43] as the trusted
counter to safely update the counter within a single round
of communication. For sealing functionality, we invoke Intel
SDK [72] to retrieve the measurement (mr td) of TDX, and
derive a stable sealing key from it. This key is used to encrypt
and decrypt data for the persistent storage outside TEEs.
Implementation atop Braft. We list required modifications
on Braft to realize C HIMERA:
• Metadata. Raft’s metadata consists of two fields: currentTerm and votedFor (§V-A). In Braft, these fields are combined
into a single on-disk structure called raftMeta. In C HIMERA,
we attach a counter value to each update of raftMeta to support
precise recovery.

Proof of Theorem 2. Let en be committed at index k in epoch
ep by leader L. By the commit rule, en is replicated to a
quorum Qc with |Qc | ≥ f + 1, each storing en at (ep, k).
We prove by strong induction on T > ep that every leader
LT in epoch T holds en at index k throughout its tenure.
Base case (T = ep + 1). Consider the first leader Lep+1
after en is committed. By the quorum intersection argument,
Lep+1 adopts a log containing the committed prefix including
en at k. Lemma 3 then ensures en remains at k.
Induction hypothesis. Assume that every leader LT ′ for
ep < T ′ ≤ T contains en at k throughout its tenure.
Induction step (T → T + 1). Consider leader LT +1 .
- If LT +1 assumes leadership via a normal election, let Qv
be its vote quorum. Since |Qv |, |Qc | ≥ f + 1, they intersect at
some r. By induction hypothesis and I5, r’s log contains en
at k. By I7, r votes only if LT +1 ’s last-entry tag ⪰ r’s, so I5
ensures LT +1 ’s log contains en.
- If LT +1 assumes leadership via recovery, let Qr be the
recovery-quorum with Qr ∩ Qc ̸= ∅ and r in it; by I4, LT +1
adopts the log with largest last-entry tag ≥ r’s log, which I5
ensures preserves the committed prefix with en at k.
Thus, by induction, every leader in epoch T + 1 contains
en at index k before proposing, completing the proof.

• Log. In C HIMERA-B, new log entries are first buffered in
memory. Their persistence is handled by a background thread
that runs during idle periods. This design reduces criticalpath latency and allows higher throughput compared to Braft
(§IX-B). For recovery, we reuse Braft’s existing replicator
component to transmit log entries to recovering nodes.
Implementation atop ZooKeeper. Modifications on
ZooKeeper, including the log, are similar to those on Braft.
Thus, we only list the differences:
• Metadata. Unlike Raft’s single-phase election, the Zab protocol uses a two-phase process: discovery and synchronization. In this process, a prospective leader is first nominated
and is only promoted after receiving acknowledgments from
a majority of followers [22]. Specifically, after Fast Leader
Election (FLE), the newly elected leader enters the discovery
phase. It proposes a fresh epoch ep∗ , which is greater than
any previously observed. Each follower persists this value as
acceptedEpoch and returns an acknowledgment to the leader.
In Zab, followers do not need to record which node they
voted for. Election safety is ensured instead by two metadata

Proof of Theorem 3. Consider a correct node pi beginning
recovery.
(1) Metadata recovery. pi invokes UnSeal and
ReadTC(md), both O(1) TEE operations. On match, it
restores md′ (I1); on mismatch or UnSeal failure, it applies
I2. In either case, this phase terminates.
(2) Log recovery. If pi is a follower, after GST a current
or newly elected leader is reachable within bounded time. The
leader replies to F OLLOWER R ECOVER with the missing suffix,
which pi appends to complete recovery. If pi is a recovering
leader, it broadcasts L EADER R ECOVER; after GST, either f +
1 nodes in the fresh epoch respond with R EPLY R ECOVER, in
which case I4 lets pi adopt the most up-to-date log and resume
proposing, or another leader is eventually established and pi
recovers via the follower path. A nonce in recovery messages
prevents responses from being replayed.
Each phase completes in bounded time after GST, so pi
eventually rejoins the system.

1 Source code is at https://github.com/Artifacts2026/CHIMERA.

10

fields: acceptedEpoch and currentEpoch. Together, these fields
guarantee that at most one leader is recognized by the
majority. Because the two fields are updated under different
conditions, we use separate counters to protect them independently.

bandwidth to 1 Gbps per node and enforcing a 40 ms roundtrip latency with ±4 ms jitter. Note that WAN evaluation is
performed in emulation because TEE-enabled instances are
restricted to specific cloud regions; this reflects deployment
constraints rather than a design flaw.
Parameters and Metrics. We vary the fault-tolerance parameter f ∈ {1, 2, 4, 10, 20}. The leader processes client
requests in batches of 256, with each log entry containing a
256 B payload. To balance memory consumption and network
utilization, the system bounds the number of in-flight log
entries at 5120. In addition to throughput and latency, we
measure recovery time, defined as the interval from when a
faulty node restarts until it fully recovers and resumes service.

IX. E VALUATION
We evaluate the performance of C HIMERA-B (built on
Braft [20]) and C HIMERA-Z (built on ZooKeeper [71]). Since
ZooKeeper comprises additional components beyond consensus and its performance is influenced by multiple factors,
we focus our main evaluation on C HIMERA-B, especially in
comparison with its counterparts, while providing an overhead profiling of C HIMERA-Z. We consider four recovery
taxonomies (§III) and adopt them on Braft as baselines. For
consistency, all protocols running inside TDX perform disk
I/O through sealing.
• Braft-TC adopts a software-based counter to protect state
updates as Engraft [4]. Each counter increment requires two
rounds of broadcast to advance the state.

B. Fault-Free Performance
This section evaluates C HIMERA-B’s performance of log
commitment in the fault-free scenarios with no failures or
leader changes. We compare C HIMERA-B with Braft-TC,
Braft-RFT, and Braft-DR. As Braft-DCR and Braft-RC introduce no additional overhead in normal-case operations
compared to Braft-DR, their results are not presented in detail.
Due to space constraints, we present the throughput–latency
scalability of C HIMERA-B in Appendix B.
1) Performance in WAN: We evaluate throughput and
latency under a WAN deployment while varying the fault
threshold f (Fig. 3a and 3b). In this setting, communication
overhead and bandwidth limits dominate performance.
During each transaction, Braft-TC requires two additional
communication rounds to interact with its TC for securely
recording log changes. According to Raft’s replication semantics, this results in a total of five communication rounds to
complete a transaction, incurring substantial latency. BraftRFT, by contrast, adopts a 3f + 1 configuration, which, for
the same fault-tolerance level f , entails more nodes and thus
higher processing overhead. Moreover, an additional counter
update during leader persistence incurs one more communication round, which further degrades WAN performance.
Both C HIMERA-B and Braft-DR operate with 2f + 1 nodes
and incur no additional normal-case overhead, which explains
their relatively higher throughput in this environment. Nevertheless, C HIMERA-B’s recovery mechanism does not rely on
the completeness of on-disk logs, enabling it to safely defer
disk writes to background operations and thus reduce normalcase commit latency. This design yields an average throughput
improvement of approximately 10% over Braft-DR; however,
as communication dominates in the WAN scenario, the advantage from reduced I/O latency is less pronounced.
2) Performance in LAN: We also evaluate the throughput
and latency of C HIMERA in a LAN deployment to minimize
the effect of network communication (Fig. 3c and 3d). As
the network communication cost is negligible in a LAN
environment, local processing and I/O overhead become the
dominant factors affecting performance.
C HIMERA-B exhibits a pronounced throughput advantage
over all other variants, particularly at low fault tolerance levels,
due to its normal-case optimization that defers durable disk

• Braft-RFT follows FlexiBFT [73] by increasing the node
size from 2f + 1 to 3f + 1 and employs a TC only at the
leader to prevent equivocating proposals.
• Braft-RC adopts the original reconfiguration design of Raft,
implemented through joint consensus [11].
• Braft-DCR follows Achilles [18] to ask a recovering node
to skip two epochs for safety. In Braft, the Pre-Vote mechanism bounds the growth of epoch, making DCR feasible.
In addition to the four baselines, we introduce Braft-Direct
Recovery (Braft-DR) to evaluate the overhead of rollback
resilient solutions. In Braft-DR, a node restores its state
directly from sealed data on untrusted storage.
We evaluate C HIMERA-B against all baselines under both
fault-free and faulty scenarios to answer three questions:
Q1: How does C HIMERA perform with varying nodes in
WAN and LAN compared to its counterparts? (§IX-B)
Q2: What is the performance of C HIMERA’s recovery protocol, where do the primary bottlenecks lie, and how does it
compare to prior approaches? (§IX-C)
Q3: How much overhead do TEE-related operations introduce, and how effective are our optimizations? (§IX-D)
A. Experimental Setup
We conducted all experiments on a public cloud platform
using up to 61 Intel TDX-enabled instances, with one instance
per node. Each node ran on a dedicated virtual machine
provisioned with 4 vCPUs and 16 GB of RAM, running Linux
kernel 5.10 LTS (64-bit).
We evaluate C HIMERA under two deployment scenarios:
Local Area Network (LAN) and Wide Area Network (WAN).
Both are configured using the Linux tc tool for traffic shaping.
In the LAN setting, the per-node bandwidth is limited to
10 Gbps, and the inter-node RTT stays below 1 ms. In the
WAN setting, we emulate wide-area conditions by limiting

11

20

Braft-TC
Braft-RFT

300
200

100
0

1

2

4

10

20

Number of faults

1

2

4

10

Chimera-B
Braft-DR
Braft-TC
Braft-RFT

30

20

10

2

60
40

4

10

20

1

2

Number of faults

Number of faults

(a) Normal case, WAN

Chimera-B
Braft-DR
Braft-TC
Braft-RFT

80

20
1

20

Latency (ms)

40

Chimera-B
Braft-DR

400

Throughput (wTPS)

60

Latency (ms)

Throughput (kTPS)

Chimera-B
Braft-DR
Braft-TC
Braft-RFT

80

(b) Normal case, WAN

4

10

20

Number of faults

(c) Normal case, LAN

(d) Normal case, LAN

Cost (s)

Braft-DR

Braft-TC

Braft-RC

Braft-DCR

C HIMERA-B

Prep.
Sync.
Quie.
Total

N/A
6.15
N/A
6.15

0.01
6.31
N/A
6.32

0.01
117.01
N/A
117.02

N/A
24.53
2 epochs
24.53 + 2 epochs

0.01
6.42 (∗ 18)
N/A
6.43 (∗ 18.01)

writes to background operations (as explained above). When
f = 1, this optimization leads to a 68% improvement over
Braft-DR that follows the original Braft I/O semantics of
synchronous persistence.
Braft-TC incurs further penalties from its additional communication and trusted counter operations, though in the
LAN setting, these penalties are partially masked by the low
RTT. Braft-RFT, with its 3f + 1 configuration, experiences
a more significant throughput drop as fault increases, due
to the larger quorum size and corresponding processing and
message handling overhead. Overall, C HIMERA-B shows superior performance when network delays are not the primary
bottleneck and storage-layer optimizations directly translate
into substantial end-to-end performance gains.

Throughput (kTPS)

TABLE III: Recovery overhead.

Shutdown
Restart
Recovery Completed

20

150
100

15

50
00

Available Node #

Fig. 3: Throughput and latency comparisons with varying nodes in WAN and LAN.

2

4

6

8

Time Window (s)

10

12

1410

Fig. 4: The fault recovery process of C HIMERA-B.
the node must reconstruct its state from scratch, which takes
about 110–120 seconds for 5 GB of data. For data-intensive
applications such as blockchains, where the state can reach
several terabytes (e.g., Bitcoin [74]), the recovery time under
RC can extend to hours or even days.
Braft-DCR’s recovery requires disk loading (≈ 6s) and
network synchronization (≈ 18s). By contrast, C HIMERA
leverages its optimization to safely recover without network
catch-up, which takes about 6.42s. Without the optimization,
this stage takes about 18s.
• Quiescence. This stage ensures safety during rejoining.
Braft-DCR requires a node to skip two epochs, which in
practical deployments can range from tens of seconds to
hours or even days.
Throughput under Recovering Faults. We evaluate the
system-level impact of recovery in terms of system throughput
and available nodes, focusing on C HIMERA-B. Since BraftDR, Braft-DCR (without quiescence stage), and Braft-TC
exhibit recovery latencies comparable to C HIMERA-B, their
effects are effectively captured by C HIMERA-B and are omitted here. For completeness, we defer the results for Braft-RC
to Appendix B due to space constraints.
Fig. 4 shows the throughput variation and the number
of available nodes (i.e., who participate in consensus) of
C HIMERA-B during faulty nodes’ recovery. At the shutdown
point (i.e., 2.2 seconds), the throughput briefly drops to zero
as the leader handles connection failures. Once stabilized,
the throughput surpasses the pre-failure baseline. This occurs
because, although the quorum size is unchanged, the leader
no longer replicates log entries to the failed nodes. Thus, the
leader’s available bandwidth is redistributed to the remaining
nodes, reducing contention and increasing throughput.
During recovery, faulty nodes reload their logs from disk,

C. Performance under Faults
We evaluate C HIMERA-B against Braft-DR, Braft-TC,
Braft-RC, and Braft-DCR to measure rollback-resilient recovery overhead under faults. Braft-RFT is omitted, as its
recovery is identical to Braft-DR. We deploy 21 nodes in a
LAN deployment and simulate failures by shutting down and
restarting 10 nodes while continuously issuing client requests.
Each node is equipped with 210 MB/s sequential read/write
throughput and preloaded with 5 GB of log entries to emulate
a large-scale deployment.
Single-Node Recovery Latency. Table III presents the time
for a recovering node to rejoin the protocol. To enable fair
comparison, we divide the recovery process into three stages:
preparation, synchronization, and quiescence, as below.
• Preparation. This stage includes the steps required to initialize state synchronization. In Braft-TC and C HIMERA-B,
the node reads the trusted counter, which takes about 10 ms.
In Braft-RC, the recovering node performs reconfiguration
through two consensus rounds, also taking around 10 ms.
The latency of RC depends on inter-node message delays.
• Synchronization. This stage involves loading metadata and
log entries, with the latter dominating the cost. In Braft-RC,

12

30
20
10
1

2

4

10

20

Throughput (kTPS)

Throughput (wTPS)

Chimera-B
NoTEE
SyncWrite
NoEncrypt

40

120

80

40

1

Number of faults

(a) C HIMERA-B breakdown

the Zab protocol [22]. Similarly, Brandenburger et al. [40]
integrate Intel SGX into Hyperledger Fabric [76] to secure
smart contract execution. CCF [8] uses enclaves to maintain a
distributed key-value store and runs Raft [11] to achieve low
latency and tolerate a minority of Byzantine faults.
However, most systems do not consider TEEs’ rollback
attacks. Engraft [4] first identified this threat and introduced
TIKS, i.e., software-based counters, to enforce trusted counters
for rollback protection. This approach corresponds to TC (as
introduced in §III), which represents the most general solution.
More details of the trusted counter are introduced shortly.
Later versions of CCF [9] address the issue by reconfiguration,
referred to as RC. However, these solutions degrade either
performance or availability (§III).
TEE-Assisted BFT Consensus. Unlike confidential BFT consensus that ports whole consensus protocols into TEEs, TEEAssisted BFT consensus [77], [78], [79], [80], [73], [81],
[18] usually utilizes TEEs to provide some trusted functions,
such as the append-only log and monotonic counter, to minimize Trusted Computing Base (TCB). These trusted functions
can prevent Byzantine nodes from equivocating messages,
resulting in better scalability in terms of smaller quorum size
and shorter transaction latency. Recently, FlexiBFT [73] and
Achilles [18] identified TEEs’ rollback issues in TEE-Assisted
BFT consensus and proposed RFT and NVR as solutions,
respectively. However, RFT relaxes the tolerance, while NVR
weakens the system’s tolerance. See more details in §III.

Chimera-Z
NoTEE
SyncWrite
NoEncrypt

2

4

10

20

Number of faults

(b) C HIMERA-Z breakdown

Fig. 5: Overhead profiling of TEE-related execution.
without affecting ongoing throughput. However, once recovery
completes, these nodes lag behind because the leader continues
to serve client requests. To rejoin replication, they must first
catch up, consuming bandwidth and temporarily reducing
throughput until synchronization finishes.
D. Overhead Profiling
To better understand the overhead of TEE-related operations, we compare variants of C HIMERA-B and C HIMERA-Z.
• NoTEE. It runs outside Intel TDX, serving as a baseline to
measure TEE-related overhead.
• SyncWrite. It uses synchronous disk writes instead of asynchronous persistence, isolating the cost of log persistence.
• NoEncrypt. It disables memory encryption atop SyncWrite,
revealing the overhead of cryptographic operations.
C HIMERA-B. Fig. 5a shows the throughput of C HIMERA-B
and its variants as the number of faults (f ) increases in a LAN
setting, with all other parameters identical to the fault-free
scenarios. Compared to NoTEE variant, C HIMERA-B shows
an 8–10% slowdown, capturing the inherent cost of TEE
execution. Relative to SyncWrite variant, persistence alone
adds roughly 15–20% overhead, independent of encryption.
Finally, the difference between the SyncWrite and NoEncrypt
variants quantifies the cost of cryptographic sealing, resulting
in an additional 10–15% performance degradation.
C HIMERA-Z. Fig. 5b presents the maximum throughput of
C HIMERA-Z and its variants. While C HIMERA-Z exhibits
a similar trend, the performance gaps are smaller. This is
because the additional components and coordination overhead
in ZooKeeper limit peak throughput, masking much of the
relative impact of TEE execution and persistence operations.

XI. C ONCLUSION AND F UTURE W ORK
We systematically analyze existing TEE rollback-resilient
solutions, establishing a taxonomy to assess their suitability
for confidential BFT consensus. Building on the insights, we
propose C HIMERA, a hybrid recovery framework that tailors
recovery strategies for persistent state. We prove C HIMERA’s
correctness and complement our proofs with formal verification of its Braft design. We implement proof-of-concept
prototypes atop Raft and ZooKeeper using Intel TDX, and
our extensive evaluation demonstrates that C HIMERA delivers
superior performance.
Next, we discuss our approach’s limitations and potential
extensions. First, C HIMERA focuses on rollback-resilient recovery and does not currently support dynamic reconfiguration. Integrating reconfiguration with recovery is challenging
because configuration updates are themselves stored in the
replicated log. During recovery, a node must know the current configuration to safely recover the log, while the latest
configuration may only exist inside the log being recovered.
We leave the integration of reconfiguration into C HIMERA as
future work. Second, although we focus on confidential BFT
consensus, the core insight of C HIMERA, i.e., using protocollevel semantics to design tailored TEE recovery, extends to
other confidential computing systems, including confidential
MapReduce frameworks [82], federated learning [83], and
encrypted databases [36]. More broadly, separating critical
metadata from bulk state and customizing recovery accordingly may benefit a wider range of stateful TEE applications.

X. R ELATED W ORK
We discuss prior work on confidential computing, TEEassisted BFT consensus, and trusted counters.
Confidential BFT Service. Confidential BFT services have
recently attracted significant attention from industry and
academia, driven by the explosive growth of cloud and decentralized applications. Notable industrial examples include
SVR3 [3] that ports Raft into TEEs to secure private key
management, and Azure that provides Confidential Ledger
service [75] atop CCF. Meanwhile, in academia, SecureKeeper [2] is among the first to use TEEs to protect metadata
confidentiality in cloud settings with minimal changes to

13

E THICS C ONSIDERATIONS

[22] F. P. Junqueira, B. C. Reed, and M. Serafini, “Zab: High-performance
broadcast for primary-backup systems,” in Proc. of DSN, 2011.
[23] “Intel Trust Domain Extensions,” https://www.intel.com/content/
dam/develop/external/us/en/documents/tdx-whitepaper-final9-17.pdf,
retrieved September 2025.
[24] M. Hoekstra, R. Lal, P. Pappachan, V. Phegade, and J. Del Cuvillo,
“Using innovative instructions to create trustworthy software solutions,”
in Proc. of HASP, 2013.
[25] D. Kaplan, J. Powell, and T. Woller, “AMD SEV-SNP: Strengthening
VM isolation with integrity protection and more,” AMD, Tech. Rep.,
2020. [Online]. Available: https://www.amd.com/system/files/TechDocs/
SEV-SNP-strengthening-vm-isolation-with-integrity-protection.pdf
[26] “Building a secure system using TrustZone technology,” https://
documentation-service.arm.com/static/5f212796500e883ab8e74531, retrieved September 2025.
[27] R. Cheng, F. Zhang, J. Kos, W. He, N. Hynes, N. Johnson, A. Juels,
A. Miller, and D. Song, “Ekiden: A platform for confidentialitypreserving, trustworthy, and performant smart contracts,” in Proc. of
EuroS&P, 2019.
[28] J. Lind, O. Naor, I. Eyal, F. Kelbert, E. G. Sirer, and P. Pietzuch,
“Teechain: A secure payment network with asynchronous blockchain
access,” in Proc. of SOSP, 2019.
[29] X. Wen, Q. Feng, H. Lyu, J. Niu, Y. Zhang, and C. Feng, “TeeRollup:
Efficient rollup design using heterogeneous TEE,” in IEEE Transactions
on Computers, 2025.
[30] X. Wen, Q. Feng, J. Niu, Y. Zhang, and C. Feng, “Mercury: Practical
cross-chain exchange via trusted hardware,” IEEE Transactions on
Dependable and Secure Computing, vol. 23, no. 2, pp. 2949–2961, 2026.
[31] S. Xie, D. Kang, H. Lyu, J. Niu, and M. Sadoghi, “Fides: Scalable
censorship-resistant DAG consensus via trusted components.”
[32] A. Oprea and M. K. Reiter, “Integrity checking in cryptographic file
systems with constant trusted storage.” in Proc. of USENIX Security,
2007.
[33] S. Ghemawat, H. Gobioff, and S.-T. Leung, “The google file system,”
in Proc. of SOSP, 2003.
[34] J. Hamilton, “On designing and deploying internet-scale services,” in
Proc. of LISA, 2007.
[35] M. D. Schroeder, A. D. Birrell, and R. M. Needham, “Experience with
grapevine: the growth of a distributed system,” ACM Trans. Comput.
Syst., vol. 2, no. 1, p. 3–23, 1984.
[36] C. Priebe, K. Vaswani, and M. Costa, “EnclaveDB: A secure database
using SGX,” in Proc. of S&P. IEEE, 2018.
[37] A. Wilde, T. N. Gruel, C. Soriente, and G. Karame, “The forking way:
When tees meet consensus,” arXiv preprint, 2024.
[38] “Signal
Secure
Value
Recovery,”
https://signal.org/blog/
secure-value-recovery, retrieved September 2025.
[39] M.
Azure.,
“Confidential
consortium
framework,”
https://www.microsoft.com/en-us/research/project/
confidential-consortium-framework/, retrieved September 2025.
[40] M. Brandenburger, C. Cachin, R. Kapitza, and A. Sorniotti, “Trusted
computing meets blockchain: Rollback attacks and a solution for Hyperledger Fabric,” in Proc. of SRDS, 2019.
[41] L. F. G. Sarmenta, M. van Dijk, C. W. O’Donnell, J. Rhodes, and
S. Devadas, “Virtual monotonic counters and count-limited objects using
a TPM without a trusted OS,” in Proc. of STC, 2006.
[42] “Trusted
time
and
monotonic
counters
with
intel
software
guard
extensions
platform
services,”
https:
//www.intel.com/content/www/us/en/content-details/671564/
trusted-time-and-/monotonic-counters-with-intel-software-/
guard-extensions-platform-services.html, retrieved September 2025.
[43] W. Peng, X. Li, J. Niu, X. Zhang, and Y. Zhang, “Ensuring state
continuity for confidential computing: A blockchain-based approach,”
IEEE Trans. Dependable Secure Comput., vol. 21, no. 6, pp. 5635–5649,
2024.
[44] E. Michael, D. R. K. Ports, N. K. Sharma, and A. Szekeres, “Providing
stable storage for the diskless crash-recovery failure model,” University
of Washington, Tech. Rep. UW-CSE-16-08-02, 2016.
[45] B. Liskov and J. Cowling, “Viewstamped replication revisited,” MIT
CSAIL, Tech. Rep. MIT-CSAIL-TR-2012-021, 2012.
[46] T. D. Chandra, R. Griesemer, and J. Redstone, “Paxos made live: An
engineering perspective,” in Proc. of PODC, 2007.
[47] J. Kończak, N. Santos, T. Żurkowski, P. T. Wojciechowski, and
A. Schiper, “Jpaxos: State machine replication based on the paxos

This work studies rollback-resilient recovery for confidential
BFT consensus systems. Our experiments are conducted on
controlled cloud testbeds using synthetic workloads and do
not involve human subjects, personal data, or attacks on thirdparty systems. The evaluated vulnerabilities are analyzed under
an abstract threat model, and the artifacts are intended solely
for research and reproducibility.
R EFERENCES
[1] A. Jeffery, J. Maffre, H. Howard, and R. Mortier, “LSKV: A confidential
distributed datastore to protect critical data in the cloud,” arXiv preprint,
2024.
[2] S. Brenner, C. Wulf, D. Goltzsche, N. Weichbrodt, M. Lorenz, C. Fetzer,
P. Pietzuch, and R. Kapitza, “SecureKeeper: Confidential ZooKeeper
using Intel SGX,” in Proc. of Middleware, 2016.
[3] G. Connell, V. Fang, R. Schmidt, E. Dauterman, and R. A. Popa, “Secret
key recovery in a global-scale end-to-end encryption system,” in Proc.
of OSDI, 2024.
[4] W. Wang, S. Deng, J. Niu, M. K. Reiter, and Y. Zhang, “Engraft:
Enclave-guarded raft on Byzantine faulty nodes,” in Proc. of CCS, 2022.
[5] Y. Yan, C. Wei, X. Guo, X. Lu, X. Zheng, Q. Liu, C. Zhou, X. Song,
B. Zhao, H. Zhang et al., “Confidentiality support over financial grade
consortium blockchain,” in Proc. of ACM SIGMOD, 2020.
[6] “The oasis blockchain platform,” https://assets.website-files.com/
5f59478e350b91447863f593/628ba74a9aee37587419cf65 20200623%
20The%20Oasis%20Blockchain%20Platform.pdf, retrieved September
2025.
[7] M. Russinovich, E. Ashton, C. Avanessians, M. Castro, A. Chamayou,
S. Clebsch, M. Costa, C. Fournet, M. Kerner, S. Krishna et al., “CCF:
A framework for building confidential verifiable replicated services,”
Microsoft Research and Microsoft Azure, Tech. Rep., 2019.
[8] H. Howard, F. Alder, E. Ashton, A. Chamayou, S. Clebsch, M. Costa,
A. Delignat-Lavaud, C. Fournet, A. Jeffery, M. Kerner, F. Kounelis,
M. A. Kuppe, J. Maffre, M. Russinovich, and C. M. Wintersteiger, “Confidential consortium framework: Secure multiparty applications with
confidentiality, integrity, and high availability,” Proc. VLDB Endow.,
vol. 17, no. 2, p. 225–240, 2023.
[9] H. Howard, M. A. Kuppe, E. Ashton, A. Chamayou, and N. Crooks,
“Smart casual verification of the confidential consortium framework,”
in Proc. of NSDI, 2025.
[10] M. Gao, H. Dang, and E.-C. Chang, “TEEKAP: Self-expiring data
capsule using Trusted Execution Environment,” in Proc. of ACSAC,
2021.
[11] D. Ongaro and J. Ousterhout, “In search of an understandable consensus
algorithm,” in Proc. of ATC, 2014.
[12] S. Matetic, M. Ahmed, K. Kostiainen, A. Dhar, D. Sommer, A. Gervais,
A. Juels, and S. Capkun, “ROTE: Rollback protection for trusted
execution,” in Proc. of USENIX Security, 2017.
[13] J. Niu, W. Peng, X. Zhang, and Y. Zhang, “Narrator: Secure and practical
state continuity for trusted execution in the cloud,” in Proc. of CCS,
2022.
[14] R. Strackx and F. Piessens, “Ariadne: A minimal approach to state
continuity,” in Proc. of USENIX Security, 2016.
[15] B. Parno, J. R. Lorch, J. R. Douceur, J. Mickens, and J. M. McCune,
“Memoir: Practical state continuity for protected modules,” in Proc. of
S&P, 2011.
[16] S. Gupta, S. Rahnama, S. Pandey, N. Crooks, and M. Sadoghi, “Dissecting BFT consensus: In trusted components we trust!” in Proc. of
EuroSys, 2023.
[17] B. Dinis, P. Druschel, and R. Rodrigues, “RR: A fault model for efficient
TEE replication,” in Proc. of NDSS, 2023.
[18] J. Niu, X. Wen, G. Wu, S. Liu, J. Yu, and Y. Zhang, “Achilles: Efficient
TEE-assisted BFT consensus via rollback resilient recovery,” in Proc.
of EuroSys, 2025.
[19] S. Angel, A. Basu, W. Cui, T. Jaeger, S. Lau, S. Setty, and S. Singanamalla, “Nimble: Rollback protection for confidential cloud services,” in
Proc. of OSDI, 2023.
[20] “Braft,” https://github.com/baidu/braft, retrieved September 2025.
[21] P. Hunt, M. Konar, F. P. Junqueira, and B. Reed, “ZooKeeper: Wait-free
coordination for internet-scale systems,” in Proc. of ATC, 2010.

14

protocol,” EPFL, Tech. Rep. 167765, 2011. [Online]. Available:
https://infoscience.epfl.ch/handle/20.500.14299/69874
[48] A. Bessani, M. Correia, T. Distler, R. Kapitza, P. Esteves-Verissimo, and
J. Yu, “Vivisecting the dissection: On the role of trusted components in
BFT protocols,” arXiv preprint arXiv:2312.05714, 2023.
[49] A. Clement, F. Junqueira, A. Kate, and R. Rodrigues, “On the (limited)
power of non-equivocation,” in Proc. of PODC, 2012.
[50] D. Giantsidi, E. Giortamis, J. Pritzi, M. Bailleu, M. Kapritsos, and
P. Bhatotia, “Recipe: Hardware-accelerated replication protocols,” arXiv
preprint, 2025.
[51] J.-Y. Kim, J. Lee, Y. Koo, S. Park, and S.-M. Moon, “Ethanos: efficient
bootstrapping for full nodes on account-based blockchain,” in Proc. of
EuroSys, 2021.
[52] H. Feng, Y. Hu, Y. Kou, R. Li, J. Zhu, L. Wu, and Y. Zhou, “SlimArchive: A lightweight architecture for ethereum archive nodes,” in
Proc. of USENIX ATC, 2024.
[53] M. van Dijk, J. Rhodes, L. F. G. Sarmenta, and S. Devadas, “Offline untrusted storage with immediate detection of forking and replay attacks,”
in Proc. of STC, 2007.
[54] R. Strackx, B. Jacobs, and F. Piessens, “ICE: A passive, high-speed,
state-continuity scheme,” in Proc. of ACSAC, 2014.
[55] M. Schwarz, M. Lipp, D. Moghimi, J. Van Bulck, J. Stecklina,
T. Prescher, and D. Gruss, “ZombieLoad: Cross-privilege-boundary data
sampling,” in Proc. of CCS, 2019.
[56] S. Van Schaik, A. Milburn, S. Österlund, P. Frigo, G. Maisuradze,
K. Razavi, H. Bos, and C. Giuffrida, “RIDL: Rogue in-flight data load,”
in Proc. of S&P. IEEE, 2019.
[57] G. Chen, S. Chen, Y. Xiao, Y. Zhang, Z. Lin, and T. H. Lai, “Sgxpectre:
Stealing Intel secrets from SGX enclaves via speculative execution,” in
Proc. of EuroS&P, 2019.
[58] M.-W. Shih, S. Lee, T. Kim, and M. Peinado, “T-sgx: Eradicating
controlled-channel attacks against enclave programs,” in Proc. of NDSS,
2017.
[59] O. Oleksenko, B. Trach, R. Krahn, M. Silberstein, and C. Fetzer, “Varys:
Protecting SGX enclaves from practical Side-Channel attacks,” in Proc.
of USENIX ATC, 2018.
[60] C. Dwork, N. Lynch, and L. Stockmeyer, “Consensus in the presence
of partial synchrony,” J. ACM, vol. 35, no. 2, pp. 288–323, 1988.
[61] M. Castro and B. Liskov, “Practical Byzantine fault tolerance,” in Proc.
of OSDI, 1999.
[62] M. Yin, D. Malkhi, M. K. Reiter, G. G. Gueta, and I. Abraham,
“HotStuff: BFT consensus with linearity and responsiveness,” in Proc.
of PODC, 2019.
[63] M. M. Jalalzai, J. Niu, C. Feng, and F. Gai, “Fast-HotStuff: A fast and
robust BFT protocol for blockchains,” IEEE Trans. Dependable Secure
Comput., vol. 21, no. 4, pp. 2478–2493, 2024.
[64] F. Gai, J. Niu, I. Beschastnikh, C. Feng, and S. Wang, “Scaling
blockchain consensus via a robust shared mempool,” in Proc. of ICDE,
2023.
[65] M. Castro and B. Liskov, “Practical Byzantine fault tolerance,” in Proc.
of OSDI, 1999.
[66] M. Clavel, F. Durán, S. Eker, P. Lincoln, N. Martı́-Oliet, J. Meseguer, and
C. Talcott, All About Maude: A High-Performance Logical Framework:
How to Specify, Program and Verify Systems in Rewriting Logic.
Springer, 2007.
[67] R. Bobba, J. Grov, I. Gupta, S. Liu, J. Meseguer, P. C. Ölveczky, and
S. Skeirik, “Survivability: design, formal modeling, and validation of
cloud storage systems using maude,” Assured cloud computing, pp. 10–
48, 2018.
[68] S. Liu, H. Duan, L. Heimes, M. Bearzi, J. Vieli, D. Basin, and A. Perrig,
“A formal framework for end-to-end dns resolution,” in SIGCOMM ’23.
ACM, 2023, p. 932–949.
[69] L. Chuat, M. Legner, D. A. Basin, D. Hausheer, S. Hitz, P. Müller, and
A. Perrig, The Complete Guide to SCION - From Design Principles
to Formal Verification, ser. Information Security and Cryptography.
Springer, 2022.
[70] S. Liu, J. Meseguer, P. C. Ölveczky, M. Zhang, and D. A. Basin, “Bridging the semantic gap between qualitative and quantitative models of
distributed systems,” Proc. ACM Program. Lang., vol. 6, no. OOPSLA2,
pp. 315–344, 2022.
[71] “Apache ZooKeeper,” https://zookeeper.apache.org/, retrieved September 2025.
[72] “SGX data center attestation primitives,” https://github.com/intel/
SGXDataCenterAttestationPrimitives, retrieved September 2025.

[73] F. Gai, A. Farahbakhsh, J. Niu, C. Feng, I. Beschastnikh, and H. Duan,
“Dissecting the performance of chained-BFT,” in Proc. of ICDCS, 2021.
[74] S. Nakamoto, “Bitcoin: A peer-to-peer electronic cash system,” Working
Paper, 2008.
[75] “Microsoft azure confidential ledger,” https://learn.microsoft.com/en-us/
azure/confidential-ledger/overview, retrieved September 2025.
[76] E. Androulaki, A. Barger, V. Bortnikov, C. Cachin, K. Christidis, A. D.
Caro, D. Enyeart, C. Ferris, G. Laventman, Y. Manevich, S. Muralidharan, C. Murthy, B. Nguyen, M. Sethi, G. Singh, K. Smith,
A. Sorniotti, C. Stathakopoulou, M. Vukolic, S. W. Cocco, and J. Yellick,
“Hyperledger Fabric: A distributed operating system for permissioned
blockchains,” in Proc. of EuroSys, 2018.
[77] J. Behl, T. Distler, and R. Kapitza, “Hybrids on Steroids: SGX-based
high performance BFT,” in Proc. of EuroSys, 2017.
[78] J. Liu, W. Li, G. O. Karame, and N. Asokan, “Scalable Byzantine
consensus via hardware-assisted secret sharing,” IEEE Transactions on
Computers, vol. 68, pp. 139–151, 2019.
[79] J. Zhang, J. Gao, K. Wang, Z. Wu, Y. Li, Z. Guan, and Z. Chen,
“TBFT: Efficient Byzantine fault tolerance using Trusted Execution
Environment,” in Proc. of ICC, 2022.
[80] J. Decouchant, D. Kozhaya, V. Rahli, and J. Yu, “Damysus: Streamlined
BFT consensus leveraging trusted components,” in Proc. of EuroSys,
2022.
[81] ——, “Oneshot: View-adapting streamlined BFT protocols with Trusted
Execution Environments,” in Proc. of IPDPS, 2024.
[82] F. Schuster, M. Costa, C. Fournet, C. Gkantsidis, M. Peinado, G. MainarRuiz, and M. Russinovich, “VC3: Trustworthy data analytics in the cloud
using sgx,” in Proc. of S&P, 2015.
[83] D. L. Quoc and C. Fetzer, “SecFL: Confidential federated learning using
TEEs,” arXiv preprint, 2021.
[84] A. Martin, C. Lian, F. Gregor, R. Krahn, V. Schiavoni, P. Felber,
and C. Fetzer, “ADAM-CS: Advanced asynchronous monotonic counter
service,” in Proc. of DSN, 2021.
[85] W. Wang, J. Niu, M. K. Reiter, and Y. Zhang, “Formally verifying a
rollback-prevention protocol for TEEs,” in Proc. of FORTE, 2024.
[86] G. Kaptchuk, I. Miers, and M. Green, “Giving state to the stateless:
Augmenting trustworthy computation with ledgers,” in Proc. of NDSS,
2019.
[87] G. A. Agha, ACTORS - a model of concurrent computation in distributed
systems, ser. MIT Press series in artificial intelligence. MIT Press, 1990.
[88] S. Liu, P. C. Ölveczky, M. Zhang, Q. Wang, and J. Meseguer, “Automatic
analysis of consistency properties of distributed transaction systems in
maude,” in TACAS 2019, ser. LNCS, vol. 11428. Springer, 2019, pp.
40–57.
[89] L. Ouyang, X. Sun, R. Tang, Y. Huang, M. Jivrajani, X. Ma, and T. Xu,
“Multi-grained specifications for distributed system model checking and
verification,” in Proceedings of the Twentieth European Conference on
Computer Systems, ser. EuroSys ’25. ACM, 2025, p. 379–395.
[90] S. Liu, M. R. Rahman, S. Skeirik, I. Gupta, and J. Meseguer, “Formal
modeling and analysis of cassandra in maude,” in ICFEM 2014, ser.
LNCS, vol. 8829. Springer, 2014, pp. 332–347.
[91] J. Schvimer, A. J. J. Davis, and M. Hirschhorn, “extreme modelling in
practice,” Proc. VLDB Endow., vol. 13, no. 9, pp. 1346–1358, 2020.
[92] S. Ghasemirad, S. Liu, C. Sprenger, L. Multazzu, and D. Basin, “Veriso:
Verifiable isolation guarantees for database transactions,” Proc. VLDB
Endow., vol. 18, no. 5, p. 1362–1375, Aug. 2025.

A PPENDIX
A. Supplementary Background
1) Trusted Counter and Extensions: Trusted counter is
the most representative and common method for rollback
prevention. Generally, there are two types of counters:
hardware-based and software-based. The first includes SGX
counter [42], TPM counter [84], and TPM NVRAM [14], [54],
[15]. Hardware-based counters usually have poor performance,
i.e., long latency (e.g., tens of milliseconds) for read and write
operations and limited write cycles [12], [13].
The second is virtual counters, which can be implemented
by a single-write multiple-read register [13], [43], [85] or an

15

append-only ledger [86], [19]. The former includes ROTE [12]
and Narrator [13], which adopt a two-phase broadcast protocol. The latter can be realized by blockchain [86] or CFT
consensus as adopted in Nimble [19]. However, using them
in confidential BFT consensus protocols introduces several
communication steps. In this paper, we use Narrator-Pro, i.e., a
single-write multiple-read register [43], as the software-based
counters. Despite their high costs, we employ counters only
for infrequently updated metadata, thereby avoiding protocol
overhead for log commitment.
Inc-store consistency dilemma. There are two fundamental
operations when using a trusted counter: incrementing the
counter and sealing the state. Because these operations cannot
be executed atomically, their sequence leads to two distinct
patterns.
• Inc-then-store pattern [12], [13], [43], [4]. In this approach,
the counter is incremented before the state is sealed. This
guarantees that no previously sealed state can be replayed,
since the counter always moves forward. However, if a
crash occurs after the counter has been incremented but
before the state is sealed, the counter and state become
permanently inconsistent, making recovery impossible. From
the recovering node’s perspective, a mismatch between the
counter value and the binding counter in the sealed state is
indistinguishable from either a benign crash or a deliberate
rollback attack.

B. Additional Evaluation
Throughput vs. Latency. Fig. 6 illustrates the latency of
the C HIMERA-B and its counterparts with increasing throughput until system saturation in both LAN and WAN deployments. With 10 faulty nodes, C HIMERA reaches maximum
throughputs of 135.8 kTPS in LAN and 22.6 kTPS in WAN,
closely matching or surpassing the best performance among all
counterparts. Notably, in LAN, C HIMERA-B even outperforms
Braft-DR, as asynchronous disk writes reduce persistence bottlenecks. Overall, the results confirm that C HIMERA introduces
negligible overhead to log consensus and can even enhance
performance in certain settings.
500
Chimera-B
Braft-DR
Braft-TC
Braft-RFT

100
80
60

Latency (ms)

Latency (ms)

120

40
20

Chimera-B
Braft-DR
Braft-TC
Braft-RFT

400
300
200
100

0
0.0

2.5

5.0

7.5

10.0

12.5

0

5

Throughput (wTPS)

(a) f = 10, LAN

10

20

15

Throughput (kTPS)

(b) f = 10, WAN

Fig. 6: Throughput vs. Latency of C HIMERA-B and its counterparts.

Shutdown
Restart
Recovery Completed

100

20

50

15

00

25

50

75

100 125 150 175

Time Window (s)

Fig. 7: The fault recovery process of Braft-RC.

16

Available Node #

Throughput of Braft-RC under Recovering Faults. Fig. 7
shows the throughput variation during the recovery of faulty
nodes in Braft-RC. The throughput behavior at the shutdown
point is the same as that observed in C HIMERA-B. These
effects follow the same reasoning as discussed earlier and are
not elaborated here.
A key difference in Braft-RC is that recovery is realized
through reconfiguration. Newly added nodes do not reload logs
from disk but instead start synchronizing directly from the
leader. During this synchronization phase, replication traffic
competes with ongoing client requests, which slows down
replication processing and temporarily reduces throughput
until the synchronization completes.

Throughput (kTPS)

• Store-then-inc pattern [15], [14]. The state is sealed first,
and then the counter is incremented. This avoids unrecoverable crashes, since the sealed state always exists even if the
counter increment fails. However, this introduces a rollback
window: an adversary can seal multiple different states under
the same counter value and later replay an old one. After a
reboot, the node cannot determine which state with the same
counter value is most recent, allowing its state to be rolled
back.
2) Restricted Faults: C HIMERA assumes at most f nodes
may fail concurrently. Without this assumption, the system
may gradually lose liveness as no recovering leader can
recover its log from collecting f +1 replies. Yet this limitation
is not unique to our work. Diskless CFT protocols without
stable storage, such as VR [45] and variants of Paxos [46],
[47], also share this constraint (no more than f crashed nodes
concurrently). Moreover, all BFT protocols have a security
threshold f . An adversary compromising more than f nodes
would disrupt system correctness. This also holds true for
C HIMERA.
3) Customized CFT Consensus Protocols: Except for rollback issues, Wang et al. [4] also identify several safety and
liveness violations of directly porting the Raft protocol into
TEEs. To address these violations, Wang et al. propose several
countermeasures, including file encryption, network encryption and authentication, and malicious leader detection. In this
paper, we focus on rollback-resilient recovery and so assume
a customized CFT protocol with the above countermeasures
running within TEEs. In other words, the customized CFT

protocol can guarantee safety and liveness properties without
rollback attacks.

10

TABLE IV: Model checking results.
Property

Metric

3 Nodes

3 Nodes w/ 2 reboots

5 Nodes

P1

#States
Time

/
557s

2,313,191 [45]
1,174s

2,200,414[26]
4,209s

P2

#States
Time

/
1,294s

2,691,520 [45]
2,883s

2,652,353 [26]
9,814s

P3

#States
Time

/
1,145s

2,691,520 [45]
3,274s

2,652,353 [26]
10,213s

P4

#States
Time

/
1,146s

2,691,520 [45]
3,369s

2,652,353 [26]
10,469s

properties:
• P1. Election Safety: at most one leader can be elected in
any epoch.
• P2. Leader Completeness: once a log entry is committed
by a leader, any subsequent leader contains this entry.
• P3. Recovery Liveness: a node undergoing reboot eventually
completes its recovery procedure.
• P4. Protocol Liveness: every client request is eventually
committed as a log entry.
Table IV shows the model checking results for three cases:
three nodes with one reboot, three nodes with two reboots,
and five nodes. Note that the state space grows rapidly with
additional nodes, making exhaustive verification infeasible
within a reasonable time, a well-recognized challenge in the
formal verification of distributed protocols [88], [89], [90],
[91], [92]. To address this, we utilize Maude’s bounded search,
which explores the state space with depth limits (shown in
[square brackets] in Table IV). This enables model checking
up to a specified bound. Within the explored bounds, Maude
reports no counterexamples to the four properties.

C. Formal Modeling and Analysis
Our formal specification of C HIMERA-B consists of approximately 910 LoC in Maude.2 Our modeling follows Agha’s
actors paradigm [87]. Specifically, nodes are modeled as
actors, and their communication is captured through message
passing. Upon receiving a message, a node may update its
local state and possibly generate new messages. The overall
system evolves through such message-triggered transitions.
We verify C HIMERA-B under TEE rollbacks using lineartemporal-logic (LTL) model checking, focusing on four key

2 The Maude specification is available at https://github.com/Artifacts2026/
CHIMERA/MaudeSpec.

17

