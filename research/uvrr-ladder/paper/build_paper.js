const fs = require('fs');
const { Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell, HeadingLevel,
  AlignmentType, LevelFormat, BorderStyle, WidthType, ShadingType, TableOfContents,
  Footer, PageNumber, ExternalHyperlink } = require('docx');

const outcome = fs.existsSync('leanstral-outcome.txt') ? fs.readFileSync('leanstral-outcome.txt','utf8').trim().split('\n\n') : ['Outcome pending.'];

const P = (t, o={}) => new Paragraph({ children: [new TextRun({ text: t, ...o })], spacing: { after: 120 } });
const Rich = (runs) => new Paragraph({ children: runs.map(r => typeof r === 'string' ? new TextRun(r) : new TextRun(r)), spacing: { after: 120 } });
const H1 = t => new Paragraph({ heading: HeadingLevel.HEADING_1, children: [new TextRun(t)] });
const H2 = t => new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun(t)] });
const Code = t => new Paragraph({ children: [new TextRun({ text: t, font: 'Courier New', size: 18 })], spacing: { after: 0 }, shading: { fill: 'F2F2F2', type: ShadingType.CLEAR } });
const CodeBlock = s => s.split('\n').map(Code).concat([new Paragraph({ spacing: { after: 120 } })]);
const B = t => new Paragraph({ numbering: { reference: 'bullets', level: 0 }, children: [new TextRun(t)], spacing: { after: 60 } });
const Bm = runs => new Paragraph({ numbering: { reference: 'bullets', level: 0 }, children: runs.map(r => typeof r === 'string' ? new TextRun(r) : new TextRun(r)), spacing: { after: 60 } });
const N = t => new Paragraph({ numbering: { reference: 'numbers', level: 0 }, children: [new TextRun(t)], spacing: { after: 60 } });
const border = { style: BorderStyle.SINGLE, size: 1, color: 'BBBBBB' };
const borders = { top: border, bottom: border, left: border, right: border };
function table(widths, rows) {
  const total = widths.reduce((a,b)=>a+b,0);
  return new Table({ width: { size: total, type: WidthType.DXA }, columnWidths: widths,
    rows: rows.map((r, i) => new TableRow({ children: r.map((c, j) => new TableCell({ borders,
      width: { size: widths[j], type: WidthType.DXA },
      shading: i === 0 ? { fill: 'D9E2F3', type: ShadingType.CLEAR } : undefined,
      margins: { top: 60, bottom: 60, left: 100, right: 100 },
      children: [new Paragraph({ children: [new TextRun({ text: c, bold: i === 0, size: 20 })] })] })) })) });
}
const gap = () => new Paragraph({ spacing: { after: 120 } });

const children = [
  new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 200 }, children: [new TextRun({ text: 'Reconfiguring a Strongly Consistent Log Without Stopping: Machine-Checked Safety of Overlapping-Quorum Reconfiguration for uVRR', bold: true, size: 36 })] }),
  new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 80 }, children: [new TextRun({ text: 'A Lean 4 proof ladder over David C. Turner’s UPaxos invariants, with a necessity counterexample', italics: true, size: 24 })] }),
  new Paragraph({ alignment: AlignmentType.CENTER, spacing: { after: 300 }, children: [new TextRun({ text: 'vrr-core project, branch uVRR — 6 September 2026 — artefacts: formal/uvrr-lean/', size: 20 })] }),

  H1('Abstract'),
  P('We give machine-checked (Lean 4.33.1, core library only) safety results for non-stop cluster reconfiguration of a strongly consistent replicated log, the design target of uVRR (Unbounded Viewstamped Replication Revisited). Following Turner’s UPaxos paper we formalise the Synod invariants S1–S6 and prove agreement (Theorem 8); we then index configurations by era, take Turner’s safety equation QII_e ⌐ QI_e ⌐ QII_{e+1} as the sole assumption on configurations, and prove Lemma 9 and Theorem 10: no instance can choose two values across a reconfiguration. We add a result the paper does not state: a two-node counterexample in which every invariant except the cross-era overlap holds and two different values are chosen, so the cross-era overlap is necessary, not merely sufficient. We formalise the leader’s casting vote as non-interference and phase-I completion lemmas and show agreement needs no further assumption. Concrete schedules (the 3-zone server hot-swap with plain and weighted majorities) are kernel-checked by decision procedures, each with a negative control. Every rung is packaged as an executable markdown document whose commands a reader can extract and re-run. We do not prove liveness and we do not re-prove VRR itself.'),

  H1('1. Problem and scope'),
  P('The uVRR programme asks whether a VRR-2012 style protocol can support non-stop cluster reconfiguration (adding or removing nodes, or changing voting weights) with a proof of correctness. The blog posts that motivate the work reduce the question to quorum overlaps between consecutive configurations plus a leader casting vote that lets the transition to the new era happen without stalling the pipeline. This report is deliberately narrow:'),
  B('Safety only. Strong consistency is the product requirement; liveness is explicitly out of scope, and no liveness claim is made anywhere.'),
  B('We do not re-prove VRR. We prove that reconfiguration with overlapping quorums works, and we exhibit a counterexample showing it does not work when the quorums do not overlap.'),
  B('Baby steps. Each rung is a small, independently compilable Lean module with a matching executable document. Nothing on a higher rung is claimed unless the lower rungs are in the bag.'),
  B('Honesty about assumptions. Every hypothesis is an explicit structure field; the axiom footprint of every theorem is printed in the evidence documents.'),

  H1('2. Prior art taken into account'),
  Bm([{ text: 'Turner, D. C. ', bold: true }, '“Unbounded Pipelining in Dynamically Reconfigurable Paxos Clusters” (UPaxos). Source of the invariants S1–S6 (Fig. 1), P1–P7 (Fig. 2), Lemma 6–7, Theorem 8, Lemma 9, Theorem 10, the weighted-majority Lemma 2–4 and the casting-vote construction (§V). Our Lean statements follow the paper’s numbering; the PDF was fetched from tessanddave.com and text-extracted for this work since no copy was on disk.']),
  Bm([{ text: 'Howard, Malkhi, Spiegelman, ', bold: true }, '“Flexible Paxos” (2016). The intra-era result QI ⌐ QII; UPaxos generalises it across eras.']),
  Bm([{ text: 'Liskov, Cowling, ', bold: true }, '“Viewstamped Replication Revisited” (MIT-CSAIL-TR-2012-021). The base protocol; the correspondence to our ballots/instances is given in §5. The earlier fact-check in this project established that VRR-2012 contains no automated verification results.']),
  Bm([{ text: 'Lamport, ', bold: true }, '“Paxos Made Simple” and “Reconfiguring a State Machine”: the classical reconfiguration-as-a-command approach that UPaxos improves on by removing the pipeline stall.']),
  Bm([{ text: 'simbo1905 blog posts: ', bold: true }, '“UPaxos: Unbounded Paxos Reconfigurations” (2016), “Paxos Voting Weights” (2017), “One More Frown Please? (UPaxos Quorum Overlaps)” (2020), “Unbounded Viewstamped Replication Revisited?” (2026). The 2020 post records Turner’s correction that QI_e ⌐ QI_{e+1} is not required for safety; our proof indeed never assumes it. The 2017 post supplies the 3-zone hot-swap tables checked in Rungs 1 and 7.']),
  Bm([{ text: 'TigerBeetle: ', bold: true }, 'the only production VRR adopter identified, without dynamic membership (per the 2026 post); this is the gap uVRR targets.']),
  Bm([{ text: 'Tooling survey (this project, 5 Sept 2026): ', bold: true }, 'Veil (CAV 2025), LeanLTL (ITP 2025), lean-auto (2025), TLA+/TLC/TLAPS. Kick-the-tires outcomes: TLC hello-world green; Lean 4.33.1 installed; omega/aesop/lean-auto+Duper solve stated theorems from the command line; Veil not exercised (pins Lean 4.32.0 plus Mathlib). This report uses none of those frameworks: the proofs are plain Lean core, which keeps the trusted base at the Lean kernel.']),
  Bm([{ text: 'This repository’s TLA+ models: ', bold: true }, 'formal/VrrCore.tla (base protocol) and formal/VrrCoreEras.tla (one era transition, weights differ by era, exhaustive gate over quorum families). The Lean ladder is complementary: TLC explores finite instances of the design; Lean proves the quorum-overlap theorem for all node sets, eras and ballots.']),
  Bm([{ text: 'Earlier scratch baby step (5 Sept 2026, .tmp/lean-upaxos/Upaxos.lean): ', bold: true }, 'encoded QI_{e+1} ⌐ QII_e as the third overlap. That is not the paper’s P1 (whose third frown is the intra-era QII_{e+1} ⌐ QI_{e+1}); Rung 1 corrects it.']),

  H1('3. The mathematical setting'),
  P('Nodes A, ballots B, values V are arbitrary types. A node-set is a predicate A → Prop; a quorum system Q is a set of node-sets. Turner’s frown is'),
  ...CodeBlock('def Frown (Q1 Q2 : QSys A) : Prop :=\n  ∀ q1 q2, Q1 q1 → Q2 q2 → ∃ a, q1 a ∧ q2 a'),
  P('A ballot carries an era e(b) in its most significant position; an instance (slot) i carries an era e(i). Phase I of ballot b uses a quorum from QI_{e(b)}; phase II of instance i uses a quorum from QII_{e(i)}. Turner’s era rule is e(b) ≤ e(i) ≤ e(b)+1: a ballot may fix an instance in its own era or the next. The only assumption about configurations is P1:'),
  ...CodeBlock('def P1 : Prop := ∀ e, Frown (P.QII e) (P.QI e) ∧ Frown (P.QI e) (P.QII (e+1))'),

  H1('4. The ladder'),
  P('Every rung is a module UVRR/<Name>.lean and an executable document ladder/NN-<name>.md. Table 1 summarises; the subsections state each result precisely.'),
  table([700, 2300, 4560, 1800], [
    ['Rung', 'Module', 'Result', 'Axioms'],
    ['1', 'Structure.lean', 'frownB ⇔ Frown; 3-zone hot-swap satisfies P1 for all eras; wholesale replacement fails the cross-era frown', 'propext, Quot.sound'],
    ['2', 'LexBallot.lean', '(era, round) lexicographic ballots: well-founded strict total order; respects eras', 'std + Classical.choice'],
    ['3', 'Synod.lean', 'S1–S6 ⇒ Lemma 6, Lemma 7, Theorem 8 (agreement)', 'none'],
    ['4', 'Eras.lean', 'P1 + era rules ⇒ Lemma 9, Theorem 10 (agreement across reconfiguration)', 'propext, Quot.sound'],
    ['5', 'Counterexample.lean', 'cross-era frown dropped, all else holds, two values chosen: necessity', 'propext, Quot.sound'],
    ['6', 'CastingVote.lean', 'non-interference, guard preservation, phase-I completion; agreement inherited', 'std + Classical.choice'],
    ['7', 'Weights.lean', 'blog’s 7-row weighted schedule satisfies the frowns; +2 on one node breaks it', 'propext, Quot.sound'],
  ]),
  gap(),
  P('“std” means propext and Quot.sound. Classical.choice enters only through omega and by_cases. No sorry, no native_decide, no custom axioms anywhere.', { italics: true, size: 20 }),

  H2('4.1 Rung 1 — Decidable overlaps and the 3-zone hot-swap'),
  P('For explicitly listed quorums we define a Bool checker and prove it equivalent to the Prop-level frown on the induced quorum systems:'),
  ...CodeBlock('def frownB (Q1 Q2 : List (List Nat)) : Bool :=\n  Q1.all fun l1 => Q2.all fun l2 => l1.any fun a => decide (a ∈ l2)\ntheorem frownB_iff : frownB Q1 Q2 = true ↔ Frown (ofList Q1) (ofList Q2)'),
  P('The hot-swap schedule is era 0 = majorities of {W,X,Y}, era 1 = majorities of {W,X,Y,Z}, era ≥ 2 = majorities of {W,X,Z}. Theorem hotswap_p1 proves P1 for every era, closed by decide for the two transition eras and by rewriting for the stable tail. The negative control wholesale_cross_fails replaces {0,1,2} by {3,4,5} in one step: both intra-era frowns hold, the cross-era frown fails, P1 is violated.'),

  H2('4.2 Rung 2 — Era-tagged ballots'),
  P('Ballot := Nat × Nat with Prod.Lex order. We prove well-foundedness (from Lean core’s Prod.lex), transitivity, irreflexivity, totality, and bLt_era_mono: b2 ≺ b1 ⇒ e(b2) ≤ e(b1), the property Lemma 9 needs. theorem10_lex specialises Theorem 10 to these ballots so only protocol invariants remain as hypotheses.'),

  H2('4.3 Rung 3 — Synod consistency (Appendix B)'),
  P('The structure Synod A B V records lt, QI, QII, promised(a,b), promised(a,b;b′), proposed(b), accepted(a,b), chosen(b) and v. The invariants are fields of Synod.Inv:'),
  ...CodeBlock('S1 : ∀ b1 b2, proposed b1 → chosen b2 → lt b2 b1 → Frown (QI b1) (QII b2)\nS2 : ∀ a b b′, promised a b → lt b′ b → ¬ accepted a b′\nS3 : promised(a,b;b′) → b′ ≺ b ∧ accepted a b′ ∧ nothing accepted strictly between\nS4 : proposed b → ∃ q ∈ QI b, every a ∈ q promised (free or with last vote) ∧\n     (some last vote reported → ∃ maximal bm reported, v b = v bm)\nS5 : accepted a b → proposed b\nS6 : chosen b → ∃ q ∈ QII b, ∀ a ∈ q, accepted a b'),
  P('Lemma 6 (b2 ≺ b1, accepted(a,b2), promised(a,b1;b3) ⇒ b2 ≼ b3) is immediate from S3. Lemma 7 (chosen(b2), proposed(b1), b2 ≺ b1 ⇒ v(b1) = v(b2)) is proved by well-founded induction on b1: S1 yields a node a in both quorums; S2 rules out a free promise from a, so a reported a last vote; the maximal reported vote bm satisfies b2 ≼ bm ≺ b1; either bm = b2 or the induction hypothesis applies to bm. Theorem 8 (agreement) follows by trichotomy, using nonempty phase-II quorums to obtain proposed(b) from chosen(b). Theorem 8 depends on no axioms.'),
  P('Encoding decision: the paper’s v(b) = v(max P) is stated as the existence of a maximal element of the reported last votes with v(b) equal to its value. This is what a leader computes from a finite set of responses, and it avoids importing finiteness of quorums into the abstract theorem.'),

  H2('4.4 Rung 4 — Era-indexed Paxos (Appendix C)'),
  P('The structure Paxos A B V adds era : B → Nat, eInst : Nat → Nat, per-era QI/QII and per-instance message predicates. Its invariants are P1, EraMono (order respects eras), EraLe (proposed_i(b) ⇒ e(b) ≤ e(i)), P23, P4, P5 (phase-I quorum from QI_{e(b)}), P6, and P7 (chosen_i(b) ⇒ e(i) ≤ e(b)+1 and a QII_{e(i)} quorum accepted).'),
  ...CodeBlock('theorem lemma9 (hI : P.Inv) (hpr : P.proposed i b1) (hch : P.chosen i b2) (hlt : P.lt b2 b1) :\n    Frown (P.QI (P.era b1)) (P.QII (P.eInst i))\ntheorem theorem10 (hI : P.Inv) (ho : order side conditions) (hne : QII quorums nonempty)\n    (h1 : P.chosen i b1) (h2 : P.chosen i b2) : P.v i b1 = P.v i b2'),
  P('Lemma 9: from EraLe, e(b1) ≤ e(i); from P7, e(i) ≤ e(b2)+1; from EraMono, e(b2) ≤ e(b1); hence e(i) ∈ {e(b1), e(b1)+1} and the required frown is one of the two halves of P1. Theorem 10 is Theorem 8 applied to the per-instance Synod view toSynod i, whose invariants toSynod_inv are supplied by the Paxos invariants with S1 discharged by Lemma 9. This is the statement that a reconfiguring cluster cannot be corrupted: no slot ever has two chosen values, whatever the sequence of configurations, provided P1 holds.'),

  H2('4.5 Rung 5 — The cross-era frown is necessary'),
  P('Two nodes (false = node 1, true = node 2). Era-0 quorums are {node 1} for both phases; era-1 quorums are {node 2}. Instance 0 is in era 1. Ballots (0,0) and (0,1) are era-0 ballots, legal since e(b) = 0 ≤ e(i) = 1 ≤ e(b)+1. Node 1 grants both phase-I quorums but never accepts, so it reports no last vote and constrains nothing; node 2 grants both phase-II quorums but never promised, so it accepts both. Ballot (0,0) chooses false; ballot (0,1) chooses true.'),
  ...CodeBlock('theorem cross_frown_necessary :\n    (∀ e, Frown (bad.QII e) (bad.QI e)) ∧ bad.EraMono ∧ bad.EraLe ∧ bad.P23 ∧ bad.P4 ∧\n    bad.P5 ∧ bad.P6 ∧ bad.P7 ∧ ¬ bad.P1 ∧\n    ∃ i b1 b2, bad.chosen i b1 ∧ bad.chosen i b2 ∧ bad.v i b1 ≠ bad.v i b2'),
  P('Consequence for uVRR: any reconfiguration scheme that does not preserve QI_e ⌐ QII_{e+1} is unsafe, not merely unproven. Combined with Theorem 10 this pins the cross-era overlap as exactly the condition needed (the intra-era frowns are FPaxos’s standing requirement).'),

  H2('4.6 Rung 6 — The leader’s casting vote (§V)'),
  P('Leader ℓ in era e with ballot b has a casting vote if there are q ∈ QII_e and q′ ∈ QI_{e+1} with q ∩ q′ = {ℓ}. It sends prepare(b′), e(b′) = e+1, only to q′ \\ {ℓ}, keeps streaming era-e proposals under b to q, and finally promises b′ to itself. We prove:'),
  B('noninterference: no node of q other than ℓ can have promised b′.'),
  B('guard_preserved: hence the acceptance guard for b at every node of q \\ {ℓ} is unchanged by the new promises — the era-e pipeline is not stalled by phase I of b′. This is the non-stop claim stated as a safety property of the guard.'),
  B('completes_phase1: the instant ℓ promises to itself, all of q′ has promised, meeting P5’s quorum obligation for b′.'),
  P('Agreement needs no new assumption: the casting-vote mode only chooses who receives prepare(b′) and when ℓ promises; it never relaxes P1–P7, so Theorem 10 applies to every history it can produce. What is not proved: that a casting vote exists for a given pair of configurations, or that messages arrive.'),

  H2('4.7 Rung 7 — Voting weights'),
  P('Weighted majorities M(w) = { q | 2·Σ_{a∈q} w(a) > Σ_a w(a) } over the four servers W,X,Y,Z. The blog’s schedule (1,1,1,0) → (2,2,2,0) → (2,2,2,1) → (2,2,1,1) → (2,2,0,1) → (2,2,0,2) → (1,1,0,1) satisfies the UPaxos frowns at every step (rows_ok, one decide over all 16 subsets per row; rows_p1 unpacks it into six Prop-level frowns). The negative control plus_two_breaks shows (1,1,1,0) → (3,1,1,0) fails: {X,Y} is a majority under the first, {W} alone under the second. This matches the tightness of the paper’s Lemma 3 bound (total scaled change ≤ 1).'),

  H1('5. Correspondence to uVRR and the repository’s TLA+ model'),
  table([3400, 5960], [
    ['UPaxos (this proof)', 'VRR-2012 / uVRR'],
    ['ballot b with era in the high bits', 'view-number with era prefix ([era |-> e, idx |-> v] in VrrCoreEras.tla)'],
    ['phase I of b, quorum from QI_{e(b)}', 'view change (DoViewChange/StartView); view quorum'],
    ['phase II of instance i, quorum from QII_{e(i)}', 'Prepare/PrepareOK for op-number i; commit quorum'],
    ['promised(a,b;b′) carrying the last vote', 'DoViewChange carrying the log'],
    ['chosen_i(b)', 'commit of op-number i'],
    ['P1: QII_e ⌐ QI_e ⌐ QII_{e+1}', 'the startup gate’s intersection checks in VrrCoreEras.tla'],
  ]),
  gap(),
  P('Observation (inference, not proved here). The TLA+ startup gate requires both directions of cross-era view/commit intersection. Under the UPaxos era rules (e(b) ≤ e(i) ≤ e(b)+1) the Lean proof needs only QI_e ⌐ QII_{e+1}. If the uVRR design lets a new-era view complete old-era slots, i.e. a new-era ballot proposing into an old-era instance, that violates EraLe and the reverse overlap QI_{e+1} ⌐ QII_e becomes necessary too; if old-era slots must be finished under old-era views as in the paper, the extra gate is conservative. Which of these the Rust crate implements is a design decision, and settling it is the natural next rung.'),

  H1('6. Leanstral and tooling outcomes'),
  P('The user’s request was to check the proofs with Lean and to use the Leanstral engine where it helps. Outcomes:'),
  B('All seven rungs were written directly and compiled with lake build under Lean 4.33.1; total build time about ten seconds. Rungs 3 and 4 compiled at the first attempt; Rungs 2 and 5 needed one fix round each (a subst on a non-variable equation; decide on a goal containing a free variable).'),
  B('Leanstral was used through the user’s vibe fork (vibe -p --agent lean --auto-approve --trust) as a background drafting agent for the one result that is genuinely algebraic and not needed by the ladder: the paper’s general Lemma 2 (weighted majorities overlap when the scaled weights differ by at most one in total). A first launch without --trust blocked for ten minutes on the working-directory trust prompt with no output; relaunched with --trust it streamed immediately.'),
  ...outcome.map(t => P(t)),
  B('Earlier in this project the direct Mistral API path (model labs-leanstral-1-5-1) repeatedly imported Mathlib despite instructions and did not compile, whereas the vibe lean agent produced compiling files after one fix round. The pattern held here: the agent path is the productive one.'),

  H1('7. What is not proved'),
  B('Liveness of any kind: no claim that a casting vote exists, that leaders are elected, or that messages arrive.'),
  B('Multi-promises promised≥i, i_max, and the paper’s derivation of proposed_i(b) ⇒ e(b) ≤ e(i); we take that consequence as the hypothesis EraLe.'),
  B('VRR-specific mechanics: recovery, state transfer, and the log-prefix semantics of view change. The correspondence in §5 is a mapping, not a refinement proof.'),
  B('Refinement from an operational state machine (or from the Rust crate) to the invariant sets S1–S6 / P1–P7. The theorems are about any history satisfying the invariants; showing that vrr-core’s transitions preserve them is the next major step.'),
  B('The general weighted-majority Lemma 2, unless the Leanstral outcome above reports a compiling proof; the concrete 4-server schedule in Rung 7 does not depend on it.'),

  H1('8. How to verify'),
  ...CodeBlock('export PATH=$HOME/.elan/bin:$PATH        # Lean/Lake 4.33.1 via elan\ncd formal/uvrr-lean\nlake build\nprintf \'import UVRR\\n#print axioms Paxos.theorem10\\n#print axioms Counterexample.cross_frown_necessary\\n\' | lake env lean --stdin\nuv tool install showboat\nfor f in ladder/0*.md; do showboat verify "$f" && echo "OK $f"; done'),
  P('Each ladder document embeds the full module source, the build line, and the axiom printout, so the commands can be extracted (showboat extract) and re-run independently of this report.'),

  H1('9. Next steps (proposed rungs)'),
  N('Rung 8: operational single-instance state machine (promise/accept/propose transitions with guards) and an inductive invariant implying S1–S6, so Theorem 8 becomes a theorem about executions rather than histories.'),
  N('Rung 9: multi-promises and i_max, discharging EraLe from P2–P5 as in the paper.'),
  N('Rung 10: decide the EraLe question for the uVRR design (§5) and either drop the reverse cross-era gate in VrrCoreEras.tla or prove the strengthened theorem that needs it.'),
  N('Rung 11: connect to the Rust crate: extract the quorum-family checker frownB into a property test over vrr-core’s Membership structure so every configuration change the library accepts is one the theorem covers.'),

  H1('References'),
  B('D. C. Turner. Unbounded Pipelining in Dynamically Reconfigurable Paxos Clusters. tessanddave.com/paxos-reconf-latest.pdf.'),
  B('H. Howard, D. Malkhi, A. Spiegelman. Flexible Paxos: Quorum Intersection Revisited. OPODIS 2016.'),
  B('B. Liskov, J. Cowling. Viewstamped Replication Revisited. MIT-CSAIL-TR-2012-021, 2012.'),
  B('L. Lamport. Paxos Made Simple, 2001; Reconfiguring a State Machine (with Malkhi, Zhou), SIGACT News 2010.'),
  B('simbo1905. UPaxos: Unbounded Paxos Reconfigurations (2016); Paxos Voting Weights (2017); One More Frown Please? (2020); Unbounded Viewstamped Replication Revisited? (2026). simbo1905.wordpress.com.'),
  B('G. Pîrlea et al. Veil: A Framework for Automated and Interactive Verification of Transition Systems. CAV 2025.'),
  B('vrr-core repository: formal/README.md, formal/VrrCoreEras.tla, formal/lean-leanstral-report.md, formal/uvrr-lean/.'),
];

const doc = new Document({
  styles: { default: { document: { run: { font: 'Arial', size: 22 } } },
    paragraphStyles: [
      { id: 'Heading1', name: 'Heading 1', basedOn: 'Normal', next: 'Normal', quickFormat: true, run: { size: 30, bold: true, font: 'Arial' }, paragraph: { spacing: { before: 280, after: 160 }, outlineLevel: 0 } },
      { id: 'Heading2', name: 'Heading 2', basedOn: 'Normal', next: 'Normal', quickFormat: true, run: { size: 25, bold: true, font: 'Arial' }, paragraph: { spacing: { before: 200, after: 120 }, outlineLevel: 1 } },
    ] },
  numbering: { config: [
    { reference: 'bullets', levels: [{ level: 0, format: LevelFormat.BULLET, text: '•', alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 720, hanging: 360 } } } }] },
    { reference: 'numbers', levels: [{ level: 0, format: LevelFormat.DECIMAL, text: '%1.', alignment: AlignmentType.LEFT, style: { paragraph: { indent: { left: 720, hanging: 360 } } } }] },
  ] },
  sections: [{
    properties: { page: { size: { width: 11906, height: 16838 }, margin: { top: 1440, right: 1440, bottom: 1440, left: 1440 } } },
    footers: { default: new Footer({ children: [new Paragraph({ alignment: AlignmentType.CENTER, children: [new TextRun({ text: 'uVRR reconfiguration safety — page ', size: 18 }), new TextRun({ children: [PageNumber.CURRENT], size: 18 })] })] }) },
    children,
  }],
});
Packer.toBuffer(doc).then(b => { fs.writeFileSync(process.argv[2] || 'paper.docx', b); console.log('written'); });
