import UVRR.ReincarnationGeneral
import UVRR.CastingVote
namespace ReincarnationFive
def nodes5 : List Nat := [0, 1, 2, 3, 4, 5]
def unit5 : Reincarnation.Config Nat :=
  fun a => if a = 0 ∨ a = 1 ∨ a = 2 ∨ a = 3 ∨ a = 4 then 1 else 0
theorem nodes5_nodup : nodes5.Nodup := by
  unfold nodes5; decide
theorem unit5_total : WeightedGeneral.total nodes5 unit5 = 5 := by
  simp [nodes5, WeightedGeneral.total, unit5] <;> omega
end ReincarnationFive
