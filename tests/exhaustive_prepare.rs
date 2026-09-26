//! Exhaustive per-message properties: the `Prepare` a backup receives (§4).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the receiver's boot status, the message's view, the offered slot
//! against the frontier's successor, and the piggybacked commit frontier
//! against its committed frontier. Exhaustive by construction under the host
//! obligations: the identity law (universally unique, durable before
//! emission, never recycled, so a slot is assigned once), the boot-gate
//! marker states (a halt classifies `Clean`), and the quorum gate (Q1).

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_pair};
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, Era, NodeId, OperationId, Slot, SystemId, View, ViewId};
use vrr::journal::{LogEntry, Payload};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
use vrr::replica::{PlanRefusal, ViewChangeKnobs};
use vrr::wire::{Header, Tag};

const TIMEOUT: u64 = 3;
const PAYLOAD: &[u8] = b"proposed";

#[rustfmt::skip]
fn n(id: u32) -> NodeId {
    let system = SystemId::new((id + 1) as u16).expect("small and non-zero");
    let life = CrashCounter::new(1).expect("one is non-zero");
    NodeId::new(system, life)
}

#[rustfmt::skip]
fn view(number: u32) -> ViewId { ViewId { era: Era(1), view: View(number) } }

#[rustfmt::skip]
fn op_entry(slot: Slot, era: Era, id: OperationId) -> LogEntry {
    LogEntry { slot, era, payload: Payload::Operation { id, payload: PAYLOAD.into() } }
}

#[rustfmt::skip]
fn prepare(view: ViewId, slot: Slot, entry: LogEntry, piggy: Slot) -> Message {
    Message {
        header: Header { tag: Tag::Prepare, view, slot },
        body: Body::Prepare { entry, committed: piggy },
    }
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

/// The receiver's assembled boot status.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Boot { Normal, Restarting }

const BOOT_ALL: [Boot; 2] = [Boot::Normal, Boot::Restarting];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { Mismatch, Retransmit, Accept, PiggyRefused, Gap, Fence }

#[rustfmt::skip]
fn route(boot: Boot, v: Rel, s: Rel, p: Rel) -> Route {
    match (v, s, p) {
        (Rel::Less, _, _) => Route::Mismatch,
        (Rel::Greater, _, _) => match boot {
            Boot::Normal => Route::Fence,
            Boot::Restarting => Route::Mismatch,
        },
        (Rel::Equal, Rel::Less, _) => Route::Retransmit,
        (Rel::Equal, Rel::Equal, Rel::Greater) => Route::PiggyRefused,
        (Rel::Equal, Rel::Equal, _) => Route::Accept,
        (Rel::Equal, Rel::Greater, _) => Route::Gap,
    }
}

/// A three-node cluster, every node `Normal` at view 1, no datagram in flight.
#[rustfmt::skip]
fn cluster_at_view_one() -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT { h.tick(n(1)); }
    h.deliver_all();
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
fn run_case(seq: u64, boot: Boot, v: Rel, s: Rel, p: Rel) {
    let (system, crash) = mint_pair();
    let op = OperationId { msb: (u64::from(system.get()) << 16) | u64::from(crash.get()), lsb: seq };
    assert!(op.msb != 0 && op.lsb != 0, "the mint never draws a zero half");
    let mut h = cluster_at_view_one();
    let receiver = n(2);
    if let Boot::Restarting = boot {
        h.halt(receiver); h.restart_with(receiver).expect("the halt classifies clean");
    }
    let before = h.snapshot(receiver).expect("the receiver is live");
    let (accepted, committed) = (before.accepted, before.committed);
    let header_view = view(rel(v, u64::from(before.view)) as u32);
    let sender = n(rel(v, 1) as u32);
    let entry_slot = Slot(rel(s, accepted + 1));
    let piggyback = Slot(rel(p, committed));
    let entry = match s {
        Rel::Less => h.journal_entry(receiver, entry_slot).expect("the slot is held"),
        _ => op_entry(entry_slot, header_view.era, op),
    };
    let outcome = h.inject(sender, receiver, prepare(header_view, entry_slot, entry, piggyback));
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let published = matches!(outcome, StepOutcome::Published { .. });
    let sent = released(&outcome);
    let mismatch = matches!(diagnostic, Some(Diagnostic::ViewMismatch { got, .. }) if got == header_view);
    let gap = matches!(diagnostic, Some(Diagnostic::GapDetected { expected: Slot(e), got: Slot(g) })
        if e == accepted + 1 && g == entry_slot.0);
    let fetch = |sent: &[(NodeId, Message)]| sent.iter().any(|(to, m)| *to == sender
        && m.header.tag == Tag::GetState
        && matches!(&m.body, Body::GetState { from: Slot(f) } if *f == accepted + 1));
    match route(boot, v, s, p) {
        Route::Mismatch => {
            assert!(sent.is_empty() && mismatch, "{sent:?} {diagnostic:?}");
            assert_eq!(after.accepted, accepted, "a drop moves no frontier");
        }
        Route::Retransmit | Route::Accept => {
            let accepting = matches!(route(boot, v, s, p), Route::Accept);
            assert!(published);
            assert_eq!(after.accepted, if accepting { entry_slot.0 } else { accepted });
            assert_eq!(after.committed, committed, "a piggyback below the slot commits nothing");
            assert!(sent.len() == 1 && sent[0].0 == sender, "{sent:?}");
            assert!(sent[0].1.header.tag == Tag::PrepareOk && sent[0].1.header.slot == entry_slot);
            if accepting {
                let held = h.journal_entry(receiver, entry_slot).expect("the entry is journaled");
                let Payload::Operation { id, payload } = &held.payload else {
                    panic!("an operation slot: {:?}", held.payload);
                };
                assert_eq!(*id, op, "the minted identity is carried opaque");
                assert_eq!(payload.as_ref(), PAYLOAD);
            }
        }
        Route::PiggyRefused => {
            let refused = matches!(outcome, StepOutcome::PlanRefused(
                PlanRefusal::JournalEntryUnavailable { slot: Slot(x) }) if x == entry_slot.0);
            assert!(refused, "the arriving slot's piggyback is refused: {outcome:?}");
            assert_eq!(after.accepted, accepted, "the refusal installs nothing");
        }
        Route::Gap => {
            assert!(published && gap, "{diagnostic:?}");
            assert!(fetch(&sent), "{sent:?}");
            assert_eq!(after.accepted, accepted);
        }
        Route::Fence => {
            assert_eq!(Status::from_word(after.status), Some(Status::ViewChange));
            assert_eq!(after.view, before.view + 1);
            assert_eq!(sent.len(), 3, "two fences and the fetch: {sent:?}");
            assert_eq!(sent.iter().filter(|(_, m)| m.header.tag == Tag::StartViewChange).count(), 2);
            assert!(fetch(&sent), "{sent:?}");
        }
    }
}

#[test]
fn exhaustive_prepare_at_a_backup() {
    let mut seq = 1;
    for boot in BOOT_ALL {
        for v in REL_ALL {
            for s in REL_ALL {
                for p in REL_ALL {
                    run_case(seq, boot, v, s, p);
                    seq += 1;
                }
            }
        }
    }
}
