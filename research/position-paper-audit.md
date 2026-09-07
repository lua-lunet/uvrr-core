# Audit of `.tmp/research/position-paper.md`

Scope: internal consistency against `.tmp/research/glossary.md`, the corpus
`.tmp/papers/ocr/*.md`, and the five brief-claims (a)–(e). No network used.
Verdicts: PASS/FAIL per brief-claim, plus glossary-consistency findings.
All pointers are `file:line` in `.tmp/`.

## 1. Brief-claim verdicts

### (a) Distorted Veil quote — PASS

The paper does not repeat the brief's distortion ("the former provides no
automation, while the latter lacks a logical foundation"). It quotes the
accurate passage verbatim — "We developed it out of frustration with existing
tools…" — at `research/position-paper.md:60`, matching the corpus at
`papers/ocr/veil-lessons-auto-active-dafny2026.md:64` word-for-word, and at
`position-paper.md:62` it *explicitly* states the framing is "not a claim that
one tool 'provides no automation' while the other 'lacks a logical
foundation.'" The distorted string appears nowhere in the paper.

### (b) LeanLTL past operators — PASS

The paper states past-time (and bounded-time) operators are *future work*
twice: `position-paper.md:41` ("its authors list as *future work* 'support for
other LTL variants, including past-time and bounded-time operators' (§5)") and
`position-paper.md:67` (Gap 2). Corpus confirmation:
`papers/ocr/leanltl-vin-miller-fremont-2025.md:255` — the future-work sentence
in §5, quoted accurately. Nowhere does the paper claim LeanLTL currently has
past operators; `position-paper.md:43` correctly says no corpus tool connects
past-time machinery to Lean today.

### (c) VR Revisited contains no automated-verification results — PASS

Grep for "automated verification"/"logless" over
`papers/ocr/liskov-cowling-vr-revisited-2012.md`: zero matches — the paper
contains no machine-verification claims. The position paper never attributes
the brief's "logless replicated state machine … automated verification of its
safety properties" quote to Liskov & Cowling; the string "logless" appears
nowhere in `position-paper.md`. The paper correctly characterizes VRR's §8 as
informal: `position-paper.md:54` ("informal prose — precise, but not
machine-checked") and `position-paper.md:70` (Gap 5), matching the corpus's own
words at `liskov-cowling-vr-revisited-2012.md:439` ("we provide an informal
discussion of the correctness of the protocol").

### (d) No published full VR verification in Lean — PASS

Stated as Gap 1 at `position-paper.md:66`, with the correct supporting detail:
Veil's 16 case studies include Paxos variants, Rabia, SCP, Suzuki-Kasami — the
string "Viewstamped" does not occur in
`papers/ocr/veil-pirlea-et-al-cav2025.md` at all. Corpus:
`veil-pirlea-et-al-cav2025.md:327–331` (benchmark list, all verified, Ivy
failing two). Nothing in the corpus contradicts the claim, and the paper
correctly separates the feasibility speculation (flagged as such at
`position-paper.md:72`).

### (e) Nonstandard "LTA+" terminology — PASS

"LTA+" appears nowhere in `position-paper.md` (or anywhere under
`.tmp/research/`). The paper uses the standard term "LTL+Past"
(`position-paper.md:43`), consistent with the glossary definition
(`research/glossary.md:59`) and the BLACK corpus
(`papers/ocr/black-ltl-past-geatti-et-al-2021.md:25,59`).

**All five brief-claims: PASS. The paper repeats none of the brief's errors.**

## 2. Glossary-consistency findings

### 2.1 Terms used consistently (spot-checked against corpus)

- **Veil / `#check_invariants` (+ `?`/`!`) / CTI / BMC (`sat trace`, `unsat
  trace`)** — usage at `position-paper.md:15–18,76` matches glossary
  `glossary.md:139,95,97` and corpus `veil-pirlea-et-al-cav2025.md:188,196`.
- **Lean-auto vs Lean-SMT roles** — `position-paper.md:33` ("Lean-auto as its
  default SMT translator and Lean-SMT as the alternative") matches corpus
  `veil-pirlea-et-al-cav2025.md:296` ("Veil uses Auto by default") and glossary
  `glossary.md:39`.
- **Lean-auto description** — `position-paper.md:33` ("general-purpose,
  ATP-based proof automation in Lean 4 for the first time", translation into
  monomorphic HOL) matches `lean-auto-qian-et-al-2025.md:15,37,118`.
- **3–5x proof-reconstruction penalty** — `position-paper.md:27` matches
  `veil-lessons-auto-active-dafny2026.md:78`.
- **10x slower than Ivy; largest case study 160 s** — `position-paper.md:28`
  matches `veil-lessons-auto-active-dafny2026.md:224` (Veil 160 s vs Ivy 50 s).
- **750 obligations / ~3 minutes; TLAPS undocumented bug** —
  `position-paper.md:37` matches
  `multi-paxos-chand-liu-stoller-fm2016.md:33,37`.
- **Quorum intersection, 2f+1/f+1, view-change correctness condition, "in
  view 0", "two primaries", f'+1 EPOCHSTARTED, "at least as recent"** —
  `position-paper.md:49–52` all match
  `liskov-cowling-vr-revisited-2012.md:83,392,439,465,491,503` and glossary
  §4.
- **Ivy timeouts on Paxos/Vertical Paxos** — `position-paper.md:16` matches
  `veil-pirlea-et-al-cav2025.md:355`.
- **TLAPS default backends outside the default path** — consistent with
  glossary `glossary.md:151` and corpus.

No term is used in a way that contradicts its glossary definition. No
contradiction found anywhere between the paper and the glossary.

### 2.2 Minor citation nuance (not a failure)

`position-paper.md:15` says Veil "verified all 16 distributed-protocol case
studies automatically via `#check_invariants` … including two Paxos variants
from Padon et al." Corpus `veil-pirlea-et-al-cav2025.md:327` confirms all were
verified automatically without interactive proof, with the caveat that Rabia
required a raised timeout (120 s vs default 5 s) — the paper's "automatically"
is accurate but silently elides the Rabia timeout exception. Cosmetic only.

### 2.3 Notable corpus terms missing from the paper

Ranked by relevance to the paper's own argument:

1. **IronFleet** (`glossary.md:161`; corpus
   `multi-paxos-chand-liu-stoller-fm2016.md`) — verified SMR with Multi-Paxos
   at its core in Dafny, safety *and* liveness, >30,000 proof LOC. Directly
   relevant to the paper's effort-estimation discussion (§5), which cites only
   TLAPS+Multi-Paxos as the reference point. Omission weakens the
   effort-estimation basis; recommend adding.
2. **nuXmv** (`glossary.md:75`; corpus `black-ltl-past-geatti-et-al-2021.md:69`)
   — per BLACK, "the only widely available tool that directly supports past
   operators." Relevant to §2.5's past-time tooling comparison; the paper
   names BLACK but omits that the corpus's only other past-capable tool is a
   maintained model checker. Recommend mentioning.
3. **State transfer with checkpoints / Merkle trees** (`glossary.md:117`) —
   `position-paper.md:52` covers reconfiguration state transfer but omits the
   checkpoint/Merkle machinery, which materially affects the size of the
   replica state a Veil model must encode. Minor.
4. **VR optimizations: witnesses, batching, leases, fast reads**
   (`glossary.md:121`) — omitted; defensible given the safety-only scope, but
   witness replicas (no service state) would change the state model if
   modeled. Minor.
5. **TLPVS, Duper, Alloy, preemption** — peripheral to the paper's thesis;
   omission acceptable.

## 3. Verdict

The position paper is internally consistent with the glossary and the corpus,
and repeats none of the fact-check brief's five errors: **(a) PASS, (b) PASS,
(c) PASS, (d) PASS, (e) PASS.** The only improvements worth making are the
omissions in §2.3 items 1–2 (IronFleet as an effort reference point; nuXmv as
the corpus's other past-operator tool).
