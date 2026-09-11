//! The cold-start corpus: the staggered genesis start, the post-genesis
//! cold restart through the marker machine, and the crash-shape Joining
//! pin.
//!
//! All shapes are deterministic and clockless: ticks only, no sleeps, no
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
//! 2. **Post-genesis cold restart through the marker machine**
//!    (§5.1 of `docs/vrr-durability-model.md`): committed normal
//!    operations past the genesis plus one committed reconfiguration era,
//!    then the HOST performs the controlled stop of each node —
//!    `begin_stop`, the drain, `finish_stop` — and the boot's quorum read
//!    (`SuperblockCopies::restart`) answers the one question it asks: the
//!    2-of-4 `Stopped` verdict is the clean stop, the identity continues,
//!    and the node boots as `Restarting` — a member with complete state
//!    and no amnesia. Staggered: n(0) restarts and ticks ALONE through
//!    several timeout windows — it ticks the FULL protocol, so it now
//!    suspects the silent primary and issues `StartViewChange`; no quorum
//!    evidence can arrive until n(1) restarts — then n(1) restarts, the
//!    mutually-heard fence completes, and the ordinary
//!    fence/evidence/install pipeline finishes the restart: a leader is
//!    elected at/beyond the folded era, a NEW value commits, both apply.
//!    There is NO `AdminForceView` anywhere: the tick does the work — the
//!    old cold-start "wedge" was a test's misunderstanding of uVRR, and
//!    under the marker machine it does not exist.
//! 3. **The crash-shape Joining pin**: a node restarted through the
//!    machine with a NON-stopped marker set — the markers its boot wrote,
//!    the crash shape — bumps and enters `Joining`. It is not a member: it
//!    neither votes nor view-changes (its tick drives only its own
//!    re-drive, and its solo windows emit not one datagram), and anything
//!    it emits is dropped by the §6 membership checks — a fabricated
//!    fence vote from the bumped identity is discarded by name and
//!    disturbs nothing.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{INIT_SLOT, SystemOperation};
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
use vrr::replica::{CopyState, Incarnation, Marker, RestartDecision, SuperblockCopies};
use vrr::wire::{Header, Tag};

/// The timeout knob: a suspecting node — `Normal` backup or `Restarting`
/// member (§5.1) — fires after more than three ticks of silence (S4).
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
        vrr::replica::ViewChangeKnobs {
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

/// One node's four superblock copies at the given incarnation, all four
/// carrying `marker` — the uniform 4x write shape every marker
/// transition leaves.
fn copies(marker: Marker, identity: u64) -> SuperblockCopies {
    SuperblockCopies {
        copies: [CopyState {
            identity: Incarnation(identity),
            marker,
        }; 4],
    }
}

/// The HOST's controlled stop of one node (§5.1): `begin_stop` (the node
/// has stopped sending — no disk flush sits on the protocol's hot path),
/// the drain (the host flushes WALs and grids, strictly between the two
/// marker writes — here it is work the host does outside the core, and it
/// is protocol-invisible), `finish_stop` (the drain's proof), then the
/// boot's quorum read: 2-of-4 `Stopped` is the clean stop, the identity
/// continues.
fn host_stop(identity: u64) -> (RestartDecision, SuperblockCopies) {
    let stopping = copies(Marker::Restarting, identity).begin_stop();
    assert!(
        stopping
            .copies
            .iter()
            .all(|copy| copy.marker == Marker::Stopping),
        "the stop command wrote `Stopping` 4x"
    );
    let stopped = stopping.finish_stop();
    stopped.restart().expect("the identity space is not spent")
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
    assert_eq!(status_of(&h, n(1)), Status::Restarting, "fenced at birth");
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

/// The post-genesis cold restart through the marker machine — the pin the
/// old corpus got wrong: there is no wedge, because a `Restarting` node
/// ticks the full protocol.
///
/// §5.1: the HOST performs the stop — `begin_stop`, the drain,
/// `finish_stop` — and the boot's quorum read (`SuperblockCopies::restart`)
/// proves the clean stop: 2-of-4 `Stopped` means the transition completed,
/// the drain completed, there is no amnesiac risk, and the identity
/// continues as `Restarting` — a member with complete state ticking the
/// full protocol. Staggered: n(0) restarts first and ticks ALONE through
/// several timeout windows; its first timeout fires the fence into the
/// succession view — the folded era's order names n(0) that view's
/// primary — and no quorum evidence can arrive while n(1) is down. n(1)
/// then restarts through the same machine; the queued and fresh
/// `StartViewChange` votes mutually complete the fence, the target
/// primary's evidence quorum wins, and the ordinary install seats the
/// leader. The pin: a leader is elected at/beyond the folded era, the
/// restarted application walks the §11.1 catch-up, a NEW value commits at
/// the elected leader, and both apply it — with NO `AdminForceView`
/// anywhere.
#[test]
fn post_genesis_cold_restart_completes_through_the_machine() {
    let mut h = cluster();
    let folded = post_genesis_history(&mut h);
    let target = ViewId {
        era: folded.era,
        view: View(folded.view.0 + 1),
    };
    assert_eq!(
        primary_of(&h, n(0), target),
        Some(n(0)),
        "the succession view's primary under the folded era's order"
    );

    // n(0) stops and boots through the machine: the host stop, then the
    // quorum read continuing the identity as `Restarting`.
    let (decision, written) = host_stop(7);
    assert_eq!(
        decision,
        RestartDecision::Continue {
            identity: Incarnation(7)
        },
        "the stopped quorum continues the identity"
    );
    assert!(
        written
            .copies
            .iter()
            .all(|copy| copy.marker == Marker::Restarting),
        "the boot wrote `Restarting` 4x"
    );
    h.crash(n(0));
    h.restart_with(n(0))
        .expect("n(0) boots over its recorded disk");
    assert_eq!(
        status_of(&h, n(0)),
        Status::Restarting,
        "a controlled shutdown restarts as Restarting"
    );
    assert_eq!(current_era(&h, n(0)), Era(2), "the folded era persisted");

    // The solo phase: n(0) ticks ALONE through several timeout windows —
    // and it ticks the FULL protocol now: its first timeout fires the
    // fence into the succession view. No quorum evidence can arrive until
    // n(1) restarts: the attempt sits armed with one queued fence vote.
    for _ in 0..(WINDOWS * (TIMEOUT + 1)) {
        h.tick(n(0));
    }
    assert_eq!(status_of(&h, n(0)), Status::ViewChange, "the fence fired");
    assert_eq!(
        current_view(&h, n(0)),
        target,
        "the target view was entered"
    );
    assert_eq!(h.queued_len(), 1, "the solo phase emitted the fence vote");

    // n(1) stops and boots through the machine the same way.
    let (decision, written) = host_stop(8);
    assert_eq!(
        decision,
        RestartDecision::Continue {
            identity: Incarnation(8)
        },
        "the stopped quorum continues the identity"
    );
    assert!(
        written
            .copies
            .iter()
            .all(|copy| copy.marker == Marker::Restarting),
        "the boot wrote `Restarting` 4x"
    );
    h.crash(n(1));
    h.restart_with(n(1))
        .expect("n(1) boots over its recorded disk");
    assert_eq!(status_of(&h, n(1)), Status::Restarting, "fenced at boot");
    assert_eq!(
        current_view(&h, n(1)),
        folded,
        "n(1) reopens at the folded view"
    );

    // Both tick in lockstep rounds until a leader serves. The loop stops
    // at the FIRST serving round: an idle cluster churns views by design
    // (S4 — an idle primary's own timeout deposes it exactly like a
    // backup's). No `AdminForceView` anywhere: the tick did the work.
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
    assert_eq!(elected, target, "the staggered restart's fence won");
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

/// The crash-shape Joining pin (§5.1's `Anything-else→Joining`).
///
/// A running node's markers hold the `Restarting` 4x its boot wrote, so a
/// crash leaves exactly the no-controlled-shutdown evidence: no 2-of-4
/// `Stopped`. The boot's quorum read (`SuperblockCopies::restart`) bumps
/// the identity and writes `Joining` 4x. The bumped node is NOT a member:
/// it neither votes nor view-changes — its tick drives only its own
/// re-drive, so its solo windows emit not one datagram — and anything it
/// emits is dropped by the §6 membership checks: a fabricated
/// `StartViewChange` from the bumped identity is discarded by name
/// (`UnknownSender`) and disturbs the live member not at all.
#[test]
fn crash_shape_bumps_and_joins_without_membership() {
    let mut h = cluster();
    bootstrap(&mut h);
    commit_quiet(&mut h, n(0), 1, b"one");

    // The crash shape: n(1) died while running — the markers on disk are
    // the `Restarting` 4x its boot wrote, no `Stopped` in sight.
    h.crash(n(1));
    let (decision, written) = copies(Marker::Restarting, 8)
        .restart()
        .expect("the identity space is not spent");
    assert_eq!(
        decision,
        RestartDecision::Bump {
            old: Incarnation(8),
            new: Incarnation(9),
        },
        "no stopped quorum bumps the identity"
    );
    assert!(
        written
            .copies
            .iter()
            .all(|copy| copy.identity == Incarnation(9) && copy.marker == Marker::Joining),
        "the resurrection wrote `Joining` 4x under the bumped identity"
    );

    // The harness restart idiom for the bump: the new identity reopens
    // over the old disk — the dirty path's identity change.
    h.restart_as(n(1), n(2))
        .expect("the bumped identity reopens over the recorded disk");
    let member_view = current_view(&h, n(0));

    // Its tick drives only its own re-drive: through several timeout
    // windows it emits not one datagram — it cannot promote (it is not the
    // primary of any view it can name), it cannot suspect (§5.1: not a
    // member — no vote, no view change — and the suspicion gate is a voting
    // member's act), and it has no open fetch to re-run.
    for _ in 0..(WINDOWS * (TIMEOUT + 1)) {
        h.tick(n(2));
    }
    assert_eq!(h.queued_len(), 0, "the solo windows emitted nothing");

    // And no message of its regains eligibility: a fabricated fence vote
    // from the bumped identity is discarded by the §6 membership check
    // before it is ever counted — n(0) never arms a fence and stays Normal
    // in its view.
    let before = h.queued_len();
    h.inject(
        n(2),
        n(0),
        Message {
            header: Header {
                tag: Tag::StartViewChange,
                view: ViewId {
                    era: current_era(&h, n(0)),
                    view: View(current_view(&h, n(0)).view.0 + 1),
                },
                slot: Slot::NONE,
            },
            body: Body::StartViewChange {},
        },
    );
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::UnknownSender { sender: n(2) }),
        "the bumped identity is discarded by name"
    );
    assert_eq!(h.queued_len(), before, "nothing was queued by the drop");
    assert_eq!(
        status_of(&h, n(0)),
        Status::Normal,
        "n(0) was not disturbed"
    );
    assert_eq!(
        current_view(&h, n(0)),
        member_view,
        "n(0)'s view did not move"
    );

    // Further solo windows emit nothing more; the cluster is undisturbed.
    for _ in 0..(WINDOWS * (TIMEOUT + 1)) {
        h.tick(n(2));
        h.deliver_all();
    }
    assert_eq!(h.queued_len(), 0, "the dead identity emitted nothing more");
    assert_eq!(status_of(&h, n(0)), Status::Normal, "n(0) still serves");
    assert_eq!(status_of(&h, n(2)), Status::Restarting, "still fenced");
    h.assert_safety();
}
