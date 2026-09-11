import UVRR.ReincarnationGeneral
import UVRR.CastingVote
namespace ReincarnationFive
def nodes5 : List Nat := [0, 1, 2, 3, 4, 5]
def unit5 : Reincarnation.Config Nat :=
  fun a => if a = 0 ∨ a = 1 ∨ a = 2 ∨ a = 3 ∨ a = 4 then 1 else 0
theorem unit5_pair_not_majority :
    ¬ WeightedGeneral.majority nodes5 unit5 (fun a => a = 1 ∨ a = 2) := by
  simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
    nodes5, unit5] <;> omega
end ReincarnationFive
