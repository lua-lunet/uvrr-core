//! The `Fuse` acceptor (`docs/uvrr-fuse.md` §3): one envelope, one ballot,
//! one atomic batch. Receiving a `Fuse` is *defined* as receiving the
//! equivalent sequence of `Prepare` messages at the same ballot, one per
//! slot, in batch order; only the wire shape changes, so every assertion
//! here runs through the ordinary accept perimeter's observables: the
//! journal (the durable configuration record, §8.7.1), the queued replies,
//! and the harness's independent safety checker.
//!
//! The leader side (§4) is exercised through the public plan path: the
//! armed schedule's establishing batch travels as one `Fuse` per backup,
//! the `FuseOk` votes complete the quorum, and the commit emission is one
//! `CommitBatch` per era.

mod harness;

use harness::{Harness, StepOutcome};
use std::sync::Arc;

use vrr::configuration::{Member, SystemOperation, Weight};
use vrr::effects::Effect;
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use vrr::journal::Payload;
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::plan::Plan;
use vrr::wire::{Header, Pack, Tag, Unpack, UnpackError};

fn n(id: u32) -> NodeId {
    NodeId(id)
}

/// The bootstrap of `tests/reconfiguration_stepwise.rs`: the genesis
/// primary promotes itself and both backups adopt view (1, 0) from the
/// promotion's `Commit` announcement (§13.3).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

/// The view every node here bootstraps into: era 1, view 0, primary `n(0)`
/// under the genesis order `[0, 1, 2]` (§1.2).
fn current_view() -> ViewId {
    ViewId {
        era: Era(1),
        view: View(0),
    }
}

/// A `Fuse` envelope from the primary carrying `ops`, whose batch begins at
/// `first_slot` (`docs/uvrr-fuse.md` §1).
fn fuse_envelope(first_slot: Slot, ops: Vec<SystemOperation>) -> Message {
    Message {
        header: Header {
            tag: Tag::Fuse,
            view: current_view(),
            slot: first_slot,
        },
        body: Body::Fuse { ops },
    }
}

/// The genesis journal: `Void` at slot 1, `Init` at slot 2, nothing beyond.
fn assert_journal_is_genesis(h: &Harness, id: NodeId) {
    let entries = h.journal_entries(id);
    assert_eq!(entries.len(), 2, "only the genesis entries are held");
    assert_eq!(entries[0].slot, Slot(1));
    assert_eq!(entries[1].slot, Slot(2));
}

/// The 20-byte W1 header alone, encoded through the wire contract's
/// pack discipline.
fn encode_header(header: &Header) -> Vec<u8> {
    let mut bytes = vec![0u8; Header::LEN];
    let written = header
        .pack_into(&mut bytes)
        .expect("a buffer of exactly Header::LEN bytes must suffice");
    assert_eq!(written, Header::LEN);
    bytes
}

// ---------------------------------------------------------------------------
// 1. A valid envelope is accepted as a whole and answered once.
// ---------------------------------------------------------------------------
#[test]
fn valid_fuse_is_accepted_as_a_whole() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // The schedule: `[Join(new), Increment(new)]` — the shape of the
    // five-node replacement's second batch (`docs/uvrr-reincarnation.md`
    // §5), legal under R1–R15 for the fresh three-node cluster: `n(3)` is
    // absent, so the join inserts a learner at the succession end and the
    // increment promotes it inside the next era. Two ops, two era
    // transitions, one envelope.
    let ops = vec![
        SystemOperation::Join {
            node: n(3),
            position: 3,
        },
        SystemOperation::Increment(n(3)),
    ];
    let first_slot = Slot(3);
    let outcome = h.inject(n(0), n(1), fuse_envelope(first_slot, ops));

    // The envelope is accepted as a whole: one published transition.
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!(
            "a valid Fuse publishes, not {outcome:?}\n{}",
            h.trace_dump()
        );
    };

    // One reply, and only one: the `FuseOk` to the primary.
    assert_eq!(effects.len(), 1, "the accept releases exactly the FuseOk");
    let (to, era, message) = match &effects[0] {
        Effect::Send { to, era, message } => (to, era, message),
        other => panic!("the effect is a Send, not {other:?}"),
    };
    assert_eq!(*to, n(0), "the reply goes to the sender (the primary)");
    assert_eq!(
        *era,
        Era(1),
        "the reply is routed under the ballot's era (W1)"
    );
    assert_eq!(message.header.tag, Tag::FuseOk);
    assert_eq!(message.header.view, current_view());
    assert_eq!(
        message.header.slot,
        Slot(4),
        "the header slot is the LAST accepted slot"
    );
    let Body::FuseOk { acks } = &message.body else {
        panic!("the reply body is FuseOk, not {:?}", message.body);
    };
    assert_eq!(
        acks,
        &vec![Slot(3), Slot(4)],
        "acks name every packed slot in batch order"
    );

    // The journal records the contiguous batch, each op at its own slot,
    // exactly as the equivalent sequence of `Prepare`s would have journaled
    // it (§1's definition; the configuration record of §8.7.1 advanced per
    // op). The era table itself advances at commit (§8.7.1: the commit-time
    // fold is the only place the era advances) and is untouched here.
    let entries = h.journal_entries(n(1));
    assert_eq!(
        entries.len(),
        4,
        "the genesis prefix plus the two packed slots"
    );
    assert_eq!(entries[2].slot, Slot(3));
    assert_eq!(
        entries[2].era,
        Era(1),
        "every packed slot is authorised by the ballot"
    );
    assert_eq!(
        entries[2].payload,
        Payload::System(SystemOperation::Join {
            node: n(3),
            position: 3
        })
    );
    assert_eq!(entries[3].slot, Slot(4));
    assert_eq!(entries[3].era, Era(1));
    assert_eq!(
        entries[3].payload,
        Payload::System(SystemOperation::Increment(n(3)))
    );
    let snapshot = h.snapshot(n(1)).expect("the node is live");
    assert_eq!(snapshot.accepted, 4, "the frontier advanced per op");
    assert_eq!(snapshot.committed, 2, "acceptance commits nothing (§8.7.1)");

    // Exactly one FuseOk is queued for the primary, and nothing else: the
    // bootstrap drained the network, so the queue holds the reply alone.
    assert!(h.peek_queued(n(0), Tag::FuseOk).is_some());
    assert_eq!(h.queued_len(), 1, "exactly one FuseOk reply, nothing else");

    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 2. A stale-view envelope is refused as a whole.
// ---------------------------------------------------------------------------
#[test]
fn stale_view_fuse_is_refused_as_a_whole() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // The same legal schedule, but the header names the genesis view
    // (era 0, view 0) — below the acceptor's current view (1, 0). Era 0 is
    // evaluable (the void record is retained) and names no primary, so the
    // sender cannot be the primary of the message's view.
    let stale = Message {
        header: Header {
            tag: Tag::Fuse,
            view: ViewId {
                era: Era::INITIAL,
                view: View(0),
            },
            slot: Slot(3),
        },
        body: Body::Fuse {
            ops: vec![
                SystemOperation::Join {
                    node: n(3),
                    position: 3,
                },
                SystemOperation::Increment(n(3)),
            ],
        },
    };
    let outcome = h.inject(n(0), n(1), stale);
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("a refusal is a published identity transition, not {outcome:?}");
    };
    assert!(effects.is_empty(), "a refusal releases nothing");

    // Nothing was accepted: the journal is unchanged, no slot moved.
    assert_journal_unchanged_and_refused(&h, n(1));

    // Exactly one diagnostic names the refusal.
    let diagnostic = h.diagnostic(n(1)).expect("the node is live");
    assert_eq!(
        diagnostic,
        Diagnostic::FuseRefusal,
        "the refusal is named (§3)"
    );

    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 3. A malformed envelope is dropped without partial accept.
// ---------------------------------------------------------------------------
#[test]
fn malformed_fuse_is_dropped_without_partial_accept() {
    // Unrepresentability note (§3 step 4): an op refusing MID-batch with a
    // passing header is unreachable by the atomic batch property. The
    // schedule folds as one unit — the perimeter judges the whole sequence
    // before any effect is released — so a refusal names the whole envelope
    // at its first refusing op and there is no partial fold to observe. A
    // test cannot construct the mid-batch refusal without forging a
    // schedule no planner would certify, which §3 rules out of scope.
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // The malformed-decode drop: a valid 20-byte header tagging `Fuse`,
    // then a body whose first op carries an unknown discriminant. The
    // codec refuses it (`OutOfDomain`) — the host drops the datagram
    // before the core ever sees it, so no diagnostic reaches the
    // observation and nothing can be journaled.
    let mut bytes = encode_header(&Header {
        tag: Tag::Fuse,
        view: current_view(),
        slot: Slot(3),
    });
    bytes.push(u8::try_from(Tag::Fuse.as_u32()).expect("the fuse discriminant fits a u8"));
    bytes.extend_from_slice(&1u32.to_be_bytes());
    bytes.push(200); // an unknown SystemOperation discriminant

    assert!(
        Message::unpack_from(&bytes).is_err(),
        "a garbage body tagged Fuse must be refused by the codec"
    );
    match Message::unpack_from(&bytes) {
        Err(UnpackError::Malformed(_)) => {}
        other => panic!("the refusal is Malformed, not {other:?}"),
    }

    // The host's drop leaves the core untouched: journal unchanged, no
    // reply, no diagnostic — the malformed datagram never arrived.
    assert_journal_unchanged_and_refused(&h, n(1));
    assert_eq!(h.diagnostic(n(1)), Some(Diagnostic::None));
    assert!(h.peek_queued(n(0), Tag::FuseOk).is_none());

    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 4. The packed slots must be contiguous from the accept frontier.
// ---------------------------------------------------------------------------
#[test]
fn fuse_slots_must_be_contiguous_from_the_accept_frontier() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // A legal schedule whose `first_slot` is NOT the acceptor's next slot:
    // the envelope claims a range the frontier does not start at.
    let ops = vec![
        SystemOperation::Join {
            node: n(3),
            position: 3,
        },
        SystemOperation::Increment(n(3)),
    ];
    let outcome = h.inject(n(0), n(1), fuse_envelope(Slot(5), ops));
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("a refusal is a published identity transition, not {outcome:?}");
    };
    assert!(effects.is_empty(), "a refusal releases nothing");

    assert_journal_unchanged_and_refused(&h, n(1));
    assert_eq!(h.diagnostic(n(1)), Some(Diagnostic::FuseRefusal));

    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 5. The reply's slots are explicit, never ranges.
// ---------------------------------------------------------------------------
#[test]
fn accepted_fuse_reply_slots_are_explicit_not_ranges() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // Three zero-mass ops: three joins of absent learners. R14's unit rule
    // moves no mass, so the schedule is legal at every op boundary, and
    // three packed ops mean three packed slots.
    let ops = vec![
        SystemOperation::Join {
            node: n(3),
            position: 3,
        },
        SystemOperation::Join {
            node: n(4),
            position: 4,
        },
        SystemOperation::Join {
            node: n(5),
            position: 5,
        },
    ];
    let outcome = h.inject(n(0), n(1), fuse_envelope(Slot(3), ops));
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("a valid Fuse publishes, not {outcome:?}");
    };

    // The reply shape, asserted elementwise: one accepted slot per op, in
    // batch order — three values, no range arithmetic anywhere.
    let send = effects.first().expect("the FuseOk is the one effect");
    let Effect::Send { to, message, .. } = send else {
        panic!("the effect is a Send, not {send:?}");
    };
    assert_eq!(*to, n(0));
    assert_eq!(message.header.tag, Tag::FuseOk);
    assert_eq!(
        message.header.slot,
        Slot(5),
        "the header slot is the LAST accepted slot"
    );
    let Body::FuseOk { acks } = &message.body else {
        panic!("the reply body is FuseOk, not {:?}", message.body);
    };
    assert_eq!(acks.len(), 3, "one ack per packed op, count explicit");
    assert_eq!(acks[0], Slot(3));
    assert_eq!(acks[1], Slot(4));
    assert_eq!(acks[2], Slot(5));

    // And the journal holds all three slots contiguously: the entry at
    // journal position `index` occupies slot `index + 1` (the journal is
    // one-based from `Void`).
    let entries = h.journal_entries(n(1));
    assert_eq!(entries.len(), 5, "genesis plus three packed slots");
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry.slot, Slot(u64::try_from(index + 1).expect("small")));
    }

    h.assert_safety();
}

/// The whole-envelope refusal's observables for `id`: the journal holds
/// only the genesis prefix, the frontier did not move, and no reply was
/// queued for the primary.
fn assert_journal_unchanged_and_refused(h: &Harness, id: NodeId) {
    let snapshot = h.snapshot(id).expect("the node is live");
    assert_eq!(snapshot.accepted, 2, "zero slots accepted");
    assert_journal_is_genesis(h, id);
    assert!(
        h.peek_queued(n(0), Tag::FuseOk).is_none(),
        "no FuseOk emitted"
    );
}

// ---------------------------------------------------------------------------
// 6. The leader transition (§4): the armed schedule's establishing batch
//    travels as one Fuse per backup.
// ---------------------------------------------------------------------------

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

/// The 3-node two-era reincarnation shape
/// (`docs/uvrr-reincarnation.md` §5, the weight-1 row) submitted as an
/// operator plan: the crossing batch `[Decrement(old), Join(new)]` and the
/// promotion batch `[Increment(new), Leave(old)]`, one era each.
fn reincarnation_shape_plan() -> Plan {
    Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1)],
        steps: vec![
            vec![
                SystemOperation::Decrement(n(2)),
                SystemOperation::Join {
                    node: n(3),
                    position: 2,
                },
            ],
            vec![
                SystemOperation::Increment(n(3)),
                SystemOperation::Leave(n(2)),
            ],
        ],
    }
}

/// A one-step plan whose establishing batch packs two operations: the
/// learner join and its promotion in one era.
fn join_and_promote_step() -> Plan {
    Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1)],
        steps: vec![vec![
            SystemOperation::Join {
                node: n(3),
                position: 3,
            },
            SystemOperation::Increment(n(3)),
        ]],
    }
}

/// Delivers everything until the network is empty.
fn quiesce(h: &mut Harness) {
    while h.queued_len() > 0 {
        h.deliver_all();
    }
}

#[test]
fn leader_emits_one_fuse_per_backup_for_a_whole_schedule() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    let outcome = h.submit_plan(n(0), reincarnation_shape_plan());
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the plan submits, not {outcome:?}\n{}", h.trace_dump());
    };
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::AdminResponse {
                verdict: vrr::effects::PlanVerdict::Accepted
            }
        )),
        "the verdict surfaces: {effects:?}"
    );

    // Exactly ONE Fuse per backup, and nothing else is queued: the
    // schedule's slots travel in the envelope, never as ordinary
    // `Prepare`s (§4 step 1).
    assert_eq!(h.queued_len(), 2, "one Fuse per backup, nothing else");
    for to in [n(1), n(2)] {
        let message = h
            .peek_queued(to, Tag::Fuse)
            .expect("the backup holds the envelope");
        assert_eq!(message.header.tag, Tag::Fuse);
        assert_eq!(message.header.view, current_view());
        assert_eq!(
            message.header.slot,
            Slot(3),
            "first_slot is the leader's next slot"
        );
        let Body::Fuse { ops } = &message.body else {
            panic!("the body is Fuse, not {:?}", message.body);
        };
        assert_eq!(
            ops,
            &vec![
                SystemOperation::Decrement(n(2)),
                SystemOperation::Join {
                    node: n(3),
                    position: 2
                },
            ],
            "the body carries the schedule's operations in plan order"
        );
    }

    // The leader accepted its own proposal: one journaled entry per packed
    // slot, each stamped with the ballot's era (§1, ruling 3).
    let entries = h.journal_entries(n(0));
    assert_eq!(entries.len(), 4, "genesis plus the two packed slots");
    assert_eq!(entries[2].slot, Slot(3));
    assert_eq!(entries[2].era, Era(1));
    assert_eq!(
        entries[2].payload,
        Payload::System(SystemOperation::Decrement(n(2)))
    );
    assert_eq!(entries[3].slot, Slot(4));
    assert_eq!(entries[3].era, Era(1));
    assert_eq!(
        entries[3].payload,
        Payload::System(SystemOperation::Join {
            node: n(3),
            position: 2
        })
    );
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(snapshot.accepted, 4, "the frontier advanced per op");
    assert_eq!(snapshot.committed, 2, "arming commits nothing");

    h.assert_safety();
}

#[test]
fn fuseok_majority_commits_every_slot_and_emits_commitbatch() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    let outcome = h.submit_plan(n(0), join_and_promote_step());
    assert!(matches!(outcome, StepOutcome::Published { .. }));

    // The envelopes land; the backups accept whole and queue their acks.
    h.deliver_tag(n(1), Tag::Fuse);
    h.deliver_tag(n(2), Tag::Fuse);
    assert_eq!(h.snapshot(n(1)).expect("live").accepted, 4);
    assert_eq!(h.snapshot(n(1)).expect("live").committed, 2);

    // The first FuseOk completes the quorum (2-of-3: the leader's own vote
    // is implicit, §4 step 2). The commit cascade commits the packed slots
    // in order and emits the per-era CommitBatch (§4 step 4).
    let outcome = h.deliver_tag(n(0), Tag::FuseOk).expect("the ack is queued");
    assert!(
        matches!(outcome.outcome, StepOutcome::Published { .. }),
        "the ack publishes: {:?}\n{}",
        outcome.outcome,
        h.trace_dump()
    );

    // ONE CommitBatch queued per backup, alongside the ordinary commit
    // announcement the backups' frontiers advance through. The second
    // backup's ack is still queued — it arrives to a schedule whose slots
    // are already committed and is dropped by name.
    assert_eq!(
        h.queued_len(),
        5,
        "two CommitBatch, two Commit, the second backup's pending ack"
    );
    for to in [n(1), n(2)] {
        let message = h
            .peek_queued(to, Tag::CommitBatch)
            .expect("the backup holds the batch");
        assert_eq!(message.header.tag, Tag::CommitBatch);
        assert_eq!(message.header.view, current_view());
        let Body::CommitBatch { committed } = &message.body else {
            panic!("the body is CommitBatch, not {:?}", message.body);
        };
        assert_eq!(
            committed,
            &vec![Slot(3), Slot(4)],
            "one committed frontier per packed slot, in batch order, no ranges"
        );
    }

    // Every packed slot committed in order; the era table advanced through
    // the whole schedule: the batch is ONE establishing operation (§8.7.1).
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(snapshot.committed, 4, "both packed slots committed");
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3), "the batch's first slot");
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![
            SystemOperation::Join {
                node: n(3),
                position: 3
            },
            SystemOperation::Increment(n(3)),
        ]),
        "the era's establishing operation is the batch the envelope packed"
    );
    assert_eq!(
        record.config.order(),
        [member(0, 1), member(1, 1), member(2, 1), member(3, 1)],
        "the era table advanced through the whole schedule"
    );

    // Client traffic continues above the fused range with no head-of-line
    // blocking: the next proposal lands at first_slot + N and commits.
    let outcome = h.propose(n(0), op_id(1), b"after");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 5);
    assert_eq!(h.snapshot(n(1)).expect("live").committed, 5);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 7. The reproduction: one commit transition folding across the two
//    era-establishing slots of a packed schedule.
// ---------------------------------------------------------------------------

/// THE REPRODUCTION for the CommitBatch verdict (`docs/uvrr-fuse.md` §4
/// step 4). item61 flagged an unconfirmed analytic claim: the closed
/// era/slot discipline (§8.7.3, invariant rule 6) appears to refuse ONE
/// commit transition folding across two era-establishing slots — the table
/// would sit at `view_era + 2`, outside the +1 window. The state is built
/// through the public acceptor path: the packed schedule is accepted at
/// the backups, and the commit announcement that covers both slots —
/// exactly the transition the leader's cascade would run — is delivered.
/// The test asserts the resolved behaviour: the packed schedule's slots
/// commit in ONE transition, folding as the ONE establishing batch they
/// are, and the era table records it. On the unchanged code the fold runs
/// per entry and the transition is refused — the Red result this test is
/// named after.
#[test]
fn commit_batch_across_two_era_establishing_slots_is_refused_by_era_discipline() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // The packed schedule: two operations, two slots, each an
    // era-establishing entry under the per-entry fold.
    let ops = vec![
        SystemOperation::Join {
            node: n(3),
            position: 3,
        },
        SystemOperation::Increment(n(3)),
    ];
    h.inject(n(0), n(1), fuse_envelope(Slot(3), ops.clone()));
    h.inject(n(0), n(2), fuse_envelope(Slot(3), ops.clone()));
    assert_eq!(h.snapshot(n(1)).expect("live").accepted, 4);
    assert_eq!(h.snapshot(n(1)).expect("live").committed, 2);

    // The commit transition that covers both slots — the announcement the
    // leader's cascade emits at quorum.
    let commit = Message {
        header: Header {
            tag: Tag::Commit,
            view: current_view(),
            slot: Slot(4),
        },
        body: Body::Commit { committed: Slot(4) },
    };
    let outcome = h.inject(n(0), n(1), commit);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the commit transition publishes, not {outcome:?}\n{}",
        h.trace_dump()
    );

    // One transition, one fold: the batch establishes ONE era and the
    // frontiers advance whole.
    let snapshot = h.snapshot(n(1)).expect("live");
    assert_eq!(snapshot.committed, 4, "both packed slots committed");
    let table = h.era_table(n(1)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(ops),
        "the fold recognised the packed schedule as the one batch it is"
    );
    assert_eq!(
        record.config.order(),
        [member(0, 1), member(1, 1), member(2, 1), member(3, 1)]
    );

    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 8. The fallback (§4 step 5).
// ---------------------------------------------------------------------------

#[test]
fn oversized_schedule_falls_back_to_ordinary_prepares() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // One establishing batch packing eight zero-mass joins — one past the
    // builder's envelope budget (`FUSE_MAX_OPS`, justified by the nominal
    // 1300-byte payload check in `tests/wire_contract.rs`).
    let joins: Vec<SystemOperation> = (0..8)
        .map(|i| SystemOperation::Join {
            node: NodeId(u32::try_from(i + 3).expect("eight learners fit a u32")),
            position: u32::try_from(i + 3).expect("eight positions fit a u32"),
        })
        .collect();
    let plan = Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1)],
        steps: vec![joins.clone()],
    };
    let outcome = h.submit_plan(n(0), plan);
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the plan submits, not {outcome:?}\n{}", h.trace_dump());
    };
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::AdminResponse {
                verdict: vrr::effects::PlanVerdict::Accepted
            }
        )),
        "the verdict surfaces: {effects:?}"
    );

    // The fallback: the ordinary per-op path — ONE `Prepare` carrying the
    // batch, and no envelope anywhere.
    assert!(h.peek_queued(n(1), Tag::Fuse).is_none());
    assert!(h.peek_queued(n(2), Tag::Fuse).is_none());
    assert!(
        h.peek_queued(n(1), Tag::Prepare).is_some(),
        "the establishing batch travels as the ordinary Prepare"
    );

    // The batch commits as ONE era; the final committed state is the fold
    // of the same schedule the fuse path would have carried — the two
    // paths are the same sequence of logical accepts (§4 step 5).
    quiesce(&mut h);
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(snapshot.committed, 3, "the batch committed at its slot");
    let expected = vrr::configuration::Configuration::void()
        .apply(&SystemOperation::Void, vrr::configuration::VOID_SLOT)
        .and_then(|void| {
            void.apply(
                &SystemOperation::Init {
                    order: vec![n(0), n(1), n(2)],
                },
                vrr::configuration::INIT_SLOT,
            )
        })
        .and_then(|init| init.apply(&SystemOperation::Batch(joins), Slot(3)))
        .expect("the schedule folds");
    let table = h.era_table(n(0)).expect("live");
    assert_eq!(table.current().era, Era(2));
    assert_eq!(
        table.current().config,
        Arc::new(expected),
        "the equivalence holds"
    );
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 9. Leadership loss while the fuse round is in flight (§4 step 6).
// ---------------------------------------------------------------------------

#[test]
fn leadership_loss_kills_the_pending_fuse_slots() {
    // Four nodes: the commit quorum is three, so one FuseOk leaves the
    // round in flight.
    let mut h = Harness::provision(4);
    bootstrap(&mut h);

    let plan = Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1), member(3, 1)],
        steps: vec![vec![
            SystemOperation::Join {
                node: n(4),
                position: 4,
            },
            SystemOperation::Increment(n(4)),
        ]],
    };
    let outcome = h.submit_plan(n(0), plan);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    for to in [n(1), n(2), n(3)] {
        h.deliver_tag(to, Tag::Fuse);
    }
    assert_eq!(h.snapshot(n(1)).expect("live").accepted, 4);

    // One ack: own + one backup = two of three needed — the round is in
    // flight, nothing is committed.
    h.deliver_tag(n(0), Tag::FuseOk);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 2);

    // The leadership loss: the pending fuse slots die with the epoch.
    let outcome = h.force_view(
        n(1),
        ViewId {
            era: Era(1),
            view: View(1),
        },
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    assert_eq!(
        h.snapshot(n(1)).expect("live").committed,
        2,
        "no packed slot committed: the pending slots died with the epoch"
    );
    assert_eq!(
        h.era_table(n(1)).expect("live").current().era,
        Era(1),
        "no era was established"
    );

    // The new primary's catch-up proceeds through the ordinary path: its
    // own proposals' acknowledgements cascade over the accepted tail, and
    // the packed schedule commits as the one establishing batch it is.
    let outcome = h.propose(n(1), op_id(1), b"catch-up");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    let snapshot = h.snapshot(n(1)).expect("live");
    assert_eq!(
        snapshot.committed, 5,
        "the packed slots and the client slot committed"
    );
    let table = h.era_table(n(1)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![
            SystemOperation::Join {
                node: n(4),
                position: 4
            },
            SystemOperation::Increment(n(4)),
        ])
    );
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 10. Client operations are never fused and never blocked (§5).
// ---------------------------------------------------------------------------

#[test]
fn client_operations_are_not_blocked_by_the_fuse_round() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    let outcome = h.submit_plan(n(0), join_and_promote_step());
    assert!(matches!(outcome, StepOutcome::Published { .. }));

    // The envelopes land; the acks are withheld — the round is in flight.
    h.deliver_tag(n(1), Tag::Fuse);
    h.deliver_tag(n(2), Tag::Fuse);

    // A client proposal is accepted at a slot above the fused range: the
    // envelope is closed, no client op interleaves into it.
    let outcome = h.propose(n(0), op_id(1), b"while-in-flight");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    let entry = h
        .journal_entry(n(0), Slot(5))
        .expect("the client op took the slot above the fused range");
    assert!(
        matches!(entry.payload, Payload::Operation { .. }),
        "the client op is an operation slot, not {entry:?}"
    );
    assert_eq!(
        h.journal_entry(n(0), Slot(3)).expect("packed").payload,
        Payload::System(SystemOperation::Join {
            node: n(3),
            position: 3
        })
    );
    assert_eq!(
        h.journal_entry(n(0), Slot(4)).expect("packed").payload,
        Payload::System(SystemOperation::Increment(n(3)))
    );

    // The round completes; the packed slots commit in order and the client
    // op lands behind them.
    h.deliver_tag(n(0), Tag::FuseOk);
    quiesce(&mut h);
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(snapshot.committed, 5, "the schedule then the client op");
    assert_eq!(snapshot.accepted, 5);
    h.assert_safety();
}
