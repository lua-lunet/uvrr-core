# From Hello-World to Reconfiguration Safety: Kick-the-Tires Outcomes for a Replication Protocol's Proof Tooling

Field report — smoke-test outcomes mapped to the verification literature
ITEM-13 / September 2026
Subject: proving VRR cluster-reconfiguration safety in vrr-core / Mistral leanstral · TLA+/TLC · Lean 4 + Veil/LeanLTL/lean-auto

---

## Verdict

**What held.** Every tool we could actually run passed its hello-world. TLC model-checked a one-bit-register spec with no errors (3 states, 2 distinct, invariant holds, under 1 s). A Lean 4.33.1 toolchain installed via elan in one ~2-minute attempt; `1 + 1 = 2` proved by `rfl`, clean. Mistral's single-shot Lean file — previously unverified — now compiles clean, exit 0, same toolchain.

**What broke or bounded.** Nothing failed outright; the edges showed. The prescribed TLA+ jar URL is dead (repo folded into the `tlaplus/tlaplus` monorepo) and needed substitution. Mistral chose `native_decide` over a kernel-checked tactic: compiles, but discharged by compiled evaluation, not the Lean kernel — working, weaker. Veil was not tire-kicked: it pins `v4.32.0` (a second toolchain fetch) and pulls Mathlib, a multi-GB resolve outside the timebox.

**Final verdict: all smoke tests green, yet the distance from "runs hello-world" to "proves VRR reconfiguration safety" is undiminished — exactly the position paper's Gap 1.**

*Attributed to the synthesis pass. Working documents linked in the footer.*

## The method

Survey → Submodule → Hello-world → Compile-check → Map to literature.

Eleven maintained candidates ranked by "submodule and run it today" (solver-survey); three tool families smoke-tested under a timebox (tool-kick-tires); the item10 LLM candidate re-checked against the now-installed toolchain; outcomes mapped against the position paper and glossary. All runs local; submodules staged, not committed.

## Three tool outcomes, applied

### Mistral labs-leanstral-1-5-1 (item10 + this item's compile check)

Is a one-shot LLM usable as a Lean proof generator? Single shot, temperature 0 forced with `top_p: 1` (the endpoint rejects temperature 0 alone — HTTP 400, code 3054). Exactly one code block, no prose: `import Init`, `theorem two_plus_two : (2 : Nat) + 2 = 4`, `:= by native_decide`. **Verified — compiles clean, exit 0, Lean 4.33.1 via `lake env lean` from `.tmp/lean-hello/`; VERDICT.md's "tbd" is resolved.** **Caveat — `native_decide` semantics: accepted by the elaborator, but discharged by compiled evaluation of a `Decidable` instance, not the kernel's trusted checker.** Weaker than `decide` or `rfl`; a VRR safety proof must stay kernel-checked.

### TLA+ / TLC (item11)

Does the reference ecosystem run today? Java already present; jar URL needed the monorepo substitution (TLC2 2.19, Aug 2024). `Hello.tla` (one-bit register, `EXTENDS Naturals` — `EXTENDS TLC` alone does not define `-`) ran via `java -cp tla2tools.jar tlc2.TLC`. **Verified — model checking completed, no errors, invariant holds, under 1 s.** The repo's own `formal/VrrCore.tla` and `VrrCoreEras.tla` exist but were not run, per spec. TLC's concrete-state checking is the corpus's default-path strength — the one model checker we ran.

### Lean 4 tools: Veil / LeanLTL / lean-auto (item12 + compile check)

Does the Lean side stand up? Toolchain installed (4.33.1, Lake 5.0.0, arm64); `tools/veil`, `tools/LeanLTL`, `tools/lean-auto` submoduled shallow, staged only. `Hello.lean` proved `1 + 1 = 2` by `rfl` — **verified, kernel-checked, clean.** `lake build` alone warns "no targets" (bare toml lakefile); `lake env lean` is the working path. **Skipped — building a Veil example: toolchain pin mismatch plus the Mathlib resolve, out of timebox.** LeanLTL and lean-auto were submoduled but not separately smoke-tested; only core Lean is verified *[inference: nothing above tests the libraries themselves]*.

## Evidence table — tasks run, and what the artifacts said

| # | Task / claim | Source | Verdict |
|---|---|---|---|
| 1 | TLA+ jar at prescribed URL | tool-kick-tires.md | Dead — repo folded into monorepo; substituted |
| 2 | TLC hello-world model check | tool-kick-tires.md | Verified — no errors, 3 states, <1 s |
| 3 | Lean toolchain install | tool-kick-tires.md | Verified — 4.33.1, one attempt, ~2 min |
| 4 | Lean hello-world (`rfl`) | tool-kick-tires.md | Verified — kernel-checked, clean |
| 5 | Veil example build | tool-kick-tires.md | Skipped — v4.32.0 pin + Mathlib, out of timebox |
| 6 | Mistral one-shot output shape | VERDICT.md | Verified — one block, no prose; temp-0 quirk |
| 7 | Mistral candidate compiles | this item, `lake env lean` | Verified — exit 0 |
| 8 | Mistral proof strength | VERDICT.md | Weak — `native_decide` trusts compiled evaluation |
| 9 | Survey's "runnable today" ranking | solver-survey.md | Survey-based only — Veil/Quint/Apalache not run *[inference]* |
| 10 | Published VRR reconfiguration proof | position-paper.md Gap 1, audit (d) | Not found — corpus §8 is informal prose |

## Practicality ranking, skipped tools, and the mapping

**Survey ranking (item08.5, "submodule and kick the tires today"):** 1. **Veil** — the position paper's own recommendation, at its most runnable point (template repo, Veil 2.0 preview, active liveness work); a VRR normal-case/view-change model is a realistic first timebox. 2. **Quint + Apalache** — npm + docker, minutes to hello-world, the natural place to prototype VRR reconfiguration before committing to Lean. 3. **TLA+/TLAPS with the logless-reconfig artifact** — the only published, mechanically checked *reconfiguration* safety proof in the Raft family (MongoRaftReconfig, OPODIS 2021), cheap to re-check, with Ongaro's `raft.tla` and the etcd-aligned spec as baselines. *The ranking is reading-based; our smoke tests covered only TLC and core Lean — the ranking's own claim, not a tested fact.*

**Skipped or worked around, with reasons:** Veil build (timebox); Quint, Apalache, Ivy, mypyvy, Verus, Bythos, and the invariant-inference tools (survey entries, never scheduled); repo `formal/` specs (out of scope); watch-list items — Antithesis, CCF, the federated Raft campaign — claims, not submoduleable tools.

**Mapping to the literature for vrr-core.** The position paper recommends modeling VRR in Veil's DSL, reconfiguration as a first-class action family, safety-first: BMC for non-vacuousness; the §8.1–8.3 conditions as invariants (committed operations survive view changes and epoch transitions in order; one primary per view/epoch; recovering replicas rejoin no staler than they failed; the new epoch starts in view 0; f'+1 EPOCHSTARTED before shutdown); automation where decidable, interactive fallback elsewhere. Our outcomes neither confirm nor refute that route — Veil ran nothing here. What they do support: the TLA+ default path works locally, and the closest published analogue of the actual goal — a reconfiguration safety proof — lives on that stack, not in Lean (Gap 1; "Viewstamped" appears nowhere in Veil's case studies). LeanLTL's past-time operators remain future work (audit claim (b), PASS), so historically-phrased reconfiguration properties need explicit history variables — a manual, correctness-critical encoding the corpus does not evaluate. Effort references: Multi-Paxos in TLAPS (750 obligations, ~3 minutes, one undocumented-bug history) and IronFleet (>30,000 proof LOC). The `native_decide` result sharpens the TCB argument: Veil's wager reduces trust to the Lean kernel; an LLM reaching for `native_decide` quietly enlarges it. Liveness is out of scope everywhere in the corpus — acceptable here, since vrr-core's interest is safety.

## What this report does not do

Does not claim any tool verified anything about VRR itself. Does not present survey plausibility as tested fact. Does not run the repo's own TLA+ specs. Does not present opinions as facts — evaluative language is attributed to the synthesis pass; inferences are flagged.

---

Method (format): a1-poster-pipeline — gist.github.com/simbo1905/6bdbf862052f6548afbac89fbb857264
Documents: tool-kick-tires.md · VERDICT.md + candidate.lean · solver-survey.md · position-paper.md + audit · glossary.md · papers/ocr/ (7 files)
ITEM-13 / September 2026 — a summary, not a substitute for the working documents.
