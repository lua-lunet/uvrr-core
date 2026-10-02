# uVRR pre-voting: the leader-overlap handshake a candidate runs before it is entitled to a phase 1

A candidate does not propose because a wait expired. It proposes because it asked, the
peers answered, and the answers were sufficient. That is the whole of the mechanism, and
this document states it as an invariant and a transition function: the ballot owner, the
two waits, the four roles, the message alphabet, the reply rule, the dual-quorum rule,
and the liveness claim with its scope boundary stated plainly rather than blurred.
`src/prevote/` is the code, and it is judged against this statement.

## 1. Model statement

A ballot in uVRR has a well-defined owner: [`Configuration::primary`] names the member
the ballot selects, and [`next_view_selecting`] is the arithmetic that finds the least
later ballot selecting a named member. A node that learns a value was chosen in ballot `b`
therefore knows, without an election, which member `b` belongs to.

Pre-voting turns that fact into the right to propose. A node compares the ballot that
owns the value it last saw chosen against itself, and it compares the *recency* of that
choice against two waits. From those two comparisons it names its own role. A candidate
does not then propose: it solicits, it collects replies, and it prepares only when the
replies carry both quorum families. Every wait is a local clock event; no node reads
another's clock, and no clock value is shared, so there is nothing to synchronise
(`src/timeout.rs`, the S4 ruling, `docs/uvrr-durability-model.md` §13.2).

The mechanism is leader overlap in the sense of §7: an up-to-date peer that still believes
a leader is active answers a solicitation with silence, so a partitioned node rejoining
cannot displace a healthy leader by acting on a stale local view.

## 2. The two waits

A pre-voting node runs two waits, not one, and the difference between them is the whole
of the leader/incumbent distinction:

- The **leadership wait** is the shorter. While it is unexpired the value this node owns
  counts as *very recent*, and the node may call itself the leader.
- The **recency wait** is the longer. While it is unexpired but the leadership wait has
  expired, the value still counts as *recent*, and the node is an incumbent: it still owns
  the ballot of a value chosen recently, but it is no longer fresh enough to lead.

The waits are ordered, and the ordering is the only thing the protocol states. **No
variant in this module carries a duration**, for the reason `src/timeout.rs` states of its
own flavours: a duration is host policy, may be dynamic, and is never the protocol's to
state. What crosses the module boundary is the *rank* of an event — which of the two waits
expired — not how long it took.

## 3. The four roles

The role is a total function of two closed domains: who owns the ballot of the value this
node last saw chosen, and how recently that value was chosen. There is no third input and
no default arm.

Ownership is one of:

- `Mine` — this node owns the ballot in which that value was chosen.
- `Theirs(NodeId)` — that named member owns it.
- `Unknown` — no chosen value is known, or one is known whose ballot this node cannot
  attribute to a member of the configuration it holds.

Recency is one of:

- `VeryRecent` — within the leadership wait.
- `Recent` — within the recency wait, not within the leadership wait.
- `Stale` — older than the recency wait.
- `Never` — no value is known to have been chosen.

The table, which is the whole of the domain:

| Ownership | VeryRecent | Recent | Stale | Never |
|---|---|---|---|---|
| `Mine` | Leader | Incumbent | Candidate | Candidate |
| `Theirs` | Follower | Follower | Candidate | Candidate |
| `Unknown` | Candidate | Candidate | Candidate | Candidate |

Three rows deserve their reasons stated.

A node whose own value has gone stale is a **Candidate**, not an Incumbent. An incumbent
is defined by owning a *recently* chosen value; once the recency wait has expired there is
nothing to be incumbent over, and the node needs the peers' permission like anybody else.

A node that knows a very recently chosen value but cannot attribute its ballot is a
**Candidate**. This row is the safe answer and it is not vacuous: after a
reconfiguration a node may hold a chosen value whose ballot belongs to a configuration
generation it has not yet adopted. Claiming leadership on an unattributable ballot would
be claiming a seat it cannot prove, so the row answers Candidate and the solicitation
path resolves the uncertainty.

A follower that has not heard anything for the recency wait becomes a **Candidate**. That
is the only transition into candidacy from outside it, and it is why the recency wait
exists: it is the node's own statement that it no longer believes a leader is active.

## 4. The message alphabet

Three messages, all point-to-point or broadcast, none of them on the wire yet (§9):

- **Solicit** — a candidate's request. It carries the sender's committed frontier, so a
  peer can tell whether it knows something later, and the sender's own ballot, so the
  peer can tell whether the candidate is behind (§7.3).
- **OfferVote** — a peer's answer naming the greatest ballot for which it has already
  sent a promise. This is the floor the candidate must clear.
- **OfferCatchUp** — a peer's answer naming a later chosen value it knows of. The
  candidate learns from that peer instead of proposing.

The solicitation carries the sender's ballot for a reason that is easy to get wrong. A
leader that receives a solicitation carrying a *later* ballot than its own must perform
another phase 1 at a later ballot still, rather than ignore it. Without the ballot in the
message, a minority that has run ahead sits in a later view, cannot accept the leader's
proposals, and cannot gather enough offers to elect anyone either — it is stuck being
caught up, visibly a candidate, while making progress. The system is not incorrect, but
the candidate signal that operators watch for has stopped meaning what it should.

## 5. The reply rule

A peer answers a solicitation in exactly one of three ways, and the three are exhaustive
over what the peer knows:

1. If the peer knows a value chosen **later** than the one the sender advertised, it
   answers **OfferCatchUp**.
2. Otherwise, if the peer believes a leader is active — that is, the peer's own role is
   not Candidate — it **does not answer**.
3. Otherwise it answers **OfferVote** with its promise floor.

Rule 2 is the pre-vote property. Silence is the answer of a healthy cluster: a node that
still believes the leader is fine declines to encourage an election, so a partitioned
node that rejoins and solicits cannot talk the cluster into a view change it does not
need. Rule 1 precedes rule 2 deliberately — a peer with something later to say says so
even if it also believes a leader is active, because the candidate's premise is stale and
the answer to a stale premise is the truth, not a vote.

## 6. The candidate's verdict: the dual-quorum rule

The candidate proceeds only when the offers satisfy **both** families, not either:

- the offering set contains a **phase-1 quorum** ([`quorum::Role::ViewChange`]) of the
  configuration corresponding to the first unchosen instance, **and**
- the offering set contains a **phase-2 quorum** ([`quorum::Role::Commit`]) of that same
  configuration.

If it cannot reach both, there is no point proceeding: the candidate would run a phase 1
it cannot complete into an acceptance, and the round trip would be wasted.

**Why both, and not either.** The candidate's real problem is choosing a ballot that is
high enough. A ballot chosen as "a little above the greatest one I have seen" is not
demonstrably high enough, because a node that has been partitioned may hold a promise
nobody in the soliciting quorum has heard of; the election then fails, repeatedly, with no
proof that it ever will not. Collecting offers from a phase-1 quorum makes the ballot
provably fresh, because every member of that quorum has now stated its floor. Collecting
offers from a phase-2 quorum is what makes the round *decidable*: the same set that
clears phase 1 must also be able to accept, or the candidate cannot finish what it starts.

The ballot to prepare at is no less than any ballot offered — the fold's maximum over the
offers, never less than the candidate's own. The quorum questions are asked of the
[`QuorumStrategy`], never counted here: the strategy is the single point of truth for
what a quorum is, exactly as it is for the view change (`src/quorum.rs`, the "no counts
are computed in the handler" rule).

An offer the candidate may not act on does not count towards the quorum. A peer offering
a ballot that is not a legal successor of the candidate's own floor is offering something
the candidate has no right to prepare at, and [`Ballot::is_legal_successor`] is the one
place that rule is stated; such an offer is dropped, and a round carrying one reports
Pending rather than preparing at a ballot it cannot justify.

**The second conjunct is latent under the shipped strategy, and that is worth knowing.**
On unit weights the shipped [`WeightedMajority`] draws the same families for
[`quorum::Role::Commit`] and [`quorum::Role::ViewChange`], so any set that is a quorum of
one is a quorum of the other and the rule reduces to a single check. The conjunct earns
its place as soon as weights differ, which is the case it was written for; the refusals
in `tests/prevote_verdict.rs` observe it under a strategy that separates the families,
because the shipped one cannot.

## 7. Liveness

### 7.1 The claim

**No lock-up.** A cluster in which waits keep expiring does not stall: the handshake of
§5–§6 either decides the first unchosen instance or is not the only thing running, and
between two clock events the work outstanding is bounded.

### 7.2 What is proved

In `formal/uvrr-lean/UVRR/PreVote.lean`, over the whole domain rather than sampled:

1. **The role function is total and closed.** `roleTableAgrees` is the twelve cells of
   §3, one conjunct each, discharged by `decide` over the entire product of the two
   enums — so a thirteenth cell added to the code falsifies the statement rather than
   surviving it. The two argued rows are separate theorems (`staleIsNotIncumbent`,
   `unattributableProvesNoSeat`) so a failure names which argument broke.
   `demoteTableAgrees` does the same for the eight cells of §2, and
   `replyTableAgrees` for the four cells of §5.

2. **The reply rule is total over its two deciding facts, and only candidates encourage
   an election.** `onlyCandidatesEncourageAnElection` is the pre-vote property as a
   theorem: a silent answer is possible only from a non-candidate.

3. **The overlap is what the second conjunct is for, and it is not free.**
   `phaseOneQuorumMeetsEveryPhaseOne` derives the bridge from `Frown QI QII`: a set that
   is a phase-1 quorum meets every phase-2 quorum, which is why that set can both clear the
   candidate's ballot and reach an acceptance. The converse direction is stated as its own
   theorem, `phaseTwoQuorumMeetsEveryPhaseOne`, and it needs `Frown QII QI` — the safety
   condition does not supply it. That asymmetry is the argument for requiring both
   families rather than one and inferring the other, and `overlapIsNotFree` shows the
   bridge genuinely does not follow without the overlap.

4. **No self-sustaining round.** `burst_is_three` and `burst_is_the_closure` fix the work a
   single clock event buys at three emissions — solicit out, reply in, silence — and
   `repliesEmitNothing` shows the replies emit nothing further.
   `noCycleBelowTheClock` and `oneSolicitationPerWait` state the two consequences that
   matter: nothing the exchange puts on the wire is a clock event or a phase-1 proposal,
   and there is exactly one solicitation per wait.

Items 1, 2 and 4 together are the formal content of §7.1's "no lock-up": the work a clock
event starts is bounded and cannot sustain itself, so a wait that expires is never starved
by the exchange the previous one started.

### 7.3 What is not proved, and why

Two steps are **not** proved, and neither is claimed.

**The promise-freshness step.** The argument that a candidate's ballot is fresh for every
member of the phase-1 quorum — that a phase 1 at any ballot at or above the greatest
offer meets no outstanding promise — is a corollary of the promise discipline (S2 and the
promise rule of `UVRR/Synod.lean`), and it is *not* restated here. What is proved is the
overlap it rests on. Turning `Frown` plus the promise discipline into "phase 1 completes,
phase 2 decides" is an integration obligation against the existing Synod development, not
a fresh theorem, and calling it proved would be a claim this file does not carry.

**The eventual-almost-surely step.** That some candidate eventually finds itself the only
active node is a claim about an infinite time line under a randomised wait schedule. This
repository has no temporal or measure-theoretic development to state it in — the
temporal-logic tooling was surveyed and deliberately not adopted, and past-time operators
are recorded as future work (`formal/uvrr-lean/README.md`, Proof tooling). So it is a
**host obligation, not a theorem**: waits must be randomised and growing, which
`src/backoff.rs` supplies as a window schedule with a cap. Nothing in `src/prevote/`
depends on it for safety, and the module's contract does not claim it.

### 7.4 The cost of being wrong

Two failure modes are named so they are not mistaken for each other. Too few solicitations
deadlock: a candidate that never completes a round never proposes. Too many livelock: a
cluster that keeps starting elections never settles on one. The wait ordering of §2 is
what separates them — the waits are set so an incumbent becomes a leader again before any
follower becomes a candidate, which is why the two waits are distinct and why the
incumbent's action (propose an empty entry in the ballot it already owns) is cheaper than a
candidate's.

## 8. Ephemeral state: nothing here is persisted

Every field of this module's state is volatile, and none of it survives a restart, in
either direction:

- A **clean shutdown** loses it, and a restarted node re-derives its role from what it
  learns. Nothing is owed.
- A **crash restart** loses it, and a reincarnated node rejoins as a weight-0 standby and
  is caught up by ordinary state transfer, exactly as any joiner is
  (`docs/uvrr-protocols.md`, the reincarnation chapter).

This is deliberate rather than incidental. The state is a *belief about failure* — who I
think is leading, and how stale my information is. Persisting a belief about failure is
what forces a design to reconcile a durable record of suspicion with a durable record of
progress, and uVRR has no crash-recover class to reconcile it into (§10 of the
reconfiguration rules). The cost of the choice is that a restarted node re-solicits, which
is one round trip; the benefit is that no pre-vote record can ever be stale on disk.

## 9. What this module does not do

- **It is not on the wire.** No [`Tag`], no [`Body`], no codec. Binding the alphabet to
  the normative binary format is a separate change with its own obligations: the §13.1
  suffix budget, and the pinned tag tables.
- **It is not called by the engine.** `src/replica/` does not consult it, the timeout
  matcher does not return its actions, and no [`Effect`] it describes is ever released.
  It is a specification that compiles and is tested, not a code path.
- **It does not persist anything** (§8).
- **It does not weaken the quorum strategy.** It asks; the strategy answers.
- **It does not claim Byzantine tolerance.** The trust model is the non-Byzantine one the
  rest of the protocol assumes (`docs/uvrr-protocols.md`, §1).

## 10. Test obligations

| # | Obligation | Where |
|---|---|---|
| 1 | Every ownership × recency cell names one role, and the table is closed over the product of the two domains | `tests/prevote_role_machine.rs` |
| 2 | Every ownership × recency × wait yields the action and the demoted recency this document states | `tests/exhaustive_prevote.rs` |
| 3 | The reply rule answers in exactly one of the three ways on all four cells, and only a candidate offers | `tests/prevote_reply.rs` |
| 4 | The verdict is `Prepare` only with both families, and the ballot is the greatest *legal* offer | `tests/prevote_verdict.rs` |
| 5 | A one-family set is refused under a strategy that separates the families — the negative control of §7.2, which fails if either conjunct is deleted | `tests/prevote_verdict.rs` || 6 | No [`Tag`] and no [`Body`] carries an alphabet message, and no discriminant past the allocated tags is claimed | `tests/prevote_contract.rs` |
| 7 | This document's role table **is** the code's table: the test parses the twelve cells out of this file and compares them against [`uvrr::prevote::role`] | `tests/prevote_contract.rs` |

Obligation 7 is what makes this document load-bearing rather than decorative. A cell
edited here and not in the code is a failing test, not a disagreement found in review.

## 11. Grounding sources

- [TURNER-PREVOTE] — David Turner, *Pre-voting in distributed consensus*, 17 August
  2017. The four roles, the two waits as an ordering, the three-way reply rule, the
  dual-quorum rule of the 2020 addendum, the disruption-on-reconnection fix of carrying
  the candidate's own ballot in the solicitation, and the quiescence argument. CC BY-SA
  4.0. Harvested at `research/agent473/`.
- [VR-2012] and [PMS-2001] — for the ballot, the owner-by-arithmetic rule, and the
  promise floor the offers report.
- `docs/architecture.md` decisions G1 and G3 — the two runtime tiers, which fix what this
  module asserts and what it merely treats as a maybe.

[`Ballot`]: ../src/ids.rs
[`Ballot::is_legal_successor`]: ../src/ids.rs
[`Body`]: ../src/message.rs
[`Configuration::primary`]: ../src/configuration.rs
[`Effect`]: ../src/effects.rs
[`QuorumStrategy`]: ../src/quorum.rs
[`Tag`]: ../src/wire.rs
[`WeightedMajority`]: ../src/quorum.rs
[`next_view_selecting`]: ../src/ids.rs
[`quorum::Role::Commit`]: ../src/quorum.rs
[`quorum::Role::ViewChange`]: ../src/quorum.rs
[`uvrr::prevote::role`]: ../src/prevote/mod.rs
