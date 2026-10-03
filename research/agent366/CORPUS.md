# agent366 — harvested reference corpus

Every external work cited from `docs/*` or `paper/*`, pulled down on 2026-09-28,
plus Raft (user instruction: "i have not mentioned RAFT … yet go get it").
PDFs in `papers/`; extracted text in `md/`. Extraction: `pdftotext -layout`
for born-digital PDFs, `pdftoppm` + `tesseract` (300 dpi, eng) for scans.
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
| `par` (FAST'18) | fast18-alagappan-par.md | usenix.org |
| `diskless` (UW TR 16-08-02) | diskless-tr16.md | syslab.cs.washington.edu |
| `corfu` (TOCS 2013) | corfu-tocs2013.md | malkhi.com (exact cited version) |
| `zookeeper` (ATC'10) | zookeeper-atc10-hunt.md | usenix.org |
| `lean4` (CADE 28) | lean4-cade28.md | lean-lang.org |
| FQI (Howard et al. 2016) | fqi-2016-howard.md | arxiv 1608.06696 |
| **Raft (ATC'14)** | raft-atc14-ongaro.md | usenix.org — added per user instruction, not cited anywhere in the tree |
| [TURNER-RECONF] / `turner` | papers/turner-paxos-reconf.tex (LaTeX source) | github.com/DaveCTurner/paxos-membership, branch raft-like-reconfiguration |

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
