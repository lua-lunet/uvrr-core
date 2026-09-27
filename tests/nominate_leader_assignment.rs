//! The NOMINATE force-feed ladder (`docs/nominate-leader-assignment.md`):
//! the solver's plans driven through the ordinary pipeline, one establishing
//! `Batch` entry at one slot, never a `Fuse` envelope, with the script
//! feeding every released message to its addressee, and the leader asserted
//! at every committed slot of every scenario.
//!
//! The leader a node computes is the configuration its own committed
//! history has established, evaluated at its current view number: the
//! arithmetic that slot's commit switched to, asked for the node's view.
//! The constant-leader invariant: at every committed slot, every live
//! voter names the same node, the leader the plan started under. Without
//! the nominations the modulo rule moves the answer mid-plan (the Red
//! rung); the solver's emission and the commit-time bump keep it constant
//! (the Green rung). Where no rider landed, the era boundary the §8.7.8
//! gate guards is crossed by the leader-preserving forced change (§14.2),
//! the same view number the rider would have named.

mod harness;

use harness::{Harness, StepOutcome};
use uvrr::configuration::{Configuration, Member, Snapshot, SystemOperation, Weight};
use uvrr::ids::{Ballot, Era, NodeId, Slot, View, next_view_selecting};
use uvrr::reconfiguration::EraStep;
use uvrr::solver::{solve, solve_replacement};

fn n(id: u32) -> NodeId {
    NodeId::new(
        uvrr::ids::SystemId::new((id + 1) as u16).expect("test system ids are small and non-zero"),
        uvrr::ids::CrashCounter::new(1).expect("one is non-zero"),
    )
}

fn member(id: u32, weight: u32) -> Member {
    Member {
        node: n(id),
        weight: Weight(weight),
    }
}

/// Bootstraps the cluster: the genesis primary promotes itself and the
/// backups adopt view (1, 0) from the promotion's `Commit` announcement
/// (§13.3).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    quiesce(h);
    h.assert_safety();
}

/// Delivers everything until the network is empty: proposals, cascades
/// and announcements are all ordinary steps, and a committed cascade may
/// queue more.
fn quiesce(h: &mut Harness) {
    while h.queued_len() > 0 {
        h.deliver_all();
    }
}

/// Forces the cluster to the scenario's serving view (§14.2): the ordinary
/// fence/evidence/install exchange driven through the host input, with the
/// scenario's constant leader as the target's primary.
fn serve_at(h: &mut Harness, view: u32) {
    h.force_view(
        n(0),
        Ballot {
            era: Era(1),
            view: View(view),
        },
    );
    quiesce(h);
    for id in [n(0), n(1), n(2)] {
        let snapshot = h.snapshot(id).expect("the node is live");
        assert_eq!(
            (snapshot.era, snapshot.view),
            (1, view),
            "n{} serves at the forced view",
            id.0
        );
    }
    h.assert_safety();
}

/// The leader the node's own committed history names at its view number.
fn committed_leader(h: &Harness, id: NodeId) -> Option<NodeId> {
    let snapshot = h.snapshot(id).expect("the node is live");
    h.era_table(id)
        .expect("the node is live")
        .current()
        .config
        .primary(View(snapshot.view))
}

/// The constant-leader assertion: at the slot that just committed, every
/// live member whose own committed configuration gives it positive weight
/// names the same leader.
fn assert_constant_leader(
    h: &Harness,
    roster: &[NodeId],
    leader: NodeId,
    scenario: &str,
    step: usize,
    slot: Slot,
) {
    for id in roster {
        let Some(snapshot) = h.snapshot(*id) else {
            continue;
        };
        let table = h.era_table(*id).expect("the node is live");
        let voting = table
            .current()
            .config
            .weight_of(*id)
            .is_some_and(|weight| weight.0 > 0);
        if !voting {
            continue;
        }
        let named = committed_leader(h, *id);
        assert_eq!(
            named,
            Some(leader),
            "{scenario} step {step} (committed slot {:?}): n{} names {named:?} at view {}, the constant leader is n{}",
            slot.0,
            id.0,
            snapshot.view,
            leader.0
        );
    }
}

/// The era boundary where no rider landed (§14.2, §8.7.8): the view lags
/// the established era and the gate would refuse the next establishing
/// operation, so the script drives the leader-preserving forced change,
/// the least view past the current one selecting the constant leader
/// under the era's voters, the same number the rider the wrap would have
/// carried names.
fn cross_boundary(h: &mut Harness, roster: &[NodeId], leader: NodeId, scenario: &str) {
    let snapshot = h.snapshot(leader).expect("the leader is live");
    let table = h.era_table(leader).expect("the leader is live");
    let established = table.current().era;
    if Era(snapshot.era) == established {
        return;
    }
    let voters: Vec<NodeId> = table
        .current()
        .config
        .order()
        .iter()
        .filter(|member| member.weight.0 > 0)
        .map(|member| member.node)
        .collect();
    let index = voters
        .iter()
        .position(|&voter| voter == leader)
        .expect("the constant leader is a voter of the established era");
    let target = next_view_selecting(
        View(snapshot.view),
        u32::try_from(index).expect("the roster fits the view arithmetic"),
        u32::try_from(voters.len()).expect("the roster fits the view arithmetic"),
    )
    .expect("a view selecting the leader always exists past any view");
    h.force_view(
        leader,
        Ballot {
            era: established,
            view: target,
        },
    );
    quiesce(h);
    for id in roster {
        let Some(live) = h.snapshot(*id) else {
            continue;
        };
        // The fenced non-member does not cross: the exchange installs at
        // the era's members, and a standby outside the configuration is
        // the §10 acquisition's business, not the boundary's.
        let voting = h
            .era_table(*id)
            .expect("the node is live")
            .current()
            .config
            .weight_of(*id)
            .is_some_and(|weight| weight.0 > 0);
        if !voting {
            continue;
        }
        assert_eq!(
            (live.era, live.view),
            (established.0, target.0),
            "{scenario}: n{} crossed the boundary into the established era",
            id.0
        );
    }
    assert_constant_leader(h, roster, leader, scenario, usize::MAX, Slot(0));
    h.assert_safety();
}

/// Drives the solver's steps through the ordinary pipeline: the constant
/// leader proposes each step as ONE establishing `Batch` entry at one
/// slot (§8.7.4, never a `Fuse` envelope), the script feeds every released
/// message to its addressee, the leader is asserted at each committed
/// slot, and the era boundary where no rider landed is crossed by the
/// leader-preserving forced change.
fn drive_steps(
    h: &mut Harness,
    roster: &[NodeId],
    leader: NodeId,
    steps: &[EraStep],
    scenario: &str,
) {
    for (index, step) in steps.iter().enumerate() {
        let outcome = h.reconfigure(leader, SystemOperation::Batch(step.ops.clone()), None);
        assert!(
            matches!(outcome, StepOutcome::Published { .. }),
            "{scenario} step {index} proposes: {outcome:?}\n{}",
            h.trace_dump()
        );
        quiesce(h);
        let slot = Slot(h.snapshot(leader).expect("the leader is live").committed);
        assert_constant_leader(h, roster, leader, scenario, index, slot);
        h.assert_safety();
        cross_boundary(h, roster, leader, scenario);
    }
}

/// The committed configuration the cluster's history has established.
fn committed_config(h: &Harness, id: NodeId) -> Configuration {
    h.era_table(id)
        .expect("the node is live")
        .current()
        .config
        .as_ref()
        .clone()
}

/// The expansion scenario: 3 to 5 at serving view 3, the leader constant at
/// `n(0)`. The solver's route drains and leaves the members to be reseated,
/// joins the target order at weight zero around the anchor, and promotes
/// them; the moving wraps, the first drain and the promotions of the last
/// three, carry the re-electing riders.
#[test]
fn expansion_keeps_the_leader_constant() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);
    serve_at(&mut h, 3);

    let start = committed_config(&h, n(0));
    let target = Snapshot {
        era: Era(1),
        order: [
            member(0, 1),
            member(1, 1),
            member(2, 1),
            member(3, 1),
            member(4, 1),
        ]
        .into(),
    }
    .inflate()
    .expect("the target is a legal configuration");
    let live: Vec<NodeId> = (0..5).map(n).collect();
    let steps = solve(&start, &target, &live, View(3)).expect("the expansion solves");
    // The user's route (`docs/nominate-leader-assignment.md`): the current
    // membership stays as the target's prefix, each appended member joins
    // at weight zero and is promoted at its join, the joiner's promotion
    // the wrap that carries the re-electing rider.
    assert_eq!(steps.len(), 4, "join, promote, join, promote");
    assert_eq!(
        steps
            .iter()
            .filter(|step| matches!(step.ops.first(), Some(SystemOperation::Join { .. })))
            .count(),
        2,
        "the route joins both appended members"
    );
    assert_eq!(
        steps
            .iter()
            .filter(|step| matches!(step.ops.first(), Some(SystemOperation::Increment(_))))
            .count(),
        2,
        "the route promotes both appended members"
    );
    drive_steps(&mut h, &live, n(0), &steps, "expansion");

    let final_config = committed_config(&h, n(0));
    assert_eq!(
        final_config.order(),
        &[
            member(0, 1),
            member(1, 1),
            member(2, 1),
            member(3, 1),
            member(4, 1)
        ],
        "the plan reaches the unit-weight five"
    );
}

/// The three-node replacement scenario: `n(2)` replaced by `n(3)` at
/// serving view 3, the leader constant at `n(0)`. The moving wraps, the
/// first drain and the fresh identity's promotion, carry the riders.
#[test]
fn replacement_keeps_the_leader_constant() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);
    serve_at(&mut h, 3);

    let start = committed_config(&h, n(0));
    let target = Snapshot {
        era: Era(1),
        order: [member(0, 1), member(1, 1), member(3, 1)].into(),
    }
    .inflate()
    .expect("the target is a legal configuration");
    let live: Vec<NodeId> = (0..4).map(n).collect();
    let steps = solve(&start, &target, &live, View(3)).expect("the replacement solves");
    drive_steps(&mut h, &live, n(0), &steps, "replacement");

    let final_config = committed_config(&h, n(0));
    assert_eq!(
        final_config.order(),
        &[member(0, 1), member(1, 1), member(3, 1)],
        "the old identity is evicted and the fresh one seated"
    );
}

/// The five-node crash-reincarnation replace: `n(4)` crashes, its bumped
/// life reopens, and the solver's replacement schedule runs at serving
/// view 5 with the leader constant at `n(0)`. The moving wraps, the bumped
/// identity's promotion, voters 5 to 6, and the old identity's drain to
/// zero, voters 6 to 5, carry the riders; the scaling steps stay solitary.
#[test]
fn crash_reincarnation_replace_keeps_the_leader_constant() {
    let mut h = Harness::provision(5);
    bootstrap(&mut h);
    h.force_view(
        n(0),
        Ballot {
            era: Era(1),
            view: View(5),
        },
    );
    quiesce(&mut h);
    for id in 0..5 {
        let snapshot = h.snapshot(n(id)).expect("the node is live");
        assert_eq!((snapshot.era, snapshot.view), (1, 5));
    }
    h.assert_safety();

    h.crash(n(4));
    let bumped = n(4)
        .next_life()
        .expect("the test never exhausts the counter");
    h.restart_as(n(4), bumped)
        .expect("the bumped life reopens fenced");

    let start = committed_config(&h, n(0));
    let live: Vec<NodeId> = (0..4).map(n).chain(std::iter::once(bumped)).collect();
    let steps =
        solve_replacement(&start, n(4), bumped, &live, View(5)).expect("the replacement solves");
    assert_eq!(steps.len(), 6, "the five-node schedule is the forced six");
    drive_steps(&mut h, &live, n(0), &steps, "crash-reincarnation");

    let final_config = committed_config(&h, n(0));
    assert_eq!(
        final_config.order(),
        &[
            member(0, 1),
            member(1, 1),
            member(2, 1),
            member(3, 1),
            Member {
                node: bumped,
                weight: Weight(1)
            }
        ],
        "the bumped identity is seated in the old identity's seat"
    );
}
