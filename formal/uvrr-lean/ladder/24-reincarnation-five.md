# Rung 24: the five-node Crash-Stop-Reincarnation rung

*2026-09-08T15:55:23Z by Showboat 0.6.1*
<!-- showboat-id: 3797d695-ceea-4b7b-a399-019399417b1c -->

This rung lifts the reincarnation scenario from rung 22's three voters to a
FIVE-voter unit-scale cluster `{n0..n4}`, with identity `0` the crashed victim
returning as identity `5`. The eras are E0 (five unit voters), E1 =
`DECREMENT(0), JOIN(5)` (`c1`, the rung 22 crossing batch) and E2 =
`INCREMENT(5), LEAVE(0)` (`c2`, the eviction batch). The majority families are
computed, not axiomatized: E0's strict majorities are 3-of-5 subsets
(`unit5_pair_not_majority` refuses a pair), and with the victim dead every
formable E0 majority contains at least three of the four survivors
(`e0_dead_majority_survivors`); E1's voting weight is 4, all of it carried by
survivors, so the same three-of-four shape holds for every E1 majority
(`e1_majority_survivors`). The forming quorum `{n1, n2, n3}` is a majority in
E0, E1 and E2 alike (`forming_majorities`). Era safety is rung 22G's general
discharge instantiated — `five_forced_safe` is
`Reincarnation.forced_sequence_era_safe` at the five-node support, and
`five_evicted_never_voting` is `Reincarnation.evicted_never_voting` — so both
boundaries overlap by rung 9's one-unit rule and nothing is hand-proved. The
casting-vote verdict on five nodes with one member dead is the honest
generalization of rung 23's degenerate pattern, and it SPLITS by boundary:
across E0/E1 every formable quorum pair meets in at least two survivors, so no
singleton pivot exists and none is needed (`forming_no_casting_vote`); across
E1/E2 the replacement joins the voter set, the pair {n1,n2,n3} and {n3,n4,n5}
meet in `n3` ALONE, and the pivot genuinely exists (`e1_e2_pivot`, rung 6's
shape at the concrete leader). The scale-iteration boundary case is the
doubled victim: at weight 2 one crossing era is still a legal one-unit batch
but leaves the victim voting (`doubled_crossEra_leaves_voter`), which is why
larger scales iterate the crossing era; the iterated two-unit instance is the
rung's recorded obligation. Proofs were drafted by Leanstral
(`labs-leanstral-1-5`) in the compiler-feedback loop and kernel-checked
locally; the axiom footprint is the standard one.

```bash
cat UVRR/ReincarnationFive.lean
```

```output
import UVRR.ReincarnationGeneral
import UVRR.CastingVote

/-! uVRR reincarnation at five-node unit scale: the crashed voter's
leader-driven two-era return as a new identity, on the cluster `{n0..n4}`.
Identity `0` is the victim; it returns as identity `5`. The eras are E0 (five
unit voters), E1 = `DECREMENT(0), JOIN(5)` and E2 = `INCREMENT(5), LEAVE(0)`,
the rung 22 batch forms. Era safety is rung 22G's general mass bounds
instantiated at the five-node support — rung 9's one-unit overlap does the
work, nothing here is hand-proved. With the victim dead the forming quorum is
`{n1, n2, n3}`-shaped; the casting-vote analysis is the honest five-node
verdict: the E0/E1 boundary is degenerate (every formable quorum pair meets in
at least two survivors, so no pivot exists or is needed), while the E1/E2
boundary genuinely admits a pivot at `n3` where the new identity's side joins
the voter set. A double-weight victim is the scale-iteration boundary case:
one crossing era still leaves it voting, which is why larger scales iterate
the crossing era; the iterated two-unit instance is a recorded obligation. -/

namespace ReincarnationFive

/-- The finite support: identities `0`..`4` vote in the old era; identity `5`
is the reincarnated node. -/
def nodes5 : List Nat := [0, 1, 2, 3, 4, 5]

/-- E0 baseline: five unit-weight voters. -/
def unit5 : Reincarnation.Config Nat :=
  fun a => if a = 0 ∨ a = 1 ∨ a = 2 ∨ a = 3 ∨ a = 4 then 1 else 0

/-- E1: the crossing era `DECREMENT(0), JOIN(5)` — the victim driven to
weight 0, the replacement joined as a weight-0 standby. -/
def c1 : Reincarnation.Config Nat := Reincarnation.crossEra 0 5 unit5

/-- E2: the eviction era `INCREMENT(5), LEAVE(0)`. -/
def c2 : Reincarnation.Config Nat := Reincarnation.evictEra 5 c1

/-- The forming quorum with the victim dead: three of the four survivors. -/
def forming : NSet Nat := fun a => a = 1 ∨ a = 2 ∨ a = 3

/-- The new identity's side of the E1/E2 boundary: the pivot quorum through
`n3` that includes the promoted replacement. -/
def pivotNew : NSet Nat := fun a => a = 3 ∨ a = 4 ∨ a = 5

/-- The doubled victim: weight 2 before the crossing era, the scale at which
one crossing batch no longer evicts. -/
def unit5d : Reincarnation.Config Nat := fun a => if a = 0 then 2 else unit5 a

theorem nodes5_nodup : nodes5.Nodup := by
  unfold nodes5; decide

theorem unit5_total : WeightedGeneral.total nodes5 unit5 = 5 := by
  simp [nodes5, WeightedGeneral.total, unit5] <;> omega

theorem c1_membership :
    c1 0 = 0 ∧ c1 5 = 0 ∧ c1 1 = 1 ∧ c1 2 = 1 ∧ c1 3 = 1 ∧ c1 4 = 1 := by
  unfold c1 Reincarnation.crossEra unit5
  simp <;> omega

theorem c2_membership :
    c2 5 = 1 ∧ c2 1 = 1 ∧ c2 2 = 1 ∧ c2 3 = 1 ∧ c2 4 = 1 ∧ c2 0 = 0 ∧
    ¬ Reincarnation.voting c2 0 := by
  unfold c2 Reincarnation.evictEra c1 Reincarnation.crossEra unit5 Reincarnation.voting
  simp <;> omega

/-- Strict majorities in E0 are 3-of-5 subsets: a pair is not enough. -/
theorem unit5_pair_not_majority :
    ¬ WeightedGeneral.majority nodes5 unit5 (fun a => a = 1 ∨ a = 2) := by
  simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
    nodes5, unit5] <;> omega

/-- With the victim dead, every formable E0 majority contains at least three
of the four survivors. -/
theorem e0_dead_majority_survivors {q : NSet Nat}
    (hm : WeightedGeneral.majority nodes5 unit5 q) (hd : ¬ q 0) :
    (q 1 ∧ q 2 ∧ q 3) ∨ (q 1 ∧ q 2 ∧ q 4) ∨ (q 1 ∧ q 3 ∧ q 4) ∨ (q 2 ∧ q 3 ∧ q 4) := by
  by_cases h1 : q 1 <;> by_cases h2 : q 2 <;> by_cases h3 : q 3 <;> by_cases h4 : q 4 <;>
    simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, unit5, hd, h1, h2, h3, h4] at hm ⊢ <;> omega

/-- Every E1 majority contains at least three of the four survivors: the
crossing era's voting weight is 4, all carried by survivors. -/
theorem e1_majority_survivors {q : NSet Nat}
    (hm : WeightedGeneral.majority nodes5 c1 q) :
    (q 1 ∧ q 2 ∧ q 3) ∨ (q 1 ∧ q 2 ∧ q 4) ∨ (q 1 ∧ q 3 ∧ q 4) ∨ (q 2 ∧ q 3 ∧ q 4) := by
  by_cases h1 : q 1 <;> by_cases h2 : q 2 <;> by_cases h3 : q 3 <;> by_cases h4 : q 4 <;>
    simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, Reincarnation.crossEra, unit5, c1, h1, h2, h3, h4] at hm ⊢ <;> omega

/-- The forming quorum `{n1, n2, n3}` is a majority in E0, in E1 and in E2
alike: the same set forms throughout the forced sequence. -/
theorem forming_majorities :
    WeightedGeneral.majority nodes5 unit5 forming ∧
    WeightedGeneral.majority nodes5 c1 forming ∧
    WeightedGeneral.majority nodes5 c2 forming := by
  refine ⟨?_, ?_, ?_⟩
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, unit5, forming] <;> omega
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, c1, Reincarnation.crossEra, unit5, forming] <;> omega
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, c2, Reincarnation.evictEra, c1, Reincarnation.crossEra, unit5,
      forming] <;> omega

/-- Era safety of the whole forced sequence at the five-node support: rung
22G's general era-safety theorem instantiated at `unit5` — both boundaries by
rung 9's one-unit overlap, nothing hand-proved. -/
theorem five_forced_safe :
    Frown (WeightedGeneral.majority nodes5 unit5)
        (WeightedGeneral.majority nodes5 c1) ∧
    Frown (WeightedGeneral.majority nodes5 c1)
        (WeightedGeneral.majority nodes5 c2) := by
  exact Reincarnation.forced_sequence_era_safe (by decide) nodes5 unit5
    (by simp [unit5]) (by simp [unit5]) (by decide)

/-- Rung 22G's non-membership invariant at the five-node support: from the
bumped phase on, the victim's old identity never votes again in any later
configuration of the forced run. -/
theorem five_evicted_never_voting {t : Reincarnation.Phase}
    (hrun : Reincarnation.ForcedRun .bumped t) :
    ¬ Reincarnation.voting (Reincarnation.eraConfig 0 5 unit5 t) 0 := by
  apply Reincarnation.evicted_never_voting (by decide) unit5 (by simp [unit5]) hrun

/-- The honest five-node negative: with the victim dead, the forming quorum
and ANY E1 majority meet in at least two survivors, so no single node is a
pivot — no casting vote exists across the E0/E1 boundary, and none is needed
(the overlap is `five_forced_safe`). -/
theorem forming_no_casting_vote (ℓ : Nat) {q' : NSet Nat}
    (hm : WeightedGeneral.majority nodes5 c1 q') :
    ¬ CastingVote.HasCastingVote forming q' ℓ := by
  intro h
  rcases e1_majority_survivors hm with (⟨h1, h2, h3⟩ | ⟨h1, h2, h4⟩ | ⟨h1, h3, h4⟩ | ⟨h2, h3, h4⟩)
  · have h1ℓ : 1 = ℓ := ((h 1).mp ⟨by simp [forming], h1⟩)
    have h2ℓ : 2 = ℓ := ((h 2).mp ⟨by simp [forming], h2⟩)
    omega
  · have h1ℓ : 1 = ℓ := ((h 1).mp ⟨by simp [forming], h1⟩)
    have h2ℓ : 2 = ℓ := ((h 2).mp ⟨by simp [forming], h2⟩)
    omega
  · have h1ℓ : 1 = ℓ := ((h 1).mp ⟨by simp [forming], h1⟩)
    have h3ℓ : 3 = ℓ := ((h 3).mp ⟨by simp [forming], h3⟩)
    omega
  · have h2ℓ : 2 = ℓ := ((h 2).mp ⟨by simp [forming], h2⟩)
    have h3ℓ : 3 = ℓ := ((h 3).mp ⟨by simp [forming], h3⟩)
    omega

/-- The E1/E2 boundary genuinely admits a pivot: the forming quorum and the
new identity's side meet in `n3` alone, so rung 6's casting-vote shape is
exercised where the replacement joins the voter set. -/
theorem e1_e2_pivot :
    WeightedGeneral.majority nodes5 c1 forming ∧
    WeightedGeneral.majority nodes5 c2 pivotNew ∧
    CastingVote.HasCastingVote forming pivotNew 3 := by
  refine ⟨?_, ?_, ?_⟩
  · exact forming_majorities.2.1
  · simp [WeightedGeneral.majority, WeightedGeneral.mass, WeightedGeneral.total,
      nodes5, c2, Reincarnation.evictEra, c1, Reincarnation.crossEra, unit5,
      pivotNew] <;> omega
  · intro a
    constructor
    · intro h
      rcases h with ⟨hfa, hpa⟩
      rcases hfa with (hfa | hfa | hfa)
      · rcases hpa with (hpa | hpa | hpa)
        · omega
        · omega
        · omega
      · rcases hpa with (hpa | hpa | hpa)
        · omega
        · omega
        · omega
      · rcases hpa with (hpa | hpa | hpa)
        · omega
        · omega
        · omega
    · intro h
      subst h
      constructor
      · simp [forming]
      · simp [pivotNew]

/-- The scale-iteration boundary case: a double-weight victim survives one
crossing era — the batch is legal (one unit of mass) but the victim still
votes, so eviction at larger scales requires iterating the crossing era. -/
theorem doubled_crossEra_leaves_voter :
    Reincarnation.voting (Reincarnation.crossEra 0 5 unit5d) 0 ∧
    WeightedGeneral.total nodes5
      (fun a => WeightedGeneral.distance (unit5d a)
        (Reincarnation.crossEra 0 5 unit5d a)) ≤ 1 := by
  have h0 : (0 : Nat) ≠ 5 := by decide
  have hhold : 1 ≤ unit5d 0 := by
    unfold unit5d
    simp
  have hnew : unit5d 5 = 0 := by
    unfold unit5d unit5
    simp
  have hnodup : nodes5.Nodup := by decide
  have h := Reincarnation.crossEra_mass h0 nodes5 unit5d hhold hnew hnodup
  refine ⟨?_, h⟩
  unfold Reincarnation.voting Reincarnation.crossEra unit5d unit5
  simp

end ReincarnationFive
```

```bash
lake env lean UVRR/ReincarnationFive.lean
```

```output
```

```bash
lake env lean --stdin <<'LEAN'
import UVRR.ReincarnationFive
#print axioms ReincarnationFive.nodes5_nodup
#print axioms ReincarnationFive.unit5_total
#print axioms ReincarnationFive.c1_membership
#print axioms ReincarnationFive.c2_membership
#print axioms ReincarnationFive.unit5_pair_not_majority
#print axioms ReincarnationFive.e0_dead_majority_survivors
#print axioms ReincarnationFive.e1_majority_survivors
#print axioms ReincarnationFive.forming_majorities
#print axioms ReincarnationFive.five_forced_safe
#print axioms ReincarnationFive.five_evicted_never_voting
#print axioms ReincarnationFive.forming_no_casting_vote
#print axioms ReincarnationFive.e1_e2_pivot
#print axioms ReincarnationFive.doubled_crossEra_leaves_voter
LEAN
```

```output
'ReincarnationFive.nodes5_nodup' does not depend on any axioms
'ReincarnationFive.unit5_total' depends on axioms: [propext]
'ReincarnationFive.c1_membership' depends on axioms: [propext]
'ReincarnationFive.c2_membership' depends on axioms: [propext]
'ReincarnationFive.unit5_pair_not_majority' depends on axioms: [propext, Classical.choice, Quot.sound]
'ReincarnationFive.e0_dead_majority_survivors' depends on axioms: [propext, Classical.choice, Quot.sound]
'ReincarnationFive.e1_majority_survivors' depends on axioms: [propext, Classical.choice, Quot.sound]
'ReincarnationFive.forming_majorities' depends on axioms: [propext, Classical.choice, Quot.sound]
'ReincarnationFive.five_forced_safe' depends on axioms: [propext, Classical.choice, Quot.sound]
'ReincarnationFive.five_evicted_never_voting' depends on axioms: [propext, Quot.sound]
'ReincarnationFive.forming_no_casting_vote' depends on axioms: [propext, Classical.choice, Quot.sound]
'ReincarnationFive.e1_e2_pivot' depends on axioms: [propext, Classical.choice, Quot.sound]
'ReincarnationFive.doubled_crossEra_leaves_voter' depends on axioms: [propext, Quot.sound]
```

## Proof obligations

The finite instances here are kernel-checked; the following are future rungs'
targets, stated with the definitions they would use. None is claimed here.

**(a) The iterated two-unit crossing (the doubled victim).** For the doubled
five-node baseline (`unit5d`, victim at weight 2) the forced sequence must
iterate the crossing era twice — `crossEra 0 5` applied, then the remaining
unit driven to zero in a second legal batch — before the eviction era applies;
the composition is era-safe boundary by boundary and the victim is
non-voting from the first crossing on. `doubled_crossEra_leaves_voter` is the
checked boundary fact that motivates the iteration; the iterated instance
itself is not proved here. Definitions used: `unit5d`, `Reincarnation.crossEra`,
`Reincarnation.crossEra_mass`, `Reincarnation.forced_sequence_era_safe`,
`Reincarnation.evicted_never_voting`.

**(b) The pivot dichotomy at five nodes in general.** For an arbitrary
unit-weight five-voter configuration and arbitrary victim/replacement pair,
classify each boundary of the forced sequence: either every formable quorum
pair meets in at least two nodes (the E0/E1 shape checked in
`forming_no_casting_vote`) or there exists a quorum pair meeting in a singleton
and the pivot is exactly the node where the new identity's side joins the
voter set (the E1/E2 shape checked in `e1_e2_pivot`). The checked instances
are the concrete configurations; the general statement over arbitrary
supports is not admitted here. Definitions used: `forming`, `pivotNew`,
`CastingVote.HasCastingVote`, `e0_dead_majority_survivors`,
`e1_majority_survivors`, `WeightedGeneral.majority`.

**(c) Casting-vote agreement on the wire at five nodes.** The pivot of
`e1_e2_pivot` only exhibits rung 6's quorum shape; a general agreement theorem
must connect `noninterference`, `guard_preserved` and `completes_phase1` to
rung 3/4 agreement over the reincarnation eras with the wire protocol modeled
at five nodes. Not admitted here.
