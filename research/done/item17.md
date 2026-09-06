# item17 — Multi-nonce recovery (production)

Read `.tmp/_brief.md` first — binding rules, git protocol, verification gate, and the
**multi-nonce recovery pivot** section (the approved design). This file states only
what is specific to this item.

Repo: `/Users/Shared/lua-lunet/vrr-core`
Depends on: item16 (its Red output is the specification this item turns Green).

## Gating precondition

Item16's `recovery_host_redrive_invalidates_inflight_response` must have failed on
unchanged code. If item16 recorded it `resolved unreproducible`, stop and report —
the stall claim is then unconfirmed and this item's scope shrinks to whatever the
remaining Red tests justify.

## Objective

Replace the single-nonce recovery attempt with a bounded nonce set so a host-driven
`Input::Recover` re-drive no longer invalidates in-flight responses. Turns item16's
Red Green.

## Scope

1. `src/replica/mod.rs` — `RecoveryVolatile`: `nonce: Tick` becomes
   `nonces: BTreeSet<Tick>`; add `const MAX_RECOVERY_NONCES: usize = 8` with a
   doc comment stating the bound and the eviction rule (oldest evicted on
   overflow). The set is volatile by design exactly like the attempt it lives in.
2. `src/replica/recovery.rs` — `plan_recover`: a re-drive inserts the fresh tick
   into the existing attempt's set (evicting the oldest at capacity) and preserves
   `responses`; a fresh start creates a singleton set. The solicitation broadcast
   is unchanged.
3. `src/replica/recovery.rs` — `plan_recovery_response`: the staleness check
   becomes set membership (`!attempt.nonces.contains(&nonce)`);
   `Diagnostic::StaleRecoveryResponse.attempt` reports the newest nonce in the
   set. No other ruling in the function changes: the `R_g` quorum counts distinct
   responders across all nonces in the set, and completion still selects the
   maximum reported view's primary suffix (§6.1, §8.3).
4. Update the module docs that describe single-nonce semantics
   (`src/replica/recovery.rs` header, the S4 mentions in `src/replica/mod.rs`)
   to state the set semantics. No other doc files.

## Constraints

- Sources: `src/replica/mod.rs`, `src/replica/recovery.rs` only. Anything else:
  stop and ask.
- The invariant gate and quorum gate stay closed — this change must not touch
  `src/invariant.rs` or `src/quorum.rs`.
- No wire-format change; no new message; no per-attempt quorum isolation.
- `git add` touched files; never commit (outer agent commits on Green).

## Report

Red→Green output for each item16 test, exact test count delta, full verification
gate output verbatim, `git status --porcelain`, follow-ons found but not done.
