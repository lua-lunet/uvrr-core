# Checklist protocol for published paper versions

Papers are published as immutable, versioned PDFs at
`formal/uvrr-lean/paper/papers/<id>.pdf`, where `<id>` is
`YYYYMMDD-<shortsha>` computed from the commit the paper was built from.
Every published version carries a matching checklist `papers/<id>.md` that
records what that version had to contain and the grades it received. This
document defines how a checklist is generated and how a version is graded.

## Sources

- `papers/paper-dod.md` — the private, gitignored definition-of-done file. It
  holds the Director's actual words and phrases with session pointers, a
  margin-noted glossary of must-contain and must-avoid items, and the
  definition-of-done position paper. It is never staged, never committed, and
  never quoted verbatim in any tracked file. The glossary in it is the sole
  source for checklist items.
- `papers/<id>.pdf` — the built paper version under review.
- The tracked paper sources (`paper.tex` and the reference artifacts under
  `paper/turner/`) as supporting evidence.

## Generating a checklist

Before the commit that publishes a paper version, an agent derives
`papers/<id>.md` from the private `paper-dod.md`:

1. Read `paper-dod.md` in full.
2. Convert every must-contain and must-avoid glossary entry into one
   checkable statement, phrased in clean-room language: no private quotes, no
   emotion, no session identifiers, no task-item identifiers.
3. Group the statements by paper region (introduction, vocabulary,
   references, formal content, figures, demo, experiments, publication
   governance).
4. Each statement carries a `Grade:` field left pending for the grading pass.
5. Write the file as `papers/<id>.md` with the same id as the PDF.

A checklist item must be falsifiable against the PDF: it names a property a
reader can locate and verify in the paper (a section that states something, a
citation that exists, a caption form, a boxed warning, a footer id). An item
that cannot fail is not an item.

## Captions and first references

Captions are nouns and short: a caption names what the float is and carries
no prose, claims, or narrative. Every figure and table must be referenced in
the text. The FIRST in-text reference to a figure or table states more of
what would have been a long-form caption --- what the float shows, its axes
or columns, and its provenance --- so that a user reading the paper aloud
hears what is shown at the point it is discussed. A checklist derived under
this protocol carries one item for this rule.

## Grading

Grading is performed by a second agent — never the agent that authored or
built the version — and it happens BEFORE the commit that publishes the paper.
The grading agent reads the published PDF, the private `paper-dod.md`, and the
checklist `papers/<id>.md`, then grades every checklist item:

- **A** — the item is fully satisfied; the location in the paper is recorded.
- **B** — the item is satisfied with a minor deviation that does not
  compromise the mandate; the deviation is recorded.
- **C** — the item is a gap: the paper carries a clearly warned placeholder
  where the item's content would belong (for example, experiments that are
  designed but not performed, carrying the boxed warning and placeholder
  values). A C is a work-in-progress pass, permitted while the paper iterates.

Rules:

1. Grading must complete and be recorded in `papers/<id>.md` before the
   publishing commit. A version is never committed ungraded.
2. Each `papers/<id>.md` records the grades for that version, with a pointer
   to where in the paper each graded item is satisfied (or its placeholder
   found).
3. Any C must correspond to a placeholder that carries an explicit warning in
   the paper itself; a silent gap is never a C — it is a failure of the
   grading pass.
4. The final paper must reach A or B on every checklist item. A C on the
   final version blocks release.
5. Checklists are per-version and immutable once graded: a later version gets
   a new checklist; earlier checklists are not rewritten.

## Publication context

The checklist and grading gates sit on top of the governed build: the paper id
is derived from the commit, the build refuses dirty sources and refuses to
overwrite an existing published PDF, the footer id is asserted present in the
built PDF, and papers are published locally, not from CI. A version that lies
about its commit cannot be graded, because its id is wrong at the source.
