//! Exhaustive per-message properties: the `StartViewChange` fence a node
//! votes with (§9.1).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: whether the node already runs an in-progress attempt, the fenced view
//! against the node's fence target (or current view, without one), and the
//! sender's membership. Exhaustive by construction under the host
//! obligations: the identity law (universally unique and never recycled, so
//! a minted foreign sender is outside the configuration for the life of the
//! test), the boot-gate marker states (the receiver was never halted, so it
//! holds its assembled `Normal` life), and the quorum gate fixed at
//! construction (Q1), which makes the fence and evidence quorums the
//! strategy's own decisions. The attempt state is volatile by design (§9.3),
//! so the assembled attempt is the whole of what a fence vote can meet.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_id};
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, Era, NodeId, Slot, SystemId, View, ViewId};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
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

/// Whether the receiver already runs an attempt.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Attempt { Unattempted, Attempted }

const ATTEMPT_ALL: [Attempt; 2] = [Attempt::Unattempted, Attempt::Attempted];

/// Who the fence claims to come from.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Sender { Member, Foreign }

const SENDER_ALL: [Sender; 2] = [Sender::Member, Sender::Foreign];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { UnknownSender, Stale, Vote, JoinSelf { target: u64 }, Join { target: u64, to: u32 } }

#[rustfmt::skip]
fn route(sender: Sender, attempt: Attempt, v: Rel) -> Route {
    match sender {
        Sender::Foreign => Route::UnknownSender,
        Sender::Member => match (attempt, v) {
            (_, Rel::Less) => Route::Stale,
            (Attempt::Unattempted, Rel::Equal) => Route::Stale,
            (Attempt::Attempted, Rel::Equal) => Route::Vote,
            (Attempt::Unattempted, Rel::Greater) => Route::JoinSelf { target: 2 },
            (Attempt::Attempted, Rel::Greater) => Route::Join { target: 4, to: 1 },
        },
    }
}

/// A three-node cluster at view 1, the receiver `Normal`, with an attempt at
/// view 3 already under way when the case names one.
#[rustfmt::skip]
fn assembled(attempt: Attempt) -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT { h.tick(n(1)); }
    h.deliver_all();
    if let Attempt::Attempted = attempt {
        let header = Header { tag: Tag::StartViewChange, view: view(3), slot: Slot::NONE };
        let body = Body::StartViewChange {};
        h.inject(n(0), n(2), Message { header, body });
    }
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
fn run_case(attempt: Attempt, v: Rel, sender: Sender) {
    let mut h = assembled(attempt);
    let receiver = n(2);
    let before = h.snapshot(receiver).expect("the receiver is live");
    let header_view = view(rel(v, u64::from(before.view)) as u32);
    let from = match sender { Sender::Member => n(1), Sender::Foreign => mint_id() };
    let header = Header { tag: Tag::StartViewChange, view: header_view, slot: Slot::NONE };
    let body = Body::StartViewChange {};
    let outcome = h.inject(from, receiver, Message { header, body });
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let sent = released(&outcome);
    let published = matches!(outcome, StepOutcome::Published { .. });
    match route(sender, attempt, v) {
        Route::UnknownSender | Route::Stale | Route::Vote => {
            assert!(published, "a named outcome publishes: {outcome:?}");
            let named = match route(sender, attempt, v) {
                Route::UnknownSender => matches!(diagnostic, Some(Diagnostic::UnknownSender { sender: x }) if x == from),
                Route::Stale => matches!(diagnostic, Some(Diagnostic::StaleViewChange { got, .. }) if got == header_view),
                Route::Vote => matches!(diagnostic, Some(Diagnostic::None)),
                _ => false,
            };
            assert!(named, "{diagnostic:?}");
            assert!(sent.is_empty(), "a drop or a vote emits nothing: {sent:?}");
            let fenced = matches!(route(sender, attempt, v), Route::Vote);
            if fenced {
                assert_eq!(Status::from_word(after.status), Some(Status::ViewChange),
                    "the receiver stays fenced in its attempt");
                assert_eq!(after.view, before.view, "the target did not move");
            } else {
                assert_eq!(after.view, before.view, "the receiver's view never regressed");
            }
        }
        Route::JoinSelf { target } | Route::Join { target, .. } => {
            assert!(published, "the join publishes: {outcome:?}");
            assert_eq!(Status::from_word(after.status), Some(Status::ViewChange),
                "the receiver fences into the named view");
            assert_eq!(after.view, target as u32);
            let reporting = matches!(route(sender, attempt, v), Route::Join { .. });
            let expected = if reporting { 3 } else { 2 };
            assert_eq!(sent.len(), expected, "{sent:?}");
            assert_eq!(sent.iter().filter(|(_, m)| m.header.tag == Tag::StartViewChange
                && m.header.view == view(target as u32)).count(), 2, "{sent:?}");
            if reporting {
                let to = n(route(sender, attempt, v).report_to());
                assert!(sent.iter().any(|(x, m)| *x == to && m.header.tag == Tag::DoViewChange
                    && m.header.view == view(target as u32)), "{sent:?}");
            }
        }
    }
}

#[rustfmt::skip]
impl Route {
    fn report_to(self) -> u32 {
        match self { Route::Join { to, .. } => to, _ => 0 }
    }
}

#[test]
fn exhaustive_start_view_change_at_a_node() {
    for attempt in ATTEMPT_ALL {
        for v in REL_ALL {
            for sender in SENDER_ALL {
                run_case(attempt, v, sender);
            }
        }
    }
}
