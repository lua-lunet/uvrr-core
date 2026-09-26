//! Exhaustive per-message properties: the `StartView` a node adopts (§9.1,
//! §13.1).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the installed view against the node's current view, and the offered
//! history against the node's durable frontiers. Exhaustive by construction
//! under the host obligations: the identity law (never recycled, so a slot
//! is assigned once and a shared committed slot has one honest entry, which
//! makes a conflicting offer the declared safety breach), the boot-gate
//! marker states (the receiver was never halted, so its checkpoint frontier
//! is the `Slot(0)` sentinel), and the quorum gate (Q1). An honest evidence
//! quorum can never produce the conflict offer; the case pins the fault.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_pair};
use vrr::configuration::{INIT_SLOT, SystemOperation};
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, Era, Fault, NodeId, OperationId, Slot, SystemId, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, EraProof, Message};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
use vrr::replica::{PublishRefusal, ViewChangeKnobs};
use vrr::wire::{Header, Tag};

const TIMEOUT: u64 = 3;
const PAYLOAD: &[u8] = b"selected";

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

/// The offered history, against the receiver's durable frontiers.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Offer { Less, Equal, Greater, Distant, Conflict }

const OFFER_ALL: [Offer; 5] = [
    Offer::Less,
    Offer::Equal,
    Offer::Greater,
    Offer::Distant,
    Offer::Conflict,
];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { StaleView, StaleClaim, Install { accepted: u64 }, Gap, Conflict }

#[rustfmt::skip]
fn route(v: Rel, offer: Offer) -> Route {
    match v {
        Rel::Less | Rel::Equal => Route::StaleView,
        Rel::Greater => match offer {
            Offer::Less => Route::StaleClaim,
            Offer::Equal => Route::Install { accepted: 2 },
            Offer::Greater => Route::Install { accepted: 3 },
            Offer::Distant => Route::Gap,
            Offer::Conflict => Route::Conflict,
        },
    }
}

/// A three-node cluster at view 1, the receiver `Normal` at the genesis
/// frontiers, the conflict case's fault declared.
#[rustfmt::skip]
fn assembled(offer: Offer) -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT { h.tick(n(1)); }
    h.deliver_all();
    if let Offer::Conflict = offer { h.expect_fault(n(2)); }
    h
}

/// The era-1 proof: the real `Init` at the genesis slot (§8.7.8).
#[rustfmt::skip]
fn era_proof() -> EraProof {
    EraProof { op: SystemOperation::Init { order: vec![n(0), n(1), n(2)] }, committed_at: INIT_SLOT }
}

/// The genesis prefix the fabricated offers verify against.
#[rustfmt::skip]
fn genesis(h: &Harness, receiver: NodeId) -> Vec<LogEntry> {
    (1..=2).map(|slot| h.journal_entry(receiver, Slot(slot)).expect("held")).collect()
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
fn run_case(v: Rel, offer: Offer) {
    let (system, crash) = mint_pair();
    let op = OperationId { msb: u64::from(system.get()) << 16 | u64::from(crash.get()), lsb: 1 };
    assert!(op.msb != 0 && op.lsb != 0, "the mint never draws a zero half");
    let mut h = assembled(offer);
    let receiver = n(2);
    let before = h.snapshot(receiver).expect("the receiver is live");
    let (header_view, sender) = match v {
        Rel::Less => (view(before.view - 1), n(0)),
        Rel::Equal => (view(before.view), n(1)),
        Rel::Greater => (view(before.view + 2), n(0)),
    };
    let op_entry = |slot: Slot| LogEntry {
        slot, era: Era(1), payload: Payload::Operation { id: op, payload: PAYLOAD.into() },
    };
    let genesis = genesis(&h, receiver);
    let (accepted, committed, suffix) = match offer {
        Offer::Less => (Slot(2), Slot(1), genesis.clone()),
        Offer::Equal => (Slot(2), Slot(2), genesis),
        Offer::Greater => (Slot(3), Slot(2), vec![op_entry(Slot(3))]),
        Offer::Distant => (Slot(5), Slot(2), vec![op_entry(Slot(5))]),
        Offer::Conflict => (Slot(2), Slot(2), vec![LogEntry {
            slot: Slot(2), era: Era(1), payload: Payload::System(SystemOperation::Halve),
        }]),
    };
    let header = Header { tag: Tag::StartView, view: header_view, slot: accepted };
    let body = Body::StartView { suffix, accepted, committed, era_proof: era_proof() };
    let outcome = h.inject(sender, receiver, Message { header, body });
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    let routed = route(v, offer);
    let faultless = h.fault_of(receiver).is_none();
    assert!(h.check_safety().is_ok() && (faultless || matches!(routed, Route::Conflict)));
    let sent = released(&outcome);
    match routed {
        Route::StaleView | Route::StaleClaim => {
            assert!(sent.is_empty() && matches!(outcome, StepOutcome::Published { .. }), "{sent:?}");
            let stale = matches!(diagnostic, Some(Diagnostic::StartViewFromStaleView { got, .. }) if got == header_view);
            let claimed = matches!(diagnostic, Some(Diagnostic::StaleEvidence { got, .. }) if got == header_view);
            let named = matches!(routed, Route::StaleView) == stale && matches!(routed, Route::StaleClaim) == claimed;
            assert!(named, "{diagnostic:?}");
            assert_eq!(after.committed, before.committed, "nothing installs");
        }
        Route::Install { accepted } => {
            assert!(matches!(outcome, StepOutcome::Published { .. })
                && Status::from_word(after.status) == Some(Status::Normal), "{outcome:?}");
            assert_eq!(after.view, header_view.view.0, "the offered history installs");
            assert_eq!((after.accepted, after.committed), (accepted, committed.0),
                "the installed history's frontiers stand");
            assert!(sent.is_empty(), "nothing newly commits: {sent:?}");
        }
        Route::Gap => {
            let named = matches!(diagnostic, Some(Diagnostic::GapDetected { expected: Slot(x), got: Slot(y) })
                if x == 3 && y == 5);
            assert!(named && matches!(outcome, StepOutcome::Published { .. }), "{diagnostic:?}");
            assert!(sent.len() == 1 && sent[0].0 == sender && sent[0].1.header.tag == Tag::GetState
                && matches!(&sent[0].1.body, Body::GetState { from: Slot(x) } if *x == 3)
                && after.view == before.view, "the gap ruling fetches: {sent:?}");
        }
        Route::Conflict => {
            let declared = matches!(outcome, StepOutcome::PublishRefused(
                PublishRefusal::IllegalCandidate(Fault::IllegalTransition)));
            assert!(declared, "the conflict is the declared fault: {outcome:?}");
            assert_eq!(h.fault_of(receiver), Some(Fault::IllegalTransition));
            assert_eq!(after.view, before.view, "the candidate was discarded");
        }
    }
}

#[test]
fn exhaustive_start_view_at_a_node() {
    for v in REL_ALL {
        for offer in OFFER_ALL {
            run_case(v, offer);
        }
    }
}
