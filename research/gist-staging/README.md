# Mistral OCR: PDF → Markdown (math as LaTeX)

OCR every `*.pdf` in an input directory with the Mistral OCR API and write one
Markdown file per PDF. Math is preserved as LaTeX.

## Setup

Export your key, or put it in a `.env` file in the directory you run from:

```
export MISTRAL_API_KEY=...
# or
echo 'MISTRAL_API_KEY=...' > .env
```

`.env` must never be committed to git.

## Run

```
python3 ocr_run.py [in_dir out_dir]
```

Defaults: `in_dir=./papers`, `out_dir=./ocr`. Requires Python 3 (stdlib only).

## What you get

- One `out_dir/<pdf-stem>.md` per PDF, pages joined with `---` separators.
- Math rendered as LaTeX (e.g. `$$ ... $$`), tables/figures as Markdown.
- Files already OCR'd (existing non-trivial output) are skipped.
- Large PDFs that fail as a data URI automatically fall back to the Mistral
  files-API upload path.

## No-leak properties

- The key is read only at runtime from `MISTRAL_API_KEY` or `.env`.
- The key is never printed, logged, or embedded in output.
- These gist files contain no credentials.
