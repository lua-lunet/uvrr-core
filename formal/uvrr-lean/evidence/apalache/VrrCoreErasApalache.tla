-------------------------- MODULE VrrCoreErasApalache --------------------------
(***************************************************************************
 * Executable safety model of the intended uVRR reconfiguration protocol.
 *
 * This is a design model.  It deliberately does not describe the current
 * implementation state.  One transition crosses one era boundary, which is
 * sufficient for the pairwise R1/R2 and diskless-recovery obligations and
 * keeps the protocol interaction within exhaustive TLC reach.
 *
 * The network is a monotone set.
 *
 * ANNOTATED COPY: this module is VrrCoreEras.tla plus the Snowcat type
 * annotations (constants, variables, and field-accessing operators) that
 * the Apalache checker requires.  The normative model is never modified.  An enabled receive may be delayed forever,
 * repeated, or ignored, modelling reordering, duplication, and loss without
 * adding transport state.
 *************************************************************************)

EXTENDS FiniteSets, Naturals, Sequences, TLC

CONSTANTS
    \* @type: Str;
    N0,
    \* @type: Str;
    N1,
    \* @type: Str;
    N2,
    \* @type: Str;
    N3,
    \* @type: Str;
    N4,
    \* @type: Str;
    N5,
    \* @type: Str;
    Scenario,
    \* @type: Set(Str);
    Commands,
    \* @type: Int;
    MaxViewIndex,
    \* @type: Int;
    MaxLogLength,
    \* @type: Int;
    MaxEpoch,
    \* @type: Int;
    MaxNonce,
    \* @type: Bool;
    CrashRetainsDurable,
    \* @type: Str;
    Defect

NoDefect                 == "none"
TruncateOnTransfer       == "truncate-on-transfer"
EagerViewAdoption        == "eager-view-adoption"
PlannedCountsAsFence     == "planned-counts-as-fence"
UnserializedCast         == "unserialized-cast"
NoPivotGuard             == "no-pivot-guard"

Defects == {NoDefect, TruncateOnTransfer, EagerViewAdoption,
            PlannedCountsAsFence, UnserializedCast, NoPivotGuard}

Inc3     == "inc3"
Even4    == "even4"
Even4Static == "even4-static"
Maj5     == "maj5"
GateDemo == "gate-demo"
SplitPivot4 == "split-pivot4"
Scenarios == {Inc3, Even4, Even4Static, Maj5, GateDemo, SplitPivot4}

Nodes == CASE Scenario = Inc3     -> {N0, N1, N2}
           [] Scenario = Even4    -> {N0, N1, N2, N3}
           [] Scenario = Even4Static -> {N0, N1, N2, N3}
           [] Scenario = Maj5     -> {N0, N1, N2, N3, N4}
           [] Scenario = GateDemo -> {N0, N1, N2, N3, N4, N5}
           [] Scenario = SplitPivot4 -> {N0, N1, N2, N3}
           [] OTHER               -> {}

Order0 == CASE Scenario = Inc3     -> <<N0, N1, N2>>
            [] Scenario = Even4    -> <<N0, N1, N2, N3>>
            [] Scenario = Even4Static -> <<N0, N1, N2, N3>>
            [] Scenario = Maj5     -> <<N0, N1, N2, N3, N4>>
            [] Scenario = GateDemo -> <<N0, N1, N2, N3, N4, N5>>
            [] Scenario = SplitPivot4 -> <<N0, N1, N2, N3>>
            [] OTHER               -> <<>>

(* Membership and succession order are fixed for this one transition. *)
Order(e) == Order0

    \* @type: (Int, Str) => Int;
Weight(e, r) ==
    CASE Scenario = Even4Static -> 1
      [] Scenario = SplitPivot4 /\ e = 0 /\ r = N0 -> 3
      [] Scenario = SplitPivot4 /\ e = 1 /\ r = N2 -> 3
      [] e = 0 -> 1
      [] e = 1 /\ r = N0 -> 2
      [] OTHER -> 1

    \* @type: (Int, Set(Str)) => Int;
SumW(e, members) ==
      (IF N0 \in members THEN Weight(e, N0) ELSE 0)
    + (IF N1 \in members THEN Weight(e, N1) ELSE 0)
    + (IF N2 \in members THEN Weight(e, N2) ELSE 0)
    + (IF N3 \in members THEN Weight(e, N3) ELSE 0)
    + (IF N4 \in members THEN Weight(e, N4) ELSE 0)
    + (IF N5 \in members THEN Weight(e, N5) ELSE 0)

CommitThreshold(e) ==
    CASE Scenario \in {Even4, Even4Static} /\ e = 0 -> 2
      [] Scenario = Even4Static -> 2
      [] Scenario = GateDemo       -> 3
      [] Scenario = SplitPivot4    -> 6
      [] OTHER -> (SumW(e, Nodes) \div 2) + 1

(******************************************************************************
 * Even-split view-quorum threshold for era 1.  Era-1 weights give
 * SumW(1, Nodes) = 5 with Weight(1, N0) = 2.  The complement of the era-0
 * commit quorum {N0, N1} is {N2, N3}, whose era-1 weight is 2, so any
 * era-1 view quorum disjoint from {N0, N1} carries weight at most 2;
 * threshold 4 is the least value that cannot be met inside {N2, N3} union
 * N1 alone: the maximal era-1 view quorum avoiding N0 is {N1, N2, N3} of
 * weight 3.  See the per-scenario derivation in the README.
 *****************************************************************************)
Even4EraOneViewThreshold == 4

ViewThreshold(e) ==
    CASE Scenario \in {Even4, Even4Static} /\ e = 0 -> 3
      [] Scenario = Even4 /\ e = 1 -> Even4EraOneViewThreshold
      [] Scenario = Even4Static -> 3
      [] Scenario = GateDemo       -> 4
      [] Scenario = SplitPivot4    -> 4
      [] OTHER -> (SumW(e, Nodes) \div 2) + 1

    \* @type: (Int, Set(Str)) => Bool;
IsCommitQuorum(e, members) ==
    /\ members \subseteq Nodes
    /\ SumW(e, members) >= CommitThreshold(e)

    \* @type: (Int, Set(Str)) => Bool;
IsViewQuorum(e, members) ==
    /\ members \subseteq Nodes
    /\ SumW(e, members) >= ViewThreshold(e)

(* Fence and recovery use the view family in this design. *)
IsFenceQuorum(e, members)    == IsViewQuorum(e, members)
IsRecoveryQuorum(e, members) == IsViewQuorum(e, members)

    \* @type: (Set(Str) -> Bool, Set(Str) -> Bool) => Bool;
Intersects(familyA, familyB) ==
    \A a \in SUBSET Nodes, b \in SUBSET Nodes :
        (familyA[a] /\ familyB[b]) => a \cap b # {}

R1(e) == Intersects([s \in SUBSET Nodes |-> IsViewQuorum(e, s)],
                    [s \in SUBSET Nodes |-> IsCommitQuorum(e, s)])

R2Forward ==
    Intersects([s \in SUBSET Nodes |-> IsViewQuorum(0, s)],
               [s \in SUBSET Nodes |-> IsCommitQuorum(1, s)])

R2Reverse ==
    Intersects([s \in SUBSET Nodes |-> IsViewQuorum(1, s)],
               [s \in SUBSET Nodes |-> IsCommitQuorum(0, s)])

VV(e) == Intersects([s \in SUBSET Nodes |-> IsViewQuorum(e, s)],
                    [s \in SUBSET Nodes |-> IsViewQuorum(e, s)])

FR(e) == Intersects([s \in SUBSET Nodes |-> IsFenceQuorum(e, s)],
                    [s \in SUBSET Nodes |-> IsRecoveryQuorum(e, s)])

FRCross01 ==
    Intersects([s \in SUBSET Nodes |-> IsFenceQuorum(0, s)],
               [s \in SUBSET Nodes |-> IsRecoveryQuorum(1, s)])

FRCross10 ==
    Intersects([s \in SUBSET Nodes |-> IsFenceQuorum(1, s)],
               [s \in SUBSET Nodes |-> IsRecoveryQuorum(0, s)])

GateOK == /\ R1(0) /\ R1(1)
          /\ R2Forward /\ R2Reverse
          /\ VV(0) /\ VV(1)
          /\ FR(0) /\ FR(1)
          \* Cross-era fence/recovery, both directions: a recovery quorum
          \* gathered in one era must meet the other era's fence family,
          \* or a recovering replica can complete while an unfenced
          \* identity of the other era still casts view votes (the
          \* exclusion obligation of section 8.3).
          /\ FRCross01 /\ FRCross10

\* @type: (Seq(Str)) => Set(Str);
SeqToSet(s) == {s[i] : i \in {j \in 1..6 : j <= Len(s)}}

(****************************************************************************
 * Constant well-formedness stays an ASSUME.  The quorum obligations are
 * per-obligation startup Asserts in Init so a gate mutation can require
 * the exact rejected obligation rather than any assumption failure.
 ***************************************************************************)
ASSUME /\ Scenario \in Scenarios
       /\ Defect \in Defects
       /\ Commands # {}
       /\ IsFiniteSet(Commands)
       /\ MaxViewIndex \in Nat
       /\ MaxLogLength >= 2
       /\ MaxEpoch \in Nat
       /\ MaxNonce \in Nat
       /\ CrashRetainsDurable \in BOOLEAN
       /\ Len(Order0) = Cardinality(Nodes)
       /\ SeqToSet(Order0) = Nodes
       /\ Cardinality(Nodes) >= 3
       /\ Cardinality({N0, N1, N2, N3, N4, N5}) = 6

(***************************************************************************
 * View identifiers and log algebra.
 *************************************************************************)

Eras        == {0, 1}
ViewIndices == 0..MaxViewIndex
Views       == [era : Eras, idx : ViewIndices]

View(e, i) == [era |-> e, idx |-> i]
InitialView == View(0, 0)

    \* @type: ([era: Int, idx: Int], [era: Int, idx: Int]) => Bool;
ViewLt(a, b) == \/ a.era < b.era
                 \/ /\ a.era = b.era
                    /\ a.idx < b.idx

    \* @type: ([era: Int, idx: Int], [era: Int, idx: Int]) => Bool;
ViewLe(a, b) == a = b \/ ViewLt(a, b)
    \* @type: ([era: Int, idx: Int], [era: Int, idx: Int]) => Bool;
ViewGt(a, b) == ViewLt(b, a)

    \* @type: ([era: Int, idx: Int]) => Int;
ViewRank(v) == v.era * (MaxViewIndex + 1) + v.idx

    \* @type: ([era: Int, idx: Int]) => Str;
Primary(v) == Order(v.era)[(v.idx % Len(Order(v.era))) + 1]

    \* @type: (Str) => [era: Int, idx: Int];
PlannedView(leader) ==
    CHOOSE v \in Views :
        /\ v.era = 1
        /\ Primary(v) = leader
        /\ \A w \in Views :
              (w.era = 1 /\ Primary(w) = leader) => v.idx <= w.idx

GenesisKind  == "genesis"
CommandKind  == "command"
ReconfigKind == "reconfig"
EntryKinds   == {GenesisKind, CommandKind, ReconfigKind}

GenesisValue  == "genesis-value"
ReconfigValue == "increment-n0"
EntryValues   == Commands \cup {GenesisValue, ReconfigValue}

Entry(kind_, value_, era_) ==
    [kind |-> kind_, value |-> value_, era |-> era_]

GenesisEntry         == Entry(GenesisKind, GenesisValue, 0)
ReconfigurationEntry == Entry(ReconfigKind, ReconfigValue, 0)
CommandEntry(c, e)    == Entry(CommandKind, c, e)
Entries               == [kind : EntryKinds, value : EntryValues, era : Eras]
\* @type: Seq([kind: Str, value: Str, era: Int]);
Genesis               == <<GenesisEntry>>

    \* @type: (Int, Int) => Int;
Min(a, b) == IF a <= b THEN a ELSE b
    \* @type: (Int, Int) => Int;
Max(a, b) == IF a >= b THEN a ELSE b
    \* @type: (Set(Int)) => Int;
SetMax(s) == CHOOSE x \in s : \A y \in s : y <= x

    \* @type: (Seq([kind: Str, value: Str, era: Int]), Seq([kind: Str, value: Str, era: Int]), Int) => Bool;
PrefixEqual(a, b, through) ==
    /\ through <= Len(a)
    /\ through <= Len(b)
    /\ \A i \in {j \in 1..MaxLogLength : j <= through} : a[i] = b[i]

    \* @type: (Seq([kind: Str, value: Str, era: Int]), Int) => Int;
CountReconfigBefore(log, slot) ==
    Cardinality({i \in {j \in 1..MaxLogLength : j <= slot - 1} :
                     log[i].kind = ReconfigKind})

    \* @type: (Seq([kind: Str, value: Str, era: Int]), Int) => Int;
EraOfSlot(log, slot) == CountReconfigBefore(log, slot)

    \* @type: (Seq([kind: Str, value: Str, era: Int])) => Set(Int);
ReconfigPositions(log) ==
    {i \in {j \in 1..MaxLogLength : j <= Len(log)} :
        log[i].kind = ReconfigKind}

HasReconfig(log) == ReconfigPositions(log) # {}

    \* @type: (Seq([kind: Str, value: Str, era: Int])) => Int;
EstablishingSlot(log) ==
    IF ReconfigPositions(log) = {}
    THEN 0
    ELSE CHOOSE i \in ReconfigPositions(log) : TRUE

    \* @type: (Seq([kind: Str, value: Str, era: Int]), Int) => Bool;
HasCommittedReconfig(log, frontier) ==
    \E i \in {j \in 1..MaxLogLength : j <= frontier} :
        log[i].kind = ReconfigKind

    \* @type: (Seq([kind: Str, value: Str, era: Int])) => Seq([kind: Str, value: Str, era: Int]);
DropLast(s) == IF Len(s) = 0 THEN <<>> ELSE SubSeq(s, 1, Len(s) - 1)

(***************************************************************************
 * Protocol state and the single total message shape.
 *************************************************************************)

Normal     == "normal"
ViewChange == "view-change"
Recovering == "recovering"
Statuses   == {Normal, ViewChange, Recovering}

PrepareMsg          == "prepare"
PrepareOkMsg        == "prepare-ok"
CommitMsg           == "commit"
StartViewChangeMsg  == "start-view-change"
DoViewChangeMsg     == "do-view-change"
StartViewMsg        == "start-view"
PlannedViewChangeMsg == "planned-view-change"
PlannedOkMsg        == "planned-ok"

MessageKinds == {PrepareMsg, PrepareOkMsg, CommitMsg,
                 StartViewChangeMsg, DoViewChangeMsg, StartViewMsg,
                 PlannedViewChangeMsg, PlannedOkMsg}

    \* @type: (Str, Str, Str, [era: Int, idx: Int], [era: Int, idx: Int], Int, Int, [kind: Str, value: Str, era: Int], Seq([kind: Str, value: Str, era: Int]), [era: Int, idx: Int], Int, Int, Int) => [type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int];
Message(kind_, from_, to_, view_, priorView_, routeEra_, slot_, entry_,
        history_, retained_, accepted_, committed_, nonce_) ==
    [type      |-> kind_,
     from      |-> from_,
     to        |-> to_,
     view      |-> view_,
     priorView |-> priorView_,
     routeEra  |-> routeEra_,
     slot      |-> slot_,
     entry     |-> entry_,
     history   |-> history_,
     retained  |-> retained_,
     accepted  |-> accepted_,
     committed |-> committed_,
     nonce     |-> nonce_]

NeutralEntry == GenesisEntry

Prepare(p, r, v, n, entry_, k, routeEra_) ==
    Message(PrepareMsg, p, r, v, v, routeEra_, n, entry_, <<>>,
            v, 0, k, 0)

PrepareOk(r, p, v, n, routeEra_) ==
    Message(PrepareOkMsg, r, p, v, v, routeEra_, n, NeutralEntry, <<>>,
            v, 0, 0, 0)

Commit(p, r, v, n, routeEra_) ==
    Message(CommitMsg, p, r, v, v, routeEra_, n, NeutralEntry, <<>>,
            v, 0, n, 0)

StartViewChangeVote(r, to, v) ==
    Message(StartViewChangeMsg, r, to, v, v, v.era, 0, NeutralEntry, <<>>,
            v, 0, 0, 0)

DoViewChange(r, p, v, retained_, history_, committed_) ==
    Message(DoViewChangeMsg, r, p, v, v, v.era, Len(history_), NeutralEntry,
            history_, retained_, Len(history_), committed_, 0)

StartView(p, r, v, history_, committed_) ==
    Message(StartViewMsg, p, r, v, v, v.era, Len(history_), NeutralEntry,
            history_, v, Len(history_), committed_, 0)

PlannedViewChange(leader, r, prior_, target_) ==
    Message(PlannedViewChangeMsg, leader, r, target_, prior_, 0, 0,
            NeutralEntry, <<>>, prior_, 0, 0, 0)

(* Abstract projection of a planned DoViewChange reply.  History is retained so
 * the evidence-kind mutation can exercise the dangerous conflation. *)
PlannedOk(r, leader, prior_, target_, retained_, history_, committed_) ==
    Message(PlannedOkMsg, r, leader, target_, prior_, 0, Len(history_),
            NeutralEntry, history_, retained_, Len(history_), committed_, 0)

VARIABLES
    \* @type: Str -> Str;
    status,
    \* @type: Str -> [era: Int, idx: Int];
    currentView,
    \* @type: Str -> [era: Int, idx: Int];
    retainedView,
    \* @type: Str -> Seq([kind: Str, value: Str, era: Int]);
    logs,
    \* @type: Str -> Int;
    committed,
    \* @type: Set([type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]);
    messages,
    \* @type: Str -> Int;
    epochs,
    \* @type: Set([slot: Int, entry: [kind: Str, value: Str, era: Int]]);
    historicalCommitted,
    \* @type: Bool;
    everEnteredViewChange,
    \* @type: Bool;
    castOccurred,
    \* @type: Bool;
    overlapStreamed

vars == <<status, currentView, retainedView, logs, committed, messages,
          epochs, historicalCommitted,
          everEnteredViewChange, castOccurred, overlapStreamed>>

Init ==
    /\ status       = [r \in Nodes |-> Normal]
    /\ currentView  = [r \in Nodes |-> InitialView]
    /\ retainedView = [r \in Nodes |-> InitialView]
    /\ logs         = [r \in Nodes |-> Genesis]
    /\ committed    = [r \in Nodes |-> Len(Genesis)]
    /\ messages     = {}
    /\ epochs       = [r \in Nodes |-> 0]
    /\ historicalCommitted =
           {[slot |-> i, entry |-> Genesis[i]] :
               i \in {j \in 1..6 : j <= Len(Genesis)}}
    /\ everEnteredViewChange = FALSE
    /\ castOccurred = FALSE
    /\ overlapStreamed = FALSE
    /\ Assert(R1(0), "gate: r1-era0")
    /\ Assert(R1(1), "gate: r1-era1")
    /\ Assert(R2Forward, "gate: r2-forward")
    /\ Assert(R2Reverse, "gate: r2-reverse")
    /\ Assert(VV(0), "gate: vv-era0")
    /\ Assert(VV(1), "gate: vv-era1")
    /\ Assert(FR(0), "gate: fr-era0")
    /\ Assert(FR(1), "gate: fr-era1")
    /\ Assert(FRCross01, "gate: fr-cross-fence0-recovery1")
    /\ Assert(FRCross10, "gate: fr-cross-fence1-recovery0")

CanVote(r) == status[r] # Recovering

BroadcastPrepare(p, v, n, entry_, k, routeEra_) ==
    {Prepare(p, r, v, n, entry_, k, routeEra_) : r \in Nodes \ {p}}

ProposeCommand(p, command) ==
    /\ status[p] = Normal
    /\ p = Primary(currentView[p])
    /\ Len(logs[p]) < MaxLogLength
    /\ \A i \in {j \in 1..MaxLogLength : j <= Len(logs[p])} :
           logs[p][i].value # command
    /\ LET n == Len(logs[p]) + 1
           era == EraOfSlot(logs[p], n)
           entry_ == CommandEntry(command, era)
       IN /\ era \in Eras
          /\ currentView[p].era <= era
          /\ era <= currentView[p].era + 1
          /\ logs' = [logs EXCEPT ![p] = Append(@, entry_)]
          /\ messages' = messages \cup
                 BroadcastPrepare(p, currentView[p], n, entry_, committed[p], era)
    /\ UNCHANGED <<status, currentView, retainedView, committed, epochs,
                   everEnteredViewChange,
                   castOccurred,
                   overlapStreamed>>

ProposeReconfig(p) ==
    /\ status[p] = Normal
    /\ currentView[p].era = 0
    /\ p = Primary(currentView[p])
    /\ ~HasReconfig(logs[p])
    /\ Len(logs[p]) < MaxLogLength
    /\ LET n == Len(logs[p]) + 1
       IN /\ logs' = [logs EXCEPT ![p] = Append(@, ReconfigurationEntry)]
          /\ messages' = messages \cup
                 BroadcastPrepare(p, currentView[p], n,
                                  ReconfigurationEntry, committed[p], 0)
    /\ UNCHANGED <<status, currentView, retainedView, committed, epochs,
                   everEnteredViewChange,
                   castOccurred,
                   overlapStreamed>>

    \* @type: (Str, [type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]) => Bool;
ReceivePrepare(r, m) ==
    /\ m \in messages
    /\ m.type = PrepareMsg
    /\ m.to = r
    /\ status[r] = Normal
    /\ m.view = currentView[r]
    /\ m.from = Primary(m.view)
    /\ m.slot = Len(logs[r]) + 1
    /\ m.slot <= MaxLogLength
    /\ m.routeEra = EraOfSlot(logs[r], m.slot)
    /\ m.entry.era = m.routeEra
    /\ LET newLog == Append(logs[r], m.entry)
           learned == Max(committed[r], Min(m.committed, Len(newLog)))
       IN /\ logs' = [logs EXCEPT ![r] = newLog]
          /\ committed' = [committed EXCEPT ![r] = learned]
          /\ messages' = messages \cup
                 {PrepareOk(r, m.from, m.view, m.slot, m.routeEra)}
    /\ UNCHANGED <<status, currentView, retainedView, epochs,
                   everEnteredViewChange,
                   castOccurred,
                   overlapStreamed>>

    \* @type: (Str, [era: Int, idx: Int], Int, Int) => Set(Str);
AckSenders(p, v, n, era) ==
    {m.from : m \in {candidate \in messages :
              /\ candidate.type = PrepareOkMsg
              /\ candidate.to = p
              /\ candidate.view = v
              /\ candidate.slot >= n
              /\ candidate.routeEra = era}}

CommitNext(p) ==
    /\ status[p] = Normal
    /\ p = Primary(currentView[p])
    /\ committed[p] < Len(logs[p])
    /\ LET n == committed[p] + 1
           era == EraOfSlot(logs[p], n)
           support == {p} \cup AckSenders(p, currentView[p], n, era)
           outbound == {Commit(p, r, currentView[p], n, era) :
                           r \in Nodes \ {p}}
       IN /\ IsCommitQuorum(era, support)
          /\ committed' = [committed EXCEPT ![p] = n]
          /\ messages' = messages \cup outbound
          /\ overlapStreamed' =
                 (overlapStreamed \/ (era = 1 /\ currentView[p].era = 0))
    /\ UNCHANGED <<status, currentView, retainedView, logs, epochs,
                   everEnteredViewChange,
                   castOccurred>>

    \* @type: (Str, [type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]) => Bool;
ReceiveCommit(r, m) ==
    /\ m \in messages
    /\ m.type = CommitMsg
    /\ m.to = r
    /\ status[r] = Normal
    /\ m.view = currentView[r]
    /\ m.from = Primary(m.view)
    /\ m.routeEra \in Eras
    /\ LET learned == Max(committed[r], Min(m.committed, Len(logs[r])))
       IN /\ learned > committed[r]
          /\ committed' = [committed EXCEPT ![r] = learned]
    /\ UNCHANGED <<status, currentView, retainedView, logs, messages, epochs,
                   everEnteredViewChange,
                   castOccurred,
                   overlapStreamed>>

(***************************************************************************
 * Ordinary view change.  Only explicit initiation has A1's establishing-op
 * guard.  Following an existing fence is allowed to fetch that history.
 *************************************************************************)

ViewChangeBroadcast(r, v) ==
    {StartViewChangeVote(r, to, v) : to \in Nodes}

    \* @type: (Str, [era: Int, idx: Int]) => Bool;
EnterViewChange(r, target) ==
    /\ target \in Views
    /\ ViewGt(target, currentView[r])
    /\ status[r] # Recovering
    /\ (target.era = 0 \/ HasCommittedReconfig(logs[r], committed[r]))
    /\ status' = [status EXCEPT ![r] = ViewChange]
    /\ currentView' = [currentView EXCEPT ![r] = target]
    /\ messages' = messages \cup ViewChangeBroadcast(r, target)
    /\ everEnteredViewChange' = TRUE
    /\ UNCHANGED <<retainedView, logs, committed, epochs,
                   castOccurred,
                   overlapStreamed>>

    \* @type: (Str, [type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]) => Bool;
FollowHigherViewChange(r, m) ==
    /\ m \in messages
    /\ m.type = StartViewChangeMsg
    /\ m.to = r
    /\ m.view \in Views
    /\ ViewGt(m.view, currentView[r])
    /\ CanVote(r)
    /\ status' = [status EXCEPT ![r] = ViewChange]
    /\ currentView' = [currentView EXCEPT ![r] = m.view]
    /\ messages' = messages \cup ViewChangeBroadcast(r, m.view)
    /\ everEnteredViewChange' = TRUE
    /\ UNCHANGED <<retainedView, logs, committed, epochs,
                   castOccurred,
                   overlapStreamed>>

FenceKinds == IF Defect = PlannedCountsAsFence
              THEN {StartViewChangeMsg, PlannedOkMsg}
              ELSE {StartViewChangeMsg}

    \* @type: (Str, [era: Int, idx: Int]) => Set(Str);
FenceSenders(r, v) ==
    {m.from : m \in {candidate \in messages :
              /\ candidate.type \in FenceKinds
              /\ candidate.to = r
              /\ candidate.view = v}}

ReportAlreadySent(r, v) ==
    \E m \in messages :
        /\ m.type = DoViewChangeMsg
        /\ m.from = r
        /\ m.view = v

SendDoViewChange(r) ==
    /\ status[r] = ViewChange
    /\ CanVote(r)
    /\ IsFenceQuorum(currentView[r].era,
                     FenceSenders(r, currentView[r]))
    /\ ~ReportAlreadySent(r, currentView[r])
    /\ messages' = messages \cup
           {DoViewChange(r, Primary(currentView[r]), currentView[r],
                         retainedView[r], logs[r], committed[r])}
    /\ UNCHANGED <<status, currentView, retainedView, logs, committed, epochs,
                   everEnteredViewChange,
                   castOccurred,
                   overlapStreamed>>

ReportKinds == IF Defect = PlannedCountsAsFence
               THEN {DoViewChangeMsg, PlannedOkMsg}
               ELSE {DoViewChangeMsg}

    \* @type: (Str, [era: Int, idx: Int]) => Set([type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]);
Reports(p, v) ==
    {m \in messages :
        /\ m.type \in ReportKinds
        /\ m.to = p
        /\ m.view = v}

ReportSenders(p, v) == {m.from : m \in Reports(p, v)}

    \* @type: ([type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]) => Int;
ReportRank(m) ==
    LET provenance == IF Defect = PlannedCountsAsFence
                            /\ m.type = PlannedOkMsg
                       THEN m.view
                       ELSE m.retained
    IN ViewRank(provenance) * (MaxLogLength + 1) + m.accepted

    \* @type: (Str, [type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]) => Bool;
InstallView(p, chosen) ==
    /\ status[p] = ViewChange
    /\ p = Primary(currentView[p])
    /\ LET reports == Reports(p, currentView[p])
           maxCommitted == CHOOSE k \in {m.committed : m \in reports} :
                               \A j \in {m.committed : m \in reports} : j <= k
       IN /\ IsViewQuorum(currentView[p].era,
                          ReportSenders(p, currentView[p]))
          /\ chosen \in reports
          /\ \A report \in reports : ReportRank(report) <= ReportRank(chosen)
          /\ chosen.accepted = Len(chosen.history)
          /\ (maxCommitted <= chosen.accepted
              \/ Defect = PlannedCountsAsFence)
          /\ (currentView[p].era = 0
              \/ /\ HasReconfig(chosen.history)
                 /\ EstablishingSlot(chosen.history) <= maxCommitted)
          /\ (PrefixEqual(chosen.history, logs[p], committed[p])
              \/ Defect = PlannedCountsAsFence)
          /\ status' = [status EXCEPT ![p] = Normal]
          /\ retainedView' = [retainedView EXCEPT ![p] = currentView[p]]
          /\ logs' = [logs EXCEPT ![p] = chosen.history]
          /\ committed' = [committed EXCEPT ![p] = Max(@, maxCommitted)]
          /\ messages' = messages \cup
                 {StartView(p, r, currentView[p], chosen.history,
                            Max(committed[p], maxCommitted)) :
                    r \in Nodes \ {p}}
    /\ UNCHANGED <<currentView, epochs,
                   everEnteredViewChange, castOccurred, overlapStreamed>>

    \* @type: (Str, [type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]) => Bool;
ReceiveStartView(r, m) ==
    /\ m \in messages
    /\ m.type = StartViewMsg
    /\ m.to = r
    /\ CanVote(r)
    /\ m.from = Primary(m.view)
    /\ m.accepted = Len(m.history)
    /\ m.committed <= m.accepted
    /\ \/ ViewGt(m.view, currentView[r])
       \/ /\ m.view = currentView[r]
          /\ status[r] = ViewChange
    /\ m.committed >= committed[r]
    /\ (PrefixEqual(m.history, logs[r], committed[r])
        \/ Defect = TruncateOnTransfer)
    \* The defect drops the newest entry on receipt but reports the
    \* transferred committed frontier verbatim: the receiver's frontier
    \* claims entries its installed history no longer holds.  The normative
    \* path lowers the frontier consistently with the installed history.
    \* (A pure DropLast with a consistently lowered frontier is ABSORBED by
    \* the protocol's guard net: a truncated replica never leads again and
    \* Prepare never overwrites, so no committed fact is ever contradicted.)
    /\ LET installed == IF Defect = TruncateOnTransfer
                         THEN DropLast(m.history)
                         ELSE m.history
           learned == IF Defect = TruncateOnTransfer
                      THEN m.committed
                      ELSE Min(m.committed, Len(installed))
           ack == IF Len(installed) > learned
                  THEN {PrepareOk(r, m.from, m.view, Len(installed),
                                  EraOfSlot(installed, Len(installed)))}
                  ELSE {}
       IN /\ messages' = messages \cup ack
          /\ logs' = [logs EXCEPT ![r] = installed]
          /\ committed' = [committed EXCEPT ![r] = learned]
    /\ status' = [status EXCEPT ![r] = Normal]
    /\ currentView' = [currentView EXCEPT ![r] = m.view]
    /\ retainedView' = [retainedView EXCEPT ![r] = m.view]
    /\ UNCHANGED <<epochs, everEnteredViewChange,
                   castOccurred, overlapStreamed>>

(* The eager-adoption mutation changes the view fence without installing the
 * selected history.  It is unreachable in the normative model. *)
    \* @type: (Str, [type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]) => Bool;
EagerAdopt(r, m) ==
    /\ Defect = EagerViewAdoption
    /\ m \in messages
    /\ m.to = r
    /\ m.type \in {PrepareMsg, StartViewMsg}
    /\ ViewGt(m.view, currentView[r])
    /\ status[r] # Recovering
    /\ status' = [status EXCEPT ![r] = Normal]
    /\ currentView' = [currentView EXCEPT ![r] = m.view]
    /\ retainedView' = [retainedView EXCEPT ![r] = m.view]
    /\ UNCHANGED <<logs, committed, messages, epochs,
                   everEnteredViewChange, castOccurred,
                   overlapStreamed>>

(***************************************************************************
 * Planned view change and the leader's casting vote.
 *************************************************************************)

PlannedAlreadySent(leader, target) ==
    \E m \in messages :
        /\ m.type = PlannedViewChangeMsg
        /\ m.from = leader
        /\ m.view = target

    \* @type: (Str) => [era: Int, idx: Int];
PlannedTarget(leader) == PlannedView(leader)

SendPlannedViewChange(leader) ==
    /\ status[leader] = Normal
    /\ currentView[leader].era = 0
    /\ leader = Primary(currentView[leader])
    /\ HasCommittedReconfig(logs[leader], committed[leader])
    /\ PlannedTarget(leader) \in Views
    /\ ~PlannedAlreadySent(leader, PlannedTarget(leader))
    /\ messages' = messages \cup
           {PlannedViewChange(leader, r, currentView[leader],
                              PlannedTarget(leader)) :
              r \in Nodes \ {leader}}
    /\ UNCHANGED <<status, currentView, retainedView, logs, committed, epochs,
                   everEnteredViewChange,
                   castOccurred,
                   overlapStreamed>>

    \* @type: (Str, [type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]) => Bool;
AnswerPlannedViewChange(r, m) ==
    /\ m \in messages
    /\ m.type = PlannedViewChangeMsg
    /\ m.to = r
    /\ status[r] = Normal
    /\ currentView[r] = m.priorView
    /\ messages' = messages \cup
           {PlannedOk(r, m.from, m.priorView, m.view,
                      retainedView[r], logs[r], committed[r])}
    /\ UNCHANGED <<status, currentView, retainedView, logs, committed, epochs,
                   everEnteredViewChange,
                   castOccurred,
                   overlapStreamed>>

    \* @type: (Str, [era: Int, idx: Int]) => Set(Str);
PlannedResponders(leader, target) ==
    {m.from : m \in {candidate \in messages :
              /\ candidate.type = PlannedOkMsg
              /\ candidate.to = leader
              /\ candidate.view = target}}

(****************************************************************************
 * The casting vote needs one old-era view quorum confined to the
 * responders plus the leader, meeting the planned (both-era) commit
 * quorum in the leader alone.  View quorums are weight-upward-closed, so
 * for each candidate shared commit quorum qII it suffices to test the
 * MAXIMAL legal old-view set: the responders outside qII, plus the
 * leader.  One powerset quantification instead of two.
 ***************************************************************************)
    \* @type: (Str, Set(Str)) => Bool;
PivotExists(leader, responders) ==
    \E qII \in SUBSET Nodes :
        /\ IsCommitQuorum(0, qII)
        /\ IsCommitQuorum(1, qII)
        /\ leader \in qII
        /\ IsViewQuorum(0, (responders \ qII) \cup {leader})

PlannedReports(leader, target) ==
    {m \in messages :
        /\ m.type = PlannedOkMsg
        /\ m.to = leader
        /\ m.view = target}

(****************************************************************************
 * The normative cast installs the leader's own log; the chosen PlannedOk
 * is quantified only by the unserialized-cast defect action, so the
 * normative transition does not generate one identical successor per
 * irrelevant message.
 ***************************************************************************)
CastPlannedVote(leader) ==
    /\ Defect # UnserializedCast
    /\ status[leader] = Normal
    /\ currentView[leader].era = 0
    /\ leader = Primary(currentView[leader])
    /\ HasCommittedReconfig(logs[leader], committed[leader])
    /\ LET target == PlannedView(leader)
           responders == PlannedResponders(leader, target)
       IN /\ (PivotExists(leader, responders) \/ Defect = NoPivotGuard)
          /\ currentView' = [currentView EXCEPT ![leader] = target]
          /\ retainedView' = [retainedView EXCEPT ![leader] = target]
          /\ messages' = messages \cup
                 {StartView(leader, r, target, logs[leader],
                            committed[leader]) :
                    r \in Nodes \ {leader}}
    /\ castOccurred' = TRUE
    /\ UNCHANGED <<status, logs, committed, epochs,
                   everEnteredViewChange, overlapStreamed>>

    \* @type: (Str, [type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]) => Bool;
CastPlannedVoteFromEvidence(leader, chosen) ==
    /\ Defect = UnserializedCast
    /\ status[leader] = Normal
    /\ currentView[leader].era = 0
    /\ leader = Primary(currentView[leader])
    /\ HasCommittedReconfig(logs[leader], committed[leader])
    /\ LET target == PlannedView(leader)
           responders == PlannedResponders(leader, target)
           evidence == PlannedReports(leader, target)
       IN /\ PivotExists(leader, responders)
          /\ chosen \in evidence
          /\ currentView' = [currentView EXCEPT ![leader] = target]
          /\ retainedView' = [retainedView EXCEPT ![leader] = target]
          /\ logs' = [logs EXCEPT ![leader] = chosen.history]
          /\ messages' = messages \cup
                 {StartView(leader, r, target, chosen.history,
                            committed[leader]) :
                    r \in Nodes \ {leader}}
    /\ castOccurred' = TRUE
    /\ UNCHANGED <<status, committed, epochs,
                   everEnteredViewChange, overlapStreamed>>

(***************************************************************************
 * Crash and operational recovery: two durability profiles.  Crash fences
 * the identity and forgets the volatile protocol state (status, views,
 * evidence, recovery attempt); it then either forgets the durable records
 * as well (volatile deployment: in-memory journal strategy) or retains
 * them (the persisted Progress record and journal).  A recovering identity
 * sends only recovery requests.  An attempt is a bounded set of nonces,
 * one per solicitation, so a delayed response to a remembered nonce still
 * counts and responses across in-set nonces combine by sender.  An
 * accepted response whose committed exceeds the local frontier
 * fast-forwards it over the locally held contiguous prefix that agrees
 * with the response history.  Quorum guards count the recorded evidence
 * itself; a replica's live status gates only what it may SEND.
 *************************************************************************)

Crash(r) ==
    /\ MaxEpoch > 0
    /\ status[r] # Recovering
    /\ epochs[r] < MaxEpoch
    /\ \A s \in Nodes : epochs[s] = 0
    /\ status' = [status EXCEPT ![r] = Recovering]
    /\ currentView' = [currentView EXCEPT ![r] = InitialView]
    /\ retainedView' = [retainedView EXCEPT ![r] = InitialView]
    /\ epochs' = [epochs EXCEPT ![r] = @ + 1]
    /\ \/ /\ logs' = [logs EXCEPT ![r] = Genesis]
          /\ committed' = [committed EXCEPT ![r] = Len(Genesis)]
       \/ /\ CrashRetainsDurable
          /\ UNCHANGED <<logs, committed>>
    /\ UNCHANGED <<messages, everEnteredViewChange, castOccurred,
                   overlapStreamed>>

Next ==
    /\ \/ \E p \in Nodes, command \in Commands : ProposeCommand(p, command)
       \/ \E p \in Nodes : ProposeReconfig(p)
       \/ \E r \in Nodes, m \in messages : ReceivePrepare(r, m)
       \/ \E p \in Nodes : CommitNext(p)
       \/ \E r \in Nodes, m \in messages : ReceiveCommit(r, m)
       \/ \E r \in Nodes, target \in Views : EnterViewChange(r, target)
       \/ \E r \in Nodes, m \in messages : FollowHigherViewChange(r, m)
       \/ \E r \in Nodes : SendDoViewChange(r)
       \/ \E p \in Nodes, chosen \in messages : InstallView(p, chosen)
       \/ \E r \in Nodes, m \in messages : ReceiveStartView(r, m)
       \/ \E r \in Nodes, m \in messages : EagerAdopt(r, m)
       \/ \E leader \in Nodes : SendPlannedViewChange(leader)
       \/ \E r \in Nodes, m \in messages : AnswerPlannedViewChange(r, m)
       \/ \E leader \in Nodes : CastPlannedVote(leader)
       \/ \E leader \in Nodes, chosen \in messages :
              CastPlannedVoteFromEvidence(leader, chosen)
       \/ \E r \in Nodes : Crash(r)
    \* Monotone ghost record of every (slot, entry) fact any transition
    \* commits, computed centrally from post-state logs and frontiers.
    /\ historicalCommitted' = historicalCommitted \cup
           UNION {{[slot |-> i, entry |-> logs'[r][i]] :
                      i \in {j \in 1..MaxLogLength :
                                committed[r] < j /\ j <= committed'[r]}} :
                     r \in Nodes}

Spec == Init /\ [][Next]_vars


(***************************************************************************
 * Safety obligations and non-vacuity witnesses.
 *************************************************************************)

    \* @type: ([type: Str, from: Str, to: Str, view: [era: Int, idx: Int], priorView: [era: Int, idx: Int], routeEra: Int, slot: Int, entry: [kind: Str, value: Str, era: Int], history: Seq([kind: Str, value: Str, era: Int]), retained: [era: Int, idx: Int], accepted: Int, committed: Int, nonce: Int]) => Bool;
MessageTypeOK(m) ==
    /\ m.type \in MessageKinds
    /\ m.from \in Nodes
    /\ m.to \in Nodes
    /\ m.view \in Views
    /\ m.priorView \in Views
    /\ m.routeEra \in Eras
    /\ m.slot \in 0..MaxLogLength
    /\ m.entry \in Entries
    /\ m.history \in Seq(Entries)
    /\ Len(m.history) <= MaxLogLength
    /\ m.retained \in Views
    /\ m.accepted \in 0..MaxLogLength
    /\ m.committed \in 0..MaxLogLength
    /\ m.nonce \in 0..MaxNonce

TypeOK ==
    /\ status \in [Nodes -> Statuses]
    /\ currentView \in [Nodes -> Views]
    /\ retainedView \in [Nodes -> Views]
    /\ logs \in [Nodes -> Seq(Entries)]
    /\ committed \in [Nodes -> 0..MaxLogLength]
    /\ messages \subseteq [type : MessageKinds,
                             from : Nodes, to : Nodes,
                             view : Views, priorView : Views,
                             routeEra : Eras,
                             slot : 0..MaxLogLength,
                             entry : Entries,
                             history : Seq(Entries),
                             retained : Views,
                             accepted : 0..MaxLogLength,
                             committed : 0..MaxLogLength,
                             nonce : 0..MaxNonce]
    /\ \A r \in Nodes : Len(logs[r]) <= MaxLogLength
    /\ \A m \in messages : MessageTypeOK(m)
    /\ epochs \in [Nodes -> 0..MaxEpoch]
    /\ historicalCommitted \subseteq [slot : 1..MaxLogLength,
                                      entry : Entries]
    /\ everEnteredViewChange \in BOOLEAN
    /\ castOccurred \in BOOLEAN
    /\ overlapStreamed \in BOOLEAN

FrontiersOrdered ==
    \A r \in Nodes : committed[r] <= Len(logs[r])

NormalViewInstalled ==
    \A r \in Nodes : status[r] = Normal => currentView[r] = retainedView[r]

ViewChangeRetainsHistoryProvenance ==
    \A r \in Nodes :
        status[r] = ViewChange => ViewLe(retainedView[r], currentView[r])

CommittedLogsAgree ==
    \A r \in Nodes, s \in Nodes :
        PrefixEqual(logs[r], logs[s], Min(committed[r], committed[s]))

(****************************************************************************
 * Historical committed facts: no committed slot is ever repopulated with a
 * different entry, and every entry once committed remains present in at
 * least one current replica history.  These replace the retired
 * self-witnessing survival predicate (its existential admitted s = r).
 ***************************************************************************)
CommittedHistoryUnique ==
    \A f \in historicalCommitted, g \in historicalCommitted :
        f.slot = g.slot => f.entry = g.entry

CommittedHistoryPresent ==
    \A f \in historicalCommitted :
        \E s \in Nodes, i \in {j \in 1..MaxLogLength : j = f.slot} :
            f.slot <= Len(logs[s]) /\ logs[s][i] = f.entry

EntryEraWindow ==
    /\ \A r \in Nodes :
          \A i \in {j \in 1..MaxLogLength : j <= Len(logs[r])} :
              logs[r][i].era = EraOfSlot(logs[r], i)
    /\ \A m \in messages :
          m.type = PrepareMsg =>
              /\ m.routeEra = m.entry.era
              /\ m.view.era <= m.entry.era
              /\ m.entry.era <= m.view.era + 1

EraOneHasEstablishingOp ==
    \A r \in Nodes :
        (status[r] = Normal /\ currentView[r].era = 1) =>
            HasCommittedReconfig(logs[r], committed[r])

AtMostOneReconfig ==
    /\ \A r \in Nodes : Cardinality(ReconfigPositions(logs[r])) <= 1
    /\ \A m \in messages :
          Cardinality(ReconfigPositions(m.history)) <= 1

PlannedTargetsWellFormed ==
    \A m \in {candidate \in messages :
                 candidate.type \in {PlannedViewChangeMsg, PlannedOkMsg}} :
            /\ m.view = PlannedView(
                   IF m.type = PlannedViewChangeMsg THEN m.from ELSE m.to)
            /\ m.view.era = 1
            /\ ViewLt(m.priorView, m.view)

(* TLC checks this action formula as a temporal safety property.  The status is
 * sampled before the message is added, which a state-only predicate cannot do.
 * The fenced replica sends nothing (the reincarnation ingress rule: messages
 * TO it are fine, FROM it ignored). *)
RecoveringSendStep ==
    \A m \in messages' \ messages : status[m.from] # Recovering

RecoveringSendsNothing == [] [RecoveringSendStep]_vars

WNonStop == ~(castOccurred /\ ~everEnteredViewChange)
WOverlapStreams == ~overlapStreamed

=============================================================================
