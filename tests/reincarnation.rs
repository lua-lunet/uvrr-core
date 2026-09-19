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
//! * **D** — the marker transition machine (§5.1): the EXHAUSTIVE closed
//!   4-copy domain (4⁴ = 256 assignments) — 2-of-4 `Stopped` ⟺ the clean
//!   stop, every other assignment bumps, the identity resolved INSIDE the
//!   working quorum (higher-identity-wins, never across all copies),
//!   all-`Joining` reincarnates again; plus the stop path
//!   (`begin_stop`/`finish_stop`, the marker order as the drain's proof)
//!   and continuation commitment.
//! * **E** — membership discard: messages from an unknown or superseded
//!   identity ignored by the leader.
//! * **F** — the reincarnated weight-0 standby: the memo stream keeps it
//!   current without a fetch; the §10 self-fetch route pinned separately;
//!   never votes, cannot influence.
//! * **G** — mutation negative controls: mutated variants are rejected.
//!
//! The old amnesia-era corpus (the classic §4.3 recovery tests) was
//! deleted with the classic recovery path; this corpus is its
//! replacement — the same restart/freshness surface, now the
//! Crash-Stop-Self-Evict protocol, which has no recovery protocol to test.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{ConfigError, Configuration, INIT_SLOT, SystemOperation, VOID_SLOT};
use vrr::effects::Effect;
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use vrr::lifecycle::{
    CopyState, Incarnation, Marker, RestartClass, RestartDecision, RestartRefusal, SuperblockCopies,
};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
use vrr::quorum::{QuorumStrategy, Role, WeightedMajority};
use vrr::replica::{PlanRefusal, forced_steps};
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
    // history through the memo stream's ack, which lands the missed
    // range and the establishing prepare before the promotion era
    // commits — voting authority is a matter for the committed
    // `Increment`, which this corpus does not exercise).
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

/// The announcement carries the past-life frontiers (§4): every member of
/// the configuration the bumped node can still name receives it, and the
/// carried `committed`/`prepared` are what the node's durable journal
/// recorded when it crashed.
#[test]
fn b_announcement_carries_past_life_frontiers() {
    let mut h = cluster();
    bootstrap(&mut h);
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    h.crash(n(2));
    h.restart_as(n(2), n(3)).expect("the bumped node reopens");
    let outcome = h.reincarnate(n(3), n(2));
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the announcement publishes: {outcome:?}");
    };
    let mut addressed = Vec::new();
    for effect in effects {
        let Effect::Send { to, message, .. } = effect else {
            panic!("announcement effects are sends: {effect:?}");
        };
        assert_eq!(message.header.tag, Tag::Reincarnation);
        let Body::Reincarnation {
            old,
            new,
            committed,
            prepared,
        } = &message.body
        else {
            unreachable!("the tag names the body");
        };
        assert_eq!(*old, n(2), "the pair the node bumped from");
        assert_eq!(*new, n(3), "the pair the node bumped to");
        assert_eq!(*committed, Slot(3), "the past-life committed frontier");
        assert_eq!(*prepared, Slot(3), "the past-life prepared frontier");
        addressed.push(to);
    }
    addressed.sort();
    assert_eq!(
        addressed,
        vec![n(0), n(1), n(2)],
        "every member of the configuration the node can name is addressed"
    );
    h.assert_safety();
}

/// The leader crashes mid-sequence (§8): the armed machine and the memo
/// stream die with its volatile state, the standby's answers keep being
/// discarded, and the cluster must reach a stable leader before anything
/// resumes. The standby re-announces to that stable leader, whose first
/// transition is the full ack: the missed-range push and the armed
/// machine's next forced step, recomputed from the configuration the
/// observed eras committed, in ONE transition. The memo'd copy lands and
/// the standby syncs without a fetch.
#[test]
fn b_leader_crash_mid_sequence_memo_dies_and_resumes() {
    let mut h = Harness::with_knobs(
        5,
        vrr::replica::ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    );
    bootstrap(&mut h);
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    h.crash(n(4));
    h.restart_as(n(4), n(5)).expect("the bumped node reopens");

    // The leader acks at once and proposes the first forced step.
    let outcome = h.reincarnate(n(5), n(4));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_to(n(0));

    // The leader dies before the step lands anywhere: the machine and
    // the memo die with it. Every datagram the ack had put in flight is
    // dropped, the standby's included.
    h.crash(n(0));
    for id in [n(1), n(2), n(3), n(5)] {
        h.drop_queued(id);
    }
    h.assert_safety();
    assert_eq!(current_era(&h, n(1)), Era(1));
    assert_eq!(
        snap(&h, n(5)).committed,
        3,
        "the standby stayed at its past life"
    );

    // The cluster reaches a stable leader (§8): the surviving voters
    // n(1), n(2), n(3) form every quorum the era needs.
    for _ in 0..=TIMEOUT {
        h.tick(n(1));
    }
    h.deliver_all();
    for id in [n(1), n(2), n(3)] {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} installs");
        assert_eq!(
            current_view(&h, id).view,
            View(1),
            "{id:?} is in the target"
        );
    }
    h.assert_safety();

    // The memo is dead with the machine: the new leader's own proposal
    // streams to its configuration's members only, and the standby is
    // not addressed.
    let outcome = h.propose(n(1), op_id(2), b"z");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert!(
        !h.step_trace()
            .iter()
            .any(|line| line.contains("n1->n5") && line.contains("Prepare")),
        "the memo died with the machine"
    );
    h.assert_safety();

    // The standby re-announces (§8); the stable leader acks and resumes:
    // the missed range (the commit it missed) and the armed machine's
    // next forced step, recomputed from the configuration the observed
    // eras committed, in ONE transition.
    let outcome = h.reincarnate(n(5), n(4));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    let outcome = h
        .deliver_to(n(1))
        .expect("the re-announce reaches the stable leader");
    let StepOutcome::Published { effects, .. } = outcome.outcome else {
        panic!("the ack publishes: {:?}", outcome.outcome);
    };
    let Effect::Send { to, message, .. } = &effects[0] else {
        panic!("the ack's effects are sends: {:?}", effects[0]);
    };
    assert_eq!(*to, n(5), "the push is addressed to the standby");
    assert_eq!(message.header.tag, Tag::NewState);
    let Body::NewState {
        entries,
        through,
        committed: _,
        more,
    } = &message.body
    else {
        unreachable!("the tag names the body");
    };
    assert_eq!(entries.len(), 1, "exactly the missed slot");
    assert_eq!(
        entries[0].slot,
        Slot(4),
        "the range starts past the standby's prepared frontier"
    );
    assert_eq!(*through, Slot(4));
    assert!(!(*more));
    // The recompute's first step is the forced schedule's first batch.
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::Send { message, .. }
                if message.header.tag == Tag::Prepare
                    && matches!(
                        &message.body,
                        Body::Prepare { entry, .. }
                            if matches!(
                                &entry.payload,
                                vrr::journal::Payload::System(
                                    SystemOperation::Batch(ops)
                                ) if *ops == vec![SystemOperation::Double]
                            )
                    )
        )),
        "the recompute's first step is proposed in the same transition"
    );

    // The pushed range lands first (§7's order); the standby advances
    // past its past life without a fetch of its own.
    h.deliver_all();
    assert!(
        snap(&h, n(5)).committed >= 4,
        "the standby advanced past its past life through the ack's push"
    );
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// D. The marker transition machine
// ---------------------------------------------------------------------------

fn copies(marks: [Marker; 4], identity: u64) -> SuperblockCopies {
    SuperblockCopies {
        copies: marks.map(|marker| CopyState {
            identity: Incarnation(identity),
            marker,
        }),
    }
}

/// The closed 4-copy domain, EXHAUSTIVE: every assignment of the four
/// marker states to four copies — 4⁴ = 256, all cheap. For each
/// assignment:
///
/// * the verdict is `Stopped` ⟺ ≥2 copies hold `Stopped` — 2-of-4 of the
///   state right of the Stopping→Stopped transition, the clean stop;
/// * the quorum-resolved identity is the single written identity;
/// * a stopped quorum continues: `Continue { identity }` and
///   `(identity, Restarting)` 4x — a member with complete state;
/// * EVERY non-clean assignment bumps: `Bump { old, new }` and
///   `(new, Joining)` 4x — a crash, a torn marker set, and death
///   mid-join all reincarnate.
///
/// The mid-join case falls out of the table and is asserted explicitly:
/// all-`Joining` reads no stopped quorum, bumps, writes `Joining` again,
/// and the rewritten set reincarnates AGAIN — the commitment never wedges.
#[test]
fn d_marker_domain_exhaustive() {
    let states = [
        Marker::Stopping,
        Marker::Stopped,
        Marker::Restarting,
        Marker::Joining,
    ];
    for bits in 0..256usize {
        let marks = [
            states[bits & 3],
            states[(bits >> 2) & 3],
            states[(bits >> 4) & 3],
            states[(bits >> 6) & 3],
        ];
        let stored = copies(marks, 7);
        let clean = marks
            .iter()
            .filter(|mark| **mark == Marker::Stopped)
            .count()
            >= 2;
        let (class, identity) = stored
            .classify()
            .expect("the single written identity reaches the open threshold");
        assert_eq!(identity, Incarnation(7), "marks {marks:?}");
        assert_eq!(
            class == RestartClass::Stopped,
            clean,
            "the verdict must follow the 2-of-4 Stopped rule: {marks:?}"
        );
        let (decision, written) = stored.restart().expect("the identity space is not spent");
        if clean {
            assert_eq!(
                decision,
                RestartDecision::Continue {
                    identity: Incarnation(7)
                },
                "marks {marks:?}"
            );
            assert!(
                written.copies.iter().all(
                    |copy| copy.identity == Incarnation(7) && copy.marker == Marker::Restarting
                ),
                "the continue writes Restarting 4x: {marks:?}"
            );
        } else {
            assert_eq!(
                decision,
                RestartDecision::Bump {
                    old: Incarnation(7),
                    new: Incarnation(8)
                },
                "marks {marks:?}"
            );
            assert!(
                written
                    .copies
                    .iter()
                    .all(|copy| copy.identity == Incarnation(8) && copy.marker == Marker::Joining),
                "the bump writes Joining 4x: {marks:?}"
            );
        }
    }

    // Death mid-join: all-`Joining` reincarnates again — and again.
    let mid_join = copies([Marker::Joining; 4], 7);
    let (decision, rejoined) = mid_join.restart().expect("the bump succeeds");
    assert_eq!(
        decision,
        RestartDecision::Bump {
            old: Incarnation(7),
            new: Incarnation(8)
        }
    );
    let (again, reincarnated) = rejoined.restart().expect("the second bump succeeds");
    assert_eq!(
        again,
        RestartDecision::Bump {
            old: Incarnation(8),
            new: Incarnation(9)
        }
    );
    assert!(
        reincarnated
            .copies
            .iter()
            .all(|copy| copy.identity == Incarnation(9) && copy.marker == Marker::Joining)
    );
}

/// The stop path (§5.1): the stop command writes `Stopping` 4x; the
/// drain — flush WALs and grids — is the HOST's and sits strictly
/// between the two marker writes; after it `finish_stop` writes
/// `Stopped` 4x. The marker order is the drain's proof, so the completed
/// stop boots as a member with complete state: `Continue` and
/// `Restarting` 4x under the same identity.
#[test]
fn d_stop_path_marks_the_drain() {
    let running = copies([Marker::Restarting; 4], 7);
    let stopping = running.begin_stop();
    assert!(
        stopping
            .copies
            .iter()
            .all(|copy| copy.marker == Marker::Stopping)
    );
    // The host drains here: flush the WALs and the grids. No marker
    // write in between — the marker order is the drain's proof.
    let stopped = stopping.finish_stop();
    assert!(
        stopped
            .copies
            .iter()
            .all(|copy| copy.marker == Marker::Stopped)
    );
    assert_eq!(
        stopped.classify(),
        Some((RestartClass::Stopped, Incarnation(7)))
    );
    let (decision, restarted) = stopped.restart().expect("the identity space is not spent");
    assert_eq!(
        decision,
        RestartDecision::Continue {
            identity: Incarnation(7)
        }
    );
    assert!(
        restarted
            .copies
            .iter()
            .all(|copy| copy.marker == Marker::Restarting)
    );
}

/// The identity is resolved INSIDE the working quorum
/// (higher-identity-wins), never highest-observed-across-all: the
/// highest identity whose cohort reaches the open threshold — 2 of 4 —
/// wins; a lone copy at a higher identity cannot impose it.
#[test]
fn d_identity_resolved_inside_the_working_quorum() {
    let mut higher_cohort = copies([Marker::Joining; 4], 7);
    higher_cohort.copies[0].identity = Incarnation(9);
    higher_cohort.copies[1].identity = Incarnation(9);
    assert_eq!(
        higher_cohort.classify(),
        Some((RestartClass::NotStopped, Incarnation(9)))
    );
    let (decision, _) = higher_cohort
        .restart()
        .expect("the identity space is not spent");
    assert_eq!(
        decision,
        RestartDecision::Bump {
            old: Incarnation(9),
            new: Incarnation(10)
        },
        "the highest identity reaching the open threshold wins"
    );

    let mut lone_higher = copies([Marker::Joining; 4], 7);
    lone_higher.copies[2].identity = Incarnation(12);
    assert_eq!(
        lone_higher.classify(),
        Some((RestartClass::NotStopped, Incarnation(7)))
    );
    let (decision, _) = lone_higher
        .restart()
        .expect("the identity space is not spent");
    assert_eq!(
        decision,
        RestartDecision::Bump {
            old: Incarnation(7),
            new: Incarnation(8)
        },
        "a single stale-or-superseded copy cannot impose its identity"
    );
}

/// Continuation commitment: once bumped, the node ALWAYS continues under
/// the bumped identity — the identity never regresses, and the next
/// restart re-enters the same protocol from it.
#[test]
fn d_continuation_commitment() {
    let mut marks = [Marker::Restarting; 4];
    marks[3] = Marker::Stopping;
    let (decision, bumped) = copies(marks, 7).restart().expect("the bump succeeds");
    let RestartDecision::Bump { new, .. } = decision else {
        panic!("no stopped quorum bumps");
    };
    // The wire phase runs; the node then restarts AGAIN: no stopped
    // quorum once more, and the identity only moves forward.
    let (second, rebumped) = bumped.restart().expect("the second bump succeeds");
    assert_eq!(
        second,
        RestartDecision::Bump {
            old: new,
            new: Incarnation(new.0 + 1)
        }
    );
    // A clean stop then a restart continues the committed identity —
    // never a reversion to anything lower.
    let stopped = rebumped.finish_stop();
    let (third, continued) = stopped.restart().expect("the clean path continues");
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
            .all(|copy| copy.marker == Marker::Restarting)
    );
}

/// The identity space is checked: a bump one short of `u64` exhaustion
/// refuses rather than wrapping a superseded identity into circulation.
#[test]
fn d_identity_exhaustion_refuses() {
    let stored = copies([Marker::Joining; 4], u64::MAX);
    assert_eq!(
        stored.restart().err(),
        Some(RestartRefusal::Exhausted(Incarnation(u64::MAX))),
        "the bump refuses at exhaustion"
    );
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

/// The reincarnated weight-0 standby: the memo stream keeps it current.
/// The ack's missed-range push and every leader-originated prepare and
/// commit land at it (addressed TO a non-member, §6); it folds the
/// committed operations they carry, adopts no view and stays fenced —
/// and its own answers are discarded before they are ever counted. The
/// §10 self-fetch route stays available but is not needed: no fetch is
/// opened.
#[test]
fn f_standby_streams_current_without_fetch_and_cannot_influence() {
    let mut h = cluster();
    reincarnate_backup(&mut h, Stop::AfterFirstEra);
    // Instant sync without a round trip: the ack's missed-range push and
    // the memo'd establishing prepare and its commit carried the standby
    // to the leader's committed frontier. The admitting era folded; the
    // boot fence is intact (the standby adopted no view).
    assert_eq!(
        snap(&h, n(3)).committed,
        snap(&h, n(0)).committed,
        "the standby synced to the leader's committed frontier"
    );
    assert_eq!(current_era(&h, n(3)), Era(2), "the admitting era folded");
    assert_eq!(
        status_of(&h, n(3)),
        Status::Restarting,
        "the standby stays fenced; the memo'd stream adopts no view"
    );
    assert!(
        !h.step_trace()
            .iter()
            .any(|line| line.contains("net send n3") && line.contains("GetState")),
        "no fetch was opened: the §10 self-fetch route is not needed here"
    );

    // The era the join committed awaits the ordinary view change
    // (§8.7.4); the fence view's recipients are the era that includes the
    // standby, so the StartView reaches it and installs — the standby,
    // already current, adopts the view through the ordinary install.
    let _ = drive_view_change(&mut h, &[n(0), n(1)]);
    assert_eq!(
        status_of(&h, n(3)),
        Status::Normal,
        "the standby is caught up"
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
        "the standby's vote is discarded by name"
    );

    // The quorum outcome is unaffected: the commit lands on the voting
    // members alone, and the ordinary stream keeps the standby current.
    let outcome = h.propose(n(1), op_id(2), b"y");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert!(
        snap(&h, n(1)).committed >= 4,
        "the operation committed without the standby"
    );
    assert_eq!(
        snap(&h, n(3)).committed,
        snap(&h, n(1)).committed,
        "the standby stayed current through the client commit"
    );
    h.assert_safety();
}

/// The §10 self-fetch route stays available: a boot-fenced standby whose
/// memo'd stream was lost — the ack's establishing prepare and the commit
/// it rode both dropped — retains the fence view's StartView offer, fetches
/// the missing range under its boot view, folds the era that admitted it
/// through the §10 acquisition, and completes the catch-up through the
/// ordinary install. It never voted and stays fenced throughout.
#[test]
fn f_boot_fetch_route_acquires_the_admitting_era() {
    let mut h = cluster();
    bootstrap(&mut h);
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    h.crash(n(2));
    h.restart_as(n(2), n(3)).expect("the bumped node reopens");
    // The announcement reaches the leader; the ack's establishing prepare
    // (the memo stream's first beat) and the commit it rode are then both
    // dropped, so the standby's only route back is the fetch it opens
    // itself (§10).
    let outcome = h.reincarnate(n(3), n(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_to(n(0));
    h.drop_queued(n(3));
    h.deliver_all();
    h.drop_queued(n(3));
    h.assert_safety();
    assert_eq!(current_era(&h, n(0)), Era(2), "the forced step committed");
    assert_eq!(snap(&h, n(3)).committed, 3, "the standby heard nothing");

    // The first forced era awaits the ordinary view change (§8.7.4); the
    // fence view's recipients are the era that includes the standby, so
    // the StartView reaches it. The era is one past its boot table (§10):
    // the ruling retains the offer and fetches the missing range under
    // the boot view, the leader serves the fetch (the standby is a member
    // of the leader's current committed configuration), and the boot-fenced
    // acquisition folds the era that admitted it — the standby never
    // voted, adopted nothing, and stays fenced.
    let _ = drive_view_change(&mut h, &[n(0), n(1)]);
    assert_eq!(
        current_era(&h, n(3)),
        Era(2),
        "the standby folded the era that admitted it through the §10 acquisition"
    );
    assert_eq!(
        status_of(&h, n(3)),
        Status::Restarting,
        "the acquisition runs at the boot fence, never voting"
    );

    // The stream arrives and is processed: the standby, boot-fenced and
    // outside every configuration it can name, takes the committed
    // operations the leader-originated stream carries at its boot fence
    // (§10) — no view adopted, no vote ever counted.
    let outcome = h.propose(n(1), op_id(2), b"y");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        status_of(&h, n(3)),
        Status::Restarting,
        "the standby stays fenced through the stream"
    );

    // The retained offer re-runs on an ordinary tick (§13.1 step 5): the
    // fetched era makes it evaluable, the install adopts the view, and
    // the boot fence is discharged by the fetch the standby opened.
    h.tick(n(3));
    assert_eq!(
        status_of(&h, n(3)),
        Status::Normal,
        "the standby is caught up"
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
        "the standby's vote is discarded by name"
    );

    // The quorum outcome is unaffected: the commit lands on the voting
    // members alone.
    assert!(
        snap(&h, n(1)).committed >= 4,
        "the operation committed without the standby"
    );
    h.assert_safety();
}

/// The leader's immediate ack pushes exactly the missed range (§4, §7):
/// the standby's past-life prepared frontier sat below the leader's
/// committed frontier, and the FIRST transition answering the fresh
/// announcement carries the range as one `NewState` addressed to the
/// standby, alongside the armed machine's first forced step.
#[test]
fn b_ack_pushes_missed_range_and_proposes_first_step_in_one_transition() {
    let mut h = cluster();
    bootstrap(&mut h);
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    // A second operation commits without n(2): its prepare is dropped,
    // so the reincarnated standby's past-life prepared frontier sits
    // below the leader's committed frontier.
    let outcome = h.propose(n(0), op_id(2), b"y");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_to(n(1));
    h.crash(n(2));
    h.deliver_to(n(0));
    h.deliver_to(n(1));
    h.assert_safety();
    assert_eq!(current_era(&h, n(0)), Era(1));
    assert_eq!(snap(&h, n(0)).committed, 4);

    h.restart_as(n(2), n(3)).expect("the bumped node reopens");
    let outcome = h.reincarnate(n(3), n(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    // The FIRST transition answering the fresh announcement: the ack.
    let outcome = h
        .deliver_to(n(0))
        .expect("the announcement reaches the leader");
    let StepOutcome::Published { effects, .. } = outcome.outcome else {
        panic!("the ack publishes: {:?}", outcome.outcome);
    };
    // The missed range, first in release order (§7's streaming order).
    let Effect::Send { to, message, .. } = &effects[0] else {
        panic!("the ack's effects are sends: {:?}", effects[0]);
    };
    assert_eq!(*to, n(3), "the push is addressed to the standby");
    assert_eq!(message.header.tag, Tag::NewState);
    let Body::NewState {
        entries,
        through,
        committed: _,
        more,
    } = &message.body
    else {
        unreachable!("the tag names the body");
    };
    assert_eq!(entries.len(), 1, "exactly the missed slot");
    assert_eq!(
        entries[0].slot,
        Slot(4),
        "the range starts past the standby's prepared frontier"
    );
    assert_eq!(
        *through,
        Slot(4),
        "the chunk ends at the leader's committed frontier"
    );
    assert!(!(*more), "the whole missed range fits one chunk");
    // The armed machine proposes the first forced step in the SAME
    // transition: no waiting for an era to commit.
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::Send { message, .. }
                if message.header.tag == Tag::Prepare
                    && matches!(
                        &message.body,
                        Body::Prepare { entry, .. }
                            if matches!(
                                &entry.payload,
                                vrr::journal::Payload::System(
                                    SystemOperation::Batch(ops)
                                ) if *ops
                                    == vec![
                                        SystemOperation::Decrement(n(2)),
                                        SystemOperation::Join {
                                            node: n(3),
                                            position: 2
                                        }
                                    ]
                            )
                    )
        )),
        "the first forced step is proposed in the same transition"
    );
    // A memo'd copy of that establishing prepare is addressed to the
    // standby too: the memo stream's first beat.
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::Send { to, message, .. }
                if *to == n(3)
                    && message.header.tag == Tag::Prepare
                    && matches!(
                        &message.body,
                        Body::Prepare { entry, .. }
                            if matches!(
                                &entry.payload,
                                vrr::journal::Payload::System(
                                    SystemOperation::Batch(_)
                                )
                            )
                    )
        )),
        "the establishing prepare is memo'd to the standby"
    );

    // Instant sync: the pushed range alone brings the standby to the
    // leader's committed frontier, with no fetch of its own.
    let outcome = h.deliver_to(n(3)).expect("the push delivers");
    assert!(
        matches!(outcome.outcome, StepOutcome::Published { .. }),
        "the push installs: {:?}",
        outcome.outcome
    );
    assert_eq!(
        snap(&h, n(3)).committed,
        snap(&h, n(0)).committed,
        "the standby synced to the leader's committed frontier"
    );
    assert!(
        !h.step_trace()
            .iter()
            .any(|line| line.contains("net send n3") && line.contains("GetState")),
        "no fetch was opened"
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
            committed: Slot(3),
            prepared: Slot(3),
        },
    };
    let encoded = encode(&message);
    assert_eq!(
        encoded.len(),
        20 + 1 + 4 + 4 + 8 + 8,
        "header + discriminant + two u32 identities + two u64 slots"
    );

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
            committed: Slot(2),
            prepared: Slot(2),
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
            committed: Slot::NONE,
            prepared: Slot::NONE,
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
            committed: Slot(3),
            prepared: Slot(3),
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
            committed: Slot(3),
            prepared: Slot(3),
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

// ---------------------------------------------------------------------------
// H. The host obligation: the announcement must be delivered AS the bumped
// identity
// ---------------------------------------------------------------------------

/// The downstream wedge of lunet-locks issue #26, reproduced as a protocol
/// fact. A crashed node bumps its identity and emits the CORRECT
/// reincarnation (`old=2, new=3`, frontiers present); a non-compliant host
/// transport stamps the outbound sender from a stale learned map, so the
/// frame arrives attributed to the OLD identity. The engine's refusal is
/// the protocol's anti-spoof guard working: an announcement whose claimed
/// sender does not match the claimed new identity must never drive a
/// reconfiguration. The compliant host (the corpus's `reincarnate` drive,
/// sender = the bumped node) commits the fused batch in one pass; the
/// mis-attributed announcement wedges, with safety held.
#[test]
fn h_the_announcement_must_be_attributed_to_the_new_identity() {
    let mut h = cluster();
    bootstrap(&mut h);
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    h.crash(n(2));
    h.restart_as(n(2), n(3)).expect("the bumped node reopens");

    // The violation, exactly as the wire carried it: the correct body,
    // delivered with sender = the OLD identity. The leader refuses by
    // name, and the fused walk never arms — the wedge their rig sat in.
    let mis_attributed = Message {
        header: Header {
            tag: Tag::Reincarnation,
            view: current_view(&h, n(0)),
            slot: Slot::NONE,
        },
        body: Body::Reincarnation {
            old: n(2),
            new: n(3),
            committed: Slot(3),
            prepared: Slot(3),
        },
    };
    h.inject(n(2), n(0), mis_attributed);
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::ReincarnationRefused {
            sender: n(2),
            view: current_view(&h, n(0)),
        }),
        "an announcement attributed to the old identity is refused by name"
    );
    assert_eq!(
        current_era(&h, n(0)),
        Era(1),
        "no fused batch commits off a mis-attributed announcement"
    );

    // The compliant delivery of the SAME pair — sender = the bumped
    // node — commits the fused batch in one pass: the protocol's part is
    // proven, and the obligation is the host's attribution, not the
    // announcement's content.
    let outcome = h.reincarnate(n(3), n(2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(
        current_era(&h, n(0)),
        Era(2),
        "the compliant announcement commits Batch([Decrement, Join])"
    );
    assert_eq!(current_order(&h, n(0)), vec![n(0), n(1), n(3), n(2)]);
    assert_eq!(current_weights(&h, n(0)), vec![1, 1, 0, 0]);
    h.assert_safety();
}

// Silence the unused-import lint for the era-table type used in helpers.
