//! Normal operation: VRR-2012 §4 (Prepare/PrepareOk/Commit), the
//! Propose/Apply/Applied application boundary (§11.1), commit-frontier
//! piggybacking (§13.3), and the bootstrap from the fenced `Recovering`
//! genesis state.
//!
//! The bootstrap (the genesis ruling, §1.3): a `Recovering` node whose journal holds
//! the complete committed genesis (slots 1–2, nothing missing) and which IS
//! `config.primary(View(0))` enters `Normal` on a tick — at initial
//! provisioning there is no prior state to be amnesiac about, so the §14.2
//! objection does not apply; the tick keeps construction uniform. Backups
//! adopt the view from a legitimate primary `Prepare`/`Commit` (§4's own
//! mechanism), never from ticks.

use vrr::configuration::INIT_SLOT;
use vrr::effects::Effect;
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
use vrr::replica::PlanRejection;
use vrr::wire::{Header, Tag};

#[path = "harness/mod.rs"]
mod harness;

use harness::{BoundaryEvent, Harness, StepOutcome};

fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// An operation identity for the scripts: the host assigns it, the core
/// carries it opaque (§11.1, B2).
fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

/// The view every node in these tests provisions and bootstraps into: era 1
/// (genesis reconfiguration), view 0.
fn genesis_view() -> ViewId {
    ViewId {
        era: Era(1),
        view: View::INITIAL,
    }
}

/// A three-node cluster past the bootstrap: the genesis primary promoted on
/// the first tick, every backup adopted view 0 from the promotion's `Commit`
/// announcement (§13.3).
fn bootstrapped() -> Harness {
    let mut h = Harness::provision(3);
    h.tick_all();
    h.deliver_all();
    h
}

/// The `Prepare` a view-0 primary would send for an operation at `slot`.
/// Scripts use it to re-offer a dropped `Prepare` (the harness's documented
/// fabrication path), standing in for the primary's retransmit, which
/// belongs to state transfer (§10). The entry carries its operation
/// identity (§11.1), so the fabrication must name it.
fn prepare(slot: u64, committed: u64, operation_id: OperationId, payload: &[u8]) -> Message {
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
                    id: operation_id,
                    payload: payload.into(),
                },
            },
            committed: Slot(committed),
        },
    }
}

/// The `Apply` slots one step released, in release order — the
/// duplicate-delivery assertions are stated over these.
fn apply_slots(effects: &[Effect]) -> Vec<Slot> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Apply { slot, .. } => Some(*slot),
            _ => None,
        })
        .collect()
}

/// Bootstrap plus one operation end to end (§4): the tick promotes the
/// genesis primary; the `Prepare` adopts+accepts at the backup; the
/// `PrepareOk` completes the Commit quorum; the `Apply` carries the
/// operation's identity to the host, whose `Applied` acknowledgement
/// advances the applied frontier (§11.1).
#[test]
fn bootstrap_and_one_request_end_to_end() {
    let mut h = Harness::provision(3);
    for id in [n(0), n(1), n(2)] {
        let snapshot = h.snapshot(id).expect("provisioned");
        assert_eq!(
            snapshot.status,
            Status::Recovering.to_word(),
            "every node starts fenced Recovering"
        );
    }

    h.tick_all();
    let s0 = h.snapshot(n(0)).expect("up");
    assert_eq!(
        s0.status,
        Status::Normal.to_word(),
        "the genesis primary self-promoted on the tick"
    );
    assert_eq!(
        (s0.era, s0.view),
        (1, 0),
        "the genesis view is (era 1, view 0)"
    );
    for id in [n(1), n(2)] {
        assert_eq!(
            h.snapshot(id).expect("up").status,
            Status::Recovering.to_word(),
            "backups adopt the view from the primary's messages, not from ticks"
        );
    }
    assert_eq!(
        h.queued_len(),
        2,
        "the promotion announced the committed frontier to both backups (§13.3)"
    );

    let outcome = h.propose(n(0), op_id(1), b"first");
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the primary accepts the proposal: {outcome:?}");
    };
    assert_eq!(effects.len(), 2, "Prepare broadcast to both backups");
    for effect in &effects {
        assert!(
            matches!(effect, Effect::Send { era, message, .. }
                if *era == Era(1)
                    && message.header.tag == Tag::Prepare
                    && message.header.slot == Slot(3)),
            "a Prepare for slot 3 under era 1: {effect:?}"
        );
    }

    // The Prepare reaches backup n1: it adopts view 0, accepts slot 3, and
    // answers PrepareOk.
    let delivery = h
        .deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("the Prepare is queued");
    assert_eq!(delivery.from, n(0));
    assert!(matches!(delivery.outcome, StepOutcome::Published { .. }));
    let s1 = h.snapshot(n(1)).expect("up");
    assert_eq!(
        s1.status,
        Status::Normal.to_word(),
        "the backup adopted view 0"
    );
    assert_eq!(s1.accepted, 3);

    // The PrepareOk completes the Commit quorum (own + n1): the primary
    // commits, releasing the Apply FIRST and then the §13.3 Commit broadcast.
    let delivery = h
        .deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("the PrepareOk is queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the commit publishes: {:?}", delivery.outcome);
    };
    assert!(
        matches!(&effects[0], Effect::Apply { slot, operation_id, payload }
            if *slot == Slot(3) && *operation_id == op_id(1) && payload.as_ref() == b"first"),
        "the Apply is the first released effect, carrying the operation's identity (§11.1): {effects:?}"
    );
    assert_eq!(h.snapshot(n(0)).expect("up").committed, 3);

    // The application performs the slot and acknowledges it; the applied
    // frontier advances. Nothing comes back: `Applied` carries no result
    // and the boundary has no reply (B2).
    let applies = h.execute_apply_effects(n(0));
    assert_eq!(applies.len(), 1);
    assert_eq!(applies[0].slot, Slot(3));
    assert!(matches!(applies[0].outcome, StepOutcome::Published { .. }));
    assert_eq!(
        h.snapshot(n(0)).expect("up").applied,
        3,
        "the Applied acknowledgement advanced the applied frontier"
    );

    // Quiesce: every node commits and applies slot 3, in order, and the
    // applied records agree.
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    let expected_applied: &[(Slot, Box<[u8]>)] = &[(Slot(3), Box::from(&b"first"[..]))];
    for id in [n(0), n(1), n(2)] {
        assert_eq!(h.applied(id), expected_applied, "n{} applied slot 3", id.0);
    }
    h.assert_safety();
}

/// The committed frontier travels both §13.3 ways: the explicit `Commit`,
/// and the piggyback on the next `Prepare`. A backup applies only what it
/// has accepted, in order, and all three applied records agree.
#[test]
fn commit_propagates_by_commit_message_and_by_piggyback() {
    let mut h = bootstrapped();
    h.propose(n(0), op_id(1), b"one"); // slot 3
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    assert_eq!(h.snapshot(n(0)).expect("up").committed, 3);
    assert_eq!(
        h.snapshot(n(2)).expect("up").committed,
        2,
        "n2 has not accepted slot 3"
    );

    // The explicit Commit alone cannot advance n2 past its accepted frontier
    // (§5 invariant 2): the target is min(3, 2) = 2.
    h.deliver_to_matching(n(2), Tag::Commit, Slot(3))
        .expect("queued");
    assert_eq!(h.snapshot(n(2)).expect("up").committed, 2);

    // n2 accepts slot 3; its piggyback was stale, so it still cannot commit.
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(3))
        .expect("queued");
    assert_eq!(
        h.snapshot(n(2)).expect("up").committed,
        2,
        "the slot-3 Prepare piggybacked the stale frontier 2"
    );

    // The NEXT Prepare carries the new frontier: the piggyback commits slot
    // 3 at n2 without any further Commit message (§13.3).
    h.propose(n(0), op_id(2), b"two"); // slot 4, piggyback 3
    let delivery = h
        .deliver_to_matching(n(2), Tag::Prepare, Slot(4))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("n2 accepts slot 4: {:?}", delivery.outcome);
    };
    assert!(
        effects.iter().any(
            |effect| matches!(effect, Effect::Apply { slot, payload, .. }
            if *slot == Slot(3) && payload.as_ref() == b"one")
        ),
        "the piggybacked frontier applied slot 3 at n2: {effects:?}"
    );
    assert_eq!(h.snapshot(n(2)).expect("up").committed, 3);

    // Quiesce: everything commits and applies everywhere, in slot order.
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    let expected: &[(Slot, Box<[u8]>)] = &[
        (Slot(3), Box::from(&b"one"[..])),
        (Slot(4), Box::from(&b"two"[..])),
    ];
    for id in [n(0), n(1), n(2)] {
        assert_eq!(h.applied(id), expected, "n{} applied in slot order", id.0);
    }
    h.assert_safety();
}

/// A Commit quorum is the strategy's weighted majority, not unanimity (Q1):
/// with three unit-weight members, ONE backup's `PrepareOk` plus the
/// primary's own vote commits. The delayed second `PrepareOk` is a named,
/// harmless drop.
#[test]
fn commit_lands_on_a_weighted_majority_not_unanimity() {
    let mut h = bootstrapped();
    h.propose(n(0), op_id(1), b"op"); // slot 3
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    assert_eq!(
        h.snapshot(n(0)).expect("up").committed,
        3,
        "own vote + one backup is 2 of 3 unit weights: a Commit quorum (Q1)"
    );

    // The delayed second PrepareOk changes nothing and faults nothing.
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(3))
        .expect("queued");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::SlotNotOutstanding {
            slot: Slot(3),
            sender: n(2)
        }),
        "the delayed duplicate is a named drop"
    );
    assert_eq!(h.snapshot(n(0)).expect("up").committed, 3);

    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    h.assert_safety();
}

/// No deduplication at the boundary (§11.1, B2): the same [`OperationId`]
/// proposed twice is TWO operations to the core — two slots, two commits,
/// two `Apply` effects, each carrying the identity the host assigned.
/// Exactly-once is the host's deduplication policy above the boundary,
/// never the core's.
#[test]
fn duplicate_proposal_is_a_distinct_operation() {
    let mut h = bootstrapped();
    let id = op_id(42);
    h.propose(n(0), id, b"first"); // slot 3
    h.propose(n(0), id, b"second"); // the same identity again: slot 4
    assert_eq!(
        h.snapshot(n(0)).expect("up").accepted,
        4,
        "the duplicate identity took the next slot — nothing was dropped"
    );

    h.deliver_all();
    for node in [n(0), n(1), n(2)] {
        h.execute_apply_effects(node);
    }
    let applies: Vec<(Slot, OperationId)> = h
        .boundary_events()
        .iter()
        .filter_map(|event| match event {
            BoundaryEvent::Applied {
                node,
                slot,
                operation_id,
            } if *node == n(0) => Some((*slot, *operation_id)),
            _ => None,
        })
        .collect();
    assert_eq!(
        applies,
        vec![(Slot(3), id), (Slot(4), id)],
        "the duplicate identity applied twice, once per proposal, in slot order"
    );
    let expected: &[(Slot, Box<[u8]>)] = &[
        (Slot(3), Box::from(&b"first"[..])),
        (Slot(4), Box::from(&b"second"[..])),
    ];
    for node in [n(0), n(1), n(2)] {
        assert_eq!(
            h.applied(node),
            expected,
            "n{} applied both proposals",
            node.0
        );
    }
    h.assert_safety();
}

/// A duplicate `Prepare` never re-appends: one journal entry, two
/// `PrepareOk`s (idempotent retransmission, §4).
#[test]
fn duplicate_prepare_is_idempotent() {
    let mut h = bootstrapped();
    h.propose(n(0), op_id(1), b"x"); // slot 3
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    assert_eq!(h.snapshot(n(0)).expect("up").committed, 3);

    // The retransmitted Prepare carries the same entry: re-acknowledged,
    // never re-appended.
    h.send(n(0), n(1), prepare(3, 2, op_id(1), b"x"));
    let delivery = h
        .deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the re-acknowledgement publishes: {:?}", delivery.outcome);
    };
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::Send { message, .. }
            if message.header.tag == Tag::PrepareOk)),
        "the retransmission is re-acknowledged: {effects:?}"
    );
    assert_eq!(
        h.snapshot(n(1)).expect("up").accepted,
        3,
        "still one journal entry — a re-append would have faulted through the journal"
    );

    // The second PrepareOk is the harmless delayed duplicate at the primary.
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::SlotNotOutstanding {
            slot: Slot(3),
            sender: n(1)
        })
    );

    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    h.assert_safety();
}

/// An out-of-order `Prepare` is a gap: dropped, reported as `GapDetected`,
/// never faulted — and the fetch half of the ruling (§13.1 step 5) rides
/// the same transition, asking the primary for the missing range. The
/// missing `Prepare` closes the gap; the re-offered slot then completes
/// the chain. The re-offer stands in for the primary's retransmit.
#[test]
fn out_of_order_prepare_gap_is_dropped_and_recovered() {
    let mut h = bootstrapped();
    h.propose(n(0), op_id(1), b"first"); // slot 3
    h.propose(n(0), op_id(2), b"second"); // slot 4

    // Slot 4 before slot 3: a gap.
    let delivery = h
        .deliver_to_matching(n(1), Tag::Prepare, Slot(4))
        .expect("queued");
    assert!(
        matches!(delivery.outcome, StepOutcome::Published { ref effects, .. } if effects.len() == 1),
        "the gap publishes only its fetch: {:?}",
        delivery.outcome
    );
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::GapDetected {
            expected: Slot(3),
            got: Slot(4)
        }),
        "the gap is reported, not silent"
    );
    assert_eq!(
        h.snapshot(n(1)).expect("up").accepted,
        2,
        "the gap was dropped, not accepted"
    );

    // The fetch half of the ruling: a `GetState` for the missing range,
    // addressed to the primary, resuming one past the frontier.
    let request = h
        .peek_queued(n(0), Tag::GetState)
        .expect("the gap fetches from the primary");
    let Body::GetState { from } = &request.body else {
        panic!("expected GetState");
    };
    assert_eq!(*from, Slot(3));
    assert_eq!(request.header.slot, Slot(2));
    h.drop_queued(n(0)); // the re-offer below closes the gap instead

    // The missing Prepare arrives: the chain resumes without any fault.
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    assert_eq!(h.snapshot(n(1)).expect("up").accepted, 3);

    // Slot 4 is re-offered and the chain completes.
    h.send(n(0), n(1), prepare(4, 3, op_id(2), b"second"));
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    assert_eq!(h.snapshot(n(0)).expect("up").committed, 4);
    let expected: &[(Slot, Box<[u8]>)] = &[
        (Slot(3), Box::from(&b"first"[..])),
        (Slot(4), Box::from(&b"second"[..])),
    ];
    for id in [n(0), n(1), n(2)] {
        assert_eq!(
            h.applied(id),
            expected,
            "n{} completed the chain in order",
            id.0
        );
    }
    h.assert_safety();
}

/// Proposals route to the primary of the current view, and only there: the
/// named `NotPrimary` refusal, before and after the bootstrap. The refusal
/// carries the node's current view and the primary of that view (§13.4's
/// convergence hint), so a host can redirect the proposer.
#[test]
fn proposals_to_non_primaries_are_not_primary_refusals() {
    // The fenced genesis primary is not yet Normal: NotPrimary, not a silent
    // no-op. It names itself — it IS the primary of its current view, just
    // not yet serving it.
    let mut fresh = Harness::provision(3);
    assert_eq!(
        fresh.propose(n(0), op_id(1), b"early"),
        StepOutcome::PlanRefused(PlanRejection::NotPrimary {
            view: genesis_view(),
            primary: Some(n(0)),
        })
    );

    let mut h = bootstrapped();
    // n1 is the primary of view 1 — but the current view is 0, so it is the
    // primary of the wrong view: NotPrimary, redirecting to n0. n2 is a
    // plain backup, same redirection.
    assert_eq!(
        h.propose(n(1), op_id(1), b"wrong-view-primary"),
        StepOutcome::PlanRefused(PlanRejection::NotPrimary {
            view: genesis_view(),
            primary: Some(n(0)),
        })
    );
    assert_eq!(
        h.propose(n(2), op_id(1), b"backup"),
        StepOutcome::PlanRefused(PlanRejection::NotPrimary {
            view: genesis_view(),
            primary: Some(n(0)),
        })
    );
    // The refusals consumed nothing: the next proposal still accepts at the
    // primary.
    assert!(matches!(
        h.propose(n(0), op_id(1), b"right"),
        StepOutcome::Published { .. }
    ));
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    h.assert_safety();
}

/// Three pipelined proposals, one quorum: `PrepareOk`s delivered in slot
/// order commit the slots one after another, and Apply effects release
/// strictly in slot order (§11.1).
#[test]
fn commit_cascade_orders_applies() {
    let mut h = bootstrapped();
    for request in 1..=3u64 {
        h.propose(n(0), op_id(request), format!("r{request}").as_bytes());
    }
    // All three Prepares to n1, in slot order; no PrepareOk delivered yet.
    for slot in 3..=5u64 {
        h.deliver_to_matching(n(1), Tag::Prepare, Slot(slot))
            .expect("queued");
    }
    // PrepareOks in slot order: each completes the quorum for its slot.
    let mut applies = Vec::new();
    for slot in 3..=5u64 {
        let delivery = h
            .deliver_to_matching(n(0), Tag::PrepareOk, Slot(slot))
            .expect("queued");
        let StepOutcome::Published { effects, .. } = delivery.outcome else {
            panic!("the commit publishes: {:?}", delivery.outcome);
        };
        let newly: Vec<Slot> = effects
            .iter()
            .filter_map(|effect| match effect {
                Effect::Apply { slot, .. } => Some(*slot),
                _ => None,
            })
            .collect();
        assert_eq!(
            newly,
            vec![Slot(slot)],
            "exactly the one newly committed slot applies"
        );
        applies.extend(newly);
    }
    assert_eq!(
        applies,
        vec![Slot(3), Slot(4), Slot(5)],
        "Apply effects strictly in slot order"
    );

    let outcomes = h.execute_apply_effects(n(0));
    assert_eq!(outcomes.len(), 3);

    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    h.assert_safety();
}

/// A load script: proposals, deliveries, applies, ticks, and one crash with
/// an amnesiac restart. The legality gate stands after every step;
/// `assert_safety` runs after every quiesce. The amnesiac node lags behind
/// on gaps (the state transfer is §10's) but never faults and never
/// diverges. Reused identities across rounds exercise the boundary's
/// no-deduplication rule (§11.1) under load.
fn load_script() -> Harness {
    let mut h = bootstrapped();
    for round in 0..30u64 {
        h.propose(n(0), op_id(round % 3 + 1), format!("op-{round}").as_bytes());
        if round % 4 == 1 {
            h.tick_all();
        }
        if round == 12 {
            h.crash(n(2));
            h.restart_amnesiac(n(2)).expect("amnesiac restart");
        }
        h.deliver_all();
        for id in [n(0), n(1), n(2)] {
            h.execute_apply_effects(id);
        }
        if round % 3 == 0 {
            h.assert_safety();
        }
    }
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    h.assert_safety();
    h
}

/// ~200 steps of the load script, twice: byte-identical traces. The script
/// is the only source of nondeterminism.
#[test]
fn the_legality_gate_stands_under_load() {
    let first = load_script();
    let second = load_script();
    assert_eq!(
        first.trace_dump(),
        second.trace_dump(),
        "same script, same trace"
    );
    assert!(
        first.snapshot(n(0)).expect("up").committed > INIT_SLOT.0,
        "the cluster made progress through the load"
    );
}

/// The boundary carries identity end to end, at every node (§11.1): every
/// `Apply` names the slot and the proposing host's [`OperationId`], in slot
/// order, and the host's `Applied` acknowledgement is a bare slot — no
/// result travels back, and no node ever emits a reply (B2).
#[test]
fn apply_carries_the_proposals_identity() {
    let mut h = bootstrapped();
    for round in 0..6u64 {
        h.propose(n(0), op_id(round + 1), format!("req-{round}").as_bytes());
    }
    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }

    let events = h.boundary_events().to_vec();
    assert!(events.len() >= 6, "the property is exercised, not vacuous");
    let expected: Vec<(Slot, OperationId)> = (3..=8u64)
        .map(|slot| (Slot(slot), op_id(slot - 2)))
        .collect();
    for id in [n(0), n(1), n(2)] {
        let seen: Vec<(Slot, OperationId)> = events
            .iter()
            .filter_map(|event| match event {
                BoundaryEvent::Applied {
                    node,
                    slot,
                    operation_id,
                } if *node == id => Some((*slot, *operation_id)),
                _ => None,
            })
            .collect();
        assert_eq!(
            seen, expected,
            "n{} applied every proposal in slot order, identity attached",
            id.0
        );
    }
    h.assert_safety();
}

/// A duplicate `Commit` to a backup — same view, same committed frontier,
/// redelivered by a lossy transport — is a silent no-effect transition: no
/// diagnostic, no second `Apply`, and the frontier does not move. The
/// redelivery is the very datagram the primary emitted, captured from the
/// queue and re-enqueued.
#[test]
fn duplicate_commit_to_a_backup_is_a_silent_no_effect() {
    let mut h = bootstrapped();
    h.propose(n(0), op_id(1), b"one"); // slot 3
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    assert_eq!(h.snapshot(n(0)).expect("up").committed, 3);

    let commit = h.peek_queued(n(1), Tag::Commit).expect("queued");
    let delivery = h
        .deliver_to_matching(n(1), Tag::Commit, Slot(3))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the first Commit publishes: {:?}", delivery.outcome);
    };
    assert_eq!(
        apply_slots(&effects),
        vec![Slot(3)],
        "the first Commit applies slot 3"
    );
    assert_eq!(h.snapshot(n(1)).expect("up").committed, 3);

    h.send(n(0), n(1), commit);
    let delivery = h
        .deliver_to_matching(n(1), Tag::Commit, Slot(3))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the duplicate publishes: {:?}", delivery.outcome);
    };
    assert!(
        effects.is_empty(),
        "the duplicate Commit publishes nothing: {effects:?}"
    );
    assert_eq!(
        h.snapshot(n(1)).expect("up").committed,
        3,
        "the frontier does not move"
    );

    let applies = h.execute_apply_effects(n(1));
    assert_eq!(applies.len(), 1, "slot 3 applied exactly once");
    assert_eq!(applies[0].slot, Slot(3));

    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    h.assert_safety();
}

/// A duplicate `Prepare` to a backup — same slot, same entry — delivered
/// after the backup accepted the original: the primary's retransmit now
/// piggybacks the committed frontier (§13.3), so the first copy commits
/// and applies the slot; the second is a bare re-acknowledgement — a
/// fresh `PrepareOk`, no re-append, and no second `Apply`. The re-offer
/// stands in for the primary's retransmit, the harness's documented
/// fabrication path.
#[test]
fn duplicate_prepare_reacknowledges_without_reapplying() {
    let mut h = bootstrapped();
    h.propose(n(0), op_id(1), b"x"); // slot 3
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(3))
        .expect("queued");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    assert_eq!(h.snapshot(n(0)).expect("up").committed, 3);
    assert_eq!(
        h.snapshot(n(1)).expect("up").committed,
        2,
        "n1 accepted slot 3 but the original Prepare piggybacked the stale frontier"
    );

    // The retransmit's first copy: the piggybacked frontier commits and
    // applies slot 3 at n1.
    h.send(n(0), n(1), prepare(3, 3, op_id(1), b"x"));
    let delivery = h
        .deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the re-offer publishes: {:?}", delivery.outcome);
    };
    assert_eq!(
        apply_slots(&effects),
        vec![Slot(3)],
        "the piggybacked frontier applies slot 3"
    );
    assert_eq!(h.snapshot(n(1)).expect("up").committed, 3);

    // The duplicate, same slot and same entry: re-acknowledged, never
    // re-appended, never re-applied.
    h.send(n(0), n(1), prepare(3, 3, op_id(1), b"x"));
    let delivery = h
        .deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the re-acknowledgement publishes: {:?}", delivery.outcome);
    };
    assert!(
        apply_slots(&effects).is_empty(),
        "no second Apply for the slot: {effects:?}"
    );
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::Send { message, .. }
            if message.header.tag == Tag::PrepareOk)),
        "the duplicate is re-acknowledged: {effects:?}"
    );
    let s1 = h.snapshot(n(1)).expect("up");
    assert_eq!(s1.accepted, 3, "still one journal entry");
    assert_eq!(s1.committed, 3, "the frontier does not move");

    let applies = h.execute_apply_effects(n(1));
    assert_eq!(applies.len(), 1, "slot 3 applied exactly once");
    assert_eq!(applies[0].slot, Slot(3));

    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    h.assert_safety();
}

/// A duplicate `PrepareOk` at the primary — the same backup's
/// acknowledgement delivered twice for one proposal — is the named
/// [`Diagnostic::DuplicatePrepareOk`] drop while the slot is still
/// outstanding: no vote is double-counted, the commit fires exactly once
/// when the quorum later lands, and the slot applies exactly once. Five
/// unit-weight members make the Commit quorum 3 (Q1), so the first
/// acknowledgement alone does not commit and the duplicate arrives while
/// the slot is still outstanding.
#[test]
fn duplicate_prepare_ok_at_the_primary_commits_once() {
    let mut h = Harness::provision(5);
    h.tick_all();
    h.deliver_all();
    h.propose(n(0), op_id(1), b"op"); // slot 3
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");

    let prepare_ok = h.peek_queued(n(0), Tag::PrepareOk).expect("queued");
    let delivery = h
        .deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the vote publishes: {:?}", delivery.outcome);
    };
    assert!(
        effects.is_empty(),
        "own vote + one backup is short of the quorum: {effects:?}"
    );
    assert_eq!(h.snapshot(n(0)).expect("up").committed, 2);

    // The duplicate: a named drop, no commit, no Apply.
    h.send(n(1), n(0), prepare_ok);
    let delivery = h
        .deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the drop publishes: {:?}", delivery.outcome);
    };
    assert!(
        effects.is_empty(),
        "the duplicate publishes nothing: {effects:?}"
    );
    assert_eq!(
        h.diagnostic(n(0)),
        Some(Diagnostic::DuplicatePrepareOk {
            slot: Slot(3),
            sender: n(1)
        }),
        "the duplicate is a named drop"
    );
    assert_eq!(
        h.snapshot(n(0)).expect("up").committed,
        2,
        "the duplicate vote counted nothing"
    );

    // A second backup's acknowledgement completes the quorum: the commit
    // fires once, the Apply once.
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(3))
        .expect("queued");
    let delivery = h
        .deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the commit publishes: {:?}", delivery.outcome);
    };
    assert_eq!(
        apply_slots(&effects),
        vec![Slot(3)],
        "the commit applies slot 3 exactly once"
    );
    assert_eq!(h.snapshot(n(0)).expect("up").committed, 3);

    let applies = h.execute_apply_effects(n(0));
    assert_eq!(applies.len(), 1, "slot 3 applied exactly once");
    assert_eq!(applies[0].slot, Slot(3));

    h.deliver_all();
    for id in [n(0), n(1), n(2), n(3), n(4)] {
        h.execute_apply_effects(id);
    }
    h.assert_safety();
}

/// A reordered duplicate `Commit` carrying a stale frontier, delivered
/// after the frontier already advanced past it: nothing is re-emitted and
/// the frontier never moves backward. Both commits land at the backup in
/// order first; then the FIRST commit's `Commit` datagram is redelivered.
#[test]
fn stale_commit_after_frontier_advanced_is_a_no_op() {
    let mut h = bootstrapped();
    h.propose(n(0), op_id(1), b"one"); // slot 3
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(3))
        .expect("queued");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");
    assert_eq!(h.snapshot(n(0)).expect("up").committed, 3);

    let stale = h.peek_queued(n(1), Tag::Commit).expect("queued");
    h.deliver_to_matching(n(1), Tag::Commit, Slot(3))
        .expect("queued");
    assert_eq!(h.snapshot(n(1)).expect("up").committed, 3);

    // The frontier advances: slot 4 commits and its Commit lands at n1.
    h.propose(n(0), op_id(2), b"two"); // slot 4
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(4))
        .expect("queued");
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(4))
        .expect("queued");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(4))
        .expect("queued");
    let delivery = h
        .deliver_to_matching(n(1), Tag::Commit, Slot(4))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the second Commit publishes: {:?}", delivery.outcome);
    };
    assert_eq!(
        apply_slots(&effects),
        vec![Slot(4)],
        "the advancing frontier applies slot 4"
    );
    assert_eq!(h.snapshot(n(1)).expect("up").committed, 4);

    // The reordered stale duplicate: a silent no-effect transition.
    h.send(n(0), n(1), stale);
    let delivery = h
        .deliver_to_matching(n(1), Tag::Commit, Slot(3))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the stale duplicate publishes: {:?}", delivery.outcome);
    };
    assert!(
        effects.is_empty(),
        "the stale Commit re-emits nothing: {effects:?}"
    );
    assert_eq!(
        h.snapshot(n(1)).expect("up").committed,
        4,
        "the frontier never moves backward"
    );

    let applies = h.execute_apply_effects(n(1));
    assert_eq!(
        applies.iter().map(|apply| apply.slot).collect::<Vec<_>>(),
        vec![Slot(3), Slot(4)],
        "each slot applied exactly once, in slot order"
    );

    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    h.assert_safety();
}

/// A duplicate `Commit` arriving at the applied boundary — after the host
/// performed the slot's `Apply` and acknowledged it with `Input::Applied`
/// (§11.1) — re-emits nothing: the applied frontier stands and the slot's
/// upcall fired exactly once within the life.
#[test]
fn duplicate_commit_after_host_acknowledgement_does_not_reemit() {
    let mut h = bootstrapped();
    h.propose(n(0), op_id(1), b"one"); // slot 3
    h.deliver_to_matching(n(1), Tag::Prepare, Slot(3))
        .expect("queued");
    h.deliver_to_matching(n(0), Tag::PrepareOk, Slot(3))
        .expect("queued");

    let commit = h.peek_queued(n(1), Tag::Commit).expect("queued");
    h.deliver_to_matching(n(1), Tag::Commit, Slot(3))
        .expect("queued");
    let applies = h.execute_apply_effects(n(1));
    assert_eq!(applies.len(), 1);
    assert_eq!(applies[0].slot, Slot(3));
    assert_eq!(
        h.snapshot(n(1)).expect("up").applied,
        3,
        "the host's acknowledgement advanced the applied frontier"
    );

    // The duplicate arrives after the acknowledgement: a silent no-effect
    // transition, nothing left to apply.
    h.send(n(0), n(1), commit);
    let delivery = h
        .deliver_to_matching(n(1), Tag::Commit, Slot(3))
        .expect("queued");
    let StepOutcome::Published { effects, .. } = delivery.outcome else {
        panic!("the duplicate publishes: {:?}", delivery.outcome);
    };
    assert!(
        effects.is_empty(),
        "the duplicate Commit re-emits nothing: {effects:?}"
    );
    assert!(
        h.execute_apply_effects(n(1)).is_empty(),
        "nothing is pending after the duplicate"
    );
    let expected: &[(Slot, Box<[u8]>)] = &[(Slot(3), Box::from(&b"one"[..]))];
    assert_eq!(
        h.applied(n(1)),
        expected,
        "slot 3 applied exactly once within the life"
    );

    h.deliver_all();
    for id in [n(0), n(1), n(2)] {
        h.execute_apply_effects(id);
    }
    h.assert_safety();
}
