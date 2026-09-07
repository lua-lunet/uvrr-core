# item18 — Committed fast-forward in recovery

Read `.tmp/_brief.md` first — binding rules, git protocol, verification gate, and the
**multi-nonce recovery pivot** section (the approved design). This file states only
what is specific to this item.

Repo: `/Users/Shared/lua-lunet/vrr-core`
Depends on: item17 (multi-nonce recovery, Green).

## Objective

While `Recovering`, an accepted `RecoveryResponse` whose `committed` exceeds the
local committed frontier fast-forwards it: the node applies the
sequentially-adjacent, locally journal-present entries above its committed frontier
— the same ordered `Apply` upcalls it would have emitted had it never crashed —
and advances `committed`/`applied` accordingly. Duplicate or overlapping responses
are idempotent: slots at or below the frontier are skipped.

## Scope

1. A planner-shaped helper (pure, `&self`, returning candidate/effects — composed
   into the caller's `PlannedTransition`; never `&mut self`) that walks
   takeWhile-style from `committed + 1` while the journal physically holds each
   next slot, reusing the existing `apply_effects_merged` / `applied_walk`
   machinery. Stops at the first gap. Emits the ordered `Apply` effects exactly
   once per slot.
2. `plan_recovery_response`: after a response is accepted (nonce in set, all
   existing rulings unchanged), if `evidence.committed > progress.committed()`,
   compose the helper's fast-forward into the same transition.
3. Monotonicity guard: `recovery_candidate` takes
   `max(evidence.committed, progress.committed())` (and likewise `applied`) so a
   completion after a fast-forward never moves a frontier backward. The
   completion's replay walk already starts from `applied`, so fast-forwarded
   slots are never re-applied — assert that.

## Red (new tests in `tests/recovery.rs`)

- Fast-forward emits the ordered `Apply` upcalls for locally-present committed
  entries.
- The same response delivered twice emits each upcall once.
- A response claiming `committed` beyond a local journal gap applies only up to
  the gap.
- A completion after fast-forward does not re-apply fast-forwarded slots.
- A completion whose evidence `committed` is below the fast-forwarded frontier
  does not regress `committed`.

## Constraints

- Primary-only suffix installation (§6.1) is unchanged; this item touches only
  committed-frontier advancement and its application.
- The invariant and quorum gates stay closed.
- Sources: `src/replica/recovery.rs`, `src/replica/mod.rs`. Tests in
  `tests/recovery.rs`. Anything else: stop and ask.
- `git add` touched files; never commit.

## Report

Red→Green output per test, exact test count delta with names, full gate output
verbatim, `git status --porcelain`, follow-ons found but not done.
