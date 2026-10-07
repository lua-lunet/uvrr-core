"""Load the paragraph fragments and the bibliography into the in-repo
knowledge graph (a derived index; the golden source stays in git).

The graph home is .kg/ (gitignored): .kg/paper.gql.db, .kg/paper.gql.obj,
.kg/.trash. Each agent works in its own schema namespace, so pass your
session id: --session, or set KG_SESSION. Writers do not tread on each
other.

Usage:
  python3 paper/kg_load.py --session <your-session-id> [--bib FILE]
                           [--structure DIR]
"""

import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
KGLITE = ROOT.parent / "knowledge-graphlite"
sys.path.insert(0, str(KGLITE))

from kglite.importers import import_bib  # noqa: E402
from kglite.kg import KG  # noqa: E402

HOME = ROOT / ".kg"
NAME = "paper"


def classify(text):
    if text.lstrip().startswith("[figure block"):
        return "Figure"
    if text.lstrip().startswith("[table block"):
        return "Table"
    return "Paragraph"


def parse_fragment(path):
    raw = path.read_text()
    m = re.match(
        r'<p x-name="([^"]+)"(?: x-tex="([^"]*)")?(?: x-labels="([^"]*)")?>\n?\n?(.*?)\n?\n?</p>',
        raw.strip(),
        re.S,
    )
    if not m:
        raise SystemExit(f"unparsable fragment: {path}")
    return m.group(1), m.group(2) or "", m.group(3) or "", m.group(4).strip()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--session", default=None)
    ap.add_argument("--bib", default=ROOT / "paper/papers/references.bib")
    ap.add_argument("--fragments", default=ROOT / "papers/fragments")
    args = ap.parse_args()
    session = args.session
    if not session:
        raise SystemExit("pass --session (your agent session id); writers stay in their own schema")

    fresh = not (HOME / f"{NAME}.gql.db").exists()
    kg = KG(HOME, NAME, session=session, create=True)
    with kg:
        if fresh:
            kg.dbinit(default_session=session)
        bib_out = import_bib(kg, args.bib)
        fragments = sorted(pathlib.Path(args.fragments).glob("p-*.md"))
        parsed = [parse_fragment(p) for p in fragments]
        label_targets = {}
        for name, _tex, _labels, text in parsed:
            for lm in re.finditer(r"\\label\{([^}]+)\}", text):
                label_targets[lm.group(1)] = name
        nodes = 0
        cites = 0
        refs = 0
        for name, tex, labels, text in parsed:
            kg.put(
                f"/paper/{name}",
                text,
                content_type="text/x-tex",
                label=classify(text),
                props={"x_tex": tex, "x_labels": labels},
            )
            nodes += 1
            for cm in re.finditer(r"\\cite(?:\[[^\]]*\])?\{([^}]+)\}", text):
                for key in cm.group(1).split(","):
                    key = key.strip()
                    if key:
                        try:
                            kg.link(f"/paper/{name}", "CITES", f"/citations/{key}")
                            cites += 1
                        except Exception:
                            pass
            for rm in re.finditer(r"\\ref\{([^}]+)\}", text):
                target = label_targets.get(rm.group(1))
                if target and target != name:
                    try:
                        kg.link(f"/paper/{name}", "REFERENCES", f"/paper/{target}")
                        refs += 1
                    except Exception:
                        pass
        ordered = sorted(parsed, key=lambda t: int(t[1].split(":")[1].split("-")[0]) if ":" in t[1] and "-" in t[1] else 0)
        follows = 0
        for a, b in zip(ordered, ordered[1:]):
            kg.link(f"/paper/{a[0]}", "FOLLOWS", f"/paper/{b[0]}")
            follows += 1
        rows = []
        for name, _tex, _labels, _text in parsed:
            nid = kg.node_id(f"/paper/{name}")
            props = kg.propfind(f"/paper/{name}")
            rows.append((name, nid or "", props.get("_sha256", "")))
        tsv = HOME / "nodes.tsv"
        tsv.write_text("name\tid\tsha256\n" + "\n".join("\t".join(r) for r in rows) + "\n")
        print(
            {
                "fragments": len(fragments),
                "nodes": nodes,
                "cites_links": cites,
                "references_links": refs,
                "follows_links": follows,
                "bib_entries": bib_out["entries"],
                "bib_abstracts": bib_out["abstracts_stored"],
                "name_id_map": str(tsv),
            }
        )


if __name__ == "__main__":
    main()
