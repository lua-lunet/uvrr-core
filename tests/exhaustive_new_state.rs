//! Exhaustive per-message properties: the `NewState` chunk a node installs
//! (§4, §10, §13.1 step 5).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the sender against the fetch's responder, the chunk's view against the
//! fetch's view, the chunk's covered end against the fetch cursor, and the
//! more flag. Exhaustive by construction under the host obligations: the
//! identity law (never recycled, so the fabricated entries' identities belong
//! to exactly one slot each), the boot-gate marker states (the receiver was
//! never halted, so only its own open fetch qualifies a chunk), and the
//! quorum gate fixed at construction (Q1). A chunk is history, not a
//! completing ruling: the installed chunk moves the accepted frontier only;
//! the boot-fenced acquisition route is the reincarnation suite's.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_pair};
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, Era, NodeId, OperationId, Slot, SystemId, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::replica::ViewChangeKnobs;
use vrr::wire::{Header, Tag};

const TIMEOUT: u64 = 3;
const PAYLOAD: &[u8] = b"fetched";

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

/// Who the chunk claims to come from.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Sender { Responder, Other }

const SENDER_ALL: [Sender; 2] = [Sender::Responder, Sender::Other];

/// The more flag's two values.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum More { Yes, No }

const MORE_ALL: [More; 2] = [More::Yes, More::No];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { Stale, Duplicate { more: bool }, Install { accepted: u64 } }

#[rustfmt::skip]
fn route(sender: Sender, v: Rel, t: Rel, more: More) -> Route {
    match sender {
        Sender::Other => Route::Stale,
        Sender::Responder => match v {
            Rel::Less | Rel::Greater => Route::Stale,
            Rel::Equal => match t {
                Rel::Less => Route::Duplicate { more: matches!(more, More::Yes) },
                Rel::Equal => Route::Install { accepted: 3 },
                Rel::Greater => Route::Install { accepted: 4 },
            },
        },
    }
}

/// A three-node cluster at view 1 with the receiver holding an open fetch:
/// a gap `Prepare` from the primary opened it, cursor at slot 3.
#[rustfmt::skip]
fn assembled() -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT { h.tick(n(1)); }
    h.deliver_all();
    let (system, crash) = mint_pair();
    let op = OperationId { msb: u64::from(system.get()) << 16 | u64::from(crash.get()), lsb: 1 };
    let header = Header { tag: Tag::Prepare, view: view(1), slot: Slot(5) };
    let entry = LogEntry {
        slot: Slot(5),
        era: Era(1),
        payload: Payload::Operation { id: op, payload: PAYLOAD.into() },
    };
    let body = Body::Prepare { entry, committed: Slot(2) };
    h.inject(n(1), n(2), Message { header, body });
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
fn run_case(sender: Sender, v: Rel, t: Rel, more: More) {
    let (system, crash) = mint_pair();
    let op = OperationId { msb: u64::from(system.get()) << 16 | u64::from(crash.get()), lsb: 2 };
    assert!(op.msb != 0 && op.lsb != 0, "the mint never draws a zero half");
    let mut h = assembled();
    let receiver = n(2);
    let before = h.snapshot(receiver).expect("the receiver is live");
    let header_view = view(rel(v, u64::from(before.view)) as u32);
    let from = match sender { Sender::Responder => n(1), Sender::Other => n(0) };
    let through = Slot(rel(t, 3));
    let entry_of = |slot: Slot| LogEntry {
        slot,
        era: Era(1),
        payload: Payload::Operation {
            id: OperationId { msb: op.msb, lsb: slot.0 }, payload: PAYLOAD.into(),
        },
    };
    let entries = match t {
        Rel::Less => vec![h.journal_entry(receiver, through).expect("held")],
        _ => (3..=through.0).map(|slot| entry_of(Slot(slot))).collect(),
    };
    let header = Header { tag: Tag::NewState, view: header_view, slot: through };
    let body = Body::NewState {
        entries: entries.clone(), through, committed: Slot(2),
        more: matches!(more, More::Yes),
    };
    let outcome = h.inject(from, receiver, Message { header, body });
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let sent = released(&outcome);
    let published = matches!(outcome, StepOutcome::Published { .. });
    match route(sender, v, t, more) {
        Route::Stale => {
            assert!(published && sent.is_empty(), "an unqualified chunk emits nothing: {sent:?}");
            let named = matches!(diagnostic, Some(Diagnostic::StaleTransfer { sender: x, view })
                if x == from && view == header_view);
            assert!(named, "{diagnostic:?}");
            assert_eq!(after.accepted, before.accepted, "the fetch stays open, nothing installs");
        }
        Route::Duplicate { more: true } => {
            assert!(published && sent.is_empty(), "a duplicate claiming more is stale: {sent:?}");
            assert!(matches!(diagnostic, Some(Diagnostic::StaleTransfer { sender: x, .. }) if x == from),
                "{diagnostic:?}");
            assert_eq!(after.accepted, before.accepted);
        }
        Route::Duplicate { more: false } => {
            assert!(published && sent.is_empty(), "a duplicate close emits nothing: {sent:?}");
            assert!(matches!(diagnostic, Some(Diagnostic::None)), "{diagnostic:?}");
            assert_eq!(after.accepted, before.accepted, "a duplicate installs nothing new");
        }
        Route::Install { accepted } => {
            assert!(published && matches!(diagnostic, Some(Diagnostic::None)), "{diagnostic:?}");
            assert_eq!(after.accepted, accepted, "the chunk installs through its covered end");
            assert_eq!(after.committed, before.committed, "a chunk is history, never a ruling");
            let expected = if matches!(more, More::Yes) { 1 } else { 0 };
            assert_eq!(sent.len(), expected, "{sent:?}");
            if let More::Yes = more {
                assert!(sent[0].0 == from && sent[0].1.header.tag == Tag::GetState
                    && matches!(&sent[0].1.body, Body::GetState { from: Slot(x) } if *x == accepted + 1),
                    "a partial answer resumes the fetch: {sent:?}");
            }
        }
    }
}

#[test]
fn exhaustive_new_state_at_a_node() {
    for sender in SENDER_ALL {
        for v in REL_ALL {
            for t in REL_ALL {
                for more in MORE_ALL {
                    run_case(sender, v, t, more);
                }
            }
        }
    }
}
