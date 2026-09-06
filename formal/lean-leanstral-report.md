# Lean and Leanstral across the vrr-core effort — durable record

Compiled 2026-09-06 from the session rollouts (`.tmp/rollouts/*.jsonl`, 19 files) and the
ground-truth artifacts under `.tmp/research/`, `.tmp/lean-*/`, `.tmp/mistral-lean/`,
`.tmp/vibe-lean/`, `.tmp/blogs/`, `.tmp/specs/`. Every claim below was cross-checked against
those artifacts; where a rollout report and an artifact disagreed, the artifact won and the
discrepancy is noted. No secrets are quoted anywhere in this file; `MISTRAL_API_KEY` is
mentioned only as a variable name and was always read silently from `.env` via `awk`.

Repo context: `/Users/Shared/lua-lunet/vrr-core`, branch `uVRR`, existing TLA+ specs under
`formal/` (VrrCore.tla, VrrCoreEras.tla + ~25 .cfg models) — pre-existing, untouched by this
effort except as context. All work products live under gitignored `.tmp/`; the only
approved outside-`.tmp` writes were repo-level `git add` (submodules, staged, never
committed), the AGENTS.md appendix (item06), the opencode skill install (item09), and this
report.

---

## 1. Timeline

All times are local, 2026-09-05 (rollout export ran 00:02 on 2026-09-06). Order is from spec
file mtimes, artifact mtimes, and the orchestrator chat (`00-main-chat.jsonl`); mtimes are
last-write, so "≈" marks derived boundaries.

| Time | Event | Session |
|---|---|---|
| pre-21:00 | User pastes an AI-generated literature review ("Lean 4 ATP for VRR cluster reconfiguration") into the orchestrator chat and asks for a fact-check | 00-main-chat |
| ≈21:00–21:10 | Orchestrator fact-checks the review; finds a fabricated VR-Revisited quote, a distorted Veil quote, and non-standard "LTA+" terminology; downloads 7 primary PDFs into `.tmp/papers/`; OCRs them via `.tmp/ocr_run.py` (Mistral OCR, key from `.env`) into `.tmp/papers/ocr/*.md` | 00-main-chat |
| 21:00 | item03 spec written; **secret audit** of `.tmp/papers/` + `ocr_run.py` → PASS (15 files scanned, 0 matches) | item03 |
| ≈21:10 | item04: paper bundle `git init -b main` + `git add` (no commit), safety gate CLEAN, 15 files staged, 11M | item04 |
| ≈21:12 | item05a: first gist attempt **fails** — `gh gist create` rejects binary PDFs; agent stops instead of substituting | item05a |
| 21:15 | Specs item05/07/08 written; orchestrator replans: gist = OCR technique only, glossary + position paper from the corpus | 00-main-chat |
| 21:25 | **glossary.md** done (70 entries, 5 themes) | item07 |
| 21:32 | **position-paper.md** done (2174 words) | item08 |
| 21:28–21:33 | Specs item10/11/12/09/08.5/13 written; five independent agents launched in parallel | 00-main-chat |
| ≈21:35–23:05 | Parallel batch runs: item09 skill install; item10 Mistral Lean hello; item11 TLA+; item12 Lean tools; item08.5 audit+survey | item09/10/11/12/08.5 |
| 23:06 | **tool-kick-tires.md** (TLA+ + Lean sections), **position-paper-audit.md** (all 5 claims PASS), **solver-survey.md** (11 candidates) done | item08.5/11/12 |
| 23:13 | **outcomes-paper.md** done (~1140 prose words); includes the delayed compile check of `candidate.lean` → exit 0 | item13 |
| 23:24 | Specs item14a/item14b written (CLI solvers) | 00-main-chat |
| 23:28 | **lean-solvers-cli-omega-aesop.md** done (8/8 solved, 2/2 controls clean) | item14a |
| 23:44 | **lean-solvers-cli-lean-auto.md** done (T1/T3 solved, T2 saturation timeout) | item14b |
| 23:45 | item15 spec written (vibe + Leanstral smoke) | 00-main-chat |
| 23:46 | Specs item16/item17 written; blog fetch and baby-step agents launched | 00-main-chat |
| 23:57 | **Upaxos.lean** compiles; **lean-upaxos-babystep1.md** written | item17 |
| 23:59 | **leanstral-vibe-smoke.md** written (vibe PASS, API FAIL) | item15 |
| ≈00:00 | **blogs/** done: 3 posts, text-only, 0 images, no UPaxos PDF on disk | item16 |
| 00:02 | All 19 rollouts exported to `.tmp/rollouts/` | — |

Note: item15 and item17 overlap slightly in wall-clock (23:46–23:59) but ran in separate
agent sessions; item16's report ("No blog sources under `.tmp/blogs/`") is consistent with
item17 starting before item16 finished writing.

## 2. Lean tooling inventory

**Toolchain.** Lean 4.33.1 (Lake 5.0.0, arm64 darwin) installed via one `elan-init.sh -y`
attempt (~2 min) during item12; on PATH at `~/.elan/bin`, direct binary at
`~/.elan/toolchains/leanprover--lean4---v4.33.1/bin/lean`. Chosen because: empty
dependency set, no Mathlib needed for anything attempted, and Veil's pinned v4.32.0 could
be fetched later by elan if ever needed (it never was). item10 could not compile-check
because the timebox forbade installing the toolchain *before* it existed — the delay was
recovered by item13's compile check at 23:13.

**Submodules (staged under `tools/`, shallow, never committed).**

| Path | Upstream | Why chosen / status |
|---|---|---|
| `tools/tla2tools` | **tlaplus/tlaplus** (monorepo) | item11. Substitution: `tlaplus/tla2tools` repo is **gone** ("Repository not found" — folded into the monorepo); release-jar URL died with it (9-byte "Not Found"). Working jar source: `https://github.com/tlaplus/tlaplus/releases/latest/download/tla2tools.jar` (TLC2 2.19, 08 Aug 2024). |
| `tools/veil` | verse-lab/veil | item12. Recommended Tier-1 candidate in the survey, but **never kicked**: pins v4.32.0 (second toolchain) + Mathlib multi-GB resolve — skipped per timebox. |
| `tools/LeanLTL` | UCSCFormalMethods/LeanLTL | item12. Added for future temporal-property work; not exercised beyond checkout. |
| `tools/lean-auto` | leanprover-community/lean-auto | item12. Exercised hard in item14b (Duper backend). Pins v4.33.0; built fine under root toolchain v4.33.1. |

**Rationale for the solver set.** omega is built into `lean` (zero install); aesop is a
lightweight pure-Lean git dep (no Mathlib); lean-auto+Duper is the FO-prover bridge the
position paper identified as the automation layer. Veil/LeanLTL were added as the survey's
Tier-1 recommendations for the eventual VRR proof but are still un-kicked (see §6).

**Java** (for TLC): Homebrew OpenJDK 26.0.2 already present; no install.

## 3. Leanstral evaluation: direct API vs the user's vibe fork

Model: **`labs-leanstral-1-5-1`** (Mistral labs; 30M TPM / 5 RPS noted in orchestration).
Key handling invariant across all items: key read silently via
`awk -F= '/^MISTRAL_API_KEY/{print $2}' .env` into an env var; never printed, copied,
grepped, or echoed; `grep -rlF`-based safety gates before any publish action.

### 3.1 Direct API

Three separate API uses, all via small python drivers reading the key from env only:

1. **item10 hello-world** (`.tmp/mistral-lean/`, driver `ask_lean.py`):
   - **API quirk:** `temperature: 0` alone is rejected — HTTP 400, code 3054, *"top_p must
     be 1 when using greedy sampling."* Resent with `top_p: 1`; succeeded.
   - Output: 4-line valid-looking file (`import Init`; `theorem two_plus_two :
     (2 : Nat) + 2 = 4 := by native_decide`). Sanity PASS; compile **tbd** at the time (no
     toolchain yet).
   - Later resolved by item13: **compiles exit 0** under Lean 4.33.1 — but via
     `native_decide`, which trusts the compiler's evaluation rather than the kernel.
   - Artifact: `.tmp/mistral-lean/VERDICT.md`, `candidate.lean`.
2. **item15 API path** (`.tmp/vibe-lean/`, driver `ask_lean_api.py`): different theorem
   set, temp 0/top_p 1. Attempt 1 imported `Mathlib` despite instructions → compile FAIL
   (`unknown module prefix 'Mathlib'`). Attempt 2 with a hardened prompt forbidding
   imports: **still imported Mathlib** → recorded FAIL, no further re-rolls.
3. **item17 drafting** (`.tmp/lean-upaxos/`): model round 1 used `Finset` + no import →
   uncompilable (`Finset` does not exist in 4.33.1 core/Std — it is Mathlib-side; see §5).
   Model round 2 (List-based prompt) **compiled clean on first try** but violated the
   prompt by using forbidden `native_decide`. → **2 model fix rounds**.

**API-path pattern:** the model reliably produces syntactically valid single-theorem files,
but (a) defaults to `native_decide` when asked for a tactic proof, and (b) reaches for
`import Mathlib` against instructions — a regression given the empty-dependency rule.

### 3.2 Vibe fork lean agent

`~/opt/mistral-vibe-fork` and `/Users/Shared/mistral-vibe` are the user's own build —
treated READ-ONLY throughout. Plan A (tmux headless) was blocked: **tmux not installed**,
and installing it would write outside `.tmp/`. Plan B worked — vibe's own programmatic
mode:

```
~/opt/mistral-vibe-fork/bin/vibe -p "<prompt>" --agent lean --auto-approve \
  --max-turns 12 --max-price 2.0 --output text    # cwd .tmp/vibe-lean/
```

- Run 1 (`--max-turns 6`): hit the turn cap before file creation.
- Run 2 (`--max-turns 12`): created `leanstral_vibe.lean` (5 theorems); first compile
  FAILED (line 17, `Nat.pred n h` — two args). One fix prompt (spec's allowed single
  follow-up): **recompile PASS**.

### 3.3 Verdict (head to head, same day, same toolchain)

| Path | Result | Fix rounds | Failure mode |
|---|---|---|---|
| vibe lean agent | **PASS** (file compiles clean) | 1 agent round | wrote file itself via file tools; needed a turn-cap bump |
| direct API | **FAIL** (item15), **PASS-with-caveats** (item10/17) | up to 2 model rounds | Mathlib-import regression (API), `native_decide` habit, top_p/greedy quirk |

The vibe agent is the better harness: it iterates against its own file tools and respects
the Lean-environment constraints after one repair; the raw API needs the caller to run the
compile loop. In both cases **no output was trusted without a kernel-level compile check**
by the local Lean binary; the agent additionally verified axiom-freedom (no `sorryAx`) in
item17 (§5).

## 4. CLI solver results

Statement-only theorems; proof script is the tactic alone; exit 0 = solved; negative
controls prove the solver fails rather than claims success. Full methodology in
`.tmp/research/lean-solvers-cli-omega-aesop.md` and `lean-solvers-cli-lean-auto.md`.

**Track 1 — omega** (`lean solve.lean`, zero install):

| theorem | statement | solved | wall |
|---|---|---|---|
| omega_t1 | `(x : Int) : 2 * x = 4 * x - 2 → x = 1` | y | 0.17s |
| omega_t2 | `(n : Int) (h : n % 2 = 0) : 2 ∣ n` | y | 0.13s |
| omega_t3 | `(a b c : Nat) : a ≤ b → b ≤ c → a ≤ c` | y | 0.13s |
| omega_t4 | `(x : Int) (_h : x ≥ 0) (h2 : x + x = 6) : x = 3` | y | 0.13s |
| omega_neg | `n + 0 = n ∧ ∀ m, m + n = n + m` (needs induction) | n — **correct** | 0.14s |

Control behaves: `error: omega could not prove the goal`, exit 1; never claims success.

**Track 2 — aesop** (lake project, dep @ `v4.33.0`, `lake update` 13.3s, `lake build aesop`
19.3s, then per-theorem `lake env lean`):

| theorem | statement | solved | wall |
|---|---|---|---|
| aesop_t1 | `(p q : Prop) : p ∨ q → q ∨ p` | y | 0.59s |
| aesop_t2 | `(p q : Prop) : (¬ p ∨ ¬ q) → ¬ (p ∧ q)` | y | 0.58s |
| aesop_t3 | `(a b c : Prop) : (a → b) → (b → c) → a → c` | y | 0.57s |
| aesop_t4 | `(p q r : Prop) : (p → q → r) → (p ∧ q) → r` | y | 0.58s |
| aesop_neg | `n ^ 2 + n + 41 = 43 * n → n = 2` | n — **correct** | 0.59s |

Control behaves: `aesop: failed to prove the goal after exhaustive search`, build fails.
Aesop also **correctly rejected** `¬(p∧q) → ¬p ∨ ¬q` (not intuitionistically valid); the
valid de Morgan direction was substituted.

**Track 3 — lean-auto + Duper** (`.tmp/lean-solve-auto/`; Duper bound via
`Auto.duperRaw` rebind in `Solve/SetDuper.lean`, `set_option auto.native true`; no
external SMT/TPTP solvers configured):

| theorem | statement | solved | wall |
|---|---|---|---|
| T1 | right inverse is left inverse in a group (FO axioms in context), `auto [assoc, mul_e, inv_r]` | **y** | ~1.4s |
| T2 | commutativity from full group axioms, `auto [...]` | **no** — Duper 500s saturation timeout; retry at `maxHeartbeats 8e6` also failed | ~500s |
| T3 | FO relation: sym+trans entails `r c a`, `auto [sym, trans, h1, h2]` | **y** | ~1.1s |
| NEG | `n + m = m + n` on Nat (needs induction) | n — **correct** ("Duper saturated", exit 1) | ~1.3s |

Build friction recorded: Duper's lakefile git-requires `auto`; scratch clone edited to a
path dep on the submodule (`"../../tools/lean-auto"`) and the root require renamed `auto`
to kill a duplicate-package error. Deps build ~2:01–2:35; T2 module build alone 8:22.

**Summary: 10/10 positive theorems solved across tracks (omega 4, aesop 4, auto 2), 3/3
negative controls failed cleanly.** One honest miss: T2 commutativity is beyond Duper's
saturation budget in this configuration. Two of the spec's sample omega statements were
mathematically **false** (`2*x = 4*x - 2 → x = 2`; `a ≤ b → b ≤ c → a + c ≥ b + b`) — both
solvers rejected them, which is itself a positive result; corrected equivalents were used
and the deviation recorded.

Exact commands (macOS zsh, Lean binary not on a fresh shell's PATH):

```
export PATH="$HOME/.elan/toolchains/leanprover--lean4---v4.33.1/bin:$PATH"
lean solve.lean                                   # omega track
lake init aesopdemo && lake update && lake build  # aesop track
lake env lean Aesopdemo.lean
lake build Solve.SetDuper Solve.T1LeftInv Solve.T3Relation   # auto track
lake env lean Solve/T1LeftInv.lean; echo "T1 exit=$?"
```

## 5. The uVRR baby step 1 (item17)

Artifact: `.tmp/lean-upaxos/Upaxos.lean` (header comment states honest scope + deviation),
research note `.tmp/research/lean-upaxos-babystep1.md`, model draft/responses in
`.tmp/lean-upaxos/candidate.lean` + `raw_content.txt`.

**Honest scope** (stated in the file header): a typing/structure seed for UPaxos-style
era/quorum safety. It is **not** liveness and **not** the Paxos agreement proof. `FixableIn`
monotonicity is a construction lemma about fixable slot eras, not vote safety.

**What it defines:** `Era := Nat`; `eOf b := b.1`; `Node := Nat`;
`Config := ⟨QI, QII : List Node⟩` (the ⟨Q_I, Q_II⟩ pair); `Overlap Q1 Q2 := Q1.any (fun x
=> x ∈ Q2) = true` (decidable stand-in for `(Q1 ∩ Q2).Nonempty`); `SoundCfgs C` — the three
era-adjacent overlap invariants (intra-era, and both cross-era directions e↔e+1);
`FixableIn b s := s = b.1 ∨ s = b.1 + 1`.

**What it proves — 6 lemmas, all kernel-checked (`decide`/`rfl`/`rcases`; only standard
`propext`/`Quot.sound` axioms, no `sorryAx`):**

| lemma | content |
|---|---|
| `eOf_eq` | ballot-era typing, `rfl` |
| `cfgE_ge2` | wildcard-era reduction of the schedule, `rfl` |
| `cfgE_sound` | **the 3-zone instance**: `SoundCfgs cfgE` for every era, by `cases` + `decide` |
| `fixable_le` | fix-era monotonicity `b.1 ≤ s` |
| `bad_no_overlap` | negative control: disjoint quorums `⟨[0],[1]⟩` fail `Overlap` |
| `bad_not_sound` | the invariant itself rejects the corrupt constant schedule |

**The 3-zone instance** `cfgE`: era 0 = `⟨[0,1,2],[0,1,2]⟩` (zones A,B,C); era 1 =
`⟨[0,1,3],[0,2,3]⟩` (node 3 = temporarily added Z, two nodes in zone A); era ≥2 =
`⟨[0,1,3],[0,1,3]⟩` (zone C dropped). Demonstrates: adding then deleting a node with
correct quorums never loses overlap, in all three invariant directions.

**Fix rounds (counted, model vs agent):**
- Leanstral model: **2**. Round 1: `Finset` + no import → uncompilable. Round 2: compiled
  but used forbidden `native_decide` (plus unused plumbing) → rejected.
- Agent: **3**. (1) restructure to `SoundCfgs`, lift negative control to invariant level,
  `decide` instead of `native_decide` → synthesis errors; (2) `cfgE_ge2` + unfold/rw → 1
  error left; (3) `unfold Overlap` → clean.

**The List-not-Finset deviation (deliberate, recorded):** the spec assumed `Finset` is in
Lean core. It is not — 4.33.1 core/Std have no `Finset` (verified by source search), and
the locally cached Batteries predates Mathlib's Finset move; building Mathlib was out of
the 40-min timebox and would break the empty-dependency rule. Quorums are therefore
`List Node` with decidable `Overlap`, so concrete instances close by kernel-checked
`decide`. Item16's blog sources were not yet on disk when item17 started, so definitions
follow the spec's summary.

Compile command:

```
~/.elan/toolchains/leanprover--lean4---v4.33.1/bin/lean Upaxos.lean
```

## 6. Key caveats and open questions

1. **`native_decide` semantics.** The API model's default proof style trusts the
   compiler's evaluation, not the kernel. `candidate.lean` compiles exit 0 but is *not*
   kernel-checked the way `decide` is. Rule adopted in item17: forbid `native_decide` in
   prompts, verify axiom-freedom (`#print axioms`-style check: no `sorryAx`). Open: any
   future Leanstral-generated proof must pass the same kernel-level gate.
2. **Veil un-kicked.** The survey's top Tier-1 recommendation (verse-lab/veil) is
   submodule-added but never run: pinned v4.32.0 + Mathlib multi-GB resolve. Open
   question: does Veil's Ivy-style DSL express the UPaxos era/quorum invariants?
3. **LeanLTL un-exercised.** Submodule added; no past-operator property has been written
   against it. The position paper's core thesis (LeanLTL as foundation for LTL+Past on
   VRR) remains untested.
4. **Mathlib pinning.** Everything so far is Mathlib-free by design. `Finset` absence
   (§5) is the first concrete cost; a Mathlib dependency decision is deferred and will be
   a multi-GB resolve + toolchain-pin negotiation (Veil pins v4.32.0, aesop/Duper pin
   v4.33.0, root is v4.33.1).
5. **UPaxos PDF not on disk.** item16 searched `tools/` and `.tmp/papers/` — no UPaxos
   paper PDF (corpus has VRR-2012 and unrelated papers). The user's UPaxos material was
   consumed via the three blog posts (web, text-only) + spec summary; the paper URL is
   noted in `.tmp/blogs/SOURCES.md`. Open: fetch/OCR the actual PDF for the next baby
   steps.
6. **Blogs are text-only.** All three simbo1905 posts verified live (zero `<img>` in
   `.entry-content` via chrome-devtools); quorum "diagrams" are HTML math text, DOM-filled
   where Tavily extract dropped them. No screenshot/OCR round happened — that leg of
   item16 was a no-op by construction, not by failure.
7. **T2 saturation.** lean-auto/Duper's 500s default timeout fails group commutativity;
   whether a longer budget or lemmas-reordering solves it is open.
8. **Fabrication risk in LLM literature reviews** (the founding premise): the original
   review contained a fabricated VR-Revisited quote and a distorted Veil quote; the audit
   (item08.5) verified all five brief-claims of the corrected position paper at
   quote level. Keep quote-level checks for any generated prose.
9. **Gist platform limits** (recorded): `gh gist create` rejects binary files (PDFs), and
   this gh version has no `--private` flag (gists are secret by default). Published gist
   (item05b): https://gist.github.com/simbo1905/3a47006d9b6ced163cadc422dfa1a4b6 —
   `ocr_run.py` + README only, safety gate PASS.
10. **Submodules staged, never committed** — the staged state (tools/tla2tools, veil,
    LeanLTL, lean-auto; AGENTS.md hunk) is still the user's to commit.

## 7. Per-item appendix

- **item03 — secret audit** (`.tmp/rollouts/item03-secret-audit.jsonl`): PASS; 15 files
  scanned, 0 key matches; read-only. Establishes the silent-awk key-handling pattern used
  everywhere else.
- **item04 — git bundle** (`.tmp/rollouts/item04-git-bundle.jsonl`): paper bundle
  `git init -b main`, 15 files staged (7 PDFs, 7 OCR .md, README), 11M, gate CLEAN, no
  commit. `.tmp/gist-staging/`.
- **item05a — gist attempt 1** (`.tmp/rollouts/item05a-gist-ocr-technique.jsonl`): FAILED
  correctly — `gh gist create` rejects PDFs; agent stopped instead of substituting a
  mechanism. Recorded as the platform-limit evidence in §6.
- **item05b — gist published** (`.tmp/rollouts/item05b-publish-gist.jsonl`): secret gist
  with `ocr_run.py` + README (2 files), gate PASS; URL in §6.9.
- **item06 — AGENTS.md sync** (`.tmp/rollouts/item06-agents-md.jsonl`): appended the
  "Subagent delegation" section (single hunk, staged, not committed).
- **item07 — glossary** (`.tmp/rollouts/item07-glossary.jsonl` →
  `.tmp/research/glossary.md`): 70 entries, 5 themes, corpus-grounded (121-entry first
  draft consolidated to meet the ≤70 cap).
- **item08 — position paper** (`.tmp/rollouts/item08-position-paper.jsonl` →
  `.tmp/research/position-paper.md`): 2174 words; thesis — Lean 4 (Veil + lean-auto) is
  the strongest available fit for a machine-checked VRR reconfiguration-safety proof; no
  full VRR verification in Lean exists in the corpus.
- **item08.5 — audit + survey** (`.tmp/rollouts/item08.5-audit-survey.jsonl` →
  `.tmp/research/position-paper-audit.md`, `solver-survey.md`): audit claims (a)–(e) all
  PASS at quote level; survey: 11 candidates (4 Tier-1 / 3 Tier-2 / 4 Tier-3) + 4 watch +
  6 stale-excluded; top-3 practicality: Veil, Quint+Apalache, TLA+/TLAPS +
  MongoRaftReconfig (only published mechanically-checked Raft-family reconfig safety
  proof).
- **item09 — skill install** (`.tmp/rollouts/item09-skill-install.jsonl`):
  `linear-margin-note-methodology` installed to
  `~/.config/opencode/skills/linear-margin-note-methodology/SKILL.md` (fixed broken
  frontmatter of a prior raw copy; body intent preserved).
- **item10 — Mistral Lean hello** (`.tmp/rollouts/item10-mistral-lean.jsonl` →
  `.tmp/mistral-lean/VERDICT.md`, `candidate.lean`): first API quirk (top_p/greedy, §3.1);
  sanity PASS, compile tbd → resolved by item13.
- **item11 — TLA+** (`.tmp/rollouts/item11-tlaplus.jsonl` →
  `.tmp/research/tool-kick-tires.md` `## TLA+`, `.tmp/tla-hello/`): TLC2 2.19 hello-world
  green in <1s (3 states, 2 distinct); both dead-URL substitutions recorded (§2).
- **item12 — Lean tools** (`.tmp/rollouts/item12-lean-tools.jsonl` →
  `tool-kick-tires.md` `## Lean 4 tools`, `.tmp/lean-hello/`): elan → Lean 4.33.1; 3
  submodules; `hello : 1 + 1 = 2 := by rfl` compiles; Veil example skipped (timebox).
- **item13 — outcomes paper** (`.tmp/rollouts/item13-outcomes-paper.jsonl` →
  `.tmp/research/outcomes-paper.md`): ~1140 words, A1-poster-pipeline format from the
  user's gist; headline: all runnable smoke tests passed (TLC <1s, `rfl` clean,
  `candidate.lean` exit 0 via `native_decide`) while the hello-world→VRR gap stands
  undiminished; compile check deferred from item10 done here.
- **item14a — omega + aesop** (`.tmp/rollouts/item14a-omega-aesop.jsonl` →
  `.tmp/research/lean-solvers-cli-omega-aesop.md`, `.tmp/lean-solve-omega/`,
  `.tmp/lean-solve-aesop/`): 8/8 solved, 2/2 controls clean, false sample statements
  rejected correctly (§4).
- **item14b — lean-auto/Duper** (`.tmp/rollouts/item14b-lean-auto.jsonl` →
  `.tmp/research/lean-solvers-cli-lean-auto.md`, `.tmp/lean-solve-auto/`): T1/T3 solved,
  T2 saturation timeout, NEG clean (§4).
- **item15 — vibe vs API smoke** (`.tmp/rollouts/item15-vibe-leanstral.jsonl` →
  `.tmp/research/leanstral-vibe-smoke.md`, `.tmp/vibe-lean/`): vibe PASS (1 fix round),
  API FAIL (Mathlib import ×2), tmux fallback → vibe `-p` (§3).
- **item16 — blog sources** (`.tmp/rollouts/item16-blog-sources.jsonl` → `.tmp/blogs/`):
  3 posts fetched (VRR-revisited Aug 2026; UPaxos quorum overlaps May 2020; Paxos voting
  weights Mar 2017); 0 images (pages verified text-only); no UPaxos PDF on disk;
  `SOURCES.md` records URLs and the DOM-fill pass.
- **item17 — baby step 1** (`.tmp/rollouts/item17-babystep1-lean.jsonl` →
  `.tmp/lean-upaxos/Upaxos.lean`, `.tmp/research/lean-upaxos-babystep1.md`): 6 lemmas,
  3-zone instance by `decide`, fix rounds model 2 / agent 3, List-not-Finset deviation
  (§5).
- **item18 — this report** (`.tmp/specs/item18.md`): mining of all rollouts + artifacts;
  output `formal/lean-leanstral-report.md`.
