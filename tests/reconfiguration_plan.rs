//! The era planner and the batch era rule
//! (`docs/uvrr-reconfiguration-rules.md`).
//!
//! The corpus covers the §11 test matrix row by row: the batch fold's refusals
//! (R13–R15, the weight domain R1, the per-op boundaries R7–R12), the reduce-left
//! partitioner of §5 (the canonical splits: the resurrection four, the reordered
//! stream, the blog grids), the checked `Snapshot` constructor (§9), and the
//! equivalence "snapshot + WAL fold ≡ flat planned stream" (§9).
//!
//! Every batch folded here is committed as ONE era by the fold, and every
//! consecutive pair of committed configurations is checked against the closed
//! gate `vrr::quorum::validate_transition` — the exhaustive disjoint-pair search
//! the rules doc §8 names as the mechanical form of the intersection argument.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use vrr::configuration::{
    ConfigError, Configuration, INIT_SLOT, MAX_WEIGHT, Member, SystemOperation, VOID_SLOT, Weight,
};
use vrr::ids::{Era, NodeId, Slot};
use vrr::quorum::{QuorumStrategy, Role, WeightedMajority};

fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// `Join { node, position }` at the named succession position.
fn join(node: NodeId, position: u32) -> SystemOperation {
    SystemOperation::Join { node, position }
}

/// A genesis `(a, b, c)` unit cluster: the shape §4's worked examples start from.
fn fold_genesis() -> Configuration {
    let void = Configuration::void()
        .apply(&SystemOperation::Void, VOID_SLOT)
        .expect("Void at slot 1");
    void.apply(
        &SystemOperation::Init {
            order: vec![n(0), n(1), n(2)],
        },
        INIT_SLOT,
    )
    .expect("Init at slot 2")
}

/// Folds the flat `ops` stream onto `start`, one era per op, asserting every
/// fold accepts.
fn fold_from(start: &Configuration, ops: &[SystemOperation]) -> Configuration {
    let mut config = start.clone();
    let mut slot = Slot(100);
    for op in ops {
        slot = slot.next().expect("the slot space is not exhausted");
        config = config.apply(op, slot).expect("the step folds");
    }
    config
}

/// Folds `steps` (as batches, one era each) starting from `start` and returns
/// the committed sequence for the safety helpers above.
fn committed_sequence(
    start: &Configuration,
    steps: &[vrr::reconfiguration::EraStep],
) -> Vec<Configuration> {
    let mut configs = vec![start.clone()];
    let mut config = start.clone();
    let mut slot = Slot(100);
    for step in steps {
        slot = slot.next().expect("the slot space is not exhausted");
        config = config
            .apply(&SystemOperation::Batch(step.ops.clone()), slot)
            .expect("the planned batch folds");
        configs.push(config.clone());
    }
    configs
}

fn weights(config: &Configuration) -> Vec<u64> {
    config
        .order()
        .iter()
        .map(|member| u64::from(member.weight.0))
        .collect()
}

fn order(config: &Configuration) -> Vec<NodeId> {
    config.order().iter().map(|member| member.node).collect()
}

/// The members' `(node, weight)` pairs, in order.
fn shape(config: &Configuration) -> Vec<(NodeId, u32)> {
    config
        .order()
        .iter()
        .map(|member| (member.node, member.weight.0))
        .collect()
}

/// Every consecutive pair of the sequence is quorum-safe (the closed gate
/// accepts the transition), and the resulting era admits a commit quorum. This
/// is the rules doc §8's mechanical check over every boundary of the sequence.
fn assert_era_safe(steps: &[Configuration]) {
    for pair in steps.windows(2) {
        vrr::quorum::validate_transition(&WeightedMajority, &pair[0], &pair[1])
            .expect("the transition is era-safe");
        let voters: Vec<NodeId> = pair[1]
            .order()
            .iter()
            .filter(|member| member.weight.0 > 0)
            .map(|member| member.node)
            .collect();
        assert!(
            WeightedMajority.is_quorum(Role::Commit, &pair[1], &voters),
            "the resulting era admits a quorum"
        );
    }
}

/// Every weight of every configuration is inside the domain {0, 1, 2} (R1):
/// the closure property the whole safety argument rests on, asserted over
/// every committed era of a sequence.
fn assert_weight_domain(steps: &[Configuration]) {
    for config in steps {
        for member in config.order() {
            assert!(
                member.weight.0 <= MAX_WEIGHT,
                "{member:?} left the weight domain"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// R2, R4, R14: many zero-weight joins commit one era
// ---------------------------------------------------------------------------

/// Any number of learners may join in one era: each `Join` moves no mass
/// (`|0 − 0| = 0`), so R14 admits them without bound, and the quorum families
/// are unchanged (R4) — the exhaustive gate agrees. The planner keeps the whole
/// stream in ONE era for the same reason.
#[test]
fn many_zero_weight_joins_commit_one_era() {
    let genesis = fold_genesis();
    let joins = vec![join(n(3), 3), join(n(4), 4), join(n(5), 5), join(n(6), 6)];

    let batch = SystemOperation::Batch(joins.clone());
    let era = genesis
        .apply(&batch, Slot(3))
        .expect("zero-weight joins fold as one batch");
    assert_eq!(era.era(), Era(2), "one batch, one era");
    assert_eq!(
        weights(&era),
        vec![1, 1, 1, 0, 0, 0, 0],
        "every join is a learner (R2)"
    );
    assert_eq!(era.total(), genesis.total(), "learners move no mass (R14)");

    let steps = genesis.plan(&joins).expect("the stream plans");
    assert_eq!(steps.len(), 1, "the planner keeps the joins in one era");
    assert_eq!(steps[0].ops, joins);
    assert_era_safe(&committed_sequence(&genesis, &steps));
}

/// The per-op refusals of R11 still fire inside a batch, at the op's point in
/// the sequence: a join naming an already-present member is a duplicate.
#[test]
fn join_inside_a_batch_still_refuses_a_duplicate() {
    let genesis = fold_genesis();
    let batch = SystemOperation::Batch(vec![
        join(n(3), 3),
        SystemOperation::Join {
            node: n(3),
            position: 4,
        },
    ]);
    assert_eq!(
        genesis.apply(&batch, Slot(3)),
        Err(ConfigError::DuplicateNode(n(3))),
        "the second join names an already-present member (R11)"
    );
}

// ---------------------------------------------------------------------------
// R3, R14: learners coming and going, mixed, in one era
// ---------------------------------------------------------------------------

/// A learner may leave and learners may join in the same era: every op moves no
/// mass, so R14 admits the mixed batch, and R3 is satisfied because every
/// `Leave` names a weight-0 member.
#[test]
fn learners_come_and_go_in_one_era() {
    let genesis = fold_genesis();
    let learner = genesis
        .apply(&join(n(3), 3), Slot(3))
        .expect("the learner joins");
    assert_eq!(weights(&learner), vec![1, 1, 1, 0]);

    let mixed = SystemOperation::Batch(vec![
        SystemOperation::Leave(n(3)),
        join(n(4), 3),
        join(n(5), 4),
    ]);
    let era = learner
        .apply(&mixed, Slot(4))
        .expect("the mixed zero-mass batch folds");
    assert_eq!(era.era(), Era(3), "one batch, one era");
    assert_eq!(order(&era), vec![n(0), n(1), n(2), n(4), n(5)]);
    assert_eq!(weights(&era), vec![1, 1, 1, 0, 0]);
    assert_era_safe(&[genesis, learner.clone(), era]);

    let stream = vec![SystemOperation::Leave(n(3)), join(n(4), 3), join(n(5), 4)];
    let steps = learner.plan(&stream).expect("the stream plans");
    assert_eq!(
        steps.len(),
        1,
        "the mixed zero-mass stream plans to one era"
    );
}

// ---------------------------------------------------------------------------
// R14 mass 1: one unit change plus learners in one era
// ---------------------------------------------------------------------------

/// One unit weight change and any number of learner changes together move mass
/// exactly 1: legal in one era, and era-safe.
#[test]
fn one_unit_change_plus_learners_in_one_era() {
    let genesis = fold_genesis();
    let stream = vec![
        SystemOperation::Increment(n(0)),
        join(n(3), 3),
        join(n(4), 4),
    ];
    let steps = genesis.plan(&stream).expect("the stream plans");
    assert_eq!(steps.len(), 1, "mass 1 stays in one era");
    let configs = committed_sequence(&genesis, &steps);
    assert_eq!(weights(&configs[1]), vec![2, 1, 1, 0, 0]);
    assert_era_safe(&configs);
}

// ---------------------------------------------------------------------------
// R14 sharpness: two unit changes, and the zero-net identity swap
// ---------------------------------------------------------------------------

/// Two unit changes move mass 2 and are refused, naming the mass moved. Net
/// change alone would call this batch `+2` and refuse it for the wrong reason;
/// the refusal is the per-node mass, which is what R14 states.
#[test]
fn two_unit_changes_in_one_era_refused() {
    let genesis = fold_genesis();
    let batch = SystemOperation::Batch(vec![
        SystemOperation::Increment(n(0)),
        SystemOperation::Increment(n(1)),
    ]);
    assert_eq!(
        genesis.apply(&batch, Slot(3)),
        Err(ConfigError::BatchMassMoved { moved: 2 }),
        "R14: mass 2 > 1"
    );
}

/// The identity swap — `[DECREMENT(c), JOIN(d), INCREMENT(d), LEAVE(c)]` on
/// `(a:1, b:1, c:1)` — has net total change 0 yet moves mass 2, and its
/// endpoint's era-`e` majority `{b, c}` and era-`e+1` majority `{a, d}` are
/// disjoint (§4). The per-node mass is the checkable condition; the net total
/// would admit the swap it exists to forbid.
#[test]
fn identity_swap_refused_despite_zero_net() {
    let genesis = fold_genesis();
    let swap = vec![
        SystemOperation::Decrement(n(2)),
        join(n(3), 2),
        SystemOperation::Increment(n(3)),
        SystemOperation::Leave(n(2)),
    ];
    let batch = SystemOperation::Batch(swap.clone());
    assert_eq!(
        genesis.apply(&batch, Slot(3)),
        Err(ConfigError::BatchMassMoved { moved: 2 }),
        "R14 over per-node mass moved, not net total"
    );
}

// ---------------------------------------------------------------------------
// §5: the reduce-left partitioner
// ---------------------------------------------------------------------------

/// The resurrection four — `[DECREMENT(c), JOIN(d), INCREMENT(d), LEAVE(c)]` —
/// split into exactly two eras: after `DECREMENT(c), JOIN(d)` the mass moved is
/// 1, so the batch closes; the next era takes `INCREMENT(d), LEAVE(c)` (mass
/// 1). The canonical two-era form `(a:1,b:1,c:1) → (a:1,b:1,c:0,d:0) →
/// (a:1,b:1,d:1)` is what the partitioner produces, and every intermediate era
/// is quorum-safe.
#[test]
fn resurrection_four_split_into_exactly_two_eras() {
    let genesis = fold_genesis();
    let four = vec![
        SystemOperation::Decrement(n(2)),
        join(n(3), 2),
        SystemOperation::Increment(n(3)),
        SystemOperation::Leave(n(2)),
    ];
    let steps = genesis.plan(&four).expect("the stream plans");
    assert_eq!(steps.len(), 2, "the canonical two-era form");
    assert_eq!(
        steps[0].ops,
        vec![SystemOperation::Decrement(n(2)), join(n(3), 2)]
    );
    assert_eq!(
        steps[1].ops,
        vec![
            SystemOperation::Increment(n(3)),
            SystemOperation::Leave(n(2))
        ]
    );
    assert_eq!(weights(&steps[0].config), vec![1, 1, 0, 0]);
    assert_eq!(weights(&steps[1].config), vec![1, 1, 1]);
    assert_eq!(order(&steps[1].config), vec![n(0), n(1), n(3)]);

    let configs = committed_sequence(&genesis, &steps);
    assert_era_safe(&configs);
    assert_weight_domain(&configs);
}

/// The reorder attempt `(a:1,b:1,c:1) → (a:1,b:1,c:1,d:1) → (a:1,b:1,c:0,d:1)`
/// is not expressible as one legal batch (mass 2), and the alphabet has no
/// join-at-weight-one. The partitioner splits the reordered stream into its own
/// legal eras instead.
#[test]
fn reordered_stream_partitions_into_legal_eras() {
    let genesis = fold_genesis();
    let stream = vec![
        join(n(3), 3),
        SystemOperation::Increment(n(3)),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
    ];
    let steps = genesis.plan(&stream).expect("the stream plans");
    assert_eq!(steps.len(), 2);
    assert_eq!(weights(&steps[0].config), vec![1, 1, 1, 1]);
    assert_eq!(weights(&steps[1].config), vec![1, 1, 1]);
    assert_eq!(order(&steps[1].config), vec![n(0), n(1), n(3)]);
    assert_era_safe(&committed_sequence(&genesis, &steps));
}

/// An operation that is illegal even alone refuses the plan outright: the
/// partitioner closes nothing and invents no eras around a boundary the fold
/// refuses (R7 here: the member is already at the cap).
#[test]
fn an_op_illegal_alone_refuses_the_plan() {
    let genesis = fold_genesis();
    let doubled = genesis
        .apply(&SystemOperation::Double, Slot(3))
        .expect("Double");
    let stream = vec![
        SystemOperation::Increment(n(0)),
        SystemOperation::Increment(n(0)),
    ];
    assert_eq!(
        doubled.plan(&stream),
        Err(ConfigError::WeightCapExceeded {
            node: n(0),
            cap: MAX_WEIGHT
        }),
        "the second increment crosses R7 and no close can repair it"
    );
}

/// A refused stream leaves the receiver untouched: `plan` is a pure what-if,
/// and the committed configuration history is the only authority.
#[test]
fn a_refused_plan_does_not_mutate_the_receiver() {
    let genesis = fold_genesis();
    let stream = vec![
        SystemOperation::Increment(n(0)),
        SystemOperation::Increment(n(1)),
    ];
    let _ = genesis.plan(&stream);
    assert_eq!(shape(&genesis), vec![(n(0), 1), (n(1), 1), (n(2), 1)]);
}

// ---------------------------------------------------------------------------
// R13: DOUBLE/HALVE are solitary
// ---------------------------------------------------------------------------

/// A batch containing `Double` or `Halve` contains nothing else. Refused with
/// company on either side.
#[test]
fn scaling_ops_are_solitary_in_a_batch() {
    let genesis = fold_genesis();
    for batch in [
        SystemOperation::Batch(vec![join(n(3), 3), SystemOperation::Double]),
        SystemOperation::Batch(vec![
            SystemOperation::Double,
            SystemOperation::Increment(n(0)),
        ]),
        SystemOperation::Batch(vec![
            SystemOperation::Increment(n(0)),
            SystemOperation::Halve,
        ]),
    ] {
        assert_eq!(
            genesis.apply(&batch, Slot(3)),
            Err(ConfigError::ScalingNotSolitary),
            "{batch:?} mixes a scaling op with company (R13)"
        );
    }
    // Solitary scaling ops fold: exactly one op, exactly one era.
    let doubled = genesis
        .apply(
            &SystemOperation::Batch(vec![SystemOperation::Double]),
            Slot(3),
        )
        .expect("a solitary Double is its own legal batch");
    assert_eq!(weights(&doubled), vec![2, 2, 2]);
    assert_eq!(doubled.era(), Era(2));
    let halved = doubled
        .apply(
            &SystemOperation::Batch(vec![SystemOperation::Halve]),
            Slot(4),
        )
        .expect("a solitary Halve is its own legal batch");
    assert_eq!(weights(&halved), vec![1, 1, 1]);
}

/// A scaling op forces the next op into a new batch: the planner closes the
/// scaling era and starts the follower's era. The followers group with each
/// other, never with the scaling op.
#[test]
fn scaling_ops_split_off_alone_in_a_stream() {
    let genesis = fold_genesis();
    let steps = genesis
        .plan(&[
            join(n(3), 3),
            SystemOperation::Double,
            SystemOperation::Decrement(n(0)),
        ])
        .expect("the stream plans");
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0].ops, vec![join(n(3), 3)]);
    assert_eq!(steps[1].ops, vec![SystemOperation::Double]);
    assert_eq!(steps[2].ops, vec![SystemOperation::Decrement(n(0))]);
    assert_era_safe(&committed_sequence(&genesis, &steps));
}

// ---------------------------------------------------------------------------
// R7–R12: the per-op boundaries, one refusal per precondition
// ---------------------------------------------------------------------------

/// `Increment` at weight 2 refuses (R7): the member would leave the domain, and
/// the refusal names it and the cap. `Double` with any member at 2 refuses the
/// same way (R9), naming the first member that would pass 2.
#[test]
fn increment_and_double_refuse_at_the_weight_cap() {
    let genesis = fold_genesis();
    let doubled = genesis
        .apply(&SystemOperation::Double, Slot(3))
        .expect("unit weights double");
    assert_eq!(weights(&doubled), vec![2, 2, 2]);
    assert_eq!(
        doubled.apply(&SystemOperation::Increment(n(1)), Slot(4)),
        Err(ConfigError::WeightCapExceeded {
            node: n(1),
            cap: MAX_WEIGHT
        }),
        "R7: W(n) + 1 = 3 > 2"
    );
    assert_eq!(
        doubled.apply(&SystemOperation::Double, Slot(4)),
        Err(ConfigError::WeightCapExceeded {
            node: n(0),
            cap: MAX_WEIGHT
        }),
        "R9: the first member at 2 would pass the cap"
    );
    assert_eq!(
        doubled.apply(
            &SystemOperation::Batch(vec![SystemOperation::Increment(n(1))]),
            Slot(4),
        ),
        Err(ConfigError::WeightCapExceeded {
            node: n(1),
            cap: MAX_WEIGHT
        }),
        "the cap holds inside a batch too"
    );
}

/// `Halve` with an odd weight present refuses (R10), naming the first odd
/// member; nothing rounds.
#[test]
fn halve_refuses_an_odd_weight() {
    let genesis = fold_genesis();
    let odd = genesis
        .apply(&SystemOperation::Decrement(n(1)), Slot(3))
        .expect("Decrement makes N1 odd among evens");
    assert_eq!(weights(&odd), vec![1, 0, 1]);
    assert_eq!(
        odd.apply(&SystemOperation::Halve, Slot(4)),
        Err(ConfigError::OddWeight(n(0))),
        "R10: the first odd member is named"
    );
}

/// `Decrement` at weight 0 refuses (R8): a learner has no weight to give.
#[test]
fn decrement_refuses_at_zero() {
    let genesis = fold_genesis();
    let learner = genesis
        .apply(&join(n(3), 3), Slot(3))
        .expect("the learner joins");
    assert_eq!(
        learner.apply(&SystemOperation::Decrement(n(3)), Slot(4)),
        Err(ConfigError::WeightUnderflow(n(3))),
        "R8: W(n) = 0"
    );
}

/// `Leave` at any positive weight refuses (R12): the member still votes.
#[test]
fn leave_refuses_at_positive_weight() {
    let genesis = fold_genesis();
    assert_eq!(
        genesis.apply(&SystemOperation::Leave(n(1)), Slot(3)),
        Err(ConfigError::NonZeroWeight(n(1))),
        "R12: W(n) > 0"
    );
}

/// After any legal sequence — the grids, the resurrection eras — every weight
/// is still inside {0, 1, 2} (R1): the closure the domain argument rests on.
#[test]
fn weights_never_leave_the_domain() {
    for configs in [
        grid_one_configs(),
        grid_two_configs(),
        resurrection_configs(),
    ] {
        assert_weight_domain(&configs);
    }
}

// ---------------------------------------------------------------------------
// R15: genesis is not plannable
// ---------------------------------------------------------------------------

/// `Void` and `Init` never appear in a batch, a batch never nests a batch, and
/// an empty batch is not an era. The planner refuses genesis in the stream.
#[test]
fn genesis_and_nested_batches_are_refused() {
    let genesis = fold_genesis();
    assert_eq!(
        genesis.apply(&SystemOperation::Batch(vec![]), Slot(3)),
        Err(ConfigError::EmptyBatch)
    );
    assert_eq!(
        genesis.apply(
            &SystemOperation::Batch(vec![SystemOperation::Void]),
            Slot(3),
        ),
        Err(ConfigError::GenesisNotPlannable)
    );
    assert_eq!(
        genesis.apply(
            &SystemOperation::Batch(vec![SystemOperation::Init {
                order: vec![n(0), n(1)]
            }]),
            Slot(3),
        ),
        Err(ConfigError::GenesisNotPlannable)
    );
    assert_eq!(
        genesis.apply(
            &SystemOperation::Batch(vec![SystemOperation::Batch(vec![
                SystemOperation::Increment(n(0))
            ])]),
            Slot(3),
        ),
        Err(ConfigError::NestedBatch)
    );
    assert_eq!(
        genesis.plan(&[SystemOperation::Void]),
        Err(ConfigError::GenesisNotPlannable),
        "R15: genesis is not plannable in a stream either"
    );
    assert_eq!(
        genesis.plan(&[SystemOperation::Init { order: vec![n(0)] }]),
        Err(ConfigError::GenesisNotPlannable)
    );
}

// ---------------------------------------------------------------------------
// §7: the blog grids, row by row
// ---------------------------------------------------------------------------

/// Grid 1 — the unit hot swap. Op stream `JOIN(Z), INCREMENT(Z), DECREMENT(Y),
/// LEAVE(Y)` → eras `[JOIN(Z), INCREMENT(Z)]` then `[DECREMENT(Y), LEAVE(Y)]`.
/// Every row of the grid is one committed era, and every consecutive pair
/// passes the exhaustive gate.
#[test]
fn grid_one_is_reproduced_row_by_row() {
    let genesis = fold_genesis();
    let z = NodeId(3);
    let stream = vec![
        join(z, 3),
        SystemOperation::Increment(z),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
    ];
    let steps = genesis.plan(&stream).expect("the stream plans");
    assert_eq!(steps.len(), 2);
    let configs = committed_sequence(&genesis, &steps);

    // Row by row: (1,1,1,–) → (1,1,1,1) → (1,1,–,1).
    assert_eq!(weights(&configs[0]), vec![1, 1, 1]);
    assert_eq!(weights(&configs[1]), vec![1, 1, 1, 1]);
    assert_eq!(weights(&configs[2]), vec![1, 1, 1]);
    assert_eq!(order(&configs[2]), vec![n(0), n(1), z]);
    assert_era_safe(&configs);
    assert_weight_domain(&configs);
}

/// Grid 2 — the doubled-scale safe replacement. Op stream `DOUBLE, JOIN(Z),
/// INCREMENT(Z), DECREMENT(Y), DECREMENT(Y), LEAVE(Y), INCREMENT(Z), HALVE` →
/// eras `[DOUBLE]`, `[JOIN(Z), INCREMENT(Z)]`, `[DECREMENT(Y)]`,
/// `[DECREMENT(Y), LEAVE(Y)]`, `[INCREMENT(Z)]`, `[HALVE]`. The final `HALVE`
/// is integral because the joiner was driven to weight 2 first — the doubled
/// corner that makes the return to unit weights possible at all.
#[test]
fn grid_two_is_reproduced_row_by_row() {
    let genesis = fold_genesis();
    let z = NodeId(3);
    let stream = vec![
        SystemOperation::Double,
        join(z, 3),
        SystemOperation::Increment(z),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
        SystemOperation::Increment(z),
        SystemOperation::Halve,
    ];
    let steps = genesis.plan(&stream).expect("the stream plans");
    assert_eq!(steps.len(), 6, "the doubled grid splits into six eras");
    assert_eq!(steps[0].ops, vec![SystemOperation::Double]);
    assert_eq!(
        steps[1].ops,
        vec![join(z, 3), SystemOperation::Increment(z)]
    );
    assert_eq!(steps[2].ops, vec![SystemOperation::Decrement(n(2))]);
    assert_eq!(
        steps[3].ops,
        vec![
            SystemOperation::Decrement(n(2)),
            SystemOperation::Leave(n(2))
        ]
    );
    assert_eq!(steps[4].ops, vec![SystemOperation::Increment(z)]);
    assert_eq!(steps[5].ops, vec![SystemOperation::Halve]);

    let configs = committed_sequence(&genesis, &steps);
    // Row by row, exactly as published.
    let rows = [
        vec![1, 1, 1],
        vec![2, 2, 2],
        vec![2, 2, 2, 1],
        vec![2, 2, 1, 1],
        vec![2, 2, 1],
        vec![2, 2, 2],
        vec![1, 1, 1],
    ];
    for (row, config) in rows.iter().zip(configs.iter()) {
        assert_eq!(weights(config), *row, "grid row {row:?}");
    }
    assert_eq!(order(&configs[6]), vec![n(0), n(1), z]);
    assert_era_safe(&configs);
    assert_weight_domain(&configs);
}

/// Grid 1 and Grid 2 and the resurrection two-era form, folded once, for the
/// domain-closure and era-safety helpers.
fn grid_one_configs() -> Vec<Configuration> {
    let genesis = fold_genesis();
    let z = NodeId(3);
    let stream = vec![
        join(z, 3),
        SystemOperation::Increment(z),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
    ];
    let steps = genesis.plan(&stream).expect("the stream plans");
    committed_sequence(&genesis, &steps)
}

fn grid_two_configs() -> Vec<Configuration> {
    let genesis = fold_genesis();
    let z = NodeId(3);
    let stream = vec![
        SystemOperation::Double,
        join(z, 3),
        SystemOperation::Increment(z),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
        SystemOperation::Increment(z),
        SystemOperation::Halve,
    ];
    let steps = genesis.plan(&stream).expect("the stream plans");
    committed_sequence(&genesis, &steps)
}

fn resurrection_configs() -> Vec<Configuration> {
    let genesis = fold_genesis();
    let four = vec![
        SystemOperation::Decrement(n(2)),
        join(n(3), 2),
        SystemOperation::Increment(n(3)),
        SystemOperation::Leave(n(2)),
    ];
    let steps = genesis.plan(&four).expect("the stream plans");
    committed_sequence(&genesis, &steps)
}

// ---------------------------------------------------------------------------
// §9: the snapshot and the operation WAL
// ---------------------------------------------------------------------------

/// A configuration inflates to itself: the snapshot round trip through the
/// checked constructor preserves the era, the order and the weights.
#[test]
fn snapshot_inflates_to_the_configuration_it_serialized() {
    let genesis = fold_genesis();
    let doubled = genesis
        .apply(&SystemOperation::Double, Slot(3))
        .expect("Double");
    let with_learner = doubled
        .apply(&join(n(3), 3), Slot(4))
        .expect("the learner joins");
    for config in [genesis.clone(), doubled, with_learner] {
        let snapshot = config.to_snapshot();
        assert_eq!(snapshot.era, config.era());
        assert_eq!(snapshot.order, config.order());
        let inflated = snapshot.inflate().expect("the snapshot is legal");
        assert_eq!(inflated, config);
    }
}

/// Every invariant of §9 is re-checked at inflation, one named refusal per
/// precondition: the weight domain, duplicate-freeness, the era/void
/// correspondence, and the total floor.
#[test]
fn tampered_snapshots_are_refused_at_inflation() {
    // A weight outside {0, 1, 2}.
    let over = vrr::configuration::Snapshot {
        era: Era(1),
        order: vec![Member {
            node: n(0),
            weight: Weight(3),
        }],
    };
    assert_eq!(
        over.inflate(),
        Err(ConfigError::WeightCapExceeded {
            node: n(0),
            cap: MAX_WEIGHT
        })
    );

    // A duplicated identity.
    let duplicate = vrr::configuration::Snapshot {
        era: Era(1),
        order: vec![
            Member {
                node: n(0),
                weight: Weight(1),
            },
            Member {
                node: n(0),
                weight: Weight(1),
            },
        ],
    };
    assert_eq!(duplicate.inflate(), Err(ConfigError::DuplicateNode(n(0))));

    // A positive era with no members.
    let empty = vrr::configuration::Snapshot {
        era: Era(1),
        order: vec![],
    };
    assert_eq!(
        empty.inflate(),
        Err(ConfigError::SnapshotEmptyOrder { era: Era(1) })
    );

    // Era 0 carrying members: the void is empty by definition.
    let void_with_members = vrr::configuration::Snapshot {
        era: Era(0),
        order: vec![Member {
            node: n(0),
            weight: Weight(1),
        }],
    };
    assert_eq!(
        void_with_members.inflate(),
        Err(ConfigError::SnapshotVoidWithMembers)
    );

    // A positive era whose total is 0: quorum-impossible after commitment.
    let zero_total = vrr::configuration::Snapshot {
        era: Era(1),
        order: vec![Member {
            node: n(0),
            weight: Weight(0),
        }],
    };
    assert_eq!(
        zero_total.inflate(),
        Err(ConfigError::SnapshotZeroTotal { era: Era(1) })
    );

    // The void itself inflates: era 0, empty, total 0.
    let void = vrr::configuration::Snapshot {
        era: Era(0),
        order: vec![],
    };
    assert_eq!(void.inflate(), Ok(Configuration::void()));
}

/// The snapshot serializes (`serde` feature) and re-inflates only through the
/// checked constructor: byte round trip, and a tampered byte form is refused at
/// inflation, not at deserialization.
#[cfg(feature = "serde")]
#[test]
fn snapshot_serde_round_trip_and_tamper() {
    let genesis = fold_genesis();
    let with_learner = genesis
        .apply(&join(n(3), 3), Slot(3))
        .expect("the learner joins");
    let snapshot = with_learner.to_snapshot();

    let encoded = serde_json::to_string(&snapshot).expect("the snapshot serializes");
    let decoded: vrr::configuration::Snapshot =
        serde_json::from_str(&encoded).expect("the snapshot deserializes");
    assert_eq!(decoded, snapshot);
    assert_eq!(decoded.inflate(), Ok(with_learner));

    // Tampering with a member's weight deserializes fine — serde cannot check
    // domain membership — and the checked constructor refuses it.
    let mut tampered = snapshot.clone();
    tampered.order[0].weight = Weight(9);
    assert_eq!(
        tampered.inflate(),
        Err(ConfigError::WeightCapExceeded {
            node: n(0),
            cap: MAX_WEIGHT
        })
    );
}

/// Snapshot + WAL fold ≡ folding the flat planned stream (§9): the WAL is
/// exactly the sequence of legal eras the planner produced, and replay from an
/// inflated snapshot lands on the same membership the flat stream reaches.
#[test]
fn snapshot_plus_wal_fold_equals_the_flat_stream() {
    let genesis = fold_genesis();
    let z = NodeId(3);
    let stream = vec![
        SystemOperation::Double,
        join(z, 3),
        SystemOperation::Increment(z),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
        SystemOperation::Increment(z),
        SystemOperation::Halve,
    ];
    let steps = genesis.plan(&stream).expect("the stream plans");

    // The WAL: one Batch entry per era (§9).
    let wal: Vec<SystemOperation> = steps
        .iter()
        .map(|step| SystemOperation::Batch(step.ops.clone()))
        .collect();

    // Replay the WAL from the inflated snapshot: one Batch entry per era, one
    // apply per entry — the fold is the replay (§9).
    let restored = genesis
        .to_snapshot()
        .inflate()
        .expect("the snapshot is legal");
    assert_eq!(restored, genesis);
    let from_wal = fold_from(&restored, &wal);

    // The flat stream, one era per op.
    let flat = fold_from(&restored, &stream);

    // Same membership, same weights — the eras differ by design (the batch
    // collapses them), the state does not.
    assert_eq!(weights(&from_wal), weights(&flat));
    assert_eq!(order(&from_wal), order(&flat));
    assert_eq!(shape(&from_wal), shape(&flat));
    assert_eq!(from_wal.total(), flat.total());
    assert_eq!(
        from_wal.era(),
        Era(7),
        "six batches + the genesis two = era 7"
    );
    assert_eq!(
        flat.era(),
        Era(9),
        "eight singles + the genesis two = era 9"
    );
}

// ---------------------------------------------------------------------------
// The random-stream property: the planner's eras are legal, and the domain closes
// ---------------------------------------------------------------------------

proptest! {
    /// Any op stream either refuses (the planner invents no eras around a
    /// boundary) or partitions into eras each of which folds, whose every
    /// consecutive pair passes the exhaustive gate, and whose every weight
    /// stays inside {0, 1, 2} (R1's closure, exercised mechanically).
    #[test]
    fn planned_streams_fold_into_safe_in_domain_eras(
        ops in prop::collection::vec(op_strategy(), 0..8),
    ) {
        let genesis = fold_genesis();
        let steps = match genesis.plan(&ops) {
            Ok(steps) => steps,
            Err(_) => {
                return Err(TestCaseError::Reject(
                    "the stream refuses; nothing to assert".into(),
                ))
            }
        };
        let configs = committed_sequence(&genesis, &steps);
        assert_weight_domain(&configs);
        assert_era_safe(&configs);
        // The final state is what the flat fold reaches (the eras differ, the
        // membership does not).
        let flat = fold_from(&genesis, &ops);
        let last = configs.last().expect("the sequence always holds the start");
        prop_assert_eq!(weights(last), weights(&flat));
        prop_assert_eq!(order(last), order(&flat));
    }
}

fn op_strategy() -> impl Strategy<Value = SystemOperation> {
    // Weighted toward the legal majority: arithmetic over the member range,
    // joins over fresh identities. Refusals still happen — a member at the cap,
    // a leave at a positive weight — and those are exactly the streams the
    // property skips, but the accept rate keeps the case count meaningful.
    prop_oneof![
        3 => (0u32..3).prop_map(|id| SystemOperation::Increment(NodeId(id))),
        3 => (0u32..3).prop_map(|id| SystemOperation::Decrement(NodeId(id))),
        5 => (4u32..8).prop_map(|id| SystemOperation::Join {
            node: NodeId(id),
            position: 0,
        }),
        2 => (0u32..3).prop_map(|id| SystemOperation::Leave(NodeId(id))),
        1 => Just(SystemOperation::Double),
        1 => Just(SystemOperation::Halve),
    ]
}
