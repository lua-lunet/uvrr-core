import UVRR.ReincarnationGeneral
import UVRR.CastingVote
namespace ReincarnationFive
def nodes5 : List Nat := [0, 1, 2, 3, 4, 5]
def unit5 : Reincarnation.Config Nat :=
  fun a => if a = 0 ∨ a = 1 ∨ a = 2 ∨ a = 3 ∨ a = 4 then 1 else 0
def c1 : Reincarnation.Config Nat := Reincarnation.crossEra 0 5 unit5
def c2 : Reincarnation.Config Nat := Reincarnation.evictEra 5 c1
def forming : NSet Nat := fun a => a = 1 ∨ a = 2 ∨ a = 3
theorem forming_majorities :
    WeightedGeneral.majority nodes5 unit5 forming ∧
    WeightedGeneral.majority nodes5 c1 forming ∧
    WeightedGeneral.majority nodes5 c2 forming := by
  refine ⟨?_, ?_, ?_⟩
  · unfold WeightedGeneral.majority ReincarnationFive.forming ReincarnationFive.unit5 nodes5
    simp [ReincarnationFive.nodes5, ReincarnationFive.unit5, ReincarnationFive.forming]
    omega
  · unfold WeightedGeneral.majority ReincarnationFive.forming ReincarnationFive.c1 nodes5
    simp [ReincarnationFive.nodes5, ReincarnationFive.unit5, ReincarnationFive.c1, ReincarnationFive.forming, Reincarnation.crossEra]
    omega
  · unfold WeightedGeneral.majority ReincarnationFive.forming ReincarnationFive.c2 nodes5
    simp [ReincarnationFive.nodes5, ReincarnationFive.unit5, ReincarnationFive.c1, ReincarnationFive.c2, ReincarnationFive.forming, Reincarnation.crossEra, Reincarnation.evictEra]
    omega
end ReincarnationFive
