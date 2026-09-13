"""Exhaustive finite checks for the Fuse addendum; standard library only.

Checks quorum arithmetic, not network or journal atomicity. Run from any cwd.
"""
import itertools
import json
from pathlib import Path


def mass(weights, mask):
    return sum(w for i, w in enumerate(weights) if mask & (1 << i))


def majority(weights, mask):
    return 2 * mass(weights, mask) > sum(weights)


def schedule_checks(rows):
    masks = range(1 << len(rows[0]))
    pairs = 0
    for left, right in zip(rows, rows[1:]):
        for a in masks:
            for b in masks:
                if majority(left, a) and majority(right, b):
                    assert a & b, (left, right, a, b)
                    pairs += 1
    responders = [r for r in masks if not r & 1 and majority(rows[0], r)]
    for r in responders:
        assert all(majority(w, r) for w in rows), (r, rows)
    return {"boundaries": len(rows) - 1, "cross_quorum_pairs": pairs,
            "retired_excluding_start_quorums": len(responders),
            "totals": list(map(sum, rows))}


def monotone_path(start, target, responders):
    current = list(start)
    yield tuple(current)
    # First improve the response margin, then spend only down to target margin.
    for inside, direction in ((False, -1), (True, 1), (True, -1), (False, 1)):
        for i in range(len(current)):
            if bool(responders & (1 << i)) != inside:
                continue
            while (target[i] - current[i]) * direction > 0:
                current[i] += direction
                yield tuple(current)


def main():
    three = [(1, 1, 1, 0), (0, 1, 1, 0), (0, 1, 1, 1)]
    five = [(1, 1, 1, 1, 1, 0), (2, 2, 2, 2, 2, 0),
            (2, 2, 2, 2, 2, 1), (1, 2, 2, 2, 2, 1),
            (0, 2, 2, 2, 2, 1), (0, 2, 2, 2, 2, 2),
            (0, 1, 1, 1, 1, 1)]
    report = {"three": schedule_checks(three), "five": schedule_checks(five)}
    profiles = list(itertools.product(range(3), repeat=4))
    paths = edges = 0
    for r in range(16):
        eligible = [w for w in profiles if majority(w, r)]
        for s in eligible:
            for t in eligible:
                path = list(monotone_path(s, t, r))
                assert path[-1] == t
                assert all(majority(w, r) for w in path)
                assert len(path) - 1 == sum(abs(a-b) for a, b in zip(s, t))
                for a, b in zip(path, path[1:]):
                    assert sum(abs(x-y) for x, y in zip(a, b)) == 1
                paths += 1
                edges += len(path) - 1
    # A legal unit-step path still need not preserve an arbitrary first quorum.
    swap = [(1, 2, 1, 2), (2, 2, 1, 2), (2, 1, 1, 2),
            (2, 1, 2, 2), (2, 1, 2, 1)]
    r = 0b1010  # B,D
    assert majority(swap[0], r) and not majority(swap[-1], r)
    for a, b in zip(swap, swap[1:]):
        assert sum(abs(x-y) for x, y in zip(a, b)) == 1
    report["common_response_paths"] = {"identities": 4, "weights": [0, 1, 2],
                                        "paths": paths, "unit_edges": edges}
    report["negative_control"] = {"rows": swap, "responders": ["B", "D"],
                                  "masses": [mass(w, r) for w in swap],
                                  "quorums": [majority(w, r) for w in swap]}
    out = Path(__file__).parent / "generated" / "fuse-summary.json"
    out.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
