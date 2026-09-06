import UVRR

namespace ExistingAudit

-- P1 alone does not imply nonempty phase-II quorums: an empty phase-I
-- family makes both frowns vacuous. This is admitted by public QSys.
def vacuous : Paxos Unit Ballot Bool :=
  { Counterexample.bad with
    QI := fun _ _ => False
    QII := fun _ _ => True
    promised := fun _ _ _ => False
    promisedV := fun _ _ _ _ => False
    accepted := fun _ _ _ => False
    proposed := fun _ _ => False
    chosen := fun _ _ => False }

theorem p1_without_nonempty : vacuous.P1 ∧
    ¬ (∀ e q, vacuous.QII e q → ∃ a, q a) := by
  constructor
  · intro _
    constructor
    · intro _ _ _ h
      exact h.elim
    · intro _ _ h
      exact h.elim
  · intro h
    obtain ⟨a, ha⟩ := h 0 (fun _ => False) True.intro
    exact ha

-- A safe restriction need not satisfy P1: require all proposals to carry
-- one fixed value. This demonstrates the quantifier gap in universal
-- "any scheme without overlap is unsafe" wording, not a useful consensus
-- protocol (unrestricted validity is intentionally absent).
def constantHistory : Paxos Bool Ballot Bool :=
  { Counterexample.bad with v := fun _ _ => false }

theorem constantHistory_safe :
    ¬ constantHistory.P1 ∧
    (∀ i b1 b2, constantHistory.chosen i b1 → constantHistory.chosen i b2 →
      constantHistory.v i b1 = constantHistory.v i b2) := by
  constructor
  · exact Counterexample.not_p1
  · intros
    rfl

-- Positive weight change >1 need not destroy overlap. The existing +2
-- negative control proves an existential obstruction, not its converse.
theorem plus_two_can_preserve :
    Frown (ofList (majW [2,2,2,0])) (ofList (majW [4,2,2,0])) := by
  rw [← frownB_iff]
  decide

#print axioms ExistingAudit.p1_without_nonempty
#print axioms ExistingAudit.constantHistory_safe
#print axioms ExistingAudit.plus_two_can_preserve
end ExistingAudit
