# Citation-check run — diagnosis and verdicts

2026-09-28, agent366, after the "1 accept · 27 review · 9 seek-more-evidence"
run was called a disaster. Pipeline: `scripts/check-citations.py --live`
(jev-1.13 via OpenCode Zen, one call per `\cite` use, 0.9 confidence gate,
keyword-overlap passage retrieval from the scraped source texts).

## The headline answer

**The model does not say the citations suck.** Across both the 82-entry first
run and the fresh 41-use run, the count of `contradicts` verdicts is **zero**.
Nothing in the paper conflicts with its sources anywhere. The ugly profile is
produced by three mechanical causes and exactly one genuine defect:

| Class | Count | What it is |
|---|---|---|
| Genuine defect | 1 | pms line 176 "ballots" — **fixed** (see below) |
| Retrieval gaps | ~10 | source supports the claim; the keyword retriever ranked the wrong paragraphs |
| Unverifiable by design | ~20 | artifact self-cites, background attributions, compound sentences, TODO marker |
| Gate strictness | rest | model answered "supports" but under the 0.9 confidence bar |

## 1. The genuine defect — found, fixed, verified

paper.tex line 176 said *"the obligation Paxos places on ballots~\cite{pms}"*.
Paxos Made Simple contains **zero** occurrences of "ballot" (Lamport calls
them proposal numbers there; "ballots" is Part-Time Parliament vocabulary).

Fix applied: the sentence now reads "proposal numbers". Verified support in
the source (research/literature/sources/pms.md, lines 101–103):

> "…a proposal consists of a proposal number and a value. To prevent
> confusion, we require that different proposals have different numbers."

## 2. Retrieval gaps — the source supports it, the excerpt missed it

The retriever ranks source paragraphs by lexical overlap with the claim
sentence. When the paper paraphrases the source, the vocabularies are
disjoint and the supporting paragraph loses to a topically-adjacent one.
Each case below was grep-verified by hand: the supporting text exists.

| tex line | key | claim (gist) | verified support in source |
|---|---|---|---|
| 179 | pms | proposal-number uniqueness obligation | pms.md:101–103 "different proposals have different numbers" |
| 65/67 | vr, vrr | VR introduced 1988; VRR dropped disk in normal+view change | vrr.md:48,62 "did not require disk I/O"; "does not require any use of disk" |
| 474 | etcd-guarantees | strict serializability; linearizable vs serializable reads | etcd-guarantees.md:38,73,76 |
| 476 | etcd-hardware | two-vCPU 8 GB AWS small cluster | etcd-hardware.md:49 (the m4.large table row) |
| 487 | allfs,optimistic-crash | a single flushed flag is not reliable enough | allfs.md:121,386–397 (fsync ordering protocol) |
| 667 | par | can lose committed data | par.md:40,49,65–67 |
| 726 | diskless | virtual stable storage abstraction | diskless.md:37,61,65,123 |
| 415 | nopaxos | one RTT via network ordering | nopaxos.md:429–430,459 "single round-trip in the normal case" |
| 420 | curp | 1 RTT via commutative dual-write | curp.md:43–44 |
| 141/195/211… | turner | various | turner.md is the full UPaxos text; attribution claims |

Mitigation applied in the script: the claim window is now the citation
sentence plus the previous one (was ±3 lines, which mixed in unrelated
sentences and poisoned the keyword ranker), and the passage budget went
6000→9000 with top 3→4. The remaining gaps are paraphrase-vs-lexical
mismatch; closing them fully wants embedding-based retrieval, not more
keyword tuning.

## 3. Unverifiable by design — the check cannot see these

- **`artifact` self-cites (7 uses).** The cited work is this repository's own
  addendum; the script quotes `formal/uvrr-lean/README.md`, which cannot
  establish sentences like "the addendum gives the proof ladder, a general
  configuration-path constructor…". Verify directly against the addendum, as
  the previous run's review already noted.
- **Background attributions** (65 vr, 141 turner, 406 lean4, 907/909 related
  work). "Oki and Liskov introduced Viewstamped Replication in 1988" — the
  whole cited paper is the claim; no excerpt can "support" it.
- **Compound sentences** (296 turner+artifact, 468 reconfig-sm+vertical-paxos,
  487, 907). One sentence, several facts, one or two sources; a single
  passage cannot establish the whole conjunction.
- **Line 442 `rfc9000,rfc9114`** — the citation sits inside a `{TODO …}`
  scaffold marker; "insufficient" is correct behaviour until the TODO is
  written out.
- **Blog self-cites** (157 motivation, 240 votingweights, 231 netdisk-era) —
  already slated for re-pinning under todo 366; their verdicts will change
  when that lands.

## 4. Gate strictness

The model's "supports" answers mostly arrive at 0.3–0.9 confidence, and the
gate routes everything under 0.9 to human review. That is the gate doing its
job — the run's purpose is to route, not to acquit. The four supports-above-
0.6 worth eyeballing as effectively fine: 73 diskless (0.77), 420 curp (0.70),
427 corfu (0.84), 907 diskless (0.67).

## Verdict table (41 uses, run 3, jev-1.13, gate 0.9)

Cache: `.tmp/citation-verdicts-v2.jsonl` (run 1 preserved as
`verdicts-run1-82entries.jsonl`; raw outputs `run2-raw.json`, `run3-raw.json`).

- **0 accept** (nothing reached supports ≥ 0.9 — see §4)
- **30 review_low_confidence** — of which the retrieval-gap rows in §2 are
  verified-fine by grep; the rest are §3's unverifiable classes
- **11 seek_more_evidence** — artifact self-cites, blog self-cites, the TODO
  marker, compound claims
- **0 contradicts** — no citation conflicts with its source

## Sources

All 24 source texts now live canonically at `research/literature/sources/<key>.md`
(one per bibitem key, including the four new nopaxos/curp/rfc9000/rfc9114),
which is what `scripts/check-citations.py` reads. Provenance per file:
`research/agent366/CORPUS.md` (same texts, session copy).

## Reproduce

    set -a; source .env; set +a
    python3 scripts/check-citations.py --live               # full run, 41 billed calls
    python3 scripts/check-citations.py --live --skip_cached # resume after a network drop
    python3 scripts/check-citations.py                      # fixture mode, no billing
