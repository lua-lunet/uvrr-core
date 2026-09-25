//! Exhaustive per-message properties: the `PlannedViewChange` solicitation a
//! member answers (§8.7.7 step 2, step 3).
//!
//! One message over the cross product of the dimensions the handler branches
//! on: the solicitation's era against the successor era the answer speaks
//! from, the sender against the current view's serving primary, and the
//! receiver's boot status (a fenced node answers no solicitation). Exhaustive
//! by construction under the host obligations: the identity law (universally
//! unique and never recycled, so a minted foreign sender is outside the
//! configuration for the life of the test), the boot-gate marker states (a
//! halt classifies `Clean` and reopens fenced `Restarting` at its retained
//! view), and the quorum gate fixed at construction (Q1). The solicitation
//! is not a fence: a `Normal` recipient retains its view, stays `Normal`,
//! and answers from its own bounded suffix with planned evidence, which
//! fences nothing and counts toward no ordinary quorum.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_id};
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, Era, NodeId, Slot, SystemId, View, ViewId};
use vrr::message::{Body, EvidenceKind, Message};
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

/// The solicitation's era against the successor era.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum EraRel { Less, Equal, Greater }

const ERA_ALL: [EraRel; 3] = [EraRel::Less, EraRel::Equal, EraRel::Greater];

/// Who the solicitation claims to come from.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Sender { Primary, Foreign }

const SENDER_ALL: [Sender; 2] = [Sender::Primary, Sender::Foreign];

/// The receiver's assembled boot status.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Boot { Normal, Restarting }

const BOOT_ALL: [Boot; 2] = [Boot::Normal, Boot::Restarting];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { UnknownSender, Stale, Unevaluable, Fenced, Answer }

#[rustfmt::skip]
fn route(sender: Sender, era: EraRel, boot: Boot) -> Route {
    match sender {
        Sender::Foreign => Route::UnknownSender,
        Sender::Primary => match era {
            EraRel::Less => Route::Stale,
            EraRel::Greater => Route::Unevaluable,
            EraRel::Equal => match boot {
                Boot::Normal => Route::Answer,
                Boot::Restarting => Route::Fenced,
            },
        },
    }
}

/// A three-node cluster at view 1, the receiver assembled `Normal` or
/// reopened fenced from a halt.
#[rustfmt::skip]
fn assembled(boot: Boot) -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT { h.tick(n(1)); }
    h.deliver_all();
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
fn run_case(sender: Sender, era: EraRel, boot: Boot) {
    let mut h = assembled(boot);
    let receiver = n(2);
    let before = h.snapshot(receiver).expect("the receiver is live");
    let header_view = ViewId {
        era: match era {
            EraRel::Less => Era(before.era),
            EraRel::Equal => Era(before.era + 1),
            EraRel::Greater => Era(before.era + 2),
        },
        view: View(0),
    };
    let from = match sender { Sender::Primary => n(1), Sender::Foreign => mint_id() };
    let header = Header { tag: Tag::PlannedViewChange, view: header_view, slot: Slot::NONE };
    let body = Body::PlannedViewChange {};
    let outcome = h.inject(from, receiver, Message { header, body });
    let diagnostic = h.diagnostic(receiver);
    let after = h.snapshot(receiver).expect("the receiver is live");
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let sent = released(&outcome);
    let published = matches!(outcome, StepOutcome::Published { .. });
    match route(sender, era, boot) {
        Route::UnknownSender | Route::Stale | Route::Unevaluable | Route::Fenced => {
            assert!(published && sent.is_empty(), "a dropped solicitation emits nothing: {sent:?}");
            let named = match route(sender, era, boot) {
                Route::UnknownSender => matches!(diagnostic, Some(Diagnostic::UnknownSender { sender: x }) if x == from),
                Route::Stale => matches!(diagnostic, Some(Diagnostic::StaleEvidence { got, .. }) if got == header_view),
                Route::Unevaluable => matches!(diagnostic, Some(Diagnostic::UnevaluableEra { era }) if era == header_view.era),
                Route::Fenced => matches!(diagnostic, Some(Diagnostic::StaleViewChange { got, .. }) if got == header_view),
                _ => false,
            };
            assert!(named, "{diagnostic:?}");
        }
        Route::Answer => {
            assert!(published, "the answer publishes: {outcome:?}");
            assert_eq!(Status::from_word(after.status), Some(Status::Normal),
                "the solicitation is not a fence: the recipient stays normal");
            assert_eq!(after.view, before.view, "the recipient retains its view");
            assert_eq!(sent.len(), 2, "the planned answer and the suffix fetch: {sent:?}");
            assert!(sent[0].0 == from && sent[0].1.header.tag == Tag::DoViewChange
                && sent[0].1.header.view == header_view
                && matches!(&sent[0].1.body, Body::DoViewChange { evidence: EvidenceKind::Planned, .. }),
                "the answer is planned evidence: {sent:?}");
            assert!(sent[1].0 == from && sent[1].1.header.tag == Tag::GetState
                && matches!(&sent[1].1.body, Body::GetState { from: Slot(x) } if *x == 3),
                "the successor era's missing range is fetched: {sent:?}");
        }
    }
}

#[test]
fn exhaustive_planned_view_change_at_a_node() {
    for sender in SENDER_ALL {
        for era in ERA_ALL {
            for boot in BOOT_ALL {
                run_case(sender, era, boot);
            }
        }
    }
}
