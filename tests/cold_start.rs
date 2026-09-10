//! The cold-start corpus: the staggered genesis start, and the
//! post-genesis full-cluster cold restart.
//!
//! Both shapes are deterministic and clockless: ticks only, no sleeps, no
//! RNG — the harness is the host (`tests/harness/mod.rs`). The knobs every
//! script runs: `primary_timeout = 3` host ticks, `view_change_budget`
//! unbounded — the same view-change knob values the other knob-driven
//! corpora use.
//!
//! # The staggered-provisioning expression
//!
//! The harness provisions a deployment at construction and offers no
//! provision-later entry point, so a node whose process starts later is
//! expressed as: crash it at epoch 0 — before it ever ran — and `reopen`
//! it over the recorded disk when it starts. The disk of a node crashed at
//! epoch 0 is exactly `provision`'s state (the pristine genesis), so the
//! reopened node and a freshly provisioned one are state-identical, and
//! the bootstrap rules treat them alike. Datagrams sent while the node is
//! down sit in the harness's queue and are delivered once it is back: the
//! script decides delivery, as in every corpus.
//!
//! # The shapes
//!
//! 1. **Staggered genesis start**: two weight-1 nodes, `WeightedMajority`.
//!    n(0) alone through several `primary_timeout` windows — the machinery
//!    fires exactly one fence (a solo node's first firing moves it to
//!    `ViewChange`, and a `ViewChange` node has nothing to suspect), the
//!    fence into the succession view whose primary is the absent n(1) —
//!    then n(1) starts; both tick in lockstep rounds. The pin: a leader is
//!    elected, a first value commits, both apply.
//! 2. **Post-genesis cold restart**: committed normal operations past the
//!    genesis (`accepted > INIT_SLOT`) plus one committed
//!    reconfiguration era, then a full-cluster staggered restart over the
//!    recorded disks (the `tests/reincarnation.rs` restart idiom): n(0)
//!    reopens and ticks alone through several timeout windows, then n(1)
//!    reopens. The shape is pinned under two tick schedules: a lockstep
//!    (metronome) schedule — both nodes tick in the same round — and a
//!    lawful phase-shifted schedule (n(0) on even rounds, n(1) on odd)
//!    that no real host's timers phase-lock into.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{INIT_SLOT, SystemOperation};
use vrr::ids::{Era, NodeId, OperationId, View, ViewId};
use vrr::progress::Status;
use vrr::replica::{PlanRefusal, ViewChangeKnobs};

/// The timeout knob: a `Normal` node suspects its primary after more than
/// three ticks of silence (S4).
const TIMEOUT: u64 = 3;

/// The staggered-start window multiplier: several `primary_timeout`
/// windows per phase, so a phase that fires nothing fires nothing over a
/// span no reading of the knob could mistake for one.
const WINDOWS: u64 = 3;

fn n(id: u32) -> NodeId {
    NodeId(id)
}

fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

/// A two-node weight-(1,1) cluster whose view-change machinery is live.
fn cluster() -> Harness {
    Harness::with_knobs(
        2,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    )
}

/// Bootstraps a live cluster: the genesis primary self-promotes on the
/// first tick and every backup adopts view (1, 0) from the promotion's
/// `Commit` announcement (§13.3).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    for id in [n(0), n(1)] {
        assert_eq!(status_of(h, id), Status::Normal, "{id:?} is Normal");
    }
    h.assert_safety();
}

fn snap(h: &Harness, id: NodeId) -> vrr::progress::ProgressSnapshot {
    h.snapshot(id).expect("the node is live")
}

fn status_of(h: &Harness, id: NodeId) -> Status {
    Status::from_word(snap(h, id).status).expect("the word is a status")
}

fn current_view(h: &Harness, id: NodeId) -> ViewId {
    let snapshot = snap(h, id);
    ViewId {
        era: Era(snapshot.era),
        view: View(snapshot.view),
    }
}

fn current_era(h: &Harness, id: NodeId) -> Era {
    h.era_table(id).expect("the node is live").current().era
}

fn primary_of(h: &Harness, observer: NodeId, view: ViewId) -> Option<NodeId> {
    h.era_table(observer)?
        .record(view.era)
        .and_then(|record| record.config.primary(view.view))
}

/// Drives the view change the fence machinery targets, asserting every
/// live node installs it (the `tests/reincarnation.rs` idiom).
fn drive_view_change(h: &mut Harness, live: &[NodeId]) -> ViewId {
    let current = current_view(h, live[0]);
    let target = ViewId {
        era: current_era(h, live[0]),
        view: View(current.view.0 + 1),
    };
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

/// The payloads a node has applied, in slot order.
fn applied_payloads(h: &Harness, id: NodeId) -> Vec<Vec<u8>> {
    h.applied(id)
        .iter()
        .map(|(_, payload)| payload.to_vec())
        .collect()
}

/// Commits one client value at `primary` and applies it everywhere: the
/// §11.1 boundary runs, so the node's apply record holds the slot.
fn commit_one(h: &mut Harness, primary: NodeId, lsb: u64, payload: &[u8]) {
    let outcome = h.propose(primary, op_id(lsb), payload);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the proposal publishes: {outcome:?}"
    );
    h.deliver_all();
    for id in [n(0), n(1)] {
        let applies = h.execute_apply_effects(id);
        assert_eq!(applies.len(), 1, "one Apply at {id:?}: {applies:?}");
    }
}

/// Commits one client value at `primary` without exercising the §11.1
/// boundary: the cold-restart scripts assert frontiers, and the pending
/// `Apply` effects are volatile host state that dies at the crash anyway.
fn commit_quiet(h: &mut Harness, primary: NodeId, lsb: u64, payload: &[u8]) {
    let outcome = h.propose(primary, op_id(lsb), payload);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the proposal publishes: {outcome:?}"
    );
    h.deliver_all();
}

/// Builds the post-genesis history the cold restart reopens over: two
/// committed normal operations (slots 3–4), one committed reconfiguration
/// era — the stop-the-world `Double` at slot 5, establishing era 2 with
/// weights (2, 2) — the ordinary view change into that era, and one more
/// committed operation inside it (slot 6). Every node ends `Normal` at
/// era-2 view 1 with `accepted == committed == 6`.
fn post_genesis_history(h: &mut Harness) -> ViewId {
    bootstrap(h);
    commit_quiet(h, n(0), 1, b"one");
    commit_quiet(h, n(0), 2, b"two");

    let outcome = h.reconfigure(n(0), SystemOperation::Double, None);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the DOUBLE publishes: {outcome:?}"
    );
    h.deliver_all();
    assert_eq!(current_era(h, n(0)), Era(2), "the reconfiguration folded");
    assert_eq!(
        current_era(h, n(1)),
        Era(2),
        "the commit announcement folded the era at the backup"
    );

    let folded = drive_view_change(h, &[n(0), n(1)]);
    assert_eq!(folded.era, Era(2));
    commit_quiet(h, n(1), 3, b"three");

    for id in [n(0), n(1)] {
        let snapshot = snap(h, id);
        assert!(snapshot.accepted > INIT_SLOT.0, "post-genesis history");
        assert_eq!(snapshot.committed, 6, "the whole history committed");
    }
    h.assert_safety();
    folded
}

/// The staggered genesis start: n(0) provisioned and ticked alone through
/// several `primary_timeout` windows, then n(1); both tick. The solo
/// phase's one fence firing targets the succession view whose primary is
/// the absent n(1), so the completion is n(1)'s to carry once it starts:
/// it adopts the queued promotion `Commit` (§4's bootstrap rule), joins
/// n(0)'s fence from the queued `StartViewChange`, wins the evidence
/// quorum as the target view's primary, and installs — the leader is
/// elected, a first value commits, both apply. The election completes in
/// the first lockstep round after n(1) starts; the serving round is
/// asserted before S4's idle-view churn (an idle primary's own timeout
/// deposes it) can move the view on.
#[test]
fn staggered_genesis_start_completes() {
    let mut h = cluster();
    // n(1) has not started: its epoch-0 crash records the pristine genesis
    // disk (the staggered-provisioning expression, see the module docs).
    h.crash(n(1));

    // The solo phase: n(0) promotes on the first tick, fires its fence at
    // the first timeout, and cannot fire again — a `ViewChange` node has
    // nothing to suspect. WINDOWS further windows elapse on the fenced,
    // silent node.
    for _ in 0..(WINDOWS * (TIMEOUT + 1)) {
        h.tick(n(0));
    }
    assert_eq!(status_of(&h, n(0)), Status::ViewChange, "the fence fired");
    assert_eq!(
        current_view(&h, n(0)),
        ViewId {
            era: Era(1),
            view: View(1)
        },
        "the fence targeted the succession view"
    );
    assert_eq!(primary_of(&h, n(0), current_view(&h, n(0))), Some(n(1)));
    assert_eq!(h.queued_len(), 2, "the promotion Commit and the fence vote");

    // n(1) starts; both tick in lockstep rounds until a leader serves.
    // The loop stops at the FIRST serving round: an idle cluster churns
    // views by design (S4 — an idle primary's own timeout deposes it
    // exactly like a backup's), so the leader assertion is about the
    // serving round, not about the view number settling.
    h.restart_with(n(1))
        .expect("n(1) reopens over the genesis disk");
    assert_eq!(status_of(&h, n(1)), Status::Recovering, "fenced at birth");
    let mut elected = None;
    for _ in 0..8 {
        h.tick_all();
        h.deliver_all();
        if status_of(&h, n(0)) == Status::Normal && status_of(&h, n(1)) == Status::Normal {
            elected = Some(current_view(&h, n(1)));
            break;
        }
    }
    let elected = elected.unwrap_or_else(|| panic!("no leader elected\n{}", h.trace_dump()));

    // A leader is elected: both nodes hold the succession view and its
    // primary serves.
    assert_eq!(
        elected,
        ViewId {
            era: Era(1),
            view: View(1)
        },
        "the succession view whose primary is the late starter"
    );
    for id in [n(0), n(1)] {
        assert_eq!(
            status_of(&h, id),
            Status::Normal,
            "{id:?} is Normal (last diagnostic {:?})",
            h.diagnostic(id)
        );
        assert_eq!(current_view(&h, id), elected, "{id:?} is in the view");
    }
    assert_eq!(primary_of(&h, n(1), elected), Some(n(1)));
    assert_eq!(status_of(&h, n(1)), Status::Normal, "the leader serves");

    // A first value commits; both apply it.
    commit_one(&mut h, n(1), 1, b"first");
    for id in [n(0), n(1)] {
        let snapshot = snap(&h, id);
        assert_eq!(snapshot.committed, 3, "the value committed at {id:?}");
        assert_eq!(snapshot.applied, 3, "the application ran at {id:?}");
    }
    assert_eq!(applied_payloads(&h, n(0)), vec![b"first"]);
    assert_eq!(applied_payloads(&h, n(1)), vec![b"first"]);
    h.assert_safety();
}

/// The post-genesis cold restart under a lockstep (metronome) schedule:
/// both nodes tick in the same round, a schedule no real host produces.
///
/// SPEC OF CORRECT BEHAVIOR: a full-cluster staggered restart reopens
/// every node fenced `Recovering` (§5's boot rule), and the cluster's own
/// machinery then elects a leader at/beyond the folded era and commits a
/// NEW value.
///
/// Today the restart wedges, and this test pins the wedge. The refusing
/// gates, in order: the tick's bootstrap self-promotion requires
/// `accepted == committed == INIT_SLOT` at the genesis view
/// (`src/replica/mod.rs`, `plan_tick`), which post-genesis history
/// excludes; tick-driven suspicion requires `Status::Normal`
/// (`plan_tick`'s second decision), which the boot fence excludes — so no
/// `StartViewChange` is ever broadcast; and the remaining
/// `Recovering → Normal` routes (the §4 bootstrap adoption and the §9.1
/// `StartView` install) require a sender that already holds a live
/// primary's authority. With every member reopened there is no such
/// sender: no first datagram ever exists, the cluster is silent whatever
/// the tick schedule, and the client surface is the named refusal — both
/// nodes answer `NotPrimary` naming the view's designated, still-fenced
/// primary.
#[test]
fn post_genesis_cold_restart_wedges_under_lockstep_ticking() {
    let mut h = cluster();
    let folded = post_genesis_history(&mut h);

    // The full-cluster crash; n(0) reopens first.
    h.crash(n(0));
    h.crash(n(1));
    h.restart_with(n(0))
        .expect("n(0) reopens over its recorded disk");
    assert_eq!(status_of(&h, n(0)), Status::Recovering, "fenced at reopen");
    assert_eq!(current_era(&h, n(0)), Era(2), "the folded era persisted");

    // n(0) alone through several timeout windows: no datagram is ever
    // emitted — a `Recovering` node can neither promote (post-genesis
    // history) nor suspect (suspicion requires `Normal`).
    for _ in 0..(WINDOWS * (TIMEOUT + 1)) {
        h.tick(n(0));
    }
    assert_eq!(status_of(&h, n(0)), Status::Recovering);
    assert_eq!(current_view(&h, n(0)), folded, "the view did not move");
    assert_eq!(h.queued_len(), 0, "the solo phase emitted nothing");

    // n(1) reopens; both tick in lockstep rounds.
    h.restart_with(n(1))
        .expect("n(1) reopens over its recorded disk");
    assert_eq!(status_of(&h, n(1)), Status::Recovering, "fenced at reopen");
    for _ in 0..(WINDOWS * (TIMEOUT + 1)) {
        h.tick_all();
        h.deliver_all();
    }

    // The pinned wedge: no leader, no message, no client service.
    for id in [n(0), n(1)] {
        assert_eq!(status_of(&h, id), Status::Recovering, "{id:?} stays fenced");
        assert_eq!(current_view(&h, id), folded, "{id:?} never moved");
        assert_eq!(current_era(&h, id), Era(2), "{id:?} holds the folded era");
    }
    assert_eq!(
        h.queued_len(),
        0,
        "the whole window emitted not one datagram"
    );
    for (id, lsb) in [(n(0), 4u64), (n(1), 5)] {
        assert_eq!(
            h.propose(id, op_id(lsb), b"new"),
            StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
                view: folded,
                primary: Some(n(1)),
            }),
            "the client surface is the named refusal at n({})",
            id.0
        );
    }
    h.assert_safety();
}

/// The same post-genesis cold restart under a lawful host tick schedule:
/// deterministic but phase-shifted — n(0) ticks on even rounds, n(1) on
/// odd — so no two timers ever fire in the same round. The wedge is
/// schedule-independent (see the lockstep pin's gates): the first
/// datagram never exists, so no phase relationship can matter, and this
/// control pins that the failure is not a host-timing artifact.
#[test]
fn post_genesis_cold_restart_wedges_under_a_phase_shifted_schedule() {
    let mut h = cluster();
    let folded = post_genesis_history(&mut h);

    h.crash(n(0));
    h.crash(n(1));
    h.restart_with(n(0))
        .expect("n(0) reopens over its recorded disk");
    for _ in 0..(WINDOWS * (TIMEOUT + 1)) {
        h.tick(n(0));
    }
    h.restart_with(n(1))
        .expect("n(1) reopens over its recorded disk");

    // The lawful schedule: one node's timer per round, phases disjoint.
    for round in 0..(2 * WINDOWS * (TIMEOUT + 1)) {
        if round % 2 == 0 {
            h.tick(n(0));
        } else {
            h.tick(n(1));
        }
        h.deliver_all();
    }

    for id in [n(0), n(1)] {
        assert_eq!(status_of(&h, id), Status::Recovering, "{id:?} stays fenced");
        assert_eq!(current_view(&h, id), folded, "{id:?} never moved");
        assert_eq!(current_era(&h, id), Era(2), "{id:?} holds the folded era");
    }
    assert_eq!(
        h.queued_len(),
        0,
        "the phase-shifted window emitted not one datagram"
    );
    for (id, lsb) in [(n(0), 4u64), (n(1), 5)] {
        assert_eq!(
            h.propose(id, op_id(lsb), b"new"),
            StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
                view: folded,
                primary: Some(n(1)),
            }),
            "the client surface is the named refusal at n({})",
            id.0
        );
    }
    h.assert_safety();
}
