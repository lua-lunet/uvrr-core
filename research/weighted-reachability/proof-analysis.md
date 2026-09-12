# Weighted configuration reachability: exact theorem and casting witnesses

This note separates two quantified statements: every available-majority endpoint is reachable through safe available-majority configurations; and every boundary on a path admits specified, responding singleton casting witnesses. The first statement has the constructive proof below for every finite identity set, including arbitrary unavailable identities. The second has an additional partition condition that should be tested explicitly.

## Definitions

Let `I` be any finite set of identities, `A ⊆ I` the identities able to respond during this construction, and `l ∈ A` the retained leader identity. An identity need not represent existing hardware. A configuration is `w : I → Nat`, with total weight `T(w)`. Its strict majority family is

`M(w) = {S ⊆ I : 2 w(S) > T(w)}`.

A configuration is available when `A ∈ M(w)`, equivalently `w(A) > w(I \ A)`. Assume `w(l) > 0` for both source and destination. Availability is a hypothesis about voting weight, not the unweighted number of live identities.

The elementary edges are changing one coordinate by one, positive common scaling (doubling and its legal exact inverse), and adding or removing a zero-weight identity. Extend source and destination to their common identity universe with zero weights before constructing the path.

## Theorem: all available configurations with a positive retained leader are connected

Given source `s` and destination `t` satisfying these hypotheses, a finite path from `s` to `t` exists such that:

1. Every edge changes one identity weight by exactly one.
2. Every intermediate configuration has a responding strict majority and a positive leader weight.
3. Every old strict-majority quorum intersects every new strict-majority quorum across every edge.
4. Each coordinate never exceeds the larger of its endpoint values, except that the common canonical leader has weight one, already at most both positive endpoint leader values.

Consequently, if both endpoints use only `{0,1,2}`, so does the entire path. Any number of unavailable phantom identities is allowed.

**Construction.** Reduce a configuration `w` to `c`, where `c(l)=1` and every other coordinate is zero:

* Decrement every unavailable coordinate to zero. These operations strictly increase `w(A)-w(I\A)`, preserving availability.
* Decrement every available nonleader coordinate to zero. Unavailable weight is now zero, and positive leader weight keeps available weight strictly positive.
* Decrement the leader to one. Again available weight remains strictly positive and unavailable weight remains zero.

Concatenate `s → c` with the reverse of `t → c`. Availability is a property of states, so reversing the destination reduction preserves it. Both increment and decrement are permitted edges. There are exactly `T(s)+T(t)-2` unit edges before cancelling any immediate reversals. This is at most `4|I|-2` when all endpoint weights are at most two. A direct path may be much shorter.

Join and leave operations can be inserted at zero weight. Doubling and exact halving are compatible additional operations, but are unnecessary for this reachability theorem. Their absence from this proof does not prescribe a production replacement schedule.

## Proof of cross-edge safety

Suppose `w'` increments one weight of `w`; write `T=T(w)`. If old majority `P` and new majority `Q` were disjoint, then under the new weights:

`w'(P)+w'(Q) ≤ T+1`.

But integrality and monotonicity give

`w'(P)+w'(Q) ≥ floor(T/2)+1 + floor((T+1)/2)+1 = T+2`,

a contradiction. Decrement is the reverse edge. Positive common scaling preserves exactly the majority family. Adding or removing a zero coordinate preserves the weighted majority predicate on the common identity universe. This is an identity-level intersection proof: no physical existence or response assumption appears in it.

## Exact casting-witness condition

For specified configurations `w,w'`, a specified leader `l`, and responding identities `A`, responding singleton witnesses exist precisely when there is a subset `B ⊆ A\{l}` satisfying

`w(B)+w(l) ≥ floor(T(w)/2)+1`,

`w'((A\{l})\B)+w'(l) ≥ floor(T(w')/2)+1`.

Take the old quorum to be `B∪{l}` and the new quorum to be `((A\{l})\B)∪{l}`. Conversely, from any singleton witness pair, place its old nonleader members in `B`; any unused responding identities can be added to either side because weights are nonnegative. Thus the condition is necessary and sufficient, not merely a counting heuristic. Replace `A` by the full identity universe `I` to obtain the purely abstract version. Phantom identities may appear in that abstract witness, but a promise recipient must belong to `A` if its response is required.

The canonical configuration in the construction has both quorum witnesses `{l}` and therefore has a responding casting vote. The connectivity theorem establishes that every permitted endpoint can reach this configuration while preserving quorum availability and cross-edge safety. It does not identify this property with having singleton witnesses at every intermediate boundary.

An equivalent algorithmic test is a two-dimensional subset-sum frontier over available nonleader identities: selecting an identity adds its old weight to the old side and removes its new weight from the new side. Since all coordinates are at most two, a dynamic program or exhaustive enumeration can directly emit actual witness identity sets for every accepted edge.

## Why leader choice belongs in the stronger theorem

The requirement to retain one specified leader is substantive even for fully responding identities. Consider `(l,a,b,c)=(1,2,2,2)`. The current strict threshold is four. Every quorum containing `l` must contain at least two of `{a,b,c}`. After any permitted unit increment/decrement, every new quorum containing `l` also needs at least two of those same three identities; a newly incremented zero-weight phantom contributes only one and does not change this fact. Doubling and zero-weight joins/leaves preserve the original quorum family. Exact halving is unavailable because the leader weight is odd. Consequently these particular elementary boundaries do not admit a singleton witness at `l`. A weight-two node, however, already has a casting vote in this same configuration: for leader `a`, the quorums `{a,b}` and `{a,c}` intersect exactly at `a`.

This example narrows the stronger theorem's assumptions; it is not a claim that the configuration cannot be reconfigured. The safe available path above still exists. Turner explicitly allows transferring leadership to a node with a casting vote.

## Source relationship

Turner's checked-in text at `research/uvrr-ladder/papers/upaxos-turner.txt`, around lines 370–389, defines the operational casting vote using “quorums of nonfailed nodes” and permits leader abdication to another node having such a vote. The cross-era majority intersection theorem itself concerns abstract identities and weights. These are different predicates and should remain separate in charts, tests, and theorem names.

## Phantom completion theorem: an abstract casting witness with a responding prepare side

Here the two witness roles are deliberately different. The prepare quorum must respond; the complementary quorum is an abstract identity-set witness. The following theorem establishes exactly that arrangement, without requiring hardware or responses from phantom identities.

Let `P ⊆ A` be any current strict majority containing `l`. Write `p=w(P)`, `h=w(l)≥1`, and `T=T(w)`. Add fresh unavailable identities with total voting weight

`δ = max(0, 2p - 2h + 1 - T)`.

Use weights at most two by distributing this mass over `ceil(δ/2)` fresh identities. Let `w*` be the completed profile, `T*=T+δ`, and define

`qPrepare = P`,

`qAbstract = (I* \ P) ∪ {l}`.

Then:

* `qPrepare ∩ qAbstract = {l}`.
* `qPrepare` is a strict majority under `w*`.
* `qAbstract` is a strict majority under `w*`.
* Every requested promise from `qPrepare\{l}` goes to a responding identity.
* The full responding set remains a strict majority during every intermediate phantom increment.

**Proof.** The chosen `δ` ensures

`2p - 2h < T* < 2p`.

The right inequality follows from the current majority `T<2p` when `δ=0`; otherwise `T*=2p-2h+1≤2p-1`. Hence `qPrepare` has weight `p>T*/2`. The other quorum has weight `T*-p+h`, and the left inequality is exactly `2(T*-p+h)>T*`. Their intersection follows from the set definitions. During construction total weight never exceeds `T*<2p≤2w(A)`, while responding weight is unchanged. Thus the responding majority remains available at every increment. Each increment also has the previously proved universal cross-edge intersection property.

For this chosen `P`, `δ` is the *smallest* nonnegative amount of added phantom mass allowing this complementary witness construction: the abstract quorum inequality requires precisely `T+δ>2p-2h`.

A simpler formula, which does not use the full leader weight, is `δ=2p-1-T`. It is always nonnegative because `P` was already a strict majority, and yields total `2p-1`. Taking `P=A` gives the particularly simple bound `δ=w(A)-w(I\A)-1`. The optimal formula can save phantom weight when `h>1`.

### Sharp bounds for weights zero, one, and two

Choose a responding majority containing the leader with minimum total weight `p`. If it contains another positive-weight identity `x`, minimality means removing `x` leaves weight at most `floor(T/2)`. Therefore

`p ≤ floor(T/2)+2`.

If it contains only the leader, the leader alone is already a quorum and `δ=0`. Otherwise substituting the bound in the optimal formula yields:

| Leader weight | Even total weight | Odd total weight |
|---|---:|---:|
| 1 | `δ≤3` | `δ≤2` |
| 2 | `δ≤1` | `δ=0` |

Thus **at most three phantom vote increments, spread over at most two fresh identities of weights at most two, always suffice** to obtain an abstract casting witness whose entire prepare side responds. This holds for every finite cluster size and every responding strict majority, with a positive retained leader.

The bound three is sharp. Let responding weights be `(leader=1, a=2, b=2)` and let one unavailable identity have weight one. Total weight is six. Every responding quorum containing the leader has weight five. The complementary abstract quorum has weight `2+δ`, so it is a strict majority exactly when `2(2+δ)>6+δ`, that is, `δ≥3`. A phantom of weight two and a second phantom of weight one attain the bound. The prepare quorum consists only of the three responding identities.

When all existing positive-weight identities respond, at most **two** increments suffice. The only case not already covered by the table is even `T` with leader weight one. Even total weight implies an even number of weight-one identities; there is therefore another responding weight-one identity. A multiset of weights one and two containing at least one one represents every integer between zero and its total. Choose nonleader weight exactly `T/2`, giving `p=T/2+1` and `δ=1`. The all-responding profile `(leader=1,2,2,2)` attains the bound two.

These bounds concern creation of the *final* abstract witness. Every construction increment preserves universal quorum intersection and responding-majority availability. A claim that each increment already has a specified-leader responding singleton pair is a different statement and is not used in this proof.

## Every unit edge has some abstract casting identity

There is also a general theorem for nonweightless endpoints without any bound on integer weights. Across an increment edge `w→w'`, choose an inclusion-minimal old strict-majority quorum `P`. Let `x` be the incremented identity. If `x∈P`, choose `l=x`; otherwise choose any `l∈P`. (An old zero-weight `x` cannot belong to an inclusion-minimal old quorum.) Minimality gives

`w(P\{l})≤floor(T(w)/2)`.

The increment does not affect `P\{l}`, by the choice of `l`. Therefore its complement `Q=I\(P\{l})` has new weight at least

`T(w)+1-floor(T(w)/2)> (T(w)+1)/2`.

So `P∈M(w)`, `Q∈M(w')`, and `P∩Q={l}`. Decrement edges follow by reversal. Thus *every unit increment/decrement boundary has an abstract singleton casting witness at some identity*. This need not be the same identity at all boundaries, nor must the selected identity and both quorums all respond. Doubling preserves the quorum family and is a separate case.
