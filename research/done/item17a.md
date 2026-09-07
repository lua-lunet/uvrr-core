# item17a — Re-author `redrive_with_fresh_nonce_outlives_dead_primary` under pivot semantics

**Follow-on work to item17** (not newly discovered scope). Item17's production
change is staged and turns the four specified Red tests Green, but the staged
`resolved unreproducible` reproduction `redrive_with_fresh_nonce_outlives_dead_primary`
(`tests/recovery.rs:980`) still encodes the replaced single-nonce staleness
semantics at lines 1058–1081: it asserts that a delayed response echoing the first
nonce — from the dead view-1 primary, carrying the genuine view-1 history — is
`StaleRecoveryResponse` and counts toward nothing. Under the approved pivot
(`_brief.md`, multi-nonce section) that nonce is in the attempt's set, so the
response is accepted; the quorum then holds with the view-1 primary's suffix and
the recovery completes into view 1. The item16 authorization named only
`stale_nonce_responses_are_ignored`; this test was not covered. Item17 stopped the
line here rather than touch a frozen expectation.

Read `.tmp/_brief.md` and `.tmp/item16.md` first. Repo:
`/Users/Shared/lua-lunet/vrr-core`.

## BLOCKED — owner authorization required

**RULING RECORDED 2026-08-14: authorized.** The owner was asked (clarification:
re-author under pivot semantics vs amend the pivot) and elected to proceed under
the pivot semantics — the delayed in-set-nonce response counts. This item is
unblocked; the scope below is the work.

This item may not start until the owner rules on the expectation change. The
measured facts for that ruling:

- Scenario: 5 nodes, cluster settles in view 1 (primary n1); n1 dies; n4 reopens
  fenced in view 1. Attempt T1 gathers {n0, n2, n3} — quorum held, no primary
  history, attempt waits. Re-drive T2 gathers {n0, n3}. The dead primary's delayed
  T1 response then arrives with the genuine view-1 suffix.
- Under the pivot the response is accepted: responders {n0, n1, n2, n3} hold the
  `R_g` quorum, latest reported view is 1, n1's response is the view-1 primary's
  suffix — completion installs view 1. Observed: the assertion
  `StaleRecoveryResponse { nonce: T1, attempt: Some(T2) }` fails with
  `left: Some(None)` (accepted, no diagnostic) and n4 becomes `Normal` in view 1.
- Safety reasoning for acceptance: the suffix is the genuine selected history of
  view 1; anything committed in it is preserved by any later view's `F_g ⌢ V_g`
  intersection. A node that completes into view 1 while the cluster fences view 2
  learns view 2 through the ordinary `StartView` path — the same as if the
  response had arrived before the re-drive. The nonce set changes WHEN the
  response counts, never WHAT may install (§6.1 primary-only suffix ruling is
  unchanged).

## Scope once authorized (tests only)

Re-author the tail of `redrive_with_fresh_nonce_outlives_dead_primary` (from the
delayed-response injection at line 1062 onward) so the scenario asserts the pivot
semantics:

1. The delayed T1 response from the dead view-1 primary is accepted (no
   `StaleRecoveryResponse` diagnostic) and completes the recovery into view 1
   with the genuine view-1 history (`Status::Normal`, `current_view == view(1)`,
   journal equals the view-1 history).
2. The view-2 fence proceeds as the test already drives it; n4 — now Normal in
   view 1 — installs view 2 through the ordinary `StartView` path (NOT via a
   recovery completion). Final assertions: n4 in view 2, history matches the
   replacement primary's, `assert_safety()` holds.
3. Rename only if the scenario's meaning changed enough to warrant it; if
   renamed, say so in the report. Keep the first half (lines 980–1056: quorum
   without primary history waits; no tick re-drive; fresh re-drive accepted
   mid-attempt; fresh responses one short of quorum) byte-for-byte in expectation
   — those assertions are pivot-compatible and already Green.

## Constraints

- Tests in `tests/recovery.rs` only. No production edits.
- Do not start before the owner's ruling is recorded in this file or in the
  delegation. If the owner rules the pivot must instead be amended (delayed
  older-view responses must not complete), STOP — that is a design change with
  its own reproduction-first burden, not a test re-authoring.
- `git add tests/recovery.rs`; never commit; never stage `.tmp/`.

## Report

The owner's ruling quoted, the re-authored scenario summary, Red→Green output,
suite counts, gate output verbatim, `git status --porcelain`, follow-ons not done.
