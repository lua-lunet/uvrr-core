/-
  Rung 4 — Consistency of era-indexed Paxos (Turner, UPaxos paper, Appendix C).

  Configurations ⟨QI_e, QII_e⟩ are indexed by era e. Every ballot b carries an
  era e(b) (encoded in its most significant bits, so the ballot order respects
  eras). Every instance (slot) i has an era e(i). Phase I for ballot b uses
  QI_{e(b)}; phase II for instance i uses QII_{e(i)}.

  P1 (the UPaxos safety equation)      QII_e ⌢ QI_e ⌢ QII_{e+1}   for every e
  is the ONLY assumption about configurations. We prove Lemma 9
  (QI_{e(b1)} ⌢ QII_{e(i)} whenever proposed_i(b1), chosen_i(b2), b2 ≺ b1) and
  Theorem 10 (per-instance agreement) by instantiating the Synod result.

  Honest scope: the paper derives "proposed_i(b) ⇒ e(b) ≤ e(i)" from the
  multi-promise machinery (P2–P5, i_max, e nondecreasing). We take that
  consequence as the hypothesis `eraLe` here; multi-promises are not modelled.
-/
import UVRR.Synod

universe u v w

theorem Frown.symm {A : Type u} {Q1 Q2 : QSys A} (h : Frown Q1 Q2) : Frown Q2 Q1 := by
  intro q2 q1 h2 h1
  obtain ⟨a, ha1, ha2⟩ := h q1 q2 h1 h2
  exact ⟨a, ha2, ha1⟩

/-- Era-indexed Paxos history (instances indexed by `Nat`). -/
structure Paxos (A : Type u) (B : Type v) (V : Type w) where
  lt        : B → B → Prop
  era       : B → Nat                  -- e(b)
  QI        : Nat → QSys A             -- QI_e
  QII       : Nat → QSys A             -- QII_e
  eInst     : Nat → Nat                -- e(i)
  promised  : Nat → A → B → Prop       -- promised_i(a,b) (multi-promises already expanded)
  promisedV : Nat → A → B → B → Prop   -- promised_i(a,b;b')
  proposed  : Nat → B → Prop
  accepted  : Nat → A → B → Prop
  chosen    : Nat → B → Prop
  v         : Nat → B → V

namespace Paxos
variable {A : Type u} {B : Type v} {V : Type w} (P : Paxos A B V)

/-- P1: the UPaxos safety equation, era-adjacent. -/
def P1 : Prop := ∀ e, Frown (P.QII e) (P.QI e) ∧ Frown (P.QI e) (P.QII (e+1))

/-- Ballot order respects eras (era lives in the most significant bits). -/
def EraMono : Prop := ∀ b1 b2, P.lt b2 b1 → P.era b2 ≤ P.era b1

/-- Consequence of P2–P5 in the paper: a proposal's ballot era is at most the instance era. -/
def EraLe : Prop := ∀ i b, P.proposed i b → P.era b ≤ P.eInst i

/-- P2/P3 (promise part): a free promise for b forbids acceptances below b. -/
def P23 : Prop := ∀ i a b b', P.promised i a b → P.lt b' b → ¬ P.accepted i a b'
/-- P4: promised_i(a,b;b') carries the greatest acceptance below b. -/
def P4 : Prop := ∀ i a b b', P.promisedV i a b b' →
  P.lt b' b ∧ P.accepted i a b' ∧ ∀ b'', P.lt b' b'' → P.lt b'' b → ¬ P.accepted i a b''
/-- P5: proposals are backed by a QI_{e(b)} quorum; value rule as in S4. -/
def P5 : Prop := ∀ i b, P.proposed i b → ∃ q, P.QI (P.era b) q ∧
  (∀ a, q a → P.promised i a b ∨ ∃ b', P.promisedV i a b b') ∧
  ((∃ a b', q a ∧ P.promisedV i a b b') →
    ∃ bm, (∃ a, q a ∧ P.promisedV i a b bm) ∧
      (∀ a b', q a → P.promisedV i a b b' → ¬ P.lt bm b') ∧
      P.v i b = P.v i bm)
/-- P6. -/
def P6 : Prop := ∀ i a b, P.accepted i a b → P.proposed i b
/-- P7: chosen_i(b) ⇒ e(i) ≤ e(b)+1 and a QII_{e(i)} quorum accepted. -/
def P7 : Prop := ∀ i b, P.chosen i b → P.eInst i ≤ P.era b + 1 ∧
  ∃ q, P.QII (P.eInst i) q ∧ ∀ a, q a → P.accepted i a b

structure Inv : Prop where
  p1    : P.P1
  mono  : P.EraMono
  eraLe : P.EraLe
  p23   : P.P23
  p4    : P.P4
  p5    : P.P5
  p6    : P.P6
  p7    : P.P7

variable {P}

/-- Lemma 9: proposed_i(b1), chosen_i(b2), b2 ≺ b1 ⇒ QI_{e(b1)} ⌢ QII_{e(i)}. -/
theorem lemma9 (hI : P.Inv) {i : Nat} {b1 b2 : B}
    (hpr : P.proposed i b1) (hch : P.chosen i b2) (hlt : P.lt b2 b1) :
    Frown (P.QI (P.era b1)) (P.QII (P.eInst i)) := by
  have h1 : P.era b1 ≤ P.eInst i := hI.eraLe i b1 hpr
  have h2 : P.eInst i ≤ P.era b2 + 1 := (hI.p7 i b2 hch).1
  have h3 : P.era b2 ≤ P.era b1 := hI.mono b1 b2 hlt
  -- e(i) ∈ {e(b1), e(b1)+1}
  rcases Nat.eq_or_lt_of_le h1 with h | h
  · rw [← h]; exact (hI.p1 (P.era b1)).1.symm
  · have : P.eInst i = P.era b1 + 1 := by omega
    rw [this]; exact (hI.p1 (P.era b1)).2

/-- The Synod history seen by a single instance `i`. -/
def toSynod (i : Nat) : Synod A B V where
  lt        := P.lt
  QI        := fun b => P.QI (P.era b)
  QII       := fun _ => P.QII (P.eInst i)
  promised  := P.promised i
  promisedV := P.promisedV i
  proposed  := P.proposed i
  accepted  := P.accepted i
  chosen    := P.chosen i
  v         := P.v i

/-- The Paxos invariants imply the Synod invariants for every instance. -/
theorem toSynod_inv (hI : P.Inv) (i : Nat) : (P.toSynod i).Inv where
  s1 := fun _ _ hpr hch hlt => lemma9 hI hpr hch hlt
  s2 := fun a b b' hp hlt => hI.p23 i a b b' hp hlt
  s3 := fun a b b' h => hI.p4 i a b b' h
  s4 := fun b h => hI.p5 i b h
  s5 := fun a b h => hI.p6 i a b h
  s6 := fun b h => (hI.p7 i b h).2

/-- Theorem 10 (per-instance agreement under reconfiguration):
chosen_i(b1) and chosen_i(b2) ⇒ v_i(b1) = v_i(b2). -/
theorem theorem10 (hI : P.Inv)
    (ho : WellFounded P.lt ∧ (∀ a b c, P.lt a b → P.lt b c → P.lt a c) ∧
          (∀ a, ¬ P.lt a a) ∧ (∀ a b, P.lt a b ∨ a = b ∨ P.lt b a))
    (hne : ∀ e q, P.QII e q → ∃ a, q a)
    {i : Nat} {b1 b2 : B} (h1 : P.chosen i b1) (h2 : P.chosen i b2) :
    P.v i b1 = P.v i b2 :=
  Synod.theorem8 (S := P.toSynod i) ⟨ho.1, ho.2.1, ho.2.2.1, ho.2.2.2⟩ (toSynod_inv hI i)
    (fun _ q hq => hne _ q hq) h1 h2

end Paxos
