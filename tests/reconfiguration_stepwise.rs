//! End-to-end integration tests for the stop-the-world reconfiguration
//! path (§8.7.4): genesis as ordinary committed log history, one
//! establishing operation at a time through the ordinary pipeline, the
//! pre-proposal intersection gate (Q1), era authorization on entries
//! (§8.7.3), reclamation-proof configuration (§4), the era-0 ruling
//! (§8.7.8), the VOID/INIT refusals (§8.7.2), and the never-half-installs
//! recovery guarantee.

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{
    ConfigError, Configuration, INIT_SLOT, SystemOperation, VOID_SLOT, Weight,
};
use vrr::effects::{Effect, Stability};
use vrr::ids::{Era, NodeId, OperationId, Slot, Tick, View, ViewId};
use vrr::journal::{Journal, JournalView, LogEntry, Payload, SegmentedLog};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::{ProgressSnapshot, Status};
use vrr::quorum::{QuorumError, QuorumStrategy, R2Direction, Role};
use vrr::replica::{Input, PlanRefusal, Replica, TimedInput, ViewChangeKnobs};
use vrr::wire::{Header, Tag};

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

/// A view in era 2 — the era the first committed reconfiguration
/// establishes.
fn era2_view(number: u32) -> ViewId {
    ViewId {
        era: Era(2),
        view: View(number),
    }
}

/// The snapshot of a live node (tests never snapshot a crashed one).
fn snap(h: &Harness, id: NodeId) -> ProgressSnapshot {
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

/// The primary of a view under the genesis order 0, 1, 2 (§1.2) — the
/// position rotates on the view number alone, whatever the weights say.
fn primary_of(view: ViewId) -> NodeId {
    n(view.view.0 % 3)
}

/// The newest era the node's committed configuration history has
/// established (§8.7.1).
fn current_era(h: &Harness, id: NodeId) -> Era {
    h.era_table(id).expect("the node is live").current().era
}

/// The member weights of the node's current configuration, in order —
/// the configuration equality a reconfiguration script asserts.
fn current_weights(h: &Harness, id: NodeId) -> Vec<u64> {
    let table = h.era_table(id).expect("the node is live");
    table
        .current()
        .config
        .order()
        .iter()
        .map(|member| u64::from(member.weight.0))
        .collect()
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

/// Ticks a node until its timeout fires (one tick past the knob),
/// asserting it fenced into exactly `target`.
fn tick_into_view_change(h: &mut Harness, id: NodeId, target: ViewId) {
    for _ in 0..=TIMEOUT {
        h.tick(id);
    }
    assert_eq!(status_of(h, id), Status::ViewChange);
    assert_eq!(current_view(h, id), target);
}

/// Drives a complete view change to `target` among the live nodes: the
/// prime times out first, the exchange runs out, every live node installs
/// the new view.
fn drive_view_change(h: &mut Harness, live: &[NodeId], target: ViewId) {
    let prime = primary_of(target);
    tick_into_view_change(h, prime, target);
    h.deliver_all();
    for &id in live {
        assert_eq!(status_of(h, id), Status::Normal, "{id:?} installs");
        assert_eq!(current_view(h, id), target, "{id:?} is in the target");
    }
    h.assert_safety();
}

// ---------------------------------------------------------------------
// 1. Genesis is ordinary committed log history — no special casing.
// ---------------------------------------------------------------------
#[test]
fn genesis_is_law() {
    let mut h = Harness::provision(3);
    for id in [n(0), n(1), n(2)] {
        // VOID occupies slot 1 in era 0; INIT occupies slot 2 in era 1 —
        // two ordinary committed system entries, nothing privileged
        // (§8.7.1, §8.7.2).
        let void = h.journal_entry(id, VOID_SLOT).expect("genesis entry");
        assert_eq!(void.era, Era::INITIAL);
        assert_eq!(void.payload, Payload::System(SystemOperation::Void));
        let init = h.journal_entry(id, INIT_SLOT).expect("genesis entry");
        assert_eq!(init.era, Era(1));
        assert_eq!(
            init.payload,
            Payload::System(SystemOperation::Init {
                order: vec![n(0), n(1), n(2)]
            })
        );
        // Both are committed by construction, and the applied frontier
        // walks them like any system slot (B2's system-slot ruling).
        let snapshot = snap(&h, id);
        assert_eq!((snapshot.era, snapshot.view), (1, 0));
        assert_eq!((snapshot.accepted, snapshot.committed), (2, 2));
        assert_eq!(snapshot.applied, 2);
        // The initial configuration is DERIVED from the entries: the
        // founding order as given, every weight 1 (§8.7.1).
        let table = h.era_table(id).expect("the node is live");
        let current = table.current();
        assert_eq!(current.era, Era(1));
        assert_eq!(current.established_by, INIT_SLOT);
        assert_eq!(current.config.len(), 3);
        for (position, member) in current.config.order().iter().enumerate() {
            assert_eq!(member.node, n(u32::try_from(position).expect("small")));
            assert_eq!(member.weight, Weight(1));
        }
        // The era-0 record is the VOID record: an empty configuration,
        // established at slot 1 (§8.7.8).
        let zero = table.record(Era::INITIAL).expect("era 0 is recorded");
        assert_eq!(zero.established_by, VOID_SLOT);
        assert_eq!(zero.config.len(), 0);
        // Before participation is proved there is no legal proposal: the
        // fenced node refuses by name.
        let outcome = h.propose(id, op_id(1), b"x");
        assert!(matches!(
            outcome,
            StepOutcome::PlanRefused(PlanRefusal::NotPrimary { .. })
        ));
    }
    // Identical provisioning — three nodes, one genesis: after the
    // bootstrap tick the primary serves the first real slot.
    bootstrap(&mut h);
    let outcome = h.propose(n(0), op_id(1), b"x");
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "after genesis the primary serves"
    );
}

// ---------------------------------------------------------------------
// 2. The stop-the-world path: propose, commit, era advances.
// ---------------------------------------------------------------------
#[test]
fn stop_the_world_reconfigure_advances_the_era() {
    let mut h = cluster();
    bootstrap(&mut h);

    // The primary proposes INCREMENT(n2) through the ordinary pipeline —
    // one published proposal, Prepare effects to the era-1 membership,
    // the entry stamped with the era that authorizes its slot (§8.7.3:
    // the table has not advanced yet, so the stamp is the view's era).
    let outcome = h.reconfigure(n(0), SystemOperation::Increment(n(2)), None);
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the reconfiguration is a legal proposal, not {outcome:?}");
    };
    let Effect::Send { era, message, .. } = &effects[0] else {
        panic!("the first effect is the Prepare send, not {effects:?}");
    };
    assert_eq!(*era, Era(1));
    let Body::Prepare { entry, .. } = &message.body else {
        panic!("the message is a Prepare, not {message:?}");
    };
    assert_eq!(entry.slot, Slot(3));
    assert_eq!(entry.era, Era(1));
    assert_eq!(
        entry.payload,
        Payload::System(SystemOperation::Increment(n(2)))
    );
    // The entry is in the proposer's own journal, era 1.
    let own = h
        .journal_entry(n(0), Slot(3))
        .expect("the proposal is logged");
    assert_eq!(own.era, Era(1));

    // One PrepareOk completes the era-1 commit quorum (QII_1 = 2 of
    // weight 3): the COMMIT of slot 3 establishes era 2 (§8.7.1) — and
    // not one step earlier.
    assert_eq!(current_era(&h, n(0)), Era(1), "accepted is not established");
    h.deliver_all();
    assert_eq!(snap(&h, n(0)).committed, 3);
    assert_eq!(current_era(&h, n(0)), Era(2));
    let table = h.era_table(n(0)).expect("the node is live");
    assert_eq!(table.current().established_by, Slot(3));
    assert_eq!(table.current().config.weight_of(n(2)), Some(Weight(2)));
    // The era-1 record still names the genesis slot (§8.7.8).
    assert_eq!(
        table
            .record(Era(1))
            .expect("era 1 is recorded")
            .established_by,
        INIT_SLOT
    );

    // The backups fold on the Commit announcement (§13.3): the same
    // configuration history, everywhere.
    assert_eq!(current_era(&h, n(1)), Era(2));
    assert_eq!(current_era(&h, n(2)), Era(2));
    assert_eq!(current_weights(&h, n(0)), vec![1, 1, 2]);
    assert_eq!(current_weights(&h, n(1)), vec![1, 1, 2]);
    assert_eq!(current_weights(&h, n(2)), vec![1, 1, 2]);

    // The view has NOT changed — only the era advanced — and subsequent
    // proposals are stamped with, and counted under, the era their slots
    // authorize: era 2, inside the overlap §8.7.3's relation admits.
    let outcome = h.propose(n(0), op_id(1), b"after");
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the proposal publishes, not {outcome:?}");
    };
    let Effect::Send { message, .. } = &effects[0] else {
        panic!("the first effect is the Prepare send, not {effects:?}");
    };
    let Body::Prepare { entry, .. } = &message.body else {
        panic!("the message is a Prepare, not {message:?}");
    };
    assert_eq!(entry.era, Era(2), "the slot is authorized by era 2");
    h.deliver_all();
    assert_eq!(snap(&h, n(0)).committed, 4);
    let accepted = h.journal_entry(n(1), Slot(4)).expect("the backup accepted");
    assert_eq!(accepted.era, Era(2));
    // The era-2 quorum is the weighted one (QII_2 = 3 of weight 4): the
    // operation committed under the configuration its own era names.
    h.assert_safety();
}

// ---------------------------------------------------------------------
// 5. The era/slot relation is enforced at accept: out-of-era entries are
//    refused by name, and only the relation's window is accepted.
// ---------------------------------------------------------------------
#[test]
fn era_slot_relation_is_enforced_at_accept() {
    let mut h = cluster();
    bootstrap(&mut h);

    // A forged Prepare from the current primary, in the current view, at
    // the next slot — everything legal except the era. Three fabrications:
    // below the view's era, inside the relation's +1 window but naming an
    // era no committed operation has established, and beyond the window.
    for forged_era in [0, 2, 3] {
        let forged = Message {
            header: Header {
                tag: Tag::Prepare,
                view: view(0),
                slot: Slot(3),
            },
            body: Body::Prepare {
                entry: LogEntry {
                    slot: Slot(3),
                    era: Era(forged_era),
                    payload: Payload::Operation {
                        id: op_id(90 + u64::from(forged_era)),
                        payload: Box::from(&b"forged"[..]),
                    },
                },
                committed: Slot(2),
            },
        };
        h.inject(n(0), n(1), forged);
        assert_eq!(
            h.diagnostic(n(1)),
            Some(Diagnostic::EraDiscipline {
                entry: Era(forged_era),
                view: view(0),
            }),
            "era {forged_era} is refused by name"
        );
        assert!(
            h.journal_entry(n(1), Slot(3)).is_none(),
            "era {forged_era} never enters the log"
        );
        assert_eq!(snap(&h, n(1)).accepted, 2, "the frontier is unmoved");
    }

    // The honest +1: a committed reconfiguration establishes era 2, and
    // the NEXT proposal — still in view (1, 0) — is stamped era 2 and
    // accepted at the backups (§8.7.3's overlap sentence).
    let outcome = h.reconfigure(n(0), SystemOperation::Increment(n(2)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(current_era(&h, n(1)), Era(2));
    h.propose(n(0), op_id(1), b"after");
    h.deliver_all();
    let entry = h.journal_entry(n(1), Slot(4)).expect("the backup accepted");
    assert_eq!(entry.era, Era(2));
    h.assert_safety();
}

// ---------------------------------------------------------------------
// 6. The committed configuration is reclamation-proof: the journal may
//    drop the establishing entry; the era table never forgets it.
// ---------------------------------------------------------------------
#[test]
fn committed_configuration_survives_reclamation() {
    let mut h = Harness::with_knobs_and_journal_capacity(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
        1,
    );
    bootstrap(&mut h);

    // A reconfiguration commits at slot 3; the applied frontier walks
    // the system slot (B2's system-slot ruling), so the host may
    // checkpoint through it.
    let outcome = h.reconfigure(n(0), SystemOperation::Increment(n(2)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(snap(&h, n(0)).committed, 3);
    assert_eq!(current_era(&h, n(0)), Era(2));
    let outcome = h.checkpoint(n(0), Slot(3));
    assert!(matches!(outcome, StepOutcome::Published { .. }));

    // The next proposal moves the frontier and the reclaim drops the
    // whole slabs at or below the checkpoint (§4) — genesis AND the
    // establishing entry are gone from the journal.
    h.propose(n(0), op_id(1), b"x");
    assert!(
        h.journal_entry(n(0), Slot(3)).is_none(),
        "the establishing entry was reclaimed"
    );
    assert!(
        h.journal_entry(n(0), VOID_SLOT).is_none(),
        "genesis was reclaimed with it"
    );
    // The era table is checkpoint-stable: era 2 is still established by
    // the slot the journal no longer holds (§4, §8.7.1).
    let table = h.era_table(n(0)).expect("the node is live");
    assert_eq!(table.current().era, Era(2));
    assert_eq!(table.current().established_by, Slot(3));
    h.deliver_all();
    assert_eq!(snap(&h, n(0)).committed, 4);

    // An era-2 view change runs over the reclaimed prefix: the evidence
    // suffix is bounded by the retained window, the slots the node itself
    // checkpoint-authorized away are fixed by §9.2's quorum-identity
    // argument, and the view installs — from the era table, never from
    // the reclaimed entries.
    drive_view_change(&mut h, &[n(0), n(1), n(2)], era2_view(1));
    assert_eq!(primary_of(era2_view(1)), n(1));

    // The new primary reconfigures again: the chain keeps folding from
    // the table alone, never re-reading the reclaimed entries.
    let outcome = h.reconfigure(n(1), SystemOperation::Increment(n(0)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        assert_eq!(current_era(&h, id), Era(3), "{id:?} folded era 3");
        assert_eq!(current_weights(&h, id), vec![2, 1, 2], "{id:?} agrees");
    }
    h.assert_safety();
}

// ---------------------------------------------------------------------
// 8. A primary crash mid-reconfiguration never half-installs an era:
//    either the establishing operation's commit survives the view change
//    and the era advances when history does, or the operation is absent
//    from the selected history and the era never moved.
// ---------------------------------------------------------------------
#[test]
fn primary_crash_mid_reconfiguration_never_half_installs() {
    // Case A: one backup accepted the establishing operation before the
    // crash. The era-1 view change carries it into the new history
    // (§9.3), still uncommitted — the era has NOT advanced...
    let mut h = cluster();
    bootstrap(&mut h);
    let outcome = h.reconfigure(n(0), SystemOperation::Increment(n(1)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_to(n(1)).expect("the Prepare is queued for n1");
    h.drop_queued(n(2));
    h.crash(n(0));
    h.deliver_all();

    drive_view_change(&mut h, &[n(1), n(2)], view(1));
    assert_eq!(snap(&h, n(1)).accepted, 3, "the suffix survived");
    assert_eq!(snap(&h, n(1)).committed, 2, "but is not committed");
    assert_eq!(
        current_era(&h, n(1)),
        Era(1),
        "an uncommitted operation establishes nothing"
    );
    assert_eq!(current_era(&h, n(2)), Era(1));

    // ...until the new primary's next proposal gathers the quorum that
    // re-commits slot 3 — and the era advances exactly there (§8.7.1).
    h.propose(n(1), op_id(1), b"after");
    h.deliver_all();
    assert_eq!(snap(&h, n(1)).committed, 4);
    for id in [n(1), n(2)] {
        assert_eq!(current_era(&h, id), Era(2), "{id:?} folded on commit");
        assert_eq!(current_weights(&h, id), vec![1, 2, 1], "{id:?} agrees");
    }
    h.assert_safety();

    // Case B: NO backup accepted the establishing operation. The selected
    // history ends at slot 2; the operation is gone, the era never
    // moved, and the slot is reused by the next ordinary proposal (§9.2:
    // the suffix beyond the committed frontier is replaced wholesale).
    let mut h = cluster();
    bootstrap(&mut h);
    let outcome = h.reconfigure(n(0), SystemOperation::Increment(n(1)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.drop_queued(n(1));
    h.drop_queued(n(2));
    h.crash(n(0));
    h.deliver_all();

    drive_view_change(&mut h, &[n(1), n(2)], view(1));
    assert_eq!(snap(&h, n(1)).accepted, 2, "the suffix did not survive");
    h.propose(n(1), op_id(1), b"replacement");
    h.deliver_all();
    let reused = h.journal_entry(n(1), Slot(3)).expect("the slot was reused");
    assert!(
        matches!(reused.payload, Payload::Operation { .. }),
        "slot 3 holds the replacement, not the lost reconfiguration"
    );
    for id in [n(1), n(2)] {
        assert_eq!(current_era(&h, id), Era(1), "{id:?} never half-installed");
        assert_eq!(current_weights(&h, id), vec![1, 1, 1], "{id:?} agrees");
    }
    h.assert_safety();
}

// ---------------------------------------------------------------------
// 3. The pre-proposal intersection gate (Q1): an operation whose QI_e /
//    QII_(e+1) intersection can be empty is refused BEFORE the proposal,
//    carrying the witness — the two disjoint vote sets that cannot both
//    be legal.
// ---------------------------------------------------------------------

/// Fixed-threshold quorum strategy (test-only): thresholds per role,
/// independent of the membership. Six members of weight 1 each, fence 4 /
/// view-change 4 / commit 3, is legality-symmetric WITHIN one era — but
/// INCREMENT(n0) takes the weights to (2,1,1,1,1,1): the same fixed
/// thresholds then admit a disjoint view-change quorum {n1,n2,n3,n4}
/// (weight 4, under era e) and commit quorum {n0,n5} (weight 3, under era
/// e+1) — the Q1 violation `validate_transition` must refuse.
struct Thresholds {
    fence: u64,
    view_change: u64,
    commit: u64,
    recovery: u64,
}

impl Thresholds {
    fn threshold_of(&self, role: Role) -> u64 {
        match role {
            Role::Commit => self.commit,
            Role::ViewChange => self.view_change,
            Role::Recovery => self.recovery,
            Role::Fence => self.fence,
        }
    }
}

impl QuorumStrategy for Thresholds {
    fn is_quorum(&self, role: Role, config: &Configuration, members: &[NodeId]) -> bool {
        match config.weight_of_set(members) {
            Some(weight) => weight >= self.threshold_of(role),
            None => false,
        }
    }

    fn threshold(&self, role: Role, _config: &Configuration) -> Option<u64> {
        Some(self.threshold_of(role))
    }
}

#[test]
fn unsafe_transition_is_refused_pre_proposal_with_its_witness() {
    let order: Vec<NodeId> = (0..6).map(n).collect();
    let strategy = Thresholds {
        fence: 4,
        view_change: 4,
        commit: 3,
        recovery: 4,
    };
    let mut replica = Replica::provision(
        n(0),
        order,
        strategy,
        SegmentedLog::new(),
        Stability::Volatile,
        ViewChangeKnobs {
            primary_timeout: 0,
            view_change_budget: usize::MAX,
        },
    )
    .expect("the fixed thresholds are legal WITHIN era 1");
    // Bootstrap the view like the harness does: the genesis primary
    // promotes itself on the first tick.
    let promoted = replica
        .plan(
            &TimedInput {
                at: Tick(1),
                event: Input::Tick,
            },
            &replica.journal().view(),
        )
        .expect("the bootstrap tick plans");
    replica
        .publish(promoted)
        .expect("the bootstrap tick installs");

    // The unsafe operation: refused BEFORE proposal, with the witness —
    // and the log is untouched.
    let view = replica.journal().view();
    let refusal = replica
        .plan(
            &TimedInput {
                at: Tick(2),
                event: Input::Reconfigure {
                    op: SystemOperation::Increment(n(0)),
                    pivot: None,
                },
            },
            &view,
        )
        .expect_err("the gate refuses the unsafe transition");
    assert_eq!(
        refusal,
        PlanRefusal::ReconfigureQuorum(QuorumError::R2Violation {
            direction: R2Direction::Forward,
            view_change: vec![n(1), n(2), n(3), n(4)],
            commit: vec![n(0), n(5)],
        })
    );
    assert_eq!(
        replica.progress().accepted(),
        INIT_SLOT,
        "a refused operation never enters the log"
    );
    assert_eq!(replica.journal().view().accepted(), Some(INIT_SLOT));
    // The era table is untouched too.
    assert_eq!(replica.progress().config().current().era, Era(1));
}

// ---------------------------------------------------------------------
// 4. The §8.7.2 preconditions refuse by name — before the log moves.
// ---------------------------------------------------------------------
#[test]
fn precondition_refusals_are_named_and_leave_the_log_untouched() {
    let mut h = cluster();
    bootstrap(&mut h);

    // Every refusal leaves the frontier exactly where it was.
    let refused = |h: &mut Harness, op: SystemOperation, expected: PlanRefusal| {
        let before = snap(h, n(0)).accepted;
        let outcome = h.reconfigure(n(0), op, None);
        assert_eq!(outcome, StepOutcome::PlanRefused(expected));
        assert_eq!(snap(h, n(0)).accepted, before, "the log is untouched");
    };

    // INCREMENT of a node outside the membership.
    refused(
        &mut h,
        SystemOperation::Increment(n(9)),
        PlanRefusal::Reconfigure(ConfigError::NotAMember(n(9))),
    );

    // A legal DECREMENT commits (slot 3, era 2: weights 1, 0, 1)...
    let outcome = h.reconfigure(n(0), SystemOperation::Decrement(n(1)), None);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    assert_eq!(current_era(&h, n(0)), Era(2));
    // ...the ordinary view change carries the cluster into era 2, and
    // the position — not the weight — makes n1 the primary (§8.7.8).
    drive_view_change(&mut h, &[n(0), n(1), n(2)], era2_view(1));
    assert_eq!(primary_of(era2_view(1)), n(1));

    let refused = |h: &mut Harness, op: SystemOperation, expected: PlanRefusal| {
        let before = snap(h, n(1)).accepted;
        let outcome = h.reconfigure(n(1), op, None);
        assert_eq!(outcome, StepOutcome::PlanRefused(expected));
        assert_eq!(snap(h, n(1)).accepted, before, "the log is untouched");
    };

    // DECREMENT below zero.
    refused(
        &mut h,
        SystemOperation::Decrement(n(1)),
        PlanRefusal::Reconfigure(ConfigError::WeightUnderflow(n(1))),
    );
    // HALVE with an odd weight in the membership.
    refused(
        &mut h,
        SystemOperation::Halve,
        PlanRefusal::Reconfigure(ConfigError::OddWeight(n(0))),
    );
    // JOIN of an existing member.
    refused(
        &mut h,
        SystemOperation::Join {
            node: n(0),
            position: 0,
        },
        PlanRefusal::Reconfigure(ConfigError::DuplicateNode(n(0))),
    );
    // LEAVE with weight still on the member.
    refused(
        &mut h,
        SystemOperation::Leave(n(0)),
        PlanRefusal::Reconfigure(ConfigError::NonZeroWeight(n(0))),
    );

    // A legal JOIN commits: the joined member is a LEARNER — the fold
    // grants weight 0, whatever the operator asked for (§8.7.2; rules §2, R2).
    let outcome = h.reconfigure(
        n(1),
        SystemOperation::Join {
            node: n(3),
            position: 3,
        },
        None,
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        assert_eq!(current_era(&h, id), Era(3), "{id:?} folded era 3");
        assert_eq!(current_weights(&h, id), vec![1, 0, 1, 0], "{id:?} agrees");
    }
    h.assert_safety();
}

// ---------------------------------------------------------------------
// 7. VOID and INIT outside genesis are refused — at the gate and at the
//    accept path — and a fabricated genesis replay never enters the log.
// ---------------------------------------------------------------------
#[test]
fn void_and_init_outside_genesis_are_refused() {
    let mut h = cluster();
    bootstrap(&mut h);
    let genesis: Vec<NodeId> = vec![n(0), n(1), n(2)];

    // VOID after genesis: refused by name at the gate.
    let outcome = h.reconfigure(n(0), SystemOperation::Void, None);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Reconfigure(ConfigError::AlreadyInitialised))
    );
    // INIT after genesis — a duplicate — same refusal.
    let outcome = h.reconfigure(
        n(0),
        SystemOperation::Init {
            order: genesis.clone(),
        },
        None,
    );
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Reconfigure(ConfigError::AlreadyInitialised))
    );
    assert_eq!(snap(&h, n(0)).accepted, 2, "the log is untouched");

    // A fabricated VOID Prepare at an ordinary slot — genesis replay —
    // never enters a backup's log: the accept path runs the same fold
    // (§8.7.2) and drops the entry by name.
    let forged = Message {
        header: Header {
            tag: Tag::Prepare,
            view: view(0),
            slot: Slot(3),
        },
        body: Body::Prepare {
            entry: LogEntry {
                slot: Slot(3),
                era: Era(1),
                payload: Payload::System(SystemOperation::Void),
            },
            committed: Slot(2),
        },
    };
    h.inject(n(0), n(1), forged);
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::InvalidSystemOperation {
            slot: Slot(3),
            error: ConfigError::AlreadyInitialised,
        })
    );
    assert!(h.journal_entry(n(1), Slot(3)).is_none());
    assert_eq!(snap(&h, n(1)).accepted, 2, "the frontier is unmoved");

    // A fabricated duplicate INIT meets the same wall.
    let forged = Message {
        header: Header {
            tag: Tag::Prepare,
            view: view(0),
            slot: Slot(3),
        },
        body: Body::Prepare {
            entry: LogEntry {
                slot: Slot(3),
                era: Era(1),
                payload: Payload::System(SystemOperation::Init { order: genesis }),
            },
            committed: Slot(2),
        },
    };
    h.inject(n(0), n(1), forged);
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::InvalidSystemOperation {
            slot: Slot(3),
            error: ConfigError::AlreadyInitialised,
        })
    );
    assert!(h.journal_entry(n(1), Slot(3)).is_none());
    h.assert_safety();
}
