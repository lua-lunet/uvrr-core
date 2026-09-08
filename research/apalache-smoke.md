# Apalache smoke on the reincarnation model

Dated evidence artifact. Checks the reduced reincarnation design model
(`formal/VrrCoreReincarnation.tla`, sha256 `54f919e6…`) under the symbolic
checker Apalache, timeboxed and honestly reported. Companion TLC evidence:
`formal/uvrr-lean/evidence/tlc/`.

**Verdict: incompatible — stopped.** Apalache rejects the model on two
independent grounds; the model was not rewritten to please the tool, so no
bounded check of the model's invariants was performed.

## Toolchain

- Image `ghcr.io/apalache-mc/apalache:main`,
  digest `sha256:0c3314a6df2200d5b1ca67086385ce9c2dec7b7ac8ad9d62c97ffafaab77cd52`
  (id matches; created 2026-09-05T08:15:33Z). Apalache build `0ed8817`.
- Platform `linux/amd64` only (no arm64 variant); run under emulation on an
  arm64 host. JVM inside the image: OpenJDK 25.0.4 LTS (not 17/21 as
  commonly documented).
- The image bundles no example `.tla` specs (only standard modules; a
  filesystem search and a listing of the 36 `.tla` entries in
  `/opt/apalache/lib/apalache.jar` found none). Phase 0 therefore used a
  minimal annotated counter spec written for the smoke.
- Harness note: bind-mounts of host paths are unusable on this host (colima
  virtiofs view is bidirectionally inconsistent between host and VM). Every
  phase ran as `docker create` + `docker cp` (spec in) + `docker start -a`
  (capture log and exit) + `docker cp` (artifacts out). Commands below are
  the apalache-mc argument lists; each ran with
  `--entrypoint /opt/apalache/bin/apalache-mc -w /var/apalache`.

## Phase 0 — smoke (tool functional check)

Spec: `SmokeCounter.tla`, variable `x : Int`, `Init == x = 0`,
`Next == x' = x + 1`, `Inv == x <= 5`.

| command | exit | wall | outcome |
|---|---|---|---|
| `check --length=5 --inv=Inv SmokeCounter.tla` | 0 | 3.3s (1.75s internal) | green: "Checker reports no error up to computation length 5" |
| `check --length=10 --inv=Inv SmokeCounter.tla` | 12 | 2.0s (1.80s internal) | red-with-witness: `x = 6` at state 6 violates `Inv`; trace emitted (`violation1.tla`, `violation1.itf.json`) |

Both verdicts are correct for the lengths requested; the checker and its
counterexample machinery work. An earlier run without `@type` annotations
was rejected at parse (`Expected a type annotation for VARIABLE x`),
establishing that Snowcat typing is mandatory.

## Phase 1 — bounded check, 3-node config

Command: `check --length=20 --config=VrrCoreReincarnation3.cfg
VrrCoreReincarnation.tla` (all seven invariants picked up from the TLC
config: TypeOK, FrownChain, EvictedNeverVoter, MassRule, NoStandbyVote,
RebornAfterFence, SequenceCompletes).

- Exit 255, 3.0s wall (1.47s internal). Rejected before any state
  exploration, at the type-checker pass:

  ```
  VrrCoreReincarnation.tla:55:11-55:12: type input error:
  Expected a type annotation for VARIABLE wt
  ```

- The model is unannotated (it is a TLC model); Apalache requires an
  `@type` annotation comment for every variable and constant, inline in the
  spec — its HOWTO documents no sidecar/external annotation file. Adding
  them would be editing the model. Before stopping, one probe checked
  whether annotation is even sufficient (Phase 1a).

### Phase 1a — annotated recursion probe

`RecursiveProbe.tla`: fully annotated (`S : Set(Int)`, `x : Int`) with
`EXTENDS Integers` and one recursive operator `Sum` mirroring the model's
`SumWt` shape. Command: `check --length=5 RecursiveProbe.tla`.

- Exit 255, 0.9s wall (0.89s internal). Type checking passed ("Your types
  are purrfect!"), then rejected in the next pass with the categorical
  error:

  ```
  RecursiveProbe.tla:13:1-17:28: unexpected expression: RECURSIVE Sum(_).
  Apalache does not support recursive operators.
  ```

So the model is doubly incompatible: its `RECURSIVE SumWt(_)`
(VrrCoreReincarnation.tla:59) is categorically unsupported, and its
variables are unannotated. Annotating the model would not unblock it.

## Phase 2 — five-node config

Not run. Stopped per the phase-1 incompatibility rule.

## Context

TLC (pinned 2.19 jar) exhausts the full 3-node state graph (1,264 distinct
states) in ~1s; Apalache never reached a state, so no runtime comparison
exists — the incompatibility, not performance, is the recorded outcome.
