# Rung 1: Frown checker and the 3-zone hot-swap

*2026-09-05T23:31:02Z by Showboat 0.6.1*
<!-- showboat-id: 45f94ec7-07d8-4c66-9b91-eedd96aa50f6 -->

**Claim.** Turner's frown operator Q1 ⌢ Q2 (every quorum of Q1 meets every quorum of Q2) is decidable for explicitly listed quorums, and the paper's/blog's server-replacement schedule (zones A,B,C hold W,X,Y; add Z beside W; remove Y) satisfies the UPaxos safety equation P1: QII_e ⌢ QI_e ⌢ QII_{e+1} at every era boundary, using plain majorities.

**Source.** UPaxos paper §IV-A (invariant P1) and the blog post 'Paxos Voting Weights' (first table). This corrects the earlier scratch baby-step, which had encoded QI_{e+1} ⌢ QII_e as the third overlap; the paper's third frown is the intra-era QII_{e+1} ⌢ QI_{e+1}, and Turner confirms the prepare-to-prepare overlap is not required.

**Negative control.** Replacing the whole cluster {0,1,2} by {3,4,5} in one step keeps both intra-era frowns but fails the cross-era frown, so P1 rejects it.

```bash
cat UVRR/Structure.lean
```

```output
/-
  Rung 1 — Finite, decidable quorum systems and the 3-zone hot-swap schedule.

  Abstract `Frown` (Synod.lean) is a Prop over predicates. For CONCRETE
  configurations we list quorums explicitly (`List (List Nat)`) and give a
  boolean checker `frownB` proved equivalent to `Frown` on the induced quorum
  systems. Concrete schedules are then checked by `decide` — kernel-checked,
  no `native_decide`.

  The schedule below is the "replace a server" scenario from the paper /
  blog: zones A,B,C hold W,X,Y (nodes 0,1,2); Z (node 3) is added next to W in
  zone A, then Y is removed. Quorums are simple majorities in each era. P1
  (QII_e ⌢ QI_e ⌢ QII_{e+1}) holds for every era, so by Theorem 10 no
  instance can choose two values across this reconfiguration.
-/
import UVRR.Eras

/-- The quorum system induced by an explicit list of quorums. -/
def ofList (Q : List (List Nat)) : QSys Nat := fun q => ∃ l, l ∈ Q ∧ ∀ a, q a ↔ a ∈ l

/-- Boolean frown check on explicit quorum lists. -/
def frownB (Q1 Q2 : List (List Nat)) : Bool :=
  Q1.all fun l1 => Q2.all fun l2 => l1.any fun a => decide (a ∈ l2)

theorem frownB_iff (Q1 Q2 : List (List Nat)) :
    frownB Q1 Q2 = true ↔ Frown (ofList Q1) (ofList Q2) := by
  unfold frownB Frown ofList
  simp only [List.all_eq_true, List.any_eq_true, decide_eq_true_eq]
  constructor
  · intro h q1 q2 ⟨l1, hl1, hq1⟩ ⟨l2, hl2, hq2⟩
    obtain ⟨a, ha1, ha2⟩ := h l1 hl1 l2 hl2
    exact ⟨a, (hq1 a).2 ha1, (hq2 a).2 ha2⟩
  · intro h l1 hl1 l2 hl2
    obtain ⟨a, ha1, ha2⟩ := h (· ∈ l1) (· ∈ l2) ⟨l1, hl1, fun _ => Iff.rfl⟩ ⟨l2, hl2, fun _ => Iff.rfl⟩
    exact ⟨a, ha1, ha2⟩

/-- An explicit configuration ⟨QI, QII⟩. -/
structure CfgL where
  QI  : List (List Nat)
  QII : List (List Nat)

/-- Boolean P1 for one era boundary: QII_e ⌢ QI_e and QI_e ⌢ QII_{e+1}. -/
def p1B (c c' : CfgL) : Bool := frownB c.QII c.QI && frownB c.QI c'.QII

theorem p1B_sound (c c' : CfgL) (h : p1B c c' = true) :
    Frown (ofList c.QII) (ofList c.QI) ∧ Frown (ofList c.QI) (ofList c'.QII) := by
  unfold p1B at h
  rw [Bool.and_eq_true] at h
  exact ⟨(frownB_iff _ _).1 h.1, (frownB_iff _ _).1 h.2⟩

/-- Majorities of {W,X,Y} = {0,1,2}. -/
def maj3 : List (List Nat) := [[0,1],[0,2],[1,2]]
/-- Majorities of {W,X,Y,Z} = {0,1,2,3} (three of four). -/
def maj4 : List (List Nat) := [[0,1,2],[0,1,3],[0,2,3],[1,2,3]]
/-- Majorities of {W,X,Z} = {0,1,3}. -/
def maj3' : List (List Nat) := [[0,1],[0,3],[1,3]]

/-- The hot-swap schedule: era 0 = {W,X,Y}, era 1 = {W,X,Y,Z}, era ≥ 2 = {W,X,Z}. -/
def hotswap : Nat → CfgL
  | 0 => ⟨maj3, maj3⟩
  | 1 => ⟨maj4, maj4⟩
  | _ => ⟨maj3', maj3'⟩

theorem hotswap_ge2 (n : Nat) : hotswap (n+2) = ⟨maj3', maj3'⟩ := rfl

/-- P1 holds at every era boundary of the hot-swap schedule (kernel-checked). -/
theorem hotswap_p1 : ∀ e,
    Frown (ofList (hotswap e).QII) (ofList (hotswap e).QI) ∧
    Frown (ofList (hotswap e).QI) (ofList (hotswap (e+1)).QII) := by
  intro e
  apply p1B_sound
  match e with
  | 0 => decide
  | 1 => decide
  | n+2 => rw [hotswap_ge2, hotswap_ge2]; decide

/-- Negative control: replacing the whole cluster {0,1,2} by {3,4,5} in one
step. Intra-era majorities are fine, but the cross-era frown fails, so P1 is
violated and (Rung 5) agreement can be broken. -/
def wholesale : Nat → CfgL
  | 0 => ⟨maj3, maj3⟩
  | _ => ⟨[[3,4],[3,5],[4,5]], [[3,4],[3,5],[4,5]]⟩

theorem wholesale_intra : frownB (wholesale 0).QII (wholesale 0).QI = true ∧
    frownB (wholesale 1).QII (wholesale 1).QI = true := by decide

theorem wholesale_cross_fails : ¬ Frown (ofList (wholesale 0).QI) (ofList (wholesale 1).QII) := by
  rw [← frownB_iff]; decide
```

```bash
lake build UVRR.Structure 2>&1 | tail -1
```

```output
Build completed successfully (4 jobs).
```

```bash
printf 'import UVRR.Structure\n#print axioms frownB_iff\n#print axioms hotswap_p1\n#print axioms wholesale_cross_fails\n' | lake env lean --stdin
```

```output
'frownB_iff' depends on axioms: [propext, Quot.sound]
'hotswap_p1' depends on axioms: [propext, Quot.sound]
'wholesale_cross_fails' depends on axioms: [propext, Quot.sound]
```

**Reading the output.** frownB_iff is the soundness/completeness bridge from the Bool checker to the Prop-level Frown; hotswap_p1 is closed by decide (kernel evaluation, no native_decide) for eras 0 and 1 and by rewriting for all later eras; wholesale_cross_fails shows the checker also refutes. Axioms listed are Lean's standard propext/Quot.sound only.
