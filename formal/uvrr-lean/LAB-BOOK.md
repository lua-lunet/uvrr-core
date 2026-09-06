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
user-staged `tools/tla2tools` submodule directory and matches an indexed
constructor parameter in its Java code. No vendor code or unrelated staging
was changed to suppress it.

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

Checkpoint D commit: `970b743`. All eleven then-existing Showboat documents
verified after the new modules were added; the whole Lean library built.

## 2026-09-06 E — operational acceptor

`UVRR/Acceptor.lean` proves by induction on arbitrary finite executions that
the watermark-guarded accept/promise transitions preserve truthful historical
free and last-vote reports. The exported `free_forbids` and `report_last`
theorems establish the S2/S3 obligations of the seed theorem for one acceptor
in a single era. A concrete old-ballot acceptance after a higher promise
violates the invariant if the watermark guard is bypassed. The unmodified
transition relation refuses that step. Rung 10 embeds source, build and
axiom reports. This does not implement the proposer, whole-log selection,
cross-era acceptors, or recovery; no crash transition is silently identified
with persistent memory.

The next scientifically important integration work is whole-log view-change
selection and recovery fencing, not more arithmetic examples. The repository's
`VrrCoreEras.tla` explicitly identifies itself as a design model, fixes a
single era transition, and allows multi-nonce recovery evidence; VRR-2012
Section 4.3 uses one fresh nonce. These need an explicit refinement argument,
not a name mapping to Paxos.

### Remaining work after checkpoint E

1. Prove whole-log VRR view selection, including divergent uncommitted suffixes
   and incomplete prior view changes. Discharge message-history invariants
   from transitions, without using agreement as a guard.
2. Prove crash/recovery exclusion, freshness and restoration of prior vote
   obligations. Address the design model's multi-nonce evidence explicitly.
3. Compose operational reconfiguration across arbitrarily many eras; justify
   casting-vote schedules and safe member retirement.
4. Connect committed log prefixes to client-visible execution, including
   deduplication, client restart, and nondeterministic inputs.
5. Establish conditional progress and define the benchmark assumptions for
   failover latency. No asynchronous safety proof supplies a timing bound.
6. Extend the manuscript as each obligation closes. Keep its coverage matrix
   synchronized with checked declarations. Audit the Rust/model refinement
   rather than assuming it from existing regression tests.

Primary VRR paper successfully downloaded from MIT DSpace to
`.tmp/uvrr-audit/papers/vr-revisited.pdf` and text-extracted beside it.
The supplied blog is an aspiration ending in “TBC”, not a complete algorithm.
The operational specification must therefore be made explicit in this work.

## 2026-09-06 F — manuscript and complete declaration audit

Checkpoint E commit: `a57870e`. `check_axioms.py` queries all 131 named
theorems/definitions/abbreviations exported by the current source files through
`import UVRR`; only standard Lean axioms are permitted. It is intentionally
independent of the report's prose claims. All 12 Showboat documents pass after
integration, and the unchanged-control / two mutant / axiom-hole checks pass.

The new rungs were renumbered consecutively: suffix promises are now
`ladder/11-multi-promises.md`, and negative controls are
`ladder/12-negative-controls.md`. Earlier diary entries describe paths in
their checkpoint commits; the current README gives current paths.

`paper/paper.tex` is a six-page research manuscript with theorem statements,
proof arguments, a coverage matrix spanning VRR-2012 Sections 1–9, a trust
boundary, and the bounded Leanstral methodology. Tectonic builds the PDF.
Every rendered page was inspected; the overfull transition equation was
reformatted and the final checked layout has no overfull boxes or unresolved
references. The manuscript explicitly retains the incomplete end-to-end
obligation; it is not a claim that the research goal is finished.

The current README no longer repeats superseded baseline claims. The original
DOCX and baseline commit remain the audit inputs. The full VRR paper was read,
including recovery, reconfiguration, pragmatics and optional read optimizations.
In particular the source's backup-read variant has weaker semantics than
linearizability, so it cannot silently become part of a strong-consistency claim.

Bounded TLC spot audit: `VrrCoreErasM1.cfg` on the unchanged design model
reproduced `Invariant FrontiersOrdered is violated` (exit 12) in 1.2 seconds,
with 2,111 distinct states found. This is the intended transfer-fault
counterexample, not a newly diagnosed defect in the unmutated model. Its
configuration and source hashes are preserved with the run report.

The unchanged `VrrCoreEras.cfg` breadth-first run completed in 137.67 seconds
(exit 0): 2,743,933 generated states, 837,204 distinct states, queue empty,
depth 32, no error found. Bounds are `inc3`, one command, log length 3,
view index 0, two eras. `MaxEpoch=0` disables crashes; this is not evidence
for recovery. TLC 2.19 reported an observed-fingerprint collision estimate
of 2.2e-7. The run used two workers, a 2-GiB Java heap cap, and a 180-second
wall limit. Both run reports and logs are banked under `evidence/tlc/`.

Final checkpoint checks: all 12 Showboat documents replay; `lake build`
passes; 131 declarations pass the standard-axiom allowlist; fault controls
pass. Rust format, all-feature/all-target Clippy/tests, doctests and source
placement checks pass. The separate vendor-directory text-gate baseline
failure remains documented above. No Rust implementation or user-staged
submodule changes have been included in the research commits.

## 2026-09-06 G — executable whole-log view selection

The preceding goal turn was **progress**: it banked proofs, an axiom audit,
independent model-checking evidence, and a rendered manuscript. On resumption,
the current worktree and lab book were inspected; the baseline audit was not
repeated. User-staged changes remain outside this work.

`UVRR/ViewSelection.lean` implements VRR-2012's single-era report selection:
lexicographically maximize last-normal view and log length. A seed report
enforces a nonempty collection, and each report's committed frontier is bounded
by its log length in its type. The selector uses only rank comparisons. It
does not check or assume global committed-prefix agreement as an operation.

Checked results: the returned report is a member and dominates every input;
same-view comparability plus a surviving quorum witness and the strictly
later-view induction hypothesis imply preservation of a committed prefix.
`quorum_preserves` obtains that witness by intersecting the committing and
view-change quorums. Report coverage is by sender identity; duplicates cannot
create additional quorum identities. An older longer log with a divergent
uncommitted suffix loses to a newer shorter log. A length-only mutant instead
loses the later-view committed prefix.

This closes the selection subproblem, not the complete view-change protocol:
voter-history survival, same-view log comparability and later-view preservation
must be established from reachable message states. Those premises are explicit
in the theorem and are not claimed as proved merely because selection is
correct. Natural view numbers cover a single configuration era; composition
with uVRR era-tagged views remains on the queue. The first compile found a
reserved identifier (`prefix`) and a missing equality decision instance; the
identifier and concrete proof were corrected without changing the algorithm
or theorem assumptions. The next compile was clean.

The model correspondence is also checked: `scalar_rank_equivalent` proves
that the TLA `ReportRank` arithmetic `view * (MaxLogLength + 1) + length`
agrees exactly with the selector's lexicographic order when both lengths obey
the bound. `scalar_without_bound_misranks` witnesses failure without that
bound. This is a concrete ranking refinement; it does not establish the
rest of `InstallView` or the provenance of the reports it receives.

Validation for this increment: all 13 Showboat rungs replay successfully;
151 named declarations pass the axiom allowlist; library build and mutation
controls pass. Rust formatting, all-target/all-feature Clippy and tests,
doctests, and source-placement checks pass. All seven rendered manuscript
pages were visually inspected; no overfull boxes or unresolved references.
The inherited vendor text-gate exception remains unchanged. Evidence and
source/PDF hashes are in `evidence/view-selection/validation.json`. No
additional model-service calls were made. Resume by deriving the selection
premises from explicit message transitions, starting with same-view log
provenance; do not re-run the unchanged baseline TLC exploration.

## 2026-09-06 H — normal-log provenance and prefix retention

The preceding turn was **progress**, committed as `376cafd`. This increment
addresses the same-view premise left by the selector. `NormalLog.lean` defines
an explicit fixed-view message projection: primary append and send, StartView
installation once per replica, ordered prepare receipt, and report emission.
There is no global prefix check in a transition guard. Prepare records retain
the pre-send log as ghost evidence; the receiver uses only its length (the
wire slot) and entry. Messages persist, allowing arbitrary delivery delay and
duplication attempts. Replica identities and entry types are arbitrary.

The inductive invariant derives that every replica, report, and sent prepare
prefix is a prefix of the current primary log. Equal-length prefixes coincide,
which justifies an ordered receipt. `reports_comparable` and
`selection_same_view` discharge same-view comparability in this projection.
`replica_history` proves that a normal replica retains its prefix across any
finite continuation in the same view. The installed base is arbitrary, not
assumed globally safe. Unique primary activation, the erasure/simulation into
the full protocol, view fencing, and crash recovery are still composition
obligations. The projection intentionally does not claim cross-view voter
survival or full operational refinement.

Fault control: an explicitly reachable state has both authentic prepares
for `[false, true]` pending at an empty replica. Applying the second entry
without the next-slot guard yields `[true]`, which is not a prefix of the
primary log. The actual guard rejects that delivery. The mutation harness
compiles the unchanged normal-log module, then replaces only the receipt's
slot predicate with True; Lean rejects the altered preservation proof.
This is evidence about this transition rule, not universal necessity.

Local failures and corrections: the initial file write used the wrong working
path and wrote nothing; the first proof compile supplied a redundant source
state in dependent constructor cases. Removing those binder names fixed it.
A concrete-state record indentation error was corrected. Build then passed.
No production Rust behavior changed and no protocol bug is claimed. A replay
correctly detected the changed mutation script embedded in rung 12; its
captured source and output were refreshed, then replayed successfully. The
eight original rungs remain unchanged. New rung 14 contains source, compilation,
and axiom output. No additional external model-service calls were made.

Validation: library build passes (16 jobs); all 14 ladder rungs replay (rungs
1–11 in the first pass, refreshed 12 and 13–14 in the final pass); all 167 named
declarations pass the axiom allowlist; mutation controls pass. Rust formatting,
all-target/all-feature Clippy/tests, doctests and source-placement checks pass.
The inherited vendor-directory text-gate exception is unchanged. The seven-page
paper was rendered; changed pages 1–3 and 5–7 were visually inspected, and page
4 is byte-identical to the previously inspected render. No new TLC run or
baseline re-audit was needed.

Next: connect per-view provenance to view-change fencing and historical voter
reports in one multi-view transition system, then add fresh-evidence recovery.
Do not treat the current one-view no-crash projection as that completed model.

## 2026-09-06 I — view fencing and induction over activated views

The preceding turn was **progress**, committed as `2cf0f2e`. This increment
uses `ViewFence.lean` to derive the voter-history premise that rung 13 had
assumed. A local replica installs a strictly higher normal view with an arbitrary
log, appends in normal mode, enters view change by raising its floor, and emits
a reply only after that fence. The arbitrary installation log avoids embedding
cross-view agreement in a transition guard. Immutable votes and replies are
ghost evidence, not a proposed durable-storage requirement.

`report_covers_vote` covers every historical vote below a reply's target,
including votes occurring later in the trace. The vote's view cannot exceed
the reported last-normal view; at equality its log is a prefix of the report.
`voter_history` derives the exact selector interface from local executions and
commit-quorum votes that extend a committed prefix. `selection_preserves`
consumes that result. The complete first view-change message round and recovery
are not represented by this local projection.

`all_later_views_preserve` then closes the strong induction over natural view
numbers: every later activated view's selected base preserves a quorum-voted
prefix. The caller no longer assumes later-view preservation. Remaining global
interfaces are authentic report provenance, same-view comparability, and report
extension of the base installed in its retained view. The latter records that a
positive retained view was actually activated. These must be derived jointly
from one full protocol execution, not treated as already refined from Rust/TLA.
Configuration changes still require era composition; this theorem fixes one
quorum family and supplies no recovery or progress result.

A draft required reports for every natural view number, which would not admit a
finite ghost history. Before banking, the interface was corrected to quantify
only activated views. `two_view_application` constructs a nonempty reachable
execution (fence/report, install, append/vote, fence/report) and discharges every
hypothesis of the general induction. It is an interface-inhabitation control,
not the extent of the general theorem. The first local induction compiled on
its first attempt. The fault example initially lacked a decidable record-equality
instance; using direct list-membership constructors fixed that proof without
changing its statement.

Fault evidence: after a reply at target 1 reporting the empty view-0 log,
bypassing normal-mode fencing allows a view-0 append. The immutable reply then
omits that below-target vote. The constructive witness proves the broken
historical property; the temporary mutation replaces only the append guard
with True, and Lean rejects the preservation proof. Unchanged controls compile.

Validation: build passes (17 jobs), all 192 named declarations pass the standard
axiom allowlist, affected rungs 12 and 15 replay, and mutation checks pass.
Other rungs and their sources are unchanged from their banked replays. Rust
formatting, all-target/all-feature Clippy/tests, doctests, and source-placement
checks pass; the inherited vendor-directory text-gate exception remains.
No external model-service call, TLC rerun, or baseline re-audit was needed.
The manuscript remains the rendered rung-14 checkpoint; integrate this rung
with the next global-provenance result to avoid repeated layout work.

Next: derive the global provenance interfaces jointly from multi-view message
transitions. In particular connect NormalLog's per-view message chains to
ViewFence's installs/reports, prove report extension of the installed base,
and preserve unique activation across recovery. Keep the distinction between
history-level induction and full operational simulation visible in the paper.
