//! The generated-stream property over the batch/era logic: random operation
//! streams against random in-domain three-node configurations, with the
//! TESTER's own oracle recomputing every applied era from first principles
//! (rules §5, §8; R1, R13, R14).
//!
//! Unlike `reconfiguration_plan.rs`'s corpus (hand-picked streams from the
//! genesis shape), this file generates both the starting configuration —
//! every node independently 0, 1 or 2, inflated through the checked
//! `Snapshot` constructor — and the operation stream, including streams
//! that are illegal (a `Leave` at a positive weight, a `Decrement` at 0, an
//! `Increment` at the cap, a `Double` with a 2 present, a `Halve` with an
//! odd weight, a batch moving more than mass 1, a nested batch). The
//! property must hold for refused AND accepted streams:
//!
//! 1. every weight of every applied era is inside {0, 1, 2}, integral,
//!    never negative (R1's closure);
//! 2. per applied era, the per-node mass moved Σ|Δw| over the union of
//!    touched nodes is ≤ 1, OR the era is a solitary scaling op — exactly
//!    the R13/R14 rule, recomputed here from the era's own weight tables;
//! 3. every admitted transition passes the exhaustive closed gate
//!    `vrr::quorum::validate_transition` (the mechanical intersection
//!    argument, an independent component);
//! 4. a refused stream leaves the configuration unchanged;
//! 5. for accepted streams, folding the planned era sequence and folding
//!    the flat stream op by op land on the same final weights, and each
//!    era's configuration is what its own ops produce folded step by step.

use std::collections::BTreeMap;

use proptest::prelude::*;
use proptest::test_runner::Config;

use vrr::configuration::{Configuration, Member, Snapshot, SystemOperation, Weight};
use vrr::ids::{Era, NodeId, Slot};
use vrr::quorum::WeightedMajority;
use vrr::reconfiguration::EraStep;

/// The three-node identity set every stream speaks about, plus the small
/// join pool that lets membership grow. The union stays far below
/// `MAX_MEMBERS`, so the exhaustive gate's subset scan is microseconds.
fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// A random in-domain starting configuration: three nodes, each weight
/// independently in {0, 1, 2}, inflated through the checked constructor so
/// the start itself is gate-validated (a nonzero total is required; the
/// filter only excludes the all-zero shape the constructor refuses).
fn arb_start() -> impl Strategy<Value = Configuration> {
    (0u32..3, 0u32..3, 0u32..3)
        .prop_filter("a positive era needs a nonzero total", |(a, b, c)| {
            a + b + c > 0
        })
        .prop_map(|(a, b, c)| {
            let order = [(a, n(0)), (b, n(1)), (c, n(2))]
                .into_iter()
                .map(|(weight, node)| Member {
                    node,
                    weight: Weight(weight),
                })
                .collect();
            Snapshot { era: Era(1), order }
                .inflate()
                .expect("in-domain weights inflate")
        })
}

/// One stream op: the six membership/weight verbs plus a raw op list inside
/// a batch (which the fold refuses as a nested batch — an illegal shape the
/// property must also hold for). The arithmetic verbs aim at the identity
/// set; joins and leaves may name the join pool too, so refusals arise
/// naturally at every boundary.
fn op_strategy() -> impl Strategy<Value = SystemOperation> {
    prop_oneof![
        3 => (0u32..3).prop_map(|id| SystemOperation::Increment(n(id))),
        3 => (0u32..3).prop_map(|id| SystemOperation::Decrement(n(id))),
        2 => (3u32..6, 0u32..6).prop_map(|(node, position)| SystemOperation::Join {
            node: n(node),
            position,
        }),
        2 => (0u32..6).prop_map(|id| SystemOperation::Leave(n(id))),
        1 => Just(SystemOperation::Double),
        1 => Just(SystemOperation::Halve),
        1 => prop::collection::vec(raw_op_strategy(), 1..4).prop_map(SystemOperation::Batch),
    ]
}

/// The verbs without the batch wrapper — the raw op list a batch carries.
fn raw_op_strategy() -> impl Strategy<Value = SystemOperation> {
    prop_oneof![
        3 => (0u32..3).prop_map(|id| SystemOperation::Increment(n(id))),
        3 => (0u32..3).prop_map(|id| SystemOperation::Decrement(n(id))),
        2 => (3u32..6, 0u32..6).prop_map(|(node, position)| SystemOperation::Join {
            node: n(node),
            position,
        }),
        2 => (0u32..6).prop_map(|id| SystemOperation::Leave(n(id))),
        1 => Just(SystemOperation::Double),
        1 => Just(SystemOperation::Halve),
    ]
}

fn stream_strategy() -> impl Strategy<Value = Vec<SystemOperation>> {
    prop::collection::vec(op_strategy(), 0..8)
}

/// The node → weight table, recomputed from the public member list — the
/// oracle's own view of a configuration, no planner internals involved.
fn weight_table(config: &Configuration) -> BTreeMap<NodeId, u32> {
    config
        .order()
        .iter()
        .map(|member| (member.node, member.weight.0))
        .collect()
}

fn weights(config: &Configuration) -> Vec<u32> {
    config.order().iter().map(|m| m.weight.0).collect()
}

fn order(config: &Configuration) -> Vec<NodeId> {
    config.order().iter().map(|m| m.node).collect()
}

/// Property 1: every weight of every applied era is inside {0, 1, 2} —
/// integral and non-negative by the `u32` type, capped by R1 here.
fn assert_domain(config: &Configuration) {
    for weight in weights(config) {
        assert!(
            weight <= vrr::configuration::MAX_WEIGHT,
            "weight {weight} left the domain {{0, 1, 2}}: {config:?}"
        );
    }
}

/// The nodes an op touches: the op's own member, or every member for the
/// whole-configuration scaling ops.
fn touched(op: &SystemOperation, config: &Configuration) -> Vec<NodeId> {
    match op {
        SystemOperation::Increment(node)
        | SystemOperation::Decrement(node)
        | SystemOperation::Leave(node) => vec![*node],
        SystemOperation::Join { node, .. } => vec![*node],
        SystemOperation::Double | SystemOperation::Halve => {
            config.order().iter().map(|m| m.node).collect()
        }
        SystemOperation::Void | SystemOperation::Init { .. } | SystemOperation::Batch(_) => {
            Vec::new()
        }
    }
}

/// Property 2, recomputed from first principles: the per-node mass moved
/// across the era boundary, Σ|Δw| over the union of touched nodes, is ≤ 1 —
/// OR the era is a solitary scaling op (exactly the R13/R14 rule). Absent
/// members count as weight 0: a `Join` adds a 0-weight node, a `Leave`
/// removes one, both move no mass.
fn assert_mass_rule(prev: &Configuration, next: &Configuration, ops: &[SystemOperation]) {
    let before = weight_table(prev);
    let after = weight_table(next);
    let mut union: Vec<NodeId> = ops.iter().flat_map(|op| touched(op, prev)).collect();
    union.sort();
    union.dedup();

    let solitary_scaling =
        ops.len() == 1 && matches!(ops[0], SystemOperation::Double | SystemOperation::Halve);
    if solitary_scaling {
        return;
    }

    let mut moved = 0u64;
    for node in union {
        let w_before = before.get(&node).copied().unwrap_or(0);
        let w_after = after.get(&node).copied().unwrap_or(0);
        moved += (i64::from(w_after) - i64::from(w_before)).unsigned_abs();
    }
    assert!(
        moved <= 1,
        "era moved mass {moved} (> 1) over the touched nodes, {before:?} → {after:?} via {ops:?}"
    );
}

/// Property 3: the exhaustive closed gate accepts the transition.
fn assert_quorum_safe(prev: &Configuration, next: &Configuration) {
    vrr::quorum::validate_transition(&WeightedMajority, prev, next).unwrap_or_else(|error| {
        panic!("transition {prev:?} → {next:?} failed the gate: {error:?}")
    });
}

/// Folds `ops` one op per `apply` from `start`, returning the final weights
/// and order. Every op of an accepted plan must fold step by step — the
/// batch never legalized anything the flat fold cannot.
fn fold_flat(start: &Configuration, ops: &[SystemOperation]) -> (Vec<u32>, Vec<NodeId>) {
    let mut config = start.clone();
    for (i, op) in ops.iter().enumerate() {
        config = config
            .apply(op, Slot(1_000 + i as u64))
            .unwrap_or_else(|error| {
                panic!("flat fold refused {op:?} at step {i}: {error:?}");
            });
    }
    (weights(&config), order(&config))
}

proptest! {
    #![proptest_config(Config {
        cases: 512,
        ..Config::default()
    })]

    /// The generated-stream property: every stream, refused or accepted,
    /// obeys the oracle above.
    #[test]
    fn generated_streams_obey_the_era_oracle(
        start in arb_start(),
        ops in stream_strategy(),
    ) {
        assert_domain(&start);
        let untouched = start.clone();

        let steps: Vec<EraStep> = match start.plan(&ops) {
            Ok(steps) => steps,
            Err(_) => {
                // Property 4: a refusal invents no eras and mutates nothing.
                // `plan` takes the receiver by shared reference, so the
                // equality is the contract stated in type form.
                assert_eq!(start, untouched, "a refused stream mutated the receiver");
                return Ok(());
            }
        };

        // The accepted path: walk the era sequence, prev → era config.
        let mut prev = start.clone();
        let mut era_sequence = vec![prev.clone()];
        for (i, step) in steps.iter().enumerate() {
            // Property 1: the domain closes over every applied era.
            assert_domain(&step.config);
            // Property 2: R13/R14, recomputed from the weight tables.
            assert_mass_rule(&prev, &step.config, &step.ops);
            // Property 3: the exhaustive gate on the admitted transition.
            assert_quorum_safe(&prev, &step.config);
            // Property 5 (per era): the era's configuration is what its own
            // ops produce folded step by step.
            let (folded_weights, folded_order) = fold_flat(&prev, &step.ops);
            prop_assert_eq!(folded_weights, weights(&step.config), "era {} weights", i);
            prop_assert_eq!(folded_order, order(&step.config), "era {} order", i);
            era_sequence.push(step.config.clone());
            prev = step.config.clone();
        }

        // Property 5 (composition): the planned era sequence and the flat
        // op-by-op fold land on the same final weights and order.
        let (flat_weights, flat_order) = fold_flat(&start, &ops);
        let last = era_sequence.last().expect("the sequence holds the start");
        prop_assert_eq!(flat_weights, weights(last), "final weights");
        prop_assert_eq!(flat_order, order(last), "final order");
    }
}
