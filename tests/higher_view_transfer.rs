//! Protocol suite for the qualified transfer path (§10, §13.1 step 5) and
//! the higher-view signal (§10, §13.4).
//!
//! Spec sections: §10 (higher-view messages: proof of staleness vs.
//! installation evidence), §13.1 step 5 (the recipient that cannot
//! construct the offered history fetches it and re-runs the ruling),
//! §13.4 (the bare header is a hint, never evidence), §6.1 (recovery
//! completion through the same suffix ruling), W4/W5 (the transport
//! budget is host policy; a partial answer is well-formed).
//!
//! The properties pinned here:
//!
//! 1.  a higher-view `Prepare` from the legitimate primary of the view it
//!     names pulls a laggard into the qualified transfer path: it ceases
//!     lower-view participation, fetches the missing range, and installs
//!     from the qualified `StartView` — the message itself installs
//!     nothing;
//! 2.  a bare higher-view hint with no qualified evidence installs
//!     nothing and faults nothing; the node awaits evidence;
//! 3.  a forged `NewState` — wrong view, non-contiguous range, regressed
//!     committed frontier, unsolicited sender — is a named drop and
//!     never a fault;
//! 4.  the active fetch is chunked under the host's transport budget
//!     with a `more` cursor, resumes from the cursor, and tolerates
//!     duplicates and reordering;
//! 5.  a recovery whose evidence suffix was budget-truncated fetches the
//!     missing range and completes on an ordinary tick, closing the
//!     budget-truncation wait;
//! 6.  a `StartView` whose suffix gapped at the fence re-runs its ruling
//!     on an ordinary tick once the fetched range has arrived — no
//!     repeated offer from the primary is required;
//! 7.  a `NewState` chunk lost mid-stream re-issues the fetch from its
//!     cursor on an ordinary tick — the fetch never stalls silently;
//! 8.  a chunk answering a fetch whose range a `StartView` has since
//!     installed is a named drop or a harmless close, never a
//!     mis-install.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome};
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::{ProgressSnapshot, Status};
use vrr::replica::ViewChangeKnobs;
use vrr::wire::{Header, Pack, Tag};

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

/// An operation log entry at `slot` — the era-1 genesis kind every
/// pre-reconfiguration journal holds.
fn operation_entry(slot: u64, lsb: u64, payload: &[u8]) -> LogEntry {
    LogEntry {
        slot: Slot(slot),
        era: Era(1),
        payload: Payload::Operation {
            id: op_id(lsb),
            payload: payload.to_vec().into_boxed_slice(),
        },
    }
}

/// A `NewState` chunk, shaped for injection: the header slot names the
/// covered range's end, matching the per-tag slot table.
fn new_state(
    view: ViewId,
    entries: Vec<LogEntry>,
    through: Slot,
    committed: Slot,
    more: bool,
) -> Message {
    Message {
        header: Header {
            tag: Tag::NewState,
            view,
            slot: through,
        },
        body: Body::NewState {
            entries,
            through,
            committed,
            more,
        },
    }
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

/// Drives one proposal at `primary` to full commitment at every
/// reachable node and applies it where the commit landed.
fn commit_one(h: &mut Harness, primary: NodeId, lsb: u64, payload: &[u8]) {
    h.propose(primary, op_id(lsb), payload);
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
}

/// Drives `suspect` past the primary timeout so it enters view change
/// for `target`, and lets the fence quorum form among the reachable
/// nodes.
fn tick_into_view_change(h: &mut Harness, suspect: NodeId, target: ViewId) {
    for _ in 0..=TIMEOUT {
        h.tick(suspect);
    }
    assert_eq!(status_of(h, suspect), Status::ViewChange);
    assert_eq!(current_view(h, suspect), target);
}

fn apply_all(h: &mut Harness, ids: impl IntoIterator<Item = u32>) {
    for id in ids {
        h.execute_apply_effects(n(id));
    }
}

// ---------------------------------------------------------------------------
// 1. A qualified higher-view Prepare pulls the laggard into the transfer
//    path: fence, fetch, install from the qualified StartView.
// ---------------------------------------------------------------------------

#[test]
fn higher_view_prepare_pulls_laggard_into_qualified_transfer() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.partition(vec![n(0), n(1)], vec![n(2)]);
    commit_one(&mut h, n(0), 1, b"a"); // slot 3 commits on the majority side
    tick_into_view_change(&mut h, n(1), view(1));
    h.deliver_all(); // the change completes among n0, n1; n2's copies held
    assert_eq!(status_of(&h, n(1)), Status::Normal);
    assert_eq!(current_view(&h, n(1)), view(1));
    // The new primary serves an operation: slot 4.
    h.propose(n(1), op_id(2), b"b");
    h.deliver_all();
    apply_all(&mut h, [0, 1]);
    h.heal();

    // The laggard meets the higher-view Prepare before the StartView
    // queued ahead of it (§10): the sender is the legitimate primary of
    // the advertised view, so the message proves n2 stale.
    let higher_prepare = Message {
        header: Header {
            tag: Tag::Prepare,
            view: view(1),
            slot: Slot(4),
        },
        body: Body::Prepare {
            entry: operation_entry(4, 2, b"b"),
            committed: Slot(3),
        },
    };
    let outcome = h.inject(n(1), n(2), higher_prepare);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the qualified signal is processed, not refused: {outcome:?}"
    );
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    assert_eq!(
        current_view(&h, n(2)),
        view(1),
        "the laggard fences into the advertised view"
    );
    assert_eq!(
        snap(&h, n(2)).accepted,
        2,
        "the signal installed nothing: a §13.4 hint is never installation evidence"
    );

    // The fence asks the sender for the history the node lacks (§13.1
    // step 5): one past the local frontier, the frontier in the header
    // slot.
    let request = h
        .peek_queued(n(1), Tag::GetState)
        .expect("the laggard fetches from the sender");
    let Body::GetState { from } = &request.body else {
        panic!("expected GetState");
    };
    assert_eq!(*from, Slot(3));
    assert_eq!(request.header.view, view(1));
    assert_eq!(request.header.slot, Slot(2));

    // The laggard ceased lower-view participation: the old view's
    // Prepare is refused with the named mismatch and never answered.
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(3))
        .expect("the old-view Prepare is queued");
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::ViewMismatch {
            got: view(0),
            current: view(1),
        })
    );
    assert!(
        h.peek_queued(n(0), Tag::PrepareOk).is_none(),
        "no acknowledgement for the old view"
    );

    // The qualified evidence — the in-flight StartView — installs first:
    // it was packed when the change completed, so it names the selected
    // history through slot 3 with committed frontier 3.
    h.deliver_tag(n(2), Tag::StartView);
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(1));
    h.execute_apply_effects(n(2));
    assert_eq!(snap(&h, n(2)).accepted, 3);
    assert_eq!(snap(&h, n(2)).committed, 3);

    // The fetch is answered now: a current-view transfer at a Normal
    // node — the history and the committed frontier both move (§13.3's
    // frontier rides the chunk).
    h.deliver_tag(n(1), Tag::GetState);
    let chunk = h
        .peek_queued(n(2), Tag::NewState)
        .expect("the primary serves the range");
    let Body::NewState { through, more, .. } = &chunk.body else {
        panic!("expected NewState");
    };
    assert_eq!(*through, Slot(4));
    assert!(!more, "the chunk reaches the responder's frontier");
    assert_eq!(chunk.header.slot, Slot(4));
    h.deliver_tag(n(2), Tag::NewState);
    assert_eq!(
        snap(&h, n(2)).accepted,
        4,
        "the fetched history filled the gap"
    );
    h.execute_apply_effects(n(2));
    assert_eq!(snap(&h, n(2)).committed, 4);
    assert_eq!(
        h.journal_entries(n(2)),
        h.journal_entries(n(1)),
        "the laggard holds the primary's history"
    );

    // ... and participates in the new view's next round.
    h.drop_queued(n(2)); // the stale view-0 traffic is spent
    h.propose(n(1), op_id(3), b"c");
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(5))
        .expect("the new Prepare reaches n2");
    assert!(
        h.peek_queued(n(1), Tag::PrepareOk).is_some(),
        "the laggard participates in the new view"
    );
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    assert_eq!(snap(&h, n(2)).committed, 5);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 2. A bare higher-view hint is never installation evidence.
// ---------------------------------------------------------------------------

#[test]
fn bare_higher_view_hint_is_never_installation_evidence() {
    let mut h = cluster();
    bootstrap(&mut h);
    commit_one(&mut h, n(0), 1, b"a");

    // A Commit advertising view 2 from n1 — but view 2's primary under
    // the genesis order is n2. Nothing qualifies this header as the
    // view's evidence (§13.4): no fence, no fetch, no install, no fault.
    let hint = Message {
        header: Header {
            tag: Tag::Commit,
            view: view(2),
            slot: Slot(3),
        },
        body: Body::Commit { committed: Slot(3) },
    };
    let outcome = h.inject(n(1), n(0), hint);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::SenderNotPrimary {
            sender: n(1),
            view: view(2),
        }),
        "the unqualified hint is a named drop"
    );
    assert_eq!(status_of(&h, n(0)), Status::Normal);
    assert_eq!(current_view(&h, n(0)), view(0));
    assert_eq!(snap(&h, n(0)).committed, 3);
    assert_eq!(snap(&h, n(0)).accepted, 3);
    assert!(!snap(&h, n(0)).faulted);
    assert!(
        h.peek_queued(n(1), Tag::GetState).is_none()
            && h.peek_queued(n(2), Tag::GetState).is_none(),
        "an unqualified hint never opens a fetch"
    );
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 3. A forged NewState is rejected — and is never a fault.
// ---------------------------------------------------------------------------

#[test]
fn forged_new_state_is_rejected_and_never_faults() {
    let mut h = cluster();
    bootstrap(&mut h);

    // Drive n2 into the qualified transfer path for view 1: a higher-view
    // Prepare from the legitimate primary fences it and opens the fetch.
    let higher_prepare = Message {
        header: Header {
            tag: Tag::Prepare,
            view: view(1),
            slot: Slot(3),
        },
        body: Body::Prepare {
            entry: operation_entry(3, 7, b"x"),
            committed: Slot(2),
        },
    };
    h.inject(n(1), n(2), higher_prepare);
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    assert!(h.peek_queued(n(1), Tag::GetState).is_some());

    // Wrong view: the chunk answers no fetch the node opened.
    h.inject(
        n(1),
        n(2),
        new_state(
            view(2),
            vec![operation_entry(3, 7, b"x")],
            Slot(3),
            Slot(2),
            false,
        ),
    );
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::StaleTransfer {
            sender: n(1),
            view: view(2),
        })
    );

    // A range with a hole: not a contiguous ascending run ending at the
    // header slot (§13.1's shape rule).
    h.inject(
        n(1),
        n(2),
        new_state(
            view(1),
            vec![operation_entry(3, 7, b"x"), operation_entry(5, 9, b"z")],
            Slot(5),
            Slot(2),
            true,
        ),
    );
    assert_eq!(h.diagnostic(n(2)), Some(Diagnostic::MalformedTransfer));

    // A regressed committed frontier: shaped like knowledge the node
    // already holds.
    h.inject(
        n(1),
        n(2),
        new_state(
            view(1),
            vec![operation_entry(3, 7, b"x")],
            Slot(3),
            Slot(1),
            false,
        ),
    );
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::StaleEvidence {
            got: view(1),
            current: view(1),
        })
    );

    // An unsolicited sender: the open fetch named n1, not n0.
    h.inject(
        n(0),
        n(2),
        new_state(
            view(1),
            vec![operation_entry(3, 7, b"x")],
            Slot(3),
            Slot(2),
            false,
        ),
    );
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::StaleTransfer {
            sender: n(0),
            view: view(1),
        })
    );

    // After every forgery: nothing installed, nothing faulted.
    assert_eq!(snap(&h, n(2)).accepted, 2, "no forged chunk installed");
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    assert!(
        !snap(&h, n(2)).faulted,
        "a forged NewState is a drop, never a fault"
    );
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 4. Chunked transfer under the host budget: `more` cursor, resume,
//    duplicates and reordering.
// ---------------------------------------------------------------------------

#[test]
fn chunked_transfer_resumes_from_cursor_and_tolerates_reordering() {
    let per_entry = operation_entry(3, 1, b"c").packed_len();
    let mut h = Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: per_entry,
        },
    );
    bootstrap(&mut h);
    h.partition(vec![n(0), n(1)], vec![n(2)]);
    for lsb in 1..=3u64 {
        h.propose(n(0), op_id(lsb), b"c");
        h.deliver_all();
    }
    apply_all(&mut h, [0, 1]);
    assert_eq!(snap(&h, n(0)).committed, 5);
    h.heal();

    // The laggard meets the newest Prepare first: a gap it cannot close
    // locally — and the gap now fetches (§13.1 step 5).
    let newest = Message {
        header: Header {
            tag: Tag::Prepare,
            view: view(0),
            slot: Slot(5),
        },
        body: Body::Prepare {
            entry: operation_entry(5, 3, b"c"),
            committed: Slot(5),
        },
    };
    h.inject(n(0), n(2), newest.clone());
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::GapDetected {
            expected: Slot(3),
            got: Slot(5),
        })
    );
    let request = h
        .peek_queued(n(0), Tag::GetState)
        .expect("the gap fetches from the primary");
    let Body::GetState { from } = &request.body else {
        panic!("expected GetState");
    };
    assert_eq!(*from, Slot(3), "the fetch resumes one past the frontier");
    assert_eq!(request.header.slot, Slot(2));

    // Chunk 1: exactly one entry under the budget, `more` set, the
    // committed frontier riding along.
    h.deliver_tag(n(0), Tag::GetState);
    let chunk = h.peek_queued(n(2), Tag::NewState).expect("the first chunk");
    let Body::NewState {
        entries,
        through,
        committed,
        more,
    } = &chunk.body
    else {
        panic!("expected NewState");
    };
    let bytes: usize = entries.iter().map(Pack::packed_len).sum();
    assert!(
        bytes <= per_entry,
        "the chunk never exceeds the budget by a byte (W4)"
    );
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].slot, Slot(3));
    assert_eq!(*through, Slot(3));
    assert!(more, "the responder's frontier sits past the chunk");
    assert_eq!(*committed, Slot(5));
    assert_eq!(chunk.header.slot, Slot(3));
    h.deliver_tag(n(2), Tag::NewState);
    assert_eq!(snap(&h, n(2)).accepted, 3);
    h.execute_apply_effects(n(2));
    assert_eq!(
        snap(&h, n(2)).committed,
        3,
        "the current-view transfer moves the committed frontier"
    );

    // `more` re-issues the fetch from the cursor.
    let resume = h
        .peek_queued(n(0), Tag::GetState)
        .expect("the fetch resumes from the cursor");
    let Body::GetState { from } = &resume.body else {
        panic!("expected GetState");
    };
    assert_eq!(*from, Slot(4));
    assert_eq!(resume.header.slot, Slot(3));

    // A reordered chunk waits for its prefix: the gap is named, nothing
    // installs, the fetch stays open.
    h.inject(
        n(0),
        n(2),
        new_state(
            view(0),
            vec![operation_entry(5, 3, b"c")],
            Slot(5),
            Slot(5),
            false,
        ),
    );
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::GapDetected {
            expected: Slot(4),
            got: Slot(5),
        })
    );
    assert_eq!(snap(&h, n(2)).accepted, 3);

    // A duplicate chunk — a range the node already holds whole — is a
    // named stale drop.
    h.inject(
        n(0),
        n(2),
        new_state(
            view(0),
            vec![operation_entry(3, 1, b"c")],
            Slot(3),
            Slot(5),
            true,
        ),
    );
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::StaleTransfer {
            sender: n(0),
            view: view(0),
        })
    );
    assert_eq!(snap(&h, n(2)).accepted, 3);

    // The remaining chunks resume from the cursor and close the fetch.
    h.deliver_tag(n(0), Tag::GetState); // chunk 2
    h.deliver_tag(n(2), Tag::NewState);
    assert_eq!(snap(&h, n(2)).accepted, 4);
    h.deliver_tag(n(0), Tag::GetState); // chunk 3
    let chunk = h.peek_queued(n(2), Tag::NewState).expect("the final chunk");
    let Body::NewState { through, more, .. } = &chunk.body else {
        panic!("expected NewState");
    };
    assert_eq!(*through, Slot(5));
    assert!(!more, "the chunk reaches the responder's frontier");
    h.deliver_tag(n(2), Tag::NewState);
    assert_eq!(snap(&h, n(2)).accepted, 5);
    h.execute_apply_effects(n(2));
    assert_eq!(snap(&h, n(2)).committed, 5);
    assert_eq!(
        h.journal_entries(n(2)),
        h.journal_entries(n(0)),
        "the laggard holds the primary's history"
    );

    // The Prepare that exposed the gap now meets held history: an
    // idempotent re-acknowledgement.
    h.inject(n(0), n(2), newest);
    assert_eq!(h.diagnostic(n(2)), Some(Diagnostic::None));
    assert!(
        h.peek_queued(n(0), Tag::PrepareOk).is_some(),
        "the held entry is re-acknowledged"
    );
    h.drop_queued(n(2));
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 5. Recovery with budget-truncated evidence fetches and completes on a
//    tick — the budget-truncation wait is closed.
// ---------------------------------------------------------------------------

#[test]
fn recovery_with_truncated_evidence_fetches_and_completes() {
    let per_entry = operation_entry(3, 1, b"c").packed_len();
    let mut h = Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: per_entry,
        },
    );
    bootstrap(&mut h);
    h.partition(vec![n(0), n(1)], vec![n(2)]);
    for lsb in 1..=3u64 {
        h.propose(n(0), op_id(lsb), b"c");
        h.deliver_all();
    }
    apply_all(&mut h, [0, 1]);
    h.heal();
    h.drop_queued(n(2)); // n2 stays at genesis: slots 3–5 passed it by

    h.crash(n(2));
    h.restart_with(n(2)).expect("the journal survived");
    h.recover(n(2));
    h.deliver_to(n(0));
    h.deliver_to(n(1));
    // n0's evidence suffix packs the newest entry the budget admits;
    // n1's response completes the quorum and the ruling sees the gap —
    // which now fetches from the responder instead of stalling.
    h.deliver_to(n(2));
    h.deliver_to(n(2));
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::GapDetected {
            expected: Slot(3),
            got: Slot(5),
        })
    );
    assert_eq!(
        status_of(&h, n(2)),
        Status::Recovering,
        "still fenced: the fetch is history, not the completing ruling"
    );
    let request = h
        .peek_queued(n(0), Tag::GetState)
        .expect("the recovery gap fetches");
    let Body::GetState { from } = &request.body else {
        panic!("expected GetState");
    };
    assert_eq!(*from, Slot(3));
    assert_eq!(request.header.view, view(0));

    // The chunks fill the range; the committed frontier does not move
    // while the node is fenced.
    for (slot, more) in [(3u64, true), (4, true), (5, false)] {
        h.deliver_tag(n(0), Tag::GetState);
        let chunk = h.peek_queued(n(2), Tag::NewState).expect("the chunk");
        let Body::NewState {
            through,
            more: flag,
            ..
        } = &chunk.body
        else {
            panic!("expected NewState");
        };
        assert_eq!(*through, Slot(slot));
        assert_eq!(*flag, more);
        h.deliver_tag(n(2), Tag::NewState);
        assert_eq!(snap(&h, n(2)).accepted, slot);
    }
    assert_eq!(
        snap(&h, n(2)).committed,
        2,
        "fetched history never moves the committed frontier of a fenced node"
    );

    // The stalled completion re-runs on an ordinary tick now that the
    // range has arrived (§13.1 step 5): the same evidence, the same
    // ruling — now constructible.
    h.tick(n(2));
    assert_eq!(status_of(&h, n(2)), Status::Replaying);
    assert_eq!(snap(&h, n(2)).committed, 5);
    h.execute_apply_effects(n(2));
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(
        h.journal_entries(n(2)),
        h.journal_entries(n(0)),
        "the recovered node holds the primary's history"
    );
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 6. A StartView whose suffix gapped at the fence re-runs its ruling on an
//    ordinary tick once the fetched range has arrived — no repeated offer
//    from the primary is required.
// ---------------------------------------------------------------------------

#[test]
fn start_view_gap_install_redrives_on_tick() {
    let per_entry = operation_entry(3, 1, b"c").packed_len();
    let mut h = Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: per_entry,
        },
    );
    bootstrap(&mut h);
    h.partition(vec![n(0), n(1)], vec![n(2)]);
    for lsb in 1..=3u64 {
        h.propose(n(0), op_id(lsb), b"c");
        h.deliver_all();
    }
    apply_all(&mut h, [0, 1]);
    assert_eq!(snap(&h, n(0)).committed, 5);
    h.heal();
    h.drop_queued(n(2)); // n2 stays at genesis: slots 3–5 passed it by

    // The change completes among {n0, n1}: n1 is view 1's primary, and
    // its StartView suffix packs the newest entry the budget admits —
    // slot 5 alone. n2 joins the fence, then rules the offer a gap: it
    // cannot construct history from a suffix that starts at slot 5.
    tick_into_view_change(&mut h, n(1), view(1));
    h.deliver_tag(n(0), Tag::StartViewChange); // n0 joins and reports
    h.deliver_tag(n(1), Tag::StartViewChange); // n0's vote completes n1's fence
    h.deliver_tag(n(1), Tag::DoViewChange); // n0's evidence: n1 wins
    assert_eq!(status_of(&h, n(1)), Status::Normal);
    h.deliver_tag(n(2), Tag::StartViewChange); // n2 joins the fence
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    h.deliver_tag(n(2), Tag::StartView); // the truncated suffix gaps
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::GapDetected {
            expected: Slot(3),
            got: Slot(5),
        })
    );
    let request = h
        .peek_queued(n(1), Tag::GetState)
        .expect("the gap fetches from the new primary");
    let Body::GetState { from } = &request.body else {
        panic!("expected GetState");
    };
    assert_eq!(*from, Slot(3));

    // The fetch fills the range chunk by chunk; the committed frontier
    // waits for the completing ruling while the node is fenced.
    for slot in 3..=5u64 {
        h.deliver_tag(n(1), Tag::GetState);
        let chunk = h.peek_queued(n(2), Tag::NewState).expect("the chunk");
        let Body::NewState {
            through,
            more: flag,
            ..
        } = &chunk.body
        else {
            panic!("expected NewState");
        };
        assert_eq!(*through, Slot(slot));
        assert_eq!(*flag, slot < 5);
        h.deliver_tag(n(2), Tag::NewState);
        assert_eq!(snap(&h, n(2)).accepted, slot);
    }
    assert_eq!(
        snap(&h, n(2)).committed,
        2,
        "a fenced node's committed frontier waits for the completing ruling"
    );
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);

    // The completing ruling re-runs on an ordinary tick now that the
    // range has arrived (§13.1 step 5): the same offer, now
    // constructible. No repeated StartView was sent — the primary moved
    // on after its broadcast.
    h.tick(n(2));
    assert_eq!(
        status_of(&h, n(2)),
        Status::Normal,
        "the gap-ruled install completes on the tick"
    );
    assert_eq!(current_view(&h, n(2)), view(1));
    assert_eq!(snap(&h, n(2)).committed, 5);
    h.execute_apply_effects(n(2));
    assert_eq!(snap(&h, n(2)).applied, 5);
    assert_eq!(
        h.journal_entries(n(2)),
        h.journal_entries(n(1)),
        "the installed history is the selected one"
    );
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 7. A NewState chunk lost mid-stream re-issues the fetch from its cursor
//    on an ordinary tick — the fetch never stalls silently.
// ---------------------------------------------------------------------------

#[test]
fn lost_transfer_chunk_reissues_get_state_on_tick() {
    let per_entry = operation_entry(3, 1, b"c").packed_len();
    let mut h = Harness::with_knobs(
        3,
        ViewChangeKnobs {
            // Suspicion stays out of the way: the script's ticks ask about
            // the fetch, never about the primary's life.
            primary_timeout: 100,
            view_change_budget: per_entry,
        },
    );
    bootstrap(&mut h);
    h.partition(vec![n(0), n(1)], vec![n(2)]);
    for lsb in 1..=3u64 {
        h.propose(n(0), op_id(lsb), b"c");
        h.deliver_all();
    }
    apply_all(&mut h, [0, 1]);
    assert_eq!(snap(&h, n(0)).committed, 5);
    h.heal();

    // The newest Prepare exposes the gap; the fetch opens from one past
    // the local frontier (§13.1 step 5).
    let newest = Message {
        header: Header {
            tag: Tag::Prepare,
            view: view(0),
            slot: Slot(5),
        },
        body: Body::Prepare {
            entry: operation_entry(5, 3, b"c"),
            committed: Slot(5),
        },
    };
    h.inject(n(0), n(2), newest);
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::GapDetected {
            expected: Slot(3),
            got: Slot(5),
        })
    );

    // The first chunk lands; the resume request is answered — and the
    // answer is lost, with the rest of the stranded view-0 traffic.
    h.deliver_tag(n(0), Tag::GetState);
    h.deliver_tag(n(2), Tag::NewState); // slot 3, `more` set
    assert_eq!(snap(&h, n(2)).accepted, 3);
    h.deliver_tag(n(0), Tag::GetState); // the resume asks from slot 4
    assert!(
        h.drop_queued(n(2)) > 0,
        "the second chunk never arrives: lost in transit"
    );
    assert_eq!(snap(&h, n(2)).accepted, 3);

    // An ordinary tick re-issues the fetch from the cursor: the request
    // names slot 4, not a restart from slot 3, and the cursor never
    // advanced without the chunk.
    h.tick(n(2));
    let retry = h
        .peek_queued(n(0), Tag::GetState)
        .expect("a lost chunk re-issues the fetch on an ordinary tick");
    let Body::GetState { from } = &retry.body else {
        panic!("expected GetState");
    };
    assert_eq!(*from, Slot(4), "the retry resumes from the cursor");
    assert_eq!(retry.header.slot, Slot(3));
    assert_eq!(
        snap(&h, n(2)).accepted,
        3,
        "the cursor never advanced without the chunk"
    );

    // The retried stream completes: chunk by chunk from the cursor, then
    // the fetch closes and asks nothing more.
    h.deliver_tag(n(0), Tag::GetState);
    h.deliver_tag(n(2), Tag::NewState); // slot 4
    assert_eq!(snap(&h, n(2)).accepted, 4);
    h.deliver_tag(n(0), Tag::GetState);
    h.deliver_tag(n(2), Tag::NewState); // slot 5, `more` clear
    assert_eq!(snap(&h, n(2)).accepted, 5);
    h.execute_apply_effects(n(2));
    assert_eq!(snap(&h, n(2)).committed, 5);
    assert_eq!(
        h.journal_entries(n(2)),
        h.journal_entries(n(0)),
        "the laggard holds the primary's history"
    );
    h.tick(n(2));
    assert!(
        h.peek_queued(n(0), Tag::GetState).is_none(),
        "a closed fetch asks nothing"
    );
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 8. A chunk answering a fetch whose range a StartView has since installed
//    is a named drop or a harmless close — never a mis-install.
// ---------------------------------------------------------------------------

#[test]
fn superseded_fetch_chunk_is_a_named_drop() {
    let per_entry = operation_entry(3, 1, b"c").packed_len();
    let mut h = Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: per_entry,
        },
    );
    bootstrap(&mut h);
    h.partition(vec![n(0), n(1)], vec![n(2)]);
    commit_one(&mut h, n(0), 1, b"a"); // slot 3 on the majority side
    tick_into_view_change(&mut h, n(1), view(1));
    h.deliver_all();
    h.propose(n(1), op_id(2), b"b"); // slot 4 on the majority side
    h.deliver_all();
    apply_all(&mut h, [0, 1]);
    h.heal();

    // The qualified higher-view signal fences n2 and opens the fetch
    // (§10, §13.1 step 5); the responder's first chunk goes in flight.
    let higher_prepare = Message {
        header: Header {
            tag: Tag::Prepare,
            view: view(1),
            slot: Slot(4),
        },
        body: Body::Prepare {
            entry: operation_entry(4, 2, b"b"),
            committed: Slot(4),
        },
    };
    h.inject(n(1), n(2), higher_prepare);
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    assert!(h.peek_queued(n(1), Tag::GetState).is_some());
    h.deliver_tag(n(1), Tag::GetState);
    let chunk = h
        .peek_queued(n(2), Tag::NewState)
        .expect("the answer is in flight");
    let Body::NewState {
        through,
        more: flag,
        ..
    } = &chunk.body
    else {
        panic!("expected NewState");
    };
    assert_eq!(*through, Slot(3));
    assert!(*flag, "the responder's frontier sits past the chunk");

    // The qualified StartView installs the chunk's whole range first:
    // the suffix the budget admits is slot 3 alone, and n2's frontier is
    // one behind it.
    h.deliver_tag(n(2), Tag::StartView);
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(1));
    assert_eq!(snap(&h, n(2)).accepted, 3);
    h.execute_apply_effects(n(2));
    assert_eq!(snap(&h, n(2)).committed, 3);

    // The in-flight chunk now answers a fetch whose range is already
    // installed: a range the node holds whole, with more claimed — a
    // named stale drop. Nothing installs, nothing faults.
    h.deliver_tag(n(2), Tag::NewState);
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::StaleTransfer {
            sender: n(1),
            view: view(1),
        }),
        "the superseded chunk is a named drop"
    );
    assert_eq!(
        snap(&h, n(2)).accepted,
        3,
        "the stale chunk installed nothing"
    );
    assert_eq!(snap(&h, n(2)).committed, 3);
    assert!(!snap(&h, n(2)).faulted);
    assert_eq!(
        h.journal_entry(n(2), Slot(3)),
        h.journal_entry(n(1), Slot(3)),
        "the installed history is the primary's, untouched by the stale chunk"
    );

    // A duplicate final chunk — no more claimed — is the harmless close:
    // the fetch ends, nothing moves, and a later tick asks nothing.
    h.inject(
        n(1),
        n(2),
        new_state(
            view(1),
            vec![operation_entry(3, 1, b"a")],
            Slot(3),
            Slot(3),
            false,
        ),
    );
    assert_eq!(h.diagnostic(n(2)), Some(Diagnostic::None));
    assert_eq!(snap(&h, n(2)).accepted, 3);
    assert!(!snap(&h, n(2)).faulted);
    h.tick(n(2));
    assert!(
        h.peek_queued(n(1), Tag::GetState).is_none(),
        "the closed fetch asks nothing"
    );
    // The view's ordinary traffic then converges n2 on the frontier.
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(4))
        .expect("the view-1 Prepare is queued");
    h.deliver_to_matching(n(2), Tag::Commit, Slot(4))
        .expect("the view-1 Commit is queued");
    h.execute_apply_effects(n(2));
    assert_eq!(snap(&h, n(2)).committed, 4);
    h.deliver_all();
    apply_all(&mut h, [0, 1, 2]);
    h.assert_safety();
}
