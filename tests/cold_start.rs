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
//!    genesis (`accepted > INIT_SLOT`) plus one committed reconfiguration
//!    era, then a full-cluster staggered restart over the recorded disks
//!    (the `tests/reincarnation.rs` restart idiom): n(0) reopens and
//!    ticks alone through several timeout windows, then n(1) reopens. The
//!    restarted cluster is fenced by design — §5's boot fence never
//!    self-arms from persisted knowledge — so the shape pins both sides
//!    of the host obligation:
//!    - **with the lever**: once both nodes have reopened, the host arms
//!      the first fence through the §14.2 force-view input on the fenced
//!      node, and the ordinary fence/evidence/install pipeline completes
//!      the restart: a leader is elected at/beyond the folded era, a new
//!      value commits, both apply (the §11.1 catch-up walk included). A
//!      real deployment's cluster manager does exactly this.
//!    - **without the lever**: the same restart stays fenced — both
//!      nodes `Recovering`, not one datagram under either tick schedule
//!      (lockstep and phase-shifted), and the client surface the named
//!      refusal. The fence is the host's to arm.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{INIT_SLOT, SystemOperation};
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
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

/// The full-cluster staggered restart over the post-genesis disks: n(0)
/// reopens first and ticks alone through several timeout windows — fenced,
/// silent, the view unmoved — then n(1) reopens. Both nodes end fenced
/// `Recovering` at the folded view (§5's boot rule).
fn staggered_reopen(h: &mut Harness, folded: ViewId) {
    h.crash(n(0));
    h.crash(n(1));
    h.restart_with(n(0))
        .expect("n(0) reopens over its recorded disk");
    assert_eq!(status_of(h, n(0)), Status::Recovering, "fenced at reopen");
    assert_eq!(current_era(h, n(0)), Era(2), "the folded era persisted");

    // n(0) alone through several timeout windows: no datagram is ever
    // emitted — a `Recovering` node can neither promote (post-genesis
    // history) nor suspect (suspicion requires `Normal`).
    for _ in 0..(WINDOWS * (TIMEOUT + 1)) {
        h.tick(n(0));
    }
    assert_eq!(status_of(h, n(0)), Status::Recovering);
    assert_eq!(current_view(h, n(0)), folded, "the view did not move");
    assert_eq!(h.queued_len(), 0, "the solo phase emitted nothing");

    h.restart_with(n(1))
        .expect("n(1) reopens over its recorded disk");
    assert_eq!(status_of(h, n(1)), Status::Recovering, "fenced at reopen");
}

/// The boundary property the fenced restart holds: every member
/// `Recovering` at the folded view it reopened at, the folded era
/// persisted, and not one datagram in flight.
fn assert_fenced_boundary(h: &Harness, folded: ViewId) {
    for id in [n(0), n(1)] {
        assert_eq!(status_of(h, id), Status::Recovering, "{id:?} stays fenced");
        assert_eq!(current_view(h, id), folded, "{id:?} never moved");
        assert_eq!(
            current_era(h, id),
            folded.era,
            "{id:?} holds the folded era"
        );
    }
    assert_eq!(
        h.queued_len(),
        0,
        "the whole window emitted not one datagram"
    );
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

/// The post-genesis cold restart, resolved: the host arms the first fence.
///
/// The restarted cluster reopens fenced and its own machinery fires
/// nothing — the tick's bootstrap self-promotion is genesis-only, and
/// suspicion requires `Status::Normal` (`src/replica/mod.rs`,
/// `plan_tick`), so no first datagram ever exists. The host obligation
/// (§5, §14.2): arm the first fence through the force-view lever, after
/// which the ORDINARY fence/evidence/install pipeline does the rest,
/// exactly as in every view change — the forced node fences into the
/// target view, the other member joins from the `StartViewChange`, both
/// reach the fence quorum, the target view's primary wins the evidence
/// quorum, installs, and serves.
///
/// The pin: after the staggered reopen and the solo fenced windows, one
/// force-view input completes the restart — a leader is elected
/// at/beyond the folded era, a NEW value commits at it, and both apply
/// it, the restarted application walking the §11.1 catch-up first (the
/// reconfiguration slot is a system operation and walks itself, §11).
#[test]
fn post_genesis_cold_restart_completes_when_the_host_arms_the_first_fence() {
    let mut h = cluster();
    let folded = post_genesis_history(&mut h);
    staggered_reopen(&mut h, folded);

    // THE HOST OBLIGATION: arm the first fence through the §14.2 lever.
    // The target is the next view in the folded era; its primary is the
    // member the era's membership order names — n(0), the node that
    // waited alone.
    let target = ViewId {
        era: folded.era,
        view: View(folded.view.0 + 1),
    };
    assert_eq!(
        primary_of(&h, n(0), target),
        Some(n(0)),
        "the target's primary under the folded era's order"
    );
    assert!(
        matches!(h.force_view(n(0), target), StepOutcome::Published { .. }),
        "the forced fence is accepted"
    );
    assert_eq!(status_of(&h, n(0)), Status::ViewChange, "the fence armed");
    assert_eq!(current_view(&h, n(0)), target);

    // The ordinary pipeline runs from the armed fence; both nodes tick
    // in lockstep rounds until a leader serves. The loop stops at the
    // FIRST serving round: an idle cluster churns views by design (S4 —
    // an idle primary's own timeout deposes it exactly like a backup's).
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

    // A leader is elected at/beyond the folded era, and it serves.
    assert_eq!(elected, target, "the armed fence's target won");
    assert_eq!(
        elected.era, folded.era,
        "the election sits in the folded era"
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
    assert_eq!(
        primary_of(&h, n(1), elected),
        Some(n(0)),
        "the elected view's primary serves"
    );
    h.assert_safety();

    // The restarted application catches up first (§11.1): the restored
    // committed history is acknowledged slot by slot — the committed
    // reconfiguration slot is core-internal and walks itself (§11) — so
    // the applied frontier lands on the folded history before new
    // traffic.
    for id in [n(0), n(1)] {
        for slot in [Slot(3), Slot(4), Slot(6)] {
            assert!(
                matches!(h.report_applied(id, slot), StepOutcome::Published { .. }),
                "the restored history acknowledges at n({}) s{}",
                id.0,
                slot.0
            );
        }
        assert_eq!(
            snap(&h, id).applied,
            6,
            "the application caught up at {id:?}"
        );
    }
    h.assert_safety();

    // A NEW value commits at the elected leader, and both apply it.
    assert!(
        matches!(
            h.propose(n(0), op_id(4), b"new"),
            StepOutcome::Published { .. }
        ),
        "the leader serves"
    );
    h.deliver_all();
    for id in [n(0), n(1)] {
        assert_eq!(
            snap(&h, id).committed,
            7,
            "the new value committed at {id:?}"
        );
    }
    h.assert_safety();
    for id in [n(0), n(1)] {
        let applies = h.execute_apply_effects(id);
        assert_eq!(applies.len(), 1, "one Apply at {id:?}: {applies:?}");
        assert_eq!(applies[0].slot, Slot(7), "the new value's upcall at {id:?}");
        assert!(
            matches!(applies[0].outcome, StepOutcome::Published { .. }),
            "the acknowledgement taken at n({})",
            id.0
        );
        assert_eq!(
            applied_payloads(&h, id),
            vec![b"new"],
            "the application ran the new value at {id:?}"
        );
        assert_eq!(
            snap(&h, id).applied,
            7,
            "the applied frontier passed the new commit at {id:?}"
        );
    }
}

/// The same post-genesis cold restart with the lever withheld: the
/// boundary, stated as the design it is.
///
/// §5's boot fence never self-arms from persisted knowledge. A reopened
/// member can neither promote (the tick's bootstrap self-promotion is
/// genesis-only) nor suspect (suspicion requires `Status::Normal`), and
/// every `Recovering → Normal` route — the §4 bootstrap adoption, the
/// §9.1 `StartView` install — needs a sender that already holds a live
/// primary's authority. A full-cluster cold start has no such sender: no
/// first datagram ever exists, so the cluster is silent under any tick
/// schedule and the fence stays up until the host arms it (§14.2 — the
/// completion pin above). A real deployment's cluster manager does
/// exactly this.
///
/// The pin: both nodes stay `Recovering` at the folded view through
/// lockstep AND lawful phase-shifted windows, the queue holds not one
/// datagram, and the client surface is the named refusal — `NotPrimary`
/// naming the view's designated, still-fenced primary.
#[test]
fn post_genesis_cold_restart_stays_fenced_without_the_lever() {
    let mut h = cluster();
    let folded = post_genesis_history(&mut h);
    staggered_reopen(&mut h, folded);

    // A lockstep (metronome) schedule — both nodes tick in the same
    // round, a schedule no real host produces.
    for _ in 0..(WINDOWS * (TIMEOUT + 1)) {
        h.tick_all();
        h.deliver_all();
    }
    assert_fenced_boundary(&h, folded);

    // A lawful phase-shifted schedule — n(0) on even rounds, n(1) on
    // odd, so no two timers ever fire in the same round. The silence has
    // no phase relationship: no first datagram exists for any schedule
    // to matter to.
    for round in 0..(2 * WINDOWS * (TIMEOUT + 1)) {
        if round % 2 == 0 {
            h.tick(n(0));
        } else {
            h.tick(n(1));
        }
        h.deliver_all();
    }
    assert_fenced_boundary(&h, folded);

    // The client surface: the named refusal, naming the view's
    // designated, still-fenced primary.
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
