# Rung 32: a scalar ballot encoding is sound exactly when it is strictly monotone

Evidence recorded 2 October 2026 against the source based on `aedd9e7`.

The target is `UVRR/BallotEncoding.lean`. Everywhere above rung 2 a ballot is
the pair `(era, view)` under the lexicographic order `bLt`. A host is not
obliged to carry the pair: it may carry one scalar — a byte packing, a scaled
integer, or an integer-valued IEEE binary64 — provided the encoding is
strictly monotone into a well-founded strict total order.
`encodedLt_iff_bLt` proves the pulled-back scalar comparison is then exactly
`bLt`, in both directions; the converse direction is where monotonicity is
load-bearing, since a colliding pair can recover neither trichotomy branch.
`Synod.order_encoded` and `Paxos.theorem10_encoded` discharge the order side
conditions of the agreement theorem for any such encoding, so the entire
ladder above rung 2 is representation-independent: the agreement proof does
not know whether the wire carried a pair or a scalar.

The concrete instances state the ruling's arithmetic. `pack_strictMono`: the
scaled packing `era * m + view` is strictly monotone in `bLt` for views below
the modulus, era dominating because a full view cycle is one era step.
`pack32_decides_bLt`: at modulus `2^32` the scalar comparison of two packings
decides `bLt` in both directions; the statement is per compared pair because
strict monotonicity holds only on the domain, which is what a host's
comparison site discharges. `pack_u32_below_f64_horizon`: with thirty-two
view bits every packing with era below `2^21` lands below `2^53`, the
binary64 exactness horizon, so a host comparing such packings as float
scalars decides the same order as `Nat` comparison; past the horizon the
representation rounds, and rounding is a collision. `collision_not_total` is
the negative control: on the era-blind encoding two distinct ballots share
one scalar, and the pulled-back relation cannot decide them — totality fails
at exactly the colliding pair, which is the shape of the float failure.

## Generation and verification

Hand-written, kernel-checked, no `sorry`. Two compile rounds against
`lake env lean`: the first round's failures were structural (rewriting a
structure's projection inside its own constructor goal, a `subst` on a pair
projection, a decidability synthesis on an unfolded definition, and an
`omega` recursion limit on the horizon literals); the landed proofs construct
the order fields field-wise under the transported equality, destruct the
pairs before substitution, close the control by `simp`, and prove the horizon
bound by a `Nat` calc chain with no solver. Direct queries:

```sh
printf '%s\n' 'import UVRR' \
  '#print axioms BallotEncoding.encodedLt_eq_bLt' \
  '#print axioms BallotEncoding.pack_strictMono' \
  '#print axioms BallotEncoding.pack_u32_below_f64_horizon' \
  '#print axioms BallotEncoding.collision_not_total' \
  '#print axioms BallotEncoding.Paxos.theorem10_encoded' \
  | lake env lean --stdin
```

Replay from `formal/uvrr-lean/`:

```sh
lake build
python3 check_axioms.py
```

Replay after integration completed successfully: 32 Lake jobs; the axiom
audit covered 539 declarations and found only the permitted standard Lean
axioms (`propext`, `Quot.sound`, `Classical.choice`).

The Rust corpus is untouched by this rung: the core carries the pair
(`src/ids.rs`, `Ballot { era, view }`, derived lexicographic `Ord`), and this
rung licenses a host scalar representation rather than installing one.

## Scope

The transfer theorem is order theory, not floating-point semantics: no IEEE
rounding is modelled in Lean. The horizon theorem is `Nat` arithmetic whose
conclusion is the bound a host must keep for a binary64 scalar to decide
`bLt`; inside the bound the encoding is injective and strictly monotone,
outside it the negative control's failure shape applies. The rung says
nothing about which encoding a deployment chooses, only the condition any
choice must meet.
