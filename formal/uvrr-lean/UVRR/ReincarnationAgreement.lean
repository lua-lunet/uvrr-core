import UVRR.ReincarnationFive

/-! Agreement across the reincarnation sequence, under every view schedule.

The two forced eras of the resurrection — E0 the cluster before the crash,
E1 the crossing batch `DECREMENT(old), JOIN(new)`, E2 the eviction batch
`INCREMENT(new), LEAVE(old)` — are made the configuration sequence of rung 4's
era-indexed agreement theorem: in every era the phase-I and phase-II quorum
families are the weighted strict majority of that era's configuration, held at
E2 once the sequence has completed. P1 (`QII_e ⌢ QI_e ⌢ QII_{e+1}` for every
`e`) is discharged by rung 9's self-overlap within an era and by rung 22G's
`forced_sequence_era_safe` across the two boundaries. Theorem 10 then gives
per-instance agreement for EVERY history that satisfies the protocol invariants
P2–P7 — every view change, within an era or across the boundaries, the
leader-overlap schedule of rung 6 included: the casting vote chooses who is
sent `prepare` and when the leader promises, and relaxes none of P1–P7.

The general form is stated at arbitrary scale for any node type; the
five-node instance (rung 24's `unit5`, victim `0` reborn as `5`) follows from
it, together with the leader-overlap pivot at the E1/E2 boundary and the
majority boundary: three survivors form the same quorum in every era, two
form none. What can happen during the two rounds is a view change that takes
the majority away; what cannot happen is disagreement. Liveness (that a
casting vote or a majority exists, that messages arrive) is not claimed. -/

namespace ReincarnationAgreement

/-- The configuration in force at era `e` of the forced sequence: `w` before
it, the crossing era at `1`, the eviction era from `2` on. -/
def sequenceConfig {A : Type} [DecidableEq A] (old new : A)
    (w : Reincarnation.Config A) : Nat → Reincarnation.Config A
  | 0 => w
  | 1 => Reincarnation.crossEra old new w
  | _ + 2 => Reincarnation.evictEra new (Reincarnation.crossEra old new w)

/-- The quorum family of era `e`: the weighted strict majority of its
configuration over the finite support `nodes`. It serves as both `QI_e` and
`QII_e`. -/
def family {A : Type} [DecidableEq A] (nodes : List A) (old new : A)
    (w : Reincarnation.Config A) : Nat → QSys A :=
  fun e => WeightedGeneral.majority nodes (sequenceConfig old new w e)

theorem mass_eq_zero_of_none {A : Type} (nodes : List A) (w : A → Nat)
    (q : NSet A) (h : ∀ a, ¬ q a) : WeightedGeneral.mass nodes w q = 0 := by
  classical
  unfold WeightedGeneral.mass
  induction nodes with
  | nil => rfl
  | cons a rest ih =>
    rw [WeightedGeneral.total, if_neg (h a), ih]

/-- A strict majority is nonempty: the phase-II nonemptiness Theorem 10
requires. -/
theorem majority_nonempty {A : Type} (nodes : List A) (w : A → Nat)
    {q : NSet A} (h : WeightedGeneral.majority nodes w q) : ∃ a, q a := by
  apply Classical.byContradiction
  intro hn
  have hz : WeightedGeneral.mass nodes w q = 0 :=
    mass_eq_zero_of_none nodes w q (fun a ha => hn ⟨a, ha⟩)
  unfold WeightedGeneral.majority at h
  omega

/-- P1 for the forced sequence at arbitrary scale: within every era the
family meets itself (rung 9), and across the two boundaries consecutive
families meet by the one-unit mass rule (rung 22G); from E2 on the
configuration is constant. -/
theorem sequence_p1 {A : Type} [DecidableEq A] {old new : A} (hne : old ≠ new)
    (nodes : List A) (w : Reincarnation.Config A) (hold : 1 ≤ w old)
    (hnew : w new = 0) (hnodup : nodes.Nodup) :
    ∀ e, Frown (family nodes old new w e) (family nodes old new w e) ∧
      Frown (family nodes old new w e) (family nodes old new w (e + 1)) := by
  intro e
  have hsafe := Reincarnation.forced_sequence_era_safe hne nodes w hold hnew hnodup
  refine ⟨WeightedGeneral.self_overlap nodes _, ?_⟩
  match e with
  | 0 => exact hsafe.1
  | 1 => exact hsafe.2
  | _ + 2 => exact WeightedGeneral.self_overlap nodes _

/-- Per-instance agreement across the forced sequence: for every era-indexed
history whose quorum families are the sequence's majorities and which
satisfies the protocol invariants P2–P7, any two chosen ballots of one
instance carry the same value — under every view schedule. -/
theorem sequence_agreement {A B V : Type} [DecidableEq A] {old new : A}
    (hne : old ≠ new) (nodes : List A) (w : Reincarnation.Config A)
    (hold : 1 ≤ w old) (hnew : w new = 0) (hnodup : nodes.Nodup)
    (P : Paxos A B V)
    (hQI : P.QI = family nodes old new w) (hQII : P.QII = family nodes old new w)
    (hmono : P.EraMono) (heraLe : P.EraLe) (hp23 : P.P23) (hp4 : P.P4)
    (hp5 : P.P5) (hp6 : P.P6) (hp7 : P.P7)
    (ho : WellFounded P.lt ∧ (∀ a b c, P.lt a b → P.lt b c → P.lt a c) ∧
          (∀ a, ¬ P.lt a a) ∧ (∀ a b, P.lt a b ∨ a = b ∨ P.lt b a))
    {i : Nat} {b1 b2 : B} (h1 : P.chosen i b1) (h2 : P.chosen i b2) :
    P.v i b1 = P.v i b2 := by
  have hp1 : P.P1 := by
    intro e
    rw [hQI, hQII]
    exact sequence_p1 hne nodes w hold hnew hnodup e
  have hne' : ∀ e q, P.QII e q → ∃ a, q a := by
    intro e q hq
    rw [hQII] at hq
    exact majority_nonempty _ _ hq
  exact Paxos.theorem10 ⟨hp1, hmono, heraLe, hp23, hp4, hp5, hp6, hp7⟩ ho hne' h1 h2

/-! ### The five-node instance -/

/-- The five-node sequence's families: rung 24's `unit5`, victim `0`, reborn
identity `5`. -/
def family5 : Nat → QSys Nat :=
  family ReincarnationFive.nodes5 0 5 ReincarnationFive.unit5

theorem family5_eras :
    family5 0 = WeightedGeneral.majority ReincarnationFive.nodes5 ReincarnationFive.unit5 ∧
    family5 1 = WeightedGeneral.majority ReincarnationFive.nodes5 ReincarnationFive.c1 ∧
    family5 2 = WeightedGeneral.majority ReincarnationFive.nodes5 ReincarnationFive.c2 :=
  ⟨rfl, rfl, rfl⟩

/-- P1 at five nodes. -/
theorem five_p1 :
    ∀ e, Frown (family5 e) (family5 e) ∧ Frown (family5 e) (family5 (e + 1)) :=
  sequence_p1 (by decide) ReincarnationFive.nodes5 ReincarnationFive.unit5
    (by simp [ReincarnationFive.unit5]) (by simp [ReincarnationFive.unit5]) (by decide)

/-- Agreement at five nodes: the two-round evict/join of the reincarnated
identity is agreement-safe under every view schedule. -/
theorem five_agreement {B V : Type} (P : Paxos Nat B V)
    (hQI : P.QI = family5) (hQII : P.QII = family5)
    (hmono : P.EraMono) (heraLe : P.EraLe) (hp23 : P.P23) (hp4 : P.P4)
    (hp5 : P.P5) (hp6 : P.P6) (hp7 : P.P7)
    (ho : WellFounded P.lt ∧ (∀ a b c, P.lt a b → P.lt b c → P.lt a c) ∧
          (∀ a, ¬ P.lt a a) ∧ (∀ a b, P.lt a b ∨ a = b ∨ P.lt b a))
    {i : Nat} {b1 b2 : B} (h1 : P.chosen i b1) (h2 : P.chosen i b2) :
    P.v i b1 = P.v i b2 :=
  sequence_agreement (by decide) ReincarnationFive.nodes5 ReincarnationFive.unit5
    (by simp [ReincarnationFive.unit5]) (by simp [ReincarnationFive.unit5]) (by decide)
    P hQI hQII hmono heraLe hp23 hp4 hp5 hp6 hp7 ho h1 h2

/-- The leader-overlap round at five nodes: the E1/E2 boundary admits the
casting vote at `n3` (rung 24's pivot) and the boundary frowns — the
non-stop schedule of rung 6 is available and covered by `five_agreement`. -/
theorem five_leader_overlap :
    CastingVote.HasCastingVote ReincarnationFive.forming ReincarnationFive.pivotNew 3 ∧
    Frown (family5 1) (family5 2) :=
  ⟨ReincarnationFive.e1_e2_pivot.2.2, (five_p1 1).2⟩

/-- The majority boundary: with the victim dead, three survivors form one
quorum that is a majority in every era of the sequence, and two survivors are
a majority in none — a view change that leaves two is the stall, not a
disagreement. -/
theorem five_majority_boundary :
    (family5 0 ReincarnationFive.forming ∧ family5 1 ReincarnationFive.forming ∧
      family5 2 ReincarnationFive.forming) ∧
    ¬ family5 0 (fun a => a = 1 ∨ a = 2) :=
  ⟨ReincarnationFive.forming_majorities, ReincarnationFive.unit5_pair_not_majority⟩

end ReincarnationAgreement
