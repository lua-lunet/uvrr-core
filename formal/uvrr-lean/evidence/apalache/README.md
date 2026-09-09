# Apalache evidence — era model (VrrCoreEras)

Dated 2026-09-09. Symbolic-checker evidence for the era design model,
produced with `ghcr.io/apalache-mc/apalache:main`, digest
`sha256:0c3314a6df2200d5b1ca67086385ce9c2dec7b7ac8ad9d62c97ffafaab77cd52`
(build `0ed8817`), `linux/amd64` under emulation, docker-create/cp
harness. Full method, command list, and the input-language walls are
recorded in `research/apalache-smoke.md` (era-model section).

## What is banked

Three completed bounded checks of `CommittedLogsAgree` on the NORMATIVE
3-node configuration (Scenario inc3, MaxViewIndex 0, MaxLogLength 3,
MaxEpoch 0, Defect none), each green:

| file | bound | exit | wall |
|---|---|---|---|
| `VrrCoreErasApalache-lens0-init.log` | length 0 | 0 | 36s |
| `VrrCoreErasApalache-lens1-init.log` | length 1 | 0 | 47s |
| `VrrCoreErasApalache-lens2-init.log` | length 2 | 0 | 72s |

Provenance caveat (recorded honestly): these runs were launched with the
intention of an inductive-invariant check (`--init=CommittedLogsAgree`),
but the TLC config file's `SPECIFICATION Spec` clause silently overrides
CLI `--init` in this Apalache build, so the effective initial predicate
was `Init`. They are therefore banked as ordinary bounded checks from
`Init` at lengths 0-2 — NOT as an induction result. The actual induction
attempt produced a counterexample-to-induction (machine-confirmed, exit
12) and is recorded in `research/apalache-era/` only.

Deeper bounds are unreachable in this environment: `--length=32` was
OOM-killed at 438s (symbolic state 4, all completed checks green); the
8 GiB VM is the cap. The TLC baseline (banked under `evidence/tlc/`)
exhausts the same graph at depth 32 in 137.67s.

## Model provenance

The checks run against `VrrCoreErasApalache.tla` — a separate annotated
copy of the normative model `formal/VrrCoreEras.tla` (never modified).
The copy differs from the normative model only in:

- the module name,
- Snowcat `@type` annotations (constants, variables, 37 operators),
- one mechanical restructuring of `SendPlannedViewChange` (`LET target`
  replaced by the top-level operator `PlannedTarget`; deterministic
  CHOOSE, semantics identical),
- ten set-comprehension ranges rewritten from symbolic to constant
  bounds with equivalent guards (Apalache known-issue workaround), e.g.
  `PrefixEqual`, `CountReconfigBefore`, `ReconfigPositions`,
  `HasCommittedReconfig`, `ProposeCommand`, `EntryEraWindow`,
  `CommittedHistoryPresent`, the `Next` ghost-record update, `SeqToSet`,
  and the `historicalCommitted` initializer.

Digests (sha256):

    VrrCoreEras.tla                  2d51449d707f104a506971a8e0a915b69255e1f99b6bbc5a6594384d49311ef2
    VrrCoreErasApalache.tla          23eed3fdcbaf6358186707de73e6f7e9270f826314e9ecc558265505ecb0e526
    VrrCoreErasApalache3.cfg         7dd53e71683d4a39ba6b7b1f39d7d9d5a3bee0940d587d0ae22aa7a46da5b63c
    VrrCoreErasApalacheCinit.tla     923843669add59b01fe6d2c51176326e6835ba7e11515178fa4c78e25c5fdd5f
    VrrCoreErasApalacheM1.cfg        5e0f340aa9a142b97ce3dc0d9b34c7032c204bb8a5d478719e5e65885417c513
    VrrCoreErasApalacheM2.cfg        ce9cb4fa61729988b925b9f7eba4c3e106c742be5cc0dfd842457d5a1fb47507
    VrrCoreErasApalacheGate.cfg      147fcb03c6a1687cab64008e5e72230100b1a420b8bf856f0d58b720fa283686
    VrrCoreErasApalache3Ind.cfg      2e2280a1d994daf6ed070616f94bea32dcfb37da4e2896a602604d773a7c7078

The TLC baseline json in `evidence/tlc/` records `VrrCoreEras.tla` at
sha256 `3eebcfa9…`; the normative model has since been revised (current
digest above). The digests here are the ones actually checked against.

M1/M2/Gate/3Ind cfgs and the Cinit wrapper are reproduction artifacts for
the runs and walls documented in `research/apalache-smoke.md`; the runs
they served were OOM-killed or rejected, and their logs live in
`research/apalache-era/` and `research/apalache-smoke.md`.
