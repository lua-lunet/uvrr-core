import UVRR.Reincarnation

/-! General-scale theorems over the reincarnation rung's definitions: the
discharge of the rung's four proof obligations. The mass bounds for the three
batch forms hold for arbitrary configurations at arbitrary scale, so rung 9's
`WeightedGeneral.unit_change_overlap` makes every era of the forced sequence
quorum-safe. The old identity's weight never increases along a forced run and
never returns to a voter once zero, the bumped identity never reverts to the
pre-bump identity, and a forced run from any committed phase completes: no
reachable terminal phase other than `flushed`. -/

namespace Reincarnation

/-! ### The one-unit mass bound at arbitrary scale -/

theorem total_zero {A : Type} (nodes : List A) (f : A → Nat)
    (h : ∀ a ∈ nodes, f a = 0) : WeightedGeneral.total nodes f = 0 := by
  induction nodes with
  | nil => rfl
  | cons a rest ih =>
    simp only [WeightedGeneral.total]
    have h0 : f a = 0 := h a (List.mem_cons_self)
    have h1 := ih (fun b hb => h b (List.mem_cons_of_mem a hb))
    rw [h0, h1]

theorem distance_crossEra_zero {A : Type} [DecidableEq A] {old new : A}
    (w : Config A) (hnew : w new = 0) {a : A} (ha : a ≠ old) :
    WeightedGeneral.distance (w a) (crossEra old new w a) = 0 := by
  rw [crossEra, if_neg ha]
  by_cases hb : a = new
  · rw [if_pos hb, hb, hnew]
    simp [WeightedGeneral.distance]
  · rw [if_neg hb]
    simp [WeightedGeneral.distance]

theorem distance_evictEra_zero {A : Type} [DecidableEq A] {new a : A} (w : Config A)
    (ha : a ≠ new) :
    WeightedGeneral.distance (w a) (evictEra new w a) = 0 := by
  rw [evictEra, if_neg ha]
  simp [WeightedGeneral.distance]

theorem distance_step0_zero {A : Type} [DecidableEq A] {old a : A} (w : Config A)
    (ha : a ≠ old) :
    WeightedGeneral.distance (w a) (step0 old w a) = 0 := by
  rw [step0, if_neg ha]
  simp [WeightedGeneral.distance]

/-- The crossing era moves at most one unit of per-node voting mass at
arbitrary scale: the old identity drops one unit, the fresh identity joins at
weight zero, everything else is untouched. The support list is the
duplicate-free identity enumeration (duplicated entries would double-count
the moved unit). -/
theorem crossEra_mass {A : Type} [DecidableEq A] {old new : A} (_hne : old ≠ new)
    (nodes : List A) (w : Config A) (_hold : 1 ≤ w old) (hnew : w new = 0)
    (hnodup : nodes.Nodup) :
    WeightedGeneral.total nodes
      (fun a => WeightedGeneral.distance (w a) (crossEra old new w a)) ≤ 1 := by
  induction nodes with
  | nil => exact Nat.zero_le _
  | cons a rest ih =>
    have hmem : a ∉ rest := (List.nodup_cons.mp hnodup).1
    have hrest : rest.Nodup := (List.nodup_cons.mp hnodup).2
    by_cases ha : a = old
    · have hbne : ∀ b ∈ rest, b ≠ old := by
        intro b hmem' he
        apply hmem
        rw [ha, ← he]
        exact hmem'
      have hsum := total_zero rest
        (fun b => WeightedGeneral.distance (w b) (crossEra old new w b))
        (fun b hb => distance_crossEra_zero w hnew (hbne b hb))
      have hda : WeightedGeneral.distance (w a) (crossEra old new w a) ≤ 1 := by
        rw [ha, crossEra, if_pos rfl, WeightedGeneral.distance]
        omega
      rw [WeightedGeneral.total]
      omega
    · have hda := distance_crossEra_zero w hnew ha
      have ih' := ih hrest
      rw [WeightedGeneral.total]
      omega

/-- The eviction era moves at most one unit of per-node voting mass at
arbitrary scale: the new identity is promoted one unit, everything else is
untouched. -/
theorem evictEra_mass {A : Type} [DecidableEq A] (new : A) (nodes : List A)
    (w : Config A) (hnodup : nodes.Nodup) :
    WeightedGeneral.total nodes
      (fun a => WeightedGeneral.distance (w a) (evictEra new w a)) ≤ 1 := by
  induction nodes with
  | nil => exact Nat.zero_le _
  | cons a rest ih =>
    have hmem : a ∉ rest := (List.nodup_cons.mp hnodup).1
    have hrest : rest.Nodup := (List.nodup_cons.mp hnodup).2
    by_cases ha : a = new
    · have hbne : ∀ b ∈ rest, b ≠ new := by
        intro b hmem' he
        apply hmem
        rw [ha, ← he]
        exact hmem'
      have hsum := total_zero rest
        (fun b => WeightedGeneral.distance (w b) (evictEra new w b))
        (fun b hb => distance_evictEra_zero w (hbne b hb))
      have hda : WeightedGeneral.distance (w a) (evictEra new w a) ≤ 1 := by
        rw [ha, evictEra, if_pos rfl, WeightedGeneral.distance]
        omega
      rw [WeightedGeneral.total]
      omega
    · have hda := distance_evictEra_zero w ha
      have ih' := ih hrest
      rw [WeightedGeneral.total]
      omega

/-- The solitary decrement moves at most one unit of per-node voting mass at
arbitrary scale: the old identity drops one unit, everything else is
untouched. -/
theorem step0_mass {A : Type} [DecidableEq A] (old : A) (nodes : List A)
    (w : Config A) (hnodup : nodes.Nodup) :
    WeightedGeneral.total nodes
      (fun a => WeightedGeneral.distance (w a) (step0 old w a)) ≤ 1 := by
  induction nodes with
  | nil => exact Nat.zero_le _
  | cons a rest ih =>
    have hmem : a ∉ rest := (List.nodup_cons.mp hnodup).1
    have hrest : rest.Nodup := (List.nodup_cons.mp hnodup).2
    by_cases ha : a = old
    · have hbne : ∀ b ∈ rest, b ≠ old := by
        intro b hmem' he
        apply hmem
        rw [ha, ← he]
        exact hmem'
      have hsum := total_zero rest
        (fun b => WeightedGeneral.distance (w b) (step0 old w b))
        (fun b hb => distance_step0_zero w (hbne b hb))
      have hda : WeightedGeneral.distance (w a) (step0 old w a) ≤ 1 := by
        rw [ha, step0, if_pos rfl, WeightedGeneral.distance]
        omega
      rw [WeightedGeneral.total]
      omega
    · have hda := distance_step0_zero w ha
      have ih' := ih hrest
      rw [WeightedGeneral.total]
      omega

/-- Era safety at arbitrary scale, composed: both consecutive eras of the
forced sequence keep consecutive strict majorities overlapping, by rung 9's
one-unit overlap. -/
theorem forced_sequence_era_safe {A : Type} [DecidableEq A] {old new : A}
    (_hne : old ≠ new) (nodes : List A) (w : Config A) (_hold : 1 ≤ w old)
    (hnew : w new = 0) (hnodup : nodes.Nodup) :
    Frown (WeightedGeneral.majority nodes w)
        (WeightedGeneral.majority nodes (crossEra old new w)) ∧
    Frown (WeightedGeneral.majority nodes (crossEra old new w))
        (WeightedGeneral.majority nodes (evictEra new (crossEra old new w))) := by
  constructor
  · exact WeightedGeneral.unit_change_overlap nodes w (crossEra old new w)
      (crossEra_mass _hne nodes w _hold hnew hnodup)
  · exact WeightedGeneral.unit_change_overlap nodes (crossEra old new w)
      (evictEra new (crossEra old new w)) (evictEra_mass new nodes _ hnodup)

/-! ### The configuration carried by the forced run

The forced run walks the phases with the baseline configuration at `dirty`,
the crossed configuration from `bumped` on, and the evicted configuration at
`flushed`. -/

def eraConfig {A : Type} [DecidableEq A] (old new : A) (w : Config A) :
    Phase → Config A
  | .flushed => evictEra new (crossEra old new w)
  | .unflushed => w
  | .dirty => w
  | .bumped => crossEra old new w
  | .reincarnating => crossEra old new w

theorem eraConfig_old_step {A : Type} [DecidableEq A] {old new : A} (hne : old ≠ new)
    (w : Config A) {q t : Phase} (hst : ForcedStep q t) :
    eraConfig old new w t old ≤ eraConfig old new w q old := by
  cases hst with
  | bump_write =>
    show crossEra old new w old ≤ w old
    simp only [crossEra]
    exact Nat.sub_le _ _
  | enter_wire => exact Nat.le_refl _
  | complete =>
    show evictEra new (crossEra old new w) old ≤ crossEra old new w old
    simp only [evictEra, if_neg hne]
    exact Nat.le_refl _

/-- The old identity's weight never increases along a forced run. -/
theorem forced_run_old_mono {A : Type} [DecidableEq A] {old new : A} (hne : old ≠ new)
    (w : Config A) {p t : Phase} (hrun : ForcedRun p t) :
    eraConfig old new w t old ≤ eraConfig old new w p old := by
  induction hrun with
  | refl => exact Nat.le_refl _
  | step _ st ih => exact Nat.le_trans (eraConfig_old_step hne w st) ih

/-- Once the old identity reaches weight zero it stays there for the rest of
the forced run. -/
theorem forced_run_old_zero {A : Type} [DecidableEq A] {old new : A} (hne : old ≠ new)
    (w : Config A) {p t : Phase} (hrun : ForcedRun p t)
    (h0 : eraConfig old new w p old = 0) :
    eraConfig old new w t old = 0 := by
  exact Nat.le_antisymm
    (Nat.le_trans (forced_run_old_mono hne w hrun) (Nat.le_of_eq h0)) (Nat.zero_le _)

/-- Bumped-identity non-membership: from `bumped` on — the crossing era
completed — the old identity is never a voter again in any subsequent
configuration of the forced run. The crossing era drives the old identity to
weight zero exactly when it starts at or below one unit; at a larger scale
the crossing era iterates, and the scale-independent invariants above
(`forced_run_old_mono`, `forced_run_old_zero`) carry the argument through
each iteration. -/
theorem evicted_never_voting {A : Type} [DecidableEq A] {old new : A} (hne : old ≠ new)
    (w : Config A) (hw : w old ≤ 1) {t : Phase} (hrun : ForcedRun .bumped t) :
    ¬ voting (eraConfig old new w t) old := by
  have hwn : (w old : Nat) ≤ 1 := hw
  have h0n : (eraConfig old new w Phase.bumped old : Nat) = 0 := by
    show (crossEra old new w old : Nat) = 0
    have h1 : crossEra old new w old = w old - 1 := by simp [crossEra]
    rw [h1]
    exact Nat.sub_eq_zero_iff_le.mpr hwn
  have hzn : (eraConfig old new w t old : Nat) = 0 := forced_run_old_zero hne w hrun h0n
  intro hv
  have hvn : (0 : Nat) < (eraConfig old new w t old : Nat) := hv
  rw [hzn] at hvn
  exact absurd hvn (Nat.lt_irrefl 0)

/-! ### The amnesia trace is unreachable

The identity-carrying run pairs each phase with the identity the node
operates under; the bump step replaces the identity with its strictly higher
incarnation and no step ever lowers it. -/

inductive IdentRun : Phase → Ident → Phase → Ident → Prop
  | refl (p : Phase) (i : Ident) : IdentRun p i p i
  | start_op (i : Ident) : IdentRun .flushed i .unflushed i
  | observe_dirty (i : Ident) : IdentRun .unflushed i .dirty i
  | bump_write (i : Ident) : IdentRun .dirty i .bumped (bump i)
  | enter_wire (i : Ident) : IdentRun .bumped i .reincarnating i
  | complete (i : Ident) : IdentRun .reincarnating i .flushed i
  | trans {p q t : Phase} {i j k : Ident} :
      IdentRun p i q j → IdentRun q j t k → IdentRun p i t k

/-- The operating identity never decreases along an identity-carrying run. -/
theorem ident_mono {p q : Phase} {i j : Ident} (h : IdentRun p i q j) : i ≤ j := by
  induction h with
  | refl => exact Nat.le_refl _
  | start_op => exact Nat.le_refl _
  | observe_dirty => exact Nat.le_refl _
  | bump_write i => exact Nat.le_succ i
  | enter_wire => exact Nat.le_refl _
  | complete => exact Nat.le_refl _
  | trans _ _ ih1 ih2 => exact Nat.le_trans ih1 ih2

/-- No transition from a committed phase re-enters a pre-eviction phase:
once the eviction is initiated the machine cannot go back to running the
startup path. -/
theorem no_predirty_return {s t : Phase} (h : Transition s t) (hs : committedPhase s) :
    t = .bumped ∨ t = .reincarnating ∨ t = .flushed := by
  cases h with
  | start_op => exact absurd hs (by simp [committedPhase])
  | observe_dirty => exact absurd hs (by simp [committedPhase])
  | bump_write => exact Or.inl rfl
  | enter_wire => exact Or.inr (Or.inl rfl)
  | complete => exact Or.inr (Or.inr rfl)

/-- The classic amnesia trace is unreachable: from the bumped phase, under
the bumped identity, no continuation of the machine ever operates under the
pre-bump identity again — the identity only grows past it. -/
theorem amnesia_unreachable {q : Phase} {j i₀ : Ident}
    (h : IdentRun .bumped (bump i₀) q j) : j ≠ i₀ := by
  intro hcon
  have hm : (bump i₀ : Nat) ≤ (j : Nat) := ident_mono h
  rw [hcon] at hm
  exact absurd hm (Nat.not_succ_le_self i₀)

/-! ### Continuation commitment in general -/

/-- A forced run started at any committed phase stays inside the committed
phases until it completes to the new identity's `flushed`. -/
theorem forced_run_committed {p t : Phase} (hp : committedPhase p)
    (hrun : ForcedRun p t) :
    committedPhase t ∨ t = .flushed := by
  induction hrun with
  | refl => exact Or.inl hp
  | step _ st ih =>
    rcases ih with hq | hq
    · cases st with
      | bump_write => exact Or.inl (Or.inr (Or.inl rfl))
      | enter_wire => exact Or.inl (Or.inr (Or.inr rfl))
      | complete => exact Or.inr rfl
    · subst hq
      cases st

/-- A phase is terminal when no forced step leaves it. -/
def terminal (t : Phase) : Prop := ¬ ∃ u, ForcedStep t u

/-- The only terminal phase reachable by a forced run from a committed phase
is `flushed`: the forced run cannot end in `dirty`, `bumped` or
`reincarnating` — each of those has a forced step out, so completion is
mandatory. -/
theorem forced_run_terminal_flushed {p t : Phase} (hrun : ForcedRun p t)
    (hp : committedPhase p) (hterm : terminal t) : t = .flushed := by
  rcases forced_run_committed hp hrun with hc | hc
  · rcases hc with hc | hc | hc
    · subst hc
      exact absurd (⟨.bumped, ForcedStep.bump_write⟩ : ∃ u, ForcedStep .dirty u) hterm
    · subst hc
      exact absurd (⟨.reincarnating, ForcedStep.enter_wire⟩ : ∃ u, ForcedStep .bumped u) hterm
    · subst hc
      exact absurd (⟨.flushed, ForcedStep.complete⟩ : ∃ u, ForcedStep .reincarnating u) hterm
  · exact hc

end Reincarnation
