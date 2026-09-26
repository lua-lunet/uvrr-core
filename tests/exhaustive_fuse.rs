//! Exhaustive per-message properties: the `Fuse` envelope an acceptor folds
//! (`docs/uvrr-fuse.md` §1, §3).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the envelope's view against the receiver's current view, the
//! envelope's first slot against the accept frontier's successor, and the
//! packed schedule's legality against the configuration fold. Exhaustive by
//! construction under the host obligations: the identity law (universally
//! unique and never recycled, so the minted identity the schedule names is
//! outside the configuration for the life of the test, which is what makes
//! the illegal schedule's fold refuse), the boot-gate marker states (the
//! receiver was never halted, so it holds its assembled `Normal` life), and
//! the quorum gate fixed at construction (Q1). The envelope is atomic (§2):
//! the packed schedule folds whole or the whole envelope is refused, never a
//! partial fold and never a wire nack, so the one named outcome covers every
//! refusal.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_id, mint_pair};
use vrr::configuration::SystemOperation;
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, Era, NodeId, OperationId, Slot, SystemId, View, ViewId};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::replica::ViewChangeKnobs;
use vrr::wire::{Header, Tag};

const TIMEOUT: u64 = 3;

#[rustfmt::skip]
fn n(id: u32) -> NodeId {
    let system = SystemId::new((id + 1) as u16).expect("small and non-zero");
    let life = CrashCounter::new(1).expect("one is non-zero");
    NodeId::new(system, life)
}

#[rustfmt::skip]
fn view(number: u32) -> ViewId { ViewId { era: Era(1), view: View(number) } }

/// The relation of one numeric field against the receiver's own value.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Rel { Less, Equal, Greater }

const REL_ALL: [Rel; 3] = [Rel::Less, Rel::Equal, Rel::Greater];

#[rustfmt::skip]
fn rel(r: Rel, base: u64) -> u64 {
    match r { Rel::Less => base - 1, Rel::Equal => base, Rel::Greater => base + 1 }
}

/// The packed schedule's legality against the configuration fold.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Ops { Legal, Illegal }

const OPS_ALL: [Ops; 2] = [Ops::Legal, Ops::Illegal];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { Refused, Accepted }

#[rustfmt::skip]
fn route(v: Rel, s: Rel, ops: Ops) -> Route {
    match (v, s, ops) {
        (Rel::Equal, Rel::Equal, Ops::Legal) => Route::Accepted,
        _ => Route::Refused,
    }
}

/// The datagrams one step released, empty on a refusal.
#[rustfmt::skip]
fn released(outcome: &StepOutcome) -> Vec<(NodeId, Message)> {
    match outcome {
        StepOutcome::Published { effects, .. } => effects.iter().filter_map(|e| match e {
            Effect::Send { to, message, .. } => Some((*to, message.clone())),
            _ => None,
        }).collect(),
        _ => Vec::new(),
    }
}

/// A three-node cluster at view 1, the receiver a `Normal` backup.
#[rustfmt::skip]
fn assembled() -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT { h.tick(n(1)); }
    h.deliver_all();
    h
}

#[rustfmt::skip]
fn run_case(v: Rel, s: Rel, ops: Ops) {
    let (system, crash) = mint_pair();
    let op = OperationId { msb: (u64::from(system.get()) << 16) | u64::from(crash.get()), lsb: 1 };
    assert!(op.msb != 0 && op.lsb != 0, "the mint never draws a zero half");
    let _ = op;
    let joined = mint_id();
    assert!(joined.is_lawful(), "the mint draws strictly inside the identity space");
    let mut h = assembled();
    let receiver = n(2);
    let before = h.snapshot(receiver).expect("the receiver is live");
    let header_view = view(rel(v, u64::from(before.view)) as u32);
    let sender = n(rel(v, 1) as u32);
    let first_slot = Slot(rel(s, before.accepted + 1));
    let packed = match ops {
        Ops::Legal => SystemOperation::Join { node: joined, position: 3 },
        Ops::Illegal => SystemOperation::Decrement(joined),
    };
    let header = Header { tag: Tag::Fuse, view: header_view, slot: first_slot };
    let body = Body::Fuse { ops: vec![packed.clone()] };
    let outcome = h.inject(sender, receiver, Message { header, body });
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let published = matches!(outcome, StepOutcome::Published { .. });
    let fuse_ok = |sent: &Vec<(NodeId, Message)>| sent.len() == 1 && sent[0].0 == sender
        && sent[0].1.header.tag == Tag::FuseOk && sent[0].1.header.slot == first_slot;
    match route(v, s, ops) {
        Route::Refused => {
            assert!(published, "the refusal publishes: {outcome:?}");
            assert!(matches!(diagnostic, Some(Diagnostic::FuseRefusal)), "{diagnostic:?}");
            assert_eq!(after.accepted, before.accepted, "never a partial fold");
        }
        Route::Accepted => {
            assert!(published, "the folded envelope publishes: {outcome:?}");
            assert_eq!(after.accepted, first_slot.0, "the frontier advances per packed op");
            assert_eq!(after.committed, before.committed, "the fold commits nothing");
            assert!(fuse_ok(&released(&outcome)), "{:?}",
                released(&outcome));
            let held = h.journal_entry(receiver, first_slot).expect("the entry is journaled");
            assert!(
                matches!(&held.payload, vrr::journal::Payload::System(op) if *op == packed),
                "the folded entry carries the packed op: {:?}", held.payload
            );
        }
    }
}

#[test]
fn exhaustive_fuse_at_an_acceptor() {
    for v in REL_ALL {
        for s in REL_ALL {
            for ops in OPS_ALL {
                run_case(v, s, ops);
            }
        }
    }
}
