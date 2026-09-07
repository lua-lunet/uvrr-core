# item17b — `src/observe.rs` doc: `StaleRecoveryResponse.attempt` semantics

**Follow-on work to item17** (newly discovered by the item17 agent, not done by it
because its scope was limited to `src/replica/mod.rs` and `src/replica/recovery.rs`).

Read `.tmp/_brief.md` first. Repo: `/Users/Shared/lua-lunet/vrr-core`.

## Objective

`src/observe.rs:224` documents `Diagnostic::StaleRecoveryResponse.attempt` as
"The open attempt's nonce". Under the multi-nonce pivot the field reports the
NEWEST nonce in the attempt's set. Correct the doc comment to state that. One
comment; no behavior change; no test change (the diagnostic's shape is unchanged).

## Constraints

- `src/observe.rs` only. Doc-comment register: terse, spec-cited (§6.1, S4).
- `git add src/observe.rs`; never commit; never stage `.tmp/`.
- Run `cargo fmt --check` and
  `cargo clippy --all-targets --all-features -- -D warnings` after the edit.

## Report

The before/after comment text, gate output verbatim, `git status --porcelain`.
