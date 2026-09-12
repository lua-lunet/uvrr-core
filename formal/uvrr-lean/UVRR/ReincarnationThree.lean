import UVRR.ReincarnationGeneral
import UVRR.CastingVote

/-! Three-voter replacement with the two unit transitions
`(old, leader, survivor, replacement): (1,1,1,0) → (0,1,1,0) → (0,1,1,1)`.
The majority intersections quantify over all abstract quorum sets. The casting
witnesses are also abstract identity sets; no response assumptions are encoded.
-/
namespace ReincarnationThree

def nodes : List Nat := [0, 1, 2, 3]
def c0 : Reincarnation.Config Nat :=
  fun a => if a = 0 ∨ a = 1 ∨ a = 2 then 1 else 0
def c1 : Reincarnation.Config Nat := Reincarnation.crossEra 0 3 c0
def c2 : Reincarnation.Config Nat := Reincarnation.evictEra 3 c1

def qOld : NSet Nat := fun a => a = 0 ∨ a = 1
def qMiddle : NSet Nat := fun a => a = 1 ∨ a = 2
def qNew : NSet Nat := fun a => a = 1 ∨ a = 3

/-- All three rows, including the intermediate configuration. -/
theorem stage_weights :
    (c0 0, c0 1, c0 2, c0 3) = (1, 1, 1, 0) ∧
    (c1 0, c1 1, c1 2, c1 3) = (0, 1, 1, 0) ∧
    (c2 0, c2 1, c2 2, c2 3) = (0, 1, 1, 1) := by
  decide

/-- Every old majority intersects every new majority at both boundaries. -/
theorem adjacent_overlap :
    Frown (WeightedGeneral.majority nodes c0) (WeightedGeneral.majority nodes c1) ∧
    Frown (WeightedGeneral.majority nodes c1) (WeightedGeneral.majority nodes c2) := by
  exact Reincarnation.forced_sequence_era_safe (by decide) nodes c0
    (by simp [c0]) (by simp [c0]) (by decide)

/-- The retired identity has zero weight and its replacement has unit weight. -/
theorem final_membership : c2 0 = 0 ∧ c2 3 = 1 := by
  decide

/-- First boundary: the abstract old-side witness contains the old identity. -/
theorem first_boundary_casting :
    WeightedGeneral.majority nodes c0 qOld ∧
    WeightedGeneral.majority nodes c1 qMiddle ∧
    CastingVote.HasCastingVote qOld qMiddle 1 := by
  refine ⟨?_, ?_, ?_⟩
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes, c0, qOld]
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes, c1, Reincarnation.crossEra, c0, qMiddle]
  · intro a
    simp [qOld, qMiddle]
    omega

/-- Second boundary: the replacement belongs to the new-side witness. -/
theorem second_boundary_casting :
    WeightedGeneral.majority nodes c1 qMiddle ∧
    WeightedGeneral.majority nodes c2 qNew ∧
    CastingVote.HasCastingVote qMiddle qNew 1 := by
  refine ⟨?_, ?_, ?_⟩
  · exact first_boundary_casting.2.1
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes, c2, Reincarnation.evictEra, c1, Reincarnation.crossEra, c0, qNew]
  · intro a
    simp [qMiddle, qNew]
    omega

end ReincarnationThree
