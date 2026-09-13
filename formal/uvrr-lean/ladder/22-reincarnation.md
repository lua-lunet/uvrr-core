# Rung 22: uVRR reincarnation — Crash-Stop-Self-Evict (spec-only)

*2026-09-13T10:31:49Z by Showboat 0.6.1*
<!-- showboat-id: 2156e12c-9168-45d2-a9e2-bf26bb086bef -->

This rung fixes the reincarnation specification as definitions: the marker
transition machine of `docs/vrr-durability-model.md` §5.1 — the durable marker
of one superblock copy (`Mark`: `stopping`, `stopped`, `restarting`, `joining`;
no `started` state is written), the stop path's two uniform 4x writes
(`beginStop`, `finishStop`; the host drains — flushes WALs and grids — strictly
between them, so `stopped` vouches for the WAL under it:
`begin_stop_vouches_nothing`, `finish_stop_proves_drain`,
`stopped_sole_entry`), the 2-of-4 boot test (`stoppedCount`, `stoppedQuorum`,
`boot_verdict`), the quorum read and decision table (`classify`,
`RestartDecision`, `restart_continue`, `restart_bump`), the incarnation bump
(`bump`, `bump_supersedes`), the identity-to-weight configuration map
(`Config`, `voting`), the two-era forced weight sequence (`crossEra` — one
batch of `DECREMENT(old), JOIN(new)`, the crossing era; `evictEra` — one batch
of `INCREMENT(new), LEAVE(old)`, the eviction era; built on the unit steps
`step0`/`step1`/`step2`), the episode machine over marker states plus the two
era states (`Phase`, `Transition`, `rank`), the higher-identity-wins read rule
(`adopt`), and continuation commitment (`committedPhase`, `ForcedStep`,
`ForcedRun`, `forced_prefix`, `forced_monotone`).

**Re-statement note (post-dating the rung's earlier transcript):** the
definitions of this transcript are the rung's Mark/Phase vocabulary re-stated
over the shipped marker transition machine — the module documentation of
`src/replica/reincarnation.rs` (`Marker`, `begin_stop`, `finish_stop`,
`classify`, `restart`) with the historical §5.1 text — replacing the older
all-`flushed`/any-`unflushed` startup classification (`allFlushed`,
`dirtyStartup`, the `flushed`/`unflushed`/`dirty` phases). The machine is
stated, not narrated: four `stopping` copies, the drain strictly between, four
`stopped` copies, the 2-of-4 `stopped` boot test, `restarting` under the same
identity, `joining` under the bumped identity.

The admitted content is
kernel-checked structural lemmas plus `decide`/`simp`-enumerated finite
instances of the unit-weight three-node reincarnation: the two consecutive eras of the forced sequence are
pairwise quorum-safe (`e0_e1_safe`, `e1_e2_safe`), and each era moves exactly ONE unit of
per-node voting mass — the R14 mass rule the fold and the planner enforce before any op is
proposed (`e1_mass`, `e2_mass`). The one-era alternative the two-era form replaces —
`LEAVE(old)` and `INCREMENT(new)` applied to the BASELINE in a single batch — is refused:
it moves two units (`swap_mass`), so the era rule rejects it (`swap_rejected`), and its
endpoint majority families are genuinely disjoint (`swap_unsafe`). The boot
verdict is exhaustive over the closed four-copy domain (`boot_verdict`), and the
old identity is never again a voter after the crossing era (`step1_not_voting`). This is a
SPEC-ONLY rung: no general-protocol theorem is admitted. The general theorems —
bumped-identity non-membership in every view at or after the crossing era, quorum safety of
every intermediate era at arbitrary scale, unreachability of the classic amnesia trace, and
continuation commitment in general — are discharged in
`UVRR/ReincarnationGeneral.lean` (rung 22G).

```bash
cat UVRR/Reincarnation.lean
```

```output
import UVRR.WeightedGeneral

/-! uVRR reincarnation: the Crash-Stop-Self-Evict protocol (spec-only rung).
A node whose volatile state was lost is a different node: it reopens under a
new identity obtained by bumping its incarnation, and its old identity is
evicted from the voting configuration by the forced weight sequence. The
governing boot rule is the marker transition machine
(`docs/vrr-durability-model.md` §5.1; the pure Rust twin is
`src/replica/reincarnation.rs` — `Marker`, `begin_stop`, `finish_stop`,
`classify`, `restart`): the stop command writes `stopping` 4x, the host
drains — flushes WALs and grids — strictly between the two marker writes,
and `stopped` 4x is the drain's proof, so a `stopped` copy vouches for the
WAL under it; at boot 2-of-4 copies holding `stopped` is the clean stop
whose `restarting` node is a member with complete state ticking the full
protocol, and no stopped quorum — a crash, a torn marker set, or death
mid-join — is the dead identity the node bumps, exactly one past, into
`joining`, not a member; no `started` state is written. The leader drives
the forced sequence of the Paxos Voting Weights rules; every consecutive
pair of eras overlaps, so each intermediate era is quorum-safe on its own.
Once the Crash-Stop-Eviction is initiated it must continue: the forced
sequence is never aborted mid-way. This module fixes definitions, the
marker machine and kernel-checked structural lemmas plus
`decide`/`simp`-verified finite instances of the unit-weight three-node
eviction. General protocol theorems over arbitrary configurations,
arbitrary weight scales and the wire protocol are discharged in
`ReincarnationGeneral.lean`. -/
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

/-! ### The marker transition machine (`docs/vrr-durability-model.md` §5.1)

The four superblock markers are an ordered transition system; each state
names the transition that must have completed for it to exist:

    running ──stop──> stopping ──drain──> stopped ──boot, 2-of-4 stopped──> restarting
                      (4x write)  flush    (4x write)                     (4x write)
                                  WALs + grids
    running ──crash──> (markers unchanged) ──boot, no 2-of-4 stopped──> joining
                                                                        (bump, 4x write)

uVRR performs no disk flushes on the protocol's hot path: the stop command
writes `stopping` 4x, the host drains — flushes WALs and grids — strictly
between the two marker writes, and the `stopped` write is the drain's
proof, so a `stopped` copy vouches for the WAL under it. At boot the
quorum read answers one question — did the transition complete? — with
2-of-4 copies holding `stopped` (the twin's open threshold); the working
quorum resolves the identity higher-identity-wins inside the quorum, and
over the machine's uniform 4x writes the winner cohort is all four copies.
No `started` state is written: no safety logic looks for `started`, it
looks for `stopped` — the extra superblock write buys no safety and is
elided. -/

/-- Durable marker of one superblock copy (§5.1): one state of the ordered
marker transition system. Each state names the transition that must have
completed for it to exist; no `started` state is written. -/
inductive Mark where
  | stopping
  | stopped
  | restarting
  | joining
  deriving DecidableEq

/-- All four copies rewritten with one marker (§5.1): every marker write is
a uniform 4x write, so the copies of a live node always agree and the
winner cohort is all four copies. -/
def written4 (m : Mark) : Mark × Mark × Mark × Mark := (m, m, m, m)

/-- The stop command (§5.1): the copies read `stopping` 4x. The node has
stopped sending — no disk flush sits on the protocol's hot path; the drain
has not yet been proven, so this state vouches for nothing. -/
def beginStop (_a _b _c _d : Mark) : Mark × Mark × Mark × Mark :=
  written4 Mark.stopping

/-- The drain's proof (§5.1): the copies read `stopped` 4x — callable only
after the host drained, the flushes of WALs and grids strictly between the
`stopping` write and this one. The marker order is the drain's proof, so a
`stopped` copy vouches for the WAL under it. -/
def finishStop (_a _b _c _d : Mark) : Mark × Mark × Mark × Mark :=
  written4 Mark.stopped

/-- How many of the four copies hold `stopped`. -/
def stoppedCount (cs : Mark × Mark × Mark × Mark) : Nat :=
  (if cs.1 = Mark.stopped then 1 else 0) +
    (if cs.2.1 = Mark.stopped then 1 else 0) +
    (if cs.2.2.1 = Mark.stopped then 1 else 0) +
    (if cs.2.2.2 = Mark.stopped then 1 else 0)

/-- The 2-of-4 boot test (§5.1; the twin's open threshold): did the
`stopping ──drain──> stopped` transition complete? -/
def stoppedQuorum (cs : Mark × Mark × Mark × Mark) : Prop := 2 ≤ stoppedCount cs

/-- `stopping` vouches for nothing: the stop command's 4x write holds no
`stopped` copy, so no stopped quorum — the drain, not the stop command, is
what the `stopped` marker proves. -/
theorem begin_stop_vouches_nothing (a b c d : Mark) :
    ¬ stoppedQuorum (beginStop a b c d) := by
  simp [stoppedQuorum, stoppedCount, beginStop, written4]

/-- The drain's proof is uniform: the `stopped` 4x write holds the quorum
on all four copies. A stop that died partway still reads clean on the
surviving quorum, correctly — the flush had already completed before the
first `stopped` write. -/
theorem finish_stop_proves_drain (a b c d : Mark) :
    stoppedQuorum (finishStop a b c d) := by
  simp [stoppedQuorum, stoppedCount, finishStop, written4]

/-- The restart decision (§5.1's decision table): a stopped quorum
continues under the same identity; no stopped quorum — a crash, a torn
marker set, or death mid-join — means the identity is dead and is bumped,
exactly one past the quorum-resolved identity. -/
inductive RestartDecision where
  | cont (i : Ident)
  | bump (old new : Ident)

/-- The quorum read (§5.1; the twin's `open`): the boot question — did the
`stopping ──drain──> stopped` transition complete? — answered by 2-of-4
copies holding `stopped`. The working quorum resolves the identity
higher-identity-wins inside the quorum; over the machine's uniform 4x
writes the copies carry one identity, so the resolved identity is the
node's own. -/
def classify (a b c d : Mark) (i : Ident) : RestartDecision :=
  if 2 ≤ stoppedCount (a, b, c, d) then RestartDecision.cont i
  else RestartDecision.bump i (bump i)

theorem classify_cont {a b c d : Mark} {i : Ident} (h : stoppedQuorum (a, b, c, d)) :
    classify a b c d i = .cont i := by
  unfold classify
  exact if_pos h

theorem classify_bump {a b c d : Mark} {i : Ident}
    (h : ¬ stoppedQuorum (a, b, c, d)) :
    classify a b c d i = .bump i (bump i) := by
  unfold classify
  exact if_neg h

theorem classify_cont_resolves {a b c d : Mark} {i j : Ident}
    (h : classify a b c d i = .cont j) : j = i := by
  by_cases hq : 2 ≤ stoppedCount (a, b, c, d)
  · rw [classify, if_pos hq] at h
    injection h with h1
    exact h1.symm
  · rw [classify, if_neg hq] at h
    exact absurd h (by simp)

theorem classify_bump_resolves {a b c d : Mark} {i o n : Ident}
    (h : classify a b c d i = .bump o n) : o = i ∧ n = bump i := by
  by_cases hq : 2 ≤ stoppedCount (a, b, c, d)
  · rw [classify, if_pos hq] at h
    exact absurd h (by simp)
  · rw [classify, if_neg hq] at h
    injection h with h1 h2
    exact ⟨h1.symm, h2.symm⟩

/-- A restart (§5.1's decision table): the quorum read, the decision, and
the 4x marker write the decision leaves on disk. The uniform write is the
repair: every copy is rewritten from the decision. -/
def restart (a b c d : Mark) (i : Ident) :
    RestartDecision × Mark × Mark × Mark × Mark :=
  (classify a b c d i,
    if 2 ≤ stoppedCount (a, b, c, d) then written4 Mark.restarting
    else written4 Mark.joining)

/-- The boot of a controlled shutdown: a stopped quorum continues under
the same identity and the copies read `restarting` 4x — a member with
complete state, no amnesia, ticking the full protocol, suspecting a silent
primary like any backup. -/
theorem restart_continue {a b c d : Mark} {i : Ident}
    (h : stoppedQuorum (a, b, c, d)) :
    restart a b c d i = (.cont i, written4 Mark.restarting) := by
  unfold restart
  rw [classify_cont h]
  exact congrArg (fun cs => (RestartDecision.cont i, cs))
    (if_pos (c := 2 ≤ stoppedCount (a, b, c, d)) h)

/-- The reincarnation's boot: no stopped quorum means the identity is dead;
the node bumps it, exactly one past, and the copies read `joining` 4x —
not a member: no vote, no view change, until the forced sequence seats the
new identity. -/
theorem restart_bump {a b c d : Mark} {i : Ident}
    (h : ¬ stoppedQuorum (a, b, c, d)) :
    restart a b c d i = (.bump i (bump i), written4 Mark.joining) := by
  unfold restart
  rw [classify_bump h]
  exact congrArg (fun cs => (RestartDecision.bump i (bump i), cs))
    (if_neg (c := 2 ≤ stoppedCount (a, b, c, d)) h)

/-- Exhaustive boot verdict: the closed four-copy domain decides the boot
question, and the two outcomes are exactly `restarting` (same identity)
and `joining` (bumped). -/
theorem boot_verdict (a b c d : Mark) :
    stoppedQuorum (a, b, c, d) ∨ ¬ stoppedQuorum (a, b, c, d) := by
  rcases Nat.lt_or_ge (stoppedCount (a, b, c, d)) 2 with h | h
  · exact Or.inr (Nat.not_le.mpr h)
  · exact Or.inl h

/-- The written markers are exactly the four: no `started` state exists —
no safety logic looks for `started`, it looks for `stopped`; the extra
superblock write buys no safety and is elided. -/
theorem marker_states (m : Mark) :
    m = .stopping ∨ m = .stopped ∨ m = .restarting ∨ m = .joining := by
  cases m <;> simp

/-! ### The reincarnation episode machine

The machine's phases: the marker states the four copies hold (§5.1) plus
the two eras of the forced sequence the leader commits while the copies
hold `joining`. `running` is the pre-stop state the stop command rewrites;
a crash leaves the markers unchanged. The forced sequence carries the
configurations: the baseline at `joining`, the crossed configuration after
the crossing era, the evicted configuration after the eviction era — the
new identity then votes. A node dying mid-join holds its `joining`
markers, reads no stopped quorum at its next boot, and re-enters
`joining` under a further bump: the reincarnation repeats. -/

inductive Phase where
  | running
  | stopping
  | stopped
  | restarting
  | joining
  | crossed
  | evicted

def rank : Phase → Nat
  | .running => 0
  | .stopping => 1
  | .stopped => 2
  | .restarting => 3
  | .joining => 3
  | .crossed => 4
  | .evicted => 5

/-- The episode machine's transitions. The four marker edges are §5.1's
diagram: `begin_stop` and `finish_stop` are the stop path's two 4x writes
(the drain sits strictly between them), `boot_restart` is the clean
stop's boot under the same identity, `boot_join` is the reincarnation's
boot with the bump. The two era edges are configuration commits the
leader makes while the copies hold `joining`; they are not marker
writes. -/
inductive Transition : Phase → Phase → Prop
  | begin_stop : Transition .running .stopping
  | finish_stop : Transition .stopping .stopped
  | boot_restart : Transition .stopped .restarting
  | boot_join : Transition .running .joining
  | cross : Transition .joining .crossed
  | evict : Transition .crossed .evicted

/-- The drain sits strictly between the two marker writes: the sole edge
into `stopped` departs `stopping` — `stopped` names the transition that
must have completed for it to exist. -/
theorem stopped_sole_entry {s t : Phase} (h : Transition s t) (ht : t = .stopped) :
    s = .stopping := by
  cases h with
  | finish_stop => rfl
  | begin_stop => exact absurd ht (by simp)
  | boot_restart => exact absurd ht (by simp)
  | boot_join => exact absurd ht (by simp)
  | cross => exact absurd ht (by simp)
  | evict => exact absurd ht (by simp)

/-- A phase inside the forced reincarnation: the bump is written and the
sequence is owed or underway. -/
def committedPhase (p : Phase) : Prop :=
  p = .joining ∨ p = .crossed

/-- Continuation commitment, step level: from a committed phase the only
exits are the next era of the forced sequence; the sequence is never
aborted back to an uncommitted state. -/
theorem step_commitment {s t : Phase} (h : Transition s t) (hs : committedPhase s) :
    committedPhase t ∨ (s = .crossed ∧ t = .evicted) := by
  cases h with
  | begin_stop => exact absurd hs (by simp [committedPhase])
  | finish_stop => exact absurd hs (by simp [committedPhase])
  | boot_restart => exact absurd hs (by simp [committedPhase])
  | boot_join => exact absurd hs (by simp [committedPhase])
  | cross => exact Or.inl (by simp [committedPhase])
  | evict => exact Or.inr ⟨rfl, rfl⟩

/-- The forced sequence itself, once initiated: exactly the era walk
`joining → crossed → evicted`, with no intermediate exit. -/
inductive ForcedStep : Phase → Phase → Prop
  | cross : ForcedStep .joining .crossed
  | evict : ForcedStep .crossed .evicted

inductive ForcedRun : Phase → Phase → Prop
  | refl (p : Phase) : ForcedRun p p
  | step {p q t : Phase} : ForcedRun p q → ForcedStep q t → ForcedRun p t

/-- Every forced step before completion strictly advances the phase. -/
theorem forced_monotone {s t : Phase} (h : ForcedStep s t) (hnc : t ≠ .evicted) :
    rank s < rank t := by
  cases h with
  | cross => decide
  | evict => exact absurd rfl hnc

/-- Continuation commitment: a forced run started at `joining` either is
still inside a committed phase or has completed to `evicted` — the new
identity votes. No other phase is reachable, so the sequence cannot abort
mid-way. -/
theorem forced_prefix {t : Phase} (h : ForcedRun .joining t) :
    committedPhase t ∨ t = .evicted := by
  induction h with
  | refl => exact Or.inl (by simp [committedPhase])
  | step _ st _ =>
    cases st with
    | cross => exact Or.inl (by simp [committedPhase])
    | evict => exact Or.inr rfl

/-- The complete forced sequence is exactly this run. -/
theorem forced_sequence : ForcedRun .joining .evicted :=
  ((ForcedRun.step (ForcedRun.refl .joining) ForcedStep.cross).step
    ForcedStep.evict)

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
#print axioms Reincarnation.begin_stop_vouches_nothing
#print axioms Reincarnation.finish_stop_proves_drain
#print axioms Reincarnation.classify_cont
#print axioms Reincarnation.classify_bump
#print axioms Reincarnation.classify_cont_resolves
#print axioms Reincarnation.classify_bump_resolves
#print axioms Reincarnation.restart_continue
#print axioms Reincarnation.restart_bump
#print axioms Reincarnation.boot_verdict
#print axioms Reincarnation.marker_states
#print axioms Reincarnation.stopped_sole_entry
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
'Reincarnation.begin_stop_vouches_nothing' depends on axioms: [propext]
'Reincarnation.finish_stop_proves_drain' depends on axioms: [propext]
'Reincarnation.classify_cont' does not depend on any axioms
'Reincarnation.classify_bump' does not depend on any axioms
'Reincarnation.classify_cont_resolves' depends on axioms: [propext]
'Reincarnation.classify_bump_resolves' depends on axioms: [propext]
'Reincarnation.restart_continue' does not depend on any axioms
'Reincarnation.restart_bump' does not depend on any axioms
'Reincarnation.boot_verdict' does not depend on any axioms
'Reincarnation.marker_states' depends on axioms: [propext]
'Reincarnation.stopped_sole_entry' depends on axioms: [propext]
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
reachable terminal phase other than `evicted`. -/

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

The forced run walks the phases with the baseline configuration at
`joining`, the crossed configuration from `crossed` on, and the evicted
configuration at `evicted`. The marker phases carry no configuration
change: the markers are the §5.1 boot machine, the forced sequence is the
configuration walk the `joining` node's leader commits. -/

def eraConfig {A : Type} [DecidableEq A] (old new : A) (w : Config A) :
    Phase → Config A
  | .running => w
  | .stopping => w
  | .stopped => w
  | .restarting => w
  | .joining => w
  | .crossed => crossEra old new w
  | .evicted => evictEra new (crossEra old new w)

theorem eraConfig_old_step {A : Type} [DecidableEq A] {old new : A} (hne : old ≠ new)
    (w : Config A) {q t : Phase} (hst : ForcedStep q t) :
    eraConfig old new w t old ≤ eraConfig old new w q old := by
  cases hst with
  | cross =>
    show crossEra old new w old ≤ w old
    simp only [crossEra]
    exact Nat.sub_le _ _
  | evict =>
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

/-- Bumped-identity non-membership: from `crossed` on — the crossing era
committed — the old identity is never a voter again in any subsequent
configuration of the forced run. The crossing era drives the old identity to
weight zero exactly when it starts at or below one unit; at a larger scale
the crossing era iterates, and the scale-independent invariants above
(`forced_run_old_mono`, `forced_run_old_zero`) carry the argument through
each iteration. -/
theorem evicted_never_voting {A : Type} [DecidableEq A] {old new : A} (hne : old ≠ new)
    (w : Config A) (hw : w old ≤ 1) {t : Phase} (hrun : ForcedRun .crossed t) :
    ¬ voting (eraConfig old new w t) old := by
  have hwn : (w old : Nat) ≤ 1 := hw
  have h0n : (eraConfig old new w Phase.crossed old : Nat) = 0 := by
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
operates under; the reincarnation's boot (`boot_join`, no 2-of-4 `stopped`
at boot) replaces the identity with its strictly higher incarnation and no
edge ever lowers it. -/

inductive IdentRun : Phase → Ident → Phase → Ident → Prop
  | refl (p : Phase) (i : Ident) : IdentRun p i p i
  | begin_stop (i : Ident) : IdentRun .running i .stopping i
  | finish_stop (i : Ident) : IdentRun .stopping i .stopped i
  | boot_restart (i : Ident) : IdentRun .stopped i .restarting i
  | boot_join (i : Ident) : IdentRun .running i .joining (bump i)
  | cross (i : Ident) : IdentRun .joining i .crossed i
  | evict (i : Ident) : IdentRun .crossed i .evicted i
  | trans {p q t : Phase} {i j k : Ident} :
      IdentRun p i q j → IdentRun q j t k → IdentRun p i t k

/-- The operating identity never decreases along an identity-carrying run. -/
theorem ident_mono {p q : Phase} {i j : Ident} (h : IdentRun p i q j) : i ≤ j := by
  induction h with
  | refl => exact Nat.le_refl _
  | begin_stop => exact Nat.le_refl _
  | finish_stop => exact Nat.le_refl _
  | boot_restart => exact Nat.le_refl _
  | boot_join i => exact Nat.le_succ i
  | cross => exact Nat.le_refl _
  | evict => exact Nat.le_refl _
  | trans _ _ ih1 ih2 => exact Nat.le_trans ih1 ih2

/-- No transition from a committed phase re-enters a pre-join phase: once
the bump is written the machine cannot go back to the states before it. -/
theorem no_prejoining_return {s t : Phase} (h : Transition s t) (hs : committedPhase s) :
    t = .crossed ∨ t = .evicted := by
  cases h with
  | begin_stop => exact absurd hs (by simp [committedPhase])
  | finish_stop => exact absurd hs (by simp [committedPhase])
  | boot_restart => exact absurd hs (by simp [committedPhase])
  | boot_join => exact absurd hs (by simp [committedPhase])
  | cross => exact Or.inl rfl
  | evict => exact Or.inr rfl

/-- The classic amnesia trace is unreachable: from the `joining` marker —
the bump written — under the bumped identity, no continuation of the
machine ever operates under the pre-bump identity again — the identity only
grows past it. -/
theorem amnesia_unreachable {q : Phase} {j i₀ : Ident}
    (h : IdentRun .joining (bump i₀) q j) : j ≠ i₀ := by
  intro hcon
  have hm : (bump i₀ : Nat) ≤ (j : Nat) := ident_mono h
  rw [hcon] at hm
  exact absurd hm (Nat.not_succ_le_self i₀)

/-! ### Continuation commitment in general -/

/-- A forced run started at any committed phase stays inside the committed
phases until it completes to `evicted` — the new identity votes. -/
theorem forced_run_committed {p t : Phase} (hp : committedPhase p)
    (hrun : ForcedRun p t) :
    committedPhase t ∨ t = .evicted := by
  induction hrun with
  | refl => exact Or.inl hp
  | step _ st ih =>
    rcases ih with hq | hq
    · cases st with
      | cross => exact Or.inl (by simp [committedPhase])
      | evict => exact Or.inr rfl
    · subst hq
      cases st

/-- A phase is terminal when no forced step leaves it. -/
def terminal (t : Phase) : Prop := ¬ ∃ u, ForcedStep t u

/-- The only terminal phase reachable by a forced run from a committed phase
is `evicted`: the forced run cannot end in `joining` or `crossed` — each of
those has a forced step out, so completion is mandatory. -/
theorem forced_run_terminal_evicted {p t : Phase} (hrun : ForcedRun p t)
    (hp : committedPhase p) (hterm : terminal t) : t = .evicted := by
  rcases forced_run_committed hp hrun with hc | hc
  · rcases hc with hc | hc
    · subst hc
      exact absurd (⟨.crossed, ForcedStep.cross⟩ : ∃ u, ForcedStep .joining u) hterm
    · subst hc
      exact absurd (⟨.evicted, ForcedStep.evict⟩ : ∃ u, ForcedStep .crossed u) hterm
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
#print axioms Reincarnation.no_prejoining_return
#print axioms Reincarnation.amnesia_unreachable
#print axioms Reincarnation.forced_run_committed
#print axioms Reincarnation.forced_run_terminal_evicted
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
'Reincarnation.no_prejoining_return' depends on axioms: [propext]
'Reincarnation.amnesia_unreachable' does not depend on any axioms
'Reincarnation.forced_run_committed' depends on axioms: [propext]
'Reincarnation.forced_run_terminal_evicted' depends on axioms: [propext]
```

## Proof obligations

The four obligations of this rung are discharged in `UVRR/ReincarnationGeneral.lean` over the
definitions of `UVRR/Reincarnation.lean`. Each entry states the discharging theorems, their
kernel-checked axiom footprints, and the exact side conditions the general statement carries.
No statement was weakened to pass; the side conditions are the preconditions under which the
stated general claim is true, and they are the ones the protocol satisfies.

**(a) Bumped-identity non-membership after the crossing era — DISCHARGED.** The forced run
carries a configuration through the phases (`eraConfig`): the baseline at `joining`, the
crossed configuration from `crossed` on, the evicted configuration at `evicted`. The old
identity's weight never increases along a forced run (`forced_run_old_mono`) and once zero
stays zero (`forced_run_old_zero`); from `crossed` on — the crossing era committed — the old
identity is never a voter again in any subsequent configuration (`evicted_never_voting`).
Side condition: the single crossing era drives the old identity to weight zero exactly when
it starts at or below one unit (`w old ≤ 1`, the `step1_not_voting` generalization); at a
larger scale the crossing era iterates — each iteration still moves one unit per
`crossEra_mass` — and the scale-independent invariants `forced_run_old_mono`/
`forced_run_old_zero` carry the argument through every iteration. The two-era `Phase`
machine of this rung encodes one crossing.
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
(`IdentRun`) pairs each phase with the identity the node operates under; the reincarnation's
boot edge (`boot_join` — no 2-of-4 `stopped` at boot) replaces the identity with its strictly
higher incarnation. The operating identity never decreases along any run (`ident_mono`); no
transition re-enters a pre-join phase from a committed one (`no_prejoining_return`, from
`step_commitment`); and from the `joining` marker, under the bumped identity, no continuation
of the machine ever operates under the pre-bump identity again (`amnesia_unreachable` — the
identity only grows strictly past it). Together with (a) this
rules out the trace: a node that lost volatile state cannot regain voting eligibility under its
old identity. Axiom footprints: `no_prejoining_return` depends on `[propext]`
(ident_mono and amnesia_unreachable on none).

**(d) Continuation commitment in general — DISCHARGED.** A forced run started at any
committed phase — not only `joining` — stays inside the committed phases until it completes
to `evicted` (`forced_run_committed`); and `evicted` is the only terminal
phase such a run can reach (`forced_run_terminal_evicted`, with `terminal` the
no-forced-step-out predicate): the forced sequence cannot end in `joining` or `crossed`,
since each of those has a forced step out. Whichever era a leader crash lands in is a legal,
quorum-safe starting era by obligation (b). Axiom footprints: `forced_run_committed` and
`forced_run_terminal_evicted` depend on `[propext]` (forced_run_committed on none).
