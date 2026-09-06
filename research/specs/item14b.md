# item14b — CLI solver: lean-auto / Duper ATP, macOS

Repo root: /Users/Shared/lua-lunet/vrr-core. Lean 4.33.1 toolchain on PATH (elan).
Submodule `tools/lean-auto` is already checked out on disk. TIMEBOX ~20 min:
skip on friction, record why.

Goal: demonstrate feeding Lean theorem STATEMENTS (no proof script) to the
lean-auto ATP and getting automated proofs back.

Steps:
1. Read `tools/lean-auto` README + lakefile + `Examples/` (or similar) to find
   the supported usage for the installed toolchain: the `auto`/`duper` tactic
   entry points and how to consume the package from a consumer lake project
   (path dependency `require lean_auto from "../../../tools/lean-auto"` or
   equivalent — verify exact package name from its lakefile).
2. `.tmp/lean-solve-auto/`: minimal lake project with a path dep on
   tools/lean-auto (plus its own declared deps as its lakefile requires).
3. Demo file(s) with statement-only theorems closed automatically:
   - propositional/first-order group-theory style facts the lean-auto paper
     touts (e.g. left/right inverses in a group, commutativity from axioms),
     - a simple first-order relation fact,
   - one negative control (a theorem needing arithmetic/induction the ATP
     should fail on cleanly, recorded as such).
   IMPORTANT: do not copy proofs — only STATEMENTS + `by auto`/`by duper` (or
   whatever the README prescribes). Exit 0 = solved.
4. If toolchain pin mismatch (submodule pins an older Lean): try
   `lake exe cache`? no — just retry build with the required toolchain via
   elan if the version file asks for it (allowed, it is small); if that exceeds
   the timebox, skip and record.
5. Do NOT configure external SMT backends (z3/cvc5); use the built-in Duper
   backend only. Note in the results file that external-solver modes exist.

Write results to `.tmp/research/lean-solvers-cli-lean-auto.md`: per theorem —
statement, tactic used, solved y/n, wall time; exact CLI commands; any
toolchain pin notes.

Report back (<6 lines, no content dumps): build ok y/n, theorems solved,
negative control behaved y/n, output path.
