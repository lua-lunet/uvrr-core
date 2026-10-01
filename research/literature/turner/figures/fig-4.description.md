# Fig. 4 (page 7) — Message flow for Dynamic Paxos with α = 2

Standalone description written from the rendered figure image alone.

---

### Diagram Overview
- **Type**: Node-arrow diagram
- **Overall Geometry**: Three vertical columns (lifelines) with horizontal and diagonal arrows connecting them. The diagram is divided into two horizontal eras labeled "era e" and "era e + 1".

---

### Elements Present
- **Nodes/Lifelines**:
  - Three vertical lines labeled at the top: `a1`, `ℓ`, `a2` (from left to right).
- **Arrows/Directed Edges**:
  - Multiple diagonal arrows connecting the lifelines, indicating actions or messages.
- **Bands/Regions**:
  - Two horizontal eras marked by dashed lines: `era e` and `era e + 1`.
- **Text Labels**:
  - Actions or events are labeled next to the arrows or lifelines.

---

### Text Labels and Their Positions
- **Left Column (`a1`)**:
  - `prepare(b)`: Near the top, pointing to the middle lifeline (`ℓ`).
  - `prepare(b')`: Below `prepare(b)`, pointing to the middle lifeline (`ℓ`).
- **Middle Column (`ℓ`)**:
  - No labels directly on this lifeline, but it is the target or source of many arrows.
- **Right Column (`a2`)**:
  - `proposed_i(b)`: Near the top, receiving arrows from the middle lifeline.
  - `proposed_{i+1}(b)`: Below `proposed_i(b)`.
  - `proposed_{i+2}(b)`: Below `proposed_{i+1}(b)`.
  - `proposed_{i+3}(b)`: Below `proposed_{i+2}(b)`.
  - `proposed_{i+4}(b)`: Below `proposed_{i+3}(b)`.
  - `proposed_{i+5}(b)`: Below `proposed_{i+4}(b)`.
  - `proposed_{i+6}(b')`: Below `proposed_{i+5}(b)`.
  - `proposed_{i+7}(b')`: Below `proposed_{i+6}(b')`.

- **Eras**:
  - `era e`: Dashed horizontal line separating the upper and lower parts of the diagram.
  - `era e + 1`: Below `era e`, marking the next phase.

---

### Direction and Endpoints of Arrows
- **From `a1` to `ℓ`**:
  - Two arrows labeled `prepare(b)` and `prepare(b')`, pointing from `a1` to `ℓ`.
- **From `ℓ` to `a2`**:
  - Multiple arrows pointing from `ℓ` to `a2`, each corresponding to a `proposed` label (e.g., `proposed_i(b)`, `proposed_{i+1}(b)`, etc.).
- **From `a2` to `ℓ`**:
  - No arrows directly from `a2` to `ℓ` are visible.

---

### Ordering, Phasing, or Grouping
- **Top-to-Bottom Flow**:
  - The diagram progresses from top to bottom, with `prepare` actions initiating the sequence, followed by `proposed` actions.
- **Eras**:
  - The diagram is divided into two eras: `era e` (upper part) and `era e + 1` (lower part).
  - The `prepare(b)` and `prepare(b')` actions occur in `era e`.
  - The `proposed` actions are split between `era e` and `era e + 1`, with `proposed_i(b)` to `proposed_{i+5}(b)` in `era e` and `proposed_{i+6}(b')` and `proposed_{i+7}(b')` in `era e + 1`.

---
### Overall Geometry
- **Columns**:
  - Three vertical lifelines (`a1`, `ℓ`, `a2`) aligned from left to right.
- **Alignment**:
  - Arrows are diagonal, connecting `a1` to `ℓ` and `ℓ` to `a2`.
  - Labels are aligned with their respective arrows or lifelines.
