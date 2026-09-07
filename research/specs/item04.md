# item04 — Build the primary-sources bundle (in .tmp)

**Guidance update (2026-09-05): all work lives under `.tmp/` (gitignored). Git steps are NOOPS — skip them entirely. The earlier `git init`/`git add` inside the bundle was inapplicable and has been undone (`.git` removed).**

Work dir: repo root `/Users/Shared/lua-lunet/vrr-core`.

Create the directory `.tmp/lean4-vrr-temporal-verification-primary-sources/` with:

1. `papers/` — copy of `.tmp/papers/*.pdf` (7 PDFs)
2. `ocr-text/` — copy of `.tmp/papers/ocr/*.md` (7 markdown files, same basenames with .md)
3. `README.md` — exactly the manifest below (you may fix trivial typos, keep all facts/URLs):

```markdown
# Lean 4 / Viewstamped Replication — temporal verification primary sources

Working bundle for fact-checking a literature review on Lean 4 automated theorem
proving for VR cluster-reconfiguration verification. Assembled 2026-09-05.
Contains source PDFs plus OCR text (Mistral `mistral-ocr-latest`; math rendered
as LaTeX in the markdown). No credentials in this bundle.

| Paper | Authors | Venue | File | Source |
|---|---|---|---|---|
| LeanLTL: A unifying framework for linear temporal logics in Lean | Vin, Miller, Fremont | ITP 2025 (arXiv:2507.01780) | `papers/leanltl-vin-miller-fremont-2025.pdf` | https://arxiv.org/pdf/2507.01780 |
| Veil: A Framework for Automated and Interactive Verification of Transition Systems | Pirlea, Gladshtein, Kinsbruner, Zhao, Sergey | CAV 2025, LNCS 15933, pp. 26-41 | `papers/veil-pirlea-et-al-cav2025.pdf` | https://verse-lab.org/papers/veil-cav25.pdf |
| Lessons from Building an Auto-Active Verifier in Lean | Pirlea, Gladshtein, Zhao, Sergey | Dafny'26 workshop | `papers/veil-lessons-auto-active-dafny2026.pdf` | https://verse-lab.org/papers/veil-dafny26.pdf |
| Lean-auto: An Interface between Lean 4 and Automated Theorem Provers | Qian, Clune, Barrett, Avigad | CAV 2025, LNCS 15933, pp. 175-196 (arXiv:2505.14929) | `papers/lean-auto-qian-et-al-2025.pdf` | https://arxiv.org/pdf/2505.14929 |
| Viewstamped Replication Revisited | Liskov, Cowling | MIT-CSAIL-TR-2012-021, July 2012 | `papers/liskov-cowling-vr-revisited-2012.pdf` | https://dspace.mit.edu/bitstream/handle/1721.1/71763/MIT-CSAIL-TR-2012-021.pdf |
| Past Matters: Supporting LTL+Past in the BLACK Satisfiability Checker | Geatti, Gigante, Montanari, Venturato | TIME 2021, LIPIcs vol. 206, 8:1-8:17 | `papers/black-ltl-past-geatti-et-al-2021.pdf` | https://drops.dagstuhl.de/storage/00lipics/lipics-vol206-time2021/LIPIcs.TIME.2021.8/LIPIcs.TIME.2021.8.pdf |
| Formal Verification of Multi-Paxos for Distributed Consensus | Chand, Liu, Stoller | FM 2016, LNCS 9995, pp. 119-136 (arXiv:1606.01387) | `papers/multi-paxos-chand-liu-stoller-fm2016.pdf` | https://arxiv.org/pdf/1606.01387 |

OCR text lives in `ocr-text/` with matching basenames.

Not included (paywalled, no open copy): Cimatti, Roveri, Sheridan, "Bounded
Verification of Past LTL", FMCAD 2004, DOI 10.1007/978-3-540-30494-4_18.
```

4. Safety gate BEFORE git init: from repo root run
   `KEY=$(awk -F= '/^MISTRAL_API_KEY/{print $2}' .env | tr -d '"' | tr -d "'"); grep -rlF -- "$KEY" .tmp/lean4-vrr-temporal-verification-primary-sources || echo CLEAN`
   and also `find .tmp/lean4-vrr-temporal-verification-primary-sources -name ".env*"`.
   Never echo the key. If any hit or `.env` file is found, STOP and report FAIL.

5. **SKIPPED AS NOOP (2026-09-05 guidance):** no `git init`, no `git add`, no git commands at all — the bundle lives under `.tmp/` which is gitignored.

Report back (terse, <10 lines, no file dumps): file count, `du -sh` of bundle,
and audit verdict CLEAN/FAIL.
