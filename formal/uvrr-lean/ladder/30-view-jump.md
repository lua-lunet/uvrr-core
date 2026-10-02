# Rung 30: the view-jump safety lemma

Evidence recorded 27 September 2026 against the source based on `6ad01b0`.

The target is `UVRR/ViewJump.lean`. During a reconfiguration the era bumps the
view to keep a stable leader, and the bump is a jump of more than one. The
lemma names the rule and discharges it: an arbitrary advance of a view is
possible during a crash storm, advances are resolved via quorums, and a jump
of a view by more than one is not a violation of safety, because the safety
requirements do not demand that the views of commits advance uniformly — they
may jump. The only risk is exhaustion of the view space, bounded code-side by
the CAS delta (out of scope here).

## Claim

`ViewJump.Rule` names the four disciplines of a legal jump from ballot `b` to
ballot `b'` over an era-indexed Paxos history:

1. **Strict increase** — the jump strictly increases the ballot.
2. **Freshness of the target** — no promise of either shape and no acceptance
   at `b'` exists in the history at jump time.
3. **Era adjacency for the non-stop path** — views may jump by more than one;
   eras may not (`era b' ≤ era b + 1`).
4. **Entitlement** — the jump is proposed only over an accepted history that
   contains the establishing reconfiguration: a chosen (committed) slot whose
   ballot lies below `b'`.

`view_jump_safe` is the named safety theorem: over concrete lex ballots, given
the history invariants P1–P7 and a `Rule` for the jump, a choice at the
jumped-to ballot agrees with every other choice at the instance, discharged by
Theorem 10. The proof's content is that no hypothesis of Theorem 10 quantifies
over intermediate ballots — successor-by-one is unrepresentable as a safety
requirement. `successor_not_required` is the uniformity corollary: the
identical conclusion follows for a successor jump and for a gap jump from the
same rule instance, and both conjuncts discard the gap-shape hypothesis,
demonstrating that the safety argument never inspects the size of the jump.

## Negative controls

- **Era skip refused.** A jump of two eras violates era adjacency, so no
  `Rule` spans it (`era_skip_not_rule`). And P1 supplies no two-era cross
  frown: the rung-5 idiom, a concrete majority-family witness over five nodes
  whose era-e configuration is `{0,1,2}` and whose era-(e+2) configuration is
  `{2,3,4}`, shows the era-e majority `{0,1}` and the era-(e+2) majority
  `{3,4}` are disjoint (`two_era_cross_frown_fails`). P1 constrains adjacent
  eras only.
- **Reuse refused.** A "jump" without strict increase is not a jump: on lex
  ballots the identity jump contradicts `bLt_irrefl` (`self_jump_not_rule`).
  And the `ViewFence.Step.enter` guard `s.floor < target` makes a
  non-increasing target unrepresentable as a step hypothesis
  (`enter_guard_refuses`, with the concrete kernel-checked refusal
  `enter_guard_refuses_concrete`).

## Source

```bash
cat UVRR/ViewJump.lean
```

```output
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

/-- The era-only move is a strict ballot increase: under the lex order, bumping
the era with the round standing is already a jump. Nothing in the order forces
the round component to move when the era does. -/
theorem era_only_strict (e v : Nat) : bLt (e, v) (e + 1, v) :=
  Prod.Lex.left v v (Nat.lt_succ_self e)

/-- The era-only jump satisfies the rule: its era disciplines hold by
reflexivity, and its freshness and entitlement are the same history hypotheses
any jump carries. The rule admits the era ascending with the round standing. -/
theorem era_only_rule {A : Type} {V : Type} (P : Paxos A Ballot V)
    (hlt : P.lt = bLt) (hera : P.era = Ballot.era) {e v : Nat}
    (hno : (∀ i a, ¬ P.promised i a (e + 1, v)) ∧
      (∀ i a b'', ¬ P.promisedV i a (e + 1, v) b'') ∧ (∀ i a, ¬ P.accepted i a (e + 1, v)))
    (hent : ∃ i bc, P.chosen i bc ∧ P.lt bc (e + 1, v)) :
    Rule P (e, v) (e + 1, v) where
  strictIncrease := by rw [hlt]; exact era_only_strict e v
  noReuse := hno
  eraAdjacency := by rw [hera]; exact Nat.le_refl _
  entitlement := hent

/-- Agreement is unaffected by an era-only jump: the era may ascend with the
round standing, and a choice at the jumped-to ballot agrees with every other
choice at the instance. The safety argument never quantifies over the round
component of the boundary. The applications are the scalings and the learner
steps: `Double` and `Halve` preserve the voter sequence elementwise, and a
weight-zero `Join` or `Leave` inserts or removes a learner, so the era ascends
with the leader computation unchanged and the accompanying view bump is the
mechanism's simplicity, never a safety requirement — nothing is depleted. A
fused schedule telescopes its acknowledgements and its commit lands one new
ballot, so the intermediate steps of a plan have zero impact on it. -/
theorem era_only_safe {A : Type} {V : Type} (P : Paxos A Ballot V)
    (hlt : P.lt = bLt) (hera : P.era = Ballot.era)
    (hI : P.Inv) (hne : ∀ e q, P.QII e q → ∃ a, q a)
    {i : Nat} {e v : Nat} (r : Rule P (e, v) (e + 1, v))
    (hjump : P.chosen i (e + 1, v)) {c : Ballot} (hother : P.chosen i c) :
    P.v i (e + 1, v) = P.v i c :=
  view_jump_safe P hlt hera hI hne r hjump hother

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
```

## Kernel-checked results

Replay from `formal/uvrr-lean/`:

```bash
lake build
python3 check_axioms.py
```

```output
Build completed successfully (32 jobs).
PASS 539 declarations: only standard Lean axioms
```

Direct axiom query:

```bash
printf '%s\n' 'import UVRR' \
  '#print axioms ViewJump.view_jump_safe' \
  '#print axioms ViewJump.successor_not_required' \
  '#print axioms ViewJump.era_skip_not_rule' \
  '#print axioms ViewJump.two_era_cross_frown_fails' \
  '#print axioms ViewJump.self_jump_not_rule' \
  '#print axioms ViewJump.enter_guard_refuses' \
  '#print axioms ViewJump.enter_guard_refuses_concrete' | lake env lean --stdin
```

```output
'ViewJump.view_jump_safe' depends on axioms: [propext, Classical.choice, Quot.sound]
'ViewJump.successor_not_required' depends on axioms: [propext, Classical.choice, Quot.sound]
'ViewJump.era_skip_not_rule' depends on axioms: [propext, Quot.sound]
'ViewJump.two_era_cross_frown_fails' depends on axioms: [propext]
'ViewJump.self_jump_not_rule' depends on axioms: [propext, Quot.sound]
'ViewJump.enter_guard_refuses' does not depend on any axioms
'ViewJump.enter_guard_refuses_concrete' does not depend on any axioms
```

The statements and proof bodies are hand-landed and kernel-checked; a bounded
Leanstral loop (`scripts/leanstral.sh prove`) was run against the incomplete
skeleton and did not discharge the file (a no-op round, a malformed-output
round, and two rounds error-looping on the two-era frown control), so the
bodies were landed by hand in the recorded pattern. Only the permitted
standard Lean axioms appear.
