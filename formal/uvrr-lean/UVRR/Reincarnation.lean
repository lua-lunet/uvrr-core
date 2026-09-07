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
learner or an evicted identity. -/
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

/-- Step 0: the exiting node's weight drops by one unit (1 → 0 in the
unit-weight scenario). -/
def step0 {A : Type} [DecidableEq A] (old : A) (w : Config A) : Config A :=
  fun a => if a = old then w a - 1 else w a

/-- Step 1: the old identity is evicted (weight clamped to 0, never a voter
again) and the new identity joins at weight 0 as a learner. -/
def step1 {A : Type} [DecidableEq A] (old new : A) (w : Config A) : Config A :=
  fun a => if a = old then 0 else if a = new then 0 else w a

/-- Step 2: the new identity is promoted by one unit (0 → 1 in the
unit-weight scenario). -/
def step2 {A : Type} [DecidableEq A] (new : A) (w : Config A) : Config A :=
  fun a => if a = new then w a + 1 else w a

theorem step0_decrements {A : Type} [DecidableEq A] (old : A) (w : Config A) :
    step0 old w old = w old - 1 := by simp [step0]

theorem step0_other {A : Type} [DecidableEq A] {old a : A} (h : a ≠ old) (w : Config A) :
    step0 old w a = w a := by simp [step0, h]

theorem step1_evicts {A : Type} [DecidableEq A] (old new : A) (w : Config A) :
    step1 old new w old = 0 := by simp [step1]

theorem step1_joins_learner {A : Type} [DecidableEq A] (old new : A) (w : Config A) :
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

States per the spec: `flushed` (durable checkpoint), `unflushed` (running
sentinel), `dirty` (restart observed any-`unflushed`; eviction must begin),
`bumped` (incarnation incremented, four superblocks rewritten), and
`reincarnating` (wire phase: old identity pending eviction, new identity a
weight-0 learner). -/

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

/-! ### Worked scenario: the unit-weight three-node eviction (docs §5)

Identities `0, 1, 2` are `N0, N1, N2` at unit weight; identity `3` is the
reincarnated `N2′`. The forced sequence is D0 → D1 → D2 → D3. -/

def nodes : List Nat := [0, 1, 2, 3]

/-- D0 baseline: three unit-weight voters. -/
def unit3 : Config Nat := fun a => if a = 0 ∨ a = 1 ∨ a = 2 then 1 else 0

/-- D1: exiting node weight 1 → 0 (subtract one). -/
def d1 : Config Nat := step0 2 unit3

/-- D2: old identity evicted, new node joins at weight 0 (total unchanged). -/
def d2 : Config Nat := step1 2 3 d1

/-- D3: new node 0 → 1 (add one). -/
def d3 : Config Nat := step2 3 d2

theorem unit3_total : WeightedGeneral.total nodes unit3 = 3 := by
  simp [nodes, WeightedGeneral.total, unit3] <;> omega

theorem d3_total : WeightedGeneral.total nodes d3 = 3 := by
  simp [nodes, WeightedGeneral.total, d3, step2, d2, step1, d1, step0, unit3] <;> omega

theorem d2_membership :
    d2 2 = 0 ∧ d2 3 = 0 ∧ d2 0 = 1 ∧ d2 1 = 1 := by
  simp [d2, step1, d1, step0, unit3]

/-- D1 → D2 leaves every weight unchanged: the eviction of the already
weight-0 old identity and the learner join preserve the weight map
pointwise, so the two eras are the same majority family. -/
theorem d1_d2_equal : ∀ a, d2 a = d1 a := by
  intro a
  by_cases h2 : a = 2
  · subst a; simp [d2, step1, d1, step0, unit3]
  · by_cases h3 : a = 3
    · subst a; simp [d2, step1, d1, step0, unit3]
    · simp [d2, step1, d1, step0, unit3, h2, h3]

/-- Era safety D0 → D1: the single unit decrement keeps consecutive strict
majorities overlapping (rung 9's distance-one overlap, at the concrete
configuration). -/
theorem d0_d1_safe :
    Frown (WeightedGeneral.majority nodes unit3) (WeightedGeneral.majority nodes d1) := by
  have hd :
      WeightedGeneral.total nodes
        (fun a => WeightedGeneral.distance (unit3 a) (d1 a)) = 1 := by
    simp [nodes, WeightedGeneral.total, WeightedGeneral.distance, unit3, d1, step0] <;> omega
  exact WeightedGeneral.unit_change_overlap nodes unit3 d1 (Nat.le_of_eq hd)

/-- Era safety D1 → D2: the eras coincide pointwise. -/
theorem d1_d2_safe :
    Frown (WeightedGeneral.majority nodes d1) (WeightedGeneral.majority nodes d2) := by
  have he : d2 = d1 := funext d1_d2_equal
  rw [he]; exact WeightedGeneral.self_overlap nodes d1

/-- Era safety D2 → D3: the one-unit promotion keeps consecutive strict
majorities overlapping. -/
theorem d2_d3_safe :
    Frown (WeightedGeneral.majority nodes d2) (WeightedGeneral.majority nodes d3) := by
  have hd :
      WeightedGeneral.total nodes
        (fun a => WeightedGeneral.distance (d2 a) (d3 a)) = 1 := by
    simp [nodes, WeightedGeneral.total, WeightedGeneral.distance, d3, step2, d2, step1,
      d1, step0, unit3] <;> omega
  exact WeightedGeneral.unit_change_overlap nodes d2 d3 (Nat.le_of_eq hd)

end Reincarnation
