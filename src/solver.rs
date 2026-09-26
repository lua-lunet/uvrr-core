//! Constructive operator planning for strict weighted majorities.
//!
//! Endpoints must each have an available majority. Equal memberships use the
//! shortest unit-weight path: decrease unavailable, increase available, decrease
//! available, increase unavailable. Membership/order changes use an available
//! one-voter intermediate configuration, transferring its vote if necessary.
//! This preserves quorum availability but need not preserve failure tolerance.
//! The output is a proposal: commit each batch through the ordinary protocol,
//! acquire state before promotion, and perform any required view change.

use crate::configuration::{ConfigError, Configuration, SystemOperation, Weight};
use crate::ids::{NodeId, Slot};
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
/// Leadership and state acquisition are protocol actions, not implied by a plan.
/// This guarantee applies to `WeightedMajority`, not arbitrary quorum strategies.
pub fn solve(
    current: &Configuration,
    target: &Configuration,
    live: &[NodeId],
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
                return Ok(current.plan(&[scale])?);
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
    if same_order {
        Ok(current.plan(&b.ops)?)
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
        Ok(steps)
    }
}

/// Replace one incarnation, retaining its weight and succession position.
///
/// Prefer the standard forced schedule (including all six weighted eras for a
/// five-unit-voter replacement) whenever every intermediate has an available
/// majority and the endpoint matches. Otherwise use the general constructor.
/// The new identity must have acquired state before its promotion is committed.
pub fn solve_replacement(
    current: &Configuration,
    old: NodeId,
    new: NodeId,
    live: &[NodeId],
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
        Ok(steps)
    } else {
        solve(current, &target, live)
    }
}
