#!/usr/bin/env python3
"""Mirror the knowledge-base fragment corpus into a cold, append-only copy.

One master, eventual consistency elsewhere. The corpus is golden in git and
indexed by a derived, gitignored database; this directory is a durable copy
that lives outside both, so the corpus survives losing the index or the
tooling that builds it.

The copy is content addressed. `objects/<sha256>` holds the bytes and
`fragments.tsv` holds one row per revision of a fragment. A new revision is a
new row: rows are never deleted and never overwritten, and an object is
written once and never rewritten. A mistaken revision therefore adds a row and
a file; it cannot corrupt either.

The encoding is scaffolding. `version` is a unix epoch standing in for a
revision number, so two revisions may share a version, and it is not
guaranteed to be a timestamp; `ctime` and `mtime` are carried separately
because of that. Fork, deprecate and the other revision semantics are
deferred: this encoding pre-dates the knowledge-base refactor and is expected
to be replaced by an encoding in the database itself.

Exit status is 0 when the mirror is consistent and 1 when it is not, so the
check is usable as a gate.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import os
import pathlib
import sys

FIELDS = ("name", "version", "ctime", "mtime", "sha256")
HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parent.parent


def fragments_dir() -> pathlib.Path:
    return REPO / "papers" / "fragments"


def now() -> dt.datetime:
    return dt.datetime.now(dt.timezone.utc)


def read_rows(tsv: pathlib.Path) -> list[dict]:
    if not tsv.exists():
        return []
    with tsv.open(newline="", encoding="utf-8") as fh:
        return [row for row in csv.DictReader(fh, delimiter="\t") if row.get("name")]


def write_object(objects: pathlib.Path, sha: str, data: bytes) -> str:
    dest = objects / sha
    if dest.exists():
        return "present"
    fd = os.open(dest, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
    with os.fdopen(fd, "wb") as fh:
        fh.write(data)
    return "written"


def mirror(root: pathlib.Path, source: pathlib.Path) -> int:
    tsv = root / "fragments.tsv"
    objects = root / "objects"
    objects.mkdir(parents=True, exist_ok=True)

    rows = read_rows(tsv)
    seen = {(row["name"], row["sha256"]) for row in rows}
    first_seen = {}
    for row in rows:
        first_seen.setdefault(row["name"], row["ctime"])

    new_file = not tsv.exists()
    added_rows = 0
    added_objects = 0
    fragments = sorted(
        p for p in source.iterdir() if p.is_file() and not p.name.startswith(".") and p.suffix != ".gitignore"
    )

    with tsv.open("a", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=FIELDS, delimiter="\t", lineterminator="\n")
        if new_file:
            writer.writeheader()
        for path in fragments:
            data = path.read_bytes()
            sha = hashlib.sha256(data).hexdigest()
            name = path.stem
            if (name, sha) in seen:
                continue
            stamp = now()
            epoch = int(stamp.timestamp())
            writer.writerow(
                {
                    "name": name,
                    "version": epoch,
                    "ctime": first_seen.setdefault(name, epoch),
                    "mtime": stamp.replace(microsecond=0).isoformat(),
                    "sha256": sha,
                }
            )
            seen.add((name, sha))
            added_rows += 1
            added_objects += write_object(objects, sha, data) == "written"

    total = len(read_rows(tsv))
    print(f"source fragments: {len(fragments)}")
    print(f"rows appended:    {added_rows}")
    print(f"objects written:  {added_objects}")
    print(f"rows in mirror:   {total}")
    print(f"mirror:           {root}")
    return 0


def check(root: pathlib.Path) -> int:
    tsv = root / "fragments.tsv"
    objects = root / "objects"
    if not tsv.exists():
        print(f"FAIL: no mirror at {root}")
        return 1
    rows = read_rows(tsv)
    problems = []
    seen = set()
    for row in rows:
        key = (row["name"], row["sha256"])
        if key in seen:
            problems.append(f"duplicate row {key}")
        seen.add(key)
        if not row["name"] or not row["version"] or not row["ctime"] or not row["mtime"]:
            problems.append(f"incomplete row {key}")
        blob = objects / row["sha256"]
        if not blob.exists():
            problems.append(f"missing object for {key}")
        elif hashlib.sha256(blob.read_bytes()).hexdigest() != row["sha256"]:
            problems.append(f"object does not match its name for {key}")
    orphans = [p.name for p in objects.iterdir() if p.is_file() and p.name not in {r["sha256"] for r in rows}]
    print(f"rows:    {len(rows)}")
    print(f"objects: {sum(1 for p in objects.iterdir() if p.is_file())}")
    print(f"unique fragments: {len({r['name'] for r in rows})}")
    for problem in problems:
        print(f"FAIL: {problem}")
    if orphans:
        print(f"note: {len(orphans)} object(s) present but unreferenced")
    print("consistent" if not problems else f"{len(problems)} problem(s)")
    return 1 if problems else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="kb_mirror.py",
        description=(
            "Mirror the knowledge-base fragment corpus to a cold, append-only copy: "
            "one row per revision in fragments.tsv, content addressed by sha256 in objects/."
        ),
        epilog=(
            "The mirror is append only. A new revision of a fragment is a new row; no row is\n"
            "ever deleted or overwritten, and no object is ever rewritten, so an incorrect\n"
            "revision can only add to the mirror and never damage it. That makes the check\n"
            "the real test of the mirror's health: every row must name an object whose\n"
            "contents hash to its own name.\n\n"
            "The encoding is scaffolding pending the knowledge-base refactor. version is a\n"
            "unix epoch standing in for a revision number, two revisions may share one, and\n"
            "ctime and mtime are carried separately because a version is not necessarily a\n"
            "timestamp. Fork and deprecate semantics are deferred.\n\n"
            "examples:\n"
            "  kb_mirror.py                     append any new revisions, then report\n"
            "  kb_mirror.py --check             verify every row against its object, exit 1 on fault\n"
            "  kb_mirror.py --root DIR --check  check a mirror held elsewhere\n"
            "  kb_mirror.py --fragments DIR     mirror a different corpus\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--root",
        type=pathlib.Path,
        default=pathlib.Path.home() / "icloud" / "2026" / "UVRR" / "kb",
        help="destination mirror (default: ~/icloud/2026/UVRR/kb)",
    )
    parser.add_argument(
        "--fragments",
        type=pathlib.Path,
        default=None,
        help=f"corpus to mirror (default: {fragments_dir()})",
    )
    parser.add_argument("--check", action="store_true", help="verify the mirror instead of writing to it")
    args = parser.parse_args(argv)
    if args.check:
        return check(args.root)
    return mirror(args.root, args.fragments or fragments_dir())


if __name__ == "__main__":
    sys.exit(main())
