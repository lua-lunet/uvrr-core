# item10 — Kick the tires: Mistral labs-leanstral-1-5-1 on a Lean 4 hello-world theorem

Goal: check whether Mistral model `labs-leanstral-1-5-1` can produce a correct
Lean 4 proof for a simple, well-known theorem ("hello world" for Lean 4).

Rules: all work under `.tmp/mistral-lean/`; no git; never read/print `.env`
(silent awk capture only); never print the key; keep API response dumps out of
the final report. Timeboxed: do NOT install a Lean toolchain.

Steps:
1. `command -v lean elan lake` — record whether a Lean 4 toolchain exists.
2. Write `.tmp/mistral-lean/ask_lean.py` (stdlib only): key from env var or
   `.env` (never echoed); POST to `https://api.mistral.ai/v1/chat/completions`
   with `model: "labs-leanstral-1-5-1"`, temperature 0, prompt asking for a
   complete Lean 4 file proving a simple well-known theorem (e.g.
   `theorem two_plus_two : (2 : Nat) + 2 = 4`) using tactics; save raw JSON to
   `response.json`; extract the Lean code block to `candidate.lean`.
3. If `lean` exists, run `lean candidate.lean` and record pass/fail with any
   error output; otherwise do a sanity check only (non-empty, single theorem,
   no prose outside the proof).
4. Write `.tmp/mistral-lean/VERDICT.md`: model, prompt summary, result, lean
   check outcome, notes (JSON errors, timeouts, model quirks).

Report back (<6 lines, no dumps): model used, lean available y/n, proof
compiled y/n/tbd, output dir.
