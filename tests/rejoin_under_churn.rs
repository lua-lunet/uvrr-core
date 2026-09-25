//! Rejoin under churn: a behind node must converge back into a serving
//! cluster whose view keeps advancing.
//!
//! The live traces named the blocked transition — a `StartView` landing
//! exactly on the restarted node's current view while it fences, and
//! `GapDetected` fetches that never complete before the next churn
//! completion resets the attempt. This corpus strips the phi/jitter host
//! policy and scripts the churn deterministically, one link per test:
//! the delta-0 adoption in isolation, the polite-churn rejoin, the
//! retained-base shortfall rejoin, and the reincarnated identity's path
//! back into membership.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::ids::{CrashCounter, Era, NodeId, OperationId, Slot, SystemId, View, ViewId};
use vrr::message::{Body, Message};
use vrr::progress::Status;
use vrr::wire::{Header, Pack, Tag};

const ALL: [NodeId; 3] = [n(0), n(1), n(2)];

const fn n(id: u32) -> NodeId {
    NodeId::new(
        SystemId::new((id + 1) as u16).expect("test system ids are small and non-zero"),
        CrashCounter::new(1).expect("one is non-zero"),
    )
}

fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

/// A journal entry as the wire packs it, for budget arithmetic.
fn operation_entry(slot: u64, lsb: u64, payload: &[u8]) -> vrr::journal::LogEntry {
    vrr::journal::LogEntry {
        slot: Slot(slot),
        era: Era(1),
        payload: vrr::journal::Payload::Operation {
            id: op_id(lsb),
            payload: payload.to_vec().into_boxed_slice(),
        },
    }
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

fn primary_of(h: &Harness, observer: NodeId, view: ViewId) -> Option<NodeId> {
    h.era_table(observer)?
        .record(view.era)
        .and_then(|record| record.config.primary(view.view))
}

/// The view-change target the fence machinery itself chooses for `node`:
/// the next view in the era the committed history has established
/// (§8.7.8).
fn fence_target(h: &Harness, node: NodeId) -> ViewId {
    let current = current_view(h, node);
    ViewId {
        era: current.era,
        view: View(current.view.0 + 1),
    }
}

/// Drives the view change the fence machinery targets, asserting every
/// node in `live` installs it. The driver is any live `Normal` member —
/// two live members of a three-node unit cluster reach the fence quorum.
fn drive_view_change(h: &mut Harness, live: &[NodeId]) -> ViewId {
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
    target
}

/// One churn round: silence long enough for every live `Normal` member to
/// suspect the primary, then the drain that completes the rotation. The
/// cluster's view advances by one at a polite, regular cadence — no phi,
/// no jitter, nothing but the S4 suspicion rule.
fn rotate(h: &mut Harness, live: &[NodeId]) {
    let before = current_view(h, live[0]);
    for _ in 0..=TIMEOUT {
        h.tick_all();
        h.deliver_all();
    }
    let after = current_view(h, live[0]);
    assert!(after > before, "the rotation advanced the view: {after:?}");
}

/// The cluster serves `ops` more operations through the current view's
/// primary and drains the traffic. The primary is picked among every up
/// node: the churn may well have rotated into the restarted member. A
/// refusal is storm weather — the primary may be mid-attempt — and the
/// attempt is simply skipped.
fn serve(h: &mut Harness, all: &[NodeId], ops: &mut u64) {
    for _ in 0..2 {
        *ops += 1;
        let observer = all
            .iter()
            .copied()
            .find(|&id| h.snapshot(id).is_some())
            .expect("some node is up to serve");
        let view = current_view(h, observer);
        let Some(primary) =
            primary_of(h, observer, view).filter(|&primary| h.snapshot(primary).is_some())
        else {
            return;
        };
        if h.snapshot(primary)
            .and_then(|s| Status::from_word(s.status))
            != Some(Status::Normal)
        {
            return;
        }
        let _ = h.propose(primary, op_id(*ops), b"o");
        h.deliver_all();
    }
}

/// A restarted node rejoins a serving, politely churning cluster: it
/// adopts the completions, wins its own attempt, and ends `Normal` at the
/// cluster's current view with the cluster's committed frontier.
#[test]
fn a_restarted_node_adopts_completions_while_the_cluster_churns() {
    let mut h = cluster();
    bootstrap(&mut h);
    let live = [n(1), n(2)];

    // n0 serves a few operations, then crashes with a complete disk.
    let mut ops = 0u64;
    serve(&mut h, &ALL, &mut ops);
    h.halt(n(0));

    // The survivors keep serving and rotating while n0 is down. A rotation
    // whose designated primary is the down member cannot complete (the
    // evidence lands nowhere), so the down-period advances the two live
    // positions and stops: two views, a served frontier, the disk stale.
    rotate(&mut h, &live);
    serve(&mut h, &ALL, &mut ops);
    rotate(&mut h, &live);
    serve(&mut h, &ALL, &mut ops);
    let cluster_view = current_view(&h, n(1));
    let cluster_committed = snap(&h, n(1)).committed;
    assert_eq!(cluster_view.view, View(2), "the cluster is two views on");

    // The restart: the boot rule fences the node `Restarting` at its
    // stale persisted view.
    h.restart_with(n(0)).expect("the disk reopens");
    assert_eq!(status_of(&h, n(0)), Status::Restarting);

    // The rejoin drive: the cluster keeps serving and rotating; the
    // restarted node must adopt the completions and converge.
    let mut converged = false;
    for round in 0..80 {
        h.tick_all();
        h.deliver_all();
        if round % 4 == 3 {
            serve(&mut h, &ALL, &mut ops);
        }
        if round % 8 == 7 {
            rotate(&mut h, &live);
        }
        let status = status_of(&h, n(0));
        if status == Status::Normal && current_view(&h, n(0)) >= cluster_view {
            converged = true;
            break;
        }
    }
    assert!(
        converged,
        "the restarted node never rejoined: status={:?} view={:?} cluster={:?} committed={:?}\n{}",
        status_of(&h, n(0)),
        current_view(&h, n(0)),
        current_view(&h, n(1)),
        snap(&h, n(0)).committed,
        h.trace_dump()
    );
    assert_eq!(current_view(&h, n(0)), current_view(&h, n(1)));
    assert!(snap(&h, n(0)).committed >= cluster_committed);
    h.assert_safety();
}

/// The storm shape: the cluster rotates back-to-back — a new attempt is
/// under way every couple of ticks, matching the live run's ~4 views/s —
/// while the restarted node chases. The node must still converge: adopt a
/// completion or win an attempt before the next rotation retargets it.
#[test]
fn a_restarted_node_converges_under_a_dense_churn_storm() {
    let mut h = cluster();
    bootstrap(&mut h);
    let live = [n(1), n(2)];

    let mut ops = 0u64;
    serve(&mut h, &ALL, &mut ops);
    h.halt(n(0));
    rotate(&mut h, &live);
    serve(&mut h, &ALL, &mut ops);
    rotate(&mut h, &live);
    serve(&mut h, &ALL, &mut ops);
    let cluster_view = current_view(&h, n(1));
    let cluster_committed = snap(&h, n(1)).committed;

    h.restart_with(n(0)).expect("the disk reopens");

    // Dense storm: every round is a rotation; serving is rare. No polite
    // gaps for the chaser to land in.
    let mut converged = false;
    for round in 0..120 {
        h.tick_all();
        h.deliver_all();
        if round % 7 == 6 {
            serve(&mut h, &ALL, &mut ops);
        }
        let status = status_of(&h, n(0));
        if status == Status::Normal && current_view(&h, n(0)) >= cluster_view {
            converged = true;
            break;
        }
    }
    assert!(
        converged,
        "the restarted node never converged in the storm: status={:?} view={:?} cluster={:?} committed={:?}\n{}",
        status_of(&h, n(0)),
        current_view(&h, n(0)),
        current_view(&h, n(1)),
        snap(&h, n(0)).committed,
        h.trace_dump()
    );
    assert_eq!(current_view(&h, n(0)), current_view(&h, n(1)));
    assert!(snap(&h, n(0)).committed >= cluster_committed);
    h.assert_safety();
}

/// The live run's distinguishing ingredient: the restarted node's host
/// polls the §14.2 forced view upward on a timer — `current + 1`, blind to
/// what the cluster is completing. Every forced view moves the node's
/// current one past the in-flight attempt, and the adoption rule refuses a
/// `StartView` one view behind (`StartViewFromStaleView`). The node must
/// still converge: the host's poll keeps firing while the completions keep
/// arriving, and something has to give.
///
/// The escalation fires the poll on EVERY round — the live trace's ~90
/// views/s climb, a forced view per arrival in the limbo.
#[test]
fn a_forced_view_poll_on_the_restarted_node_still_converges() {
    let mut h = cluster();
    bootstrap(&mut h);
    let live = [n(1), n(2)];

    let mut ops = 0u64;
    serve(&mut h, &ALL, &mut ops);
    h.halt(n(0));
    rotate(&mut h, &live);
    serve(&mut h, &ALL, &mut ops);
    rotate(&mut h, &live);
    serve(&mut h, &ALL, &mut ops);
    let cluster_view = current_view(&h, n(1));
    let cluster_committed = snap(&h, n(1)).committed;

    h.restart_with(n(0)).expect("the disk reopens");

    // The storm runs, and the host's forced-view poll fires on the
    // restarted node every round — blindly, current + 1.
    let mut converged = false;
    for round in 0..120 {
        h.tick_all();
        h.deliver_all();
        let current = current_view(&h, n(0));
        h.force_view(
            n(0),
            ViewId {
                era: current.era,
                view: View(current.view.0 + 1),
            },
        );
        if round % 9 == 8 {
            serve(&mut h, &ALL, &mut ops);
        }
        let status = status_of(&h, n(0));
        if status == Status::Normal && current_view(&h, n(0)) >= cluster_view {
            converged = true;
            break;
        }
    }
    assert!(
        converged,
        "the forced-view poll wedged the rejoin: status={:?} view={:?} cluster={:?} committed={:?}\n{}",
        status_of(&h, n(0)),
        current_view(&h, n(0)),
        current_view(&h, n(1)),
        snap(&h, n(0)).committed,
        h.trace_dump()
    );
    assert_eq!(current_view(&h, n(0)), current_view(&h, n(1)));
    assert!(snap(&h, n(0)).committed >= cluster_committed);
    h.assert_safety();
}

/// The live storm's second dimension: the node is not merely behind in
/// views but in slots, and the host's `view_change_budget` cuts every
/// `StartView` suffix to a sliver — so every completion the restarted node
/// receives starts past its frontier (`GapDetected`) and demands a fetch
/// that must survive the churn. The node must still converge.
#[test]
fn a_frontier_gap_rejoins_through_the_fetch_under_churn() {
    // The budget carries exactly one journal entry: every StartView
    // suffix, every chunk, one slot wide — the storm's budget-cut shape.
    let per_entry = operation_entry(3, 1, b"o").packed_len();
    let mut h = Harness::with_knobs(
        3,
        vrr::replica::ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: per_entry,
        },
    );
    bootstrap(&mut h);
    let live = [n(1), n(2)];

    // A served history, then the crash: n0's disk holds the whole range.
    let mut ops = 0u64;
    for _ in 0..3 {
        serve(&mut h, &ALL, &mut ops);
    }
    let n0_frontier = snap(&h, n(0)).accepted;
    h.halt(n(0));

    // Two views and more served slots while n0 is down.
    rotate(&mut h, &live);
    for _ in 0..2 {
        serve(&mut h, &ALL, &mut ops);
    }
    rotate(&mut h, &live);
    for _ in 0..2 {
        serve(&mut h, &ALL, &mut ops);
    }
    let cluster_view = current_view(&h, n(1));
    let cluster_committed = snap(&h, n(1)).committed;
    assert!(cluster_committed > n0_frontier + 2, "the frontier raced on");

    h.restart_with(n(0)).expect("the disk reopens");

    // The rejoin drive with light churn. Every completion arrives with a
    // one-entry suffix that starts past n0's frontier: GapDetected, fetch,
    // one chunk per round trip — while the cluster keeps rotating.
    let mut converged = false;
    for round in 0..400 {
        h.tick_all();
        h.deliver_all();
        if round % 9 == 8 {
            serve(&mut h, &ALL, &mut ops);
        }
        if round % 27 == 26 {
            rotate(&mut h, &live);
        }
        let status = status_of(&h, n(0));
        if status == Status::Normal && current_view(&h, n(0)) >= cluster_view {
            converged = true;
            break;
        }
    }
    assert!(
        converged,
        "the gapped node never rejoined: status={:?} view={:?} accepted={:?} cluster={:?} cluster_committed={:?}\n{}",
        status_of(&h, n(0)),
        current_view(&h, n(0)),
        snap(&h, n(0)).accepted,
        current_view(&h, n(1)),
        snap(&h, n(1)).committed,
        h.trace_dump()
    );
    assert_eq!(current_view(&h, n(0)), current_view(&h, n(1)));
    assert!(snap(&h, n(0)).committed >= cluster_committed);
    h.assert_safety();
}

/// The dirty-boot bump: the crashed genesis member comes back under a NEW
/// identity that no configuration names — every peer discards it by name
/// (`UnknownSender`) until the announcement machine seats it. The
/// announcement must carry it through the forced sequence into voting
/// membership, and the seated member must then survive the churn.
#[test]
fn a_reincarnated_identity_is_seated_by_the_forced_sequence_then_survives_churn() {
    let mut h = cluster();
    bootstrap(&mut h);
    let live = [n(1), n(2)];

    let mut ops = 0u64;
    for _ in 0..2 {
        serve(&mut h, &ALL, &mut ops);
    }
    h.crash(n(0));
    rotate(&mut h, &live);
    for _ in 0..2 {
        serve(&mut h, &ALL, &mut ops);
    }

    // The dirty boot: the disk reopens under a fresh identity nobody
    // names. Until seated, every peer discards its traffic by name.
    let bumped = n(0)
        .next_life()
        .expect("the test never exhausts the counter");
    h.restart_as(n(0), bumped)
        .expect("the disk reopens under the bump");
    h.reincarnate(bumped, n(0));

    // The announcement drives the forced sequence: era by era the leader
    // joins the new identity, grants its weight, and evicts the dead one.
    let mut seated = false;
    for _ in 0..400 {
        h.tick_all();
        h.deliver_all();
        let weights = h
            .era_table(bumped)
            .map(|table| {
                table
                    .current()
                    .config
                    .weight_of(bumped)
                    .map(|weight| weight.0)
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        if weights >= 1
            && h.snapshot(bumped).and_then(|s| Status::from_word(s.status)) == Some(Status::Normal)
        {
            seated = true;
            break;
        }
    }
    assert!(
        seated,
        "the bumped identity was never seated: members={:?} status={:?}\n{}",
        h.era_table(n(1)).map(|t| t
            .current()
            .config
            .order()
            .iter()
            .map(|m| (m.node, u64::from(m.weight.0)))
            .collect::<Vec<_>>()),
        h.snapshot(bumped).and_then(|s| Status::from_word(s.status)),
        h.trace_dump()
    );

    // The seated member survives the ordinary churn: rotations roll
    // through its position and it keeps serving.
    for round in 0..12 {
        h.tick_all();
        h.deliver_all();
        if round % 4 == 3 {
            serve(&mut h, &ALL, &mut ops);
        }
    }
    assert_eq!(status_of(&h, bumped), Status::Normal);
    h.assert_safety();
}

/// A `StartView` for the very view a member is fencing into — the delta-0
/// case the live traces dropped sixty-four times — installs the offered
/// history: the adoption rule admits "fencing into this very view"
/// (§13.1), the offer names the view's primary, and the suffix verifies
/// against the member's journal.
#[test]
fn a_start_view_landing_on_the_fencing_node_adopts() {
    let mut h = cluster();
    bootstrap(&mut h);

    // A committed operation so the histories have substance beyond genesis.
    let outcome = h.propose(n(0), op_id(1), b"a");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();

    // The primary is partitioned away; the two backups complete view 1
    // between themselves. The StartView aimed at the primary is held.
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    let target = drive_view_change(&mut h, &[n(1), n(2)]);
    assert_eq!(target.view, View(1));
    assert_eq!(primary_of(&h, n(1), target), Some(n(1)));

    // The partitioned primary is fenced into that same view by a scripted
    // fence vote: its current is now the target view, status ViewChange.
    h.inject(
        n(2),
        n(0),
        Message {
            header: Header {
                tag: Tag::StartViewChange,
                view: target,
                slot: Slot::NONE,
            },
            body: Body::StartViewChange {},
        },
    );
    assert_eq!(status_of(&h, n(0)), Status::ViewChange, "n0 is fencing");
    assert_eq!(current_view(&h, n(0)), target, "the fence names the view");

    // The partition heals: the held StartView lands on the fencing node
    // with got == current. This is the delta-0 adoption.
    h.heal();
    h.deliver_all();

    assert_eq!(
        status_of(&h, n(0)),
        Status::Normal,
        "the delta-0 StartView installs"
    );
    assert_eq!(current_view(&h, n(0)), target);
    assert_eq!(snap(&h, n(0)).committed, snap(&h, n(1)).committed);
    h.assert_safety();
}
