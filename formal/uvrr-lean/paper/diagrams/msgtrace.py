#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""message-traces: time-stepped message-sequence renderer.

Reads a small trace language (mermaid-sequence-like) and emits a
deterministic SVG: lifelines per actor, numbered arrows placed at
discrete time steps, crash / reincarnate markers, and shaded era bands.

Language (one directive per line, `#` comments, blank lines ignored):

    title <text>                 caption drawn at the top (optional)
    actors NAME[(label)] ...     lifelines, declared once, left to right
    era <text>                   begins a shaded band over subsequent rows
    crash NAME@STEP              crash marker on NAME's lifeline at STEP
    reincarnate NAME@STEP        birth marker on NAME's lifeline at STEP
    STEP SRC->DST: label         solid numbered arrow
    STEP SRC-->DST: label        dashed numbered arrow

Rows are the sorted set of step numbers; every arrow and marker lands on
its step's row, so layout is time-stepped by construction. Output is
byte-identical for identical input.
"""

import argparse
import html
import re
import sys
from pathlib import Path

ARROW = re.compile(r"^(\d+)\s+([A-Za-z0-9_]+)(-{1,2}>)([A-Za-z0-9_]+)\s*:\s*(.+)$")
MARKER = re.compile(r"^(crash|reincarnate)\s+([A-Za-z0-9_]+)@(\d+)$")

BAND_FILL = ["#eef3fb", "#fdf4e7"]
FONT = "Helvetica, Arial, sans-serif"
ROW_H = 58
COL_W = 190
LEFT = 90
RIGHT = 230
TOP_TITLE = 30
TOP_ACTORS = 66
LIFELINE_TOP = 96


def esc(s: str) -> str:
    return html.escape(s, quote=True)


def fmt(v: float) -> str:
    r = round(v, 2)
    if abs(r - int(r)) < 1e-9:
        return str(int(r))
    return f"{r:.2f}"


def wrap(text: str, width: int) -> list[str]:
    lines, cur = [], ""
    for word in text.split():
        if cur and len(cur) + 1 + len(word) > width:
            lines.append(cur)
            cur = word
        else:
            cur = f"{cur} {word}".strip()
    if cur:
        lines.append(cur)
    return lines


def parse(path: Path):
    events = []
    for lineno, raw in enumerate(path.read_text().splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("title "):
            events.append(("title", line[6:].strip()))
        elif line.startswith("actors "):
            names = []
            for tok in line[7:].split():
                if "(" in tok:
                    name, label = tok.split("(", 1)
                    names.append((name, label.rstrip(")")))
                else:
                    names.append((tok, tok))
            events.append(("actors", names))
        elif line.startswith("era "):
            events.append(("era", line[4:].strip()))
        elif m := MARKER.match(line):
            events.append((m.group(1), (m.group(2), int(m.group(3)))))
        elif m := ARROW.match(line):
            events.append(
                ("arrow", (int(m.group(1)), m.group(2), m.group(3) == "-->", m.group(4), m.group(5)))
            )
        else:
            sys.exit(f"{path}:{lineno}: cannot parse line: {raw}")
    return events


def build(events, srcname: str):
    if not events or events[0][0] != "actors":
        sys.exit(f"{srcname}: must begin with an `actors` directive")
    actors = {name: (i, label) for i, (name, label) in enumerate(events[0][1])}
    title = next((payload for kind, payload in events if kind == "title"), "")

    steps = {p[0] for k, p in events if k == "arrow"} | {p[1] for k, p in events if k in ("crash", "reincarnate")}
    row_of = {step: i for i, step in enumerate(sorted(steps))}

    bands = []  # {label, rows: [row indexes]}
    current = None
    arrows, crashes, births = [], [], []
    for kind, payload in events[1:]:
        if kind == "era":
            current = {"label": payload, "rows": []}
            bands.append(current)
        elif kind in ("crash", "reincarnate"):
            name, step = payload
            if name not in actors:
                sys.exit(f"{srcname}: unknown actor {name!r}")
            if current is not None:
                current["rows"].append(row_of[step])
            (crashes if kind == "crash" else births).append((name, step))
        elif kind == "arrow":
            step, src, dashed, dst, label = payload
            for a in (src, dst):
                if a not in actors:
                    sys.exit(f"{srcname}: unknown actor {a!r}")
            if src == dst:
                sys.exit(f"{srcname}: self arrow at step {step} not supported")
            if current is not None:
                current["rows"].append(row_of[step])
            arrows.append((step, src, dashed, dst, label))
    return actors, title, row_of, bands, arrows, crashes, births


def render(actors, title, row_of, bands, arrows, crashes, births, srcname: str) -> str:
    n = len(actors)
    steps_sorted = sorted(row_of)
    band_label_lines = []
    band_first_row = []
    for band in bands:
        if not band["rows"]:
            sys.exit(f"{srcname}: era band with no rows: {band['label']!r}")
        band_label_lines.append(wrap(band["label"], 96))
        band_first_row.append(min(band["rows"]))
    # Extra vertical gap inserted before a band's first row so the band's
    # label block fits inside the band above its first arrow without
    # touching the previous row or the band above.
    gap_before = {}
    for lines, first in zip(band_label_lines, band_first_row):
        gap_before[first] = max(6, 76 + 15 * (len(lines) - 1) - ROW_H)
    y = {}
    cursor = LIFELINE_TOP
    for r, step in enumerate(steps_sorted):
        if r in gap_before:
            cursor += gap_before[r]
        y[r] = cursor
        cursor += ROW_H
    yr = y  # keyed by row index; resolve steps via row_of
    bottom = yr[len(steps_sorted) - 1] + 34
    width = LEFT + (n - 1) * COL_W + RIGHT
    x = {name: LEFT + i * COL_W for name, (i, _) in actors.items()}
    ys = lambda step: yr[row_of[step]]

    crash_steps = {name: step for name, step in crashes}
    birth_steps = {name: step for name, step in births}

    out: list[str] = []
    out.append('<?xml version="1.0" encoding="UTF-8"?>')
    out.append(
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{fmt(width)}" height="{fmt(bottom + 16)}" '
        f'viewBox="0 0 {fmt(width)} {fmt(bottom + 16)}" font-family="{FONT}">'
    )
    out.append(f'<rect width="{fmt(width)}" height="{fmt(bottom + 16)}" fill="#ffffff"/>')

    def text(xp, yp, s, size: float = 13, fill="#111827", anchor="middle", weight="normal", style=None):
        attrs = f' font-style="{style}"' if style else ""
        out.append(
            f'<text x="{fmt(xp)}" y="{fmt(yp)}" font-size="{size}" fill="{fill}" '
            f'text-anchor="{anchor}" font-weight="{weight}"{attrs}>{esc(s)}</text>'
        )

    if title:
        text(24, TOP_TITLE, title, size=15, anchor="start", weight="bold")

    # Era bands (behind everything else); labels sit inside the band above
    # its first row.
    for bi, band in enumerate(bands):
        lines = band_label_lines[bi]
        first = band_first_row[bi]
        last = max(band["rows"])
        top = y[first] - 44 - 15 * (len(lines) - 1)
        bot = y[last] + 20
        out.append(
            f'<rect x="16" y="{fmt(top)}" width="{fmt(width - 32)}" height="{fmt(bot - top)}" '
            f'fill="{BAND_FILL[bi % len(BAND_FILL)]}" stroke="#c9d2de" stroke-width="1"/>'
        )
        ly = top + 16
        for line in lines:
            text(24, ly, line, size=12.5, fill="#334155", anchor="start", weight="bold")
            ly += 15

    # Lifelines.
    for name, (i, _) in actors.items():
        xn = x[name]
        y0, y1 = LIFELINE_TOP, bottom
        if name in crash_steps:
            yc = ys(crash_steps[name])
            out.append(
                f'<line x1="{fmt(xn)}" y1="{fmt(y0)}" x2="{fmt(xn)}" y2="{fmt(yc)}" stroke="#111827" stroke-width="1.5"/>'
            )
            out.append(
                f'<line x1="{fmt(xn)}" y1="{fmt(yc)}" x2="{fmt(xn)}" y2="{fmt(y1)}" stroke="#9ca3af" '
                f'stroke-width="1.5" stroke-dasharray="3 5"/>'
            )
        elif name in birth_steps:
            yb = ys(birth_steps[name])
            out.append(
                f'<line x1="{fmt(xn)}" y1="{fmt(yb)}" x2="{fmt(xn)}" y2="{fmt(y1)}" stroke="#111827" stroke-width="1.5"/>'
            )
        else:
            out.append(
                f'<line x1="{fmt(xn)}" y1="{fmt(y0)}" x2="{fmt(xn)}" y2="{fmt(y1)}" stroke="#111827" stroke-width="1.5"/>'
            )

    # Actor headers.
    for name, (i, label) in actors.items():
        display = name if label == name else f"{name} ({label})"
        xn = x[name]
        box_w = 44 + 9 * len(display)
        out.append(
            f'<rect x="{fmt(xn - box_w / 2)}" y="{fmt(TOP_ACTORS - 18)}" width="{fmt(box_w)}" height="26" rx="5" '
            f'fill="#1f2937"/>'
        )
        out.append(
            f'<text x="{fmt(xn)}" y="{fmt(TOP_ACTORS)}" font-size="13.5" fill="#ffffff" '
            f'text-anchor="middle" font-weight="bold">{esc(display)}</text>'
        )

    # Markers.
    for name, step in crashes:
        xn, yn = x[name], ys(step)
        out.append(
            f'<line x1="{fmt(xn - 8)}" y1="{fmt(yn - 8)}" x2="{fmt(xn + 8)}" y2="{fmt(yn + 8)}" '
            f'stroke="#b3261e" stroke-width="3" stroke-linecap="round"/>'
        )
        out.append(
            f'<line x1="{fmt(xn - 8)}" y1="{fmt(yn + 8)}" x2="{fmt(xn + 8)}" y2="{fmt(yn - 8)}" '
            f'stroke="#b3261e" stroke-width="3" stroke-linecap="round"/>'
        )
        text(xn + 14, yn + 4, "crash (fenced)", size=12, fill="#b3261e", anchor="start", weight="bold")
    for name, step in births:
        xn, yn = x[name], ys(step)
        out.append(
            f'<circle cx="{fmt(xn)}" cy="{fmt(yn)}" r="7" fill="#ffffff" stroke="#0b7a3e" stroke-width="2.5"/>'
        )
        text(xn + 14, yn + 4, "reincarnate (new identity)", size=12, fill="#0b7a3e", anchor="start", weight="bold")

    # Numbered arrows.
    for step, src, dashed, dst, label in arrows:
        x1, x2 = x[src], x[dst]
        ya = ys(step)
        direction = 1 if x2 >= x1 else -1
        dash = ' stroke-dasharray="6 4"' if dashed else ""
        out.append(
            f'<line x1="{fmt(x1)}" y1="{fmt(ya)}" x2="{fmt(x2 - direction * 9)}" y2="{fmt(ya)}" '
            f'stroke="#111827" stroke-width="1.6"{dash}/>'
        )
        tip, base = x2, x2 - direction * 10
        out.append(
            f'<polygon points="{fmt(tip)},{fmt(ya)} {fmt(base)},{fmt(ya - 4.5)} {fmt(base)},{fmt(ya + 4.5)}" '
            f'fill="#111827"/>'
        )
        mid = (x1 + x2) / 2
        out.append(
            f'<text x="{fmt(mid)}" y="{fmt(ya - 8)}" font-size="12.5" fill="#111827" text-anchor="middle">'
            f'<tspan font-weight="bold">{step}</tspan><tspan dx="6">{esc(label)}</tspan></text>'
        )

    out.append("</svg>")
    return "\n".join(out) + "\n"


def main() -> None:
    ap = argparse.ArgumentParser(description="render a message trace to deterministic SVG")
    ap.add_argument("trace", type=Path)
    ap.add_argument("-o", "--out", type=Path, default=None)
    args = ap.parse_args()
    out = args.out or args.trace.with_suffix(".svg")
    events = parse(args.trace)
    actors, title, row_of, bands, arrows, crashes, births = build(events, str(args.trace))
    svg = render(actors, title, row_of, bands, arrows, crashes, births, str(args.trace))
    out.write_text(svg, encoding="utf-8")
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
