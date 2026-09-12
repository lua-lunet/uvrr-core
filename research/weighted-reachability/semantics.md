# Weighted identities, reachability and casting witnesses

Let `A` be a finite identity universe, `L ⊆ A` the identities which can respond,
and `w : A → ℕ` the committed weights. Define `M(w)` by
`q ∈ M(w) ↔ 2 Σ(q) w > Σ(A) w`.

An identity need not have a running process to occur in `A` or in a mathematical
quorum. A zero-weight identity changes no majority inequality. Assigning it a
positive weight changes the inequalities without manufacturing a response.

Three different obligations must remain separate:

1. **Agreement geometry:** every old phase-one quorum intersects every next
   decision quorum. These are universally quantified sets of identities;
   hypothetical quorums are included and need not respond.
2. **Available reconfiguration:** each configuration has a quorum contained in
   `L`, so the next protocol round can obtain its messages.
3. **Concurrent casting schedule:** choose the actual preparers so their new
   promises leave an actual decision quorum usable. For the singleton schedule,
   the two participating quorums intersect only at the leader.

An unavailable identity can occur in an unused safety witness. It cannot be
counted as having accepted a fresh proposal. Thus a failed casting schedule
does not establish failed reachability; conversely, reachability alone does not
establish the same casting schedule at every boundary.

## A constructive reachability theorem

Suppose `Σ(L) w > Σ(A \ L) w` and `Σ(L) v > Σ(A \ L) v`. There is a path from
`w` to `v`, changing one identity's weight by one on every edge, with a live
strict majority in every row. Its length is exactly `Σ(A) |w(a) - v(a)|`.

Construct the path in this order:

1. Decrease unavailable weights to their componentwise minimum with `v`.
2. Increase live weights to their componentwise maximum with `v`.
3. Decrease live weights to their target values in `v`.
4. Increase unavailable weights to their target values in `v`.

During the first two parts, live weight never decreases and unavailable weight
never increases, so the source strict majority remains. During the last two,
live weight is at least its target and unavailable weight is at most its target,
so the target strict majority applies. Each changed coordinate moves only towards
its target, proving the stated length. The one-unit weighted intersection lemma
establishes agreement geometry on each edge. Joins and leaves are the cases
`0 → 1` and `1 → 0`. The proof includes phantom identities and arbitrary finite
cluster size. If both endpoints use only weights `0,1,2`, so does the whole path.

Exact doubling and exact halving preserve every majority and the live-majority
inequality. They can be composed with these paths. Halving here requires even
weights; rounded halving is a different operation.

## Quantifying over the leader

The fixed leader's initial casting property is a separate hypothesis. For
`w = (1,2,2,2)` with the weight-one identity chosen as leader, a majority needs
weight at least four. Every majority containing that leader contains at least
two of the three weight-two identities. Therefore two such quorums share another
identity. This statement is wholly abstract and does not involve failures.
Choosing another leader, allowing an initial non-casting edge, or changing the
target theorem changes this example's relevance. It does not contradict the
reachability theorem above.

## Source and existing formalisation

[Turner's source](https://github.com/DaveCTurner/paxos-membership/blob/raft-like-reconfiguration/paxos-reconf.tex)
defines the fully concurrent operational casting witness using nonfailed nodes.
His message-flow example also requires the decision quorum to belong to the
new slot's configuration while its ballot still belongs to the preceding era.
The weighted-majority section permits finitely supported natural-number weights
over identities; its unit-edit and exact-scaling arguments are independent of
whether a particular identity currently responds.

`formal/uvrr-lean/UVRR/CastingVote.lean` proves that the stipulated singleton
witness preserves the old acceptance guards and that received promises cover
the new phase-one quorum after the leader promises. It does not construct a
witness from a live-majority hypothesis. Its guard-preservation conclusion is
valid even for an abstract quorum with unavailable members; obtaining new
decisions requires the separate message witnesses.
