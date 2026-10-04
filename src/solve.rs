//! The reconfiguration solver: the legal path of alphabet operations between
//! two configurations, and the reduce-left fold that places the path into
//! slots, computing the era and the view each slot establishes.
//!
//! The alphabet is [`crate::configuration::SystemOperation`]: `Increment`,
//! `Decrement`, `Double`, `Halve`, `Join`, `Leave`, and the `Nominate` rider.
//! Legality is judged by [`Configuration::apply`], the proved fold engine
//! (rules §3–§4, R7–R15); the search never re-implements a rule. The search
//! is exhaustive over the mover set with a visited set over membership
//! shapes, so termination follows from the finite space (at most 16 members,
//! weights in {0, 1, 2}). The era advances at every establishing slot and the
//! `Nominate` rider tracks the running view, so the leader stays stable while
//! the cluster changes (`docs/uvrr-protocols.md`, the NOMINATE chapter).

use crate::configuration::{Configuration, MAX_WEIGHT, SystemOperation};
use crate::ids::{Era, NodeId, Slot, View};
use std::collections::BTreeSet;

/// The membership shape: the ordered (node, weight) pairs; the visited set
/// keys on this, so an era bump can never disguise an exhausted shape.
fn shape(c: &Configuration) -> Vec<(u32, u32)> {
    c.order().iter().map(|m| (m.node.0, m.weight.0)).collect()
}

/// The candidate movers, ordered towards the target first. Legality is
/// `apply`'s to refuse, except the two structural floors the solver owns: the
/// cluster never drops below two members, and `Join` positions name the
/// target's own succession index.
fn candidates(current: &Configuration, target: &Configuration) -> Vec<SystemOperation> {
    let at = |c: &Configuration, n: NodeId| c.weight_of(n).map(|w| w.0).unwrap_or(0);
    let mut ops = Vec::new();
    for m in target.order() {
        if !current.order().iter().any(|x| x.node == m.node) {
            let position = target
                .order()
                .iter()
                .position(|x| x.node == m.node)
                .expect("in target") as u32;
            ops.push(SystemOperation::Join {
                node: m.node,
                position,
            });
        }
        match at(current, m.node).cmp(&m.weight.0) {
            std::cmp::Ordering::Less => ops.push(SystemOperation::Increment(m.node)),
            std::cmp::Ordering::Greater => ops.push(SystemOperation::Decrement(m.node)),
            std::cmp::Ordering::Equal => {}
        }
    }
    for m in current.order() {
        if at(target, m.node) == 0 && current.len() > 2 {
            let op = if m.weight.0 == 0 {
                SystemOperation::Leave(m.node)
            } else {
                SystemOperation::Decrement(m.node)
            };
            ops.push(op);
        }
    }
    if current.order().iter().all(|m| m.weight.0 < MAX_WEIGHT) {
        ops.push(SystemOperation::Double);
    }
    if current.order().iter().all(|m| m.weight.0 % 2 == 0) {
        ops.push(SystemOperation::Halve);
    }
    ops
}

/// The legal operation path from `current` to `target`, or `None` when none
/// exists: the search is exhaustive, so `None` is a proof of unreachability
/// under the rules, not a timeout. The path carries no `Nominate` riders:
/// those are the fold's to mint, because their view arithmetic needs the
/// running view.
pub fn solve(current: &Configuration, target: &Configuration) -> Option<Vec<SystemOperation>> {
    fn go(
        current: &Configuration,
        target: &Configuration,
        visited: &mut BTreeSet<Vec<(u32, u32)>>,
        path: &mut Vec<SystemOperation>,
    ) -> bool {
        if shape(current) == shape(target) {
            return true;
        }
        if !visited.insert(shape(current)) {
            return false;
        }
        for op in candidates(current, target) {
            if let Ok(next) = current.apply(&op, Slot(0))
                && go(&next, target, visited, path)
            {
                path.push(op);
                return true;
            }
        }
        false
    }
    let mut path = Vec::new();
    if go(current, target, &mut BTreeSet::new(), &mut path) {
        path.reverse();
        Some(path)
    } else {
        None
    }
}

/// One slot of the plan: the era it establishes, the view the cluster holds
/// at its commit, and the operations it carries.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Step {
    /// The slot the establishing operation occupies.
    pub slot: Slot,
    /// The era the slot establishes.
    pub era: Era,
    /// The view the cluster holds at the slot's commit (after any rider bump).
    pub view: View,
    /// The operations the slot carries: the establishing operation, then its
    /// `Nominate` rider when one is owed.
    pub ops: Vec<SystemOperation>,
}

impl Step {
    /// The slot's payload: the bare operation, or the batch when a rider
    /// rides (R13/R15: the rider is never solitary, so it exists only as a
    /// batch's last sub-operation).
    pub fn payload(&self) -> SystemOperation {
        match self.ops.as_slice() {
            [op] => op.clone(),
            _ => SystemOperation::Batch(self.ops.clone()),
        }
    }
}

/// The reduce-left fold: the path into slots. The era advances at every
/// establishing slot. The running view advances by the rider's offset at
/// every ridden slot; the offset is the least positive bump with
/// `primary(next, view + offset) == primary(previous, view)`, the documented
/// leader-stability arithmetic (`docs/uvrr-protocols.md`, the NOMINATE
/// chapter). A scaling operation rides no rider (R13); a step that evicts the
/// leader gets no rider either, because no offset can preserve an evicted
/// leader: that leader change is genuine and is the host's view change to
/// drive.
pub fn fold(
    path: &[SystemOperation],
    current: &Configuration,
    start: (Era, View, Slot),
) -> Option<Vec<Step>> {
    let (mut era, mut view, mut slot) = start;
    let mut base = current.clone();
    let mut steps = Vec::with_capacity(path.len());
    for op in path {
        (era, slot) = (era.next()?, slot.next()?);
        let after = base.apply(op, slot).ok()?;
        let mut ops = vec![op.clone()];
        if !matches!(op, SystemOperation::Double | SystemOperation::Halve)
            && let Some(leader) = base.primary(view)
        {
            for offset in 1..=2 * base.len() + 2 {
                if after.primary(View(view.0 + offset)) == Some(leader) {
                    ops.push(SystemOperation::Nominate { from: view, offset });
                    view = View(view.0 + offset);
                    break;
                }
            }
        }
        steps.push(Step {
            slot,
            era,
            view,
            ops,
        });
        base = after;
    }
    Some(steps)
}
