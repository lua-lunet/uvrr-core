# paper/diagrams — working rules

Message-trace diagrams for the paper: sources in a small trace language,
rendered to deterministic SVG. `papers/` stays a pure version record; the
paper's `.tex` is not touched here (figure incorporation happens in a paper
update).

## Contents

- `msgtrace.py` — PEP-723 `uv` script, stdlib only. Renders a `.trace` file
  to a byte-deterministic SVG: lifelines per actor, numbered arrows at
  discrete time steps, crash / reincarnate markers, shaded era bands.
- `reincarnation.trace` — the uVRR reincarnation two-era forced sequence
  (unit scale): C crashes and is fenced, reincarnates as D, era 1 crosses
  `DECREMENT(C)+JOIN(D)` with D joining at weight 0 (standby), era 2 runs
  `INCREMENT(D)+LEAVE(C)` with D promoted 0→1 and C leaving at weight 0.
  The quorum facts are stated per rung 23's three-node degenerate case: the
  forming quorum `{A,B}` coincides across both eras (mass 2 of total 2 in
  era 1, mass 2 of total 3 in era 2), leader B is in it throughout, and no
  casting-vote pivot is exercised on this path.
- `Makefile` — verbs: `make diagram NAME=<trace name>`, `make clean`.

## Trace language

One directive per line; `#` comments; blank lines ignored.

    title <text>                  caption (optional)
    actors NAME[(label)] ...      lifelines, declared once, left to right
    era <text>                    begins a shaded band over subsequent rows
    crash NAME@STEP               crash marker on NAME's lifeline at STEP
    reincarnate NAME@STEP         birth marker on NAME's lifeline at STEP
    STEP SRC->DST: label          solid numbered arrow
    STEP SRC-->DST: label         dashed numbered arrow

Example:

    actors A B(leader) C D
    crash C@3
    6 B->A: prepare batch: DECREMENT(C)+JOIN(D)

Rows are the sorted step numbers; every arrow and marker lands on its
step's row.

## Output rules

- SVG is always written and is deterministic (same input → same bytes);
  SVGs are tracked.
- PNG is a regeneration cache: it is written only when absent and is never
  regenerated over an existing file. PNGs are gitignored. Rasterization
  uses headless Chrome, falling back to ImageMagick.
- `make clean` removes both; `make diagram NAME=x` reproduces from clean.

## Vision check

Rendered output is verified by looking at it before it is tracked: actors,
numbered arrows in order, crash marker, era bands, and the reincarnated
node's weight rise must all be visibly correct. Record the check here.

- **2026-09-08** — `reincarnation.svg` viewed in Chrome at full page.
  Confirmed: four actor headers with `B (leader)`; arrows 1–11 in step
  order; red crash X on C's lifeline with C's lifeline dashed grey
  (fenced) below it; green reincarnate marker on D's lifeline with D's
  lifeline starting there; era 1 shaded band containing arrows 6–8 with
  the `(1,1,0,0)` weights and quorum `{A,B}` = 2 of 2; era 2 shaded band
  containing arrows 9–11 with D promoted 0→1, C leaving at weight 0, and
  the same forming quorum `{A,B}` = 2 of 3 with no pivot. Two earlier
  defects (era labels colliding with the first arrow row; band extents
  keyed by step number instead of row index) were found in the first two
  renders and fixed before this check passed.
