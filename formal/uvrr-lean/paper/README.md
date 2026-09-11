# Draft and publish the paper

The current draft is **Unbounded Viewstamped Replication Revisited for
Diskless Strong Consistency**, centred on leader overlap and the five-voter
replacement schedule. Applications are discussed separately from the introduction.

For a local draft, run from the repository root:

```sh
mkdir -p output/pdf
tectonic --keep-logs --outdir output/pdf formal/uvrr-lean/paper/paper.tex
```

This produces `output/pdf/paper.pdf` without publishing a version of record.
The proof baseline is identified in the manuscript. From `formal/uvrr-lean`,
`lake build` and `python3 check_axioms.py` check that baseline's Lean sources.

## Publication

The editable source is **paper.tex**. `./build.sh` typesets it with Tectonic and
publishes an immutable, versioned copy to **papers/<id>.pdf**, where `<id>` is
`YYYYMMDD-<short HEAD sha>`. The same id is stamped into the bottom footer of
every page and into the title-page revision line through the generated,
gitignored **version.tex**; the id of the most recent publication is recorded
in the gitignored **.published-id** stamp.

From a terminal:

```sh
cd formal/uvrr-lean/paper
./build.sh
open "papers/$(cat .published-id).pdf"
```

Publication rules:

- `papers/` is tracked in git: each PDF there is a version of record and is
  never overwritten. Building an already-published id is refused; remove the
  PDF deliberately to reattempt. When HEAD moves, the next build publishes
  under the new id and older PDFs remain untouched as history.
- The build refuses while tracked files under `paper/` carry uncommitted
  modifications relative to HEAD, staged or not: the paper publishes only
  from committed sources, because the id names HEAD's commit. The flow is
  commit the sources, then build, then commit the published PDF.
- After publishing, the script asserts with `pdftotext` that the footer id is
  present in the PDF text; a PDF that fails the check is withdrawn, not
  published.
- The intermediate **paper.pdf**, **version.tex**, **.published-id** and
  **paper.log** are gitignored build artifacts; only the sources and
  `papers/*.pdf` are tracked.

`./build.sh --ocr` additionally uploads the published PDF to Mistral OCR
(`mistral-ocr-latest`) using `MISTRAL_API_KEY` from the repository `.env`,
requires every LaTeX caption string to appear in the OCR text, and writes the
OCR markdown to the gitignored `.tmp/` at the repository root. The key is
never echoed, staged, or committed, and the pass is off by default.

Verified with Tectonic 0.17.0. The script fixes `SOURCE_DATE_EPOCH`, so the
layout is stable for a given Tectonic release and package bundle; publication
is by id, not by byte-identical rebuild. Install the
typesetter once, if needed, with `brew install tectonic`; Tectonic obtains and
caches the ordinary LaTeX packages on the first build, and
`./build.sh --only-cached` builds without downloading. The script is
equivalent to `tectonic --keep-logs paper.tex` in this directory plus the
publication steps above. It stops with a nonzero exit status on a failed
build and leaves diagnostics in paper.log. The source has inline bibliography
entries, so no separate BibTeX step is needed.

Edit the title, author/email and `\paperrevision` near the beginning of
paper.tex; edit the prose, equations and bibliography directly below them.
The author email is simon.massey@stenographer.cloud. The layout follows
David C. Turner's paper: US Letter, a two-column IEEE journal layout,
Times text, a centered title/author, first-page contact notes, a
top-right page numbers, the revision and paper id in the bottom
footer, and Roman-numbered section headings. His affiliation, copyright and
license are not assigned to this manuscript.

The proof ladder lives in the parent directory. Publishing this paper does
not rerun or change the formal evidence. Rung 26 records the final conditional
composition and its checks; its transcript is `../ladder/26-reincarnation-safety.md`.
