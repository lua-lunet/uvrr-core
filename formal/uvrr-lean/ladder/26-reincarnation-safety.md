# Rung 26: five-voter reincarnation safety from the existing invariants

*2026-09-11T10:11:12Z by Showboat 0.6.1*
<!-- showboat-id: d44bf85c-963b-4dda-b871-dd44b30dea54 -->

The final target is `ReincarnationSafety.five_safe`. Given the existing Turner history contract, five unit-weight voters can replace identity 0 with identity 5 through two committed eras without conflicting decisions. The argument uses the existing ladder as premises; it does not re-prove the protocol or its disk adapter.

1. The weights are `(1,1,1,1,1,0)`, then `(0,1,1,1,1,0)`, then `(0,1,1,1,1,1)`. Each boundary moves one unit. Rung 25 derives P1 from weighted-majority intersection, rather than assuming P1.
2. Turner's Theorem 10, instantiated in `five_agreement`, then rules out two different chosen values for any slot. View schedules are unrestricted subject to the history invariants in `Contract`; there is no timeout or fairness premise.
3. `Replacement` records that old and new are both non-voters in the intermediate era, and only the new identity has weight one after promotion. The E1/E2 casting-vote example is the existing pivot at node 3, checked below.
4. `amnesia_unreachable` rules out a return to the pre-bump identity in the existing identity transition model. The host supplies the clean-restart versus fresh-incarnation classification. Its disk implementation is outside this theorem.

The final proof concludes agreement, replacement weights, and identity non-reuse together. It is a composition theorem over the stated abstract invariants, not a Rust refinement or liveness theorem. Two eras means two committed reconfiguration batches, not a wall-clock bound or a claim that catch-up needs no additional messages.

Leanstral supplied the proof in two bounded API rounds. The first attempt projected the wrong component of the final membership lemma; compiler feedback corrected it on round two. Definitions and the theorem statement were compared byte-for-byte with the submitted target. The accepted proof was then shortened without changing that statement and compiled again. No new disk model, liveness proof, or state-space search is needed for this rung.

```bash
cat UVRR/ReincarnationSafety.lean
```

```output
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
```

```bash
set -o pipefail; lake build 2>&1 | tail -1; python3 check_axioms.py
```

```output
Build completed successfully (24 jobs).
PASS 355 declarations: only standard Lean axioms
```

```bash
printf '%s\n' 'import UVRR.ReincarnationSafety' '#print axioms ReincarnationSafety.five_safe' '#print axioms ReincarnationAgreement.five_leader_overlap' | lake env lean --stdin
```

```output
'ReincarnationSafety.five_safe' depends on axioms: [propext, Classical.choice, Quot.sound]
'ReincarnationAgreement.five_leader_overlap' depends on axioms: [propext, Classical.choice, Quot.sound]
```

Negative control: claim the replacement already votes in the intermediate era. The same proof must fail. This checks that the concrete replacement conjunct is enforced.

```bash
python3 - <<'PYTEST'
from pathlib import Path
import subprocess, tempfile
source = Path('UVRR/ReincarnationSafety.lean').read_text()
mutant = source.replace('ReincarnationFive.c1 5 = 0 ∧', 'ReincarnationFive.c1 5 = 1 ∧')
assert mutant != source
with tempfile.TemporaryDirectory() as tmp:
    path = Path(tmp) / 'PrematureVote.lean'
    path.write_text(mutant)
    run = subprocess.run(['lake', 'env', 'lean', str(path)], text=True,
                         capture_output=True, timeout=30)
    assert run.returncode != 0, 'unsafe replacement was accepted'
    assert 'Application type mismatch' in run.stdout, run.stdout + run.stderr
print('PASS: premature replacement vote rejected by Lean')
PYTEST
```

```output
PASS: premature replacement vote rejected by Lean
```
