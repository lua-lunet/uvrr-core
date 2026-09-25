//! Exhaustive per-message properties: the `GetState` a node serves (§4, §10,
//! §13.1 step 5).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the requester's membership against the responder's committed
//! configuration, the responder's boot status (the serving gate: fenced entry
//! states and `Replaying` serve nothing), and the requested base against the
//! responder's accept frontier. Exhaustive by construction under the host
//! obligations: the identity law (never recycled, so a minted foreign
//! requester is foreign for the life of the test), the boot-gate marker
//! states (a halt classifies `Clean` and reopens fenced `Restarting`), and
//! the quorum gate fixed at construction (Q1). Serving is read-only
//! retransmission of durable journal content: the request's view is a
//! correlation token, echoed, never a serving condition, and the responder
//! mutates nothing.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_id};
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, Era, NodeId, Slot, SystemId, View, ViewId};
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

/// The relation of one numeric field against the receiver's own value.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Rel { Less, Equal, Greater }

const REL_ALL: [Rel; 3] = [Rel::Less, Rel::Equal, Rel::Greater];

#[rustfmt::skip]
fn rel(r: Rel, base: u64) -> u64 {
    match r { Rel::Less => base - 1, Rel::Equal => base, Rel::Greater => base + 1 }
}

/// The responder's assembled boot status.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Boot { Normal, Restarting }

const BOOT_ALL: [Boot; 2] = [Boot::Normal, Boot::Restarting];

/// Who the request claims to come from.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Sender { Member, Foreign }

const SENDER_ALL: [Sender; 2] = [Sender::Member, Sender::Foreign];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { UnknownSender, NotServed, Served { base: u64 } }

#[rustfmt::skip]
fn route(sender: Sender, boot: Boot, r: Rel) -> Route {
    match sender {
        Sender::Foreign => Route::UnknownSender,
        Sender::Member => match boot {
            Boot::Restarting => Route::NotServed,
            Boot::Normal => match r {
                Rel::Greater => Route::NotServed,
                Rel::Less => Route::Served { base: 1 },
                Rel::Equal => Route::Served { base: 2 },
            },
        },
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

/// A three-node cluster with the responder assembled `Normal` at the genesis
/// view or reopened fenced from a halt, frontiers at the genesis pair.
#[rustfmt::skip]
fn assembled(boot: Boot) -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    if let Boot::Normal = boot { h.tick(n(0)); }
    if let Boot::Restarting = boot {
        h.halt(n(0)); h.restart_with(n(0)).expect("the halt classifies clean");
    }
    h
}

#[rustfmt::skip]
fn run_case(sender: Sender, boot: Boot, r: Rel) {
    let mut h = assembled(boot);
    let responder = n(0);
    let before = h.snapshot(responder).expect("the responder is live");
    let frontier = before.accepted;
    let header_view = ViewId { era: Era(before.era), view: View(before.view) };
    let base = Slot(rel(r, frontier));
    let from = match sender {
        Sender::Member => n(1),
        Sender::Foreign => mint_id(),
    };
    let header = Header { tag: Tag::GetState, view: header_view, slot: Slot(rel(r, frontier - 1)) };
    let body = Body::GetState { from: base };
    let outcome = h.inject(from, responder, Message { header, body });
    let diagnostic = h.diagnostic(responder);
    let after = h.snapshot(responder).expect("the responder is live");
    assert!(h.fault_of(responder).is_none() && h.check_safety().is_ok());
    assert_eq!(after.accepted, frontier, "serving mutates nothing");
    assert_eq!(after.committed, before.committed);
    let published = matches!(outcome, StepOutcome::Published { .. });
    match route(sender, boot, r) {
        Route::UnknownSender | Route::NotServed => {
            assert!(published, "a named drop publishes: {outcome:?}");
            assert!(released(&outcome).is_empty(), "nothing is emitted: {outcome:?}");
            let named = match sender {
                Sender::Foreign => matches!(diagnostic, Some(Diagnostic::UnknownSender { sender: x }) if x == from),
                Sender::Member => matches!(diagnostic, Some(Diagnostic::TransferNotServed { .. })),
            };
            assert!(named, "{diagnostic:?}");
        }
        Route::Served { base } => {
            assert!(published, "a servable request publishes: {outcome:?}");
            let chunk = released(&outcome);
            assert_eq!(chunk.len(), 1, "one chunk back: {chunk:?}");
            assert_eq!(chunk[0].0, from, "the chunk addresses the requester");
            assert_eq!(chunk[0].1.header.tag, Tag::NewState);
            assert_eq!(chunk[0].1.header.view, header_view);
            assert_eq!(chunk[0].1.header.view, header_view, "the correlation token echoes");
            let Body::NewState { entries, through, committed, more } = &chunk[0].1.body
            else { panic!("a NewState body: {:?}", chunk[0].1.body); };
            assert_eq!(through.0, frontier, "the chunk reaches the frontier");
            assert_eq!(entries.len() as u64, frontier - base + 1, "contiguous from the base");
            assert_eq!(*committed, Slot(before.committed), "the frontier rides the chunk");
            assert!(!more, "the frontier sits at the chunk's end");
        }
    }
}

#[test]
fn exhaustive_get_state_at_a_node() {
    for sender in SENDER_ALL {
        for boot in BOOT_ALL {
            for r in REL_ALL {
                run_case(sender, boot, r);
            }
        }
    }
}
