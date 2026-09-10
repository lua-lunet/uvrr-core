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

/// Admits `joiner` at the next appended position (stop-the-world — no
/// pivot exists for a membership change), drives the ordinary view change
/// into the era the join committed, exactly as [`joined_and_caught_up`]
/// does for the first joiner, and runs the joiner's §10 catch-up: the
/// offer installs on the ordinary tick, not on delivery alone. Returns
/// the fence target; the caller asserts the era.
fn admit_joiner(h: &mut Harness, joiner: NodeId, position: u32) -> ViewId {
    let proposer =
        primary_of(h, n(0), current_view(h, n(0))).expect("a live view names its primary");
    h.boot_as(joiner)
        .expect("the joiner boots over genesis knowledge");
    let outcome = h.reconfigure(
        proposer,
        SystemOperation::Join {
            node: joiner,
            position,
        },
        None,
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    let target = drive_view_change(h, &[n(0), n(1), n(2), n(3)]).0;
    catch_up(h, joiner);
    target
}

/// Ticks the learner between delivery rounds until it is caught up — the
/// amended tick shape this corpus uses throughout: a real sans-I/O host
/// ticks, so each ordinary tick re-runs the retained offer's stalled
/// ruling, each round folds one era (the §8.7.3 window) and the next
/// round proceeds, and the offer installs once its era is evaluable.
fn catch_up(h: &mut Harness, learner: NodeId) {
    let mut rounds = 0;
    while status_of(h, learner) != Status::Normal
        || snap(h, learner).committed != snap(h, n(0)).committed
    {
        rounds += 1;
        assert!(
            rounds <= 8,
            "the learner's era-by-era catch-up wedged after {rounds} rounds (last diagnostic {:?})\n{}",
            h.diagnostic(learner),
            h.trace_dump()
        );
        h.tick(learner);
        h.deliver_all();
    }
}

/// A joiner admitted SEVERAL eras past its boot table (the issue-#13
/// fourth finding): boot three genesis voters, admit three joiners across
/// successive eras — one committed `Join` per era — then promote the
/// second joiner. The promoted joiner folds the eras its boot table is
/// behind on through the §10 acquisition, era by era — one fold per
/// stalled-ruling re-run (the §8.7.3 window caps each round at one era
/// past the view it carries), the next round proceeds, and the offer
/// installs once its era is evaluable — never voting in an era it has not
/// folded. The voter-only succession then designates it for a view it can
/// evaluate, and the cluster commits under the new configuration — first
/// under its own leadership, then with one incumbent partitioned, when
/// its vote is the difference.
#[test]
fn a_joiner_admitted_several_eras_past_its_boot_table_when_promoted_keeps_the_cluster_committing() {
    let mut h = cluster();
    joined_and_caught_up(&mut h);

    // The second and third joiners admit across successive eras.
    let second = admit_joiner(&mut h, n(4), 4);
    assert_eq!(second.era, Era(3), "the second join committed");
    let third = admit_joiner(&mut h, n(5), 5);
    assert_eq!(third.era, Era(4), "the third join committed");
    // The promoted joiner's own catch-up keeps pace era by era: its era-4
    // offer was retained while the third joiner admitted.
    catch_up(&mut h, n(4));

    // The promotion of the second joiner: era 5 folds under the new
    // configuration — order [0, 1, 2, 3, 4, 5], the promoted joiner the
    // fourth voter.
    let promote_primary =
        primary_of(&h, n(0), current_view(&h, n(0))).expect("a live view names its primary");
    let outcome = h.reconfigure(promote_primary, SystemOperation::Increment(n(4)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(current_era(&h, n(0)), Era(5), "the promotion committed");
    assert_eq!(current_weights(&h, n(0)), vec![1, 1, 1, 0, 1, 0]);

    // The ordinary view change into the promotion era.
    let (era5, _) = drive_view_change(&mut h, &[n(0), n(1), n(2), n(3)]);
    assert_eq!(era5.era, Era(5));
    catch_up(&mut h, n(4));

    // The promoted joiner caught up: Normal, at the live view, holding
    // the leader's committed frontier.
    assert_eq!(
        status_of(&h, n(4)),
        Status::Normal,
        "the promoted joiner folded the eras past its boot table and caught up (last diagnostic {:?})",
        h.diagnostic(n(4)),
    );
    assert_eq!(current_view(&h, n(4)), era5);
    assert_eq!(snap(&h, n(4)).committed, snap(&h, n(0)).committed);

    // Electable: the voter-only succession walks to the promoted joiner —
    // view 7 of era 5 selects the fourth voter.
    for expected in [View(5), View(6), View(7)] {
        let (target, _) = drive_view_change(&mut h, &[n(0), n(1), n(2), n(3)]);
        assert_eq!(target.view, expected, "the succession advances");
    }
    let last = ViewId {
        era: Era(5),
        view: View(7),
    };
    assert_eq!(
        primary_of(&h, n(0), last),
        Some(n(4)),
        "the promoted joiner is the designated primary of view 7"
    );
    // The promoted joiner holds the view it leads: the succession's
    // ordinary announcements carried it there.
    assert_eq!(
        current_view(&h, n(4)),
        last,
        "the promoted joiner leads the view it can evaluate"
    );

    // Effective: the cluster commits under the promoted joiner's own
    // leadership.
    let outcome = h.propose(n(4), op_id(30), b"under-new-config");
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the cluster commits under the promoted joiner: {outcome:?}\n{}",
        h.trace_dump()
    );
    h.deliver_all();
    let committed = snap(&h, n(4)).committed;

    // And its vote is required: with one incumbent partitioned, the
    // era-5 arithmetic (total 4, threshold 3) commits only with the
    // promoted joiner's vote — the leader and one incumbent weigh 2.
    h.partition(vec![n(2)], vec![n(0), n(1), n(3), n(4)]);
    let outcome = h.propose(n(4), op_id(31), b"vote-required");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert!(
        snap(&h, n(4)).committed > committed,
        "the cluster keeps committing under the new configuration with an incumbent partitioned"
    );

    h.heal();
    h.drop_held();
    h.deliver_all();
    h.assert_safety();
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

/// The live acquisition shape: the responder's committed frontier RUNS
/// between the retained offer and the answering chunk. The establishing
/// fan-out reaches members only, so the learner's first sight of its
/// admitting era is the fence's `StartView` — one or more eras past its boot
/// table, retained with a fetch (§13.1 step 5). Live, the primary keeps
/// committing while the fetch is in flight, so the answering chunk's
/// committed frontier runs past the offer's — and the acquisition that takes
/// the chunk's frontier whole strands the offer forever: the retained
/// ruling's staleness gate then refuses an offer that claims less than the
/// node durably holds, no tick ever completes it, and the learner is fenced
/// for good. The acquisition therefore folds what the retained offer needs —
/// the chunk's frontier, capped by the offer's own committed frontier — and
/// the offer installs on the next ordinary tick; the era's stream (R6
/// reaches learners) carries the tail.
#[test]
fn the_acquisition_completes_when_the_responder_runs_ahead_of_the_offer() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.boot_as(n(3))
        .expect("the joiner boots over genesis knowledge");
    // Client operations commit before the join, so the fetched chunk
    // carries operation slots — the §11 upcalls a live stream supplies.
    for lsb in 1..=2u64 {
        let outcome = h.propose(n(0), op_id(lsb), format!("op{lsb}").as_bytes());
        assert!(matches!(outcome, StepOutcome::Published { .. }));
    }
    h.deliver_all();
    // The join commits; era 2 folds at every incumbent. The learner's
    // table still holds only the genesis fold.
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
    assert_eq!(current_era(&h, n(0)), Era(2), "the join committed");
    assert_eq!(current_era(&h, n(3)), Era(1), "the learner is fenced");

    // The fence into era 2 — driven and delivered among the incumbents
    // only, so the era-2 `StartView` sits queued at the learner, unused.
    let target = fence_target(&h, n(0));
    for _ in 0..=TIMEOUT {
        h.tick(n(0));
    }
    for _ in 0..16 {
        for id in [n(0), n(1), n(2)] {
            h.deliver_to(id);
        }
    }
    assert_eq!(
        current_view(&h, n(0)),
        target,
        "the incumbents entered the established era"
    );
    // The era-2 primary (view 1 selects position 1) commits MORE client
    // operations — the responder now runs ahead of the retained offer.
    let win_committed = snap(&h, n(1)).committed;
    for lsb in 10..=11u64 {
        let outcome = h.propose(n(1), op_id(lsb), format!("op{lsb}").as_bytes());
        assert!(matches!(outcome, StepOutcome::Published { .. }));
    }
    for _ in 0..8 {
        for id in [n(0), n(1), n(2)] {
            h.deliver_to(id);
        }
    }
    assert!(
        snap(&h, n(0)).committed > win_committed,
        "the incumbents commit newer operations past the offer's frontier"
    );

    // The learner's first sight of its admitting era: the retained offer
    // and the fetch it opens.
    h.deliver_tag(n(3), Tag::StartView);
    h.deliver_to(n(1));
    h.deliver_tag(n(3), Tag::NewState);
    // The acquisition folds the admitting era; the retained offer
    // installs on the next ordinary tick.
    assert_eq!(
        current_era(&h, n(3)),
        Era(2),
        "the learner folded the era that admitted it"
    );
    h.tick(n(3));
    assert_eq!(
        status_of(&h, n(3)),
        Status::Normal,
        "the retained offer installed"
    );
    assert_eq!(
        current_view(&h, n(3)),
        target,
        "the learner joined the offered view"
    );
    // The offer's own frontier: the acquisition folded what the offer
    // needed; the responder's later commits arrive with the stream.
    assert_eq!(
        snap(&h, n(3)).committed,
        win_committed,
        "the learner's frontier is the offer's; the stream carries the tail"
    );

    // The era's stream catches the learner up to the responder.
    h.deliver_all();
    assert_eq!(
        snap(&h, n(3)).committed,
        snap(&h, n(0)).committed,
        "the learner is caught up"
    );
    h.assert_safety();
}

/// The serving gate answers a fetch whose requested era the responder's
/// retention window has moved past. The window is two eras (§8.7.1): a
/// reincarnated node — or any boot-fenced learner — fetches under an era of
/// its OWN choosing, its boot view, and an incumbent that has folded two
/// reconfigurations since no longer retains that record. Serving is
/// read-only retransmission of durable journal content to a member the
/// current configuration vouches for; the requested era's unavailability
/// removes only the requested-era disjunct, never the service.
#[test]
fn the_serving_gate_answers_a_fetch_from_a_past_window_era() {
    let mut h = cluster();
    joined_and_caught_up(&mut h);
    // A second reconfiguration folds era 3 at every incumbent: the
    // retention window is now [2, 3] and era 1 — the learner's boot era —
    // is evicted.
    let outcome = h.reconfigure(n(1), SystemOperation::Double, None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(current_era(&h, n(0)), Era(3), "era 3 folded");
    assert_eq!(
        current_era(&h, n(3)),
        Era(3),
        "the learner folded along with the incumbents"
    );

    // A boot-fenced node of this deployment (the reincarnation shape)
    // fetches under its boot view: era 1, view 0. The requester is a
    // member of the responder's current configuration at weight 0.
    let request = Message {
        header: Header {
            tag: Tag::GetState,
            view: ViewId {
                era: Era(1),
                view: View(0),
            },
            slot: vrr::ids::Slot(2),
        },
        body: Body::GetState {
            from: vrr::ids::Slot(3),
        },
    };
    let before = h.queued_len();
    h.inject(n(3), n(0), request);
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::None),
        "the fetch is served, not dropped: the sender is a current-configuration member"
    );
    assert!(
        h.queued_len() > before,
        "a NewState answer is queued for the requester"
    );
    assert!(!snap(&h, n(0)).faulted, "a serve, never a fault");
    h.assert_safety();
}
