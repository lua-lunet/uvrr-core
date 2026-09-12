use proptest::prelude::*;
use vrr::configuration::{Configuration, Member, Snapshot, SystemOperation, Weight};
use vrr::ids::{Era, NodeId, Slot};
use vrr::quorum::{WeightedMajority, validate_transition};
use vrr::solver::{solve, solve_replacement};

fn config(weights: &[u32]) -> Configuration {
    Snapshot {
        era: Era(1),
        order: weights
            .iter()
            .enumerate()
            .map(|(i, &w)| Member {
                node: NodeId(i as u32),
                weight: Weight(w),
            })
            .collect(),
    }
    .inflate()
    .unwrap()
}
fn check(start: &Configuration, target: &Configuration, live: &[NodeId]) {
    let steps = solve(start, target, live).unwrap();
    let mut c = start.clone();
    for step in steps {
        let next = c
            .apply(&SystemOperation::Batch(step.ops), Slot(100))
            .unwrap();
        assert_eq!(next, step.config);
        validate_transition(&WeightedMajority, &c, &next).unwrap();
        let mass: u64 = next
            .order()
            .iter()
            .filter(|m| live.contains(&m.node))
            .map(|m| u64::from(m.weight.0))
            .sum();
        assert!(2 * mass > next.total());
        c = next;
    }
    assert_eq!(c.order(), target.order());
}
#[test]
fn constant_mass_swap_needs_intermediate_eras() {
    let s = config(&[1, 2, 1, 2]);
    let t = config(&[2, 1, 2, 1]);
    assert!(validate_transition(&WeightedMajority, &s, &t).is_err());
    let live = [NodeId(0), NodeId(1), NodeId(2), NodeId(3)];
    assert_eq!(solve(&s, &t, &live).unwrap().len(), 4);
    check(&s, &t, &live);
}
#[test]
fn six_nodes_three_datacentres_every_leader_and_reincarnation() {
    for leader in 0..6 {
        for killed in 0..6 {
            let start = config(&[1; 6]);
            let live: Vec<_> = (0..7).filter(|&n| n != killed).map(NodeId).collect();
            // A crashed leader is replaced by a surviving positive voter before planning.
            let elected = if leader == killed {
                (leader + 1) % 6
            } else {
                leader
            };
            assert!(live.contains(&NodeId(elected)));
            let mut target = start.to_snapshot();
            target.order[killed as usize].node = NodeId(6);
            let target = target.inflate().unwrap();
            check(&start, &target, &live);
            assert!(
                !solve_replacement(&start, NodeId(killed), NodeId(6), &live)
                    .unwrap()
                    .is_empty()
            );
            for dc in 0..3 {
                let survivors: u64 = target
                    .order()
                    .iter()
                    .filter(|m| {
                        let host = if m.node == NodeId(6) {
                            killed
                        } else {
                            m.node.0
                        };
                        host / 2 != dc
                    })
                    .map(|m| u64::from(m.weight.0))
                    .sum();
                assert!(2 * survivors > target.total());
            }
        }
    }
}
#[test]
fn replacement_preserves_full_standard_schedules() {
    for (n, expected) in [(3, 2), (5, 6)] {
        let start = config(&vec![1; n]);
        let live: Vec<_> = (1..=n as u32).map(NodeId).collect();
        let steps = solve_replacement(&start, NodeId(0), NodeId(n as u32), &live).unwrap();
        assert_eq!(steps.len(), expected);
        if n == 5 {
            assert_eq!(steps.first().unwrap().ops, vec![SystemOperation::Double]);
            assert_eq!(steps.last().unwrap().ops, vec![SystemOperation::Halve]);
        }
    }
}
#[test]
fn disjoint_full_memberships_and_order_are_reachable_at_cap() {
    let s = config(&[1; 16]);
    let mut t = s.to_snapshot();
    for (i, m) in t.order.iter_mut().enumerate() {
        m.node = NodeId(31 - i as u32);
    }
    check(
        &s,
        &t.inflate().unwrap(),
        &(0..32).map(NodeId).collect::<Vec<_>>(),
    );
}
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn arbitrary_weights_and_available_majorities(
        source in prop::collection::vec(0u32..=2, 6),
        target in prop::collection::vec(0u32..=2, 6),
        mask in 1u32..64,
    ) {
        let live: Vec<_> = (0..6).filter(|i| mask & (1 << i) != 0).map(NodeId).collect();
        let majority = |w: &[u32]| 2*w.iter().enumerate().filter(|(i,_)| mask & (1 << i) != 0).map(|(_,w)| w).sum::<u32>() > w.iter().sum();
        prop_assume!(majority(&source) && majority(&target));
        check(&config(&source), &config(&target), &live);
    }
}

#[test]
fn endpoint_availability_and_scaling_are_explicit() {
    let c = config(&[1, 1, 2, 2]);
    assert!(matches!(
        solve(&c, &c, &[NodeId(0), NodeId(1)]),
        Err(vrr::solver::SolveError::NoAvailableMajority {
            target: false,
            available: 2,
            total: 6
        })
    ));
    let s = config(&[1, 1, 1]);
    let t = config(&[2, 2, 2]);
    let live = [NodeId(0), NodeId(1), NodeId(2)];
    assert_eq!(
        solve(&s, &t, &live).unwrap()[0].ops,
        vec![SystemOperation::Double]
    );
    assert_eq!(
        solve(&t, &s, &live).unwrap()[0].ops,
        vec![SystemOperation::Halve]
    );
    let mut reversed = s.to_snapshot();
    reversed.order.reverse();
    check(&s, &reversed.inflate().unwrap(), &live);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn arbitrary_reincarnation_weights(
        weights in prop::collection::vec(0u32..=2, 6), killed in 0usize..6,
    ) {
        let total: u32 = weights.iter().sum();
        prop_assume!(2 * (total - weights[killed]) > total);
        let s = config(&weights);
        let live: Vec<_> = (0..7).filter(|&id| id != killed as u32).map(NodeId).collect();
        let steps = solve_replacement(&s, NodeId(killed as u32), NodeId(6), &live).unwrap();
        let mut c = s.clone();
        for step in steps {
            validate_transition(&WeightedMajority, &c, &step.config).unwrap();
            let mass: u64 = step.config.order().iter().filter(|m| live.contains(&m.node)).map(|m| u64::from(m.weight.0)).sum();
            prop_assert!(2 * mass > step.config.total());
            prop_assert_eq!(c.apply(&SystemOperation::Batch(step.ops), Slot(100)).unwrap(), step.config.clone());
            c = step.config;
        }
        let mut target = s.to_snapshot();
        target.order[killed].node = NodeId(6);
        prop_assert_eq!(c.order(), target.order.as_slice());
    }
}
