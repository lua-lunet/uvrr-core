//! Exhaustive per-message properties: the `Reincarnation` announcement the
//! leader acts on (`docs/uvrr-reincarnation.md` §4, §7).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the receiver's role (only the leader drives the forced sequence), the
//! sender against the pair the announcement names (a forgery drops), the
//! announced pair's distinctness, and the past-life prepared frontier against
//! the leader's committed frontier (the immediate ack's extent). Exhaustive
//! by construction under the host obligations: the identity law (universally
//! unique, durable before emission and never recycled, so the minted old and
//! new identities name a pair no lawful life has held, and the mint check at
//! the flow's end verifies the halves never moved, reverted or read as zero),
//! the boot-gate marker states (the receiver was never halted, so it holds
//! its assembled `Normal` life), and the quorum gate fixed at construction
//! (Q1). Every node that hears the announcement records the sender as a
//! gossip witness, whatever its own transition's outcome.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_pair};
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

/// The relation of one numeric field against the receiver's own value.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Rel { Less, Equal, Greater }

const REL_ALL: [Rel; 3] = [Rel::Less, Rel::Equal, Rel::Greater];

#[rustfmt::skip]
fn rel(r: Rel, base: u64) -> u64 {
    match r { Rel::Less => base - 1, Rel::Equal => base, Rel::Greater => base + 1 }
}

/// The receiver's role against the current view's primary.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Role { Leader, Backup }

const ROLE_ALL: [Role; 2] = [Role::Leader, Role::Backup];

/// The sender against the pair the announcement names.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Sender { Matches, Mismatch }

const SENDER_ALL: [Sender; 2] = [Sender::Matches, Sender::Mismatch];

/// Whether the announced pair is distinct.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Pair { Distinct, Same }

const PAIR_ALL: [Pair; 2] = [Pair::Distinct, Pair::Same];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { Refused, Armed { push: bool } }

#[rustfmt::skip]
fn route(role: Role, sender: Sender, pair: Pair, r: Rel) -> Route {
    match role {
        Role::Backup => Route::Refused,
        Role::Leader => match sender {
            Sender::Mismatch => Route::Refused,
            Sender::Matches => match pair {
                Pair::Same => Route::Refused,
                Pair::Distinct => match r {
                    Rel::Less => Route::Armed { push: true },
                    _ => Route::Armed { push: false },
                },
            },
        },
    }
}

/// A three-node cluster at view 1, every node `Normal`.
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
fn run_case(role: Role, sender: Sender, pair: Pair, r: Rel) {
    let (old_system, old_crash) = mint_pair();
    let (new_system, new_crash) = mint_pair();
    let op = OperationId { msb: (u64::from(old_system.get()) << 16) | u64::from(old_crash.get()), lsb: 1 };
    assert!(op.msb != 0 && op.lsb != 0, "the mint never draws a zero half");
    let _ = op;
    let old = NodeId::new(old_system, old_crash);
    let mut new = NodeId::new(new_system, new_crash);
    if new == old {
        // Two draws can land on one pair in one clock tick; the crash
        // counter's lawful advance separates them.
        new = new.next_life().expect("the mint keeps two lives of bump headroom");
    }
    if let Pair::Same = pair { new = old; }
    assert!(old.is_lawful() && new.is_lawful(), "the mint draws strictly inside");
    let mut h = assembled();
    let receiver = match role { Role::Leader => n(1), Role::Backup => n(2) };
    let before = h.snapshot(receiver).expect("the receiver is live");
    let header_view = ViewId { era: Era(before.era), view: View(before.view) };
    let announced = match sender { Sender::Matches => new, Sender::Mismatch => { let (a, b) = mint_pair(); NodeId::new(a, b) } };
    let prepared = Slot(rel(r, before.committed));
    let header = Header { tag: Tag::Reincarnation, view: header_view, slot: Slot::NONE };
    let body = Body::Reincarnation { old, new, committed: Slot(2), prepared };
    let outcome = h.inject(announced, receiver, Message { header, body });
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let sent = released(&outcome);
    let published = matches!(outcome, StepOutcome::Published { .. });
    assert!(h.witnesses(receiver).contains(&announced),
        "every node that hears the announcement records the sender");
    match route(role, sender, pair, r) {
        Route::Refused => {
            assert!(published && sent.is_empty(), "a refused announcement emits nothing: {sent:?}");
            let named = matches!(diagnostic, Some(Diagnostic::ReincarnationRefused { sender: x, .. })
                if x == announced);
            assert!(named, "{diagnostic:?}");
            assert_eq!(after.committed, before.committed, "the machine never armed");
        }
        Route::Armed { push } => {
            assert!(published, "the armed sequence publishes: {outcome:?}");
            assert!(matches!(diagnostic, Some(Diagnostic::None)), "{diagnostic:?}");
            let expected = if push { 4 } else { 3 };
            assert_eq!(sent.len(), expected, "{sent:?}");
            if push {
                assert!(sent[0].0 == new && sent[0].1.header.tag == Tag::NewState
                    && sent[0].1.header.view == header_view,
                    "the immediate ack precedes the first step: {sent:?}");
            }
            assert!(sent.iter().any(|(to, m)| *to == new && m.header.tag == Tag::Prepare),
                "the memo stream's first beat rides the establishing prepare: {sent:?}");
            assert_eq!(after.accepted, 3, "the first forced step is proposed");
            let held = h.journal_entry(receiver, Slot(3)).expect("the entry is journaled");
            let joins = matches!(&held.payload, vrr::journal::Payload::System(
                vrr::configuration::SystemOperation::Batch(ops))
                if matches!(ops.as_slice(),
                    [vrr::configuration::SystemOperation::Join { node, .. }] if *node == new));
            assert!(joins, "the joining identity joins at weight zero: {:?}", held.payload);
        }
    }
}

#[test]
fn exhaustive_reincarnation_at_a_node() {
    for role in ROLE_ALL {
        for sender in SENDER_ALL {
            for pair in PAIR_ALL {
                for r in REL_ALL {
                    run_case(role, sender, pair, r);
                }
            }
        }
    }
}
