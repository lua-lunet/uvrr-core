/-
  UPaxos baby step 1: leader-casting-vote overlap structure, in Lean 4.33.1 core.

  HONEST SCOPE: this is a typing/structure seed only. It states the UPaxos
  era/quorum shape and proves that a concrete 3-zone upgrade schedule keeps
  quorum overlap. It is NOT a liveness result and NOT the full Paxos agreement
  proof. The `FixableIn` monotonicity fact is a construction lemma about which
  slot eras a ballot may fix into; safety of actual votes is future work.

  DEVIATION NOTE: the spec asked for `Finset`, but Lean 4.33.1 core (and the
  locally cached Batteries) has no `Finset` — it is Mathlib-only. To keep the
  dependency set empty we model quorums as `List Node` and make `Overlap`
  decidable, so concrete instances close by `decide` (kernel-checked).
-/

abbrev Era := Nat

/-- The era of a ballot: e⟨b⟩ = b.1. -/
def eOf (b : Era × Nat) : Era := b.1

abbrev Node := Nat

/-- ⟨Q_I, Q_II⟩ configuration pair. -/
structure Config where
  QI : List Node
  QII : List Node

/-- Two quorums overlap: some node is in both. -/
def Overlap (Q1 Q2 : List Node) : Prop := Q1.any (fun x => x ∈ Q2) = true

/-- The three UPaxos safety-of-structure overlap invariants, era-adjacent. -/
def SoundCfgs (C : Era → Config) : Prop :=
  (∀ e, Overlap (C e).QI (C e).QII) ∧
  (∀ e, Overlap (C e).QI (C (e+1)).QII) ∧
  (∀ e, Overlap (C (e+1)).QI (C e).QII)

theorem eOf_eq (b : Era × Nat) : eOf b = b.1 := rfl

/-- 3-zone upgrade: zones A,B,C are nodes 0,1,2; node 3 is the temporarily
added Z (two nodes in zone A at era 1); zone C (node 2) is dropped at era 2. -/
def cfgE : Era → Config
  | 0 => ⟨[0, 1, 2], [0, 1, 2]⟩
  | 1 => ⟨[0, 1, 3], [0, 2, 3]⟩
  | _ => ⟨[0, 1, 3], [0, 1, 3]⟩

theorem cfgE_ge2 (n : Era) : cfgE (n + 2) = ⟨[0, 1, 3], [0, 1, 3]⟩ := rfl

/-- The upgrade schedule keeps all three overlap invariants for every era. -/
theorem cfgE_sound : SoundCfgs cfgE := by
  refine ⟨?_, ?_, ?_⟩
  · intro e
    cases e with
    | zero => unfold cfgE Overlap; decide
    | succ e => cases e with
      | zero => unfold cfgE Overlap; decide
      | succ n => rw [cfgE_ge2]; unfold Overlap; decide
  · intro e
    cases e with
    | zero => unfold cfgE Overlap; decide
    | succ e => cases e with
      | zero => unfold cfgE Overlap; decide
      | succ n =>
        rw [cfgE_ge2, cfgE_ge2]
        unfold Overlap
        decide
  · intro e
    cases e with
    | zero => unfold cfgE Overlap; decide
    | succ e => cases e with
      | zero => unfold cfgE Overlap; decide
      | succ n =>
        rw [cfgE_ge2, cfgE_ge2]
        unfold Overlap
        decide

/-- A ballot may fix its vote in its own era or the next. -/
def FixableIn (b : Era × Nat) (s : Era) : Prop := s = b.1 ∨ s = b.1 + 1

/-- Fix-era monotonicity: a fixable slot is never below the ballot's era
(matching e⟨s⟩ ≥ e⟨b⟩). Construction lemma only — not vote safety. -/
theorem fixable_le (b : Era × Nat) (s : Era) (h : FixableIn b s) : b.1 ≤ s := by
  rcases h with h | h
  · exact h ▸ Nat.le_refl b.1
  · exact h ▸ Nat.le_succ b.1

/-- Negative control: disjoint quorums; `Overlap` provably fails, so the
invariant rejects a corrupt configuration. -/
def bad : Config := ⟨[0], [1]⟩

theorem bad_no_overlap : ¬ Overlap bad.QI bad.QII := by
  unfold bad Overlap
  decide

/-- The invariant itself rejects the corrupt config as a constant schedule. -/
theorem bad_not_sound : ¬ SoundCfgs (fun _ => bad) := by
  intro h
  exact bad_no_overlap (h.2.2 0)
