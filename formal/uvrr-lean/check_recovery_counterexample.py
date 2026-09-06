#!/usr/bin/env python3
"""Replay a known failing public-path schedule without hiding its Red outcome.

Zero exit here means the documented counterexample reproduced, NOT that the
protocol is safe. The temporary integration test asserts safety and must fail
with the exact committed-divergence diagnostic. A future repair must update
this research witness deliberately and install a passing regression test.
"""
from pathlib import Path
import os
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).resolve().parent / "evidence/delayed-fence/reproduction.rs"


def main():
    target = f"uvrr_recovery_counterexample_{os.getpid()}"
    path = ROOT / "tests" / f"{target}.rs"
    with path.open("x") as file:
        file.write(SOURCE.read_text())
    try:
        result = subprocess.run(
            ["cargo", "test", "--test", target, "--", "--nocapture"],
            cwd=ROOT, capture_output=True, text=True, timeout=60,
        )
    finally:
        path.unlink()
    output = result.stdout + result.stderr
    expected = "CommittedDivergence { a: NodeId(0), b: NodeId(1), slot: Slot(3) }"
    if result.returncode != 101 or expected not in output or "0 passed; 1 failed" not in output:
        raise RuntimeError("known counterexample changed; investigate rather than blessing it:\n" + output)
    print("REPRODUCED RED: unchanged core commits different operations at slot 3")
    print("Ordinary timeouts; authentic delayed messages; fresh recovery ticks; serial crashes")
    print("This result is a counterexample, not a protocol safety pass")


if __name__ == "__main__":
    main()
