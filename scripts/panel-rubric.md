# Panel Review Rubric for Academic Papers

## Purpose

This rubric guides a multi-model panel review of a LaTeX academic paper. The
panel uses a light version of the Linear Margin-Note Methodology: a first pass
to write margin notes and a glossary, a second pass to build a concept map with
forward and backward references, then an error-detection pass, and finally a
recommendations pass for tone, ordering, and impact.

## Draft awareness

The paper may be an early draft. Some sections may contain placeholder text
(lorem ipsum) or carry explicit warnings that results are not yet done. A
reviewer must:

- Grade placeholder sections as **U** (Ungraded) — not F. A U grade means the
  section is recognized as a placeholder in an early outline draft and is not
  evaluated for quality.
- Record the warning text verbatim so the author knows the reviewer saw it.
- Not penalize the paper for warned omissions; only penalize silent gaps.

## Grading scale

| Grade | Meaning |
|-------|---------|
| A | Fully satisfied. The section, figure, or table is complete, correct, and well-presented. |
| B | Satisfied with minor deviations. The content is present but could be improved. |
| C | Gap with warned placeholder. The paper carries an explicit warning that this is work-in-progress. |
| D | Gap without warning. The content is missing or incomplete with no acknowledgment. |
| F | Broken or wrong. The content is present but contains errors, broken references, or false claims. |
| U | Ungraded. The section is a placeholder (lorem ipsum, TODO, or explicit draft warning). |

## Impact assessment

### Title and abstract impact

Judge whether the title and abstract make the research contribution jump off
the page or bury it. Consider:

- Does the title name the actual contribution, or is it generic?
- Does the abstract state the problem, the approach, and the result in the
  first three sentences?
- Would a reader who scans only the title and abstract understand what is new?
- Does the abstract hint at the key result without giving away the full proof?

### Research contribution impact

Judge whether the research contribution itself has impact:

- Is the problem real and clearly motivated?
- Is the approach novel or a straightforward application of known techniques?
- Does the result change what practitioners or researchers should do?
- Is the evidence (proofs, experiments, demonstrations) sufficient for the
  claims?

### Presentation quality

Judge whether the presentation makes the value jump off the page or buries it:

- Are key results foregrounded or hidden in dense prose?
- Are figures and tables referenced and discussed at the point they appear?
- Does the paper follow a logical progression that a reader can follow?
- Is the tone well-calibrated — confident without being cocky, precise without
  being pedantic?

## Light Margin-Note Methodology (two-pass)

### Pass 1: margin notes and glossary

Read the paper in order. For each section, write append-only notes:

- Ideas, terms, and concepts encountered.
- What would be underlined or scribbled in the margins.
- Questions or trade-offs the paper leaves open.
- Every figure and table — record its label, caption, and where it is first
  referenced.

After reading, consolidate the margin notes into a single glossary:

- One preferred term per concept.
- Source aliases recorded.
- Every figure and table must appear as a glossary entry.
- No duplicate definitions.

### Pass 2: concept map and reference check

Go through the paper again with the glossary in hand. For each concept:

- Record where it first appears (section/line).
- Record every subsequent occurrence.
- Classify references as forward ("shown below", "in Section X") or backward
  ("as defined in Section Y", "see Figure Z").
- Check that every figure and table is referenced at least once.
- Record which concepts relate to or depend on other concepts.

The output is a concept map in text: a list of concepts with their dependencies
and their forward/backward references.

## Error detection

After building the concept map, list:

- Omissions: material that is referenced but not present.
- Errors: wrong citations, broken references, mislabeled figures.
- Broken items: LaTeX errors, undefined references, missing captions.
- Dangling references: "see Section X" where Section X does not exist.
- Unreferenced figures or tables: a float that is never cited in the text.

## Section and figure grading

Grade each section, figure, and table F through A (or U for placeholders) with
specific comments about what would improve it.

## Sweeping recommendations

After grading, make recommendations about:

- Title changes (more specific, more punchy, or more accurate).
- Pulling results forward into the abstract.
- Reordering sections for better flow.
- Tone calibration (more confident, more precise, less hyperbolic).
- Making concepts easier to follow without dumbing them down.
- Being persuasive about impact without shallow pathos or ethos techniques.
- Ensuring necessary details back up the claims.
