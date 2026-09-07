# lean-upaxos — baby step 1 (item17)

Date: 2026-09-05. Lean 4.33.1 (`~/.elan/toolchains/leanprover--lean4---v4.33.1/bin/lean`), no Mathlib, no lake project, plain `lean`.

## Deviation from spec (stated, not hidden)

Spec assumed `Finset` is in Lean core. It is not: this toolchain's core/Std contain
no `Finset` (verified by source search), and the locally cached Batteries
(`.tmp/lean-solve-aesop/.lake/packages/batteries`) predates Mathlib's Finset move.
Building Mathlib was out of the 40-min timebox and would break the empty-dependency
rule. Quorums are therefore `List Node` and `Overlap` is a decidable List
computation, so concrete facts close by kernel-checked `decide`. The blog sources
under `.tmp/blogs/` were not delivered (img only), so definitions follow the spec
summary.

## Definitions (Upaxos.lean)

- `Era := Nat`; ballot era `eOf (b : Era × Nat) := b.1` with `eOf_eq : eOf b = b.1`.
- `Config := ⟨QI, QII : List Node⟩` (the ⟨Q_I, Q_II⟩ pair).
- `Overlap Q1 Q2 : Prop := Q1.any (fun x => x ∈ Q2) = true` (decidable stand-in for `(Q1 ∩ Q2).Nonempty`).
- `SoundCfgs C` : the three era-adjacent overlap invariants —
  `∀ e, Overlap (C e).QI (C e).QII`, `∀ e, Overlap (C e).QI (C (e+1)).QII`,
  `∀ e, Overlap (C (e+1)).QI (C e).QII`.
- `FixableIn b s := s = b.1 ∨ s = b.1 + 1` (a ballot fixes its vote in its own or the next era).

## Lemmas proven (6, all kernel-checked; only propext/Quot.sound)

- `eOf_eq (b) : eOf b = b.1` — ballot-era typing, `rfl`.
- `cfgE_ge2 (n) : cfgE (n+2) = ⟨[0,1,3],[0,1,3]⟩` — wildcard-branch reduction, `rfl`.
- `cfgE_sound : SoundCfgs cfgE` — the 3-zone upgrade schedule satisfies all three
  invariants for every era, by `cases` + `decide` (the demo that adding then
  deleting a node with correct quorums never loses overlap).
- `fixable_le (b s h) : b.1 ≤ s` — fix-era monotonicity (typing lemma, NOT vote safety).
- `bad_no_overlap : ¬ Overlap bad.QI bad.QII` for `bad := ⟨[0],[1]⟩` — negative control.
- `bad_not_sound : ¬ SoundCfgs (fun _ => bad)` — the invariant itself rejects corruption.

## 3-zone instance

`cfgE 0 = ⟨[0,1,2],[0,1,2]⟩` (zones A,B,C), `cfgE 1 = ⟨[0,1,3],[0,2,3]⟩` (Z=3 added,
two nodes in A), `cfgE e≥2 = ⟨[0,1,3],[0,1,3]⟩` (C dropped). `cfgE_sound` covers
same-era and both cross-era directions for all eras.

## Compile command

```
~/.elan/toolchains/leanprover--lean4---v4.33.1/bin/lean Upaxos.lean
```

## Fix rounds

- Leanstral (labs-leanstral-1-5-1): 2 rounds. Round 1 draft used `Finset` + no
  import → uncompilable in this toolchain. Round 2 (List-based prompt) compiled
  but used forbidden `native_decide` and had unused plumbing → rejected.
- Agent: 3 rounds. (1) rewrite: `SoundCfgs`, header, negative control lifted to
  invariant level, `decide` instead of `native_decide` → synthesis errors.
  (2) `cfgE_ge2` + `unfold`/`rw` → 1 error left. (3) add `unfold Overlap` → clean.
