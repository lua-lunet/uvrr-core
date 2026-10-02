//! The equivalence claim (`docs/uvrr-fuse.md` §8): a fused slab of N ops
//! produces the same accepts, journal entries, commits, and promise as the
//! same N ops arriving as N individual [`uvrr::wire::Tag::Prepare`]s at
//! consecutive slots, only the datagram count differing. The test drives one
//! reconfiguration schedule twice over identical clusters, once slot by slot
//! as individual `Prepare`s with nothing between them, once as a single
//! [`uvrr::wire::Tag::Fuse`] envelope, and asserts the acceptor's post-state
//! is identical: the promise (era AND view), the installed value at every
//! slot, and both frontiers. Nothing is presumed about whether the batch is
//! stepped through or installed as a slab; the observation is the public
//! snapshot and journal alone.
//!
//! The schedule is the join-and-promote batch of `tests/fuse.rs`, the shape
//! of the replacement schedules' second batch: the promote is legal only
//! against the configuration the join's own fold established, which is
//! exactly the chain fold the equivalence claim names (§8: each payload's
//! guards are evaluated against the in-memory configuration the fold of the
//! preceding payloads established).

mod harness;

use harness::Harness;
use uvrr::configuration::SystemOperation;
use uvrr::ids::{Ballot, CrashCounter, Era, NodeId, Slot, SystemId, View};
use uvrr::journal::{LogEntry, Payload};
use uvrr::message::{Body, Message};
use uvrr::wire::{Header, Tag};

/// The view every node here bootstraps into: era 1, view 0, primary `n(0)`
/// under the genesis order (§1.2).
fn current_view() -> Ballot {
    Ballot {
        era: Era(1),
        view: View(0),
    }
}

fn n(id: u32) -> NodeId {
    NodeId::new(
        SystemId::new((id + 1) as u16).expect("test system ids are small and non-zero"),
        CrashCounter::new(1).expect("one is non-zero"),
    )
}

/// The bootstrap of `tests/fuse.rs`: the genesis primary promotes itself and
/// the backups adopt view (1, 0) from the promotion's commit announcement
/// (§13.3).
fn bootstrap(h: &mut Harness) {
    h.tick_all();
    h.deliver_all();
    h.assert_safety();
}

/// The schedule: join a fresh learner at the succession end, then promote it
/// inside the era. One establishing batch, one era (`docs/uvrr-fuse.md` §1).
fn schedule(size: u32) -> Vec<SystemOperation> {
    let learner = n(size);
    vec![
        SystemOperation::Join {
            node: learner,
            position: size,
        },
        SystemOperation::Increment(learner),
    ]
}

/// One acceptor post-state, named whole so the two paths' equality is
/// asserted in one step and a divergence prints both states entire.
#[derive(Clone, PartialEq, Eq, Debug)]
struct PostState {
    /// The promise's era (`current.era`).
    era: u32,
    /// The promise's view number (`current.view`).
    view: u32,
    /// The accepted frontier.
    accepted: u64,
    /// The committed frontier.
    committed: u64,
    /// The journal whole: the installed value at every slot, era stamps
    /// included.
    slots: Vec<LogEntry>,
}

fn post_state(h: &Harness, id: NodeId) -> PostState {
    let snapshot = h.snapshot(id).expect("the acceptor is live");
    PostState {
        era: snapshot.era,
        view: snapshot.view,
        accepted: snapshot.accepted,
        committed: snapshot.committed,
        slots: h.journal_entries(id),
    }
}

/// The per-slot step-through: the schedule's ops arrive as individual
/// `Prepare`s at consecutive slots, each carrying the head ballot and the
/// sender's commit frontier, nothing between them (§8: the payloads' guards
/// are evaluated exactly as though they had arrived as separate datagrams
/// with nothing between them).
fn step_through_slot_by_slot(
    h: &mut Harness,
    primary: NodeId,
    acceptor: NodeId,
    ops: &[SystemOperation],
) {
    let frontier = h
        .snapshot(acceptor)
        .expect("the acceptor is live")
        .committed;
    for (offset, op) in ops.iter().enumerate() {
        let slot = Slot(frontier + 1 + u64::try_from(offset).expect("the offset fits u64"));
        let entry = LogEntry {
            slot,
            era: current_view().era,
            payload: Payload::System(op.clone()),
        };
        let prepare = Message {
            header: Header {
                tag: Tag::Prepare,
                view: current_view(),
                slot,
            },
            body: Body::Prepare {
                entry,
                committed: Slot(frontier),
            },
        };
        h.inject(primary, acceptor, prepare);
    }
}

/// The fused slab: the same schedule as one envelope, head ballot and
/// first slot in the header (§1).
fn fuse_whole(
    h: &mut Harness,
    primary: NodeId,
    acceptor: NodeId,
    first_slot: Slot,
    ops: &[SystemOperation],
) {
    let fuse = Message {
        header: Header {
            tag: Tag::Fuse,
            view: current_view(),
            slot: first_slot,
        },
        body: Body::Fuse { ops: ops.to_vec() },
    };
    h.inject(primary, acceptor, fuse);
}

/// The commit half of the cycle, identical for both paths: the batch commits
/// whole, the ordinary commit announcement carries the slab's tail frontier
/// (§4 step 4).
fn commit_the_batch(h: &mut Harness, primary: NodeId, acceptor: NodeId, tail: Slot) {
    let commit = Message {
        header: Header {
            tag: Tag::Commit,
            view: current_view(),
            slot: tail,
        },
        body: Body::Commit { committed: tail },
    };
    h.inject(primary, acceptor, commit);
}

fn equivalence_case(size: u32) {
    let ops = schedule(size);
    let tail = Slot(2 + ops.len() as u64);

    // The per-slot path: the schedule driven slot by slot, then the commit.
    let mut stepped = Harness::provision(size as usize);
    bootstrap(&mut stepped);
    let primary = n(0);
    let acceptor = n(1);
    step_through_slot_by_slot(&mut stepped, primary, acceptor, &ops);
    commit_the_batch(&mut stepped, primary, acceptor, tail);
    let stepped_state = post_state(&stepped, acceptor);

    // The fused path: the same schedule as one envelope, then the same
    // commit.
    let mut fused = Harness::provision(size as usize);
    bootstrap(&mut fused);
    fuse_whole(&mut fused, primary, acceptor, Slot(3), &ops);
    commit_the_batch(&mut fused, primary, acceptor, tail);
    let fused_state = post_state(&fused, acceptor);

    // The observation the equivalence claim rests on is not vacuous: the
    // envelope is accepted and committed whole (the acceptance itself is
    // pinned by `tests/fuse.rs`), so the per-slot path must end exactly
    // where it does.
    assert_eq!(
        fused_state.accepted,
        tail.0,
        "the fused slab is accepted whole\n{}",
        fused.trace_dump()
    );
    assert_eq!(
        fused_state.committed,
        tail.0,
        "the fused slab is committed whole\n{}",
        fused.trace_dump()
    );

    // The claim under test: only the datagram count differs.
    assert_eq!(
        stepped_state,
        fused_state,
        "the per-slot step-through and the fused slab disagree\nstepped trace:\n{}\nfused trace:\n{}",
        stepped.trace_dump(),
        fused.trace_dump()
    );
    stepped.assert_safety();
    fused.assert_safety();
}

#[test]
fn fuse_matches_the_per_slot_step_through_on_three_nodes() {
    equivalence_case(3);
}

#[test]
fn fuse_matches_the_per_slot_step_through_on_five_nodes() {
    equivalence_case(5);
}
