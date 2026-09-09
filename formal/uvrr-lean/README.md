# uvrr-lean — Lean 4 proofs for uVRR cluster reconfiguration safety

## Crash-stop eviction (reincarnation)

uVRR forbids the classic crash-recover class: a node whose durable superblocks
record an unflushed session reopens with a NEW identity (an incarnation bump),
sends the leader a reincarnation message, and rejoins as a weight-0 standby —
(a **standby** is TigerBeetle's term for its non-voting cluster members; this
README's older drafts called it a learner. Standby nodes have a zero voting weight
so cannot form part of any quorum nor actively participate in the VSR algorithm.)
Crash-Stop-Self-Evict. The classic VRR-2012 diskless quorum recovery (§4.3)
and the DISC'17 Appendix B.1 amnesia class it exposes are therefore avoided by
construction in uVRR; those mechanisms are literature about classic
crash-recovery designs, not open problems here. The ladder's rung 17–20
fence/acquisition machinery is the proof skeleton the reincarnated weight-0
standby obeys. The protocol specification is [docs/uvrr-reincarnation.md](../../docs/uvrr-reincarnation.md).

## Editing the paper

Edit [paper/paper.tex](paper/paper.tex), then run `paper/build.sh`.
The [standalone build instructions](paper/README.md) require only Tectonic;
no proof harness or model service is involved. Each run publishes an
immutable, id-stamped PDF under [paper/papers/](paper/papers/).

## Verified research checkpoint — 6 September 2026

The current [LaTeX manuscript](paper/paper.tex) and the published PDFs under
[paper/papers/](paper/papers/) state the results through rung 20 and the
remaining end-to-end proof obligations.
The [laboratory book](LAB-BOOK.md) records commands, failures, bounded Leanstral
experiments, and commit checkpoints (its counterexample-era entries are dated
history about the classic crash-recovery attempt that reincarnation removed).
The original DOCX and rungs 1–8 remain
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
| 18 | `CrashVector.lean` | Published crash-vector collector: arbitrary reachable reply sets satisfy incarnation consistency; stale-quorum witness and filter-removal control |
| 19 | `AcquisitionOrder.lean` | Temporal acquisition induction under explicit recovery provenance; incarnation-retention premise discharged by the durable superblock identity; backward-response countermodel |
| 20 | `RecoveryAcquire.lean` | Operational crash/start/emit/answer/collect/finish acquisition; finished certificates are crash-consistent quorums for exactly their request and incarnation; stale-request countermodel |
| 22 | `Reincarnation.lean` | Crash-Stop-Self-Evict spec rung: two-era forced weight sequence (`crossEra`/`evictEra` batches, R14 one-unit mass rule), flushed/unflushed/dirty/bumped/reincarnating state machine, continuation commitment, unit-weight three-node era-safety instances, one-era swap refusal with disjoint-majority witness; general obligations discharged in `ReincarnationGeneral.lean` — arbitrary-scale mass bounds and era safety (`forced_sequence_era_safe`), bumped-identity non-membership (`evicted_never_voting`), amnesia unreachability (`amnesia_unreachable`), continuation commitment (`forced_run_committed`, `forced_run_terminal_flushed`) |
| 23 | `CastingVoteReincarnation.lean` | Rung 6's casting vote instantiated on the reincarnation eras: two-node leader-overlap case (`{0,1}` → `{0}` meeting in the leader alone, every E0/E1 quorum pair decisive at the leader, boundary overlap through the casting-vote structure) and the honest three-node degenerate negative (forming quorums coincide, no pivot needed, none exercised); leader-contained quorum families and leader-alone phase-I quorum as general shapes |
| 24 | `ReincarnationFive.lean` | The five-voter unit-scale reincarnation rung: victim `0` returns as identity `5` through the two-era batch form; majority families computed at the concrete configs (3-of-5 strictness, dead-victim three-of-four survivor bounds, the forming quorum `{1,2,3}` a majority in all three eras); era safety by rung 22G's general discharge instantiated (`forced_sequence_era_safe`, `evicted_never_voting`); the split casting-vote verdict — E0/E1 degenerate (every formable pair meets in ≥ 2 survivors, no pivot, none needed), E1/E2 pivot genuine (`{1,2,3}` and `{3,4,5}` meet in `3` alone); doubled victim survives one legal crossing era, motivating the iterated two-unit obligation |

```sh
export PATH="$HOME/.elan/bin:$PATH"
cd formal/uvrr-lean
showboat verify REPRODUCE.md
```

[REPRODUCE.md](REPRODUCE.md) is the single executable reproduction document:
exact tool versions, the library build, the axiom audit, every rung replay,
the mutation controls, the two TLC runs from a hash-pinned jar, the Rust
gates, the paper build and input digests.
`showboat extract REPRODUCE.md` emits the shell commands that recreate it.
The individual rungs can still be replayed one at a time:

```sh
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
Whole-log view selection, repeated operational
reconfiguration, client linearizability and progress remain integration work.
