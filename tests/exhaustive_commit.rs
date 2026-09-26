//! Exhaustive per-message properties: the `Commit` any node receives (§4, §13.3).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the receiver's boot status, the message's view against its current
//! view, and the carried commit frontier against its committed frontier.
//! Exhaustive by construction under the host obligations: the identity law
//! (never recycled, so the receiver's durable frontiers are exactly what the
//! assembled state publishes), the boot-gate marker states (a halt
//! classifies `Clean` and reopens fenced `Restarting`, where the boot fence
//! holds the §10 carve-out), and the quorum gate fixed at construction (Q1).
//! The frontier never claims what the journal does not record (§5 invariant
//! 2), which the assembled accepted frontier makes exact here.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_pair};
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, Era, NodeId, OperationId, Slot, SystemId, View, ViewId};
use vrr::message::{Body, Message};
use vrr::observe::Diagnostic;
use vrr::progress::Status;
use vrr::replica::ViewChangeKnobs;
use vrr::wire::{Header, Tag};

const TIMEOUT: u64 = 3;
const PAYLOAD: &[u8] = b"carried";

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

/// The receiver's assembled boot status.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Boot { Normal, Restarting }

const BOOT_ALL: [Boot; 2] = [Boot::Normal, Boot::Restarting];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { Mismatch, Idle, AdoptIdle, Commit, Fence }

#[rustfmt::skip]
fn route(boot: Boot, v: Rel, f: Rel) -> Route {
    match v {
        Rel::Less => Route::Mismatch,
        Rel::Greater => match boot {
            Boot::Normal => Route::Fence,
            Boot::Restarting => Route::Mismatch,
        },
        Rel::Equal => match f {
            Rel::Greater => Route::Commit,
            _ => match boot {
                Boot::Normal => Route::Idle,
                Boot::Restarting => Route::AdoptIdle,
            },
        },
    }
}

/// A three-node cluster at view 1, the receiver holding slot 3 accepted but
/// uncommitted, assembled `Normal` or reopened fenced from a halt.
#[rustfmt::skip]
fn assembled(boot: Boot, op: OperationId) -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT { h.tick(n(1)); }
    h.deliver_all();
    h.propose(n(1), op, PAYLOAD);
    h.deliver_to_matching(n(2), Tag::Prepare, Slot(3));
    if let Boot::Restarting = boot {
        h.halt(n(2)); h.restart_with(n(2)).expect("the halt classifies clean");
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
fn run_case(boot: Boot, v: Rel, f: Rel) {
    let (system, crash) = mint_pair();
    let op = OperationId { msb: u64::from(system.get()) << 16 | u64::from(crash.get()), lsb: 1 };
    assert!(op.msb != 0 && op.lsb != 0, "the mint never draws a zero half");
    let mut h = assembled(boot, op);
    let receiver = n(2);
    let before = h.snapshot(receiver).expect("the receiver is live");
    let committed = before.committed;
    let header_view = view(rel(v, u64::from(before.view)) as u32);
    let sender = n(rel(v, 1) as u32);
    let frontier = Slot(rel(f, committed));
    let header = Header { tag: Tag::Commit, view: header_view, slot: frontier };
    let body = Body::Commit { committed: frontier };
    let outcome = h.inject(sender, receiver, Message { header, body });
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let sent = released(&outcome);
    let published = matches!(outcome, StepOutcome::Published { .. });
    let mismatch = matches!(diagnostic, Some(Diagnostic::ViewMismatch { got, .. }) if got == header_view);
    let normal = Status::from_word(after.status) == Some(Status::Normal);
    match route(boot, v, f) {
        Route::Mismatch => {
            assert!(published && sent.is_empty() && mismatch, "{sent:?} {diagnostic:?}");
            assert_eq!(after.committed, committed, "a drop moves no frontier");
        }
        Route::Idle => {
            assert!(published && sent.is_empty() && matches!(diagnostic, Some(Diagnostic::None)),
                "{sent:?} {diagnostic:?}");
            assert_eq!(after.committed, committed);
            assert_eq!(after.accepted, before.accepted);
        }
        Route::AdoptIdle => {
            assert!(published && sent.is_empty(), "{sent:?}");
            assert!(normal, "the boot-fenced member adopts the legitimate stream's view");
            assert_eq!(after.committed, committed);
        }
        Route::Commit => {
            assert!(published);
            assert_eq!(after.committed, committed + 1, "the frontier advanced to the carried one");
            assert_eq!(sent.len(), 0, "the newly committed operation applies, it does not send");
            assert!(
                matches!(released_apply(&outcome), Some((Slot(3), id)) if id == op),
                "the Apply carries the minted identity: {outcome:?}"
            );
        }
        Route::Fence => {
            assert!(published && sent.len() == 3, "two fences and the fetch: {sent:?}");
            assert_eq!(Status::from_word(after.status), Some(Status::ViewChange));
            assert_eq!(after.view, before.view + 1);
            assert!(sent.iter().any(|(to, m)| *to == sender && m.header.tag == Tag::GetState
                && matches!(&m.body, Body::GetState { from: Slot(x) } if *x == 4)), "{sent:?}");
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
fn exhaustive_commit_at_a_backup() {
    for boot in BOOT_ALL {
        for v in REL_ALL {
            for f in REL_ALL {
                run_case(boot, v, f);
            }
        }
    }
}
