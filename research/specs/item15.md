# item15 — Leanstral smoke test via the user's vibe fork (lean agent), macOS

Repo root: /Users/Shared/lua-lunet/vrr-core. Platform: darwin, zsh. TIMEBOX ~25 min.

Context:
- The user's personal build of Mistral's vibe CLI: binary
  `~/opt/mistral-vibe-fork/bin/vibe` (alias `vibe-glm52`), source copy at
  `/Users/Shared/mistral-vibe` (READ-ONLY — never modify the fork or its config).
- Banner shows: v2.24.1, model `leanstral[high]`, Pro subscription, `--agent lean`
  mode ("lean agent", by default in lean mode; the user guesses it may carry Lean
  prompts / a language server).
- A Lean 4.33.1 toolchain is installed via elan at
  `~/.elan/toolchains/leanprover--lean4---v4.33.1/bin` (it is NOT on the default
  shell PATH — use the full path or `~/.elan/bin/elan`).

Goal: prove that we can ask Leanstral (through the vibe lean agent) to write Lean
code, and that the code it writes compiles/runs on our local toolchain.

Steps:
1. Boot headless: `tmux new-session -d -s vibe-lean '<vibe binary> --agent lean'`
   in `.tmp/vibe-lean/` (cwd there so any files it writes land under .tmp).
   Wait for the prompt; capture state with `tmux capture-pane -p -S -200`.
   If the CLI needs interactive auth or hangs >3 min, record the failure and
   FALL BACK to the direct API path (see below). Never type credentials.
2. Drive it: send one prompt asking Leanstral to write a single self-contained
   Lean 4 file `.tmp/vibe-lean/leanstral_vibe.lean` with 2–3 small theorems
   (e.g. `1 + 1 = 2 := by rfl`, a `Nat` lemma provable by `decide` or `omega`),
   imports from core only, no Mathlib, no `native_decide`.
3. Let it finish (poll capture-pane until idle). Capture the full transcript to
   `.tmp/vibe-lean/transcript.txt`.
4. Compile-check: run the local lean binary on the file; exit 0 = PASS. If it
   fails, send ONE follow-up fix prompt in the tmux session, re-check, then stop.
5. FALLBACK API path (also record even if vibe path works, using a DIFFERENT
   theorem set so we compare): reuse the pattern from `.tmp/mistral-lean/ask_lean.py`
   (model `labs-leanstral-1-5-1`, key read SILENTLY via
   `awk -F= '/^MISTRAL_API_KEY/{print $2}' .env` — never print or store the key,
   never copy .env). Output `.tmp/vibe-lean/leanstral_api.lean`, compile-check it.
6. Write `.tmp/research/leanstral-vibe-smoke.md`: vibe vs API verdicts, per-file
   compile results, exact commands.

Report back (<6 lines, no content dumps): vibe path ok/blocked, API path ok,
files compiled y/n per source, output paths. Never echo the key.
