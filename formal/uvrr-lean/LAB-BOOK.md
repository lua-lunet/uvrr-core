# uVRR validation laboratory book

## Objective and working rules

Prove the uVRR protocol, validate the inherited proof ladder, and produce a
publication-quality LaTeX paper. Preserve intermediate results so the work can
resume without repeating expensive model calls. The research budget is finite.
Use one small obligation at a time, compile locally, preserve negative controls,
and commit verified increments. Do not equate a passing transcript with a proof
of its prose. Do not equate a safety theorem with liveness or measured latency.

## 2026-09-06 A — inherited evidence checked

Inputs: the objective attachment; `uVRR-reconfiguration-safety-lean.docx`;
the seven `UVRR/*.lean` modules imported by the original `UVRR.lean`; all eight
original `ladder/0*.md` documents; Turner's UPaxos PDF (revision 1A9DBA37).

Independent commands, run before changing these inputs:

```sh
export PATH="$HOME/.elan/bin:$PATH"
cd formal/uvrr-lean
lake build
for f in ladder/0*.md; do showboat verify "$f" || exit; done
```

Observed: build exit 0; all eight transcript verifications exit 0. Original
file hashes, build log, individual verification logs and all theorem axiom
reports are in `evidence/baseline/`. Empty Showboat logs mean successful silent
verification, not missing test execution.

Scientific conclusions:

- Rungs 1–7 contain compiled proofs. Rung 8 reproduces a failed draft; it has
  neither a build command nor an axiom printout. Thus the statement that *all
  eight are successful proofs embedding those commands* is false, although
  all eight transcripts reproduce.
- The Synod agreement theorem is conditional on history invariants, ballot
  order, and nonempty phase-II quorums. It is not an operational VRR proof.
- The era theorem additionally assumes `EraLe`; suffix promises and the
  known-instance frontier were omitted from the inherited abstraction.
- The cross-era counterexample establishes that the overlap hypothesis cannot
  simply be dropped from the abstract theorem. It does not establish that
  every conceivable protocol lacking that overlap is unsafe.
- Casting-vote results preserve an acceptance guard under the stated delivery
  restriction. They do not prove election progress or an elapsed-time bound.
- The concrete weighted schedule does not prove the general weighted lemma.

Three extra public-interface witnesses compiled unchanged against the seed:
P1 can hold with empty phase-I families; a constant-value restriction can have
agreement without P1; a particular +2 weight change can preserve intersection.
Their source and compiler output are preserved in `evidence/baseline/`.
These delimit quantifiers; they are not production protocol changes.

## 2026-09-06 B — credit interruption and bounded Leanstral attempt

Three audit/proof agents were interrupted by exhausted account credits.
No operational protocol module or general weighted proof was delivered.
The user then requested tighter cost control and persistent checkpoints.
Continue serially unless a clearly bounded delegation saves total effort.

One new Leanstral experiment ran before the interruption. It asked for a
single list-induction lemma, with unchanged definitions and theorem statement.
Limits: 12 turns, USD 2 configured cap, 28,000 total tokens, 180 seconds,
2 GiB monitored process-tree RSS; Lean child command limited to 512 MiB.
Observed: 8.19 seconds, peak sampled RSS 210,176 KiB, exit 1. The Vibe scaffold
reported `Token limit exceeded: 35,113 > 28,000` after reading the file; no
proof edit was made. This is a harness-budget failure, not a failed
mathematical proof attempt. Actual billed cost is unavailable; the configured
cap is not an invoice. Preserve prompt, run summary and sanitized output in
`leanstral/attempt-20260906/`. Do not repeat this identical launch.

Mistral's [Leanstral announcement](https://mistral.ai/news/leanstral/), read
2026-09-06, reports benchmark comparisons and two case studies. Its reported
costs do not measure this repository's workflow. Our eventual paper must
report model provenance, accepted edits, compilation, elapsed time and
available cost evidence separately. Multi-model authorship and any claimed
efficiency improvement need evidence; no controlled comparison exists yet.

## 2026-09-06 C — next verified increment

`UVRR/MultiPromise.lean` now compiles with Lean 4.33.1. It expands finite suffix
certificates to per-slot promises and derives `EraLe` from Figure 2's P2–P5,
monotonic instance eras and nonempty phase-I quorums. It carries the `imax`
choice bound and reduces the expanded history to the inherited agreement
theorem. This closes a history-level gap, not operational invariant
preservation. The first compile exposed a namespace elaboration error; the
namespace was corrected and the same file compiled with exit 0.

## Resume queue

Checkpoint C verification: `lake env lean UVRR/MultiPromise.lean`,
`lake build UVRR.MultiPromise`, and `showboat verify ladder/12-multi-promises.md`
all exit 0. The Rust gates also passed: format check, Clippy with all targets
and features and warnings denied, all-feature/all-target tests, doctests, and
the searches forbidding inline tests and Paxos references in Rust sources.
No Rust implementation changes were made.

Checkpoint commit: `05a4d67`. This also banks the previously untracked seed
artifact subtree, without committing the user's other staged changes.
One additional repository text gate is **not green on the inherited tree**:
`git ls-files | rg -v '^maelstrom' | xargs rg -n 'item[0-9]'` descends into the
user-staged `tools/tla2tools` submodule directory and matches `item0` in its
Java code. No vendor code or unrelated staging was changed to suppress it.

## 2026-09-06 D — general weights and negative controls

`UVRR/WeightedGeneral.lean` proves the general scaled weighted-majority
intersection theorem (UPaxos Lemma 2), unit-change and equal-scale corollaries,
and self-intersection for arbitrary finite node lists. Natural subtraction is
used symmetrically to represent absolute difference; positive scale factors
preserve strict majority. A list-induction disjointness bound plus the two
integer majority margins yields contradiction when distance is at most one.
A two-node distance-two example has disjoint majorities, proving sharpness of
the *uniform* distance bound. All compile without Mathlib, holes or custom
axioms. The first two local compile iterations repaired proof elaboration;
definitions and the target mathematical statement were retained.

`UVRR/NegativeControls.lean` gives checked histories admitting disagreement
when respectively S4's value choice, S2's promise fence, or decision-quorum
nonemptiness is removed, preserving the other listed invariant conditions.
`check_mutations.py` compiles the unchanged Structure module, then separately
replaces universal quorum checking by existential checking and installs an
unsafe quorum in the concrete schedule. Both mutants are rejected by Lean.
A deliberately injected `sorry` compiles but exposes `sorryAx`, demonstrating
why axiom inspection is an independent gate. No original source is mutated
in place. These tests do not prove mutation completeness or every assumption's
universal necessity.

Leanstral retry: same one-lemma target, 6-turn / USD 0.50 configured cap,
150,000 cumulative-token cap, 120-second and 2-GiB RSS watchdogs. It ended
after 76.39 seconds, sampled peak 210,368 KiB, with
`Token limit exceeded: 166,965 > 150,000`. The final target still contained
the original hole. No Leanstral-generated proof is credited as accepted.
Two minimal direct-API routing probes also failed with HTTP errors; the
advertised `labs-leanstral-2603` route returned HTTP 400. Stop retries on this
obligation: completing the short induction locally avoids further scaffold
overhead. Evidence is in `leanstral/attempt-20260906-small/`. A dollar saving
has not been established; actual billed cost is unavailable. This negative
methodology result is useful for future budgeting.

The user identifies the preceding work as GLM-5.3-Flash followed by Claude
Fable 5.1, with rollout artifacts under `.tmp/rollouts/` and the report
`formal/lean-leanstral-report.md`. This is supplied provenance, not a new audit
of every historical model invocation. The independently reproducible Lean
artifacts are the correctness evidence regardless of authorship.

### Remaining work after checkpoint D

1. Bank baseline evidence and the MultiPromise module with an executable rung.
2. Add targeted kernel-checked counterexamples and compile-failure mutations;
   classify independence of assumptions separately from universal necessity.
3. Prove the general weighted intersection lemma; reuse the prepared tiny
   Leanstral target only after fixing the scaffold token overhead.
4. Build an explicit operational state machine and prove its reachable
   invariants. Do not put agreement itself in transition guards.
5. Cover VRR normal operation, view-change selection, state transfer, recovery
   fencing, clients, execution, and reconfiguration with a source-to-theorem
   coverage matrix. Check omissions against the whole 2012 paper.
6. Prove the relation between uVRR's era rules and operational VRR. Establish
   conditional progress separately; latency requires an actual benchmark.
7. Write and render the LaTeX paper, using exact checked statements and an
   explicit claim ledger. Keep unpublished obligations visible until closed.

Primary VRR paper successfully downloaded from MIT DSpace to
`.tmp/uvrr-audit/papers/vr-revisited.pdf` and text-extracted beside it.
The supplied blog is an aspiration ending in “TBC”, not a complete algorithm.
The operational specification must therefore be made explicit in this work.
