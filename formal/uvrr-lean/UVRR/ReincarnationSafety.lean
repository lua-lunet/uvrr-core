import UVRR.ReincarnationAgreement

/-! Final safety composition for five unit-weight voters.
The existing history invariants are the interface, not new proof obligations here.
The disk classifier is a host contract: clean restarts retain state; crashes
allocate fresh identities before sending. There is no same-identity amnesia step.
No liveness, timing, or Rust refinement claim is made by this theorem. -/
namespace ReincarnationSafety

/-- Turner history obligations other than quorum overlap. Overlap is proved
from the concrete two-era replacement, not assumed in this contract. -/
structure Contract {B V : Type} (P : Paxos Nat B V) : Prop where
  qi : P.QI = ReincarnationAgreement.family5
  qii : P.QII = ReincarnationAgreement.family5
  mono : P.EraMono
  eraLe : P.EraLe
  p23 : P.P23
  p4 : P.P4
  p5 : P.P5
  p6 : P.P6
  p7 : P.P7
  order : WellFounded P.lt ∧
    (∀ a b c, P.lt a b → P.lt b c → P.lt a c) ∧
    (∀ a, ¬ P.lt a a) ∧ (∀ a b, P.lt a b ∨ a = b ∨ P.lt b a)

/-- The replacement's intermediate and final voting weights, without a
simultaneous one-era identity swap. -/
def Replacement : Prop :=
  ReincarnationFive.c1 0 = 0 ∧ ReincarnationFive.c1 5 = 0 ∧
  ReincarnationFive.c2 0 = 0 ∧ ReincarnationFive.c2 5 = 1

/-- No two decisions for one slot disagree; the replacement obeys the two
weight steps; no continuation after a bump uses the pre-bump identity.
These are safety statements even for executions that stop forever. -/
def Safe {B V : Type} (P : Paxos Nat B V) : Prop :=
  (∀ i b c, P.chosen i b → P.chosen i c → P.v i b = P.v i c) ∧
  Replacement ∧
  (∀ old phase current,
    Reincarnation.IdentRun .bumped (Reincarnation.bump old) phase current →
    current ≠ old)

/-- Five-voter crash-stop reincarnation is safe across the two committed
reconfiguration eras, for every view schedule satisfying Contract. -/
theorem five_safe {B V : Type} (P : Paxos Nat B V) (h : Contract P) : Safe P := by
  refine ⟨?_, ?_, ?_⟩
  · intro i b c hb hc
    exact ReincarnationAgreement.five_agreement P h.qi h.qii h.mono h.eraLe
      h.p23 h.p4 h.p5 h.p6 h.p7 h.order hb hc
  · exact ⟨ReincarnationFive.c1_membership.1,
      ReincarnationFive.c1_membership.2.1,
      ReincarnationFive.c2_membership.2.2.2.2.2.1,
      ReincarnationFive.c2_membership.1⟩
  · intro old phase current hbump
    exact Reincarnation.amnesia_unreachable hbump

end ReincarnationSafety
