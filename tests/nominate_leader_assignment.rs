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
//! (the Green rung).

mod harness;

use harness::{Harness, StepOutcome};
use uvrr::configuration::{Configuration, Member, Snapshot, SystemOperation, Weight};
use uvrr::ids::{Era, NodeId, Slot, View, ViewId};
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
        ViewId {
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

/// Drives the solver's steps through the ordinary pipeline: the constant
/// leader proposes each step as ONE establishing `Batch` entry at one
/// slot (§8.7.4, never a `Fuse` envelope), the script feeds every released
/// message to its addressee, and the leader is asserted at each committed
/// slot.
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
/// them; the wraps are the drain of the first reseat and the promotions of
/// the last three.
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
    let steps = solve(&start, &target, &live).expect("the expansion solves");
    assert_eq!(
        steps
            .iter()
            .filter(|step | matches!(step.ops.as_slice(), [SystemOperation::Increment(_)]))
            .count(),
        4,
        "the route promotes all four reseated members"
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
/// serving view 3, the leader constant at `n(0)`. The wraps are the first
/// drain and the promotion of the fresh identity.
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
    let steps = solve(&start, &target, &live).expect("the replacement solves");
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
/// view 5 with the leader constant at `n(0)`. The wraps are the bumped
/// identity's promotion, voters 5 to 6, and the old identity's drain to
/// zero, voters 6 to 5.
#[test]
fn crash_reincarnation_replace_keeps_the_leader_constant() {
    let mut h = Harness::provision(5);
    bootstrap(&mut h);
    h.force_view(
        n(0),
        ViewId {
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
    let live: Vec<NodeId> = (0..4)
        .map(n)
        .chain(std::iter::once(bumped))
        .collect();
    let steps = solve_replacement(&start, n(4), bumped, &live).expect("the replacement solves");
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
