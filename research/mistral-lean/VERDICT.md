# VERDICT — Mistral labs-leanstral-1-5-1 on a Lean 4 hello-world theorem

- **Model:** `labs-leanstral-1-5-1` (temperature 0, top_p 1 forced by API, single shot)
- **Prompt summary:** asked for a complete, self-contained Lean 4 file proving
  `theorem two_plus_two : (2 : Nat) + 2 = 4` using tactics, core library only,
  exactly one fenced lean code block.
- **Result:** produced a valid-looking 4-line file: `import Init`, the theorem
  statement, and `:= by native_decide`. Single theorem, no prose outside the
  block, correct statement syntax.
- **Lean check:** not compiled — `lean`, `elan`, and `lake` are all absent from
  PATH and the spec forbids installing a toolchain. Sanity check only: PASS
  (non-empty, exactly one theorem, tactic proof, no surrounding prose).
- **Notes / model quirks:**
  - First request was rejected with HTTP 400, code 3054: "top_p must be 1 when
    using greedy sampling." The endpoint does not accept `temperature: 0`
    alone; the request was resent with `top_p: 1` and succeeded.
  - The model chose `native_decide` rather than a kernel-checked tactic like
    `decide` or `rfl`. `native_decide` relies on compiled evaluation rather
    than the kernel, so this is a working but weaker style of proof; it is
    expected to compile with plain `lean` but this was not verified.
  - No timeouts, no truncated output, exactly one code block in the response.

Compilation status: **tbd** (no local Lean toolchain available).
