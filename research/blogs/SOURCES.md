# SOURCES — item16 (UPaxos/VRR source material)

Fetched 2026-09-05. Output root: `.tmp/blogs/`.

## 1. Unbounded Viewstamped Replication Revisited?
- URL: https://simbo1905.wordpress.com/2026/08/12/viewstamped-replication-revisited/
- Saved: `.tmp/blogs/viewstamped-replication-revisited.md`
- Contains: motivation piece (Aug 2026) arguing nobody has proven nonstop cluster
  reconfiguration for VRR-2012 (Liskov & Cowling); contrasts VRR-2012 vs VSR-1988
  vs Paxos Made Simple; notes VRR's disk-flush-free crash safety costs an extra
  network round trip and majority-contact rejoin; TigerBeetle as the only
  production VRR adopter, without dynamic membership. Positioning for the uVRR
  proof effort. No diagrams (verified: zero `<img>` in page content).
- Comments: none.

## 2. One More Frown Please? (UPaxos Quorum Overlaps)
- URL: https://simbo1905.wordpress.com/2020/05/23/one-more-frown-please-upaxos-quorum-overlaps/
- Saved: `.tmp/blogs/one-more-frown-please-upaxos-quorum-overlaps.md`
- Contains: the UPaxos safety equation QIIe ⌢ QIe ⌢ QIIe+1 ⌢ QIe+1 and the
  "frown operator" definition; the three required quorum overlaps (intra-era
  before and after reconfiguration, plus cross-era QIe ⌢ QIIe+1); correction by
  Dave Turner that QIe ⌢ QIe+1 (prepare-to-prepare overlap across eras) is NOT
  required for safety; two-node counterexample quorum system; Trex low-ball
  prepare / nack optimisation; grid-quorum remark in comments.
- Diagrams: NONE. The quorum expressions that looked like images in the raw
  extract are inline HTML math text; a live-DOM pass (chrome-devtools) confirmed
  `.entry-content` contains zero `<img>` elements, so no screenshots or OCR were
  needed. Tavily-extract gaps were filled from the live DOM.

## 3. Paxos Voting Weights
- URL: https://simbo1905.wordpress.com/2017/03/16/paxos-voting-weights/
- Saved: `.tmp/blogs/paxos-voting-weights.md`
- Contains: integer voting weights in Paxos quorums; weight-0 acceptors as
  learners; the 3-AZ server hot-swap scenario with weight tables (naive 1s
  table showing the 4-node/2-AZ vulnerability, and the weighted 2s/1s sequence
  preserving majority under loss of any one zone); why consecutive weight
  transitions (scale-all, or ±1 on one node) keep majorities overlapping —
  the {3,4,5,6,7,...} majority-overlap argument; comments with paper author
  David Turner (Isabelle/HOL safety proofs, informal liveness, abdication
  pipeline-stall wrinkle) and the Trex UPaxos sketch implementation plan.
- Diagrams: NONE (tables are HTML text; only emoji-SVG images on page).

## 4. UPaxos paper PDF
- NOT FOUND on disk. Globbed `**/*.pdf` across the repo and `.tmp/papers/`:
  `.tmp/papers/` holds VRR-2012 (`liskov-cowling-vr-revisited-2012.pdf`, already
  OCR'd at `.tmp/papers/ocr/liskov-cowling-vr-revisited-2012.md`) and other
  unrelated papers; `tools/tla2tools/` holds TLA toolbox docs only. No UPaxos
  ("paxos-reconf", 902f8b7) PDF exists, so no UPaxos OCR was produced.
  Web location if needed later: http://tessanddave.com/paxos-reconf-902f8b7.pdf
  (linked from post 3).

## Images / OCR
- `.tmp/blogs/img/` is empty: all three pages are text-only; there are no
  diagram images to capture or OCR. `.tmp/ocr_run.py` was not invoked (no
  inputs); the API key was never read.
