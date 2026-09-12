//! The leader side of plan/apply (`docs/weighted-reconfiguration-solver.md`):
//! the verdict, the plan-execution machine, the drift abort, the admin-first
//! dual-queue rule, and the leader-crash discard — over the in-memory harness
//! (no real UDP, no PAXE packing at this layer).

mod harness;

use harness::{Harness, StepOutcome};
use vrr::configuration::{Member, SystemOperation, Weight};
use vrr::effects::{Effect, PlanVerdict};
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use vrr::journal::Payload;
use vrr::observe::Diagnostic;
use vrr::plan::{Plan, PlanRejection};

fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// An operation identity for the scripts: the host assigns it, the core
/// carries it opaque (§11.1, B2).
fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

fn member(id: u32, weight: u32) -> Member {
    Member {
        node: NodeId(id),
        weight: Weight(weight),
    }
}

/// A three-node unit cluster whose view-change machinery is inert: the
/// era-boundary view changes these scripts drive are the host's say-so
/// (§14.2), not timeouts.
fn cluster() -> Harness {
    Harness::provision(3)
}

/// Bootstraps the cluster: the genesis primary promotes itself and both
/// backups adopt view (1, 0) from the promotion's `Commit` announcement
/// (§13.3).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    quiesce(h);
    h.assert_safety();
}

/// Delivers everything until the network is empty: proposals, cascades and
/// announcements are all ordinary steps, and a committed cascade may queue
/// more.
fn quiesce(h: &mut Harness) {
    while h.queued_len() > 0 {
        h.deliver_all();
    }
}

/// The two-era plan every execution script runs: join node 3 as a learner
/// (era 2), promote it (era 3). Both folds are legal from the genesis
/// `(1, 1, 1)`, and the learner's weight 0 keeps the era-2 voter succession
/// intact — view 3 selects the leader again, so one §14.2 forced change
/// carries the boundary while the machine stays armed at it.
fn join_then_promote() -> Plan {
    Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1)],
        steps: vec![
            vec![SystemOperation::Join {
                node: n(3),
                position: 3,
            }],
            vec![SystemOperation::Increment(n(3))],
        ],
    }
}

/// The view of the era the plan's first step establishes that retains the
/// leader: the learner's weight 0 keeps three voters, so view 3 selects the
/// node at voter position 0 — the leader that holds the machine.
fn retained_era_view() -> ViewId {
    ViewId {
        era: Era(2),
        view: View(3),
    }
}

// ---------------------------------------------------------------------------
// Accepted plan executes
// ---------------------------------------------------------------------------

#[test]
fn accepted_plan_executes_both_eras_amid_client_traffic() {
    let mut h = cluster();
    bootstrap(&mut h);

    // Client traffic before the plan.
    assert!(matches!(
        h.propose(n(0), op_id(1), b"before"),
        StepOutcome::Published { .. }
    ));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 3);

    let outcome = h.submit_plan(n(0), join_then_promote());
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the plan submits, not {outcome:?}");
    };
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::AdminResponse {
                verdict: PlanVerdict::Accepted
            }
        )),
        "the verdict surfaces: {effects:?}"
    );
    quiesce(&mut h);
    // The plan's first era committed, establishing era 2 (§8.7.1).
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 4);
    assert_eq!(
        h.era_table(n(0))
            .expect("live")
            .record(Era(2))
            .expect("era 2 is recorded")
            .establishing_operation,
        SystemOperation::Batch(vec![SystemOperation::Join {
            node: n(3),
            position: 3
        }]),
        "the plan's first step is era 2's establishing batch"
    );

    // Client traffic between eras — the cluster keeps running normally.
    assert!(matches!(
        h.propose(n(0), op_id(2), b"between"),
        StepOutcome::Published { .. }
    ));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 5);

    // The era boundary: the machine is armed with the second step pending,
    // and the established-but-unentered era awaits the view change (§8.7.8).
    // The §14.2 forced change retains the leader — view 3 selects it under
    // the era-2 voter succession — and the machine survives the change.
    assert!(matches!(
        h.force_view(n(0), retained_era_view()),
        StepOutcome::Published { .. }
    ));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 5);

    // The continuation proposes the second step on an ordinary tick.
    assert!(matches!(h.tick(n(0)), StepOutcome::Published { .. }));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 6);
    assert_eq!(
        h.era_table(n(0))
            .expect("live")
            .record(Era(3))
            .expect("era 3 is recorded")
            .establishing_operation,
        SystemOperation::Batch(vec![SystemOperation::Increment(n(3))]),
        "the plan's second step is era 3's establishing batch"
    );

    // Client traffic after completion.
    assert!(matches!(
        h.propose(n(0), op_id(3), b"after"),
        StepOutcome::Published { .. }
    ));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 7);

    // The final configuration is the plan's target.
    let target = h.era_table(n(0)).expect("live").current().config.clone();
    assert_eq!(
        target.order(),
        [member(0, 1), member(1, 1), member(2, 1), member(3, 1)]
    );

    // The machine cleared with the last step's commit: further ticks propose
    // no establishing batch and record no abort — a still-armed machine
    // would either re-propose the promotion (era 4) or abort by name.
    h.tick_all();
    h.tick_all();
    quiesce(&mut h);
    assert_eq!(h.era_table(n(0)).expect("live").current().era, Era(3));
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 7);
    assert_eq!(h.diagnostic(n(0)), Some(Diagnostic::None));
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// Rejected stale plans
// ---------------------------------------------------------------------------

#[test]
fn rejected_stale_plans_name_the_reason_and_change_nothing() {
    let mut h = cluster();
    bootstrap(&mut h);

    // Wrong weights, wrong order, wrong membership: one plan per refusal.
    let cases = [
        (
            Plan {
                initial: vec![member(0, 2), member(1, 1), member(2, 1)],
                steps: vec![vec![SystemOperation::Decrement(n(0))]],
            },
            PlanRejection::InitialWeight {
                node: n(0),
                plan: Weight(2),
                committed: Weight(1),
            },
        ),
        (
            Plan {
                initial: vec![member(0, 1), member(2, 1), member(1, 1)],
                steps: Vec::new(),
            },
            PlanRejection::InitialMembership {
                plan: vec![member(0, 1), member(2, 1), member(1, 1)],
                committed: vec![member(0, 1), member(1, 1), member(2, 1)],
            },
        ),
        (
            Plan {
                initial: vec![member(0, 1), member(1, 1), member(3, 1)],
                steps: Vec::new(),
            },
            PlanRejection::InitialMembership {
                plan: vec![member(0, 1), member(1, 1), member(3, 1)],
                committed: vec![member(0, 1), member(1, 1), member(2, 1)],
            },
        ),
    ];

    for (plan, expected) in cases {
        let outcome = h.submit_plan(n(0), plan);
        let StepOutcome::Published { effects, .. } = outcome else {
            panic!("the verdict is a published transition, not {outcome:?}");
        };
        assert!(
            matches!(
                &effects[..],
                [Effect::AdminResponse {
                    verdict: PlanVerdict::Rejected { reason },
                }] if *reason == expected.to_string()
            ),
            "the rejection names {expected} and nothing else: {effects:?}"
        );
        // Nothing about the cluster changes; no step was proposed.
        assert_eq!(h.queued_len(), 0, "no datagram was queued");
        assert_eq!(h.snapshot(n(0)).expect("live").accepted, 2);
        assert_eq!(h.era_table(n(0)).expect("live").current().era, Era(1));
        assert_eq!(
            h.era_table(n(0)).expect("live").current().config.order(),
            [member(0, 1), member(1, 1), member(2, 1)]
        );
        quiesce(&mut h);
    }
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// Drift aborts
// ---------------------------------------------------------------------------

#[test]
fn drift_aborts_the_machine_with_a_named_diagnostic() {
    let mut h = cluster();
    bootstrap(&mut h);

    let outcome = h.submit_plan(n(0), join_then_promote());
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 3);
    assert_eq!(h.era_table(n(0)).expect("live").current().era, Era(2));

    // Leadership retained across the era boundary (§14.2).
    assert!(matches!(
        h.force_view(n(0), retained_era_view()),
        StepOutcome::Published { .. }
    ));
    quiesce(&mut h);

    // Drift: a host reconfiguration departs the very learner the plan's
    // second step promotes. The commit is an ordinary, legal transition.
    assert!(matches!(
        h.reconfigure(n(0), SystemOperation::Leave(n(3)), None),
        StepOutcome::Published { .. }
    ));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 4);
    assert_eq!(h.era_table(n(0)).expect("live").current().era, Era(3));
    assert_eq!(
        h.era_table(n(0)).expect("live").current().config.order(),
        [member(0, 1), member(1, 1), member(2, 1)]
    );

    // The next tick finds the plan's second step refused by the fold: a
    // plan computed to be legal cannot become illegal, so the machine
    // clears and names the abort.
    assert!(matches!(h.tick(n(0)), StepOutcome::Published { .. }));
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::PlanAborted { step: 1 }),
        "the abort names the refused step"
    );
    h.assert_safety();

    // The cluster remains safe in the era it reached, and the cleared
    // machine proposes nothing further.
    h.tick(n(0));
    h.deliver_all();
    assert_eq!(h.era_table(n(0)).expect("live").current().era, Era(3));
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 4);
    assert_eq!(h.diagnostic(n(0)), Some(Diagnostic::None));
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// Admin-first polling (sim)
// ---------------------------------------------------------------------------

#[test]
fn admin_first_polling_puts_the_first_era_in_flight_before_client_traffic() {
    let mut h = cluster();
    bootstrap(&mut h);

    // The host holds a client command in its regular queue and a plan in
    // its admin queue. The selection polls the admin queue FIRST.
    let plan = Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1)],
        steps: vec![vec![SystemOperation::Join {
            node: n(3),
            position: 3,
        }]],
    };
    let outcome = h.submit_plan(n(0), plan);
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the plan submits, not {outcome:?}");
    };
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::AdminResponse {
                verdict: PlanVerdict::Accepted
            }
        )),
        "the verdict surfaces: {effects:?}"
    );
    // The plan's first era is in flight: the establishing batch took its
    // slot and its Prepare is queued before anything client-side moved.
    assert_eq!(
        h.journal_entry(n(0), Slot(3))
            .expect("the establishing entry is accepted")
            .payload,
        Payload::System(SystemOperation::Batch(vec![SystemOperation::Join {
            node: n(3),
            position: 3
        }]))
    );

    // Only now does the regular queue surface its command — the plan's
    // first era was in flight before the client command was proposed.
    assert!(matches!(
        h.propose(n(0), op_id(1), b"client"),
        StepOutcome::Published { .. }
    ));
    let client = h
        .journal_entry(n(0), Slot(4))
        .expect("the client command took the next slot");
    assert!(
        matches!(client.payload, Payload::Operation { .. }),
        "the client command is an operation slot, not {client:?}"
    );

    quiesce(&mut h);
    // Both commit in slot order: the plan's era first.
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 4);
    assert_eq!(
        h.era_table(n(0))
            .expect("live")
            .record(Era(2))
            .expect("era 2 is recorded")
            .established_by,
        Slot(3),
        "the plan's first era established era 2 at the slot the client command follows"
    );
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// Leader-crash discard
// ---------------------------------------------------------------------------

#[test]
fn leader_crash_discards_the_machine_and_the_new_leader_continues_nothing() {
    let mut h = cluster();
    bootstrap(&mut h);

    let outcome = h.submit_plan(n(0), join_then_promote());
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 3);
    assert_eq!(h.era_table(n(0)).expect("live").current().era, Era(2));

    // The leader crashes with the machine armed at the second step.
    h.crash(n(0));

    // The era boundary completes without it: the forced change elects the
    // new primary of the established-but-unentered era (§8.7.8, §14.2).
    assert!(matches!(
        h.force_view(
            n(1),
            ViewId {
                era: Era(2),
                view: View(1)
            }
        ),
        StepOutcome::Published { .. }
    ));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(1)).expect("live").committed, 3);

    // The new leader continues nothing automatically: ticks propose no
    // establishing batch, and the era the crash's survivor reached stands.
    h.tick(n(1));
    h.tick(n(1));
    quiesce(&mut h);
    assert_eq!(h.era_table(n(1)).expect("live").current().era, Era(2));
    assert_eq!(h.snapshot(n(1)).expect("live").committed, 3);
    assert_eq!(h.diagnostic(n(1)), Some(Diagnostic::None));
    h.assert_safety();

    // The dumb-operator contract: the operator re-solicits the remaining
    // plan against the configuration that committed; the new leader
    // validates and executes it.
    let remaining = Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1), member(3, 0)],
        steps: vec![vec![SystemOperation::Increment(n(3))]],
    };
    let outcome = h.submit_plan(n(1), remaining);
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the re-solicited plan submits, not {outcome:?}");
    };
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::AdminResponse {
                verdict: PlanVerdict::Accepted
            }
        )),
        "the verdict surfaces: {effects:?}"
    );
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(1)).expect("live").committed, 4);
    assert_eq!(
        h.era_table(n(1)).expect("live").current().config.order(),
        [member(0, 1), member(1, 1), member(2, 1), member(3, 1)],
        "the plan's target is reached"
    );
    h.assert_safety();
}
