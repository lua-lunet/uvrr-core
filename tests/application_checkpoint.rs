//! The application boundary closed: ordered `Apply` upcalls, the
//! result-free `Applied` acknowledgement, the checkpoint frontier, and the
//! reclamation the checkpoint alone authorizes (§4, §11, §11.1; decisions
//! S1, B2).
//!
//! The rulings under test:
//!
//! - `Effect::Apply` is emitted in strict slot order and never above
//!   `committed` (§11.1).
//! - `Input::Applied { slot }` carries no result (B2); a duplicate or
//!   out-of-order report — including one above `committed` — is rejected
//!   without state change.
//! - `Input::Checkpointed { through }` is accepted only when
//!   `through <= applied`; the published checkpoint frontier is the SOLE
//!   reclamation authorization, and the drop is lazy: whole slabs whose
//!   final slot the checkpoint covers, never the tail, opportunistically
//!   on append (§4, S1).
//! - The system-slot ruling (§11): `applied` walks EVERY slot. The genesis
//!   system operations (`Void`@1, `Init`@2, §8.7.2) are core-internal —
//!   they emit no `Apply` upcall and expect no acknowledgement — but they
//!   advance `applied` the moment the contiguous committed prefix allows.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::effects::Effect;
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::{ProgressSnapshot, Status};
use vrr::replica::PlanRefusal;
use vrr::wire::{Header, Tag};

/// Node id shorthand (the harness's own pattern).
fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// An operation identity for the scripts: the host assigns it, the core
/// carries it opaque (§11.1, B2).
fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

/// The snapshot of a live node (tests never snapshot a crashed one).
fn snap(h: &Harness, id: NodeId) -> ProgressSnapshot {
    h.snapshot(id).expect("the node is live")
}

/// The node's status, decoded from the observation word.
fn status_of(h: &Harness, id: NodeId) -> Status {
    Status::from_word(snap(h, id).status).expect("the word is a status")
}

/// Bootstraps the cluster: the genesis primary promotes itself and both
/// backups adopt view (1, 0).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
}

/// One operation end to end at every node: proposed, committed, applied.
fn commit_one(h: &mut Harness, lsb: u64, payload: &[u8]) {
    let outcome = h.propose(n(0), op_id(lsb), payload);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the primary accepts its own proposal: {outcome:?}"
    );
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        let applies = h.execute_apply_effects(id);
        assert_eq!(applies.len(), 1, "one committed slot, one upcall");
        assert!(
            matches!(applies[0].outcome, StepOutcome::Published { .. }),
            "the acknowledgement publishes: {:?}",
            applies[0].outcome
        );
    }
}

/// One operation committed everywhere but applied nowhere: the upcalls sit
/// in the nodes' pending lists.
fn commit_unapplied(h: &mut Harness, lsb: u64, payload: &[u8]) {
    let outcome = h.propose(n(0), op_id(lsb), payload);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the primary accepts its own proposal: {outcome:?}"
    );
    h.deliver_all();
}

/// A fabricated `RecoveryResponse` envelope (the header slot is the Absent
/// sentinel; the frontiers ride in the body).
#[test]
fn ordered_apply_never_above_committed() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    for lsb in 1..=3u64 {
        let slot = Slot(2 + lsb);
        let outcome = h.propose(n(0), op_id(lsb), &[lsb as u8]);
        let StepOutcome::Published { effects, .. } = outcome else {
            panic!("the primary accepts its own proposal: {outcome:?}");
        };
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, Effect::Apply { .. })),
            "a proposal commits nothing by itself: no upcall yet: {effects:?}"
        );

        // A `Prepare` the backup accepts without a quorum commits nothing:
        // the acknowledgement goes out, no upcall rides along (§11.1).
        let delivery = h.deliver_to(n(1)).expect("the Prepare is queued");
        let StepOutcome::Published { effects, .. } = delivery.outcome else {
            panic!("the accept publishes: {:?}", delivery.outcome);
        };
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, Effect::Apply { .. })),
            "accepted is not committed: no upcall above the frontier: {effects:?}"
        );

        h.deliver_all();
        for id in [n(0), n(1), n(2)] {
            let applies = h.execute_apply_effects(id);
            assert_eq!(applies.len(), 1, "exactly one slot newly committed");
            assert_eq!(applies[0].slot, slot, "upcalls arrive in strict slot order");
            assert_eq!(
                snap(&h, id).applied,
                slot.0,
                "the acknowledgement advanced the applied frontier"
            );
        }
    }
    h.assert_safety();
}

// 2. A duplicate `Applied` is rejected without state change (§11.1).
#[test]
fn duplicate_applied_is_rejected_without_state_change() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);
    commit_one(&mut h, 1, b"a");

    let before = snap(&h, n(1));
    let outcome = h.report_applied(n(1), Slot(3));
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::UnexpectedApplied {
            expected: None,
            got: Slot(3),
        }),
        "slot 3 is already applied and nothing newer is committed"
    );
    assert_eq!(
        snap(&h, n(1)),
        before,
        "a rejected acknowledgement changes nothing, revision included"
    );
    h.assert_safety();
}

// 3. An out-of-order `Applied` — skipping a slot, or naming a slot above
//    the committed frontier — is rejected without state change (§11.1).
#[test]
fn out_of_order_applied_is_rejected() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);
    commit_unapplied(&mut h, 1, b"a");
    commit_unapplied(&mut h, 2, b"b");
    // Committed through 4 everywhere; applied nowhere past the genesis
    // system slots.
    assert_eq!(snap(&h, n(1)).committed, 4);
    assert_eq!(snap(&h, n(1)).applied, 2);

    let before = snap(&h, n(1));
    let skipping = h.report_applied(n(1), Slot(4));
    assert_eq!(
        skipping,
        StepOutcome::PlanRefused(PlanRefusal::UnexpectedApplied {
            expected: Some(Slot(3)),
            got: Slot(4),
        }),
        "slot 3 must complete first"
    );
    let above_committed = h.report_applied(n(1), Slot(9));
    assert_eq!(
        above_committed,
        StepOutcome::PlanRefused(PlanRefusal::UnexpectedApplied {
            expected: Some(Slot(3)),
            got: Slot(9),
        }),
        "slot 9 is not even committed"
    );
    assert_eq!(
        snap(&h, n(1)),
        before,
        "rejected acknowledgements change nothing, revision included"
    );

    // The honest sequence still completes: the rejections disturbed nothing.
    let applies = h.execute_apply_effects(n(1));
    assert_eq!(applies.len(), 2);
    assert_eq!(applies[0].slot, Slot(3));
    assert_eq!(applies[1].slot, Slot(4));
    assert_eq!(snap(&h, n(1)).applied, 4);
    h.assert_safety();
}

// 4. `Checkpointed { through }` with `through > applied` is rejected: a
//    checkpoint cannot claim state the application has not incorporated
//    (§5's frontier chain, §11).
#[test]
fn checkpoint_beyond_applied_is_rejected() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);
    commit_unapplied(&mut h, 1, b"a");
    // Slot 3 is committed but not applied at n1.
    assert_eq!(snap(&h, n(1)).committed, 3);
    assert_eq!(snap(&h, n(1)).applied, 2);

    let before = snap(&h, n(1));
    let outcome = h.checkpoint(n(1), Slot(3));
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::CheckpointExceedsApplied {
            applied: Slot(2),
            through: Slot(3),
        }),
    );
    assert_eq!(
        snap(&h, n(1)),
        before,
        "a rejected checkpoint changes nothing, revision included"
    );

    // The boundary value is accepted: `through == applied`.
    let outcome = h.checkpoint(n(1), Slot(2));
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "through == applied publishes the frontier: {outcome:?}"
    );
    assert_eq!(snap(&h, n(1)).checkpoint, 2);
    h.assert_safety();
}

// 5. The checkpoint authorizes reclamation: after `Checkpointed` through
//    slot k and the next append, slabs ending at or below k are gone; the
//    tail is never reclaimed; entries above k remain readable (§4, S1).
#[test]
fn checkpoint_authorizes_lazy_whole_slab_reclaim() {
    // Tail capacity 2 places the slab boundaries deterministically: slots
    // [1,2] and [3,4] seal as slots 3 and 5 are accepted.
    let mut h = Harness::with_journal_capacity(3, 2);
    bootstrap(&mut h);
    for lsb in 1..=3u64 {
        commit_one(&mut h, lsb, &[lsb as u8]);
    }
    assert_eq!(snap(&h, n(0)).applied, 5);
    assert_eq!(
        h.retained(n(0)),
        Some((Slot(1), Slot(5))),
        "nothing reclaimed: no checkpoint was ever published"
    );

    for id in [n(0), n(1), n(2)] {
        let outcome = h.checkpoint(id, Slot(4));
        assert!(
            matches!(outcome, StepOutcome::Published { .. }),
            "through <= applied publishes: {outcome:?}"
        );
    }
    assert_eq!(
        h.retained(n(0)),
        Some((Slot(1), Slot(5))),
        "the authorization alone drops nothing: the drop is lazy, on append"
    );

    // The next append is the opportunistic moment.
    commit_one(&mut h, 4, b"d");
    for id in [n(0), n(1), n(2)] {
        assert_eq!(
            h.retained(id),
            Some((Slot(5), Slot(6))),
            "slabs ending at or below the checkpoint are gone"
        );
        assert!(
            h.journal_entry(id, Slot(3)).is_none() && h.journal_entry(id, Slot(4)).is_none(),
            "the covered slabs were dropped whole"
        );
        assert!(
            h.journal_entry(id, Slot(5)).is_some() && h.journal_entry(id, Slot(6)).is_some(),
            "entries above the checkpoint remain readable — the tail is never reclaimed"
        );
        assert_eq!(
            snap(&h, id).accepted,
            6,
            "the logical frontier is a protocol fact: reclamation never moves it (§4)"
        );
    }
    h.assert_safety();
}

// 6. Without a published checkpoint there is no reclamation, regardless of
//    age: the lazy drop never fires without the authorization (§4, S1).
#[test]
fn no_checkpoint_no_reclamation_regardless_of_age() {
    let mut h = Harness::with_journal_capacity(3, 2);
    bootstrap(&mut h);
    for lsb in 1..=6u64 {
        commit_one(&mut h, lsb, &[lsb as u8]);
    }
    // Four slabs sealed behind the tail by now; every append offered the
    // reclamation opportunity, and every one was a no-op.
    for id in [n(0), n(1), n(2)] {
        assert_eq!(
            h.retained(id),
            Some((Slot(1), Slot(8))),
            "no checkpoint was ever published: everything is retained"
        );
        assert!(
            h.journal_entry(id, Slot(1)).is_some(),
            "even the genesis slots survive: age is not an authorization"
        );
    }
    h.assert_safety();
}

#[test]
fn get_state_below_retained_base_is_refused() {
    // Tail capacity 2 places the slab boundaries deterministically: slabs
    // [1,2] and [3,4] seal as slots 3 and 5 are accepted.
    let mut h = Harness::with_journal_capacity(3, 2);
    bootstrap(&mut h);
    for lsb in 1..=3u64 {
        commit_one(&mut h, lsb, &[lsb as u8]);
    }
    let outcome = h.checkpoint(n(0), Slot(4));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    commit_one(&mut h, 4, b"d");
    assert_eq!(
        h.retained(n(0)),
        Some((Slot(5), Slot(6))),
        "the checkpoint-authorized drop fired: slots 1..=4 are physically gone"
    );

    // A lagging peer asks for the range from slot 1 — below the retained
    // base. The header slot is the requester's accepted frontier (the
    // per-tag table's Frontier role).
    let view = ViewId {
        era: Era(1),
        view: View(0),
    };
    let request = Message {
        header: Header {
            tag: Tag::GetState,
            view,
            slot: Slot(4),
        },
        body: Body::GetState { from: Slot(1) },
    };
    let before = snap(&h, n(0));
    let outcome = h.inject(n(1), n(0), request);
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the refusal publishes an identity transition: {outcome:?}");
    };
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::Send { .. })),
        "nothing is served from a range the journal let go: {effects:?}"
    );
    assert!(
        h.peek_queued(n(1), Tag::NewState).is_none(),
        "no chunk was fabricated"
    );
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::TransferNotServed { sender: n(1), view }),
        "the named refusal, on the observation"
    );
    let after = snap(&h, n(0));
    assert_eq!(after.accepted, before.accepted);
    assert_eq!(after.committed, before.committed);
    assert_eq!(after.applied, before.applied);
    assert_eq!(h.fault_of(n(0)), None, "a refusal, never a fault");
    h.assert_safety();
}

#[test]
fn view_change_after_reclaiming_the_era_establishing_slab() {
    let mut h = Harness::with_journal_capacity(3, 2);
    bootstrap(&mut h);
    commit_one(&mut h, 1, b"a");
    for id in [n(0), n(1), n(2)] {
        let outcome = h.checkpoint(id, Slot(3));
        assert!(matches!(outcome, StepOutcome::Published { .. }));
    }
    commit_one(&mut h, 2, b"b");
    for id in [n(0), n(1), n(2)] {
        assert!(
            h.journal_entry(id, Slot(2)).is_none(),
            "the establishing slot is physically absent"
        );
    }

    let target = ViewId {
        era: Era(1),
        view: View(1),
    };
    let outcome = h.force_view(n(1), target);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        let snapshot = snap(&h, id);
        assert_eq!(snapshot.era, target.era.0);
        assert_eq!(snapshot.view, target.view.0);
        assert_eq!(status_of(&h, id), Status::Normal);
    }
    h.assert_safety();
}
