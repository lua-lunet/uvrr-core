/-
  Rung 5 — The cross-era frown QI_e ⌢ QII_{e+1} is NECESSARY.

  We exhibit a concrete two-node history that satisfies every UPaxos invariant
  EXCEPT the cross-era half of P1 (both intra-era frowns still hold) in which
  two different values are chosen for the same instance. Hence Theorem 10
  cannot be proved from intra-era overlaps alone: reconfiguring without the
  cross-era overlap can corrupt the cluster.

  Nodes: `false` = node 1, `true` = node 2. Era 0 quorums are {node 1} for both
  phases; era 1 quorums are {node 2} for both phases. Instance 0 is in era 1.
  Ballots (0,0) and (0,1) are both era-0 ballots (legal for an era-1 instance
  since e(b) ≤ e(i) ≤ e(b)+1). Each ballot gets its phase-I quorum from node 1
  (who never accepts anything, so reports no last vote and constrains nothing)
  and its phase-II quorum from node 2 (who never promised anything). Ballot
  (0,0) chooses `false`, ballot (0,1) chooses `true`.
-/
import UVRR.LexBallot

namespace Counterexample

/-- Node-set {node 1}. -/
def only1 : QSys Bool := fun q => ∀ a, q a ↔ a = false
/-- Node-set {node 2}. -/
def only2 : QSys Bool := fun q => ∀ a, q a ↔ a = true

def Q : Nat → QSys Bool
  | 0 => only1
  | _ => only2

def isB (b : Ballot) : Prop := b = (0,0) ∨ b = (0,1)

/-- The bad history. -/
def bad : Paxos Bool Ballot Bool where
  lt        := bLt
  era       := Ballot.era
  QI        := Q
  QII       := Q
  eInst     := fun _ => 1
  promised  := fun i a b => i = 0 ∧ a = false ∧ isB b
  promisedV := fun _ _ _ _ => False
  proposed  := fun i b => i = 0 ∧ isB b
  accepted  := fun i a b => i = 0 ∧ a = true ∧ isB b
  chosen    := fun i b => i = 0 ∧ isB b
  v         := fun _ b => decide (b.2 = 1)

/-- Both intra-era frowns hold in every era. -/
theorem intra_ok : ∀ e, Frown (bad.QII e) (bad.QI e) := by
  intro e q1 q2 h1 h2
  cases e with
  | zero =>
    exact ⟨false, (h1 false).2 rfl, (h2 false).2 rfl⟩
  | succ _ =>
    exact ⟨true, (h1 true).2 rfl, (h2 true).2 rfl⟩

/-- The cross-era frown QI_0 ⌢ QII_1 FAILS. -/
theorem cross_fails : ¬ Frown (bad.QI 0) (bad.QII 1) := by
  intro h
  obtain ⟨a, ha1, ha2⟩ := h (· = false) (· = true) (fun _ => Iff.rfl) (fun _ => Iff.rfl)
  subst ha1; exact Bool.false_ne_true ha2

/-- So P1 does not hold for `bad`. -/
theorem not_p1 : ¬ bad.P1 := fun h => cross_fails (h 0).2

/-- Every other invariant DOES hold. -/
theorem eraMono : bad.EraMono := Paxos.eraMono_lex bad rfl rfl

theorem eraLe : bad.EraLe := by
  intro i b h
  obtain ⟨_, hb⟩ := h
  rcases hb with hb | hb <;> subst hb <;> exact Nat.zero_le _

theorem p23 : bad.P23 := by
  intro i a b b' hp _ hacc
  obtain ⟨_, ha, _⟩ := hp
  obtain ⟨_, ha', _⟩ := hacc
  subst ha; exact Bool.false_ne_true ha'

theorem p4 : bad.P4 := by
  intro i a b b' h; exact h.elim

theorem p5 : bad.P5 := by
  intro i b hp
  obtain ⟨hi, hb⟩ := hp
  refine ⟨(· = false), ?_, ?_, ?_⟩
  · -- QI_{e(b)} = QI_0 = only1
    rcases hb with hb | hb <;> subst hb <;> exact fun _ => Iff.rfl
  · intro a ha; exact Or.inl ⟨hi, ha, hb⟩
  · intro ⟨_, _, _, hf⟩; exact hf.elim

theorem p6 : bad.P6 := by
  intro i a b h; exact ⟨h.1, h.2.2⟩

theorem p7 : bad.P7 := by
  intro i b hc
  obtain ⟨hi, hb⟩ := hc
  refine ⟨?_, (· = true), fun _ => Iff.rfl, fun a ha => ⟨hi, ha, hb⟩⟩
  rcases hb with hb | hb <;> subst hb <;> exact Nat.le_refl _

/-- Two different values are chosen for instance 0. -/
theorem two_chosen : bad.chosen 0 (0,0) ∧ bad.chosen 0 (0,1) ∧ bad.v 0 (0,0) ≠ bad.v 0 (0,1) :=
  ⟨⟨rfl, Or.inl rfl⟩, ⟨rfl, Or.inr rfl⟩, by decide⟩

/-- Summary: all invariants except the cross-era frown, yet agreement fails.
Hence the cross-era frown is not merely sufficient (Theorem 10) but necessary. -/
theorem cross_frown_necessary :
    (∀ e, Frown (bad.QII e) (bad.QI e)) ∧ bad.EraMono ∧ bad.EraLe ∧ bad.P23 ∧ bad.P4 ∧
    bad.P5 ∧ bad.P6 ∧ bad.P7 ∧ ¬ bad.P1 ∧
    ∃ i b1 b2, bad.chosen i b1 ∧ bad.chosen i b2 ∧ bad.v i b1 ≠ bad.v i b2 :=
  ⟨intra_ok, eraMono, eraLe, p23, p4, p5, p6, p7, not_p1, 0, (0,0), (0,1), two_chosen⟩

end Counterexample
