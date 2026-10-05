//! The NOMINATE force-feed ladder (`docs/uvrr-protocols.md`, the NOMINATE chapter):
//! the solver's plans driven through the ordinary pipeline, one establishing
//! [`uvrr::configuration::SystemOperation::Batch`] entry at one slot, never a [`uvrr::wire::Tag::Fuse`] envelope, with the script
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
#[path = "support/nominate.rs"]
mod nominate;

use harness::{Harness, StepOutcome};
use nominate::{
    assert_constant_leader, committed_config, cross_boundary, member, n, serve_at, solver_steps,
};
use uvrr::configuration::{Configuration, Member, Snapshot, SystemOperation, Weight};
use uvrr::ids::{Ballot, Era, NodeId, Slot, View};

/// The replacement target: `old`'s seat and weight under the fresh
/// identity, the order otherwise unchanged.
fn replacement_target(start: &Configuration, old: NodeId, new: NodeId) -> Configuration {
    let mut snapshot = start.to_snapshot();
    for member in &mut snapshot.order {
        if member.node == old {
            member.node = new;
        }
    }
    snapshot.inflate().expect("the replacement target is legal")
}

/// Drives the solver's steps through the ordinary pipeline: the constant
/// leader proposes each step as ONE establishing [`uvrr::configuration::SystemOperation::Batch`] entry at one
/// slot (§8.7.4, never a [`uvrr::wire::Tag::Fuse`] envelope), the script feeds every released
/// message to its addressee, the leader is asserted at each committed
/// slot, and the era boundary where no rider landed is crossed by the
/// leader-preserving forced change.
fn drive_steps(
    h: &mut Harness,
    roster: &[NodeId],
    leader: NodeId,
    steps: &[Vec<SystemOperation>],
    scenario: &str,
) {
    for (index, step) in steps.iter().enumerate() {
        let outcome = h.reconfigure(leader, SystemOperation::Batch(step.clone()), None);
        assert!(
            matches!(outcome, StepOutcome::Published { .. }),
            "{scenario} step {index} proposes: {outcome:?}\n{}",
            h.trace_dump()
        );
        h.quiesce();
        let slot = Slot(h.snapshot(leader).expect("the leader is live").committed);
        assert_constant_leader(h, roster, leader, scenario, index, slot);
        h.assert_safety();
        cross_boundary(h, roster, leader, scenario);
    }
}

/// The expansion scenario: 3 to 5 at serving view 3, the leader constant at
/// `n(0)`. The solver's route joins each appended member at weight zero and
/// promotes it at its join, one era per establishing slot, every step's
/// rider carrying the re-electing offset the fold computed.
#[test]
fn expansion_keeps_the_leader_constant() {
    let mut h = Harness::provision(3);
    h.bootstrap();
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
    let steps = solver_steps(&start, &target, View(3));
    // The route: join, promote, join, promote — each appended member
    // joins at weight zero and is promoted at its join, the promotion the
    // wrap that carries the re-electing rider.
    assert_eq!(steps.len(), 4, "join, promote, join, promote");
    assert_eq!(
        steps
            .iter()
            .filter(|step| matches!(step.first(), Some(SystemOperation::Join { .. })))
            .count(),
        2,
        "the route joins both appended members"
    );
    assert_eq!(
        steps
            .iter()
            .filter(|step| matches!(step.first(), Some(SystemOperation::Increment(_))))
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
/// fresh identity's promotion and the old identity's drain, carry the
/// riders.
#[test]
fn replacement_keeps_the_leader_constant() {
    let mut h = Harness::provision(3);
    h.bootstrap();
    serve_at(&mut h, 3);

    let start = committed_config(&h, n(0));
    let target = replacement_target(&start, n(2), n(3));
    let live: Vec<NodeId> = (0..4).map(n).collect();
    let steps = solver_steps(&start, &target, View(3));
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
/// view 5 with the leader constant at `n(0)`. Every step carries its
/// rider: the bumped identity's promotion walks the voters 5 to 6, the
/// old identity's drain 6 back to 5.
#[test]
fn crash_reincarnation_replace_keeps_the_leader_constant() {
    let mut h = Harness::provision(5);
    h.bootstrap();
    h.force_view(
        n(0),
        Ballot {
            era: Era(1),
            view: View(5),
        },
    );
    h.quiesce();
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
    let target = replacement_target(&start, n(4), bumped);
    let steps = solver_steps(&start, &target, View(5));
    assert_eq!(
        steps.len(),
        4,
        "the five-node schedule: join, promote, drain, evict"
    );
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
