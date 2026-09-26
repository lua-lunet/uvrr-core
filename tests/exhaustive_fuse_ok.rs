//! Exhaustive per-message properties: the `FuseOk` the serving primary counts
//! (`docs/uvrr-fuse.md` §2, §4 step 3).
//!
//! One message, applied to the primary in an assembled state, over the plain
//! cross product of the dimensions the handler branches on: the sender's
//! membership, the ack's view against the primary's current view, and the
//! ack's header slot against the outstanding proposal. The space is exhaustive
//! by construction under these host obligations: the identity law (a lawful
//! pair is universally unique and never recycled, so a minted foreign sender
//! is outside the configuration for the life of the test); the boot-gate
//! marker states (the primary was never halted, so it holds its assembled
//! `Normal` life); and the quorum gate fixed at construction (Q1). One
//! `FuseOk` is ONE atomic vote vouching for the whole envelope (§2): the acks
//! body is the acceptor's wire evidence and is never examined for counting,
//! so the fabricated body carries the batch's slots and the counting runs on
//! the header's coverage alone.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_id, mint_pair};
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, Era, NodeId, OperationId, Slot, SystemId, View, ViewId};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::replica::ViewChangeKnobs;
use vrr::wire::{Header, Tag};

const TIMEOUT: u64 = 3;
const PAYLOAD: &[u8] = b"packed";

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

/// Who the ack claims to come from.
#[derive(Clone, Copy)]
enum Sender {
    /// The acceptor that folded the envelope.
    Voter,
    /// A minted identity outside the configuration.
    Foreign,
}

const SENDER_ALL: [Sender; 2] = [Sender::Voter, Sender::Foreign];

/// What the handler does with the case, named by an exhaustive match.
enum Route {
    Refused,
    Commit,
}

fn route(sender: Sender, v: Rel, s: Rel) -> Route {
    match sender {
        Sender::Foreign => Route::Refused,
        Sender::Voter => match (v, s) {
            (Rel::Equal, Rel::Equal) => Route::Commit,
            _ => Route::Refused,
        },
    }
}

/// A three-node cluster at view 1 with one outstanding proposal at slot 3,
/// whose only `Prepare` was accepted by the acceptor that acks here.
fn assembled(op: OperationId) -> Harness {
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
    h.propose(n(1), op, PAYLOAD);
    h.deliver_to_matching(n(0), Tag::Prepare, Slot(3));
    h
}

fn run_case(sender: Sender, v: Rel, s: Rel) {
    let (system, crash) = mint_pair();
    let op = OperationId {
        msb: (u64::from(system.get()) << 16) | u64::from(crash.get()),
        lsb: 1,
    };
    assert!(
        op.msb != 0 && op.lsb != 0,
        "the mint never draws a zero half"
    );
    let mut h = assembled(op);
    let receiver = n(1);
    let current = view(1);
    let header_view = match v {
        Rel::Less => view(current.view.0 - 1),
        Rel::Equal => current,
        Rel::Greater => view(current.view.0 + 1),
    };
    let slot = Slot(match s {
        Rel::Less => Slot(3).0 - 1,
        Rel::Equal => Slot(3).0,
        Rel::Greater => Slot(3).0 + 1,
    });
    let from = match sender {
        Sender::Voter => n(0),
        Sender::Foreign => mint_id(),
    };
    let message = Message {
        header: Header {
            tag: Tag::FuseOk,
            view: header_view,
            slot,
        },
        body: Body::FuseOk { acks: vec![slot] },
    };
    let outcome = h.inject(from, receiver, message);
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(
        h.fault_of(receiver).is_none(),
        "a refused ack never faults the node"
    );
    h.check_safety().expect("the assembled cluster stays safe");
    match route(sender, v, s) {
        Route::Refused => {
            let StepOutcome::Published { effects, .. } = outcome else {
                panic!("the refusal publishes: {outcome:?}");
            };
            assert!(
                effects.is_empty(),
                "a refused ack emits nothing: {effects:?}"
            );
            let named = match sender {
                Sender::Foreign => {
                    matches!(diagnostic, Some(Diagnostic::UnknownSender { sender: got }) if got == from)
                }
                Sender::Voter => matches!(diagnostic, Some(Diagnostic::FuseRefusal)),
            };
            assert!(named, "{diagnostic:?}");
            assert_eq!(after.committed, 2, "the vote was never counted");
        }
        Route::Commit => {
            let StepOutcome::Published { effects, .. } = outcome else {
                panic!("the atomic vote commits: {outcome:?}");
            };
            assert_eq!(after.committed, 3, "the batch's quorum landed whole");
            assert!(
                matches!(&effects[0], Effect::Apply { slot, operation_id, payload }
                    if *slot == Slot(3) && *operation_id == op && payload.as_ref() == PAYLOAD),
                "the Apply leads, carrying the minted identity: {effects:?}"
            );
            assert_eq!(
                effects.len(),
                3,
                "the Apply and the commit broadcast: {effects:?}"
            );
        }
    }
}

#[test]
fn exhaustive_fuse_ok_at_the_primary() {
    for sender in SENDER_ALL {
        for v in REL_ALL {
            for s in REL_ALL {
                run_case(sender, v, s);
            }
        }
    }
}
