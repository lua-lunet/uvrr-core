# lean-solvers-cli-omega-aesop (item14a) — macOS Darwin, zsh, Lean 4.33.1 via elan

Setup: `~/.elan/toolchains/leanprover--lean4---v4.33.1/bin` was NOT on this shell's PATH; every
command used `export PATH="$HOME/.elan/toolchains/leanprover--lean4---v4.33.1/bin:$PATH"` first.
Note: the spec's sample statement `2*x = 4*x - 2 → x = 2` is false (solution is x = 1), and
`a ≤ b → b ≤ c → a + c ≥ b + b` is false (a=0,b=2,c=2); corrected equivalents were used and
omega rejected the originals, confirming it does not accept false statements.

## Track 1 — omega (built into `lean`, zero install) — DONE
File: `.tmp/lean-solve-omega/solve.lean`
Command: `lean solve.lean` (exit 0 when only-no-control; with control present exit 1 from control line only)

| theorem | statement | solved | wall |
|---|---|---|---|
| omega_t1 | `(x : Int) : 2 * x = 4 * x - 2 → x = 1` | y | 0.17s |
| omega_t2 | `(n : Int) (h : n % 2 = 0) : 2 ∣ n` | y | 0.13s |
| omega_t3 | `(a b c : Nat) : a ≤ b → b ≤ c → a ≤ c` | y | 0.13s |
| omega_t4 | `(x : Int) (_h : x ≥ 0) (h2 : x + x = 6) : x = 3` | y | 0.13s |
| omega_neg (control) | `∀ m : Nat, m + n = n + m` (induction) | n (correct) | 0.14s |

Negative control behaves: YES — `lean` prints `error: omega could not prove the goal` and exits 1;
it never claims success.

## Track 2 — aesop (git dep, no Mathlib) — DONE (no timebox overrun)
Dir: `.tmp/lean-solve-aesop/` (lake project, `lakefile.lean` requires aesop @ tag `v4.33.0`,
pulled batteries; aesop's `lean-toolchain` pins v4.33.0 vs local 4.33.1 — compatible).
Commands: `lake init aesopdemo`; `lake update` (13.3s); `lake build aesop` (19.3s wall, 170 jobs);
then `lake build`; per-theorem: `lake env lean <file>`.

| theorem | statement | solved | wall |
|---|---|---|---|
| aesop_t1 | `(p q : Prop) : p ∨ q → q ∨ p` | y | 0.59s |
| aesop_t2 | `(p q : Prop) : (¬ p ∨ ¬ q) → ¬ (p ∧ q)` | y | 0.58s |
| aesop_t3 | `(a b c : Prop) : (a → b) → (b → c) → a → c` | y | 0.57s |
| aesop_t4 | `(p q r : Prop) : (p → q → r) → (p ∧ q) → r` | y | 0.58s |
| aesop_neg (control) | `(n : Nat) : n ^ 2 + n + 41 = 43 * n → n = 2` | n (correct) | 0.59s |

Negative control behaves: YES — `aesop: failed to prove the goal after exhaustive search`,
`unsolved goals`, build fails; never claims success. Also rejected `¬ (p ∧ q) → ¬ p ∨ ¬ q`
(correctly — not intuitionistically provable); replaced with the valid de Morgan direction.

## Conclusion
Both CLI solvers demonstrated taking statement-only theorems (`by omega` / `by aesop` as the
entire proof script) and returning checked proofs. 8/8 positive theorems solved; 2/2 negative
controls failed cleanly.
