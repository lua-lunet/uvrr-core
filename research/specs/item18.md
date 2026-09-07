# item18 — Report: use of Lean and Leanstral across the vrr-core effort → formal/

Repo root: /Users/Shared/lua-lunet/vrr-core. Platform: darwin, zsh.

Task: mine the session rollouts in `.tmp/rollouts/` and write a comprehensive
report `formal/lean-leanstral-report.md`. THIS file write outside .tmp is
explicitly approved by the user; touch nothing else outside .tmp, no git.

CAUTION — rollouts are huge. Protocol (from the opencode-chat-history skill):
never read a whole .jsonl; profile first with
`awk '{print NR, length($0)}' file | sort -k2 -rn | head -20` to find the big
lines, then extract with jq:
- user prompts: `jq -r 'select(.role=="user") | .content' f.jsonl`
- assistant text: `jq -r 'select(.role=="assistant") | .content' f.jsonl | head -c 4000`
- tool names/commands: `jq -r 'select(.role=="assistant") | .tool_calls[]?.function.name' f.jsonl`
Pipe any jq output to a scratch file and profile before reading. Never dump
giant blobs into your context — sample and summarize.

Source rollouts (the whole effort):
- 00-main-chat.jsonl (the orchestrator chat; 278 lines)
- item03 secret audit; item04 git bundle; item05a/b gists; item07 glossary;
  item08 position paper; item08.5 audit+survey; item09 skill install; item06
  AGENTS.md; item10 Mistral Lean hello; item11 TLA+; item12 Lean tools;
  item13 outcomes paper; item14a omega+aesop; item14b lean-auto; item15 vibe
  leanstral smoke; item16 blog sources; item17 baby-step-1 Lean.

Also ground truth in .tmp: `.tmp/research/*.md` (leanstral-vibe-smoke,
lean-upaxos-babystep1, lean-solvers-cli-*, tool-kick-tires, outcomes-paper),
`.tmp/lean-upaxos/Upaxos.lean`, `.tmp/lean-solve-*/`, `.tmp/mistral-lean/`,
`.tmp/vibe-lean/`, `.tmp/specs/`. Cross-check rollout claims against these.

Report contents (be thorough and specific — this is the durable record):
1. Timeline of the effort (date/time, what ran when, which session).
2. Lean tooling inventory: what was installed (elan 4.33.1), submodules, and
   why each was chosen; the tla2tools-repo-gone substitution.
3. Leanstral evaluation: model `labs-leanstral-1-5-1` (direct API) vs the
   user's vibe fork lean agent (`vibe -p --agent lean --auto-approve`) —
   behavior differences (Mathlib-import regression via API; fix rounds;
   native_decide episode; top_p/temp quirks), exact commands used, verdicts.
4. CLI solver results table: omega, aesop, lean-auto/Duper — theorems solved,
   timings, negative controls, failures (T2 saturation timeout).
5. The uVRR baby step 1: what Upaxos.lean defines/proves (6 lemmas, 3-zone
   instance via decide), fix-round counts model vs agent, List-not-Finset
   deviation, honest scope.
6. Key caveats and open questions (native_decide semantics, Veil un-kicked,
   Mathlib pinning, UPaxos PDF not on disk, blogs text-only).
7. Per-item appendix: one short paragraph each with pointer to its rollout
   file and artifact paths.
Include exact CLI commands where they matter (macOS zsh). Do NOT include any
API key or env secret — never quote .env contents; if a rollout line contains
one, redact it.

Report back (<6 lines, no content dumps): word count, sections, artifacts
cross-checked count, output path.
