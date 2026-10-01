# agent366 — append-only working log

Session identifier: sidecar item 366 (task_sidecar DB).
Lane: `research/` writable; `paper/` read-only until the user says go; `docs/` untouched.

Append-only: entries are added at the bottom, never edited in place. Corrections
are new entries that supersede earlier ones by reference.

---

## 2026-09-28 — entry 1: session opened

Brief: the paper must not cite the author's own blogs; where `docs/*.md` back
statements with blog entries, the paper should carry similar statements backed
by real published work (or by the paper's own math). Docs keep their blog
entries; that is the project lane's own record.

Audit result (unchanged tree, commit f884ac7):

- Exactly three blog bibitems in the paper tree:
  - `motivation` (blog UVRR-2026) — cited once in paper.tex:169 and once in
    paper_ladder.tex:142, both at the "agreement over small amounts of
    metadata embedded in a distributed application" sentence.
  - `netdisk` (blog NETDISK-2024) — bibitem in paper.tex:888; cited once in
    paper_exceeding.tex:160 ("what makes the round affordable at all").
  - `votingweights` (blog VW-2017) — bibitem in paper.tex:891; cited once in
    paper.tex:249 ("the voting-weights rules").
- [UPAXOS-2016] and [FROWN-2020] (docs/references.md) never appear as `\cite`
  in the paper; their substance enters the paper only as unattributed math.
- docs/ keeps all five blog entries (references.md + the "Grounding sources"
  block in uvrr-protocols.md). No doc edits planned.

## 2026-09-28 — entry 2: correction — Flexible Paxos / FQI dropped

User steer: Turner is known to the user inside out; Turner's work does not
draw on Flexible Paxos (Howard et al.) and the FQI paper is not in his
lineage. Citing it would fabricate a provenance that does not exist — the
same sin as citing the blog, just dressed differently.

Revised re-pin plan:

- `votingweights` sentence (paper.tex:249): the halve/double, ±1-unit and
  standby-at-zero rules are the paper's own weighted schedule, grounded in
  Turner's weighted-majority overlap lemma (`\cite{turner}`, already cited
  15× in paper.tex). The sentence should read as the paper's own construction
  following Turner's overlap argument — no new bib entry needed.
- `motivation` sentences (paper.tex:169, paper_ladder.tex:142): re-pin to
  the metadata-service literature already in the bib (`zookeeper`, `corfu`,
  the etcd entries) and/or `diskless` for the crash-recovery framing.
- `netdisk` sentence (paper_exceeding.tex:160): re-pin to the
  crash-consistency cost literature the paper already discusses (the
  [OSDI-2014]/[SOSP-2013] line) or restate as the paper's own measurement
  claim; the blog URL leaves the bibliography.
- UPAXOS-2016 / FROWN-2020 substance stays attributed, where attributed at
  all, to `turner`, `reconfig-sm`, `vertical-paxos` — all already in the bib.

Then delete the three blog bibitems so no simbo1905.wordpress.com URL remains
in any of the four bibliographies.

Status: plan revised; .tex edits not yet made; awaiting user go-ahead.

## 2026-09-28 — entry 3: vocabulary correction (user)

User: "there is no such 'dropping Flexible Paxos/FQI' i do not know
`dropping` means". Clarified: nothing was deleted from any file. "Dropping"
meant retracting my own suggestion (entry 2) to cite Flexible Paxos/FQI. The
retraction stands — Turner lineage only — but no tree content changed then or
now. Interesting note surfaced during the harvest: TigerBeetle's own safety
documentation *does* cite Flexible Quorums (arxiv 1608.06696) — so FQI is in
the corpus as a TigerBeetle dependency, not as a Turner antecedent.

## 2026-09-28 — entry 4: reference harvest complete

User instruction: pull every reference cited in docs/* or paper/*, plus Raft
("i sort of ... hate that paper. yet go get it"), and extract the text.

Done. research/agent366/papers/ holds 19 PDFs + Turner's paxos-reconf.tex;
research/agent366/md/ holds 42 extracted markdown files. CORPUS.md is the
manifest mapping each citation key (docs `[XX-YYYY]` and paper `\bibitem`) to
its local file and source URL.

Method: pdftotext -layout for born-digital PDFs; pdftoppm 300dpi + tesseract
5.5.3 for the three scans (Lampson-Sturgis 1979, VR-1988, Oki dissertation).
paper.pdf got both routes (paper.md, paper.ocr.md) since the user asked for
tesseract on it specifically.

Notable findings during harvest:
- pmg.csail.mit.edu (the canonical VR-2012 host used in docs/references.md)
  refused connection; VR-2012 was fetched from the dspace.mit.edu bitstream
  URL that docs/uvrr-durability-model.md also lists. The docs' primary URL
  for [VR-2012] may be rotting.
- docs.tigerbeetle.com/about/safety/ (cited in paper_exceeding.tex as
  `tigerbeetle`) now returns HTTP 404; the content lives at
  /concepts/safety. A live citation in the paper points at a dead URL.
- CORFU: the paper cites the TOCS 2013 version (DOI 10.1145/2535930);
  harvested the exact TOCS PDF from malkhi.com rather than the NSDI'12
  conference version.
- The netdisk blog post itself cites two latency sources (a Yugabyte dev.to
  fsync post and a cross-AZ ping measurement post) — i.e. the blog's own
  evidence base is two web posts, not a paper. If the paper wants the claim
  "the network is faster than the disk" it should measure it (paper_results
  lane) or cite the OSDI/SOSP crash-consistency cost literature, not relay
  the blog's sources second-hand.

Status: corpus complete; citation-alignment edits (todo 366) still pending
user go-ahead.

## 2026-09-28 — entry 5: docs URL sweep + VR-2012 fix (user item 2)

User: "if the docs locations are rotting then fix them ffs". Swept every URL
in docs/*.md with curl HEAD. Result: exactly one dead link —
https://pmg.csail.mit.edu/papers/vr-revisited.pdf (connection refused; an
earlier session's rollout hit the same refusal, so it is genuinely down, not
transient). ACM DOI links return 403 to curl but that is bot-blocking, not
rot; the HN link 405s HEAD but serves GET.

Fixed docs/references.md [VR-2012] to <https://hdl.handle.net/1721.1/71763>
(canonical MIT handle, resolves 200 to dspace.mit.edu — the same handle the
paper's `vrr` bibitem already uses). No other docs/ URL touched.

Note: the paper_exceeding.tex `tigerbeetle` bibitem (about/safety → 404)
lives in the paper lane; that is user item 1's subject, handled separately.

## 2026-09-28 — entry 6: Ousterhout thread (user items 1, 3)

**Item 1 — TigerBeetle paper?** None exists (searched: no peer-reviewed
technical paper by TigerBeetle/Joran Greef). Citable artifacts: PaPoC '21
lightning-talk slides "Viewstamped Replication Made Famous"
(papoc-workshop.github.io/2021/slides/greef.pdf), the QCon talk, the
docs (concepts/safety), and the Jepsen analysis as the independent
third-party evaluation. Grey-lit practice for such systems is what the
paper already does for etcd: software/documentation citation with access
date. Recommendation: repoint the `tigerbeetle` bibitem URL to
/concepts/safety and consider adding the Jepsen analysis for the
durability claims. User's call (paper lane).

**Item 2 — docs URL fix:** done (entry 5).

**Item 3 — Ousterhout/Homa thread:**
- Fact-check: Ousterhout IS Raft's co-author (Ongaro & Ousterhout, ATC'14
  Best Paper) and was Ongaro's primary PhD advisor (Stanford purl qr033xr6097).
  LWN: "of Tcl/Tk and Raft fame". The user's "stop bashing Raft" instinct is
  the right call if the goal is his readership.
- Talk harvested: yt-dlp 403'd with the stale system yt-dlp (2026.07.04);
  fixed by installing current yt-dlp (2026.08.19) in an isolated venv under
  .tmp/ — no system changes. Audio via yt-dlp+ffmpeg (homa-talk.wav),
  STT via mlx-whisper with whisper-large-v3 in .tmp/stt-venv.
  Outputs: ousterhout/homa-talk-transcript.txt + homa-talk-segments.json.
- Key transcript fact: he retired from Stanford to spend "100% of my time
  hacking on HOMA" and closes the talk by explicitly soliciting collaborators
  ("I'd be delighted to work with you... my email is on the slide").
- Hardware reality-check (user asked): Homa needs NO special hardware — only
  the priority queues already present in modern commodity switches; the
  kernel module runs on stock Linux.
- Bibliography with verbatim abstracts: ousterhout/bibliography.md —
  Raft ATC'14, Ongaro dissertation, LogCabin, RAMCloud (2009 + 2015),
  CURP NSDI'19, EPaxos Revisited NSDI'21, Homa SIGCOMM'18, Homa/Linux ATC'21,
  Replace-TCP arXiv 2210.00714, the 2026 talk. Full texts NOT fetched (user:
  abstracts first).
- Two-thread convergence worth flagging to the user: CURP (Ousterhout's
  NSDI'19) is the closest Ousterhout-line work to uVRR's territory — strong
  consistency with durability decoupled from ordering. And RAMCloud's
  13.5µs durable writes with 1-2s whole-cluster recovery is the
  durability-by-replication precedent for the uVRR superblock framing.
- ECharts graph: research/agent366/graph/ (index.html + data.js), vanilla
  JS, ECharts 5.5 CDN, force layout, 9 invented categories (Paxos/VR
  classics, reconfiguration, Raft, RAMCloud, Homa, crash-consistency,
  metadata services, uVRR, self-blog grey). 32 nodes, 46 edges — every edge
  verified against local corpus texts or the works' own abstracts; none
  speculative. Click → abstract panel verified in headless Chrome.
  Serve: any static server (python3 -m http.server works); nginx optional.
  NB: ECharts loads from jsdelivr CDN, so the page needs internet.

**Turner nuance found during edge verification:** Turner's paxos-reconf.tex
cites Flexible Paxos — in his own words, "need not all mutually intersect, as
discovered independently by Howard, Malkhi and Spiegelman". So the user is
right that Turner's result is not derived FROM Flexible Paxos (independent
discovery), but Turner does cite it. If paper.tex ever needs the quorum
relaxation, "independently discovered by Howard et al." is the accurate and
Turner-compatible phrasing. User's call.

## 2026-09-28 — entry 7: per-paper review queued (user)

User: assign an agent per harvested paper; ask each "is this paper
solving/repeating/contrasting/contracting the paper in this repo"; extract
ONLY useful quotes that reinforce, contrast, confirm the main doc's
statements; no spurious associations. Two todos added (plan + execute) and
parked after the current work per user instruction.

## 2026-09-28 — entry 8: CURP pulled; graph tagged

- CURP (Park & Ousterhout, NSDI'19) pulled in full:
  papers/curp-nsdi19-park.pdf, extracted to md/curp-nsdi19-park.md, opened
  in the user's browser. User's Ousterhout reading begins here.
- Graph upgraded per user: every node now carries topic tags and a
  "See also" list (cited/substitute/adjacent works), clickable cross-links
  between panels. Verified in Chrome: CURP panel shows abstract, 4 tags,
  and see-also RAMCloud/Raft/VRR/uVRR.

## 2026-09-28 — entry 9: paper scaffold edits (user go-ahead, build mode)

User pasted the CURP/NOPaxos analysis chat (verdict: hardware-sequencer
designs are undeployable in stock cloud; CURP's 1-RTT is a client dual-write
valid only under commutativity) and directed: qualify the paper politely as
aiming at commodity cloud infrastructure. Executed:

1. paper.tex title: `DRAFT --- ` prefix (footer already carried DRAFT).
2. §I Introduction: added the objective sentence — "strong consistency
   deployable on commodity cloud infrastructure: stock virtual machines,
   stock load balancing, and no custom networking hardware."
3. §VII Discussion: new subsection "Deployment without custom ordering
   hardware" BEFORE "Applications". States NOPaxos's requirement
   (programmable switching/SDN; \cite{nopaxos}), CURP's dual-write +
   commutativity qualification (\cite{curp}), CORFU as the software-sequencer
   counterpoint (\cite{corfu} — verified from the TOCS text: user-space
   sequencer 200K→500K tokens/s; the FPGA in that paper is network-attached
   flash, not the sequencer), our any-node accounting (1 RTT at leader,
   1 RTT + one-way forwarding leg at a follower, no leader-aware routing
   needed), plus three {TODO} markers: cloud measurement of the forwarding
   leg, QUIC/HTTP/3 connectionless framing (\cite{rfc9000,rfc9114}), and the
   S3-offload/agent-logging pattern.
4. §VIII lunet-locks Components: {TODO} for the upcall-at-any-node latency
   check and the availability↔latency knob. User raises the lunet-locks
   ticket themselves.
5. Bibliography: +nopaxos (OSDI'16), +curp (NSDI'19), +rfc9000, +rfc9114.
6. Spell gate: registered the new technical names in british-words.txt.
   Build: ./build.sh --draft → published papers/20260928-5a66467-draft.pdf,
   no undefined references.
7. Records: CORFU NSDI'12 fetched (papers/corfu-nsdi12.pdf + md), 2016
   "Just say NO to custom hardware for Paxos" blog archived
   (md/blog-nopaxos-2016.md). The blog is NOT cited in the paper; the
   argument went in as prose citing the literature.

PAXE naming: user confirmed it is real Rust in-repo (uvrr-core sans-IO core
+ C ABI); per plan the paper stays neutral ("the demonstration's
connectionless datagram transport"), name held for later.

## 2026-09-28 — entry 10: the jev citation-check "disaster" diagnosed

User: the jev-1.13 citation run (1 accept / 27 review / 9 seek-more) is "a
disaster"; get the citation data into a new location, verify the scraped
docs, re-run the experiment, and find out "why this model says your
citations suck ass".

Answer, with the run to back it: the model says no such thing. Zero
contradicts across all runs. The profile decomposes into:

1. ONE genuine defect — paper.tex:176 cited PMS for "ballots"; PMS has zero
   occurrences (Lamport renamed them proposal numbers). Fixed to "proposal
   numbers"; support verified at pms.md:101-103. Paper rebuilt clean.
2. Retrieval gaps — keyword-overlap retrieval ranks the wrong paragraphs
   when the paper paraphrases the source; every flagged case grep-verified
   as supported (pms, vrr, etcd×3, allfs, par, diskless, nopaxos, curp).
   Script mitigations: claim is now the citation sentence + previous (was a
   ±3-line window), passage budget 6000→9000, top 3→4.
3. Unverifiable-by-design — artifact self-cites (a README can't prove the
   addendum's theorems), background attributions, compound sentences, the
   rfc9000/rfc9114 TODO marker, blog self-cites (already todo 366).
4. Gate strictness — supports answers arriving at 0.3–0.9 confidence all
   route to review by design.

Actions: sources consolidated to research/literature/sources/<key>.md (24
files, incl. newly-fetched nopaxos + RFC 9000/9114); scripts/check-citations.py
SOURCES updated; fresh live run over all 41 uses (wifi dropped mid-run;
resumed with --skip_cached); full diagnosis in
research/literature/citation-check/REPORT.md.

## 2026-09-28 — entry 11: Chimera harvested, graph extended

User spotted Chimera (arXiv 2606.09101, Liu et al., 2026) — protocol-aware
recovery for confidential BFT consensus. Harvested:
papers/chimera-2026-liu.pdf + md/chimera-2026-liu.md.

Relevance to uVRR: Chimera's first systematic taxonomy of rollback-resilient
recovery for confidential BFT names Diskless Crash Recovery as one of its
four categories — the crash-recovery territory uVRR repairs. It model-checks
with Maude/LTL. It cites the diskless TR [44] and VRR [45] directly, Raft
[11], ZooKeeper [21]; it does NOT cite PAR (notable).

Graph updated: +1 category (TEE confidential consensus), +12 nodes (Chimera
with verbatim abstract; Paxos Made Live; LSKV, SecureKeeper, CCF, Engraft,
ROTE, Nimble, Achilles, DAMYSUS, Narrator, JPaxos with one-line
characterisations sourced from Chimera's own related-work text where no
verbatim abstract was fetched), +15 edges ([n] = Chimera's reference
numbers). Now 46 nodes / 60 edges, verified no dangling links; Chimera
panel checked in Chrome.
