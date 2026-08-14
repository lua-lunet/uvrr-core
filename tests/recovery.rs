//! Recovery protocol integration tests: VRR-2012 §6.1 against spec §6.1,
//! §8.3, §10, §11.1 and §14.2, on the scripted harness.
//!
//! Every test drives a three-node cluster, crashes a backup, reopens it
//! fenced `Recovering`, and drives the solicitation/response exchange by
//! hand. The obligations under test: the nonce is the recovery input's
//! tick (S4), the `R_g` quorum is weighted and never counts the node
//! itself, the latest fenced view's ruling rides the `F_g ⌢ R_g` and
//! `V_g ⌢ V_g` intersections (§8.3), history installs only from the latest
//! fenced view's primary, a completion below `committed` replays the
//! committed-but-unapplied suffix as ordered `Apply` upcalls (§11.1, B2),
//! and a retained-base shortfall surfaces an application-state transfer
//! request (§4) — never a fault.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::INIT_SLOT;
use vrr::effects::Effect;
use vrr::ids::{Era, NodeId, OperationId, Slot, Tick, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::{ProgressSnapshot, Status};
use vrr::replica::{PlanRejection, ViewChangeKnobs};
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

/// An operation identity for the scripts: the host assigns it, the core
/// carries it opaque (§11.1, B2).
fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

/// An operation log entry for a fabricated message.
fn operation_entry(slot: u64, lsb: u64, payload: &[u8]) -> LogEntry {
    LogEntry {
        slot: Slot(slot),
        era: Era(1),
        payload: Payload::Operation {
            id: op_id(lsb),
            payload: payload.into(),
        },
    }
}

/// A fabricated `RecoveryResponse` envelope (the header slot is the Absent
/// sentinel; the frontiers ride in the body).
fn recovery_response(
    nonce: Tick,
    view: ViewId,
    accepted: Slot,
    committed: Slot,
    suffix: Option<Vec<LogEntry>>,
) -> Message {
    Message {
        header: Header {
            tag: Tag::RecoveryResponse,
            view,
            slot: Slot::FIRST,
        },
        body: Body::RecoveryResponse {
            nonce,
            view,
            accepted,
            committed,
            suffix,
        },
    }
}

/// A fabricated `StartViewChange` envelope (the header slot is the Absent
/// sentinel; the body is empty).
fn svc(view: ViewId) -> Message {
    Message {
        header: Header {
            tag: Tag::StartViewChange,
            view,
            slot: Slot::FIRST,
        },
        body: Body::StartViewChange {},
    }
}

/// The timeout knob used by every test in this suite: a backup enters a
/// view change after more than three ticks of primary silence (S4).
const TIMEOUT: u64 = 3;

/// A three-node cluster with the view-change knobs set explicitly.
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
/// backups adopt view (1, 0).
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

/// Drives a complete view change to `target` with all three nodes live:
/// `prime` (which must be `primary_of(target)`) times out first, and one
/// delivery drain carries the fence votes, the evidence, and the
/// `StartView` installs.
fn drive_view_change(h: &mut Harness, prime: NodeId, target: ViewId) {
    assert_eq!(primary_of(target), prime, "the prime must own the target");
    tick_into_view_change(h, prime, target);
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        assert_eq!(status_of(h, id), Status::Normal, "{id:?} installs");
        assert_eq!(current_view(h, id), target);
    }
    h.assert_safety();
}

/// Executes every pending `Apply` at every live node, the way a host
/// would.
fn apply_all(h: &mut Harness, ids: [u32; 3]) {
    for id in ids {
        h.execute_apply_effects(n(id));
    }
}

/// Commits one operation at `primary` (one slot per call, in call order)
/// and applies it everywhere it landed.
fn commit_one(h: &mut Harness, primary: NodeId, lsb: u64, payload: &[u8]) {
    let outcome = h.propose(primary, op_id(lsb), payload);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the primary accepts its own proposal: {outcome:?}"
    );
    h.deliver_all();
    apply_all(h, [0, 1, 2]);
}

/// Crashes a node and reopens it from the recorded disk: the §10 fenced
/// `Recovering` state every recovery scenario starts from.
fn crash_and_reopen(h: &mut Harness, id: NodeId) {
    h.crash(id);
    h.restart_with(id).expect("the disk record reopens");
    assert_eq!(status_of(h, id), Status::Recovering);
}

/// The nonce of the `Recovery` solicitation queued to `to`, asserting one
/// is queued.
fn queued_recovery_nonce(h: &Harness, to: NodeId) -> Tick {
    let message = h
        .peek_queued(to, Tag::Recovery)
        .expect("a Recovery solicitation is queued");
    let Body::Recovery { nonce } = &message.body else {
        panic!("the peeked message is a Recovery");
    };
    *nonce
}

// 1. Happy path: the nonce is the recovery input's tick, the solicitation
//    reaches the rest of the configuration, only weighted `R_g` responses
//    count (never the node's own), a fenced `Recovering` node refuses
//    proposals, and completion returns it to `Normal`.
#[test]
fn happy_path_recovery_completes_from_weighted_quorum() {
    let mut h = cluster();
    bootstrap(&mut h);
    crash_and_reopen(&mut h, n(2));

    // A fenced Recovering node does not participate.
    let refused = h.propose(n(2), op_id(9), b"x");
    assert!(
        matches!(
            refused,
            StepOutcome::PlanRefused(PlanRejection::NotPrimary { .. })
        ),
        "a Recovering node refuses proposals: {refused:?}"
    );

    // The solicitation's nonce IS the input's tick (S4).
    h.recover(n(2));
    let nonce = h.now();
    assert_eq!(queued_recovery_nonce(&h, n(0)), nonce);
    assert_eq!(queued_recovery_nonce(&h, n(1)), nonce);

    // One weighted response is not the `R_g` quorum (2 of 3 required).
    h.deliver_to(n(0)).expect("the solicitation reaches n0");
    h.deliver_to(n(2)).expect("n0's response reaches n2");
    assert_eq!(status_of(&h, n(2)), Status::Recovering);

    // A self-attributed response never counts (§8.3's R_g is other
    // replicas' evidence).
    let outcome = h.inject(
        n(2),
        n(2),
        recovery_response(nonce, view(0), INIT_SLOT, INIT_SLOT, None),
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::RecoveryResponseFromSelf),
    );
    assert_eq!(status_of(&h, n(2)), Status::Recovering);

    // The second distinct responder completes the quorum; genesis state
    // needs no replay, so the node is Normal immediately.
    h.deliver_to(n(1)).expect("the solicitation reaches n1");
    h.deliver_to(n(2)).expect("n1's response reaches n2");
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(0));
    h.assert_safety();
}

// 2. A retry carries a fresh tick — hence a fresh nonce — and a response
//    to the earlier attempt is stale: named, ignored, counted toward
//    nothing.
#[test]
fn stale_nonce_responses_are_ignored() {
    let mut h = cluster();
    bootstrap(&mut h);
    crash_and_reopen(&mut h, n(2));

    h.recover(n(2));
    let first = h.now();
    assert_eq!(queued_recovery_nonce(&h, n(0)), first);
    h.drop_queued(n(0));
    h.drop_queued(n(1));

    // The retry's nonce is the fresh tick, never a caller-supplied value.
    h.recover(n(2));
    let second = h.now();
    assert!(second > first, "every attempt carries a fresh tick");
    assert_eq!(queued_recovery_nonce(&h, n(0)), second);

    // A delayed response to the first attempt is stale (§6.1).
    let outcome = h.inject(
        n(1),
        n(2),
        recovery_response(first, view(0), INIT_SLOT, INIT_SLOT, None),
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::StaleRecoveryResponse {
            nonce: first,
            attempt: Some(second),
        }),
    );

    // n0's fresh response alone cannot complete: had the stale n1 answer
    // counted, the quorum would hold with the primary's suffix and the
    // attempt would complete here.
    h.deliver_to(n(0)).expect("the solicitation reaches n0");
    h.deliver_to(n(2)).expect("n0's response reaches n2");
    assert_eq!(status_of(&h, n(2)), Status::Recovering);

    h.deliver_to(n(1)).expect("the solicitation reaches n1");
    h.deliver_to(n(2)).expect("n1's response reaches n2");
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    h.assert_safety();
}

// 3. A node that fenced view v+1 and voted in its `DoViewChange` before
//    crashing recovers INTO v+1 — the `F_g ⌢ R_g` intersection (§8.3) is
//    what carries the fence knowledge to the responder quorum — installing
//    the view's selected history (its never-committed local tail is
//    replaced) and moving its retained fence: the `V_g ⌢ V_g` obligation
//    that the fence pre-exists the crash.
#[test]
fn recovery_installs_the_latest_fenced_view() {
    let mut h = cluster();
    bootstrap(&mut h);
    commit_one(&mut h, n(0), 1, b"a");

    // n2 alone accepts a fabricated tail the primary never sent: the
    // never-committed local excess a recovery must not keep.
    let z = Message {
        header: Header {
            tag: Tag::Prepare,
            view: view(0),
            slot: Slot(4),
        },
        body: Body::Prepare {
            entry: operation_entry(4, 99, b"z"),
            committed: Slot(3),
        },
    };
    let outcome = h.inject(n(0), n(2), z);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.drop_queued(n(0));

    // The cluster changes view; n2 fences view 1 (the durable vote §14.2
    // relies on) but never sees the StartView.
    tick_into_view_change(&mut h, n(1), view(1));
    h.deliver_to(n(0)).expect("n0 joins the fence");
    h.deliver_to(n(2)).expect("n2 joins the fence");
    h.deliver_to(n(1)).expect("n1 hears n0");
    h.deliver_to(n(1)).expect("n1 hears n0's evidence");
    h.deliver_to_matching(n(0), Tag::StartView, Slot(3))
        .expect("n0 adopts view 1");
    assert_eq!(status_of(&h, n(0)), Status::Normal);
    assert_eq!(status_of(&h, n(1)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(1));
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    h.drop_queued(n(0));
    h.drop_queued(n(1));
    h.drop_queued(n(2));

    // The fence pre-exists the crash: the reopened node opens in view 1.
    crash_and_reopen(&mut h, n(2));
    assert_eq!(current_view(&h, n(2)), view(1));

    h.recover(n(2));
    h.deliver_to(n(0)).expect("the solicitation reaches n0");
    h.deliver_to(n(1)).expect("the solicitation reaches n1");
    h.deliver_to(n(2)).expect("n0's response reaches n2");
    assert_eq!(status_of(&h, n(2)), Status::Recovering);
    h.deliver_to(n(2)).expect("n1's response reaches n2");

    // Recovered into the latest fenced view, with the view's selected
    // history: the never-committed tail is gone, not kept.
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(1));
    assert_eq!(
        h.journal_entries(n(2)),
        h.journal_entries(n(1)),
        "the installed history is the view's selected one",
    );
    assert_eq!(h.journal_entries(n(2)).len(), 3, "the z tail is replaced");
    h.assert_safety();
}

// 4. §14.2's amnesiac-voter guard: a node whose fence is durable does not
//    re-vote in that view — not mid-recovery, and not after recovering
//    into it.
#[test]
fn recovered_node_does_not_revote_in_fenced_view() {
    let mut h = cluster();
    bootstrap(&mut h);
    drive_view_change(&mut h, n(1), view(1));
    crash_and_reopen(&mut h, n(2));

    // Mid-recovery: the fence is durable (current view is the fenced one),
    // so the re-offered vote is stale, not counted.
    h.recover(n(2));
    let outcome = h.inject(n(0), n(2), svc(view(1)));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    assert!(
        matches!(h.diagnostic(n(2)), Some(Diagnostic::StaleViewChange { .. })),
        "the durable fence makes the re-offered vote stale: {:?}",
        h.diagnostic(n(2)),
    );
    assert_eq!(status_of(&h, n(2)), Status::Recovering);
    assert!(
        h.peek_queued(n(0), Tag::StartViewChange).is_none()
            && h.peek_queued(n(1), Tag::StartViewChange).is_none(),
        "no re-vote leaves the node",
    );

    // Complete the recovery into the fenced view.
    h.deliver_to(n(0)).expect("the solicitation reaches n0");
    h.deliver_to(n(1)).expect("the solicitation reaches n1");
    h.deliver_to(n(2)).expect("n0's response reaches n2");
    h.deliver_to(n(2)).expect("n1's response reaches n2");
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(1));

    // Post-recovery: still no re-vote in the view it fenced pre-crash.
    let outcome = h.inject(n(1), n(2), svc(view(1)));
    assert!(
        matches!(outcome, StepOutcome::Published { ref effects, .. } if effects.is_empty()),
        "the re-vote releases nothing: {outcome:?}",
    );
    assert!(
        matches!(h.diagnostic(n(2)), Some(Diagnostic::StaleViewChange { .. })),
        "the recovered node does not re-vote: {:?}",
        h.diagnostic(n(2)),
    );
    assert_eq!(status_of(&h, n(2)), Status::Normal);

    // The guard is fence-target equality, not paralysis: a HIGHER view
    // change engages normally.
    let outcome = h.inject(n(0), n(2), svc(view(2)));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    assert_eq!(current_view(&h, n(2)), view(2));
    h.assert_safety();
}

// 5. History installs only from the latest fenced view's primary: a suffix
//    from any other sender is dropped with the response that carried it,
//    and a quorum without the primary's history keeps waiting.
#[test]
fn history_installs_only_from_the_views_primary() {
    let mut h = cluster();
    bootstrap(&mut h);
    let history = h.journal_entries(n(0));
    crash_and_reopen(&mut h, n(2));
    h.recover(n(2));
    let nonce = h.now();

    // A suffix-carrying response from a non-primary is dropped, not
    // recorded (§6.1: only the reported view's primary carries history).
    let outcome = h.inject(
        n(1),
        n(2),
        recovery_response(nonce, view(0), INIT_SLOT, INIT_SLOT, Some(history.clone())),
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::RecoveryHistoryNotFromPrimary {
            sender: n(1),
            view: view(0),
        }),
    );

    // A quorum of suffix-less answers cannot complete: the installation
    // evidence is the primary's history, and none is on offer.
    h.inject(
        n(0),
        n(2),
        recovery_response(nonce, view(0), INIT_SLOT, INIT_SLOT, None),
    );
    h.inject(
        n(1),
        n(2),
        recovery_response(nonce, view(0), INIT_SLOT, INIT_SLOT, None),
    );
    assert_eq!(
        status_of(&h, n(2)),
        Status::Recovering,
        "the quorum holds but the primary's history has not answered",
    );

    // The primary's refreshed answer (a later response from the same
    // sender supersedes) carries the history: the attempt completes.
    h.inject(
        n(0),
        n(2),
        recovery_response(nonce, view(0), INIT_SLOT, INIT_SLOT, Some(history)),
    );
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    h.assert_safety();
}

// 6. Completion below `committed` leaves the node `Replaying`: the
//    committed-but-unapplied suffix re-emits as ordered `Apply` upcalls
//    (§11.1) until `applied == committed`, and only then does
//    participation resume.
#[test]
fn completion_below_committed_replays_until_caught_up() {
    let mut h = cluster();
    bootstrap(&mut h);
    commit_one(&mut h, n(0), 1, b"a");
    // Slots 4 and 5 commit everywhere but are never applied at n2: the
    // pending upcalls die with the crash.
    for (lsb, payload) in [(2, &b"b"[..]), (3, &b"c"[..])] {
        h.propose(n(0), op_id(lsb), payload);
        h.deliver_all();
        h.execute_apply_effects(n(0));
        h.execute_apply_effects(n(1));
    }
    h.crash(n(2));
    h.restart_with(n(2)).expect("the disk record reopens");
    let reopened = snap(&h, n(2));
    assert_eq!(reopened.committed, 5);
    assert_eq!(reopened.applied, 3);

    h.recover(n(2));
    h.deliver_to(n(0)).expect("the solicitation reaches n0");
    h.deliver_to(n(1)).expect("the solicitation reaches n1");
    h.deliver_to(n(2)).expect("n0's response reaches n2");
    let completing = h.deliver_to(n(2)).expect("n1's response reaches n2");

    let StepOutcome::Published { effects, .. } = completing.outcome else {
        panic!(
            "the completing response publishes: {:?}",
            completing.outcome
        );
    };
    assert_eq!(
        effects,
        vec![
            Effect::Apply {
                slot: Slot(4),
                operation_id: op_id(2),
                payload: b"b"[..].into(),
            },
            Effect::Apply {
                slot: Slot(5),
                operation_id: op_id(3),
                payload: b"c"[..].into(),
            },
        ],
        "the committed-but-unapplied suffix re-emits in slot order",
    );
    assert_eq!(status_of(&h, n(2)), Status::Replaying);

    // Each application step advances the replay; the last one flips the
    // node to Normal (applied == committed).
    let outcomes = h.execute_apply_effects(n(2));
    assert_eq!(outcomes.len(), 2);
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    let caught_up = snap(&h, n(2));
    assert_eq!(caught_up.applied, 5);
    assert_eq!(caught_up.committed, 5);

    // Participation resumes: n2 backs the next proposal.
    commit_one(&mut h, n(0), 4, b"d");
    h.assert_safety();
}

// 7. Retained-base shortfall: the recovery must read history the journal
//    physically let go (S1), so the host's application-state transfer
//    facility is asked for it (§4, §11) — surfaced, never faulted.
#[test]
fn retained_base_shortfall_requests_application_state() {
    let mut h = cluster();
    bootstrap(&mut h);
    commit_one(&mut h, n(0), 1, b"a");
    commit_one(&mut h, n(0), 2, b"b");

    // The host's retention policy lets everything below slot 4 go while
    // the logical frontier stands.
    h.crash_with_retained_base(n(2), Slot(4));
    h.restart_with(n(2))
        .expect("the truncated disk record reopens");

    h.recover(n(2));
    h.deliver_to(n(0)).expect("the solicitation reaches n0");
    h.deliver_to(n(1)).expect("the solicitation reaches n1");
    h.deliver_to(n(2)).expect("n0's response reaches n2");
    let completing = h.deliver_to(n(2)).expect("n1's response reaches n2");

    let StepOutcome::Published { effects, .. } = completing.outcome else {
        panic!(
            "the completing response publishes: {:?}",
            completing.outcome
        );
    };
    assert_eq!(
        effects,
        vec![Effect::RequestApplicationState { through: Slot(4) }],
        "the shortfall asks the host's transfer facility, covering committed",
    );
    assert_eq!(
        h.application_state_requests(),
        &[(n(2), Slot(4))],
        "the host-visible record agrees",
    );
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::ApplicationStateShortfall {
            required: Slot(1),
            retained: Slot(4),
        }),
    );
    // Surfaced, not faulted: the node is still fenced Recovering, and the
    // safety gate (which trips on unaccounted faults) passes.
    assert_eq!(status_of(&h, n(2)), Status::Recovering);
    h.assert_safety();
}

// 8. Amnesia (§14.2): a node that lost everything — the state the
//    amnesiac-voter guard cannot help — reconstructs purely from the
//    responders: the latest fenced view, its history, and the replay of
//    everything committed-but-unapplied.
#[test]
fn amnesiac_reconstructs_from_responder_history() {
    let mut h = cluster();
    bootstrap(&mut h);
    commit_one(&mut h, n(0), 1, b"a");
    drive_view_change(&mut h, n(1), view(1));
    commit_one(&mut h, n(1), 2, b"b");

    h.crash(n(2));
    h.restart_amnesiac(n(2)).expect("genesis provisions");
    assert_eq!(status_of(&h, n(2)), Status::Recovering);
    assert_eq!(current_view(&h, n(2)), view(0));

    h.recover(n(2));
    h.deliver_to(n(0)).expect("the solicitation reaches n0");
    h.deliver_to(n(1)).expect("the solicitation reaches n1");
    h.deliver_to(n(2)).expect("n0's response reaches n2");
    let completing = h.deliver_to(n(2)).expect("n1's response reaches n2");

    let StepOutcome::Published { effects, .. } = completing.outcome else {
        panic!(
            "the completing response publishes: {:?}",
            completing.outcome
        );
    };
    assert_eq!(
        effects,
        vec![
            Effect::Apply {
                slot: Slot(3),
                operation_id: op_id(1),
                payload: b"a"[..].into(),
            },
            Effect::Apply {
                slot: Slot(4),
                operation_id: op_id(2),
                payload: b"b"[..].into(),
            },
        ],
        "an amnesiac re-applies the whole committed history (§11's at-least-once)",
    );
    assert_eq!(status_of(&h, n(2)), Status::Replaying);
    assert_eq!(current_view(&h, n(2)), view(1));
    assert_eq!(
        h.journal_entries(n(2)),
        h.journal_entries(n(1)),
        "the reconstructed history is the responders'",
    );

    h.execute_apply_effects(n(2));
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    let recovered = snap(&h, n(2));
    assert_eq!(recovered.applied, 4);
    assert_eq!(recovered.committed, 4);
    h.assert_safety();
}

#[test]
fn recovery_tick_redrive_after_view_advance() {
    let budget = operation_entry(3, 1, b"c").packed_len();
    let mut h = Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: budget,
        },
    );
    bootstrap(&mut h);
    h.crash(n(2));
    tick_into_view_change(&mut h, n(1), view(1));
    h.deliver_all();
    for id in [n(0), n(1)] {
        assert_eq!(status_of(&h, id), Status::Normal);
        assert_eq!(current_view(&h, id), view(1));
    }
    h.assert_safety();
    for lsb in 1..=3u64 {
        commit_one(&mut h, n(1), lsb, b"c");
    }

    h.restart_with(n(2)).expect("the journal survived");
    h.recover(n(2));
    h.deliver_to(n(0));
    h.deliver_to(n(1));
    h.deliver_to(n(2));
    h.deliver_to(n(2));
    assert_eq!(status_of(&h, n(2)), Status::Recovering);
    assert_eq!(
        h.diagnostic(n(2)),
        Some(Diagnostic::GapDetected {
            expected: Slot(3),
            got: Slot(5),
        })
    );

    for expected_through in 3..=5u64 {
        h.deliver_tag(n(1), Tag::GetState);
        let chunk = h.peek_queued(n(2), Tag::NewState).expect("state chunk");
        let Body::NewState { through, .. } = &chunk.body else {
            panic!("expected NewState");
        };
        assert_eq!(*through, Slot(expected_through));
        h.deliver_tag(n(2), Tag::NewState);
    }

    h.tick(n(2));
    assert_eq!(h.fault_of(n(2)), None);
    assert_eq!(current_view(&h, n(2)), view(1));
    assert!(matches!(
        status_of(&h, n(2)),
        Status::Normal | Status::Replaying
    ));
    h.execute_apply_effects(n(2));
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    h.assert_safety();
}

#[test]
fn same_view_recovery_preserves_the_primary_accepted_tail() {
    let mut h = cluster();
    bootstrap(&mut h);

    let proposed = h.propose(n(0), op_id(1), b"tail");
    assert!(matches!(proposed, StepOutcome::Published { .. }));
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(3))
        .expect("one backup accepts the tail");
    h.drop_queued(n(0));
    h.drop_queued(n(1));
    assert_eq!(snap(&h, n(0)).accepted, 3);
    assert_eq!(snap(&h, n(1)).accepted, 2);
    assert_eq!(snap(&h, n(2)).accepted, 3);
    assert_eq!(snap(&h, n(0)).committed, 2);

    h.crash(n(2));
    h.restart_with(n(2)).expect("the accepted journal survived");
    h.recover(n(2));
    h.deliver_to(n(0));
    h.deliver_to(n(1));
    h.deliver_to(n(2));
    h.deliver_to(n(2));

    assert_eq!(h.fault_of(n(2)), None);
    assert_eq!(current_view(&h, n(2)), view(0));
    assert_eq!(snap(&h, n(2)).accepted, 3);
    assert_eq!(snap(&h, n(2)).committed, 2);
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    h.assert_safety();
}
