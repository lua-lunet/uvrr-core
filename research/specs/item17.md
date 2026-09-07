# item17 — Baby step 1: UPaxos leader-casting-vote overlap in Lean, Leanstral-drafted

Repo root: /Users/Shared/lua-lunet/vrr-core. Platform: darwin, zsh.
Lean 4.33.1 via elan at `~/.elan/toolchains/leanprover--lean4---v4.33.1/bin`
(NOT on shell PATH — use full path). TIMEBOX ~40 min.

Goal (baby step ONLY — no liveness, no full Paxos safety): state the UPaxos
era/quorum structure in Lean and prove the smallest non-vacuous safety facts
about the leader-casting-vote overlap scheme, so we have a compilable seed the
next baby steps can grow. No Mathlib — Lean core (Finset) only.

Drafting: use the Leanstral API (pattern from `.tmp/mistral-lean/ask_lean.py`,
model `labs-leanstral-1-5-1`; key read SILENTLY via
`awk -F= '/^MISTRAL_API_KEY/{print $2}' .env` — never print/copy/grep the key).
Prompt it for definitions + lemmas; then YOU compile-fix iteratively (record how
many fix rounds the model needed vs you).

Scope (from the user's UPaxos material — see `.tmp/blogs/` if item16 has
delivered, else work from the summary below):
- `Era := Nat`; ballot `(b : Era × Nat)` with `e⟨b⟩ = b.1`; slot era `e⟨s⟩`.
- `Config := Finset Node × Finset Node` (⟨Q_I, Q_II⟩), `Overlap Q1 Q2 :=
  (Q1 ∩ Q2).Nonempty`.
- Safety-of-structure invariant: `∀ e, Overlap (Q_I e) (Q_II e)`,
  `Overlap (Q_I e) (Q_II (e+1))`, `Overlap (Q_I (e+1)) (Q_II e)`.
- The 3-zone upgrade scenario (zones A,B,C; temporarily add Z → four nodes with
  two in one zone, then drop C): give a concrete instantiation of configs
  ⟨Q_I,Q_II⟩ for eras e, e+1, e+2 and PROVE all overlap facts by
  `decide`/Finset computation. This is the demo that "adding then deleting a
  node with correct quorums never loses overlap".
- Leader-casting-vote micro-fact: state `FixableIn (b : Era × Nat) (s : Era) :=
  s = b.1 ∨ s = b.1 + 1` and prove the trivially-true-but-stated fact that a
  ballot's fixable slots satisfy `b.1 ≤ s` (monotonicity of the fix-era bound,
  matching `e⟨s⟩ ≥ e⟨b⟩`). Keep honest: this is a typing/construction lemma,
  NOT the full Paxos agreement proof — say so in a header comment.
- One negative control: a config pair with disjoint quorums where `Overlap`
  FAILS to hold — recorded as unprovable (the invariant rejects corruption).

Work in `.tmp/lean-upaxos/` (a dir with one or two `.lean` files compiled via
`<elan lean path> Upaxos.lean`; lake project optional, plain lean is fine).

Write `.tmp/research/lean-upaxos-babystep1.md`: definitions given, lemmas proven
(name + one line each), negative control, fix-round counts (Leanstral vs you),
exact compile commands.

Report back (<6 lines, no content dumps): compiled y/n, lemmas proven count,
scenario instance verified y/n, fix rounds (model vs agent), output paths.
Never echo the key.
