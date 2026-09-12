# Skill: rigorous-journal-paper

Use when writing, editing, or reviewing a scientific paper intended for a
reputable journal — the standards of pre-paper-mill CS and physics publishing
(roughly the 1990s bar). The skill enforces three principles, prescribes the
classic structure and LaTeX practice, and runs a pre-submission
anti-paper-mill checklist. It actively combats paper-mill culture: unsupported
claims, citation padding, missing error bars, non-reproducible figures, and
predatory venue targeting.

---

## Why this skill exists

Paper mills manufacture manuscripts with faked data, stock images, padded
citations, and predatory venue targeting. Detection (tortured-phrase scanners,
image forensics, retraction watches) is reactive. The root cause is authorial
discipline, and that is what this skill enforces — at the authoring stage,
before submission.

## The core rule: three principles

**1. Falsifiability.** Every claim is stated precisely enough to be refuted.

- ❌ "Our approach is effective."
- ✅ "Our approach reduces median latency by >10% on the DAWNBench dataset
  (Section 5)."

Every claim names its evidence and the conditions under which it was measured.
A claim that cannot be refuted by any experiment or proof is not a claim — it
is decoration. Cut it or make it testable.

**2. Reproducibility.** The description alone lets a reader replicate the
result. Mandatory, per experiment:

- Dataset: source URL, version, and every preprocessing step.
- Parameters: every hyperparameter, threshold, and configuration value.
- Hardware: CPU/GPU model, memory, OS.
- Random seeds: fixed and stated.
- Statistical treatment: test, sample size, error bars or confidence
  intervals — where a claim is statistical, the uncertainty travels with it.

Omitting any of these is a violation, not a style choice.

**3. Honest limitations.** The paper states what the work does *not* do:

- Threats to validity: internal, external, construct.
- Failed approaches and why they failed (negative results are results).
- The conditions under which the method would not work.

A Limitations section that only flatters is a red flag in itself.

## Structure (strict)

Abstract → Introduction → Method → Results → Discussion → Related Work.
(IMRaD extended with an explicit Related Work section that *engages* prior
work, and an explicit Limitations subsection inside Discussion.)

- **Abstract**: the claim, the evidence, the number, the uncertainty. No
  adjectives doing the work of data.
- **Introduction**: the gap, the contribution list, each contribution mapped
  to a section.
- **Method**: complete enough to replicate; every condition stated.
- **Results**: numbers with uncertainties; every figure regenerable from
  stated parameters; negative results reported, not buried.
- **Discussion**: interpretation, threats to validity, limitations,
  generalizability (and non-generalizability).
- **Related Work**: each citation engaged with — what it did, how it differs,
  what gap remains. No drive-by citations.

## LaTeX practice

- Use the venue's own class: `IEEEtran` for IEEE, `acmart` for ACM,
  `llncs`/LNCS for Springer. Never a homemade layout.
- Compiles cleanly with zero unresolved references and no orphan warnings.
- Every figure regenerable from stated parameters (keep the plotting script
  or the raw data next to the paper).
- Math in proper environments (`amsmath`/`amsthm`); theorems labelled;
  cross-references via `\ref`, never hardcoded numbers.
- Bibliography in the venue's style (`ACM-Reference-Format`, IEEEtran bst);
  every `\cite` resolves to a real, engaged-with source.

## Red flags — these mean STOP

| Thought | Reality |
|---------|---------|
| "Our approach significantly improves performance." | No number, no baseline, no conditions — not falsifiable. State the metric, the magnitude, and the dataset. |
| "As shown in [1], [2], [3]…" | Citation padding. Each citation is engaged with, or removed. |
| A figure with no axis units, no error bars, no script | Not reproducible — regenerate it from stated parameters or cut it. |
| "Detailed results omitted for brevity." | The reader cannot verify the claim. Omitting evidence is claiming nothing. |
| p-values without the test, n, or effect size | Statistical theatre. State the test, the sample, and the interval. |
| "Extensive experiments demonstrate…" | Which experiments? List them or the sentence is advertising. |
| A venue that emailed you, promises 48-hour review | Predatory. Verify before submitting (below). |
| Copyedited "tortured phrases" ("counterfeit consciousness") | A paper-mill linguistic marker (Cabanac 2021). If you did not write it honestly, delete it. |
| "Future work will show…" | Claims deferred to work that does not exist are claims not made. State them as open questions instead. |
| Self-citation clusters | Reviewer-visible padding. Cite your own work only where it is load-bearing. |

## Pre-submission anti-paper-mill checklist

A manuscript is not submittable until every item is checked or justified as N/A:

1. Every claim is stated as a falsifiable hypothesis (precise metric,
   magnitude, dataset or theorem).
2. Every claim is evidenced in the text — no orphan claims.
3. All parameters, datasets, preprocessing, and hardware are specified.
4. Random seeds are fixed and stated.
5. Every figure can be regenerated from the stated parameters (script or
   data attached).
6. Every figure has axis units, baselines, and uncertainty where applicable.
7. Statistical claims carry the test name, sample size, and confidence
   interval or error bars.
8. Every citation is engaged with in the text (not listed for volume).
9. No citation is padded in — delete any reference that is not load-bearing.
10. The target venue is verified as non-predatory (DOAJ listing, JCR, Beall's
    list successors, publisher check).
11. The paper has an explicit Limitations section with threats to validity
    (internal, external, construct).
12. Negative results and failed approaches are reported.
13. Conditions under which the method does NOT work are stated.
14. Every `\cite` resolves; the LaTeX compiles cleanly with no unresolved
    references.
15. Every figure and table is referenced in the text.
16. The abstract contains the number and the uncertainty, not adjectives.
17. The contribution list matches the delivered sections one-to-one.
18. No stock images, no reused figures without licence and citation.
19. Author contributions and conflicts of interest are stated.
20. The paper was read start-to-finish by a human who could refute any claim
    in it.

## Venue verification

Before submission: check the venue against DOAJ, the Journal Citation
Reports, and the Beall's-list successors; confirm the editorial board is real
and reachable; confirm indexation claims independently. A venue that guarantees
acceptance, charges undisclosed fees, or has no verifiable editorial board is
predatory — do not submit, and do not cite it as evidence of legitimacy.

## Worked example — the shape of a falsifiable contribution

A paper *about* this skill states its contribution as a refutable claim, e.g.:
"adopting this skill's discipline reduces paper-mill red flags in submitted
manuscripts by X% (95% CI […, …], test, n)". The study design that backs such
a claim specifies: participants and assignment (randomized control vs skill
use), the counting instrument (this checklist's red flags, scored by
independent reviewers with reported inter-rater agreement, e.g. Cohen's κ),
and the statistical test with the full interval. Every number in that sentence
comes from a study actually run — a template with invented numbers is the
paper-mill pattern this skill exists to eliminate. Per-category breakdown,
negative results (e.g. which checklist categories did NOT improve and why),
and the validity threats (Hawthorne effect from non-blinding, CS-only sample,
author-designed instrument) belong in the Results and Discussion sections.

## Summary

Falsifiable claims. Reproducible descriptions. Honest limitations. A checklist
that is a gate, not decoration. A verified venue. If a claim cannot be
refuted, evidenced, and reproduced — cut it. A paper that survives this skill
is one a 1990s referee would take seriously.

---

## Language, locale, and authorial voice

Choose the manuscript language and regional variety before drafting. Record the choice in the work log and use it consistently in prose, captions, tables, supplementary material, and the spell-check gate.

- **British English:** use `en_GB`. Decide whether the project follows the `-ise` or `-ize` British variant and keep that choice consistent. The repository default is British English; do not Americanise it to satisfy a generic style suggestion.
- **United States English:** use `en_US` only when the author or venue has selected it. Do not mix `colour`/`color`, `organise`/`organize`, or `licence`/`license` accidentally. A quotation, proper name, title, or code identifier may retain its source spelling and should be identified as such.
- **Spanish:** distinguish mainland Spanish (`es_ES`) from Latin-American Spanish (`es_419`) and, when needed, a country variety such as `es_AR`, `es_CL`, `es_CO`, or `es_MX`. “South American Spanish” is not one universal editorial standard; name the selected variety rather than silently combining regional forms.

The language setting constrains editorial voice; it is not evidence about the research. Technical names, identifiers, citations, URLs, equations, and source quotations need an explicit exclusion or project dictionary. Ordinary prose must still be checked. If a venue house style conflicts with the selected voice, record the exception and obtain author approval before changing the manuscript.

## IEEE requirements and first-use rules

When the target is IEEE, read the complete current IEEE Editorial Style Manual and IEEE Reference Guide before the draft is called final. Do not rely on a search result, a remembered rule, or a copied excerpt. Record the document URL, revision/access date, and the pages or sections checked. The primary references are the [IEEE Editorial Style Manual for Authors](https://journals.ieeeauthorcenter.ieee.org/wp-content/uploads/sites/7/IEEE-Editorial-Style-Manual-for-Authors.pdf) and the [IEEE Reference Guide](https://journals.ieeeauthorcenter.ieee.org/wp-content/uploads/sites/7/IEEE_Reference_Guide.pdf).

The Editorial Style Manual, printed p. 10 under **Abstract**, requires an accurate standalone abstract and says that it shall contain no numbered mathematical equations, numbered reference citations, or footnotes. The IEEE Author Center's [Structure Your Article](https://journals.ieeeauthorcenter.ieee.org/create-your-ieee-journal-article/create-the-text-of-your-article/structure-your-article/) page also calls for one paragraph of at most 250 words, unambiguous terminology, no abbreviations, references, footnotes, or equations, and 3--5 keywords or phrases.

1. Write the abstract so it stands alone in an index. State the problem, method, achieved result, evidence type, and principal limitation. Do not put a citation number, footnote, numbered equation, or unexplained acronym in it. Attribute prior work and give the citation in the Introduction.
2. Add IEEE Index Terms after the abstract, normally 3--5 terms, defining any acronym used there and following the target publication's capitalization and ordering rules.
3. Use the venue's class and template (`IEEEtran` for IEEE) and remove local page furniture, draft stamps, and bespoke headers before final submission unless the venue explicitly asks for them. Keep generated build metadata separate from editable source.
4. Use numbered equations and cross-references consistently. Never type a figure, table, equation, or section number into prose when `\ref` or the venue's mechanism can supply it. Every displayed equation, figure, and table must be introduced and interpreted in the text.
5. Review every paragraph in order. Check that the first sentence states the paragraph's point, each pronoun has a clear antecedent, each claim has evidence or an explicit hypothesis, and the final sentence hands the reader to the next point. Do this after technical review; do not let automated copy-editing flatten the author's voice.

For citation syntax, the IEEE Reference Guide, p. 4, §I.A, gives the in-text examples “as shown by Brown [4], [5]” and “Smith [4] and Brown and Jones [5].” Put the bracketed number inside the punctuation. Use forms such as `[3, Th. 1]`, `[3, pp. 5--10]`, `[3, Fig. 1]`, or `[3, Appendix I]` when pointing to one part of a source. The same guide specifies author initials and surnames, and lists all authors up to six before using `et al.`; follow the target IEEE publication's current guide when its template differs.

## Full-document finalisation gate

Before any draft is described as final, read the entire applicable style manual, not only the abstract and reference pages. Make a checklist with one row per requirement and record `pass`, `N/A with reason`, or `fix`, plus the file, page, command output, or reviewer note that supports the decision. Check venue/template/submission rules; abstract, Index Terms and author matter; locale and first-use definitions; equations, figures, tables and permissions; citation order and reference accuracy; reproducibility, uncertainty, negative results and limitations; TeX build and unresolved references; locale spell checking; visual inspection of every rendered page; and a human start-to-finish reading able to challenge every claim. Do not submit while any row is blank. Silence is not an exemption.

## Appendix: small snags found in a real review

These recurring failures were found while rewriting a formal distributed-systems paper. They are small individually, but compound into an inaccurate or non-compliant manuscript.

### Claims and attribution

- Do not put “following [Author]'s construction” in an IEEE abstract. Describe the mechanism in standalone terms and first attribute it in the Introduction with the numbered citation.
- Distinguish an author's construction from the present paper's application, instantiation, or mechanisation. Say what is inherited, specialised, and new.
- “Strong consistency” is not a defined property. Say whether the result is per-slot agreement, linearizability, strict serialisability, lease exclusion, or another property, and state the bridge between them or its absence.
- A theorem conditional on P2--P7, a quorum condition, or a lifecycle contract is conditional. Do not present its hypotheses as if executable code had already been refined to them. If composition is open, name it as an exercise or open obligation.
- A mutation test shows sensitivity to that mutation; it is not exhaustive fault coverage. A compiler success is not an axiom audit, and a proof generator's output is not evidence until the kernel checks it.
- Preserve provenance for externally supplied experiments: runner, model/version, isolation, budget, discarded invalid runs, and transcript availability. Do not write “we ran” when the authors only received a report.

### LaTeX and build behaviour

- Hard-wrapped source lines are valid TeX and help diffs. A newline is normally a space, but do not split control sequences, verbatim material, URLs, or syntax-sensitive arguments. Editor soft wrapping is independent of source wrapping.
- A generated `version.tex` is build metadata, not a second manuscript. Keep it ignored and explain its provenance. Do not let generated metadata silently change submitted source.
- `fancyhdr` page numbers, revision strings, draft labels, and custom running heads may be useful internally but are not automatically IEEE front matter. Check the target template and remove them for submission.
- Tectonic or latexmk compilation is necessary but not sufficient. Check unresolved references, overfull boxes affecting readability, missing glyphs, accidental blank pages, and every rendered page.
- Run the selected locale spell checker in TeX mode. Maintain a reviewed technical word list instead of accepting every flagged token; never add an American spelling merely to silence a TeX macro false positive. Make the build fail on an unapproved ordinary-prose spelling.

### Formal methods and systems papers

- State whether diskless means “no synchronous flush on the normal path” or “no durable state anywhere.” Describe startup fencing, clean shutdown, identity allocation, and total-state-loss behaviour precisely.
- Separate a clean same-identity restart from a crash-stop followed by a fresh identity. Host identity is not protocol voting authority.
- A quorum intersection at one configuration boundary does not prove that the next boundary has a singleton leader overlap. Show the actual sets and say where the construction does and does not apply.
- “Two rounds” may mean committed configuration batches, message exchanges, or network round trips. Define the unit and do not substitute one for another.
- Exact doubling or halving of weights preserves a majority family; arbitrary rounding during halving does not follow from that fact. State the distance or integrality hypothesis used by the proof.
- A zero-weight member and an absent member are different predicates if the protocol transfers state before granting a vote. Tables and prose must preserve that distinction.
- A leader-overlap diagram is a schematic unless its messages, guards, replies, time direction, and omitted traffic are defined. Do not present it as a benchmark or implementation trace.

### Applications and comparisons

- Advisory-lock safety needs generation/fencing semantics and, for real-time expiry, clock and timing assumptions. Agreement alone does not prove that an external resource rejects stale owners.
- A strongly consistent membership authority does not make surrounding gossip or bulk algorithms strongly consistent. State the boundary of the guarantee.
- CORFU's projection-size estimate is not a general configuration-service memory measurement. Report workload, scale, percentage and estimate status, and distinguish CORFU's strongly consistent log from any weaker application data plane.
- ZooKeeper and etcd have operation-specific semantics. Do not claim all reads are linearizable when a service documents local, serialisable, or watch-specific weaker behaviour. Resource guidance is workload-specific, not a universal minimum and not a measurement of a new design.

### Final author review

For every paragraph, the final reader should be able to answer: What is claimed? Under which conditions? Where is the evidence? What would falsify it? If the answer is “the code probably does this,” “the citation is famous,” or “the model said it passed,” the paragraph is not ready for submission.
