# Fig. 1 (page 3) — Invariants preserved by the Synod algorithm

Standalone description written from the rendered figure image alone.

---

The image contains a list of formal statements labeled S1 through S6. These statements appear to be axioms or conditions related to a system involving quorums, ballots, nodes, and certain predicates such as proposed, chosen, promised, and accepted. The text is mathematical and logical in nature, using symbols like subset (⊆), logical implication (→), and quantifiers (∀, ∃). Below is a detailed breakdown of each statement:

---

- **S1**
  For all b1 ≻ b2 ∈ B there are sets of quorums Q^I(b1) and Q^II(b2) ⊆ P(A) such that if **proposed(b1)** and **chosen(b2)** then Q^I(b1) → Q^II(b2).

- **S2**
  If **promised(a, b)** then ¬**accepted(a, b')** for all b' ≺ b.

- **S3**
  If **promised(a, b; b')** then b' ≺ b, **accepted(a, b')**, and b' is the greatest such ballot in the sense that ¬**accepted(a, b'')** for all b'' having b' ≺ b'' ≺ b.

- **S4**
  If **proposed(b)** then there is a quorum q^I ∈ Q^I(b) such that for every node a ∈ q^I either **promised(a, b)** or else there exists a b' such that **promised(a, b; b')**; if also P ≜ {b' | ∃a ∈ q^I. **promised(a, b; b')**} ≠ ∅ then v(b) = v(max(P)).

- **S5**
  If **accepted(a, b)** then **proposed(b)**.

- **S6**
  If **chosen(b)** then there is a quorum q^II ∈ Q^II(b) such that **accepted(a, b)** for every a ∈ q^II.

---
The statements are presented in a structured, logical format, with each condition or axiom clearly delineated. The notation and terminology suggest a formal system, possibly related to distributed computing or consensus algorithms.
