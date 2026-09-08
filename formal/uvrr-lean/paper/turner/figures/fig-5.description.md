# Fig. 5 (page 8) — Message flow for Stoppable Paxos

Standalone description written from the rendered figure image alone.

---

### Diagram Overview
- **Type**: Node-arrow diagram
- **Geometry**: Three vertical lifelines (columns) labeled \(a_1\), \(\ell\), and \(a_2\) from left to right. Horizontal and diagonal arrows connect events across these lifelines. The diagram is divided into two eras, \(e\) and \(e+1\), separated by a dashed horizontal line.

---

### Elements
#### Lifelines (Vertical Lines)
- \(a_1\) (leftmost)
- \(\ell\) (middle)
- \(a_2\) (rightmost)

#### Eras (Horizontal Bands)
- **Era \(e\)**: Above the dashed line
- **Era \(e+1\)**: Below the dashed line

---

### Text Labels and Positions
- **Top of \(a_1\) lifeline**: No label
- **Top of \(\ell\) lifeline**: No label
- **Top of \(a_2\) lifeline**: No label
- **Left of \(a_1\) lifeline, upper section**: "prepare(\(b\))" (arrow starts here)
- **Left of \(a_1\) lifeline, lower section**: "prepare(\(b'\))" (arrow starts here)
- **Right of \(a_2\) lifeline, upper section**:
  - "proposed\(_i(b)\)"
  - "proposed\(_{i+3}(b)\)"
  - "proposed\(_{i+1}(b)\)"
  - "proposed\(_{i+2}(b)\)"
- **Right of \(a_2\) lifeline, lower section**:
  - "proposed\(_{i+4}(b')\)"
  - "proposed\(_{i+5}(b')\)"
  - "proposed\(_{i+6}(b')\)"
- **Between \(a_1\) and \(\ell\) lifelines**: Dashed horizontal line labeled "era \(e\)" (above) and "era \(e+1\)" (below)

---

### Arrows (Directed Edges)
#### From \(a_1\) to \(\ell\):
- **Upper section (Era \(e\))**:
  - Arrow from "prepare(\(b\))" on \(a_1\) to \(\ell\).
- **Lower section (Era \(e+1\))**:
  - Arrow from "prepare(\(b'\))" on \(a_1\) to \(\ell\).

#### From \(\ell\) to \(a_2\):
- **Upper section (Era \(e\))**:
  - Four diagonal arrows from \(\ell\) to \(a_2\), each pointing to one of the following labels on \(a_2\):
    - "proposed\(_i(b)\)"
    - "proposed\(_{i+3}(b)\)"
    - "proposed\(_{i+1}(b)\)"
    - "proposed\(_{i+2}(b)\)"
- **Lower section (Era \(e+1\))**:
  - Three diagonal arrows from \(\ell\) to \(a_2\), each pointing to one of the following labels on \(a_2\):
    - "proposed\(_{i+4}(b')\)"
    - "proposed\(_{i+5}(b')\)"
    - "proposed\(_{i+6}(b')\)"

---
---
### Ordering and Phasing
- **Top-to-Bottom**: The diagram progresses from era \(e\) (top) to era \(e+1\) (bottom).
- **Left-to-Right**: Actions originate at \(a_1\), pass through \(\ell\), and terminate at \(a_2\).
- **Grouping**: Proposals are grouped by era and by the input (\(b\) or \(b'\)).

---
### Overall Geometry
- **Columns**: Three aligned vertical lifelines (\(a_1\), \(\ell\), \(a_2\)).
- **Bands**: Two horizontal eras (\(e\) and \(e+1\)) separated by a dashed line.
- **Arrows**: Diagonal arrows indicate the flow of actions from preparation to proposal stages.
