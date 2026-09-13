# Fuse: finite response-quorum telescoping

Evidence recorded 12 September 2026 against the source based on `562bfbf`.

The target is `UVRR/Fuse.lean`. `eligible_through` proves finite induction over
a fixed response set's quorum eligibility. `chosen_all` combines that with
acceptance of every specified slot by every responder. Its conclusion is
quorum-backed evidence, not an unstated converse of Paxos invariant P7.
`decide_all` explicitly takes the decision rule which consumes that evidence.
Concrete eligibility calculations include all three rows of the two-transition
three-voter schedule and all seven rows of the six-transition five-voter
schedule. Eligibility of the final row concerns subsequent traffic; it does
not add an extra command to either Fuse.

## Generation and verification

The actual repository driver was run with model `labs-leanstral-1-5-1`:

```sh
scripts/leanstral.sh prove formal/uvrr-lean/UVRR/Fuse.lean 2
```

Both generation rounds failed to discharge the target. The candidate and
compiler feedback were retained in `.tmp/fuse-proof/`. The proof bodies were
repaired locally; the original theorem signatures and imports were checked
against the saved skeleton. The successful result is the repaired file, not
an unassisted Leanstral success. Subsequent source review corrected the example
slot counts to two and six, retaining the final configuration eligibility
checks separately.

Replay from `formal/uvrr-lean/`:

```sh
lake build
python3 check_axioms.py
```

Replay after integration completed successfully: 27 Lake jobs; the axiom audit
covered 444 declarations and found only the permitted standard Lean axioms
(`propext`, `Quot.sound`, `Classical.choice`). The added
`same_ballot_era_guard` theorem also checks the bound imposed by the current
ordinary acceptor's ballot/entry-era rule; it does not waive that rule for Fuse.

The independent finite arithmetic check is:

```sh
python3 research/weighted-reachability/check_fuse.py
```

Its domains, counts and negative control are recorded in
`research/weighted-reachability/generated/fuse-summary.json`. The three-voter
domain covers both boundaries, and the five-voter domain covers all six.

## Scope

The proof assumes a responder's acknowledgement certifies all specified
acceptances. It does not prove the Rust receiver's transactional fold, storage
behaviour, or correspondence with the era-indexed protocol. Its first-quorum
negative control demonstrates why the quorum-preservation hypothesis is needed.
The LaTeX addendum explains how the solver can preserve a common response set,
and states the separate local-acceptance and era-history refinement obligations.
