import UVRR.Counterexample

/-! Kernel-checked independence witnesses for the abstract agreement contract.
Each witness preserves the other stated conditions. These are histories, not
executions of a correct protocol. They show why an invariant must be discharged
by an implementation; they do not claim every possible algorithm needs it. -/
namespace NegativeControls

def singleton : QSys Unit := fun q => q ()

def wrongValue : Synod Unit Nat Bool where
  lt := (· < ·)
  QI := fun _ => singleton
  QII := fun _ => singleton
  promised := fun _ b => b = 0
  promisedV := fun _ b c => b = 1 ∧ c = 0
  proposed := fun b => b ≤ 1
  accepted := fun _ b => b ≤ 1
  chosen := fun b => b ≤ 1
  v := fun b => decide (b = 1)

theorem wrongValue_order : wrongValue.Order where
  wf := Nat.lt_wfRel.wf
  trans := fun _ _ _ => Nat.lt_trans
  irrefl := Nat.lt_irrefl
  total := fun a b => by change a < b ∨ a = b ∨ b < a; omega

theorem wrongValue_nonempty : ∀ b q, wrongValue.QII b q → ∃ a, q a :=
  fun _ _ h => ⟨(), h⟩

theorem wrongValue_other_invariants : wrongValue.S1 ∧ wrongValue.S2 ∧
    wrongValue.S3 ∧ wrongValue.S5 ∧ wrongValue.S6 := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · intro _ _ _ _ _ q r hq hr; exact ⟨(), hq, hr⟩
  · intro a b c hp hlt; change b = 0 at hp; change c < b at hlt; omega
  · intro a b c hp
    change b = 1 ∧ c = 0 at hp
    refine ⟨?_, ?_, ?_⟩
    · change c < b; omega
    · change c ≤ 1; omega
    · intro d hcd hdb; change c < d at hcd; change d < b at hdb; omega
  · intro a b h; exact h
  · intro b h; exact ⟨fun _ => True, True.intro, fun _ _ => h⟩

theorem wrongValue_not_s4 : ¬ wrongValue.S4 := by
  intro h
  obtain ⟨q, hq, _, hm⟩ := h 1 (by change 1 ≤ 1; omega)
  obtain ⟨bm, ⟨a, _, ha⟩, _, hv⟩ := hm ⟨(), 0, hq, rfl, rfl⟩
  change 1 = 1 ∧ bm = 0 at ha
  have hb : bm = 0 := ha.2
  subst bm
  change true = false at hv
  cases hv

theorem wrongValue_disagrees : wrongValue.chosen 0 ∧ wrongValue.chosen 1 ∧
    wrongValue.v 0 ≠ wrongValue.v 1 := by simp [wrongValue]

/-- Mutation: a node reports a free promise after already accepting below it.
The proposal rule is now unconstrained; only the promise-fencing invariant fails. -/
def forgottenVote : Synod Unit Nat Bool :=
  { wrongValue with promised := fun _ b => b ≤ 1, promisedV := fun _ _ _ => False }

theorem forgottenVote_other_invariants : forgottenVote.S1 ∧ forgottenVote.S3 ∧
    forgottenVote.S4 ∧ forgottenVote.S5 ∧ forgottenVote.S6 := by
  refine ⟨wrongValue_other_invariants.1, ?_, ?_, ?_, ?_⟩
  · intro _ _ _ h; exact h.elim
  · intro b h
    refine ⟨fun _ => True, True.intro, fun _ _ => Or.inl h, ?_⟩
    intro ⟨_, _, _, hf⟩; exact hf.elim
  · intro _ _ h; exact h
  · intro b h; exact ⟨fun _ => True, True.intro, fun _ _ => h⟩

theorem forgottenVote_not_s2 : ¬ forgottenVote.S2 := by
  intro h
  exact h () 1 0 (by change 1 ≤ 1; omega) (by change 0 < 1; omega) (by change 0 ≤ 1; omega)

theorem forgottenVote_disagrees : forgottenVote.chosen 0 ∧ forgottenVote.chosen 1 ∧
    forgottenVote.v 0 ≠ forgottenVote.v 1 := by simp [wrongValue, forgottenVote]

/-- Mutation: an empty decision quorum permits choice with no proposal. -/
def emptyDecision : Synod Unit Nat Bool :=
  { wrongValue with
    QII := fun _ q => ∀ a, ¬ q a
    promised := fun _ _ => False
    promisedV := fun _ _ _ => False
    proposed := fun _ => False
    accepted := fun _ _ => False }

theorem emptyDecision_inv : emptyDecision.Inv where
  s1 := by intro _ _ h; exact h.elim
  s2 := by intro _ _ _ h; exact h.elim
  s3 := by intro _ _ _ h; exact h.elim
  s4 := by intro _ h; exact h.elim
  s5 := by intro _ _ h; exact h.elim
  s6 := by
    intro b _
    exact ⟨fun _ => False, fun _ h => h, fun _ h => h⟩

theorem emptyDecision_not_nonempty :
    ¬ (∀ b q, emptyDecision.QII b q → ∃ a, q a) := by
  intro h
  obtain ⟨_, hf⟩ := h 0 (fun _ => False) (fun _ hf => hf)
  exact hf

theorem emptyDecision_disagrees : emptyDecision.chosen 0 ∧ emptyDecision.chosen 1 ∧
    emptyDecision.v 0 ≠ emptyDecision.v 1 := by simp [wrongValue, emptyDecision]

end NegativeControls
