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

## IEEE format conformance

The paper follows the IEEE transaction format. The items below cover only the
properties that can silently regress and that are verifiable by string checks
on `paper.tex` or by `pdftotext` on the built `papers/<id>.pdf`; layout
properties visible on the page (column count, fonts, margins, page numbers)
are not checklisted.

### Abbreviation discipline

Each long name appears exactly once, at its definition; everywhere else the
abbreviation is used.

- View-Stamped Replication — `VSR-1988` (Oki and Liskov, PODC 1988)
- Viewstamped Replication Revisited — `VRR-2012` (Liskov and Cowling,
  MIT-CSAIL-TR-2012-021)
- Unbounded Pipelining in Dynamically Reconfigurable Paxos Clusters —
  `UPX-2017` (Turner, rev 1A9DBA37)
- Unbounded Viewstamped Replication — `uVRR`
- `VSR` / `VRR` where the year-tagged form is heavy in the surrounding text

CHECK (definition counts — each long name must appear exactly twice in
`paper.tex`: once in the title block, once at its in-text definition):

```
rg -c 'Viewstamped Replication' formal/uvrr-lean/paper/paper.tex
rg -c 'Unbounded Viewstamped Replication' formal/uvrr-lean/paper/paper.tex
rg -c 'Dynamically Reconfigurable Paxos' formal/uvrr-lean/paper/paper.tex
```

CHECK (unexpanded repeats after the definition — none beyond the definition
sites): `rg -n 'Viewstamped Replication' formal/uvrr-lean/paper/paper.tex`
and confirm every hit is the title or a definition line.

CHECK (undefined short forms): `rg -c 'uVRR' formal/uvrr-lean/paper/paper.tex`
must be nonzero, and each of `VSR-1988`, `VRR-2012`, `UPX-2017` must appear
before or with the corresponding long name.

### Abstract

One paragraph, 150–250 words, no citations, no tables, no equations inside
the abstract.

CHECK: extract the abstract body
(`awk '/\\begin{abstract}/,/\\end{abstract}/' paper.tex | sed '1d;$d'`),
pipe it through `wc -w`, and require the count in 150–250.
CHECK: the same extraction must contain no `\cite`, no `\begin{table}`,
and no `\begin{equation}`.

### Index Terms

Index Terms follow the abstract, are comma-separated, and alphabetically
ordered.

CHECK: extract the `IEEEkeywords` environment from `paper.tex`; the body must
contain no empty entries when split on commas, and the comma-split list must
be case-insensitively sorted (`sort -c -f`).

### Reference numbering in first-citation order

References are numbered in the order of first citation, not alphabetically.

CHECK: for each `\cite{key}` (and each key inside a multi-key `\cite{a,b}`),
record the line of first appearance in `paper.tex`; sort keys by that line.
The resulting sequence must equal the sequence of `\bibitem{key}` entries in
`thebibliography`. A one-line script can compute both sequences with
`rg -o '\\cite\{[^}]*\}'` and `rg -o '\\bibitem\{[^}]*\}'`.

### In-text citation form

Citations are bracketed numbers placed before punctuation; multi-source
groups are ascending, e.g. [4], [6], [9].

CHECK: `pdftotext papers/<id>.pdf -` piped through
`rg -o '\[[0-9, -]+\]'` — every bracket citation match must be a comma list of
integers in ascending order. CHECK: `rg -n '\[[0-9, -]+\][.,;:]' paper.tex`
must return no hits (citation followed by punctuation means the citation sat
after the punctuation mark).

### Author citation forms

Bibliography author lists follow IEEE forms: one author "J. Carter"; two
"J. Carter and R. Brown"; three or more "J. Carter et al."; more than six
authors — first six then "et al.".

CHECK: in the `thebibliography` block, no `\bibitem` author field may carry
more than seven given-name initials or the token `and` more than five times;
the token `et al.` appears only where the full list exceeds six authors.

### IEEE abbreviations in the bibliography

Only approved IEEE abbreviations appear: `no.`, `vol.`, `pp.`, `Proc.`,
`Trans.`, `ed.`, `eds.`; never the unabbreviated words.

CHECK: extract `thebibliography`; `rg -n 'Volume|Pages|Proceedings|Editor'`
must return no hits; `rg -n 'vol\.|pp\.|no\.|Proc\.|Trans\.'` must cover the
entries that carry such fields.

### Reference title case

Reference titles use sentence case; journal and venue names use their
standard abbreviations.

CHECK: extract `thebibliography`; every quoted title between ` `` ` and `'' `
must begin with a capital followed by zero or more words whose internal
capital letters (beyond the first word and proper nouns/acronyms on a
project allow-list) do not recur; verify venue tokens against the standard
abbreviations (e.g. `Proc.`, `Trans.`, conference acronym forms).

### Equation numbering

Display equations are numbered (1), (2), ... consecutively.

CHECK: `rg -c '\\begin{equation\*}' paper.tex` must be zero (no unnumbered
display equations), and the sequence of equation numbers extracted from
`pdftotext papers/<id>.pdf -` via `rg -o '^\(([0-9]+)\)$'` must be
`1, 2, ...` with no gaps.

### Caption placement

Figure captions sit below the figure; table captions sit above the table.

CHECK: for each `\begin{figure}` ... `\end{figure}` block in `paper.tex`, the
`\caption` line must come after the `\includegraphics`/content lines; for
each `\begin{table}` block, the `\caption` line must come before the
`\begin{tabular}` line. A short `awk` state machine over the environment
blocks checks both.

### Appendices

Appendices appear after the references.

CHECK: the line number of the first `\appendix`/`appendices` environment in
`paper.tex` must exceed the line number of `\end{thebibliography}`; in the
PDF text, the first appendix heading must follow the `References` heading.

### Appendix and nomenclature references

Every appendix and every nomenclature entry is referenced from the main text.

CHECK: collect all `\label{app:...}` and nomenclature labels in `paper.tex`;
each must appear in at least one `\ref{...}` outside its own definition
environment. `rg -o '\\label\{(app|sec|eq|fig|tab)[^}]*\}'` and
`rg -o '\\ref\{[^}]*\}'` produce the two sets to diff.

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
