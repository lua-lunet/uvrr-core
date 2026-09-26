//! Exhaustive per-message properties: the `DoViewChange` evidence the
//! designated new primary collects (§9.1, §8.7.7, §13.1).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the evidence kind, the sender's membership, the evidence's view
//! against the attempt target, and the reported accepted frontier against
//! the receiver's own. Exhaustive by construction under the host
//! obligations: the identity law (universally unique and never recycled, so
//! a minted foreign sender is outside the configuration for the life of the
//! test, and the fabricated entries' identities belong to exactly one slot
//! each), the boot-gate marker states (the receiver was never halted, so its
//! evidence and attempt are the assembled life's whole), and the quorum gate
//! (Q1), which makes the receiver and one reporter the evidence quorum. The
//! retained-provenance relation is not enumerated: every honest reporter of
//! the assembled era holds the same retained view, and a divergent
//! provenance is the ranking rule's, covered by view change.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_id, mint_pair};
use vrr::configuration::SystemOperation;
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, Era, NodeId, OperationId, Slot, SystemId, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, EraProof, EvidenceKind, Message};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
use vrr::replica::ViewChangeKnobs;
use vrr::wire::{Header, Tag};

const TIMEOUT: u64 = 3;
const PAYLOAD: &[u8] = b"reported";

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

/// The evidence kind the body carries.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Kind { Ordinary, Planned }

const KIND_ALL: [Kind; 2] = [Kind::Ordinary, Kind::Planned];

/// Who the evidence claims to come from.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Sender { Member, Foreign }

const SENDER_ALL: [Sender; 2] = [Sender::Member, Sender::Foreign];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { UnknownSender, Stale, NotCollected, Win { accepted: u64 } }

#[rustfmt::skip]
fn route(kind: Kind, sender: Sender, v: Rel, a: Rel) -> Route {
    match sender {
        Sender::Foreign => Route::UnknownSender,
        Sender::Member => match kind {
            Kind::Planned => Route::Stale,
            Kind::Ordinary => match v {
                Rel::Less => Route::Stale,
                Rel::Greater => Route::NotCollected,
                Rel::Equal => match a {
                    Rel::Greater => Route::Win { accepted: 3 },
                    _ => Route::Win { accepted: 2 },
                },
            },
        },
    }
}

/// A three-node cluster at view 1, the receiver fenced with an attempt at
/// view 2 it is itself designated to install.
#[rustfmt::skip]
fn assembled() -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT { h.tick(n(1)); }
    h.deliver_all();
    let header = Header { tag: Tag::StartViewChange, view: view(2), slot: Slot::NONE };
    h.inject(n(0), n(2), Message { header, body: Body::StartViewChange {} });
    h
}

/// The era-1 proof every fabricated evidence carries: the real `Init`
/// operation committed at the genesis slot (§8.7.8).
#[rustfmt::skip]
fn era_proof() -> EraProof {
    EraProof { op: SystemOperation::Init { order: vec![n(0), n(1), n(2)] }, committed_at: Slot(2) }
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
fn run_case(kind: Kind, sender: Sender, v: Rel, a: Rel) {
    let (system, crash) = mint_pair();
    let op = OperationId { msb: (u64::from(system.get()) << 16) | u64::from(crash.get()), lsb: 1 };
    assert!(op.msb != 0 && op.lsb != 0, "the mint never draws a zero half");
    let mut h = assembled();
    let receiver = n(2);
    let before = h.snapshot(receiver).expect("the receiver is live");
    let header_view = view(rel(v, u64::from(before.view)) as u32);

    let from = match sender { Sender::Member => n(1), Sender::Foreign => mint_id() };
    let accepted = Slot(rel(a, 2));
    let tail = LogEntry {
        slot: Slot(3), era: Era(1),
        payload: Payload::Operation { id: op, payload: PAYLOAD.into() },
    };
    let (reported, suffix) = match a {
        Rel::Less => (Slot(1), vec![h.journal_entry(receiver, Slot(1)).expect("held")]),
        Rel::Equal => (Slot(2), (1..=2).map(|s| h.journal_entry(receiver, Slot(s))
            .expect("held")).collect()),
        Rel::Greater => (Slot(2), vec![tail]),
    };
    let header = Header { tag: Tag::DoViewChange, view: header_view, slot: accepted };
    let body = Body::DoViewChange {
        retained: view(before.retained_view),
        accepted,
        committed: reported,
        suffix: suffix.clone(),
        evidence: match kind {
            Kind::Ordinary => EvidenceKind::Ordinary, Kind::Planned => EvidenceKind::Planned,
        },
        era_proof: era_proof(),
    };
    let outcome = h.inject(from, receiver, Message { header, body });
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let sent = released(&outcome);
    match route(kind, sender, v, a) {
        Route::UnknownSender | Route::Stale | Route::NotCollected => {
            assert!(sent.is_empty() && matches!(outcome, StepOutcome::Published { .. }), "dropped: {sent:?}");
            let named = match route(kind, sender, v, a) {
                Route::UnknownSender => matches!(diagnostic, Some(Diagnostic::UnknownSender { sender: x }) if x == from),
                Route::Stale => matches!(diagnostic, Some(Diagnostic::StaleEvidence { got, .. }) if got == header_view),
                Route::NotCollected => matches!(diagnostic, Some(Diagnostic::EvidenceNotCollected { sender: x, view }) if x == from && view == header_view),
                _ => false,
            };
            assert!(named && after.view == before.view, "{diagnostic:?}");
        }
        Route::Win { accepted } => {
            assert_eq!(Status::from_word(after.status), Some(Status::Normal));
            assert_eq!(after.view, before.view, "the attempt's target installed");
            assert_eq!(after.accepted, accepted, "the selected history's frontier stands");
            assert_eq!(after.committed, 2, "the reported commitments carry, never regress");
            assert!(sent.len() == 2 && sent.iter().all(|(_, m)| m.header.tag == Tag::StartView
                && m.header.view == view(before.view)), "the broadcast: {sent:?}");
        }
    }
}

#[test]
fn exhaustive_do_view_change_at_the_designated_primary() {
    for kind in KIND_ALL {
        for sender in SENDER_ALL {
            for v in REL_ALL {
                for a in REL_ALL {
                    run_case(kind, sender, v, a);
                }
            }
        }
    }
}
