import UVRR.ReincarnationGeneral
import UVRR.CastingVote
namespace ReincarnationFive
def nodes5 : List Nat := [0, 1, 2, 3, 4, 5]
def unit5 : Reincarnation.Config Nat :=
  fun a => if a = 0 ∨ a = 1 ∨ a = 2 ∨ a = 3 ∨ a = 4 then 1 else 0
def c1 : Reincarnation.Config Nat := Reincarnation.crossEra 0 5 unit5
def c2 : Reincarnation.Config Nat := Reincarnation.evictEra 5 c1
def unit5d : Reincarnation.Config Nat := fun a => if a = 0 then 2 else unit5 a
theorem five_forced_safe :
    Frown (WeightedGeneral.majority nodes5 unit5)
        (WeightedGeneral.majority nodes5 c1) ∧
    Frown (WeightedGeneral.majority nodes5 c1)
        (WeightedGeneral.majority nodes5 c2) := by
  exact Reincarnation.forced_sequence_era_safe (by decide) nodes5 unit5 (by simp [unit5]) (by simp [unit5]) (by decide)
theorem five_evicted_never_voting {t : Reincarnation.Phase}
    (hrun : Reincarnation.ForcedRun .bumped t) :
    ¬ Reincarnation.voting (Reincarnation.eraConfig 0 5 unit5 t) 0 := by
  apply Reincarnation.evicted_never_voting (by decide) unit5 (by simp [unit5]) hrun
theorem doubled_crossEra_leaves_voter :
    Reincarnation.voting (Reincarnation.crossEra 0 5 unit5d) 0 ∧
    WeightedGeneral.total nodes5
      (fun a => WeightedGeneral.distance (unit5d a)
        (Reincarnation.crossEra 0 5 unit5d a)) ≤ 1 := by
  have h0 : (0 : Nat) ≠ 5 := by decide
  have hhold : 1 ≤ unit5d 0 := by
    unfold unit5d
    simp
  have hnew : unit5d 5 = 0 := by
    unfold unit5d unit5
    simp
  have hnodup : nodes5.Nodup := by decide
  have h := Reincarnation.crossEra_mass h0 nodes5 unit5d hhold hnew hnodup
  refine ⟨?_, h⟩
  unfold Reincarnation.voting Reincarnation.crossEra unit5d unit5
  simp
  omega
end ReincarnationFive
