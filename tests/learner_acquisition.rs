//! Learner acquisition (§10): a joined weight-0 member folds the era that
//! admitted it through its own fetch, stays fenced while its weight is 0,
//! and — only after a committed `INCREMENT` grants it weight —
//! participates in the era's quorum arithmetic.
//!
//! The gap this corpus pins: the establishing operation's fan-out reaches
//! configuration members only, the serving gate judged the sender against
//! the requested era's configuration, and the boot fence never moved the
//! committed frontier — so a fresh join could never fold its admitting era
//! and no fresh join converged. The learner acquisition rule
//! (`docs/uvrr-reincarnation.md` §10) closes it with the ordinary state
//! transfer: the leader serves a member of its current committed
//! configuration, and the boot-fenced node's own fetch is its qualified
//! evidence.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::SystemOperation;
use vrr::ids::{Era, NodeId, OperationId, View, ViewId};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
use vrr::wire::{Header, Tag};

fn n(id: u32) -> NodeId {
    NodeId(id)
}

fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

/// The timeout knob for the view-change scripts: a node suspects its
/// primary after more than three ticks of silence (S4).
const TIMEOUT: u64 = 3;

fn cluster() -> Harness {
    Harness::with_knobs(
        3,
        vrr::replica::ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    )
}

fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

fn snap(h: &Harness, id: NodeId) -> vrr::progress::ProgressSnapshot {
    h.snapshot(id).expect("the node is live")
}

fn current_view(h: &Harness, id: NodeId) -> ViewId {
    let snapshot = snap(h, id);
    ViewId {
        era: Era(snapshot.era),
        view: View(snapshot.view),
    }
}

fn status_of(h: &Harness, id: NodeId) -> Status {
    Status::from_word(snap(h, id).status).expect("the word is a status")
}

fn current_era(h: &Harness, id: NodeId) -> Era {
    h.era_table(id).expect("the node is live").current().era
}

/// The member weights of the node's current configuration, in order.
fn current_weights(h: &Harness, id: NodeId) -> Vec<u64> {
    h.era_table(id)
        .expect("the node is live")
        .current()
        .config
        .order()
        .iter()
        .map(|member| u64::from(member.weight.0))
        .collect()
}

/// The member identities of the node's current configuration, in order.
fn current_order(h: &Harness, id: NodeId) -> Vec<NodeId> {
    h.era_table(id)
        .expect("the node is live")
        .current()
        .config
        .order()
        .iter()
        .map(|member| member.node)
        .collect()
}

fn primary_of(h: &Harness, observer: NodeId, view: ViewId) -> Option<NodeId> {
    h.era_table(observer)?
        .record(view.era)
        .and_then(|record| record.config.primary(view.view))
}

/// The view-change target the fence machinery itself chooses for `node`:
/// the next view in the era the committed history has established
/// (§8.7.8). The primary of that view is whoever the position names.
fn fence_target(h: &Harness, node: NodeId) -> ViewId {
    let current = current_view(h, node);
    ViewId {
        era: current_era(h, node),
        view: View(current.view.0 + 1),
    }
}

/// Drives the view change the fence machinery targets, asserting every
/// node in `live` installs it.
fn drive_view_change(h: &mut Harness, live: &[NodeId]) -> (ViewId, NodeId) {
    let target = fence_target(h, live[0]);
    let driver = live
        .iter()
        .copied()
        .find(|&id| status_of(h, id) == Status::Normal)
        .expect("a live Normal member drives the fence");
    for _ in 0..=TIMEOUT {
        h.tick(driver);
    }
    h.deliver_all();
    for &id in live {
        assert_eq!(status_of(h, id), Status::Normal, "{id:?} installs");
        assert_eq!(current_view(h, id), target, "{id:?} is in the target");
    }
    h.assert_safety();
    (target, driver)
}

/// Joins `n(3)` at weight 0 (stop-the-world — no pivot exists for a
/// membership change), drives the ordinary view change into the era the
/// join committed, and runs the §10 learner acquisition to completion:
/// the StartView one era past the learner's boot table retains its offer
/// and fetches the missing range, the leader serves the learner, the
/// boot-fenced acquisition folds the admitting era, and the retained
/// offer installs on the next ordinary tick. Returns the harness with
/// the learner a caught-up, still vote-less member in the new era.
fn joined_and_caught_up(h: &mut Harness) -> ViewId {
    bootstrap(h);
    // The joining member boots over the deployment's genesis knowledge:
    // transport-addressable, fenced, holding the shared genesis and
    // nothing else.
    h.boot_as(n(3))
        .expect("the joiner boots over genesis knowledge");
    // The establishing Prepare reaches configuration members only; the
    // learner is a non-member at proposal time and receives nothing. The
    // commit folds era 2 at every incumbent.
    let outcome = h.reconfigure(
        n(0),
        SystemOperation::Join {
            node: n(3),
            position: 3,
        },
        None,
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(current_era(h, n(0)), Era(2), "the join committed");
    assert_eq!(current_order(h, n(0)), vec![n(0), n(1), n(2), n(3)]);
    assert_eq!(current_weights(h, n(0)), vec![1, 1, 1, 0]);
    assert_eq!(
        current_era(h, n(3)),
        Era(1),
        "the learner's table still holds only the genesis fold"
    );

    // The fence into era 2 announces the view to every member of the new
    // configuration — the learner included. The offer is one era past its
    // boot table: retained, fetched under the boot view, served, folded,
    // and installed on the next ordinary tick.
    let (target, _) = drive_view_change(h, &[n(0), n(1), n(2)]);
    assert_eq!(target.era, Era(2));
    assert_eq!(
        current_era(h, n(3)),
        Era(2),
        "the learner folded the era that admitted it"
    );
    assert_eq!(
        status_of(h, n(3)),
        Status::Recovering,
        "the acquisition runs at the boot fence, never voting"
    );
    h.tick(n(3));
    assert_eq!(
        status_of(h, n(3)),
        Status::Normal,
        "the learner is caught up"
    );
    assert_eq!(current_view(h, n(3)), target);
    assert_eq!(
        snap(h, n(3)).committed,
        snap(h, n(0)).committed,
        "the learner's frontier equals the leader's"
    );
    target
}

/// A joined learner folds the era that admitted it, catches up to the
/// leader's frontiers, and serves the same applied history — without ever
/// influencing a quorum while its weight is 0.
#[test]
fn joined_learner_folds_its_admitting_era_and_catches_up() {
    let mut h = cluster();
    joined_and_caught_up(&mut h);
    // The learner's journal is the leader's: the fetched range supplied
    // the establishing Join and the stream kept it current.
    let leader_committed = snap(&h, n(0)).committed;
    for slot in 1..=leader_committed {
        let (leader, learner) = (
            h.journal_entry(n(0), vrr::ids::Slot(slot)),
            h.journal_entry(n(3), vrr::ids::Slot(slot)),
        );
        assert_eq!(leader, learner, "slot {slot} matches the leader's");
    }
    h.assert_safety();
}

/// While its weight is 0 the learner cannot influence any quorum — its
/// acknowledgement is discarded, named, before counting, and a proposal
/// does not commit on the leader's and the learner's votes alone. After a
/// committed `INCREMENT` the same learner's vote is required: with one
/// voting member partitioned, the era-3 arithmetic (threshold 3 of total
/// 4) commits only when the promoted member votes.
#[test]
fn learner_cannot_influence_until_its_committed_increment_then_participates() {
    let mut h = cluster();
    let target = joined_and_caught_up(&mut h);
    // The primary of the era-2 view proposes the promotion. The learner's
    // vote — injected at the primary — is discarded by name: weight 0
    // counts against no quorum (R4), even for its own promotion.
    let outcome = h.reconfigure(n(1), SystemOperation::Increment(n(3)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    let promotion_slot = vrr::ids::Slot(snap(&h, n(1)).accepted);
    h.inject(
        n(3),
        n(1),
        Message {
            header: Header {
                tag: Tag::PrepareOk,
                view: target,
                slot: promotion_slot,
            },
            body: Body::PrepareOk {},
        },
    );
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::LearnerSender { sender: n(3) }),
        "the learner's vote is discarded by name"
    );
    // The voting members commit the promotion alone; era 3 folds.
    h.deliver_all();
    assert_eq!(current_era(&h, n(0)), Era(3), "the Increment committed");
    assert_eq!(current_weights(&h, n(0)), vec![1, 1, 1, 1]);
    assert_eq!(
        current_era(&h, n(3)),
        Era(3),
        "the promoted member folded the era that granted its weight"
    );

    // The ordinary view change into the era the promotion established:
    // the next view selects position 2 — n(2).
    let (era3_target, _) = drive_view_change(&mut h, &[n(0), n(1), n(2), n(3)]);
    assert_eq!(era3_target.era, Era(3));
    assert_eq!(primary_of(&h, n(0), era3_target), Some(n(2)));

    // One voting member is partitioned. The era-3 arithmetic is total 4,
    // threshold 3: the leader and one incumbent are not a quorum; the
    // promoted member's vote is the difference.
    h.partition(vec![n(1)], vec![n(0), n(2), n(3)]);
    let outcome = h.propose(n(2), op_id(3), b"z");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    // The incumbent's acknowledgement alone cannot commit: the leader
    // and one incumbent weigh 2 against a threshold of 3.
    h.deliver_to(n(0));
    h.deliver_to(n(2));
    let stalled = snap(&h, n(2)).committed;
    // The promoted member's acknowledgement completes it.
    h.deliver_to(n(3));
    h.deliver_to(n(2));
    assert!(
        snap(&h, n(2)).committed > stalled,
        "the promoted member's vote completes the era-3 quorum"
    );

    h.heal();
    h.drop_held();
    h.deliver_all();
    h.assert_safety();
}

/// The learner acquisition gate admits current-configuration members
/// only: a foreign identity is refused by name, on the same serving gate
/// the rule opened — and the refusal never faults the responder.
#[test]
fn the_serving_gate_still_refuses_a_non_member() {
    let mut h = cluster();
    joined_and_caught_up(&mut h);
    let request = Message {
        header: Header {
            tag: Tag::GetState,
            view: current_view(&h, n(0)),
            slot: vrr::ids::Slot(3),
        },
        body: Body::GetState {
            from: vrr::ids::Slot(5),
        },
    };
    let before = h.queued_len();
    h.inject(n(9), n(0), request);
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::UnknownSender { sender: n(9) }),
        "a node outside the current configuration is refused by name"
    );
    assert_eq!(h.queued_len(), before, "nothing was queued by the refusal");
    assert!(!snap(&h, n(0)).faulted, "a refusal, never a fault");
    h.assert_safety();
}
