-------------------------- MODULE VrrCoreReincarnation -------------------------
(***************************************************************************
 * Reduced design model of the crash-stop-self-evict reincarnation sequence.
 *
 * The voting identities crash-stop: one member crashes with a dirty
 * superblock, is fenced (bumped), and reopens under the new identity d.
 * Two committed eras follow: era 1 batches DECREMENT(Victim)+JOIN(d)
 * (the victim lands at 0, d joins at 0), era 2 batches INCREMENT(d)+
 * LEAVE(Victim) (d 0->1, the victim leaves at 0).  The message set is
 * monotone: no message is ever removed, modelling reordering,
 * duplication, and loss without transport state.
 *
 * Scenario constants:
 *   Members    -- the original voting identities;
 *   Victim     -- the member that crashes dirty and is fenced;
 *   StartWt    -- the initial voting weight of every original member
 *                 (the reborn identity always starts at 0);
 *   CrashFirst -- when TRUE, era 1 may commit only before a mid-sequence
 *                 leader crash and era 2 only after one, forcing the
 *                 leader-crash-between-eras path (the remaining era is
 *                 recomputed idempotently from current state);
 *   Defect     -- a single weakened guard per mutation config; "none" is
 *                 the intact design.  Mass is the weight delta an era
 *                 produces on each node.
 ***************************************************************************)

EXTENDS FiniteSets, Naturals, TLC

CONSTANTS Members, Victim, StartWt, CrashFirst, Defect

Reborn   == "d"
AllIds   == Members \cup {Reborn}
MaxEra   == 2
Leader   == "a"

Weights  == 0..2
Clean    == "clean"
Dirty    == "dirty"
Marks    == {Clean, Dirty}

NoDefect     == "none"
LeaveNonzero == "leave-nonzero"
OneEraSwap   == "one-era-swap"
DirtyNoBump  == "dirty-no-bump"
AbortMidway  == "abort-midway"
CountStandby == "count-standby"

Defects == {NoDefect, LeaveNonzero, OneEraSwap, DirtyNoBump,
            AbortMidway, CountStandby}

AnnounceKind == "reincarnate"
AckKind      == "ack"
MsgKinds     == {AnnounceKind, AckKind}

VARIABLES wt, marks, bumped, era, msgs, leader, quorums, moved, crashed, aborted

vars == <<wt, marks, bumped, era, msgs, leader, quorums, moved, crashed, aborted>>

RECURSIVE SumWt(_)
SumWt(s) ==
    IF s = {} THEN 0
    ELSE LET x == CHOOSE i \in s : TRUE
         IN wt[x] + SumWt(s \ {x})

(* Strict majority over the current weight table. *)
IsStrictMajority(s) == 2 * SumWt(s) > SumWt(Members)

(* Quorum support counts only the leader plus replies from identities
 * whose current weight is nonzero; replies from weight-0 identities are
 * discarded at ingress and never counted. *)
CountedAcks == {m.from : m \in {r \in msgs :
                   /\ r.type = AckKind
                   /\ r.to = Leader
                   /\ (Defect = CountStandby \/ wt[r.from] > 0)}}

Support == {Leader} \cup CountedAcks

ZeroMoved == [i \in AllIds |-> 0]

Init ==
    /\ wt       = [i \in AllIds |-> IF i \in Members THEN StartWt ELSE 0]
    /\ marks    = [i \in AllIds |-> Clean]
    /\ bumped   = {}
    /\ era      = 0
    /\ msgs     = {}
    /\ leader   = Leader
    /\ quorums  = [e \in 1..MaxEra |-> {}]
    /\ moved    = [e \in 1..MaxEra |-> ZeroMoved]
    /\ crashed  = FALSE
    /\ aborted  = FALSE
    /\ Defect \in Defects

(***************************************************************************
 * Superblock lifecycle.  DirtyRestart records an unflushed superblock;
 * RestartFlush records the flush.
 ***************************************************************************)

DirtyRestart(id) ==
    /\ marks[id] = Clean
    /\ marks' = [marks EXCEPT ![id] = Dirty]
    /\ UNCHANGED <<wt, bumped, era, msgs, leader, quorums, moved, crashed, aborted>>

RestartFlush(id) ==
    /\ marks[id] = Dirty
    /\ marks' = [marks EXCEPT ![id] = Clean]
    /\ UNCHANGED <<wt, bumped, era, msgs, leader, quorums, moved, crashed, aborted>>

(***************************************************************************
 * Bump: the dirty identity is fenced (weight forced to 0, recorded in
 * bumped, never to vote again) and the node reopens under the new
 * identity (the victim reopens as d, which starts at weight 0 until it
 * joins).
 ***************************************************************************)

Bump ==
    /\ marks[Victim] = Dirty
    /\ wt[Victim] > 0
    /\ Victim \notin bumped
    /\ era < MaxEra
    /\ wt'      = IF Defect = LeaveNonzero
                  THEN wt
                  ELSE [wt EXCEPT ![Victim] = 0]
    /\ bumped'  = bumped \cup {Victim}
    /\ marks'   = [marks EXCEPT ![Victim] = Clean]
    /\ UNCHANGED <<era, msgs, leader, quorums, moved, crashed, aborted>>

(***************************************************************************
 * Reincarnate announcement: the leader advertises the new identity.
 ***************************************************************************)

Announce ==
    /\ bumped # {} \/ Defect = DirtyNoBump \/ Defect = OneEraSwap
    /\ marks[leader] = Clean
    /\ msgs' = msgs \cup
            {[type |-> AnnounceKind, from |-> leader, to |-> m, era |-> era] :
                m \in AllIds \ {leader}}
    /\ UNCHANGED <<wt, marks, bumped, era, leader, quorums, moved, crashed, aborted>>

(***************************************************************************
 * Reply ingress.  A weight-0 identity's reply is discarded (StandbyDiscard,
 * a stuttering step: the message never enters the monotone set).  A
 * positive-weight identity's reply is recorded.
 ***************************************************************************)

Ack(id) ==
    /\ \E m \in msgs :
           /\ m.type = AnnounceKind
           /\ m.to = id
    /\ (wt[id] > 0 \/ Defect = CountStandby)
    /\ msgs' = msgs \cup
            {[type |-> AckKind, from |-> id, to |-> leader, era |-> era]}
    /\ UNCHANGED <<wt, marks, bumped, era, leader, quorums, moved, crashed, aborted>>

StandbyDiscard(id) ==
    /\ \E m \in msgs :
           /\ m.type = AnnounceKind
           /\ m.to = id
    /\ wt[id] = 0
    /\ UNCHANGED vars

(***************************************************************************
 * Era 1 commit: batch DECREMENT(Victim) + JOIN(d).  The victim lands at 0
 * (mass 1), d joins at 0 (mass 0).  Guarded by a commit quorum over the
 * current weight table; the per-node mass bound is ASSERTED.
 ***************************************************************************)

LeaderCommitEra1 ==
    /\ era = 0
    /\ bumped # {} \/ Defect = DirtyNoBump
    /\ marks[leader] = Clean
    /\ ~CrashFirst \/ ~crashed
    /\ IsStrictMajority(Support)
    /\ Assert(\A i \in AllIds :
                  (IF i = Victim THEN 1 ELSE 0) <= 1,
              "mass: era1 batch per-node mass <= 1")
    /\ wt'      = IF Defect = LeaveNonzero
                  THEN [wt EXCEPT ![Reborn] = 0]
                  ELSE [wt EXCEPT ![Victim] = 0, ![Reborn] = 0]
    /\ quorums' = [quorums EXCEPT ![1] = Support]
    /\ moved'   = IF Defect = LeaveNonzero
                  THEN moved
                  ELSE [moved EXCEPT ![1] = [moved[1] EXCEPT ![Victim] = 1]]
    /\ era'     = 1
    /\ UNCHANGED <<marks, bumped, msgs, leader, crashed, aborted>>

(***************************************************************************
 * Era 2 commit: batch INCREMENT(d) + LEAVE(Victim).  d 0->1 (mass 1), the
 * victim leaves at 0 (mass 0).  Same quorum and mass discipline.  Both
 * actions depend only on current state and assign (rather than
 * accumulate) their effects, so recomputation after a leader crash
 * mid-way is idempotent.  Under CrashFirst, era 2 may commit only after
 * a mid-sequence leader crash.
 ***************************************************************************)

LeaderCommitEra2 ==
    /\ era = 1
    /\ marks[leader] = Clean
    /\ ~CrashFirst \/ crashed
    /\ IsStrictMajority(Support)
    /\ Assert(\A i \in AllIds :
                  (IF i = Reborn THEN 1 ELSE 0) <= 1,
              "mass: era2 batch per-node mass <= 1")
    /\ wt'      = [wt EXCEPT ![Victim] = 0, ![Reborn] = 1]
    /\ quorums' = [quorums EXCEPT ![2] = Support]
    /\ moved'   = [moved EXCEPT ![2] =
                   [moved[2] EXCEPT
                        ![Victim] = IF Defect = LeaveNonzero THEN wt[Victim]
                                    ELSE moved[2][Victim],
                        ![Reborn] = 1]]
    /\ era'     = 2
    /\ UNCHANGED <<marks, bumped, msgs, leader, crashed, aborted>>

(***************************************************************************
 * Mutation path: the one-era identity swap.  The intact design never
 * takes this action: the old identity is evicted at its full weight and
 * the new identity is promoted in one committed era, moving the old
 * identity's whole weight in a single era.
 ***************************************************************************)

LeaderCommitSwap ==
    /\ Defect = OneEraSwap
    /\ era = 0
    /\ marks[leader] = Clean
    /\ IsStrictMajority(Support)
    /\ wt'      = [wt EXCEPT ![Victim] = 0, ![Reborn] = 1]
    /\ quorums' = [quorums EXCEPT ![1] = Support]
    /\ moved'   = [moved EXCEPT ![1] = [moved[1] EXCEPT
                        ![Victim] = wt[Victim], ![Reborn] = 1]]
    /\ era'     = 1
    /\ UNCHANGED <<marks, bumped, msgs, leader, crashed, aborted>>

(***************************************************************************
 * Mutation path: the forced sequence aborts after era 1 without ever
 * proposing era 2.  The intact design never takes this action.
 ***************************************************************************)

AbortSequence ==
    /\ Defect = AbortMidway
    /\ era = 1
    /\ ~aborted
    /\ aborted' = TRUE
    /\ UNCHANGED <<wt, marks, bumped, era, msgs, leader, quorums, moved,
                   crashed>>

(***************************************************************************
 * Leader crash between eras; the remaining eras are recomputed by the
 * same guarded commit actions above, which are idempotent.
 ***************************************************************************)

LeaderCrashMid ==
    /\ bumped # {}
    /\ marks[leader] = Clean
    /\ marks' = [marks EXCEPT ![leader] = Dirty]
    /\ crashed' = TRUE
    /\ UNCHANGED <<wt, bumped, era, msgs, leader, quorums, moved, aborted>>

Next ==
    \/ \E id \in AllIds : DirtyRestart(id)
    \/ \E id \in AllIds : RestartFlush(id)
    \/ Bump
    \/ Announce
    \/ \E id \in AllIds : Ack(id)
    \/ \E id \in AllIds : StandbyDiscard(id)
    \/ LeaderCommitEra1
    \/ LeaderCommitEra2
    \/ LeaderCommitSwap
    \/ AbortSequence
    \/ LeaderCrashMid

Spec == Init /\ [][Next]_vars

(***************************************************************************
 * Safety obligations.
 ***************************************************************************)

TypeOK ==
    /\ wt      \in [AllIds -> Weights]
    /\ marks   \in [AllIds -> Marks]
    /\ bumped  \subseteq Members
    /\ era     \in 0..MaxEra
    /\ msgs    \subseteq [type : MsgKinds, from : AllIds, to : AllIds,
                          era : 0..MaxEra]
    /\ leader  = Leader
    /\ quorums \in [1..MaxEra -> SUBSET AllIds]
    /\ moved   \in [1..MaxEra -> [AllIds -> 0..2]]
    /\ crashed \in BOOLEAN
    /\ aborted \in BOOLEAN

(* Consecutive committed eras used strict majorities that overlap. *)
FrownChain ==
    /\ \A e \in 1..MaxEra :
           quorums[e] # {} => IsStrictMajority(quorums[e])
    /\ \A e \in 1..(MaxEra - 1) :
           (quorums[e] # {} /\ quorums[e + 1] # {})
               => quorums[e] \cap quorums[e + 1] # {}

(* Every bumped identity is at weight 0 and never regains weight. *)
EvictedNeverVoter == \A i \in bumped : wt[i] = 0

(* Each committed era moved at most mass 1 per node. *)
MassRule == \A e \in 1..MaxEra : \A i \in AllIds : moved[e][i] <= 1

(* No reply from a weight-0 identity is counted in any committed quorum. *)
NoStandbyVote ==
    \A e \in 1..MaxEra :
        \A i \in quorums[e] :
            i = Leader \/ \E m \in msgs :
                /\ m.type = AckKind
                /\ m.from = i
                /\ wt[i] > 0

(* Identity rule: the reborn identity may hold weight only once the
 * victim identity has been fenced (bumped). *)
RebornAfterFence == wt[Reborn] > 0 => Victim \in bumped

(* Continuation rule: a committed forced sequence is never abandoned
 * before its final era commits. *)
SequenceCompletes == ~aborted \/ quorums[2] # {}

=============================================================================
