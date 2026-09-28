# uVRR host notes: discussion points, not obligations

> These notes are discussion points, not obligations. The library is
> sans-IO: it cannot and does not prescribe how a host names its files,
> lays out its disks, or rotates its machines. Your mileage will vary.
> The suggestions here, which discuss what a host might do about leader
> timeouts, are the same grade: discussion, never obligation. Where a
> worked example helps, consult the latest official demos; nothing here is
> kept current, and nothing here is authoritative.

The normative host obligations live elsewhere and they rule:
`uvrr-io-obligations.md`, the boot-gate chapter, the termination chapter,
and the identity law (the boot-gate chapter §5). This document holds the
practices a deployment
has found useful and nothing more.

## The system-identifier practice

- The system identifier is sysadmin-assigned. It is burnt into the boot
  marker before first boot, and where the deployment keeps a WAL or
  superblock header, the same identifier may be burnt there for the same
  reason: every durable artifact names the system it belongs to.
- A separate genesis command is the only writer of that field. It runs
  once, under the operator's hand, and the running process never mints a
  system identifier of its own.
- The process command line takes its system identifier and aborts on a
  boot-fence mismatch. The identifier never begins at zero: a zero read is
  an uninitialised field or a corrupt marker, never an identity.

## The halt-and-move practice

Cluster nodes may be halted, their files moved between machines, and the
system booted elsewhere. Opening the durable artifacts against the
command-line system identifier is what prevents a node from accidentally
mixing state between machines: a file that names another system is refused
at open, before any protocol state is read.

## The crash-loop practice

The identity bump refuses at the counter bound: the sixteenth bit of lives
is the last the packing holds, and a bump past it is a refusal, never a
wrap. A host process monitor that retries a failing node indefinitely
walks that bound one life at a time. Kubernetes-style health and readiness
checks are the sharp edge: a node whose startup is slow can be killed and
retried before it ever joins, and a perma crash loop depletes the
deployment's headroom one burned identity per attempt.

A node that was never able to join the cluster owes the cluster nothing,
and its counter carries no commitment the cluster ever saw. Resetting such
a node's counter to a number just above its last committed membership
value does not harm safety: the identity law rules the identities the
cluster has seen, and a life that never joined was never seen. The reset
is the host's deliberate act, taken with the same care as the genesis
mint, and never applied to a node the cluster has seated.

This suggests what the cluster-membership history should record: the host
id alongside the separate halves. The actual limit is the highest
committed crash counter any node has held while a member of the cluster at
any point, which exhausts at `u16::MAX` rejoins of the cluster: a minimum
of `u16::MAX` cluster reconfigurations of headroom, which is ample.

## The goalpost practices

The goalpost practices name the signs a host watches for. They answer, for a
host reader, how a deployment might keep a healthy quiet leader from being
deposed, and each section below states one suggestion with its source.

The suggestions gather under one question: what does a host do in the
intervals when the protocol has nothing to say? Viewstamped Replication
Revisited (2012) leaves the repeated sending of lost messages a minor detail
of the implementation (§4.2, "re-sending of messages that appear to have
been lost"; the same convention at §4.1's opening), and Paxos Made Simple
(2001) treats the timeout measures as implementation suggestions whose
failure never touches safety. The library is sans-IO (`architecture.md`
"Host obligations for correct running", opening paragraph): every timeout is
the host's, and so is every mechanism that arms one. Whether any suggestion
below is wanted, in what form, and at what cadence, is a deployment's
decision; none of it is required.

## The heartbeat-of-a-commit practice

A leader with no outstanding matters (no proposal record standing,
`src/replica/mod.rs:706-719`) and matching frontiers (its accepted and
committed frontiers have met) may, on its timeout, send its last commit as a
heartbeat: the idle shape Viewstamped Replication Revisited (2012) §4.1
describes for a primary that has no new client request to propose, informing
the backups of the latest commit in place of a proposal. The normative
statement of the option lives in `architecture.md`, liveness subsection: it
is one option among many, no mechanism is required, and the duration is the
host's.

What the heartbeat buys is idle-alive evidence: the suspicion gate of
`plan_tick` (`src/replica/mod.rs:1782`) reads only same-view `Prepare` and
`Commit` from the legitimate primary (`src/replica/mod.rs:2944-2948`), so a
quiet leader running the heartbeat is not deposed while a fully silent one
eventually is. What it costs is an emission the core would not otherwise
release on a tick: the core's tick plan emits nothing for a quiescent leader
(`plan_tick` at `src/replica/mod.rs:1726` plans a promotion, a suspicion,
a stalled offer's revival, and an open fetch's retry, and no heartbeat), so
the host releases it as an effect itself. A deployment weighing the trade
might ask what its spare datagram flow is worth against how its operators
interpret silence: one idle line per period keeps the backups' suspicion
gate fed, while a deployment that prefers silence keeps the suspicion
instead, and both choices are correct; the choice is the host's, and
nothing in it touches safety (Paxos Made Simple 2001).

## The piggybacked-commit practice

A related suggestion puts the last commit on the tail of any outgoing leader
message, so receivers' suspicion gates feed without a dedicated heartbeat:
the same idle shape (Viewstamped Replication Revisited 2012 §4.1) delivered
opportunistically instead of on a schedule. The discussion for the host is
what exists to build on. The commit handler takes the frontier it is handed
idempotently: a delivery whose frontier does not move and whose gate status
is unchanged is absorbed without a transition
(`src/replica/normal.rs:679-687`, the frontier folded at 675-678), so a
receiver that hears the same frontier repeatedly takes from the repeat only
the suspicion baseline's refresh (`src/replica/mod.rs:2944-2948`), which is
exactly the evidence the suggestion exists to supply. Whether the leader's
outgoing traffic in a busy interval is frequent enough to make a dedicated
heartbeat redundant, and whether opportunistic delivery keeps the backups'
gates fed in the quiet intervals where a heartbeat earns its keep, is a
deployment's question to answer against its own traffic.

## The any-other-mechanism practice

The durations and the mechanisms that arm suspicion are the host's, and any
other leader timeout mechanism the deployment wants is allowable by
construction. The protocol's only rule is that a node may do view changes
and advance the view, resolved via quorums (Viewstamped Replication
Revisited 2012 §9.1; the suspicion-and-view-change sketch of Paxos Made
Simple 2001). The host's suspicion timeout firing, or not firing, is a host
knob (`primary_timeout`, `src/replica/mod.rs:499-519`, where a value of
zero disables tick-driven suspicion altogether), and the core deposes on
the backed-up-tick path per its designed cold-start behaviour (`plan_tick`,
`src/replica/mod.rs:1718-1794`). A deployment might time the suspicion on
round trips, on silences, on a failure detector's eventual completeness, or
on anything else it can wire to the same fence; what the fence is for and
who may call it are the protocol's, everything else is the host's. Paxos
Made Simple (2001) makes the point in its own voice: the timeout measures
are suggestions, needed for progress in some reasonable proportion to the
failures they answer, and nothing turns on their exact shape.

## Open questions for the host

The suggestions above leave questions a deployment answers for itself, and
two of them the sources also leave open. They are stated here as questions,
what exists today pinned to the code and the normative documents, and no
answer is proposed.

- **Is the resend of an un-acked proposal host transport policy entirely?**
  Today's answer in the normative documents is yes: a repeated send is a
  relay of effects the core has already released, never a message the host
  composes (`architecture.md`, liveness subsection, "The leader's resend
  policy", resting on `uvrr-durability-model.md` §13.2's authorization rule
  for replayed messages and on Viewstamped Replication Revisited 2012 §4.1,
  where the primary re-sends the proposal that is lagging). A deployment
  preferring the core to re-emit on tick would instead want the core's tick
  plan to carry the resend effect itself, and the open question is what
  that tick-emission contract would say: the core reads no clock, so the
  cadence of any re-emission would be expressed only in ticks, and the
  contract would have to state what the host's timeout input is and when a
  re-emission stops. No such contract exists today, and nothing here
  proposes one.

- **Does the leader suspect itself on idleness?** Today's designed
  behaviour is that it does: an idle primary's own timeout deposes it
  exactly like a backup's, the voting-member gate admits the primary of the
  current view and its baseline refresh depends on its own emitted stream
  (`plan_tick`, `src/replica/mod.rs:1718-1794`; the cold-start corpus pins
  the behaviour). The heartbeat suggestion can weaken the practical case
  for keeping that behaviour: a leader that feeds its backups idle evidence
  has, in a deployment that runs the heartbeat, less ordinary need to be
  deposed by its own clock. The question for the host is what its
  deployment wants; any change to the current behaviour is a documented
  design decision made expressly, never a drift, and this discussion
  document does not propose one.

## The genesis boot-fence practice

- The boot fence markers the first boot reads are written by the sysadmin's
  genesis command, not by the running process: a host that embeds the
  application needs an equivalent of that command, whether the demo
  binaries' genesis flag or its own writer. This is a recommendation, not
  an obligation: a host that bootstraps its markers another way is the
  host's business, provided what it writes satisfies the boot gate.
- The system identifier a genesis write burns never begins at zero, for the
  reason the identity law states: a zero read is an uninitialised field or
  a corrupt marker, never an identity. The crash-restart counter of the
  identity the host names on a restart obeys the same rule.
- The worked example is the TigerBeetle-superblock demo
  (`examples/uvrr-reincarnation/`), whose own README states what it
  demonstrates; nothing here is kept current.
