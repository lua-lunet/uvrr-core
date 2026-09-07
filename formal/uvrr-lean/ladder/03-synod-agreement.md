# Rung 3: Synod consistency (Turner Appendix B, Theorem 8)

*2026-09-05T23:31:04Z by Showboat 0.6.1*
<!-- showboat-id: ce779a0a-d325-43f9-b239-d98667363c69 -->

**Claim.** Given the six Synod invariants S1–S6 of the paper's Fig. 1 as hypotheses over an arbitrary ballot type with a well-founded strict total order, we prove Lemma 6, Lemma 7 and Theorem 8: if chosen(b1) and chosen(b2) then v(b1) = v(b2).

**Why this is the heart of it.** S1 is the weakened quorum requirement: only QI(b1) ⌢ QII(b2) is required when proposed(b1), chosen(b2) and b2 ≺ b1 — not equality of quorum systems. That weakening is exactly what lets configurations change between ballots. Lemma 7 is proved by well-founded induction on b1 exactly as in the paper.

**Encoding notes.** S4's 'v(b) = v(max P)' is stated as the existence of a maximal reported last-vote bm with v(b) = v(bm); this is what a leader actually computes from a finite set of responses. Theorem 8 additionally needs phase-II quorums to be nonempty (the paper's 'nonempty quorum').

```bash
cat UVRR/Synod.lean
```

```output
/-
  Rung 3 — Consistency of the Synod algorithm (Turner, UPaxos paper, Appendix B).

  We take the six invariants S1–S6 of Fig. 1 as hypotheses over an arbitrary
  ballot type `B` with a well-founded strict total order `lt`, arbitrary node
  type `A`, value type `V`, and per-ballot quorum systems `QI QII : B → QSys A`.
  We prove Lemma 6, Lemma 7 and Theorem 8 (agreement) exactly as in the paper.

  Lean 4 core only (no Mathlib). Quorum systems are predicates on node-sets,
  node-sets are predicates on nodes, so `Frown` is the paper's ⌢ operator.
-/

universe u v w

/-- A set of nodes, as a predicate. -/
abbrev NSet (A : Type u) := A → Prop
/-- A quorum system: a set of node-sets. -/
abbrev QSys (A : Type u) := NSet A → Prop

/-- Turner's frown: every q1 ∈ Q1 and q2 ∈ Q2 intersect. -/
def Frown {A : Type u} (Q1 Q2 : QSys A) : Prop :=
  ∀ q1 q2, Q1 q1 → Q2 q2 → ∃ a, q1 a ∧ q2 a

/-- The abstract Synod history: which messages were (ever) sent, plus the
value function `v : B → V` and the quorum systems in force for each ballot. -/
structure Synod (A : Type u) (B : Type v) (V : Type w) where
  lt        : B → B → Prop
  QI        : B → QSys A
  QII       : B → QSys A
  promised  : A → B → Prop          -- promised(a,b)   : free promise
  promisedV : A → B → B → Prop      -- promised(a,b;b'): promise carrying last vote b'
  proposed  : B → Prop
  accepted  : A → B → Prop
  chosen    : B → Prop
  v         : B → V

namespace Synod
variable {A : Type u} {B : Type v} {V : Type w} (S : Synod A B V)

/-- Ballot order is a well-founded strict total order. -/
structure Order : Prop where
  wf     : WellFounded S.lt
  trans  : ∀ a b c, S.lt a b → S.lt b c → S.lt a c
  irrefl : ∀ a, ¬ S.lt a a
  total  : ∀ a b, S.lt a b ∨ a = b ∨ S.lt b a

/-- S1: if proposed(b1), chosen(b2) and b2 ≺ b1 then QI(b1) ⌢ QII(b2). -/
def S1 : Prop := ∀ b1 b2, S.proposed b1 → S.chosen b2 → S.lt b2 b1 → Frown (S.QI b1) (S.QII b2)
/-- S2: a free promise for b forbids any acceptance below b. -/
def S2 : Prop := ∀ a b b', S.promised a b → S.lt b' b → ¬ S.accepted a b'
/-- S3: promised(a,b;b') means b' ≺ b, accepted(a,b'), and nothing accepted strictly between. -/
def S3 : Prop := ∀ a b b', S.promisedV a b b' →
  S.lt b' b ∧ S.accepted a b' ∧ ∀ b'', S.lt b' b'' → S.lt b'' b → ¬ S.accepted a b''
/-- S4: proposed(b) is backed by a phase-I quorum of promises; if any promise
carried a last vote, `v b` is the value of the greatest such last vote
(`max P` is bundled as an existential, which is what a real leader computes). -/
def S4 : Prop := ∀ b, S.proposed b → ∃ q, S.QI b q ∧
  (∀ a, q a → S.promised a b ∨ ∃ b', S.promisedV a b b') ∧
  ((∃ a b', q a ∧ S.promisedV a b b') →
    ∃ bm, (∃ a, q a ∧ S.promisedV a b bm) ∧
      (∀ a b', q a → S.promisedV a b b' → ¬ S.lt bm b') ∧
      S.v b = S.v bm)
/-- S5: accepted(a,b) ⇒ proposed(b). -/
def S5 : Prop := ∀ a b, S.accepted a b → S.proposed b
/-- S6: chosen(b) is backed by a phase-II quorum of acceptances. -/
def S6 : Prop := ∀ b, S.chosen b → ∃ q, S.QII b q ∧ ∀ a, q a → S.accepted a b

/-- All six Synod invariants. -/
structure Inv : Prop where
  s1 : S.S1
  s2 : S.S2
  s3 : S.S3
  s4 : S.S4
  s5 : S.S5
  s6 : S.S6

variable {S}

/-- Lemma 6: accepted(a,b2), promised(a,b1;b3), b2 ≺ b1 ⇒ b2 ≼ b3. -/
theorem lemma6 (_ho : S.Order) (hI : S.Inv) {a : A} {b1 b2 b3 : B}
    (hacc : S.accepted a b2) (hpv : S.promisedV a b1 b3) (hlt : S.lt b2 b1) :
    ¬ S.lt b3 b2 := by
  intro h32
  obtain ⟨_, _, hgap⟩ := hI.s3 a b1 b3 hpv
  exact hgap b2 h32 hlt hacc

/-- Lemma 7: chosen(b2), proposed(b1), b2 ≺ b1 ⇒ v(b1) = v(b2).
Proof by well-founded induction on b1, following the paper. -/
theorem lemma7 (ho : S.Order) (hI : S.Inv) :
    ∀ b1 b2, S.chosen b2 → S.proposed b1 → S.lt b2 b1 → S.v b1 = S.v b2 := by
  intro b1
  induction b1 using ho.wf.induction with
  | _ b1 ih =>
  intro b2 hch hpr hlt
  obtain ⟨qII, hqII, haccAll⟩ := hI.s6 b2 hch
  obtain ⟨qI, hqI, hprom, hmax⟩ := hI.s4 b1 hpr
  obtain ⟨a, haI, haII⟩ := hI.s1 b1 b2 hpr hch hlt qI qII hqI hqII
  have hacc : S.accepted a b2 := haccAll a haII
  -- a cannot have made a free promise for b1 (S2), so it reported a last vote
  have hnotfree : ¬ S.promised a b1 := fun hp => hI.s2 a b1 b2 hp hlt hacc
  obtain ⟨b', hb'⟩ : ∃ b', S.promisedV a b1 b' := by
    rcases hprom a haI with h | h
    · exact absurd h hnotfree
    · exact h
  obtain ⟨bm, ⟨a', ha'I, hpvm⟩, hbm_max, hv⟩ := hmax ⟨a, b', haI, hb'⟩
  -- b2 ≼ b' ≼ bm  (Lemma 6 and maximality)
  have h2b' : ¬ S.lt b' b2 := lemma6 ho hI hacc hb' hlt
  have hb'm : ¬ S.lt bm b' := hbm_max a b' haI hb'
  obtain ⟨hmlt, haccm, _⟩ := hI.s3 a' b1 bm hpvm
  -- either bm = b2, or b2 ≺ bm ≺ b1 and we recurse
  rcases ho.total b2 bm with h | h | h
  · -- b2 ≺ bm ≺ b1: proposed(bm) by S5; induction hypothesis gives v bm = v b2
    have hprm : S.proposed bm := hI.s5 a' bm haccm
    have := ih bm hmlt b2 hch hprm h
    exact hv.trans this
  · exact hv.trans (congrArg S.v h.symm)
  · -- bm ≺ b2: contradicts b2 ≼ b' ≼ bm
    exfalso
    rcases ho.total b' b2 with h1 | h1 | h1
    · exact h2b' h1
    · exact hb'm (h1 ▸ h)
    · exact hb'm (ho.trans _ _ _ h h1)

/-- Theorem 8 (agreement): chosen(b1) and chosen(b2) ⇒ v(b1) = v(b2). -/
theorem theorem8 (ho : S.Order) (hI : S.Inv)
    (hne : ∀ b q, S.QII b q → ∃ a, q a)   -- phase-II quorums are nonempty
    {b1 b2 : B} (h1 : S.chosen b1) (h2 : S.chosen b2) : S.v b1 = S.v b2 := by
  have prop_of_chosen : ∀ b, S.chosen b → S.proposed b := by
    intro b hb
    obtain ⟨q, hq, hacc⟩ := hI.s6 b hb
    obtain ⟨a, ha⟩ := hne b q hq
    exact hI.s5 a b (hacc a ha)
  rcases ho.total b2 b1 with h | h | h
  · exact lemma7 ho hI b1 b2 h2 (prop_of_chosen b1 h1) h
  · exact congrArg S.v h.symm
  · exact (lemma7 ho hI b2 b1 h1 (prop_of_chosen b2 h2) h).symm

end Synod
```

```bash
lake build UVRR.Synod 2>&1 | tail -1
```

```output
Build completed successfully (2 jobs).
```

```bash
printf 'import UVRR.Synod\n#print axioms Synod.lemma6\n#print axioms Synod.lemma7\n#print axioms Synod.theorem8\n' | lake env lean --stdin
```

```output
'Synod.lemma6' does not depend on any axioms
'Synod.lemma7' does not depend on any axioms
'Synod.theorem8' does not depend on any axioms
```

**Reading the output.** Theorem 8 depends on no axioms at all: the proof is fully constructive relative to the hypotheses. Every hypothesis is an explicit field of Synod.Inv, so nothing is hidden in typeclass instances.
