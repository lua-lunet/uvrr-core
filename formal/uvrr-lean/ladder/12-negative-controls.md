# Rung 12: Fault injection and assumption independence

*2026-09-06T07:32:54Z by Showboat 0.6.1*
<!-- showboat-id: 1c648562-4fe7-4d7e-b26e-7be5e9f2ec8e -->

Claim: wrong value selection, a forgotten prior vote, and empty decision quorums each admit conflicting choices under the other stated abstract invariants. These witnesses establish independence in this history model. Compile-failure mutations additionally test sensitivity of the existing quorum checker and schedule; a sorryAx injection confirms why compiler success alone is insufficient.

```bash
cat UVRR/NegativeControls.lean; cat check_mutations.py
```

```output
import UVRR.Counterexample

/-! Kernel-checked independence witnesses for the abstract agreement contract.
Each witness preserves the other stated conditions. These are histories, not
executions of a correct protocol. They show why an invariant must be discharged
by an implementation; they do not claim every possible algorithm needs it. -/
namespace NegativeControls

def singleton : QSys Unit := fun q => q ()

def wrongValue : Synod Unit Nat Bool where
  lt := (· < ·)
  QI := fun _ => singleton
  QII := fun _ => singleton
  promised := fun _ b => b = 0
  promisedV := fun _ b c => b = 1 ∧ c = 0
  proposed := fun b => b ≤ 1
  accepted := fun _ b => b ≤ 1
  chosen := fun b => b ≤ 1
  v := fun b => decide (b = 1)

theorem wrongValue_order : wrongValue.Order where
  wf := Nat.lt_wfRel.wf
  trans := fun _ _ _ => Nat.lt_trans
  irrefl := Nat.lt_irrefl
  total := fun a b => by change a < b ∨ a = b ∨ b < a; omega

theorem wrongValue_nonempty : ∀ b q, wrongValue.QII b q → ∃ a, q a :=
  fun _ _ h => ⟨(), h⟩

theorem wrongValue_other_invariants : wrongValue.S1 ∧ wrongValue.S2 ∧
    wrongValue.S3 ∧ wrongValue.S5 ∧ wrongValue.S6 := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩
  · intro _ _ _ _ _ q r hq hr; exact ⟨(), hq, hr⟩
  · intro a b c hp hlt; change b = 0 at hp; change c < b at hlt; omega
  · intro a b c hp
    change b = 1 ∧ c = 0 at hp
    refine ⟨?_, ?_, ?_⟩
    · change c < b; omega
    · change c ≤ 1; omega
    · intro d hcd hdb; change c < d at hcd; change d < b at hdb; omega
  · intro a b h; exact h
  · intro b h; exact ⟨fun _ => True, True.intro, fun _ _ => h⟩

theorem wrongValue_not_s4 : ¬ wrongValue.S4 := by
  intro h
  obtain ⟨q, hq, _, hm⟩ := h 1 (by change 1 ≤ 1; omega)
  obtain ⟨bm, ⟨a, _, ha⟩, _, hv⟩ := hm ⟨(), 0, hq, rfl, rfl⟩
  change 1 = 1 ∧ bm = 0 at ha
  have hb : bm = 0 := ha.2
  subst bm
  change true = false at hv
  cases hv

theorem wrongValue_disagrees : wrongValue.chosen 0 ∧ wrongValue.chosen 1 ∧
    wrongValue.v 0 ≠ wrongValue.v 1 := by simp [wrongValue]

/-- Mutation: a node reports a free promise after already accepting below it.
The proposal rule is now unconstrained; only the promise-fencing invariant fails. -/
def forgottenVote : Synod Unit Nat Bool :=
  { wrongValue with promised := fun _ b => b ≤ 1, promisedV := fun _ _ _ => False }

theorem forgottenVote_other_invariants : forgottenVote.S1 ∧ forgottenVote.S3 ∧
    forgottenVote.S4 ∧ forgottenVote.S5 ∧ forgottenVote.S6 := by
  refine ⟨wrongValue_other_invariants.1, ?_, ?_, ?_, ?_⟩
  · intro _ _ _ h; exact h.elim
  · intro b h
    refine ⟨fun _ => True, True.intro, fun _ _ => Or.inl h, ?_⟩
    intro ⟨_, _, _, hf⟩; exact hf.elim
  · intro _ _ h; exact h
  · intro b h; exact ⟨fun _ => True, True.intro, fun _ _ => h⟩

theorem forgottenVote_not_s2 : ¬ forgottenVote.S2 := by
  intro h
  exact h () 1 0 (by change 1 ≤ 1; omega) (by change 0 < 1; omega) (by change 0 ≤ 1; omega)

theorem forgottenVote_disagrees : forgottenVote.chosen 0 ∧ forgottenVote.chosen 1 ∧
    forgottenVote.v 0 ≠ forgottenVote.v 1 := by simp [wrongValue, forgottenVote]

/-- Mutation: an empty decision quorum permits choice with no proposal. -/
def emptyDecision : Synod Unit Nat Bool :=
  { wrongValue with
    QII := fun _ q => ∀ a, ¬ q a
    promised := fun _ _ => False
    promisedV := fun _ _ _ => False
    proposed := fun _ => False
    accepted := fun _ _ => False }

theorem emptyDecision_inv : emptyDecision.Inv where
  s1 := by intro _ _ h; exact h.elim
  s2 := by intro _ _ _ h; exact h.elim
  s3 := by intro _ _ _ h; exact h.elim
  s4 := by intro _ h; exact h.elim
  s5 := by intro _ _ h; exact h.elim
  s6 := by
    intro b _
    exact ⟨fun _ => False, fun _ h => h, fun _ h => h⟩

theorem emptyDecision_not_nonempty :
    ¬ (∀ b q, emptyDecision.QII b q → ∃ a, q a) := by
  intro h
  obtain ⟨_, hf⟩ := h 0 (fun _ => False) (fun _ hf => hf)
  exact hf

theorem emptyDecision_disagrees : emptyDecision.chosen 0 ∧ emptyDecision.chosen 1 ∧
    emptyDecision.v 0 ≠ emptyDecision.v 1 := by simp [wrongValue, emptyDecision]

end NegativeControls
#!/usr/bin/env python3
"""Small bounded fault-injection checks. No network, edits to source, or packages.

Compile rejection measures proof sensitivity, not universal necessity. See
UVRR/NegativeControls.lean for constructive independence witnesses.
"""
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent


def lean(path):
    return subprocess.run(
        ["lake", "env", "lean", "-M512", "-T20000", str(path)],
        cwd=ROOT, text=True, capture_output=True, timeout=30,
    )


def main():
    source = (ROOT / "UVRR/Structure.lean").read_text()
    mutations = [
        ("existential quorum checker", "Q2.all fun l2", "Q2.any fun l2"),
        ("unsafe four-node decision family",
         "[[0,1,2],[0,1,3],[0,2,3],[1,2,3]]", "[[3]]"),
    ]
    with tempfile.TemporaryDirectory(prefix="uvrr-mutations-") as tmp:
        path = Path(tmp) / "Control.lean"
        path.write_text(source)
        baseline = lean(path)
        if baseline.returncode != 0:
            raise RuntimeError("unchanged control did not compile:\n" + baseline.stdout + baseline.stderr)
        print("PASS unchanged control compiles")
        for name, old, new in mutations:
            assert source.count(old) == 1, f"mutation location changed: {name}"
            path.write_text(source.replace(old, new))
            result = lean(path)
            # A timeout, import failure, or resource failure is not a killed mutant.
            output = result.stdout + result.stderr
            if result.returncode == 0 or "error:" not in output:
                raise RuntimeError(f"mutation was not rejected by Lean: {name}\n{output}")
            if any(marker in output for marker in
                   ("unknown module", "unknown module prefix", "memory", "heartbeats")):
                raise RuntimeError(f"infrastructure failure: {name}\n{output}")
            print(f"PASS Lean rejects {name}")
        normal = (ROOT / "UVRR/NormalLog.lean").read_text()
        path.write_text(normal)
        result = lean(path)
        if result.returncode != 0:
            raise RuntimeError("unchanged normal-log control failed:\n" + result.stdout + result.stderr)
        old = "(next : m.slot = log.length + 1) :\n      Step base s"
        assert normal.count(old) == 1, "receive-guard location changed"
        path.write_text(normal.replace(old, "(next : True) :\n      Step base s"))
        result = lean(path)
        output = result.stdout + result.stderr
        if result.returncode == 0 or "error:" not in output or any(
            marker in output for marker in
            ("unknown module", "unknown module prefix", "memory", "heartbeats")
        ):
            raise RuntimeError("slot-guard mutant was not rejected by the proof:\n" + output)
        print("PASS Lean rejects omitted next-slot guard")
        fence = (ROOT / "UVRR/ViewFence.lean").read_text()
        path.write_text(fence)
        result = lean(path)
        if result.returncode != 0:
            raise RuntimeError("unchanged view-fence control failed:\n" + result.stdout + result.stderr)
        old = "(normal : s.floor = s.retained) : Step s (append s x)"
        assert fence.count(old) == 1, "normal-mode guard location changed"
        path.write_text(fence.replace(old, "(normal : True) : Step s (append s x)"))
        result = lean(path)
        output = result.stdout + result.stderr
        if result.returncode == 0 or "error:" not in output or any(
            marker in output for marker in
            ("unknown module", "unknown module prefix", "memory", "heartbeats")
        ):
            raise RuntimeError("normal-mode mutant was not rejected by the proof:\n" + output)
        print("PASS Lean rejects append after view-change fence")
        protocol = (ROOT / "UVRR/LogProvenance.lean").read_text()
        path.write_text(protocol)
        result = lean(path)
        if result.returncode != 0:
            raise RuntimeError("unchanged shared-model control failed:\n" + result.stdout + result.stderr)
        old = "(view : m.view = (s.nodes a).retained)\n      (next :"
        assert protocol.count(old) == 1, "message-view guard location changed"
        path.write_text(protocol.replace(old, "(view : True)\n      (next :"))
        result = lean(path)
        output = result.stdout + result.stderr
        if result.returncode == 0 or "error:" not in output or any(
            marker in output for marker in
            ("unknown module", "unknown module prefix", "memory", "heartbeats")
        ):
            raise RuntimeError("message-view mutant was not rejected by the proof:\n" + output)
        print("PASS Lean rejects cross-view prepare receipt")
        recovery = (ROOT / "UVRR/RecoveryFence.lean").read_text()
        path.write_text(recovery)
        result = lean(path)
        if result.returncode != 0:
            raise RuntimeError("unchanged recovery control failed:\n" + result.stdout + result.stderr)
        old = "r.recipient = a ∧ r.generation = s.generation a)\n      (maximum :"
        assert recovery.count(old) == 1, "episode-freshness guard location changed"
        path.write_text(recovery.replace(old, "r.recipient = a)\n      (maximum :"))
        result = lean(path)
        output = result.stdout + result.stderr
        if result.returncode == 0 or "error:" not in output or any(
            marker in output for marker in
            ("unknown module", "unknown module prefix", "memory", "heartbeats")
        ):
            raise RuntimeError("episode-freshness mutant was not rejected by the proof:\n" + output)
        print("PASS Lean rejects stale recovery episode evidence")
        vector = (ROOT / "UVRR/CrashVector.lean").read_text()
        path.write_text(vector)
        result = lean(path)
        if result.returncode != 0:
            raise RuntimeError("unchanged crash-vector control failed:\n" + result.stdout + result.stderr)
        old = "⟨known, prune known (r :: s.replies)⟩"
        assert vector.count(old) == 1, "crash-vector filter location changed"
        path.write_text(vector.replace(old, "⟨known, r :: s.replies⟩"))
        result = lean(path)
        output = result.stdout + result.stderr
        if result.returncode == 0 or "error:" not in output or any(
            marker in output for marker in
            ("unknown module", "unknown module prefix", "memory", "heartbeats")
        ):
            raise RuntimeError("crash-vector mutant was not rejected by the proof:\n" + output)
        print("PASS Lean rejects omitted crash-vector filtering")
        acquisition = (ROOT / "UVRR/AcquisitionOrder.lean").read_text()
        path.write_text(acquisition)
        result = lean(path)
        if result.returncode != 0:
            raise RuntimeError("unchanged acquisition control failed:\n" + result.stdout + result.stderr)
        old = "(∀ b, responders b → support b → witness b ≤ sent b)"
        assert acquisition.count(old) == 1, "acquisition ordering location changed"
        path.write_text(acquisition.replace(old, "(∀ b, responders b → support b → True)"))
        result = lean(path)
        output = result.stdout + result.stderr
        if result.returncode == 0 or "error:" not in output or any(
            marker in output for marker in
            ("unknown module", "unknown module prefix", "memory", "heartbeats")
        ):
            raise RuntimeError("acquisition-order mutant was not rejected by the proof:\n" + output)
        print("PASS Lean rejects omitted acquisition response ordering")
        path.write_text("theorem injected_hole : False := by sorry\n#print axioms injected_hole\n")
        result = lean(path)
        if result.returncode != 0 or "sorryAx" not in result.stdout:
            raise RuntimeError("axiom-hole control did not behave as expected")
        print("PASS axiom audit detects sorryAx despite compiler exit 0")


if __name__ == "__main__":
    main()
```

```bash
set -euo pipefail; lake build UVRR.NegativeControls >/dev/null; echo "UVRR.NegativeControls built"
```

```output
UVRR.NegativeControls built
```

```bash
printf 'import UVRR.NegativeControls\n#print axioms NegativeControls.wrongValue_order\n#print axioms NegativeControls.wrongValue_other_invariants\n#print axioms NegativeControls.wrongValue_not_s4\n#print axioms NegativeControls.wrongValue_disagrees\n#print axioms NegativeControls.forgottenVote_other_invariants\n#print axioms NegativeControls.forgottenVote_not_s2\n#print axioms NegativeControls.forgottenVote_disagrees\n#print axioms NegativeControls.emptyDecision_inv\n#print axioms NegativeControls.emptyDecision_not_nonempty\n#print axioms NegativeControls.emptyDecision_disagrees\n' | lake env lean --stdin
```

```output
'NegativeControls.wrongValue_order' depends on axioms: [propext, Quot.sound]
'NegativeControls.wrongValue_other_invariants' depends on axioms: [propext, Quot.sound]
'NegativeControls.wrongValue_not_s4' depends on axioms: [propext, Quot.sound]
'NegativeControls.wrongValue_disagrees' depends on axioms: [propext, Quot.sound]
'NegativeControls.forgottenVote_other_invariants' depends on axioms: [propext, Quot.sound]
'NegativeControls.forgottenVote_not_s2' depends on axioms: [propext, Quot.sound]
'NegativeControls.forgottenVote_disagrees' depends on axioms: [propext, Quot.sound]
'NegativeControls.emptyDecision_inv' does not depend on any axioms
'NegativeControls.emptyDecision_not_nonempty' does not depend on any axioms
'NegativeControls.emptyDecision_disagrees' depends on axioms: [propext, Quot.sound]
```

```bash
python3 check_mutations.py
```

```output
PASS unchanged control compiles
PASS Lean rejects existential quorum checker
PASS Lean rejects unsafe four-node decision family
PASS Lean rejects omitted next-slot guard
PASS Lean rejects append after view-change fence
PASS Lean rejects cross-view prepare receipt
PASS Lean rejects stale recovery episode evidence
PASS Lean rejects omitted crash-vector filtering
PASS Lean rejects omitted acquisition response ordering
PASS axiom audit detects sorryAx despite compiler exit 0
```
