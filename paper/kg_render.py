# /// script
# requires-python = ">=3.9"
# dependencies = ["jinja2"]
# ///
"""Render the paper order from paper/paper.md.j2, emitting each paragraph
from its fragment file (papers/fragments/, the git golden source; no
database involved — the graph is the derived index, the files are the
content).

Usage: uv run paper/kg_render.py [--order FILE] [--out FILE]
"""

import argparse
import pathlib
import re
import sys

import jinja2

ROOT = pathlib.Path(__file__).resolve().parent.parent
ORDER = ROOT / "paper" / "paper.md.j2"
FRAGMENTS = ROOT / "papers" / "fragments"


def fragment(name):
    path = FRAGMENTS / f"{name}.md"
    raw = path.read_text().strip()
    m = re.match(r'<p [^>]*>\n?\n?(.*?)\n?\n?</p>', raw, re.S)
    if not m:
        raise SystemExit(f"unparsable fragment: {path}")
    return m.group(1).strip()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--order", default=ORDER)
    ap.add_argument("--out", default=None)
    args = ap.parse_args()
    env = jinja2.Environment(
        undefined=jinja2.StrictUndefined,
        trim_blocks=True,
        lstrip_blocks=True,
    )
    env.globals["p"] = fragment
    rendered = env.from_string(pathlib.Path(args.order).read_text()).render()
    if args.out:
        pathlib.Path(args.out).write_text(rendered + "\n")
        print(f"rendered: {args.out}")
    else:
        sys.stdout.write(rendered + "\n")


if __name__ == "__main__":
    main()
