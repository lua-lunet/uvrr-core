//! The [`uvrr::wire::Tag::Fuse`] acceptor (`docs/uvrr-fuse.md` §3): one envelope,
//! one slab of consecutive slots, each at its own ballot,
//! one atomic batch. Only the wire shape changes, so every assertion
//! here runs through the ordinary accept perimeter's observables: the
//! journal (the durable configuration record, §8.7.1), the queued replies,
//! and the harness's independent safety checker. Accepting a Phase2 at a slot
//! promises that slot's ballot even when the Phase1 was lost, so the acceptor's
//! promise ends at the slab's tail ballot.
//!
//! The leader side (§4) is exercised through the public plan path: the
//! armed schedule's establishing batch travels as one [`uvrr::wire::Tag::Fuse`] per backup,
//! the [`uvrr::wire::Tag::FuseOk`] votes complete the quorum, and the commit emission is one
//! [`uvrr::wire::Tag::CommitBatch`] per era.

mod harness;

use harness::{Harness, StepOutcome};
use std::sync::Arc;

use uvrr::configuration::{Member, SystemOperation, Weight};
use uvrr::effects::Effect;
use uvrr::ids::{Ballot, CrashCounter, Era, NodeId, OperationId, Slot, SystemId, View};
use uvrr::journal::Payload;
use uvrr::message::{Body, Message};
use uvrr::observe::Diagnostic;
use uvrr::plan::Plan;
use uvrr::wire::{Header, Pack, Tag, Unpack, UnpackError};

fn n(id: u32) -> NodeId {
    NodeId::new(
        SystemId::new((id + 1) as u16).expect("test system ids are small and non-zero"),
        CrashCounter::new(1).expect("one is non-zero"),
    )
}

/// The bootstrap of `tests/reconfiguration_stepwise.rs`: the genesis
/// primary promotes itself and both backups adopt view (1, 0) from the
/// promotion's [`uvrr::wire::Tag::Commit`] announcement (§13.3).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

/// The view every node here bootstraps into: era 1, view 0, primary `n(0)`
/// under the genesis order `[0, 1, 2]` (§1.2).
fn current_view() -> Ballot {
    Ballot {
        era: Era(1),
        view: View(0),
    }
}

/// A [`uvrr::wire::Tag::Fuse`] envelope from the primary carrying `ops`, whose batch begins at
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

/// The genesis journal: [`uvrr::configuration::SystemOperation::Void`] at slot 1, [`uvrr::configuration::SystemOperation::Init`] at slot 2, nothing beyond.
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

    // The schedule: `[Join(new), Increment(new)]`, the shape of the
    // five-node replacement's second batch (`docs/uvrr-protocols.md`, the reincarnation chapter
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

    // The journal records the contiguous slab, each op at its own slot and
    // its own ballot, exactly as the equivalent sequence of `Prepare`s
    // would have journaled it (§1; the configuration record of §8.7.1
    // advanced per op): the head slot authorised by the header's era, the
    // tail slot by the era the head slot's fold established. The era table
    // itself advances at commit (§8.7.1: the commit-time
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
        "the head slot is authorised by the header's era"
    );
    assert_eq!(
        entries[2].payload,
        Payload::System(SystemOperation::Join {
            node: n(3),
            position: 3
        })
    );
    assert_eq!(entries[3].slot, Slot(4));
    assert_eq!(
        entries[3].era,
        Era(2),
        "the tail slot carries the era the head slot's fold established"
    );
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
    // (era 0, view 0), below the acceptor's current view (1, 0). Era 0 is
    // evaluable (the void record is retained) and names no primary, so the
    // sender cannot be the primary of the message's view.
    let stale = Message {
        header: Header {
            tag: Tag::Fuse,
            view: Ballot {
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
    // schedule folds as one unit, the perimeter judges the whole sequence
    // before any effect is released, so a refusal names the whole envelope
    // at its first refusing op and there is no partial fold to observe. A
    // test cannot construct the mid-batch refusal without forging a
    // schedule no planner would certify, which §3 rules out of scope.
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // The malformed-decode drop: a valid 20-byte header tagging `Fuse`,
    // then a body whose first op carries an unknown discriminant. The
    // codec refuses it (`OutOfDomain`), the host drops the datagram
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
    // reply, no diagnostic, the malformed datagram never arrived.
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
    // batch order, three values, no range arithmetic anywhere.
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
        node: n(id),
        weight: Weight(weight),
    }
}

/// The 3-node two-step reincarnation shape
/// (`docs/uvrr-protocols.md`, the reincarnation chapter §5, the weight-1 row) submitted as an
/// operator plan: the crossing batch `[Decrement(old), Join(new), Nominate]`
/// and the promotion batch `[Increment(new), Leave(old), Nominate]`, each
/// spanning one era per establishing slot, the riders keeping the leader
/// stable across the slab (`docs/uvrr-fuse.md` §1: the nomination rides
/// with the era the slot establishes). The rider arithmetic is the
/// solver's: `from` the running view, `offset` the least positive jump
/// re-electing the serving leader under the step's own succession.
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
                // Two voters after the crossing: the least jump from view
                // 0 re-electing the leader is 2.
                SystemOperation::Nominate {
                    from: View(0),
                    offset: 2,
                },
            ],
            vec![
                SystemOperation::Increment(n(3)),
                SystemOperation::Leave(n(2)),
                // Three voters after the promotion: from the running view
                // 2 the least re-electing jump is 1.
                SystemOperation::Nominate {
                    from: View(2),
                    offset: 1,
                },
            ],
        ],
    }
}

/// A one-step plan whose establishing batch packs two establishing
/// operations and the nomination rider: the learner join and its
/// promotion, one era per establishing slot (`docs/uvrr-fuse.md` §1). The
/// rider's arithmetic: four voters after the promotion, so the least
/// positive jump from view 0 re-electing the leader is 4.
fn join_and_promote_step() -> Plan {
    Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1)],
        steps: vec![vec![
            SystemOperation::Join {
                node: n(3),
                position: 3,
            },
            SystemOperation::Increment(n(3)),
            SystemOperation::Nominate {
                from: View(0),
                offset: 4,
            },
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
                verdict: uvrr::effects::PlanVerdict::Accepted
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
                SystemOperation::Nominate {
                    from: View(0),
                    offset: 2
                },
            ],
            "the body carries the schedule's operations in plan order"
        );
    }

    // The leader accepted its own proposal: one journaled entry per packed
    // slot, each stamped with the era of that slot's own ballot (§1): the
    // head slot at the header's era, each later establishing slot at the
    // era the fold of the preceding slots established, and the rider
    // sharing its carrier's era.
    let entries = h.journal_entries(n(0));
    assert_eq!(entries.len(), 5, "genesis plus the three packed slots");
    assert_eq!(entries[2].slot, Slot(3));
    assert_eq!(entries[2].era, Era(1));
    assert_eq!(
        entries[2].payload,
        Payload::System(SystemOperation::Decrement(n(2)))
    );
    assert_eq!(entries[3].slot, Slot(4));
    assert_eq!(entries[3].era, Era(2));
    assert_eq!(
        entries[3].payload,
        Payload::System(SystemOperation::Join {
            node: n(3),
            position: 2
        })
    );
    assert_eq!(entries[4].slot, Slot(5));
    assert_eq!(entries[4].era, Era(2), "the rider shares its carrier's era");
    assert_eq!(
        entries[4].payload,
        Payload::System(SystemOperation::Nominate {
            from: View(0),
            offset: 2
        })
    );
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(snapshot.accepted, 5, "the frontier advanced per op");
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
    assert_eq!(h.snapshot(n(1)).expect("live").accepted, 5);
    assert_eq!(h.snapshot(n(1)).expect("live").committed, 2);

    // The first FuseOk completes the quorum (2-of-3: the leader's own vote
    // is implicit, §4 step 2). The commit cascade commits the packed slots
    // in order — one era per establishing slot, the rider's bump carrying
    // the serving view to the slab's final era in the same advance — and
    // emits the per-era CommitBatch (§4 step 4).
    let outcome = h.deliver_tag(n(0), Tag::FuseOk).expect("the ack is queued");
    assert!(
        matches!(outcome.outcome, StepOutcome::Published { .. }),
        "the ack publishes: {:?}\n{}",
        outcome.outcome,
        h.trace_dump()
    );

    // ONE CommitBatch queued per backup, alongside the ordinary commit
    // announcement the backups' frontiers advance through. The second
    // backup's ack is still queued, it arrives to a schedule whose slots
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
            &vec![Slot(4), Slot(5)],
            "one committed frontier per slot of the slab's second era, in \
             batch order, no ranges"
        );
    }

    // Every packed slot committed in order; the era table advanced through
    // the whole schedule: one era per establishing slot (§8.7.1), the
    // serving view landing at the slab's final era on the rider's bump.
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(snapshot.committed, 5, "the packed slots committed");
    assert_eq!(
        (snapshot.era, snapshot.view),
        (3, 4),
        "the nomination carried the leader's view into the slab's final era"
    );
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3), "the join's own slot");
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Join {
            node: n(3),
            position: 3
        },
        "the head slot establishes its own era"
    );
    let record = table.record(Era(3)).expect("era 3 is recorded");
    assert_eq!(record.established_by, Slot(4), "the promotion's own slot");
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![
            SystemOperation::Increment(n(3)),
            SystemOperation::Nominate {
                from: View(0),
                offset: 4
            },
        ]),
        "the promotion and its rider fold as the second era's establishing run"
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
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 6);
    assert_eq!(h.snapshot(n(1)).expect("live").committed, 6);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 7. The pin: a `FuseOk` is ONE atomic vote; the leader counts the sender
//    on the first message in batch and telescopes (§2, §4 step 3).
// ---------------------------------------------------------------------------

/// THE PIN (`docs/uvrr-fuse.md` §2): majority is computed on the first
/// message in batch. A [`uvrr::wire::Tag::FuseOk`] is one atomic vote vouching for the whole
/// envelope, a node processes the full datagram before reading any other
/// message, so a partial or staggered acks body is wire evidence the
/// leader never examines for counting. The backup `n(1)` HAS accepted the
/// batch (its journal holds both packed slots); only its reply's acks
/// body names a proper SUBSET of the batch's slots, with the header slot
/// the last slot of the batch as the acceptor stamps. The leader must
/// count `n(1)` once, cumulatively onto every outstanding slot the header
/// covers: when the next backup's genuine [`uvrr::wire::Tag::FuseOk`] lands, both packed
/// slots hold quorum and the commit fires. The elementwise counter would
/// mark "just those slots", slot 4 never holds `n(1)`'s vote, and the
/// batch never commits, the Red result this test is named after.
#[test]
fn fuseok_is_one_atomic_vote_telescoping_the_batch() {
    // Four nodes: the commit quorum is three, so the forged subset vote
    // alone leaves the round in flight and the next genuine vote decides.
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
            // Five voters after the promotion: the least jump from view 0
            // re-electing the leader is 5.
            SystemOperation::Nominate {
                from: View(0),
                offset: 5,
            },
        ]],
    };
    let outcome = h.submit_plan(n(0), plan);
    assert!(matches!(outcome, StepOutcome::Published { .. }));

    // Two of the three backups accept the envelope; their genuine FuseOks
    // are queued at the leader. The third backup's envelope stays queued.
    h.deliver_tag(n(1), Tag::Fuse);
    h.deliver_tag(n(2), Tag::Fuse);
    assert_eq!(h.snapshot(n(1)).expect("live").accepted, 5);
    assert_eq!(h.snapshot(n(2)).expect("live").accepted, 5);

    // The forged subset: `n(1)` HAS accepted (the journal above), but its
    // reply names only the batch's first slot, the header slot the last,
    // exactly as the acceptor stamps. One atomic vote, whatever the body
    // says.
    let subset = Message {
        header: Header {
            tag: Tag::FuseOk,
            view: current_view(),
            slot: Slot(5),
        },
        body: Body::FuseOk {
            acks: vec![Slot(3)],
        },
    };
    let outcome = h.inject(n(1), n(0), subset);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the subset vote publishes, not {outcome:?}\n{}",
        h.trace_dump()
    );
    assert_eq!(
        h.snapshot(n(0)).expect("live").committed,
        2,
        "one vote of three leaves the round in flight"
    );

    // The leader re-delivers `n(1)`'s genuine FuseOk: the sender is
    // already counted on the header slot's coverage, a harmless but
    // named duplicate, exactly the ordinary rule's repeat handling.
    let outcome = h
        .deliver_tag(n(0), Tag::FuseOk)
        .expect("the genuine ack is queued");
    assert!(matches!(outcome.outcome, StepOutcome::Published { .. }));

    // The next backup's genuine FuseOk completes the quorum on every
    // packed slot, the header-slot vouch telescoped the batch, and the
    // commit fires: the cascade commits the batch whole, one era per
    // establishing slot, the rider's bump carrying the serving view to
    // the slab's final era in the same advance.
    let outcome = h
        .deliver_tag(n(0), Tag::FuseOk)
        .expect("the second ack is queued");
    assert!(
        matches!(outcome.outcome, StepOutcome::Published { .. }),
        "the ack publishes: {:?}\n{}",
        outcome.outcome,
        h.trace_dump()
    );
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(
        snapshot.committed, 5,
        "the ONE atomic vote covers the batch: every packed slot holds quorum"
    );

    // The per-era commit emission and the era table: the slab established
    // an era per establishing slot (§4 step 4).
    for to in [n(1), n(2), n(3)] {
        let message = h
            .peek_queued(to, Tag::CommitBatch)
            .unwrap_or_else(|| panic!("n{to:?} holds the batch, {:?}", h.trace_dump()));
        let Body::CommitBatch { committed } = &message.body else {
            panic!("the body is CommitBatch, not {:?}", message.body);
        };
        assert_eq!(
            committed,
            &vec![Slot(4), Slot(5)],
            "one committed frontier per slot of the slab's second era, in batch order"
        );
    }
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3), "the join's own slot");
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Join {
            node: n(4),
            position: 4
        },
        "the head slot establishes its own era"
    );
    let record = table.record(Era(3)).expect("era 3 is recorded");
    assert_eq!(record.established_by, Slot(4), "the promotion's own slot");
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![
            SystemOperation::Increment(n(4)),
            SystemOperation::Nominate {
                from: View(0),
                offset: 5
            },
        ]),
        "the promotion and its rider fold as the second era's establishing run"
    );
    assert_eq!(
        record.config.order(),
        [
            member(0, 1),
            member(1, 1),
            member(2, 1),
            member(3, 1),
            member(4, 1),
        ],
        "the era table advanced through the whole schedule"
    );

    // The third backup's envelope lands late; its FuseOk arrives to a
    // committed header slot and is dropped as harmless-but-named.
    quiesce(&mut h);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 8. The paced commit: what one commit transition folds when the packed
//    schedule spans more eras than the serving view can carry
//    (`docs/uvrr-fuse.md` §4 step 4).
// ---------------------------------------------------------------------------

/// THE PACED COMMIT (`docs/uvrr-fuse.md` §4 step 4). A riderless slab
/// carries no nomination, so nothing redeems the second era's advance in
/// the committing transition: the paced fold commits the head era and
/// defers the tail, the tail committing once the durable view has walked
/// into the era the head established. The state is built through the
/// public path: the leader proposes the packed schedule, the backups
/// accept it, the quorum's ack fires the commit, and the leader's commit
/// announcement carries the paced frontier to the backups. The deferral
/// this pins: the era window (§8.7.3, W1) stops the fold before the run
/// whose era the view cannot carry, never a split run, never a refused
/// transition.
#[test]
fn the_riderless_slab_commits_as_the_view_paces_its_eras() {
    let mut h = Harness::provision(3);
    bootstrap(&mut h);

    // The packed schedule: two establishing operations, two slots, two
    // eras under the per-slot fold — and no rider to carry the view.
    let plan = Plan {
        initial: vec![member(0, 1), member(1, 1), member(2, 1)],
        steps: vec![vec![
            SystemOperation::Join {
                node: n(3),
                position: 3,
            },
            SystemOperation::Increment(n(3)),
        ]],
    };
    let outcome = h.submit_plan(n(0), plan);
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_tag(n(1), Tag::Fuse);
    h.deliver_tag(n(2), Tag::Fuse);
    assert_eq!(h.snapshot(n(1)).expect("live").accepted, 4);
    assert_eq!(h.snapshot(n(1)).expect("live").committed, 2);

    // The first ack completes the quorum: the leader's commit transition
    // covers both packed slots, the head era commits, the tail defers —
    // the fold stops before the era the view cannot carry.
    let outcome = h
        .deliver_tag(n(0), Tag::FuseOk)
        .expect("the ack is queued while the round is in flight");
    assert!(
        matches!(outcome.outcome, StepOutcome::Published { .. }),
        "the commit transition publishes: {:?}",
        outcome.outcome
    );
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(
        snapshot.committed, 3,
        "the head era committed; the tail defers past the era window"
    );
    assert_eq!(
        (snapshot.era, snapshot.view),
        (1, 0),
        "the deferral moves no view"
    );

    // The announcement carries the paced frontier to the backup: the same
    // head-only commit, the same standing view, the head era established
    // at the head slot alone.
    let outcome = h
        .deliver_tag(n(1), Tag::Commit)
        .expect("the commit announcement is queued");
    assert!(
        matches!(outcome.outcome, StepOutcome::Published { .. }),
        "the backup's commit transition publishes: {:?}",
        outcome.outcome
    );
    let snapshot = h.snapshot(n(1)).expect("live");
    assert_eq!(snapshot.committed, 3, "the head era committed");
    assert_eq!((snapshot.era, snapshot.view), (1, 0));
    let table = h.era_table(n(1)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Join {
            node: n(3),
            position: 3
        },
        "the head slot establishes its own era"
    );
    assert!(
        table.record(Era(3)).is_none(),
        "the tail's era is not established before the view walks"
    );
    h.assert_safety();

    // The view walks into the era the head slot established (the §14.2
    // host say-so), and the tail commits behind it.
    let target = view_selecting(&h, n(0), Era(2));
    assert_eq!(
        target,
        Ballot {
            era: Era(2),
            view: View(3)
        },
        "the least view past the current one selecting the leader"
    );
    view_change_between_eras(&mut h, n(0), target, &[n(0), n(1), n(2)]);

    // The new view's first proposal gathers the votes that vouch the
    // accepted tail: the deferred run's era is the view's own successor
    // now, so the fold covers it and the client slot behind it.
    let outcome = h.propose(n(0), op_id(1), b"paced");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    let snapshot = h.snapshot(n(1)).expect("live");
    assert_eq!(
        snapshot.committed, 5,
        "the tail committed behind the walked view"
    );
    let table = h.era_table(n(1)).expect("live");
    let record = table.record(Era(3)).expect("era 3 is recorded");
    assert_eq!(record.established_by, Slot(4));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Increment(n(3)),
        "the tail slot establishes its own era"
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

    // One establishing batch packing eight zero-mass joins, one past the
    // builder's envelope budget (`FUSE_MAX_OPS`, justified by the nominal
    // 1300-byte payload check in `tests/wire_contract.rs`).
    let joins: Vec<SystemOperation> = (0..8)
        .map(|i| SystemOperation::Join {
            node: n(u32::try_from(i + 3).expect("eight learners fit a u32")),
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
                verdict: uvrr::effects::PlanVerdict::Accepted
            }
        )),
        "the verdict surfaces: {effects:?}"
    );

    // The fallback: the ordinary per-op path, ONE `Prepare` carrying the
    // batch, and no envelope anywhere.
    assert!(h.peek_queued(n(1), Tag::Fuse).is_none());
    assert!(h.peek_queued(n(2), Tag::Fuse).is_none());
    assert!(
        h.peek_queued(n(1), Tag::Prepare).is_some(),
        "the establishing batch travels as the ordinary Prepare"
    );

    // The batch commits as ONE era; the final committed state is the fold
    // of the same schedule the fuse path would have carried, the two
    // paths are the same sequence of logical accepts (§4 step 5).
    quiesce(&mut h);
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(snapshot.committed, 3, "the batch committed at its slot");
    let expected = uvrr::configuration::Configuration::void()
        .apply(&SystemOperation::Void, uvrr::configuration::VOID_SLOT)
        .and_then(|void| {
            void.apply(
                &SystemOperation::Init {
                    order: vec![n(0), n(1), n(2)],
                },
                uvrr::configuration::INIT_SLOT,
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

    // The packed schedule carries no rider: the pending round has nothing
    // to carry a surviving view across the slab's eras.
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

    // One ack: own + one backup = two of three needed, the round is in
    // flight, nothing is committed.
    h.deliver_tag(n(0), Tag::FuseOk);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 2);

    // The leadership loss: the pending fuse slots die with the epoch.
    let outcome = h.force_view(
        n(1),
        Ballot {
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
    // the paced fold commits what the new view can carry — the head era
    // alone; the tail's era is past the window and defers, taking the
    // client slot with it, the frontier a prefix as ever.
    let outcome = h.propose(n(1), op_id(1), b"catch-up");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    let snapshot = h.snapshot(n(1)).expect("live");
    assert_eq!(
        snapshot.committed, 3,
        "the head era committed; the tail and the client slot defer past the era window"
    );
    let table = h.era_table(n(1)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Join {
            node: n(4),
            position: 4
        },
        "the head slot establishes its own era"
    );
    assert!(
        table.record(Era(3)).is_none(),
        "the tail's era waits for the view to pace it"
    );

    // The host paces the view into the era the head established, and the
    // next proposal's acknowledgements vouch the accepted tail: the
    // deferred run, the stranded client slot, and the new proposal commit
    // in slot order.
    let target = view_selecting(&h, n(1), Era(2));
    view_change_between_eras(&mut h, n(1), target, &[n(0), n(1), n(2), n(3)]);
    let outcome = h.propose(n(1), op_id(2), b"paced");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    let snapshot = h.snapshot(n(1)).expect("live");
    assert_eq!(
        snapshot.committed, 6,
        "the tail, the stranded client slot, and the new proposal committed"
    );
    let table = h.era_table(n(1)).expect("live");
    let record = table.record(Era(3)).expect("era 3 is recorded");
    assert_eq!(record.established_by, Slot(4));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Increment(n(4)),
        "the tail slot establishes its own era"
    );
    assert_eq!(
        record.config.order(),
        [
            member(0, 1),
            member(1, 1),
            member(2, 1),
            member(3, 1),
            member(4, 1)
        ],
        "the schedule's final configuration"
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

    // The envelopes land; the acks are withheld, the round is in flight.
    h.deliver_tag(n(1), Tag::Fuse);
    h.deliver_tag(n(2), Tag::Fuse);

    // A client proposal is accepted at a slot above the fused range: the
    // envelope is closed, no client op interleaves into it.
    let outcome = h.propose(n(0), op_id(1), b"while-in-flight");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    let entry = h
        .journal_entry(n(0), Slot(6))
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
    assert_eq!(
        h.journal_entry(n(0), Slot(5)).expect("packed").payload,
        Payload::System(SystemOperation::Nominate {
            from: View(0),
            offset: 4
        }),
        "the rider packs its own slot, sharing its carrier's era"
    );

    // The client op's acks gather while the fuse round is in flight: the
    // serving view still stands, so they count. The nomination bump the
    // slab's commit publishes retires the view — an ack arriving after it
    // is the stale-view drop of every view succession — so the op gathers
    // its quorum before the round's ack, never blocked by it.
    h.deliver_tag(n(1), Tag::Prepare);
    h.deliver_tag(n(2), Tag::Prepare);
    h.deliver_tag(n(0), Tag::PrepareOk);
    h.deliver_tag(n(0), Tag::PrepareOk);

    // The round completes; the packed slots commit in order and the client
    // op lands behind them, in the same advance.
    h.deliver_tag(n(0), Tag::FuseOk);
    quiesce(&mut h);
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(
        snapshot.committed,
        6,
        "the schedule then the client op\n{}",
        h.trace_dump()
    );
    assert_eq!(snapshot.accepted, 6);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 11. The full forced schedule, end to end through the fuse path (§8 of
//     `docs/uvrr-fuse.md`): the canonical two-era 3-node split and the
//     six-era 5-node weighted sequence, submitted as one operator plan,
//     one round trip per era, the era table and the final configuration
//     asserted at every row.
// ---------------------------------------------------------------------------

/// Delivers queued FuseOk answers to `leader` one at a time until the
/// committed frontier reaches `through`. Each delivery is one published
/// step; the ack that completes the quorum fires the commit, and any ack
/// after it arrives to a committed header slot and drops by name.
fn fuse_acks_until_committed(h: &mut Harness, leader: NodeId, through: u64) {
    while h.snapshot(leader).expect("live").committed < through {
        let outcome = h
            .deliver_tag(leader, Tag::FuseOk)
            .expect("a FuseOk is queued while the round is in flight");
        assert!(
            matches!(outcome.outcome, StepOutcome::Published { .. }),
            "the ack publishes: {:?}\n{}",
            outcome.outcome,
            h.trace_dump()
        );
    }
}

/// Asserts the queue holds exactly one datagram per named recipient, all
/// with `tag`, the per-era emission shape: one per backup, nothing else.
/// The caller quiesces between eras, so the queue is otherwise empty and
/// the count is the whole emission.
fn assert_one_queued_per(h: &Harness, recipients: &[NodeId], tag: Tag) {
    assert_eq!(
        h.queued_len(),
        recipients.len(),
        "exactly one {tag:?} per recipient, nothing else\n{}",
        h.trace_dump()
    );
    for to in recipients {
        let message = h
            .peek_queued(*to, tag)
            .unwrap_or_else(|| panic!("{to:?} holds the {tag:?}, {:?}", h.trace_dump()));
        assert_eq!(message.header.tag, tag);
    }
}

/// Ticks the plan machine's leader and asserts the continuation proposed.
fn propose_next_step(h: &mut Harness, leader: NodeId) {
    let outcome = h.tick(leader);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the continuation proposes: {outcome:?}\n{}",
        h.trace_dump()
    );
}

/// The wire half of one fused era: exactly ONE Fuse per named backup
/// (the RTT accounting, the queue is otherwise empty), the envelopes
/// land, the acks telescope, the commit fires. The caller asserts the
/// era row afterwards.
fn fuse_round(h: &mut Harness, leader: NodeId, backups: &[NodeId], committed_through: u64) {
    assert_one_queued_per(h, backups, Tag::Fuse);
    for &to in backups {
        if let Some(delivery) = h.deliver_tag(to, Tag::Fuse) {
            assert!(
                matches!(
                    delivery.outcome,
                    StepOutcome::Published { .. } | StepOutcome::NodeDown
                ),
                "the envelope lands or is undeliverable: {:?}",
                delivery.outcome
            );
        }
    }
    fuse_acks_until_committed(h, leader, committed_through);
    quiesce(h);
    h.assert_safety();
}

/// The wire half of one ordinary era: exactly ONE [`uvrr::wire::Tag::Prepare`] per named
/// backup, a single-op batch needs no envelope (§4 step 1), and no Fuse
/// anywhere. The commit completes on the ordinary PrepareOk cascade.
fn prepare_round(h: &mut Harness, leader: NodeId, backups: &[NodeId], committed_through: u64) {
    assert_one_queued_per(h, backups, Tag::Prepare);
    assert!(
        backups
            .iter()
            .all(|to| h.peek_queued(*to, Tag::Fuse).is_none()),
        "no envelope on an ordinary era"
    );
    h.deliver_all();
    assert_eq!(
        h.snapshot(leader).expect("live").committed,
        committed_through,
        "the ordinary establishing batch committed"
    );
    quiesce(h);
    h.assert_safety();
}

/// The least view in `era` past the node's current one whose voter-only
/// succession (§8.4) names `leader` primary: the between-eras view change
/// that keeps the plan machine's leader in the chair.
fn view_selecting(h: &Harness, leader: NodeId, era: Era) -> Ballot {
    let snapshot = h.snapshot(leader).expect("live");
    let table = h.era_table(leader).expect("live");
    let record = table
        .record(era)
        .unwrap_or_else(|| panic!("era {era:?} is established"));
    for index in snapshot.view + 1.. {
        let view = View(index);
        if record.config.primary(view) == Some(leader) {
            return Ballot { era, view };
        }
    }
    unreachable!("a view selecting the leader is representable")
}

/// Drives the ordinary view change into `target` with `leader` as both
/// the driver and the designated primary, asserting every live node
/// installs it.
fn view_change_between_eras(h: &mut Harness, leader: NodeId, target: Ballot, live: &[NodeId]) {
    let outcome = h.force_view(leader, target);
    assert!(
        matches!(outcome, StepOutcome::Published { .. }),
        "the forced fence publishes: {outcome:?}\n{}",
        h.trace_dump()
    );
    quiesce(h);
    for &id in live {
        let snapshot = h
            .snapshot(id)
            .unwrap_or_else(|| panic!("{id:?} is not live\n{}", h.trace_dump()));
        assert_eq!(
            (snapshot.era, snapshot.view),
            (target.era.0, target.view.0),
            "{id:?} installs the target"
        );
    }
    h.assert_safety();
}

/// The two-step 3-node canonical split (`docs/uvrr-protocols.md`, the reincarnation chapter §5)
/// driven end to end through the fuse path: one Fuse per backup per step,
/// the acks telescoping, the commits firing per step, and the final
/// configuration the plan's steps reach. Under the per-slot era law each
/// step spans one era per establishing slot, and the nomination rider
/// carries the serving view across the slab's eras in the committing
/// transition, so the machine's leader stays in the chair with no
/// between-steps view change. Two round trips for the whole forced
/// schedule, single digits, as §6 promises.
#[test]
fn full_forced_reincarnation_schedule_travels_the_fuse_path_on_three_nodes() {
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
                verdict: uvrr::effects::PlanVerdict::Accepted
            }
        )),
        "the verdict surfaces: {effects:?}"
    );

    // The crossing step proposed WITH the verdict: two establishing ops
    // and the nomination rider, one Fuse per backup of the era-1 cluster,
    // the body carrying the schedule in plan order, each packed slot at
    // its own ballot.
    for to in [n(1), n(2)] {
        let message = h
            .peek_queued(to, Tag::Fuse)
            .expect("the backup holds the envelope");
        assert_eq!(message.header.view, current_view());
        assert_eq!(message.header.slot, Slot(3), "first_slot");
        assert_eq!(
            message.body,
            Body::Fuse {
                ops: vec![
                    SystemOperation::Decrement(n(2)),
                    SystemOperation::Join {
                        node: n(3),
                        position: 2
                    },
                    SystemOperation::Nominate {
                        from: View(0),
                        offset: 2
                    },
                ]
            }
        );
    }
    fuse_round(&mut h, n(0), &[n(1), n(2)], 5);

    // The era rows: one era per establishing slot — the crossing's
    // decrement establishes era 2 at its own slot, the join and its rider
    // fold as era 3's establishing run.
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Decrement(n(2)),
        "the head slot establishes its own era"
    );
    assert_eq!(
        record.config.order(),
        [member(0, 1), member(1, 1), member(2, 0)],
        "the draining era: the old identity at zero"
    );
    let record = table.record(Era(3)).expect("era 3 is recorded");
    assert_eq!(record.established_by, Slot(4));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![
            SystemOperation::Join {
                node: n(3),
                position: 2
            },
            SystemOperation::Nominate {
                from: View(0),
                offset: 2
            },
        ]),
        "the join and its rider fold as the era's establishing run"
    );
    assert_eq!(
        record.config.order(),
        [member(0, 1), member(1, 1), member(3, 0), member(2, 0)],
        "the crossing era: the old identity at zero, the new one joined at zero"
    );

    // The rider's bump carried the serving view into the slab's final era
    // in the committing transition: no view change between the steps.
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(
        (snapshot.era, snapshot.view),
        (3, 2),
        "the nomination kept the leader stable across the slab's eras"
    );

    // The eviction step: three backups of the era-3 cluster (the weight-0
    // standbys included, they accept and journal, their acks count
    // nothing), one Fuse each. The schedule's second round trip.
    propose_next_step(&mut h, n(0));
    fuse_round(&mut h, n(0), &[n(1), n(3), n(2)], 8);
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(4)).expect("era 4 is recorded");
    assert_eq!(record.established_by, Slot(6));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Increment(n(3)),
        "the promotion establishes its own era"
    );
    let record = table.record(Era(5)).expect("era 5 is recorded");
    assert_eq!(record.established_by, Slot(7));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![
            SystemOperation::Leave(n(2)),
            SystemOperation::Nominate {
                from: View(2),
                offset: 1
            },
        ])
    );
    assert_eq!(
        record.config.order(),
        [member(0, 1), member(1, 1), member(3, 1)],
        "the final configuration: the old identity evicted, the new one voting"
    );
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(
        (snapshot.era, snapshot.view),
        (5, 3),
        "the second rider carried the view across the eviction slab's eras"
    );

    // The client stream is never blocked: the next proposal lands above
    // the fused range and commits under the final configuration.
    let outcome = h.propose(n(0), op_id(1), b"after");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 9);
    h.assert_safety();
}

/// The 5-node weighted sequence
/// (`docs/uvrr-protocols.md`, the reincarnation chapter §5, rules §6) driven end to end through
/// the fuse path under the per-slot era law: the solitary scaling batches
/// travel as ordinary establishing [`uvrr::wire::Tag::Prepare`]s and their era
/// boundaries are crossed by the host's view changes (R13 rides no rider);
/// every other step carries its nomination rider and commits its whole
/// slab in one advance, the serving view landing at the slab's final era.
/// Six steps, one round trip each, and the era table advances one era per
/// establishing slot through the eight configurations the schedule names.
#[test]
fn full_forced_reincarnation_schedule_travels_the_fuse_path_on_five_nodes() {
    let mut h = Harness::provision(5);
    bootstrap(&mut h);

    // The schedule (rules §6): `[DOUBLE]`, `[JOIN(new), INCREMENT(new)]`,
    // `[DECREMENT(old)]`, `[DECREMENT(old), LEAVE(old)]`, `[INCREMENT(new)]`,
    // `[HALVE]`, old `n(4)`, new `n(5)` joined at the old identity's
    // succession position. Every non-scaling step carries its `Nominate`
    // rider (the NOMINATE chapter's emission): `from` the running view,
    // `offset` the least positive jump re-electing the leader under the
    // step's own succession. The plan carries it as one operator artefact.
    let plan = Plan {
        initial: vec![
            member(0, 1),
            member(1, 1),
            member(2, 1),
            member(3, 1),
            member(4, 1),
        ],
        steps: vec![
            vec![SystemOperation::Double],
            vec![
                SystemOperation::Join {
                    node: n(5),
                    position: 4,
                },
                SystemOperation::Increment(n(5)),
                SystemOperation::Nominate {
                    from: View(5),
                    offset: 1,
                },
            ],
            vec![
                SystemOperation::Decrement(n(4)),
                SystemOperation::Nominate {
                    from: View(6),
                    offset: 6,
                },
            ],
            vec![
                SystemOperation::Decrement(n(4)),
                SystemOperation::Leave(n(4)),
                SystemOperation::Nominate {
                    from: View(12),
                    offset: 3,
                },
            ],
            vec![
                SystemOperation::Increment(n(5)),
                SystemOperation::Nominate {
                    from: View(15),
                    offset: 5,
                },
            ],
            vec![SystemOperation::Halve],
        ],
    };
    let outcome = h.submit_plan(n(0), plan);
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("the plan submits, not {outcome:?}\n{}", h.trace_dump());
    };
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::AdminResponse {
                verdict: uvrr::effects::PlanVerdict::Accepted
            }
        )),
        "the verdict surfaces: {effects:?}"
    );

    // `[DOUBLE]` is solitary (R13): the ordinary establishing `Prepare`,
    // one per backup, no envelope. Proposed with the verdict.
    prepare_round(&mut h, n(0), &[n(1), n(2), n(3), n(4)], 3);
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![SystemOperation::Double])
    );
    assert_eq!(
        record.config.order(),
        [
            member(0, 2),
            member(1, 2),
            member(2, 2),
            member(3, 2),
            member(4, 2)
        ]
    );

    // The scaling step rides no rider: the host's view change crosses the
    // boundary. The doubled voters keep the leader.
    let target = view_selecting(&h, n(0), Era(2));
    assert_eq!(
        target,
        Ballot {
            era: Era(2),
            view: View(5)
        }
    );
    view_change_between_eras(&mut h, n(0), target, &[n(0), n(1), n(2), n(3), n(4)]);

    // The introduce step packs two establishing ops and the rider: ONE
    // Fuse per backup of the doubled era-2 cluster. The slab spans eras 2
    // and 3, establishes eras 3 and 4, and the rider lands the serving
    // view at the slab's final era in the committing transition.
    propose_next_step(&mut h, n(0));
    fuse_round(&mut h, n(0), &[n(1), n(2), n(3), n(4)], 6);
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(3)).expect("era 3 is recorded");
    assert_eq!(record.established_by, Slot(4));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Join {
            node: n(5),
            position: 4
        },
        "the head slot establishes its own era"
    );
    assert_eq!(
        record.config.order(),
        [
            member(0, 2),
            member(1, 2),
            member(2, 2),
            member(3, 2),
            member(5, 0),
            member(4, 2)
        ]
    );
    let record = table.record(Era(4)).expect("era 4 is recorded");
    assert_eq!(record.established_by, Slot(5));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![
            SystemOperation::Increment(n(5)),
            SystemOperation::Nominate {
                from: View(5),
                offset: 1
            },
        ]),
        "the promotion and its rider fold as the era's establishing run"
    );
    assert_eq!(
        record.config.order(),
        [
            member(0, 2),
            member(1, 2),
            member(2, 2),
            member(3, 2),
            member(5, 1),
            member(4, 2)
        ]
    );
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!(
        (snapshot.era, snapshot.view),
        (4, 6),
        "the rider carried the view across the introduce slab's eras"
    );

    // The first drain step packs the `DECREMENT(old)` and its rider: ONE
    // Fuse per backup of the era-4 cluster. The new identity (a weight-1
    // voter now) and the old identity both receive it.
    propose_next_step(&mut h, n(0));
    fuse_round(&mut h, n(0), &[n(1), n(2), n(3), n(5), n(4)], 8);
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(5)).expect("era 5 is recorded");
    assert_eq!(record.established_by, Slot(7));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![
            SystemOperation::Decrement(n(4)),
            SystemOperation::Nominate {
                from: View(6),
                offset: 6
            },
        ])
    );
    assert_eq!(
        record.config.order(),
        [
            member(0, 2),
            member(1, 2),
            member(2, 2),
            member(3, 2),
            member(5, 1),
            member(4, 1)
        ]
    );
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!((snapshot.era, snapshot.view), (5, 12));

    // The remove step packs two establishing ops and the rider: ONE Fuse
    // per backup of the era-5 cluster; the old identity leaves. The slab
    // spans eras 5 and 6, establishes eras 6 and 7.
    propose_next_step(&mut h, n(0));
    fuse_round(&mut h, n(0), &[n(1), n(2), n(3), n(5), n(4)], 11);
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(6)).expect("era 6 is recorded");
    assert_eq!(record.established_by, Slot(9));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Decrement(n(4)),
        "the head slot establishes its own era"
    );
    assert_eq!(
        record.config.order(),
        [
            member(0, 2),
            member(1, 2),
            member(2, 2),
            member(3, 2),
            member(5, 1),
            member(4, 0)
        ]
    );
    let record = table.record(Era(7)).expect("era 7 is recorded");
    assert_eq!(record.established_by, Slot(10));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![
            SystemOperation::Leave(n(4)),
            SystemOperation::Nominate {
                from: View(12),
                offset: 3
            },
        ])
    );
    assert_eq!(
        record.config.order(),
        [
            member(0, 2),
            member(1, 2),
            member(2, 2),
            member(3, 2),
            member(5, 1)
        ]
    );
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!((snapshot.era, snapshot.view), (7, 15));

    // The corner step packs the `INCREMENT(new)` and its rider: the
    // joiner reaches the doubled corner that makes the final halve
    // integral. ONE Fuse per backup of the era-7 cluster.
    propose_next_step(&mut h, n(0));
    fuse_round(&mut h, n(0), &[n(1), n(2), n(3), n(5)], 13);
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(8)).expect("era 8 is recorded");
    assert_eq!(record.established_by, Slot(12));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![
            SystemOperation::Increment(n(5)),
            SystemOperation::Nominate {
                from: View(15),
                offset: 5
            },
        ])
    );
    assert_eq!(
        record.config.order(),
        [
            member(0, 2),
            member(1, 2),
            member(2, 2),
            member(3, 2),
            member(5, 2)
        ]
    );
    let snapshot = h.snapshot(n(0)).expect("live");
    assert_eq!((snapshot.era, snapshot.view), (8, 20));

    // The final `HALVE` is solitary: the ordinary path, and the
    // sequence's last round trip.
    propose_next_step(&mut h, n(0));
    prepare_round(&mut h, n(0), &[n(1), n(2), n(3), n(5)], 14);
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(9)).expect("era 9 is recorded");
    assert_eq!(record.established_by, Slot(14));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Batch(vec![SystemOperation::Halve])
    );
    assert_eq!(
        record.config.order(),
        [
            member(0, 1),
            member(1, 1),
            member(2, 1),
            member(3, 1),
            member(5, 1)
        ],
        "the final configuration: five voting members at unit weight, the \
         old identity replaced"
    );

    // The host's view change crosses the scaling boundary: the old
    // identity left with era 7, it is addressed by nothing now and
    // installs nothing; the joiner was never booted in this harness —
    // it exists as a journal identity alone — so the installing set is
    // the four live voters.
    let target = view_selecting(&h, n(0), Era(9));
    assert_eq!(
        target,
        Ballot {
            era: Era(9),
            view: View(25)
        }
    );
    view_change_between_eras(&mut h, n(0), target, &[n(0), n(1), n(2), n(3)]);

    // The client stream is never blocked by the sequence.
    let outcome = h.propose(n(0), op_id(1), b"after");
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    quiesce(&mut h);
    assert_eq!(h.snapshot(n(0)).expect("live").committed, 15);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// 12. Even-sized configurations, unit voting weights (§8.5: even-sized
//     clusters optimisation): the eager strict majority of an even
//     total, `N/2 + 1` votes including the leader's, decides the fused
//     batch. The acks telescope, so the batch commits whole at exactly
//     that many acks, and the per-era `CommitBatch` joins the ordinary
//     commit announcement.
// ---------------------------------------------------------------------------

/// The even-sized commit-quorum choreography shared by the 4-node and
/// 6-node tests: the join-and-promote schedule, rider riding the
/// promotion slot, travels as one Fuse per backup; the named number of
/// acks leaves the round in flight and the NEXT ack commits the batch
/// whole in one transition; a further ack arrives to a committed header
/// slot and drops by name; the [`uvrr::wire::Tag::CommitBatch`] names one
/// committed frontier per packed slot of the slab's second era, no
/// ranges, and the ordinary commit announcement advances the backups.

#[test]
fn even_sized_four_node_cluster_commits_the_fused_batch_with_an_eager_majority() {
    let mut h = Harness::provision(4);
    bootstrap(&mut h);

    // Four unit voters: the strict majority of the even total is three
    // (§8.4's eager `2n → n+1`), the leader plus TWO acks, one fewer
    // than the odd-sized equivalent would demand of the next size up.
    // The schedule carries its nomination rider: five voters after the
    // promotion, so the least positive jump from view 0 re-electing the
    // leader is 5.
    let outcome = h.submit_plan(
        n(0),
        Plan {
            initial: vec![member(0, 1), member(1, 1), member(2, 1), member(3, 1)],
            steps: vec![vec![
                SystemOperation::Join {
                    node: n(4),
                    position: 4,
                },
                SystemOperation::Increment(n(4)),
                SystemOperation::Nominate {
                    from: View(0),
                    offset: 5,
                },
            ]],
        },
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));

    // Three backups, one Fuse each: the whole emission, one round trip.
    assert_one_queued_per(&h, &[n(1), n(2), n(3)], Tag::Fuse);
    // One ack: the leader's implicit vote plus one, two of three, the
    // round stays in flight.
    h.deliver_tag(n(1), Tag::Fuse);
    h.deliver_tag(n(0), Tag::FuseOk);
    assert_eq!(
        h.snapshot(n(0)).expect("live").committed,
        2,
        "two of three is not the eager majority of four"
    );
    // The second ack completes it: ALL packed slots commit in ONE
    // transition (the telescoping vouch), the rider carrying the serving
    // view across the slab's eras in the same advance.
    h.deliver_tag(n(2), Tag::Fuse);
    h.deliver_tag(n(0), Tag::FuseOk);
    assert_eq!(
        h.snapshot(n(0)).expect("live").committed,
        5,
        "the eager majority commits the batch whole"
    );

    // The per-era emission: one `CommitBatch` per backup naming one
    // committed frontier per packed slot of the slab's second era, no
    // ranges, and the ordinary commit announcement is what advances the
    // backups.
    for to in [n(1), n(2), n(3)] {
        let message = h
            .peek_queued(to, Tag::CommitBatch)
            .expect("the backup holds the batch");
        assert_eq!(message.header.view, current_view());
        assert_eq!(
            message.body,
            Body::CommitBatch {
                committed: vec![Slot(4), Slot(5)]
            }
        );
    }
    // The `CommitBatch` is received by name and dropped: the backup's
    // frontier moves only through the ordinary commit announcement.
    h.deliver_tag(n(1), Tag::CommitBatch);
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::FuseRefusal),
        "the batch emission is dropped by name"
    );
    assert_eq!(
        h.snapshot(n(1)).expect("live").committed,
        2,
        "the drop moved no frontier"
    );
    h.deliver_tag(n(1), Tag::Commit);
    assert_eq!(
        h.snapshot(n(1)).expect("live").committed,
        5,
        "the ordinary commit announcement advances the backup"
    );
    quiesce(&mut h);

    // The era rows: one era per establishing slot, the rider sharing its
    // carrier's era, the joiner voting.
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Join {
            node: n(4),
            position: 4
        },
        "the head slot establishes its own era"
    );
    let record = table.record(Era(3)).expect("era 3 is recorded");
    assert_eq!(record.established_by, Slot(4));
    assert_eq!(
        record.config.order(),
        [
            member(0, 1),
            member(1, 1),
            member(2, 1),
            member(3, 1),
            member(4, 1)
        ]
    );
    h.assert_safety();
}

#[test]
fn even_sized_six_node_cluster_commits_the_fused_batch_with_an_eager_majority() {
    let mut h = Harness::provision(6);
    bootstrap(&mut h);

    // Six unit voters: the strict majority of the even total is four,
    // the leader plus THREE acks. The schedule carries its nomination
    // rider: seven voters after the promotion, so the least positive
    // jump from view 0 re-electing the leader is 7.
    let outcome = h.submit_plan(
        n(0),
        Plan {
            initial: vec![
                member(0, 1),
                member(1, 1),
                member(2, 1),
                member(3, 1),
                member(4, 1),
                member(5, 1),
            ],
            steps: vec![vec![
                SystemOperation::Join {
                    node: n(6),
                    position: 6,
                },
                SystemOperation::Increment(n(6)),
                SystemOperation::Nominate {
                    from: View(0),
                    offset: 7,
                },
            ]],
        },
    );
    assert!(matches!(outcome, StepOutcome::Published { .. }));

    // Five backups, one Fuse each: the whole emission, one round trip.
    assert_one_queued_per(&h, &[n(1), n(2), n(3), n(4), n(5)], Tag::Fuse);

    // Two acks: three of four, the round stays in flight.
    for &to in &[n(1), n(2), n(3)] {
        h.deliver_tag(to, Tag::Fuse);
    }
    h.deliver_tag(n(0), Tag::FuseOk);
    h.deliver_tag(n(0), Tag::FuseOk);
    assert_eq!(
        h.snapshot(n(0)).expect("live").committed,
        2,
        "three of four is not the eager majority of six"
    );
    // The third ack completes it: the batch commits whole, the rider
    // carrying the serving view across the slab's eras in the one
    // transition.
    h.deliver_tag(n(0), Tag::FuseOk);
    assert_eq!(
        h.snapshot(n(0)).expect("live").committed,
        5,
        "the eager majority commits the batch whole"
    );
    quiesce(&mut h);

    // The era rows: one era per establishing slot, the batch's span
    // established over seven members.
    let table = h.era_table(n(0)).expect("live");
    let record = table.record(Era(2)).expect("era 2 is recorded");
    assert_eq!(record.established_by, Slot(3));
    assert_eq!(
        record.establishing_operation,
        SystemOperation::Join {
            node: n(6),
            position: 6
        },
        "the head slot establishes its own era"
    );
    let record = table.record(Era(3)).expect("era 3 is recorded");
    assert_eq!(record.established_by, Slot(4));
    assert_eq!(
        record.config.order(),
        [
            member(0, 1),
            member(1, 1),
            member(2, 1),
            member(3, 1),
            member(4, 1),
            member(5, 1),
            member(6, 1)
        ]
    );
    h.assert_safety();
}
