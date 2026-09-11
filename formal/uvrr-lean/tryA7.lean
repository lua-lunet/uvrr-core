import UVRR.ReincarnationGeneral
import UVRR.CastingVote
namespace ReincarnationFive
def nodes5 : List Nat := [0, 1, 2, 3, 4, 5]
def unit5 : Reincarnation.Config Nat :=
  fun a => if a = 0 ∨ a = 1 ∨ a = 2 ∨ a = 3 ∨ a = 4 then 1 else 0
def c1 : Reincarnation.Config Nat := Reincarnation.crossEra 0 5 unit5
theorem e0_dead_majority_survivors {q : NSet Nat}
    (hm : WeightedGeneral.majority nodes5 unit5 q) (hd : ¬ q 0) :
    (q 1 ∧ q 2 ∧ q 3) ∨ (q 1 ∧ q 2 ∧ q 4) ∨ (q 1 ∧ q 3 ∧ q 4) ∨ (q 2 ∧ q 3 ∧ q 4) := by
  by_cases h1 : q 1 <;> by_cases h2 : q 2 <;> by_cases h3 : q 3 <;> by_cases h4 : q 4 <;>
    simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, unit5, hd, h1, h2, h3, h4] at hm ⊢ <;> omega
theorem e1_majority_survivors {q : NSet Nat}
    (hm : WeightedGeneral.majority nodes5 c1 q) :
    (q 1 ∧ q 2 ∧ q 3) ∨ (q 1 ∧ q 2 ∧ q 4) ∨ (q 1 ∧ q 3 ∧ q 4) ∨ (q 2 ∧ q 3 ∧ q 4) := by
  by_cases h1 : q 1 <;> by_cases h2 : q 2 <;> by_cases h3 : q 3 <;> by_cases h4 : q 4 <;>
    simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, Reincarnation.crossEra, unit5, c1, h1, h2, h3, h4] at hm ⊢ <;> omega
end ReincarnationFive
