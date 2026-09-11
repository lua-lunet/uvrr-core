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
theorem e1_majority_survivors {q : NSet Nat}
    (hm : WeightedGeneral.majority nodes5 c1 q) :
    (q 1 ∧ q 2 ∧ q 3) ∨ (q 1 ∧ q 2 ∧ q 4) ∨ (q 1 ∧ q 3 ∧ q 4) ∨ (q 2 ∧ q 3 ∧ q 4) := by
  by_cases h1 : q 1 <;> by_cases h2 : q 2 <;> by_cases h3 : q 3 <;> by_cases h4 : q 4 <;>
    simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, Reincarnation.crossEra, unit5, c1, h1, h2, h3, h4] at hm ⊢ <;> omega
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
theorem forming_no_casting_vote (ℓ : Nat) {q' : NSet Nat}
    (hm : WeightedGeneral.majority nodes5 c1 q') :
    ¬ CastingVote.HasCastingVote forming q' ℓ := by
  intro h
  rcases e1_majority_survivors hm with (⟨h1, h2, h3⟩ | ⟨h1, h2, h4⟩ | ⟨h1, h3, h4⟩ | ⟨h2, h3, h4⟩)
  · have h1ℓ : 1 = ℓ := ((h 1).mp ⟨by simp [forming], h1⟩)
    have h2ℓ : 2 = ℓ := ((h 2).mp ⟨by simp [forming], h2⟩)
    omega
  · have h1ℓ : 1 = ℓ := ((h 1).mp ⟨by simp [forming], h1⟩)
    have h2ℓ : 2 = ℓ := ((h 2).mp ⟨by simp [forming], h2⟩)
    omega
  · have h1ℓ : 1 = ℓ := ((h 1).mp ⟨by simp [forming], h1⟩)
    have h3ℓ : 3 = ℓ := ((h 3).mp ⟨by simp [forming], h3⟩)
    omega
  · have h2ℓ : 2 = ℓ := ((h 2).mp ⟨by simp [forming], h2⟩)
    have h3ℓ : 3 = ℓ := ((h 3).mp ⟨by simp [forming], h3⟩)
    omega
theorem e1_e2_pivot :
    WeightedGeneral.majority nodes5 c1 forming ∧
    WeightedGeneral.majority nodes5 c2 pivotNew ∧
    CastingVote.HasCastingVote forming pivotNew 3 := by
  refine ⟨?_, ?_, ?_⟩
  · exact forming_majorities.2.1
  · have hc2 : c2 = Reincarnation.evictEra 5 c1 := rfl
    rw [hc2]
    unfold WeightedGeneral.majority Reincarnation.evictEra Reincarnation.crossEra unit5 c1
    simp [nodes5, forming, pivotNew]
    omega
  · intro a
    constructor
    · intro ⟨hf, hp⟩
      rcases hf with (hf | hf | hf)
      · rcases hp with (hp | hp | hp)
        · omega
        · omega
        · omega
      · rcases hp with (hp | hp | hp)
        · omega
        · omega
        · omega
      · rcases hp with (hp | hp | hp)
        · omega
        · omega
        · omega
    · intro h
      subst h
      exact ⟨by simp [forming], by simp [pivotNew]⟩
end ReincarnationFive
