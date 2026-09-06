# item11 — Maelstrom adapter, binlog, binlog2json

Read `.tmp/_brief.md` first — binding rules, git protocol, verification gate,
test-freeze nuance, milestone-commit rule. This file states only what is specific to
this item.

Repo: `/Users/Shared/lua-lunet/vrr-core`
Depends on: item10 (C ABI). Lands in `src/bin/` (new adapter binary +
`binlog2json`), `Makefile`, `Dockerfile.maelstrom`, `docker-entrypoint.sh`.

## Objective

The integration proof: a lin-kv workload under Maelstrom with nemesis faults, driven
through the C ABI, plus the binary→JSON debug bridge. Also repairs the build plumbing:
`Makefile`, `Dockerfile.maelstrom`, and `docker-entrypoint.sh` all still invoke
`target/release/maelstrom-lin-kv`, which no longer exists.

## Scope

1. **Adapter** (ordinary host, owns its KV — no core coupling): samples its own `u64`
   tick and passes it on every input; selects `Stability::Volatile` (fully diskless,
   first-class); routes each `Send` by the effect's route era; drains `APPLY` and feeds
   `Applied` back; issues `Checkpointed` on its own policy.
2. **Restart discipline**: `provision` vs `reopen` across process restart — never an
   amnesiac Normal voter (§14.2). Documents the supervisor's ≥ 1 ms restart rule (§6.1).
3. **Binlog**: append every inbound and outbound datagram as raw binpack to a per-node
   tmpfile; `binlog2json` replays all node logs to JSON post-mortem, byte-identical
   round-trip.
4. **Plumbing repair**: `[[bin]]` targets in `Cargo.toml` with
   `required-features = ["maelstrom"]`; Makefile `build`/`test-*`/`e2e` targets work
   again against the new binary.
5. `maelstrom/` is a third-party submodule — read-only.

## Red

`tests/maelstrom_adapter.rs` and `tests/binlog_roundtrip.rs` (authorized): adapter
behaviors above through the harness with a scripted transport; binlog round-trip
byte-identical.

## Verification

The `_brief.md` gate plus `cargo test --features maelstrom` and `make build &&
make test-clean` if the local JDK/Leiningen permits; if not, state precisely what was
and was not runnable locally.

## Constraints

- Serde/serde_json stay optional behind the `maelstrom` feature; the library's
  non-optional dependency set stays empty (manifest gate must keep passing).
- Baseline 199 committed (+3 staged, plus items 01–10 deltas). State your delta.

## Report

Red→Green output, plumbing diff summary, what ran and what could not run locally and
why, exact delta with names, gate output verbatim, `git status --porcelain`,
follow-ons not done.
