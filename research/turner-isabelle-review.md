# Turner's informal Isabelle/HOL consistency proofs and the Lean ladder — relationship review

*Review artifact, 8 September 2026. Read-only review of extracted sources; no production code touched.*

**Subject of review.** David C. Turner, "Unbounded Pipelining in Dynamically Reconfigurable Paxos
Clusters," revision 1A9DBA37, 14 August 2017 (10 pages; extracted text, summary and manifest under
`formal/uvrr-lean/paper/turner/`). © 2016-7 Tracsis plc, CC BY-SA 4.0. All statements of his
results below are his; the extracted text is the pinned source of record.

**The question.** Turner writes (p. 2): "The appendices are informal versions of formal proofs
performed using the Isabelle/HOL proof assistant [6]." Does our Lean ladder work (1) build upon,
(2) can be expressed in, or (3) support that sort of work — and should we do more of it?

**Method.** Page-by-page read of `text.txt` (page boundaries verified against the running
revision-marker stream), checked against `summary.md` and `MANIFEST.md`, then each of his proofs
located in the Lean sources by name and statement. The correspondence is not conjectural: the
ladder's theorem names are his lemma numbers, and the rung transcripts state the correspondence
explicitly (e.g. the rung-9 transcript claims "UPaxos Appendix A Lemmas 2–4 and Corollary 5").

## Page-by-page findings: where his proofs live and what we already kernel-check

All page references are to Turner's revision 1A9DBA37. Lean references are to
`formal/uvrr-lean/UVRR/` modules; "kernel-checked" means built by `lake build` with the axiom
audit admitting only `propext`, `Quot.sound`, `Classical.choice`.

- **p. 2** — the sentence that poses this review's question. His appendices A–C (pp. 9–10) are
  the informal residue of unpublished Isabelle/HOL efforts; the formal scripts themselves are not
  in the paper, so a line-by-line cross-check against his actual Isabelle theories is not
  possible from the paper alone. Everything checkable is checked below.

- **p. 3, fig. 1 (invariants S1–S6)** — the single-instance history conditions his Synod
  consistency rests on. Encoded as `Synod.S1`–`Synod.S6` (rung 3), with the weakened per-phase
  quorum condition S1 (`QI(b1) ⌢ QII(b2)` only for the relevant ballot pairs, his §III-B on
  p. 4) rather than global majority intersection.

- **p. 9, Appendix B, Lemma 6** — "If accepted(a, b2), promised(a, b1; b3) and b2 ≺ b1 then
  b2 ⪯ b3": a forced promise reports the greatest accepted ballot below b1. Kernel-checked as
  `Synod.lemma6` (rung 3) under the same hypotheses (order + invariants).

- **pp. 9–10, Appendix B, Lemma 7** — "If chosen(b2), proposed(b1) and b2 ≺ b1 then v(b1) = v(b2)",
  by minimal-counterexample well-founded induction on b1. Kernel-checked as `Synod.lemma7`
  (rung 3); the paper's proof of it follows Turner's Appendix B (`paper/paper.tex` says so with
  attribution).

- **p. 10, Appendix B, Theorem 8** — Synod consistency: two chosen ballots carry the same value.
  Kernel-checked as `Synod.theorem8` (rung 3), conditional on S1–S6, a well-founded strict total
  ballot order, and nonempty phase-II decision quorums. This is the complete content of his
  Appendix B: rung 3 kernel-checks all three of its results.

- **p. 4 (§IV-A) and p. 5, fig. 2 (invariants P1–P7)** — the era-indexed Paxos conditions:
  configuration chain `QIIe ⌢ QIe ⌢ QIIe+1`, nondecreasing era functions on instances and
  ballots, and the promise/choice era discipline. Encoded as `Eras.P1`, `P23`, `P4`, `P5`, `P6`,
  `P7` with `EraMono`/`EraLe` (rung 4), redrawing his fig. 2 in our notation.

- **p. 10, Appendix C, Lemma 9** — for b1 ≻ b2, `proposed_i(b1)` and `chosen_i(b2)` force
  `QI_{e(b1)} ⌢ QII_{e(i)}`: the era discipline pins `e(i) ∈ {e(b1), e(b1)+1}` so P1 applies.
  Kernel-checked as `Eras.lemma9` (rung 4). Note our rung 11 (`MultiPromise.lean`) goes beyond
  the informal text here: his Lemma 9 proof cites the multi-promise alternative
  `promised≥i′(a,b1)` without expanding it; rung 11 derives `EraLe` from the full
  suffix/point promise bounds of his fig. 2 and re-proves agreement for that expanded history
  (`MultiPromise.eraLe`, `MultiPromise.agreement`), with `MultiPromise.chosen_frontier`
  discharging his P7 frontier condition `i ≤ imax`.

- **p. 10, Appendix C, Theorem 10** — Paxos consistency per instance, by translating instance i's
  Paxos history into a Synod history (his displayed substitution table) and applying Theorem 8.
  Kernel-checked as `Eras.theorem10` (rung 4) via `Eras.toSynod`/`toSynod_inv` — the same
  translation, constructed in Lean.

- **p. 9, Appendix A, Lemma 2** — weight functions w, w′ with positive integers k, k′ and
  Σ|k′w′ − kw| ≤ 1 imply M(w) ⌢ M(w′). Kernel-checked as `WeightedGeneral.scaled_overlap`
  (rung 9), for arbitrary finite support and without his integrality-by-hand argument (the Lean
  proof is a disjoint-mass bound plus `omega`).

- **p. 9, Appendix A, Lemma 3** — the same for Σ|w′ − w| ≤ 1 (k = k′ = 1). Kernel-checked as
  `WeightedGeneral.unit_change_overlap` (rung 9).

- **p. 9, Appendix A, Lemma 4** — constant-factor rescaling preserves intersection. Kernel-checked
  as `WeightedGeneral.scaled_equal_overlap` (rung 9).

- **p. 9, Appendix A, Corollary 5** — M(w) ⌢ M(w). Kernel-checked as `WeightedGeneral.self_overlap`
  (rung 9). Rung 9 adds what his appendix lacks: `WeightedGeneral.distance_two_counterexample`,
  a sharpness witness that the uniform distance bound ≤ 1 cannot be relaxed to ≤ 2. Turner's
  examples (pp. 7–8: the w1...3 / w1...5 disjoint-majority pair) gesture at this; rung 9 proves
  it, and rung 7 (`Weights.plus_two_breaks`) gives a concrete failing schedule.

- **p. 5 (§IV-C), ballot space** — liveness forces ballots that are large yet era-bounded; he
  requires B = N × N × A lexicographic with e(⟨e, n, a⟩) = e. Rung 2 (`LexBallot.lean`)
  kernel-checks the well-foundedness, transitivity, totality and era monotonicity of the
  lexicographic order (`bLt_wf`, `bLt_era_mono`, ...) and instantiates Theorem 10 on it
  (`Paxos.theorem10_lex`). Ours is an era/round pair without his owner component, which we do
  not need for the safety ladder.

- **p. 5, Theorem 1 (liveness)** — eventual choice for every instance given an eventually unique
  nonfaulty prepare-emitter and eventually nonempty value supply. **Not kernel-checked.** No
  rung proves liveness; the ladder is a safety ladder and states so.

- **pp. 5–6 (§IV-D) and fig. 3, the casting vote** — the leader's ability to complete phase I in
  the new era without blocking the old one when quorums q ∈ QII_e and q′ ∈ QI_{e+1} meet in
  {ℓ} alone. In his paper this is operational prose, not a stated or proved lemma. Rung 6
  (`CastingVote.lean`) kernel-checks the underlying guard property generally
  (`CastingVote.noninterference`, `guard_preserved`, `completes_phase1`, plus a fig.-3-shaped
  instance `example3`); rung 23 re-instantiates it on the reincarnation eras
  (`CastingVoteReincarnation.lean`). The full IV-D reconfiguration *procedure* as an operational
  end-to-end statement is not kernel-checked.

- **p. 3 (§III-A), value-function elision** — v as an insert-only convergent replicated set kept
  out of the consensus messages. This is an implementation construct; he notes himself that
  treating v as fixed is what makes the consistency proof tractable. Our ladder follows that
  choice; nothing here is a proof gap.

**Score for his consistency content:** every lemma and theorem in his three appendices — Lemma 6,
Lemma 7, Theorem 8 (Appendix B); Lemma 9, Theorem 10 (Appendix C); Lemmas 2–4 and Corollary 5
(Appendix A) — is already kernel-checked in this repository, most under his own names, in rungs
2, 3, 4, 9 and 11.

## Where our rungs go beyond his paper

- **Rung 5 (`Counterexample.lean`)** — a machine-checked counterexample showing that removing the
  cross-era overlap admits disagreement under the remaining conditions: a necessity result for
  his P1 that his paper asserts only through the positive proofs.
- **Rung 9's sharpness witness** — the distance-two counterexample above; his appendix proves the
  ≤ 1 bound but does not isolate it as exact.
- **Rung 11** — the multi-promise expansion of his fig. 2 conditions (see Lemma 9 above).
- **Rungs 13–20 (`ViewSelection`, `NormalLog`, `ViewFence`, `LogProvenance`, `RecoveryFence`,
  `CrashVector`, `AcquisitionOrder`, `RecoveryAcquire`)** — whole-log view selection, committed
  prefix preservation across views, and the crash/recovery fence-and-acquire machinery. His paper
  contains no recovery content at all: liveness is relative to failure assumptions and no
  recovery protocol is stated or proved.
- **Rungs 22–23 (`Reincarnation`, `CastingVoteReincarnation`)** — Crash-Stop-Self-Evict:
  crash-restart converted to crash-stop-reincarnation, with the casting vote instantiated on the
  reincarnation eras. Entirely absent from his paper.
- **Rung 12 (`NegativeControls.lean`)** — independent witnesses that each guard is load-bearing,
  in the same spirit as rung 5.

## The value-add frame

It is worth being blunt about where the mathematical difficulty is, because the Director's
question invites an unfavorable comparison with his appendices.

- **The pigeonhole part is trivial and has a name.** Proving that overlapping integer-weight
  quorum sets differing by at most one unit of total mass intersect is the classical
  majority-intersection argument: two subsets of a weighted finite set whose combined mass
  exceeds the total must overlap, and an L1 perturbation of one unit cannot break that. His
  Appendix A is exactly this, and rung 9 re-proves it mechanically with `omega` after a
  one-line disjoint-mass bound. This is not where consensus bugs live, and kernel-checking it
  — his or ours — is table stakes, not contribution.

- **The load-bearing work is leader-overlap across two eras.** The same value must be
  identified when, across a crash and a view change, a recovery leader finds that value
  accepted under different ballot numbers issued by different leaders in different eras. The
  Director covers precisely this problem in the Trex posts
  (simbo1905, "slash dev slash null," Paxos category page 2,
  https://simbo1905.wordpress.com/category/paxos/page/2/ — the 2015–2016 Trex replication-engine
  series, including "Paxos Dynamic Cluster Membership," 30 July 2015, and the TRex posts
  built on it). Turner's era machinery defers this to the leader: his casting vote keeps one
  leader across the era boundary so that the identifying node is trivially the same. Our rungs
  13–20 and 22–23 are where the two-era identification is proved for VRR: view selection
  compares reports across views by rank, the recovery fence retains a known fence through
  arbitrary crash/recovery episodes, and acquisition certificates are crash-consistent for
  exactly their request and incarnation. That machinery does not exist in his paper.

- **Diskless strong consistency throughout VRR view changes is our original result to present.**
  VRR-2012's diskless mode leaves recovery correctness exposed; our fence/acquisition rungs
  carry strong consistency through view changes with no durable per-ballot state, and
  Crash-Restart is converted to Crash-Stop-Reincarnation: a node whose durable state records an
  unflushed session reopens as a new identity at weight zero.

- **The FAST'18 amnesia flaw is a failed condition removed by design, not proved away.**
  Alagappan, Ganesan, Lee, Albarghouthi, Chidambaram, Arpaci-Dusseau and Arpaci-Dusseau,
  "Protocol-Aware Recovery for Consensus-Based Storage," USENIX FAST 2018, showed that
  same-identity recovery after storage loss (amnesia) breaks the promise/acceptance invariants
  of real Paxos systems — in our notation, a recovered node violates S2/S3 (his p. 3, fig. 1)
  while still passing as itself. The classic Isabelle/HOL-style response would be to add a
  recovery protocol and prove the invariants re-established: a *proved-away* flaw. uVRR instead
  makes the flaw's precondition unrepresentable — no node ever resumes a prior identity — so
  the corresponding condition never appears in any theorem type and there is nothing to prove.
  This maps directly onto Turner's informal-proof style: where his appendices add hypotheses to
  keep histories well-formed, reincarnation removes the states that would need them. The
  negative rungs (5, 12) are the machine-checked form of the same discipline: each guard is
  shown load-bearing by a witness, not by assertion.

## Verdicts on the three relationships

1. **Builds upon — yes, and already delivered.** The ladder's rungs 2–4, 9 and 11 restate his
   S1–S6 and P1–P7 and kernel-check every result in his appendices A–C, several under his own
   theorem names; `paper/paper.tex` attributes the reasoning to him explicitly. The build-upon
   relationship is not a proposal; it is the existing state of the repository.

2. **Can be expressed in — yes, for his safety content; liveness is expressible but unproven.**
   His entire safety apparatus is not merely expressible but already expressed in our
   era-indexed history style. The only constructs of his paper not yet in the ladder are
   Theorem 1 (liveness, p. 5) and the §IV-D reconfiguration procedure (pp. 5–6) as an
   operational statement. Both are expressible — liveness as a conditional eventual-choice
   statement under his exact hypotheses, the procedure as an operational transition system with
   the rung-4 conditions as invariants — but neither extends the safety result.

3. **Supports — yes, and in a stronger form than his.** The ladder's style — conditional history
   invariants, per-instance reduction, machine-checked necessity counterexamples, negative
   controls, and an axiom audit restricted to the three standard axioms — is the same genre of
   work his Isabelle/HOL appendices represent, carried further: it proves not only that the
   conditions suffice but (rungs 5, 9, 12) that they are individually needed. Any future
   Isabelle/HOL-style formalization in this space can be checked against our rungs as a
   reference, and conversely his unpublished Isabelle scripts, if they ever surface, would be
   worth a cross-read against rungs 3, 4 and 9 — that cross-read, not a re-derivation, is the
   only outstanding item his formalization could contribute to.

## Recommendation

**Do not commission further restatement of Turner's material.** His remaining informal
constructs are Theorem 1 and the §IV-D procedure; restating them for uVRR would add no safety
coverage (the ladder's open items are integration work — whole-log view selection, repeated
operational reconfiguration, client linearizability — as the README already records) and would
duplicate, in a different prover, results we already kernel-check. A liveness rung under his
exact hypotheses is the only self-contained candidate and is estimated at two to four weeks of
Lean effort for a statement that no protocol decision depends on; recommend against. The paper's
value-add frame should state plainly: his Appendix A is the trivial pigeonhole argument and is
done; the leader-overlap two-era identification (Trex posts, cited above), diskless strong
consistency through VRR view changes, and the FAST'18 amnesia class removed by design are ours,
and they are where the rungs 13–23 effort went.

**Attribution.** David C. Turner is the author of the reviewed paper and of the consistency
argument the ladder retells: the single-instance conditions and agreement proof follow his
Appendix B, the era-indexed reduction his Appendix C, and the weighted-intersection results his
Appendix A, in each case with attribution in `paper/paper.tex` and in the rung transcripts. The
Trex framing of the two-era problem is the Director's (simbo1905's blog, URL above). The FAST'18
findings are Alagappan et al.'s. The Lean restatements and everything beyond them are this
repository's work.
