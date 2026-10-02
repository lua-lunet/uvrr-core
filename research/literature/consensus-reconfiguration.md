# Reconfiguration reading set from Consensus

The following works were selected from the Consensus search results supplied
by the user. They are adjacent research, not a claim that David Turner's
UPaxos paper cites them.

| Work | Reading focus |
| --- | --- |
| Bortnikov et al., *Reconfigurable State Machine Replication from Non-Reconfigurable Building Blocks* | Builds reconfigurable SMR from black-box fixed-membership state machines and uses speculation to maintain progress during changes. The 2015 arXiv paper extends the two-page 2012 PODC publication cited by Turner. |
| Fraga and Alchieri, *Reconfigurable Scalable State Machine Replication* | Changes the degree of parallelism while retaining strong consistency across requests that span partitions. |
| Alchieri et al., *Reconfiguring Parallel State Machine Replication* | Reconfigures the parallel execution degree as workload conflicts change. |
| Whittaker et al., *Matchmaker Paxos* | Separates configuration discovery from the acceptor configuration, aiming for low-latency vertical reconfiguration. The 2020 technical report and 2021 publication describe the same protocol lineage; retain one canonical record and relate the other as a version. |
| Distler, *Byzantine Fault-tolerant State-machine Replication from a Systems Perspective* | Survey of BFT system components, implementation constraints, checkpointing, and recovery. It broadens the fault model beyond the crash-fault Paxos set. |
| *Donut Paxos* | A Consensus result whose record has damaged title typography and no authors. Keep it provisional until the primary source supplies canonical metadata. |
| Clement et al., *It's not a lie if you don't get caught* | The 2026 Gauss paper separates an inner consensus log from a sanitised outer log so membership and protocol evolution can be handled independently. |
| Aublin et al., *The Next 700 BFT Protocols* | Introduces Abstract, an abortable state-machine abstraction for composing BFT protocols. |
| Bessani et al., *State Machine Replication for the Masses with BFT-SMART* | A practical Java BFT-SMR library with modularity, multicore support, and reconfiguration. |

## Reading order

Read Bortnikov, Matchmaker Paxos, and the Gauss paper together for the
relationship between black-box construction, configuration lookup, and
protocol replacement. Then read the parallel-SMR pair for execution
reconfiguration. Read the BFT survey before the BFT-SMART and Abstract papers
so fault-model and implementation terms are fixed before comparing designs.

## Sources

Each entry was fetched from Consensus before this note was written. The
canonical identifiers are retained in the Zotero import material and should be
checked against the primary paper before bibliographic publication.
