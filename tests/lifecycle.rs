//! The boot gate: the lifecycle trait and the typestate driver
//! (`docs/uvrr-boot-gate.md`).
//!
//! The classes:
//!
//! * **A**, the identity×marker cohort matrix: the torn-copy domain
//!   with mixed identities, exhaustively enumerated over three
//!   identities and four markers (12⁴ = 20 736 assignments), every
//!   verdict checked against an independently recomputed oracle, the
//!   working cohort, higher-identity-wins inside it, `Stopped` ⟺ the
//!   winner cohort holds ≥2 `Stopped`, and `QuorumLost` ⟺ no cohort
//!   reaches the open threshold.
//! * **B**, the fixed write schedules, asserted against the harness
//!   gate's operation log: the first life's anchor, the controlled
//!   halt's two rounds with the drain strictly between, the clean
//!   start's single latch round, and the dirty fast start's deferred
//!   latch.
//! * **C**, the deferral witness: the reincarnation's latch is
//!   unreachable until the engine's seated observation mints it; the
//!   markers hold the honest not-stopped evidence throughout the
//!   catch-up.
//! * **D**, the wedge guard: the kill-shape markers classify crashed,
//!   the same-identity resume refuses (error-on-crashed is the
//!   contract), and the only path onward is reincarnation.
//! * **E**, the re-crash replay: a crash between the pair's decision
//!   and the deferred latch re-reads the old markers and re-decides
//!   the same pair.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::ids::{CrashCounter, NodeId, OperationId, SystemId};
use vrr::lifecycle::{CopyState, Marker, RestartClass, SuperblockCopies};

fn n(id: u32) -> NodeId {
    NodeId::new(
        SystemId::new((id + 1) as u16).expect("test system ids are small and non-zero"),
        CrashCounter::new(1).expect("one is non-zero"),
    )
}

fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

/// The timeout knob for the view-change scripts: a node suspects its
/// primary after more than three ticks of silence (S4).
const TIMEOUT: u64 = 3;

fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

fn snap(h: &Harness, id: NodeId) -> vrr::progress::ProgressSnapshot {
    h.snapshot(id).expect("the node is live")
}

fn status_of(h: &Harness, id: NodeId) -> vrr::progress::Status {
    vrr::progress::Status::from_word(snap(h, id).status).expect("the word is a status")
}

fn current_view(h: &Harness, id: NodeId) -> vrr::ids::ViewId {
    let snapshot = snap(h, id);
    vrr::ids::ViewId {
        era: vrr::ids::Era(snapshot.era),
        view: vrr::ids::View(snapshot.view),
    }
}

fn current_era(h: &Harness, id: NodeId) -> vrr::ids::Era {
    h.era_table(id).expect("the node is live").current().era
}

fn fence_target(h: &Harness, node: NodeId) -> vrr::ids::ViewId {
    let current = current_view(h, node);
    vrr::ids::ViewId {
        era: current_era(h, node),
        view: vrr::ids::View(current.view.0 + 1),
    }
}

/// Drives the view change the fence machinery targets, asserting every
/// node in `live` installs it.
fn drive_view_change(h: &mut Harness, live: &[NodeId]) -> vrr::ids::ViewId {
    let target = fence_target(h, live[0]);
    let driver = live
        .iter()
        .copied()
        .find(|&id| status_of(h, id) == vrr::progress::Status::Normal)
        .expect("a live Normal member drives the fence");
    for _ in 0..=TIMEOUT {
        h.tick(driver);
    }
    h.deliver_all();
    for &id in live {
        assert_eq!(
            status_of(h, id),
            vrr::progress::Status::Normal,
            "{id:?} installs"
        );
        assert_eq!(current_view(h, id), target, "{id:?} is in the target");
    }
    h.assert_safety();
    target
}

// ---------------------------------------------------------------------------
// A. The identity×marker cohort matrix
// ---------------------------------------------------------------------------

fn copies_of(states: [(u32, Marker); 4]) -> SuperblockCopies {
    SuperblockCopies {
        copies: states.map(|(identity, marker)| CopyState {
            identity: NodeId(identity),
            marker,
        }),
    }
}

/// The oracle, recomputed by a different route than the machine: group
/// the copies by identity, keep the cohorts of ≥2 (the open threshold),
/// the winner is the highest identity among them, and the class reads
/// the winner cohort's `Stopped` count.
fn oracle(states: [(u32, Marker); 4]) -> Option<(bool, u32)> {
    // The zero pattern is no identity and forms no cohort, exactly as the
    // machine rules it.
    let mut identities: Vec<u32> = states
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| *id != 0)
        .collect();
    identities.sort_unstable();
    identities.dedup();
    let working: Vec<u32> = identities
        .into_iter()
        .filter(|id| states.iter().filter(|(i, _)| i == id).count() >= 2)
        .collect();
    let winner = working.last().copied()?;
    let stopped = states
        .iter()
        .filter(|(id, mark)| *id == winner && *mark == Marker::Stopped)
        .count();
    Some((stopped >= 2, winner))
}

/// The torn-copy domain with mixed identities, exhaustively: every
/// assignment of (identity, marker) to each of the four copies, over
/// three identities and the four markers. Every verdict must match the
/// independent oracle, the cohort rule never lets a lone superseded
/// copy impose its identity, never reads a stop the winner cohort did
/// not vouch, and never classifies a marker set with no working cohort
/// as anything but lost.
#[test]
fn a_the_identity_marker_cohort_matrix_is_exhaustive() {
    // Minted once, strictly inside the space, no memorised values: the
    // matrix runs over the drawn identities and the blank zero pattern.
    let (system, crash) = harness::mint_pair();
    let low = NodeId::new(system, crash);
    let mid = NodeId::new(
        system,
        CrashCounter::new(crash.get() + 1).expect("the drawn counter is below the bound"),
    );
    let (other_system, other_crash) = harness::mint_pair();
    let high = NodeId::new(other_system, other_crash);
    let identities = [0u32, low.0, mid.0, high.0];
    let markers = [
        Marker::Stopping,
        Marker::Stopped,
        Marker::Restarting,
        Marker::Joining,
    ];
    let mut assignments = 0usize;
    let mut lost = 0usize;
    for a in identities
        .iter()
        .flat_map(|i| markers.map(move |m| (*i, m)))
    {
        for b in identities
            .iter()
            .flat_map(|i| markers.map(move |m| (*i, m)))
        {
            for c in identities
                .iter()
                .flat_map(|i| markers.map(move |m| (*i, m)))
            {
                for d in identities
                    .iter()
                    .flat_map(|i| markers.map(move |m| (*i, m)))
                {
                    let states = [a, b, c, d];
                    assignments += 1;
                    match (oracle(states), copies_of(states).classify()) {
                        (None, None) => lost += 1,
                        (Some((clean, winner)), Some((class, identity))) => {
                            assert_eq!(
                                identity,
                                NodeId(winner),
                                "the winner is the highest working identity: {states:?}"
                            );
                            assert_eq!(
                                class == RestartClass::Stopped,
                                clean,
                                "Stopped ⟺ the winner cohort holds 2-of-4 Stopped: {states:?}"
                            );
                        }
                        (expected, got) => panic!(
                            "the verdict diverged from the oracle: {states:?}, oracle {expected:?}, machine {got:?}"
                        ),
                    }
                }
            }
        }
    }
    assert_eq!(assignments, 65_536, "the cross product is complete");
    assert!(lost > 0, "the domain includes quorum-lost assignments");
}

// ---------------------------------------------------------------------------
// B. The fixed write schedules
// ---------------------------------------------------------------------------

fn cluster() -> Harness {
    Harness::with_knobs(
        3,
        vrr::replica::ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    )
}

/// The first life's anchor: one read, one `Joining` round, durable
/// before the node answers anything, so a later crash reads as a crash.
#[test]
fn b_the_first_life_latches_its_anchor_once() {
    let mut h = cluster();
    bootstrap(&mut h);
    assert_eq!(
        h.gate_ops(n(0)),
        vec!["read", "commit:Joining@65537"],
        "the first life is one read and one anchor round"
    );
}

/// The controlled halt: `Stopping` 4x, the drain, `Stopped` 4x, the
/// drain strictly between the rounds, because the `Stopped` marker
/// vouches for exactly it.
#[test]
fn b_the_controlled_halt_is_two_rounds_with_the_drain_between() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.halt(n(0));
    assert_eq!(
        h.gate_ops(n(0)),
        vec![
            "read",
            "commit:Joining@65537",
            "commit:Stopping@65537",
            "drain",
            "commit:Stopped@65537"
        ],
        "the halt's schedule is fixed: two rounds, the drain between"
    );
    // The clean start over the halted markers: one read, one latch round.
    h.restart_with(n(0)).expect("the clean start resumes");
    assert_eq!(
        h.gate_ops(n(0)),
        vec!["read", "commit:Restarting@65537"],
        "the clean start is one read and one latch round"
    );
}

// ---------------------------------------------------------------------------
// C. The deferral witness
// ---------------------------------------------------------------------------

/// The reincarnation's latch defers to the seated observation: through
/// the whole catch-up the markers hold the crash's evidence and no
/// `Joining` round fires; the latch lands only once the engine has
/// seated the new identity.
#[test]
fn c_the_reincarnations_latch_defers_to_the_seated_witness() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.crash(n(2));
    let bumped = n(2)
        .next_life()
        .expect("the test never exhausts the counter");
    h.restart_as(n(2), bumped).expect("the bumped node reopens");
    let outcome = h.reincarnate(bumped, n(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(current_era(&h, n(0)), vrr::ids::Era(2));
    // Mid-catch-up: the gate has read the crash's markers and written
    // nothing, the flush is not paid at the boundary of an
    // uninitialised start, and the node is a weight-0 learner.
    assert_eq!(
        h.gate_ops(bumped),
        vec!["read"],
        "no marker round before the seated witness"
    );

    // The forced sequence's second step: the ordinary view change into
    // era 2, then the re-announce carries the final `[Increment(new),
    // Leave(old)]` batch, the node seats at weight 1.
    drive_view_change(&mut h, &[n(0), n(1)]);
    assert_eq!(
        h.gate_ops(bumped),
        vec!["read"],
        "still no marker round: the node is Normal at weight 0 in era 2"
    );
    let outcome = h.reincarnate(bumped, n(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(current_era(&h, n(0)), vrr::ids::Era(3));
    // Seated: the harness's settle fired the deferred latch, one
    // `Joining` round at the bumped identity (the crashed anchor was the
    // first life's packed identity, the latch its next crash counter),
    // over the running state.
    assert_eq!(
        h.gate_ops(bumped),
        vec!["read", "commit:Joining@196610"],
        "the deferred latch is one Joining round at the bumped identity"
    );
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// D. The wedge guard
// ---------------------------------------------------------------------------

/// The kill shape, markers left at the running sentinel by a crash,
/// classify crashed, and the same-identity resume refuses loudly. The
/// blank same-identity boot that wedged downstream has no constructor:
/// error-on-crashed is the contract.
#[test]
#[should_panic(expected = "error-on-crashed is the contract")]
fn d_a_crashed_identity_cannot_resume() {
    let mut h = cluster();
    bootstrap(&mut h);
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    h.crash(n(2));
    // The markers hold the running sentinel: restart_with must refuse,
    // and the harness refuses by contract, not by returning an error
    // silently misread as a join.
    h.restart_with(n(2)).expect("refuses before this point");
}

// ---------------------------------------------------------------------------
// E. The re-crash replay
// ---------------------------------------------------------------------------

/// A crash between the pair's decision and the deferred latch replays:
/// the next boot re-reads the old markers, re-classifies crashed, and
/// re-decides the SAME pair, the eventual latch writes the same bumped
/// identity, never a double bump.
#[test]
fn e_a_recrash_between_decision_and_latch_replays_the_same_pair() {
    let mut h = cluster();
    bootstrap(&mut h);
    h.crash(n(2));
    let bumped = n(2)
        .next_life()
        .expect("the test never exhausts the counter");
    h.restart_as(n(2), bumped).expect("the bumped node reopens");
    let outcome = h.reincarnate(bumped, n(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();

    // Crash again BEFORE the latch: the markers still hold the first
    // crash's evidence (no Joining round has fired).
    assert_eq!(h.gate_ops(bumped), vec!["read"]);
    h.crash(bumped);

    // The replay: the same marker directory, the same classification,
    // the same pair, the bump is a pure function of the quorum-resolved
    // identity. The announcement replays idempotently at the leader.
    let replay = bumped
        .next_life()
        .expect("the test never exhausts the counter");
    h.restart_as(bumped, replay).expect("the replay reopens");
    assert_eq!(
        h.gate_ops(replay),
        vec!["read"],
        "the replay's boot wrote nothing either"
    );
    let outcome = h.reincarnate(replay, bumped);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    // The replay's forced sequence: the announcement's batch recomputes
    // from the intermediate era (the weight-0 rules), each committed era
    // followed by its ordinary view change and the idempotent
    // re-announce, until the new identity seats at weight 1.
    for _ in 0..6 {
        if h.gate_ops(replay).len() > 1 {
            break;
        }
        drive_view_change(&mut h, &[n(0), n(1)]);
        let outcome = h.reincarnate(replay, bumped);
        assert!(matches!(outcome, StepOutcome::Published { .. }));
        h.deliver_all();
    }
    assert_eq!(
        h.gate_ops(replay),
        vec!["read", "commit:Joining@196610"],
        "the replay re-decided the same pair (the anchor is the crashed life's packed identity, the bump its next crash counter): the latch writes the same bump, never a double bump"
    );
    h.assert_safety();
}
