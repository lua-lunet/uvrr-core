# item05 — Private gist: Mistral-OCR extraction technique (no credential leak)

Goal: publish a SECRET gist the user can reuse: how to OCR local PDFs (math-heavy)
to Markdown with Mistral OCR without ever leaking MISTRAL_API_KEY.

Rules:
- Nothing outside `.tmp/` except the `gh gist create` call itself. No git commands.
- Never read/print `.env`; never embed any key value in gist files.
- gh gists are secret by default — do NOT pass `--public`.

Steps:
1. Stage two files in `.tmp/gist-staging/`:
   - `ocr_run.py` — adapted from `.tmp/ocr_run.py`: keep behavior (data-URI OCR
     with Mistral files-API upload fallback, per-PDF `.md` output, LaTeX math
     preserved), but make it standalone: resolve the key from env var
     `MISTRAL_API_KEY` or a `.env` in the current directory; take input/output
     dirs as optional argv (default `./papers`, `./ocr`). No other changes.
     Include no key value or account info.
   - `README.md` — short usage note: what it does, setup (export key or use
     `.env`; `.env` must never be committed), how to run
     (`python3 ocr_run.py [in_dir out_dir]`), what you get (per-PDF Markdown,
     math as LaTeX), and the no-leak properties (key read only at runtime,
     never logged, gist contains no credentials).
2. Safety gate: silently capture the key from `.env` (awk, never echo) and run
   `grep -rlF -- "$KEY" .tmp/gist-staging` — must be empty. Also
   `grep -rn "MISTRAL_API_KEY" .tmp/gist-staging` may only match the variable
   NAME, never a value.
3. From `.tmp/gist-staging/`:
   `gh gist create --desc "Mistral OCR: PDF→Markdown extraction (math as LaTeX), no credential leak" ocr_run.py README.md`
4. Capture the gist URL.

Report back (<5 lines, no content dumps): gist URL, file count, safety-gate verdict.
