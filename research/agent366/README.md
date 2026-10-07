# The agent366 corpus

The literature corpus behind the paper: the papers themselves, the metadata
derived from them, and the tools that produce both.

## What is here

- `papers/` — the corpus. PDFs, numbered `.txt` extracts, and the extraction
  manifest. Kept to PDFs and their extracts; nothing else belongs in it.
- `CORPUS.md` — what the corpus contains, where each paper came from, and how
  the numbered extractions are produced and verified.
- `md/` — Markdown renderings of the papers, including OCR for the four that
  carry no text layer, plus the extraction script.
- `tools/` — the extraction tooling.

## Reading a source

Each `papers/*.txt` is a paper in single-column reading order with a line number
on every body line, so a passage is quotable verbatim with a locator: a
right-column run is one contiguous stretch of ascending numbers, which the
interleaved `-layout` extraction cannot give. Every file carries a header with
the Zotero metadata, the source PDF's path and sha256, the page count and the
tool version, plus page markers.

Quote the `.txt` for the line number and the paper for authority. The `.txt` is
a locator, not a reading copy.

The four papers with no text layer name their OCR'd Markdown as the reading
surface instead of shipping a stub: a geometry extractor cannot invent a text
layer, and an empty file is worse than an honest gap. `00-linear-manifest.md`
records which are which.

## Bibliographic metadata

The metadata in those headers comes from the Zotero export, and Zotero is the
database of record. Do not hand-edit the snapshot and do not query the sqlite
file for anything but a read.

Use `research/zotero/zotero_api.py`, which is the one place that talks to
Zotero: `research/zotero/README.md` has the command list, where the API key
lives, and the sharp edges of the local API encoded once.

## A cold copy of the fragment corpus

The knowledge-base fragment corpus is mirrored, append-only and content
addressed, to `~/icloud/2026/UVRR/kb` by `research/kb-mirror/kb_mirror.py`. That
mirror is derived from git, so it is a backup rather than a second source of
truth; its rules and its deferrals are stated in the README beside it.
