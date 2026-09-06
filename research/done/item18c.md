# item18c — Within-life duplicate `Apply` suppression (re-attempt, widened scope)

**Follow-on work to item18b** (second attempt). Item18b stopped at a scope
boundary after PROVING the suppression seed is not computable from state
reachable in `src/replica/recovery.rs`: the crash-after-fast-forward completion
and the Red-test completion observe byte-identical durable state yet require
opposite emissions, so the seed must be volatile state discarded on crash. The
owner's ruling (b) stands and the owner has directed the "trade up": recovery
reuses the normal path's frontier-delta idempotence (item21 proves the normal
half). This item implements it.

Read `.tmp/_brief.md`, `.tmp/item18b.md` (ruling recorded), and
`.tmp/item18a.md` (the Red evidence) first. Repo: `/Users/Shared/lua-lunet/vrr-core`.

## GATING PRECONDITION

Item21 must have recorded the normal path duplicate-safe. If item21 found a Red
outside recovery, STOP — the owner is re-ruling the premise.

## Design (traced by the item18b agent against all pinned scenarios)

One volatile field on `RecoveryVolatile` (`src/replica/mod.rs`): the `committed`
frontier AT ATTEMPT OPEN, set in `plan_recover`'s fresh-start arm, preserved
across re-drives (the existing `open.clone()` path), cleared on completion,
discarded on crash (volatile by design — no durability change).

`plan_recovery_completion` then emits the `Apply` effects as the UNION of two
ranges:

1. `(applied, attempt_open_committed]` — the durable debt: slots committed
   before the attempt opened, never emitted this life (a fenced node emits
   nothing while `Recovering`), possibly emitted in a prior life — the §11.1
   crash-boundary replay.
2. `(local_committed, final_committed]` — the newly installed range the
   completion itself advances.

Slots in `(attempt_open_committed, local_committed]` are exactly what the
fast-forward emitted this life — skipped, which turns the preserved Red test
Green AS WRITTEN. The `applied` walk and all acknowledgement bookkeeping are
untouched: the host still owes `Input::Applied` per slot and the node waits in
`Replaying` for them.

Traced outcomes (verify each in tests): the Red test emits nothing at
completion; `completion_below_committed_replays_until_caught_up` (no
fast-forward, durable debt 4–5) still emits `Apply{4,5}`; a fast-forward on top
of durable debt emits only the debt; crash-after-fast-forward-before-ack reopens
with the field gone and re-emits the full debt.

## Scope

1. `src/replica/mod.rs` — the `RecoveryVolatile` field, its doc comment
   (volatile, this-life emission boundary; terse, §11.1-cited), and its
   preservation/clearing in the `RecoveryUpdate` paths.
2. `src/replica/recovery.rs` — set the field in `plan_recover`; two-range
   emission in `plan_recovery_completion`; correct the doc claim at
   ~lines 404-407 to state the suppression precisely (emitted-this-life slots
   are not re-emitted; crash re-emission from durable `applied` unchanged).
3. `tests/recovery.rs` — ONE new companion test: crash AFTER fast-forward but
   BEFORE acknowledgement, reopen, recover, complete — the unacknowledged slots
   ARE re-emitted (crash boundary survives suppression). The staged Red test
   (`completion_before_acknowledgement_does_not_reemit_fast_forwarded_upcalls`)
   goes Green unmodified; do not touch it.

## Constraints

- Sources: `src/replica/mod.rs`, `src/replica/recovery.rs` ONLY. Tests:
  `tests/recovery.rs` ONLY. Anything else: STOP and report.
- The invariant and quorum gates stay closed.
- Every pre-existing test stays Green with byte-for-byte expectations.
- Unstaged `Makefile`, `README.md`, `.dockerignore`, untracked `formal/` and
  `Dockerfile.tla` are parallel agents' — never touch or stage.
- `git add` your three paths only. Never commit. No stash/reset/checkout.
- Baseline: 211 committed Green + 1 staged Red + item21's additions. State your
  delta (expect +1 companion test).

## Report

Red→Green for the preserved test, the companion test name and what it pins, the
per-scenario trace verification, exact count delta, full gate output verbatim,
`git status --porcelain`, follow-ons not done.
