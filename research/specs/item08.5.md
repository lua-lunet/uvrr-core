# item08.5 — Audit the position paper + survey for missed CURRENT tooling

Two tasks. All outputs under `.tmp/research/`. No git. Keep the final report terse.

## A. Internal consistency audit (no network)

1. Read `.tmp/research/position-paper.md`, `.tmp/research/glossary.md`, and the
   corpus `.tmp/papers/ocr/*.md`.
2. Check the position paper against the glossary: terminology used consistently;
   no term contradicting its glossary definition; notable corpus terms missing
   from the paper.
3. Check the paper against the original fact-check brief's key claims (the brief
   contained errors the paper must NOT repeat):
   (a) the distorted Veil quote "the former provides no automation, while the
       latter lacks a logical foundation" — the paper must use the accurate
       "out of frustration" passage;
   (b) LeanLTL has no past operators yet (future work only);
   (c) the VR Revisited paper does NOT contain automated verification results
       (the brief's "logless replicated state machine ... automated verification
       of its safety properties" quote was from a different paper, not Liskov &
       Cowling);
   (d) no published full VR verification in Lean;
   (e) the brief's nonstandard "LTA+" terminology — the paper should use standard
       terms (LTL+Past / PLTL), not "LTA+".
4. Write `.tmp/research/position-paper-audit.md`: PASS/FAIL per check (a)–(e),
   glossary-consistency findings, and any discrepancies with page/line pointers.

## B. Obvious-oversight survey (use Tavily MCP search tools)

Goal: find CURRENTLY-TOOLED work on proving strong-consistency algorithms in
practice — Paxos, Raft, VSR/VRR family — that the corpus missed and that one
could realistically download/submodule and kick the tires on TODAY. EXCLUDE old
results with no maintained tooling.

Suggested search angles: "Raft formal verification tool", "Verdi raft verified
systems Coq", "etcd raft TLA+ specification", "Paxos verification Coq Isabelle
Lean 2025", "Viewstamped Replication formal verification", "Ivy mypyvy protocol
verification", "Apalache TLA+ 2025", "Verus consensus verification",
"PSync partially synchronous systems verification", "Disel verified distributed
systems", "Coq distributed consensus proof".

For each candidate: name; what it proves; tool + language; maintenance status
(last activity/release); URL; whether a timeboxed hello-world smoke test is
plausible. Write `.tmp/research/solver-survey.md`. Aim for 8–15 candidates,
prioritized by "can we submodule and run it today".

Report back (<8 lines, no content dumps): audit verdicts for (a)–(e), survey
candidate count, top-3 by kick-the-tires practicality, output paths.
