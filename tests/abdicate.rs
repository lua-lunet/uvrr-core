//! Protocol suite for the abdication message (rules §12 of
//! `docs/uvrr-reconfiguration-rules.md`).
//!
//! The administrator's message names the view the cluster is in and the view
//! it should move to. The leader validates, in order: the CAS, the
//! receiver-is-primary check, and the delta rule
//! `0 < (target − current) ≤ N` — each a named refusal. On a valid
//! abdication the leader emits the standard view-change message set for the
//! target view and steps down in the same transition: no new wire message
//! exists, and the successor the target schedule names resumes as primary
//! through the ordinary fence/evidence/install pipeline. Every property is
//! exercised through the public harness interface.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Harness, StepOutcome};
use vrr::effects::Effect;
use vrr::ids::{Era, NodeId, OperationId, Slot, View, ViewId};
use vrr::message::Body;
use vrr::observe::Diagnostic;
use vrr::progress::{ProgressSnapshot, Status};
use vrr::reconfiguration::{Abdication, AbdicationRefusal};
use vrr::replica::PlanRefusal;
use vrr::wire::Tag;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn n(id: u32) -> NodeId {
    NodeId(id)
}

fn at(era: u32, term: u32) -> ViewId {
    ViewId {
        era: Era(era),
        view: View(term),
    }
}

fn op_id(lsb: u64) -> OperationId {
    OperationId { msb: 0, lsb }
}

fn snap(h: &Harness, id: NodeId) -> ProgressSnapshot {
    h.snapshot(id).expect("the node is live")
}

fn status_of(h: &Harness, id: NodeId) -> Status {
    Status::from_word(snap(h, id).status).expect("the word is a status")
}

fn current_view(h: &Harness, id: NodeId) -> ViewId {
    let snapshot = snap(h, id);
    ViewId {
        era: Era(snapshot.era),
        view: View(snapshot.view),
    }
}

/// Drives the cluster to its first primary's view: every node `Normal`, the
/// genesis primary serving.
fn bootstrap(h: &mut Harness, ids: &[NodeId]) {
    h.tick_all();
    h.deliver_all();
    for &id in ids {
        assert_eq!(status_of(h, id), Status::Normal, "{id:?} settled");
        assert_eq!(current_view(h, id), at(1, 0), "{id:?} at the genesis view");
    }
    h.assert_safety();
}

/// Drives one proposal at `primary` to full commitment everywhere and applies
/// it.
fn commit_one(h: &mut Harness, ids: &[NodeId], primary: NodeId, lsb: u64, payload: &[u8]) {
    h.propose(primary, op_id(lsb), payload);
    h.deliver_all();
    for &id in ids {
        h.execute_apply_effects(id);
    }
}

/// The start of the relocation: `DC1:a` leads, the membership order holds
/// `a, b` per site, so `DC3:a` sits four positions along the order.
fn three_site_cluster() -> Harness {
    Harness::provision(6)
}

fn three_site_ids() -> Vec<NodeId> {
    (0..6).map(n).collect()
}

// ---------------------------------------------------------------------------
// The named refusals, in §12's order.
// ---------------------------------------------------------------------------

#[test]
fn abdic_cas_failure_is_refused() {
    let mut h = three_site_cluster();
    let ids = three_site_ids();
    bootstrap(&mut h, &ids);

    // The message names a view the cluster is not in: the CAS fails.
    let named = at(1, 7);
    let message = Abdication {
        current: named,
        target: View(11),
    };
    let outcome = h.abdicate(n(0), message);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Abdication(AbdicationRefusal::NotTheNamedEra {
            current: at(1, 0),
            named,
        })),
        "a stale CAS pair is refused by name"
    );

    // Zero protocol messages, no state change, no fault.
    assert_eq!(h.queued_len(), 0, "the refusal emits nothing");
    for &id in &ids {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} unmoved");
        assert_eq!(current_view(&h, id), at(1, 0), "{id:?} unmoved");
    }
    h.assert_safety();
}

#[test]
fn abdic_by_a_non_primary_is_refused() {
    let mut h = Harness::provision(3);
    let ids: Vec<NodeId> = (0..3).map(n).collect();
    bootstrap(&mut h, &ids);

    // The abdication arrives at a backup: the receiver is not the primary of
    // the named view.
    let message = Abdication {
        current: at(1, 0),
        target: View(1),
    };
    let outcome = h.abdicate(n(1), message);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Abdication(
            AbdicationRefusal::ReceiverNotPrimary {
                primary: Some(n(0))
            }
        )),
        "a backup is not the primary of the named view"
    );

    // Zero protocol messages, no state change.
    assert_eq!(h.queued_len(), 0, "the refusal emits nothing");
    for &id in &ids {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} unmoved");
        assert_eq!(current_view(&h, id), at(1, 0), "{id:?} unmoved");
    }
    h.assert_safety();
}

#[test]
fn abdic_non_positive_delta_is_refused() {
    let mut h = Harness::provision(3);
    let ids: Vec<NodeId> = (0..3).map(n).collect();
    bootstrap(&mut h, &ids);

    // A target equal to the named view: the delta is zero.
    let message = Abdication {
        current: at(1, 0),
        target: View(0),
    };
    let outcome = h.abdicate(n(0), message);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Abdication(
            AbdicationRefusal::NonPositiveDelta {
                current: View(0),
                target: View(0),
            }
        )),
        "a zero delta is refused by name"
    );

    // A target behind the named view: the delta is negative. The cluster
    // first advances to term 2, whose primary is n2.
    let outcome = h.force_view(n(0), at(1, 2));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    for &id in &ids {
        assert_eq!(current_view(&h, id), at(1, 2), "{id:?} entered term 2");
    }
    let message = Abdication {
        current: at(1, 2),
        target: View(1),
    };
    let outcome = h.abdicate(n(2), message);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Abdication(
            AbdicationRefusal::NonPositiveDelta {
                current: View(2),
                target: View(1),
            }
        )),
        "a negative delta is refused by name"
    );

    // Nothing moved: no fence, no traffic.
    for &id in &ids {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} unmoved");
        assert_eq!(current_view(&h, id), at(1, 2), "{id:?} unmoved");
    }
    assert_eq!(h.queued_len(), 0, "the refusals emit nothing");
    h.assert_safety();
}

#[test]
fn abdic_delta_above_n_is_refused() {
    let mut h = three_site_cluster();
    let ids = three_site_ids();
    bootstrap(&mut h, &ids);

    // A cluster at genesis term 0 with N = 6 members: a bump of N + 1 is
    // beyond the total set.
    let message = Abdication {
        current: at(1, 0),
        target: View(7),
    };
    let outcome = h.abdicate(n(0), message);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Abdication(
            AbdicationRefusal::DeltaAboveMembers {
                delta: 7,
                members: 6,
            }
        )),
        "a bump beyond the member count is refused by name"
    );

    // The long-running shape: the cluster is driven to term 1234 (view gaps
    // are legal, §8.7.3), whose primary is order[1234 mod 6] = n4.
    let outcome = h.force_view(n(0), at(1, 1234));
    assert!(matches!(outcome, StepOutcome::Published { .. }));
    h.deliver_all();
    for &id in &ids {
        assert_eq!(
            current_view(&h, id),
            at(1, 1234),
            "{id:?} entered term 1234"
        );
    }

    // Era 1234 asked to become era 1234 + N + 1: the bump wastes the space
    // the schedule depends on.
    let message = Abdication {
        current: at(1, 1234),
        target: View(1241),
    };
    let outcome = h.abdicate(n(4), message);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Abdication(
            AbdicationRefusal::DeltaAboveMembers {
                delta: 7,
                members: 6,
            }
        )),
        "the 1234-to-1234-plus-N-plus-one shape is refused by name"
    );

    // The 2^32 shape: a bump to the end of the succession space, far beyond
    // the total set.
    let message = Abdication {
        current: at(1, 1234),
        target: View(u32::MAX),
    };
    let outcome = h.abdicate(n(4), message);
    assert_eq!(
        outcome,
        StepOutcome::PlanRefused(PlanRefusal::Abdication(
            AbdicationRefusal::DeltaAboveMembers {
                delta: u32::MAX - 1234,
                members: 6,
            }
        )),
        "the 2^32 shape is refused by name"
    );

    // Nothing moved: no fence, no traffic, no fault.
    for &id in &ids {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} unmoved");
        assert_eq!(current_view(&h, id), at(1, 1234), "{id:?} unmoved");
    }
    assert_eq!(h.queued_len(), 0, "the refusals emit nothing");
    assert!(!snap(&h, n(4)).faulted);
    h.assert_safety();
}

// ---------------------------------------------------------------------------
// The valid relocation: the emission is the standard view-change set, and the
// leader steps down in the same transition.
// ---------------------------------------------------------------------------

#[test]
fn valid_abdic_relocation_emits_the_standard_view_change() {
    let mut h = three_site_cluster();
    let ids = three_site_ids();
    bootstrap(&mut h, &ids);

    // The +4 relocation: DC1:a hands the leadership to DC3:a, skipping
    // DC1:b, DC2:a and DC2:b along the order.
    let message = Abdication {
        current: at(1, 0),
        target: View(4),
    };
    let outcome = h.abdicate(n(0), message);
    let effects = match outcome {
        StepOutcome::Published { effects, .. } => effects,
        other => panic!("the valid abdication publishes: {other:?}"),
    };

    // The emission is exactly the standard view-change message set for the
    // target view: one StartViewChange per other member, header slot absent,
    // routed under the target view's era. No new tag appears — every tag on
    // the emission is an existing protocol tag.
    assert_eq!(effects.len(), 5, "one datagram per other member");
    let mut recipients = Vec::new();
    for effect in &effects {
        let Effect::Send { to, era, message } = effect else {
            panic!("the emission is sends only: {effect:?}")
        };
        assert_eq!(*era, Era(1), "the datagram is routed under the target era");
        assert_eq!(message.header.tag, Tag::StartViewChange, "the standard tag");
        assert_eq!(message.header.view, at(1, 4), "the target view's header");
        assert_eq!(message.header.slot, Slot::NONE, "a fence claims no history");
        assert_eq!(message.body, Body::StartViewChange {}, "the standard body");
        recipients.push(*to);
    }
    recipients.sort();
    assert_eq!(
        recipients,
        vec![n(1), n(2), n(3), n(4), n(5)],
        "every other member is addressed, in order"
    );

    // The network holds exactly the emission, and nothing else.
    assert_eq!(h.queued_len(), 5, "only the emission is in flight");

    // The leader stepped down in the same transition: it is fenced at the
    // target view, and a proposal is redirected to the successor the
    // target's schedule names — order[4 mod 6] = n4 = DC3:a.
    assert_eq!(
        status_of(&h, n(0)),
        Status::ViewChange,
        "the leader stepped down"
    );
    assert_eq!(
        current_view(&h, n(0)),
        at(1, 4),
        "fenced at the target view"
    );
    let refused = h.propose(n(0), op_id(1), b"z");
    assert_eq!(
        refused,
        StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
            view: at(1, 4),
            primary: Some(n(4)),
        }),
        "the abdicating leader is leader no more"
    );
    h.assert_safety();
}

#[test]
fn the_abdicating_leader_is_leader_no_more() {
    let mut h = three_site_cluster();
    let ids = three_site_ids();
    bootstrap(&mut h, &ids);
    commit_one(&mut h, &ids, n(0), 1, b"a"); // slot 3 committed and applied everywhere

    // The +4 relocation. The emission is armed and the leader steps down in
    // the same transition.
    let message = Abdication {
        current: at(1, 0),
        target: View(4),
    };
    let outcome = h.abdicate(n(0), message);
    assert!(matches!(outcome, StepOutcome::Published { .. }));

    // The standard view-change pipeline carries the handover: the successor
    // the target era's schedule names — n4 — resumes as primary.
    h.deliver_all();
    for &id in &ids {
        assert_eq!(status_of(&h, id), Status::Normal, "{id:?} settled");
        assert_eq!(
            current_view(&h, id),
            at(1, 4),
            "{id:?} entered the target view"
        );
    }

    // The former leader's further leadership is refused, and any message it
    // still sends in its old view is discarded by the standard checks.
    let refused = h.propose(n(0), op_id(2), b"b");
    assert_eq!(
        refused,
        StepOutcome::PlanRefused(PlanRefusal::NotPrimary {
            view: at(1, 4),
            primary: Some(n(4)),
        }),
        "the former leader redirects"
    );
    let stale = vrr::message::Message {
        header: vrr::wire::Header {
            tag: Tag::Commit,
            view: at(1, 0),
            slot: Slot(3),
        },
        body: Body::Commit { committed: Slot(3) },
    };
    let injected = h.inject(n(0), n(1), stale);
    assert!(
        matches!(injected, StepOutcome::Published { .. }),
        "the stale message is dropped, not faulted: {injected:?}"
    );
    assert_eq!(
        h.diagnostic(n(1)),
        Some(Diagnostic::ViewMismatch {
            got: at(1, 0),
            current: at(1, 4),
        }),
        "the standard fence check discarded the former leader's message"
    );

    // The successor serves the stream in the target view.
    commit_one(&mut h, &ids, n(4), 2, b"b"); // slot 4
    for &id in &ids {
        let applied: Vec<Vec<u8>> = h
            .applied(id)
            .iter()
            .map(|(_, payload)| payload.to_vec())
            .collect();
        assert_eq!(
            applied,
            vec![b"a".to_vec(), b"b".to_vec()],
            "{id:?} applied the whole stream in slot order"
        );
    }
    h.assert_safety();
}
