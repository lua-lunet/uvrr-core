# Rung 28: the Telescoping Theorem — the first transition passes ⟹ the second does

*2026-09-14T14:02:36Z by Showboat 0.6.1*
<!-- showboat-id: 5a4c4083-0d8b-4e3f-88d8-3d004e2137b5 -->

Evidence recorded 14 September 2026 against the source based on `b0bf2f9`.

The target is `Fuse.Telescope.telescope_pass` in `UVRR/Fuse.lean`. In a
three-node cluster whose reconfiguration schedule has two transitions — the
cluster states `E0 → E1 → E2` — under the fuse constraints: if `E0 → E1` will
pass, then `E1 → E2` will also pass. This is the Telescoping Theorem: the
first transition's acceptance telescopes the remaining slots of the fused
schedule.

The argument is grounded in the fuse machinery's own definitions. The batch is
one datagram (`docs/uvrr-fuse.md` §2, the atomic batch property): a node
processes the whole slab before reading any other node's message, so the batch
cannot be interrupted — all pass or all fail, and one response set `R`
acknowledges every packed slot. The fuse header ballot is both Phase 1 and
Phase 2 for every command in the fused batch, so the same replies count at
every slot; a majority on the first transition is a majority on every
transition, with the same outcome. The theorem states this as a quorum family
per era (`ScheduleFamily`, the weighted strict majority of each era's
configuration) with a base case at `E0` and a preservation step across
`E0 → E1`, and concludes eligibility at every slot `i ≤ 2` — the finite
induction over the batch the atomicity argument licenses, mirroring Lamport's
Paxos Made Simple induction (one leader, one ballot, all future slots, until
interrupted) as stated in the paper's addendum (`paper/fuse_ladder.tex`,
Theorem fuse telescoping).

The proof at three nodes is the two-step unfold of rung 27's `eligible_through`
pattern: the base quorum and the single preservation step chain to slots `1`
and `2`; `i ≤ 2` case-splits to three values. This is the same inductive logic
that rung 27's five-voter schedule carries over six boundaries; the
three-node two-transition statement is the instance the paper's three-voter
paragraph fixes.

## Generation and verification

The driver was run with model `labs-leanstral-1-5-1`:

```sh
scripts/leanstral.sh prove tryT1.lean 3
```

where `tryT1.lean` held the theorem statement with a `sorry` body plus the
library signatures the model needed (`eligible_through` as the inductive
pattern; rung 8's documented skeleton practice). The first three-round loop
failed all three rounds: the candidate chained the preservation step
correctly but ended `exact hi`, reusing the bound as the family membership
goal (a type error, not a proof-shape error). Two further two-round loops
first produced `interval_cases` (not in plain Lean 4.33.1's core — the
documented unqualified/Mathlib-habit quirk), then settled: the candidate
replaced `interval_cases` with an `omega` trichotomy and `rcases`, which
compiled clean. The statement and imports were byte-identical to the
submitted target throughout; only the `sorry` body changed. The negative
control (`Fuse.Telescope.swap_control`, the paper's equal-total swap
`(1,2,1,2) → (2,1,2,1)`, where `BD` is a majority at `E0` and a minority at
`E1`) was written by hand and is hand-landed, as is the final integration
into `UVRR/Fuse.lean` under `namespace Fuse.Telescope`; the theorem proof
body is Leanstral-drafted, kernel-checked unchanged.

Replay from `formal/uvrr-lean/`:

```sh
lake build
python3 check_axioms.py
```

Replay after integration completed successfully: 27 Lake jobs; the axiom
audit covered 450 declarations and found only the permitted standard Lean
axioms (`propext`, `Quot.sound`, `Classical.choice`). Direct query:

```sh
printf '%s\n' 'import UVRR' \
  '#print axioms Fuse.Telescope.telescope_pass' \
  '#print axioms Fuse.Telescope.swap_control' | lake env lean --stdin
```

both report `depends on axioms: [propext, Classical.choice, Quot.sound]`.

The Rust corpus is untouched by this rung (`cargo test --features maelstrom`
green, 33 suites; no `#[cfg(test)]` under `src/`).

## Scope

The theorem assumes the schedule's quorum family is preserved across both
boundaries (the certified-schedule hypothesis of the paper's Theorem fuse
telescoping). It does not claim that the first transition passing under one
quorum telescopes the remaining slots on a schedule that switches the quorum
family wholesale — the negative control shows that claim fails, as the
paper's equal-total swap analysis requires. The acceptance certificates, the
local fold refinement (§3 of `docs/uvrr-fuse.md`), and the era-history
agreement obligations remain the separate rungs 25–27 subjects; the
telescoping theorem alone does not perform phase one, as the addendum
records.
