# Rung 2: Era-tagged ballots are a well-founded total order

*2026-09-05T23:31:03Z by Showboat 0.6.1*
<!-- showboat-id: 6eba725e-becb-420e-b066-a48e66d50f7b -->

**Claim.** UPaxos encodes the era in the most significant bits of the ballot number, so ballots are (era, round) pairs under lexicographic order. This order is well-founded, transitive, irreflexive and total (the side conditions needed for the agreement proofs), and it respects eras: b2 ≺ b1 ⇒ e(b2) ≤ e(b1), which the paper's Lemma 9 relies on.

**Also.** theorem10_lex specialises the era-indexed agreement theorem (Rung 4) to these concrete ballots, discharging all order-theoretic obligations so only the protocol invariants remain as hypotheses.

```bash
cat UVRR/LexBallot.lean
```

```output
/-
  Rung 2 — Concrete UPaxos ballots: (era, round) ordered lexicographically.
  The era occupies the most significant position, exactly as the paper encodes
  the era in the high bits of the ballot number. We prove the order is a
  well-founded strict total order (so `Synod.Order` / `theorem10` apply) and
  that it respects eras (`EraMono`).
-/
import UVRR.Eras

/-- A ballot is an (era, round) pair. -/
abbrev Ballot := Nat × Nat

/-- The era of a ballot: e⟨b⟩. -/
def Ballot.era (b : Ballot) : Nat := b.1

/-- Lexicographic order, era first. -/
def bLt (a b : Ballot) : Prop := Prod.Lex (· < ·) (· < ·) a b

theorem bLt_iff (a b : Ballot) : bLt a b ↔ a.1 < b.1 ∨ (a.1 = b.1 ∧ a.2 < b.2) := by
  constructor
  · intro h
    cases h with
    | left _ _ h => exact Or.inl h
    | right _ h => exact Or.inr ⟨rfl, h⟩
  · intro h
    obtain ⟨a1, a2⟩ := a
    obtain ⟨b1, b2⟩ := b
    rcases h with h | ⟨h1, h2⟩
    · exact Prod.Lex.left _ _ h
    · simp only at h1; subst h1; exact Prod.Lex.right _ h2

theorem bLt_wf : WellFounded bLt :=
  (Prod.lex Nat.lt_wfRel Nat.lt_wfRel).wf

theorem bLt_trans (a b c : Ballot) : bLt a b → bLt b c → bLt a c := by
  intro h1 h2
  rw [bLt_iff] at *
  omega

theorem bLt_irrefl (a : Ballot) : ¬ bLt a a := by
  rw [bLt_iff]; omega

theorem bLt_total (a b : Ballot) : bLt a b ∨ a = b ∨ bLt b a := by
  rw [bLt_iff, bLt_iff]
  obtain ⟨a1, a2⟩ := a
  obtain ⟨b1, b2⟩ := b
  simp only [Prod.mk.injEq]
  omega

/-- Ballot order respects eras: b2 ≺ b1 ⇒ e⟨b2⟩ ≤ e⟨b1⟩. -/
theorem bLt_era_mono (b1 b2 : Ballot) : bLt b2 b1 → b2.era ≤ b1.era := by
  rw [bLt_iff]; unfold Ballot.era; omega

/-- Theorem 10 specialised to concrete (era, round) ballots: the order-theoretic
side conditions are discharged, only the protocol invariants remain. -/
theorem Paxos.theorem10_lex {A : Type} {V : Type} (P : Paxos A Ballot V)
    (hlt : P.lt = bLt) (_hera : P.era = Ballot.era)
    (hI : P.Inv) (hne : ∀ e q, P.QII e q → ∃ a, q a)
    {i : Nat} {b1 b2 : Ballot} (h1 : P.chosen i b1) (h2 : P.chosen i b2) :
    P.v i b1 = P.v i b2 := by
  apply Paxos.theorem10 hI _ hne h1 h2
  rw [hlt]
  exact ⟨bLt_wf, bLt_trans, bLt_irrefl, bLt_total⟩

/-- For lex ballots the `EraMono` invariant is free. -/
theorem Paxos.eraMono_lex {A : Type} {V : Type} (P : Paxos A Ballot V)
    (hlt : P.lt = bLt) (hera : P.era = Ballot.era) : P.EraMono := by
  intro b1 b2 h
  rw [hlt] at h; rw [hera]
  exact bLt_era_mono b1 b2 h
```

```bash
lake build UVRR.LexBallot 2>&1 | tail -1
```

```output
Build completed successfully (4 jobs).
```

```bash
printf 'import UVRR.LexBallot\n#print axioms bLt_wf\n#print axioms bLt_total\n#print axioms bLt_era_mono\n#print axioms Paxos.theorem10_lex\n' | lake env lean --stdin
```

```output
'bLt_wf' does not depend on any axioms
'bLt_total' depends on axioms: [propext, Classical.choice, Quot.sound]
'bLt_era_mono' depends on axioms: [propext, Quot.sound]
'Paxos.theorem10_lex' depends on axioms: [propext, Classical.choice, Quot.sound]
```

**Reading the output.** Well-foundedness comes from Lean core's Prod.lex on Nat; the remaining facts are settled by omega after unfolding the lexicographic characterisation bLt_iff. Classical.choice appears only through the decidability instances used by omega.
