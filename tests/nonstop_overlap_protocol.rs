//! End-to-end integration tests for the non-stop overlap transition
//! (§8.7.6–§8.7.7): the seven steps in order with an uninterrupted client
//! stream, the routing discipline (`qI − {L}` only, never `qII − {L}`),
//! recipient retention, the pivot threaded onto the era record, the
//! single published transition, transition-view exhaustion, idempotent
//! duplicates and reorderings, and the suffix fallback through the
//! ordinary fetch path.

mod harness;

use harness::{DeliveryOutcome, Harness, StepOutcome};
use vrr::configuration::SystemOperation;
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};

use vrr::message::{Body, Message};
use vrr::progress::Status;
use vrr::replica::{Pivot, PlanRefusal, ViewChangeKnobs};
use vrr::wire::Tag;

fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// An operation identity for the scripts: the host assigns it, the core
/// carries it opaque (§11.1, B2).
fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

/// A view in era 1 — the era every node here bootstraps into.
fn view(number: u32) -> ViewId {
    ViewId {
        era: Era(1),
        view: View(number),
    }
}

/// A view in era 2 — the era the committed reconfiguration establishes.
fn era2_view(number: u32) -> ViewId {
    ViewId {
        era: Era(2),
        view: View(number),
    }
}

/// The pivot the scripts run (§8.7.6): `qI = {n0, n2}` is a view-change
/// quorum under the genesis configuration, `qII = {n0, n1}` is a commit
/// quorum under both configurations (weight 3 of 4 in era 2), and the
/// two sets intersect in exactly the leader n0.
fn overlap_pivot() -> Pivot {
    Pivot {
        q_i: vec![n(0), n(2)],
        q_ii: vec![n(0), n(1)],
    }
}

/// The transition view the pivot names (§8.7.7 step 5): the least view
/// past (1, 0) selecting position 0 under the era-2 order — view 3,
/// since views 1 and 2 select the other members.
fn transition_view() -> ViewId {
    era2_view(3)
}

/// The timeout knob for the view-change scripts: a node suspects its
/// primary after more than three ticks of silence (S4).
const TIMEOUT: u64 = 3;

/// A three-node cluster whose view-change machinery is live.
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
/// backups adopt view (1, 0) from the promotion's `Commit` announcement
/// (§13.3).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

/// The snapshot of a live node (tests never snapshot a crashed one).
fn snap(h: &Harness, id: NodeId) -> vrr::progress::ProgressSnapshot {
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

/// The newest era the node's committed configuration history has
/// established (§8.7.1).
fn current_era(h: &Harness, id: NodeId) -> Era {
    h.era_table(id).expect("the node is live").current().era
}

/// The `Send` effects of a published step as `(to, route era, message)`.
fn sends(outcome: &StepOutcome) -> Vec<(NodeId, Era, Message)> {
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the step published: {outcome:?}");
    };
    effects
        .iter()
        .filter_map(|effect| match effect {
            vrr::effects::Effect::Send { to, era, message } => Some((*to, *era, message.clone())),
            _ => None,
        })
        .collect()
}

/// Delivers the queued datagram with `tag` to `id`, asserting it exists.
fn deliver(h: &mut Harness, id: NodeId, tag: Tag) -> DeliveryOutcome {
    h.deliver_tag(id, tag)
        .unwrap_or_else(|| panic!("a {tag:?} is queued for n{}", id.0))
}

/// Delivers the queued datagram with `tag` and header `slot` to `id`,
/// asserting it exists — the way a script picks among several queued
/// datagrams of one tag.
fn deliver_matching(h: &mut Harness, id: NodeId, tag: Tag, slot: Slot) -> DeliveryOutcome {
    h.deliver_to_matching(id, tag, slot)
        .unwrap_or_else(|| panic!("a {tag:?} at {slot:?} is queued for n{}", id.0))
}

/// Drives the establishing operation to its commit at the leader:
/// proposal routed to `qII − {L}` only, acceptance, the commit cascade
/// that folds the era, and the `qI − {L}` solicitation. Returns the
/// commit step's outcome for the caller's own assertions.
fn establish(h: &mut Harness) -> DeliveryOutcome {
    let outcome = h.reconfigure(
        n(0),
        SystemOperation::Increment(n(1)),
        Some(overlap_pivot()),
    );
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the establishing proposal publishes: {outcome:?}"
    );
    deliver(h, n(1), Tag::Prepare);
    deliver(h, n(0), Tag::PrepareOk)
}

#[test]
fn overlap_transition_runs_the_seven_steps_without_stopping_the_stream() {
    let mut h = cluster();
    bootstrap(&mut h);

    // Step 1: the establishing proposal goes to `qII − {L}` and nowhere
    // else, routed under the era its entry is stamped with (§8.7.6).
    let outcome = h.reconfigure(
        n(0),
        SystemOperation::Increment(n(1)),
        Some(overlap_pivot()),
    );
    let sent = sends(&outcome);
    assert_eq!(sent.len(), 1, "the pivot routes the Prepare: {sent:?}");
    assert_eq!(sent[0].0, n(1), "qII − {{L}} is exactly n1");
    assert_eq!(sent[0].1, Era(1), "the route era is the entry's era");
    let Body::Prepare { entry, .. } = &sent[0].2.body else {
        panic!("the send is the establishing Prepare");
    };
    assert_eq!(entry.slot, Slot(3));
    assert_eq!(entry.era, Era(1));
    assert!(
        h.peek_queued(n(2), Tag::Prepare).is_none(),
        "never qII-outsiders"
    );
    assert_eq!(
        current_era(&h, n(0)),
        Era(1),
        "acceptance establishes nothing"
    );

    // Step 2: acceptance through qII, then the commit — the era folds,
    // the pivot lands on the era record, and the solicitation goes to
    // `qI − {L}` (never `qII − {L}`) while the leader stays in view
    // (1, 0).
    deliver(&mut h, n(1), Tag::Prepare);
    let commit = deliver(&mut h, n(0), Tag::PrepareOk);
    let sent = sends(&commit.outcome);
    let solicitations: Vec<_> = sent
        .iter()
        .filter(|(_, _, message)| message.header.tag == Tag::PlannedViewChange)
        .collect();
    assert_eq!(solicitations.len(), 1, "exactly one solicitation: {sent:?}");
    assert_eq!(solicitations[0].0, n(2), "qI − {{L}} is exactly n2");
    assert_eq!(solicitations[0].1, Era(1), "routed under the current era");
    assert_eq!(solicitations[0].2.header.view, transition_view());
    assert!(
        sent.iter()
            .all(|(to, _, message)| *to != n(1) || message.header.tag != Tag::PlannedViewChange),
        "qII − {{L}} never sees the solicitation"
    );
    let record = h
        .era_table(n(0))
        .expect("the leader is live")
        .record(Era(2))
        .expect("the fold recorded era 2")
        .clone();
    assert_eq!(record.established_by, Slot(3));
    assert_eq!(
        record.pivot,
        Some(overlap_pivot()),
        "the pivot is threaded onto the era record at the fold"
    );
    assert_eq!(
        current_view(&h, n(0)),
        view(0),
        "the leader has not switched"
    );

    // Step 3: the qI recipient answers with PLANNED evidence and RETAINS
    // its view — no fence, still Normal in (1, 0) — and opens the suffix
    // fallback fetch for the establishing operation it never saw.
    let answer = deliver(&mut h, n(2), Tag::PlannedViewChange);
    let sent = sends(&answer.outcome);
    let evidence = sent
        .iter()
        .find(|(_, _, message)| message.header.tag == Tag::DoViewChange)
        .expect("the planned answer went out");
    assert_eq!(evidence.0, n(0));
    assert_eq!(evidence.1, Era(1), "routed under the retained era");
    assert_eq!(evidence.2.header.view, transition_view());
    assert_eq!(evidence.2.header.slot, Slot(2), "the responder's frontier");
    let Body::DoViewChange { evidence: kind, .. } = &evidence.2.body else {
        panic!("the answer is a DoViewChange");
    };
    assert_eq!(*kind, vrr::message::EvidenceKind::Planned);
    let fetch = sent
        .iter()
        .find(|(_, _, message)| message.header.tag == Tag::GetState)
        .expect("the suffix fallback fetch opened");
    assert_eq!(fetch.0, n(0));
    assert_eq!(fetch.1, Era(1), "the fetch runs under the current view");
    assert_eq!(
        status_of(&h, n(2)),
        Status::Normal,
        "no fence (§8.7.7 step 2)"
    );
    assert_eq!(current_view(&h, n(2)), view(0), "current_view = v retained");

    // Step 4 (interleaved): the client stream continues — an ordinary
    // era-(e+1) prepare commits through qII while the planned exchange
    // is mid-flight.
    let proposal = h.propose(n(0), op_id(1), b"overlap-a");
    let sent = sends(&proposal);
    assert_eq!(
        sent.len(),
        2,
        "the ordinary prepare fans out to the era-e backups"
    );
    let Body::Prepare { entry, .. } = &sent[0].2.body else {
        panic!("the send is a Prepare");
    };
    assert_eq!(
        entry.era,
        Era(2),
        "stamped with the era that authorizes the slot"
    );
    assert!(
        sent.iter().all(|(_, era, _)| *era == Era(2)),
        "routed under the entry's era"
    );
    deliver(&mut h, n(1), Tag::Prepare);
    deliver(&mut h, n(0), Tag::PrepareOk);
    assert_eq!(
        snap(&h, n(0)).committed,
        4,
        "the client operation committed through qII = {{n0, n1}} mid-transition"
    );
    assert_eq!(
        current_view(&h, n(0)),
        view(0),
        "and the leader is still in v"
    );

    // Step 5: the responder's fetch folds the era through the ordinary
    // state-transfer path — still no view change anywhere.
    deliver(&mut h, n(0), Tag::GetState);
    deliver(&mut h, n(2), Tag::NewState);
    assert_eq!(
        current_era(&h, n(2)),
        Era(2),
        "the fetched commit folded the era"
    );
    assert_eq!(snap(&h, n(2)).committed, 4);
    assert_eq!(
        status_of(&h, n(2)),
        Status::Normal,
        "retention survives the fetch"
    );
    assert_eq!(current_view(&h, n(2)), view(0));

    // Step 6: the planned answer completes the quorum — the casting vote
    // and the switch are ONE published transition, and StartView(v')
    // goes to every member of config(e+1).
    let transition = deliver(&mut h, n(0), Tag::DoViewChange);
    let sent = sends(&transition.outcome);
    let announcements: Vec<_> = sent
        .iter()
        .filter(|(_, _, message)| message.header.tag == Tag::StartView)
        .collect();
    assert_eq!(
        announcements.len(),
        2,
        "every member of config(e+1): {sent:?}"
    );
    assert!(
        announcements
            .iter()
            .all(|(_, era, message)| *era == Era(2) && message.header.view == transition_view())
    );
    let mut recipients: Vec<_> = announcements.iter().map(|(to, _, _)| *to).collect();
    recipients.sort();
    assert_eq!(recipients, vec![n(1), n(2)]);
    let Body::StartView { committed, .. } = &announcements[0].2.body else {
        panic!("the announcement is a StartView");
    };
    assert_eq!(
        *committed,
        Slot(4),
        "the committed frontier the vote certified"
    );
    assert_eq!(
        current_view(&h, n(0)),
        transition_view(),
        "the single switch"
    );
    assert_eq!(
        status_of(&h, n(0)),
        Status::Normal,
        "the leader resumed normal operation in v'"
    );
    let snapshot = snap(&h, n(0));
    assert_eq!(snapshot.retained_era, 2);
    assert_eq!(
        snapshot.retained_view, 3,
        "retained joined current in one step"
    );

    // Step 7: the members install and the stream continues in the new
    // view under the NEW configuration's arithmetic.
    deliver(&mut h, n(1), Tag::StartView);
    deliver(&mut h, n(2), Tag::StartView);
    for id in [n(1), n(2)] {
        assert_eq!(current_view(&h, id), transition_view());
        assert_eq!(status_of(&h, id), Status::Normal);
    }
    h.propose(n(0), op_id(2), b"overlap-b");
    // The light responder's answer does NOT commit: {n0, n2} weighs 2
    // under the era-2 threshold 3 — the transition's whole point is that
    // qII, not qI, carries the stream.
    deliver_matching(&mut h, n(2), Tag::Prepare, Slot(5));
    deliver_matching(&mut h, n(0), Tag::PrepareOk, Slot(5));
    assert_eq!(snap(&h, n(0)).committed, 4, "qI alone cannot commit");
    deliver_matching(&mut h, n(1), Tag::Prepare, Slot(5));
    deliver_matching(&mut h, n(0), Tag::PrepareOk, Slot(5));
    assert_eq!(
        snap(&h, n(0)).committed,
        5,
        "the heavy qII member commits it"
    );

    h.deliver_all();
    h.assert_safety();
    h.execute_apply_effects(n(0));
    let payloads: Vec<_> = h
        .applied(n(0))
        .iter()
        .map(|(_, payload)| payload.to_vec())
        .collect();
    assert!(
        payloads
            .windows(2)
            .any(|pair| pair[0] == b"overlap-a" && pair[1] == b"overlap-b"),
        "the stream ran through the transition in order: {payloads:?}"
    );
    assert_eq!(
        h.journal_entries(n(0)),
        h.journal_entries(n(2)),
        "the qI member's history is the leader's"
    );
    assert_eq!(current_era(&h, n(2)), Era(2));
}

#[test]
fn solicitation_duplicate_and_evidence_duplicate_are_idempotent() {
    let mut h = cluster();
    bootstrap(&mut h);
    establish(&mut h);

    // The solicitation is answered statelessly: a duplicate delivery
    // produces the same planned answer, and the recipient still retains
    // its view.
    deliver(&mut h, n(2), Tag::PlannedViewChange);
    let solicitation = Message {
        header: vrr::wire::Header {
            tag: Tag::PlannedViewChange,
            view: transition_view(),
            slot: Slot::NONE,
        },
        body: Body::PlannedViewChange {},
    };
    h.inject(n(0), n(2), solicitation);
    assert_eq!(status_of(&h, n(2)), Status::Normal, "still no fence");
    assert_eq!(current_view(&h, n(2)), view(0));

    // Both answers reach the leader: the first completes the quorum and
    // publishes the transition; the second is stale by name and moves
    // nothing.
    deliver(&mut h, n(0), Tag::DoViewChange);
    assert_eq!(current_view(&h, n(0)), transition_view());
    let stale = deliver(&mut h, n(0), Tag::DoViewChange);
    assert!(
        matches!(stale.outcome, StepOutcome::Published { .. }),
        "the duplicate is dropped, not refused: {:?}",
        stale.outcome
    );
    assert_eq!(
        current_view(&h, n(0)),
        transition_view(),
        "one transition only"
    );
    assert!(
        h.peek_queued(n(1), Tag::StartView).is_some()
            && h.peek_queued(n(2), Tag::StartView).is_some(),
        "exactly one announcement round went out"
    );

    h.deliver_all();
    h.assert_safety();
    assert_eq!(h.journal_entries(n(0)), h.journal_entries(n(2)));
}

#[test]
fn reordered_start_view_is_retained_and_installed_by_the_tick_redrive() {
    let mut h = cluster();
    bootstrap(&mut h);
    establish(&mut h);

    // The responder answers and opens its fetch — but the leader's
    // transition completes BEFORE the fetch does: the StartView arrives
    // at a node whose table does not yet record the successor era.
    deliver(&mut h, n(2), Tag::PlannedViewChange);
    deliver(&mut h, n(0), Tag::DoViewChange);
    assert_eq!(current_view(&h, n(0)), transition_view());

    // The offer is not dropped: it is retained, and the already-open
    // fetch keeps running (§13.1 step 5's ruling, applied to the era
    // that makes the offer evaluable).
    let offer = deliver(&mut h, n(2), Tag::StartView);
    assert!(
        matches!(offer.outcome, StepOutcome::Published { .. }),
        "the early offer is retained, not refused: {:?}",
        offer.outcome
    );
    assert_eq!(status_of(&h, n(2)), Status::Normal);
    assert_eq!(current_view(&h, n(2)), view(0), "not yet installed");

    // The fetch completes through the leader's post-switch serving; the
    // era folds; the ordinary tick re-runs the retained ruling and the
    // node installs the transition view.
    deliver(&mut h, n(0), Tag::GetState);
    deliver(&mut h, n(2), Tag::NewState);
    assert_eq!(current_era(&h, n(2)), Era(2));
    h.tick(n(2));
    assert_eq!(
        current_view(&h, n(2)),
        transition_view(),
        "the retained offer installed once evaluable"
    );
    assert_eq!(status_of(&h, n(2)), Status::Normal);

    h.deliver_all();
    h.assert_safety();
    assert_eq!(h.journal_entries(n(0)), h.journal_entries(n(2)));
}

#[test]
fn leader_serves_the_fetch_across_its_own_switch() {
    let mut h = cluster();
    bootstrap(&mut h);
    establish(&mut h);

    // The responder's fetch reaches the leader only AFTER the leader has
    // switched: serving is read-only retransmission, and a known era is
    // a known era (§13.1) — the leader answers from view v' for the
    // current-view request.
    deliver(&mut h, n(2), Tag::PlannedViewChange);
    deliver(&mut h, n(0), Tag::DoViewChange);
    assert_eq!(current_view(&h, n(0)), transition_view());
    let served = deliver(&mut h, n(0), Tag::GetState);
    let sent = sends(&served.outcome);
    let chunk = sent
        .iter()
        .find(|(_, _, message)| message.header.tag == Tag::NewState)
        .expect("the leader served the fetch after its switch");
    assert_eq!(chunk.1, Era(1), "served under the requested era's record");
    assert_eq!(chunk.2.header.view, view(0), "echoing the request's view");

    deliver(&mut h, n(2), Tag::NewState);
    assert_eq!(current_era(&h, n(2)), Era(2));
    deliver(&mut h, n(2), Tag::StartView);
    assert_eq!(current_view(&h, n(2)), transition_view());

    h.deliver_all();
    h.assert_safety();
    assert_eq!(h.journal_entries(n(0)), h.journal_entries(n(2)));
}

#[test]
fn transition_view_exhaustion_refuses_the_proposal_before_the_log() {
    let mut h = cluster();
    bootstrap(&mut h);

    // Establish era 2 with weights [2, 1, 1] — the shape that keeps a
    // pivot legal — and walk the leader to the last representable view:
    // the forced change lands at MAX − 1 (the fence that could never be
    // superseded is itself refused), and one ordinary suspicion-driven
    // change lands at MAX.
    let outcome = h.reconfigure(n(0), SystemOperation::Increment(n(0)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    deliver(&mut h, n(1), Tag::Prepare);
    deliver(&mut h, n(2), Tag::Prepare);
    deliver(&mut h, n(0), Tag::PrepareOk);
    deliver(&mut h, n(0), Tag::PrepareOk);
    deliver_matching(&mut h, n(1), Tag::Commit, Slot(3));
    deliver_matching(&mut h, n(2), Tag::Commit, Slot(3));
    assert_eq!(current_era(&h, n(0)), Era(2));
    assert_eq!(current_era(&h, n(1)), Era(2));
    assert_eq!(current_era(&h, n(2)), Era(2));
    // The forced change's primary under the era-2 order is n2; the fence
    // and evidence quorums (threshold 3 under weights [2, 1, 1]) each
    // need n0's weight, so the completion waits on n0's evidence.
    let forced = era2_view(u32::MAX - 1);
    let outcome = h.force_view(n(0), forced);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the forced fence publishes: {outcome:?}"
    );
    deliver_matching(&mut h, n(1), Tag::StartViewChange, Slot::NONE);
    deliver_matching(&mut h, n(2), Tag::StartViewChange, Slot::NONE);
    deliver_matching(&mut h, n(0), Tag::StartViewChange, Slot::NONE);
    deliver_matching(&mut h, n(2), Tag::DoViewChange, Slot(3));
    deliver_matching(&mut h, n(2), Tag::DoViewChange, Slot(3));
    deliver_matching(&mut h, n(0), Tag::StartView, Slot(3));
    deliver_matching(&mut h, n(1), Tag::StartView, Slot(3));
    assert_eq!(current_view(&h, n(0)), forced);
    h.deliver_all();
    h.assert_safety();
    // One ordinary change past the forced view: a backup suspects the
    // silent primary, the fence and the evidence reach n0 — the primary
    // of (2, MAX) — and the quorum installs the last representable view.
    for _ in 0..=TIMEOUT {
        h.tick(n(1));
    }
    assert_eq!(status_of(&h, n(1)), Status::ViewChange);
    deliver_matching(&mut h, n(0), Tag::StartViewChange, Slot::NONE);
    // n1's own fence quorum completes only when the leader's fence
    // arrives; then its evidence flows to the target's primary.
    deliver_matching(&mut h, n(1), Tag::StartViewChange, Slot::NONE);
    deliver_matching(&mut h, n(0), Tag::DoViewChange, Slot(3));
    let last = era2_view(u32::MAX);
    assert_eq!(
        current_view(&h, n(0)),
        last,
        "the ordinary change reached the top of the view space"
    );

    // Step 5's `v'` does not exist: the refusal is named, and the
    // operation never entered the log. The pivot is legal under era 2's
    // arithmetic ({n0, n1} a view quorum, {n0, n2} a commit quorum under
    // both configurations), so ONLY the exhaustion refuses.
    let exhaustion_pivot = Pivot {
        q_i: vec![n(0), n(1)],
        q_ii: vec![n(0), n(2)],
    };
    let outcome = h.reconfigure(
        n(0),
        SystemOperation::Increment(n(1)),
        Some(exhaustion_pivot),
    );
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::ReconfigureViewExhausted { current: last })
    );
    assert!(
        h.peek_queued(n(1), Tag::Prepare).is_none(),
        "nothing was proposed"
    );
    assert!(h.peek_queued(n(2), Tag::Prepare).is_none());
    assert_eq!(snap(&h, n(0)).accepted, 3, "the log did not grow");

    h.assert_safety();
}
