//! Exhaustive per-message properties: the `CommitBatch` a backup receives
//! (`docs/uvrr-fuse.md` §4).
//!
//! One message, applied to a node in an assembled state, over the plain cross
//! product of the dimensions the receive path branches on: the receiver's role
//! and the batch's view against the receiver's current view. The space is
//! exhaustive by construction under these host obligations: the identity law
//! (a lawful pair is universally unique and never recycled, so the fabricated
//! batch's frontiers name slots no lawful history could confuse); the
//! boot-gate marker states (the receiver was never halted, so it holds its
//! assembled `Normal` life); and the quorum gate fixed at construction (Q1).
//! The backups already learn commitment through the ordinary commit
//! announcement, so the batch is the leader's per-era emission, received by
//! name and dropped at every node, every state.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome};
use vrr::ids::{CrashCounter, Era, NodeId, Slot, SystemId, View, ViewId};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::replica::ViewChangeKnobs;
use vrr::wire::{Header, Tag};

const TIMEOUT: u64 = 3;

fn n(id: u32) -> NodeId {
    NodeId::new(
        SystemId::new((id + 1) as u16).expect("test system ids are small and non-zero"),
        CrashCounter::new(1).expect("one is non-zero"),
    )
}

fn view(number: u32) -> ViewId {
    ViewId {
        era: Era(1),
        view: View(number),
    }
}

/// The relation of one numeric field against the receiver's own.
#[derive(Clone, Copy)]
enum Rel {
    Less,
    Equal,
    Greater,
}

const REL_ALL: [Rel; 3] = [Rel::Less, Rel::Equal, Rel::Greater];

/// The receiver's role against the current view's primary.
#[derive(Clone, Copy)]
enum Role {
    Leader,
    Backup,
}

const ROLE_ALL: [Role; 2] = [Role::Leader, Role::Backup];

/// A three-node cluster at view 1, every node `Normal`.
fn assembled() -> Harness {
    let mut h = Harness::with_knobs(
        3,
        ViewChangeKnobs {
            primary_timeout: TIMEOUT,
            view_change_budget: usize::MAX,
        },
    );
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT {
        h.tick(n(1));
    }
    h.deliver_all();
    h
}

fn run_case(role: Role, v: Rel) {
    let mut h = assembled();
    let receiver = match role {
        Role::Leader => n(1),
        Role::Backup => n(2),
    };
    let before = h.snapshot(receiver).expect("the receiver is live");
    let current = ViewId {
        era: Era(before.era),
        view: View(before.view),
    };
    let header_view = match v {
        Rel::Less => view(current.view.0 - 1),
        Rel::Equal => current,
        Rel::Greater => view(current.view.0 + 1),
    };
    let message = Message {
        header: Header {
            tag: Tag::CommitBatch,
            view: header_view,
            slot: Slot(3),
        },
        body: Body::CommitBatch {
            committed: vec![Slot(3)],
        },
    };
    let outcome = h.inject(n(0), receiver, message);
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(
        h.fault_of(receiver).is_none(),
        "a dropped batch never faults the node"
    );
    h.check_safety().expect("the assembled cluster stays safe");
    let StepOutcome::Published { effects, .. } = outcome else {
        panic!("a named drop publishes: {outcome:?}");
    };
    assert!(
        effects.is_empty(),
        "the batch is dropped by name: {effects:?}"
    );
    assert!(
        matches!(diagnostic, Some(Diagnostic::FuseRefusal)),
        "{diagnostic:?}"
    );
    assert_eq!(
        after.committed, before.committed,
        "the batch moves no frontier"
    );
    assert_eq!(after.accepted, before.accepted);
}

#[test]
fn exhaustive_commit_batch_at_a_node() {
    for role in ROLE_ALL {
        for v in REL_ALL {
            run_case(role, v);
        }
    }
}
