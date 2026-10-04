"""Generate the paragraph fragments and the initial render order from the
graded structure view.

Chain: paper/paper.tex -> (structure_extract.py) -> the graded structure
view -> this script -> papers/fragments/*.md (git, the golden source) and
paper/paper.md.j2 (the render order; edit freely, the paper renders from
it).

Usage: python3 paper/fragments_gen.py [structure.md]
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DEFAULT_STRUCTURE = ROOT / ".tmp" / "paper-structure" / "paper-structure-graded.md"
FRAGMENTS = ROOT / "papers" / "fragments"
ORDER = ROOT / "paper" / "paper.md.j2"


def main(structure_path=DEFAULT_STRUCTURE):
    src = pathlib.Path(structure_path).read_text()
    FRAGMENTS.mkdir(parents=True, exist_ok=True)
    seen = set()
    order_lines = ["<!-- render order: edit freely. Each p-line of the form",
                   "     p(open-quote name close-quote) emits the fragment",
                   "     papers/fragments/name.md -->"]
    scan = re.compile(
        r"^(#+ [^\n]*)$|<p x-name=\"([^\"]+)\"([^>]*)>([\s\S]*?)</p>",
        re.M,
    )
    for m in scan.finditer(src):
        if m.group(1):
            body = re.sub(r"\s*<!--[^>]*-->\s*$", "", m.group(1)).rstrip()
            order_lines.append("")
            order_lines.append(body)
            continue
        name = m.group(2)
        fragment = f'<p x-name="{name}"{m.group(3)}>{m.group(4)}</p>'
        out = FRAGMENTS / f"{name}.md"
        if name in seen:
            raise SystemExit(f"duplicate x-name in structure view: {name}")
        seen.add(name)
        out.write_text(fragment.strip() + "\n")
        order_lines.append("")
        order_lines.append(f"{{{{ p('{name}') }}}}")
    ORDER.write_text("\n".join(order_lines).strip() + "\n")
    print(f"fragments: {len(seen)} written to {FRAGMENTS}")
    print(f"order: {ORDER}")


if __name__ == "__main__":
    main(pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_STRUCTURE)
