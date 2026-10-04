//! The solver's spec, written clean-room against the alphabet's law.
//!
//! The alphabet is `Increment`, `Decrement`, `Double`, `Halve`, `Join`,
//! `Leave`, `Nominate`; legality is `Configuration::apply`'s; the search is
//! exhaustive; the fold computes the era and the view per slot and mints the
//! `Nominate` rider that keeps the leader stable across the reconfiguration.

use uvrr::configuration::{Configuration, INIT_SLOT, SystemOperation};
use uvrr::ids::{CrashCounter, Era, NodeId, Slot, SystemId, View};
use uvrr::solve::{self, Step};

/// A lawful identity: system id in the high half, the first life.
fn node(system: u16) -> NodeId {
    NodeId::new(
        SystemId::new(system).expect("the system id is non-zero"),
        CrashCounter::new(1).expect("the crash counter is non-zero"),
    )
}

/// A configuration built through the public fold: init at weight 1, then the
/// alphabet adjusts each member to its named weight.
fn configuration(members: &[(u16, u32)]) -> Configuration {
    let order: Vec<NodeId> = members.iter().map(|(system, _)| node(*system)).collect();
    let mut built = Configuration::void()
        .apply(&SystemOperation::Init { order }, INIT_SLOT)
        .expect("the init is legal");
    for (system, weight) in members {
        let node = node(*system);
        loop {
            let have = built.weight_of(node).expect("the member exists").0;
            if have == *weight {
                break;
            }
            let op = if have < *weight {
                SystemOperation::Increment(node)
            } else {
                SystemOperation::Decrement(node)
            };
            built = built.apply(&op, Slot(0)).expect("the adjustment is legal");
        }
    }
    built
}

/// The path is sound: every op applies legally in order and the result is the
/// target's shape, and the fold's eras strictly advance while its riders hold
/// the leader stable.
fn assert_legal_path(current: &Configuration, target: &Configuration) -> Vec<Step> {
    let path = solve::solve(current, target).expect("a legal path exists");
    let mut walking = current.clone();
    for op in &path {
        walking = walking.apply(op, Slot(0)).expect("every step is legal");
    }
    let shape = |c: &Configuration| -> Vec<(u32, u32)> {
        c.order().iter().map(|m| (m.node.0, m.weight.0)).collect()
    };
    assert_eq!(
        shape(&walking),
        shape(target),
        "the path reaches the target"
    );
    let steps = solve::fold(&path, current, (Era(1), View(0), Slot(10)))
        .expect("the fold rides a legal path");
    for (left, right) in steps.iter().zip(steps.iter().skip(1)) {
        assert!(
            right.era.0 == left.era.0 + 1,
            "the era advances at every establishing slot: {:?} then {:?}",
            left.era,
            right.era
        );
        assert!(right.view >= left.view, "views never go backwards");
    }
    let mut before = current.clone();
    for step in &steps {
        let after = before
            .apply(&step.payload(), step.slot)
            .expect("the slot's payload is legal");
        for op in &step.ops {
            if let SystemOperation::Nominate { from, offset } = op {
                let leader = before.primary(*from).expect("a leader exists");
                assert_eq!(
                    after.primary(View(from.0 + offset)),
                    Some(leader),
                    "the rider preserves the leader"
                );
            }
        }
        before = after;
    }
    steps
}

#[test]
fn three_members_gain_two_and_reach_five() {
    let steps = assert_legal_path(
        &configuration(&[(1, 1), (2, 1), (3, 1)]),
        &configuration(&[(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)]),
    );
    assert!(!steps.is_empty());
}

#[test]
fn five_members_shed_two_and_reach_three() {
    let steps = assert_legal_path(
        &configuration(&[(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)]),
        &configuration(&[(1, 1), (2, 1), (3, 1)]),
    );
    assert!(!steps.is_empty());
}

#[test]
fn a_member_is_replaced_in_place() {
    let steps = assert_legal_path(
        &configuration(&[(1, 1), (2, 1), (3, 1)]),
        &configuration(&[(1, 1), (2, 1), (4, 1)]),
    );
    assert!(!steps.is_empty());
}

#[test]
fn a_weight_raise_solves() {
    let steps = assert_legal_path(
        &configuration(&[(1, 1), (2, 1), (3, 1)]),
        &configuration(&[(1, 2), (2, 1), (3, 1)]),
    );
    assert_eq!(steps.len(), 1, "one unit edit");
}

#[test]
fn a_weight_above_the_domain_is_unrepresentable() {
    let config = configuration(&[(1, 2), (2, 1), (3, 1)]);
    let refused = config.apply(&SystemOperation::Increment(node(1)), Slot(0));
    assert!(
        refused.is_err(),
        "the domain is {{0, 1, 2}}: the fold refuses"
    );
}

#[test]
fn a_singleton_target_is_unreachable() {
    let path = solve::solve(
        &configuration(&[(1, 1), (2, 1), (3, 1)]),
        &configuration(&[(1, 1)]),
    );
    assert_eq!(path, None, "the cluster never drops below two members");
}

#[test]
fn the_exhaustive_grid_over_three_members_is_sound() {
    let weights = [1u32, 2u32];
    for a in weights {
        for b in weights {
            for c in weights {
                let current = configuration(&[(1, a), (2, b), (3, c)]);
                for d in weights {
                    for e in weights {
                        for f in weights {
                            let target = configuration(&[(1, d), (2, e), (3, f)]);
                            assert_legal_path(&current, &target);
                        }
                    }
                }
            }
        }
    }
}
