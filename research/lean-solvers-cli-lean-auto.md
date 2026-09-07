# lean-auto + Duper CLI run (item14b)

Setup: Lean 4.33.1 via `~/.elan/bin` (elan toolchain `leanprover/lean4:v4.33.1`).
Project `.tmp/lean-solve-auto/` (path deps on `tools/lean-auto` submodule + Duper clone
`.tmp/lean-solve-auto/duper`, whose lakefile was edited: `auto` git require -> path
`"../../tools/lean-auto"`; root require named `auto` to dedupe with Duper's). Duper backend
bound via README's `Auto.duperRaw` rebind of `Auto.Native.solverFunc` (`Solve/SetDuper.lean`),
`set_option auto.native true`. Statement-only theorems, tactic `auto [...]`, no proof scripts.
Toolchain pin note: submodule and Duper pin v4.33.0; built under v4.33.1 root toolchain
without issue (path deps use root toolchain). External-solver modes exist in lean-auto
(SMT `auto.smt`, TPTP zipperposition) but were NOT configured/used; built-in Duper only.

## Per-theorem results

| Theorem (file) | Statement | Tactic | Solved | Wall time |
|---|---|---|---|---|
| T1 `Solve/T1LeftInv.lean` | right inverse is left inverse in a group (FO axioms in context) | `auto [assoc, mul_e, inv_r]` | yes (exit 0) | ~1.4s |
| T2 `Solve/T2Comm.lean` | commutativity from full group axioms | `auto [assoc, mul_e, e_mul, inv_r, inv_mul]` | no (Duper terminated: 500s saturation timeout; retried at maxHeartbeats 8e6, still failed) | ~500s |
| T3 `Solve/T3Relation.lean` | FO relation: sym+trans entails `r c a` from `r a b`, `r b c` | `auto [sym, trans, h1, h2]` | yes (exit 0) | ~1.1s |
| NEG `Negative.lean` | `n + m = m + n` on Nat (needs induction) | `auto` | correctly failed: "Duper saturated", elaboration error (exit 1), ~1.3s | ~1.3s |

## Exact CLI commands (zsh, macOS)

```
export PATH="$HOME/.elan/bin:$PATH"
lake build                                            # deps + lib (initial: 2:01; 2nd full attempt 2:35)
lake build Solve.SetDuper Solve.T1LeftInv Solve.T3Relation   # replay, ok
lake build Solve.T2Comm                               # failed, 8:22 (500s Duper timeout)
lake env lean Solve/T1LeftInv.lean; echo "T1 exit=$?"  # exit=0
lake env lean Solve/T3Relation.lean; echo "T3 exit=$?" # exit=0
lake env lean Negative.lean; echo "NEG exit=$?"        # exit=1 (expected)
```

Note: initial build hit duplicate-package error (`lean_auto` vs `auto` require names);
fixed by naming the root require `auto`. No external solvers installed or invoked.
