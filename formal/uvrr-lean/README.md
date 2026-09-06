# uvrr-lean — Lean 4 proofs for uVRR cluster reconfiguration safety

## Unresolved repeated-recovery counterexample

The current implementation reproduces committed divergence under serial
amnesiac recoveries and authentic delayed messages, with fresh recovery ticks.
See the [counterexample and replay instructions](evidence/delayed-fence/README.md).
This prevents an end-to-end diskless-safety claim. The checked component
results below retain their stated scope; no repair is claimed yet.

## Verified research checkpoint — 6 September 2026

The current [LaTeX manuscript](paper/paper.tex) and [rendered paper](paper/paper.pdf)
state the results through rung 16 and the remaining end-to-end proof obligations.
Rung 17 is banked below; its manuscript integration awaits recovery composition.
The [laboratory book](LAB-BOOK.md) records commands, failures, bounded Leanstral
experiments, and commit checkpoints. The original DOCX and rungs 1–8 remain
audit inputs; the original README is retained in baseline commit `05a4d67`.

All eight original Showboat transcripts reproduce. Rung 8 reproduces a failed
draft, rather than a proved theorem. The current ladder is:

| Rung | Module | Verified result |
|---|---|---|
| 1 | `Structure.lean` | Finite quorum checker equivalence and a hot-swap schedule |
| 2 | `LexBallot.lean` | Well-founded lexicographic era/round order |
| 3 | `Synod.lean` | Agreement conditional on S1–S6, order and nonempty decision quorums |
| 4 | `Eras.lean` | Era-indexed reduction to the conditional agreement theorem |
| 5 | `Counterexample.lean` | Removing cross-era overlap admits disagreement under the other encoded conditions |
| 6 | `CastingVote.lean` | Conditional guard noninterference and phase-I quorum completion |
| 7 | `Weights.lean` | Concrete weighted schedule and a failing weight-change example |
| 8 | Historical Leanstral draft | Reproducible compiler failure; no proved target theorem |
| 9 | `WeightedGeneral.lean` | General finite-support scaled majority intersection; distance-two sharpness witness |
| 10 | `Acceptor.lean` | Reachable single-era promise/acceptance invariants and a guard-bypass counterexample |
| 11 | `MultiPromise.lean` | Derives `EraLe` from suffix/point promise bounds, monotone instance eras and nonempty phase-I quorums |
| 12 | `NegativeControls.lean` | Independent value-selection, promise-fencing and empty-decision witnesses; temporary source mutations |
| 13 | `ViewSelection.lean` | Executable latest-normal-view/length selection and quorum-based prefix preservation under explicit induction premises |
| 14 | `NormalLog.lean` | Fixed-view message induction derives report comparability and replica prefix retention; out-of-order delivery fault control |
| 15 | `ViewFence.lean` | Multi-view local history derives voter-report bounds; strong induction preserves committed prefixes in later activated views under explicit global provenance conditions |
| 16 | `LogProvenance.lean` | Shared multi-view transition induction proves committed-log compatibility for a fixed configuration without crashes; concrete trace and cross-view delivery control |
| 17 | `RecoveryFence.lean` | A supporting quorum retains a known fence through arbitrary crash/recovery sequences using fresh episode replies; stale-quorum fault control |
| 18 | Public Rust path and directed TLC trace | Reproduced unresolved committed divergence after serial recoveries; an expected Red witness, not a safety theorem |

```sh
export PATH="$HOME/.elan/bin:$PATH"
cd formal/uvrr-lean
lake build
python3 check_axioms.py
for f in ladder/[0-9][0-9]-*.md; do showboat verify "$f" || exit; done
python3 check_mutations.py
```

The broad glob includes rungs numbered 10 and above. The axiom audit queries
every named theorem, definition and abbreviation in the compiled project
sources. Only `propext`, `Quot.sound`, and `Classical.choice` are allowed.

Two corrections to the baseline wording are essential: the cross-era witness
proves that removing overlap admits inconsistency in the abstract contract;
it does not prove that every conceivable scheme without overlap is unsafe.
And passing the entire ladder is not yet an end-to-end uVRR or VRR-2012 proof.
Whole-log view selection, diskless recovery, repeated operational
reconfiguration, client linearizability and progress remain integration work.
