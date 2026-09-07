# leanstral-vibe smoke test (item15)

Date: 2026-09-05. Repo: /Users/Shared/lua-lunet/vrr-core. Lean 4.33.1 via
`~/.elan/toolchains/leanprover--lean4---v4.33.1/bin/lean`.

## Verdicts

| Path | Verdict |
|---|---|
| vibe lean agent | PASS (1 fix round) |
| direct API (`labs-leanstral-1-5-1`) | FAIL (2 attempts) |

## Vibe path

`tmux` is not installed on this machine; the headless driver was instead vibe's
own programmatic mode `-p` (same effect: send prompt, get files, exit):

```
~/opt/mistral-vibe-fork/bin/vibe -p "<prompt>" --agent lean --auto-approve \
  --max-turns 12 --max-price 2.0 --output text   # cwd .tmp/vibe-lean/
```

- Turn 1 (`--max-turns 6`): hit the turn cap before file creation; file absent.
- Turn 2 (`--max-turns 12`): created `leanstral_vibe.lean` (5 theorems).
  First compile FAILED: line 17 `Nat.pred n h` applied `Nat.pred` to two args.
- Fix round (one prompt, per spec): agent repaired the theorem. Recompile: PASS.

Prompt style: "create the file immediately on your first turn, then stop" —
worked; agent writes the file itself via its file tools.

## API path

Pattern reused from `.tmp/mistral-lean/ask_lean.py`; key consumed via
`MISTRAL_API_KEY` env var only (read with awk, never printed/stored).
Different theorem set from the vibe run. Two attempts, temperature 0:

- Attempt 1: response imported `Mathlib` → compile FAIL
  (`unknown module prefix 'Mathlib'`).
- Attempt 2 (prompt hardened to forbid imports): still imported `Mathlib` →
  FAIL. Recorded as FAIL; no further re-rolls.

## Exact commands

```
~/.elan/toolchains/leanprover--lean4---v4.33.1/bin/lean <file>.lean   # exit 0 = PASS
```

## Outputs

- `.tmp/vibe-lean/leanstral_vibe.lean` — vibe lean agent, compiles clean
- `.tmp/vibe-lean/leanstral_api.lean` — API output, does not compile
- `.tmp/vibe-lean/transcript.txt` — vibe programmatic-mode output (3 runs)
- `.tmp/vibe-lean/ask_lean_api.py` — API driver (key from env only)
