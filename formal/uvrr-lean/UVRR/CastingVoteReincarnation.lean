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
