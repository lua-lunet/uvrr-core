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

## 2026-09-06 J — shared multi-view committed-log safety

The preceding turn was **progress**, committed as `22cd219`. Rung 16 now joins
the local histories in one shared transition model, `LogProvenance.lean`.
Sources have immutable installation bases and append-only primary histories.
Activation is allowed once for a view and requires actual quorum reports plus
the executable maximum-rank selector. Installation reads that source base;
receivers require an issued prepare, normal mode, matching view, and next slot.
Local fence/report transitions are the ones checked in ViewFence. Pre-send
logs and immutable vote/report histories are ghost proof evidence.

The shared invariant proves both previously open provenance interfaces:
all replica logs, votes and reports extend their installed base and are
prefixes of one per-view source history. Source certificates remain authentic
because local reply histories persist. Local traces satisfy ViewFence's
reachable-state invariant. `later_base_preserves` supplies the strong view
induction directly from this invariant. `committed_comparable` proves that
any two quorum-voted prefixes in a reachable state are comparable;
`committed_equal` proves equality at equal lengths. No global agreement check
appears in a transition guard. View numbers, log length, and number of
activations are unbounded. The configuration and quorum family are fixed.

This is a complete committed-log safety induction for the stated crash-free
shared model. It is NOT the complete uVRR objective: unique source activation
is an explicit unused-view guard, source histories can issue prepares after
replicas have fenced the view, and acceptance is limited by local guards.
These are abstractions requiring simulation by the concrete primary/wire
protocol. The model does not yet include crashes, first-round view-change
knowledge, repeated membership changes, client replies, or progress.

Nonvacuity and fault evidence: Example.reachable constructs a real two-view
trace with an initial vote, a delayed second prepare, fence/report, certified
activation, and installation. Example.commits_both_views certifies a nonempty
prefix in both views. The delayed old-view prepare has exactly the next slot
at the installed replica; bypassing view equality permits a vote unsupported
by that view's source history. The constructive wrong_view_breaks_origin
witness and a temporary source mutation check this boundary. The unchanged
model compiles; removing only the receive view guard is rejected by Lean.

Local development: the provenance-only draft compiled on its first attempt.
Activation was then restricted by executable selection certificates, and the
certificate-preservation invariant was added. A record-layout parse error was
corrected. The complete shared-model safety proof compiled on its first
attempt. The concrete example initially needed an explicit Unit type on an
initial-source projection; adding it resolved elaboration. No Rust production
change or concrete implementation bug is claimed.

Validation: library build passes (18 jobs), all 232 named declarations pass
the standard-axiom allowlist, affected rungs 12 and 16 replay, and all mutation
controls pass. Unchanged rung sources retain their banked replay evidence.
Rust formatting, all-target/all-feature Clippy/tests, doctests, and source
placement pass; the inherited vendor text-gate exception remains unchanged.
Rungs 15 and 16 are integrated into the manuscript. An initial render left
one reference on a spare page; consolidating the superseded standalone normal-
log discussion produced seven pages. Final changed pages 5–7 were visually
inspected; pages 1–4 were unchanged from the inspected previous checkpoint.
No overfull boxes or unresolved references. No model-service calls or TLC
reruns were needed.

Next: model crash/recovery without preserving a physical replica's volatile
floor by assumption. Recovering replicas must be excluded; recovery responses
need fresh incarnation/nonce evidence and the first view-change round's
replicated knowledge. Establish the allowed failure/availability model before
claiming recovery safety. The shared source uniqueness abstraction must also
survive this extension or be justified by a concrete primary-activation proof.

## 2026-09-06 K — replicated recovery fences and stale episodes

The preceding turn was **progress**, committed as `9aa37d9`. Recovery was
compared with VRR-2012 Sections 2.2, 4.3 and 8.2 and the existing source model.
The source explicitly excludes recovering nodes from normal processing and
view changes, requires distinct fresh responses including the primary of the
latest reported view, and treats a node as failed until it recovers. The first
view-change message round replicates fencing knowledge. Nonce uniqueness is an
environment obligation (a monotone clock or counter), not something an erased
protocol state can magically remember.

The unchanged TLA Crash action permits only one crash overall: it requires all
recorded epochs to be zero before incrementing one. Its earlier MaxEpoch=0
baseline run already excluded all crashes. Neither is evidence of repeated
recovery. This is a scope observation, not an implementation bug. Rust's Tick
documentation in src/ids.rs explicitly assigns nonce non-reuse while old
messages may arrive to the host clock strategy. src/replica/recovery.rs retains
multiple solicitation nonces within one volatile episode, discards them on
crash, and combines current-episode evidence by sender. A refinement must map
that set to the new proof's episode freshness abstraction.

`RecoveryFence.lean` models online bound knowledge as Option Nat. A crash sets
it to None, so the bound is actually erased. An excluded node cannot generate
responses. Generation is ghost environment metadata identifying a fresh
recovery episode, not a durable local field or an implementation claim.
Response causality bounds an echoed episode by the recipient's current episode;
this abstracts authenticated request/response causality. Completion requires a
quorum of distinct senders, authentic historical responses to this recipient
and episode, and a maximum reported bound. Current sender status is not used
to invalidate already sent evidence.

Starting from a checkpoint where a fixed support quorum is online above a
known fence, `replicated_fence` proves that every online member of that support
continues to know at least that fence after any finite crash/recovery sequence.
There is no bound on crash count and no assumption that volatile floors survive.
The result protects the supporting quorum; it does not yet prove every arbitrary
recovering node restores its full pre-crash log or floor. Its checkpoint premise
must be established by the first-round message protocol, including timing and
recovery interactions. No liveness follows if fresh quorum evidence cannot be
obtained; no new physical failure-independence guarantee is inferred.

The concrete history first emits two low-bound replies, raises a two-of-three
support quorum to bound one, crashes a support member, and obtains fresh
responses from the other two nodes. `recovery_run` and
`recovery_preserves_fence` check the successful recovery. The two older replies
still have distinct senders and the right recipient; without freshness they
would form a quorum restoring zero. `stale_quorum_breaks_fence` proves the
failure, and the source-mutation harness checks rejection when only episode
equality is removed. This tests a reachable stale-message pattern, not an
invented unreachable state.

Local compile corrections: `protected` is a reserved Lean keyword, so the
invariant field was named `retained`. One arithmetic proof needed recipient
equality rewritten into its hypothesis. Two example goals needed explicit
unfolding of the checkpoint generation. Statements and transition rules were
not weakened to solve these elaboration issues. The final build has no warnings.

Next: compose the checkpoint premise with the first view-change message round,
and recover log contents from a fresh latest-view primary response. Preserve
historical votes/replies as ghost events while allowing actual local state to
be erased. Do not replace the missing composition with a guard asserting the
very safety property being proved. Rung 17 manuscript integration is deferred
to that recovery checkpoint; the current rendered paper covers through rung 16.

Validation: build passes (19 jobs), all 264 named declarations pass the axiom
allowlist, affected rungs 12 and 17 replay, and all mutation controls pass.
Rust formatting, all-target/all-feature Clippy/tests, doctests and source
placement pass. The inherited vendor text-gate exception is unchanged. No
baseline proof re-audit, TLC rerun, model-service call or manuscript render was
needed for this increment. Evidence hashes are under evidence/recovery-fence/.

## 2026-09-06 L — public-path repeated-recovery counterexample (Red)

The preceding turn was **progress**, committed as `0b8e89b`. Trying to discharge
the asynchronous first-round checkpoint obligation produced a concrete failure.
On unchanged production code, the public harness reports CommittedDivergence
between nodes 0 and 1 at slot 3. Node 0 committed operation x; node 1 subsequently
committed y at that slot. Three amnesiac recoveries occur serially (node 1 once,
node 2 twice), with at most one recovering node at a time. Every replayed packet
was first emitted by the live core; recovery ticks are fresh. The script first
used host-forced view changes, then reproduced the same failure with ordinary
backup timeouts. Both raw Red traces and source are saved under
`evidence/delayed-fence/`. No production fix or root-cause claim is made here.

A separate directed TLC execution also violates CommittedLogsAgree after 34
protocol actions (35 states). It uses an explicit research copy of VrrCoreEras:
replace the original at-most-one-crash-overall guard with at-most-one-recovering,
and use the incremented recovery epoch as that node's fresh recovery nonce.
No protocol safety guard is removed. TypeOK and OneRecovering precede the
agreement invariant in the configuration and hold throughout the trace.
The original TLA source is unchanged. The executable schedule, research copy,
configuration and complete Red log are preserved alongside the Rust evidence.

This contradicts end-to-end safety of the current implementation under this
repeated-amnesia schedule; it does not invalidate the checked crash-free
LogProvenance theorem or RecoveryFence's explicit online-quorum checkpoint
premise. That premise has not been proved for asynchronously collected first-
round messages. Do not silently promote it to a full recovery theorem, hide the
Red result, or treat an ordinary green suite as having refuted this witness.
The reproduction is archived as an explicitly expected negative research
witness, not installed as a silently skipped passing regression test. A repair
must face the same public-path schedule and preserve the intended no-forced-
disk, non-stop service requirements; merely preventing progress is not enough.

Final evidence checks: the ordinary-timeout Rust counterexample replays through
`check_recovery_counterexample.py` with the exact expected Red diagnostic, and
new rung 18 replays. The archived TLA schedule was rerun with captured exit code
12: CommittedLogsAgree fails, 35 distinct states, 0.873 seconds, 512-MiB heap,
20-second wall cap. Its TypeOK and one-recovering checks remain satisfied.
The original TLA model is untouched. The production source and harness hashes
are recorded in provenance.json, including the preserved user Cargo.toml state.

The ordinary Rust format/Clippy/all-target/all-feature tests/doctests and source
placement checks pass after archiving the research reproduction; this does not
negate its failure. The Lean build and 264-declaration axiom audit still pass.
No production code was changed, no failing assertion was weakened, and no
repair is claimed. No model-service calls were made. The README now leads with
the unresolved counterexample. Manuscript integration of rungs 17–18 remains
pending; the current PDF is explicitly the earlier component-proof checkpoint.

Next: investigate a protocol-level resolution and reproduce it against this
same schedule without imposing forced local storage or quietly sacrificing the
non-stop goal. Also review the temporal premises of the published recovery
argument; do not announce a literature novelty claim from this reproduction
alone. A passing narrow guard test is not sufficient to close the full objective.

## 2026-09-06 M — published antecedent and crash-vector collector

Continued from `1149eec`. Primary literature resolves the novelty question for
this failure pattern: Michael, Ports, Sharma and Szekeres describe delayed
view-change evidence across diskless recoveries in their 2017 extended report,
Appendix B.1 and Figure 1. Its predecessor is UW-CSE-16-08-02 (2016); the
updated report is UW-CSE-17-08-01, an extended DISC 2017 paper. Full URLs and
PDF hashes are banked in evidence/crash-vector/. Downloaded reading copies
remain in .tmp/uvrr-audit/recovery-literature/. This is a known published
failure class. Our Rust/TLC witnesses are useful implementation evidence,
not grounds for claiming a new discovery of that class.

Read Algorithm 1, Definition 6, the quorum-knowledge persistence/acquisition
arguments, liveness condition, and Appendix B.1. The next mechanism is more
than attaching incarnation vectors to ordinary recovery replies. A recovering
process must acquire and propagate its fresh incarnation through a
crash-consistent quorum; reconstruction and operational exclusion matter.
The publication explicitly warns that related epoch-vector approaches can
still fail. Do not substitute a local message filter for its temporal argument.

Rung 19 now checks the collector primitive in core Lean. State is a known
incarnation vector and a reply list. Receive joins vectors pointwise and
prunes replies whose sender incarnation is behind the joined frontier.
Induction derives pairwise crash consistency for every reachable reply set.
Sender membership is a set predicate; duplicates add no quorum identities.
A concrete unfiltered two-of-three quorum is inconsistent. Filtering removes
the stale reply and a current replacement restores quorum size. Removing
filtering in a temporary source copy makes Lean reject the proof, after the
unchanged copy compiles. This is proof sensitivity and a concrete witness,
not universal necessity across all possible algorithms.

The component deliberately leaves request freshness, resending, actual
recovery acquisition, quorum-knowledge persistence and value reconstruction
open. The replacement-reply example proves collector progress only; it does
not assert a network liveness theorem. The published termination premise is
a suitable stable quorum over the acquisition interval, not just a bound on
simultaneous recovering nodes. Production code is unchanged; no repair is
claimed and the banked public-path Red remains the regression target.

The manuscript now includes rungs 17–19, the explicit counterexample, and
credit to the published antecedent. It renders to eight pages. All pages
were visually checked; a too-wide vector equation was split and rerendered.
No overfull boxes remain. The earlier Leanstral methodology remains intact;
no model-service calls or additional paid proof experiments were made.

Next substantive step: mechanize crash-consistent acquisition and the temporal
quorum-knowledge argument, then compose recovery with committed-log safety.
Do not hide that obligation behind a global safety guard or count refusal to
make progress as successful repair. Preserve the exact public-path schedule
for testing any production change. Avoid repeating baseline audits or TLC
explorations already banked.

Validation: Lean build passes (20 jobs), the 290-declaration axiom audit allows
only standard Lean axioms, and rungs 12/19 replay. All mutation controls pass.
Rust format, all-target/all-feature Clippy/tests, doctests and source placement
pass. The inherited vendor text-gate exception is unchanged. The archived
public-path recovery runner again reproduces the exact slot-3 committed
divergence; its expected-Red result remains explicit. Evidence hashes and
check metadata are in evidence/crash-vector/validation.json. No TLC or
inherited-rung rerun was needed for this component-only increment.

## 2026-09-06 N — temporal reduction and independent paper editing

The previous turn was progress, banked as `d61be84`. Rung 20 now checks the
secondary acquisition induction in the published recovery argument. It works
with asynchronous per-participant witness times and a generic stable property.
An online participant either retains its witness knowledge or inherits it
from an earlier response quorum. Quorum intersection and forward ordering of
those responses relative to witness events discharge strong induction.
A separate lemma shows how crash-vector consistency forces that ordering
conditional on recovery participants retaining incarnation knowledge.

This is an explicit reduction: local recovery provenance and the outer
incarnation-retention induction are still premises to derive from protocol
transitions. It does not complete Theorem 3 or repair the implementation.
The countermodel satisfies the entire origin contract except forward
ordering, and loses the property. The mutation harness removes that ordering
premise and requires compiler rejection. Initial elaboration failed because
core Lean uses Nat.strongRecOn rather than a Mathlib induction name and needed
an explicit reduction of a constant function; both were corrected without
changing the statement. The final 21-job build and 300-declaration standard-
axiom audit pass. Rungs 12/20 replay and all mutations pass. Rust code/tests
are unchanged, so the immediately preceding passing Rust gates are reused;
no additional recovery/TLC run or model-service call was needed.

The user then requested an independently editable paper, identified himself
as Simon Massey, supplied simon.massey@stenograher.cloud, and asked to mimic
David Turner's paper layout. Compared the downloaded primary PDF's first two
pages, font information and dimensions. Changed the manuscript to IEEEtran,
US Letter, Times text, centered title/author, first-page contact notes,
Roman-numbered sections, title/revision header and top-right page numbers.
No affiliation or license was invented. The email is exactly as supplied.
PDF metadata also identifies Simon Massey.

paper/paper.tex is the editable source; paper/build.sh runs Tectonic from its
own directory and paper/README.md documents CLI use and installation. Tested
Tectonic 0.17.0 normally, then --only-cached from /tmp using the absolute script
path. No Codex, Lean, Rust or Showboat dependency is needed to rebuild the
paper. The first build cached the standard IEEE/font packages. Set T1 encoding
before class loading to avoid IEEEtran's initial font substitution warnings.
All eight rendered pages were inspected; the single-appendix title was
corrected for IEEEtran and the affected page checked again. No overfull boxes
or missing font warnings remain. The paper text stays at the rung-19 checkpoint;
rung 20 is explicitly a later conditional component awaiting integration.

Next: derive temporal recovery provenance and incarnation retention from the
actual crash-vector acquisition transitions. Preserve the user's forthcoming
manuscript edits: inspect the current diff before editing, and never regenerate
paper.tex from an older snapshot. The full recovery counterexample remains
unresolved and the end-to-end goal remains active.
