# Architectural classes of Paxos-based consensus

WPaxos classifies non-Byzantine consensus architectures by how they assign command progress to leaders and acceptors. The categories describe where ordering responsibility sits and how commands may proceed concurrently; they are not a ranking of correctness or performance. The classification is from Ailijiang et al., *WPaxos: Wide Area Network Flexible Consensus*, §II-B and Figure 2 ([arXiv:1703.08905](https://arxiv.org/abs/1703.08905)).

| Class | Architectural rule | Examples in WPaxos | Ordering and concurrency |
|---|---|---|---|
| Single-leader | One leader drives progress for the system. | Multi-Paxos; Raft | A single total order is straightforward to maintain; the leader can become a communication and processing bottleneck. |
| Multi-leader | Different leaders handle distinct conflict domains. A leader may also accept proposals for other domains. | M²Paxos; ZooNet; Mencius is discussed as a rotating-leader design | Commands in separate domains can progress concurrently. Commands within a domain retain an order; the whole system need not have one total order. |
| Multi-leader, multi-quorum | Multiple leaders handle separate domains, and each may use its own phase quorums, subject to the required cross-phase intersection. | WPaxos | Localised phase-2 quorums can reduce wide-area communication. WPaxos assigns object ownership dynamically through phase 1, then commits that object's updates through phase 2. |
| Hierarchical | Leaders or quorums form levels; a higher-level coordinator assigns or manages work for lower-level leaders. | WanKeeper; Vertical Paxos | The upper level coordinates ownership or configuration. Vertical Paxos's master manages configurations and does not process ordinary client commands. |
| Leaderless | Any replica may initiate progress for a command; there is no fixed partition of conflict domains among leaders. | EPaxos | Non-interfering commands may commit concurrently. Interfering commands acquire dependencies and may need the slower path to agree on order. |

## Reading the classification

The dimensions are (1) whether a stable, shared leader exists, (2) whether concurrent leaders own separate conflict domains, (3) whether phase quorums differ by leader, and (4) whether coordination is flat or hierarchical. The rows are the paper's five named families; the dimensions explain why the families differ.

WPaxos is in the multi-leader, multi-quorum class. It partitions ownership by object, but ownership can move: a leader uses phase 1 to take an object and then uses its zone-local phase-2 quorum until another leader takes the object. This means its multi-leader classification does not imply fixed sharding.

EPaxos is the leaderless example. “E-Paxos” in the paper and in discussion refers to **Egalitarian Paxos**, introduced in *There Is More Consensus in Egalitarian Parliaments* by Moraru, Andersen, and Kaminsky (SOSP 2013, [DOI: 10.1145/2517349.2517350](https://doi.org/10.1145/2517349.2517350)). Replicas may initiate commands; the protocol records dependencies between interfering commands so that replicas can execute them consistently.

## Scope of the terms

“Leaderless” means no permanently distinguished leader is required to initiate every command. EPaxos still has a command-local proposer/leader role while that command is being proposed. “Multi-leader” means multiple leaders make progress concurrently on disjoint conflict domains; it does not by itself specify the quorum geometry. WPaxos adds per-leader flexible quorums as a separate architectural property.

## Sources

- Ailijiang, Charapko, Demirbas, and Kosar. “WPaxos: Wide Area Network Flexible Consensus.” arXiv:1703.08905v4, 2019. Sections II-A–B, especially Figure 2.
- Moraru, Andersen, and Kaminsky. “There Is More Consensus in Egalitarian Parliaments.” SOSP 2013. [Author-hosted paper page](https://www.cs.cmu.edu/~dga/papers/epaxos-sosp2013-abstract.html).
