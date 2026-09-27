import UVRR.LexBallot
import UVRR.ViewFence

/-! The view-jump safety lemma: a view (ballot round) may jump by more than
one during a reconfiguration, and agreement is unaffected.

During a crash storm a leader change may bump the view repeatedly before a
quorum is re-formed; nothing in the safety contract requires the views of
commits to advance uniformly. The `Rule` structure names the four disciplines
of a legal jump from ballot `b` to ballot `b'`: strict increase, freshness of
the target, era adjacency for the non-stop path, and the entitlement to
propose only over an accepted history that contains the establishing
reconfiguration. The safety theorem discharges agreement at the jumped-to
ballot from the history invariants P1–P7 via Theorem 10; no hypothesis of
Theorem 10 quantifies over intermediate ballots, so successor-by-one is
unrepresentable as a safety requirement. The only risk of a jump is
exhaustion of the view space, bounded code-side by the CAS delta.

Negative controls: an era skip of two violates era adjacency, and P1 supplies
no two-era cross frown (concrete majority-family witness); a jump without
strict increase is refused by `bLt_irrefl`, and a non-increasing view-change
target is unrepresentable in the view-fence step relation. -/
namespace ViewJump

universe u v w

/-- The view-jump rule: the four disciplines of a legal jump from ballot `b`
to ballot `b'` over an era-indexed Paxos history. -/
structure Rule {A : Type u} {B : Type v} {V : Type w} (P : Paxos A B V) (b b' : B) : Prop where
  /-- The jump strictly increases the ballot. -/
  strictIncrease : P.lt b b'
  /-- Freshness of the target: no promise of either shape and no acceptance at
  `b'` exists in the history at jump time. -/
  noReuse : (∀ i a, ¬ P.promised i a b') ∧ (∀ i a b'', ¬ P.promisedV i a b' b'') ∧
    (∀ i a, ¬ P.accepted i a b')
  /-- Era adjacency for the non-stop path: views may jump by more than one;
  eras may not. -/
  eraAdjacency : P.era b' ≤ P.era b + 1
  /-- Entitlement: the jump is proposed only over an accepted history that
  contains the establishing reconfiguration — a chosen (committed) slot whose
  ballot lies below `b'`. -/
  entitlement : ∃ i bc, P.chosen i bc ∧ P.lt bc b'

/-- The view-jump safety lemma. Over concrete lex ballots, given the history
invariants P1–P7 and a `Rule` for the jump from `b` to `b'`, a choice at the
jumped-to ballot `b'` agrees with every other choice at the instance. The
proof discharges agreement via Theorem 10; the rule's disciplines name the
legal jump but are not hypotheses of the agreement argument, which never
quantifies over intermediate ballots — successor-by-one is unrepresentable as
a safety requirement. -/
theorem view_jump_safe {A : Type} {V : Type} (P : Paxos A Ballot V)
    (hlt : P.lt = bLt) (hera : P.era = Ballot.era)
    (hI : P.Inv) (hne : ∀ e q, P.QII e q → ∃ a, q a)
    {i : Nat} {b b' : Ballot} (_rule : Rule P b b')
    (hjump : P.chosen i b') {c : Ballot} (hother : P.chosen i c) :
    P.v i b' = P.v i c :=
  Paxos.theorem10_lex P hlt hera hI hne hjump hother

/-- Uniformity: the identical conclusion follows for a successor jump and for
a gap jump from the same rule instance. Both conjuncts discard the gap-shape
hypothesis — the safety argument never inspects the size of the jump. -/
theorem successor_not_required {A : Type} {V : Type} (P : Paxos A Ballot V)
    (hlt : P.lt = bLt) (hera : P.era = Ballot.era)
    (hI : P.Inv) (hne : ∀ e q, P.QII e q → ∃ a, q a)
    {i : Nat} {b b' : Ballot} (r : Rule P b b')
    (hjump : P.chosen i b') {c : Ballot} (hother : P.chosen i c) :
    (b' = (b.1, b.2 + 1) → P.v i b' = P.v i c) ∧
    (b.2 + 2 ≤ b'.2 → P.v i b' = P.v i c) :=
  ⟨fun _ => view_jump_safe P hlt hera hI hne r hjump hother,
   fun _ => view_jump_safe P hlt hera hI hne r hjump hother⟩

/-- Era skip refused, rule side: a jump of two eras violates era adjacency, so
no rule spans it. -/
theorem era_skip_not_rule {A : Type u} {B : Type v} {V : Type w} (P : Paxos A B V)
    {b b' : B} (h : P.era b' = P.era b + 2) : ¬ Rule P b b' := by
  intro r
  have hadj := r.eraAdjacency
  omega

/-- Era skip refused, quorum side (rung-5 idiom). Five nodes separate era e
from era e+2: the era-e configuration is {0,1,2} and the era-(e+2)
configuration is {2,3,4}; each family is the strict majorities of its
configuration. P1 constrains adjacent eras only. -/
def eraEMajorities : QSys (Fin 5) := fun q => (q 0 ∧ q 1) ∨ (q 0 ∧ q 2) ∨ (q 1 ∧ q 2)

/-- The strict majorities of the era-(e+2) configuration {2,3,4}. -/
def eraE2Majorities : QSys (Fin 5) := fun q => (q 2 ∧ q 3) ∨ (q 2 ∧ q 4) ∨ (q 3 ∧ q 4)

/-- The two-era cross frown fails: the era-e majority {0,1} and the era-(e+2)
majority {3,4} are disjoint, so P1 supplies no `QI_e ⌢ QII_{e+2}` overlap. -/
theorem two_era_cross_frown_fails : ¬ Frown eraEMajorities eraE2Majorities := by
  intro h
  obtain ⟨a, ha1, ha2⟩ := h (fun a => a = 0 ∨ a = 1) (fun a => a = 3 ∨ a = 4)
    (Or.inl ⟨Or.inl rfl, Or.inr rfl⟩) (Or.inr (Or.inr ⟨Or.inl rfl, Or.inr rfl⟩))
  rcases ha1 with rfl | rfl <;> rcases ha2 with h' | h' <;> exact absurd h' (by decide)

/-- Reuse refused, ballot side: a "jump" without strict increase is not a
jump — on lex ballots the identity jump contradicts `bLt_irrefl`. -/
theorem self_jump_not_rule {A : Type} {V : Type} (P : Paxos A Ballot V)
    (hlt : P.lt = bLt) {b : Ballot} : ¬ Rule P b b := by
  intro r
  have h1 : P.lt b b := r.strictIncrease
  rw [hlt] at h1
  exact bLt_irrefl b h1

/-- Reuse refused, fence side: the `ViewFence.Step.enter` guard demands
`s.floor < target`, so a non-increasing target is unrepresentable as a step
hypothesis. -/
theorem enter_guard_refuses {V : Type} (s : ViewFence.State V) (target : Nat)
    (h : target ≤ s.floor) : ¬ s.floor < target :=
  Nat.not_lt_of_le h

/-- The concrete refusal, kernel-checked: after entering view 2, a further
"entry" of view 2 fails the guard. -/
theorem enter_guard_refuses_concrete :
    ¬ (ViewFence.enter (ViewFence.initial (V := Bool)) 2).floor < 2 := by
  decide

end ViewJump
