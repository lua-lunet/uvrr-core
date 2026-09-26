//! Exhaustive per-message properties: the `PrepareOk` the serving primary
//! counts (§4).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the sender's membership, the acknowledgement's view against the
//! primary's current view, and the acknowledged slot against the outstanding
//! proposal. Exhaustive by construction under the host obligations: the
//! identity law (universally unique and never recycled, so a minted foreign
//! sender is outside the configuration for the life of the test), the
//! boot-gate marker states (the primary was never halted, so it holds its
//! assembled `Normal` life), and the quorum gate fixed at construction (Q1).
//! The learner-discard dimension is not enumerated: the assembled
//! configuration holds no weight-zero member, and the learner rule is the
//! reincarnation suite's.

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
const PAYLOAD: &[u8] = b"ordered";

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

/// Who the acknowledgement claims to come from.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Sender { Voter, Foreign }

const SENDER_ALL: [Sender; 2] = [Sender::Voter, Sender::Foreign];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { UnknownSender, ViewMismatch, SlotNotOutstanding, Commit }

#[rustfmt::skip]
fn route(sender: Sender, v: Rel, s: Rel) -> Route {
    match sender {
        Sender::Foreign => Route::UnknownSender,
        Sender::Voter => match v {
            Rel::Less | Rel::Greater => Route::ViewMismatch,
            Rel::Equal => match s {
                Rel::Equal => Route::Commit,
                _ => Route::SlotNotOutstanding,
            },
        },
    }
}

/// A three-node cluster at view 1, whose primary is the node the change
/// designated: one outstanding proposal at slot 3, whose only `Prepare` was
/// accepted by the backup that votes here.
#[rustfmt::skip]
fn assembled(op: OperationId) -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT { h.tick(n(1)); }
    h.deliver_all();
    h.propose(n(1), op, PAYLOAD);
    h.deliver_to_matching(n(0), Tag::Prepare, Slot(3));
    h
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

#[rustfmt::skip]
fn run_case(sender: Sender, v: Rel, s: Rel) {
    let (system, crash) = mint_pair();
    let op = OperationId { msb: (u64::from(system.get()) << 16) | u64::from(crash.get()), lsb: 1 };
    assert!(op.msb != 0 && op.lsb != 0, "the mint never draws a zero half");
    let mut h = assembled(op);
    let receiver = n(1);
    let header_view = view(rel(v, 1) as u32);
    let slot = Slot(rel(s, 3));
    let from = match sender {
        Sender::Voter => n(0),
        Sender::Foreign => mint_id(),
    };
    let header = Header { tag: Tag::PrepareOk, view: header_view, slot };
    let outcome = h.inject(from, receiver, Message { header, body: Body::PrepareOk {} });
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let sent = released(&outcome);
    let published = matches!(outcome, StepOutcome::Published { .. });
    let named = |d: &Option<Diagnostic>| match sender {
        Sender::Foreign => matches!(d, Some(Diagnostic::UnknownSender { sender: x }) if *x == from),
        Sender::Voter => matches!(d, Some(Diagnostic::ViewMismatch { got, .. }) if *got == header_view)
            || matches!(d, Some(Diagnostic::SlotNotOutstanding { slot: x, sender: y })
                if *x == slot && *y == from),
    };
    match route(sender, v, s) {
        Route::UnknownSender | Route::ViewMismatch | Route::SlotNotOutstanding => {
            assert!(published && sent.is_empty(), "{sent:?}");
            assert!(named(&diagnostic), "{diagnostic:?}");
            assert_eq!(after.committed, 2, "the vote was never counted");
        }
        Route::Commit => {
            assert!(published);
            assert_eq!(after.committed, 3, "the vote completed the quorum");
            assert!(sent.len() == 2, "the Apply leads and the commit broadcasts: {sent:?}");
            assert!(
                matches!(released_apply(&outcome), Some((Slot(3), id)) if id == op),
                "the Apply carries the minted identity: {outcome:?}"
            );
            assert!(sent.iter().all(|(_, m)| m.header.tag == Tag::Commit
                && matches!(&m.body, Body::Commit { committed: Slot(x) } if *x == 3)),
                "{sent:?}");
        }
    }
}

#[rustfmt::skip]
fn released_apply(outcome: &StepOutcome) -> Option<(Slot, OperationId)> {
    match outcome {
        StepOutcome::Published { effects, .. } => effects.iter().find_map(|e| match e {
            Effect::Apply { slot, operation_id, .. } => Some((*slot, *operation_id)),
            _ => None,
        }),
        _ => None,
    }
}

#[test]
fn exhaustive_prepare_ok_at_the_primary() {
    for sender in SENDER_ALL {
        for v in REL_ALL {
            for s in REL_ALL {
                run_case(sender, v, s);
            }
        }
    }
}
