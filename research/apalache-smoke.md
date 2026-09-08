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

---

# Apalache on the era model (VrrCoreEras)

Dated 2026-09-09. Same image and digest as above
(`ghcr.io/apalache-mc/apalache:main`, `sha256:0c3314a6…`, build `0ed8817`),
same docker-create/cp harness. Yardstick: the banked TLC evidence
`formal/uvrr-lean/evidence/tlc/VrrCoreEras.cfg.log` — 2,743,933 states
generated, 837,204 distinct, depth 32, 137.67s (the task brief cited a
3m24s figure; the banked json records 137.67s). Note: the banked TLC json
records `VrrCoreEras.tla` at sha256 `3eebcfa9…`, while the file at this
writing hashes `2d51449d…` — the TLC baseline predates a later revision of
the normative model.

**Verdict: conditionally compatible, but computationally capped.** The era
model enters Apalache's input language only after mechanical
semantics-preserving adaptations (a separate annotated copy,
`formal/VrrCoreErasApalache.tla`; the normative model is untouched). With
those adaptations the checker runs, finds a genuine counterexample on a
normative-derived probe, and completes bounded checks at small lengths;
longer bounds die of memory inside the 8 GiB colima VM (amd64 emulation)
before reaching TLC's full diameter. No invariant check of meaningful
depth completed.

## Input-language walls found (era model)

1. All 11 VARIABLES unannotated — Snowcat requires `@type` per variable
   (`Expected a type annotation for VARIABLE …`, same wall as the
   reincarnation model).
2. This build also rejects unannotated CONSTANTS (`Expected a type
   annotation for CONSTANT Defect`), including constants assigned in the
   TLC config. All 14 constants annotated (`Str`, `Set(Str)`, `Int`,
   `Bool`); node constants model values became strings in the Apalache
   configs.
3. Type System 1.2 (default) rejects the message-record set annotation:
   `Found imprecise record types … use --features=no-rows`. All runs use
   `--features=no-rows`; the TS-1.2 record complaint persisted after full
   annotation.
4. Operators that access record fields or apply function-valued parameters
   need explicit `@type` operator annotations; function-typed parameters
   take `->` (values), operators `=>` (`Intersects` rejected with `=>`).
   37 operators annotated.
5. `LET target == PlannedView(leader)` in `SendPlannedViewChange`: after
   optimization, `undeclared operator target$2` [FlatLanguagePred].
   Mechanically restructured to a top-level `PlannedTarget(leader)`
   operator (deterministic CHOOSE, semantics identical).
6. Set-comprehension ranges that feed sequence/tuple indexing reject
   symbolic bounds: `Expected a constant integer range in [ .. ], found
   1..Len(…)` (known issue). Ten sites rewritten to constant ranges with
   equivalent guards (`{j \in 1..MaxLogLength : j <= …}`), including the
   `historicalCommitted` ghost-record update in `Next` and `PrefixEqual`.
7. Invariants containing `Seq(Entries)` memberships (`TypeOK`,
   `messages \subseteq [… history : Seq(Entries) …]`) are unsupported:
   `Seq(_) produces an infinite set of unbounded sequences`. Only
   `CommittedLogsAgree`-family invariants are checkable.
8. Tool trap: a TLC config carrying `SPECIFICATION Spec` silently
   overrides CLI `--init`. Two runs labelled inductive (`check
   --length=1 --init=CommittedLogsAgree --inv=CommittedLogsAgree`) were
   in fact ordinary bounded checks from `Init` and returned green; the
   green was not an induction result. Detected by a hand-built
   counterexample probe that stayed green until the config clause was
   removed.
9. Inductive-invariant checking per the documented recipe
   (`--init=<Inv> --inv=<Inv> --length=1`) fails structurally on this
   model: Apalache requires the init predicate to ASSIGN the state
   variables; a pure constraint gives `committed' is used before it is
   assigned` (both via cfg `INIT` clause and via CLI with `--cinit`).
10. Resource wall: bounds beyond length ~3-4 are unreachable in the 8 GiB
    colima VM (amd64 emulation, 4 CPUs): `check --length=32` on the
    normative 3-node config was OOM-killed (exit 137, OOMKilled=true) at
    438s having reached symbolic state 4 with every completed invariant
    check green.

## Phase 1 — bounded check, normative 3-node config

All runs `--features=no-rows --inv=CommittedLogsAgree
--config=VrrCoreErasApalache3.cfg VrrCoreErasApalache.tla` (docker cp
harness, as above).

| run | exit | wall | outcome |
|---|---|---|---|
| `--length=0` | 0 | 36s | green: "Checker reports no error up to computation length 0" (Init ⊨ CommittedLogsAgree) |
| `--length=1` | 0 | 47s | green (bounded from Init; see wall 8 — the induction interpretation was void) |
| `--length=2` | 0 | 72s | green (same caveat) |
| `--length=32` | 137 | 438s | OOM-killed (OOMKilled=true) at symbolic state 4; every completed check green |

Runtime comparison is honest but weak: TLC exhausts the full graph (depth
32) in 137.67s; Apalache OOMs at symbolic depth 4 after 438s under
emulation. The only completed bounded checks are at lengths 0-2; the
length-1/2 greens are banked with their provenance caveat
(`formal/uvrr-lean/evidence/apalache/`).

## Phase 2 — mutation red-with-witness

Both attempts OOM-killed before reaching the TLC-known violation depths;
no mutation witness produced by the symbolic checker at scale.

| run | exit | wall | outcome |
|---|---|---|---|
| `check --length=8 --inv=FrontiersOrdered --config=VrrCoreErasApalacheM1.cfg …` (truncate-on-transfer, `Commands={a}`) | 137 | 1245s | OOM at symbolic state 5; TLC finds this red at depth 7 in <1s (579 distinct states, verified locally with the pinned jar) |
| `check --length=2 --inv=CommittedLogsAgree --config=VrrCoreErasApalacheGate.cfg …` (gate-demo) | 137 | 99s | OOM during BoundedChecker setup; the 6-node `SUBSET` powersets in the Init gate Asserts explode before any check. TLC rejects at the Init Assert (`The first argument of Assert evaluated to FALSE`). Side finding: `--inv=TypeOK` is additionally blocked by wall 7 (`Seq(_)`, exit 75 in 5.6s) |

The checker's red machinery is demonstrably sound on this model — the
phase-3 probe below returns red-with-witness in 8s — so the phase-2
failure is resource, not logic.

## Phase 3 — inductive attempt (red, stopped)

The one-step induction check could not be expressed directly (wall 9). It
was settled instead by a hand-built counterexample-to-induction run
through the checker: the scratch copy gained

    ProbeCex ==
        /\ status = [r \in Nodes |-> Normal] /\ (views, epochs, flags as Init)
        /\ logs = [r |-> IF r = "n0" THEN <<g, e*>> ELSE <<g>>]
        /\ committed = [r |-> IF r = "n0" THEN 2 ELSE 1]
        /\ messages = {Prepare("n0","n1",View(0,0),2,
                               CommandEntry("increment-n0",0), 2, 0)}

with e* = [command, "cmd", 0]. This state satisfies `CommittedLogsAgree`
(agreement holds only up to the minimum frontier, 1), and `n0` carries an
unconstrained divergent tail at slot 2. `ReceivePrepare(n1, m)` then
appends m.entry at slot 2 and raises n1's frontier to `Min(m.committed,
2) = 2`, so two frontiers sit at 2 over divergent entries.

    check --features=no-rows --length=1 --cinit=ConstInit
          --init=ProbeCex --next=Next --inv=CommittedLogsAgree
          VrrCoreErasApalacheCinit.tla
    → exit 12, 8s: "State 1: state invariant 2 violated", witness emitted.

Witness: `research/apalache-era/induction-counterexample-violation1.tla`
(State1: `committed = [n0↦2, n1↦2, n2↦1]`, n1 slot 2 = "increment-n0" vs
n0 slot 2 = "cmd", transition `_transition(2)` = ReceivePrepare); log
`research/apalache-era/induction-counterexample.log`. Per the item rule
the induction attempt stops here; nothing was weakened.

Conclusion: `CommittedLogsAgree` is NOT 1-step inductive on the normative
era model — as expected for a real protocol, the invariant needs the
reachability structure (e.g., a historicalCommitted/frontier-strength
conjugate) that Apalache's single-predicate induction cannot supply
without weakening.

## Where evidence lives

- Banked (normative greens, with provenance caveat):
  `formal/uvrr-lean/evidence/apalache/` (README with digests).
- Research only (failures, OOMs, counterexample):
  `research/apalache-era/`.
- Scratch harness and probe modules: `.tmp/apalache-era/`,
  `.tmpx/` (not committed).
