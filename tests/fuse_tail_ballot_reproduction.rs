//! Reproduction: the slab's per-slot ballot sequence.
//!
//! The slab assert `docs/uvrr-fuse.md` §3 puts on the acceptor before it
//! folds a slab is `tail_ballot > head_ballot && head_ballot >=
//! current_promise`, where `tail_ballot` is the ballot of the slab's final
//! packed slot, `first_slot + count - 1` (§1). This test drives one legal
//! fused slab through the public path and reads the ballot the library
//! itself installed at each packed slot, so the assert's premise is an
//! observation rather than an assumption.
//!
//! Red on unchanged code: the packed slots of one establishing batch carry
//! ONE ballot, so `head_ballot == tail_ballot` and the strict advance the
//! assert demands is absent.

mod harness;

use harness::Harness;
use uvrr::configuration::SystemOperation;
use uvrr::ids::{Ballot, CrashCounter, Era, NodeId, Slot, SystemId, View};
use uvrr::message::{Body, Message};
use uvrr::wire::{Header, Tag};

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

/// The join-and-promote batch of `tests/fuse_prepare_equivalence.rs`: two
/// packed slots, one establishing batch.
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

#[test]
fn the_slab_carries_one_ballot_per_packed_slot_and_strictly_advances() {
    let ops = schedule(3);
    let count = u64::try_from(ops.len()).expect("the count fits u64");
    let first_slot = Slot(3);

    let mut h = Harness::provision(3);
    h.tick_all();
    h.deliver_all();
    h.assert_safety();

    let fuse = Message {
        header: Header {
            tag: Tag::Fuse,
            view: current_view(),
            slot: first_slot,
        },
        body: Body::Fuse { ops: ops.clone() },
    };
    h.inject(n(0), n(1), fuse);
    h.assert_safety();

    let tail_slot = first_slot.0 + count - 1;
    let entries = h.journal_entries(n(1));
    let head_entry = entries
        .iter()
        .find(|entry| entry.slot == first_slot)
        .expect("the head slot is accepted");
    let tail_entry = entries
        .iter()
        .find(|entry| entry.slot.0 == tail_slot)
        .expect("the tail slot is accepted");

    // The ballot of a packed slot, as the library installed it: the era
    // stamp of the entry, under the slab's one view.
    let head_ballot = Ballot {
        era: head_entry.era,
        view: current_view().view,
    };
    let tail_ballot = Ballot {
        era: tail_entry.era,
        view: current_view().view,
    };

    assert!(
        tail_ballot > head_ballot,
        "the slab assert's premise fails on unchanged code: the packed slots \
         do not strictly advance. head_ballot={head_ballot:?} \
         tail_ballot={tail_ballot:?} (tail slot {tail_slot})\n{}",
        h.trace_dump()
    );
}
