# agent366 — harvested reference corpus

Every external work cited from `docs/*` or `paper/*`, pulled down on 2026-09-28,
plus Raft (user instruction: "i have not mentioned RAFT … yet go get it").
PDFs in `papers/`; extracted text in `md/`. Extraction: `pdftotext` default
mode (reading order; two-column layouts come out de-columnised) for
born-digital PDFs, `pdftoppm` + `tesseract` (300 dpi, eng) for scans.
`paper.pdf` has both (`paper.md` = pdftotext, `paper.ocr.md` = tesseract).

## Papers (peer-reviewed / tech reports)

| Cited as | File (md/) | Source |
|---|---|---|
| [VR-2012] / `vrr` | vr-revisited-2012.md | dspace.mit.edu bitstream (pmg.csail.mit.edu was down) |
| VSR-1988 / `vr` | vr-1988-oki-liskov.md | princeton.edu course mirror (OCR, scan) |
| Oki dissertation | oki-dissertation-tr423.md | publications.csail.mit.edu (OCR, scan) |
| [PMS-2001] / `pms`, `fuse-paxos` | paxos-made-simple-2001.md | lamport.azurewebsites.net |
| `diskpaxos` | disk-paxos-2003.md | lamport.azurewebsites.net |
| `vertical-paxos` (PODC'09) | vertical-paxos-2009.md | lamport.azurewebsites.net |
| [SMAUG-2017] (not a Vertical Paxos paper; fetched under that name by mistake) | smaug-2017-gorke-armknecht.md | arxiv 1708.04871 |
| `shraer2012` (ATC'12) / [ATC12-SHRAER] | shraer2012-atc12-dynamic-reconfig.md | usenix.org atc12-final74 |
| [DISC-2017] (Michael et al., the published `diskless`) | diskless-disc2017-recovering-shared-objects.md | drops.dagstuhl.de LIPIcs.DISC.2017.36 |
| `reconfig-sm` (SIGACT News 2010) | reconfiguring-a-state-machine-2010.md | INRIA mirror |
| [OSDI-2014] / `allfs` | osdi14-pillai-allfs.md | usenix.org |
| [SOSP-2013] / `optimistic-crash` | sosp13-optimistic-crash.md | research.cs.wisc.edu |
| [LAMPSON-1979] | lampson-sturgis-1979.md | microsoft.com/research (OCR, scan) |
| `lamport1998` (TOCS '98) | lamport1998-part-time-parliament.md | lamport.azurewebsites.net (title page verified) |
| `lamport2004` (DSN '04) | lamport2004-cheap-paxos.md | lamport.azurewebsites.net (title page verified) |
| `lamport2008` (MSR 2008) | lamport2008-stoppable-paxos.md | lamport.azurewebsites.net (title page verified) |
| `birman2010` (MSR-TR-2010-151) | birman2010-virtually-synchronous.md | microsoft.com/research (title page verified) |
| `lorch2006` (EuroSys '06) | lorch2006-smart.md | microsoft.com/research (title page verified) |
| `burrows2006` (OSDI '06) | burrows2006-chubby-osdi06.md | usenix.org legacy proceedings (title page verified) |
| `par` (FAST'18) | fast18-alagappan-par.md | usenix.org |
| `diskless` (UW TR 16-08-02) | diskless-tr16.md | syslab.cs.washington.edu |
| `corfu` (TOCS 2013) | corfu-tocs2013.md | malkhi.com (exact cited version) |
| `zookeeper` (ATC'10) | zookeeper-atc10-hunt.md | usenix.org |
| `lean4` (CADE 28) | lean4-cade28.md | lean-lang.org |
| FQI (Howard et al. 2016) | fqi-2016-howard.md | arxiv 1608.06696 |
| **Raft (ATC'14)** | raft-atc14-ongaro.md | usenix.org — added per user instruction, not cited anywhere in the tree |
| [TURNER-RECONF] / `turner` | papers/turner-paxos-reconf.tex (LaTeX source) | github.com/DaveCTurner/paxos-membership, branch raft-like-reconfiguration |
| `chandra2007` (PODC '07) | chandra2007-paxos-made-live.md | research.google.com archive (authors' 16-pp. version) |
| `nopaxos` (OSDI '16) | nopaxos-osdi16.md | usenix.org |
| `lampson1996` (WDAG '96) | lampson1996-highly-available-consensus.md | author's site |
| `fischer1985` (JACM 32(2)) | fischer1985-flp-impossibility.md | course-mirror copy, JACM header verified |
| `bortnikov2012` (PODC '12 brief announcement) | bortnikov2012-reconfigurable-smr.md | arXiv:1512.08943 full version; the 2-page BA has no free copy — the bib keys to the BA |
| `jehl2014` (ICDCN '14) | jehl2014-async-reconfiguration.md | author's UiS copy via Wayback |
| `malkhi2005` (DISC '05) | malkhi2005-omega-meets-paxos.md | DISC LNCS 3724 pp. 199–213 header verified |
| `nipkow2002` (LNCS 2283) | nipkow2002-isabelle-hol-book.md | isabelle.in.tum.de distribution copy (the maintained edition the book page sanctions) |
| `reed2008` (LADIS '08, the Zab paper) | reed2008-zab-ladis08.md | Yahoo Labs archive via Wayback |
| `shapiro2011` (SSS '11) | shapiro2011-crdt.md | HAL inria-00609399 (INRIA RR-7687 open version) |
| `duan2025` (ATC '25) | duan2025-open-cas.md | usenix.org open access |
| `kuschewski2026` (PVLDB 19(10)) | kuschewski2026-btrlog.md | VLDB open access / arXiv |
| `norris2024` (AsiaBSDCon '24) | norris2024-openzfs-fsync.md | papers.freebsd.org |
| `zhang2024` (ATC '24) | zhang2024-msfrd.md | usenix.org open access |
| `hu2026` (NSDI '26) | hu2026-cloud-block-tail.md | usenix.org open access |

## Manuscripts and standards

| Cited as | File (md/) | Source |
|---|---|---|
| `hoare1969` (CACM 12(10)) | hoare1969-axiomatic-basis.md | CMU course-mirror scan; OCR (tesseract), title page verified |
| `ewd720` | ewd720-why-correctness.md | UT EWD archive transcription (pandoc); the scan PDF sits in papers/ |
| `ewd1215` | ewd1215-courtesy-birgit-schieder.md | UT EWD archive transcription (pandoc); the scan PDF sits in papers/ |
| `rfc9000` | rfc9000-quic.md | rfc-editor.org canonical text |
| `rfc9114` | rfc9114-http3.md | rfc-editor.org canonical text |

## Blogs (the self-citations under discussion)

| Cited as | File (md/) |
|---|---|
| [VW-2017] / `votingweights` | blog-vw-2017-paxos-voting-weights.md |
| [UPAXOS-2016] | blog-upaxos-2016.md |
| [FROWN-2020] | blog-frown-2020.md |
| [NETDISK-2024] / `netdisk` | blog-netdisk-2024.md |
| [UVRR-2026] / `motivation` | blog-uvrr-2026.md |

## Web documentation / other cited material

| Cited as | File (md/) |
|---|---|
| `etcd-guarantees` | web-etcd-api-guarantees.md |
| `etcd-hardware` | web-etcd-hardware.md |
| `etcd-embed` | web-etcd-embed.md |
| `tigerbeetle` (docs.tigerbeetle.com/about/safety) | web-tigerbeetle-safety-concepts.md — **note: /about/safety now 404s; content lives at /concepts/safety** |
| TigerBeetle ARCHITECTURE.md | web-tigerbeetle-architecture.md |
| TigerBeetle DESIGN.md (history archive) | web-tigerbeetle-design-history.md |
| Jepsen TigerBeetle 0.16.11 | web-jepsen-tigerbeetle.md |
| MySQL InnoDB disk I/O | web-mysql-innodb-disk-io.md |
| MySQL doublewrite buffer | web-mysql-doublewrite-buffer.md |
| Percona torn pages | web-percona-torn-pages.md |
| transactional.blog torn writes | web-transactional-torn-writes.md |
| HN thread (Joran Greef on PAR) | web-hn-tigerbeetle-par.md |
| Allen Ling gist (unbounded-reconfig progress) | web-gist-allenling-unbounded-progress.md |
| "Director's gist" (TigerBeetle disk analysis) | web-gist-simbo1905-tigerbeetle-director.md |

## Own paper (for reference)

paper.md, paper.ocr.md, paper_ladder.md, paper_exceeding.md, paper_results.md

## Not fetched (deliberate)

- github.com/tigerbeetle/tigerbeetle and github.com/lua-lunet/* — code repos, not documents.
- https://tigerbeetle.com — landing page, no citable content.
- DOI resolver links (dl.acm.org, doi.org) — paywalled duplicates of works already harvested from open mirrors.

## Line-numbered extractions

`linear/` holds one `.txt` per paper with the paper's text in single-column
reading order and a right-aligned line number on every body line, so any
passage is quotable verbatim with a locator: a right-column run is one
contiguous stretch of ascending numbers, which the interleaved `-layout`
extraction cannot give. Each file carries a YAML header with the Zotero bib
metadata, the source PDF path and sha256, the page count and the tool
version, plus `=== PAGE n ===` page markers.

Generated by `tools/linear_corpus.py` through `tools/linearlatex.py`
(poppler's `pdftohtml -xml` geometry, interior-gutter column detection),
in the order the paper cites each work, then the rest of the corpus
alphabetically. Reruns are byte-identical: the extraction date is a
parameter, not the clock. `linear/MANIFEST.md` records every result; the
four papers not rendered there (the two EWD manuscripts and Hoare 1969 are
scans without a text layer, Lampson 1996 makes poppler emit malformed XML)
name their OCR'd or transcribed markdown under `md/` as the reading surface.

The `.txt` files are a locator and a quoting surface, not a reading copy:
quote the `.txt` for the line number and the paper for authority.
