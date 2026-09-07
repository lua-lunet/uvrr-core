//! View-change protocol integration tests: VRR-2012 §5 against spec §9, §13.1,
//! and §1.3, on the scripted harness extended with view-change timeout knobs.
//!
//! Every test drives a three-node cluster whose `primary_timeout` is small
//! enough for deterministic tick-driven view changes. View arithmetic is
//! same-era (W1); the quorum decisions under test are `Role::Fence` (the
//! `StartViewChange` quorum) and `Role::ViewChange` (the `DoViewChange`
//! quorum), both reached only through the strategy (Q1).

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{INIT_SLOT, SystemOperation};
use vrr::ids::{Era, Fault, NodeId, OperationId, Slot, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, EraProof, EvidenceKind, Message};
use vrr::observe::Diagnostic;
use vrr::progress::{ProgressSnapshot, Status};
use vrr::replica::{PlanRefusal, ViewChangeKnobs};
use vrr::wire::{Header, Pack, Tag};

/// Node id shorthand (the harness's own pattern).
fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// A view in era 1 — every scenario here is same-era (W1).
fn view(number: u32) -> ViewId {
    ViewId {
        era: Era(1),
        view: View(number),
    }
}

/// The snapshot of a live node (tests never snapshot a crashed one).
fn snap(h: &Harness, id: NodeId) -> ProgressSnapshot {
    h.snapshot(id).expect("the node is live")
}

/// The node's current view as a `ViewId`.
fn current_view(h: &Harness, id: NodeId) -> ViewId {
    let snapshot = snap(h, id);
    ViewId {
        era: Era(snapshot.era),
        view: View(snapshot.view),
    }
}

/// The node's status, decoded from the observation word.
fn status_of(h: &Harness, id: NodeId) -> Status {
    Status::from_word(snap(h, id).status).expect("the word is a status")
}

/// The primary of a view in era 1 under the genesis order (§1.2).
fn primary_of(view: ViewId) -> NodeId {
    n(view.view.0 % 3)
}

/// The operation payload bytes of a journal entry (genesis entries panic;
/// the tests only inspect operation slots).
fn payload_of(entry: &LogEntry) -> &[u8] {
    let Payload::Operation { payload, .. } = &entry.payload else {
        panic!("expected an operation payload at {:?}", entry.slot);
    };
    payload
}

/// An operation identity for the scripts: the host assigns it, the core
/// carries it opaque (§11.1, B2).
fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

/// Executes every pending `Apply` at the given nodes (one harness step per
/// effect), the way a host would.
fn apply_all(h: &mut Harness, ids: [u32; 3]) {
    for id in ids {
        h.execute_apply_effects(n(id));
    }
}

/// The timeout knob used by every test in this suite: a backup enters a view
/// change after more than three ticks of primary silence (S4).
const TIMEOUT: u64 = 3;

/// A three-node cluster with the view-change knobs set explicitly (the
/// brief's requirement; the legacy harness constructors never time out).
fn cluster() -> Harness {
    Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    )
}

/// Bootstraps the cluster: the genesis primary promotes itself and both
/// backups adopt view (1, 0). Two harness steps.
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

/// Ticks a node until its timeout fires (one tick past the knob), asserting
/// it fenced into exactly `target`.
fn tick_into_view_change(h: &mut Harness, id: NodeId, target: ViewId) {
    for _ in 0..=TIMEOUT {
        h.tick(id);
    }
    assert_eq!(status_of(h, id), Status::ViewChange);
    assert_eq!(current_view(h, id), target);
}

/// The era-1 proof every fabricated message in this suite carries: the real
/// `Init` operation committed at the genesis slot (§8.7.8, W1).
fn era_proof() -> EraProof {
    EraProof {
        op: SystemOperation::Init {
            order: vec![n(0), n(1), n(2)],
        },
        committed_at: INIT_SLOT,
    }
}

/// A fabricated `StartViewChange` envelope (the header slot is the Absent
/// sentinel; the body is empty).
fn svc(view: ViewId) -> Message {
    Message {
        header: Header {
            tag: Tag::StartViewChange,
            view,
            slot: Slot::NONE,
        },
        body: Body::StartViewChange {},
    }
}

/// Drives a complete view change to `target` among the two live nodes
/// `prime` (which must be `primary_of(target)` and times out first) and
/// `voter`, leaving messages for the partitioned third node behind. Ends
/// with both nodes Normal in `target`.
fn drive_view_change(h: &mut Harness, prime: NodeId, voter: NodeId, target: ViewId) {
    assert_eq!(primary_of(target), prime, "the prime must own the target");
    tick_into_view_change(h, prime, target);
    assert!(h.peek_queued(voter, Tag::StartViewChange).is_some());
    h.deliver_all();
    for id in [prime, voter] {
        assert_eq!(status_of(h, id), Status::Normal, "{id:?} installs");
        assert_eq!(current_view(h, id), target);
    }
    h.assert_safety();
}

// 1. Happy path: partitioned primary, tick-driven fence quorum, evidence
//    quorum, StartView install, and an operation completing under view 1.
#[test]
fn happy_path_view_change_completes_and_serves_operations() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.propose(n(0), op_id(1), b"a");
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    h.assert_safety();
    assert_eq!(snap(&h, n(0)).committed, 3);

    // Partition the primary; the backups hear nothing more from view 0.
    h.partition(vec![n(0)], vec![n(1), n(2)]);

    // One backup's timeout is enough: its StartViewChange pulls the other
    // backup into the same view change (join-ahead), both fence quorums
    // complete, the voter's DoViewChange reaches the designated primary of
    // view 1, and StartView installs the new view.
    tick_into_view_change(&mut h, n(1), view(1));
    assert!(h.peek_queued(n(2), Tag::StartViewChange).is_some());

    h.deliver_all();
    for id in [n(1), n(2)] {
        assert_eq!(status_of(&h, id), Status::Normal);
        assert_eq!(current_view(&h, id), view(1));
        let snapshot = snap(&h, id);
        assert_eq!(snapshot.accepted, 3, "the committed entry survives");
        assert_eq!(snapshot.committed, 3);
    }
    h.assert_safety();

    // A fresh proposal completes end-to-end under view 1 (§9).
    h.propose(n(1), op_id(2), b"b");
    h.deliver_all();
    apply_all(&mut h, [1, 2, 0]);
    assert_eq!(snap(&h, n(1)).committed, 4);
    assert_eq!(
        h.applied(n(1)),
        &[
            (Slot(3), b"a".to_vec().into_boxed_slice()),
            (Slot(4), b"b".to_vec().into_boxed_slice())
        ]
    );

    // Heal: the partitioned primary joins on the stale StartViewChange,
    // adopts the StartView history, accepts the new entry, and catches up.
    h.heal();
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    assert_eq!(status_of(&h, n(0)), Status::Normal);
    assert_eq!(current_view(&h, n(0)), view(1));
    assert_eq!(snap(&h, n(0)).committed, 4);
    h.assert_safety();
}

// 2. Committed entries survive: a mid-pipeline crash loses nothing the
//    commit quorum held, and the installed uncommitted tail commits under
//    the new view by cumulative acknowledgement.
#[test]
fn committed_entries_survive_a_mid_pipeline_crash() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.propose(n(0), op_id(1), b"x"); // slot 3
    h.propose(n(0), op_id(2), b"y"); // slot 4
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);

    // Slot 5 is accepted by n0 and n1 but never committed.
    h.propose(n(0), op_id(3), b"z");
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(5));
    h.crash(n(0));

    // The surviving pair completes the view change without n0: n1's own
    // evidence carries the tail, so the selected history keeps slot 5.
    drive_view_change(&mut h, n(1), n(2), view(1));
    for id in [n(1), n(2)] {
        let snapshot = snap(&h, id);
        assert_eq!(snapshot.accepted, 5, "the winning history keeps n1's tail");
        assert_eq!(
            snapshot.committed, 4,
            "committed evidence is carried, not lost"
        );
        let journal = h.journal_entries(id);
        assert_eq!(journal.len(), 5);
        assert_eq!(payload_of(&journal[2]), b"x");
        assert_eq!(payload_of(&journal[3]), b"y");
        assert_eq!(payload_of(&journal[4]), b"z");
    }
    h.assert_safety();

    // The installed uncommitted tail (slot 5) has no proposal of its own;
    // the next commit reaches over it because a PrepareOk vouches for every
    // lower slot the sender holds contiguously (VRR-2012 §4's cumulative
    // acknowledgement).
    h.propose(n(1), op_id(9), b"w"); // slot 6
    h.deliver_all();
    apply_all(&mut h, [1, 2, 0]);
    assert_eq!(snap(&h, n(1)).committed, 6);
    assert_eq!(snap(&h, n(2)).committed, 6);
    h.assert_safety();
}

// 3. A divergent uncommitted tail is discarded: the new primary's selected
//    history ends at slot 2, and the healed old primary drops its private
//    slot 3 — asserted by journal inspection, not merely the frontier.
#[test]
fn divergent_uncommitted_tail_is_discarded() {
    let mut h = cluster();
    bootstrap(&mut h);

    // Partition the primary first, then let it accept an operation no
    // backup ever sees.
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    h.propose(n(0), op_id(1), b"x");
    assert_eq!(snap(&h, n(0)).accepted, 3);
    assert_eq!(payload_of(&h.journal_entries(n(0))[2]), b"x");

    drive_view_change(&mut h, n(1), n(2), view(1));
    assert_eq!(
        snap(&h, n(1)).accepted,
        2,
        "the selected history is shorter (§1.3)"
    );

    h.heal();
    h.deliver_all();
    assert_eq!(status_of(&h, n(0)), Status::Normal);
    assert_eq!(current_view(&h, n(0)), view(1));
    assert_eq!(snap(&h, n(0)).accepted, 2, "the old primary drops its tail");
    let journal = h.journal_entries(n(0));
    assert_eq!(journal.len(), 2, "slot 3 is gone from the journal");
    h.assert_safety();
}

// 4. The §9.2 counterexample (with §1.3's ranking rule): ranking evidence by
//    `accepted` alone would select n0's longer history (accepted 5, retained
//    view 0) and displace the entry committed at slot 4 under view 1. The
//    normative ranking — `retained` view first, then `accepted` — selects
//    n2's shorter history (accepted 4, retained view 1) and the committed
//    entry survives. This is the load-bearing test of the ranking rule.
#[test]
fn retained_view_outranks_accepted_frontier_section_9_2_counterexample() {
    let mut h = cluster();
    bootstrap(&mut h);

    // View 0: X@3 committed everywhere.
    h.propose(n(0), op_id(1), b"x");
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);

    // Partition n0 and let it accept two entries nobody else holds.
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    h.propose(n(0), op_id(2), b"y"); // Y@4
    h.propose(n(0), op_id(3), b"w"); // W@5
    assert_eq!(snap(&h, n(0)).accepted, 5);

    // View 1 completes among {n1, n2}: the selected history ends at X@3.
    drive_view_change(&mut h, n(1), n(2), view(1));

    // View 1: Z@4 committed by {n1, n2}. n0 never sees it.
    h.propose(n(1), op_id(9), b"z");
    h.deliver_all();
    apply_all(&mut h, [1, 2, 0]);
    assert_eq!(snap(&h, n(2)).committed, 4);

    // Re-partition: n1 is isolated, n0 and n2 can talk. n0's view-1-bound
    // held messages stay held (heal alone requeues them), so n0 still
    // believes in view 0 with its private tail.
    h.partition(vec![n(1)], vec![n(0), n(2)]);

    // n2 times out into view 2; n0 joins from view 0 (view gaps are legal,
    // §8.7.3), its fence quorum completes on n2's vote, and its evidence
    // flows to n2, the designated primary of view 2.
    tick_into_view_change(&mut h, n(2), view(2));
    h.deliver_to_matching(n(0), Tag::StartViewChange, Slot::NONE);
    assert_eq!(status_of(&h, n(0)), Status::ViewChange);
    assert_eq!(current_view(&h, n(0)), view(2));
    let dvc = h
        .peek_queued(n(2), Tag::DoViewChange)
        .expect("n0's fence quorum sends DoViewChange to n2");
    let Body::DoViewChange {
        retained,
        accepted,
        committed,
        suffix,
        evidence,
        ..
    } = &dvc.body
    else {
        panic!("expected DoViewChange");
    };
    assert_eq!(*retained, view(0), "n0's history was selected in view 0");
    assert_eq!(*accepted, Slot(5), "n0's tail is longer...");
    assert_eq!(*committed, Slot(3));
    assert_eq!(*evidence, EvidenceKind::Ordinary);
    assert_eq!(suffix.len(), 5, "the budget is unbounded here");

    // The counterexample premise (§9.2): accepted-alone ranking would pick
    // n0's history (5 > 4) and slot 4 would become Y, displacing committed Z.
    assert!(
        *accepted > Slot(4),
        "this test must reproduce the published scenario"
    );
    assert!(
        *retained < view(1),
        "n0's retained view is older than n2's (1,1) — retained-first ranking must refuse it"
    );

    // The normative ranking (§1.3) keeps n2's history: delivering the
    // evidence completes the change with Z@4 installed.
    h.deliver_all();
    let snapshot = snap(&h, n(2));
    assert_eq!(snapshot.status, Status::Normal.to_word());
    assert_eq!(current_view(&h, n(2)), view(2));
    assert_eq!(snapshot.accepted, 4);
    assert_eq!(snapshot.committed, 4);

    // n0 adopts: its uncommitted Y@4/W@5 tail is replaced by the selected
    // history, and slot 4 holds the committed Z.
    assert_eq!(status_of(&h, n(0)), Status::Normal);
    assert_eq!(current_view(&h, n(0)), view(2));
    let journal = h.journal_entries(n(0));
    assert_eq!(journal.len(), 4, "the divergent tail is discarded");
    assert_eq!(
        payload_of(&journal[3]),
        b"z",
        "the committed entry survives"
    );
    h.assert_safety();

    // Heal n1: it joins view 2 on the stale StartViewChange and adopts.
    h.heal();
    h.deliver_all();
    assert_eq!(status_of(&h, n(1)), Status::Normal);
    assert_eq!(current_view(&h, n(1)), view(2));
    h.assert_safety();
}

// 5. A StartViewChange behind the node's current view is stale; one ahead is
//    joined (the node advances its own target and re-broadcasts).
#[test]
fn start_view_change_stale_ignored_and_ahead_joined() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    drive_view_change(&mut h, n(1), n(2), view(1));

    // Stale: view 0 is behind n1's current view 1.
    let outcome = h.inject(n(2), n(1), svc(view(0)));
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("a stale vote still publishes the (identity) transition");
    };
    assert_eq!(effects, Vec::new());
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::StaleViewChange {
            got: view(0),
            current: view(1),
        })
    );

    // Ahead: view 2 pulls n1 into the next view change.
    let outcome = h.inject(n(2), n(1), svc(view(2)));
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("joining a higher view publishes");
    };
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            vrr::effects::Effect::Send { message, .. }
                if message.header.tag == Tag::StartViewChange && message.header.view == view(2)
        )),
        "joining the higher view re-broadcasts StartViewChange"
    );
    assert_eq!(status_of(&h, n(1)), Status::ViewChange);
    assert_eq!(current_view(&h, n(1)), view(2));
    h.assert_safety();
}

// 6. The fence is real: a node in ViewChange refuses old-view Prepares and
//    proposals, with named diagnostics.
#[test]
fn the_fence_is_real() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.propose(n(0), op_id(1), b"a");
    h.deliver_all();

    h.partition(vec![n(0)], vec![n(1), n(2)]);
    tick_into_view_change(&mut h, n(1), view(1));

    // A delayed view-0 Prepare from the old primary is fenced out.
    let prepare = Message {
        header: Header {
            tag: Tag::Prepare,
            view: view(0),
            slot: Slot(4),
        },
        body: Body::Prepare {
            entry: LogEntry {
                slot: Slot(4),
                era: Era(1),
                payload: Payload::Operation {
                    id: op_id(7),
                    payload: b"late".to_vec().into_boxed_slice(),
                },
            },
            committed: Slot(3),
        },
    };
    let outcome = h.inject(n(0), n(1), prepare);
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("a fenced-out message still publishes the drop");
    };
    assert_eq!(effects, Vec::new());
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::ViewMismatch {
            got: view(0),
            current: view(1),
        })
    );

    // Proposals are refused with redirection information (the
    // §13.4 convergence hint: current view and the primary of that view).
    assert_eq!(
        h.propose(n(1), op_id(9), b"w"),
        StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
            view: view(1),
            primary: Some(n(1)),
        })
    );
    h.assert_safety();
}

// 7. The §13.1 bounded suffix: the wire suffix never exceeds the budget by a
//    byte (W4: `packed_len` is normative), newest entries in ascending order,
//    committed scalar present. A recipient whose history is missing below
//    the suffix base records GapDetected and does not fault; one with full
//    history adopts cleanly.
#[test]
fn bounded_suffix_rules_section_13_1() {
    let payload = b"pp";
    let probe = LogEntry {
        slot: Slot::NONE,
        era: Era::INITIAL,
        payload: Payload::Operation {
            id: op_id(1),
            payload: payload.to_vec().into_boxed_slice(),
        },
    };
    let per_entry = probe.packed_len();
    // The budget admits exactly one operation entry: the §13.1 suffix packs
    // newest-first inside the budget and stops, which §13.1 explicitly
    // permits (a short — even empty — suffix is well-formed).
    let mut h = Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: per_entry,
        },
    );
    bootstrap(&mut h);

    // Slot 3 committed by {n0, n1}; n2 stays behind at genesis.
    h.propose(n(0), op_id(1), payload);
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3));
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3));
    h.drop_queued(n(2));
    // Slot 4 committed by {n0, n1} as well; n2 sees none of it.
    h.propose(n(0), op_id(2), payload);
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(4));
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(4));
    h.drop_queued(n(2));
    h.deliver_tag(n(1), Tag::Commit);
    h.deliver_tag(n(1), Tag::Commit);
    assert_eq!(snap(&h, n(1)).committed, 4);
    assert_eq!(snap(&h, n(2)).accepted, 2);
    assert_eq!(snap(&h, n(2)).committed, 2);

    // View change among {n1, n2} with n0 down: n2's evidence suffix packs
    // the newest genesis entries its budget admits — and never a byte
    // beyond it (W4).
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    tick_into_view_change(&mut h, n(1), view(1));
    h.deliver_tag(n(2), Tag::StartViewChange);
    let dvc = h
        .peek_queued(n(1), Tag::DoViewChange)
        .expect("n2's fence sends evidence to n1");
    let Body::DoViewChange {
        accepted,
        committed,
        suffix,
        ..
    } = &dvc.body
    else {
        panic!("expected DoViewChange");
    };
    assert_eq!(*accepted, Slot(2));
    assert_eq!(
        *committed,
        Slot(2),
        "the committed scalar travels outside the suffix"
    );
    let offered: usize = suffix.iter().map(Pack::packed_len).sum();
    assert!(
        offered <= per_entry,
        "the suffix never exceeds the budget by a byte (W4)"
    );
    assert!(
        suffix.last().is_some_and(|entry| entry.slot == Slot(2)),
        "the suffix ends at the reported frontier, ascending order"
    );

    // Complete the change: n1 is the winner (its own history). n2 sits on
    // n1's side of the partition — but n2 is missing history below the
    // suffix base: slot 3 is unverifiable (base 4 > accepted + 1 = 3).
    // GapDetected, no fault, no adoption — and the fetch half of the
    // ruling (§13.1 step 5) asks the new primary for the missing range.
    h.deliver_tag(n(1), Tag::StartViewChange); // n2's fence completes n1's fence quorum
    h.deliver_tag(n(1), Tag::DoViewChange); // n1 wins and broadcasts StartView
    assert_eq!(status_of(&h, n(1)), Status::Normal);
    h.deliver_tag(n(2), Tag::StartView);
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::GapDetected {
            expected: Slot(3),
            got: Slot(4),
        })
    );
    assert!(!snap(&h, n(2)).faulted);
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    let request = h
        .peek_queued(n(1), Tag::GetState)
        .expect("the gap fetches from the new primary");
    let Body::GetState { from } = &request.body else {
        panic!("expected GetState");
    };
    assert_eq!(*from, Slot(3));
    assert_eq!(request.header.slot, Slot(2));

    // The rest of the exchange: the fetch is served chunk by chunk under
    // the budget; n2 stays fenced — the completing ruling is the
    // StartView's, already spent — and nothing faults.
    h.deliver_all();
    assert!(!snap(&h, n(2)).faulted);
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    assert_eq!(snap(&h, n(2)).accepted, 4, "the chunks filled the gap");

    // The StartView carries the newest entry under the budget — exactly
    // one. n0's copy was partition-held at broadcast time; heal surfaces
    // it for inspection.
    h.heal();
    let start_view = h
        .peek_queued(n(0), Tag::StartView)
        .expect("the new primary broadcasts StartView");
    let Body::StartView {
        suffix,
        accepted,
        committed,
        ..
    } = &start_view.body
    else {
        panic!("expected StartView");
    };
    assert_eq!(*accepted, Slot(4));
    assert_eq!(*committed, Slot(4));
    let bytes: usize = suffix.iter().map(Pack::packed_len).sum();
    assert!(
        bytes <= per_entry,
        "the suffix never exceeds the budget by a byte (W4)"
    );
    assert_eq!(suffix.len(), 1);
    assert_eq!(suffix[0].slot, Slot(4), "the newest entry, ascending order");

    // n0 holds the full history and adopts cleanly.
    h.deliver_all();
    assert_eq!(status_of(&h, n(0)), Status::Normal);
    assert_eq!(current_view(&h, n(0)), view(1));
    assert_eq!(snap(&h, n(0)).committed, 4);
    h.assert_safety();
}

// 8. A StartView whose suffix conflicts with a committed slot is a
//    quorum-obligation violation: the node faults locally with
//    IllegalTransition (the view-change path's one deliberate fault-on-peer-input).
#[test]
fn start_view_conflicting_at_a_committed_slot_faults() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.propose(n(0), op_id(1), b"x");
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    assert_eq!(snap(&h, n(2)).committed, 3);

    // Fabricate a StartView from the legitimate primary of view 1 whose
    // suffix rewrites the committed slot 3. No honest quorum produces this.
    let forged = Message {
        header: Header {
            tag: Tag::StartView,
            view: view(1),
            slot: Slot(3),
        },
        body: Body::StartView {
            suffix: vec![LogEntry {
                slot: Slot(3),
                era: Era(1),
                payload: Payload::Operation {
                    id: op_id(1),
                    payload: b"forged".to_vec().into_boxed_slice(),
                },
            }],
            accepted: Slot(3),
            committed: Slot(3),
            era_proof: era_proof(),
        },
    };
    h.expect_fault(n(2));
    let outcome = h.inject(n(1), n(2), forged);
    assert!(matches!(outcome, StepOutcome::PublishRefused(_)));
    assert_eq!(h.fault_of(n(2)), Some(Fault::IllegalTransition));
    assert!(snap(&h, n(2)).faulted);
    h.assert_safety();
}

// 9. Self-intersection (V_g ⌢ V_g): on a 3-node cluster two view changes
//    for the same view cannot both complete. The completed change's primary
//    is Normal; the latecomer's fence and evidence are stale-dropped, and
//    exactly one StartView takes effect.
#[test]
fn two_view_changes_for_one_view_cannot_both_complete() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.propose(n(0), op_id(1), b"a");
    h.deliver_all();

    h.partition(vec![n(0)], vec![n(1), n(2)]);
    drive_view_change(&mut h, n(1), n(2), view(1));

    // n0 heals late, still in view 0; its StartViewChange for view 1 is
    // stale at n1 (Normal) — it cannot recruit a fence quorum for a view
    // whose change already completed.
    h.drop_held();
    h.heal();
    let outcome = h.inject(n(0), n(1), svc(view(1)));
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("a stale vote still publishes the drop");
    };
    assert_eq!(effects, Vec::new());
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::StaleViewChange {
            got: view(1),
            current: view(1),
        })
    );

    // Even n0's evidence, sent straight to the view's primary, is refused:
    // n1 is no longer collecting. No second StartView leaves the cluster.
    let dvc = Message {
        header: Header {
            tag: Tag::DoViewChange,
            view: view(1),
            slot: Slot(3),
        },
        body: Body::DoViewChange {
            retained: view(0),
            accepted: Slot(3),
            committed: Slot(3),
            suffix: h.journal_entries(n(0)),
            evidence: EvidenceKind::Ordinary,
            era_proof: era_proof(),
        },
    };
    let outcome = h.inject(n(0), n(1), dvc);
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("stale evidence still publishes the drop");
    };
    assert!(
        !effects.iter().any(|effect| matches!(
            effect,
            vrr::effects::Effect::Send { message, .. } if message.header.tag == Tag::StartView
        )),
        "the competing change produces no second StartView"
    );
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::StaleEvidence {
            got: view(1),
            current: view(1),
        })
    );

    // Exactly one view change took effect: n1 and n2 are Normal in view 1;
    // n0 never left view 0 (it is its primary and never suspects itself), so
    // the two Normal primaries claim different views and the checker's
    // rule 2 holds.
    assert_eq!(status_of(&h, n(0)), Status::Normal);
    assert_eq!(current_view(&h, n(0)), view(0));
    for id in [n(1), n(2)] {
        assert_eq!(status_of(&h, id), Status::Normal);
        assert_eq!(current_view(&h, id), view(1));
    }
    h.assert_safety();
}

// 10. A retried proposal across the change: the proposer's in-flight entry
//     is discarded with the old primary's tail, the retry is refused during
//     the change with redirection, and afterwards the retried operation —
//     the same host-assigned identity — commits at a fresh slot and
//     applies exactly once in the surviving history. The core tracks
//     nothing about the retry (§11.1, B2); exactly-once across the
//     discard-and-retry window is the host's deduplication policy above
//     the boundary.
#[test]
fn retried_proposal_across_the_change() {
    let mut h = cluster();
    bootstrap(&mut h);

    // The proposal is accepted by the old primary alone, under partition.
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    h.propose(n(0), op_id(1), b"x");
    assert_eq!(snap(&h, n(0)).accepted, 3);

    // The proposer retries during the change: n1 is fenced and refuses with
    // redirection to the primary of the view it is fencing into.
    tick_into_view_change(&mut h, n(1), view(1));
    assert_eq!(
        h.propose(n(1), op_id(1), b"x"),
        StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
            view: view(1),
            primary: Some(n(1)),
        })
    );

    // The change completes without n0's tail: the selected history ends at
    // slot 2, so the payload exists nowhere in the new history.
    h.deliver_all();
    assert_eq!(status_of(&h, n(1)), Status::Normal);
    assert_eq!(snap(&h, n(1)).accepted, 2);
    assert_eq!(h.journal_entries(n(1)).len(), 2);

    // The retry is accepted fresh under view 1 — the core does not
    // deduplicate, so the same identity takes the next slot — and commits
    // exactly once.
    h.propose(n(1), op_id(1), b"x");
    h.deliver_all();
    apply_all(&mut h, [1, 2, 0]);
    assert_eq!(snap(&h, n(1)).accepted, 3);
    assert_eq!(snap(&h, n(1)).committed, 3);
    let applied: Vec<&(Slot, Box<[u8]>)> = h
        .applied(n(1))
        .iter()
        .filter(|(_, payload)| payload.as_ref() == b"x")
        .collect();
    assert_eq!(
        applied.len(),
        1,
        "the retried operation applied exactly once in the surviving history"
    );

    // The old primary heals and adopts: its private slot 3 is replaced by
    // the selected history (which happens to carry the same payload — the
    // retry found its way), and every journal holds the entry exactly once.
    h.heal();
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    for id in [n(0), n(1), n(2)] {
        let journal = h.journal_entries(id);
        let copies = journal
            .iter()
            .filter(
                |entry| matches!(&entry.payload, Payload::Operation { payload, .. } if payload.as_ref() == b"x"),
            )
            .count();
        assert_eq!(copies, 1, "the entry exists once on {id:?}");
        assert_eq!(journal.len(), 3);
    }
    h.assert_safety();
}

// 12. The §13.1 bounded suffix under an early packing break: the winner's
//     budget may stop the packing of the selected suffix part-way (an
//     entry exceeds the remaining budget — the budget is host policy, W5,
//     so a peer's evidence may honestly carry more than the winner's
//     budget admits). The packing then stops entirely: walking the journal
//     below the selected suffix's base would emit a suffix with a hole
//     between the packed journal entries and the unpacked selected entries
//     above, and §13.1's shape rule refuses a non-contiguous offer at
//     every backup — the change could never complete.
#[test]
fn bounded_suffix_packing_break_never_emits_a_hole() {
    let probe = LogEntry {
        slot: Slot::NONE,
        era: Era::INITIAL,
        payload: Payload::Operation {
            id: op_id(1),
            payload: b"pp".to_vec().into_boxed_slice(),
        },
    };
    let small = probe.packed_len();
    // Admits exactly three small entries: the selected suffix's newest two
    // pack, its big entry breaks the packing, and the remaining budget
    // still has room for one more small journal entry — the hole shape.
    let budget = 3 * small;
    let mut h = Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: budget,
        },
    );
    bootstrap(&mut h);

    // Slots 3..=5 commit everywhere; slot 4 is the big entry.
    h.propose(n(0), op_id(1), b"pp");
    h.deliver_all();
    h.propose(n(0), op_id(2), &[0x42; 64]);
    h.deliver_all();
    h.propose(n(0), op_id(3), b"pp");
    h.deliver_all();
    // Slot 6 commits at {n0, n2} only: n1's frontier stays at 5.
    h.propose(n(0), op_id(4), b"pp");
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(6));
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(6));
    h.drop_queued(n(1));
    h.deliver_tag(n(2), Tag::Commit);
    assert_eq!(snap(&h, n(1)).accepted, 5);
    assert_eq!(snap(&h, n(2)).accepted, 6);
    assert_eq!(snap(&h, n(2)).committed, 6);

    // The view-1 change: n1 (the view's primary) fences, n2 joins, both
    // fence quorums complete. n2's real evidence stays queued; in its
    // place arrives the evidence an n2 with a larger suffix budget would
    // honestly send (W5): the real entries for slots 4..=6, more than
    // n1's budget admits.
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    tick_into_view_change(&mut h, n(1), view(1));
    h.deliver_tag(n(2), Tag::StartViewChange);
    h.deliver_tag(n(1), Tag::StartViewChange);
    h.drop_queued(n(1));
    let suffix: Vec<LogEntry> = h
        .journal_entries(n(2))
        .into_iter()
        .filter(|entry| entry.slot >= Slot(4))
        .collect();
    assert_eq!(suffix.len(), 3);
    let outcome = h.inject(
        n(2),
        n(1),
        Message {
            header: Header {
                tag: Tag::DoViewChange,
                view: view(1),
                slot: Slot(6),
            },
            body: Body::DoViewChange {
                retained: view(0),
                accepted: Slot(6),
                committed: Slot(6),
                suffix,
                evidence: EvidenceKind::Ordinary,
                era_proof: era_proof(),
            },
        },
    );
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the evidence quorum completes and the change installs"
    );
    assert_eq!(status_of(&h, n(1)), Status::Normal);

    // The winner's StartView suffix: strictly contiguous, ending at the
    // selected frontier, never a byte beyond the budget (§13.1, W4).
    let start_view = h
        .peek_queued(n(2), Tag::StartView)
        .expect("the completed change broadcasts StartView");
    let Body::StartView {
        suffix,
        accepted,
        committed,
        ..
    } = &start_view.body
    else {
        panic!("expected StartView");
    };
    assert_eq!(*accepted, Slot(6));
    assert_eq!(*committed, Slot(6));
    assert_eq!(suffix.last().map(|entry| entry.slot), Some(Slot(6)));
    assert!(
        suffix
            .windows(2)
            .all(|pair| pair[0].slot.next() == Some(pair[1].slot)),
        "the emitted suffix is strictly contiguous: {suffix:?}"
    );
    let bytes: usize = suffix.iter().map(Pack::packed_len).sum();
    assert!(
        bytes <= budget,
        "the suffix never exceeds the budget by a byte (W4)"
    );

    // A backup holding the history adopts cleanly: the change completes.
    h.deliver_tag(n(2), Tag::StartView);
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(1));
    assert!(!snap(&h, n(2)).faulted);
    assert_eq!(snap(&h, n(2)).committed, 6);
    h.assert_safety();
}
