//! Exhaustive per-message properties: the rejoin gossip request a node hears
//! (`docs/uvrr-rejoin-gossip-and-witnesses.md` §2 to §3).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the receiver's role (the leader answers, every other node only
//! records), the sender's membership (a cluster member is pushed but never
//! listed; a node outside the cluster uses the request as its join), and the
//! sender's prepared frontier against the receiver's committed frontier (the
//! push's extent). Exhaustive by construction under the host obligations:
//! the identity law (universally unique and never recycled, so the minted
//! foreign sender is outside the configuration for the life of the test and
//! the witness list names it exactly once), the boot-gate marker states (the
//! receiver was never halted, so it holds its assembled `Normal` life), and
//! the quorum gate fixed at construction (Q1). Every node that hears the
//! request records the sender, whatever its own transition's outcome, and
//! the list never names a voter. The request's frontiers ride the body; the
//! header slot is the `Slot(0)` sentinel (the per-tag table's Absent role).

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

/// Who the gossip claims to come from.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Sender { Member, Foreign }

const SENDER_ALL: [Sender; 2] = [Sender::Member, Sender::Foreign];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { HearOnly, Answer { push: bool } }

#[rustfmt::skip]
fn route(role: Role, r: Rel) -> Route {
    match role {
        Role::Backup => Route::HearOnly,
        Role::Leader => match r {
            Rel::Less => Route::Answer { push: true },
            _ => Route::Answer { push: false },
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
fn run_case(role: Role, sender: Sender, r: Rel) {
    let (system, crash) = mint_pair();
    let op = OperationId { msb: u64::from(system.get()) << 16 | u64::from(crash.get()), lsb: 1 };
    assert!(op.msb != 0 && op.lsb != 0, "the mint never draws a zero half");
    let _ = op;
    let mut h = assembled();
    let receiver = match role {
        Role::Leader => n(1),
        Role::Backup => n(2),
    };
    let before = h.snapshot(receiver).expect("the receiver is live");
    let committed = before.committed;
    let header_view = ViewId { era: Era(before.era), view: View(before.view) };
    let from = match sender {
        Sender::Member => n(0),
        Sender::Foreign => mint_id(),
    };
    assert!(from.is_lawful(), "the mint draws strictly inside the identity space");
    let prepared = Slot(rel(r, committed));
    let header = Header { tag: Tag::GossipRequest, view: header_view, slot: Slot::NONE };
    let body = Body::GossipRequest { prepared, committed: prepared };
    let outcome = h.inject(from, receiver, Message { header, body });
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let sent = released(&outcome);
    let published = matches!(outcome, StepOutcome::Published { .. });
    let listed = h.witnesses(receiver).contains(&from);
    let expected_list = matches!(sender, Sender::Foreign);
    assert_eq!(listed, expected_list, "the list never names a voter; a join is named");
    assert_eq!(after.committed, committed, "hearing gossip moves no frontier");
    match route(role, r) {
        Route::HearOnly => {
            assert!(published && sent.is_empty(), "a non-leader never answers: {sent:?}");
            assert!(matches!(diagnostic, Some(Diagnostic::None)), "{diagnostic:?}");
        }
        Route::Answer { push } => {
            assert!(published, "the leader's answer publishes: {outcome:?}");
            assert!(matches!(diagnostic, Some(Diagnostic::None)), "{diagnostic:?}");
            let expected = if push { 2 } else { 1 };
            assert_eq!(sent.len(), expected, "{sent:?}");
            assert_eq!(sent[0].0, from, "the answer addresses the sender");
            if push {
                assert!(sent[0].1.header.tag == Tag::NewState
                    && sent[0].1.header.view == header_view, "{sent:?}");
                let Body::NewState { entries, .. } = &sent[0].1.body else {
                    panic!("a NewState push: {:?}", sent[0].1.body);
                };
                assert_eq!(entries.len() as u64, committed - prepared.0, "the missed range");
            } else {
                assert_eq!(sent[0].1.header.tag, Tag::Commit, "an empty push, a fresh commit");
            }
            let tail = sent.last().expect("the answer's tail");
            assert!(tail.1.header.tag == Tag::Commit && tail.1.header.view == header_view
                && matches!(&tail.1.body, Body::Commit { committed: Slot(x) } if *x == committed),
                "the fresh commit closes the answer: {sent:?}");
        }
    }
}

#[test]
fn exhaustive_gossip_request_at_a_node() {
    for role in ROLE_ALL {
        for sender in SENDER_ALL {
            for r in REL_ALL {
                run_case(role, sender, r);
            }
        }
    }
}
