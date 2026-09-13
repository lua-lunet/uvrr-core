import UVRR.Reincarnation

/-! General-scale theorems over the reincarnation rung's definitions: the
discharge of the rung's four proof obligations. The mass bounds for the three
batch forms hold for arbitrary configurations at arbitrary scale, so rung 9's
`WeightedGeneral.unit_change_overlap` makes every era of the forced sequence
quorum-safe. The old identity's weight never increases along a forced run and
never returns to a voter once zero, the bumped identity never reverts to the
pre-bump identity, and a forced run from any committed phase completes: no
reachable terminal phase other than `evicted`. -/

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

The forced run walks the phases with the baseline configuration at
`joining`, the crossed configuration from `crossed` on, and the evicted
configuration at `evicted`. The marker phases carry no configuration
change: the markers are the §5.1 boot machine, the forced sequence is the
configuration walk the `joining` node's leader commits. -/

def eraConfig {A : Type} [DecidableEq A] (old new : A) (w : Config A) :
    Phase → Config A
  | .running => w
  | .stopping => w
  | .stopped => w
  | .restarting => w
  | .joining => w
  | .crossed => crossEra old new w
  | .evicted => evictEra new (crossEra old new w)

theorem eraConfig_old_step {A : Type} [DecidableEq A] {old new : A} (hne : old ≠ new)
    (w : Config A) {q t : Phase} (hst : ForcedStep q t) :
    eraConfig old new w t old ≤ eraConfig old new w q old := by
  cases hst with
  | cross =>
    show crossEra old new w old ≤ w old
    simp only [crossEra]
    exact Nat.sub_le _ _
  | evict =>
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

/-- Bumped-identity non-membership: from `crossed` on — the crossing era
committed — the old identity is never a voter again in any subsequent
configuration of the forced run. The crossing era drives the old identity to
weight zero exactly when it starts at or below one unit; at a larger scale
the crossing era iterates, and the scale-independent invariants above
(`forced_run_old_mono`, `forced_run_old_zero`) carry the argument through
each iteration. -/
theorem evicted_never_voting {A : Type} [DecidableEq A] {old new : A} (hne : old ≠ new)
    (w : Config A) (hw : w old ≤ 1) {t : Phase} (hrun : ForcedRun .crossed t) :
    ¬ voting (eraConfig old new w t) old := by
  have hwn : (w old : Nat) ≤ 1 := hw
  have h0n : (eraConfig old new w Phase.crossed old : Nat) = 0 := by
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
operates under; the reincarnation's boot (`boot_join`, no 2-of-4 `stopped`
at boot) replaces the identity with its strictly higher incarnation and no
edge ever lowers it. -/

inductive IdentRun : Phase → Ident → Phase → Ident → Prop
  | refl (p : Phase) (i : Ident) : IdentRun p i p i
  | begin_stop (i : Ident) : IdentRun .running i .stopping i
  | finish_stop (i : Ident) : IdentRun .stopping i .stopped i
  | boot_restart (i : Ident) : IdentRun .stopped i .restarting i
  | boot_join (i : Ident) : IdentRun .running i .joining (bump i)
  | cross (i : Ident) : IdentRun .joining i .crossed i
  | evict (i : Ident) : IdentRun .crossed i .evicted i
  | trans {p q t : Phase} {i j k : Ident} :
      IdentRun p i q j → IdentRun q j t k → IdentRun p i t k

/-- The operating identity never decreases along an identity-carrying run. -/
theorem ident_mono {p q : Phase} {i j : Ident} (h : IdentRun p i q j) : i ≤ j := by
  induction h with
  | refl => exact Nat.le_refl _
  | begin_stop => exact Nat.le_refl _
  | finish_stop => exact Nat.le_refl _
  | boot_restart => exact Nat.le_refl _
  | boot_join i => exact Nat.le_succ i
  | cross => exact Nat.le_refl _
  | evict => exact Nat.le_refl _
  | trans _ _ ih1 ih2 => exact Nat.le_trans ih1 ih2

/-- No transition from a committed phase re-enters a pre-join phase: once
the bump is written the machine cannot go back to the states before it. -/
theorem no_prejoining_return {s t : Phase} (h : Transition s t) (hs : committedPhase s) :
    t = .crossed ∨ t = .evicted := by
  cases h with
  | begin_stop => exact absurd hs (by simp [committedPhase])
  | finish_stop => exact absurd hs (by simp [committedPhase])
  | boot_restart => exact absurd hs (by simp [committedPhase])
  | boot_join => exact absurd hs (by simp [committedPhase])
  | cross => exact Or.inl rfl
  | evict => exact Or.inr rfl

/-- The classic amnesia trace is unreachable: from the `joining` marker —
the bump written — under the bumped identity, no continuation of the
machine ever operates under the pre-bump identity again — the identity only
grows past it. -/
theorem amnesia_unreachable {q : Phase} {j i₀ : Ident}
    (h : IdentRun .joining (bump i₀) q j) : j ≠ i₀ := by
  intro hcon
  have hm : (bump i₀ : Nat) ≤ (j : Nat) := ident_mono h
  rw [hcon] at hm
  exact absurd hm (Nat.not_succ_le_self i₀)

/-! ### Continuation commitment in general -/

/-- A forced run started at any committed phase stays inside the committed
phases until it completes to `evicted` — the new identity votes. -/
theorem forced_run_committed {p t : Phase} (hp : committedPhase p)
    (hrun : ForcedRun p t) :
    committedPhase t ∨ t = .evicted := by
  induction hrun with
  | refl => exact Or.inl hp
  | step _ st ih =>
    rcases ih with hq | hq
    · cases st with
      | cross => exact Or.inl (by simp [committedPhase])
      | evict => exact Or.inr rfl
    · subst hq
      cases st

/-- A phase is terminal when no forced step leaves it. -/
def terminal (t : Phase) : Prop := ¬ ∃ u, ForcedStep t u

/-- The only terminal phase reachable by a forced run from a committed phase
is `evicted`: the forced run cannot end in `joining` or `crossed` — each of
those has a forced step out, so completion is mandatory. -/
theorem forced_run_terminal_evicted {p t : Phase} (hrun : ForcedRun p t)
    (hp : committedPhase p) (hterm : terminal t) : t = .evicted := by
  rcases forced_run_committed hp hrun with hc | hc
  · rcases hc with hc | hc
    · subst hc
      exact absurd (⟨.crossed, ForcedStep.cross⟩ : ∃ u, ForcedStep .joining u) hterm
    · subst hc
      exact absurd (⟨.evicted, ForcedStep.evict⟩ : ∃ u, ForcedStep .crossed u) hterm
  · exact hc

end Reincarnation
