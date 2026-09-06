# uvrr-lean — Lean 4 proofs for uVRR cluster reconfiguration safety

Machine-checked (Lean 4.33.1, core only, no Mathlib) safety results for
reconfiguring a strongly consistent replicated log with **overlapping quorums**,
following David C. Turner's *Unbounded Pipelining in Dynamically Reconfigurable
Paxos Clusters* (UPaxos) and the blog posts that motivate uVRR. Scope is
**safety only**: no liveness, and we do not re-prove VRR itself. We prove that
reconfiguration with the UPaxos quorum overlaps cannot choose two values for one
slot, and we give a counterexample showing the cross-era overlap is necessary.

## The ladder

Each rung is one Lean module plus one *executable* markdown document under
`ladder/` (built with [showboat](https://github.com/simonw/showboat)); the
document shows the full source, rebuilds the module and prints the axiom
footprint of its theorems. `showboat verify ladder/NN-*.md` re-runs every
command and diffs the output.

| Rung | Module | Result | Paper ref |
|---|---|---|---|
| 1 | `UVRR/Structure.lean` | decidable frown checker `frownB` ⇔ `Frown`; 3-zone hot-swap satisfies P1 for all eras; wholesale-replacement negative control | §IV-A P1 |
| 2 | `UVRR/LexBallot.lean` | (era, round) ballots: well-founded strict total order; order respects eras | §IV-A |
| 3 | `UVRR/Synod.lean` | S1–S6 ⇒ Lemma 6, Lemma 7, **Theorem 8** (agreement), axiom-free | App. B |
| 4 | `UVRR/Eras.lean` | P1 + era rules ⇒ **Lemma 9**, **Theorem 10** (per-instance agreement across reconfiguration) | App. C |
| 5 | `UVRR/Counterexample.lean` | drop only QI_e ⌢ QII_{e+1}: every other invariant holds, two values chosen — **the cross-era frown is necessary** | (new) |
| 6 | `UVRR/CastingVote.lean` | casting vote: non-interference with the old-era pipeline, guard preservation, phase-I completion; agreement inherited from Theorem 10 unchanged | §V |
| 7 | `UVRR/Weights.lean` | blog's 7-row weighted schedule satisfies the frowns; +2 on one node breaks it (Lemma 3 bound tight) | App. A |

The only assumption about configurations anywhere is Turner's safety equation
**P1: QII_e ⌢ QI_e ⌢ QII_{e+1}** for every era e. The prepare-to-prepare
overlap QI_e ⌢ QI_{e+1} is not assumed (Turner's correction on the blog), and
the reverse cross overlap QI_{e+1} ⌢ QII_e is not assumed either.

## Verify everything

```sh
# toolchain (once): https://leanprover.github.io/lean4/doc/setup.html
export PATH=$HOME/.elan/bin:$PATH        # lean/lake 4.33.1 via elan
cd formal/uvrr-lean
lake build                               # ~10 s, all 7 modules
printf 'import UVRR\n#print axioms Paxos.theorem10\n#print axioms Counterexample.cross_frown_necessary\n' | lake env lean --stdin
# re-run every ladder document and diff outputs (needs: uv tool install showboat)
for f in ladder/0*.md; do showboat verify "$f" && echo "OK $f"; done
```

Expected axiom footprints: `Synod.theorem8` uses no axioms; everything else uses
only Lean's standard `propext`, `Quot.sound` and (via `omega`/`by_cases`)
`Classical.choice`. No `sorry`, no `native_decide`, no custom axioms.

## Honest scope

* **Not modelled:** multi-promises `promised≥i`, `i_max`, and the derivation of
  `proposed_i(b) ⇒ e(b) ≤ e(i)` (paper P2–P5). We take that consequence as the
  hypothesis `EraLe`.
* **Not proved:** liveness; existence of a casting vote; message delivery;
  VRR-specific recovery and log-prefix transfer.
* **Not general:** Rung 7 checks the concrete 4-server schedule. The paper's
  general Lemma 2 (arbitrary node sets, integer scale factors) was handed to
  Leanstral as a drafting experiment — see `ladder/08-leanstral-lemma2.md`.

## Correspondence to uVRR and to `formal/VrrCoreEras.tla`

| UPaxos (this proof) | VRR-2012 / uVRR |
|---|---|
| ballot b, era in high bits | view-number with era prefix (`[era |-> e, idx |-> v]` in `VrrCoreEras.tla`) |
| phase I, quorum from QI_{e(b)} | view change (DoViewChange/StartView), view quorum |
| phase II for instance i, quorum from QII_{e(i)} | Prepare/PrepareOK for op-number i, commit quorum |
| promised(a,b;b') carries last vote | DoViewChange carries the log |
| chosen_i(b) | commit of op-number i |

Observation (inference, not proved here): the TLA+ startup gate in
`VrrCoreEras.tla` requires *both* directions of cross-era view/commit
intersection. Under the UPaxos era rules (`e(b) ≤ e(i) ≤ e(b)+1`) the Lean
proof needs only `QI_e ⌢ QII_{e+1}`. If the uVRR design lets a new-era view
complete old-era slots (a new-era ballot proposing into an old-era instance),
that violates `EraLe` and the reverse overlap `QI_{e+1} ⌢ QII_e` would be
needed as well; if instead old-era slots must be finished with old-era views
(as in the paper), the extra gate is conservative. Deciding which is a design
choice for the Rust crate, and is the natural next rung.

## Layout

```
lakefile.toml lean-toolchain UVRR.lean   # lake project (lean_lib UVRR)
UVRR/*.lean                              # the seven rungs
ladder/0N-*.md                           # executable evidence, one per rung
```

## Write-up

`uVRR-reconfiguration-safety-lean.docx` — the paper (abstract, prior art, the ladder, uVRR correspondence, Leanstral outcomes, what is not proved, how to verify, next rungs).
