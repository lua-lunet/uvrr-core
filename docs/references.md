# uVRR references

The external works the documents and the code cite, in one place, under
stable short identifiers. A citation of the form `[VR-2012]` in a document
or a doc comment names an entry of this file, and the code's reference gate
(`tests/doc_reference_contract.rs`) resolves every citation identifier
against the headings here. A section number following an identifier, as in
"`[VR-2012]` §4.3", names a section of the cited work itself; the gate does
not validate those, the works are not ours to restructure. This file is a
living document: a new citation adds an entry, and an entry is never
renumbered, so a citation once written stays true.

## [VR-2012]

B. Liskov, J. Cowling. *Viewstamped Replication Revisited*. 2012.
<https://hdl.handle.net/1721.1/71763>

## [PMS-2001]

L. Lamport. *Paxos Made Simple*. 2001.
<https://lamport.azurewebsites.net/pubs/paxos-simple.pdf>

## [DISC-2017]

E. Michael, D. R. K. Ports, N. K. Sharma, A. Szekeres. *Recovering
Shared Objects Without Stable Storage*. DISC 2017; the published version
of the 2016 technical report cited as `diskless`, whose Appendix A.1
carries the amnesia counterexample against Viewstamped Replication
Revisited.
<https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.DISC.2017.36>

## [OSDI-2014]

T. S. Pillai et al. *All File Systems Are Not Created Equal: On the
Complexity of Crafting Crash Consistent Applications*. OSDI 2014.
<https://www.usenix.org/conference/osdi14/technical-sessions/presentation/pillai>

## [SOSP-2013]

V. Chidambaram et al. *Optimistic Crash Consistency*. SOSP 2013.
<https://dl.acm.org/doi/10.1145/2517349.2522726>

## [LAMPSON-1979]

B. Lampson, H. Sturgis. *Crash Recovery in a Distributed Data Storage
System*. Xerox PARC, 1979.

## [TURNER-RECONF]

D. Turner. *Paxos membership change*: the weighted three-node example and
the general weighted-majority overlap lemma.
<https://github.com/DaveCTurner/paxos-membership>

## [TIGERBEETLE-REPO]

TigerBeetle. The superblock construction: four checksummed, hash-chained
copies with quorum writes and quorum reads.
<https://github.com/tigerbeetle/tigerbeetle>

## [VW-2017]

S. Massey. *Paxos Voting Weights*: the halve/double and ±1-unit rules, and
standbys at weight zero. 2017.
<https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/>

## [UPAXOS-2016]

S. Massey. *UPaxos: Unbounded Paxos Reconfigurations*: era-indexed
reconfiguration, consecutive-configuration overlap, the casting vote. 2016.
<https://simbo1905.wordpress.com/2016/12/16/upaxos-unbounded-paxos-reconfigurations/>

## [FROWN-2020]

S. Massey. *One More Frown Please: UPaxos Quorum Overlaps*: the frown
operator and the quorum-overlap chain. 2020.
<https://simbo1905.wordpress.com/2020/05/23/one-more-frown-please-upaxos-quorum-overlaps/>

## [UVRR-2026]

S. Massey. *Viewstamped Replication Revisited*: the uVRR motivation and the
TigerBeetle-style durability framing. 2026.
<https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/>

## [NETDISK-2024]

S. Massey. *The Network Is Faster Than the Disk*: the deferred-flush
economic rationale. 2024.
<https://simbo1905.wordpress.com/2024/04/12/the-network-is-faster-than-the-disk/>

## [EWD-1990]

E. W. Dijkstra. *Reasoning about Programs*: the do-loop invariant discipline
in his own notation as written on the lecture whiteboard, the pattern for
proving things about the repetitive construct; credited to C. A. R. Hoare.
1990.
<https://youtu.be/GX3URhx6i2E>
