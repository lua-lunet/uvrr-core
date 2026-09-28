//! Constructive operator planning for strict weighted majorities.
//!
//! Endpoints must each have an available majority. Equal memberships use the
//! shortest unit-weight path: decrease unavailable, increase available, decrease
//! available, increase unavailable. Membership/order changes use an available
//! one-voter intermediate configuration, transferring its vote if necessary.
//! This preserves quorum availability but need not preserve failure tolerance.
//! Every step carries a nomination rider that bumps the view into the era the
//! step establishes, keeping the leader constant through the plan
//! (`docs/uvrr-protocols.md`, the NOMINATE chapter). The output is a proposal: commit
//! each batch through the ordinary protocol and acquire state before
//! promotion; the rider removes the view change the boundaries would
//! otherwise need.

use crate::configuration::{ConfigError, Configuration, Snapshot, SystemOperation, Weight};
use crate::ids::{NodeId, Slot, View};
use crate::reconfiguration::EraStep;

/// Why no executable weighted-majority plan was returned.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SolveError {
    /// The supplied available identities cannot form a majority at this endpoint.
    NoAvailableMajority {
        /// True for the requested target, false for the current configuration.
        target: bool,
        /// Available voting mass.
        available: u64,
        /// Total voting mass.
        total: u64,
    },
    /// An operation cannot be represented by the configuration fold.
    Configuration(ConfigError),
    /// Replacement requires distinct old and fresh identities.
    InvalidReplacement,
}

impl std::fmt::Display for SolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAvailableMajority {
                target,
                available,
                total,
            } => write!(
                f,
                "{} configuration has available mass {available}/{total}; strict majority required",
                if *target { "target" } else { "current" }
            ),
            Self::Configuration(error) => write!(f, "configuration refused: {error:?}"),
            Self::InvalidReplacement => write!(
                f,
                "replacement requires an existing old identity and a fresh, distinct new identity"
            ),
        }
    }
}
impl std::error::Error for SolveError {}
impl From<ConfigError> for SolveError {
    fn from(value: ConfigError) -> Self {
        Self::Configuration(value)
    }
}

fn available_mass(c: &Configuration, live: &[NodeId]) -> u64 {
    c.order()
        .iter()
        .filter(|m| live.contains(&m.node))
        .map(|m| u64::from(m.weight.0))
        .sum()
}
fn available(c: &Configuration, live: &[NodeId], target: bool) -> Result<(), SolveError> {
    let mass = available_mass(c, live);
    if 2 * mass > c.total() {
        Ok(())
    } else {
        Err(SolveError::NoAvailableMajority {
            target,
            available: mass,
            total: c.total(),
        })
    }
}

/// The nomination riders the steps carry
/// (`docs/uvrr-protocols.md`, the NOMINATE chapter): every step whose batch carries
/// no scaling operation gains a `Nominate { from: v, offset: u }` as its
/// last sub-operation, `v` the running view and `u` the least positive
/// offset with `primary(next, v + u) == primary(previous, v)`. The rider's
/// bump is the era entry the §8.7.8 gate demands: the re-electing offset
/// across a wrap that would move the leader, and the count, the least
/// offset that preserves the index, at a step that keeps it. A scaling
/// step carries no rider: R13 keeps [`crate::configuration::SystemOperation::Double`] and [`crate::configuration::SystemOperation::Halve`] solitary, and the
/// scaling preserves the positive-weight sequence elementwise, so the
/// leader never moves at one; that step's era boundary is crossed by the
/// leader-preserving view change the host drives (§14.2), the least view
/// past the running one selecting the constant leader, the same number
/// `u` the rider would have named, so the chained views agree whichever
/// carries the boundary. The running view advances by `u` at every step;
/// the chained `from` values are the views the cluster actually holds,
/// each rider's bump landing at its step's commit. A step that evicts the
/// serving leader from the voter sequence cannot keep it: no offset
/// re-elects the evicted, the rider names the era-entering increment
/// alone, and the leadership passes to the arithmetic's choice; the next
/// step's emission continues from the leader the new arithmetic names. A
/// saturated view space stops advancing: the riders past it name views
/// the space cannot hold and commit inertly.
fn with_nominations(steps: Vec<EraStep>, start: &Configuration, view: View) -> Vec<EraStep> {
    let mut previous = start.clone();
    let mut running = view;
    let mut riders = Vec::with_capacity(steps.len());
    for mut step in steps {
        // `from` is the view the cluster holds when this step commits: the
        // running view before this step's bump or boundary change lands.
        let from = running;
        let leader = previous.primary(from);
        let voters = u32::try_from(
            step.config
                .order()
                .iter()
                .filter(|member| member.weight.0 > 0)
                .count(),
        )
        .expect("the membership cap bounds the voter count");
        let mut offset = 1;
        let mut elects = false;
        for u in 1..=voters {
            if let Some(number) = from.0.checked_add(u)
                && step.config.primary(View(number)) == leader
            {
                offset = u;
                elects = true;
                break;
            }
        }
        // The rider rides unless the batch scales: R13 keeps the scaling
        // op solitary, and the era entry is the host's boundary change.
        let scaling = step
            .ops
            .iter()
            .any(|op| matches!(op, SystemOperation::Double | SystemOperation::Halve));
        if !scaling && (elects || step.config.primary(from) != leader) {
            step.ops.push(SystemOperation::Nominate { from, offset });
        }
        running = View(from.0.checked_add(offset).unwrap_or(from.0));
        let config = step.config.clone();
        riders.push(step);
        previous = config;
    }
    riders
}

struct Builder {
    current: Configuration,
    ops: Vec<SystemOperation>,
}
impl Builder {
    fn push(&mut self, op: SystemOperation) -> Result<(), SolveError> {
        self.current = self.current.apply(&op, Slot(0))?;
        self.ops.push(op);
        Ok(())
    }
    fn weight(&mut self, node: NodeId, target: u32) -> Result<(), SolveError> {
        let mut w = self.current.weight_of(node).unwrap_or(Weight(0)).0;
        while w != target {
            self.push(if w < target {
                SystemOperation::Increment(node)
            } else {
                SystemOperation::Decrement(node)
            })?;
            w = self.current.weight_of(node).unwrap_or(Weight(0)).0;
        }
        Ok(())
    }
}

/// Compute a finite safe schedule to the target's exact ordered membership.
///
/// The target's era is ignored: returned eras follow the current era. Available
/// identities are a set (duplicates and identities outside either endpoint have
/// no additional weight). Both endpoints must have a strict available majority.
/// Every prefix retains that property and every adjacent pair has intersecting
/// strict-majority families. The weight alphabet remains `{0,1,2}`.
///
/// Equal ordered identities take exactly the L1 weight distance in unit changes;
/// an exact global double/halve takes one era. Different identities or order use
/// a constructive one-voter intermediate, within the membership cap. That route
/// can temporarily concentrate authority: operators requiring additional failure
/// tolerance must evaluate the returned configurations against their policy.
/// Leadership and state acquisition are protocol actions, not implied by a plan,
/// and the nomination riders carry the leadership through: every step's batch
/// ends in a [`crate::configuration::SystemOperation::Nominate`] whose CAS names the running view and whose bump enters
/// the era the step establishes (`docs/uvrr-protocols.md`, the NOMINATE chapter), so the
/// view the parameter names is the view the leader serves throughout, no
/// view-change message needed between eras. This guarantee applies to
/// [`crate::quorum::WeightedMajority`], not arbitrary quorum strategies.
pub fn solve(
    current: &Configuration,
    target: &Configuration,
    live: &[NodeId],
    view: View,
) -> Result<Vec<EraStep>, SolveError> {
    available(current, live, false)?;
    available(target, live, true)?;
    if current.order() == target.order() {
        return Ok(Vec::new());
    }
    let same_order = current
        .order()
        .iter()
        .map(|m| m.node)
        .eq(target.order().iter().map(|m| m.node));
    let mut b = Builder {
        current: current.clone(),
        ops: Vec::new(),
    };
    if same_order {
        for scale in [SystemOperation::Double, SystemOperation::Halve] {
            if let Ok(c) = current.apply(&scale, Slot(0))
                && c.order() == target.order()
            {
                return Ok(with_nominations(current.plan(&[scale])?, current, view));
            }
        }
        for (is_live, increase) in [(false, false), (true, true), (true, false), (false, true)] {
            for m in target.order() {
                let w = b.current.weight_of(m.node).unwrap_or(Weight(0)).0;
                if live.contains(&m.node) == is_live && (w < m.weight.0) == increase {
                    b.weight(m.node, m.weight.0)?;
                }
            }
        }
    } else {
        // The common-prefix route: the longest prefix the two orders share,
        // member and weight alike, stays put, the current's tail drains and
        // leaves, and the target's tail joins and takes its weights, each
        // live joiner promoted at its join (the expansion's shape: join at
        // weight zero, increment, join, increment) with the dead joiners'
        // weights restored last. The prefix-only intermediate must itself
        // be a legal, available configuration: a prefix whose mass cannot
        // hold a majority falls back to the anchor route below, which
        // reserves one live voter and is always available.
        let prefix = current
            .order()
            .iter()
            .zip(target.order())
            .take_while(|(before, after)| {
                before.node == after.node && before.weight == after.weight
            })
            .count();
        let prefix_only = Snapshot {
            era: current.era(),
            order: current.order()[..prefix].to_vec(),
        }
        .inflate();
        if let Ok(prefix_only) = prefix_only
            && available(&prefix_only, live, false).is_ok()
        {
            for is_live in [false, true] {
                for m in current.order().iter().skip(prefix) {
                    if live.contains(&m.node) == is_live {
                        b.weight(m.node, 0)?;
                    }
                }
            }
            for m in current.order().iter().skip(prefix) {
                b.push(SystemOperation::Leave(m.node))?;
            }
            for (position, m) in target.order().iter().enumerate().skip(prefix) {
                b.push(SystemOperation::Join {
                    node: m.node,
                    position: u32::try_from(position).expect("membership cap"),
                })?;
                if live.contains(&m.node) {
                    b.weight(m.node, m.weight.0)?;
                }
            }
            for m in target.order().iter().skip(prefix) {
                if !live.contains(&m.node) {
                    b.weight(m.node, m.weight.0)?;
                }
            }
        } else {
            // Drain unavailable votes first. Removing other live votes then leaves
            // an available anchor; there is never a zero-total configuration.
            let anchor = current
                .order()
                .iter()
                .find(|m| m.weight.0 > 0 && live.contains(&m.node))
                .expect("available majority supplies a voter")
                .node;
            for is_live in [false, true] {
                for m in current.order() {
                    if m.node != anchor && live.contains(&m.node) == is_live {
                        b.weight(m.node, 0)?;
                    }
                }
            }
            b.weight(anchor, 1)?;
            for m in current.order() {
                if m.node != anchor {
                    b.push(SystemOperation::Leave(m.node))?;
                }
            }
            let target_anchor = target
                .order()
                .iter()
                .find(|m| m.node == anchor && m.weight.0 > 0)
                .or_else(|| {
                    target
                        .order()
                        .iter()
                        .find(|m| m.weight.0 > 0 && live.contains(&m.node))
                })
                .expect("target available majority supplies a voter")
                .node;
            if target_anchor != anchor {
                b.push(SystemOperation::Join {
                    node: target_anchor,
                    position: 0,
                })?;
                b.weight(target_anchor, 1)?;
                b.weight(anchor, 0)?;
                b.push(SystemOperation::Leave(anchor))?;
            }
            // Insert around the retained anchor in target order, at zero weight.
            for (position, m) in target.order().iter().enumerate() {
                if m.node != target_anchor {
                    b.push(SystemOperation::Join {
                        node: m.node,
                        position: u32::try_from(position).expect("membership cap"),
                    })?;
                }
            }
            // Establish the target's entire live mass before restoring dead mass.
            for is_live in [true, false] {
                for m in target.order() {
                    if live.contains(&m.node) == is_live {
                        b.weight(m.node, m.weight.0)?;
                    }
                }
            }
        }
    }
    if same_order {
        Ok(with_nominations(current.plan(&b.ops)?, current, view))
    } else {
        // Keep membership edits separate: batching leaves and fresh joins could
        // make the endpoint union exceed the quorum gate's enumeration cap.
        let mut c = current.clone();
        let mut steps = Vec::new();
        for op in b.ops {
            c = c.apply(&SystemOperation::Batch(vec![op.clone()]), Slot(0))?;
            steps.push(EraStep {
                ops: vec![op],
                config: c.clone(),
            });
        }
        Ok(with_nominations(steps, current, view))
    }
}

/// Replace one incarnation, retaining its weight and succession position.
///
/// Prefer the standard forced schedule (including all six weighted eras for a
/// five-unit-voter replacement) whenever every intermediate has an available
/// majority and the endpoint matches. Otherwise use the general constructor.
/// The new identity must have acquired state before its promotion is
/// committed. The steps carry the nomination riders like `solve`'s
/// (`docs/uvrr-protocols.md`, the NOMINATE chapter): the forced-reincarnation
/// machine's own runtime recomputation (`replica::forced_steps`) emits
/// none, its leadership is the fence machinery's business, the plans this
/// function returns are the operator's.
pub fn solve_replacement(
    current: &Configuration,
    old: NodeId,
    new: NodeId,
    live: &[NodeId],
    view: View,
) -> Result<Vec<EraStep>, SolveError> {
    if old == new || current.weight_of(old).is_none() || current.weight_of(new).is_some() {
        return Err(SolveError::InvalidReplacement);
    }
    let mut snapshot = current.to_snapshot();
    for m in &mut snapshot.order {
        if m.node == old {
            m.node = new;
        }
    }
    let target = snapshot.inflate()?;
    available(current, live, false)?;
    available(&target, live, true)?;
    let ops = crate::replica::forced_steps(current, old, new);
    // forced_steps already returns one Batch per era; replay those boundaries.
    let mut c = current.clone();
    let mut steps = Vec::new();
    let mut valid = true;
    for op in ops {
        match c.apply(&op, Slot(0)) {
            Ok(next) if available(&next, live, false).is_ok() => {
                let ops = match op {
                    SystemOperation::Batch(ops) => ops,
                    op => vec![op],
                };
                steps.push(EraStep {
                    ops,
                    config: next.clone(),
                });
                c = next;
            }
            _ => {
                valid = false;
                break;
            }
        }
    }
    if valid && c.order() == target.order() {
        Ok(with_nominations(steps, current, view))
    } else {
        solve(current, &target, live, view)
    }
}
