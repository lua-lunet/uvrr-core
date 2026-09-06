# item14a — CLI solvers: omega (core) + aesop (dep), macOS

Repo root: /Users/Shared/lua-lunet/vrr-core. Lean 4.33.1 toolchain already on PATH
(elan). TIMEBOX ~20 min: skip any track that fights tooling, record why.

Goal: demonstrate feeding Lean theorem STATEMENTS (no proof script) to CLI
solvers and getting proofs back.

Track 1 — omega (built into the `lean` binary, zero install):
- `.tmp/lean-solve-omega/`: file(s) with 3+ stated-but-unproved linear/integer
  arithmetic theorems, each closed by `by omega` (e.g. `∀ x : Int, 2*x = 4*x - 2 → x = 2`,
  a divisibility/parity one, a nat triangle one).
- Run `lake env lean solve.lean` (a plain dir is fine: `lean solve.lean`); exit 0
  with no errors = solver solved them. Record one NEGATIVE control too (a theorem
  omega cannot do, e.g. needing induction) showing it reports failure rather
  than claiming false success.

Track 2 — aesop (separate dep, no Mathlib):
- `.tmp/lean-solve-aesop/`: minimal lake project, `require aesop from git
  "https://github.com/leanprover-community/aesop"` (check its README for the
  correct tag for Lean 4.33; batteries dep is expected — if the build exceeds
  the timebox, skip and record).
- Same shape: 3+ statement-only theorems closed by `by aesop` (propositional
  case-split ones, e.g. `p ∨ q → q ∨ p`-style, a simp-free de Morgan one) plus
  one negative control (e.g. a Diophantine equality aesop should NOT solve,
  stated via `by aesop` — confirm it fails cleanly).
- Build only aesop + deps, not a full Mathlib.

Write results to `.tmp/research/lean-solvers-cli-omega-aesop.md`: per theorem —
statement, solver, solved y/n, wall time. Note exact CLI commands used (this is
a macOS Darwin box, zsh).

Report back (<6 lines, no content dumps): tracks done/skipped, theorems solved
per solver, negative controls behaved y/n, output paths.
