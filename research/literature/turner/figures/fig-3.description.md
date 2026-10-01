# Fig. 3 (page 6) — Message flow during a reconfiguration

Standalone description written from the rendered figure image alone.

---

### Diagram Overview
- **Type**: Node-arrow diagram with vertical lifelines and horizontal/vertical bands.
- **Geometry**: Three vertical lines (lifelines) labeled at the top, with horizontal and vertical dashed lines dividing the diagram into regions (eras).

---

### Elements
#### Lifelines (Vertical Lines)
- **Left lifeline**: Labeled at the top as **a1**.
- **Middle lifeline**: Labeled at the top as **ℓ**.
- **Right lifeline**: Labeled at the top as **a2**.

#### Eras (Horizontal Bands)
- **Top era**: Labeled on the left as **era e**.
- **Bottom era**: Labeled on the left as **era e + 1**.
- Separated by a horizontal dashed line.

#### Nodes/Events
- **On left lifeline (a1)**:
  - **prepare(b)**: Near the top, above the era e line.
  - **prepare(b')**: Below the era e line, near the start of era e + 1.
- **On middle lifeline (ℓ)**:
  - No labeled nodes, but multiple arrows originate or terminate here.
- **On right lifeline (a2)**:
  - **proposed_i(b)**: Topmost, just below the era e line.
  - **proposed_{i+1}(b)**
  - **proposed_{i+2}(b)**
  - **proposed_{i+3}(b)**
  - **proposed_{i+4}(b)**
  - **proposed_{i+5}(b)**
  - **proposed_{i+6}(b)**
  - **proposed_{i+7}(b)**
  - **proposed_{i+8}(b)**
  - **proposed_{i+9}(b)**
  - **proposed_{i+10}(b)**
  - **proposed_{i+11}(b)**
  - **proposed_{i+12}(b')**
  - **proposed_{i+13}(b')**

---

### Arrows/Edges
- **From prepare(b) on a1**:
  - Arrow points to the middle lifeline (ℓ), just below prepare(b).
- **From prepare(b') on a1**:
  - Arrow points to the middle lifeline (ℓ), just below prepare(b').
- **From middle lifeline (ℓ) to right lifeline (a2)**:
  - Multiple arrows originate from ℓ and point to each **proposed** node on a2, in descending order from **proposed_i(b)** to **proposed_{i+13}(b')**.
  - Arrows are diagonal, slanting downward from left to right.

---

### Spatial Layout and Ordering
- **Top to Bottom**:
  - **Era e** contains **prepare(b)** on a1 and **proposed_i(b)** to **proposed_{i+5}(b)** on a2.
  - **Era e + 1** contains **prepare(b')** on a1 and **proposed_{i+6}(b)** to **proposed_{i+13}(b')** on a2.
- **Left to Right**:
  - Lifelines are ordered as a1, ℓ, a2.
  - Arrows flow from a1 to ℓ, then from ℓ to a2.

---
### Summary of Labels
- **Lifelines**: a1, ℓ, a2.
- **Nodes on a1**: prepare(b), prepare(b').
- **Nodes on a2**: proposed_i(b), proposed_{i+1}(b), ..., proposed_{i+13}(b').
- **Eras**: era e (top), era e + 1 (bottom).
