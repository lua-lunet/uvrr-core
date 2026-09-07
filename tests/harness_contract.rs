//! Contract for the scripted step-through harness (`tests/harness/mod.rs`).
//!
//! The harness is loaded infrastructure: a harness that can silently pass is
//! worse than none. The properties pinned here, each of which the protocol
//! suites are entitled to assume:
//!
//! 1. determinism — the same script run twice produces byte-identical step
//!    traces;
//! 2. force-feed — `inject` reaches a node with exactly the outcome of a
//!    queued delivery of the same datagram, and a legitimate
//!    `Prepare` from the view-0 primary is adopted and accepted by a
//!    `Recovering` backup while a proposal to a non-primary is the
//!    named `NotPrimary` refusal;
//! 3. partition accounting — datagrams sent across a partition are held,
//!    counted, and deliverable after `heal`; an explicit drop is recorded;
//! 4. crash/restart — deliveries to a down node are recorded undeliverable;
//!    a reopen yields the recorded `Recovering` state and
//!    `restart_with` restores the recorded disk under the boot rule;
//! 5. fault declaration discipline — an undeclared fault fails the step with
//!    the full trace; the same script with `expect_fault` passes;
//! 6. the safety checker is not vacuous — one planted violation per rule,
//!    each flagged with its own variant;
//! 7. tick monotonicity — the harness clock never goes backwards and every
//!    `TimedInput` carries the current value.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use vrr::configuration::{EraTable, INIT_SLOT, SystemOperation, VOID_SLOT};
use vrr::effects::{Effect, Stability, StabilityResult};
use vrr::ids::{Era, Fault, NodeId, OperationId, Slot, Tick, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, Message};
use vrr::progress::{ProgressSnapshot, Status};
use vrr::replica::PlanRefusal;
use vrr::wire::{Header, Tag};

use harness::{Harness, NodeEvidence, SafetyViolation, StepOutcome, check_cluster_safety};

#[path = "harness/mod.rs"]
mod harness;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn n(id: u32) -> NodeId {
    NodeId(id)
}

fn genesis_view() -> ViewId {
    ViewId {
        era: Era(1),
        view: View::INITIAL,
    }
}

fn prepare(slot: u64) -> Message {
    Message {
        header: Header {
            tag: Tag::Prepare,
            view: genesis_view(),
            slot: Slot(slot),
        },
        body: Body::Prepare {
            entry: LogEntry {
                slot: Slot(slot),
                era: Era(1),
                payload: Payload::Operation {
                    id: OperationId { msb: 0, lsb: slot },
                    payload: Box::new([0xAB]),
                },
            },
            committed: Slot(2),
        },
    }
}

fn prepare_ok(slot: u64) -> Message {
    Message {
        header: Header {
            tag: Tag::PrepareOk,
            view: genesis_view(),
            slot: Slot(slot),
        },
        body: Body::PrepareOk {},
    }
}

fn commit(committed: u64) -> Message {
    Message {
        header: Header {
            tag: Tag::Commit,
            view: genesis_view(),
            slot: Slot(committed),
        },
        body: Body::Commit {
            committed: Slot(committed),
        },
    }
}

fn indeterminate() -> StabilityResult {
    StabilityResult::Indeterminate {
        reason: Box::new(*b"timeout"),
    }
}

/// The genesis history exactly as `provision` installs it (§8.7.2's fixed
/// ordinals), for planting committed-history evidence.
fn genesis_log(order: &[u32]) -> Vec<LogEntry> {
    vec![
        LogEntry {
            slot: VOID_SLOT,
            era: Era::INITIAL,
            payload: Payload::System(SystemOperation::Void),
        },
        LogEntry {
            slot: INIT_SLOT,
            era: Era(1),
            payload: Payload::System(SystemOperation::Init {
                order: order.iter().map(|&id| NodeId(id)).collect(),
            }),
        },
    ]
}

fn era_table(order: &[u32]) -> Arc<EraTable> {
    let order: Vec<NodeId> = order.iter().map(|&id| NodeId(id)).collect();
    Arc::new(
        EraTable::genesis()
            .extend(&SystemOperation::Void, VOID_SLOT)
            .and_then(|table| table.extend(&SystemOperation::Init { order }, INIT_SLOT))
            .expect("a legal genesis fold"),
    )
}

/// A fabricated observation. Only test 6 plants these; the harness's own
/// evidence always comes from real replicas.
fn snapshot(
    status: Status,
    era: u32,
    view: u32,
    checkpoint: u64,
    applied: u64,
    committed: u64,
    accepted: u64,
) -> ProgressSnapshot {
    ProgressSnapshot {
        era,
        view,
        retained_era: era,
        retained_view: view,
        status: status.to_word(),
        faulted: false,
        accepted,
        committed,
        applied,
        checkpoint,
        revision: 0,
    }
}

fn evidence(
    id: u32,
    snap: ProgressSnapshot,
    order: &[u32],
    committed: Vec<LogEntry>,
    applied: Vec<(u64, u8)>,
) -> NodeEvidence {
    NodeEvidence {
        id: NodeId(id),
        snapshot: snap,
        config: era_table(order),
        committed,
        applied: applied
            .into_iter()
            .map(|(slot, byte)| (Slot(slot), Box::from([byte])))
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// 1. Determinism
// ---------------------------------------------------------------------------

/// A script exercising every harness facility except external stability
/// (which has its own test below).
fn run_script() -> Harness {
    let mut h = Harness::provision(3);
    h.send(n(0), n(1), prepare(3));
    h.send(n(0), n(2), prepare(3));
    h.deliver_next();
    h.inject(n(0), n(2), commit(2));
    h.propose(n(0), OperationId { msb: 0, lsb: 7 }, b"set x=1");
    h.tick_all();
    h.partition(vec![n(0)], vec![n(1), n(2)]);
    h.send(n(2), n(0), prepare_ok(3));
    h.send(n(1), n(2), prepare_ok(3));
    h.heal();
    h.deliver_all();
    h.crash(n(1));
    h.tick(n(0));
    h.assert_safety();
    h
}

/// The script is the only source of nondeterminism: run twice, the step
/// traces are byte-identical.
#[test]
fn the_same_script_produces_byte_identical_traces() {
    let mut harness = run_script();
    // The script commits nothing (the fabricated slot 3 was never proposed
    // by the real primary, so its PrepareOks are named drops), so there is
    // nothing to execute and the records are empty. Asserted, not assumed.
    assert!(harness.execute_apply_effects(n(0)).is_empty());
    assert!(harness.applied(n(0)).is_empty());
    assert!(harness.boundary_events().is_empty());

    let first = harness.trace_dump();
    let second = run_script().trace_dump();
    assert!(!first.is_empty(), "the script produced steps");
    assert_eq!(first, second, "same script, same trace, every run");
}

// ---------------------------------------------------------------------------
// 2. Force-feed
// ---------------------------------------------------------------------------

/// `inject` bypasses the network queues but reaches the node through the same
/// plan/publish path as a queued delivery: identical input, identical
/// outcome. The normal-operation handlers are live: a legitimate `Prepare` from
/// the view-0 primary is adopted and accepted by a `Recovering` backup
/// (VRR-2012 §4), which answers `PrepareOk`.
#[test]
fn inject_reaches_the_node_exactly_as_a_queued_delivery() {
    let expected = StepOutcome::Published {
        revision: 1,
        effects: vec![Effect::Send {
            to: n(0),
            era: Era(1),
            message: prepare_ok(3),
        }],
    };

    let mut injected = Harness::provision(3);
    let via_inject = injected.inject(n(0), n(1), prepare(3));
    assert_eq!(via_inject, expected);

    let mut queued = Harness::provision(3);
    queued.send(n(0), n(1), prepare(3));
    let delivery = queued.deliver_next().expect("one queued datagram");
    assert_eq!(delivery.from, n(0));
    assert_eq!(delivery.to, n(1));
    assert_eq!(delivery.outcome, expected);
    assert_eq!(
        via_inject, delivery.outcome,
        "force-feed and queued delivery agree"
    );

    // A proposal to a non-primary is the named `NotPrimary` refusal,
    // carrying the redirection information (§13.4's convergence hint):
    // the backup adopted view 0 above and is Normal, but the primary of
    // view 0 is n0.
    let refusal = injected.propose(n(1), OperationId { msb: 0, lsb: 9 }, b"op");
    assert_eq!(
        refusal,
        StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
            view: ViewId {
                era: Era(1),
                view: View(0),
            },
            primary: Some(n(0)),
        })
    );
}

// ---------------------------------------------------------------------------
// 3. Partition accounting
// ---------------------------------------------------------------------------

/// Datagrams crossing the partition are held — not dropped — counted, and
/// deliverable after `heal`. An explicit drop is recorded in the trace.
#[test]
fn partition_holds_counts_and_releases_on_heal() {
    let mut h = Harness::provision(3);
    h.partition(vec![n(0)], vec![n(1), n(2)]);

    h.send(n(0), n(1), prepare(3)); // crosses: held
    h.send(n(1), n(0), prepare_ok(3)); // crosses: held
    h.send(n(1), n(2), prepare_ok(3)); // inside one side: queued
    assert_eq!(h.held_len(), 2);
    assert_eq!(h.queued_len(), 1);
    assert_eq!(h.held_summary().len(), 2);

    assert_eq!(
        h.deliver_all().len(),
        1,
        "only the intra-partition datagram moves"
    );
    assert_eq!(h.queued_len(), 0);

    h.heal();
    assert_eq!(h.held_len(), 0);
    assert_eq!(
        h.queued_len(),
        2,
        "held datagrams are requeued, not dropped"
    );

    let delivery = h.deliver_to(n(0)).expect("the 1->0 datagram survived");
    assert_eq!(delivery.from, n(1));
    assert_eq!(
        h.deliver_all().len(),
        2,
        "the requeued Prepare is live: it is accepted, and its PrepareOk is delivered too"
    );

    // An explicit drop is a script decision and the trace says so.
    h.partition(vec![n(0)], vec![n(1)]);
    h.send(n(0), n(1), prepare(4));
    assert_eq!(h.held_len(), 1);
    assert_eq!(h.drop_held(), 1);
    assert_eq!(h.held_len(), 0);
    assert_eq!(h.dropped_count(), 1);
    assert!(
        h.trace_dump().contains("drop"),
        "an explicit drop is recorded in the trace"
    );
}

// ---------------------------------------------------------------------------
// 4. Crash/restart
// ---------------------------------------------------------------------------

/// A crashed node's volatile state dies with it: deliveries to it are
/// recorded undeliverable. `restart_with` reopens the recorded disk,
/// preserving the published record under the boot rule (fenced
/// `Recovering` regardless).
#[test]
fn crash_makes_deliveries_undeliverable_and_restart_restores() {
    let mut h = Harness::provision(3);
    h.tick(n(0)); // revision 1
    h.tick(n(0)); // revision 2
    h.crash(n(0));
    h.crash(n(1));
    assert!(!h.is_up(n(0)));

    h.send(n(2), n(1), prepare(3));
    let delivery = h.deliver_next().expect("the datagram was queued");
    assert_eq!(
        delivery.outcome,
        StepOutcome::NodeDown,
        "a delivery to a down node is undeliverable, not lost silently"
    );
    assert_eq!(h.undeliverable_count(), 1);

    // With the recorded disk: the published record survives; the boot rule
    // still fences (§5: evidence about the past, not authority).
    h.restart_with(n(0)).expect("reopen from the recorded disk");
    let s = h.snapshot(n(0)).expect("node 0 is up");
    assert_eq!(s.revision, 2, "the published revision survived the crash");
    assert_eq!(s.status, Status::Recovering.to_word());
    assert_eq!(s.accepted, 2);
    assert_eq!(s.committed, 2);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 5. Fault declaration discipline
// ---------------------------------------------------------------------------

/// The DoD's core requirement: an illegal transition forces the node crashed,
/// and an *undeclared* forcing is a loud harness failure carrying the full
/// step trace. The same script with `expect_fault` declared passes.
#[test]
fn an_undeclared_fault_fails_loudly_and_a_declared_fault_passes() {
    let mut undeclared = Harness::with_stability(3, Stability::Forced);
    let result = catch_unwind(AssertUnwindSafe(|| {
        undeclared.tick(n(0)); // parks behind its Persist intent
        undeclared.confirm(n(0), indeterminate()); // S3: sticky fault
    }));
    let payload = result.expect_err("an undeclared fault must fail the step");
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .expect("the harness panics with a message");
    assert!(
        message.contains("undeclared fault"),
        "the failure names the cause: {message}"
    );
    assert!(
        message.contains("step trace"),
        "the failure carries the step trace: {message}"
    );

    // Declared: the gate observes the fault, consumes the declaration, and
    // the script continues.
    let mut declared = Harness::with_stability(3, Stability::Forced);
    declared.expect_fault(n(0));
    declared.tick(n(0));
    declared.confirm(n(0), indeterminate());
    let s = declared.snapshot(n(0)).expect("node 0 is up");
    assert!(s.faulted, "the fault is observable");

    // Sticky and accounted: a later input is refused with the fault, and the
    // gate does not fire twice for the same fault.
    let outcome = declared.tick(n(0));
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Faulted(Fault::IndeterminatePersistence))
    );
}

// ---------------------------------------------------------------------------
// 6. The safety checker is not vacuous
// ---------------------------------------------------------------------------

/// One planted violation per rule, each through test-only construction of
/// [`NodeEvidence`]. A checker that has never failed is not a checker.
#[test]
fn the_safety_checker_catches_planted_violations() {
    // A sound cluster passes — the positive control, through the harness
    // itself rather than planted evidence.
    let mut h = Harness::provision(3);
    h.tick_all();
    h.assert_safety();

    // Frontier sanity: applied ahead of committed breaks the chain.
    let planted = vec![evidence(
        0,
        snapshot(Status::Recovering, 1, 0, 0, 2, 1, 2),
        &[0, 1, 2],
        vec![],
        vec![],
    )];
    assert_eq!(
        check_cluster_safety(&planted),
        Err(SafetyViolation::FrontierChain {
            node: n(0),
            checkpoint: 0,
            applied: 2,
            committed: 1,
            accepted: 2,
        })
    );

    // Single primary per view: two nodes Normal in the same (era, view),
    // each primary under its own divergent membership.
    let planted = vec![
        evidence(
            0,
            snapshot(Status::Normal, 1, 0, 0, 0, 0, 0),
            &[0, 1, 2],
            vec![],
            vec![],
        ),
        evidence(
            1,
            snapshot(Status::Normal, 1, 0, 0, 0, 0, 0),
            &[1, 0, 2],
            vec![],
            vec![],
        ),
    ];
    assert_eq!(
        check_cluster_safety(&planted),
        Err(SafetyViolation::DualPrimary {
            view: genesis_view(),
            a: n(0),
            b: n(1),
        })
    );

    // Committed-prefix agreement: the histories differ at slot 2.
    let planted = vec![
        evidence(
            0,
            snapshot(Status::Recovering, 1, 0, 0, 0, 2, 2),
            &[0, 1, 2],
            genesis_log(&[0, 1, 2]),
            vec![],
        ),
        evidence(
            1,
            snapshot(Status::Recovering, 1, 0, 0, 0, 2, 2),
            &[0, 1, 2],
            genesis_log(&[0, 1, 3]),
            vec![],
        ),
    ];
    assert_eq!(
        check_cluster_safety(&planted),
        Err(SafetyViolation::CommittedDivergence {
            a: n(0),
            b: n(1),
            slot: Slot(2),
        })
    );

    // Applied agreement: two payloads at the same slot.
    let planted = vec![
        evidence(
            0,
            snapshot(Status::Recovering, 1, 0, 0, 0, 2, 2),
            &[0, 1, 2],
            vec![],
            vec![(3, 0xAA)],
        ),
        evidence(
            1,
            snapshot(Status::Recovering, 1, 0, 0, 0, 2, 2),
            &[0, 1, 2],
            vec![],
            vec![(3, 0xBB)],
        ),
    ];
    assert_eq!(
        check_cluster_safety(&planted),
        Err(SafetyViolation::AppliedConflict {
            slot: Slot(3),
            a: n(0),
            b: n(1),
        })
    );

    // Applied contiguity: slot 4 was never applied.
    let planted = vec![evidence(
        0,
        snapshot(Status::Recovering, 1, 0, 0, 0, 2, 2),
        &[0, 1, 2],
        vec![],
        vec![(3, 0xAA), (5, 0xBB)],
    )];
    assert_eq!(
        check_cluster_safety(&planted),
        Err(SafetyViolation::AppliedGap {
            node: n(0),
            expected: Slot(4),
            got: Slot(5),
        })
    );

    // The same shapes, sound: identical histories and applied records pass.
    let sound = vec![
        evidence(
            0,
            snapshot(Status::Recovering, 1, 0, 0, 0, 2, 2),
            &[0, 1, 2],
            genesis_log(&[0, 1, 2]),
            vec![(3, 0xAA)],
        ),
        evidence(
            1,
            snapshot(Status::Recovering, 1, 0, 0, 0, 2, 2),
            &[0, 1, 2],
            genesis_log(&[0, 1, 2]),
            vec![(3, 0xAA)],
        ),
    ];
    assert!(check_cluster_safety(&sound).is_ok());
}

// ---------------------------------------------------------------------------
// 7. Tick monotonicity
// ---------------------------------------------------------------------------

/// The harness clock never goes backwards; `tick`/`tick_all` advance it and
/// every step the harness records carries the current value.
#[test]
fn the_harness_clock_is_monotone_and_carried_by_every_step() {
    let mut h = Harness::provision(3);
    assert_eq!(h.now(), Tick(0));

    h.tick(n(0));
    assert_eq!(h.now(), Tick(1));
    h.tick_all();
    assert_eq!(h.now(), Tick(2), "tick_all advances the clock once");
    h.inject(n(0), n(1), prepare(3));
    assert_eq!(h.now(), Tick(2), "only ticks advance the clock");

    let ticks: Vec<u64> = h
        .step_trace()
        .iter()
        .map(|line| {
            line.split(" t=")
                .nth(1)
                .and_then(|rest| rest.split(' ').next())
                .and_then(|word| word.parse::<u64>().ok())
                .unwrap_or_else(|| panic!("every trace line carries the clock: {line}"))
        })
        .collect();
    assert!(
        ticks.windows(2).all(|pair| pair[0] <= pair[1]),
        "the clock never goes backwards: {ticks:?}"
    );
    assert_eq!(
        ticks.last(),
        Some(&2),
        "the last step carried the current tick"
    );
}
