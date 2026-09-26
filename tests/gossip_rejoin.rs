//! Rejoin gossip: the anti-wedge design as explicit message-level tests.
//!
//! The brief this corpus pins (`docs/uvrr-rejoin-gossip-and-witnesses.md`):
//! a node outside the cluster gossips to find it, the join gossip carries
//! its frontiers, every node that hears it lists the sender as a
//! gossip-witness, and the leader, only the leader, streams all phase-2s
//! and commits to that list, push-first, until the joiner promotes (and is
//! then dropped from the list) or forever (a statically registered witness
//! is never purged). The `GossipRequest` is the gap half of the same
//! gossip: any node that cannot commit in order fires it at every node,
//! and only the node that believes itself leader answers with the missing
//! range and a fresh commit, a cluster member is pushed but never listed.
//!
//! Every assertion is on the wire or on the witness list itself, not on
//! end states alone: the messages the design names are the messages the
//! tests require.

mod harness;

use harness::Harness;
use vrr::ids::{CrashCounter, Era, NodeId, OperationId, Slot, SystemId, View, ViewId};
use vrr::message::{Body, Message};
use vrr::progress::Status;
use vrr::wire::{Header, Tag};

const ALL: [NodeId; 3] = [n(0), n(1), n(2)];

const fn n(id: u32) -> NodeId {
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

fn knobs() -> vrr::replica::ViewChangeKnobs {
    vrr::replica::ViewChangeKnobs {
        primary_timeout: TIMEOUT,
        view_change_budget: usize::MAX,
    }
}

fn cluster() -> Harness {
    Harness::with_knobs(3, knobs())
}

/// The cluster with a statically registered witness: out of the roster,
/// seeded into every node's gossip-witness list at startup, never purged.
fn cluster_with_static_witness() -> Harness {
    Harness::with_static_witnesses(3, knobs(), &[n(9)])
}

fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

fn snap(h: &Harness, id: NodeId) -> vrr::progress::ProgressSnapshot {
    h.snapshot(id).expect("the node is live")
}

fn status_of(h: &Harness, id: NodeId) -> Status {
    Status::from_word(snap(h, id).status).expect("the word is a status")
}

fn current_view(h: &Harness, id: NodeId) -> ViewId {
    let snapshot = snap(h, id);
    ViewId {
        era: Era(snapshot.era),
        view: View(snapshot.view),
    }
}

fn primary_of(h: &Harness, observer: NodeId, view: ViewId) -> Option<NodeId> {
    h.era_table(observer)?
        .record(view.era)
        .and_then(|record| record.config.primary(view.view))
}

/// The node that currently believes itself primary of the cluster's view.
fn serving_primary(h: &Harness) -> NodeId {
    let observer = ALL
        .iter()
        .copied()
        .find(|&id| h.is_up(id))
        .expect("a live node observes");
    let view = current_view(h, observer);
    ALL.iter()
        .copied()
        .filter(|&id| h.is_up(id))
        .find(|&id| status_of(h, id) == Status::Normal && primary_of(h, id, view) == Some(id))
        .expect("a primary serves")
}

/// Serves one operation through the current primary, if any node serves.
fn serve(h: &mut Harness, ops: &mut u64) {
    let Some(primary) = ALL
        .iter()
        .copied()
        .find(|&id| status_of(h, id) == Status::Normal)
        .filter(|&id| primary_of(h, id, current_view(h, id)) == Some(id))
    else {
        return;
    };
    *ops += 1;
    let _ = h.propose(primary, op_id(*ops), b"o");
    h.deliver_all();
}

/// Rotates the primary after a crash: silence long enough for the live
/// `Normal` members to suspect, then the drain that completes the fence.
fn rotate(h: &mut Harness) {
    for _ in 0..=TIMEOUT {
        h.tick_all();
        h.deliver_all();
    }
}

/// The gossip request a sender fires at every node it knows, carrying its
/// frontiers (the join half) or its gap (the catch-up half).
fn gossip_request(prepared: Slot, committed: Slot) -> Message {
    Message {
        header: Header {
            tag: Tag::GossipRequest,
            view: ViewId {
                era: Era(1),
                view: View(0),
            },
            slot: Slot(0),
        },
        body: Body::GossipRequest {
            prepared,
            committed,
        },
    }
}

/// The join gossip, crash-reincarnation flavour: the bumped identity
/// announces its pair, and every node that hears the announcement lists
/// the joiner in its gossip-witness list, not only the leader, while the
/// leader's immediate push of the missed range is on the wire.
#[test]
fn the_join_gossip_lists_the_announcer_at_every_node() {
    let mut h = cluster();
    bootstrap(&mut h);
    let mut ops = 0u64;
    serve(&mut h, &mut ops);
    serve(&mut h, &mut ops);

    h.crash(n(0));
    rotate(&mut h);
    let bumped = n(0)
        .next_life()
        .expect("the test never exhausts the counter");
    h.restart_as(n(0), bumped)
        .expect("the disk reopens under the bump");
    h.reincarnate(bumped, n(0));
    h.deliver_tag(n(1), Tag::Reincarnation);
    h.deliver_tag(n(2), Tag::Reincarnation);
    assert!(
        h.peek_queued(bumped, Tag::Prepare).is_some(),
        "the leader's stream beat to the joiner is on the wire\n{}",
        h.trace_dump()
    );
    h.deliver_all();

    for id in [n(1), n(2)] {
        assert!(
            h.witnesses(id).contains(&bumped),
            "n={id:?} did not list the announcer",
        );
    }
    h.assert_safety();
}

/// A statically registered witness is a passive data sink outside the
/// roster: the leader streams it all phase-2s and commits from startup,
/// and it never appears in any configuration, it cannot vote.
#[test]
fn a_static_witness_receives_the_stream_and_never_votes() {
    let mut h = cluster_with_static_witness();
    bootstrap(&mut h);
    let mut ops = 0u64;

    // The witness is partitioned alone: everything the leader streams to
    // it is held on the wire, where the assertions read it.
    h.partition(vec![n(9)], vec![n(0), n(1), n(2)]);
    serve(&mut h, &mut ops);

    let held = h.held_summary();
    assert!(
        held.iter()
            .any(|&(_, to, tag)| to == n(9) && tag == Tag::Prepare),
        "the leader's phase-2 to the static witness is on the wire: {held:?}",
    );
    assert!(
        held.iter()
            .any(|&(_, to, tag)| to == n(9) && tag == Tag::Commit),
        "the commit announcement to the static witness is on the wire: {held:?}",
    );
    h.heal();
    h.drop_held();

    for &id in &ALL {
        assert!(
            h.witnesses(id).contains(&n(9)),
            "n={id:?} loaded the startup witness list",
        );
        assert!(
            h.era_table(id)
                .expect("the member is live")
                .current()
                .config
                .weight_of(n(9))
                .is_none(),
            "the witness is outside the roster at n={id:?}",
        );
    }
    h.assert_safety();
}

/// Failover does not interrupt the stream: every node carries the witness
/// list, so the successor leader streams on election without any fresh
/// announcement from the witness.
#[test]
fn a_successor_leader_resumes_the_witness_stream() {
    let mut h = cluster_with_static_witness();
    bootstrap(&mut h);
    let mut ops = 0u64;

    let first = serving_primary(&h);
    ops += 1;
    let _ = h.propose(first, op_id(ops), b"o");
    assert!(
        h.peek_queued(n(9), Tag::Prepare).is_some(),
        "the first leader streams the witness",
    );
    h.drop_queued(n(9));
    h.deliver_all();

    h.crash(first);
    rotate(&mut h);
    let successor_view = current_view(&h, n(1));
    let successor = serving_primary(&h);
    ops += 1;
    let _ = h.propose(successor, op_id(ops), b"o");
    let streamed = h
        .peek_queued(n(9), Tag::Prepare)
        .expect("the successor leader resumed the witness stream");
    assert_eq!(
        streamed.header.view, successor_view,
        "the resumed stream is the successor's own phase-2",
    );
    h.assert_safety();
}

/// The gap half of the gossip: a member that cannot commit in order fires
/// a `GossipRequest` at every node; the node that believes itself leader
/// answers with the missing range, and no node lists a cluster member as
/// a witness (the push, not the list, is the answer). The same one message
/// from a cold node IS the join: pushed and listed everywhere.
#[test]
fn a_gossip_request_pushes_the_missing_range_and_lists_no_member() {
    let mut h = cluster();
    bootstrap(&mut h);
    let mut ops = 0u64;
    serve(&mut h, &mut ops);
    serve(&mut h, &mut ops);

    // n2 falls behind: partitioned while the cluster serves, and the
    // held traffic dropped so a genuine gap opens.
    h.partition(vec![n(2)], vec![n(0), n(1)]);
    serve(&mut h, &mut ops);
    serve(&mut h, &mut ops);
    h.heal();
    h.drop_held();
    let gap = snap(&h, n(2));
    assert!(
        gap.committed < snap(&h, n(0)).committed,
        "the partition opened a gap",
    );

    // The gossip request carries the frontiers; only the leader answers.
    let request = gossip_request(Slot(gap.accepted), Slot(gap.committed));
    for &id in &ALL {
        h.inject(n(2), id, request.clone());
    }
    let push = h
        .peek_queued(n(2), Tag::NewState)
        .expect("the leader answered the gossip request with the missing range");
    let Body::NewState {
        entries, committed, ..
    } = push.body
    else {
        unreachable!("the peeked message is a NewState");
    };
    assert_eq!(
        entries.first().map(|entry| entry.slot),
        Some(Slot(gap.accepted + 1)),
        "the push starts exactly at the requester's frontier",
    );
    assert!(
        committed.0 >= gap.committed,
        "the push reaches at least the requester's commit frontier",
    );

    // A cluster member is never witness-listed: the push is the answer.
    for &id in &ALL {
        assert!(
            !h.witnesses(id).contains(&n(2)),
            "n={id:?} listed a cluster member as a witness",
        );
    }

    // The cold join: one `GossipRequest` carrying frontiers IS the join.
    h.boot_as(n(9)).expect("the cold identity boots");
    let cold = snap(&h, n(9));
    let join = gossip_request(Slot(cold.accepted), Slot(cold.committed));
    for &id in &ALL {
        h.inject(n(9), id, join.clone());
    }
    let pushed = h
        .peek_queued(n(9), Tag::NewState)
        .expect("the leader answered the cold join with the missed range");
    let Body::NewState { entries, .. } = pushed.body else {
        unreachable!("the peeked message is a NewState");
    };
    assert_eq!(
        entries.first().map(|entry| entry.slot),
        Some(Slot(cold.accepted + 1)),
        "the cold joiner's push starts at its own frontier",
    );
    for &id in &ALL {
        assert!(
            h.witnesses(id).contains(&n(9)),
            "n={id:?} listed the cold joiner",
        );
    }
    h.assert_safety();
}

/// The safety half of the brief, pinned: a stream message from an older
/// view than the sink holds never moves the sink backwards, the witness
/// applies commits in order or refuses them, never rewinds.
#[test]
fn a_stale_stream_message_never_moves_the_sink_backwards() {
    let mut h = cluster();
    bootstrap(&mut h);
    let mut ops = 0u64;
    serve(&mut h, &mut ops);

    h.crash(n(0));
    rotate(&mut h);
    let bumped = n(0)
        .next_life()
        .expect("the test never exhausts the counter");
    h.restart_as(n(0), bumped)
        .expect("the disk reopens under the bump");
    h.reincarnate(bumped, n(0));
    h.deliver_all();
    let before = snap(&h, bumped);

    // A forged stream chunk from a view below the sink's current: refused.
    let stale = Message {
        header: Header {
            tag: Tag::NewState,
            view: ViewId {
                era: Era(1),
                view: View(0),
            },
            slot: Slot(2),
        },
        body: Body::NewState {
            entries: Vec::new(),
            through: Slot(2),
            committed: Slot(9_999),
            more: false,
        },
    };
    h.inject(n(0), bumped, stale);
    let after = snap(&h, bumped);
    assert_eq!(after.committed, before.committed, "no rewind");
    assert_eq!(after.view, before.view, "no view regression");
    assert!(
        after.committed < 9_999,
        "the stale frontier never installed"
    );
    h.assert_safety();
}

/// Promotion closes the witness entry: when the joiner's weight commits,
/// every node scans its gossip-witness list and drops the now-voting
/// member, duplicate delivery would be safe, the drop saves the IO.
#[test]
fn promotion_removes_the_joined_voter_from_every_witness_list() {
    let mut h = cluster();
    bootstrap(&mut h);
    let mut ops = 0u64;
    serve(&mut h, &mut ops);

    h.crash(n(0));
    rotate(&mut h);
    let bumped = n(0)
        .next_life()
        .expect("the test never exhausts the counter");
    h.restart_as(n(0), bumped)
        .expect("the disk reopens under the bump");
    h.reincarnate(bumped, n(0));
    h.deliver_tag(n(1), Tag::Reincarnation);
    h.deliver_tag(n(2), Tag::Reincarnation);
    h.deliver_all();
    for id in [n(1), n(2)] {
        assert!(
            h.witnesses(id).contains(&bumped),
            "the joiner was listed before promotion",
        );
    }

    // The forced sequence seats the joiner; the promotion commit is the
    // trigger that purges it from every list.
    let mut seated = false;
    for _ in 0..400 {
        h.tick_all();
        h.deliver_all();
        let weights = h
            .era_table(bumped)
            .map(|table| {
                table
                    .current()
                    .config
                    .weight_of(bumped)
                    .map(|weight| weight.0)
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        if weights >= 1
            && h.snapshot(bumped).and_then(|s| Status::from_word(s.status)) == Some(Status::Normal)
        {
            seated = true;
            break;
        }
    }
    assert!(seated, "the joiner was never seated\n{}", h.trace_dump());
    for id in [n(1), n(2)] {
        assert!(
            !h.witnesses(id).contains(&bumped),
            "n={id:?} still lists a promoted voter",
        );
    }
    h.assert_safety();
}
