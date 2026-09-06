# item18b — Owner ruling: within-life duplicate `Apply` at completion after fast-forward

**Follow-on work to item18a** (which reproduced the behavior and stopped per its
brief). The tree is intentionally Red by exactly one preserved reproduction;
do not milestone-commit until this is resolved.

Read `.tmp/_brief.md` first. Repo: `/Users/Shared/lua-lunet/vrr-core`.

## The reproduced behavior (Red, preserved)

`completion_before_acknowledgement_does_not_reemit_fast_forwarded_upcalls`
(`tests/recovery.rs`, staged): a recovering node fast-forwards `committed` 2→5 and
emits `Apply{3,4,5}`; the host does NOT acknowledge; the completing response then
lands. The completion's replay walk seeds from `applied == 2` and emits
`Apply{3,4,5}` a SECOND time — within the same life, no crash between emissions.

Red output (verbatim):

```
exactly-once: no second Apply for an unacknowledged fast-forwarded slot:
[Apply { slot: Slot(3), .. }, Apply { slot: Slot(4), .. }, Apply { slot: Slot(5), .. }]
test result: FAILED. 22 passed; 1 failed
```

This also contradicts the doc claim at `src/replica/recovery.rs:404-407` ("the
replay walk … never re-applies a fast-forwarded slot") whenever acknowledgement
lags the completion.

## The design question (owner's ruling)

**RULING RECORDED 2026-08-14: (b) — suppress within a life.** The completion's
replay walk must not re-emit slots already emitted this life by the fast-forward;
the preserved Red test goes Green as written. Add the companion test: crash AFTER
fast-forward but BEFORE acknowledgement still re-emits on reopen (the §11.1
crash-boundary at-least-once semantics survive the suppression). The owner
additionally directed an immediate stale-docs sweep — that is item20, disjoint
from this item.

§11.1's at-least-once boundary exists for NODE crash: emission memory is volatile,
`applied` is durable, so a reopened node re-emits committed-but-unapplied slots.
Here there is no crash — the duplicate happens inside one life. Two candidate
rulings:

**(a) Pin at-least-once within a life.** The host contract tolerates a duplicate
`Apply` for any slot it has not yet acknowledged. Flip the Red test to assert the
duplicate as intended behavior (rename accordingly), and correct the doc claim at
`src/replica/recovery.rs:404-407`. No production logic change.

**(b) Suppress within a life.** The completion's replay walk must not re-emit
slots already emitted this life by the fast-forward. The Red test goes Green as
written. Requires the completion to know the emitted frontier (the fast-forwarded
`committed` is the natural seed: every fast-forwarded slot was emitted this life).
The crash case is unaffected — after a crash the volatile emission memory is gone
and replay correctly re-emits from the durable `applied`.

Note: without fast-forward (pre-93a425f) this within-life duplicate could not
occur — completion was the only emitter. The fast-forward introduced the second
emitter, so the question is genuinely new, not a pre-existing §11 behavior.

## Scope after the ruling

- Ruling (a): tests-only change (flip + rename the staged test) plus the doc
  correction in `src/replica/recovery.rs`. Baseline 211 + 1 = 212 Green.
- Ruling (b): minimum production change in `plan_recovery_completion`
  (`src/replica/recovery.rs`) so the replay walk seeds above slots already emitted
  this life; the Red test goes Green unmodified. Baseline 211 + 1 = 212 Green.
  Add one more test: crash AFTER fast-forward but BEFORE acknowledgement still
  re-emits on reopen (the at-least-once boundary must survive the suppression).
- Then the outer agent runs the full gate and lands the closing milestone commit.

## Constraints

- Reproduction is preserved Red in the tree; do not delete it except by flipping
  it under ruling (a).
- The invariant and quorum gates stay closed.
- `git add` touched files; never commit; never stage `.tmp/`; no stash/reset.

## Report

The ruling quoted, Red→Green output (or the flipped Green under (a)), exact count
delta with names, gate output verbatim, `git status --porcelain`, follow-ons not
done.
