# Fig. 2 (page 5) — Invariants preserved by the Paxos algorithm

Standalone description written from the rendered figure image alone.

---

The image contains a list of properties labeled P1 through P7. Each property is a logical statement involving configurations, promises, acceptance, and quorums. Here is a detailed summary of each property:

- **P1**: There are configurations ⟨Q₀ᴵ, Q₀ᴵᴵ⟩, ⟨Q₁ᴵ, Q₁ᴵᴵ⟩, ... where Qᴵᴵₑ ⊂ Qᴵₑ ⊂ Qᴵᴵₑ₊₁ for each e.

- **P2**: If promised≥ᵢ(a, b) then e(b) ≤ e(min(i, i_max)) and ¬acceptedⱼ(a, b') for all j ≥ i and all b' < b.

- **P3**: If promisedᵢ(a, b) then e(b) ≤ e(min(i, i_max)) and ¬acceptedᵢ(a, b') for all b' < b.

- **P4**: If promisedᵢ(a, b; b') then e(b) ≤ e(min(i, i_max)), b' < b, acceptedᵢ(a, b'), and b' is the greatest such ballot in the sense that ¬acceptedᵢ(a, b'') for all b'' having b' < b'' < b.

- **P5**: If proposedᵢ(b) then there is a quorum q ∈ Qₑ₍ₑ₎ such that for every a ∈ q one of the following holds:
  - promised≥ⱼ(a, b) for some j ≤ i, or
  - promisedᵢ(a, b), or
  - promisedᵢ(a, b; b') for some b'.
  Furthermore, if P ≜ {b' | ∃a ∈ q: promisedᵢ(a, b; b')} ≠ ∅ then vᵢ(b) = vᵢ(max(P)).

- **P6**: If acceptedᵢ(a, b) then proposedᵢ(b).

- **P7**: If chosenᵢ(b) then i ≤ i_max, e(i) ≤ e(b) + 1, and there is a quorum q ∈ Qᴵᴵₑ₍ᵢ₎ with acceptedᵢ(a, b) for every a ∈ q.
