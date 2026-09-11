# Rung 10: Operational promise and acceptance invariants

*2026-09-11T00:42:01Z by Showboat 0.6.1*
<!-- showboat-id: 1755a4c6-4d21-426f-b09c-3bb44d28e4d7 -->

Claim: for every finite execution of the single-era acceptor, free promises exclude lower votes forever and reported last votes remain maximal below the promised round. This discharges S2 and S3 for this component. It does not yet prove the proposer, replicated-log view selection, cross-era transitions, or diskless recovery. A concrete bypass of the acceptance watermark violates the invariant.

```bash
cat UVRR/Acceptor.lean
```

```output
import UVRR.Synod

/-! An operational single-era acceptor. Ballots are natural view rounds.
Votes and emitted replies are append-only ghost histories; floor is the local
promise watermark. Promise generation reads the most recent accepted ballot.
This subsystem neither chooses values nor implements diskless restart.
-/
namespace Acceptor

structure State where
  floor : Nat
  votes : List Nat
  replies : List (Nat × Option Nat)

def initial : State := ⟨0, [], []⟩

def accept (s : State) (b : Nat) : State :=
  { s with floor := b, votes := b :: s.votes }

def promise (s : State) (b : Nat) : State :=
  { s with floor := b, replies := (b, s.votes.head?) :: s.replies }

inductive Step : State → State → Prop where
  | accept (s : State) (b : Nat) (guard : s.floor ≤ b) : Step s (accept s b)
  | promise (s : State) (b : Nat) (guard : s.floor < b) : Step s (promise s b)

inductive Reachable : State → Prop where
  | initial : Reachable initial
  | step {s t} : Reachable s → Step s t → Reachable t

def ReplySafe (s : State) (b : Nat) (last : Option Nat) : Prop :=
  b ≤ s.floor ∧ match last with
    | none => ∀ v ∈ s.votes, ¬ v < b
    | some c => c < b ∧ c ∈ s.votes ∧ ∀ v ∈ s.votes, v < b → v ≤ c

structure Inv (s : State) : Prop where
  bounded : ∀ v ∈ s.votes, v ≤ s.floor
  headMax : ∀ c rest, s.votes = c :: rest → ∀ v ∈ rest, v ≤ c
  replies : ∀ b last, (b, last) ∈ s.replies → ReplySafe s b last

theorem initial_inv : Inv initial where
  bounded := by simp [initial]
  headMax := by simp [initial]
  replies := by simp [initial]

theorem accept_preserves {s : State} (h : Inv s) {b : Nat} (hb : s.floor ≤ b) :
    Inv (accept s b) where
  bounded := by
    intro v hv
    change v ∈ b :: s.votes at hv
    rcases List.mem_cons.mp hv with he | hv
    · subst v; exact Nat.le_refl _
    · exact Nat.le_trans (h.bounded v hv) hb
  headMax := by
    intro c rest he v hv
    change b :: s.votes = c :: rest at he
    obtain ⟨hc, hr⟩ := List.cons.inj he
    subst c; subst rest
    exact Nat.le_trans (h.bounded v hv) hb
  replies := by
    intro r last hr
    obtain ⟨hf, hs⟩ := h.replies r last hr
    refine ⟨Nat.le_trans hf hb, ?_⟩
    cases last with
    | none =>
      intro v hv hvlt
      rcases List.mem_cons.mp hv with he | hv
      · subst v; omega
      · exact hs v hv hvlt
    | some c =>
      refine ⟨hs.1, List.mem_cons_of_mem _ hs.2.1, ?_⟩
      intro v hv hvlt
      rcases List.mem_cons.mp hv with he | hv
      · subst v; omega
      · exact hs.2.2 v hv hvlt

theorem promise_preserves {s : State} (h : Inv s) {b : Nat} (hb : s.floor < b) :
    Inv (promise s b) where
  bounded := fun v hv => Nat.le_trans (h.bounded v hv) (Nat.le_of_lt hb)
  headMax := h.headMax
  replies := by
    intro r last hr
    change (r, last) ∈ (b, s.votes.head?) :: s.replies at hr
    rcases List.mem_cons.mp hr with he | hr
    · obtain ⟨hrb, hl⟩ := Prod.mk.inj he
      subst r; subst last
      refine ⟨Nat.le_refl _, ?_⟩
      cases hv : s.votes with
      | nil => simp [hv, promise]
      | cons c rest =>
        simp only [List.head?_cons]
        have hc : c ∈ s.votes := by rw [hv]; exact List.mem_cons_self
        refine ⟨Nat.lt_of_le_of_lt (h.bounded c hc) hb, hc, ?_⟩
        intro v hmem _
        change v ∈ s.votes at hmem
        rw [hv] at hmem
        rcases List.mem_cons.mp hmem with he | hm
        · subst v; exact Nat.le_refl _
        · exact h.headMax c rest hv v hm
    · obtain ⟨hf, hs⟩ := h.replies r last hr
      exact ⟨Nat.le_trans hf (Nat.le_of_lt hb), hs⟩

theorem reachable_inv {s : State} (hr : Reachable s) : Inv s := by
  induction hr with
  | initial => exact initial_inv
  | step _ hs ih =>
    cases hs with
    | accept _ hg => exact accept_preserves ih hg
    | promise _ hg => exact promise_preserves ih hg

/-- Historical S2, obtained from executions rather than assumed. -/
theorem free_forbids {s : State} (hr : Reachable s) {b v : Nat}
    (hp : (b, none) ∈ s.replies) (hv : v < b) : v ∉ s.votes := by
  intro ha
  exact ((reachable_inv hr).replies b none hp).2 v ha hv

/-- Historical S3, including preservation against all subsequent accepts. -/
theorem report_last {s : State} (hr : Reachable s) {b c : Nat}
    (hp : (b, some c) ∈ s.replies) :
    c < b ∧ c ∈ s.votes ∧ ∀ v, c < v → v < b → v ∉ s.votes := by
  obtain ⟨_, hc, ha, hm⟩ := (reachable_inv hr).replies b (some c) hp
  refine ⟨hc, ha, ?_⟩
  intro v hcv hvb hva
  have := hm v hva hvb
  omega

/-- A concrete faulty transition bypassing the acceptance watermark invalidates
an emitted free promise. The good transition system refuses this exact step. -/
theorem missing_guard_counterexample :
    Reachable (promise initial 2) ∧
    ¬ (promise initial 2).floor ≤ 1 ∧
    ¬ Inv (accept (promise initial 2) 1) := by
  refine ⟨Reachable.step .initial (.promise initial 2 (by decide)), by decide, ?_⟩
  intro h
  have hs := h.replies 2 none (by simp [accept, promise, initial])
  have hf : 2 ≤ 1 := hs.1
  omega

end Acceptor
```

```bash
set -euo pipefail; lake build UVRR.Acceptor >/dev/null; echo "UVRR.Acceptor built"
```

```output
UVRR.Acceptor built
```

```bash
printf 'import UVRR.Acceptor\n#print axioms Acceptor.reachable_inv\n#print axioms Acceptor.free_forbids\n#print axioms Acceptor.report_last\n#print axioms Acceptor.missing_guard_counterexample\n' | lake env lean --stdin
```

```output
'Acceptor.reachable_inv' depends on axioms: [propext, Quot.sound]
'Acceptor.free_forbids' depends on axioms: [propext, Quot.sound]
'Acceptor.report_last' depends on axioms: [propext, Quot.sound]
'Acceptor.missing_guard_counterexample' depends on axioms: [propext, Quot.sound]
```
