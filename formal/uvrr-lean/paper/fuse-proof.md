# The fuse telescoping proof: what it is, what it builds on, how to check it

This document exists so any other model (or human referee) can check the proof
without access to our sessions. Everything needed to verify the result is in
the repository; nothing here requires trust in the drafting process.

## The result and its commits

- **Commit on main**: `68e44f1` (PR #32, "The telescoping theorem
  kernel-checked"). The branch head it was drafted and committed at:
  `d0812c3`.
- **Statement**: `Fuse.Telescope.telescope_pass` in
  `formal/uvrr-lean/UVRR/Fuse.lean:174` (namespace `Fuse.Telescope`).
- **Rung transcript**: `formal/uvrr-lean/ladder/28-fuse-telescoping.md`
  (rung 28, Showboat-verified, source hash `b0bf2f9`).

## The theorem (verbatim)

```lean
theorem telescope_pass {A : Type} (nodes : List A) (eraWeight : Nat → A → Nat)
    (R : NSet A)
    (hbase : ScheduleFamily nodes eraWeight 0 R)
    (hstep : ∀ i, ScheduleFamily nodes eraWeight i R →
      ScheduleFamily nodes eraWeight (i + 1) R) :
    ∀ i, i ≤ 2 → ScheduleFamily nodes eraWeight i R
```

## What it says

In a three-node schedule of two transitions — the cluster states
`E0 → E1 → E2` — if the first transition's quorum condition holds (`hbase`:
`R` is a quorum of era `E0`'s configuration) and every boundary preserves the
quorum family (`hstep`: quorum at era `i` implies quorum at era `i + 1`),
then `R` is a quorum at every state through `E2`. The first transition's
acceptance telescopes the remaining transitions of the fused schedule: `E0 →
E1` passing gives `E1 → E2` passing. This is the finite induction over the
batch that the atomicity argument licenses — the same inductive logic
Lamport states in *Paxos Made Simple* §3 (one distinguished leader, one
ballot defined for all future instances, streaming phase-2 accepts until
interrupted), quoted verbatim in the paper's addendum.

## What it builds upon

1. **The module's own vocabulary.** `ScheduleFamily nodes eraWeight e` is the
   weighted strict majority family of era `e`'s configuration, defined from
   the module's `QSys`/`majority` machinery in `UVRR/Fuse.lean`.
2. **The atomic batch property** (`docs/uvrr-fuse.md` §2, the governing
   design statement). A fuse envelope is one checksummed datagram of
   cache-line scale; the host processes the whole datagram before reading any
   other node's message, so loss or interruption inside a batch is
   impossible. The first operation in the batch decides the whole batch; the
   fuse header ballot is both Phase 1 and Phase 2 for every packed slot; the
   same responses count everywhere; majority is computed on the first
   message in batch.
3. **The paper addendum's theorem statement.** Theorem "Fuse telescoping" in
   `formal/uvrr-lean/paper/fuse_ladder.tex` (from line 47) — the Lean
   premises are byte-for-byte the addendum's: "the certified schedule
   preserves the invariant `R ∈ QII_i` at each step, the first slot has `R ∈
   QII_0`".
4. **Lamport's induction.** L. Lamport, *Paxos Made Simple* (2001), §3 —
   quoted verbatim in the addendum (`fuse_ladder.tex:172-180`, bibitem
   `fuse-paxos`).
5. **Turner's weighted reconfiguration setting** (bibitem `turner`), the
   era-based agreement frame the paper builds on.
6. **The negative control** (what the second premise buys). `swap_control` in
   `UVRR/Fuse.lean` enacts the addendum's equal-total swap
   `(1,2,1,2) → (2,1,2,1)`: `{B,D}` is a majority at `E0` and a minority at
   `E1`. Without boundary preservation the conclusion fails — the necessity
   of `hstep` is visible, not decorative.

## Scope — what it does NOT claim

The certificate is quorum-family preservation: what the addendum's boxed
proof dependency licenses. It does not perform phase one, and it is not a
refinement proof of the executable system (the addendum's own caveat; the
same statement is in rung 28's Scope section). The fuse machinery's Rust
implementation is verified by its own corpus (`tests/fuse.rs`); the Lean
result is the deductive layer over the design statements.

## Provenance of the draft

The proof body was drafted by Leanstral (`labs-leanstral-1-5-1`, the free
Labs endpoint) over the `scripts/leanstral.sh` check/prove loop: five failed
rounds (the documented core-only quirks — a `type` error on the bound, and an
`interval_cases` tactic that does not exist outside Mathlib), pass on round
7 with `omega`/`rcases` replacing the Mathlib habit. The statement, the
negative control, the namespace packaging, and the rung transcript landed by
hand. **The acceptance bar is mechanical, not agreement with the draft**:
`lake build` + the axiom audit. Other models may re-derive the proof from
the premises; their drafts are checked the same way.

## How to check it (repeatable, for any model)

1. Checkout the tag `v0.6.0` (or any commit ≥ `68e44f1`).
2. `cd formal/uvrr-lean && lake build` — green, 27 jobs.
3. `python3 check_axioms.py` — PASS, 450 declarations, only
   `propext`/`Quot.sound`/`Classical.choice`.
4. Direct audit: append `#print axioms Fuse.Telescope.telescope_pass` to
   `UVRR/Fuse.lean` and `lake env lean UVRR/Fuse.lean` — the output is
   `[propext, Classical.choice, Quot.sound]`.
5. Confirm the bar: `rg -n 'sorry|native_decide' UVRR/Fuse.lean` finds
   nothing; `rg -n 'theorem telescope_pass' UVRR/Fuse.lean` finds the
   statement above.
6. Read the rung transcript `ladder/28-fuse-telescoping.md` for the recorded
   evidence blocks and the scope statement.

No external dependencies exist: the ladder's `lake-manifest.json` has
`"packages": []`; all imports are internal (`UVRR.*` only).
