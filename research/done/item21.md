# item21 — Normal-operation duplicate-delivery idempotence

**Newly discovered work** (owner directive 2026-08-14): the within-life duplicate
`Apply` seen in recovery (item18a observation 2) raises the question of whether
the NON-recovery path is duplicate-safe. The owner's reasoning: if normal
operation already tolerates repeated messages without double-emitting `Apply`,
then recovery — which reuses that machinery while fenced — should be tradable up
to the same guarantee. This item proves the normal-path half. Reproduction-first:
the likely outcome is a Green characterization test that STAYS as coverage.

Read `.tmp/_brief.md` first. Repo: `/Users/Shared/lua-lunet/vrr-core`.

## Objective

Prove, through the public harness, that in NORMAL operation (no recovery
involved) repeated/duplicate protocol messages never cause a slot's `Apply`
upcall to be emitted twice within a life.

## Tests (tests/normal_operation.rs unless the file's layout says otherwise)

Cover the duplicate-delivery matrix a lossy/reordering transport actually
produces:

1. **Duplicate `Commit` to a backup.** Drive a normal commit (primary proposes,
   backups accept, primary commits, `Commit` broadcast); deliver the same
   `Commit` (same view, same committed frontier) to a backup a second time.
   Assert: no second `Apply` for the slot; the frontier does not move; the
   duplicate is a named no-op or a silent no-effect transition — whichever the
   code already does, pin it.
2. **Duplicate `Prepare` to a backup.** Same slot, same entry, delivered twice.
   Assert: no duplicate `Apply`, no duplicate `PrepareOk` if that is what the
   code already guarantees — pin the actual behavior.
3. **Duplicate `PrepareOk` at the primary.** The same backup's `PrepareOk`
   delivered twice for one proposal. Assert: commit fires once, `Apply` once.
4. **Reordered duplicates across an advancing frontier.** Two commits land;
   then the FIRST commit's `Commit` message is redelivered (stale frontier).
   Assert: nothing re-emitted, frontier unmoved.
5. **Duplicate at the applied boundary.** Host acknowledges `Input::Applied`
   for a slot; a duplicate `Commit` for that slot arrives afterwards. Assert:
   no re-emission.

If the harness cannot redeliver a consumed message, use the existing `inject`
pattern; if neither can express a scenario through the public interface, report
that rather than adding a test-only API.

## Rulings

- **All Green on unchanged code:** the normal path is confirmed duplicate-safe;
  the tests stay as pinned coverage. Record `verified already-correct` per
  scenario with the output. This UNBLOCKS item18c (recovery suppression reusing
  the same frontier-delta idempotence).
- **Any Red:** a real defect outside recovery. STOP. Preserve the Red output
  verbatim. Do not fix. The owner rules next — this would downgrade the item18b
  ruling's premise.

## Constraints

- Tests only. No production edits.
- The tree carries one intentionally staged Red test in `tests/recovery.rs`
  (`completion_before_acknowledgement_does_not_reemit_fast_forwarded_upcalls`)
  awaiting item18c. Do not touch it, do not "fix" it, and do not be alarmed by
  it in full-suite runs — report your own suites' results and note the known Red.
- Unstaged `Makefile`, `README.md`, `.dockerignore` and untracked `formal/`,
  `Dockerfile.tla` belong to parallel TLA+ agents — never touch or stage them.
- `git add` only your test file. Never commit. No stash/reset/checkout.
- Baseline: 211 committed Green + 1 staged Red + (your additions). State your
  delta.

## Report

Per-scenario verdict with unchanged-code output quoted, exact count delta with
names, gate output verbatim (`cargo fmt --check`, clippy, the three greps),
`git status --porcelain`, follow-ons not done.
