import UVRR.Synod

/-! The pre-vote role machine and the liveness content of the leader-overlap handshake.

The mechanism: a candidate does not propose because a wait expired. It asks, and it
prepares only when the answers carry both quorum families. What is proved here is the
deterministic content of that claim, in the form it can be stated without a temporal
logic — the repository has none, and past-time operators are recorded as future work in
`README.md`.

Three things, each of which the Rust suite also checks and each of which Lean can state
over the whole domain rather than sample it:

1. **The role table is total.** The twelve ownership × recency cells of `docs/prevoting.md`
   §3, discharged by `decide` over the entire product — the Lean twin of
   `tests/prevote_role_machine.rs`. A new variant in either enum makes the conjunction
   false, so the proof obligation does not silently survive an extension.

2. **The dual-quorum rule earns its second conjunct.** Under `Frown QI QII` the two
   families overlap, and a set that is a quorum of one meets every quorum of the other
   (`familiesBridge`). That is the reason a candidate must hold *both*: the phase-1 quorum
   is what makes its ballot fresh for the members that answer, and the phase-2 quorum is
   what lets it finish. Dropping either conjunct leaves a set that can promise without
   being able to accept, or accept without having been promised.

3. **No self-sustaining round.** The pre-vote messages are emitted only on a clock event
   or on receipt of a solicitation, and the replies emit nothing further. So one clock
   event buys exactly three emissions and then the exchange is silent
   (`burst_is_three`, `replies_emit_nothing`). This is the quiescence argument, and it is
   the formal content of "no lock-up for as long as waits keep expiring": no chain of
   messages can occupy the cluster between two clock events, so an event that arrives is
   never starved by the exchange the previous one started.

What is *not* proved, and is stated as a host obligation in `docs/prevoting.md` §7.3
rather than claimed here: that some candidate eventually finds itself the only active
node. That is an eventually-almost-surely statement about an infinite time line under a
randomised wait schedule, and this development has no measure-theoretic vocabulary for it.
-/

namespace PreVote

/-- Who owns the ballot in which the value this node last saw chosen was chosen. -/
inductive Ownership where
  | mine
  | theirs
  | unknown
deriving DecidableEq, Repr

/-- How recently that value was chosen, as a rank against the two waits. -/
inductive Recency where
  | veryRecent
  | recent
  | stale
  | never
deriving DecidableEq, Repr

/-- The node's own answer to whether it is leading, and whether anyone is. -/
inductive Role where
  | leader
  | incumbent
  | follower
  | candidate
deriving DecidableEq, Repr

/-- The role for an ownership and a recency: total, closed, and with no default arm.
Mirrors `uvrr::prevote::role` cell for cell. -/
def role : Ownership → Recency → Role
  | .mine, .veryRecent => .leader
  | .mine, .recent => .incumbent
  | .mine, .stale => .candidate
  | .mine, .never => .candidate
  | .theirs, .veryRecent => .follower
  | .theirs, .recent => .follower
  | .theirs, .stale => .candidate
  | .theirs, .never => .candidate
  | .unknown, .veryRecent => .candidate
  | .unknown, .recent => .candidate
  | .unknown, .stale => .candidate
  | .unknown, .never => .candidate

/-- The role table of `docs/prevoting.md` §3, discharged over the whole domain.

Each conjunct is one cell, so the twelve conjuncts are the twelve cells and a thirteenth
cell cannot be added to the code without falsifying this statement. -/
theorem roleTableAgrees :
    role .mine .veryRecent = .leader ∧
    role .mine .recent = .incumbent ∧
    role .mine .stale = .candidate ∧
    role .mine .never = .candidate ∧
    role .theirs .veryRecent = .follower ∧
    role .theirs .recent = .follower ∧
    role .theirs .stale = .candidate ∧
    role .theirs .never = .candidate ∧
    role .unknown .veryRecent = .candidate ∧
    role .unknown .recent = .candidate ∧
    role .unknown .stale = .candidate ∧
    role .unknown .never = .candidate := by decide

/-- The two argued rows, as separate obligations so a failure names which argument broke.

A node whose own value has gone stale is a candidate and not an incumbent, because an
incumbent is defined by owning a *recent* choice and there is nothing left to be
incumbent over. -/
theorem staleIsNotIncumbent :
    role .mine .recent = .incumbent ∧ role .mine .stale = .candidate := by decide

/-- An unattributable ballot proves no seat at any recency, including the freshest: this
is the row a reconfiguration produces, and the reason the answer is a candidate rather
than a claim. -/
theorem unattributableProvesNoSeat :
    role .unknown .veryRecent = .candidate ∧ role .unknown .stale = .candidate := by decide

/-- Which of the node's two waits expired. -/
inductive Wait where
  | leadership
  | recency
deriving DecidableEq, Repr

/-- The recency one wait-expiry leaves behind: a rank moved down by the wait that
expired, and never moved up. The shorter wait spends only freshness; the longer demotes
either freshness to stale and leaves a stale value and a never-chosen one alone. -/
def demote : Recency → Wait → Recency
  | .veryRecent, .leadership => .recent
  | .veryRecent, .recency => .stale
  | .recent, .leadership => .recent
  | .recent, .recency => .stale
  | .stale, .leadership => .stale
  | .stale, .recency => .stale
  | .never, .leadership => .never
  | .never, .recency => .never

/-- The demotion table of `docs/prevoting.md` §2, over all eight cells. -/
theorem demoteTableAgrees :
    demote .veryRecent .leadership = .recent ∧
    demote .veryRecent .recency = .stale ∧
    demote .recent .leadership = .recent ∧
    demote .recent .recency = .stale ∧
    demote .stale .leadership = .stale ∧
    demote .stale .recency = .stale ∧
    demote .never .leadership = .never ∧
    demote .never .recency = .never := by decide

/-- The demotion never invents knowledge: it never moves a rank up, and it never turns a
never-chosen value into anything else. The second conjunct is what makes a clock event on
a node that has seen nothing not protocol. -/
theorem demoteNeverInvents :
    (demote .never .leadership = .never) ∧ (demote .never .recency = .never) := by decide

/-- What a peer answers a solicitation with. Silence is a third answer, not an absence of
one, and it is the answer of a peer that still believes a leader is active. -/
inductive Answer where
  | catchUp
  | offerVote
  | silent
deriving DecidableEq, Repr

/-- The reply rule over the two facts that decide it: whether this peer knows a value
chosen later than the one advertised, and whether this peer is a candidate. The later
knowledge outranks the silence, because the answer to a stale premise is the truth. -/
def reply : Bool → Bool → Answer
  | true, _ => .catchUp
  | false, true => .offerVote
  | false, false => .silent

/-- The reply rule over all four cells, so a third answer cannot be introduced without
falsifying this statement. -/
theorem replyTableAgrees :
    reply true true = .catchUp ∧
    reply true false = .catchUp ∧
    reply false true = .offerVote ∧
    reply false false = .silent := by decide

/-- The pre-vote property, as the one cell that carries it: a peer that is not a candidate
answers nothing, whether or not it is itself a candidate is the *other* branch. -/
theorem onlyCandidatesEncourageAnElection :
    (∀ later isCand, reply later isCand = .silent → isCand = false) := by decide

/-- The messages of the handshake, plus the clock event that is its only external source. -/
inductive Msg where
  /-- A wait expired. The sole source of a solicitation. -/
  | waitExpired
  | solicit
  | offerVote
  | offerCatchUp
  | prepare
  | promised
  | proposed
  | accepted
deriving DecidableEq, Repr

/-- What receipt of each message emits.

A solicitation is emitted only by a clock event. The replies emit nothing further: a
candidate that learns it is behind catches up rather than soliciting again, and a peer
that has nothing later to say has already said so. That separation is the whole of the
quiescence argument. -/
def emits : Msg → List Msg
  | .waitExpired => [.solicit]
  | .solicit => [.offerVote, .offerCatchUp]
  | .offerVote => []
  | .offerCatchUp => []
  | .prepare => [.promised]
  | .promised => []
  | .proposed => []
  | .accepted => []

/-- The replies emit nothing, so no reply can start a round of its own. -/
theorem repliesEmitNothing :
    (emits .offerVote = []) ∧ (emits .offerCatchUp = []) ∧
    (emits .solicit = [.offerVote, .offerCatchUp]) := by decide

/-- The messages one clock event puts on the wire: the solicitation out and the two
replies in. Three emissions, and the clock event itself is not one of them. -/
def burst : List Msg := [.solicit, .offerVote, .offerCatchUp]

/-- One clock event buys exactly three emissions and then the exchange is silent.

Three is the round-trip count the specification states: solicit out, reply in, and nothing
further. `decide` establishes it over the whole relation rather than by inspection. -/
theorem burst_is_three :
    burst.length = 3 ∧ (emits .waitExpired).length = 1 ∧ (emits .solicit).length = 2 ∧
      (emits .offerVote).length = 0 ∧ (emits .offerCatchUp).length = 0 := by decide

/-- The burst is exactly the reachable closure of a clock event, and it names every
message the exchange can put on the wire. -/
theorem burst_is_the_closure :
    burst = emits .waitExpired ++ (emits .solicit) := by decide

/-- No cycle in the message graph below the clock: nothing one clock event puts on the
wire is itself a clock event or a phase-1 proposal, and neither reply reaches one. This
is the claim that rules out the livelock of a cluster re-soliciting itself forever with no
wait ever expiring. -/
theorem noCycleBelowTheClock :
    ∀ m ∈ burst, m ≠ .waitExpired ∧ m ≠ .prepare := by decide

/-- Exactly one solicitation per clock event, so the exchange cannot re-solicit itself: the
only thing that can put a second solicitation on the wire is a wait that expires again. -/
theorem oneSolicitationPerWait :
    burst.count .solicit = 1 ∧ burst.count .offerVote = 1 ∧ burst.count .offerCatchUp = 1 :=
  by decide

/-- The families bridge: a set that is a quorum of one family meets every quorum of the
other, so a candidate holding both holds a set that can promise and can accept.

This is the reason the rule requires both conjuncts. Under `Frown QI QII` the overlap is
the project's safety condition, and it is what makes a phase-1 quorum usable: the
quorum that clears the candidate's ballot is guaranteed to meet the quorum that will
accept. -/
theorem familiesBridge {A : Type} {QI QII : QSys A} (hF : Frown QI QII) {V : NSet A}
    (hQI : QI V) (q2 : NSet A) (hQII : QII q2) : ∃ a, V a ∧ q2 a :=
  hF V q2 hQI hQII

/-- The bridge in the direction the phase-1 conjunct needs: a set that is a phase-1 quorum
meets every phase-2 quorum. A candidate holding that set knows its ballot is fresh for the
members that answered, and that some acceptance quorum overlaps them. -/
theorem phaseOneQuorumMeetsEveryPhaseTwo {A : Type} {QI QII : QSys A} (hF : Frown QI QII)
    {V : NSet A} (hQI : QI V) (q2 : NSet A) (hQII : QII q2) : ∃ a, V a ∧ q2 a :=
  familiesBridge hF hQI q2 hQII

/-- The bridge in the direction the phase-2 conjunct needs, which needs the safety
condition supplied the other way round.

Stated as its own theorem because the asymmetry is the point: `Frown QI QII` licenses the
first use and licenses nothing about a set that is *only* a phase-2 quorum. That is why
the rule asks for both families rather than for one and inferring the other. -/
theorem phaseTwoQuorumMeetsEveryPhaseOne {A : Type} {QI QII : QSys A} (hF : Frown QII QI)
    {V : NSet A} (hQII : QII V) (q1 : NSet A) (hQI : QI q1) : ∃ a, V a ∧ q1 a :=
  familiesBridge hF hQII q1 hQI

/-- Without the overlap condition the bridge is not available, and the rule cannot be
reasoned away: there are families that do not overlap, so a quorum of one need not meet
any quorum of the other. -/
theorem overlapIsNotFree {A : Type} (QI QII : QSys A) (h : ¬ Frown QI QII) :
    ¬ (∀ V, QI V → ∀ q2, QII q2 → ∃ a, V a ∧ q2 a) :=
  fun hbridge => h (fun q1 q2 h1 h2 => hbridge q1 h1 q2 h2)

end PreVote
