# item03 — Secret-audit the OCR bundle

Goal: prove that `.tmp/papers/**` and `.tmp/ocr_run.py` contain no secrets.

Steps (read-only; change NO files; do NOT git add anything; NEVER read or print `.env` contents):

1. Check no files named `.env` or `.env.*` exist anywhere under `.tmp/papers/`.
2. Silently extract the key WITHOUT echoing it:
   `KEY=$(awk -F= '/^MISTRAL_API_KEY/{print $2}' .env | tr -d '"' | tr -d "'")`
   Then run `grep -rlF -- "$KEY" .tmp/papers .tmp/ocr_run.py` from the repo root
   (`/Users/Shared/lua-lunet/vrr-core`). Expect zero matches. Never print the key itself.
3. Pattern scan the OCR markdown only (`rg -il "api\.mistral|Bearer |gho_[A-Za-z0-9]{20,}|MISTRAL_API_KEY" .tmp/papers/ocr`).
   Note: papers legitimately mentioning the word "Mistral" in prose are fine; only credential-shaped hits are failures.

Report back (keep it under 10 lines, no file-content dumps):
- PASS or FAIL overall
- any failing file paths
- counts: files scanned, matches found
