# Paxos architecture classification

WPaxos classifies non-Byzantine Paxos-family protocols by the placement of
leadership and quorum work. The classification concerns the ordering path; it
does not state a durability, recovery, or reconfiguration guarantee by itself.

| Family | Ordering and ownership | Examples in the classification |
| --- | --- | --- |
| Single leader | One node drives agreement for the group. A stable leader avoids repeated phase 1 work, while a leader change repeats it. | Multi-Paxos, Raft; Mencius rotates the driving role. |
| Multi-leader | Several leaders process separate conflict domains. Commands within a domain are ordered; commands across domains need not form one global order. | M2Paxos, ZooNet. |
| Multi-leader, multi-quorum | Leaders also choose quorum sets near their workloads. Safety relies on the required cross-phase intersection between those sets. | WPaxos. |
| Hierarchical | A higher-level leader or quorum assigns ownership or configurations to lower-level groups. | WanKeeper, Vertical Paxos. |
| Leaderless | A replica can attempt agreement for a command without permanent ownership. Conflicts require dependency resolution or an extra round. | EPaxos. |

WPaxos places EPaxos in the leaderless family because an EPaxos replica can
opportunistically propose a non-interfering command. It contrasts this with
WPaxos, where leaders own object partitions and use locality-aware phase-2
quorums; phase 1 transfers ownership when an object is stolen.

The classification is useful for comparison, not as a proof that systems in a
row share the same safety conditions. In particular, UPaxos and uVRR should be
analysed through their configuration-transition and quorum-intersection
invariants rather than assigned a safety property solely from this table.

## Source

This is a faithful reading of section II-B and figure 2 of Ailidani, Charapko,
Demirbas, and Kosar, *WPaxos: Wide Area Network Flexible Consensus*, arXiv
1703.08905v4 (2019). The local single-column reading text is kept at
`research/literature/wpaxos/1703.08905v4-readaloud.md`.
