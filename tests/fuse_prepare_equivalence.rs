//! The equivalence claim (`docs/uvrr-fuse.md` §8) under the per-slot era
//! law: a fused slab of N ops produces the same accepts, journal entries —
//! the same slot, the same era stamp, the same value at every packed slot —
//! and the same promise as the same N ops arriving as N individual
//! [`uvrr::wire::Tag::Prepare`]s at consecutive slots, only the datagram
//! count differing. The test drives one reconfiguration schedule twice over
//! identical clusters, once slot by slot as individual `Prepare`s with
//! nothing between them, once as a single [`uvrr::wire::Tag::Fuse`]
//! envelope, and asserts the acceptor's post-state is identical: the
//! promise (era AND view), the installed value and era stamp at every slot,
//! and both frontiers. Nothing is presumed about whether the batch is
//! stepped through or installed as a slab; the observation is the public
//! snapshot and journal alone.
//!
//! The schedule is the join-and-promote batch of `tests/fuse.rs`: the
//! promote is legal only against the configuration the join's own fold
//! established, which is exactly the chain fold the equivalence claim names
//! (§8: each payload's guards are evaluated against the in-memory
//! configuration the fold of the preceding payloads established).
//!
//! The law's two consequences for the net:
//!
//! * The era advances once per establishing slot, so the step-through's
//!   second `Prepare` is stamped with the era the first slot's commit
//!   established — and is admissible only then, the ordinary accept path's
//!   era authorization (§8.7.3, §8.7.8) refusing an entry whose era the
//!   committed history has not established. The commit of the head slot
//!   separates the two proposals, exactly the paced emission §6 states.
//! * Both paths commit as the paced fold admits: the riderless slab's tail
//!   defers past the era window (§4 step 4), so both paths end with the
//!   head era committed, the tail accepted, and the view unmoved.
//!
//! The rider's seat is out of scope by construction: a `Nominate` exists
//! only as a batch's last sub-operation, so no bare-entry step-through
//! carries one; the ridered slab's correspondence is the commit fold's
//! segmentation, pinned row by row in `tests/fuse.rs`.

mod harness;

use harness::{Harness, provisioned_id as n};
use uvrr::configuration::SystemOperation;
use uvrr::ids::{Ballot, Era, NodeId, Slot, View};
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

/// The schedule: join a fresh learner at the succession end, then promote
/// it, one era per establishing slot (`docs/uvrr-fuse.md` §1).
fn schedule(size: u32) -> Vec<SystemOperation> {
    let learner = n(usize::try_from(size).expect("the size fits"));
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
    /// The journal whole: the installed value and era stamp at every slot.
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

/// The fused slab: the schedule as one envelope, head ballot and first slot
/// in the header (§1).
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

/// One slot of the per-slot step-through: a `Prepare` carrying the op at
/// its own slot, stamped with the era the fold of the preceding slots
/// established (§1), the sender's commit frontier riding as ever.
fn prepare_slot(
    h: &mut Harness,
    primary: NodeId,
    acceptor: NodeId,
    slot: Slot,
    era: Era,
    op: &SystemOperation,
    committed: Slot,
) {
    let prepare = Message {
        header: Header {
            tag: Tag::Prepare,
            view: current_view(),
            slot,
        },
        body: Body::Prepare {
            entry: LogEntry {
                slot,
                era,
                payload: Payload::System(op.clone()),
            },
            committed,
        },
    };
    h.inject(primary, acceptor, prepare);
}

/// A commit announcement, identical for both paths (§4 step 4).
fn commit_through(h: &mut Harness, primary: NodeId, acceptor: NodeId, frontier: Slot) {
    let commit = Message {
        header: Header {
            tag: Tag::Commit,
            view: current_view(),
            slot: frontier,
        },
        body: Body::Commit {
            committed: frontier,
        },
    };
    h.inject(primary, acceptor, commit);
}

fn equivalence_case(size: u32) {
    let ops = schedule(size);
    let head = Slot(3);
    let tail = Slot(2 + ops.len() as u64);
    let primary = n(0);
    let acceptor = n(1);

    // The fused path: the schedule as one envelope, then the commit
    // announcement covering both packed slots.
    let mut fused = Harness::provision(size as usize);
    fused.bootstrap();
    fuse_whole(&mut fused, primary, acceptor, head, &ops);
    commit_through(&mut fused, primary, acceptor, tail);

    // The per-slot path: the same schedule as separate datagrams with
    // nothing between them, the head slot's commit separating the two
    // proposals so the tail's era stamp is admissible, then the same
    // commit announcement.
    let mut stepped = Harness::provision(size as usize);
    stepped.bootstrap();
    prepare_slot(
        &mut stepped,
        primary,
        acceptor,
        head,
        Era(1),
        &ops[0],
        Slot(2),
    );
    commit_through(&mut stepped, primary, acceptor, head);
    prepare_slot(&mut stepped, primary, acceptor, tail, Era(2), &ops[1], head);
    commit_through(&mut stepped, primary, acceptor, tail);

    let fused_state = post_state(&fused, acceptor);
    let stepped_state = post_state(&stepped, acceptor);

    // The observation the equivalence claim rests on is not vacuous: the
    // envelope is accepted whole (the acceptance itself is pinned by
    // `tests/fuse.rs`), so the per-slot path must end exactly where it
    // does.
    assert_eq!(
        fused_state.accepted,
        tail.0,
        "the fused slab is accepted whole\n{}",
        fused.trace_dump()
    );

    // The claim under test: the same slot, the same era stamp, the same
    // value at every packed slot, the same promise, the same frontiers —
    // only the datagram count differs. The paced law defers the riderless
    // tail on BOTH paths: the head era commits, the tail awaits the view's
    // walk, and that deferral is part of the state the paths agree on.
    assert_eq!(
        stepped_state,
        fused_state,
        "the per-slot step-through and the fused slab disagree\nstepped trace:\n{}\nfused trace:\n{}",
        stepped.trace_dump(),
        fused.trace_dump()
    );
    assert_eq!(
        fused_state.committed,
        head.0,
        "the paced law: the head era committed, the tail deferred\n{}",
        fused.trace_dump()
    );
    stepped.assert_safety();
    fused.assert_safety();
}

#[test]
fn fuse_matches_the_per_slot_step_through_on_three_nodes() {
    equivalence_case(3);
}
