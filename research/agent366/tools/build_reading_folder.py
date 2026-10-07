"""Build the ~/icloud/2026/UVRR reading folder: the current draft with its
markdown copy, versions/, one reading card per bib entry with its abstract
in cards/, and the papers themselves, named by citation key, in papers/.
The September evidence package lives in evidence-2026-09/ and is never
touched here."""

import pathlib
import re
import shutil
import sys

TLAPLUS = pathlib.Path("/Users/Shared/lua-lunet/tlaplus")
sys.path.insert(0, str(TLAPLUS.parent / "knowledge-graphlite"))
from kglite.importers import parse_bib  # noqa: E402

DEST = pathlib.Path.home() / "icloud" / "2026" / "UVRR"
BIB = TLAPLUS / "research" / "literature" / "turner" / "paxos-reconf-latest.bib"
ZOTERO = pathlib.Path.home() / "Zotero" / "storage"
ICLOUD = pathlib.Path.home() / "icloud" / "2026"


def norm(s):
    return re.sub(r"[^a-z0-9]+", " ", s.lower()).strip()


def clean(v):
    v = re.sub(r"[{}]", "", v or "")
    return re.sub(r"\s+", " ", v).strip()


def title_words(title):
    stop = {
        "a", "an", "the", "of", "for", "and", "in", "on", "to", "with",
        "from", "by", "at", "as", "is", "are", "be", "that", "this",
    }
    return [w for w in norm(title).split() if len(w) > 2 and w not in stop][:6]


def main():
    (DEST / "papers").mkdir(parents=True, exist_ok=True)
    (DEST / "versions").mkdir(parents=True, exist_ok=True)

    for pattern in ("UVRR-2026.*",):
        for f in ICLOUD.glob(pattern):
            shutil.copy2(f, DEST / f.name)
    for f in ICLOUD.glob("20261002-*-draft.*"):
        shutil.copy2(f, DEST / "versions" / f.name)
    for f in ICLOUD.glob("UVRR-2026*.pdf"):
        if " " in f.name:
            shutil.copy2(f, DEST / "versions" / f.name)

    zotero_pdfs = {p: norm(p.stem) for p in ZOTERO.rglob("*.pdf")}
    repo_pdf_roots = [
        TLAPLUS / "research/agent366/papers",
        TLAPLUS / "research/literature/corpus/pdf",
        TLAPLUS / "research/literature/fast18",
        TLAPLUS / "research/literature/turner",
        TLAPLUS / "research/primary-sources/papers",
    ]
    repo_pdfs = {p: norm(p.stem) for root in repo_pdf_roots for p in root.glob("*.pdf")}
    A366 = TLAPLUS / "research/agent366/papers"
    aliases = {
        "zhang2024": ZOTERO / "GBQ94MB8/zhang-msfrd-2024.pdf",
        "duan2025": ZOTERO / "F793QKPV/duan-open-cas-2025.pdf",
        "norris2024": ZOTERO / "RDLM3HJ6/norris-openzfs-fsync-2024.pdf",
        "kuschewski2026": ZOTERO / "SWYJY8J3/kuschewski-btrlog-2026.pdf",
        "hu2026": ZOTERO / "R4PV8X26/hu-cloud-block-tail-2026.pdf",
        "turner": A366 / "paxos-reconf-latest.pdf",
        "vr": A366 / "vsr-1988-oki-liskov.pdf",
        "vrr": A366 / "vrr-2012-liskov-cowling.pdf",
        "shraer2012": A366 / "shraer2012-atc12-dynamic-reconfig.pdf",
        "diskless": A366 / "diskless-tr16.pdf",
        "ongaro2014": A366 / "raft-atc14-ongaro.pdf",
        "howard2016": A366 / "fqi-2016-howard.pdf",
        "zookeeper": A366 / "zookeeper-atc10-hunt.pdf",
        "allfs": A366 / "osdi14-pillai-allfs.pdf",
        "par": A366 / "fast18-alagappan-par.pdf",
        "optimistic-crash": A366 / "sosp13-optimistic-crash.pdf",
        "diskpaxos": A366 / "disk-paxos-2003.pdf",
        "curp": A366 / "curp-nsdi19-park.pdf",
        "moraru2013": A366 / "2013-sosp-moraru-epaxos.pdf",
        "reconfig-sm": A366 / "reconfiguring-a-state-machine-2010.pdf",
        "vertical-paxos": A366 / "vertical-paxos-2009.pdf",
        "corfu": A366 / "corfu-tocs2013.pdf",
        "lean4": A366 / "lean4-cade28.pdf",
        "lamport1998": A366 / "lamport1998-part-time-parliament.pdf",
        "lamport2004": A366 / "lamport2004-cheap-paxos.pdf",
        "lamport2008": A366 / "lamport2008-stoppable-paxos.pdf",
        "birman2010": A366 / "birman2010-virtually-synchronous.pdf",
        "lorch2006": A366 / "lorch2006-smart.pdf",
        "burrows2006": A366 / "burrows2006-chubby-osdi06.pdf",
        "chandra2007": A366 / "chandra2007-paxos-made-live.pdf",
        "nopaxos": A366 / "nopaxos-osdi16.pdf",
        "lampson1996": A366 / "lampson1996-highly-available-consensus.pdf",
        "fischer1985": A366 / "fischer1985-flp-impossibility.pdf",
        "bortnikov2012": A366 / "bortnikov2012-reconfigurable-smr.pdf",
        "jehl2014": A366 / "jehl2014-async-reconfiguration.pdf",
        "malkhi2005": A366 / "malkhi2005-omega-meets-paxos.pdf",
        "nipkow2002": A366 / "nipkow2002-isabelle-hol-book.pdf",
        "reed2008": A366 / "reed2008-zab-ladis08.pdf",
        "shapiro2011": A366 / "shapiro2011-crdt.pdf",
        "duan2025": A366 / "duan2025-open-cas.pdf",
        "kuschewski2026": A366 / "kuschewski2026-btrlog.pdf",
        "norris2024": A366 / "norris2024-openzfs-fsync.pdf",
        "zhang2024": A366 / "zhang2024-msfrd.pdf",
        "hoare1969": A366 / "hoare1969-axiomatic-basis.pdf",
        "ewd720": A366 / "ewd720-why-correctness.pdf",
        "ewd1215": A366 / "ewd1215-courtesy-birgit-schieder.pdf",
    }
    # extra corpus papers with no bib key of their own: the published
    # DISC 2017 version of the diskless UW technical report
    EXTRAS = [
        ("diskless-disc2017", A366 / "diskless-disc2017-recovering-shared-objects.pdf"),
    ]
    icloud_named = {f: norm(f.stem) for f in ICLOUD.glob("*.pdf")}

    entries = parse_bib(str(BIB))
    cards = 0
    pdfs = 0
    missing = []
    for e in entries:
        key = e["key"]
        words = title_words(e.get("title", ""))
        year = re.sub(r"\D", "", e.get("year", ""))[:4]
        if key in aliases and aliases[key].exists():
            hit = aliases[key]
        else:
            pool = {**repo_pdfs, **zotero_pdfs, **icloud_named}
            scored = []
            for p, stem in pool.items():
                overlap = sum(1 for w in words if w in stem)
                if overlap >= 3:
                    year_bonus = 1 if year and year in stem else 0
                    scored.append((overlap + year_bonus, overlap, year_bonus, p))
            scored.sort(reverse=True)
            hit = None
            if scored:
                best = scored[0]
                if len(scored) == 1 or best[0] - scored[1][0] >= 1:
                    hit = best[3]
                elif best[0] == scored[1][0] and best[2]:
                    hit = best[3]
            if hit is None and key in ("vr",):
                hit = TLAPLUS / "research/literature/turner/vsr-1988-oki-liskov.pdf"
        pdf_line = ""
        if hit:
            target = DEST / "papers" / f"{key}.pdf"
            shutil.copy2(hit, target)
            pdf_line = f"\n\n**PDF:** {target.name} (from {hit.parent.name})\n"
            pdfs += 1
        else:
            missing.append(key)
        lines = [
            f"# {clean(e.get('title', key))}",
            "",
            f"**Key:** {key}  "
            f"**Type:** {e.get('type', '')}  **Year:** {clean(e.get('year', ''))}",
        ]
        if e.get("author"):
            lines.append(f"**Authors:** {clean(e['author'])}")
        if e.get("link") or e.get("url"):
            lines.append(f"**Link:** {clean(e.get('link') or e.get('url'))}")
        if e.get("abstract"):
            lines += ["", "## Abstract", "", clean(e["abstract"])]
        lines.append(pdf_line.rstrip())
        (DEST / "papers" / f"{key}.md").write_text("\n".join(lines).rstrip() + "\n")
        cards += 1
    print(f"cards: {cards}; pdfs copied: {pdfs}; pdf not found: {len(missing)}")
    print("missing pdfs:", ", ".join(missing))

    for name, src in EXTRAS:
        if src.exists():
            shutil.copy2(src, DEST / "papers" / f"{name}.pdf")
            print(f"extra: {name}.pdf")
        else:
            print(f"extra MISSING: {src}")

    # reading surfaces for the papers with no text layer (scans) or no
    # geometry render (poppler bug): collocated as <key>-text.md so every
    # key shows a complete {card, pdf, text} group when sorted by name
    A366MD = TLAPLUS / "research/agent366/md"
    for key, stem in (("hoare1969", "hoare1969-axiomatic-basis"),
                      ("ewd720", "ewd720-why-correctness"),
                      ("ewd1215", "ewd1215-courtesy-birgit-schieder"),
                      ("lampson1996", "lampson1996-highly-available-consensus")):
        src = A366MD / f"{stem}.md"
        if src.exists():
            shutil.copy2(src, DEST / "papers" / f"{key}-text.md")
        else:
            print(f"text surface MISSING: {src}")


if __name__ == "__main__":
    main()
