-------------------------- MODULE VrrCoreReincarnation -------------------------
(***************************************************************************
 * Reduced design model of the crash-stop-self-evict reincarnation sequence
 * under leader replacement.
 *
 * Identities crash-stop: an identity marked dirty is dead under that
 * identity forever and sends nothing afterwards.  The victim crashes, is
 * fenced (bumped: weight forced to 0, recorded in bumped, never to vote
 * again), and the node reopens under the new identity d at weight 0.
 * Two committed eras follow: era 1 batches DECREMENT(Victim)+JOIN(d) (the
 * victim lands at 0, d joins at 0), era 2 batches INCREMENT(d)+
 * LEAVE(Victim) (d 0->1, the victim leaves at 0).
 *
 * Leadership is a variable.  A view change hands the leader role to a
 * live positive-weight member; the leader may also die.  Acknowledgements
 * are gathered per leader and per era: an ack answers one announcement,
 * is addressed to the announcing leader, is stamped with the announced
 * era, and is counted only by that leader while that era is current.  A
 * successor leader therefore announces and gathers its own acks; stale
 * acks addressed to an earlier leader or era are never counted.  A
 * recorded ack stays valid regardless of the sender's later status.  The
 * message set is monotone: no message is ever removed, modelling
 * reordering, duplication, and loss without transport state.
 *
 * Scenario constants:
 *   Members    -- the original voting identities;
 *   Victim     -- the member that crashes and is fenced;
 *   StartWt    -- the initial voting weight of every original member
 *                 (the reborn identity always starts at 0);
 *   CrashFirst -- when TRUE, era 1 may commit only before a leader crash
 *                 and era 2 only after one, forcing the leader-crash-
 *                 between-eras path (the remaining era is recomputed
 *                 idempotently from committed state by the successor);
 *   MaxViews   -- bound on view changes that replace a live leader;
 *                 replacing a dead leader is never rationed;
 *   MaxDead    -- bound on the number of dead identities, the victim
 *                 included;
 *   Defect     -- a single weakened guard per mutation config; "none" is
 *                 the intact design.  Mass is the weight delta an era
 *                 produces on each node.
 ***************************************************************************)

EXTENDS FiniteSets, Naturals, TLC

CONSTANTS Members, Victim, StartWt, CrashFirst, Defect, MaxViews, MaxDead

Reborn        == "d"
AllIds        == Members \cup {Reborn}
MaxEra        == 2
InitialLeader == "a"
NoLeader      == "none"

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

VARIABLES wt, marks, bumped, era, msgs, leader, views, quorums, leaders,
          moved, crashed, aborted

vars == <<wt, marks, bumped, era, msgs, leader, views, quorums, leaders,
          moved, crashed, aborted>>

RECURSIVE SumWt(_)
SumWt(s) ==
    IF s = {} THEN 0
    ELSE LET x == CHOOSE i \in s : TRUE
         IN wt[x] + SumWt(s \ {x})

(* Strict majority over the current weight table. *)
IsStrictMajority(s) == 2 * SumWt(s) > SumWt(Members)

(* Dead identities: crash-stop, never clean again under that identity. *)
Dead == {i \in AllIds : marks[i] = Dirty}

(* Acks the current leader counts for the current era: addressed to this
 * leader, stamped with this era, from an identity whose current weight
 * is nonzero.  Replies from weight-0 identities are discarded at ingress
 * and never counted; the sender's marks are not rechecked. *)
CountedAcks == {m.from : m \in {r \in msgs :
                   /\ r.type = AckKind
                   /\ r.to = leader
                   /\ r.era = era
                   /\ (Defect = CountStandby \/ wt[r.from] > 0)}}

Support == {leader} \cup CountedAcks

ZeroMoved == [i \in AllIds |-> 0]

Init ==
    /\ wt       = [i \in AllIds |-> IF i \in Members THEN StartWt ELSE 0]
    /\ marks    = [i \in AllIds |-> Clean]
    /\ bumped   = {}
    /\ era      = 0
    /\ msgs     = {}
    /\ leader   = InitialLeader
    /\ views    = 0
    /\ quorums  = [e \in 1..MaxEra |-> {}]
    /\ leaders  = [e \in 1..MaxEra |-> NoLeader]
    /\ moved    = [e \in 1..MaxEra |-> ZeroMoved]
    /\ crashed  = FALSE
    /\ aborted  = FALSE
    /\ Defect \in Defects

(***************************************************************************
 * Crash: the identity dies and stays dead.  The victim's crash opens the
 * scenario; any other identity may die only once the victim has, bounded
 * by MaxDead.  Crashing the leader raises the crashed flag that the
 * CrashFirst scenario keys on.
 ***************************************************************************)

Crash(id) ==
    /\ marks[id] = Clean
    /\ id = Victim \/ marks[Victim] = Dirty
    /\ Cardinality(Dead) < MaxDead
    /\ marks'   = [marks EXCEPT ![id] = Dirty]
    /\ crashed' = (crashed \/ id = leader)
    /\ UNCHANGED <<wt, bumped, era, msgs, leader, views, quorums, leaders,
                   moved, aborted>>

(***************************************************************************
 * View change: a live positive-weight member that has not been fenced
 * takes the leader role.  Replacing a live leader is rationed by
 * MaxViews; replacing a dead leader is never rationed, because that is
 * the view-change protocol's liveness obligation, and it is bounded by
 * MaxDead since each such change consumes a leader death.
 ***************************************************************************)

ViewChange(l) ==
    /\ l \in Members
    /\ l # leader
    /\ marks[l] = Clean
    /\ wt[l] > 0
    /\ l \notin bumped
    /\ views < MaxViews \/ marks[leader] = Dirty
    /\ leader' = l
    /\ views'  = views + 1
    /\ UNCHANGED <<wt, marks, bumped, era, msgs, quorums, leaders, moved,
                   crashed, aborted>>

(***************************************************************************
 * Bump: the dead victim identity is fenced (weight forced to 0, recorded
 * in bumped, never to vote again) and the node reopens under the new
 * identity d, which starts at weight 0 until it joins.  The victim
 * identity stays dead.
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
    /\ UNCHANGED <<marks, era, msgs, leader, views, quorums, leaders, moved,
                   crashed, aborted>>

(***************************************************************************
 * Reincarnate announcement: the current leader advertises the new
 * identity, stamped with the current era.
 ***************************************************************************)

Announce ==
    /\ bumped # {} \/ Defect = DirtyNoBump \/ Defect = OneEraSwap
    /\ marks[leader] = Clean
    /\ msgs' = msgs \cup
            {[type |-> AnnounceKind, from |-> leader, to |-> m, era |-> era] :
                m \in AllIds \ {leader}}
    /\ UNCHANGED <<wt, marks, bumped, era, leader, views, quorums, leaders,
                   moved, crashed, aborted>>

(***************************************************************************
 * Reply ingress.  A live positive-weight identity answers one
 * announcement: the ack is addressed to the announcing leader and stamped
 * with the announced era.  A weight-0 identity's reply is discarded
 * (StandbyDiscard, a stuttering step: the message never enters the
 * monotone set).  A dead identity sends nothing.
 ***************************************************************************)

Ack(id) ==
    /\ marks[id] = Clean
    /\ (wt[id] > 0 \/ Defect = CountStandby)
    /\ \E m \in msgs :
           /\ m.type = AnnounceKind
           /\ m.to = id
           /\ msgs' = msgs \cup
                {[type |-> AckKind, from |-> id, to |-> m.from, era |-> m.era]}
    /\ UNCHANGED <<wt, marks, bumped, era, leader, views, quorums, leaders,
                   moved, crashed, aborted>>

StandbyDiscard(id) ==
    /\ \E m \in msgs :
           /\ m.type = AnnounceKind
           /\ m.to = id
    /\ wt[id] = 0
    /\ UNCHANGED vars

(***************************************************************************
 * Era 1 commit: batch DECREMENT(Victim) + JOIN(d).  The victim lands at 0
 * (mass 1), d joins at 0 (mass 0).  Guarded by a live leader and a commit
 * quorum over the current weight table; the per-node mass bound is
 * ASSERTED.  The committing leader is recorded alongside its quorum.
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
    /\ leaders' = [leaders EXCEPT ![1] = leader]
    /\ moved'   = IF Defect = LeaveNonzero
                  THEN moved
                  ELSE [moved EXCEPT ![1] = [moved[1] EXCEPT ![Victim] = 1]]
    /\ era'     = 1
    /\ UNCHANGED <<marks, bumped, msgs, leader, views, crashed, aborted>>

(***************************************************************************
 * Era 2 commit: batch INCREMENT(d) + LEAVE(Victim).  d 0->1 (mass 1), the
 * victim leaves at 0 (mass 0).  Same quorum and mass discipline.  Both
 * commit actions depend only on current state and assign (rather than
 * accumulate) their effects, so a successor leader recomputes the
 * remaining era idempotently from committed state.  Under CrashFirst,
 * era 2 may commit only after a leader crash.
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
    /\ leaders' = [leaders EXCEPT ![2] = leader]
    /\ moved'   = [moved EXCEPT ![2] =
                   [moved[2] EXCEPT
                        ![Victim] = IF Defect = LeaveNonzero THEN wt[Victim]
                                    ELSE moved[2][Victim],
                        ![Reborn] = 1]]
    /\ era'     = 2
    /\ UNCHANGED <<marks, bumped, msgs, leader, views, crashed, aborted>>

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
    /\ leaders' = [leaders EXCEPT ![1] = leader]
    /\ moved'   = [moved EXCEPT ![1] = [moved[1] EXCEPT
                        ![Victim] = wt[Victim], ![Reborn] = 1]]
    /\ era'     = 1
    /\ UNCHANGED <<marks, bumped, msgs, leader, views, crashed, aborted>>

(***************************************************************************
 * Mutation path: the forced sequence aborts after era 1 without ever
 * proposing era 2.  The intact design never takes this action.
 ***************************************************************************)

AbortSequence ==
    /\ Defect = AbortMidway
    /\ era = 1
    /\ ~aborted
    /\ aborted' = TRUE
    /\ UNCHANGED <<wt, marks, bumped, era, msgs, leader, views, quorums,
                   leaders, moved, crashed>>

Next ==
    \/ \E id \in AllIds : Crash(id)
    \/ \E l \in Members : ViewChange(l)
    \/ Bump
    \/ Announce
    \/ \E id \in AllIds : Ack(id)
    \/ \E id \in AllIds : StandbyDiscard(id)
    \/ LeaderCommitEra1
    \/ LeaderCommitEra2
    \/ LeaderCommitSwap
    \/ AbortSequence

Spec == Init /\ [][Next]_vars

(* Weak fairness on the whole step relation: a state-changing step that
 * stays enabled is eventually taken.  Used only by the liveness configs. *)
FairSpec == Spec /\ WF_vars(Next)

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
    /\ leader  \in Members
    /\ views   \in 0..(MaxViews + MaxDead)
    /\ quorums \in [1..MaxEra -> SUBSET AllIds]
    /\ leaders \in [1..MaxEra -> Members \cup {NoLeader}]
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

(* No reply from a weight-0 identity is counted in any committed quorum:
 * every quorum member other than the committing leader has on file an
 * ack addressed to that leader for the era the commit was gathered in,
 * and holds positive weight. *)
NoStandbyVote ==
    \A e \in 1..MaxEra :
        \A i \in quorums[e] :
            i = leaders[e] \/ \E m \in msgs :
                /\ m.type = AckKind
                /\ m.from = i
                /\ m.to = leaders[e]
                /\ m.era = e - 1
                /\ wt[i] > 0

(* Identity rule: the reborn identity may hold weight only once the
 * victim identity has been fenced (bumped). *)
RebornAfterFence == wt[Reborn] > 0 => Victim \in bumped

(* Continuation rule: a committed forced sequence is never abandoned
 * before its final era commits. *)
SequenceCompletes == ~aborted \/ quorums[2] # {}

(* A live leader is a voter: positive weight and never fenced.  A dead
 * leader holds the role only until it is replaced and commits nothing. *)
LeaderIsVoter == marks[leader] = Clean => (wt[leader] > 0 /\ leader \notin bumped)

(***************************************************************************
 * Liveness obligation, checked under FairSpec: the forced sequence
 * reaches its final era.  It holds while the live voters keep a strict
 * majority and is the expected-failure witness when they do not.
 ***************************************************************************)

Completes == <>(era = 2)

=============================================================================
