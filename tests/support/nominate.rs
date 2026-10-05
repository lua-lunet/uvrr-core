//! The NOMINATE ladder's shared machinery (`docs/uvrr-protocols.md`, the
//! NOMINATE chapter): the solver's schedules driven through the ordinary
//! pipeline with the constant-leader assertion, shared by the suites that
//! step or fuse the same schedules.

use crate::harness::Harness;
use uvrr::configuration::{Configuration, Member, SystemOperation, Weight};
use uvrr::ids::{Ballot, Era, NodeId, Slot, View, next_view_selecting};
use uvrr::solve;

/// The harness's provisioned identity by index.
pub fn n(id: usize) -> NodeId {
    crate::harness::provisioned_id(id)
}

/// A member of a configuration the scripts build, by provisioned index.
pub fn member(id: usize, weight: u32) -> Member {
    Member {
        node: n(id),
        weight: Weight(weight),
    }
}

/// Forces the cluster's serving view to `(1, view)` (§14.2), the leader
/// the primary under the genesis order.
pub fn serve_at(h: &mut Harness, view: u32) {
    h.force_view(
        n(0),
        Ballot {
            era: Era(1),
            view: View(view),
        },
    );
    h.quiesce();
    for id in [n(0), n(1), n(2)] {
        let snapshot = h.snapshot(id).expect("the node is live");
        assert_eq!(
            (snapshot.era, snapshot.view),
            (1, view),
            "n{} serves at the forced view",
            id.0
        );
    }
    h.assert_safety();
}

/// The leader the node's own committed history names at its view number.
pub fn committed_leader(h: &Harness, id: NodeId) -> Option<NodeId> {
    let snapshot = h.snapshot(id).expect("the node is live");
    h.era_table(id)
        .expect("the node is live")
        .current()
        .config
        .primary(View(snapshot.view))
}

/// The committed configuration the cluster's history has established.
pub fn committed_config(h: &Harness, id: NodeId) -> Configuration {
    h.era_table(id)
        .expect("the node is live")
        .current()
        .config
        .as_ref()
        .clone()
}

/// The solver's schedule: the exhaustive path from the committed
/// configuration to the target, folded at the running era and view into
/// one step per establishing slot, each step's `Nominate` rider computed
/// by the fold's leader-stability arithmetic.
pub fn solver_steps(
    start: &Configuration,
    target: &Configuration,
    view: View,
) -> Vec<Vec<SystemOperation>> {
    let path = solve::solve(start, target).expect("the scenario solves");
    let steps = solve::fold(&path, start, (start.era(), view, Slot(0)))
        .expect("the fold rides a legal path");
    steps.into_iter().map(|step| step.ops).collect()
}

/// The constant-leader assertion: at the slot that just committed, every
/// live member whose own committed configuration gives it positive weight
/// names the same leader.
pub fn assert_constant_leader(
    h: &Harness,
    roster: &[NodeId],
    leader: NodeId,
    scenario: &str,
    step: usize,
    slot: Slot,
) {
    for id in roster {
        let Some(snapshot) = h.snapshot(*id) else {
            continue;
        };
        let table = h.era_table(*id).expect("the node is live");
        let voting = table
            .current()
            .config
            .weight_of(*id)
            .is_some_and(|weight| weight.0 > 0);
        if !voting {
            continue;
        }
        let named = committed_leader(h, *id);
        assert_eq!(
            named,
            Some(leader),
            "{scenario} step {step} (committed slot {:?}): n{} names {named:?} at view {}, the constant leader is n{}",
            slot.0,
            id.0,
            snapshot.view,
            leader.0
        );
    }
}

/// The era boundary where no rider landed (§14.2, §8.7.8): the view lags
/// the established era and the gate would refuse the next establishing
/// operation, so the script drives the leader-preserving forced change,
/// the least view past the current one selecting the constant leader
/// under the era's voters, the same number the rider the wrap would have
/// carried names.
pub fn cross_boundary(h: &mut Harness, roster: &[NodeId], leader: NodeId, scenario: &str) {
    let snapshot = h.snapshot(leader).expect("the leader is live");
    let table = h.era_table(leader).expect("the leader is live");
    let established = table.current().era;
    if Era(snapshot.era) == established {
        return;
    }
    let voters: Vec<NodeId> = table
        .current()
        .config
        .order()
        .iter()
        .filter(|member| member.weight.0 > 0)
        .map(|member| member.node)
        .collect();
    let index = voters
        .iter()
        .position(|&voter| voter == leader)
        .expect("the constant leader is a voter of the established era");
    let target = next_view_selecting(
        View(snapshot.view),
        u32::try_from(index).expect("the roster fits the view arithmetic"),
        u32::try_from(voters.len()).expect("the roster fits the view arithmetic"),
    )
    .expect("a view selecting the leader always exists past any view");
    h.force_view(
        leader,
        Ballot {
            era: established,
            view: target,
        },
    );
    h.quiesce();
    for id in roster {
        let Some(live) = h.snapshot(*id) else {
            continue;
        };
        // The fenced non-member does not cross: the exchange installs at
        // the era's members, and a standby outside the configuration is
        // the §10 acquisition's business, not the boundary's.
        let voting = h
            .era_table(*id)
            .expect("the node is live")
            .current()
            .config
            .weight_of(*id)
            .is_some_and(|weight| weight.0 > 0);
        if !voting {
            continue;
        }
        assert_eq!(
            (live.era, live.view),
            (established.0, target.0),
            "{scenario}: n{} crossed the boundary into the established era",
            id.0
        );
    }
    assert_constant_leader(h, roster, leader, scenario, usize::MAX, Slot(0));
    h.assert_safety();
}
