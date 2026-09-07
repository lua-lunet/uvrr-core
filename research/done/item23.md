# item23 — CI workflow: verification gate lanes + tagged release packaging

**Newly discovered work** (owner directive 2026-08-15, restated after steer): the
repository had no CI at all — no `.github/` directory — which is why the
Blacksmith migration wizard found no jobs to migrate. Model the workflow on the
org's system projects (`lua-lunet/lunet-locks` ci.yml, itself modeled on
`lua-lunet/lunet` build.yml): a setup-matrix lane split where regular builds run
the fast Linux lane and `v*` tags run the full matrix plus release packaging and
a publish-release job. Blacksmith runner labels are already in use org-wide
(`blacksmith-4vcpu-ubuntu-2404-arm`).

Note: the `.tmp/` slot item22 is taken by the parallel review thread; this item
uses the next free number.

Repo: `/Users/Shared/lua-lunet/vrr-core`. Facts: package `vrr-core` 0.1.0,
edition 2024, MSRV 1.85, lib target `vrr` producing rlib/cdylib/staticlib, no
`include/` directory yet (the C header ships later with the FFI work — package
only the built libraries).

## Scope

`.github/workflows/ci.yml` — new file, only file in this item:

- Triggers: pull_request to main; push to main and uVRR; tags `v*`.
- `permissions: contents: write` (publish-release creates GitHub releases).
- Concurrency cancel-in-progress per workflow+ref, as the siblings do.
- `setup-matrix` job: regular lane = linux-amd64 (`ubuntu-24.04`) + linux-arm64
  (`blacksmith-4vcpu-ubuntu-2404-arm`); tagged lane adds macOS (`macos-latest`).
  No Windows lane — no consumer today (YAGNI).
- `gate` job, `dtolnay/rust-toolchain@1.85` with rustfmt + clippy (the pin makes
  every lane the MSRV check), running the repo verification gate verbatim:
  `cargo fmt --check`; `cargo clippy --all-targets --all-features -- -D warnings`;
  `cargo test --all-features --all-targets`; the three hygiene greps (inline-test
  grep over `src`, forbidden-word grep over `src/` + `tests/`, ticket-identifier
  grep over tracked files). Defensive ripgrep install if absent.
- Tagged builds: `cargo build --release --all-features`, package
  `libvrr.{so,a}` (Linux) / `libvrr.{dylib,a}` (macOS) as
  `vrr-core-<target>.tar.gz`, upload-artifact.
- `publish-release` (tags only, needs gate): download artifacts, verify all
  three archives exist, create the GitHub release via
  `softprops/action-gh-release@v2` with generated notes — the sibling's shape.

## Constraints

- The workflow file is tracked, so it must itself pass the hygiene greps: no
  ticket-identifier-shaped strings, no forbidden words.
- One commit by the outer agent after local verification; push to uVRR so PR #9
  gains the checks. Message describes the change as delivered.

## Verification

Run the gate block locally exactly as the workflow runs it (already green at
0f5ddc6); parse-check the YAML by pushing and watching the workflow trigger.
