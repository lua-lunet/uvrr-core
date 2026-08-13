//! Contract for `vrr::replica`, `vrr::effects` and `vrr::message`: the transition
//! pipeline, the stability handshake, the lifecycle, and the wire vocabulary.
//!
//! Spec §5 (progress record), §6 (delta transition), §7 (serialized transition
//! interval and stability levels), §11 (application boundary), §12 (concurrency
//! contract), §14.2 (the amnesiac voter), and decisions S2 (plan/publish/confirm),
//! S3 (three-way stability), S4 (the recovery nonce is the tick), B1 (observation
//! on publish only), W1 (era in every header), Q1 (the quorum gate runs at
//! construction).
//!
//! The load-bearing property of the whole design: **nothing externally observable
//! is released before publication**. Observation changes on `publish`, never on
//! `plan`; in an external-stability mode, publication happens at the `Stable`
//! confirmation, not when the persistence intent is emitted.
//!
//! The properties pinned here, each of which the protocol suites are entitled
//! to assume:
//!
//! 1.  provision constructs exactly the genesis state (era 1, view 0, slots 1–2
//!     committed, fenced `Recovering`) and refuses a non-member, a duplicate or
//!     over-cap order, and a genesis configuration the quorum gate rejects;
//! 2.  reopen validates the persisted progress against the journal, forces the
//!     fenced `Recovering` boot rule, and preserves a persisted fault across
//!     restart;
//! 3.  volatile publication releases effects at `publish`, exactly once, and the
//!     observation changes on publish only;
//! 4.  external-stability publication emits `Persist` and nothing else, parks,
//!     and the three-way `StabilityResult` completes, discards, or faults;
//! 5.  exactly one transition is ever outstanding (§12);
//! 6.  revision discipline: a stale plan and a double publish are both rejected;
//! 7.  a faulted replica refuses EVERY `Input` variant — this test is an
//!     exhaustive match over `Input`, so a new variant fails to compile here
//!     until it is handled;
//! 8.  `invariant::legal` is genuinely on the publish path: a candidate that
//!     violates a frontier rule is discarded and the node faults — never repair;
//! 9.  SANS-I/O is mechanical: the three modules contain no clock, no network,
//!     no filesystem, no thread spawn;
//! 10. every `Body` variant round-trips the normative codec with an exact
//!     `packed_len`, and every body's header slot satisfies `header_slot_role`.

use std::sync::Arc;

use vrr::configuration::{ConfigError, Configuration, SystemOperation};
use vrr::effects::{Effect, Stability, StabilityResult};
use vrr::ids::{ClientId, Era, Fault, NodeId, RequestNumber, Slot, Tick, View, ViewId};
use vrr::invariant::{InputKind, header_slot_role};
use vrr::journal::{Journal, JournalView, LogEntry, LogView, Payload, SegmentedLog};
use vrr::message::{Body, EraProof, EvidenceKind, Message};
use vrr::progress::{Progress, Status};
use vrr::quorum::{QuorumError, QuorumStrategy, Role, WeightedMajority};
use vrr::replica::{
    Input, LifecycleError, PersistedProgress, PlanRejection, PublishOutcome, PublishRejection,
    Replica, TimedInput, ViewChangeKnobs,
};
use vrr::wire::{Header, Malformed, Pack, Tag, Unpack, UnpackError};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

type TestReplica = Replica<SegmentedLog, WeightedMajority>;

/// The view-change knobs that keep the view-change machinery inert: this
/// suite contracts the pipeline and the lifecycle, not the
/// view change, so no tick ever suspects and no suffix ever truncates.
const NO_VIEW_CHANGE: ViewChangeKnobs = ViewChangeKnobs {
    primary_timeout: 0,
    view_change_budget: usize::MAX,
};

fn order3() -> Vec<NodeId> {
    vec![NodeId(0), NodeId(1), NodeId(2)]
}

fn genesis_view() -> ViewId {
    ViewId {
        era: Era(1),
        view: View::INITIAL,
    }
}

/// The genesis history exactly as `provision` installs it: `Void` at slot 1,
/// `Init` at slot 2 (§8.7.2's fixed ordinals).
fn genesis_entries(order: Vec<NodeId>) -> [LogEntry; 2] {
    [
        LogEntry {
            slot: Slot(1),
            era: Era::INITIAL,
            payload: Payload::System(SystemOperation::Void),
        },
        LogEntry {
            slot: Slot(2),
            era: Era(1),
            payload: Payload::System(SystemOperation::Init { order }),
        },
    ]
}

fn genesis_journal() -> SegmentedLog {
    let mut log = SegmentedLog::new();
    log.install_suffix(Slot(1), &genesis_entries(order3()))
        .expect("genesis history installs on an empty log");
    log
}

fn provision_volatile() -> TestReplica {
    Replica::provision(
        NodeId(0),
        order3(),
        WeightedMajority,
        SegmentedLog::new(),
        Stability::Volatile,
        NO_VIEW_CHANGE,
    )
    .expect("a legal genesis provisions")
}

fn provision_forced() -> TestReplica {
    Replica::provision(
        NodeId(0),
        order3(),
        WeightedMajority,
        SegmentedLog::new(),
        Stability::Forced,
        NO_VIEW_CHANGE,
    )
    .expect("a legal genesis provisions")
}

fn view_of(replica: &TestReplica) -> LogView {
    replica.journal().view()
}

fn tick(at: u64) -> TimedInput {
    TimedInput {
        at: Tick(at),
        event: Input::Tick,
    }
}

fn confirm(revision: u64, result: StabilityResult) -> TimedInput {
    TimedInput {
        at: Tick(1_000),
        event: Input::StabilityConfirmation { revision, result },
    }
}

fn stable() -> StabilityResult {
    StabilityResult::Stable {
        receipt: Box::new([0xAA]),
    }
}

/// A strategy whose every set is a quorum: the Q1 gate must refuse it, because
/// disjoint "quorums" then exist and R1 fails. Used to prove `validate_era`
/// runs at construction, not at the first view change.
struct AnythingQuorums;

impl QuorumStrategy for AnythingQuorums {
    fn is_quorum(&self, _role: Role, _config: &Configuration, _members: &[NodeId]) -> bool {
        true
    }

    fn threshold(&self, _role: Role, _config: &Configuration) -> Option<u64> {
        None
    }
}

// ---------------------------------------------------------------------------
// 1. Provision
// ---------------------------------------------------------------------------

/// Provision constructs exactly the genesis state of §5 and §8.7.2: era 1,
/// view 0, `Void` at slot 1 and `Init` at slot 2 both committed, `accepted ==
/// committed == Slot(2)`, `applied == checkpoint == Slot(0)`, fenced in
/// `Recovering` (the genesis ruling: a fresh node and a reopened node are
/// uniform — fenced until they prove their state current). Nothing about a
/// fresh cluster is special-cased into `Normal`.
#[test]
fn provision_constructs_exactly_the_genesis_state() {
    let replica = provision_volatile();
    let progress = replica.progress();

    assert_eq!(progress.current(), genesis_view());
    assert_eq!(progress.retained(), genesis_view());
    assert_eq!(progress.status(), Status::Recovering);
    assert_eq!(progress.accepted(), Slot(2));
    assert_eq!(progress.committed(), Slot(2));
    assert_eq!(progress.applied(), Slot(0));
    assert_eq!(progress.checkpoint(), Slot(0));
    assert_eq!(progress.revision(), 0);
    assert_eq!(progress.fault(), None);

    // §5 invariant 1: the published accepted frontier is the journal's.
    let view = view_of(&replica);
    assert_eq!(view.accepted(), Some(Slot(2)));
    let void = view.get(Slot(1)).expect("Void at slot 1");
    assert_eq!(void.payload, Payload::System(SystemOperation::Void));
    let init = view.get(Slot(2)).expect("Init at slot 2");
    assert_eq!(
        init.payload,
        Payload::System(SystemOperation::Init { order: order3() })
    );

    // The observation is born holding the genesis snapshot (B1).
    let snapshot = replica.observer().read();
    assert_eq!(snapshot.era, 1);
    assert_eq!(snapshot.view, 0);
    assert_eq!(snapshot.status, Status::Recovering.to_word());
    assert!(!snapshot.faulted);
    assert_eq!(snapshot.accepted, 2);
    assert_eq!(snapshot.committed, 2);
    assert_eq!(snapshot.applied, 0);
    assert_eq!(snapshot.checkpoint, 0);
    assert_eq!(snapshot.revision, 0);
}

/// A node outside the genesis order cannot provision as that cluster.
#[test]
fn provision_refuses_a_non_member() {
    let result = Replica::provision(
        NodeId(9),
        order3(),
        WeightedMajority,
        SegmentedLog::new(),
        Stability::Volatile,
        NO_VIEW_CHANGE,
    );
    assert_eq!(result.unwrap_err(), LifecycleError::NotAMember(NodeId(9)));
}

/// A duplicate or over-cap genesis order is refused by the configuration fold,
/// surfacing through `LifecycleError` — genesis legality is decided at
/// construction, not discovered at the first view change.
#[test]
fn provision_surfaces_configuration_refusals() {
    let duplicate = Replica::provision(
        NodeId(0),
        vec![NodeId(0), NodeId(1), NodeId(1)],
        WeightedMajority,
        SegmentedLog::new(),
        Stability::Volatile,
        NO_VIEW_CHANGE,
    );
    assert_eq!(
        duplicate.unwrap_err(),
        LifecycleError::Configuration(ConfigError::DuplicateNode(NodeId(1)))
    );

    let over_cap: Vec<NodeId> = (0..=16).map(NodeId).collect();
    let too_many = Replica::provision(
        NodeId(0),
        over_cap,
        WeightedMajority,
        SegmentedLog::new(),
        Stability::Volatile,
        NO_VIEW_CHANGE,
    );
    assert_eq!(
        too_many.unwrap_err(),
        LifecycleError::Configuration(ConfigError::MembershipCapExceeded { cap: 16 })
    );
}

/// Q1 at construction: `validate_era` runs on the genesis configuration, so a
/// strategy that admits disjoint quorums is refused before the replica exists.
#[test]
fn provision_runs_the_quorum_gate_on_genesis() {
    let result = Replica::provision(
        NodeId(0),
        order3(),
        AnythingQuorums,
        SegmentedLog::new(),
        Stability::Volatile,
        NO_VIEW_CHANGE,
    );
    assert!(matches!(
        result.unwrap_err(),
        LifecycleError::Quorum(QuorumError::R1Violation { .. })
    ));
}

/// Provisioning over a journal that already holds history is refused: a
/// non-empty journal is evidence of a prior life, and `provision` must not
/// silently overwrite it — that overwrite is the amnesiac voter of §14.2.
#[test]
fn provision_refuses_a_non_empty_journal() {
    let result = Replica::provision(
        NodeId(0),
        order3(),
        WeightedMajority,
        genesis_journal(),
        Stability::Volatile,
        NO_VIEW_CHANGE,
    );
    assert_eq!(result.unwrap_err(), LifecycleError::JournalNotEmpty);
}

// ---------------------------------------------------------------------------
// 2. Reopen
// ---------------------------------------------------------------------------

/// A consistent persisted progress and journal reopens — fenced `Recovering`
/// whatever status was persisted, per §5's boot rule: the pre-failure status
/// is evidence about the past, not authority over the present.
#[test]
fn reopen_restores_evidence_and_fences_regardless_of_persisted_status() {
    let replica = provision_volatile();
    for status in [Status::Normal, Status::ViewChange, Status::Replaying] {
        let persisted = PersistedProgress {
            status,
            ..PersistedProgress::from(replica.progress())
        };
        let reopened = Replica::reopen(
            NodeId(0),
            WeightedMajority,
            genesis_journal(),
            persisted,
            Arc::clone(replica.progress().config()),
            Stability::Volatile,
            NO_VIEW_CHANGE,
        )
        .expect("a consistent reopen succeeds");
        assert_eq!(
            reopened.progress().status(),
            Status::Recovering,
            "persisted {status:?} reopens fenced"
        );
        assert_eq!(reopened.progress().accepted(), Slot(2));
        assert_eq!(reopened.progress().committed(), Slot(2));
        assert_eq!(reopened.progress().fault(), None);
    }
}

/// Persisted `accepted` ahead of the journal's frontier is the two durable
/// records disagreeing about history: refused as `ProgressJournalDivergence`.
#[test]
fn reopen_refuses_progress_journal_divergence() {
    let replica = provision_volatile();
    let persisted = PersistedProgress {
        accepted: Slot(5),
        ..PersistedProgress::from(replica.progress())
    };
    let result = Replica::reopen(
        NodeId(0),
        WeightedMajority,
        genesis_journal(),
        persisted,
        Arc::clone(replica.progress().config()),
        Stability::Volatile,
        NO_VIEW_CHANGE,
    );
    assert_eq!(
        result.unwrap_err(),
        LifecycleError::ProgressJournalDivergence {
            progress: Slot(5),
            journal: Slot(2),
        }
    );
}

/// Faults survive restart — they are part of progress (§5 invariant 5). A
/// persisted fault reopens as a faulted replica that refuses all input, at
/// both the plan and the publish gate.
#[test]
fn reopen_preserves_a_persisted_fault() {
    let replica = provision_volatile();
    let persisted = PersistedProgress {
        fault: Some(Fault::IndeterminatePersistence),
        ..PersistedProgress::from(replica.progress())
    };
    let mut reopened = Replica::reopen(
        NodeId(0),
        WeightedMajority,
        genesis_journal(),
        persisted,
        Arc::clone(replica.progress().config()),
        Stability::Volatile,
        NO_VIEW_CHANGE,
    )
    .expect("a faulted record is still structurally valid");

    assert_eq!(
        reopened.progress().fault(),
        Some(Fault::IndeterminatePersistence)
    );
    assert!(reopened.observer().read().faulted);
    assert_eq!(
        reopened.plan(&tick(1), &view_of(&reopened)).unwrap_err(),
        PlanRejection::Faulted(Fault::IndeterminatePersistence)
    );
    // A stale pre-fault plan is refused at the publish gate too.
    let stale = provision_volatile();
    let planned = stale.plan(&tick(1), &view_of(&stale)).expect("plan");
    assert_eq!(
        reopened.publish(planned).unwrap_err(),
        PublishRejection::Faulted(Fault::IndeterminatePersistence)
    );
}

// ---------------------------------------------------------------------------
// 3. Volatile publication
// ---------------------------------------------------------------------------

/// Volatile mode: `plan` computes and releases nothing; `publish` installs,
/// writes the observation, and releases the effects — exactly once. The
/// observation changes on publish, never on plan (B1): **nothing externally
/// observable is released before publication**. On the genesis-primary
/// fixture (this
/// node IS the genesis primary, holding the complete committed genesis) the
/// first tick is the bootstrap promotion: the promotion announces the committed
/// frontier to the two backups (§13.3); later ticks are bare.
#[test]
fn volatile_publish_releases_exactly_once_and_observes_on_publish_only() {
    let mut replica = provision_volatile();
    let observer = replica.observer();
    let before = observer.read();
    assert_eq!(before.revision, 0);
    assert_eq!(before.status, Status::Recovering.to_word());

    let planned = replica
        .plan(&tick(1), &view_of(&replica))
        .expect("a tick plans");
    assert_eq!(
        observer.read(),
        before,
        "plan publishes nothing: the observation is unchanged"
    );
    assert_eq!(replica.progress().revision(), 0, "plan mutates nothing");

    match replica.publish(planned).expect("publish") {
        PublishOutcome::Published { revision, effects } => {
            assert_eq!(revision, 1, "revision advances by exactly one");
            assert_eq!(
                effects.len(),
                2,
                "the bootstrap promotion announces the committed frontier to both backups"
            );
            assert!(
                effects.iter().all(|effect| matches!(effect,
                    Effect::Send { to, message, .. }
                        if *to != NodeId(0) && message.header.tag == Tag::Commit)),
                "two Commit announcements, one per backup: {effects:?}"
            );
        }
        PublishOutcome::Parked { .. } => panic!("volatile mode never parks"),
    }

    let after = observer.read();
    assert_eq!(after.revision, 1);
    assert_eq!(after.accepted, 2, "frontiers unchanged by a tick");
    assert_eq!(
        after.status,
        Status::Normal.to_word(),
        "the genesis primary promoted"
    );
    assert!(!after.faulted);

    // A later tick is the bare transition the pre-bootstrap pin described: no
    // protocol state moves, no effects release.
    let planned = replica
        .plan(&tick(2), &view_of(&replica))
        .expect("a second tick plans");
    match replica.publish(planned).expect("publish") {
        PublishOutcome::Published { revision, effects } => {
            assert_eq!(revision, 2);
            assert!(effects.is_empty(), "a bare tick releases no effects");
        }
        PublishOutcome::Parked { .. } => panic!("volatile mode never parks"),
    }
}

// ---------------------------------------------------------------------------
// 4. External stability (S2/S3)
// ---------------------------------------------------------------------------

/// The full handshake: publish emits `Persist` and parks; `Stable` completes
/// and releases the parked effects; a duplicate confirmation and a confirmation
/// with nothing outstanding are both rejected; `Failed` leaves the previously
/// published state visible (S3); `Indeterminate` sticky-faults (§5 invariant 5).
#[test]
fn external_stability_parks_confirms_and_faults_three_ways() {
    let mut replica = provision_forced();
    let observer = replica.observer();

    // Publish parks: exactly one effect, the persistence intent; the
    // observation is untouched — publication has not happened yet.
    let planned = replica.plan(&tick(1), &view_of(&replica)).expect("plan");
    match replica.publish(planned).expect("publish parks") {
        PublishOutcome::Parked { revision, effects } => {
            assert_eq!(revision, 0, "the base revision the intent is named by");
            assert_eq!(effects.len(), 1, "Persist and nothing else");
            assert!(
                matches!(&effects[0], Effect::Persist(intent) if intent.revision == 0),
                "the released effect is the persistence intent"
            );
        }
        PublishOutcome::Published { .. } => panic!("external mode must park"),
    }
    assert_eq!(
        observer.read().revision,
        0,
        "a parked transition is not published"
    );

    // A confirmation naming the wrong revision is rejected.
    assert_eq!(
        replica
            .plan(&confirm(7, stable()), &view_of(&replica))
            .unwrap_err(),
        PlanRejection::ConfirmationMismatch {
            expected: 0,
            got: 7
        }
    );

    // Stable completes: the parked candidate publishes, effects release.
    // This first tick is the bootstrap promotion, so the parked
    // effects are the two §13.3 Commit announcements, not none; later ticks
    // park bare.
    let planned = replica
        .plan(&confirm(0, stable()), &view_of(&replica))
        .expect("a matching confirmation plans");
    match replica.publish(planned).expect("confirmation publishes") {
        PublishOutcome::Published { revision, effects } => {
            assert_eq!(revision, 1);
            assert_eq!(
                effects.len(),
                2,
                "the bootstrap promotion announces the committed frontier to both backups"
            );
            assert!(
                effects.iter().all(|effect| matches!(effect,
                    Effect::Send { to, message, .. }
                        if *to != NodeId(0) && message.header.tag == Tag::Commit)),
                "two Commit announcements, one per backup: {effects:?}"
            );
        }
        PublishOutcome::Parked { .. } => panic!("a confirmed transition does not re-park"),
    }
    assert_eq!(observer.read().revision, 1);
    assert_eq!(
        observer.read().status,
        Status::Normal.to_word(),
        "the genesis primary promoted"
    );

    // A duplicate confirmation: nothing is outstanding any more.
    assert_eq!(
        replica
            .plan(&confirm(0, stable()), &view_of(&replica))
            .unwrap_err(),
        PlanRejection::NoTransitionOutstanding
    );

    // Failed (determinate, S3): the candidate is discarded, the previously
    // published state stays visible — frontiers unchanged, no fault.
    let planned = replica.plan(&tick(2), &view_of(&replica)).expect("plan");
    match replica.publish(planned).expect("publish") {
        PublishOutcome::Parked { revision, .. } => assert_eq!(revision, 1),
        PublishOutcome::Published { .. } => panic!("external mode must park"),
    }
    let planned = replica
        .plan(
            &confirm(
                1,
                StabilityResult::Failed {
                    reason: Box::new(*b"disk full"),
                },
            ),
            &view_of(&replica),
        )
        .expect("a determinate failure still resolves the interval");
    match replica.publish(planned).expect("resolution publishes") {
        PublishOutcome::Published { revision, effects } => {
            assert_eq!(revision, 2);
            assert!(effects.is_empty());
        }
        PublishOutcome::Parked { .. } => panic!("a resolved interval does not re-park"),
    }
    let visible = observer.read();
    assert_eq!(visible.accepted, 2, "the failed candidate was discarded");
    assert_eq!(visible.committed, 2);
    assert!(
        !visible.faulted,
        "a determinate failure is NOT a fault (S3)"
    );
    assert_eq!(replica.progress().fault(), None);

    // A confirmation with nothing outstanding is rejected — the volatile
    // replica has never parked anything.
    let volatile = provision_volatile();
    assert_eq!(
        volatile
            .plan(&confirm(0, stable()), &view_of(&volatile))
            .unwrap_err(),
        PlanRejection::NoTransitionOutstanding
    );

    // Indeterminate (S3): the node sticky-faults and the observation says so.
    let planned = replica.plan(&tick(3), &view_of(&replica)).expect("plan");
    match replica.publish(planned).expect("publish") {
        PublishOutcome::Parked { revision, .. } => assert_eq!(revision, 2),
        PublishOutcome::Published { .. } => panic!("external mode must park"),
    }
    let planned = replica
        .plan(
            &confirm(
                2,
                StabilityResult::Indeterminate {
                    reason: Box::new(*b"timeout"),
                },
            ),
            &view_of(&replica),
        )
        .expect("an indeterminate result plans the fault transition");
    replica.publish(planned).expect("the fault publishes");
    assert_eq!(
        replica.progress().fault(),
        Some(Fault::IndeterminatePersistence)
    );
    assert!(observer.read().faulted, "the fault is observable");
    assert_eq!(
        replica.plan(&tick(4), &view_of(&replica)).unwrap_err(),
        PlanRejection::Faulted(Fault::IndeterminatePersistence)
    );
}

// ---------------------------------------------------------------------------
// 5. One outstanding transition (§12)
// ---------------------------------------------------------------------------

/// While a confirmation is pending, no second transition may be planned — the
/// §12 serialized interval admits exactly one outstanding transition. After
/// the confirmation lands, planning resumes.
#[test]
fn exactly_one_transition_is_outstanding() {
    let mut replica = provision_forced();

    let planned = replica.plan(&tick(1), &view_of(&replica)).expect("plan");
    match replica.publish(planned).expect("publish parks") {
        PublishOutcome::Parked { .. } => {}
        PublishOutcome::Published { .. } => panic!("external mode must park"),
    }

    // Every non-confirmation input is refused while the interval is open —
    // the outstanding check precedes dispatch, so even an otherwise
    // unsupported input reports the real reason.
    assert_eq!(
        replica.plan(&tick(2), &view_of(&replica)).unwrap_err(),
        PlanRejection::TransitionOutstanding
    );
    let peer = TimedInput {
        at: Tick(2),
        event: Input::Peer {
            from: NodeId(0),
            message: Message {
                header: Header {
                    tag: Tag::Commit,
                    view: genesis_view(),
                    slot: Slot(2),
                },
                body: Body::Commit { committed: Slot(2) },
            },
        },
    };
    assert_eq!(
        replica.plan(&peer, &view_of(&replica)).unwrap_err(),
        PlanRejection::TransitionOutstanding
    );

    // The confirmation lands; planning resumes against the new revision.
    let planned = replica
        .plan(&confirm(0, stable()), &view_of(&replica))
        .expect("confirmation plans");
    replica.publish(planned).expect("publish");
    replica
        .plan(&tick(3), &view_of(&replica))
        .expect("planning resumes after the interval closes");
}

// ---------------------------------------------------------------------------
// 6. Revision discipline (§12)
// ---------------------------------------------------------------------------

/// A plan is publishable only against the state it was computed from: a stale
/// plan (revision behind) and a double publish are both `RevisionMismatch`.
#[test]
fn stale_and_double_publishes_are_revision_mismatches() {
    let mut replica = provision_volatile();

    let first = replica.plan(&tick(1), &view_of(&replica)).expect("plan");
    let second = replica.plan(&tick(2), &view_of(&replica)).expect("plan");
    let republished = first.clone();

    replica.publish(first).expect("first publish");
    assert_eq!(
        replica.publish(second).unwrap_err(),
        PublishRejection::RevisionMismatch {
            expected: 1,
            got: 0
        },
        "a plan computed against revision 0 is stale once revision 1 is published"
    );
    assert_eq!(
        replica.publish(republished).unwrap_err(),
        PublishRejection::RevisionMismatch {
            expected: 1,
            got: 0
        },
        "the same transition cannot publish twice"
    );
    assert_eq!(replica.observer().read().revision, 1);
}

// ---------------------------------------------------------------------------
// 7. Fault refusal — exhaustive over `Input`
// ---------------------------------------------------------------------------

/// A faulted node refuses EVERY input, ticks and confirmations included (§5
/// invariant 5). The match below is deliberately exhaustive with no wildcard:
/// a future `Input` variant fails to compile this test until a human decides
/// what faulted refusal means for it.
#[test]
fn a_faulted_replica_refuses_every_input_variant() {
    let mut replica = provision_forced();

    // Fault the node via the honest path: park a transition, then report an
    // indeterminate persistence result (S3).
    let planned = replica.plan(&tick(1), &view_of(&replica)).expect("plan");
    match replica.publish(planned).expect("publish parks") {
        PublishOutcome::Parked { .. } => {}
        PublishOutcome::Published { .. } => panic!("external mode must park"),
    }
    let planned = replica
        .plan(
            &confirm(
                0,
                StabilityResult::Indeterminate {
                    reason: Box::new(*b"unknown"),
                },
            ),
            &view_of(&replica),
        )
        .expect("the fault transition plans");
    replica.publish(planned).expect("the fault publishes");
    assert!(replica.observer().read().faulted);

    let message = Message {
        header: Header {
            tag: Tag::Prepare,
            view: genesis_view(),
            slot: Slot(3),
        },
        body: Body::Prepare {
            entry: LogEntry {
                slot: Slot(3),
                era: Era(1),
                payload: Payload::Client {
                    client: ClientId(1),
                    request: RequestNumber(1),
                    payload: Box::new([1]),
                },
            },
            committed: Slot(2),
        },
    };
    let inputs: Vec<Input> = vec![
        Input::Peer {
            from: NodeId(0),
            message,
        },
        Input::Client {
            client: ClientId(7),
            request: RequestNumber(1),
            payload: Box::new([1, 2]),
        },
        Input::Tick,
        Input::Recover,
        Input::StabilityConfirmation {
            revision: 0,
            result: stable(),
        },
        Input::Applied {
            slot: Slot(2),
            result: Box::new([1]),
        },
        Input::Checkpointed { through: Slot(2) },
        Input::Reconfigure {
            op: SystemOperation::Double,
            pivot: None,
        },
        Input::AdminForceView { view: View(9) },
    ];

    for input in inputs {
        // Exhaustive on purpose: NO wildcard arm.
        let kind = match &input {
            Input::Peer { message, .. } => InputKind::PeerMessage {
                tag: message.header.tag,
                slot: message.header.slot,
            },
            Input::Client { .. } => InputKind::ClientRequest,
            Input::Tick => InputKind::Tick,
            Input::Recover => InputKind::Recovery,
            Input::StabilityConfirmation { .. } => InputKind::StabilityConfirmed,
            Input::Applied { .. } => InputKind::Applied,
            Input::Checkpointed { .. } => InputKind::Checkpointed,
            Input::Reconfigure { .. } => InputKind::Reconfiguration,
            Input::AdminForceView { .. } => InputKind::Admin,
        };
        assert_eq!(
            replica
                .plan(
                    &TimedInput {
                        at: Tick(9),
                        event: input
                    },
                    &view_of(&replica)
                )
                .unwrap_err(),
            PlanRejection::Faulted(Fault::IndeterminatePersistence),
            "{kind:?} must be refused by a faulted node"
        );
    }
}

// ---------------------------------------------------------------------------
// 8. `invariant::legal` is on the path
// ---------------------------------------------------------------------------

/// The closed checker runs before every publish and is not bypassed: a
/// candidate that violates a frontier rule (here: `committed` regresses below
/// the published frontier) is DISCARDED and the node faults with
/// `Fault::IllegalTransition` — never repaired, never installed.
///
/// No honest planner output can violate the chain — `Progress` transitions
/// validate their results — so the candidate is injected through the
/// documented test hook [`vrr::replica::PlannedTransition::substitute_candidate_for_gate_testing`].
/// That is the point of the test: the hook can only smuggle a bad candidate
/// PAST the planner, and the publish gate still stops it, which proves the
/// gate and not the planner is what stands between a bad candidate and the
/// observation.
#[test]
fn an_illegal_candidate_is_discarded_and_faults_the_node() {
    let mut replica = provision_volatile();
    let observer = replica.observer();

    let planned = replica.plan(&tick(1), &view_of(&replica)).expect("plan");

    // A candidate whose committed frontier regresses: structurally valid
    // (the chain holds on it) but illegal against the published state.
    let regressed = Progress::reconstitute(
        genesis_view(),
        genesis_view(),
        Status::Recovering,
        Slot(2),
        Slot(1), // committed behind the published Slot(2): rule 1
        Slot(0),
        Slot(0),
        1,
        Arc::clone(replica.progress().config()),
        None,
    )
    .expect("a structurally valid candidate");
    let planned = planned.substitute_candidate_for_gate_testing(regressed);

    assert_eq!(
        replica.publish(planned).unwrap_err(),
        PublishRejection::IllegalCandidate(Fault::IllegalTransition)
    );

    // The candidate was discarded: the published frontiers are the old ones,
    // and the node now faults every input with the fault the gate reported.
    assert_eq!(replica.progress().committed(), Slot(2));
    assert_eq!(replica.progress().fault(), Some(Fault::IllegalTransition));
    let snapshot = observer.read();
    assert!(snapshot.faulted, "the fault is observable");
    assert_eq!(
        snapshot.committed, 2,
        "the illegal candidate never published"
    );
    assert_eq!(
        replica.plan(&tick(2), &view_of(&replica)).unwrap_err(),
        PlanRejection::Faulted(Fault::IllegalTransition)
    );
}

// ---------------------------------------------------------------------------
// 9. SANS-I/O, mechanically enforced
// ---------------------------------------------------------------------------

/// The three modules of the pipeline — `replica`, `effects`, `message` —
/// contain no clock read, no network, no
/// filesystem and no thread spawn. This is a gate, not a smoke test: a future
/// edit that reaches for the wall clock or a socket fails the build here.
#[test]
fn the_pipeline_reads_no_clock_and_performs_no_io() {
    let sources: [(&str, &str); 3] = [
        ("replica", include_str!("../src/replica.rs")),
        ("effects", include_str!("../src/effects.rs")),
        ("message", include_str!("../src/message.rs")),
    ];
    for (name, source) in sources {
        for banned in [
            "SystemTime",
            "Instant",
            "std::net",
            "std::fs",
            "thread::spawn",
        ] {
            assert!(
                !source.contains(banned),
                "src/{name}.rs must not mention {banned} (S4, SANS-I/O)"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 10. Message wire round trips
// ---------------------------------------------------------------------------

fn entry(slot: u64) -> LogEntry {
    LogEntry {
        slot: Slot(slot),
        era: Era(1),
        payload: Payload::Client {
            client: ClientId(1),
            request: RequestNumber(slot),
            payload: Box::new([0xAB, slot as u8]),
        },
    }
}

fn era_proof() -> EraProof {
    EraProof {
        op: SystemOperation::Init { order: order3() },
        committed_at: Slot(2),
    }
}

fn message(tag: Tag, slot: Slot, body: Body) -> Message {
    Message {
        header: Header {
            tag,
            view: genesis_view(),
            slot,
        },
        body,
    }
}

/// Every `Body` variant through `Pack`/`Unpack` with an exact `packed_len`
/// (W3: the §13.1 suffix budget sums these), and every body's header slot
/// satisfying its `header_slot_role` — Operation tags name a real slot,
/// Absent tags carry the sentinel, Frontier tags admit every value.
#[test]
fn every_body_round_trips_with_exact_length_and_a_legal_header_slot() {
    let cases: Vec<Message> = vec![
        message(
            Tag::Request,
            Slot(0),
            Body::Request {
                client: ClientId(3),
                request: RequestNumber(4),
                payload: Box::new([9, 8, 7]),
            },
        ),
        message(
            Tag::Prepare,
            Slot(3),
            Body::Prepare {
                entry: entry(3),
                committed: Slot(2),
            },
        ),
        message(Tag::PrepareOk, Slot(3), Body::PrepareOk {}),
        message(Tag::Commit, Slot(2), Body::Commit { committed: Slot(2) }),
        message(Tag::StartViewChange, Slot(0), Body::StartViewChange {}),
        message(
            Tag::DoViewChange,
            Slot(5),
            Body::DoViewChange {
                retained: genesis_view(),
                accepted: Slot(5),
                committed: Slot(2),
                suffix: vec![entry(4), entry(5)],
                client_table: Vec::new(),
                evidence: EvidenceKind::Ordinary,
                era_proof: era_proof(),
            },
        ),
        message(
            Tag::StartView,
            Slot(5),
            Body::StartView {
                suffix: vec![entry(4), entry(5)],
                accepted: Slot(5),
                committed: Slot(2),
                client_table: Vec::new(),
                era_proof: era_proof(),
            },
        ),
        message(Tag::PlannedViewChange, Slot(0), Body::PlannedViewChange {}),
        message(Tag::Recovery, Slot(0), Body::Recovery { nonce: Tick(99) }),
        message(
            Tag::RecoveryResponse,
            Slot(5),
            Body::RecoveryResponse {
                nonce: Tick(99),
                accepted: Slot(5),
                committed: Slot(2),
                suffix: Some(vec![entry(4), entry(5)]),
            },
        ),
        message(
            Tag::RecoveryResponse,
            Slot(5),
            Body::RecoveryResponse {
                nonce: Tick(99),
                accepted: Slot(5),
                committed: Slot(2),
                suffix: None,
            },
        ),
        message(Tag::GetState, Slot(2), Body::GetState { from: Slot(3) }),
        message(
            Tag::NewState,
            Slot(5),
            Body::NewState {
                entries: vec![entry(3), entry(4), entry(5)],
                through: Slot(5),
                committed: Slot(2),
            },
        ),
        message(
            Tag::Reply,
            Slot(0),
            Body::Reply {
                client: ClientId(3),
                request: RequestNumber(4),
                result: Box::new([1]),
            },
        ),
    ];

    // One case per tag: the coverage is exhaustive by construction.
    assert_eq!(cases.len(), 14, "13 tags plus the RecoveryResponse option");

    for case in &cases {
        assert_eq!(
            case.body.tag(),
            case.header.tag,
            "body and header agree on the kind"
        );
        assert!(
            header_slot_role(case.header.tag).admits(case.header.slot),
            "{:?} at {:?} satisfies its header-slot role",
            case.header.tag,
            case.header.slot,
        );

        let mut buf = vec![0u8; case.packed_len()];
        let written = case.pack_into(&mut buf).expect("pack");
        assert_eq!(written, case.packed_len(), "packed_len is normative (W3)");
        let decoded = Message::unpack_from(&buf).expect("round trip");
        assert_eq!(&decoded, case, "{:?} round trip", case.header.tag);
    }
}

/// A body whose discriminant disagrees with the header tag is not a message:
/// the wire carries the kind twice and the decode refuses a disagreement
/// rather than guessing which field lied.
#[test]
fn a_body_header_tag_mismatch_is_malformed() {
    let mismatched = Message {
        header: Header {
            tag: Tag::Commit,
            view: genesis_view(),
            slot: Slot(2),
        },
        body: Body::StartViewChange {},
    };
    let mut buf = vec![0u8; mismatched.packed_len()];
    mismatched.pack_into(&mut buf).expect("pack");
    assert_eq!(
        Message::unpack_from(&buf).unwrap_err(),
        UnpackError::Malformed(Malformed::OutOfDomain)
    );
}

/// Discriminant 0 is reserved for bodies exactly as for tags: an all-zero
/// buffer is malformed, never a valid message.
#[test]
fn body_discriminant_zero_is_reserved() {
    let header = Header {
        tag: Tag::Commit,
        view: genesis_view(),
        slot: Slot(2),
    };
    let mut buf = vec![0u8; header.packed_len() + 1];
    header
        .pack_into(&mut buf[..header.packed_len()])
        .expect("pack header");
    // buf's last byte is 0: the reserved body discriminant.
    assert_eq!(
        Message::unpack_from(&buf).unwrap_err(),
        UnpackError::Malformed(Malformed::OutOfDomain)
    );
}
