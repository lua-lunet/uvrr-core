# Claude + Leanstral: an orchestrated proof effort, fact-checked from the record

*2026-09-06. Every claim in this article was verified against the session transcript
(`d7ee54df-399f-4683-877d-c1e375685e08`, model `claude-fable-5-1`) mined with the
`claude-chat-history` skill, and against the ground-truth artifacts left on disk in
`vrr-core`. Evidence ledger in Appendix A.*

---

## 1. The question

Claude Fable 5.1 (Claude Code 2.1.261, branch `uVRR`) reported at 06:53Z on 2026-09-06:

> There is no separate script. Leanstral was driven with a one-line headless launch of
> your vibe fork … **Did I use Leanstral? Yes, once**, for the one target I judged worth
> an experiment: the paper's general weighted-majority Lemma 2. It did not land within
> the timebox (Mathlib idioms, then a 30 GB runaway Lean process), and rung 8 records
> that. Rungs 1 to 7 I wrote and compiled directly.

Two things needed checking: (a) did Claude actually drive Leanstral, or was that a
hallucination; (b) against Mistral's own report
([mistral.ai/news/leanstral](https://mistral.ai/news/leanstral/)), what is this
channel, and what did each model contribute to the delivered evidence set and final
paper.

**Verdict: not a hallucination. Every checkable claim in Claude's answer is
corroborated by an on-disk artifact or a measured number in the transcript.** The
accurate attribution is: *Claude wrote the proof; Leanstral was subcontracted for one
lemma and did not deliver* — a "Claude + Leanstral" pipeline, not "Leanstral solved it".

## 2. What Leanstral is (Mistral's own report)

From [https://mistral.ai/news/leanstral/](https://mistral.ai/news/leanstral/)
(Research post, **March 16, 2026**):

- **First open-source code agent designed for Lean 4** — not a wrapper around a
  generalist model, but a sparse architecture model **Leanstral-120B-A6B** (6B active
  parameters), trained for operating in realistic formal repositories (the FLT project
  is its evaluation substrate, via the new **FLTEval** suite of PR-completion tasks).
- Distributed three ways: **Apache 2.0 weights**, an **agent mode inside Mistral Vibe**
  (`vibe --agent lean`, or `/leanstall` + `Shift+Tab` in the CLI), and a free
  **Labs API** endpoint (`labs-leanstral-2603` at release; the project's vibe fork
  currently pins **`labs-leanstral-1-5`**, the June 30 2026 update).
- Positioned on cost curves: on FLTEval, pass@2 scores 26.3 for $36 vs Sonnet's 23.7
  for $549; **Opus remains the quality leader (39.6) at 92× Leanstral's pass@16 cost**.
  Mistral used "Mistral Vibe as the scaffold with no modifications".

That last point matters here: Mistral itself benchmarks Leanstral *under a scaffold*,
against Claude-family agents as competitors. The vrr-core experiment inverts the
composition: **a Claude agent as the orchestrating scaffold, Leanstral as a
subcontracted proof drafter.**

## 3. What the record says happened

The session (2026-09-05T23:14Z → 2026-09-06T07:50Z local) was asked to prove the
leader-overlap / reconfiguration-safety results for uVRR in Lean 4.33.1 **core only**
(no Mathlib), from the UPaxos paper and the project blogs. Claude built a "ladder":
small, independently compilable rungs, each packaged as an executable markdown
document with **showboat 0.6.1** (Simon Willison's
[simonw/showboat](https://github.com/simonw/showboat) — the user's "shoboard" is this
tool) so every command in every rung document can be re-run and diffed.

### 3.1 The Leanstral channel is real

The vibe fork's agent registry binds the `lean` agent to Leanstral directly:

```
vibe/core/agents/models.py:
  "active_model": "leanstral"
  "allowed_models": ["leanstral"]
      "name": "labs-leanstral-1-5",
      "alias": "leanstral",
```

Claude launched it headless, twice, from a sandbox `.tmp/uvrr-ladder/leanstral/`:

```sh
~/opt/mistral-vibe-fork/bin/vibe -p "$(cat PROMPT.txt)" --agent lean \
  --auto-approve --trust --max-turns 60 --max-price 6.0 \
  --output streaming --workdir "$PWD" > vibe-run2.log 2>&1 &
```

- **Launch 1 (no `--trust`)**: `vibe-run1-blocked.log` is **0 bytes** — vibe sat on its
  working-directory trust prompt with stdout redirected, invisible. Killed after ten
  minutes. (The gotcha Claude mentioned is real and reproducible: headless + trust
  prompt = silent hang.)
- **Launch 2 (`--trust`)**: `vibe-run2.log` (330,088 bytes, 44 JSONL records) holds a
  complete Leanstral agent turn:
  - 1 `user` message = `PROMPT.txt` **byte-identical** to the preserved copy in
    `formal/uvrr-lean/leanstral/PROMPT.txt` (fix the one `sorry` in
    `theorem majOverlap`, core-only, forbidden Mathlib/sorry/axiom/native_decide,
    run `lean` to check, iterate).
  - 20 `reasoning` blocks and 23 tool `effect`s: **12 file_search, 6 file_read,
    2 shell, 2 file_write, 1 file_edit** — it read the elan toolchain's
    `Init/Data/List/*` sources, edited `Weights.lean` in place, and ran the pinned
    `~/.elan/toolchains/leanprover--lean4---v4.33.1/bin/lean` repeatedly.
  - Log window 23:32:52Z → 23:47:36Z; vibe's process etime at 23:49:42Z was **16:51**
    ("ran 17 minutes" — checks out).

So the answer to "did you use it?" is yes, with machine evidence: an authentic vibe
session log in the project's own JSONL format, describing edits to the exact file the
prompt names.

### 3.2 The failure is real, and the failure numbers are measured

- Leanstral produced a **105-line draft** (`Weights.lean` → preserved as
  `formal/uvrr-lean/leanstral/Lemma2-leanstral.lean`) with a **sound proof plan**
  (sums distribute over append; a sublist's weighted sum is bounded by the total; two
  disjoint sublists of a Nodup list sum to at most the total; the single node where
  the scaled weights differ gives the contradiction) — visible in its `reasoning`
  stream: *"Let me start by understanding the current state of the file…"* through an
  induction on nodes, then a restart: *"This approach with the induction on nodes is
  getting very messy. Let me think of a simpler proof for `sum_two_disjoint_sublists_le`."*
- But the file does not compile, for exactly the reason Claude stated: it uses the
  **Mathlib keyword `lemma`** and the **Mathlib names `add_le_add_left` and
  `List.sum_append_nat`**, none of which exist in Lean core. The recorded compiler
  errors (`unexpected identifier; expected 'abbrev', 'axiom', 'def', 'inductive',
  'theorem', …` at each `lemma` site) are the Lean core parser literally listing the
  keywords available — `lemma` is not among them. Final draft state: **7 `sorry`**.
- The **30 GB** figure is not an estimate: at 23:49:42Z Claude measured
  `ps -A -o rss=` over the lean/vibe processes and summed
  **`lean+vibe RSS MB: 30427.2`**. The same minute, the harness killed the background
  waiter with the system notice *"stopped because the system is running low on
  memory"*. So a Leanstral-issued `lean` check was ballooning resident memory toward
  30 GB on a small core-only file while still reporting errors and unsolved goals.
- Timebox closed; verdict **FAILED within the timebox**; recorded honestly as rung 8
  (`formal/uvrr-lean/ladder/08-leanstral-lemma2.md`, showboat-id
  `be91b123-e539-4f6d-906a-d265ddf465f4`), with the statement that nothing in rungs
  1–7 depends on it.

### 3.3 Who wrote what

| Work | Author (per record) | Evidence |
|---|---|---|
| Rungs 1–7 (7 Lean modules, ~10 s `lake build`) | **Claude, written directly** | `formal/uvrr-lean/UVRR/*.lean`, ladder docs 01–07; the transcript shows Claude authoring them whole (e.g. `LexBallot.lean`, `Counterexample.lean` written as single heredocs), with one fix round on Rungs 2 and 5 |
| General weighted-majority Lemma 2 (paper's App. A) | **Leanstral, attempted, failed** | `vibe-run2.log`, `Lemma2-leanstral.lean` (7 `sorry`, non-compiling) |
| Executable evidence documents | Claude, generated by **showboat** | `ladder/0*.md`, `showboat verify` re-runs every command |
| Final paper | Claude | `formal/uvrr-lean/uVRR-reconfiguration-safety-lean.docx` (20,243 bytes, built via `docx` + validated) |
| Earlier direct-API experiments | Claude driving `labs-leanstral-1-5-1` via `ask_lean.py` | `.tmp/mistral-lean/{ask_lean.py,VERDICT.md}` (hello-world; model chose `native_decide`), copy in `.tmp/lean-upaxos/` — that channel "kept importing Mathlib", which is why Claude picked the agent channel this time |

Claude's own recorded assessment of the composition is worth keeping:

> Leanstral through the vibe lean agent is useful as a planner and a fast route to a
> first draft … but for Mathlib-free targets it still needs a human (or a stricter
> harness that rejects Mathlib identifiers before running Lean) to land a proof.

## 4. Reading this against Mistral's report

Mistral's pitch is Leanstral as the cheap, efficient **prover inside a scaffold**
(their vibe). Three observations from this one recorded case, consistent with their
own cost/quality table:

1. **The scaffold, not the prover, carried the proof.** The seven deliverable rungs
   were written by the orchestrating Claude agent and kernel-checked in seconds.
   Leanstral's single delegated lemma did not land. This matches Mistral's own
   benchmark stance that Opus/Claude-family agents lead on quality; what the record
   adds is that the cheaper specialist's value here was *plan-quality* (its proof
   plan was judged sound by the orchestrator) rather than deliverable Lean.
2. **The failure mode is domain mismatch, not capability theater.** FLTEval and the
   Leanstral training distribution are Mathlib-flavored realistic repositories; the
   target here was a deliberately Mathlib-free core-only codebase. The model reached
   for `lemma`/`add_le_add_left` even with the prohibition in the prompt *and* in the
   file header. Claude's proposed mitigation — a harness that rejects Mathlib
   identifiers before invoking `lean` — is the actionable engineering takeaway.
3. **Agent-runner ergonomics are part of the contract.** The `--trust` hang (launch 1,
   zero bytes out for ten minutes) and the 30 GB `lean` process are harness-level
   facts the model report does not discuss. Both are now durable records
   (`.tmp/uvrr-ladder/leanstral/vibe-run1-blocked.log`, the measured RSS line in the
   transcript at 23:49:42Z).

Also notable: a **specialized 6B-active-parameter prover still needs a general agent
to define "what to prove, under which constraints, and when to stop"**. Claude fixed
the lemma statement, the definitions, the core-only rule, the checker command, and
the timebox before Leanstral saw anything — that contract is what makes a failure
cheap, bounded, and honestly recorded instead of open-ended.

## 5. The delivered artifacts

- **Evidence set**: `formal/uvrr-lean/ladder/01..08-*.md` — eight showboat 0.6.1
  executable documents (each rebuilds its module and prints axiom footprints;
  `showboat verify` re-runs and diffs every command).
- **Final paper**: `formal/uvrr-lean/uVRR-reconfiguration-safety-lean.docx` —
  abstract, prior art, the ladder table, uVRR correspondence, Leanstral outcome §6,
  "what is not proved" §7, verification recipe §8.
- **Rung 1–7 result** (all Lean 4.33.1 core, no `sorry`/`native_decide`/axioms beyond
  `propext`/`Quot.sound`/`Classical.choice` via `omega`): Synod S1–S6 ⇒ Theorem 8;
  P1 + era rules ⇒ Lemma 9, Theorem 10 (agreement across reconfiguration); a
  two-node counterexample proving the cross-era frown **necessary**; casting-vote
  non-interference; the blog's 7-row weighted schedule checked with negative control.
  The general Lemma 2 remains open, and nothing stands on it.

## Appendix A — Evidence ledger

| Claim (Claude's answer) | Status | Primary evidence |
|---|---|---|
| Used Leanstral once, via headless vibe fork `--agent lean` | **Confirmed** | `vibe-run2.log`: 1 user msg = PROMPT.txt verbatim, 20 reasoning, 23 tool effects; fork config binds `lean` → `leanstral` (`labs-leanstral-1-5`) |
| First launch sat silently on the trust prompt | **Confirmed** | `vibe-run1-blocked.log` = 0 bytes; rung-8 doc |
| Ran ~17 minutes on launch 2 | **Confirmed** | log window 14.7 min; process `etime=16:51` at 23:49:42Z |
| Draft has sound plan but Mathlib idioms; doesn't compile | **Confirmed** | `Lemma2-leanstral.lean`: `lemma`, `add_le_add_left`, `List.sum_append_nat`; core parser rejects `lemma`; 7 `sorry` |
| ~30 GB runaway Lean process | **Confirmed (measured)** | transcript 23:49:42Z: `lean+vibe RSS MB: 30427.2`; harness killed waiter "system running low on memory" |
| Inputs preserved in `formal/uvrr-lean/leanstral/` | **Confirmed** | `PROMPT.txt` (byte-identical), `Lemma2-leanstral.lean` |
| Older direct-API script `ask_lean.py` + copy, unused this session | **Confirmed** | `.tmp/mistral-lean/ask_lean.py`, `.tmp/lean-upaxos/ask_lean.py`; no API calls in this session's log |
| Rungs 1–7 written and compiled directly by Claude | **Confirmed** | 7 modules under `formal/uvrr-lean/UVRR/`, ladder docs 01–07, `lake build` results in transcript |
| Evidence set + final paper delivered | **Confirmed** | 8 showboat documents; `uVRR-reconfiguration-safety-lean.docx` |

Session mined with `claude-chat-history` (up to date at fact-check time; single
commit `bfe7bef`, `git pull` clean). Miner queries used: `search "Leanstral"`,
`search "GB"`, `search "memory"`, raw grep on the transcript for measured tool
results, and cross-checks of the vibe fork source and `~/.claude/projects` memory
files. Tooling correction: the evidence-document tool is **showboat**
(`simonw/showboat`), not "shoboard".
