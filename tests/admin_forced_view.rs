//! Protocol suite for the host-forced view change (§14.2).
//!
//! The host forces entry into a chosen view; the core drives the
//! ORDINARY fence/evidence/install pipeline into it, so the new primary
//! is exactly the member the target's view number names under the
//! current membership order — no state is installed from the host's
//! say-so. The target must strictly advance the view within the current
//! era: a non-advancing target is bad input, an era other than the
//! current one is not the membership order the forcing maps under (an
//! uncommitted era's establishing operation was never decided), and the
//! last representable view has no successor (§8.7.3 forbids wraparound).
//!
//! The properties pinned here:
//!
//! 1.  a forced change installs the chosen primary through the ordinary
//!     pipeline — the old primary fences and redirects, the new one
//!     serves;
//! 2.  non-advancing, uncommitted-era, and exhausted targets are the
//!     named refusals;
//! 3.  an in-flight client stream survives the forced change: nothing
//!     committed is lost and the stream continues in slot order.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::SystemOperation;
use vrr::ids::{Era, NodeId, OperationId, View, ViewId};
use vrr::progress::{ProgressSnapshot, Status};
use vrr::replica::{PlanRefusal, ViewChangeKnobs};
use vrr::wire::Tag;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn n(id: u32) -> NodeId {
    NodeId(id)
}

fn view(number: u32) -> ViewId {
    ViewId {
        era: Era(1),
        view: View(number),
    }
}

fn snap(h: &Harness, id: NodeId) -> ProgressSnapshot {
    h.snapshot(id).expect("the node is live")
}

/// The node's status, decoded from the observation word.
fn status_of(h: &Harness, id: NodeId) -> Status {
    Status::from_word(snap(h, id).status).expect("the word is a status")
}

/// The node's current view as a `ViewId`.
fn current_view(h: &Harness, id: NodeId) -> ViewId {
    let snapshot = snap(h, id);
    ViewId {
        era: Era(snapshot.era),
        view: View(snapshot.view),
    }
}

fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

const TIMEOUT: u64 = 3;

fn cluster() -> Harness {
    Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    )
}

fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        assert_eq!(
            status_of(h, id),
            Status::Normal,
            "bootstrap left {id:?} Normal"
        );
    }
    h.assert_safety();
}

/// Drives one proposal at `primary` to full commitment everywhere and
/// applies it.
fn commit_one(h: &mut Harness, primary: NodeId, lsb: u64, payload: &[u8]) {
    h.propose(primary, op_id(lsb), payload);
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
}

/// The payloads a node has applied, in slot order.
fn applied_payloads(h: &Harness, id: NodeId) -> Vec<Vec<u8>> {
    h.applied(id)
        .iter()
        .map(|(_, payload)| payload.to_vec())
        .collect()
}

// ---------------------------------------------------------------------------
// 6. The forced change installs the chosen primary through the ordinary
//    pipeline.
// ---------------------------------------------------------------------------

#[test]
fn admin_forced_view_installs_the_chosen_primary_through_the_ordinary_pipeline() {
    let mut h = cluster();
    bootstrap(&mut h);
    commit_one(&mut h, n(0), 1, b"a"); // slot 3 committed everywhere

    // The host forces view 1 — whose primary under the genesis
    // membership order is n1 — at the SERVING primary itself.
    let outcome = h.force_view(n(0), view(1));
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the forced change is accepted: {outcome:?}"
    );
    assert_eq!(status_of(&h, n(0)), Status::ViewChange);
    assert_eq!(current_view(&h, n(0)), view(1));

    // The old primary is fenced: a proposal is redirected to the member
    // the forced view names.
    let refused = h.propose(n(0), op_id(9), b"z");
    assert_eq!(
        refused,
        StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
            view: view(1),
            primary: Some(n(1)),
        }),
        "the old primary fences and redirects to the chosen primary"
    );

    // The ordinary pipeline runs: fence, evidence, win, StartView.
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} settled");
        assert_eq!(current_view(&h, id), view(1));
    }

    // The chosen backup is primary: it serves; the old primary still
    // redirects.
    let refused = h.propose(n(0), op_id(10), b"y");
    assert_eq!(
        refused,
        StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
            view: view(1),
            primary: Some(n(1)),
        })
    );
    commit_one(&mut h, n(1), 2, b"b");
    assert_eq!(snap(&h, n(0)).committed, 4);
    assert_eq!(snap(&h, n(2)).committed, 4);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 7. The named refusals: non-advancing, uncommitted-era, and exhausted
//    targets.
// ---------------------------------------------------------------------------

#[test]
fn admin_force_view_rejects_non_advancing_uncommitted_era_and_exhausted_targets() {
    let mut h = cluster();
    bootstrap(&mut h);
    let current = current_view(&h, n(0));

    // A target that does not advance the view.
    let outcome = h.force_view(n(0), view(0));
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::AdminTargetNotAhead {
            current,
            target: view(0),
        })
    );

    // An era the replica does not hold the establishing operation of as
    // committed: era 2 exists nowhere in this cluster.
    let uncommitted_era = ViewId {
        era: Era(2),
        view: View(1),
    };
    let outcome = h.force_view(n(0), uncommitted_era);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::AdminEraNotCurrent {
            current: Era(1),
            got: Era(2),
        })
    );

    // The last representable view: the forced fence could never be
    // superseded, so the target is refused outright (§8.7.3 forbids
    // wraparound).
    let exhausted = ViewId {
        era: Era(1),
        view: View(u32::MAX),
    };
    let outcome = h.force_view(n(0), exhausted);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::AdminViewExhausted { target: exhausted })
    );

    // Nothing moved: no fence, no traffic, no fault.
    assert_eq!(status_of(&h, n(0)), Status::Normal);
    assert_eq!(current_view(&h, n(0)), view(0));
    assert!(h.peek_queued(n(1), Tag::StartViewChange).is_none());
    assert!(h.peek_queued(n(2), Tag::StartViewChange).is_none());
    assert!(!snap(&h, n(0)).faulted);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 8. An in-flight client stream survives the forced change.
// ---------------------------------------------------------------------------

#[test]
fn client_stream_survives_a_forced_view_change() {
    let mut h = cluster();
    bootstrap(&mut h);
    commit_one(&mut h, n(0), 1, b"a"); // slot 3 committed and applied everywhere

    // The host forces view 1 at a backup: the chosen primary is n1.
    h.force_view(n(2), view(1));
    // The fence reaches the old primary before the next proposal does.
    h.deliver_to(n(0));
    let refused = h.propose(n(0), op_id(2), b"b");
    assert_eq!(
        refused,
        StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
            view: view(1),
            primary: Some(n(1)),
        }),
        "mid-change, the old primary redirects the stream"
    );

    // The pipeline completes; the re-issued proposals commit in the new
    // view, in slot order after everything committed before the force.
    h.deliver_all();
    commit_one(&mut h, n(1), 2, b"b"); // slot 4
    commit_one(&mut h, n(1), 3, b"c"); // slot 5
    for id in [n(0), n(1), n(2)] {
        assert_eq!(
            applied_payloads(&h, id),
            vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec()],
            "{id:?} applied the whole stream in slot order"
        );
        assert_eq!(status_of(&h, id), Status::Normal);
        assert_eq!(current_view(&h, id), view(1));
    }
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 9. The forced fence into the established-but-unentered era: a committed
//    reconfiguration has advanced the era table past the current view's era,
//    and the operator forces the view change into that established era.
// ---------------------------------------------------------------------------

#[test]
fn admin_force_view_into_the_established_era() {
    let mut h = cluster();
    bootstrap(&mut h);

    // A committed reconfiguration establishes era 2. The view has NOT
    // changed — only the era table advanced (§8.7.1). The current view
    // is still (1, 0) but the committed configuration history names
    // era 2 as established.
    let outcome = h.reconfigure(n(0), SystemOperation::Increment(n(2)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(snap(&h, n(0)).committed, 3);
    let table = h.era_table(n(0)).expect("the node is live");
    assert_eq!(table.current().era, Era(2), "era 2 is established");
    assert_eq!(
        current_view(&h, n(0)).era,
        Era(1),
        "the view has not entered era 2"
    );

    // The operator forces a view into the established-but-unentered era.
    // The target names era 2, view 1 — the first view past the current
    // one, in the new era.
    let target = ViewId {
        era: Era(2),
        view: View(1),
    };
    let outcome = h.force_view(n(0), target);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the forced fence into the established era is accepted: {outcome:?}"
    );
    assert_eq!(status_of(&h, n(0)), Status::ViewChange);
    assert_eq!(current_view(&h, n(0)), target);

    // The ordinary pipeline runs: fence, evidence, win, StartView.
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} settled");
        assert_eq!(current_view(&h, id), target, "{id:?} entered era 2");
    }

    // The era-2 primary serves; the old primary redirects.
    let refused = h.propose(n(0), op_id(10), b"z");
    assert_eq!(
        refused,
        StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
            view: target,
            primary: Some(n(1)),
        }),
        "the era-2 primary of view 1 is n1"
    );
    h.assert_safety();
}
