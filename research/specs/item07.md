# item07 — Glossary across the seven primary sources

Inputs: `.tmp/papers/ocr/*.md` (7 OCR'd papers, math as LaTeX). Read them all.

Output: `.tmp/research/glossary.md` ONLY. No other writes; no git commands; no network.

Content: a glossary of concepts/terminology found across the corpus, grouped by
theme (suggested: Lean 4 & ITP machinery; temporal logic; SMT/ATP automation;
distributed protocols & VR/VRR; verification tools compared). Each entry:
term — 1–3 sentence definition grounded in the corpus, plus the source file(s)
it appears in (e.g. `veil-pirlea-et-al-cav2025.md`). Include acronyms and key
named constructs (e.g. grind, Mathlib, monomorphization, λ∗→ abstraction, QMono,
LTLf, LTL+Past, tableau, Veil, Ivy, TLA+/TLC/Apalache/TLAPS, Z3, cvc5, VR view
change, state transfer, reconfiguration, Multi-Paxos, safety/liveness, inductive
invariant, EPR). Aim for 40–70 entries. Definitions must match how the papers
actually use the terms; no outside knowledge injected as fact.

Report back (<6 lines, no content dumps): entry count, theme count, output path.
