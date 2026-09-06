# Edit and build the paper

The editable source is **paper.tex**. The output is **paper.pdf**.
Everything needed from this project is in this directory; the build does not
run Lean, Rust, Showboat, Codex, or any model service.

From a terminal:

```sh
cd formal/uvrr-lean/paper
./build.sh
open paper.pdf
```

Or invoke the script by its path from any working directory:

```sh
formal/uvrr-lean/paper/build.sh
```

Verified with Tectonic 0.17.0. The script fixes `SOURCE_DATE_EPOCH`, so two builds
with the same Tectonic release and package bundle produce byte-identical PDFs; the
recorded digest is in `../REPRODUCE.md`.

Install the typesetter once, if needed, with `brew install tectonic`.
Tectonic obtains and caches the ordinary LaTeX packages on the first build.
After that, `./build.sh --only-cached` builds without downloading packages.
The script is equivalent to `tectonic --keep-logs paper.tex` in this directory.
It stops with a nonzero exit status on a failed build and leaves diagnostics
in paper.log. The source has inline bibliography entries, so no separate
BibTeX step is needed. No credentials or paid API are required.

Edit the title, author/email and `\paperrevision` near the beginning of
paper.tex; edit the prose, equations and bibliography directly below them.
The email is written exactly as supplied: simon.massey@stenographer.cloud.
The layout follows David C. Turner's UPaxos paper: US Letter, a two-column
IEEE journal layout, Times text, a centered title/author, first-page contact
notes, a title/revision header, top-right page numbers, and Roman-numbered
section headings. His affiliation, copyright and license are not assigned
to this manuscript.

The proof ladder and lab book live in the parent directory. Editing this paper
rebuilds the document; it does not rerun or change the formal evidence.
