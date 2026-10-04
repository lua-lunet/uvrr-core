# The paper-writing process

The process the uVRR paper is written under. Each stage produces one
artefact; the author drives every gate and holds the pen on every decision.
The stages run in order, but the artefacts survive: a stage can be re-run
against its artefact without re-doing the stages before it.

## 1. Brainstorm and elaborate

The author accumulates raw material over years: papers, proofs, code,
lecture records, transcripts. Nothing is discarded at this stage. The
build-up is allowed to be large; reduction is a later, separate skill.

## 2. Extract (lossless)

The LaTeX source is rendered into a structure view: one markdown heading
per section, and every prose paragraph wrapped as

    <p x-name="…" x-tex="…" x-labels="…">…text…</p>

with `x-tex` giving the paragraph's coordinates back into the source.
Figures and tables are broken out as their own entries, with their
captions and legends as adjacent entries of their own; the environment
body is marked as a block. Coverage is cross-checked against the source:
all body prose is covered, and the only unwrapped material is the
bibliography, which is a placeholder by design.

## 3. Reduce (remove the fat)

One grading agent per paragraph reads the author's lede — the yardstick,
in the author's own words — and returns a verdict (SUPPORTS, NECESSARY,
NEUTRAL, SUBTRACTS, CONTRADICTS), a grade A–F, one sentence on how the
paragraph relates to the lede, and one sentence of fix. The verdicts merge
back into the structure view's `x-labels`. Triage then walks the
worst-first list with the author: cut, trim, or keep, one decision at a
time. Mechanical scans feed the same interview — dangling TODOs,
unreferenced labels, orphan floats, duplicate sentences, cited-but-missing
keys. Before any cut lands, inbound references are checked: no dangling
pointers, ever.

## 4. Linear-margin notes

The graded long list is published as one reading file with the paragraph
text inlined, sorted worst-first, so the whole paper can be triaged from a
single page without hunting through the source.

## 5. Dependency-ordered glossary (the lattice)

The surviving paragraphs, sections, figures, tables, and citations are
loaded as nodes in a graph database; document order, citations, and
references become edges. A flat render file — markdown headings plus one
macro line per paragraph — states the order as a list of things, not a
graph of things; rendering resolves each macro from the database. The
lattice allows the paper to be totally reordered without rewriting any
paragraph.

## 6. Andon and interview

Anything conflicting, confusing, or suspect halts the line: the question
is asked, never papered over. The interview is the triage mechanism —
every cut, reorder, or reword is the author's decision, taken from options
whose trade-offs are stated in advance. When the correct fix is outside
the current lane, the line halts and the deeper work is delegated rather
than hacked around.

## 7. Long list to short list

Every triage decision is recorded in a ledger — cut, trimmed, kept, or
pen-pending — so nothing is silently lost and nothing is left half-done.
The ledger is the difference between pruning and forgetting.

## 8. The author holds the pen

The final narrative pass is the author's, and the abstract above all: it
is written last, from the finished paper, as the paper's lede.

## Standing rules

- **Location independence.** Paragraphs are self-contained. No "the
  above", no "the below"; assume a reader who has read the paper twice.
  Cross-references are resolved at final layout, by the author, when the
  order is final.
- **No dangling pointers.** No TODOs, no unreferenced labels, no orphan
  floats, no cited-but-missing keys, no empty headings left behind by a
  cut.
- **Zotero is the database of record** for citations. The repository
  holds one auto-export snapshot per worktree and no other copies; the
  snapshot is replaced wholesale from the export, never hand-edited.
- **Placeholders are honest.** Experiment scaffolding stays visibly
  watermarked until measured data lands; the headline experiment leads
  and closes each experiment's write-up.
