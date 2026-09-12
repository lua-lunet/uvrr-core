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
cargo run --features cli --bin uvrr-reconfig -- plan --current members.jsonl --replace 2:3 --available 0,1,3
cargo run --features cli --bin uvrr-reconfig -- apply --plan plan.jsonl --leader 10.0.0.1:9000
```

Acquire state before promoting a learner.
Select a live, positive-weight leader using a quorum-backed view change when
needed; `next_view_selecting` can select a later view directly without polling
through each skipped view. A returned plan does not itself send or acknowledge
messages, commit configurations, or establish leadership. Availability is a
snapshot supplied by the operator: re-evaluate it when failures change.

## Reconfiguration plans

A plan is the dumb-operator artefact: compute it once, submit it, and the leader
steps through it while the cluster keeps running normally. `vrr::plan::Plan`
is the core's serde-free form — the initial membership (order is succession)
and one batch of operations per era, in commit order.

The plan travels as JSONL, one JSON object per line: a header line, then one
step line per era.

```
{"kind":"plan","version":1,"initial":[{"id":0,"weight":1},{"id":1,"weight":1},{"id":2,"weight":1}],"target":[{"id":0,"weight":1},{"id":1,"weight":1},{"id":3,"weight":1}]}
{"kind":"step","ops":[{"op":"decrement","node":2},{"op":"join","node":3,"position":2}]}
{"kind":"step","ops":[{"op":"increment","node":3},{"op":"leave","node":2}]}
```

`target` is the membership the steps reach. The operation vocabulary is exactly
the existing `SystemOperation` alphabet — `increment`, `decrement`, `double`,
`halve`, `join` (with `position`), `leave` — each naming `node` where the
operation has one. JSON is parsed once at the tool perimeter into the
serde-free `Plan`; the core never sees JSON. The codec refuses, on the way in,
an initial membership that is not a legal configuration, any step the fold
refuses, and a declared `target` that is not the configuration the steps reach.

The leader acceptance rule: the leader rejects any plan whose `initial`
configuration is not its current committed configuration — membership,
succession order and weights are all compared — and rejects any step the
configuration fold refuses (`vrr::plan::Plan::validate_against`). A plan that
was legal when computed but has drifted is rejected, not committed; replan
from the current configuration to the original target.

An accepted plan is executed by the leader's plan-execution machine: one step
per era, each proposed through the ordinary reconfiguration gates, until the
last step commits and the machine clears. A step the gates refuse — the
cluster changed underneath the plan — aborts the machine with `PlanAborted`
and the operator re-plans from the configuration that committed. The plan
arrives on the leader's dedicated admin ingress, and the host polls that
ingress BEFORE the regular client queue on every selection
(`docs/architecture.md` §Host obligations): reconfigurations are rare, so the
poll is usually empty, but a plan never waits behind client traffic. The core
side is the `Input::SubmitPlan` input and the `Effect::AdminResponse` verdict
effect.

`plan` computes the schedule with the solver and writes plan JSONL to stdout
or `--out`. `apply` sends the plan as ONE UDP datagram to the leader's admin
port, prefixed by the header line `{"kind":"plan_submit","version":1}`; a
serialised submission larger than 60 000 bytes is refused locally. The
verdict is one JSON line, `{"kind":"plan_response","verdict":"accepted"}` or
`{"kind":"plan_response","verdict":"rejected","reason":"..."}`; `apply` waits
ten seconds for it, prints it, and exits 0 on `accepted`, 1 on `rejected`.
Availability is a snapshot supplied by the operator: re-evaluate it when
failures change.

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
