# paper/research — terminology evidence corpus

Primary sources for the wording of the paper's opening claim about disk
flushes and small replicated metadata. The corpus evidences the register the
Raft/Paxos/VR literature uses to scope such claims:

- **on / off the critical path** — placement of a disk write in a request's
  latency chain (Cyclone, Fleet, XLL);
- **normal case / normal operation** — the protocol regime in which a claim
  holds, opposed to view change, recovery, or reconfiguration
  (Fast Paxos, VRR, QuePaxa);
- **synchronous disk writes / flushed to disk later** — whether the flush
  blocks the operation before its acknowledgement (Google SRE book, Fleet,
  XLL, Raft's stable storage);
- **fast path / slow path** — reserved in this literature for commit in one
  message round versus fallback rounds (Fast Paxos, EPaxos, QuePaxos), not
  for disk placement.

Every quote recorded in the metadata was verified against the retrieved
artifact identified by its SHA-256; none is quoted from a secondary source.

## Corpus

| ID | Paper | Venue | Year | PDF | Mistral OCR | Tesseract OCR |
|---|---|---|---|---|---|---|
| `2005-msrtr-lamport-fast-paxos` | Fast Paxos — Leslie Lamport | Microsoft Research TR MSR-TR-2005-112 | 2005 | [pdf](pdf/2005-msrtr-lamport-fast-paxos.pdf) | [md](ocr-mistral/2005-msrtr-lamport-fast-paxos.md) | [txt](ocr-tesseract/2005-msrtr-lamport-fast-paxos.txt) |
| `2012-csailtr-liskov-vr-revisited` | Viewstamped Replication Revisited — Barbara Liskov, James Cowling | MIT CSAIL TR MIT-CSAIL-TR-2012-021 | 2012 | [pdf](pdf/2012-csailtr-liskov-vr-revisited.pdf) | [md](ocr-mistral/2012-csailtr-liskov-vr-revisited.md) | [txt](ocr-tesseract/2012-csailtr-liskov-vr-revisited.txt) |
| `2013-sosp-moraru-epaxos` | There Is More Consensus in Egalitarian Parliaments — Iulian Moraru, David G. Andersen, Michael Kaminsky | ACM SOSP '13 | 2013 | [pdf](pdf/2013-sosp-moraru-epaxos.pdf) | [md](ocr-mistral/2013-sosp-moraru-epaxos.md) | [txt](ocr-tesseract/2013-sosp-moraru-epaxos.txt) |
| `2014-osdi-ongaro-raft` | In Search of an Understandable Consensus Algorithm (Extended Version) — Diego Ongaro, John Ousterhout | USENIX OSDI '14 | 2014 | [pdf](pdf/2014-osdi-ongaro-raft.pdf) | [md](ocr-mistral/2014-osdi-ongaro-raft.md) | [txt](ocr-tesseract/2014-osdi-ongaro-raft.txt) |
| `2016-srebook-managing-critical-state` | Managing Critical State: Distributed Consensus for Reliability — Laura Nolan | Google, Site Reliability Engineering, ch. 22 | 2016 | [pdf](pdf/2016-srebook-managing-critical-state.pdf) | [md](ocr-mistral/2016-srebook-managing-critical-state.md) | [txt](ocr-tesseract/2016-srebook-managing-critical-state.txt) |
| `2017-arxiv-roy-cyclone` | Cyclone: High Availability for Persistent Key Value Stores — Amitabha Roy, Subramanya R. Dulloor | arXiv:1711.06964v1 | 2017 | [pdf](pdf/2017-arxiv-roy-cyclone.pdf) | [md](ocr-mistral/2017-arxiv-roy-cyclone.md) | [txt](ocr-tesseract/2017-arxiv-roy-cyclone.txt) |
| `2019-nsdi-park-curp` | Exploiting Commutativity For Practical Fast Replication — Seo Jin Park, John Ousterhout | USENIX NSDI '19 | 2019 | [pdf](pdf/2019-nsdi-park-curp.pdf) | [md](ocr-mistral/2019-nsdi-park-curp.md) | [txt](ocr-tesseract/2019-nsdi-park-curp.txt) |
| `2024-osdi-tennage-quepaxa` | QuePaxa: Escaping the Tyranny of Timeouts in Consensus — Pasindu Tennage et al. | USENIX OSDI '24 | 2024 | [pdf](pdf/2024-osdi-tennage-quepaxa.pdf) | [md](ocr-mistral/2024-osdi-tennage-quepaxa.md) | [txt](ocr-tesseract/2024-osdi-tennage-quepaxa.txt) |
| `2025-pvldb-fan-fleet` | FLEET: High-Performance Durable Replicated State Machines using Scattered and Coordinated Log Entries — Hua Fan, Hao Tan, Wenchao Zhou, Feifei Li | PVLDB 18(5) | 2025 | [pdf](pdf/2025-pvldb-fan-fleet.pdf) | [md](ocr-mistral/2025-pvldb-fan-fleet.md) | [txt](ocr-tesseract/2025-pvldb-fan-fleet.txt) |
| `2026-nsdi-shawger-xll` | XLL: Cross-Layer Logging for Data Deduplication in Consensus-Based Storage — John Shawger, Arnav Jhingran, Andrea Arpaci-Dusseau, Remzi Arpaci-Dusseau | USENIX NSDI '26 | 2026 | [pdf](pdf/2026-nsdi-shawger-xll.pdf) | [md](ocr-mistral/2026-nsdi-shawger-xll.md) | [txt](ocr-tesseract/2026-nsdi-shawger-xll.txt) |

## Layout

```
research/
├── README.md            this file
├── papers.jdt.json      JSON Table schema for meta/*.json
├── ocr-mistral.py       Mistral OCR driver (mistral-ocr-latest)
├── ocr-tesseract.sh     Tesseract OCR driver (5.5.3, 200 dpi, eng)
├── pdf/                 retrieved artifacts, systematically named
├── meta/                one <id>.json per paper: abstract, details, evidence, OCR stats
├── ocr-mistral/         one <id>.md per paper, pages joined by "---"
└── ocr-tesseract/       one <id>.txt per paper, pages joined by "---"
```

## Naming convention

`{year}-{venue}-{firstauthor}-{slug}` — e.g. `2012-csailtr-liskov-vr-revisited`.
The stem is the ID shared by the artifact, its metadata, and both OCR outputs.

## Metadata and schema

`meta/<id>.json` carries the bibliographic record, the artifact's SHA-256 and
page count, the abstract transcribed from the artifact, the verified
terminology quotes with their locations, and the statistics of both OCR
passes. `papers.jdt.json` is the JSON Table schema for those files.

## OCR provenance

- **Mistral** (`ocr-mistral.py`): `mistral-ocr-latest` — the same model the
  paper's own build gate uses (`paper/build.sh --ocr`). The key is read
  silently from the repository `.env` (gitignored) and is never printed,
  logged, or written. PDFs are sent as base64 data-URIs to
  `api.mistral.ai/v1/ocr`, with a multipart upload fallback.
- **Tesseract** (`ocr-tesseract.sh`): tesseract 5.5.3 with the `eng` model,
  rasterised from the PDFs at 200 dpi grayscale with poppler `pdftoppm`.
  Pages are joined with `---` separators.

## Retrieval notes

- `2012-csailtr-liskov-vr-revisited` was retrieved from the KAUST-hosted
  mirror of the MIT CSAIL technical report; `dspace.mit.edu` declined the
  direct request (HTTP 405).
- `2016-srebook-managing-critical-state` is rendered from the official HTML
  chapter (body text only). The published full-book PDF no longer resolves;
  the `static.googleusercontent.com` target returns 404.
- All other artifacts are the files served by their publishers or authors.
