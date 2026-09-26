//! Exhaustive per-message properties: the client request the host proposes
//! (§6, §11.1, B2).
//!
//! One input (`Input::Propose`) over the cross product of the dimensions the
//! handler branches on: the node's role against the current view's primary,
//! the node's boot status, and whether the same identity is proposed twice.
//! Exhaustive by construction under the host obligations: the identity law
//! (universally unique and never recycled, so the proposed identity belongs
//! to exactly one operation and the refusal to deduplicate can never alias
//! two proposals onto one identity), the boot-gate marker states (a halt
//! classifies `Clean` and reopens fenced `Restarting`), and the quorum gate
//! fixed at construction (Q1). The core never inspects the identity and never
//! deduplicates on it (B2), so the twice-proposed identity orders twice, at
//! two slots.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome, mint_pair};
use vrr::effects::Effect;
use vrr::ids::{CrashCounter, NodeId, OperationId, Slot, SystemId};
use vrr::journal::Payload;
use vrr::replica::{PlanRefusal, ViewChangeKnobs};
use vrr::wire::Tag;

const TIMEOUT: u64 = 3;
const PAYLOAD: &[u8] = b"ordered";

#[rustfmt::skip]
fn n(id: u32) -> NodeId {
    let system = SystemId::new((id + 1) as u16).expect("small and non-zero");
    let life = CrashCounter::new(1).expect("one is non-zero");
    NodeId::new(system, life)
}

/// The node the host proposes to.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Role { Primary, Backup }

const ROLE_ALL: [Role; 2] = [Role::Primary, Role::Backup];

/// The node's assembled boot status.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Boot { Normal, Joining, Restarting }

const BOOT_ALL: [Boot; 3] = [Boot::Normal, Boot::Joining, Boot::Restarting];

/// Whether the host proposes the same identity twice.
#[rustfmt::skip]
#[derive(Clone, Copy)]
enum Repeat { Once, Twice }

const REPEAT_ALL: [Repeat; 2] = [Repeat::Once, Repeat::Twice];

/// What the handler does with the case, named by an exhaustive match.
#[rustfmt::skip]
enum Route { NotPrimary { primary: NodeId }, Accept }

#[rustfmt::skip]
fn route(role: Role, boot: Boot) -> Route {
    match boot {
        Boot::Joining => Route::NotPrimary { primary: n(0) },
        Boot::Restarting => Route::NotPrimary { primary: n(1) },
        Boot::Normal => match role {
            Role::Primary => Route::Accept,
            Role::Backup => Route::NotPrimary { primary: n(1) },
        },
    }
}

/// The cluster assembled for the case: provisioned, bootstrapped to view 1
/// unless the case inspects the fenced entry state, the receiver possibly
/// reopened from a halt.
#[rustfmt::skip]
fn assembled(role: Role, boot: Boot) -> Harness {
    let knobs = ViewChangeKnobs { primary_timeout: TIMEOUT, view_change_budget: usize::MAX };
    let mut h = Harness::with_knobs(3, knobs);
    if let Boot::Joining = boot { return h; }
    h.tick_all();
    h.deliver_all();
    for _ in 0..=TIMEOUT { h.tick(n(1)); }
    h.deliver_all();
    if let Boot::Restarting = boot {
        let receiver = match role { Role::Primary => n(1), Role::Backup => n(2) };
        h.halt(receiver); h.restart_with(receiver).expect("the halt classifies clean");
    }
    h
}

#[rustfmt::skip]
fn run_case(role: Role, boot: Boot, repeat: Repeat) {
    let (system, crash) = mint_pair();
    let op = OperationId { msb: u64::from(system.get()) << 16 | u64::from(crash.get()), lsb: 1 };
    assert!(op.msb != 0 && op.lsb != 0, "the mint never draws a zero half");
    let mut h = assembled(role, boot);
    let receiver = match role { Role::Primary => n(1), Role::Backup => n(2) };
    let outcome = h.propose(receiver, op, PAYLOAD);
    let second = match repeat {
        Repeat::Twice => Some(h.propose(receiver, op, PAYLOAD)),
        Repeat::Once => None,
    };
    assert!(h.fault_of(receiver).is_none() && h.check_safety().is_ok());
    let sent = |o: &StepOutcome| match o {
        StepOutcome::Published { effects, .. } => effects.iter().filter(|e| matches!(e,
            Effect::Send { message, .. } if message.header.tag == Tag::Prepare)).count(),
        _ => 0,
    };
    match route(role, boot) {
        Route::NotPrimary { primary } => {
            let refused = matches!(&outcome, StepOutcome::PlanRefused(
                PlanRefusal::NotPrimary { primary: named, .. }) if *named == Some(primary));
            assert!(refused, "the refusal names the current view's primary: {outcome:?}");
            assert!(sent(&outcome) == 0, "a refusal emits nothing");
            if let Some(repeated) = second {
                assert!(matches!(repeated, StepOutcome::PlanRefused(PlanRefusal::NotPrimary { .. })),
                    "the refusal reproduces: {repeated:?}");
            }
        }
        Route::Accept => {
            assert!(matches!(outcome, StepOutcome::Published { .. }),
                "the primary accepts the proposal: {outcome:?}");
            assert_eq!(sent(&outcome), 2, "the proposal broadcasts two Prepares for slot 3");
            match second {
                None => {
                    assert_eq!(h.snapshot(receiver).expect("live").accepted, 3,
                        "one proposal, one slot");
                }
                Some(StepOutcome::Published { .. }) => {
                    assert_eq!(sent_proposed_slot(&second), Slot(4), "the second slot is new");
                    let after = h.snapshot(receiver).expect("live");
                    assert_eq!(after.accepted, 4, "no deduplication: the second slot is new");
                    let first = op_id_at(&h, receiver, Slot(3));
                    let again = op_id_at(&h, receiver, Slot(4));
                    assert_eq!(first, op, "the identity carried opaque at slot 3");
                    assert_eq!(again, op, "the same identity ordered twice, never rewritten");
                }
                Some(other) => panic!("the second proposal publishes: {other:?}"),
            }
        }
    }
}

#[rustfmt::skip]
fn sent_proposed_slot(second: &Option<StepOutcome>) -> Slot {
    let Some(StepOutcome::Published { effects, .. }) = second else { return Slot(0) };
    let Some(Effect::Send { message, .. }) = effects.first() else { return Slot(0) };
    message.header.slot
}

#[rustfmt::skip]
fn op_id_at(h: &Harness, receiver: NodeId, slot: Slot) -> OperationId {
    let held = h.journal_entry(receiver, slot).expect("the slot is journaled");
    let Payload::Operation { id, .. } = &held.payload else {
        panic!("an operation slot at {slot:?}: {:?}", held.payload);
    };
    *id
}

#[test]
fn exhaustive_client_request_at_a_node() {
    for role in ROLE_ALL {
        for boot in BOOT_ALL {
            for repeat in REPEAT_ALL {
                run_case(role, boot, repeat);
            }
        }
    }
}
