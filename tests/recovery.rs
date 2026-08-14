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
use vrr::effects::{Effect, Stability, StabilityResult};
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
            slot: Slot::NONE,
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
            slot: Slot::NONE,
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

/// Stages an accepted-but-uncommitted tail at `straggler`: each operation
/// commits at the primary and `voters` through the ordinary pipeline, and
/// lands in the straggler's journal under a fabricated Prepare whose
/// piggybacked committed frontier is pinned at the genesis one — the
/// straggler ACCEPTS every entry but never learns that any of them
/// committed. The straggler's answering PrepareOks and every leftover
/// datagram are drained each round, so the queue starts empty per op.
fn stage_accepted_tail(
    h: &mut Harness,
    straggler: NodeId,
    voters: &[NodeId],
    ops: &[(u64, &[u8])],
) {
    for (lsb, payload) in ops {
        let proposed = h.propose(n(0), op_id(*lsb), payload);
        assert!(
            matches!(proposed, StepOutcome::Published { .. }),
            "the primary accepts its own proposal: {proposed:?}"
        );
        for voter in voters {
            h.deliver_to(*voter).expect("the Prepare reaches a voter");
        }
        for _ in voters {
            h.deliver_to(n(0))
                .expect("a voter's PrepareOk reaches the primary");
        }
        for voter in voters {
            h.deliver_to(*voter).expect("the Commit reaches a voter");
        }
        h.drop_queued(straggler);
        h.execute_apply_effects(n(0));
        for voter in voters {
            h.execute_apply_effects(*voter);
        }
        let slot = lsb + 2;
        let prepare = Message {
            header: Header {
                tag: Tag::Prepare,
                view: view(0),
                slot: Slot(slot),
            },
            body: Body::Prepare {
                entry: operation_entry(slot, *lsb, payload),
                committed: INIT_SLOT,
            },
        };
        let accepted = h.inject(n(0), straggler, prepare);
        assert!(
            matches!(accepted, StepOutcome::Published { .. }),
            "the straggler accepts the entry: {accepted:?}"
        );
        h.deliver_all();
    }
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

// A recovery episode is not one nonce: the host's re-drive carries a
// fresh tick (S4) but the episode's collected evidence survives, bounded,
// and responses to any nonce the episode still remembers combine into
// the one `R_g` quorum. A completion never requires an answer to the
// very latest nonce — the nonce names the episode, not a round.
#[test]
fn recovery_host_redrive_invalidates_inflight_response() {
    let mut h = cluster();
    bootstrap(&mut h);
    crash_and_reopen(&mut h, n(2));

    // T1: n0 answers; its response is queued to n2 but not yet delivered.
    h.recover(n(2));
    let first = h.now();
    assert_eq!(queued_recovery_nonce(&h, n(0)), first);
    h.deliver_to(n(0)).expect("the T1 solicitation reaches n0");
    h.drop_queued(n(1));

    // The host re-drives: a fresh tick, hence a fresh nonce.
    h.recover(n(2));
    let second = h.now();
    assert!(second > first, "every re-drive carries a fresh tick");
    assert_eq!(queued_recovery_nonce(&h, n(1)), second);

    // n0's in-flight T1 response finally lands: superseded, but still
    // this episode's — half the `R_g` quorum.
    h.deliver_tag(n(2), Tag::RecoveryResponse)
        .expect("n0's T1 response reaches n2");
    assert_eq!(
        status_of(&h, n(2)),
        Status::Recovering,
        "one weighted answer is not the quorum",
    );

    // n1 answers T2; no T2 round ever runs at n0.
    h.deliver_to(n(1)).expect("the T2 solicitation reaches n1");
    h.deliver_tag(n(2), Tag::RecoveryResponse)
        .expect("n1's T2 response reaches n2");

    // The two responses — one per nonce — are the quorum, and the
    // completion installs from n0's T1 evidence (§6.1: the reported
    // view's primary carries the history).
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(0));
    h.assert_safety();
}

// The quorum is one across the episode's nonces: n0's answer to the
// first nonce and n1's answer to the second share no nonce, yet they are
// the same `R_g` quorum.
#[test]
fn cross_nonce_responses_complete_quorum() {
    let mut h = cluster();
    bootstrap(&mut h);
    crash_and_reopen(&mut h, n(2));

    h.recover(n(2));
    let first = h.now();
    h.recover(n(2));
    let second = h.now();
    assert!(second > first, "every re-drive carries a fresh tick");
    h.drop_queued(n(0));
    h.drop_queued(n(1));

    // n0 answers the FIRST nonce, with the view-0 primary's history.
    let history = h.journal_entries(n(0));
    let outcome = h.inject(
        n(0),
        n(2),
        recovery_response(first, view(0), INIT_SLOT, INIT_SLOT, Some(history)),
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    assert_eq!(
        status_of(&h, n(2)),
        Status::Recovering,
        "one weighted answer is not the quorum",
    );

    // n1 answers the SECOND nonce: the quorum holds across the two.
    let outcome = h.inject(
        n(1),
        n(2),
        recovery_response(second, view(0), INIT_SLOT, INIT_SLOT, None),
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(0));
    assert_eq!(
        h.journal_entries(n(2)),
        h.journal_entries(n(0)),
        "the installed history is the reported view's primary's",
    );
    h.assert_safety();
}

// A delayed response to a SUPERSEDED nonce is still this episode's:
// accepted and counted toward the one quorum, never named stale while
// the episode remembers the nonce.
#[test]
fn mixed_nonce_responses_are_accepted() {
    let mut h = cluster();
    bootstrap(&mut h);
    crash_and_reopen(&mut h, n(2));

    h.recover(n(2));
    let first = h.now();
    assert_eq!(queued_recovery_nonce(&h, n(0)), first);
    h.drop_queued(n(0));
    h.drop_queued(n(1));

    h.recover(n(2));
    let second = h.now();
    assert!(second > first, "every re-drive carries a fresh tick");
    assert_eq!(queued_recovery_nonce(&h, n(0)), second);

    // n1's delayed answer to the first nonce: superseded, still counted.
    let outcome = h.inject(
        n(1),
        n(2),
        recovery_response(first, view(0), INIT_SLOT, INIT_SLOT, None),
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    assert_eq!(
        status_of(&h, n(2)),
        Status::Recovering,
        "one weighted answer is not the quorum",
    );

    // n0 answers the second nonce with the view-0 primary's history: the
    // quorum holds across the nonces and the completion installs n0's.
    h.deliver_to(n(0)).expect("the T2 solicitation reaches n0");
    h.deliver_to(n(2)).expect("n0's T2 response reaches n2");
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(0));
    h.assert_safety();
}

// The episode's nonce memory is bounded: nine solicitations in, the
// first nonce is forgotten, and a response to it is exactly as stale as
// one to an attempt that never happened — named, ignored, counted
// toward nothing.
#[test]
fn evicted_nonce_responses_are_stale() {
    let mut h = cluster();
    bootstrap(&mut h);
    crash_and_reopen(&mut h, n(2));

    h.recover(n(2));
    let first = h.now();
    assert_eq!(queued_recovery_nonce(&h, n(0)), first);
    h.drop_queued(n(0));
    h.drop_queued(n(1));

    // Eight more re-drives, each with a fresh tick: the ninth nonce
    // crowds the first out of the episode's bounded memory.
    let mut latest = first;
    for _ in 0..8 {
        h.recover(n(2));
        let fresh = h.now();
        assert!(fresh > latest, "every re-drive carries a fresh tick");
        assert_eq!(queued_recovery_nonce(&h, n(0)), fresh);
        h.drop_queued(n(0));
        h.drop_queued(n(1));
        latest = fresh;
    }

    // A response to the evicted first nonce is stale (§6.1).
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
            attempt: Some(latest),
        }),
    );
    assert_eq!(status_of(&h, n(2)), Status::Recovering);
    h.assert_safety();
}

// The §8.3 fence knowledge rides the reported views, not the nonces:
// n0's answer to the first nonce reports view 0, the cluster fences
// view 1 around the recovering node, and n1's answer to the second nonce
// reports view 1 with its history. n0's superseded-nonce answer still
// counts toward the quorum, and the completion installs the LATEST
// fenced view's evidence — n1's.
#[test]
fn recovery_view_fence_respected_across_nonces() {
    let mut h = cluster();
    bootstrap(&mut h);
    commit_one(&mut h, n(0), 1, b"a");
    crash_and_reopen(&mut h, n(2));

    // T1: n0 answers reporting view 0, with its history.
    h.recover(n(2));
    let first = h.now();
    h.deliver_to(n(0)).expect("the T1 solicitation reaches n0");
    h.drop_queued(n(1));
    h.deliver_tag(n(2), Tag::RecoveryResponse)
        .expect("n0's T1 response reaches n2");
    assert_eq!(
        status_of(&h, n(2)),
        Status::Recovering,
        "one weighted answer is not the quorum",
    );

    // The cluster fences view 1 around the recovering node: n0 and n1
    // complete the view change; n2 hears none of it.
    tick_into_view_change(&mut h, n(1), view(1));
    h.deliver_tag(n(0), Tag::StartViewChange)
        .expect("n0 joins the view-1 fence");
    h.drop_queued(n(2));
    h.deliver_tag(n(1), Tag::StartViewChange)
        .expect("n0's fence vote reaches n1");
    h.deliver_tag(n(1), Tag::DoViewChange)
        .expect("n0's evidence reaches n1");
    h.deliver_to_matching(n(0), Tag::StartView, Slot(3))
        .expect("n0 installs view 1");
    for id in [n(0), n(1)] {
        assert_eq!(status_of(&h, id), Status::Normal);
        assert_eq!(current_view(&h, id), view(1));
    }
    assert_eq!(status_of(&h, n(2)), Status::Recovering);
    assert_eq!(current_view(&h, n(2)), view(0));
    h.drop_queued(n(1));
    h.drop_queued(n(2));

    // T2: n1 answers as the view-1 primary, with the view's history.
    h.recover(n(2));
    let second = h.now();
    assert!(second > first, "every re-drive carries a fresh tick");
    h.drop_queued(n(0));
    h.deliver_to(n(1)).expect("the T2 solicitation reaches n1");
    h.deliver_tag(n(2), Tag::RecoveryResponse)
        .expect("n1's T2 response reaches n2");

    // n0's view-0 answer counted toward the quorum; the completion
    // selects view 1's evidence — the latest fenced view wins.
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(1));
    assert_eq!(
        h.journal_entries(n(2)),
        h.journal_entries(n(1)),
        "the installed history is the view-1 primary's",
    );
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

// 11. Request liveness when the quorum's latest-view primary answers late
//     (§6.1, §8.3, S4): a five-node cluster lets the weighted `R_g` quorum
//     hold without the dead primary. The attempt then waits — ticks
//     neither fault, complete, nor re-broadcast — until the HOST re-issues
//     `Input::Recover` with a fresh tick: the fresh nonce joins the
//     attempt's bounded nonce set and the collected responses carry over.
//     The dead primary's delayed response to the first nonce still names
//     this episode, so it is accepted, completes the quorum with the
//     genuine view-1 history, and the node recovers into view 1; the
//     replacement primary's view-2 fence then reaches it through the
//     ordinary `StartView` path.
#[test]
fn redrive_with_fresh_nonce_outlives_dead_primary() {
    let mut h = Harness::with_knobs(
        5,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    );
    h.tick_all();
    h.deliver_all();
    h.assert_safety();

    // The cluster settles in view 1 (primary n1); n1 dies; n4 reopens
    // fenced into view 1.
    tick_into_view_change(&mut h, n(1), view(1));
    h.deliver_all();
    for id in [n(0), n(1), n(2), n(3), n(4)] {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} adopts view 1");
    }
    h.assert_safety();
    h.crash(n(1));
    crash_and_reopen(&mut h, n(4));
    assert_eq!(current_view(&h, n(4)), view(1));

    // Attempt 1: the `R_g` quorum (3 of 5) gathers WITHOUT the dead
    // primary's answer. The completion ruling is not evaluable — the
    // latest fenced view's primary holds the only installation evidence —
    // so the attempt waits.
    h.recover(n(4));
    let first = h.now();
    for id in [n(0), n(2), n(3)] {
        h.deliver_to(id)
            .expect("the solicitation reaches a live backup");
    }
    for _ in 0..3 {
        h.deliver_to(n(4)).expect("a quorum response reaches n4");
    }
    assert_eq!(
        status_of(&h, n(4)),
        Status::Recovering,
        "the quorum holds but the latest view's primary never answered",
    );

    // Ticks change nothing: no fault, no completion, and no automatic
    // re-broadcast — the request's re-drive is host-initiated (S4).
    for _ in 0..8 {
        h.tick(n(4));
    }
    assert_eq!(h.fault_of(n(4)), None);
    assert_eq!(status_of(&h, n(4)), Status::Recovering);
    assert!(
        h.peek_queued(n(0), Tag::Recovery).is_none()
            && h.peek_queued(n(2), Tag::Recovery).is_none()
            && h.peek_queued(n(3), Tag::Recovery).is_none(),
        "no tick re-drives the solicitation",
    );

    // The host re-issues recovery with a fresh tick: accepted while the
    // attempt is in flight, re-broadcast under the fresh nonce.
    let redrive = h.recover(n(4));
    assert!(
        matches!(redrive, StepOutcome::Published { .. }),
        "a fresh Recover is accepted mid-attempt: {redrive:?}",
    );
    let second = h.now();
    assert!(second > first, "every attempt carries a fresh tick");
    assert_eq!(queued_recovery_nonce(&h, n(0)), second);
    assert_eq!(queued_recovery_nonce(&h, n(3)), second);

    // Two fresh responses arrive: one short of the quorum.
    h.deliver_to(n(0))
        .expect("the fresh solicitation reaches n0");
    h.deliver_to(n(3))
        .expect("the fresh solicitation reaches n3");
    h.deliver_to(n(4)).expect("n0's fresh response reaches n4");
    h.deliver_to(n(4)).expect("n3's fresh response reaches n4");
    assert_eq!(status_of(&h, n(4)), Status::Recovering);

    // The dead primary's delayed response to the FIRST nonce finally
    // arrives — with history. The first nonce still names this episode
    // (the re-drive joined the fresh nonce to the attempt's set, S4), so
    // the response is accepted: no staleness diagnostic. The `R_g`
    // quorum now holds across both nonces, the latest reported view is
    // 1, and that view's primary answered with the genuine history — the
    // completion ruling installs it (§6.1). The nonce set changes WHEN
    // a response counts, never WHAT may install: anything committed in
    // the selected view-1 history is preserved by any later view's
    // `F_g ⌢ V_g` intersection (§8.3).
    let delayed = h.inject(
        n(1),
        n(4),
        recovery_response(
            first,
            view(1),
            INIT_SLOT,
            INIT_SLOT,
            Some(h.journal_entries(n(0))),
        ),
    );
    assert!(
        matches!(delayed, StepOutcome::Published { .. }),
        "the in-set nonce is accepted: {delayed:?}",
    );
    assert_eq!(
        h.diagnostic(n(4)),
        Some(Diagnostic::None),
        "accepted, not stale",
    );
    assert_eq!(h.fault_of(n(4)), None);
    assert_eq!(status_of(&h, n(4)), Status::Normal);
    assert_eq!(current_view(&h, n(4)), view(1));
    assert_eq!(
        h.journal_entries(n(4)),
        h.journal_entries(n(0)),
        "the installed history is the genuine view-1 primary's",
    );

    // The ordinary pipeline installs a replacement primary: view 2, owned
    // by n2, driven by the three live members.
    tick_into_view_change(&mut h, n(2), view(2));
    h.deliver_to(n(0)).expect("n0 joins the view-2 fence");
    h.deliver_to(n(3)).expect("n3 joins the view-2 fence");
    // The fence quorum is 3 of 5: each fencing member emits its evidence
    // only once a third vote lands (§9.1's ordering: evidence follows the
    // fence).
    h.deliver_tag(n(0), Tag::StartViewChange)
        .expect("n3's vote completes n0's fence");
    h.deliver_tag(n(3), Tag::StartViewChange)
        .expect("n0's vote completes n3's fence");
    h.deliver_tag(n(2), Tag::StartViewChange)
        .expect("n0's fence vote reaches n2");
    h.deliver_tag(n(2), Tag::StartViewChange)
        .expect("n3's fence vote reaches n2");
    h.deliver_tag(n(2), Tag::DoViewChange)
        .expect("n0's evidence");
    h.deliver_tag(n(2), Tag::DoViewChange)
        .expect("n3's evidence");
    h.deliver_tag(n(0), Tag::StartView)
        .expect("n0 installs view 2");
    h.deliver_tag(n(3), Tag::StartView)
        .expect("n3 installs view 2");
    for id in [n(0), n(2), n(3)] {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} installs view 2");
        assert_eq!(current_view(&h, id), view(2));
    }

    // n4 — already Normal in view 1 — takes view 2 through the ordinary
    // `StartView` path (§9.1): the adoption rule installs any node the
    // change passed by — Normal in an earlier view — straight from the
    // new primary's offer. No recovery completion is involved.
    h.deliver_tag(n(4), Tag::StartView)
        .expect("the replacement primary's StartView reaches n4");
    assert_eq!(h.fault_of(n(4)), None);
    assert_eq!(status_of(&h, n(4)), Status::Normal);
    assert_eq!(current_view(&h, n(4)), view(2));
    assert_eq!(
        h.journal_entries(n(4)),
        h.journal_entries(n(2)),
        "the installed history is the replacement primary's",
    );
    h.assert_safety();
}

/// The external-stability host step (S2/S3): a parked transition is
/// confirmed `Stable`; anything else passes through.
fn settle(h: &mut Harness, id: NodeId, outcome: StepOutcome) -> StepOutcome {
    match outcome {
        StepOutcome::Parked { .. } => h.confirm(
            id,
            StabilityResult::Stable {
                receipt: b"ok"[..].into(),
            },
        ),
        other => other,
    }
}

/// Executes a node's pending `Apply` effects, confirming the park each
/// acknowledgement raises under an external-stability mode (§11.1, S2).
fn apply_settled(h: &mut Harness, id: NodeId) {
    for applied in h.execute_apply_effects(id) {
        settle(h, id, applied.outcome);
    }
}

// 12. The plan/publish handshake (§7, §12, S2/S3) under an
//     external-stability mode, driven through a recovery install: nothing
//     is observable while the completion is parked, a determinate `Failed`
//     confirmation discards the candidate and leaves the old state visible
//     with the node still `Recovering`, a duplicate confirmation is
//     rejected, and the re-driven completion publishes the install exactly
//     once — the replay upcall included — before the node replays to
//     `Normal`.
#[test]
fn external_stability_recovery_handshake() {
    let mut h = Harness::with_stability(3, Stability::ExternalTransaction);
    for (id, outcome) in h.tick_all() {
        settle(&mut h, id, outcome);
    }
    while let Some(delivery) = h.deliver_next() {
        settle(&mut h, delivery.to, delivery.outcome);
    }
    h.assert_safety();

    // One operation commits at slot 3 and applies at n0 and n1 only: n2's
    // pending upcall dies with the crash, so the recovery must replay it.
    let proposed = h.propose(n(0), op_id(1), b"a");
    let proposed = settle(&mut h, n(0), proposed);
    assert!(matches!(proposed, StepOutcome::Published { .. }));
    for id in [n(1), n(2)] {
        let delivery = h.deliver_to(id).expect("the Prepare reaches a backup");
        settle(&mut h, id, delivery.outcome);
    }
    for _ in 0..2 {
        let delivery = h.deliver_to(n(0)).expect("a PrepareOk reaches the primary");
        settle(&mut h, n(0), delivery.outcome);
    }
    for id in [n(1), n(2)] {
        let delivery = h.deliver_to(id).expect("the Commit reaches a backup");
        settle(&mut h, id, delivery.outcome);
    }
    apply_settled(&mut h, n(0));
    apply_settled(&mut h, n(1));

    h.crash(n(2));
    h.restart_with(n(2)).expect("the disk record reopens");
    let reopened = snap(&h, n(2));
    assert_eq!(reopened.committed, 3);
    assert_eq!(reopened.applied, 2);

    // Even the solicitation parks: nothing is observable before publish.
    let solicitation = h.recover(n(2));
    assert!(
        matches!(solicitation, StepOutcome::Parked { .. }),
        "the recovery input parks: {solicitation:?}",
    );
    assert!(
        h.peek_queued(n(0), Tag::Recovery).is_none()
            && h.peek_queued(n(1), Tag::Recovery).is_none(),
        "no solicitation is released before the barrier confirms",
    );
    settle(&mut h, n(2), solicitation);

    let delivery = h.deliver_to(n(0)).expect("the solicitation reaches n0");
    settle(&mut h, n(0), delivery.outcome);
    let delivery = h.deliver_to(n(2)).expect("n0's response reaches n2");
    settle(&mut h, n(2), delivery.outcome);
    let delivery = h.deliver_to(n(1)).expect("the solicitation reaches n1");
    settle(&mut h, n(1), delivery.outcome);

    // The completing response parks the install: the published state is
    // untouched and nothing releases.
    let before = snap(&h, n(2));
    let completing = h.deliver_to(n(2)).expect("n1's response reaches n2");
    let StepOutcome::Parked { revision } = completing.outcome else {
        panic!("the completion parks: {:?}", completing.outcome);
    };
    assert_eq!(snap(&h, n(2)), before, "nothing observable before publish");
    assert_eq!(status_of(&h, n(2)), Status::Recovering);

    // A determinate `Failed`: the candidate is discarded, the previously
    // published state stays visible, the node continues (S3). The revision
    // moves — the interval must close so plans against the parked base die
    // (§12) — and nothing else does.
    let failed = h.confirm(
        n(2),
        StabilityResult::Failed {
            reason: b"barrier lost"[..].into(),
        },
    );
    assert!(matches!(failed, StepOutcome::Published { .. }));
    let after_failed = snap(&h, n(2));
    assert_eq!(
        after_failed.revision,
        before.revision + 1,
        "the failed interval closes",
    );
    assert_eq!(
        ProgressSnapshot {
            revision: before.revision,
            ..after_failed
        },
        before,
        "the old state stays visible",
    );
    assert_eq!(status_of(&h, n(2)), Status::Recovering);

    // A duplicate confirmation is rejected: the interval is closed.
    let duplicate = h.confirm_raw(
        n(2),
        revision,
        StabilityResult::Failed {
            reason: b"barrier lost"[..].into(),
        },
    );
    assert_eq!(
        duplicate,
        StepOutcome::PlanRefused(PlanRejection::NoTransitionOutstanding),
    );

    // The completing response's record died with the discarded candidate
    // (S3: a `Failed` confirmation discards the record updates with the
    // candidate), so the attempt is short of the quorum again and the
    // host re-drives: a fresh `Recover` under a fresh nonce (S4), and the
    // quorum re-gathers.
    let redrive = h.recover(n(2));
    settle(&mut h, n(2), redrive);
    let delivery = h
        .deliver_to(n(0))
        .expect("the fresh solicitation reaches n0");
    settle(&mut h, n(0), delivery.outcome);
    let delivery = h.deliver_to(n(2)).expect("n0's fresh response reaches n2");
    settle(&mut h, n(2), delivery.outcome);
    let delivery = h
        .deliver_to(n(1))
        .expect("the fresh solicitation reaches n1");
    settle(&mut h, n(1), delivery.outcome);

    // The re-driven completion parks: still nothing observable.
    let parked = snap(&h, n(2));
    let completing = h.deliver_to(n(2)).expect("n1's fresh response reaches n2");
    let StepOutcome::Parked { revision: second } = completing.outcome else {
        panic!("the re-driven completion parks: {:?}", completing.outcome);
    };
    assert_eq!(snap(&h, n(2)), parked, "still nothing observable");

    // `Stable` publishes the install exactly once: one replay upcall, one
    // publication; a further confirmation names nothing outstanding.
    let published = h.confirm(
        n(2),
        StabilityResult::Stable {
            receipt: b"ok"[..].into(),
        },
    );
    let StepOutcome::Published { effects, .. } = published else {
        panic!("the confirmed completion publishes: {published:?}");
    };
    assert_eq!(
        effects,
        vec![Effect::Apply {
            slot: Slot(3),
            operation_id: op_id(1),
            payload: b"a"[..].into(),
        }],
        "the committed-but-unapplied suffix replays exactly once (§11.1)",
    );
    assert_eq!(status_of(&h, n(2)), Status::Replaying);
    let duplicate = h.confirm_raw(
        n(2),
        second,
        StabilityResult::Stable {
            receipt: b"ok"[..].into(),
        },
    );
    assert_eq!(
        duplicate,
        StepOutcome::PlanRefused(PlanRejection::NoTransitionOutstanding),
    );

    apply_settled(&mut h, n(2));
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(0));
    assert_eq!(h.journal_entries(n(2)), h.journal_entries(n(0)));
    h.assert_safety();
}

// 13. A node that recovers out of an open view-change attempt carries no
//     stale evidence forward (§6.1's completion clears the attempt
//     bookkeeping): afterwards it answers a `StartViewChange` for a LATER
//     view with its fenced evidence — the recovered view's — emits nothing
//     toward the pre-recovery attempt's primary, and the fresh view change
//     completes with the node inside it.
#[test]
fn recovery_completion_leaves_no_stale_view_change_evidence() {
    let mut h = cluster();
    bootstrap(&mut h);
    commit_one(&mut h, n(0), 1, b"a");

    // n2 fences view 1 on its own timeout: the attempt is open — its
    // fence votes are queued, never delivered.
    tick_into_view_change(&mut h, n(2), view(1));

    // The quorum completes view 1 without n2: n1 times out too, n0 joins
    // n1's fence, and the evidence exchange installs the view at n0/n1.
    tick_into_view_change(&mut h, n(1), view(1));
    h.deliver_to(n(0)).expect("n0 joins the fence");
    h.deliver_to(n(0)).expect("n0 hears n1's vote");
    h.deliver_tag(n(1), Tag::StartViewChange)
        .expect("a fence vote completes at the new primary");
    h.deliver_tag(n(1), Tag::DoViewChange)
        .expect("n0's evidence reaches the new primary");
    h.deliver_tag(n(0), Tag::StartView)
        .expect("n0 installs view 1");
    assert_eq!(status_of(&h, n(0)), Status::Normal);
    assert_eq!(status_of(&h, n(1)), Status::Normal);
    assert_eq!(current_view(&h, n(1)), view(1));
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    h.drop_queued(n(0));
    h.drop_queued(n(1));
    h.drop_queued(n(2));

    // n2 crashes mid-attempt and recovers into the quorum's view.
    crash_and_reopen(&mut h, n(2));
    assert_eq!(current_view(&h, n(2)), view(1));
    h.recover(n(2));
    h.deliver_to(n(0)).expect("the solicitation reaches n0");
    h.deliver_to(n(1)).expect("the solicitation reaches n1");
    h.deliver_to(n(2)).expect("n0's response reaches n2");
    h.deliver_to(n(2)).expect("n1's response reaches n2");
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(1));

    // A fresh `StartViewChange` for a later view: the node answers with
    // its fenced evidence — selected in the view it recovered into — and
    // emits nothing toward the pre-recovery attempt's primary.
    let outcome = h.inject(n(0), n(2), svc(view(3)));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    assert_eq!(status_of(&h, n(2)), Status::ViewChange);
    assert_eq!(current_view(&h, n(2)), view(3));
    let evidence = h
        .peek_queued(n(0), Tag::DoViewChange)
        .expect("the node answers the view-3 primary with evidence");
    assert_eq!(evidence.header.view, view(3));
    let Body::DoViewChange { retained, .. } = &evidence.body else {
        panic!("the peeked message is a DoViewChange");
    };
    assert_eq!(
        *retained,
        view(1),
        "the evidence is the recovered view's, not the stale attempt's",
    );
    assert!(
        h.peek_queued(n(1), Tag::DoViewChange).is_none(),
        "no stale evidence goes to the pre-recovery attempt's primary",
    );

    // The fresh view change completes with the recovered node inside it.
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} installs view 3");
        assert_eq!(current_view(&h, id), view(3));
    }
    h.assert_safety();
}

// 14. The committed fast-forward (§6.1, §10, §11.1): an accepted response
//     whose `committed` exceeds the local frontier — one weighted answer,
//     short of the quorum — advances it over the sequentially-adjacent,
//     locally journal-present entries, emitting the same ordered `Apply`
//     upcalls the node would have emitted had it never crashed, while the
//     node is still fenced `Recovering`. The operation slots then await
//     the host's §11.1 acknowledgements exactly as in normal operation.
#[test]
fn fast_forward_applies_locally_present_committed_entries() {
    let mut h = cluster();
    bootstrap(&mut h);
    stage_accepted_tail(&mut h, n(2), &[n(1)], &[(1, b"a"), (2, b"b"), (3, b"c")]);
    crash_and_reopen(&mut h, n(2));
    let reopened = snap(&h, n(2));
    assert_eq!(reopened.committed, 2);
    assert_eq!(reopened.accepted, 5);

    h.recover(n(2));
    let nonce = h.now();
    let outcome = h.inject(
        n(1),
        n(2),
        recovery_response(nonce, view(0), Slot(5), Slot(5), None),
    );
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the accepted response publishes: {outcome:?}");
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
            Effect::Apply {
                slot: Slot(5),
                operation_id: op_id(3),
                payload: b"c"[..].into(),
            },
        ],
        "the fast-forward emits the ordered Apply upcalls",
    );
    let forwarded = snap(&h, n(2));
    assert_eq!(forwarded.committed, 5);
    assert_eq!(
        forwarded.applied, 2,
        "operation slots await the host's acknowledgements",
    );
    assert_eq!(
        status_of(&h, n(2)),
        Status::Recovering,
        "the attempt is still open",
    );

    // The acknowledgements land through the ordinary §11.1 path, still
    // fenced: the attempt completes separately.
    let outcomes = h.execute_apply_effects(n(2));
    assert_eq!(outcomes.len(), 3);
    assert_eq!(snap(&h, n(2)).applied, 5);
    assert_eq!(status_of(&h, n(2)), Status::Recovering);
    h.assert_safety();
}

// 15. Idempotence: the same response delivered twice emits each upcall
//     exactly once — slots at or below the fast-forwarded frontier are
//     skipped.
#[test]
fn fast_forward_is_idempotent_to_duplicate_responses() {
    let mut h = cluster();
    bootstrap(&mut h);
    stage_accepted_tail(&mut h, n(2), &[n(1)], &[(1, b"a"), (2, b"b"), (3, b"c")]);
    crash_and_reopen(&mut h, n(2));

    h.recover(n(2));
    let nonce = h.now();
    let response = recovery_response(nonce, view(0), Slot(5), Slot(5), None);
    let first = h.inject(n(1), n(2), response.clone());
    let StepOutcome::Published { effects, .. } = first else {
        panic!("the accepted response publishes: {first:?}");
    };
    assert_eq!(effects.len(), 3, "the first delivery fast-forwards");

    let second = h.inject(n(1), n(2), response);
    let StepOutcome::Published { effects, .. } = second else {
        panic!("the duplicate publishes: {second:?}");
    };
    assert!(
        effects.is_empty(),
        "a duplicate response emits nothing: {effects:?}",
    );

    let outcomes = h.execute_apply_effects(n(2));
    assert_eq!(outcomes.len(), 3, "each upcall executes exactly once");
    assert_eq!(snap(&h, n(2)).applied, 5);
    h.assert_safety();
}

// 16. TakeWhile: a response claiming `committed` beyond what the local
//     journal physically holds applies only up to the first slot the
//     journal does not hold — here the accepted frontier itself is the
//     gap.
#[test]
fn fast_forward_stops_at_the_journal_gap() {
    let mut h = cluster();
    bootstrap(&mut h);
    stage_accepted_tail(&mut h, n(2), &[n(1)], &[(1, b"a"), (2, b"b"), (3, b"c")]);
    crash_and_reopen(&mut h, n(2));

    h.recover(n(2));
    let nonce = h.now();
    let outcome = h.inject(
        n(1),
        n(2),
        recovery_response(nonce, view(0), Slot(7), Slot(7), None),
    );
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the accepted response publishes: {outcome:?}");
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
            Effect::Apply {
                slot: Slot(5),
                operation_id: op_id(3),
                payload: b"c"[..].into(),
            },
        ],
        "only the locally journal-present slots apply",
    );
    assert_eq!(
        snap(&h, n(2)).committed,
        5,
        "the walk stops at the first slot the journal does not hold",
    );
    h.assert_safety();
}

// 17. A completion after a fast-forward never re-applies a fast-forwarded
//     slot: the host acknowledged the upcalls, so the completion's replay
//     walk — which starts from the applied frontier — has nothing left to
//     re-emit (§11.1).
#[test]
fn completion_after_fast_forward_does_not_reapply() {
    let mut h = cluster();
    bootstrap(&mut h);
    stage_accepted_tail(&mut h, n(2), &[n(1)], &[(1, b"a"), (2, b"b"), (3, b"c")]);
    crash_and_reopen(&mut h, n(2));

    h.recover(n(2));
    let nonce = h.now();
    let outcome = h.inject(
        n(1),
        n(2),
        recovery_response(nonce, view(0), Slot(5), Slot(5), None),
    );
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the accepted response publishes: {outcome:?}");
    };
    assert_eq!(effects.len(), 3, "the first answer fast-forwards");
    h.execute_apply_effects(n(2));
    assert_eq!(snap(&h, n(2)).applied, 5);

    // The view-0 primary's answer carries the installation history and
    // completes the quorum: the replay re-emits nothing.
    let completing = h.inject(
        n(0),
        n(2),
        recovery_response(
            nonce,
            view(0),
            Slot(5),
            Slot(5),
            Some(h.journal_entries(n(0))),
        ),
    );
    let StepOutcome::Published { effects, .. } = completing else {
        panic!("the completing response publishes: {completing:?}");
    };
    assert!(
        effects.is_empty(),
        "no fast-forwarded slot is re-applied: {effects:?}",
    );
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    let recovered = snap(&h, n(2));
    assert_eq!(recovered.committed, 5);
    assert_eq!(recovered.applied, 5);
    assert_eq!(h.journal_entries(n(2)), h.journal_entries(n(0)));
    h.assert_safety();
}

// 18. Monotonicity (§1.3): the completing evidence's `committed` — the
//     latest fenced view's primary's, recorded earlier in the episode —
//     sits BELOW the frontier a later answer already fast-forwarded. The
//     completion installs the primary's history but never moves the
//     frontier backward, and nothing re-applies. Five nodes, so the
//     primary's stale answer can be recorded before the quorum completes.
#[test]
fn completion_evidence_below_fast_forwarded_frontier_does_not_regress() {
    let mut h = Harness::with_knobs(
        5,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    );
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
    stage_accepted_tail(
        &mut h,
        n(4),
        &[n(1), n(2)],
        &[(1, b"a"), (2, b"b"), (3, b"c"), (4, b"d")],
    );
    assert_eq!(snap(&h, n(4)).accepted, 6);
    assert_eq!(snap(&h, n(4)).committed, 2);
    crash_and_reopen(&mut h, n(4));

    h.recover(n(4));
    let nonce = h.now();

    // The view-0 primary's answer — an older committed frontier, with its
    // history — is recorded first and fast-forwards what it vouches for.
    let outcome = h.inject(
        n(0),
        n(4),
        recovery_response(
            nonce,
            view(0),
            Slot(6),
            Slot(3),
            Some(h.journal_entries(n(0))),
        ),
    );
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the primary's answer publishes: {outcome:?}");
    };
    assert_eq!(
        effects,
        vec![Effect::Apply {
            slot: Slot(3),
            operation_id: op_id(1),
            payload: b"a"[..].into(),
        }],
    );
    assert_eq!(snap(&h, n(4)).committed, 3);
    h.execute_apply_effects(n(4));

    // A second answer carries the fuller frontier: fast-forward to 6.
    let outcome = h.inject(
        n(1),
        n(4),
        recovery_response(nonce, view(0), Slot(6), Slot(6), None),
    );
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the second answer publishes: {outcome:?}");
    };
    assert_eq!(effects.len(), 3, "slots 4 through 6 fast-forward");
    assert_eq!(snap(&h, n(4)).committed, 6);
    h.execute_apply_effects(n(4));
    assert_eq!(snap(&h, n(4)).applied, 6);

    // The third answer completes the `R_g` quorum: the completion
    // installs the primary's recorded evidence — committed 3, below the
    // fast-forwarded frontier. The frontier never moves backward.
    let completing = h.inject(
        n(2),
        n(4),
        recovery_response(nonce, view(0), Slot(6), Slot(6), None),
    );
    let StepOutcome::Published { effects, .. } = completing else {
        panic!("the completing answer publishes: {completing:?}");
    };
    assert!(effects.is_empty(), "nothing re-applies: {effects:?}");
    let recovered = snap(&h, n(4));
    assert_eq!(
        recovered.committed, 6,
        "the completion never regresses the frontier",
    );
    assert_eq!(recovered.applied, 6);
    assert_eq!(status_of(&h, n(4)), Status::Normal);
    assert_eq!(h.fault_of(n(4)), None);
    h.assert_safety();
}
