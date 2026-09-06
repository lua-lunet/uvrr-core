# item13 — Final outcomes paper (uses the user's gist as the guide)

Source gist: https://gist.github.com/simbo1905/6bdbf862052f6548afbac89fbb857264
(the user's own; gh is authenticated as simbo1905).

Inputs (all under `.tmp/`):
- `.tmp/research/tool-kick-tires.md` (item11 TLA+/TLC and item12 Lean-tool outcomes)
- `.tmp/mistral-lean/VERDICT.md` and `candidate.lean` (item10 Mistral
  labs-leanstral-1-5-1 outcome)
- `.tmp/research/solver-survey.md` and `.tmp/research/position-paper-audit.md`
  (item08.5)
- `.tmp/research/position-paper.md`, `.tmp/research/glossary.md`, and
  `.tmp/papers/ocr/*.md` as background context

Task:
1. Fetch the gist: `gh gist view 6bdbf862052f6548afbac89fbb857264`. If it cannot
   be fetched, report and stop. Follow its instructions as the format/method for
   the paper (template → use it; guidance → apply it; preserve the author's
   intent, fix formatting only).
2. Write `.tmp/research/outcomes-paper.md`: a paper reporting what was found,
   focusing on the OUTCOMES of the tools we can actually run — which tools solved
   the simple tasks (hello-world smoke tests), and how those outcomes map to what
   the literature (position paper + glossary) claims could be good for our
   situation: proving VRR cluster-reconfiguration safety in vrr-core. Cover at
   minimum: Mistral labs-leanstral-1-5-1 result; TLA+/TLC result; Lean tools
   (Veil/LeanLTL/lean-auto) result; the 08.5 survey candidates and their
   kick-the-tires practicality ranking; explicit notes on skipped/failed tools
   with reasons.
3. Ground every claim in the `.tmp` artifacts; no fabrication; flag inference as
   such.

Report back (<6 lines, no content dumps): gist fetch ok y/n, word count, output
path, one-line headline outcome.
