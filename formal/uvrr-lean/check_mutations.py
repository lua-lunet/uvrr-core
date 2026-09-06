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
        path.write_text("theorem injected_hole : False := by sorry\n#print axioms injected_hole\n")
        result = lean(path)
        if result.returncode != 0 or "sorryAx" not in result.stdout:
            raise RuntimeError("axiom-hole control did not behave as expected")
        print("PASS axiom audit detects sorryAx despite compiler exit 0")


if __name__ == "__main__":
    main()
