//! The plan format and the CLI surface: the plan type, its JSONL codec, and
//! the schedules `uvrr-reconfig plan` emits
//! (`docs/weighted-reconfiguration-solver.md`).
//!
//! The lib-level solver schedules are pinned in `tests/reconfiguration_solver.rs`
//! and `tests/reconfiguration_plan.rs`; what is pinned here is the plan artefact
//! itself — the paper's two-era and six-era replacement schedules expressed as
//! plan JSONL, the codec round trip, the leader-side acceptance rule, and the
//! `uvrr-reconfig` binary.

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use vrr::configuration::{Configuration, Member, Snapshot, SystemOperation, Weight};
use vrr::ids::{Era, NodeId};
use vrr::plan::{Plan, PlanRejection};
use vrr::solver::solve_replacement;

fn n(id: u32) -> NodeId {
    NodeId(id)
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

fn plan_replacement(start: &Configuration, old: NodeId, new: NodeId, live: &[NodeId]) -> Plan {
    let steps = solve_replacement(start, old, new, live).expect("the schedule solves");
    Plan {
        initial: start.order().to_vec(),
        steps: steps.into_iter().map(|step| step.ops).collect(),
    }
}

/// The header and step lines of the three-node two-era replacement plan, in the
/// JSONL schema of `docs/weighted-reconfiguration-solver.md`.
const THREE_NODE_PLAN: &str = concat!(
    "{\"kind\":\"plan\",\"version\":1,\"initial\":[{\"id\":0,\"weight\":1},",
    "{\"id\":1,\"weight\":1},{\"id\":2,\"weight\":1}],",
    "\"target\":[{\"id\":0,\"weight\":1},{\"id\":1,\"weight\":1},{\"id\":3,\"weight\":1}]}\n",
    "{\"kind\":\"step\",\"ops\":[{\"op\":\"decrement\",\"node\":2},",
    "{\"op\":\"join\",\"node\":3,\"position\":2}]}\n",
    "{\"kind\":\"step\",\"ops\":[{\"op\":\"increment\",\"node\":3},",
    "{\"op\":\"leave\",\"node\":2}]}\n",
);

/// The three-node unit cluster's replacement is exactly the paper's two-era
/// schedule: `[Decrement(old), Join(new)]` then `[Increment(new), Leave(old)]`.
#[test]
fn three_node_replacement_is_the_two_era_schedule() {
    let start = config(vec![m(0, 1), m(1, 1), m(2, 1)]);
    let live = vec![n(0), n(1), n(3)];
    let plan = plan_replacement(&start, n(2), n(3), &live);
    assert_eq!(
        plan.steps,
        vec![
            vec![
                SystemOperation::Decrement(n(2)),
                SystemOperation::Join {
                    node: n(3),
                    position: 2,
                },
            ],
            vec![
                SystemOperation::Increment(n(3)),
                SystemOperation::Leave(n(2)),
            ],
        ],
        "the paper's two-era replacement schedule"
    );
}

/// The five-node unit cluster's replacement is exactly the paper's six-batch
/// schedule.
#[test]
fn five_node_replacement_is_the_six_batch_schedule() {
    let start = config((0..5).map(|id| m(id, 1)).collect());
    let live: Vec<_> = (1..6).map(n).collect();
    let plan = plan_replacement(&start, n(4), n(5), &live);
    assert_eq!(
        plan.steps,
        vec![
            vec![SystemOperation::Double],
            vec![
                SystemOperation::Join {
                    node: n(5),
                    position: 4,
                },
                SystemOperation::Increment(n(5)),
            ],
            vec![SystemOperation::Decrement(n(4))],
            vec![
                SystemOperation::Decrement(n(4)),
                SystemOperation::Leave(n(4)),
            ],
            vec![SystemOperation::Increment(n(5))],
            vec![SystemOperation::Halve],
        ],
        "the paper's six-batch weighted replacement"
    );
}

/// The acceptance rule: a plan whose initial configuration differs from the
/// current committed one — wrong weights, wrong order, wrong membership — is
/// rejected by name.
#[test]
fn plans_drifted_from_the_committed_configuration_are_rejected_by_name() {
    let start = config(vec![m(0, 1), m(1, 1), m(2, 1)]);
    let live = vec![n(0), n(1), n(3)];
    let plan = plan_replacement(&start, n(2), n(3), &live);

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

/// A plan whose steps no longer fold — legal when computed, drift afterwards —
/// is refused at the step, not at the header: the initial matches, the fold
/// refuses.
#[test]
fn a_step_that_no_longer_folds_is_refused_at_the_step() {
    let start = config(vec![m(0, 1), m(1, 1), m(2, 1)]);
    // A legal plan on its own initial: decrement node 2, then promote node 1
    // while the drained node 2 leaves — every batch folds.
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
            refusal: vrr::configuration::ConfigError::WeightUnderflow(n(2)),
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
        let live: Vec<_> = (0..7)
            .filter(|&id| id != killed as u32)
            .map(n)
            .collect();
        let plan = plan_replacement(&start, n(killed as u32), n(6), &live);
        plan.validate_against(&start)
            .map_err(|error| TestCaseError::fail(format!("the fresh plan is refused: {error}")))?;
        let mut current = start.clone();
        for ops in &plan.steps {
            current = current
                .apply(
                    &SystemOperation::Batch(ops.clone()),
                    vrr::ids::Slot(0),
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
    use vrr::plan::{Plan, PlanCodecError};

    /// The three-node plan's JSONL is exactly the schema's two-era form, and
    /// the codec round-trips it to the same plan.
    #[test]
    fn the_three_node_plan_jsonl_is_exact_and_round_trips() {
        let start = config(vec![m(0, 1), m(1, 1), m(2, 1)]);
        let live = vec![n(0), n(1), n(3)];
        let plan = plan_replacement(&start, n(2), n(3), &live);
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

    /// The five-node plan's JSONL is a header plus six step lines, and the
    /// codec round-trips it.
    #[test]
    fn the_five_node_plan_jsonl_round_trips() {
        let start = config((0..5).map(|id| m(id, 1)).collect());
        let live: Vec<_> = (1..6).map(n).collect();
        let plan = plan_replacement(&start, n(4), n(5), &live);
        let text = plan.to_jsonl().expect("the plan serialises");
        assert_eq!(text.lines().count(), 7, "header + six step lines");
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

    /// The datagram budget: a five-node six-era plan is far inside the
    /// 60_000-byte submission limit, so the UDP submission is a real path, not
    /// a corner case.
    #[test]
    fn a_six_era_plan_is_far_inside_the_datagram_budget() {
        let start = config((0..5).map(|id| m(id, 1)).collect());
        let live: Vec<_> = (1..6).map(n).collect();
        let plan = plan_replacement(&start, n(4), n(5), &live);
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
            let live: Vec<_> = (0..7)
                .filter(|&id| id != killed as u32)
                .map(n)
                .collect();
            let plan = plan_replacement(&start, n(killed as u32), n(6), &live);
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

/// The CLI surface: the real binary, driven exactly as an operator drives it.
/// Without the `cli` feature there is no binary, and each test says so and
/// passes vacuously.
mod cli {
    use super::*;
    use std::net::UdpSocket;
    use std::process::{Command, Stdio};

    fn binary() -> Option<&'static str> {
        option_env!("CARGO_BIN_EXE_uvrr-reconfig")
    }

    /// A scratch directory for the script's files, unique to the test process.
    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "uvrr-reconfig-format-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("the scratch directory exists");
        dir
    }

    /// `plan --current --replace` prints the paper's two-era plan exactly.
    #[test]
    fn cli_plan_replace_prints_the_two_era_plan() {
        let Some(binary) = binary() else { return };
        let dir = scratch("plan");
        let members = dir.join("members.jsonl");
        std::fs::write(
            &members,
            "{\"id\":0,\"weight\":1}\n{\"id\":1,\"weight\":1}\n{\"id\":2,\"weight\":1}\n",
        )
        .expect("the member file is written");
        let output = Command::new(binary)
            .args(["plan", "--current"])
            .arg(&members)
            .args(["--replace", "2:3", "--available", "0,1,3"])
            .output()
            .expect("the CLI runs");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).expect("the plan is UTF-8"),
            THREE_NODE_PLAN,
            "the CLI prints the two-era plan exactly"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `plan --current --target` computes the general schedule, and its output
    /// is a plan the codec accepts.
    #[cfg(feature = "serde")]
    #[test]
    fn cli_plan_target_prints_a_valid_plan() {
        let Some(binary) = binary() else { return };
        let dir = scratch("target");
        let members = dir.join("members.jsonl");
        std::fs::write(
            &members,
            "{\"id\":0,\"weight\":1}\n{\"id\":1,\"weight\":1}\n{\"id\":2,\"weight\":1}\n",
        )
        .expect("the member file is written");
        let doubled = dir.join("doubled.jsonl");
        std::fs::write(
            &doubled,
            "{\"id\":0,\"weight\":2}\n{\"id\":1,\"weight\":2}\n{\"id\":2,\"weight\":2}\n",
        )
        .expect("the target file is written");
        let output = Command::new(binary)
            .args(["plan", "--current"])
            .arg(&members)
            .args(["--target"])
            .arg(&doubled)
            .args(["--available", "0,1,2"])
            .output()
            .expect("the CLI runs");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).expect("the plan is UTF-8");
        let decoded = Plan::from_jsonl(&text).expect("the CLI's plan parses");
        assert_eq!(
            decoded.steps,
            vec![vec![SystemOperation::Double]],
            "the exact global double is one era"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `apply` sends the header-prefixed plan as one datagram, prints the
    /// verdict, and exits 0 on `accepted`, 1 on `rejected`.
    #[test]
    fn cli_apply_prints_the_verdict_and_sets_the_exit_code() {
        let Some(binary) = binary() else { return };
        let dir = scratch("apply");
        let plan_path = dir.join("plan.jsonl");
        std::fs::write(&plan_path, THREE_NODE_PLAN).expect("the plan file is written");

        let listener = UdpSocket::bind("127.0.0.1:0").expect("the fake leader binds");
        let leader = format!("127.0.0.1:{}", listener.local_addr().expect("bound").port());

        for (response, expected_code, needle) in [
            (
                "{\"kind\":\"plan_response\",\"verdict\":\"accepted\"}",
                0,
                "accepted",
            ),
            (
                "{\"kind\":\"plan_response\",\"verdict\":\"rejected\",\"reason\":\"drifted\"}",
                1,
                "rejected: drifted",
            ),
        ] {
            let child = Command::new(binary)
                .args(["apply", "--plan"])
                .arg(&plan_path)
                .args(["--leader", &leader])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("the CLI runs");
            let mut buffer = [0u8; 65_536];
            let (received, source) = listener
                .recv_from(&mut buffer)
                .expect("the submission arrives");
            assert_eq!(
                String::from_utf8_lossy(&buffer[..received]),
                format!(
                    "{{\"kind\":\"plan_submit\",\"version\":1}}\n{}\n",
                    THREE_NODE_PLAN.trim_end()
                ),
                "the submission is the header line and the plan JSONL"
            );
            listener
                .send_to(response.as_bytes(), source)
                .expect("the verdict is sent");
            let output = child.wait_with_output().expect("the CLI exits");
            assert_eq!(output.status.code(), Some(expected_code), "{response}");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stdout.contains(needle) || stderr.contains(needle),
                "stdout: {stdout} stderr: {stderr}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
