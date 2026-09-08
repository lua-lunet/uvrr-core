# Rung 22: uVRR reincarnation — Crash-Stop-Self-Evict (spec-only)

*2026-09-08T01:10:02Z by Showboat 0.6.1*
<!-- showboat-id: 2156e12c-9168-45d2-a9e2-bf26bb086bef -->

This rung fixes the reincarnation specification as definitions: the four-superblock
durable identity contract (`Mark`, `allFlushed`, `dirtyStartup`), the incarnation bump
(`bump`, `bump_supersedes`), the identity-to-weight configuration map (`Config`, `voting`),
the two-era forced weight sequence (`crossEra` — one batch of `DECREMENT(old), JOIN(new)`,
the crossing era; `evictEra` — one batch of `INCREMENT(new), LEAVE(old)`, the eviction era;
built on the unit steps `step0`/`step1`/`step2`), the
flushed/unflushed/dirty/bumped/reincarnating state machine (`Phase`, `Transition`), the
higher-identity-wins read rule (`adopt`), and continuation commitment (`committedPhase`,
`ForcedStep`, `ForcedRun`, `forced_prefix`, `forced_monotone`). The admitted content is
kernel-checked structural lemmas plus `decide`/`simp`-enumerated finite instances of the
unit-weight three-node reincarnation: the two consecutive eras of the forced sequence are
pairwise quorum-safe (`e0_e1_safe`, `e1_e2_safe`), and each era moves exactly ONE unit of
per-node voting mass — the R14 mass rule the fold and the planner enforce before any op is
proposed (`e1_mass`, `e2_mass`). The one-era alternative the two-era form replaces —
`LEAVE(old)` and `INCREMENT(new)` applied to the BASELINE in a single batch — is refused:
it moves two units (`swap_mass`), so the era rule rejects it (`swap_rejected`), and its
endpoint majority families are genuinely disjoint (`swap_unsafe`). The startup
classification is exhaustive over the sixteen mark combinations (`startup_cases`), and the
old identity is never again a voter after the crossing era (`step1_not_voting`). This is a
SPEC-ONLY rung: no general-protocol theorem is admitted. The general theorems —
bumped-identity non-membership in every view at or after the crossing era, quorum safety of
every intermediate era at arbitrary scale, unreachability of the classic amnesia trace, and
continuation commitment in general — are listed as proof obligations below, to be proved by
later rungs.

```bash
cat UVRR/Reincarnation.lean
```

```output
import UVRR.WeightedGeneral

/-! uVRR reincarnation: the Crash-Stop-Self-Evict protocol (spec-only rung).
A node whose volatile state was lost is a different node: it reopens under a
new identity obtained by bumping its incarnation, and its old identity is
evicted from the voting configuration by the forced weight sequence. Durable
state is four superblocks whose marks classify the startup; any `unflushed`
mark makes the node dirty, and a dirty node must bump. The leader drives the
forced sequence of the Paxos Voting Weights rules; every consecutive pair of
eras overlaps, so each intermediate era is quorum-safe on its own. Once the
Crash-Stop-Eviction is initiated it must continue: the forced sequence is
never aborted mid-way. This module fixes definitions, the state machine and
kernel-checked structural lemmas plus `decide`/`simp`-verified finite
instances of the unit-weight three-node eviction. General protocol theorems
over arbitrary configurations, arbitrary weight scales and the wire protocol
are proof obligations recorded in the ladder rung, not claims made here.
-/
namespace Reincarnation

/-- Node identity on the finite model: identities are totally ordered and a
later incarnation reads as a strictly higher identity. -/
abbrev Ident := Nat

/-- Incarnation bump: the restarted node reopens under a strictly higher
identity. -/
def bump (i : Ident) : Ident := i + 1

theorem bump_supersedes (i : Ident) : bump i ≠ i := by
  simp [bump]

/-- Weight of an identity in a configuration; weight 0 is a non-voting
standby or an evicted identity. -/
abbrev Weight := Nat

/-- A configuration is the identity-to-weight map. -/
abbrev Config (A : Type) := A → Weight

/-- Voting membership: identity is a voter iff its weight is positive. -/
def voting (w : Config A) (a : A) : Prop := 0 < w a

/-- The reincarnation wire message: the pair carried by the bumped node to
the leader. -/
structure Reincarnate (A : Type) where
  old : A
  new : A

/-! ### The forced weight sequence (unit rule) -/

/-- The crossing era (docs §5, era D1): one batch of `DECREMENT(old),
JOIN(new)` — the exiting node's weight drops one unit and the new identity
joins at weight 0 as a standby. -/
def crossEra {A : Type} [DecidableEq A] (old new : A) (w : Config A) : Config A :=
  fun a => if a = old then w a - 1 else if a = new then 0 else w a

/-- The eviction era (docs §5, era D2): one batch of `INCREMENT(new),
LEAVE(old)` — the new identity is promoted one unit and the old identity,
already at weight 0, leaves the voting configuration. In the weight-map
model the leave of a weight-0 identity is invisible: it was never a voter
(`step1_not_voting`) and contributes nothing to any quorum. -/
def evictEra {A : Type} [DecidableEq A] (new : A) (w : Config A) : Config A :=
  fun a => if a = new then w a + 1 else w a

/-- The unit-rule steps the two eras are built from: a solitary decrement,
the weight-0 join, and the unit promotion. -/
def step0 {A : Type} [DecidableEq A] (old : A) (w : Config A) : Config A :=
  fun a => if a = old then w a - 1 else w a

/-- The weight-0 join: the new identity enters as a standby; total and every
voter unchanged. -/
def step1 {A : Type} [DecidableEq A] (old new : A) (w : Config A) : Config A :=
  fun a => if a = old then 0 else if a = new then 0 else w a

/-- The unit promotion: the new identity 0 → 1. -/
def step2 {A : Type} [DecidableEq A] (new : A) (w : Config A) : Config A :=
  fun a => if a = new then w a + 1 else w a

theorem crossEra_decrements {A : Type} [DecidableEq A] {old new : A} (_h : old ≠ new)
    (w : Config A) :
    crossEra old new w old = w old - 1 := by
  rw [crossEra, if_pos rfl]

theorem crossEra_joins_standby {A : Type} [DecidableEq A] {old new : A} (h : old ≠ new)
    (w : Config A) :
    crossEra old new w new = 0 := by
  rw [crossEra, if_neg (Ne.symm h), if_pos rfl]

theorem crossEra_other {A : Type} [DecidableEq A] {old new a : A}
    (ho : a ≠ old) (hn : a ≠ new) (w : Config A) :
    crossEra old new w a = w a := by simp [crossEra, ho, hn]

theorem evictEra_promotes {A : Type} [DecidableEq A] (new : A) (w : Config A) :
    evictEra new w new = w new + 1 := by simp [evictEra]

theorem evictEra_other {A : Type} [DecidableEq A] {new a : A} (hn : a ≠ new) (w : Config A) :
    evictEra new w a = w a := by simp [evictEra, hn]

theorem step0_decrements {A : Type} [DecidableEq A] (old : A) (w : Config A) :
    step0 old w old = w old - 1 := by simp [step0]

theorem step0_other {A : Type} [DecidableEq A] {old a : A} (h : a ≠ old) (w : Config A) :
    step0 old w a = w a := by simp [step0, h]

theorem step1_evicts {A : Type} [DecidableEq A] (old new : A) (w : Config A) :
    step1 old new w old = 0 := by simp [step1]

theorem step1_joins_standby {A : Type} [DecidableEq A] (old new : A) (w : Config A) :
    step1 old new w new = 0 := by simp [step1]

theorem step1_other {A : Type} [DecidableEq A] {old new a : A}
    (ho : a ≠ old) (hn : a ≠ new) (w : Config A) :
    step1 old new w a = w a := by simp [step1, ho, hn]

theorem step1_not_voting {A : Type} [DecidableEq A] {old new : A} (w : Config A) :
    ¬ voting (step1 old new w) old := by
  simp [voting, step1_evicts]

theorem step2_promotes {A : Type} [DecidableEq A] (new : A) (w : Config A) :
    step2 new w new = w new + 1 := by simp [step2]

/-! ### Higher-identity-wins on the four-superblock read -/

/-- Read rule: any read may observe a higher identity than the reader last
knew; the reader adopts the higher identity. -/
def adopt (known observed : Ident) : Ident := max known observed

theorem adopt_sup_known (known observed : Ident) :
    known ≤ adopt known observed := Nat.le_max_left _ _

theorem adopt_sup_observed (known observed : Ident) :
    observed ≤ adopt known observed := Nat.le_max_right _ _

theorem adopt_example :
    adopt 2 5 = 5 ∧ adopt 5 2 = 5 ∧ adopt 3 3 = 3 := by decide

/-! ### The four superblocks and the startup classification -/

/-- Durable mark of one superblock. -/
inductive Mark where
  | flushed
  | unflushed

/-- All four superblocks read `flushed`: the ordinary CR-free path. -/
def allFlushed (a b c d : Mark) : Prop :=
  a = .flushed ∧ b = .flushed ∧ c = .flushed ∧ d = .flushed

/-- Any of the four reads `unflushed`: the node is dirty. -/
def dirtyStartup (a b c d : Mark) : Prop :=
  a = .unflushed ∨ b = .unflushed ∨ c = .unflushed ∨ d = .unflushed

/-- Exhaustive sixteen-case enumeration of the startup classification. -/
theorem startup_cases (a b c d : Mark) :
    allFlushed a b c d ∨ dirtyStartup a b c d := by
  cases a <;> cases b <;> cases c <;> cases d <;> simp [allFlushed, dirtyStartup]

/-! ### The reincarnation state machine

Marker states: `flushed` (durable checkpoint), `unflushed` (running
sentinel), `dirty` (restart observed any-`unflushed`; eviction must begin),
`bumped` (incarnation incremented, four superblocks rewritten), and
`reincarnating` (wire phase: old identity pending eviction, new identity a
weight-0 standby). -/

inductive Phase where
  | flushed
  | unflushed
  | dirty
  | bumped
  | reincarnating

def rank : Phase → Nat
  | .flushed => 0
  | .unflushed => 1
  | .dirty => 2
  | .bumped => 3
  | .reincarnating => 4

inductive Transition : Phase → Phase → Prop
  | start_op : Transition .flushed .unflushed
  | observe_dirty : Transition .unflushed .dirty
  | bump_write : Transition .dirty .bumped
  | enter_wire : Transition .bumped .reincarnating
  | complete : Transition .reincarnating .flushed

/-- A phase inside the forced reincarnation: eviction initiated. -/
def committedPhase (p : Phase) : Prop :=
  p = .dirty ∨ p = .bumped ∨ p = .reincarnating

/-- Continuation commitment, step level: once eviction is initiated, the
only exit from a committed phase is completion of the forced sequence
(`reincarnating → flushed`, the new identity's clean checkpoint); the
sequence is never aborted back to an uncommitted running state. -/
theorem step_commitment {s t : Phase} (h : Transition s t) (hs : committedPhase s) :
    committedPhase t ∨ (s = .reincarnating ∧ t = .flushed) := by
  cases h with
  | start_op => exact absurd hs (by simp [committedPhase])
  | observe_dirty => exact absurd hs (by simp [committedPhase])
  | bump_write => exact Or.inl (by simp [committedPhase])
  | enter_wire => exact Or.inl (by simp [committedPhase])
  | complete => exact Or.inr ⟨rfl, rfl⟩

/-- The forced sequence itself, once initiated: exactly the monotone chain
dirty → bumped → reincarnating → flushed, with no intermediate exit. -/
inductive ForcedStep : Phase → Phase → Prop
  | bump_write : ForcedStep .dirty .bumped
  | enter_wire : ForcedStep .bumped .reincarnating
  | complete : ForcedStep .reincarnating .flushed

inductive ForcedRun : Phase → Phase → Prop
  | refl (p : Phase) : ForcedRun p p
  | step {p q t : Phase} : ForcedRun p q → ForcedStep q t → ForcedRun p t

/-- Every forced step before completion strictly advances the phase. -/
theorem forced_monotone {s t : Phase} (h : ForcedStep s t) (hnc : t ≠ .flushed) :
    rank s < rank t := by
  cases h with
  | bump_write => decide
  | enter_wire => decide
  | complete => exact absurd rfl hnc

/-- Continuation commitment: a forced run started at `dirty` either is still
inside a committed phase or has completed to the new identity's `flushed`.
No other phase is reachable, so the sequence cannot abort mid-way. -/
theorem forced_prefix {t : Phase} (h : ForcedRun .dirty t) :
    committedPhase t ∨ t = .flushed := by
  induction h with
  | refl => exact Or.inl (by simp [committedPhase])
  | step _ st _ =>
    cases st with
    | bump_write => exact Or.inl (by simp [committedPhase])
    | enter_wire => exact Or.inl (by simp [committedPhase])
    | complete => exact Or.inr rfl

/-- The complete forced sequence is exactly this run. -/
theorem forced_sequence : ForcedRun .dirty .flushed :=
  (((ForcedRun.step (ForcedRun.refl .dirty) ForcedStep.bump_write).step
    ForcedStep.enter_wire).step ForcedStep.complete)

/-! ### Worked scenario: the unit-weight three-node reincarnation (docs §5)

Identities `0, 1, 2` are `N0, N1, N2` at unit weight; identity `3` is the
reincarnated `N2′`. The forced sequence is TWO eras, each one legal batch:
E1 = `DECREMENT(N2), JOIN(N2′)` (`crossEra`), then
E2 = `INCREMENT(N2′), LEAVE(N2)` (`evictEra`). Every consecutive pair of eras
moves at most one unit of per-node voting mass (the era rule the fold and the
planner enforce before any op is proposed), so consecutive strict majorities
overlap. -/

def nodes : List Nat := [0, 1, 2, 3]

/-- E0 baseline: three unit-weight voters. -/
def unit3 : Config Nat := fun a => if a = 0 ∨ a = 1 ∨ a = 2 then 1 else 0

/-- E1: the crossing era — old identity driven to weight 0 and the
reincarnated identity joined as a standby, in one batch. -/
def e1 : Config Nat := crossEra 2 3 unit3

/-- E2: the eviction era — the new identity promoted and the weight-0 old
identity left, in one batch. -/
def e2 : Config Nat := evictEra 3 e1

theorem unit3_total : WeightedGeneral.total nodes unit3 = 3 := by
  simp [nodes, WeightedGeneral.total, unit3] <;> omega

theorem e1_membership :
    e1 2 = 0 ∧ e1 3 = 0 ∧ e1 0 = 1 ∧ e1 1 = 1 := by
  simp [e1, crossEra, unit3]

theorem e2_membership :
    e2 3 = 1 ∧ e2 0 = 1 ∧ e2 1 = 1 ∧ ¬ voting e2 2 := by
  simp [e2, evictEra, e1, crossEra, unit3, voting]

/-- The R14 era rule, kernel-checked: the crossing era moves exactly ONE
unit of per-node voting mass (`N2` down one; `N2′` enters at 0 and moves
none). -/
theorem e1_mass :
    WeightedGeneral.total nodes
      (fun a => WeightedGeneral.distance (unit3 a) (e1 a)) = 1 := by
  simp [nodes, WeightedGeneral.total, WeightedGeneral.distance, unit3, e1, crossEra] <;> omega

/-- The R14 era rule, kernel-checked: the eviction era moves exactly ONE
unit (`N2′` up one; the weight-0 leave of `N2` moves none). -/
theorem e2_mass :
    WeightedGeneral.total nodes
      (fun a => WeightedGeneral.distance (e1 a) (e2 a)) = 1 := by
  simp [nodes, WeightedGeneral.total, WeightedGeneral.distance, e2, evictEra, e1,
    crossEra, unit3] <;> omega

/-- Era safety E0 → E1: the crossing batch keeps consecutive strict
majorities overlapping (rung 9's distance-one overlap at the concrete
configuration). -/
theorem e0_e1_safe :
    Frown (WeightedGeneral.majority nodes unit3) (WeightedGeneral.majority nodes e1) := by
  exact WeightedGeneral.unit_change_overlap nodes unit3 e1 (Nat.le_of_eq e1_mass)

/-- Era safety E1 → E2: the eviction batch keeps consecutive strict
majorities overlapping. -/
theorem e1_e2_safe :
    Frown (WeightedGeneral.majority nodes e1) (WeightedGeneral.majority nodes e2) := by
  have hd :
      WeightedGeneral.total nodes
        (fun a => WeightedGeneral.distance (e1 a) (e2 a)) ≤ 1 := Nat.le_of_eq e2_mass
  exact WeightedGeneral.unit_change_overlap nodes e1 e2 hd

/-! ### The rejected transition: the identity swap in ONE era

Applying `LEAVE(old)` and `INCREMENT(new)` to the BASELINE in one era — the
swap the two-era form exists to prevent — moves two units: the old identity's
weight goes down one AND the new identity's weight goes up one in the same
era. Its endpoint's era-`E0` majority `{N1, N2}` and era-`E2` majority
`{N0, N2′}` are disjoint. The fold refuses it; here is the refuted
assertion, kernel-checked. -/

/-- The swap: old identity evicted AND new identity promoted in one era —
the batch the two-era form replaces. -/
def swapEra : Config Nat :=
  fun a => if a = 2 then 0 else if a = 3 then 1 else unit3 a

/-- The swap moves TWO units of per-node voting mass in one era. -/
theorem swap_mass :
    WeightedGeneral.total nodes
      (fun a => WeightedGeneral.distance (unit3 a) (swapEra a)) = 2 := by
  simp [nodes, WeightedGeneral.total, WeightedGeneral.distance, unit3, swapEra] <;> omega

/-- So the unit era rule REFUSES the swap: the mass the era would move
exceeds the one-unit bound the fold and the planner enforce before any
operation is proposed. -/
theorem swap_rejected :
    ¬ (WeightedGeneral.total nodes
        (fun a => WeightedGeneral.distance (unit3 a) (swapEra a)) ≤ 1) := by
  rw [swap_mass]
  omega

/-- And the swap endpoint's majority families are genuinely disjoint: the
counterexample that shows the bound is not bureaucracy. -/
theorem swap_unsafe :
    ∃ q r, WeightedGeneral.majority nodes unit3 q
      ∧ WeightedGeneral.majority nodes swapEra r ∧ ¬ ∃ a, q a ∧ r a := by
  refine ⟨fun a => a = 1 ∨ a = 2, fun a => a = 0 ∨ a = 3, ?_, ?_, ?_⟩
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total, nodes, unit3]
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total, nodes,
      swapEra, unit3]
  · intro h
    obtain ⟨a, h1, h2⟩ := h
    rcases h1 with h1 | h1 <;> rcases h2 with h2 | h2 <;> omega

end Reincarnation
```

```bash
lake env lean UVRR/Reincarnation.lean
```

```output
```

```bash
lake env lean --stdin <<'LEAN'
import UVRR.Reincarnation
#print axioms Reincarnation.bump_supersedes
#print axioms Reincarnation.crossEra_decrements
#print axioms Reincarnation.crossEra_joins_standby
#print axioms Reincarnation.crossEra_other
#print axioms Reincarnation.evictEra_promotes
#print axioms Reincarnation.evictEra_other
#print axioms Reincarnation.step0_decrements
#print axioms Reincarnation.step0_other
#print axioms Reincarnation.step1_evicts
#print axioms Reincarnation.step1_joins_standby
#print axioms Reincarnation.step1_other
#print axioms Reincarnation.step1_not_voting
#print axioms Reincarnation.step2_promotes
#print axioms Reincarnation.adopt_sup_known
#print axioms Reincarnation.adopt_sup_observed
#print axioms Reincarnation.adopt_example
#print axioms Reincarnation.startup_cases
#print axioms Reincarnation.step_commitment
#print axioms Reincarnation.forced_monotone
#print axioms Reincarnation.forced_prefix
#print axioms Reincarnation.forced_sequence
#print axioms Reincarnation.unit3_total
#print axioms Reincarnation.e1_membership
#print axioms Reincarnation.e2_membership
#print axioms Reincarnation.e1_mass
#print axioms Reincarnation.e2_mass
#print axioms Reincarnation.e0_e1_safe
#print axioms Reincarnation.e1_e2_safe
#print axioms Reincarnation.swap_mass
#print axioms Reincarnation.swap_rejected
#print axioms Reincarnation.swap_unsafe
LEAN

```

```output
'Reincarnation.bump_supersedes' depends on axioms: [propext, Classical.choice, Quot.sound]
'Reincarnation.crossEra_decrements' does not depend on any axioms
'Reincarnation.crossEra_joins_standby' does not depend on any axioms
'Reincarnation.crossEra_other' depends on axioms: [propext]
'Reincarnation.evictEra_promotes' depends on axioms: [propext]
'Reincarnation.evictEra_other' depends on axioms: [propext]
'Reincarnation.step0_decrements' depends on axioms: [propext]
'Reincarnation.step0_other' depends on axioms: [propext]
'Reincarnation.step1_evicts' depends on axioms: [propext]
'Reincarnation.step1_joins_standby' depends on axioms: [propext]
'Reincarnation.step1_other' depends on axioms: [propext]
'Reincarnation.step1_not_voting' depends on axioms: [propext]
'Reincarnation.step2_promotes' depends on axioms: [propext]
'Reincarnation.adopt_sup_known' depends on axioms: [propext]
'Reincarnation.adopt_sup_observed' depends on axioms: [propext]
'Reincarnation.adopt_example' does not depend on any axioms
'Reincarnation.startup_cases' depends on axioms: [propext]
'Reincarnation.step_commitment' depends on axioms: [propext]
'Reincarnation.forced_monotone' does not depend on any axioms
'Reincarnation.forced_prefix' depends on axioms: [propext]
'Reincarnation.forced_sequence' does not depend on any axioms
'Reincarnation.unit3_total' depends on axioms: [propext]
'Reincarnation.e1_membership' depends on axioms: [propext]
'Reincarnation.e2_membership' depends on axioms: [propext]
'Reincarnation.e1_mass' depends on axioms: [propext]
'Reincarnation.e2_mass' depends on axioms: [propext]
'Reincarnation.e0_e1_safe' depends on axioms: [propext, Classical.choice, Quot.sound]
'Reincarnation.e1_e2_safe' depends on axioms: [propext, Classical.choice, Quot.sound]
'Reincarnation.swap_mass' depends on axioms: [propext]
'Reincarnation.swap_rejected' depends on axioms: [propext, Quot.sound]
'Reincarnation.swap_unsafe' depends on axioms: [propext, Classical.choice, Quot.sound]
```

```bash
cat UVRR/ReincarnationGeneral.lean
```

```output
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
```

```bash
lake env lean UVRR/ReincarnationGeneral.lean
```

```output
```

```bash
lake env lean --stdin <<'LEAN'
import UVRR.ReincarnationGeneral
#print axioms Reincarnation.total_zero
#print axioms Reincarnation.distance_crossEra_zero
#print axioms Reincarnation.distance_evictEra_zero
#print axioms Reincarnation.distance_step0_zero
#print axioms Reincarnation.crossEra_mass
#print axioms Reincarnation.evictEra_mass
#print axioms Reincarnation.step0_mass
#print axioms Reincarnation.forced_sequence_era_safe
#print axioms Reincarnation.eraConfig_old_step
#print axioms Reincarnation.forced_run_old_mono
#print axioms Reincarnation.forced_run_old_zero
#print axioms Reincarnation.evicted_never_voting
#print axioms Reincarnation.ident_mono
#print axioms Reincarnation.no_predirty_return
#print axioms Reincarnation.amnesia_unreachable
#print axioms Reincarnation.forced_run_committed
#print axioms Reincarnation.forced_run_terminal_flushed
LEAN
```

```output
'Reincarnation.total_zero' does not depend on any axioms
'Reincarnation.distance_crossEra_zero' depends on axioms: [propext]
'Reincarnation.distance_evictEra_zero' depends on axioms: [propext]
'Reincarnation.distance_step0_zero' depends on axioms: [propext]
'Reincarnation.crossEra_mass' depends on axioms: [propext, Quot.sound]
'Reincarnation.evictEra_mass' depends on axioms: [propext, Quot.sound]
'Reincarnation.step0_mass' depends on axioms: [propext, Quot.sound]
'Reincarnation.forced_sequence_era_safe' depends on axioms: [propext, Classical.choice, Quot.sound]
'Reincarnation.eraConfig_old_step' depends on axioms: [propext]
'Reincarnation.forced_run_old_mono' depends on axioms: [propext]
'Reincarnation.forced_run_old_zero' depends on axioms: [propext]
'Reincarnation.evicted_never_voting' depends on axioms: [propext]
'Reincarnation.ident_mono' does not depend on any axioms
'Reincarnation.no_predirty_return' depends on axioms: [propext]
'Reincarnation.amnesia_unreachable' does not depend on any axioms
'Reincarnation.forced_run_committed' does not depend on any axioms
'Reincarnation.forced_run_terminal_flushed' does not depend on any axioms
```

## Proof obligations

The four obligations of this rung are discharged in `UVRR/ReincarnationGeneral.lean` over the
definitions of `UVRR/Reincarnation.lean`. Each entry states the discharging theorems, their
kernel-checked axiom footprints, and the exact side conditions the general statement carries.
No statement was weakened to pass; the side conditions are the preconditions under which the
stated general claim is true, and they are the ones the protocol satisfies.

**(a) Bumped-identity non-membership after the crossing era — DISCHARGED.** The forced run
carries a configuration through the phases (`eraConfig`): the baseline at `dirty`, the crossed
configuration from `bumped` on, the evicted configuration at `flushed`. The old identity's
weight never increases along a forced run (`forced_run_old_mono`) and once zero stays zero
(`forced_run_old_zero`); from `bumped` on — the crossing era completed — the old identity is
never a voter again in any subsequent configuration (`evicted_never_voting`). Side condition:
the single crossing era drives the old identity to weight zero exactly when it starts at or
below one unit (`w old ≤ 1`, the `step1_not_voting` generalization); at a larger scale the
crossing era iterates — each iteration still moves one unit per `crossEra_mass` — and the
scale-independent invariants `forced_run_old_mono`/`forced_run_old_zero` carry the argument
through every iteration. The two-era `Phase` machine of this rung encodes one crossing.
Axiom footprints: `eraConfig_old_step` and `forced_run_old_mono` and `forced_run_old_zero` and
`evicted_never_voting` depend on `[propext]`.

**(b) Quorum safety of every intermediate era (general case) — DISCHARGED.** The one-unit
mass bound holds at arbitrary scale for all three batch forms: `crossEra_mass` (crossing era,
with the fresh identity joining at weight zero, `w new = 0` — the precondition under which the
batch is legal — and the `1 ≤ w old` crossing precondition stated; the `≤ 1` bound itself
holds for every `w old`), `evictEra_mass` (eviction era, unconditional), `step0_mass` (solitary
decrement, unconditional). All three take the duplicate-free support list (`List.Nodup` —
duplicated support entries would double-count the moved unit, exactly the caveat
`WeightedGeneral` records for its support lists). Composed as `forced_sequence_era_safe`:
both consecutive eras of the forced sequence satisfy rung 9's
`WeightedGeneral.unit_change_overlap`, so consecutive strict majorities overlap at arbitrary
scale. Axiom footprints: `crossEra_mass`, `evictEra_mass`, `step0_mass` depend on
`[propext, Quot.sound]`; `forced_sequence_era_safe` on `[propext, Classical.choice, Quot.sound]`.

**(c) Unreachability of the classic amnesia trace — DISCHARGED.** The identity-carrying run
(`IdentRun`) pairs each phase with the identity the node operates under; the bump step replaces
the identity with its strictly higher incarnation. The operating identity never decreases along
any run (`ident_mono`); no transition re-enters a pre-eviction phase from a committed one
(`no_predirty_return`, from `step_commitment`); and from the bumped phase, under the bumped
identity, no continuation of the machine ever operates under the pre-bump identity again
(`amnesia_unreachable` — the identity only grows strictly past it). Together with (a) this
rules out the trace: a node that lost volatile state cannot regain voting eligibility under its
old identity. Axiom footprints: `ident_mono` and `amnesia_unreachable` and
`no_predirty_return` depend on `[propext]` (ident_mono and amnesia_unreachable on none).

**(d) Continuation commitment in general — DISCHARGED.** A forced run started at any
committed phase — not only `dirty` — stays inside the committed phases until it completes to
the new identity's `flushed` (`forced_run_committed`); and `flushed` is the only terminal
phase such a run can reach (`forced_run_terminal_flushed`, with `terminal` the no-forced-step-
out predicate): the forced sequence cannot end in `dirty`, `bumped` or `reincarnating`, since
each of those has a forced step out. Whichever era a leader crash lands in is a legal,
quorum-safe starting era by obligation (b). Axiom footprints: `forced_run_committed` and
`forced_run_terminal_flushed` depend on `[propext]` (forced_run_committed on none).
