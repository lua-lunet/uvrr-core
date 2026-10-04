//! The plan format: the plan type, its JSONL codec, and the schedules the
//! solver emits (`docs/uvrr-protocols.md`, the solver chapter).
//!
//! The solver's law is pinned in `tests/solve_spec.rs`; what is pinned here
//! is the plan artefact itself: the replacement schedules the solver emits
//! expressed as plans, the codec round trip, and the leader-side acceptance
//! rule.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use uvrr::configuration::{Configuration, Member, Snapshot, SystemOperation, Weight};
use uvrr::ids::{CrashCounter, Era, NodeId, Slot, SystemId, View};
use uvrr::plan::{Plan, PlanRejection};
use uvrr::solve;

fn n(id: u32) -> NodeId {
    NodeId::new(
        SystemId::new((id + 1) as u16).expect("test system ids are small and non-zero"),
        CrashCounter::new(1).expect("one is non-zero"),
    )
}

fn m(id: u32, weight: u32) -> Member {
    Member {
        node: n(id),
        weight: Weight(weight),
    }
}

fn config(order: Vec<Member>) -> Configuration {
    Snapshot { era: Era(1), order }
        .inflate()
        .expect("the configuration is legal")
}

/// The operator's replacement plan: the target with `old`'s seat and
/// weight under the fresh identity, the exhaustive path folded at the
/// running view, one step per establishing slot with the fold's
/// `Nominate` riders minted (the NOMINATE chapter).
fn plan_replacement(start: &Configuration, old: NodeId, new: NodeId) -> Plan {
    let mut snapshot = start.to_snapshot();
    for member in &mut snapshot.order {
        if member.node == old {
            member.node = new;
        }
    }
    let target = snapshot.inflate().expect("the replacement target is legal");
    let path = solve::solve(start, &target).expect("the schedule solves");
    let steps = solve::fold(&path, start, (start.era(), View(0), Slot(0)))
        .expect("the fold rides a legal path");
    Plan {
        initial: start.order().to_vec(),
        steps: steps.into_iter().map(|step| step.ops).collect(),
    }
}

/// The header and step lines of the three-node two-era replacement plan, in the
/// JSONL schema of `docs/uvrr-protocols.md`, the solver chapter. The ids are the
/// lawful packed pairs: systems 1, 2, 3 at crash counter 1, and the
/// reincarnated identity is system 4's first life.
#[cfg(feature = "serde")]
const THREE_NODE_PLAN: &str = concat!(
    "{\"kind\":\"plan\",\"version\":1,\"initial\":[{\"id\":65537,\"weight\":1},",
    "{\"id\":131073,\"weight\":1},{\"id\":196609,\"weight\":1}],",
    "\"target\":[{\"id\":65537,\"weight\":1},{\"id\":131073,\"weight\":1},{\"id\":262145,\"weight\":1}]}\n",
    "{\"kind\":\"step\",\"ops\":[{\"op\":\"decrement\",\"node\":196609},",
    "{\"op\":\"join\",\"node\":262145,\"position\":2},{\"op\":\"nominate\",\"from\":0,\"offset\":2}]}\n",
    "{\"kind\":\"step\",\"ops\":[{\"op\":\"increment\",\"node\":262145},",
    "{\"op\":\"leave\",\"node\":196609},{\"op\":\"nominate\",\"from\":2,\"offset\":1}]}\n",
);

/// The three-node unit cluster's replacement is the per-slot schedule:
/// join the fresh identity, promote it, drain the old, evict it — one era
/// per establishing slot, every step carrying the fold's re-electing rider.
#[test]
fn three_node_replacement_is_the_per_slot_schedule() {
    let start = config(vec![m(0, 1), m(1, 1), m(2, 1)]);
    let plan = plan_replacement(&start, n(2), n(3));
    assert_eq!(
        plan.steps,
        vec![
            vec![
                SystemOperation::Join {
                    node: n(3),
                    position: 2,
                },
                SystemOperation::Nominate {
                    from: View(0),
                    offset: 3,
                },
            ],
            vec![
                SystemOperation::Increment(n(3)),
                SystemOperation::Nominate {
                    from: View(3),
                    offset: 1,
                },
            ],
            vec![
                SystemOperation::Decrement(n(2)),
                SystemOperation::Nominate {
                    from: View(4),
                    offset: 2,
                },
            ],
            vec![
                SystemOperation::Leave(n(2)),
                SystemOperation::Nominate {
                    from: View(6),
                    offset: 3,
                },
            ],
        ],
        "the per-slot replacement schedule, one era per establishing slot, \
         the riders chaining the running view across the plan"
    );
}

/// The five-node unit cluster's replacement is the per-slot schedule:
/// join, promote, drain, evict, one era per establishing slot.
#[test]
fn five_node_replacement_is_the_per_slot_schedule() {
    let start = config((0..5).map(|id| m(id, 1)).collect());
    let plan = plan_replacement(&start, n(4), n(5));
    assert_eq!(
        plan.steps,
        vec![
            vec![
                SystemOperation::Join {
                    node: n(5),
                    position: 4,
                },
                SystemOperation::Nominate {
                    from: View(0),
                    offset: 5,
                },
            ],
            vec![
                SystemOperation::Increment(n(5)),
                SystemOperation::Nominate {
                    from: View(5),
                    offset: 1,
                },
            ],
            vec![
                SystemOperation::Decrement(n(4)),
                SystemOperation::Nominate {
                    from: View(6),
                    offset: 4,
                },
            ],
            vec![
                SystemOperation::Leave(n(4)),
                SystemOperation::Nominate {
                    from: View(10),
                    offset: 5,
                },
            ],
        ],
        "the per-slot replacement schedule, one era per establishing slot, \
         the riders chaining the running view across the plan"
    );
}

/// The acceptance rule: a plan whose initial configuration differs from the
/// current committed one, wrong weights, wrong order, wrong membership, is
/// rejected by name.
#[test]
fn plans_drifted_from_the_committed_configuration_are_rejected_by_name() {
    let start = config(vec![m(0, 1), m(1, 1), m(2, 1)]);
    let plan = plan_replacement(&start, n(2), n(3));

    // Wrong weights: node 1 has moved to weight 2 since the plan was computed.
    let heavier = config(vec![m(0, 1), m(1, 2), m(2, 1)]);
    assert_eq!(
        plan.validate_against(&heavier),
        Err(PlanRejection::InitialWeight {
            node: n(1),
            plan: Weight(1),
            committed: Weight(2),
        }),
        "a drifted weight is refused by name"
    );

    // Wrong order: succession has changed.
    let reordered = config(vec![m(0, 1), m(2, 1), m(1, 1)]);
    assert!(
        matches!(
            plan.validate_against(&reordered),
            Err(PlanRejection::InitialMembership { .. })
        ),
        "a drifted succession order is refused by name"
    );

    // Wrong membership: node 2 has already left.
    let departed = config(vec![m(0, 1), m(1, 1)]);
    assert!(
        matches!(
            plan.validate_against(&departed),
            Err(PlanRejection::InitialMembership { .. })
        ),
        "a drifted membership is refused by name"
    );

    // The committed configuration the plan was computed against accepts it,
    // and every step folds.
    plan.validate_against(&start)
        .expect("the fresh plan is valid");
}

/// A plan whose steps no longer fold, legal when computed, drift afterwards,
/// is refused at the step, not at the header: the initial matches, the fold
/// refuses.
#[test]
fn a_step_that_no_longer_folds_is_refused_at_the_step() {
    let start = config(vec![m(0, 1), m(1, 1), m(2, 1)]);
    // A legal plan on its own initial: decrement node 2, then promote node 1
    // while the drained node 2 leaves, every batch folds.
    let legal = Plan {
        initial: start.order().to_vec(),
        steps: vec![
            vec![SystemOperation::Decrement(n(2))],
            vec![
                SystemOperation::Increment(n(1)),
                SystemOperation::Leave(n(2)),
            ],
        ],
    };
    legal.validate_against(&start).expect("the plan is valid");

    // The same plan with its second step replaced by a decrement of the node
    // the first step already drained: the initial still matches, the fold
    // refuses the step, by name and index.
    let illegal = Plan {
        initial: start.order().to_vec(),
        steps: vec![
            vec![SystemOperation::Decrement(n(2))],
            vec![SystemOperation::Decrement(n(2))],
        ],
    };
    assert_eq!(
        illegal.validate_against(&start),
        Err(PlanRejection::Step {
            index: 1,
            refusal: uvrr::configuration::ConfigError::WeightUnderflow(n(2)),
        }),
        "the fold refuses the drifted step at its index"
    );

    // The same legal plan against a configuration where node 2 already sits at
    // 0: the weights differ, so the header refuses first.
    let drained = config(vec![m(0, 1), m(1, 1), m(2, 0)]);
    assert_eq!(
        legal.validate_against(&drained),
        Err(PlanRejection::InitialWeight {
            node: n(2),
            plan: Weight(1),
            committed: Weight(0),
        }),
        "a drifted weight is refused before any step is considered"
    );
}

/// The target configuration of a replacement of `killed` by `n(6)`.
fn replacement_target(start: &Configuration, killed: usize) -> Vec<Member> {
    let mut target = start.to_snapshot();
    target.order[killed].node = n(6);
    target.order
}

// Every emitted plan validates against its own initial configuration, and
// replaying its steps reaches its target.
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn emitted_plans_replay_to_their_target(
        source in prop::collection::vec(0u32..=2, 6),
        killed in 0usize..6,
    ) {
        let total: u32 = source.iter().sum();
        prop_assume!(
            total > 0 && 2 * (total - source[killed]) > total,
            "the survivors keep a majority"
        );
        let start = config(
            source
                .iter()
                .enumerate()
                .map(|(id, &weight)| m(id as u32, weight))
                .collect(),
        );
        let plan = plan_replacement(&start, n(killed as u32), n(6));
        plan.validate_against(&start)
            .map_err(|error| TestCaseError::fail(format!("the fresh plan is refused: {error}")))?;
        let mut current = start.clone();
        for ops in &plan.steps {
            current = current
                .apply(
                    &SystemOperation::Batch(ops.clone()),
                    uvrr::ids::Slot(0),
                )
                .map_err(|error| TestCaseError::fail(format!("a step refuses: {error:?}")))?;
        }
        let target = replacement_target(&start, killed);
        prop_assert_eq!(
            current.order(),
            target.as_slice(),
            "the plan reaches its target"
        );
    }
}

/// The JSONL codec: available only with the `serde` feature (which carries
/// `serde_json`), so these tests are real in the feature lanes and absent from
/// the default build.
#[cfg(feature = "serde")]
mod jsonl {
    use super::*;
    use uvrr::plan::{Plan, PlanCodecError};

    /// The three-node plan's JSONL is exactly the schema's two-era form,
    /// and the codec round-trips it to the same plan. The schedule is the
    /// hand-written fixture, the codec the only thing under test.
    #[test]
    fn the_three_node_plan_jsonl_is_exact_and_round_trips() {
        let start = config(vec![m(0, 1), m(1, 1), m(2, 1)]);
        let plan = Plan {
            initial: start.order().to_vec(),
            steps: vec![
                vec![
                    SystemOperation::Decrement(n(2)),
                    SystemOperation::Join {
                        node: n(3),
                        position: 2,
                    },
                    SystemOperation::Nominate {
                        from: View(0),
                        offset: 2,
                    },
                ],
                vec![
                    SystemOperation::Increment(n(3)),
                    SystemOperation::Leave(n(2)),
                    SystemOperation::Nominate {
                        from: View(2),
                        offset: 1,
                    },
                ],
            ],
        };
        assert_eq!(
            plan.to_jsonl().expect("the plan serialises"),
            THREE_NODE_PLAN
        );
        let decoded = Plan::from_jsonl(THREE_NODE_PLAN).expect("the JSONL parses");
        assert_eq!(decoded, plan, "the codec round-trips");
        decoded
            .validate_against(&start)
            .expect("the round trip is valid");
    }

    /// The five-node plan's JSONL is a header plus one step line per
    /// establishing slot, and the codec round-trips it.
    #[test]
    fn the_five_node_plan_jsonl_round_trips() {
        let start = config((0..5).map(|id| m(id, 1)).collect());
        let plan = plan_replacement(&start, n(4), n(5));
        let text = plan.to_jsonl().expect("the plan serialises");
        assert_eq!(
            text.lines().count(),
            5,
            "header + one step line per establishing slot"
        );
        let decoded = Plan::from_jsonl(&text).expect("the JSONL parses");
        assert_eq!(decoded, plan, "the codec round-trips");
        decoded
            .validate_against(&start)
            .expect("the round trip is valid");
    }

    /// The codec refuses, by name: a non-header first line, an unsupported
    /// version, a misplaced kind, an operation outside the alphabet, and a
    /// declared target the steps do not reach.
    #[test]
    fn the_codec_refuses_malformed_plans_by_name() {
        let header = "{\"kind\":\"plan\",\"version\":1,\"initial\":[{\"id\":0,\"weight\":1}],\
                      \"target\":[{\"id\":0,\"weight\":1}]}";

        let bad_kind =
            Plan::from_jsonl("{\"kind\":\"step\",\"version\":1,\"initial\":[],\"target\":[]}");
        assert!(matches!(bad_kind, Err(PlanCodecError::Shape(_))));

        let bad_version =
            Plan::from_jsonl("{\"kind\":\"plan\",\"version\":2,\"initial\":[],\"target\":[]}");
        assert!(matches!(bad_version, Err(PlanCodecError::Shape(_))));

        let bad_step_kind =
            Plan::from_jsonl(&format!("{header}\n{{\"kind\":\"plan\",\"ops\":[]}}"));
        assert!(matches!(bad_step_kind, Err(PlanCodecError::Shape(_))));

        let bad_op = Plan::from_jsonl(&format!(
            "{header}\n{{\"kind\":\"step\",\"ops\":[{{\"op\":\"explode\",\"node\":0}}]}}"
        ));
        assert!(matches!(
            bad_op,
            Err(PlanCodecError::Syntax { line: 2, .. })
        ));

        // The declared target is not what the steps reach.
        let lying_target = "{\"kind\":\"plan\",\"version\":1,\
                            \"initial\":[{\"id\":0,\"weight\":1},{\"id\":1,\"weight\":1}],\
                            \"target\":[{\"id\":0,\"weight\":1},{\"id\":1,\"weight\":1}]}\n\
                            {\"kind\":\"step\",\"ops\":[{\"op\":\"increment\",\"node\":0}]}";
        assert!(matches!(
            Plan::from_jsonl(lying_target),
            Err(PlanCodecError::Target { .. })
        ));

        // A weight outside the domain {0, 1, 2} refuses at the perimeter.
        let over = "{\"kind\":\"plan\",\"version\":1,\
                    \"initial\":[{\"id\":0,\"weight\":9}],\
                    \"target\":[{\"id\":0,\"weight\":9}]}";
        assert!(matches!(
            Plan::from_jsonl(over),
            Err(PlanCodecError::Initial(_))
        ));
    }

    /// The datagram budget: a five-node replacement plan is far inside the
    /// 60_000-byte submission limit, so the single-datagram submission is a
    /// real path, not a corner case.
    #[test]
    fn a_replacement_plan_is_far_inside_the_datagram_budget() {
        let start = config((0..5).map(|id| m(id, 1)).collect());
        let plan = plan_replacement(&start, n(4), n(5));
        let text = plan.to_jsonl().expect("the plan serialises");
        let submission = "{\"kind\":\"plan_submit\",\"version\":1}".len() + 1 + text.len();
        assert!(submission < 60_000, "the submission fits one datagram");
    }

    // Every emitted plan parses through the codec and round-trips to the same
    // plan.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]
        #[test]
        fn emitted_plans_parse(
            source in prop::collection::vec(0u32..=2, 6),
            killed in 0usize..6,
        ) {
            let total: u32 = source.iter().sum();
            prop_assume!(
                total > 0 && 2 * (total - source[killed]) > total,
                "the survivors keep a majority"
            );
            let start = config(
                source
                    .iter()
                    .enumerate()
                    .map(|(id, &weight)| m(id as u32, weight))
                    .collect(),
            );
            let plan = plan_replacement(&start, n(killed as u32), n(6));
            let text = plan.to_jsonl().map_err(|error| {
                TestCaseError::fail(format!("the plan does not serialise: {error}"))
            })?;
            let decoded = Plan::from_jsonl(&text).map_err(|error| {
                TestCaseError::fail(format!("the emitted JSONL does not parse: {error}"))
            })?;
            prop_assert_eq!(decoded, plan.clone(), "the codec round-trips");
        }
    }
}
