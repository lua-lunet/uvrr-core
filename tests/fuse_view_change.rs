//! The fuse / view-change resolution (docs/uvrr-fuse.md §4 step 6; spec
//! §9.1, §9.2 and the §1.3 ranking rule): a fused batch accepted by a
//! quorum under view v, the leader crashed before the commit emission,
//! and the next view's recovery of what the quorum knew.
//!
//! The invariants these tests run under. The host obligations are held by
//! construction: the harness never recycles an identity (a crashed node
//! re-enters only through the boot gate's reincarnation path, which these
//! scripts never invoke) and every emission the scripts deliver was
//! durable before it was emitted (the publish path journals the
//! transition before it releases the effects). The test space is
//! therefore exhaustive over the protocol behaviour the scripts gate:
//! acceptance, fencing, evidence ranking and installation, with no
//! host-law violation able to masquerade as a protocol outcome.
//!
//! The ranking rule under test (§1.3; the Lean `ViewSelection.RankLE`
//! rule): the selected history is the greatest by retained view first,
//! then accepted frontier. Ranking by length alone is the published §9.2
//! counterexample, and `selection_is_view_first_not_length_first` is its
//! Rust witness: an older-view report with a longer log loses to a
//! newer-view report whose shorter log carries the fused values, the
//! mirror of Lean's `ViewSelection.newer_view_wins` and
//! `ViewSelection.length_only_loses_committed`.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{Member, SystemOperation, Weight};
use vrr::ids::{CrashCounter, Era, NodeId, OperationId, Slot, SystemId, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::plan::Plan;
use vrr::wire::{Header, Tag};

fn n(id: u32) -> NodeId {
    NodeId::new(
        SystemId::new((id + 1) as u16).expect("test system ids are small and non-zero"),
        CrashCounter::new(1).expect("one is non-zero"),
    )
}

/// A view in era 1: every scenario here is same-era (W1).
fn view(number: u32) -> ViewId {
    ViewId {
        era: Era(1),
        view: View(number),
    }
}

/// The bootstrap of `tests/fuse.rs`: the genesis primary promotes itself
/// and both backups adopt view (1, 0) from the promotion's `Commit`
/// announcement (§13.3).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

/// Delivers everything until the network is empty.
fn quiesce(h: &mut Harness) {
    while h.queued_len() > 0 {
        h.deliver_all();
    }
}

/// An operation identity for the scripts: the host assigns it, the core
/// carries it opaque (§11.1, B2).
fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

fn member(id: u32, weight: u32) -> Member {
    Member {
        node: n(id),
        weight: Weight(weight),
    }
}

/// The two operations the join-and-promote step packs, in plan order:
/// the learner join and its promotion in one era.
fn join_and_promote_ops() -> Vec<SystemOperation> {
    vec![
        SystemOperation::Join {
            node: n(3),
            position: 3,
        },
        SystemOperation::Increment(n(3)),
    ]
}

/// The one-step plan whose establishing batch packs two operations:
/// under view (1, 0) the envelope packs slots 3 and 4.
fn join_and_promote_plan() -> Plan {
    Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1)],
        steps: vec![join_and_promote_ops()],
    }
}

/// The three operations the three-join step packs, in plan order. R14's
/// unit rule moves no mass for a learner join, so the schedule is legal
/// at every op boundary.
fn three_learner_join_ops() -> Vec<SystemOperation> {
    vec![
        SystemOperation::Join {
            node: n(3),
            position: 3,
        },
        SystemOperation::Join {
            node: n(4),
            position: 4,
        },
        SystemOperation::Join {
            node: n(5),
            position: 5,
        },
    ]
}

/// The one-step plan whose establishing batch packs three zero-mass
/// learner joins: under view (1, 0) the envelope packs slots 3, 4 and 5.
fn three_learner_joins_plan() -> Plan {
    Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1)],
        steps: vec![three_learner_join_ops()],
    }
}

/// The operation payload bytes of a journal entry (system entries panic;
/// the scripts only inspect operation slots through this helper).
fn payload_of(entry: &LogEntry) -> &[u8] {
    let Payload::Operation { payload, .. } = &entry.payload else {
        panic!("expected an operation payload at {:?}", entry.slot);
    };
    payload
}

/// Asserts the journal at `id` carries `ops` one per slot from `first`
/// upwards, each stamped with the ballot's era: the fuse's per-slot
/// journal shape (docs/uvrr-fuse.md §1).
fn assert_fused_slots(h: &Harness, id: NodeId, first: u64, ops: &[SystemOperation]) {
    for (offset, op) in ops.iter().enumerate() {
        let slot = Slot(first + u64::try_from(offset).expect("small"));
        let entry = h
            .journal_entry(id, slot)
            .unwrap_or_else(|| panic!("{id:?} holds the fused slot {slot:?}"));
        let Payload::System(got) = &entry.payload else {
            panic!("the fused slot {slot:?} holds a system entry, not {entry:?}");
        };
        assert_eq!(got, op, "the fused value at {slot:?}");
        assert_eq!(
            entry.era,
            Era(1),
            "every packed slot is authorised by the ballot"
        );
    }
}

// ---------------------------------------------------------------------------
// 1. A fused batch accepted by the quorum survives the leader's crash into
//    the next view, and the new leader commits it with the values
//    unchanged.
// ---------------------------------------------------------------------------
#[test]
fn fused_acceptance_survives_leader_crash_into_next_view() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // The fused batch under view (1, 0): slots 3 and 4, accepted by the
    // leader and by both backups. The commit emission never leaves the
    // leader: no FuseOk is delivered before the crash, so nothing past
    // the genesis prefix is committed anywhere.
    let outcome = h.submit_plan(n(0), join_and_promote_plan());
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_tag(n(1), Tag::Fuse);
    h.deliver_tag(n(2), Tag::Fuse);
    assert_eq!(h.snapshot(n(1)).expect("live").accepted, 4);
    assert_eq!(h.snapshot(n(2)).expect("live").accepted, 4);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 2);

    // The leader crashes mid-fuse, before the commit fuse for the packed
    // slots is announced.
    h.crash(n(0));

    // The new view forms without the dead leader: n(1) is the primary
    // the view names under the genesis order (§1.2).
    let outcome = h.force_view(n(1), view(1));
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the forced fence publishes: {outcome:?}\n{}",
        h.trace_dump()
    );
    quiesce(&mut h);

    // The installed log carries the fused values at the fused slots: the
    // evidence quorum's knowledge overlaps the accept quorum's at n(1)
    // and n(2), so the selection keeps the accepted tail.
    for id in [n(1), n(2)] {
        let snapshot = h.snapshot(id).expect("live");
        assert_eq!(
            (snapshot.era, snapshot.view),
            (1, 1),
            "{id:?} installed the new view"
        );
        assert_eq!(snapshot.accepted, 4, "the fused slots survive");
        assert_eq!(
            snapshot.committed, 2,
            "nothing was committed before the crash"
        );
        assert_fused_slots(&h, id, 3, &join_and_promote_ops());
    }
    h.assert_safety();

    // The new leader commits the fused slots: the catch-up proposal's
    // acknowledgement cascades over the accepted tail (VRR-2012 §4's
    // cumulative rule), and the committed values are the values the
    // quorum fuse-accepted.
    let outcome = h.propose(n(1), op_id(1), b"after");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    for id in [n(1), n(2)] {
        let snapshot = h.snapshot(id).expect("live");
        assert_eq!(
            snapshot.committed, 5,
            "the fused slots and the client slot committed"
        );
        assert_fused_slots(&h, id, 3, &join_and_promote_ops());
    }
    let table = h.era_table(n(1)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3), "the batch's first slot");
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(join_and_promote_ops()),
        "the packed schedule committed as the one establishing batch it is"
    );
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 2. After the new view forms, a stray old-view message from the dead
//    leader's identity is fenced out by name, and the installed history
//    is untouched. The stale-view Fuse refusal is already pinned by
//    `stale_view_fuse_is_refused_as_a_whole` in tests/fuse.rs; this is
//    the general Prepare path.
// ---------------------------------------------------------------------------
#[test]
fn old_view_messages_are_fenced_after_the_view_change() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // The fused batch is accepted under view (1, 0); the leader crashes
    // before the commit emission; the new view forms without it.
    let outcome = h.submit_plan(n(0), join_and_promote_plan());
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_tag(n(1), Tag::Fuse);
    h.deliver_tag(n(2), Tag::Fuse);
    h.crash(n(0));
    let outcome = h.force_view(n(1), view(1));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(2)).expect("live").accepted, 4);

    // The stray: a view (1, 0) Prepare from the dead leader's identity,
    // carrying a client value at the slot above the fused range. The
    // sender guard passes (n(0) IS the primary view (1, 0) names), so
    // the refusal is the view fence's own, named ViewMismatch.
    let stray = Message {
        header: Header {
            tag: Tag::Prepare,
            view: view(0),
            slot: Slot(5),
        },
        body: Body::Prepare {
            entry: LogEntry {
                slot: Slot(5),
                era: Era(1),
                payload: Payload::Operation {
                    id: op_id(7),
                    payload: b"stray".to_vec().into_boxed_slice(),
                },
            },
            committed: Slot(2),
        },
    };
    let outcome = h.inject(n(0), n(2), stray);
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("a fenced-out message still publishes the drop, not {outcome:?}");
    };
    assert!(effects.is_empty(), "a refusal releases nothing");
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::ViewMismatch {
            got: view(0),
            current: view(1),
        }),
        "the old-view message is fenced out by name"
    );

    // The installed history is untouched: the frontier did not move and
    // the fused values stand.
    let snapshot = h.snapshot(n(2)).expect("live");
    assert_eq!(snapshot.accepted, 4, "the frontier did not move");
    assert_eq!(snapshot.committed, 2);
    assert_fused_slots(&h, n(2), 3, &join_and_promote_ops());
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 3. The selection ranks retained view first, not log length first: the
//    Rust witness of the §9.2 counterexample with the fused values as
//    the committed contents the newer view carries.
// ---------------------------------------------------------------------------
#[test]
fn selection_is_view_first_not_length_first() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // View (1, 0): X@3 committed everywhere.
    h.propose(n(0), op_id(1), b"x");
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }

    // The old leader's private tail: three entries no backup ever sees.
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    h.propose(n(0), op_id(2), b"y"); // Y@4
    h.propose(n(0), op_id(3), b"w"); // W@5
    h.propose(n(0), op_id(4), b"v"); // V@6
    assert_eq!(h.snapshot(n(0)).expect("live").accepted, 6);

    // View (1, 1) forms among {n1, n2}: the installed history ends at
    // X@3, so both survivors re-select under the newer retained view.
    let outcome = h.force_view(n(1), view(1));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(1)).expect("live").accepted, 3);

    // The fused batch under view (1, 1): slots 4 and 5 carry the packed
    // schedule, and the commit emission completes among {n1, n2}. The
    // fused values are committed under the newer view; the isolated old
    // leader never sees them.
    let outcome = h.submit_plan(n(1), join_and_promote_plan());
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_tag(n(2), Tag::Fuse);
    h.deliver_tag(n(1), Tag::FuseOk);
    h.deliver_all();
    assert_eq!(h.snapshot(n(1)).expect("live").committed, 5);
    assert_eq!(h.snapshot(n(2)).expect("live").committed, 5);

    // The mutation witness. n0 reports the OLDER retained view with the
    // LONGER log (accepted 6, retained (1, 0)); n2 reports the NEWER
    // retained view with the SHORTER log (accepted 5, retained (1, 1))
    // whose tail carries the fused values. Length-first ranking would
    // select n0's report and displace the committed fused values; the
    // normative rule (§1.3) selects n2's.
    h.partition(vec![n(1)], vec![n(0), n(2)]);
    let outcome = h.force_view(n(2), view(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_tag(n(0), Tag::StartViewChange);
    let report = h
        .peek_queued(n(2), Tag::DoViewChange)
        .expect("n0's evidence is queued for the new primary");
    let Body::DoViewChange {
        retained,
        accepted,
        committed,
        ..
    } = &report.body
    else {
        panic!("expected DoViewChange, not {:?}", report.body);
    };
    assert_eq!(
        *retained,
        view(0),
        "n0's history was selected under the old view"
    );
    assert_eq!(*accepted, Slot(6), "n0's log is the longer one");
    assert_eq!(*committed, Slot(3));
    assert!(
        *accepted > Slot(5),
        "the counterexample premise: length-first ranking would select n0's report"
    );
    assert!(
        *retained < view(1),
        "n0's retained view is older than n2's (1, 1): retained-first ranking must refuse it"
    );

    // The normative ranking keeps the newer-view history: the fused
    // values committed under view (1, 1) survive the change.
    h.deliver_all();
    let snapshot = h.snapshot(n(2)).expect("live");
    assert_eq!(
        (snapshot.era, snapshot.view),
        (1, 2),
        "n2 installs the target"
    );
    assert_eq!(
        snapshot.accepted, 5,
        "the newer-view history is installed, not the longer one"
    );
    assert_eq!(snapshot.committed, 5);
    assert_fused_slots(&h, n(2), 4, &join_and_promote_ops());

    // n0 adopts: the private tail is discarded whole and the fused
    // values stand at the slots the longer log would have overwritten.
    let entries = h.journal_entries(n(0));
    assert_eq!(
        entries.len(),
        5,
        "the private tail is gone from the journal"
    );
    assert_fused_slots(&h, n(0), 4, &join_and_promote_ops());
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 5);
    h.assert_safety();

    // The isolated node converges on the same installed history.
    h.heal();
    quiesce(&mut h);
    for id in [n(0), n(1), n(2)] {
        let snapshot = h.snapshot(id).expect("live");
        assert_eq!((snapshot.era, snapshot.view), (1, 2), "{id:?} settled");
        assert_eq!(snapshot.committed, 5);
        assert_fused_slots(&h, id, 4, &join_and_promote_ops());
    }
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 4. Per-slot resolution: the slots the quorum fuse-accepted keep their
//    values; the slot only the dead leader knew remains open until the
//    surviving quorum fills it, and never takes a third value.
// ---------------------------------------------------------------------------
#[test]
fn per_slot_resolution_after_fuse_interrupted() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // The fused batch covers slots 3, 4 and 5: three learner joins, one
    // envelope, accepted by the leader and both backups under view (1, 0).
    let outcome = h.submit_plan(n(0), three_learner_joins_plan());
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_tag(n(1), Tag::Fuse);
    h.deliver_tag(n(2), Tag::Fuse);
    assert_eq!(h.snapshot(n(1)).expect("live").accepted, 5);
    assert_eq!(h.snapshot(n(2)).expect("live").accepted, 5);

    // Slot 6's partial fate under the old view: the leader accepts a
    // client operation no backup ever sees (the partition holds the
    // Prepares), then crashes before any commit emission.
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    h.propose(n(0), op_id(1), b"private");
    assert_eq!(h.snapshot(n(0)).expect("live").accepted, 6);
    h.crash(n(0));

    // The new view forms among the survivors.
    let outcome = h.force_view(n(1), view(1));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);

    // Per slot: the quorum fuse-accepted slots keep their values, and
    // the slot only the dead leader knew remains open. No third value
    // appears: the installed history ends where the surviving quorum's
    // knowledge ends.
    for id in [n(1), n(2)] {
        let snapshot = h.snapshot(id).expect("live");
        assert_eq!(
            snapshot.accepted, 5,
            "the installed history ends at the quorum's knowledge"
        );
        assert_eq!(snapshot.committed, 2);
        assert_fused_slots(&h, id, 3, &three_learner_join_ops());
        assert!(
            h.journal_entry(id, Slot(6)).is_none(),
            "slot 6 remains open: the dead leader's private value is not recovered"
        );
    }
    h.assert_safety();

    // The surviving quorum fills the open slot: the new leader's own
    // proposal takes slot 6, the fused slots commit ahead of it with
    // their values unchanged, and the packed schedule establishes its
    // era as the one batch it is.
    let outcome = h.propose(n(1), op_id(2), b"filled");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    for id in [n(1), n(2)] {
        let snapshot = h.snapshot(id).expect("live");
        assert_eq!(
            snapshot.committed, 6,
            "the fused slots and the filled slot committed"
        );
        assert_fused_slots(&h, id, 3, &three_learner_join_ops());
        let entry = h.journal_entry(id, Slot(6)).expect("slot 6 is filled");
        assert_eq!(
            payload_of(&entry),
            b"filled",
            "the quorum's resolution, never the dead leader's private value"
        );
    }
    let table = h.era_table(n(1)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3), "the batch's first slot");
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(three_learner_join_ops()),
        "the packed schedule committed as the one establishing batch it is"
    );

    // The dead leader's actual stray Prepare for slot 6, healed back
    // into the network, cannot displace the resolution: it is fenced out
    // by name and slot 6 stands.
    h.heal();
    quiesce(&mut h);
    for id in [n(1), n(2)] {
        let entry = h.journal_entry(id, Slot(6)).expect("slot 6 stands");
        assert_eq!(payload_of(&entry), b"filled");
    }
    h.assert_safety();
}
