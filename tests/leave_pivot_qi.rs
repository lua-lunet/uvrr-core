//! The Leave-pivot `qI` finding (issue #13): a `Leave` whose pivot places
//! the departing member inside `qI` — the standby rides into `qI` on the
//! §8.7.6 cardinality rule (`|qI| + |qII| = N + 1`) — must still complete
//! its planned quorum. The construction explicitly solicits the departed
//! identity's vote (`src/replica/reconfiguration.rs`: "`qI − {L}` ... a
//! member the reconfiguration removes must still get its vote in"), so the
//! §6 membership-discard at ingress must not swallow that one answer: the
//! planned quorum completes with it, the transition publishes as one step,
//! and the era never needs the ordinary fence.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{EraTable, SystemOperation};
use vrr::ids::{Era, NodeId, Slot, View, ViewId};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
use vrr::quorum::{WeightedMajority, construct_pivot};
use vrr::replica::ViewChangeKnobs;
use vrr::wire::Tag;

fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// A view in era 2 — the era the standby-bearing `DECREMENT` establishes.
fn era2_view(number: u32) -> ViewId {
    ViewId {
        era: Era(2),
        view: View(number),
    }
}

/// A view in era 3 — the era the planned `Leave` establishes.
fn era3_view(number: u32) -> ViewId {
    ViewId {
        era: Era(3),
        view: View(number),
    }
}

/// The timeout knob for the view-change scripts: a node suspects its
/// primary after more than three ticks of silence (S4).
const TIMEOUT: u64 = 3;

/// A four-node cluster whose view-change machinery is live.
fn cluster() -> Harness {
    Harness::with_knobs(
        4,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    )
}

/// Bootstraps the cluster: the genesis primary promotes itself and every
/// backup adopts view (1, 0) from the promotion's `Commit` announcement
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

/// Delivers the queued datagram with `tag` to `id`, asserting it exists.
fn deliver(h: &mut Harness, id: NodeId, tag: Tag) -> harness::DeliveryOutcome {
    h.deliver_tag(id, tag)
        .unwrap_or_else(|| panic!("a {tag:?} is queued for n{}", id.0))
}

/// The era table one `Leave(n3)` past `table`: the fold the Leave's own
/// establishing slot would record. The slot is one past the primary's
/// accepted frontier — where the entry will sit.
fn era_after_leave(table: &EraTable, accepted: Slot) -> EraTable {
    table
        .extend(
            &SystemOperation::Leave(n(3)),
            accepted.next().expect("a frontier has a successor"),
        )
        .expect("the Leave folds at weight zero")
}

#[test]
fn the_departed_qi_members_solicited_planned_vote_is_counted() {
    let mut h = cluster();
    bootstrap(&mut h);

    // Phase 1: drive n3's weight to zero — the stop-the-world `DECREMENT`
    // commits at slot 3 and establishes era 2 (weights 1, 1, 1, 0) — then
    // the ordinary view change carries the cluster into the established
    // era (§8.7.8). n1 is the primary of era-2 view 1: the voters are
    // n0, n1, n2 (§8.4; a standby is never the primary).
    let outcome = h.reconfigure(n(0), SystemOperation::Decrement(n(3)), None);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the DECREMENT publishes: {outcome:?}"
    );
    h.deliver_all();
    assert_eq!(current_era(&h, n(0)), Era(2));
    for _ in 0..=TIMEOUT {
        h.tick_all();
    }
    assert_eq!(status_of(&h, n(0)), Status::ViewChange, "the fence fires");
    while h.queued_len() > 0 {
        h.deliver_all();
    }
    for id in [n(0), n(1), n(2), n(3)] {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} installs");
        assert_eq!(current_view(&h, id), era2_view(1), "{id:?} is in era 2");
        assert_eq!(current_era(&h, id), Era(2));
    }

    // Phase 2: the planned Leave. The pivot is `construct_pivot`'s own
    // answer for (config(e), config(e+1), leader n1): qII = {n0, n1} — a
    // commit quorum under both configurations — and qI = {n1, n2, n3} — a
    // view-change quorum under config(e) — with the cardinality rule
    // |qI| + |qII| = N + 1 forcing the departing standby n3 into qI.
    let table = h.era_table(n(1)).expect("the primary is live");
    let accepted = Slot(snap(&h, n(1)).accepted);
    let next = era_after_leave(&table, accepted);
    let pivot = construct_pivot(
        &WeightedMajority,
        &table.current().config,
        &next.current().config,
        n(1),
    )
    .expect("the Leave is plannable");
    assert_eq!(pivot.q_ii, vec![n(0), n(1)]);
    assert_eq!(
        pivot.q_i,
        vec![n(1), n(2), n(3)],
        "the departing member is inside qI by the pivot's own arithmetic"
    );

    // The proposal routes its establishing Prepare to `qII − {L}` = {n0}
    // only; the acceptance completes qII and the commit folds era 3
    // (weights 1, 1, 1 — n3 is gone) and solicits `qI − {L}` = {n2, n3}.
    let outcome = h.reconfigure(n(1), SystemOperation::Leave(n(3)), Some(pivot));
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the Leave publishes: {outcome:?}"
    );
    deliver(&mut h, n(0), Tag::Prepare);
    deliver(&mut h, n(1), Tag::PrepareOk);
    assert_eq!(current_era(&h, n(1)), Era(3), "the commit folded era 3");
    assert_eq!(
        current_view(&h, n(1)),
        era2_view(1),
        "the view has not moved"
    );
    // The commit announcement reaches the era-2 members; the serving
    // members fold era 3 from it (§13.3).
    let c0 = deliver(&mut h, n(0), Tag::Commit);
    deliver(&mut h, n(2), Tag::Commit);
    assert!(
        matches!(c0.outcome, StepOutcome::Published { .. }),
        "the commit announcement publishes at n0: {:?}",
        c0.outcome
    );
    assert_eq!(
        current_era(&h, n(0)),
        Era(3),
        "n0 folds era 3 from the announcement"
    );
    assert_eq!(
        current_era(&h, n(2)),
        Era(2),
        "n2 holds no slot 4: its fold waits for the offer route (§10)"
    );

    // The solicitation reaches BOTH qI recipients — the serving member n2
    // and the departing n3 — and both answer with planned evidence,
    // retaining their views.
    deliver(&mut h, n(2), Tag::PlannedViewChange);
    let answer_outcome = deliver(&mut h, n(3), Tag::PlannedViewChange);
    assert!(
        matches!(answer_outcome.outcome, StepOutcome::Published { .. }),
        "the departing member answers its solicitation: {:?}",
        answer_outcome.outcome
    );

    // The serving member's answer arrives first: collected, quorum still
    // incomplete (n3's vote is outstanding), no transition.
    deliver(&mut h, n(1), Tag::DoViewChange);
    assert_eq!(
        current_view(&h, n(1)),
        era2_view(1),
        "one vote does not complete the quorum"
    );

    // Correct behavior (§8.7.7, and the solicitation's own ruling: "a
    // member the reconfiguration removes must still get its vote in"):
    // the departed identity's solicited planned evidence completes the
    // quorum — the transition publishes as ONE step (the switch to v' and
    // the StartView to config(e+1)), and no drop was named for it.
    deliver(&mut h, n(1), Tag::DoViewChange);
    assert!(
        !matches!(h.diagnostic(n(1)), Some(Diagnostic::UnknownSender { .. })),
        "the departed member's solicited answer is counted, not dropped: {:?}",
        h.diagnostic(n(1))
    );
    assert_eq!(
        current_view(&h, n(1)),
        era3_view(4),
        "the planned quorum completed: the leader switched to v'"
    );
    assert_eq!(status_of(&h, n(1)), Status::Normal);
    assert!(
        h.peek_queued(n(0), Tag::StartView).is_some()
            && h.peek_queued(n(2), Tag::StartView).is_some(),
        "StartView(v') goes to every member of config(e+1)"
    );

    // The members install. n0 folded era 3 from the commit announcement,
    // so its offer installs directly. n2 holds no slot 4: its offer is
    // retained, the fetch it opened at the solicitation folds era 3
    // through the ordinary state transfer, and the tick re-runs the
    // retained ruling — the ordinary install (§10).
    deliver(&mut h, n(0), Tag::StartView);
    deliver(&mut h, n(2), Tag::StartView);
    assert_eq!(current_view(&h, n(0)), era3_view(4), "n0 installs v'");
    assert_eq!(
        current_view(&h, n(2)),
        era2_view(1),
        "n2 retains the offer until its era is evaluable"
    );
    deliver(&mut h, n(1), Tag::GetState);
    deliver(&mut h, n(2), Tag::NewState);
    assert_eq!(
        current_era(&h, n(2)),
        Era(3),
        "the fetched commit folded era 3"
    );
    h.tick(n(2));
    assert_eq!(
        current_view(&h, n(2)),
        era3_view(4),
        "n2 installs v' once evaluable"
    );
    assert_eq!(status_of(&h, n(2)), Status::Normal);

    h.deliver_all();
    h.assert_safety();
    assert_eq!(
        current_era(&h, n(3)),
        Era(2),
        "no era-3 traffic is evaluable at the departed member"
    );
}
