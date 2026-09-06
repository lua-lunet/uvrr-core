# Rung 12: Suffix promises and the known-instance frontier

*2026-09-06T07:28:19Z by Showboat 0.6.1*
<!-- showboat-id: 04267799-332c-4eca-9d79-97422173c185 -->

Claim: the full promise bounds of UPaxos Figure 2 derive EraLe, and the expanded history satisfies the existing agreement theorem. This is a reduction between history invariants, not an operational preservation proof. Nonempty phase-I and phase-II quorums are explicit.

```bash
cat UVRR/MultiPromise.lean
```

```output
import UVRR.LexBallot

/-!
UPaxos Figure 2 with suffix promises and the known-instance frontier.
This is a reduction of the full *history invariants*, not an operational
preservation proof. In particular EraLe is derived rather than assumed.
The functions describing era/configuration knowledge remain fixed in a history;
an implementation must separately justify their append-only interpretation.
-/
namespace MultiPromise

structure History (A V : Type) where
  core : Paxos A Ballot V
  imax : Nat
  suffix : Nat → A → Ballot → Prop

namespace History

variable {A V : Type} (H : History A V)

def expanded : Paxos A Ballot V :=
  { H.core with promised := fun i a b =>
      H.core.promised i a b ∨ ∃ j, j ≤ i ∧ H.suffix j a b }

/-- Literal promise/frontier obligations from Figure 2. Nonempty phase-I
quorums are stated explicitly; a universal property over an empty family
cannot supply the witness needed to derive an era bound. -/
structure Inv : Prop where
  ballot_lt : H.core.lt = bLt
  ballot_era : H.core.era = Ballot.era
  eras_monotone : ∀ i j, i ≤ j → H.core.eInst i ≤ H.core.eInst j
  phase1_nonempty : ∀ e q, H.core.QI e q → ∃ a, q a
  p1 : H.core.P1
  p2 : ∀ j a b, H.suffix j a b →
    H.core.era b ≤ H.core.eInst (min j H.imax) ∧
    ∀ i b', j ≤ i → H.core.lt b' b → ¬ H.core.accepted i a b'
  p3 : ∀ i a b, H.core.promised i a b →
    H.core.era b ≤ H.core.eInst (min i H.imax) ∧
    ∀ b', H.core.lt b' b → ¬ H.core.accepted i a b'
  p4 : ∀ i a b b', H.core.promisedV i a b b' →
    H.core.era b ≤ H.core.eInst (min i H.imax) ∧
    H.core.lt b' b ∧ H.core.accepted i a b' ∧
    ∀ b'', H.core.lt b' b'' → H.core.lt b'' b → ¬ H.core.accepted i a b''
  p5 : H.expanded.P5
  p6 : H.core.P6
  p7 : ∀ i b, H.core.chosen i b → i ≤ H.imax ∧
    H.core.eInst i ≤ H.core.era b + 1 ∧
    ∃ q, H.core.QII (H.core.eInst i) q ∧ ∀ a, q a → H.core.accepted i a b

variable {H}

theorem promise_era (h : H.Inv) {i a b}
    (hp : H.expanded.promised i a b ∨ ∃ b', H.core.promisedV i a b b') :
    H.core.era b ≤ H.core.eInst i := by
  rcases hp with (hf | ⟨j, hji, hs⟩) | ⟨b', hv⟩
  · exact Nat.le_trans (h.p3 i a b hf).1
      (h.eras_monotone _ _ (Nat.min_le_left _ _))
  · exact Nat.le_trans (h.p2 j a b hs).1
      (h.eras_monotone _ _ (Nat.le_trans (Nat.min_le_left _ _) hji))
  · exact Nat.le_trans (h.p4 i a b b' hv).1
      (h.eras_monotone _ _ (Nat.min_le_left _ _))

/-- Discharges the former standalone EraLe assumption from P2--P5. -/
theorem eraLe (h : H.Inv) : H.expanded.EraLe := by
  intro i b hp
  obtain ⟨q, hq, hprom, _⟩ := h.p5 i b hp
  obtain ⟨a, ha⟩ := h.phase1_nonempty _ q hq
  exact promise_era h (hprom a ha)

theorem expanded_inv (h : H.Inv) : H.expanded.Inv where
  p1 := h.p1
  mono := Paxos.eraMono_lex H.expanded h.ballot_lt h.ballot_era
  eraLe := eraLe h
  p23 := by
    intro i a b b' hp hlt
    rcases hp with hf | ⟨j, hji, hs⟩
    · exact (h.p3 i a b hf).2 b' hlt
    · exact (h.p2 j a b hs).2 i b' hji hlt
  p4 := fun i a b b' hp => (h.p4 i a b b' hp).2
  p5 := h.p5
  p6 := h.p6
  p7 := fun i b hc => (h.p7 i b hc).2

theorem agreement (h : H.Inv)
    (hne : ∀ e q, H.core.QII e q → ∃ a, q a)
    {i b c} (hb : H.core.chosen i b) (hc : H.core.chosen i c) :
    H.core.v i b = H.core.v i c :=
  Paxos.theorem10_lex H.expanded h.ballot_lt h.ballot_era
    (expanded_inv h) hne hb hc

/-- Choice cannot occur beyond the known era frontier. -/
theorem chosen_frontier (h : H.Inv) {i b} (hc : H.core.chosen i b) : i ≤ H.imax :=
  (h.p7 i b hc).1

/-- A single finite suffix certificate denotes arbitrarily distant instances.
This is expressiveness of promises, not eventual completion of those slots. -/
theorem suffix_unbounded {j a b} (hp : H.suffix j a b) (distance : Nat) :
    H.expanded.promised (j + distance) a b :=
  Or.inr ⟨j, Nat.le_add_right j distance, hp⟩

end History
end MultiPromise
```

```bash
set -euo pipefail; lake build UVRR.MultiPromise >/dev/null; echo "UVRR.MultiPromise built"
```

```output
UVRR.MultiPromise built
```

```bash
printf 'import UVRR.MultiPromise\n#print axioms MultiPromise.History.eraLe\n#print axioms MultiPromise.History.expanded_inv\n#print axioms MultiPromise.History.agreement\n#print axioms MultiPromise.History.chosen_frontier\n#print axioms MultiPromise.History.suffix_unbounded\n' | lake env lean --stdin
```

```output
'MultiPromise.History.eraLe' depends on axioms: [propext]
'MultiPromise.History.expanded_inv' depends on axioms: [propext, Quot.sound]
'MultiPromise.History.agreement' depends on axioms: [propext, Classical.choice, Quot.sound]
'MultiPromise.History.chosen_frontier' does not depend on any axioms
'MultiPromise.History.suffix_unbounded' does not depend on any axioms
```
