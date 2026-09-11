//! Reincarnation: the Crash-Stop-Self-Evict corpus
//! (`docs/uvrr-reincarnation.md`).
//!
//! The classes:
//!
//! * **A** — the Voting Weights worked examples (the halve/double and
//!   ±1-unit rules the forced sequence is grounded in) as ordinary
//!   reconfigurations, at the configuration-fold level and through the
//!   live replica path.
//! * **B** — a backup crashed dirty: restart-as-new-identity, the
//!   `Reincarnation` announcement, the leader's forced sequence, every
//!   intermediate era quorum-safe, final membership correct (rejoin).
//! * **C** — the leader crashed mid-sequence: a stable leader first, then
//!   the new leader completes the sequence from the observed intermediate
//!   era.
//! * **D** — the four-superblock semantics: marker transitions
//!   (flushed/unflushed/dirty/bumped), any-of-four unflushed → dirty,
//!   all-flushed → clean-continue, higher-identity-wins, continuation
//!   commitment.
//! * **E** — membership discard: messages from an unknown or superseded
//!   identity ignored by the leader.
//! * **F** — the reincarnated weight-0 learner: acquires the era that
//!   admitted it, cannot influence.
//! * **G** — mutation negative controls: mutated variants are rejected.
//!
//! The old amnesia-era corpus (the classic §4.3 recovery tests) was
//! deleted with the classic recovery path; this corpus is its
//! replacement — the same restart/freshness surface, now the
//! Crash-Stop-Self-Evict protocol, which has no recovery protocol to test.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{ConfigError, Configuration, INIT_SLOT, SystemOperation, VOID_SLOT};
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
use vrr::quorum::{QuorumStrategy, Role, WeightedMajority};
use vrr::replica::{
    CopyState, Incarnation, Marker, PlanRefusal, RestartClass, RestartDecision, SuperblockCopies,
    forced_steps,
};
use vrr::wire::{Header, Pack, Tag, Unpack, UnpackError};

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

fn cluster5() -> Harness {
    Harness::with_knobs(
        5,
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
    // Any live Normal member's timeout drives the fence into the same
    // target view; the designated primary need not be the first to
    // suspect (and a weight-0 or unprovisioned primary never suspects).
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

/// Reincarnates a crashed backup through the forced sequence, stopping at
/// the named point. Shared by B, E and F: the choreography — dirty
/// restart, announcement, the forced batches each followed by the ordinary
/// view change into the era it established, the idempotent re-announce —
/// is one script, and the classes observe it at different depths. Each
/// forced step is a `Batch` and commits ONE era through the ordinary
/// reconfiguration pipeline (§5; rules §6).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stop {
    /// Stop after the announcement armed the machine and the first forced
    /// batch — `[Decrement(old), Join(new)]` — committed: the old identity
    /// sits at weight 0 and the new identity is a weight-0 learner.
    AfterFirstEra,
    /// Run the whole sequence: the new identity rejoins at weight 1, the
    /// old identity is evicted.
    Complete,
}

/// The shared choreography over the three-node genesis `[0, 1, 2]`:
/// `n(2)` is crashed while running, reincarnates as `n(3)`, announces, and
/// the leader drives the forced sequence. Returns the harness at the
/// requested stopping point, with the era each committed step
/// established.
fn reincarnate_backup(h: &mut Harness, stop: Stop) {
    bootstrap(h);
    // The node is RUNNING when the volatile state is lost: an accepted
    // operation, then the crash — the superblocks hold the running
    // sentinel, so the restart is dirty by construction (§2).
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    h.crash(n(2));

    // The dirty path: bump the identity, reopen, announce.
    h.restart_as(n(2), n(3)).expect("the bumped node reopens");
    let outcome = h.reincarnate(n(3), n(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    // One delivery pass carries the announcement (the backups drop it by
    // name) AND the leader's first forced `Prepare` — for the batch
    // `[Decrement(old), Join(new)]` as ONE establishing operation.
    h.deliver_all();
    assert_eq!(
        current_era(h, n(0)),
        Era(2),
        "Batch([Decrement, Join]) committed"
    );
    assert_eq!(current_order(h, n(0)), vec![n(0), n(1), n(3), n(2)]);
    assert_eq!(current_weights(h, n(0)), vec![1, 1, 0, 0]);
    if stop == Stop::AfterFirstEra {
        return;
    }

    // The next forced batch waits for the view change into era 2, then the
    // re-announce (§8) recomputes the remainder from the intermediate era —
    // the old identity at weight 0, the new identity joined at 0 — which is
    // exactly the second era of the weight-1 row: `[Increment(new),
    // Leave(old)]` (rules §6).
    let (target, _) = drive_view_change(h, &[n(0), n(1)]);
    assert_eq!(target.era, Era(2));
    // Leadership rotated; the bumped node re-announces to the stable
    // leader (§8), which proposes the next step directly.
    let outcome = h.reincarnate(n(3), n(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        current_era(h, n(0)),
        Era(3),
        "Batch([Increment, Leave]) committed"
    );
    assert_eq!(current_order(h, n(0)), vec![n(0), n(1), n(3)]);
    assert_eq!(current_weights(h, n(0)), vec![1, 1, 1]);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// A. The voting-weights worked examples as ordinary reconfigurations
// ---------------------------------------------------------------------------

/// Builds a configuration by folding `ops` onto genesis, asserting every
/// fold accepts (§8.7.2's preconditions, at the configuration level).
fn fold(ops: &[SystemOperation]) -> Configuration {
    let void = Configuration::void()
        .apply(&SystemOperation::Void, VOID_SLOT)
        .expect("Void at slot 1");
    let mut config = void
        .apply(
            &SystemOperation::Init {
                order: vec![n(0), n(1), n(2)],
            },
            INIT_SLOT,
        )
        .expect("Init at slot 2");
    let mut slot = INIT_SLOT;
    for op in ops {
        slot = slot.next().expect("the slot space is not exhausted");
        config = config.apply(op, slot).expect("the step folds");
    }
    config
}

fn weights(config: &Configuration) -> Vec<u64> {
    config
        .order()
        .iter()
        .map(|member| u64::from(member.weight.0))
        .collect()
}

/// Every consecutive pair of the sequence is quorum-safe: the closed gate
/// accepts the transition (R2 across the boundary, R1 and self-
/// intersection within the resulting era) and the resulting era admits a
/// quorum. This is the blog's rule — halve/double-all or ±1-unit — made
/// mechanical: each step of the worked examples obeys it, so consecutive
/// majority families intersect, which is what every intermediate era's
/// standalone safety rests on.
fn assert_era_safe(steps: &[Configuration]) {
    for pair in steps.windows(2) {
        vrr::quorum::validate_transition(&WeightedMajority, &pair[0], &pair[1])
            .expect("the step is era-safe");
        let quorum = |config: &Configuration| {
            config
                .order()
                .iter()
                .map(|member| member.node)
                .collect::<Vec<_>>()
        };
        assert!(
            WeightedMajority.is_quorum(Role::Commit, &pair[1], &quorum(&pair[1])),
            "the resulting era admits a quorum"
        );
    }
}

/// The unit-scale ±1 rules (§5) the §6 sequence is built from: the old
/// identity driven to 0 by the subtract-one rule, departed, the new
/// identity joined at 0 in the old succession position, then promoted —
/// each step a distinct configuration, every intermediate quorum-safe, the
/// final membership the rejoin. Under the era rule (rules §4, R14) the
/// middle two steps share one era; this corpus pins the per-step rules
/// that make each era safe.
#[test]
fn a_unit_scale_forced_sequence_is_the_blog_table_d() {
    let d0 = fold(&[]);
    assert_eq!(weights(&d0), vec![1, 1, 1]);
    let d1 = fold(&[SystemOperation::Decrement(n(2))]);
    assert_eq!(weights(&d1), vec![1, 1, 0]);
    let d2 = fold(&[
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
    ]);
    assert_eq!(weights(&d2), vec![1, 1]);
    let d3 = fold(&[
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
        SystemOperation::Join {
            node: n(3),
            position: 2,
        },
    ]);
    assert_eq!(weights(&d3), vec![1, 1, 0]);
    assert_eq!(
        d3.order().iter().map(|m| m.node).collect::<Vec<_>>(),
        vec![n(0), n(1), n(3)],
        "the new identity takes the old succession position"
    );
    let d4 = fold(&[
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
        SystemOperation::Join {
            node: n(3),
            position: 2,
        },
        SystemOperation::Increment(n(3)),
    ]);
    assert_eq!(weights(&d4), vec![1, 1, 1]);
    assert_era_safe(&[d0, d1, d2, d3, d4]);
}

/// The doubled-state corner (§5): the sequence runs at the doubled scale
/// and returns to unit weights through the extra unit step that makes the
/// halve integral. Every adjacent pair obeys a scaling or unit rule, and
/// every weight stays inside the {0, 1, 2} domain (rules §1, R1).
#[test]
fn a_doubled_scale_corner_is_the_blog_table_c() {
    let c0 = fold(&[SystemOperation::Double]);
    assert_eq!(weights(&c0), vec![2, 2, 2]);
    let c1 = fold(&[SystemOperation::Double, SystemOperation::Decrement(n(2))]);
    assert_eq!(weights(&c1), vec![2, 2, 1]);
    let c2 = fold(&[
        SystemOperation::Double,
        SystemOperation::Decrement(n(2)),
        SystemOperation::Decrement(n(2)),
    ]);
    assert_eq!(weights(&c2), vec![2, 2, 0]);
    let c3 = fold(&[
        SystemOperation::Double,
        SystemOperation::Decrement(n(2)),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
    ]);
    assert_eq!(weights(&c3), vec![2, 2]);
    let c4 = fold(&[
        SystemOperation::Double,
        SystemOperation::Decrement(n(2)),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
        SystemOperation::Join {
            node: n(3),
            position: 2,
        },
    ]);
    assert_eq!(weights(&c4), vec![2, 2, 0]);
    let c5 = fold(&[
        SystemOperation::Double,
        SystemOperation::Decrement(n(2)),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
        SystemOperation::Join {
            node: n(3),
            position: 2,
        },
        SystemOperation::Increment(n(3)),
    ]);
    assert_eq!(weights(&c5), vec![2, 2, 1]);
    let c6 = fold(&[
        SystemOperation::Double,
        SystemOperation::Decrement(n(2)),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
        SystemOperation::Join {
            node: n(3),
            position: 2,
        },
        SystemOperation::Increment(n(3)),
        SystemOperation::Increment(n(3)),
    ]);
    assert_eq!(weights(&c6), vec![2, 2, 2]);
    let c7 = fold(&[
        SystemOperation::Double,
        SystemOperation::Decrement(n(2)),
        SystemOperation::Decrement(n(2)),
        SystemOperation::Leave(n(2)),
        SystemOperation::Join {
            node: n(3),
            position: 2,
        },
        SystemOperation::Increment(n(3)),
        SystemOperation::Increment(n(3)),
        SystemOperation::Halve,
    ]);
    assert_eq!(weights(&c7), vec![1, 1, 1]);
    assert_era_safe(&[c0, c1, c2, c3, c4, c5, c6, c7]);
}

/// The blog's `{3, 4, 5, 6, 7}` observation — consecutive unit totals have
/// overlapping majorities, skipping one violates it — through the live
/// replica path: `Join` at weight 0 (total unchanged), then two unit
/// increments. Every intermediate era commits, and the quorum gate (Q1)
/// is what made each proposal legal.
#[test]
fn a_unit_ladder_commits_through_the_replica_path() {
    let mut h = cluster();
    bootstrap(&mut h);

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
    assert_eq!(current_weights(&h, n(0)), vec![1, 1, 1, 0]);

    let _ = drive_view_change(&mut h, &[n(0), n(1), n(2)]);
    let outcome = h.reconfigure(n(1), SystemOperation::Increment(n(3)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(current_weights(&h, n(1)), vec![1, 1, 1, 1]);
    assert_eq!(current_era(&h, n(1)), Era(3));
    h.assert_safety();
}

/// The subtract-one rule is safe; SKIPPING a unit is not (the blog's
/// sharpness note): a proposal that would jump the total by more than one
/// unit step is refused by the closed gate before the log moves.
#[test]
fn a_skipped_unit_is_refused_by_the_gate() {
    let mut h = cluster();
    bootstrap(&mut h);
    // Leave requires weight 0, so the skip is expressed as its
    // precondition violation instead: departing a member that still votes
    // is refused by name — the one-unit route is the ONLY departure route.
    let outcome = h.reconfigure(n(0), SystemOperation::Leave(n(1)), None);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Reconfigure(ConfigError::NonZeroWeight(n(1))))
    );
}

// ---------------------------------------------------------------------------
// B. Reincarnation, backup crashed
// ---------------------------------------------------------------------------

/// A backup crashed while running: dirty restart under a new identity, the
/// announcement, the forced sequence — every intermediate era quorum-safe
/// (each proposal passed the closed gate; the safety checker held
/// throughout), and the final membership is the rejoin: the old identity
/// gone, the new identity at weight 1 in the old succession position.
#[test]
fn b_backup_crashed_reincarnates_and_rejoins() {
    let mut h = cluster();
    reincarnate_backup(&mut h, Stop::Complete);
    assert_eq!(current_order(&h, n(0)), vec![n(0), n(1), n(3)]);
    assert_eq!(current_weights(&h, n(0)), vec![1, 1, 1]);
    // The superseded identity is gone from every configuration the
    // voting members folded (the rejoined node acquires the streamed
    // history through the §10 learner acquisition and stays fenced until
    // its own StartView installs — voting authority is a matter for the
    // committed `Increment`, which this corpus does not exercise).
    for id in [n(0), n(1)] {
        assert!(
            h.era_table(id)
                .unwrap()
                .current()
                .config
                .weight_of(n(2))
                .is_none(),
            "{id:?} no longer knows the old identity"
        );
    }
}

// ---------------------------------------------------------------------------
// C. Leader crashed mid-sequence
// ---------------------------------------------------------------------------

/// The leader crashes after the first forced batch commits. The cluster
/// reaches a stable leader FIRST (§8: the reincarnated node does not force
/// eviction until one exists), and the new leader completes the sequence
/// from the era the crash landed in — the announcement is idempotent over
/// the committed configuration, so the steps already committed are not
/// re-run. The weight-1 row of the §6 table is exactly two eras, so the
/// crash intermediate era leaves one remaining batch:
/// `[Increment(new), Leave(old)]`.
#[test]
fn c_leader_crash_mid_sequence_continues_from_the_intermediate_era() {
    let mut h = cluster5();
    bootstrap(&mut h);
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    h.crash(n(4));
    h.restart_as(n(4), n(5)).expect("the bumped node reopens");

    // The announcement reaches the leader, which proposes the first forced
    // batch; the delivery pass commits it as one era.
    h.reincarnate(n(5), n(4));
    h.deliver_all();
    assert_eq!(
        current_era(&h, n(1)),
        Era(2),
        "Batch([Decrement, Join]) committed"
    );
    assert_eq!(
        current_order(&h, n(1)),
        vec![n(0), n(1), n(2), n(3), n(5), n(4)]
    );
    assert_eq!(current_weights(&h, n(1)), vec![1, 1, 1, 1, 0, 0]);

    // The leader dies mid-sequence.
    h.crash(n(0));

    // A stable leader first: the fence machinery elects n(1).
    let (target, _) = drive_view_change(&mut h, &[n(1), n(2), n(3)]);
    assert_eq!(target.era, Era(2));
    assert_eq!(primary_of(&h, n(1), target), Some(n(1)));

    // The bumped node re-announces to the stable leader. The recomputed
    // sequence starts where the observed intermediate era left off: the old
    // identity is at weight 0 and the new identity is already a member at
    // weight 0, so the only remaining era is `[Increment(new), Leave(old)]`.
    h.reincarnate(n(5), n(4));
    h.deliver_all();
    assert_eq!(
        current_era(&h, n(1)),
        Era(3),
        "Batch([Increment(new), Leave(old)]) committed"
    );
    assert_eq!(current_order(&h, n(1)), vec![n(0), n(1), n(2), n(3), n(5)]);
    assert_eq!(current_weights(&h, n(1)), vec![1, 1, 1, 1, 1]);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// Leader kill: the LEADER's own reincarnation
// ---------------------------------------------------------------------------

/// Killing the VOTING PRIMARY survives the bumped identity's forced walk
/// (issue #13): the stable leader is elected first (§8), the re-announced
/// pair drives the two forced batches through it, every intermediate era
/// commits — and the spec of correct behavior: the reincarnated identity
/// catches up (§10, every admitted era included) and the succession into
/// ITS first designated view — view 3 of the rejoin era selects the
/// rejoined identity, the first voter of the old leader's succession
/// position — installs, and the cluster keeps committing under a leader
/// that can evaluate the live view, with no announcement storm.
/// Today the reincarnated identity folds only its admitting era (the
/// `plan_start_view` era gate drops an offer more than one era past its
/// boot table), so the succession hands the leader role to a
/// still-stale-table process: the change's evidence is dropped
/// `UnevaluableEra` at the designated primary (`src/replica/view_change.rs`,
/// `plan_start_view_change`/`plan_do_view_change`), the install never
/// runs, and the commit stream dies.
#[test]
fn killing_the_voting_primary_survives_the_reincarnated_identitys_forced_walk() {
    let mut h = cluster();
    bootstrap(&mut h);
    // The leader is RUNNING when the volatile state is lost: an accepted
    // operation, then the crash (§2's dirty-by-construction restart).
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    h.crash(n(0));

    // The identity bump; the bumped identity reopens over the disk.
    h.restart_as(n(0), n(3)).expect("the bumped node reopens");
    // The first announcement: no stable leader exists — the backups drop
    // it by name (§8), and the bumped node re-announces.
    let outcome = h.reincarnate(n(3), n(0));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();

    // A stable leader first (§8): the fence elects n(1).
    let (stable, _) = drive_view_change(&mut h, &[n(1), n(2)]);
    assert_eq!(
        stable.era,
        Era(1),
        "the stable leader is elected in the old era"
    );

    // The re-announce (§8) drives the crossing batch through the stable
    // leader: the old leader's weight reaches 0 and the bumped identity
    // joins at weight 0 in the old succession position.
    let outcome = h.reincarnate(n(3), n(0));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        current_era(&h, n(1)),
        Era(2),
        "Batch([Decrement, Join]) committed"
    );
    assert_eq!(current_order(&h, n(1)), vec![n(3), n(0), n(1), n(2)]);
    assert_eq!(current_weights(&h, n(1)), vec![0, 0, 1, 1]);

    // The ordinary view change into the crossing era.
    let (crossing, _) = drive_view_change(&mut h, &[n(1), n(2)]);
    assert_eq!(crossing.era, Era(2));

    // The re-announce (§8) drives the promotion batch: the reincarnated
    // identity rejoins at weight 1, the old identity is evicted.
    let outcome = h.reincarnate(n(3), n(0));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        current_era(&h, n(1)),
        Era(3),
        "Batch([Increment, Leave]) committed"
    );
    assert_eq!(current_order(&h, n(1)), vec![n(3), n(1), n(2)]);
    assert_eq!(current_weights(&h, n(1)), vec![1, 1, 1]);
    h.assert_safety();

    // SPEC OF CORRECT BEHAVIOR: the succession into the reincarnated
    // identity's first designated view installs — the rejoined leader can
    // evaluate the live view — and the cluster keeps committing under it.
    // View 3 of the rejoin era selects the rejoined identity (the first
    // voter, the old leader's succession position).
    let target = fence_target(&h, n(1));
    assert_eq!(
        target,
        ViewId {
            era: Era(3),
            view: View(3)
        },
        "the rejoin era's next view is the reincarnated identity's first designation"
    );
    assert_eq!(
        primary_of(&h, n(1), target),
        Some(n(3)),
        "the succession designates the reincarnated identity"
    );
    for _ in 0..=TIMEOUT {
        h.tick(n(1));
    }
    h.deliver_all();
    for id in [n(3), n(1), n(2)] {
        assert_eq!(
            status_of(&h, id),
            Status::Normal,
            "{id:?} installs the view the reincarnated identity leads (last diagnostic {:?})",
            h.diagnostic(id),
        );
        assert_eq!(current_view(&h, id), target, "{id:?} is in the target");
    }
    h.assert_safety();

    // The commit stream lives on under the rejoined leader.
    let committed_before = snap(&h, n(3)).committed;
    let outcome = h.propose(n(3), op_id(2), b"y");
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the cluster keeps committing under the rejoined leader: {outcome:?}\n{}",
        h.trace_dump()
    );
    h.deliver_all();
    assert!(
        snap(&h, n(3)).committed > committed_before,
        "the commit landed under the rejoined leader"
    );
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// D. Four-superblock semantics
// ---------------------------------------------------------------------------

fn copies(marks: [Marker; 4], identity: u64) -> SuperblockCopies {
    SuperblockCopies {
        copies: marks.map(|marker| CopyState {
            identity: Incarnation(identity),
            marker,
        }),
    }
}

/// All four read `flushed` → clean: mark `unflushed` (the running
/// sentinel) and continue under the SAME identity — the ordinary CR-free
/// path, no recovery protocol run.
#[test]
fn d_all_flushed_is_clean_and_continues() {
    let stored = copies([Marker::Flushed; 4], 7);
    assert_eq!(stored.classify(), RestartClass::Clean);
    let (decision, running) = stored.restart().expect("the identity space is not spent");
    assert_eq!(
        decision,
        RestartDecision::Continue {
            identity: Incarnation(7)
        }
    );
    assert!(
        running
            .copies
            .iter()
            .all(|copy| copy.identity == Incarnation(7) && copy.marker == Marker::Unflushed),
        "the clean path marks unflushed"
    );
}

/// ANY of the four reading `unflushed` → the node is dirty: bump, rewrite
/// all four as `(new, flushed)`.
#[test]
fn d_any_unflushed_is_dirty_and_bumps() {
    let mut marks = [Marker::Flushed; 4];
    marks[2] = Marker::Unflushed;
    let stored = copies(marks, 7);
    assert_eq!(stored.classify(), RestartClass::Dirty);
    let (decision, bumped) = stored.restart().expect("the identity space is not spent");
    assert_eq!(
        decision,
        RestartDecision::Bump {
            old: Incarnation(7),
            new: Incarnation(8)
        }
    );
    assert!(
        bumped
            .copies
            .iter()
            .all(|copy| copy.identity == Incarnation(8) && copy.marker == Marker::Flushed),
        "the bump rewrites all four as (new, flushed)"
    );
    // The running sentinel replaces the bump's flushed mark the moment
    // the node starts operating.
    let running = bumped.start_operating();
    assert!(
        running
            .copies
            .iter()
            .all(|copy| copy.marker == Marker::Unflushed)
    );
    // A clean shutdown rewrites the checkpoint mark.
    let clean = running.clean_shutdown();
    assert!(
        clean
            .copies
            .iter()
            .all(|copy| copy.marker == Marker::Flushed)
    );
}

/// Higher-identity-wins: a read adopts the highest identity any copy
/// carries, whatever the others say; the bump continues from THAT.
#[test]
fn d_higher_identity_wins() {
    let mut mixed = copies([Marker::Flushed; 4], 7);
    mixed.copies[1].identity = Incarnation(9);
    assert_eq!(mixed.read_identity(), Incarnation(9));
    marks_unflushed(&mut mixed, 0);
    let (decision, _) = mixed.restart().expect("the identity space is not spent");
    assert_eq!(
        decision,
        RestartDecision::Bump {
            old: Incarnation(9),
            new: Incarnation(10)
        },
        "the bump continues from the highest observed identity"
    );
}

/// Continuation commitment: once bumped, the node ALWAYS continues under
/// the bumped identity — the identity never regresses, and the next
/// restart re-enters the same protocol from it.
#[test]
fn d_continuation_commitment() {
    let mut marks = [Marker::Flushed; 4];
    marks[3] = Marker::Unflushed;
    let (decision, bumped) = copies(marks, 7).restart().expect("the bump succeeds");
    let RestartDecision::Bump { new, .. } = decision else {
        panic!("a dirty restart bumps");
    };
    // The wire phase runs; the node starts operating (running sentinel);
    // it then restarts AGAIN: dirty once more, and the identity only
    // moves forward.
    let running = bumped.start_operating();
    let (second, rebumped) = running.restart().expect("the second bump succeeds");
    assert_eq!(
        second,
        RestartDecision::Bump {
            old: new,
            new: Incarnation(new.0 + 1)
        }
    );
    // And a CLEAN shutdown then a restart continues the committed
    // identity — never a reversion to anything lower.
    let clean = rebumped.clean_shutdown();
    let (third, continued) = clean.restart().expect("the clean path continues");
    assert_eq!(
        third,
        RestartDecision::Continue {
            identity: Incarnation(new.0 + 1)
        }
    );
    assert!(
        continued
            .copies
            .iter()
            .all(|copy| copy.marker == Marker::Unflushed)
    );
}

/// The identity space is checked: a bump one short of `u64` exhaustion
/// refuses rather than wrapping a superseded identity into circulation.
#[test]
fn d_identity_exhaustion_refuses() {
    let mut marks = [Marker::Flushed; 4];
    marks[0] = Marker::Unflushed;
    let stored = copies(marks, u64::MAX);
    assert_eq!(
        stored.restart().err(),
        Some(Incarnation(u64::MAX)),
        "the bump refuses at exhaustion"
    );
}

fn marks_unflushed(copies: &mut SuperblockCopies, index: usize) {
    copies.copies[index].marker = Marker::Unflushed;
}

// ---------------------------------------------------------------------------
// E. Membership discard
// ---------------------------------------------------------------------------

/// The leader discards messages from identities outside the current
/// committed configuration — an unknown identity, and the superseded old
/// identity after its eviction — by name, and the discard never disturbs
/// a live commit.
#[test]
fn e_membership_discard() {
    let mut h = cluster();
    reincarnate_backup(&mut h, Stop::Complete);
    // The old identity was evicted (era 3): its messages are foreign.
    let before = h.queued_len();
    h.inject(
        n(2),
        n(1),
        Message {
            header: Header {
                tag: Tag::PrepareOk,
                view: current_view(&h, n(0)),
                slot: Slot(3),
            },
            body: Body::PrepareOk {},
        },
    );
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::UnknownSender { sender: n(2) }),
        "the superseded identity is discarded by name"
    );
    assert_eq!(h.queued_len(), before, "nothing was queued by the drop");
    // An arbitrary unknown identity fares the same, on any tag.
    h.inject(
        n(9),
        n(1),
        Message {
            header: Header {
                tag: Tag::Prepare,
                view: current_view(&h, n(0)),
                slot: Slot(4),
            },
            body: Body::Prepare {
                entry: vrr::journal::LogEntry {
                    slot: Slot(4),
                    era: current_view(&h, n(0)).era,
                    payload: vrr::journal::Payload::Operation {
                        id: op_id(9),
                        payload: Box::from(b"foreign".as_slice()),
                    },
                },
                committed: Slot(2),
            },
        },
    );
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::UnknownSender { sender: n(9) }),
        "a forged prepare from outside the configuration is discarded"
    );
    // The discard never disturbs a live commit.
    let outcome = h.propose(n(1), op_id(2), b"y");
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the live commit proceeds: {outcome:?}\n{}",
        h.trace_dump()
    );
    h.deliver_all();
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// F. Learner
// ---------------------------------------------------------------------------

/// The reincarnated weight-0 learner: it acquires the era that admitted it
/// through its own fetch (§10's learner acquisition), stays fenced — its
/// OWN vote is discarded before it is ever counted — and quorum outcomes
/// complete without it.
#[test]
fn f_learner_acquires_its_admitting_era_and_cannot_influence() {
    let mut h = cluster();
    reincarnate_backup(&mut h, Stop::AfterFirstEra);
    // The era the join committed awaits the ordinary view change (§8.7.4);
    // the fence view's recipients are the era that includes the learner,
    // so the StartView reaches it. The era is one past its boot table
    // (§10): the ruling retains the offer and fetches the missing range
    // under the boot view, the leader serves the fetch (the learner is a
    // member of the leader's current configuration), and the boot-fenced
    // acquisition folds the era that admitted it — the learner never
    // voted, adopted nothing, and stays fenced.
    let _ = drive_view_change(&mut h, &[n(0), n(1)]);
    assert_eq!(
        current_era(&h, n(3)),
        Era(2),
        "the learner folded the era that admitted it"
    );
    assert_eq!(
        status_of(&h, n(3)),
        Status::Restarting,
        "the acquisition runs at the boot fence, never voting"
    );
    // The stream arrives and is processed: the prepare from the leader of
    // a higher view is the staleness signal (§10) — the learner fences
    // into the advertised view, never installation evidence.
    let outcome = h.propose(n(1), op_id(2), b"y");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        status_of(&h, n(3)),
        Status::ViewChange,
        "the learner fenced into the leader's view"
    );
    // The retained offer re-runs on an ordinary tick (§13.1 step 5): the
    // fetched era makes it evaluable, the install adopts the view, and
    // the boot fence is discharged by the fetch the learner opened.
    h.tick(n(3));
    assert_eq!(
        status_of(&h, n(3)),
        Status::Normal,
        "the learner is caught up"
    );
    assert_eq!(current_era(&h, n(3)), Era(2));

    // Its vote is discarded, named, before counting — the weight is still
    // 0, so no quorum ever counts it.
    h.inject(
        n(3),
        n(1),
        Message {
            header: Header {
                tag: Tag::PrepareOk,
                view: current_view(&h, n(1)),
                slot: Slot(4),
            },
            body: Body::PrepareOk {},
        },
    );
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::LearnerSender { sender: n(3) }),
        "the learner's vote is discarded by name"
    );

    // The quorum outcome is unaffected: the commit lands on the voting
    // members alone.
    assert!(
        snap(&h, n(1)).committed >= 4,
        "the operation committed without the learner"
    );
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// G. Mutation negative controls
// ---------------------------------------------------------------------------

/// Mutated variants of the reincarnation surface are rejected:
/// byte-level mutations of the encoding (discriminant, truncation), and
/// protocol-level mutations (a forged sender, a degenerate pair, a
/// non-leader recipient).
#[test]
fn g_mutation_negative_controls() {
    // Byte level: the encoding is pinned, and every mutation is refused.
    let message = Message {
        header: Header {
            tag: Tag::Reincarnation,
            view: ViewId {
                era: Era(1),
                view: View(0),
            },
            slot: Slot::NONE,
        },
        body: Body::Reincarnation {
            old: n(2),
            new: n(3),
        },
    };
    let encoded = encode(&message);
    assert_eq!(encoded.len(), 20 + 9, "header + discriminant + two u32s");

    // Mutating the body discriminant to a reserved value is malformed.
    let mut mutated = encoded.clone();
    let discriminant_at = 20;
    mutated[discriminant_at] = 0;
    assert!(matches!(
        Message::unpack_from(&mutated),
        Err(UnpackError::Malformed(_))
    ));
    // A truncation is incomplete, never silently accepted.
    assert!(matches!(
        Message::unpack_from(&encoded[..encoded.len() - 1]),
        Err(UnpackError::Incomplete { .. })
    ));

    // Protocol level: the sender must BE the new identity it names.
    let mut h = cluster();
    bootstrap(&mut h);
    let forged = Message {
        header: Header {
            tag: Tag::Reincarnation,
            view: current_view(&h, n(0)),
            slot: Slot::NONE,
        },
        body: Body::Reincarnation {
            old: n(1),
            new: n(2),
        },
    };
    h.inject(n(1), n(0), forged);
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::ReincarnationRefused {
            sender: n(1),
            view: current_view(&h, n(0)),
        }),
        "a forged pair is dropped, never armed"
    );
    // A degenerate pair (old == new) is dropped too.
    let degenerate = Message {
        header: Header {
            tag: Tag::Reincarnation,
            view: current_view(&h, n(0)),
            slot: Slot::NONE,
        },
        body: Body::Reincarnation {
            old: n(2),
            new: n(2),
        },
    };
    h.inject(n(2), n(0), degenerate);
    assert!(matches!(
        h.diagnostic(n(0)),
        Some(Diagnostic::ReincarnationRefused { .. })
    ));
    // A non-leader recipient drops the announcement — the bumped node
    // re-announces until a stable leader exists (§8).
    let to_backup = Message {
        header: Header {
            tag: Tag::Reincarnation,
            view: current_view(&h, n(1)),
            slot: Slot::NONE,
        },
        body: Body::Reincarnation {
            old: n(2),
            new: n(3),
        },
    };
    h.inject(n(3), n(1), to_backup);
    assert!(matches!(
        h.diagnostic(n(1)),
        Some(Diagnostic::ReincarnationRefused { .. })
    ));
    h.assert_safety();
}

/// Round-trips the announcement through the normative codec: the pair
/// survives, byte for byte.
#[test]
fn g_reincarnation_round_trip() {
    let message = Message {
        header: Header {
            tag: Tag::Reincarnation,
            view: ViewId {
                era: Era(5),
                view: View(2),
            },
            slot: Slot::NONE,
        },
        body: Body::Reincarnation {
            old: n(2),
            new: n(3),
        },
    };
    let encoded = encode(&message);
    let decoded = Message::unpack_from(&encoded).expect("the encoding is canonical");
    assert_eq!(decoded, message);
}

fn encode(message: &Message) -> Vec<u8> {
    let mut buf = vec![0u8; message.packed_len()];
    let written = message.pack_into(&mut buf).expect("the buffer is exact");
    assert_eq!(written, buf.len());
    buf
}

// ---------------------------------------------------------------------------
// The forced-step generator against the live cluster
// ---------------------------------------------------------------------------

/// The generator's idempotence: at the leader-crash intermediate eras, the
/// recomputed eras are exactly the remaining ones — never a re-run of a
/// committed step (§8; rules §6). Every element is one era's establishing
/// `Batch`.
#[test]
fn forced_steps_recompute_exactly_the_remaining_suffix() {
    let void = Configuration::void()
        .apply(&SystemOperation::Void, VOID_SLOT)
        .expect("Void");
    let genesis = void
        .apply(
            &SystemOperation::Init {
                order: vec![n(0), n(1), n(2)],
            },
            INIT_SLOT,
        )
        .expect("Init");
    // Full sequence from genesis: the weight-1 row of the §6 table —
    // exactly two eras, the crossing batch then the promotion batch.
    let full = forced_steps(&genesis, n(2), n(3));
    assert_eq!(
        full,
        vec![
            SystemOperation::Batch(vec![
                SystemOperation::Decrement(n(2)),
                SystemOperation::Join {
                    node: n(3),
                    position: 2
                },
            ]),
            SystemOperation::Batch(vec![
                SystemOperation::Increment(n(3)),
                SystemOperation::Leave(n(2)),
            ]),
        ]
    );
    // The canonical first era committed: the old identity at weight 0, the
    // new identity joined at 0 — the recompute is the weight-1 row's second
    // era, and nothing else.
    let era1 = genesis
        .apply(&SystemOperation::Decrement(n(2)), Slot(3))
        .expect("the fold accepts the decrement")
        .apply(
            &SystemOperation::Join {
                node: n(3),
                position: 2,
            },
            Slot(4),
        )
        .expect("the fold accepts the join");
    assert_eq!(
        forced_steps(&era1, n(2), n(3)),
        vec![SystemOperation::Batch(vec![
            SystemOperation::Increment(n(3)),
            SystemOperation::Leave(n(2)),
        ])]
    );
    // The observed intermediate era where the decrement committed but the
    // join has not: the §6 weight-0 row — join and leave in one zero-mass
    // era, then the promotion.
    let d1 = genesis
        .apply(&SystemOperation::Decrement(n(2)), Slot(3))
        .expect("the fold accepts the decrement");
    assert_eq!(
        forced_steps(&d1, n(2), n(3)),
        vec![
            SystemOperation::Batch(vec![
                SystemOperation::Join {
                    node: n(3),
                    position: 2
                },
                SystemOperation::Leave(n(2)),
            ]),
            SystemOperation::Batch(vec![SystemOperation::Increment(n(3))]),
        ]
    );
    // The intermediate era where the old identity is already evicted: the
    // new identity joins in the old succession position... which is gone;
    // it appends. Join alone, then promote — the §6 evicted row.
    let d2 = d1
        .apply(&SystemOperation::Leave(n(2)), Slot(4))
        .expect("the fold accepts the departure");
    assert_eq!(
        forced_steps(&d2, n(2), n(3)),
        vec![
            SystemOperation::Batch(vec![SystemOperation::Join {
                node: n(3),
                position: 2
            },]),
            SystemOperation::Batch(vec![SystemOperation::Increment(n(3))]),
        ]
    );
    // The evicted row's first era committed: only the promotion remains.
    let d3 = d2
        .apply(
            &SystemOperation::Join {
                node: n(3),
                position: 2,
            },
            Slot(5),
        )
        .expect("the fold accepts the join");
    assert_eq!(
        forced_steps(&d3, n(2), n(3)),
        vec![SystemOperation::Batch(vec![SystemOperation::Increment(n(
            3
        ))])]
    );
    // The rejoin: nothing remains.
    let d4 = d3
        .apply(
            &SystemOperation::Batch(vec![SystemOperation::Increment(n(3))]),
            Slot(6),
        )
        .expect("the fold accepts the promotion");
    assert!(forced_steps(&d4, n(2), n(3)).is_empty());
    // A degenerate pair asks for nothing.
    assert!(forced_steps(&genesis, n(2), n(2)).is_empty());
    // A doubled-scale cluster decrements once per unit above one, then the
    // crossing batch: the §6 weight-`w >= 2` row at any scale.
    let doubled = genesis
        .apply(&SystemOperation::Double, Slot(3))
        .expect("the fold accepts the double");
    assert_eq!(
        forced_steps(&doubled, n(2), n(3)),
        vec![
            SystemOperation::Batch(vec![SystemOperation::Decrement(n(2))]),
            SystemOperation::Batch(vec![
                SystemOperation::Decrement(n(2)),
                SystemOperation::Join {
                    node: n(3),
                    position: 2
                },
            ]),
            SystemOperation::Batch(vec![
                SystemOperation::Increment(n(3)),
                SystemOperation::Leave(n(2)),
            ]),
        ]
    );
}

// Silence the unused-import lint for the era-table type used in helpers.
