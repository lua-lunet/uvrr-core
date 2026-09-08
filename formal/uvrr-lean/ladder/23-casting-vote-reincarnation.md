# Rung 23: the casting vote instantiated on the reincarnation eras

*2026-09-08T06:35:08Z by Showboat 0.6.1*
<!-- showboat-id: 03517ca9-1304-4a3d-bc44-a7713cf108d8 -->

This rung instantiates rung 6's casting vote on the reincarnation eras of rung 22. The load-bearing case is the TWO-NODE cluster: identity 0 leads {0, 1} at unit weight, 1 is dead and reincarnates as 2, and the forced sequence runs E0 (1,1) → E1 (1,0,2:0) → E2 (1,∅,2:1). E0's only majority {0,1} cannot form while 1 is dead, so the leader commits the crossing batch under E1, whose total voting weight is 1 carried by 0 alone. Kernel-checked at the concrete configurations: every E0 majority contains both cluster members (unit2_majority_both), every E1 majority contains the leader (c1_majority_leader), and the casting-vote shape holds at the leader across the boundary — {0,1} is an E0 majority, {0} an E1 majority, and they intersect in the leader alone (two_casting_vote). Because every quorum pair of the two families meets at the leader, the E0/E1 overlap (rung 9's Frown obligation at this boundary) holds exactly through the casting-vote structure (two_e0_e1_safe). Two small general shapes are proved: a leader contained in every quorum of both families is decisive on both sides (meets_at_leader), and when the new era's phase-I quorum is the leader alone the casting vote reduces to the leader's membership in the old quorum (casting_vote_singleton). The THREE-NODE degenerate case is stated as the honest negative. Cluster {0,1,2} at unit weight, 2 dead and reincarnated as 3: the forming majority {0,1} is a majority in E0, in E1 and in E2 alike (degenerate_forming) — the old and new quorum sets that form coincide, so the transition is safe WITHOUT a pivot; the coinciding quorum has two members and admits no casting vote at any node (degenerate_no_pivot). A reader must not infer that the three-node path exercises the casting vote: it does not.

```bash
cat UVRR/CastingVoteReincarnation.lean
```

```output
import UVRR.CastingVote
import UVRR.Reincarnation

/-! uVRR casting vote on the reincarnation eras: the leader-overlap path of
the forced two-era sequence. The crossing era is the era boundary a leader
must bridge with its casting vote (rung 6's `HasCastingVote`): the old era's
phase-II quorum and the new era's phase-I quorum must meet, and in the
two-node cluster they meet in the leader alone. The two-node unit-weight
cluster is the load-bearing case: identity `0` leads a cluster `{0, 1}`; `1`
is dead and reincarnates as `2`. The old era's only majority `{0, 1}` cannot
form (`1` is dead), so the leader commits the crossing batch under the new
era whose minimal majority is `{0}` — its own single vote. Both cases here
are finite, kernel-checked instances: the majority families are rung 9's
weighted majorities over the fixed node lists, and the casting vote is
rung 6's, instantiated at the concrete leader. The three-node degenerate
case is stated as the honest negative: its forming majorities coincide
between the old and new eras, so its boundary needs no casting vote and
exercises none — a reader must not infer otherwise. General protocol
theorems over arbitrary configurations and the wire protocol are proof
obligations recorded in the ladder rung, not claims made here.
-/

namespace CastingVoteReincarnation

/-! ### The two-node leader-overlap case -/

/-- The finite support: identities `0`, `1` vote in the old era; identity `2`
is the reincarnated node. -/
def nodes2 : List Nat := [0, 1, 2]

/-- E0: the two-member cluster at unit weight. Its only majority is `{0, 1}`,
which cannot form while `1` is dead. -/
def unit2 : Reincarnation.Config Nat := fun a => if a = 0 ∨ a = 1 then 1 else 0

/-- E1: the crossing era `DECREMENT(1), JOIN(2)` — total weight 1, so every
majority contains the leader `0`. -/
def c1 : Reincarnation.Config Nat := Reincarnation.crossEra 1 2 unit2

/-- E2: the eviction era `INCREMENT(2), LEAVE(1)`. -/
def c2 : Reincarnation.Config Nat := Reincarnation.evictEra 2 c1

theorem unit2_total : WeightedGeneral.total nodes2 unit2 = 2 := by
  simp [nodes2, WeightedGeneral.total, unit2]

theorem c1_membership : c1 0 = 1 ∧ c1 1 = 0 ∧ c1 2 = 0 := by
  unfold c1 Reincarnation.crossEra unit2
  simp

theorem c2_membership : c2 0 = 1 ∧ c2 2 = 1 ∧ ¬ Reincarnation.voting c2 1 := by
  unfold c2 Reincarnation.evictEra c1 Reincarnation.crossEra unit2
  simp [Reincarnation.voting]

/-- Every E0 majority contains both cluster members: the old era's phase-II
quorum obligation is exactly the pair `{0, 1}`. -/
theorem unit2_majority_both {q : NSet Nat}
    (hm : WeightedGeneral.majority nodes2 unit2 q) : q 0 ∧ q 1 := by
  by_cases h0 : q 0 <;>
    by_cases h1 : q 1 <;>
      simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total, nodes2,
        unit2, h0, h1] at hm ⊢

/-- Every E1 majority contains the leader: with total weight 1, the leader's
unit is the majority. -/
theorem c1_majority_leader {q : NSet Nat}
    (hm : WeightedGeneral.majority nodes2 c1 q) : q 0 := by
  by_cases h0 : q 0 <;>
    simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total, nodes2,
      c1, Reincarnation.crossEra, unit2, h0] at hm ⊢

/-- The casting vote at the leader across the E0/E1 boundary: `{0, 1}` is an
E0 majority, `{0}` is an E1 majority, and they intersect in the leader alone.
`0` sends `prepare` to nobody but itself and promises at a moment of its own
choosing (rung 6's schedule, instantiated). -/
theorem two_casting_vote :
    WeightedGeneral.majority nodes2 unit2 (fun a : Nat => a = 0 ∨ a = 1) ∧
    WeightedGeneral.majority nodes2 c1 (fun a => a = 0) ∧
    CastingVote.HasCastingVote (fun a : Nat => a = 0 ∨ a = 1) (fun a => a = 0) 0 := by
  refine ⟨?_, ?_, ?_⟩
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total, nodes2, unit2]
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total, nodes2,
      c1, Reincarnation.crossEra, unit2]
  · intro a
    constructor
    · intro ⟨hqa, ha_eq⟩
      exact ha_eq
    · intro ha_eq
      subst ha_eq
      exact ⟨Or.inl rfl, rfl⟩

/-- General shape: if every quorum of both families contains `ℓ`, then the two
families frown and every quorum pair meets at `ℓ` — the leader's vote is
decisive on both sides of the boundary. -/
theorem meets_at_leader {A : Type} {Q0 Q1 : QSys A} {ℓ : A}
    (h0 : ∀ q, Q0 q → q ℓ) (h1 : ∀ q, Q1 q → q ℓ) :
    Frown Q0 Q1 ∧ ∀ q q', Q0 q → Q1 q' → q ℓ ∧ q' ℓ := by
  refine ⟨?_, ?_⟩
  · intro q1 q2 hq1 hq2
    refine ⟨ℓ, h0 q1 hq1, h1 q2 hq2⟩
  · intro q q' hq hq'
    exact ⟨h0 q hq, h1 q' hq'⟩

/-- General shape: when the new era's phase-I quorum is the leader alone, the
casting vote reduces to the leader's membership in the old quorum. -/
theorem casting_vote_singleton {A : Type} {q : NSet A} {ℓ : A} (h : q ℓ) :
    CastingVote.HasCastingVote q (fun a => a = ℓ) ℓ := by
  intro a
  constructor
  · intro ⟨hqa, ha_eq⟩
    exact ha_eq
  · intro ha_eq
    rw [ha_eq]
    exact ⟨h, rfl⟩

/-- Era safety E0 → E1 in the two-node case: the overlap holds exactly
because every quorum pair meets at the leader — the casting-vote structure
is what carries the boundary. -/
theorem two_e0_e1_safe :
    Frown (WeightedGeneral.majority nodes2 unit2) (WeightedGeneral.majority nodes2 c1) := by
  intro q1 q2 hq1 hq2
  have hq1' := unit2_majority_both hq1
  have hq0 := hq1'.1
  have hq2_0 : q2 0 := by
    by_cases h0 : q2 0 <;>
      simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total, nodes2,
        c1, Reincarnation.crossEra, unit2, h0] at hq2 ⊢
  refine ⟨0, hq0, hq2_0⟩

/-! ### The three-node degenerate case -/

/-- The degenerate forming quorum: the surviving pair `{0, 1}` (`2` is dead
and reincarnates as `3`). This is rung 22's three-node scenario, reused. -/
def forming : NSet Nat := fun a => a = 0 ∨ a = 1

/-- In the crossing era the total voting weight is 2 carried by `{0, 1}`: the
only majorities that form contain the pair. -/
theorem e1_majority_pair {q : NSet Nat}
    (hm : WeightedGeneral.majority Reincarnation.nodes Reincarnation.e1 q) :
    q 0 ∧ q 1 := by
  by_cases h0 : q 0 <;>
    by_cases h1 : q 1 <;>
      simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
        Reincarnation.nodes, Reincarnation.e1, Reincarnation.crossEra,
        Reincarnation.unit3, h0, h1] at hm ⊢

/-- The degenerate case's forming quorum `{0, 1}` is a majority in E0, in E1
and in E2 alike: the same quorum set forms throughout the forced sequence,
which is why this path needs no pivot. -/
theorem degenerate_forming :
    WeightedGeneral.majority Reincarnation.nodes Reincarnation.unit3 forming ∧
    WeightedGeneral.majority Reincarnation.nodes Reincarnation.e1 forming ∧
    WeightedGeneral.majority Reincarnation.nodes Reincarnation.e2 forming := by
  refine ⟨?_, ?_, ?_⟩
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      Reincarnation.nodes, Reincarnation.unit3, forming]
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      Reincarnation.nodes, Reincarnation.e1, Reincarnation.crossEra,
      Reincarnation.unit3, forming]
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      Reincarnation.nodes, Reincarnation.e2, Reincarnation.evictEra,
      Reincarnation.e1, Reincarnation.crossEra, Reincarnation.unit3, forming]

/-- In E2 the voters are `{0, 1, 3}` and majorities are pairs, e.g.
`{0, 3}`. -/
theorem degenerate_e2_pair :
    WeightedGeneral.majority Reincarnation.nodes Reincarnation.e2 (fun a => a = 0 ∨ a = 3) := by
  simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
    Reincarnation.nodes, Reincarnation.e2, Reincarnation.evictEra,
    Reincarnation.e1, Reincarnation.crossEra, Reincarnation.unit3]

/-- The honest negative: the coinciding forming quorum has two members, so no
single node is a pivot for it — no casting vote is exercised on this path,
and none is needed: the same set forms on both sides (`degenerate_forming`)
and the boundary's overlap is rung 22's `e0_e1_safe`. -/
theorem degenerate_no_pivot (ℓ : Nat) :
    ¬ CastingVote.HasCastingVote forming forming ℓ := by
  intro h
  have h0 : forming 0 := by unfold forming; simp
  have h1 : forming 1 := by unfold forming; simp
  have h0_iff := ((h 0).mp ⟨h0, h0⟩)
  have h1_iff := ((h 1).mp ⟨h1, h1⟩)
  have : (0 : Nat) = 1 := by rw [h0_iff, h1_iff]
  omega

end CastingVoteReincarnation
```

```bash
lake env lean UVRR/CastingVoteReincarnation.lean
```

```output
```

```bash
lake env lean --stdin <<'LEAN'
import UVRR.CastingVoteReincarnation
#print axioms CastingVoteReincarnation.unit2_total
#print axioms CastingVoteReincarnation.c1_membership
#print axioms CastingVoteReincarnation.c2_membership
#print axioms CastingVoteReincarnation.unit2_majority_both
#print axioms CastingVoteReincarnation.c1_majority_leader
#print axioms CastingVoteReincarnation.two_casting_vote
#print axioms CastingVoteReincarnation.meets_at_leader
#print axioms CastingVoteReincarnation.casting_vote_singleton
#print axioms CastingVoteReincarnation.two_e0_e1_safe
#print axioms CastingVoteReincarnation.e1_majority_pair
#print axioms CastingVoteReincarnation.degenerate_forming
#print axioms CastingVoteReincarnation.degenerate_e2_pair
#print axioms CastingVoteReincarnation.degenerate_no_pivot
LEAN
```

```output
'CastingVoteReincarnation.unit2_total' depends on axioms: [propext]
'CastingVoteReincarnation.c1_membership' depends on axioms: [propext]
'CastingVoteReincarnation.c2_membership' depends on axioms: [propext]
'CastingVoteReincarnation.unit2_majority_both' depends on axioms: [propext, Classical.choice, Quot.sound]
'CastingVoteReincarnation.c1_majority_leader' depends on axioms: [propext, Classical.choice, Quot.sound]
'CastingVoteReincarnation.two_casting_vote' depends on axioms: [propext, Classical.choice, Quot.sound]
'CastingVoteReincarnation.meets_at_leader' does not depend on any axioms
'CastingVoteReincarnation.casting_vote_singleton' does not depend on any axioms
'CastingVoteReincarnation.two_e0_e1_safe' depends on axioms: [propext, Classical.choice, Quot.sound]
'CastingVoteReincarnation.e1_majority_pair' depends on axioms: [propext, Classical.choice, Quot.sound]
'CastingVoteReincarnation.degenerate_forming' depends on axioms: [propext, Classical.choice, Quot.sound]
'CastingVoteReincarnation.degenerate_e2_pair' depends on axioms: [propext, Classical.choice, Quot.sound]
'CastingVoteReincarnation.degenerate_no_pivot' depends on axioms: [propext, Quot.sound]
```

## Proof obligations

The two cases checked here are finite instantiations. Each obligation below is
a future rung's target, stated with the definitions it would use; none is
claimed here.

**(a) The casting vote on the reincarnation eras in general.** For an arbitrary
identity type, arbitrary finite support list, and any leader-overlap forced
sequence — leader ℓ survives the crossing era at positive weight, the old
identity is driven to weight 0, and the crossing-era total voting weight
reduces to ℓ's unit — the crossing boundary admits a casting vote at ℓ: some
E0 majority and some E1 majority intersect in ℓ alone, every quorum pair meets
at ℓ, and ℓ can complete phase I of the new ballot by promising to itself
(rung 6's schedule). Definitions used: `HasCastingVote`, `meets_at_leader`,
`casting_vote_singleton`, `crossEra`, `WeightedGeneral.majority`,
`WeightedGeneral.mass`, `WeightedGeneral.total`; the checked instances
`unit2_majority_both`, `c1_majority_leader`, `two_casting_vote` are the
two-node unit-scale finite case.

**(b) Agreement through the leader-overlap schedule on the wire.** The
casting-vote mode only chooses who is sent prepare(b') and when ℓ promises; a
general agreement theorem for the schedule must connect rung 6's
`noninterference`, `guard_preserved` and `completes_phase1` to rung 3/4
agreement over the reincarnation eras with the wire protocol modeled. Not
admitted here.

**(c) The pivot dichotomy in general.** For an arbitrary unit-weight cluster
under reincarnation, classify the crossing boundary: either the forming
quorums coincide between the old and new eras (the degenerate case checked in
`degenerate_forming`, safe with no pivot, `degenerate_no_pivot`), or they
differ and a singleton pivot at the leader is required
(`two_casting_vote`'s shape, scaled). Definitions used: `forming`,
`HasCastingVote`, `degenerate_forming`, `degenerate_no_pivot`,
`WeightedGeneral.majority`.
