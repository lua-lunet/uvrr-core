#!/usr/bin/env python3
"""Audit every named theorem/definition in the compiled project sources.

The declarations here deliberately use simple namespace blocks; fail closed if
a declaration cannot be queried through import UVRR. This checks dependencies,
not whether a theorem statement is the desired protocol specification.
"""
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parent
ALLOWED = {"propext", "Quot.sound", "Classical.choice"}


def declaration_names():
    names = []
    for path in sorted((ROOT / "UVRR").glob("*.lean")):
        namespaces = []
        for line in path.read_text().splitlines():
            match = re.match(r"namespace\s+(\S+)\s*$", line)
            if match:
                namespaces.append(match[1])
            elif re.match(r"end(?:\s+\S+)?\s*$", line):
                if not namespaces:
                    raise RuntimeError(f"unexpected end in {path}")
                namespaces.pop()
            match = re.match(r"(?:noncomputable\s+)?(?:theorem|def|abbrev)\s+([\w.']+)", line)
            if match:
                names.append(".".join(namespaces + [match[1]]))
        if namespaces:
            raise RuntimeError(f"unclosed namespace in {path}")
    if not names or len(names) != len(set(names)):
        raise RuntimeError("empty or ambiguous declaration inventory")
    return names


def main():
    names = declaration_names()
    query = "import UVRR\n" + "".join(f"#print axioms {name}\n" for name in names)
    result = subprocess.run(
        ["lake", "env", "lean", "-M1024", "-T50000", "--stdin"],
        input=query, text=True, capture_output=True, cwd=ROOT, timeout=60,
    )
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)
    seen = set()
    for line in result.stdout.splitlines():
        match = re.match(r"'(.+)' (?:depends on axioms: \[(.*)\]|does not depend on any axioms)$", line)
        if not match:
            raise RuntimeError(f"unexpected axiom output: {line}")
        seen.add(match[1])
        dependencies = set(filter(None, (match[2] or "").split(", ")))
        if dependencies - ALLOWED:
            raise RuntimeError(f"unapproved axioms in {match[1]}: {dependencies - ALLOWED}")
    if seen != set(names):
        raise RuntimeError("axiom output did not cover the declaration inventory")
    print(f"PASS {len(names)} declarations: only standard Lean axioms")


if __name__ == "__main__":
    main()
