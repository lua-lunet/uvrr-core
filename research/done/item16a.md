# item16a — item16 re-attempt remainder: gates, staging, report

**Follow-on work to item16** (not newly discovered scope). Item16 was delegated and
a fair attempt was made: the delegated agent wrote all five tests into
`tests/recovery.rs` but the session was interrupted by a user stop before it ran
the lint gates, staged the file, or reported. The outer agent has already verified
the test state on unchanged code (see Evidence below). This item closes out only
what remains.

Read `.tmp/_brief.md` first — binding rules, git protocol, verification gate, and
the multi-nonce recovery pivot section. Read `.tmp/item16.md` for the original
specification of the tests.

Repo: `/Users/Shared/lua-lunet/vrr-core`

## State at handover

`tests/recovery.rs` working tree (unstaged) contains, beyond the pre-existing
tests: `recovery_host_redrive_invalidates_inflight_response`,
`cross_nonce_responses_complete_quorum`, `mixed_nonce_responses_are_accepted`,
`evicted_nonce_responses_are_stale`, `recovery_view_fence_respected_across_nonces`.
`stale_nonce_responses_are_ignored` is removed (authorized split). No production
file was touched. Nothing is staged for this item yet.

## Evidence (outer agent, unchanged production code, `cargo test --test recovery`)

`test result: FAILED. 13 passed; 4 failed` — the four failures are exactly the
specified Red set, each `left: Recovering, right: Normal`:

- `recovery_host_redrive_invalidates_inflight_response` (tests/recovery.rs:305)
- `cross_nonce_responses_complete_quorum` (tests/recovery.rs:348)
- `mixed_nonce_responses_are_accepted` (tests/recovery.rs:395)
- `recovery_view_fence_respected_across_nonces` (tests/recovery.rs:505)

`evicted_nonce_responses_are_stale` passes (guard test, as the brief predicted).
All 12 pre-existing tests pass, including the 3 staged `resolved unreproducible`
reproductions. Recovery suite count: 13 → 17 (+5 new, −1 split).

The stall claim is therefore **confirmed, not unreproducible**: item17 proceeds.

## Remaining scope (all of it)

1. `cargo fmt --check` — if it flags only the new test code, `cargo fmt` is
   permitted on `tests/recovery.rs` alone; re-run until clean.
2. `cargo clippy --all-targets --all-features -- -D warnings` — fix only issues in
   the new/changed test code; any complaint about production code: stop and report,
   do not edit production.
3. Re-run `cargo test --test recovery` after any format/lint touch and confirm the
   same 13-pass/4-fail Red profile (the Red must survive formatting).
4. `git add tests/recovery.rs`. Never commit. Never stage anything under `.tmp/`.
   No git reset/checkout/stash.

## Report back, verbatim

fmt and clippy output, the post-touch test profile, `git status --porcelain`,
follow-ons found but not done, any deviation from this brief with the reason.
