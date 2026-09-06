# item08 — Position paper: proving VRR with Lean 4 and other techniques

Inputs: `.tmp/papers/ocr/*.md` (7 papers). No network; no git; write ONLY
`.tmp/research/position-paper.md`.

Task: a position paper (1500–2500 words) on using Lean 4 and other techniques to
prove Viewstamped Replication Revisited (VRR) — especially safety during cluster
reconfiguration.

Required structure:
1. Thesis — stated up front, defensible from the corpus.
2. Evidence — per technique, grounded in short quotes/paraphrases + source file:
   Veil (CAV'25 + Dafny'26 lessons), LeanLTL (ITP'25), lean-auto (CAV'25), the
   TLA+ ecosystem (as characterized in the Veil papers + Multi-Paxos FM'16),
   LTL+Past tooling (BLACK, TIME'21), and the VR Revisited protocol itself
   (reconfiguration mechanics: view change, state transfer, configuration state).
3. Comparison — Lean vs TLA+ as the Veil authors actually frame it. Quote the
   "out of frustration" passage accurately; do NOT use the distorted paraphrase
   "the former provides no automation, while the latter lacks a logical foundation"
   — the real text says TLA+ is great for modelling/TLC and that symbolic checking
   (Apalache) and TLAPS exist but sit outside the default path.
4. Gaps — what the corpus does NOT support: no published full VR verification in
   Lean; LeanLTL lists past-time operators as future work, not present; liveness
   in Veil is future work; Cimatti et al. FMCAD'04 is paywalled and not in corpus.
5. Recommendation & open questions.

Every factual claim traceable to a corpus file; speculation flagged as such; no
fabricated citations.

Report back (<6 lines, no content dumps): word count, output path, one-line thesis.
