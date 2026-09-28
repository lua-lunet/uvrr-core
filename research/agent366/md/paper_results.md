                                                                                                                                    1




Diskless Viewstamped Replication with Unbounded
            Crash-Stop Reincarnation
        — Addendum: experimental results
                                                               Simon Massey



   Abstract—This document is an addendum to Diskless View-            count and elapsed time are recorded beside its log as
stamped Replication with Unbounded Crash-Stop Reincarnation;          a JSON digest in evidence/tlc/ within the Lean
the parent paper’s abstract and keywords are not repeated here.       project. Table I quotes the archived green runs; Table II
This companion reports only measured and checked ﬁndings,
each with its observed numbers, its evidence path in the repos-       the expected rejections. All logs and digests live under
itory and the command that reproduces it. It adds no method           formal/uvrr-lean/evidence/tlc/.
and changes nothing in the parent paper or in the proof-ladder           The reincarnation model’s invariant set is recorded
addendum; where the repository records no measurement, the            in formal/README.md: FrownChain (every com-
experiment’s contract is stated and its ﬁndings are marked to be      mitted quorum is a strict majority and consecu-
measured.
                                                                      tive committed eras overlap), EvictedNeverVoter,
                                                                      MassRule (at most mass one per node per com-
                   I. S COPE OF THIS REPORT                           mitted era), NoStandbyVote, RebornAfterFence,
   Every entry below is a result the repository evidence              SequenceCompletes, LeaderIsVoter, and the
records: a model-check log, a kernel-checked proof transcript,        liveness property Completes. The stall witness is the
a checker’s summarised counts, a dated audit, or a merge              intentionally failing run: with three deaths, two live voters
record. No experiment was executed for this report, and no            hold weight 2 of 4, no strict majority exists, and TLC reports
number is inferred, extrapolated or invented. Each ﬁnding             Completes violated by a trace that ends stuttering in the
carries its provenance path and, in the appendix, the command         stalled state — the recorded existence proof that the model
that reproduces it.                                                   stalls rather than committing without a majority.
   The evidence classes are: TLC model checking of the era               Two discrepancies between records are noted rather than hid-
and reincarnation design models (Section II); mutation and            den. First, formal/README.md records VrrCoreEras.
negative controls, in TLA+, in the Lean mutation harness              cfg as “837,204 distinct / 3,358,487 generated, depth 32,
and inside the proof-ladder rungs (Section III); the Showboat-        3m24s”, while the archived log records 837,204 distinct
veriﬁed rung transcripts 13–28 (Section IV); the independent          / 2,743,933 generated, depth 32, 137.7 s: the distinct-state
exhaustive checker’s counts for weighted reachability and             counts and depth agree, the generated totals and the generated
the fused batch (Section V); the dated Lean axiom audits              totals and elapsed times differ between the two records,
(Section VI); and the stress bench, the Zig twin and the              and this report quotes the archived log. Second, formal/
Maelstrom harness (Section VII).                                      README.md additionally records an era-model campaign
   One intended measurement has no recorded numbers any-              whose raw logs are not archived under evidence/tlc/: the
where in the repository and is marked to be measured through-         even-split four-node exhaustive run (green, 290,474 distinct /
out: the stress-bench lane ﬁgures beyond those preserved in           1,571,856 generated, depth 30), the ﬁxed-seed simulation runs
the merge record (Section VII). This report stands alone;             (10,000 depth-40 crash traces, 900,119 checked states; and
the parent paper [1] supplies the method and the design,              100,000 depth-60 traces, seed 1, 26,793,084 checked states,
and the proof-ladder addendum [2] supplies the mathematical           1 h56 m), the two intentionally false witness invariants (reached
statements that the transcripts below check.                          in 207 and 125 distinct states), the no-pivot-guard experiment
                                                                      (green, 837,204 distinct states), and the era mutations M2,
                                                                      M4, M5 (CommittedLogsAgree) and M7, M8 (refused
                      II. M ODEL CHECKING
                                                                      by the named startup gates, zero states). Their provenance is
   The TLA+ models are formal/VrrCoreEras.                            formal/README.md and their reproduction commands are
tla (the era-transition design model) and formal/                     the make tla family in the appendix.
VrrCoreReincarnation.tla (the Crash-Stop-Self-
Evict forced sequence). The archived campaign ran TLC                    III. M UTATION CONTROLS
2.19 (rev. 5a47802) from a jar pinned by SHA-256 in the
toolchain manifest (936a2620...), breadth-ﬁrst with two    Three independent control suites are recorded; each rejec-
workers on a 2 GB heap. Each run’s wall limit, worker   tion is a checked result, not a review observation.

  Author draft. Correspondence: simon.massey@stenographer.cloud.



                                        RESEARCH CHECKPOINT — 15 SEPTEMBER 2026 — 20260917-6048674
                                                                                                                                                       2



                                                                    TABLE I
  A RCHIVED TLC 2.19 RUNS , GREEN ; THE . C F G SUFFIX IS IMPLIED . S TATES ARE REPORTED AS DISTINCT / GENERATED . E LAPSED TIMES ARE THE
                                                      WRAPPER ’ S RECORDED WALL TIME .


 Conﬁguration                                  What was checked                                          Distinct    Generated    Depth     Elapsed
 VrrCoreEras                                   Era model, three nodes, exhaustive; no crash              837,204      2,743,933      32      137.7 s
 VrrCoreReincarnation2                         Two-node forced sequence; b crashes, reincar-                  35            162      11       0.76 s
                                               nates as d
 VrrCoreReincarnation3                         Three-node forced sequence, ﬁxed leader                        28           221       14       0.72 s
 VrrCoreReincarnationDouble                    Three nodes at double weight (all weights 2)                   28           221       14       0.71 s
 VrrCoreReincarnationFive                      Five-node forced sequence, ﬁxed leader; f rein-             3,406        53,078       20       1.07 s
                                               carnates as d
 VrrCoreReincarnationCrash                     Five nodes, leader crash forced between the two          1,253,444    26,308,632      31      108.5 s
                                               eras, one view change, two deaths
 VrrCoreReincarnationFiveViews                 Five nodes, one live-leader replacement plus un-        17,394,566   371,140,753      32    27 m 35 s
                                               rationed dead-leader replacement, all invariants
 VrrCoreReincarnationFiveCompletes             Liveness: Completes under FairSpec, leader                126,301      2,318,131      26       19.0 s
                                               may die and be replaced


                                                              TABLE II
A RCHIVED TLC 2.19 RUNS , EXPECTED REJECTIONS AND THE STALL WITNESS ; THE . C F G SUFFIX IS IMPLIED . M UTATION RUNS USE ONE WORKER ; THE
        VIOLATED INVARIANT IS THE CHECKED REJECTION . S TATES AT THE COUNTEREXAMPLE ARE REPORTED AS DISTINCT / GENERATED .


 Conﬁguration                             Defect checked                                                 Violated                             States
 VrrCoreErasM1                            Era 1 drops the transferred tail entry while reporting its     FrontiersOrdered             2,111 / 5,425
                                          frontier
 VrrCoreReincarnationM1                   Era 1 skips the decrement; era 2 leaves the victim at          MassRule                            9 / 26
                                          weight 2
 VrrCoreReincarnationM2                   One-era swap: the unfenced victim evicted and the joiner       MassRule                           11 / 22
                                          promoted in one era
 VrrCoreReincarnationM3                   The sequence runs without the fence (no bump)                  RebornAfterFence                  38 / 160
 VrrCoreReincarnationM4                   The sequence aborts after era 1; era 2 never proposed          SequenceCompletes                    8 / 19
 VrrCoreReincarnationM5                   Weight-0 identities’ replies counted in quorums                NoStandbyVote                        9 / 31
 VrrCoreReincarnationFiveM1               The M1 defect at ﬁve nodes                                     MassRule                          61 / 268
 VrrCoreReincarnationFiveM2               The M2 defect at ﬁve nodes                                     MassRule                            29 / 81
 VrrCoreReincarnationFiveStall            Intentionally failing witness: three deaths leave two live     Completes                 59,666 / 426,526
                                          voters with weight 2 of 4



A. The Lean mutation harness                                              C. Negative controls inside the rungs
   formal/uvrr-lean/check_mutations.py com-                        The rung transcripts record their own controls alongside
piles temporary mutants and requires Lean to reject each. Its the veriﬁed results: rung 13’s length-only selector loses a
recorded design and outcome (the proof-ladder addendum’s newer committed preﬁx and an omitted scalar length bound
appendix states the suite’s ﬁve rejections and the hole misranks reports; rung 14’s authentic out-of-order delivery
detection): an existential quorum checker (Q2.all replaced witnesses failure when the slot guard is bypassed; rung 15’s
by Q2.any); an unsafe four-node decision family (the concrete post-fence append breaks the historical report; rung
3-of-4 subsets replaced by [[3]]); an omitted next-slot 16’s delayed wrong-view prepare control checks the model
guard in NormalLog; an append after the view-change interface; rung 23 states the honest three-node degenerate
fence in ViewFence; and a cross-view prepare receipt negative (the forming quorums coincide, so no pivot exists
in LogProvenance. The suite additionally checks that a and none is exercised); rung 24’s casting-vote verdict splits
sorry is detected through sorryAx in the axiom query by boundary (degenerate across E0/E1, genuine pivot across
even when the compiler exits zero. Reproduction: python3 E1/E2); rung 26’s premature promotion of the replacement
check_mutations.py from formal/uvrr-lean.                       (claiming weight one in the intermediate era) is rejected by
                                                                Lean with an application type mismatch; and rung 28’s hand-
B. The TLC defect regressions                                   written swap_control kernel-checks that the equal-total
                                                                swap (1, 2, 1, 2) → (2, 1, 2, 1) makes {B, D} a majority at
   The nine expected rejections of Table II (one era-model mu-
                                                                E0 and a minority at E1 — the preservation hypothesis
tant, ﬁve three-node mutants, two ﬁve-node mutants, one stall
                                                                of the telescoping theorem is not free. Rung 27’s transcript
witness) each enable exactly one weakened perimeter through
                                                                additionally records that both Leanstral generation rounds for
the Defect constant and must exit nonzero on the named
                                                                Fuse.lean failed and that the accepted proof bodies were
invariant; the conﬁguration-to-obligation mapping is recorded
                                                                repaired locally: the successful result is the repaired ﬁle, not
in formal/README.md, and the make tla-mutations
                                                                an unassisted generator success.
lane requires both the failure and the exact violated-invariant
text. The era-model mutations M2, M4, M5, M7 and M8 are
recorded in formal/README.md as above.

                                      RESEARCH CHECKPOINT — 15 SEPTEMBER 2026 — 20260917-6048674
                                                                                                                                  3



                 IV. P ROOF - LADDER RUNGS                         is shortest; and the negative control walks the equal-total
   Table III records what each Showboat-veriﬁed rung               swap (1, 2, 1, 2) → (2, 1, 2, 1) through unit edits, recording
transcript actually checked. The transcripts live in formal/       responder masses 4, 4, 3, 3, 2 and quorum ﬂags true, true, false,
uvrr-lean/ladder/<NN>-<name>.md; the numbering                     false, false — the responding quorum is not preserved along
runs 13–16 and 22–28 (no transcripts 17–21 exist). Each            a legal unit-step path.
transcript embeds its Showboat identiﬁer, the module source           Reproduction for both checkers is in the appendix. These
and the observed check output; the replay command is               ﬁnite counts are computational evidence; the general claims
showboat verify ladder/<NN>-<name>.md from                         rest on the Lean proofs and the mathematical note recorded
formal/uvrr-lean after lake build.                                 in the same directory.
   One tooling control is recorded beside the rungs
(the proof-ladder addendum’s appendix): tests/test\                                     VI. A XIOM AUDITS
_leanstral\_driver.py has two ofﬂine regressions. A                  formal/uvrr-lean/check\_axioms.py queries
mock API forces two concurrent proof loops to ﬁnish writ-         every named theorem, deﬁnition and abbreviation in the
ing their candidates before either copies its result, exposing    compiled project and permits only propext, Quot.sound
the shared-ﬁle race deterministically, and a second assertion     and Classical.choice. Three dated audit outcomes are
checks that the prompt preserves project imports. Both failed     recorded in the repository:
on the original driver and pass with per-invocation scratch          1) The rung 26 transcript (11 September 2026): the build
directories and an import-preserving prompt.                             completed 24 jobs and the audit passed 355 declarations;
                                                                         direct queries record that ReincarnationSafety.
            V. E XHAUSTIVE CHECKER COVERAGE                              five_safe and ReincarnationAgreement.
   The     independent       Python   checker      (research/            five_leader_overlap depend only on the three
weighted-reachability/check.py, standard library                         standard axioms.
only) records its exact coverage counts in research/                 2)  The   proof-ladder addendum’s appendix: the extended
weighted-reachability/generated/summary.                                 build  completes 25 jobs and the audit covers 412 decla-
json, and the repository’s written record states the headline            rations.
totals (research/weighted-reachability/README.                       3) The rung 28 transcript (14 September 2026, recorded
md).                                                                     against the pre-addendum source): 27 Lake jobs, the
                                                                         audit covered 450 declarations with only the permitted
                                                                         axioms; Fuse.Telescope.telescope_pass and
A. Weighted reachability                                                 Fuse.Telescope.swap_control both report the
   Every weight vector and availability count with a pos-                three standard axioms.
itive live leader through ten existing identities: 310,007 No audit count is recorded at this addendum’s HEAD;
symmetry-representative proﬁles, covering 14,541,815 la- the three counts above are the dated records the repos-
belled weight/availability proﬁles, with zero failed checks. The itory holds. Reproduction: lake build then python3
maximum phantom weight added is 3 (matching the proved check_axioms.py from formal/uvrr-lean.
bound δ ≤ 3), and the maximum hub path is 19 unit edits.
Every source/target pair satisfying the premises through six            VII. S TRESS BENCH , Z IG TWIN AND M AELSTROM
identities: 774,159 representative pairs, covering 3,939,756                                    HARNESS
ordered labelled pairs, zero failed checks, every constructed
                                                                  A. The Maelstrom stress bench
path shortest in unit edits (maximum path 11). The per-identity
breakdown is preserved in generated/summary.json                     The bench’s contract is the make lane set in the Makefile:
and rendered in generated/report.html. The same                   the   Maelstrom lin-kv workload against ﬁve nodes (f =
summary records 250 independent cross-checks of the power-        2),  180-second     runs at rate 20 with nemesis interval 10,
set intersection rule, and separate negative controls that reject one   lane   per   fault proﬁle — test-clean (no faults),
an unsafe two-vote jump, a missing live majority, excessive       test-partition,          test-kill, and test-resurrect
phantom weight, and counting the abstract companion as a          (the  kill nemesis   against persisted nodes, where every kill re-
received quorum.                                                  opens   with no  stopped  quorum   and the boot bumps the identity,
                                                                  a T3 reincarnation). The linearizability verdict is Maelstrom’s
                                                                  own Knossos check; the adapter tests (tests/maelstrom\
B. The fused batch
                                                                  _adapter.rs) assert the node’s end of the transport against
   research/weighted-reachability/check_                          a scripted relay, and the recorded note is that the vendored
fuse.py records (generated/fuse-summary.json): harness is SIGKILL-only, so no clean-stop lane exists —
the three-voter fused schedule checks 2 boundaries and the T1–T2 clean-stop discipline is pinned by the adapter
64 cross-quorum majority pairs; the ﬁve-voter schedule test tests/maelstrom_adapter.rs case a_clean_
checks 6 boundaries and 5,760 pairs; over four identities shutoff_reopens_under_the_same_identity.
with weights in {0, 1, 2}, all 29,690 common-response                The recorded lane results are preserved in the merge
endpoint pairs admit a monotone unit-edit path (99,352 unit record of the stress bench’s restoration (commit b0bf2f9,
edges) that keeps a responding majority at every step and 14 September 2026, and its landed twin 44da48e): “Lanes:

                                    RESEARCH CHECKPOINT — 15 SEPTEMBER 2026 — 20260917-6048674
                                                                                                                                       4



                                                             TABLE III
 T HE RUNG TRANSCRIPTS 13–28: WHAT EACH VERIFIED RUN CHECKED , AS RECORDED IN THE TRANSCRIPT. A LL RESULTS ARE KERNEL - CHECKED BY
                                                  L EAN 4.33.1 WITHOUT M ATHLIB .

          Rung    Veriﬁed ﬁnding as recorded
          13      Whole-log selection by last-normal view: the executable selector returns a member of maximal (view, length)
                  rank, and quorum intersection with the stated temporal premises yields committed-preﬁx preservation; the
                  bounded scalar rank is proved equivalent to the TLA model’s.
          14      Normal-log message provenance: ﬁxed-view operational induction derives comparable report logs and retention
                  of replica preﬁxes; receivers enforce the next slot without any global agreement test.
          15      Historical whole-log reports across view fences: multi-view local executions derive the selector’s voter-history
                  premise; a node raises its ﬂoor before reporting and retains immutable vote and reply histories.
          16      Shared multi-view committed-log compatibility: the inductive invariant discharges same-view comparability
                  and installed-base extension; strong view induction proves committed-log compatibility for a ﬁxed conﬁgura-
                  tion without crashes.
          22      The reincarnation speciﬁcation ﬁxed as deﬁnitions: the durable identity contract, the incarnation bump, the
                  two-era forced weight sequence, the phase machine, the higher-identity-wins read rule and continuation
                  commitment. (The transcript carries a dated note that the governing boot rule is now the marker transition
                  machine; the transcript is left as recorded.)
          23      The casting vote instantiated on the reincarnation eras: at two nodes every E0/E1 quorum pair is decisive
                  at the leader and the boundary overlap holds through the casting-vote structure; the three-node case is the
                  recorded degenerate negative.
          24      The ﬁve-voter reincarnation rung: majority families computed (not axiomatised) at the concrete conﬁgurations;
                  era safety by the general one-unit discharge; the casting-vote verdict splits by boundary.
          25      Agreement across the forced sequence under every view schedule: sequence_p1 derives P1,
                  sequence_agreement instantiates the era-indexed agreement theorem, and the ﬁve-node instance carries
                  the E1/E2 leader-overlap pivot.
          26      The ﬁnal composition ReincarnationSafety.five_safe: agreement, replacement weights, eviction
                  from voting authority along the forced run, and identity non-reuse, from the existing history contract. The
                  composition was generator-supplied in two bounded rounds, kernel-checked, then shortened without changing
                  the statement. Observed: build 24 jobs; axiom audit PASS 355 declarations.
          27      Fuse: ﬁnite response-quorum telescoping: eligible_through, chosen_all, decide_all, and the
                  concrete eligibility calculations for the two-transition three-voter and six-transition ﬁve-voter schedules; ﬁrst-
                  quorum negative control. Generation failed twice; the accepted bodies were repaired locally.
          28      The Telescoping Theorem Fuse.Telescope.telescope_pass: for a three-node schedule with two
                  transitions under the fuse constraints, the ﬁrst transition passing telescopes the remaining slots. The
                  swap_control negative control is hand-written; the theorem’s proof body is generator-drafted and kernel-
                  checked unchanged. Observed at integration: 27 Lake jobs; axiom audit 450 declarations; the Rust corpus
                  green, 33 suites, no inline test modules under src/.



test-clean 0.7407, test-partition 0.2415/0.2710, test-kill 0.5813, C. The Rust corpus gate
test-resurrect 19 dirty T3 bumps — all :valid? :true.”               The rung 28 transcript records the corpus gate at its date:
Every lane’s linearizability check passed; the resurrection lane cargo test -features maelstrom green, 33 suites,
recorded 19 dirty T3 identity bumps across its kills. The and no inline test modules under src/. The repository
decimal ﬁgures are recorded in the merge message without gate (make check) adds the format check, clippy at -D
further elaboration, and the raw Maelstrom run artefacts are warnings, and the cli-featured run.
not committed (the store and state directories are gitignored);
the merge message is the recorded provenance. Lane ﬁgures                      VIII. R EPRODUCTION COMMANDS
beyond that record are to be measured.                               From the repository root unless stated. The TLC lanes
                                                                   use the hash-pinned jar recorded in formal/uvrr-lean/
B. The Zig twin                                                    evidence/tlc/toolchain.json.
                                                                        make check
   The vendored TigerBeetle 0.17.9 storage stack (zig/, ev-             make test-clean
ery patched line recorded in zig/PATCH\_MANIFEST.md)                    make test-partition
drives the marker transition machine over real direct-IO                make test-kill
                                                                        make test-resurrect
paths, compiled with the pinned Zig 0.14.1. The recorded
outcome (the same merge records): 19 of 19 Zig tests                    zig test -lc -dep stdx
                                                                            -Mroot=zig/root.zig
pass with the pinned toolchain, including the exhaustive               -Mstdx=zig/stdx/stdx.zig
44 marker-assignment test over the pure machine noted                   python3
in zig/README.md. The demo consumer is examples/                        research/weighted-reachability/check.py
uvrr-reincarnation/ (feature uvrr), which runs the                          -max-n 10 -pair-max-n 6
                                                                        python3
T1/T2/T3 transitions and membership replay over the vendored            research/weighted-reachability/check_fuse.py
IO.                                                                     cd formal/uvrr-lean && lake build


                                      RESEARCH CHECKPOINT — 15 SEPTEMBER 2026 — 20260917-6048674
                                                                                                         5



         && python3 check_axioms.py
      python3 check_mutations.py
      for f in ladder/[0-9][0-9]-*.md; do
         showboat verify "$f" || exit; done
      make tla   # era model, Docker
      make tla-mutations
      make tla-local TLA2TOOLS_JAR=<tla2tools.jar>
      java -XX:+UseParallelGC -Xmx2g
         -jar /path/to/tla2tools.jar -workers 2
         -config <config>.cfg
      VrrCoreReincarnation.tla
   The spellcheck gate for this document is the reposi-
tory’s: paper/spellcheck.sh paper_results.tex
(Aspell, en_GB).

                              R EFERENCES
[1] S. Massey, “Diskless Viewstamped Replication with Unbounded Crash-
    Stop Reincarnation,” manuscript, 2026. [Online]. Available: https://github.
    com/lua-lunet/uvrr-core
[2] S. Massey, “Diskless Viewstamped Replication with Unbounded Crash-
    Stop Reincarnation — Addendum: proof ladder, weighted reconﬁguration
    and the operator solver,” manuscript, 2026. [Online]. Available: https:
    //github.com/lua-lunet/uvrr-core
[3] S. Massey, “uVRR code and proof artifact,” 2026. [Online]. Available:
    https://github.com/lua-lunet/uvrr-core




                                            RESEARCH CHECKPOINT — 15 SEPTEMBER 2026 — 20260917-6048674
