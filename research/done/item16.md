# item16 — Recovery multi-nonce test respecification (Red)

Read `.tmp/_brief.md` first — binding rules, git protocol, verification gate,
test-freeze nuance, milestone-commit rule, and the **multi-nonce recovery pivot**
section (the approved design this item specifies tests for). This file states only
what is specific to this item.

Repo: `/Users/Shared/lua-lunet/vrr-core`
Depends on (all done): recovery, the 3 staged recovery reproductions
(`resolved unreproducible` — keep them intact and passing).

## Objective

Specify the approved multi-nonce recovery semantics as tests in `tests/recovery.rs`.
This item is **tests only**: no production change. Every new/changed test is Red on
unchanged code by construction (the semantics are not implemented yet). Preserve the
Red output verbatim in the report — it is the evidence the production change in
item17 answers.

The tree is Red after this item. Per the milestone-commit rule the outer agent does
**not** commit until item17 turns the suite Green.

## Tests

1. `recovery_host_redrive_invalidates_inflight_response` — the stall reproduction.
   3-node cluster, n2 crashed and reopened. `recover(n2)` (nonce T1); hold n0's
   response; `recover(n2)` again (nonce T2); deliver the T1 response (dropped stale
   on unchanged code); re-drive to T3 before T2's responses land; deliver a T2
   response; assert n2 is still `Status::Recovering` despite full reachability.
2. `cross_nonce_responses_complete_quorum` — n0 answers T1, n1 answers T2; assert
   the combined responses complete the `R_g` quorum and n2 reaches `Normal`.
3. Split `stale_nonce_responses_are_ignored` (its current assertion is the old
   semantics, replaced by the approved pivot — an authorized expectation change,
   see `_brief.md`):
   - `mixed_nonce_responses_are_accepted` — two `recover()` re-drives; a delayed
     response to the first nonce is accepted and counts toward the quorum.
   - `evicted_nonce_responses_are_stale` — after `MAX_RECOVERY_NONCES + 1`
     re-drives the first nonce is evicted; its response is
     `Diagnostic::StaleRecoveryResponse`. The constant name is used via whatever
     public observable boundary exists; if none, assert the behavior at 9
     re-drives without naming the constant in the test.
4. `recovery_view_fence_respected_across_nonces` — n0 answers T1 with view V; a
   view change fences V+1; n1 answers T2 with view V+1. Assert the completion
   selects the V+1 evidence (latest fenced view wins suffix selection) while the
   T1 response still counted toward the quorum.

## Constraints

- Tests in `tests/recovery.rs` only. No production edits. No test-only API.
- Through the public interface / harness only.
- The 3 staged `resolved unreproducible` tests stay byte-for-byte in expectations.
- Do not commit; `git add tests/recovery.rs` only.

## Report

Each test's Red output verbatim (compile errors from missing semantics count as Red
only if the failure is the intended missing behavior — prefer tests that compile and
fail on assertion), exact test count delta with names, `git status --porcelain`,
follow-ons found but not done.
