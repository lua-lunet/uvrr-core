1

Exceeding Virtual Stable Storage
on Commodity Hardware
Simon Massey

Abstract—Virtual stable storage repairs the diskless crash
recovery of Viewstamped Replication Revisited by replacing
each write to stable storage with a write to a quorum of
processes: a push round-trip inserted into the fencing path of
the transformed protocol, paid once per fenced action and again
at every recovery. This paper presents the alternative reading of
Unbounded Viewstamped Replication Revisited (uVRR). Read
as a repair of Viewstamped Replication it looks like the heavier
construction; read as a design point for agreement over small
metadata on commodity hardware it exceeds the virtual-stablestorage construction it is measured against. uVRR makes a crash
ﬁnal for the protocol identity: the fence and the voter share a
lifetime, so no quorum round is spent carrying a fence across
a crash, and no disk ﬂush sits on the replication path. Local
durable writes survive only at lifecycle boundaries — a clean
stop, a boot — where a TigerBeetle-derived superblock discipline
of spaced direct copies and a four-state marker machine lets the
boot decide clean from dirty with local sector reads alone. The
design retires a cost the literature has moved between disk and
network since 1988 without removing it.

counterexample is then unrepresentable rather than ﬁltered:
there is no identity left to cast it. Nothing about that argument
requires a fence to survive a crash, because the voter that could
forget the fence does not survive the crash either. The fence
and the voter share a lifetime, and the fencing tax — disk
write, push round-trip, or hot-path ﬂush — is retired with the
voter.
This paper states the design point in storage terms and prices
it against the construction it exceeds. Section II prices virtual
stable storage honestly: its cost is not disk trafﬁc but a synchronous quorum round-trip inserted into the fencing path, per
fence, forever. Sections III and IV describe uVRR’s identity
ﬁnality and the local storage discipline that distinguishes a
clean stop from a crash on commodity hardware. Section V
sets the ledger; Section VI states the soundness conditions;
Section VII states where the claim does not hold.

Index Terms—Crash-stop, diskless recovery, stable storage,
strong consistency, Viewstamped Replication.

II. T HE COST MODEL OF VIRTUAL STABLE STORAGE

Virtual stable storage is a general transformation: any protocol correct in the crash-stop or crash-recovery-with-stableI. I NTRODUCTION
storage model is transformed into one correct in the diskless
The fencing of a replicated view has been paid for three crash-recovery model by replacing each write to stable storage
times over, in three currencies, and the literature records each with an acquisition of a quorum [3]. The write terminates when
payment as progress. The 1988 Viewstamped Replication of replies arrive from a strict majority; the receiving processes
Oki and Liskov paid it in disk: a replica records its view fence add the written value to their volatile sets; a crash vector
on local stable storage before its report is released [1]. The per process detects when a replying process has crashed and
2012 Viewstamped Replication Revisited (VRR) of Liskov and recovered, and stale replies are discarded and the request
Cowling declined to pay, keeping the fence volatile; Michael et repeated. Recovery is the same acquisition with an empty
al. showed in 2016 that the corresponding recovery is unsafe, value, and a recovering process is not operational — may not
because a replica can forget a view-change commitment it has resume its protocol — until its quorum round completes.
Three properties of this construction ﬁx its price.
already sent and vote again in a stale view [3]. Their repair,
The fence costs a push round-trip, per fence, forever.
virtual stable storage, pays the fence in network: every write
to stable storage becomes a write to a quorum of processes, Whatever a protocol would persist, the transformed protocol
a push round-trip on the fencing path of the transformed persists by synchronous communication with a majority. For
protocol [3]. The TigerBeetle production design pays it in Viewstamped Replication the persisted item is the view fence:
disk again, with rigour: four spaced superblock copies, a hash- the 1988 disk write that precedes the release of a view-change
chained sequence, and a double journal write, ﬂushed on the report [1]. Under the transformation that write becomes a
request path of a ﬁnancial ledger [6]. Each design moves the quorum round on the fencing path of every view change at
every replica. The normal path stays as cheap as VRR’s; the
cost. None removes it.
Unbounded Viewstamped Replication Revisited (uVRR) fencing path, the path that must complete before a new view
removes it, and the removal is easy to miss because the may form, carries the tax. Under churn the round lengthens:
construction is usually described as a repair of VRR [8]. A replies from processes that crash mid-exchange are discarded
crash is ﬁnal for the protocol identity: the crashed identity and the acquisition is retried.
Recovery costs a push round-trip before the process may
never resumes, the process rejoins under a fresh identity as
a non-voting standby, and voting authority arrives through vote. A crashed process returns only by acquiring a quorum,
committed membership changes. The unsafe vote of the VRR and only operational processes reply. The recovering process
contributes no vote during its recovery, exactly as a uVRR
Author draft. Correspondence: simon.massey@stenographer.cloud.

RESEARCH CHECKPOINT — 19 SEPTEMBER 2026 — 20260919-3a05438

2

1988

2012

2016

2020s

2026

VR

VRR

virtual stable storage

TigerBeetle

uVRR

fence on local disk

fence removed; recovery
unsafe

fence as a quorum round

fence on disk, ﬂushed on
the request path

fence retired with the
voter

Fig. 1. The fencing cost moved, never removed. Time is not to scale.

standby contributes none during its catch-up; the two designs
are at parity here, and the parity is worth stating because it
isolates the difference, which is the fencing tax above.
Durability is the liveness of the quorum. The store is
memory only. If a majority is ever simultaneously down
or recovering, no process can acquire a quorum again: no
write completes and no recovery terminates, ever after [3]. To
keep the fencing round cheap the quorum must sit within a
datacentre; to keep the store alive that quorum must never fail
together. The construction buys freedom from disk failure by
concentrating the failure domain into quorum liveness, and the
commodity intra-datacentre network is what makes the round
affordable at all [7].
None of this is a criticism: as a general transformation with
proofs of persistence and termination, virtual stable storage
is the reference design for diskless crash recovery, and its
counterexample is what makes VRR’s recovery precise enough
to exceed. The claim here is narrower and stronger: for
agreement over small metadata on commodity hardware, none
of the three payments above is necessary.
III. C RASH - STOP REINCARNATION
uVRR retains VRR’s volatile normal path and its two-phase
view change unchanged [2], [8]. What changes is the meaning
of a crash. A crashed identity never resumes execution. The
process that replaces it joins under a fresh identity with voting
weight zero: a standby that cannot form part of any quorum. Its
catch-up — a quorum read and state transfer — runs outside
the voting protocol while it cannot vote, and its promotion to
voting weight is itself a committed membership change, organised across the conﬁguration boundary by leader overlap [5],
[8]. Messages sent before the crash remain messages of the old
identity; the mechanised development proves that a superseded
identity casts no vote in any later conﬁguration [8], [9].
The consequence for the fencing question is direct. VRR’s
recovery had to restore two things at once: the replica’s data
and the commitments of its identity — the view fences it had
already sent. Virtual stable storage restores both by repairing
the voter’s memory. uVRR restores the data and declines to
restore the identity: the commitments of the crashed identity
die with it, and the replacement has no commitments to honour
because it has sent nothing. The fence that 1988 wrote to disk
and 2016 wrote to a quorum is, in uVRR, never written at
all. There is nothing to remember, because there is no one left
who must remember it.
A cleanly stopped process is the exception that prices the
rule. A process that drains and stops deliberately may resume
under its existing identity, because its protocol state was
preserved by the drain itself. Distinguishing that case from

a crash is the only question the local disk ever answers, and
Section IV prices the answer: bounded local writes at the stop
and at the boot, and in the common case a single local read.
IV. T HE STORAGE DISCIPLINE
The hardware assumption is commodity: one local solidstate block device, direct input/output, no power-lossprotection guarantees. The device may lose writes that were
not ﬂushed, tear a sector, and return stale or corrupt data; a
checksum detects the corruption, and a per-copy ﬁeld detects
the misdirected read. Misdirected writes across copies are
outside the model, as in the design this one derives from [6].
A. The superblock
The superblock discipline derives from TigerBeetle,
slimmed to the metadata problem [6], [9]. Four copies of one
header sit at spaced offsets in a superblock zone, each written
independently through the direct path. A 128-bit checksum
covers the header; the copy ﬁeld and the lifecycle marker are
masked out of it, so all four copies of one sequence share one
checksum. A sequence number orders the headers and a parent
checksum hash-chains each header to its predecessor. Reads
resolve a working set of copies by ﬂexible quorum: two of
four to open, three of four to verify, with the sequence chain
ordering candidates before markers are consulted.
B. The marker machine
Four ordered states distinguish a clean stop from a crash:
stopping, written to all four copies when the stop command
lands and sending has ceased; stopped, written to all four
copies after the host’s drain has completed; restarting, written
at a boot that ﬁnds the drain proven, continuing the identity;
joining, written at a boot that does not, bumping the identity
exactly once. Every transition is a new set of copies —
sequence advanced, parent chained — and the uniform fourcopy write repairs every disagreeing copy. The drain sits
strictly between the two stop writes: the membership log is
ﬂushed as hash-chained data slots followed by one checkpoint
block carrying the terminal checksum and sequence, the datathen-headers discipline of the design this derives from, paid
only on a clean stop and never on the replication path [6], [9].
The marker order is the drain’s proof: a stopped copy vouches
for the log beneath it, and stopping vouches for nothing.
C. The boot verdict
The boot never consults the network. It reads local copies,
resolves the working set by sequence, and reads the verdict off
the markers of the winning set: two of four stopped continues

RESEARCH CHECKPOINT — 19 SEPTEMBER 2026 — 20260919-3a05438

3

the identity; anything else bumps it. Where the copies read
agree — the common case, a clean stop left no torn transition
— the ﬁrst copy read settles the verdict, one local sector read
that answers “did I drain?”. A torn transition engages the
two-of-four rule; a verdict drawn from a superseded sequence
cannot produce an unsafe continuation, because the identity it
would continue is excluded from every later quorum: the worst
outcome is a refused join and a fresh reincarnation, never a
stale vote. Four small reads of one local ﬁle are the general
case; a quorum round is not any case.
The disk’s whole jurisdiction is therefore two questions:
who am I? and did I drain?. It is never asked what was
committed? — that answer lives in quorum memory, as VRR
intended, and it costs nothing local.
V. T HE LEDGER

checksum and cannot fork the set of copies. A marker byte
that is not a written state decodes under a valid checksum but
never counts as stopped.
Proposition 3 (Boot verdict). The boot continues the identity
if and only if the working set of copies, resolved at the open
threshold over the hash-chained sequence, holds at least two
stopped markers; otherwise the identity is bumped exactly
once, checked against exhaustion, and the process joins. The
verdict never consults the network.
Proposition 4 (Single-read condition). Where the copies read
agree, the ﬁrst copy read settles the drain verdict. Disagreement engages Proposition 3. A verdict drawn from a
superseded sequence cannot produce an unsafe continuation:
the identity it would continue is excluded from every later
quorum, so the worst outcome is a refused join and a fresh
reincarnation.

Table I prices the ﬁve designs on the same rows.
Three rows of the table carry the argument.
Proposition 5 (Identity exclusion). Promotion to voting
The fence. uVRR is the only design whose fence is unpaid weight is a committed membership change, and a superseded
rather than moved: 1988 pays a disk write, 2016 pays a push identity casts no vote in any later conﬁguration; the property
round-trip, TigerBeetle pays a ﬂush on the request path, and is mechanised in the companion development [8], [9].
uVRR pays nothing because the identity that would forget the
The one host contract is Proposition 1’s premise: the drain
fence is retired by the crash that would lose it.
completes
before the second stop write is issued. Every other
Recovery. Virtual stable storage and uVRR both return
condition
is
enforced by the construction — by the checksum
through the network, and both withhold the vote until the
mask,
the
sequence
chain, the ﬂexible read quorum, and the
return completes; the costs differ in kind, a quorum round
membership
protocol
— not by the operator.
against a state transfer plus committed promotion batches. The
difference that persists is not the recovery cost but the tax
VII. S COPE AND LIMITS
that survives recovery: virtual stable storage keeps paying its
The claim of exceeding is scoped, and the boundaries are
fencing round at every subsequent view change, and uVRR
part of the result.
has nothing left to pay.
Generality belongs to virtual stable storage. It transforms
Durability. uVRR shares the quorum-memory envelope of
VRR and virtual stable storage: a simultaneous full-cluster any crash-stop or crash-recovery protocol; uVRR is one protoloss forfeits what the quorum held. It does not claim the col, redesigned so that the problem the transformation solves
TigerBeetle envelope, in which the log survives on disk; meta- does not arise. A system that cannot retire identities — one
data agreement and a ﬁnancial ledger are different problems, whose correctness rests on continuity of a single identity
and the ledger pays for its envelope on every batch. What — cannot use this route, and for such systems the 2016
uVRR borrows from TigerBeetle is not the envelope but construction remains the reference.
The degraded window is parity, not advantage. With three
the rigour — spaced copies, the shared checksum, the hashchained sequence, the data-then-headers ordering — applied voters and one crash, two votes remain until the standby is
only at lifecycle boundaries, where its cost is bounded and promoted; the recovering process of virtual stable storage
equally cannot vote until its quorum round completes. Neither
never on the request path.
design is faster to restore fault tolerance by construction; they
differ in what they charge afterwards.
VI. S OUNDNESS CONDITIONS
The machinery moved into reconﬁguration. Identity ﬁnality
The claims of Section IV rest on ﬁve conditions, each
is
bought
with forced weight sequences, committed promotion,
of which is a property of the construction rather than an
and
leader
overlap — machinery that virtual stable storage, an
assumption about failures.
orthogonal layer, does not need. The companion development
Proposition 1 (Drain proof). A copy whose marker is stopped mechanises that machinery’s agreement argument; this paper’s
vouches for the log beneath it: the second stop write is claim is that the machinery is paid for once, at reincarnation,
issued only after the host’s drain has completed, so the rather than per fence, forever.
drain happened strictly between the two stop writes. A single
This paper argues a cost model and reports no measuresurviving stopped copy therefore proves the drain, however ments. The companion paper states the campaign that prices
torn the remaining copies.
rejoin against forced ﬂush and the decision rule by which
Proposition 2 (Torn-marker safety). The marker is masked the diskless latency claim stands or collapses [8]. Until that
out of the shared checksum, so a torn marker write leaves campaign runs, “exceeds” is a property of the ledger of
copies of one sequence in different marker states under one Section V, not of a benchmark.
RESEARCH CHECKPOINT — 19 SEPTEMBER 2026 — 20260919-3a05438

4

TABLE I
T HE LEDGER : WHAT EACH DESIGN PAYS FOR THE VIEW FENCE , FOR RECOVERY, AND FOR DURABILITY. U VRR IS THE ONLY ROW WHOSE FENCE IS
UNPAID RATHER THAN MOVED .
Design

The fence is paid in

Crash recovery

Disk on the request path

The boot consults

Full-cluster loss

VR 1988 [1]

one local disk write per
view
nothing; fence is volatile

local disk, then state transfer

no

its own disk

state survives

unsafe: commitments are
lost [3]
one quorum round; same identity
quorum read of local copies

no

a recovery quorum

no

a quorum round

quorum memory forfeits
store lost with a majority
state survives

VRR 2012 [2]
Virtual stable storage [3]
TigerBeetle [6]
uVRR

one push round-trip per
replica per fence
disk: four copies, double
journal
unpaid: retired with the
voter

transfer plus committed promotion; new identity

VIII. R ELATED WORK
Viewstamped Replication organised replication around
views and a primary, and fenced views through local stable
storage [1]; VRR removed the storage and separated normal operation, view change, recovery and reconﬁguration [2].
Michael et al. showed the removal was unsound, exhibited
the counterexample for VRR and for two further protocols,
and constructed virtual stable storage as the general repair [3].
Protocol-aware recovery catalogues the wider class of recovery
faults in deployed consensus systems and repairs them per
protocol against storage-level fault models [4]; the present
work differs from both by removing the recovering identity
rather than repairing its memory. Turner’s reconﬁguration
proof supplies the quorum geometry and the leader overlap
that organise the replacement schedule [5]. TigerBeetle supplies the storage discipline — spaced copies, ﬂexible read
and write quorums, hash-chained sequences, the data-thenheaders journal — here applied to lifecycle state only [6].
The companion paper presents the protocol, the replacement
schedules and the mechanised development; the artefact carries
the code and the proofs [8], [9].

yes: ﬂush
batch
no

per

its own disk
local sectors only

quorum memory forfeits

[2] B. Liskov and J. Cowling, “Viewstamped replication revisited,” MIT
CSAIL, Tech. Rep. MIT-CSAIL-TR-2012-021, 2012. [Online]. Available: https://hdl.handle.net/1721.1/71763
[3] E. Michael, D. R. K. Ports, N. Kr. Sharma, and A. Szekeres, “Providing
stable storage for the diskless crash-recovery failure model,” University
of Washington, Tech. Rep. UW-CSE-16-08-02, Aug. 25, 2016. [Online].
Available: https://syslab.cs.washington.edu/papers/diskless-tr16.pdf
[4] R. Alagappan, A. Ganesan, E. Lee, A. Albarghouthi, V. Chidambaram,
A. C. Arpaci-Dusseau, and R. H. Arpaci-Dusseau, “Protocol-aware
recovery for consensus-based storage,” in USENIX FAST, 2018. [Online]. Available: https://www.usenix.org/system/ﬁles/conference/fast18/
fast18-alagappan.pdf
[5] D. Turner, “Unbounded pipelining in dynamically reconﬁgurable Paxos
clusters,” revision 1A9DBA37, Aug. 14, 2017. [Online]. Available: https:
//github.com/DaveCTurner/paxos-membership
[6] TigerBeetle, “Safety,” design documentation, accessed Sep. 19, 2026.
[Online]. Available: https://docs.tigerbeetle.com/about/safety/
[7] simbo1905, “The network is faster than the disk,” Apr. 12, 2024,
accessed Sep. 7, 2026. [Online]. Available: https://simbo1905.wordpress.
com/2024/04/12/the-network-is-faster-than-the-disk/
[8] S. Massey, “Diskless Viewstamped Replication with Unbounded CrashStop Reincarnation,” companion draft, 2026.
[9] S. Massey, “uVRR code and proof addendum,” 2026. [Online]. Available:
https://github.com/lua-lunet/uvrr-core

IX. C ONCLUSION
Since 1988 the fencing of a replicated view has been paid
in disk, left unpaid and found unsafe, paid in network roundtrips, and paid in disk again with rigour. Each design moved
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

