----------------------------- MODULE VrrCore -----------------------------
(******************************************************************************
 * Executable safety model for the protocol implemented by vrr-core.
 *
 * This is a correspondence model, not a transcription of VRR-2012.  Its
 * variables match the protocol-visible state in docs/vrr-durability-model.md:
 * currentView, retainedView, status, accepted (= Len(logs[r])), committed,
 * applied, volatile quorum evidence, and an asynchronous network.
 *
 * The network is a set that never removes messages.  An action may therefore
 * ignore a message forever (loss), consume it after arbitrary delay
 * (reordering), or consume it repeatedly while its guard remains enabled
 * (duplication).  Message bodies carry full histories where the Rust code may
 * carry a bounded suffix.  The omitted chunk/fetch steps refine one atomic
 * history installation and are not allowed to change history selection.
 *****************************************************************************)

EXTENDS FiniteSets, Naturals, Sequences

CONSTANTS R0, R1, R2, Commands, MaxView, MaxLogLength, MaxEpoch

ReplicaOrder == <<R0, R1, R2>>
Replicas == {R0, R1, R2}

Normal     == "normal"
ViewChange == "view-change"
Recovering == "recovering"
Replaying  == "replaying"
Statuses   == {Normal, ViewChange, Recovering, Replaying}

PrepareMsg          == "prepare"
PrepareOkMsg        == "prepare-ok"
CommitMsg           == "commit"
StartViewChangeMsg  == "start-view-change"
DoViewChangeMsg     == "do-view-change"
StartViewMsg        == "start-view"
RecoveryMsg         == "recovery"
RecoveryResponseMsg == "recovery-response"

MessageKinds == {
    PrepareMsg, PrepareOkMsg, CommitMsg,
    StartViewChangeMsg, DoViewChangeMsg, StartViewMsg,
    RecoveryMsg, RecoveryResponseMsg
}

Genesis == <<"Void", "Init">>
LogValues == Commands \cup {"Void", "Init"}

ToSet(s) == {s[i] : i \in 1..Len(s)}

ASSUME /\ Cardinality(Replicas) = 3
       /\ MaxLogLength >= Len(Genesis)

Primary(v) == ReplicaOrder[(v % Len(ReplicaOrder)) + 1]

IsQuorum(nodes) ==
    /\ nodes \subseteq Replicas
    /\ 2 * Cardinality(nodes) > Cardinality(Replicas)

Min(a, b) == IF a <= b THEN a ELSE b
Max(a, b) == IF a >= b THEN a ELSE b
SetMax(s) == CHOOSE x \in s : \A y \in s : y <= x

PrefixEqual(a, b, through) ==
    /\ through <= Len(a)
    /\ through <= Len(b)
    /\ \A i \in 1..through : a[i] = b[i]

(******************************************************************************
 * All message records have one shape.  Unused fields contain neutral values;
 * this keeps every perimeter total and makes malformed record access
 * unrepresentable in the model.
 *****************************************************************************)
Message(kind, from, to, view, slot, entry, history,
        retained, accepted, committed_, nonce) ==
    [type      |-> kind,
     from      |-> from,
     to        |-> to,
     view      |-> view,
     slot      |-> slot,
     entry     |-> entry,
     history   |-> history,
     retained  |-> retained,
     accepted  |-> accepted,
     committed |-> committed_,
     nonce     |-> nonce]

Prepare(p, r, v, n, command, k) ==
    Message(PrepareMsg, p, r, v, n, command, <<>>, 0, 0, k, 0)

PrepareOk(r, p, v, n) ==
    Message(PrepareOkMsg, r, p, v, n, "none", <<>>, 0, 0, 0, 0)

Commit(p, r, v, k) ==
    Message(CommitMsg, p, r, v, k, "none", <<>>, 0, 0, k, 0)

StartViewChangeVote(r, to, v) ==
    Message(StartViewChangeMsg, r, to, v, 0, "none", <<>>, 0, 0, 0, 0)

DoViewChange(r, p, v, retained_, history_, committed_) ==
    Message(DoViewChangeMsg, r, p, v, Len(history_), "none", history_,
            retained_, Len(history_), committed_, 0)

StartView(p, r, v, history_, committed_) ==
    Message(StartViewMsg, p, r, v, Len(history_), "none", history_,
            v, Len(history_), committed_, 0)

RecoveryRequest(r, to, nonce_) ==
    Message(RecoveryMsg, r, to, 0, 0, "none", <<>>, 0, 0, 0, nonce_)

RecoveryResponse(r, to, nonce_, v, history_, accepted_, committed_) ==
    Message(RecoveryResponseMsg, r, to, v, 0, "none", history_,
            v, accepted_, committed_, nonce_)

VARIABLES status,
          currentView,
          retainedView,
          logs,
          committed,
          applied,
          messages,
          epochs,
          recoveryEvidence

vars == <<status, currentView, retainedView, logs, committed, applied,
          messages, epochs, recoveryEvidence>>

Init ==
    /\ status       = [r \in Replicas |-> Normal]
    /\ currentView  = [r \in Replicas |-> 0]
    /\ retainedView = [r \in Replicas |-> 0]
    /\ logs         = [r \in Replicas |-> Genesis]
    /\ committed    = [r \in Replicas |-> Len(Genesis)]
    /\ applied      = [r \in Replicas |-> Len(Genesis)]
    /\ messages     = {}
    /\ epochs       = [r \in Replicas |-> 0]
    /\ recoveryEvidence = {}

(******************************************************************************
 * Normal operation: Propose -> Prepare -> PrepareOk -> Commit.  PrepareOk is
 * cumulative, matching src/replica/normal.rs: an acknowledgement for n also
 * covers every earlier accepted slot.
 *****************************************************************************)
Propose(p, command) ==
    /\ status[p] = Normal
    /\ p = Primary(currentView[p])
    /\ Len(logs[p]) < MaxLogLength
    /\ command \notin ToSet(logs[p])
    /\ LET n == Len(logs[p]) + 1
           outbound == {Prepare(p, r, currentView[p], n, command, committed[p]) :
                           r \in Replicas \ {p}}
       IN /\ logs' = [logs EXCEPT ![p] = Append(@, command)]
          /\ messages' = messages \cup outbound
    /\ UNCHANGED <<status, currentView, retainedView, committed, applied,
                   epochs, recoveryEvidence>>

ReceivePrepare(r, m) ==
    /\ m \in messages
    /\ m.type = PrepareMsg
    /\ m.to = r
    /\ status[r] = Normal
    /\ m.view = currentView[r]
    /\ m.from = Primary(m.view)
    /\ m.slot = Len(logs[r]) + 1
    /\ m.slot <= MaxLogLength
    /\ LET newLog == Append(logs[r], m.entry)
           learned == Max(committed[r], Min(m.committed, Len(newLog)))
       IN /\ logs' = [logs EXCEPT ![r] = newLog]
          /\ committed' = [committed EXCEPT ![r] = learned]
          /\ messages' = messages \cup {PrepareOk(r, m.from, m.view, m.slot)}
    /\ UNCHANGED <<status, currentView, retainedView, applied, epochs,
                   recoveryEvidence>>

AckSenders(p, v, n) ==
    {m.from : m \in {candidate \in messages :
        /\ candidate.type = PrepareOkMsg
        /\ candidate.to = p
        /\ candidate.view = v
        /\ candidate.slot >= n}}

CommitNext(p) ==
    /\ status[p] = Normal
    /\ p = Primary(currentView[p])
    /\ committed[p] < Len(logs[p])
    /\ LET n == committed[p] + 1
           support == {p} \cup AckSenders(p, currentView[p], n)
           outbound == {Commit(p, r, currentView[p], n) :
                           r \in Replicas \ {p}}
       IN /\ IsQuorum(support)
          /\ committed' = [committed EXCEPT ![p] = n]
          /\ messages' = messages \cup outbound
    /\ UNCHANGED <<status, currentView, retainedView, logs, applied, epochs,
                   recoveryEvidence>>

ReceiveCommit(r, m) ==
    /\ m \in messages
    /\ m.type = CommitMsg
    /\ m.to = r
    /\ status[r] = Normal
    /\ m.view = currentView[r]
    /\ m.from = Primary(m.view)
    /\ LET learned == Max(committed[r], Min(m.committed, Len(logs[r])))
       IN /\ learned > committed[r]
          /\ committed' = [committed EXCEPT ![r] = learned]
    /\ UNCHANGED <<status, currentView, retainedView, logs, applied,
                   messages, epochs, recoveryEvidence>>

(******************************************************************************
 * View change: currentView is the fence; retainedView is history provenance.
 * A report is sent only after a fence quorum.  The new primary chooses the
 * lexicographic maximum (retainedView, accepted), then carries the greatest
 * reported committed frontier without moving any local committed frontier
 * backward.
 *****************************************************************************)
ViewChangeBroadcast(r, v) ==
    {StartViewChangeVote(r, to, v) : to \in Replicas}

EnterViewChange(r, target) ==
    /\ target \in 0..MaxView
    /\ target > currentView[r]
    /\ status' = [status EXCEPT ![r] = ViewChange]
    /\ currentView' = [currentView EXCEPT ![r] = target]
    /\ messages' = messages \cup ViewChangeBroadcast(r, target)
    /\ UNCHANGED <<retainedView, logs, committed, applied, epochs,
                   recoveryEvidence>>

FollowHigherViewChange(r, m) ==
    /\ m \in messages
    /\ m.type = StartViewChangeMsg
    /\ m.to = r
    /\ m.view <= MaxView
    /\ m.view > currentView[r]
    /\ status' = [status EXCEPT ![r] = ViewChange]
    /\ currentView' = [currentView EXCEPT ![r] = m.view]
    /\ messages' = messages \cup ViewChangeBroadcast(r, m.view)
    /\ UNCHANGED <<retainedView, logs, committed, applied, epochs,
                   recoveryEvidence>>

FenceSenders(r, v) ==
    {m.from : m \in {candidate \in messages :
        /\ candidate.type = StartViewChangeMsg
        /\ candidate.to = r
        /\ candidate.view = v}}

ReportAlreadySent(r, v) ==
    \E m \in messages :
        /\ m.type = DoViewChangeMsg
        /\ m.from = r
        /\ m.view = v

SendDoViewChange(r) ==
    /\ status[r] = ViewChange
    /\ IsQuorum(FenceSenders(r, currentView[r]))
    /\ ~ReportAlreadySent(r, currentView[r])
    /\ LET report == DoViewChange(r,
                                  Primary(currentView[r]),
                                  currentView[r],
                                  retainedView[r],
                                  logs[r],
                                  committed[r])
       IN messages' = messages \cup {report}
    /\ UNCHANGED <<status, currentView, retainedView, logs, committed,
                   applied, epochs, recoveryEvidence>>

Reports(p, v) ==
    {m \in messages :
        /\ m.type = DoViewChangeMsg
        /\ m.to = p
        /\ m.view = v}

ReportSenders(p, v) == {m.from : m \in Reports(p, v)}
ReportRank(m) == m.retained * (MaxLogLength + 1) + m.accepted

InstallView(p, chosen) ==
    /\ status[p] = ViewChange
    /\ p = Primary(currentView[p])
    /\ LET reports == Reports(p, currentView[p])
           maxCommitted == SetMax({m.committed : m \in reports})
       IN /\ IsQuorum(ReportSenders(p, currentView[p]))
          /\ chosen \in reports
          /\ \A report \in reports : ReportRank(report) <= ReportRank(chosen)
          /\ chosen.accepted = Len(chosen.history)
          /\ maxCommitted <= chosen.accepted
          /\ PrefixEqual(chosen.history, logs[p], committed[p])
          /\ status' = [status EXCEPT ![p] = Normal]
          /\ retainedView' = [retainedView EXCEPT ![p] = currentView[p]]
          /\ logs' = [logs EXCEPT ![p] = chosen.history]
          /\ committed' = [committed EXCEPT ![p] = Max(@, maxCommitted)]
          /\ messages' = messages \cup
                 {StartView(p, r, currentView[p], chosen.history,
                            Max(committed[p], maxCommitted)) :
                    r \in Replicas \ {p}}
    /\ UNCHANGED <<currentView, applied, epochs, recoveryEvidence>>

ReceiveStartView(r, m) ==
    /\ m \in messages
    /\ m.type = StartViewMsg
    /\ m.to = r
    /\ m.from = Primary(m.view)
    /\ m.accepted = Len(m.history)
    /\ m.committed <= m.accepted
    /\ \/ m.view > currentView[r]
       \/ /\ m.view = currentView[r]
          /\ status[r] = ViewChange
    /\ m.committed >= committed[r]
    /\ PrefixEqual(m.history, logs[r], committed[r])
    /\ LET ack == IF m.accepted > m.committed
                  THEN {PrepareOk(r, m.from, m.view, m.accepted)}
                  ELSE {}
       IN messages' = messages \cup ack
    /\ status' = [status EXCEPT ![r] = Normal]
    /\ currentView' = [currentView EXCEPT ![r] = m.view]
    /\ retainedView' = [retainedView EXCEPT ![r] = m.view]
    /\ logs' = [logs EXCEPT ![r] = m.history]
    /\ committed' = [committed EXCEPT ![r] = m.committed]
    /\ recoveryEvidence' = {e \in recoveryEvidence : e.to # r}
    /\ UNCHANGED <<applied, epochs>>

(******************************************************************************
 * Crash and recovery: Crash is the volatile/amnesiac durability profile.  A
 * crashed identity is fenced before its log and frontiers are forgotten.
 * Recovery excludes the recovering replica from its quorum, chooses the
 * greatest reported currentView, and installs history only from that view's
 * primary.  This is src/replica/recovery.rs at full-history abstraction.
 *****************************************************************************)
Unready == {r \in Replicas : status[r] \in {Recovering, Replaying}}

Crash(r) ==
    /\ MaxEpoch > 0
    /\ status[r] \notin {Recovering, Replaying}
    /\ epochs[r] < MaxEpoch
    /\ \A s \in Replicas : epochs[s] = 0
    /\ 2 * (Cardinality(Unready) + 1) < Cardinality(Replicas)
    /\ status' = [status EXCEPT ![r] = Recovering]
    /\ currentView' = [currentView EXCEPT ![r] = 0]
    /\ retainedView' = [retainedView EXCEPT ![r] = 0]
    /\ logs' = [logs EXCEPT ![r] = Genesis]
    /\ committed' = [committed EXCEPT ![r] = Len(Genesis)]
    /\ applied' = [applied EXCEPT ![r] = Len(Genesis)]
    /\ epochs' = [epochs EXCEPT ![r] = @ + 1]
    /\ recoveryEvidence' = {e \in recoveryEvidence : e.to # r}
    /\ UNCHANGED messages

RecoveryStarted(r) ==
    \E m \in messages :
        /\ m.type = RecoveryMsg
        /\ m.from = r
        /\ m.nonce = epochs[r]

BeginRecovery(r) ==
    /\ status[r] = Recovering
    /\ ~RecoveryStarted(r)
    /\ messages' = messages \cup
           {RecoveryRequest(r, to, epochs[r]) : to \in Replicas \ {r}}
    /\ UNCHANGED <<status, currentView, retainedView, logs, committed,
                   applied, epochs, recoveryEvidence>>

RespondToRecovery(r, request) ==
    /\ request \in messages
    /\ request.type = RecoveryMsg
    /\ request.to = r
    /\ status[r] = Normal
    /\ LET history == IF r = Primary(currentView[r]) THEN logs[r] ELSE <<>>
           response == RecoveryResponse(r, request.from, request.nonce,
                                        currentView[r], history,
                                        Len(logs[r]), committed[r])
       IN messages' = messages \cup {response}
    /\ UNCHANGED <<status, currentView, retainedView, logs, committed,
                   applied, epochs, recoveryEvidence>>

RecoveryResponses(r) ==
    {m \in recoveryEvidence :
        /\ m.type = RecoveryResponseMsg
        /\ m.to = r
        /\ m.nonce = epochs[r]
        /\ m.from # r}

RecoveryResponders(r) == {m.from : m \in RecoveryResponses(r)}

RecordRecoveryResponse(r, response) ==
    /\ status[r] = Recovering
    /\ RecoveryStarted(r)
    /\ response \in messages
    /\ response.type = RecoveryResponseMsg
    /\ response.to = r
    /\ response.from # r
    /\ response.nonce = epochs[r]
    /\ response.committed >= committed[r]
    /\ LET prior == {e \in recoveryEvidence :
                        e.to = r /\ e.from = response.from}
           replacement == (recoveryEvidence \ prior) \cup {response}
       IN /\ replacement # recoveryEvidence
          /\ recoveryEvidence' = replacement
    /\ UNCHANGED <<status, currentView, retainedView, logs, committed,
                   applied, messages, epochs>>

CompleteRecovery(r, chosen) ==
    /\ status[r] = Recovering
    /\ LET responses == RecoveryResponses(r)
           latest == SetMax({m.view : m \in responses})
       IN /\ IsQuorum(RecoveryResponders(r))
          /\ chosen \in responses
          /\ chosen.view = latest
          /\ chosen.from = Primary(latest)
          /\ chosen.accepted = Len(chosen.history)
          /\ chosen.committed <= chosen.accepted
          /\ chosen.committed >= committed[r]
          /\ PrefixEqual(chosen.history, logs[r], committed[r])
          /\ status' = [status EXCEPT
                 ![r] = IF applied[r] = chosen.committed
                         THEN Normal ELSE Replaying]
          /\ currentView' = [currentView EXCEPT ![r] = latest]
          /\ retainedView' = [retainedView EXCEPT ![r] = latest]
          /\ logs' = [logs EXCEPT ![r] = chosen.history]
          /\ committed' = [committed EXCEPT ![r] = chosen.committed]
          /\ recoveryEvidence' = {e \in recoveryEvidence : e.to # r}
    /\ UNCHANGED <<applied, messages, epochs>>

(******************************************************************************
 * Application completion is a separate host input in the Rust API.  The
 * transition is monotone and a recovering replica remains fenced as Replaying
 * until every committed operation has been acknowledged by the application.
 *****************************************************************************)
ApplyNext(r) ==
    /\ status[r] \in {Normal, Replaying}
    /\ applied[r] < committed[r]
    /\ LET next == applied[r] + 1
       IN /\ applied' = [applied EXCEPT ![r] = next]
          /\ status' = [status EXCEPT
                 ![r] = IF status[r] = Replaying /\ next = committed[r]
                         THEN Normal ELSE @]
    /\ UNCHANGED <<currentView, retainedView, logs, committed,
                   messages, epochs, recoveryEvidence>>

Next ==
    \/ \E p \in Replicas, command \in Commands : Propose(p, command)
    \/ \E r \in Replicas, m \in messages : ReceivePrepare(r, m)
    \/ \E p \in Replicas : CommitNext(p)
    \/ \E r \in Replicas, m \in messages : ReceiveCommit(r, m)
    \/ \E r \in Replicas, target \in 0..MaxView : EnterViewChange(r, target)
    \/ \E r \in Replicas, m \in messages : FollowHigherViewChange(r, m)
    \/ \E r \in Replicas : SendDoViewChange(r)
    \/ \E p \in Replicas, chosen \in messages : InstallView(p, chosen)
    \/ \E r \in Replicas, m \in messages : ReceiveStartView(r, m)
    \/ \E r \in Replicas : Crash(r)
    \/ \E r \in Replicas : BeginRecovery(r)
    \/ \E r \in Replicas, m \in messages : RespondToRecovery(r, m)
    \/ \E r \in Replicas, m \in messages : RecordRecoveryResponse(r, m)
    \/ \E r \in Replicas, chosen \in messages : CompleteRecovery(r, chosen)
    \/ \E r \in Replicas : ApplyNext(r)

Spec == Init /\ [][Next]_vars

(******************************************************************************
 * Safety obligations.
 *****************************************************************************)
MessageTypeOK(m) ==
    /\ m.type \in MessageKinds
    /\ m.from \in Replicas
    /\ m.to \in Replicas
    /\ m.view \in 0..MaxView
    /\ m.slot \in 0..MaxLogLength
    /\ m.entry \in Commands \cup {"none"}
    /\ m.history \in Seq(LogValues)
    /\ Len(m.history) <= MaxLogLength
    /\ m.retained \in 0..MaxView
    /\ m.accepted \in 0..MaxLogLength
    /\ m.committed \in 0..MaxLogLength
    /\ m.nonce \in 0..MaxEpoch

TypeOK ==
    /\ status \in [Replicas -> Statuses]
    /\ currentView \in [Replicas -> 0..MaxView]
    /\ retainedView \in [Replicas -> 0..MaxView]
    /\ logs \in [Replicas -> Seq(LogValues)]
    /\ committed \in [Replicas -> 0..MaxLogLength]
    /\ applied \in [Replicas -> 0..MaxLogLength]
    /\ epochs \in [Replicas -> 0..MaxEpoch]
    /\ \A r \in Replicas : Len(logs[r]) <= MaxLogLength
    /\ \A m \in messages : MessageTypeOK(m)
    /\ recoveryEvidence \subseteq messages
    /\ \A m \in recoveryEvidence : m.type = RecoveryResponseMsg

FrontiersOrdered ==
    \A r \in Replicas : applied[r] <= committed[r] /\ committed[r] <= Len(logs[r])

NormalViewInstalled ==
    \A r \in Replicas : status[r] = Normal => currentView[r] = retainedView[r]

ViewChangeRetainsHistoryProvenance ==
    \A r \in Replicas : status[r] = ViewChange => retainedView[r] <= currentView[r]

CommittedLogsAgree ==
    \A r \in Replicas, s \in Replicas :
        PrefixEqual(logs[r], logs[s], Min(committed[r], committed[s]))

AppliedLogsAgree ==
    \A r \in Replicas, s \in Replicas :
        PrefixEqual(logs[r], logs[s], Min(applied[r], applied[s]))

CommittedEntrySurvives ==
    \A r \in Replicas :
        \A i \in 1..committed[r] :
            \E s \in Replicas : i <= Len(logs[s]) /\ logs[s][i] = logs[r][i]

ReplayingIsFenced ==
    \A r \in Replicas : status[r] = Replaying => applied[r] < committed[r]

=============================================================================
