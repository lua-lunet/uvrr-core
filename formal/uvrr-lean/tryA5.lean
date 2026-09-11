import UVRR.ReincarnationGeneral
import UVRR.CastingVote
namespace ReincarnationFive
def nodes5 : List Nat := [0, 1, 2, 3, 4, 5]
def unit5 : Reincarnation.Config Nat :=
  fun a => if a = 0 ∨ a = 1 ∨ a = 2 ∨ a = 3 ∨ a = 4 then 1 else 0
def c1 : Reincarnation.Config Nat := Reincarnation.crossEra 0 5 unit5
def c2 : Reincarnation.Config Nat := Reincarnation.evictEra 5 c1
theorem c1_membership :
    c1 0 = 0 ∧ c1 5 = 0 ∧ c1 1 = 1 ∧ c1 2 = 1 ∧ c1 3 = 1 ∧ c1 4 = 1 := by
  unfold c1 Reincarnation.crossEra unit5
  simp <;> omega
theorem c2_membership :
    c2 5 = 1 ∧ c2 1 = 1 ∧ c2 2 = 1 ∧ c2 3 = 1 ∧ c2 4 = 1 ∧ c2 0 = 0 ∧
    ¬ Reincarnation.voting c2 0 := by
  unfold c2 Reincarnation.evictEra c1 Reincarnation.crossEra unit5 Reincarnation.voting
  simp <;> omega
theorem unit5_pair_not_majority :
    ¬ WeightedGeneral.majority nodes5 unit5 (fun a => a = 1 ∨ a = 2) := by
  unfold WeightedGeneral.majority ReincarnationFive.nodes5 ReincarnationFive.unit5
  simp <;> omega
end ReincarnationFive
