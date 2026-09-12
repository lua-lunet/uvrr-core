# Weighted reachability and phantom leader overlap

The voting-weight space has a constructive completion operation. For any finite
identity set, a live weighted majority and a positive live leader suffice to
construct an **abstract singleton leader overlap whose preparing quorum is entirely
live**. Phantom identities may have no hardware. They occur in the complementary
abstract witness, outside the promise recipients.

[Open the generated charts and tables](generated/report.html).

## General phantom construction

Let available weight be `A`, unavailable weight `D`, and the live leader's weight
be `l > 0`, with `A > D`. Join fresh zero-weight phantom identities, then give them
a total of `A-D-1` voting units through single increments, at most two per identity.

The resulting total is `2A-1`. The preparing quorum is the available set, of weight
`A`. Its abstract companion is the unavailable set (including phantoms) together
with the leader, of weight `A-1+l ≥ A`. Both are strict majorities, their only common
identity is the leader, and every required nonleader promise comes from an available
identity. Every prefix of the construction preserves the available majority.

For a chosen available majority containing the leader, let its weight be `p` and
let original total weight be `T`. The minimum added phantom weight is

`δ = max(0, 2p - 2l + 1 - T)`.

Choose a minimum-weight available majority containing the leader. With weights in
`{0,1,2}`, its weight is at most `floor(T/2)+2`. Therefore `δ ≤ 3`: **at most two
phantom identities suffice, for every finite cluster size**. This bound is sharp:
available weights `(leader=1, 2, 2)` and unavailable weight `1` need three added
units with that fixed leader and unchanged original weights. Phantom weights `2,1`
provide them. This minimum is among phantom-only additions; allowing other weight
changes may give another construction.

## Reachability between configurations

Put source and target on their finite union of identities, using zero for absent
members. Fix which identities can respond. Suppose those identities are a strict
weighted majority in both endpoints, and the retained live leader has positive
weight in both endpoints.

The following unit-edit path stays within `{0,1,2}`:

1. Decrease unavailable weights towards their target values.
2. Increase available weights towards their target values.
3. Decrease available weights towards their target values.
4. Increase unavailable weights towards their target values.

The first two phases improve the source's majority margin. In the final two,
available weight is at least its target value and unavailable weight is at most
its target value. Thus every intermediate state has a responding majority.
The path length is exactly `Σ|sourceᵢ-targetᵢ|`, the minimum for single-unit edits.
Every adjacent majority family intersects by the integer one-unit overlap lemma.
Zero-weight joins and leaves can be inserted; doubling and integral halving preserve
quorum families exactly.

The Lean connectivity proof uses a second constructive route through the
configuration having only the retained leader at weight one. Both routes establish
the same reachability result. The shortest-path claim is proved in the mathematical
note and checked by Python; it is not a claim about the length of the Lean path.

## Reproduce

The numerical checker uses only the Python standard library:

```sh
python3 research/weighted-reachability/check.py --max-n 10 --pair-max-n 6
```

Rendering uses Matplotlib 3.11.2; it is a research-tool dependency, not a Rust crate
dependency. From the repository root:

```sh
python3 -m venv .tmp/weighted-proof-venv
.tmp/weighted-proof-venv/bin/python -m pip install matplotlib==3.11.2
.tmp/weighted-proof-venv/bin/python research/weighted-reachability/check.py \
  --max-n 10 --pair-max-n 6 --render
cd formal/uvrr-lean
lake build
python3 check_axioms.py
```

The generated CSV files preserve the full tables behind the PNG and SVG charts.
`generated/summary.json` records exact coverage counts and negative controls.

## Evidence and interpretation

The finite checks cover every weight vector and availability count with a positive
live leader through ten existing identities: 310,007 representative profiles,
covering 14,541,815 labelled weight/availability profiles by permutation symmetry.
The leader is fixed at index zero; renaming covers other leader identities.
Every source/target pair satisfying the premises through six identities is checked:
774,159 representative pairs, covering 3,939,756 labelled pairs. All checks pass.
These bounded checks validate the constructions; the general proofs establish the
claims for arbitrary finite sizes.

The independent powerset check tests cross-era majority intersection on all generated
small-profile edges, including phantom joins; larger cases use the proved unit-edit
rule. Separate controls reject an unsafe two-vote jump, missing live majority,
excessive phantom weight and counting an abstract companion as a received quorum.
The checker also enumerates double/halve quorum equivalence.

The statements are about three precise objects: universal intersection of abstract
quorum families; available-majority paths through configuration space; and a final
abstract singleton companion to a fully responding preparing quorum. They do not
identify these with two simultaneous disjoint responding network rounds. A failure
to find one specific pair of live quorums in one fixed replacement table is not an
impossibility result over the configuration space.

See [the full mathematical proof](proof-analysis.md), [the source-semantics comparison](semantics.md),
and [the Lean module](../../formal/uvrr-lean/UVRR/WeightedReachability.lean).
