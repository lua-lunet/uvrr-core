# /// script
# requires-python = ">=3.9"
# dependencies = ["jinja2"]
# ///
"""Render the paper order from paper/paper.md.j2.

The order file is a pointer into the content, not the content: each
paragraph line carries the NAME we assigned (the x-name from the
structure view). Three resolution modes:

- file (default): the paragraph comes from papers/fragments/, the git
  golden source. No database needed.
- graph (--graph): the name resolves through the derived graphlite
  index (.kg/): name -> node -> the database's own node id -> the
  sha256-addressed object. The fragment file is then used only to CHECK
  that the golden source and the object agree, byte for byte.
- --ids: print the name/id mapping (name = ours, id = the database's).

Usage:
  uv run paper/kg_render.py [--order FILE] [--out FILE] [--graph] [--ids]
"""

import argparse
import hashlib
import pathlib
import re
import sys

import jinja2

ROOT = pathlib.Path(__file__).resolve().parent.parent
ORDER = ROOT / "paper" / "paper.md.j2"
FRAGMENTS = ROOT / "papers" / "fragments"
GRAPH_HOME = ROOT / ".kg"
GRAPH_NAME = "paper"


def unwrap(raw):
    m = re.match(r"<p [^>]*>\n?\n?(.*?)\n?\n?</p>", raw.strip(), re.S)
    if not m:
        raise SystemExit("unparsable fragment")
    return m.group(1).strip()


def fragment_file(name):
    return (FRAGMENTS / f"{name}.md").read_text()


def open_graph(session):
    sys.path.insert(0, str(ROOT.parent / "knowledge-graphlite"))
    from kglite.kg import KG

    return KG(GRAPH_HOME, GRAPH_NAME, session=session, create=False)


def make_resolvers(graph_mode, kg, session):
    def node_id(name):
        if kg is None:
            raise SystemExit("--ids needs a graph: pass --session")
        return kg.node_id(f"/paper/{name}")

    def resolve(name):
        path = f"/paper/{name}"
        if kg is None:
            return unwrap(fragment_file(name))
        props = kg.propfind(path)
        obj = kg.obj.get(props["_sha256"])
        golden = unwrap(fragment_file(name)).encode()
        if hashlib.sha256(obj).hexdigest() != hashlib.sha256(golden).hexdigest():
            raise SystemExit(
                f"{name}: the graph object and the golden fragment disagree; "
                "re-run kg_load.py"
            )
        return obj.decode().strip()

    return resolve, node_id


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--order", default=ORDER)
    ap.add_argument("--out", default=None)
    ap.add_argument("--graph", action="store_true")
    ap.add_argument("--ids", action="store_true")
    ap.add_argument("--session", default="paper")
    args = ap.parse_args()
    kg = None
    if args.graph or args.ids:
        kg = open_graph(args.session)
    resolve, node_id = make_resolvers(args.graph, kg, args.session)
    if args.ids:
        for frag in sorted(FRAGMENTS.glob("p-*.md")):
            name = frag.stem
            print(f"{name}\t{node_id(name)}")
        if kg:
            kg.close()
        return 0
    env = jinja2.Environment(
        undefined=jinja2.StrictUndefined,
        trim_blocks=True,
        lstrip_blocks=True,
    )
    env.globals["p"] = resolve
    env.globals["pid"] = node_id
    rendered = env.from_string(pathlib.Path(args.order).read_text()).render()
    if args.out:
        pathlib.Path(args.out).write_text(rendered + "\n")
        print(f"rendered: {args.out}")
    else:
        sys.stdout.write(rendered + "\n")
    if kg:
        kg.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())