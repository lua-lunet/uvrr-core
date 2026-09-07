# item19 — Recovery arc: full gate and milestone commit

Read `.tmp/_brief.md` first — binding rules, git protocol, verification gate,
milestone-commit rule. This file states only what is specific to this item.

Repo: `/Users/Shared/lua-lunet/vrr-core`
Depends on: item16, item17, item18 (all Green).

## Objective

Close the multi-nonce recovery arc: run the full verification gate, confirm the
test-count delta against the 202 baseline (expected ≈ 209: +8 new, −1 split), and
land the milestone commit.

## Scope

1. Full gate, verbatim output:
   - `cargo test --all-features --all-targets`
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `rg -n '#\[cfg\(test\)\]|mod tests' src` — empty
   - `rg -in 'paxos' src/ tests/` — empty
   - `git ls-files | rg -v '^maelstrom' | xargs rg -n 'item[0-9]'` — empty
2. Confirm the 3 staged `resolved unreproducible` recovery tests are still green
   and were committed as part of the arc (they are staged but uncommitted at arc
   start).
3. Milestone commit by the outer agent: message per the milestone-commit rule —
   describes the change as delivered, no ticket identifiers, no pending-chore
   enumeration.

## Constraints

- No production or test edits in this item except fallout fixes that the gate
  itself demands; any such fix is reproduction-first per `AGENTS.md`.
- Nothing under `.tmp/` is ever staged.

## Report

Gate output verbatim, final test count, commit hash, `git status --porcelain`
after the commit.
