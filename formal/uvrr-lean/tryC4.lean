import UVRR.ReincarnationGeneral
import UVRR.CastingVote
namespace ReincarnationFive
def nodes5 : List Nat := [0, 1, 2, 3, 4, 5]
def unit5 : Reincarnation.Config Nat :=
  fun a => if a = 0 ∨ a = 1 ∨ a = 2 ∨ a = 3 ∨ a = 4 then 1 else 0
def c1 : Reincarnation.Config Nat := Reincarnation.crossEra 0 5 unit5
def c2 : Reincarnation.Config Nat := Reincarnation.evictEra 5 c1
def forming : NSet Nat := fun a => a = 1 ∨ a = 2 ∨ a = 3
def pivotNew : NSet Nat := fun a => a = 3 ∨ a = 4 ∨ a = 5
theorem forming_majorities :
    WeightedGeneral.majority nodes5 unit5 forming ∧
    WeightedGeneral.majority nodes5 c1 forming ∧
    WeightedGeneral.majority nodes5 c2 forming := by
  refine ⟨?_, ?_, ?_⟩
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, unit5, forming] <;> omega
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, c1, Reincarnation.crossEra, unit5, forming] <;> omega
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, c2, Reincarnation.evictEra, c1, Reincarnation.crossEra, unit5,
      forming] <;> omega
theorem e1_e2_pivot :
    WeightedGeneral.majority nodes5 c1 forming ∧
    WeightedGeneral.majority nodes5 c2 pivotNew ∧
    CastingVote.HasCastingVote forming pivotNew 3 := by
  refine ⟨?_, ?_, ?_⟩
  · exact forming_majorities.2.1
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, c2, Reincarnation.evictEra, c1, Reincarnation.crossEra, unit5,
      pivotNew] <;> omega
  · intro a
    constructor
    · intro h
      rcases h with ⟨hfa, hpa⟩
      rcases hfa with (hfa | hfa | hfa)
      · rcases hpa with (hpa | hpa | hpa)
        · omega
        · omega
        · omega
      · rcases hpa with (hpa | hpa | hpa)
        · omega
        · omega
        · omega
      · rcases hpa with (hpa | hpa | hpa)
        · omega
        · omega
        · omega
    · intro h
      subst h
      constructor
      · simp [forming]
      · simp [pivotNew]
end ReincarnationFive
