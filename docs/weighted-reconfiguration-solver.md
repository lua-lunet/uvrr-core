# Weighted reconfiguration solver

`vrr::solver::solve(current, target, available)` returns committed-era batches
and their resulting configurations. It preserves a strict available majority
at every boundary and intersects consecutive strict-majority quorum families.
It uses only the existing operation alphabet and adds no dependencies.
The target's exact membership order and weights are honoured; its era is ignored.

For identical ordered identities, the solver decreases unavailable weights,
increases available weights, decreases available weights, then increases
unavailable weights. This takes the minimum number of unit edits. Exact global
doubling or halving takes one batch. For different identities or order, it uses
an available one-voter intermediate, transfers the vote to a live target member
when necessary, and constructs the target membership. This route is complete
within the membership cap but can reduce tolerance of additional failures.

`solve_replacement(current, old, new, available)` retains the old member's weight
and position under a fresh identity. It prefers the full standard schedule when
its intermediate configurations remain available and its endpoint matches:
two batches for three unit voters, six including doubling and halving for five.
Otherwise it uses the general constructor. Replan a partially committed request
with `solve` from the current configuration to the original target.

Run the operator tool from the repository:

```sh
cargo run --bin reconfiguration-solver -- 0:1,1:2,2:1,3:2 0:2,1:1,2:2,3:1 0,1,2,3
cargo run --bin reconfiguration-solver -- 0:1,1:1,2:1,3:1,4:1 --replace 0:5 1,2,3,4,5
```

Each row names the batch, weights and available mass. Submit each batch through
the normal reconfiguration protocol. Acquire state before promoting a learner.
Select a live, positive-weight leader using a quorum-backed view change when
needed; `next_view_selecting` can select a later view directly without polling
through each skipped view. A returned plan does not itself send or acknowledge
messages, commit configurations, or establish leadership. Availability is a
snapshot supplied by the operator: re-evaluate it when failures change.

For `(1,2,1,2) -> (2,1,2,1)`, total mass is unchanged but old quorum `{B,D}` and
new quorum `{A,C}` are disjoint. The solver increments A and C before decreasing
B and D, producing four safe boundaries. Failure tolerance is separate: weights
`(1,1,2,2)` split across two datacentres tolerate loss of the lighter datacentre,
but not the heavier one. The solver's guarantee is for `WeightedMajority`;
other policies must pass the core's role-specific gates independently.

Appendix 2 of `formal/uvrr-lean/paper/paper.tex` gives the proofs and test contract.
`research/weighted-reachability/` contains the independent Python enumeration,
charts and the phantom-identity casting-vote construction. The latter constructs
an abstract quorum witness without treating absent identities as received
promises. The Rust solver computes configuration paths; it does not currently
schedule these optional phantom witnesses or optimise datacentre resilience.
