                                                                                                                                     1




                          Exceeding Virtual Stable Storage
                             on Commodity Hardware
                                                               Simon Massey




    Abstract—Virtual stable storage repairs the diskless crash        counterexample is then unrepresentable rather than ﬁltered:
recovery of Viewstamped Replication Revisited by replacing            there is no identity left to cast it. Nothing about that argument
each write to stable storage with a write to a quorum of              requires a fence to survive a crash, because the voter that could
processes: a push round-trip inserted into the fencing path of
the transformed protocol, paid once per fenced action and again       forget the fence does not survive the crash either. The fence
at every recovery. This paper presents the alternative reading of     and the voter share a lifetime, and the fencing tax — disk
Unbounded Viewstamped Replication Revisited (uVRR). Read              write, push round-trip, or hot-path ﬂush — is retired with the
as a repair of Viewstamped Replication it looks like the heavier      voter.
construction; read as a design point for agreement over small            This paper states the design point in storage terms and prices
metadata on commodity hardware it exceeds the virtual-stable-
storage construction it is measured against. uVRR makes a crash       it against the construction it exceeds. Section II prices virtual
ﬁnal for the protocol identity: the fence and the voter share a       stable storage honestly: its cost is not disk trafﬁc but a syn-
lifetime, so no quorum round is spent carrying a fence across         chronous quorum round-trip inserted into the fencing path, per
a crash, and no disk ﬂush sits on the replication path. Local         fence, forever. Sections III and IV describe uVRR’s identity
durable writes survive only at lifecycle boundaries — a clean         ﬁnality and the local storage discipline that distinguishes a
stop, a boot — where a TigerBeetle-derived superblock discipline
of spaced direct copies and a four-state marker machine lets the      clean stop from a crash on commodity hardware. Section V
boot decide clean from dirty with local sector reads alone. The       sets the ledger; Section VI states the soundness conditions;
design retires a cost the literature has moved between disk and       Section VII states where the claim does not hold.
network since 1988 without removing it.
   Index Terms—Crash-stop, diskless recovery, stable storage,             II. T HE COST MODEL OF VIRTUAL STABLE STORAGE
strong consistency, Viewstamped Replication.
                                                                    Virtual stable storage is a general transformation: any pro-
                                                                 tocol correct in the crash-stop or crash-recovery-with-stable-
                       I. I NTRODUCTION                          storage model is transformed into one correct in the diskless
   The fencing of a replicated view has been paid for three crash-recovery model by replacing each write to stable storage
times over, in three currencies, and the literature records each with an acquisition of a quorum [3]. The write terminates when
payment as progress. The 1988 Viewstamped Replication of replies arrive from a strict majority; the receiving processes
Oki and Liskov paid it in disk: a replica records its view fence add the written value to their volatile sets; a crash vector
on local stable storage before its report is released [1]. The per process detects when a replying process has crashed and
2012 Viewstamped Replication Revisited (VRR) of Liskov and recovered, and stale replies are discarded and the request
Cowling declined to pay, keeping the fence volatile; Michael et repeated. Recovery is the same acquisition with an empty
al. showed in 2016 that the corresponding recovery is unsafe, value, and a recovering process is not operational — may not
because a replica can forget a view-change commitment it has resume its protocol — until its quorum round completes.
already sent and vote again in a stale view [3]. Their repair,      Three properties of this construction ﬁx its price.
virtual stable storage, pays the fence in network: every write      The fence costs a push round-trip, per fence, forever.
to stable storage becomes a write to a quorum of processes, Whatever a protocol would persist, the transformed protocol
a push round-trip on the fencing path of the transformed persists by synchronous communication with a majority. For
protocol [3]. The TigerBeetle production design pays it in Viewstamped Replication the persisted item is the view fence:
disk again, with rigour: four spaced superblock copies, a hash- the 1988 disk write that precedes the release of a view-change
chained sequence, and a double journal write, ﬂushed on the report [1]. Under the transformation that write becomes a
request path of a ﬁnancial ledger [6]. Each design moves the quorum round on the fencing path of every view change at
cost. None removes it.                                           every replica. The normal path stays as cheap as VRR’s; the
   Unbounded Viewstamped Replication Revisited (uVRR) fencing path, the path that must complete before a new view
removes it, and the removal is easy to miss because the may form, carries the tax. Under churn the round lengthens:
construction is usually described as a repair of VRR [8]. A replies from processes that crash mid-exchange are discarded
crash is ﬁnal for the protocol identity: the crashed identity and the acquisition is retried.
never resumes, the process rejoins under a fresh identity as        Recovery costs a push round-trip before the process may
a non-voting standby, and voting authority arrives through vote. A crashed process returns only by acquiring a quorum,
committed membership changes. The unsafe vote of the VRR and only operational processes reply. The recovering process
                                                                 contributes no vote during its recovery, exactly as a uVRR
  Author draft. Correspondence: simon.massey@stenographer.cloud.


                                        RESEARCH CHECKPOINT — 19 SEPTEMBER 2026 — 20260919-3a05438
                                                                                                                                              2



            1988                          2012                         2016                        2020s                       2026

 VR                            VRR                            virtual stable storage      TigerBeetle                uVRR
  fence on local disk           fence removed; recovery       fence as a quorum round     fence on disk, ﬂushed on   fence retired with the
                                unsafe                                                    the request path           voter

Fig. 1. The fencing cost moved, never removed. Time is not to scale.



standby contributes none during its catch-up; the two designs               a crash is the only question the local disk ever answers, and
are at parity here, and the parity is worth stating because it              Section IV prices the answer: bounded local writes at the stop
isolates the difference, which is the fencing tax above.                    and at the boot, and in the common case a single local read.
   Durability is the liveness of the quorum. The store is
memory only. If a majority is ever simultaneously down                                     IV. T HE STORAGE DISCIPLINE
or recovering, no process can acquire a quorum again: no                       The hardware assumption is commodity: one local solid-
write completes and no recovery terminates, ever after [3]. To              state block device, direct input/output, no power-loss-
keep the fencing round cheap the quorum must sit within a                   protection guarantees. The device may lose writes that were
datacentre; to keep the store alive that quorum must never fail             not ﬂushed, tear a sector, and return stale or corrupt data; a
together. The construction buys freedom from disk failure by                checksum detects the corruption, and a per-copy ﬁeld detects
concentrating the failure domain into quorum liveness, and the              the misdirected read. Misdirected writes across copies are
commodity intra-datacentre network is what makes the round                  outside the model, as in the design this one derives from [6].
affordable at all [7].
   None of this is a criticism: as a general transformation with
proofs of persistence and termination, virtual stable storage               A. The superblock
is the reference design for diskless crash recovery, and its                   The superblock discipline derives from TigerBeetle,
counterexample is what makes VRR’s recovery precise enough                  slimmed to the metadata problem [6], [9]. Four copies of one
to exceed. The claim here is narrower and stronger: for                     header sit at spaced offsets in a superblock zone, each written
agreement over small metadata on commodity hardware, none                   independently through the direct path. A 128-bit checksum
of the three payments above is necessary.                                   covers the header; the copy ﬁeld and the lifecycle marker are
                                                                            masked out of it, so all four copies of one sequence share one
               III. C RASH - STOP REINCARNATION                             checksum. A sequence number orders the headers and a parent
                                                                            checksum hash-chains each header to its predecessor. Reads
   uVRR retains VRR’s volatile normal path and its two-phase
                                                                            resolve a working set of copies by ﬂexible quorum: two of
view change unchanged [2], [8]. What changes is the meaning
                                                                            four to open, three of four to verify, with the sequence chain
of a crash. A crashed identity never resumes execution. The
                                                                            ordering candidates before markers are consulted.
process that replaces it joins under a fresh identity with voting
weight zero: a standby that cannot form part of any quorum. Its
catch-up — a quorum read and state transfer — runs outside                  B. The marker machine
the voting protocol while it cannot vote, and its promotion to                 Four ordered states distinguish a clean stop from a crash:
voting weight is itself a committed membership change, organ-               stopping, written to all four copies when the stop command
ised across the conﬁguration boundary by leader overlap [5],                lands and sending has ceased; stopped, written to all four
[8]. Messages sent before the crash remain messages of the old              copies after the host’s drain has completed; restarting, written
identity; the mechanised development proves that a superseded               at a boot that ﬁnds the drain proven, continuing the identity;
identity casts no vote in any later conﬁguration [8], [9].                  joining, written at a boot that does not, bumping the identity
   The consequence for the fencing question is direct. VRR’s                exactly once. Every transition is a new set of copies —
recovery had to restore two things at once: the replica’s data              sequence advanced, parent chained — and the uniform four-
and the commitments of its identity — the view fences it had                copy write repairs every disagreeing copy. The drain sits
already sent. Virtual stable storage restores both by repairing             strictly between the two stop writes: the membership log is
the voter’s memory. uVRR restores the data and declines to                  ﬂushed as hash-chained data slots followed by one checkpoint
restore the identity: the commitments of the crashed identity               block carrying the terminal checksum and sequence, the data-
die with it, and the replacement has no commitments to honour               then-headers discipline of the design this derives from, paid
because it has sent nothing. The fence that 1988 wrote to disk              only on a clean stop and never on the replication path [6], [9].
and 2016 wrote to a quorum is, in uVRR, never written at                    The marker order is the drain’s proof: a stopped copy vouches
all. There is nothing to remember, because there is no one left             for the log beneath it, and stopping vouches for nothing.
who must remember it.
   A cleanly stopped process is the exception that prices the               C. The boot verdict
rule. A process that drains and stops deliberately may resume
under its existing identity, because its protocol state was                    The boot never consults the network. It reads local copies,
preserved by the drain itself. Distinguishing that case from                resolves the working set by sequence, and reads the verdict off
                                                                            the markers of the winning set: two of four stopped continues

                                          RESEARCH CHECKPOINT — 19 SEPTEMBER 2026 — 20260919-3a05438
                                                                                                                               3



the identity; anything else bumps it. Where the copies read       checksum and cannot fork the set of copies. A marker byte
agree — the common case, a clean stop left no torn transition     that is not a written state decodes under a valid checksum but
— the ﬁrst copy read settles the verdict, one local sector read   never counts as stopped.
that answers “did I drain?”. A torn transition engages the
                                                                  Proposition 3 (Boot verdict). The boot continues the identity
two-of-four rule; a verdict drawn from a superseded sequence
                                                                  if and only if the working set of copies, resolved at the open
cannot produce an unsafe continuation, because the identity it
                                                                  threshold over the hash-chained sequence, holds at least two
would continue is excluded from every later quorum: the worst
                                                                  stopped markers; otherwise the identity is bumped exactly
outcome is a refused join and a fresh reincarnation, never a
                                                                  once, checked against exhaustion, and the process joins. The
stale vote. Four small reads of one local ﬁle are the general
                                                                  verdict never consults the network.
case; a quorum round is not any case.
   The disk’s whole jurisdiction is therefore two questions:      Proposition 4 (Single-read condition). Where the copies read
who am I? and did I drain?. It is never asked what was            agree, the ﬁrst copy read settles the drain verdict. Dis-
committed? — that answer lives in quorum memory, as VRR           agreement engages Proposition 3. A verdict drawn from a
intended, and it costs nothing local.                             superseded sequence cannot produce an unsafe continuation:
                                                                  the identity it would continue is excluded from every later
                       V. T HE LEDGER                             quorum, so the worst outcome is a refused join and a fresh
   Table I prices the ﬁve designs on the same rows.               reincarnation.
   Three rows of the table carry the argument.                  Proposition 5 (Identity exclusion). Promotion to voting
   The fence. uVRR is the only design whose fence is unpaid weight is a committed membership change, and a superseded
rather than moved: 1988 pays a disk write, 2016 pays a push identity casts no vote in any later conﬁguration; the property
round-trip, TigerBeetle pays a ﬂush on the request path, and is mechanised in the companion development [8], [9].
uVRR pays nothing because the identity that would forget the
fence is retired by the crash that would lose it.                  The one host contract is Proposition 1’s premise: the drain
   Recovery. Virtual stable storage and uVRR both return        completes    before the second stop write is issued. Every other
through the network, and both withhold the vote until the       condition   is enforced by the construction — by the checksum
return completes; the costs differ in kind, a quorum round      mask,   the  sequence  chain, the ﬂexible read quorum, and the
against a state transfer plus committed promotion batches. The  membership     protocol — not by the operator.
difference that persists is not the recovery cost but the tax
that survives recovery: virtual stable storage keeps paying its                     VII. S COPE AND LIMITS
fencing round at every subsequent view change, and uVRR            The claim of exceeding is scoped, and the boundaries are
has nothing left to pay.                                        part of the result.
   Durability. uVRR shares the quorum-memory envelope of           Generality belongs to virtual stable storage. It transforms
VRR and virtual stable storage: a simultaneous full-cluster any crash-stop or crash-recovery protocol; uVRR is one proto-
loss forfeits what the quorum held. It does not claim the col, redesigned so that the problem the transformation solves
TigerBeetle envelope, in which the log survives on disk; meta- does not arise. A system that cannot retire identities — one
data agreement and a ﬁnancial ledger are different problems, whose correctness rests on continuity of a single identity
and the ledger pays for its envelope on every batch. What — cannot use this route, and for such systems the 2016
uVRR borrows from TigerBeetle is not the envelope but construction remains the reference.
the rigour — spaced copies, the shared checksum, the hash-         The degraded window is parity, not advantage. With three
chained sequence, the data-then-headers ordering — applied voters and one crash, two votes remain until the standby is
only at lifecycle boundaries, where its cost is bounded and promoted; the recovering process of virtual stable storage
never on the request path.                                      equally cannot vote until its quorum round completes. Neither
                                                                design is faster to restore fault tolerance by construction; they
                 VI. S OUNDNESS CONDITIONS                      differ in what they charge afterwards.
   The claims of Section IV rest on ﬁve conditions, each           The machinery moved into reconﬁguration. Identity ﬁnality
of which is a property of the construction rather than an       is bought  with forced weight sequences, committed promotion,
assumption about failures.                                      and  leader  overlap — machinery that virtual stable storage, an
                                                                orthogonal layer, does not need. The companion development
Proposition 1 (Drain proof). A copy whose marker is stopped mechanises that machinery’s agreement argument; this paper’s
vouches for the log beneath it: the second stop write is claim is that the machinery is paid for once, at reincarnation,
issued only after the host’s drain has completed, so the rather than per fence, forever.
drain happened strictly between the two stop writes. A single      This paper argues a cost model and reports no measure-
surviving stopped copy therefore proves the drain, however ments. The companion paper states the campaign that prices
torn the remaining copies.                                      rejoin against forced ﬂush and the decision rule by which
Proposition 2 (Torn-marker safety). The marker is masked the diskless latency claim stands or collapses [8]. Until that
out of the shared checksum, so a torn marker write leaves campaign runs, “exceeds” is a property of the ledger of
copies of one sequence in different marker states under one Section V, not of a benchmark.

                                    RESEARCH CHECKPOINT — 19 SEPTEMBER 2026 — 20260919-3a05438
                                                                                                                                                            4



                                                               TABLE I
  T HE LEDGER : WHAT EACH DESIGN PAYS FOR THE VIEW FENCE , FOR RECOVERY, AND FOR DURABILITY. U VRR IS THE ONLY ROW WHOSE FENCE IS
                                                     UNPAID RATHER THAN MOVED .


 Design                 The fence is paid in         Crash recovery                    Disk on the re-      The boot consults       Full-cluster loss
                                                                                       quest path
 VR 1988 [1]            one local disk write per     local disk, then state transfer   no                   its own disk            state survives
                        view
 VRR 2012 [2]           nothing; fence is volatile   unsafe: commitments are           no                   a recovery quorum       quorum memory for-
                                                     lost [3]                                                                       feits
 Virtual stable stor-   one push round-trip per      one quorum round; same iden-      no                   a quorum round          store lost with a ma-
 age [3]                replica per fence            tity                                                                           jority
 TigerBeetle [6]        disk: four copies, double    quorum read of local copies       yes: ﬂush      per   its own disk            state survives
                        journal                                                        batch
 uVRR                   unpaid: retired with the     transfer plus committed pro-      no                   local sectors only      quorum memory for-
                        voter                        motion; new identity                                                           feits



                        VIII. R ELATED WORK                                     [2] B. Liskov and J. Cowling, “Viewstamped replication revisited,” MIT
                                                                                    CSAIL, Tech. Rep. MIT-CSAIL-TR-2012-021, 2012. [Online]. Avail-
   Viewstamped Replication organised replication around                             able: https://hdl.handle.net/1721.1/71763
views and a primary, and fenced views through local stable                      [3] E. Michael, D. R. K. Ports, N. Kr. Sharma, and A. Szekeres, “Providing
storage [1]; VRR removed the storage and separated nor-                             stable storage for the diskless crash-recovery failure model,” University
                                                                                    of Washington, Tech. Rep. UW-CSE-16-08-02, Aug. 25, 2016. [Online].
mal operation, view change, recovery and reconﬁguration [2].                        Available: https://syslab.cs.washington.edu/papers/diskless-tr16.pdf
Michael et al. showed the removal was unsound, exhibited                        [4] R. Alagappan, A. Ganesan, E. Lee, A. Albarghouthi, V. Chidambaram,
the counterexample for VRR and for two further protocols,                           A. C. Arpaci-Dusseau, and R. H. Arpaci-Dusseau, “Protocol-aware
                                                                                    recovery for consensus-based storage,” in USENIX FAST, 2018. [On-
and constructed virtual stable storage as the general repair [3].                   line]. Available: https://www.usenix.org/system/ﬁles/conference/fast18/
Protocol-aware recovery catalogues the wider class of recovery                      fast18-alagappan.pdf
faults in deployed consensus systems and repairs them per                       [5] D. Turner, “Unbounded pipelining in dynamically reconﬁgurable Paxos
                                                                                    clusters,” revision 1A9DBA37, Aug. 14, 2017. [Online]. Available: https:
protocol against storage-level fault models [4]; the present                        //github.com/DaveCTurner/paxos-membership
work differs from both by removing the recovering identity                      [6] TigerBeetle, “Safety,” design documentation, accessed Sep. 19, 2026.
rather than repairing its memory. Turner’s reconﬁguration                           [Online]. Available: https://docs.tigerbeetle.com/about/safety/
                                                                                [7] simbo1905, “The network is faster than the disk,” Apr. 12, 2024,
proof supplies the quorum geometry and the leader overlap                           accessed Sep. 7, 2026. [Online]. Available: https://simbo1905.wordpress.
that organise the replacement schedule [5]. TigerBeetle sup-                        com/2024/04/12/the-network-is-faster-than-the-disk/
plies the storage discipline — spaced copies, ﬂexible read                      [8] S. Massey, “Diskless Viewstamped Replication with Unbounded Crash-
                                                                                    Stop Reincarnation,” companion draft, 2026.
and write quorums, hash-chained sequences, the data-then-                       [9] S. Massey, “uVRR code and proof addendum,” 2026. [Online]. Available:
headers journal — here applied to lifecycle state only [6].                         https://github.com/lua-lunet/uvrr-core
The companion paper presents the protocol, the replacement
schedules and the mechanised development; the artefact carries
the code and the proofs [8], [9].

                          IX. C ONCLUSION
   Since 1988 the fencing of a replicated view has been paid
in disk, left unpaid and found unsafe, paid in network round-
trips, and paid in disk again with rigour. Each design moved
the cost; the literature records each move as an advance, and
each was one. uVRR is the ﬁrst design in the line that does
not move the cost: by making a crash ﬁnal for the protocol
identity, it retires the voter with the fence, and the fence need
never be written down at all. The local disk, demoted to two
questions — who am I? and did I drain? — answers them
at lifecycle boundaries with spaced direct writes and, in the
common case, a single sector read. That is the sense in which
this is not a repair of Viewstamped Replication: for agreement
over small metadata on commodity hardware, it exceeds the
virtual-stable-storage design point that repaired it ﬁrst.

                             R EFERENCES
 [1] B. M. Oki and B. H. Liskov, “Viewstamped replication: A new primary
     copy method to support highly-available distributed systems,” in ACM
     PODC, 1988, pp. 8–17.



                                           RESEARCH CHECKPOINT — 19 SEPTEMBER 2026 — 20260919-3a05438
