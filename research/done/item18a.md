# item18a — Fast-forward follow-ons: `accepted` floor ruling and replay re-emission note

**Follow-on work to item18** (found by the item18 agent, not done by it). Two
observations from the committed fast-forward work (landed in `93a425f`). Both are
**suspected** behaviors: reproduction-first per `AGENTS.md`. This brief describes
observed paths and names reproductions; it does not name causes or fixes.

Read `.tmp/_brief.md` first. Repo: `/Users/Shared/lua-lunet/vrr-core`.

## Observation 1 — completion evidence `accepted` below a fast-forwarded frontier

A completion whose evidence carries `accepted` below the local (fast-forwarded)
`committed` would make `recovery_candidate` fail the frontier chain — a
`PlanRejection::Progress` refusal, not a fault. Reported as unreachable in today's
tests.

Reproduction to attempt (`tests/recovery.rs`, harness): fast-forward a recovering
node's `committed` past some slot S, then drive a completion whose evidence
`accepted < committed`. If the prerequisite cannot occur through the public
protocol (an honest quorum's completing evidence always carries `accepted >=`
any committed it could have taught), record `resolved unreproducible` with the
reasoning and add nothing. If it reproduces, the ruling question — an `accepted`
floor in `recovery_candidate` vs a named diagnostic — is the owner's design
decision; STOP and surface it with the Red output.

## Observation 2 — completion before host acknowledgement re-emits upcalls

A completion arriving before the host acknowledges fast-forwarded upcalls re-emits
those upcalls from the replay walk (the slot is not yet `applied`). The item18
agent assessed this as §11's standing at-least-once boundary semantics, not a new
hole; the fast-forward tests pin exactly-once only across acknowledged slots.

Reproduction to attempt: fast-forward, do NOT acknowledge, then complete; assert
through the public interface whether the host sees a second `Apply` for the same
slot. If it reproduces, the question is whether §11.1's boundary semantics should
be documented where a host implementor reads them (candidate for the observation
API/documentation items) or whether the core must suppress the re-emission —
owner's call; STOP and surface with the Red output. If it cannot reproduce, record
`resolved unreproducible`.

## Constraints

- Reproduction-first; a passing reproduction means zero production change.
- The invariant and quorum gates stay closed.
- Tests in `tests/recovery.rs`. Source edits only if a reproduction fails AND the
  owner has ruled on the design question. Anything else: stop and ask.
- Baseline 211 committed tests. State your delta.
- `git add` touched files; never commit; never stage `.tmp/`; no stash/reset.

## Report

Per-observation verdict with unchanged-code output quoted, any Red→Green output,
exact count delta with names, gate output verbatim, `git status --porcelain`,
follow-ons not done.
