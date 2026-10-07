"""Render every cited corpus paper as a line-numbered, single-column .txt.

Each output is a locator and quoting surface: contiguous reading order (a
right-column passage is one run of ascending line numbers, which
`pdftotext -layout` cannot give), `=== PAGE n ===` markers, and a YAML
header carrying the Zotero bib metadata and the source PDF's sha256.

Papers are rendered in the order the paper cites them — first-citation order
taken from the golden fragments in render order — then the remainder of the
corpus alphabetically, so the sequence is reproducible.

    python3 research/agent366/tools/linear_corpus.py [--date YYYY-MM-DD]

Outputs land beside their PDFs in ../papers/ so one name-sorted folder
holds pdf, locator text and reading surfaces together. Scans with no text
layer are recorded as skipped, not faked: the OCR'd markdown under ../md/
is their reading surface.
"""

import argparse
import datetime
import hashlib
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3]
BIB = ROOT / "paper/papers/references.bib"
TOOL = ROOT / "research/agent366/tools/pdftolines.py"
OUT = ROOT / "research/agent366/papers"
ORDER = ROOT / "paper/paper.md.j2"
FRAGMENTS = ROOT / "papers/fragments"

A366 = "research/agent366/papers"
TURNER = "research/literature/turner"
CORPUS = "research/literature/corpus"

# citation key -> repo-relative source PDF
CORPUS_PDFS = {
    "turner": f"{A366}/paxos-reconf-latest.pdf",
    "vr": f"{A366}/vsr-1988-oki-liskov.pdf",
    "vrr": f"{A366}/vrr-2012-liskov-cowling.pdf",
    "moraru2013": f"{A366}/2013-sosp-moraru-epaxos.pdf",
    "diskless": f"{A366}/diskless-tr16.pdf",
    "diskless-disc2017": f"{A366}/diskless-disc2017-recovering-shared-objects.pdf",
    "shraer2012": f"{A366}/shraer2012-atc12-dynamic-reconfig.pdf",
    "ongaro2014": f"{A366}/raft-atc14-ongaro.pdf",
    "howard2016": f"{A366}/fqi-2016-howard.pdf",
    "zookeeper": f"{A366}/zookeeper-atc10-hunt.pdf",
    "allfs": f"{A366}/osdi14-pillai-allfs.pdf",
    "par": f"{A366}/fast18-alagappan-par.pdf",
    "optimistic-crash": f"{A366}/sosp13-optimistic-crash.pdf",
    "diskpaxos": f"{A366}/disk-paxos-2003.pdf",
    "curp": f"{A366}/curp-nsdi19-park.pdf",
    "reconfig-sm": f"{A366}/reconfiguring-a-state-machine-2010.pdf",
    "vertical-paxos": f"{A366}/vertical-paxos-2009.pdf",
    "corfu": f"{A366}/corfu-tocs2013.pdf",
    "lean4": f"{A366}/lean4-cade28.pdf",
    "bortnikov2012": f"{A366}/bortnikov2012-reconfigurable-smr.pdf",
    "pms": f"{A366}/paxos-made-simple-2001.pdf",
    "lamport1998": f"{A366}/lamport1998-part-time-parliament.pdf",
    "lamport2004": f"{A366}/lamport2004-cheap-paxos.pdf",
    "lamport2008": f"{A366}/lamport2008-stoppable-paxos.pdf",
    "birman2010": f"{A366}/birman2010-virtually-synchronous.pdf",
    "lorch2006": f"{A366}/lorch2006-smart.pdf",
    "burrows2006": f"{A366}/burrows2006-chubby-osdi06.pdf",
    "chandra2007": f"{A366}/chandra2007-paxos-made-live.pdf",
    "nopaxos": f"{A366}/nopaxos-osdi16.pdf",
    "lampson1996": f"{A366}/lampson1996-highly-available-consensus.pdf",
    "fischer1985": f"{A366}/fischer1985-flp-impossibility.pdf",
    "jehl2014": f"{A366}/jehl2014-async-reconfiguration.pdf",
    "malkhi2005": f"{A366}/malkhi2005-omega-meets-paxos.pdf",
    "nipkow2002": f"{A366}/nipkow2002-isabelle-hol-book.pdf",
    "reed2008": f"{A366}/reed2008-zab-ladis08.pdf",
    "shapiro2011": f"{A366}/shapiro2011-crdt.pdf",
    "hoare1969": f"{A366}/hoare1969-axiomatic-basis.pdf",
    "ewd720": f"{A366}/ewd720-why-correctness.pdf",
    "ewd1215": f"{A366}/ewd1215-courtesy-birgit-schieder.pdf",
    "zhang2024": f"{A366}/zhang2024-msfrd.pdf",
    "duan2025": f"{A366}/duan2025-open-cas.pdf",
    "norris2024": f"{A366}/norris2024-openzfs-fsync.pdf",
    "kuschewski2026": f"{A366}/kuschewski2026-btrlog.pdf",
    "hu2026": f"{A366}/hu2026-cloud-block-tail.pdf",
}

# corpus papers with no bib entry of their own
EXTRA_META = {
    "diskless-disc2017": {
        "title": "Recovering Shared Objects Without Stable Storage",
        "authors": ["Ellis Michael", "Dan R. K. Ports", "Naveen Kr. Sharma", "Adriana Szekeres"],
        "year": "2017",
        "publication": "32nd International Symposium on Distributed Computing (DISC 2017), LIPIcs 46, article 36",
        "url": "https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.DISC.2017.36",
        "keywords": "consensus, crash recovery, diskless",
    },
}


def _clean(value: str) -> str:
    """Better BibTeX braces and line wrapping off."""
    value = value.replace("{{", "").replace("}}", "")
    value = value.replace("\\", "")
    return re.sub(r"\s+", " ", value).strip().rstrip(",").strip()


def _field(block: str, name: str) -> str:
    """One bib field, brace-aware: values may wrap across lines."""
    m = re.search(rf"(?ms)^\s*{name}\s*=\s*([{{\"].)", block)
    if not m:
        return ""
    start = m.end() - 1
    if block[start] == '"':
        end = block.find('"', start + 1)
        return _clean(block[start + 1:end if end > 0 else len(block)])
    depth, i = 0, start
    while i < len(block):
        if block[i] == "{":
            depth += 1
        elif block[i] == "}":
            depth -= 1
            if depth == 0:
                break
        i += 1
    return _clean(block[start + 1:i])


def bib_metadata(path: pathlib.Path) -> dict:
    """Citation key -> header fields, from the Zotero export.

    The trusted parser (kglite.importers, written for Better BibTeX output)
    supplies title, authors, year, url and abstract; venue and keywords are
    read from the raw block, where they are short single-line fields.
    """
    sys.path.insert(0, str(ROOT.parent / "knowledge-graphlite"))
    from kglite.importers import parse_bib  # noqa: PLC0415

    text = path.read_text()
    blocks = {}
    for b in re.split(r"(?m)^@", text)[1:]:
        m = re.match(r"\w+\{([^,]+),", b)
        if m:
            blocks[m.group(1).strip()] = b

    out = {}
    for entry in parse_bib(str(path)):
        key = entry["key"]
        block = blocks.get(key, "")
        venue = ""
        for name in ("booktitle", "journal", "series"):
            m = re.search(rf"(?m)^\s*{name}\s*=\s*\{{(.+?)\}}\s*,?\s*$", block)
            if m:
                venue = _clean(m.group(1))
                break
        keywords = ""
        m = re.search(r"(?m)^\s*keywords\s*=\s*\{([^}]*)\}", block)
        if m:
            keywords = _clean(m.group(1))
        authors = []
        for a in (entry.get("author") or "").split(" and "):
            a = a.strip().rstrip(".")
            if not a:
                continue
            if "," in a:  # "Last, First" -> "First Last"
                last, first = a.split(",", 1)
                a = f"{first.strip()} {last.strip()}"
            authors.append(a)
        out[key] = {
            "title": _clean(entry.get("title", "")),
            "authors": authors,
            "year": re.sub(r"\D", "", entry.get("year", ""))[:4],
            "publication": venue,
            "url": entry.get("url", ""),
            "abstract": _clean(entry.get("abstract", "")),
            "keywords": keywords,
        }
    return out


def citation_order() -> list:
    """First-citation order of the paper, taken from the golden fragments."""
    order = [m for m in re.findall(r"{{\s*p\('([^']+)'\)\s*}}", ORDER.read_text())]
    seen, cited = set(), []
    for name in order:
        frag = FRAGMENTS / f"{name}.md"
        if not frag.exists():
            continue
        for m in re.finditer(r"\\cite(?:\[[^\]]*\])?\{([^}]+)\}", frag.read_text()):
            for key in (k.strip() for k in m.group(1).split(",")):
                if key not in seen:
                    seen.add(key)
                    cited.append(key)
    return cited


MIN_LINES = 20  # fewer numbered lines than this is a scan, not a paper


def render(key: str, meta: dict, date: str) -> tuple:
    pdf_rel = CORPUS_PDFS[key]
    pdf = ROOT / pdf_rel
    out = OUT / f"{key}.txt"
    if not pdf.exists():
        return "missing-pdf", f"no source: {pdf_rel}"
    if not meta or not meta.get("title"):
        return "no-metadata", "no bib entry and no manual metadata"
    cmd = [
        sys.executable, str(TOOL), str(pdf),
        "-o", str(out),
        "--title", meta["title"],
    ]
    for a in meta["authors"]:
        cmd += ["--authors", a]
    for flag, value in (("--year", meta.get("year", "")),
                        ("--publication", meta.get("publication", "")),
                        ("--url", meta.get("url", "")),
                        ("--abstract", meta.get("abstract", "")),
                        ("--pdf-label", pdf_rel),
                        ("--extract-date", date)):
        if value:
            cmd += [flag, value]
    if meta["keywords"]:
        for kw in (k.strip() for k in meta["keywords"].split(",")):
            if kw:
                cmd += ["--tags", kw]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0:
        return "failed", (r.stderr or r.stdout).strip().splitlines()[-1] if (r.stderr or r.stdout).strip() else f"exit {r.returncode}"
    lines = sum(1 for line in out.read_text().splitlines() if re.match(r"\s*\d+\s", line))
    if lines < MIN_LINES:
        # a scan with no text layer: the stub is worse than nothing, so it goes
        out.unlink()
        md = ROOT / "research/agent366/md" / f"{key}.md"
        surface = f"../md/{key}.md" if md.exists() else "no md; the PDF itself"
        return "scan-no-text", f"only {lines} text runs; reading surface is {surface}"
    sha = hashlib.sha256(out.read_bytes()).hexdigest()[:16]
    return "ok", f"{lines} numbered lines, {out.stat().st_size} bytes, sha {sha}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--date", default=datetime.date.today().isoformat())
    args = ap.parse_args()
    OUT.mkdir(parents=True, exist_ok=True)

    meta = bib_metadata(BIB)
    meta.update(EXTRA_META)
    cited = citation_order()
    rest = sorted(set(CORPUS_PDFS) - set(cited))
    todo = [k for k in cited if k in CORPUS_PDFS] + [k for k in rest if k not in cited]

    print(f"corpus {len(CORPUS_PDFS)} papers; {len(cited)} cited by the paper; rendering {len(todo)}")
    results = {}
    for n, key in enumerate(todo, 1):
        status, detail = render(key, meta.get(key, {}), args.date)
        results[key] = {"status": status, "detail": detail}
        print(f"[{n:2}/{len(todo)}] {key:18} {status:12} {detail}")

    ok = [k for k, v in results.items() if v["status"] == "ok"]
    bad = {k: v for k, v in results.items() if v["status"] != "ok"}
    manifest = [
        "# Line-numbered corpus extractions",
        "",
        f"Generated by `tools/linear_corpus.py` on {args.date} with "
        "`tools/pdftolines.py` (pdftolines 1.0.0, stdlib + poppler's pdftohtml).",
        "Rendered in first-citation order, then the remainder of the corpus",
        "alphabetically. Each `.txt` is a locator and quoting surface: contiguous",
        "reading order with ascending line numbers, `=== PAGE n ===` markers, and",
        "a header carrying the Zotero bib metadata and the source PDF's sha256.",
        "It is not a substitute for the paper; quote the `.txt` for the locator and",
        "the paper for authority.",
        "",
        "| # | key | status | detail |",
        "|---|---|---|---|",
    ]
    for n, key in enumerate(todo, 1):
        r = results[key]
        manifest.append(f"| {n} | `{key}` | {r['status']} | {r['detail']} |")
    if bad:
        manifest += [
            "",
            "## Not rendered",
            "",
            "Scans with no text layer cannot be read by a geometry extractor; their",
            "reading surface is the OCR'd markdown under `../md/`, produced by",
            "`../md/extract.sh` (pdftoppm + tesseract at 300 dpi).",
            "",
        ] + [f"- `{k}`: {v['detail']}" for k, v in bad.items()]
    (OUT / "00-linear-manifest.md").write_text("\n".join(manifest) + "\n")
    print(f"\nrendered {len(ok)}/{len(todo)}; manifest at {OUT/'00-linear-manifest.md'}")
    if bad:
        print("not rendered: " + ", ".join(bad))


if __name__ == "__main__":
    main()
