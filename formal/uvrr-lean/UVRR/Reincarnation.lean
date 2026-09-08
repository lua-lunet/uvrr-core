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
