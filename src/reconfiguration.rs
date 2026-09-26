//! The era planner: reduce-left batch evaluation
//! (`docs/uvrr-reconfiguration-rules.md` §5).
//!
//! The leader never proposes an unsafe batch. Given a stream of operations it runs the
//! **reduce-left partitioner**: a fold carrying the tuple
//! `(taken ops, configuration so far, mass moved)`. For each incoming operation:
//!
//! 1. try the operation against the accumulated in-batch configuration;
//! 2. if the growing batch stays legal, R13–R15 and every per-op boundary at its
//!    point in the sequence, **take** it: extend the taken list and the
//!    configuration;
//! 3. if it would violate a rule, **pass only the prior list**: close the batch,
//!    the taken ops become one era's establishing operation, and retry the
//!    operation as the first op of the next batch.
//!
//! The legality oracle for every what-if is [`Configuration::apply`] itself, on a
//! clone: everything is an immutable value and every operation is a pure function
//! `Configuration → Result<Configuration>`, so the planner and the fold cannot
//! disagree about what is legal, and the evaluation is thread-safe by construction,
//! with no lock and no mutation.
//!
//! The classic shapes fall out mechanically (§5): a ton of zero-weight joins stays in
//! one era because each moves no mass (R14); the reincarnation four split into the
//! canonical two; a scaling op forces the next op into a new batch (R13); and a batch
//! is never nested nor seeded with genesis (R15). A stream operation that is illegal
//! even alone refuses the plan outright, the partitioner invents no eras around a
//! boundary the fold refuses.

use crate::configuration::{ConfigError, Configuration, SystemOperation};
use crate::effects::Effect;
use crate::ids::{NodeId, Slot, View, ViewId};
use crate::message::{Body, Message};
use crate::wire::{Header, Tag};

/// One committed era of a planned reconfiguration (rules §4, §5).
///
/// `ops` become ONE [`SystemOperation::Batch`] establishing operation, one era, one
/// WAL entry (§9), and `config` is the configuration that era establishes: the fold
/// of the batch onto the previous era's configuration. The planner never stores
/// steps; a [`Configuration::plan`] result is a proposal the leader still evaluates
/// against the current committed configuration.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EraStep {
    /// The batch's sub-operations, in application order.
    pub ops: Vec<SystemOperation>,
    /// The configuration this batch's era establishes.
    pub config: Configuration,
}

impl Configuration {
    /// Partitions an operation stream into legal eras (§5, the reduce-left
    /// partitioner): each returned [`EraStep`] folds as one [`SystemOperation::Batch`]
    /// establishing operation, and the steps' configurations are the era sequence the
    /// WAL would replay.
    ///
    /// # Preconditions
    ///
    /// * No genesis operation in the stream (R15: [`ConfigError::GenesisNotPlannable`]).
    /// * Every taken batch obeys the batch rules R13–R15 and every sub-operation's
    ///   own boundary at its point in the sequence, the same refusals the fold
    ///   produces, surfaced from the what-if (`EmptyBatch` cannot occur here: a
    ///   batch is only ever closed with at least one taken op).
    /// * An operation that is illegal even as the first op of a fresh batch refuses
    ///   the whole plan: the partitioner closes nothing and invents no eras around a
    ///   boundary the fold refuses.
    ///
    /// The receiver is untouched: this is a pure what-if on immutable values, the
    /// same arithmetic the leader runs on a clone before proposing a batch for
    /// consensus (§5). Zero-weight joins and leaves move no mass, so any number of
    /// them is taken into one era (R14); a scaling op is taken only alone and forces
    /// the next op into a new batch (R13).
    pub fn plan(&self, ops: &[SystemOperation]) -> Result<Vec<EraStep>, ConfigError> {
        let mut steps: Vec<EraStep> = Vec::new();
        // The reduce-left tuple: (taken ops, in-batch configuration). The mass
        // moved is carried by the fold's oracle: the batch legality each take is
        // judged by is `apply`'s own R13–R15 arithmetic, so the planner and the
        // fold cannot disagree.
        let mut batch_start = self.clone();
        let mut taken: Vec<SystemOperation> = Vec::new();
        let mut current = self.clone();
        for op in ops {
            if matches!(op, SystemOperation::Void | SystemOperation::Init { .. }) {
                return Err(ConfigError::GenesisNotPlannable);
            }
            let mut candidate = taken.clone();
            candidate.push(op.clone());
            // The `at` slot is consulted by `apply` only for the two genesis
            // ordinals, which are refused above; any slot proves the batch
            // rules. `Slot(0)` names "no committed position" here.
            match batch_start.apply(&SystemOperation::Batch(candidate.clone()), Slot(0)) {
                Ok(next) => {
                    taken = candidate;
                    current = next;
                }
                Err(error) => {
                    if taken.is_empty() {
                        return Err(error);
                    }
                    // Close the batch: the taken ops are one era's establishing
                    // operation, and the retried op starts the next batch.
                    steps.push(EraStep {
                        ops: taken,
                        config: current.clone(),
                    });
                    batch_start = current.clone();
                    match batch_start.apply(&SystemOperation::Batch(vec![op.clone()]), Slot(0)) {
                        Ok(next) => {
                            taken = vec![op.clone()];
                            current = next;
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
        }
        // The trailing batch, if any operation was taken into it.
        if !taken.is_empty() {
            steps.push(EraStep {
                ops: taken,
                config: current,
            });
        }
        Ok(steps)
    }
}

/// The administrator's abdication message (rules §12 of
/// `docs/uvrr-reconfiguration-rules.md`). It is a reconfiguration message sent
/// by an administrator to the leader; it is not a folded batch operation and
/// it never changes a weight, so it is not a [`SystemOperation`] and carries
/// no wire encoding, the host→replica input is its whole shape. Its two
/// fields are the CAS pair: the view the sender believes the cluster is in,
/// and the view the cluster should move to (§12 names both "eras": the
/// schedule the delta rule bounds is the primary-succession schedule, which
/// is this crate's view arithmetic, `primary(v) = voters[v mod voters]`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Abdication {
    /// The view the sender believes the cluster is in: both the era and the
    /// succession term, so the CAS cannot pass across an era change the
    /// sender has not seen.
    pub current: ViewId,
    /// The succession term the cluster should move to, within the named era.
    /// The successor the abdication hands the leadership to is
    /// `primary(target)` under the named era's configuration.
    pub target: View,
}

/// Why an abdication was refused (rules §12). One variant per check, in the
/// order the checks run: a host reacting to a refusal asserts *which*
/// precondition failed. A refusal emits no protocol traffic and moves no
/// state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AbdicationRefusal {
    /// The CAS failed: the cluster is not in the view the message names
    /// (§12 check 1). The stale command answers itself by naming both sides
    /// of the disagreement.
    NotTheNamedEra {
        /// The view the cluster is in.
        current: ViewId,
        /// The view the message names.
        named: ViewId,
    },
    /// The receiver is not the primary of the named view (§12 check 2): the
    /// message reached a node that is not the leader it claims to address.
    /// Carries the primary that configuration names for the view, when it
    /// can name one, so the administrator can redirect.
    ReceiverNotPrimary {
        /// The primary of the named view under its era's configuration.
        primary: Option<NodeId>,
    },
    /// The delta rule refused a target at or behind the named view (§12
    /// check 3: eras do not move backwards). A succession term at the end of
    /// its space has no nameable successor, so an abdication there refuses
    /// on this variant too, every nameable target is at or behind it.
    NonPositiveDelta {
        /// The named current term.
        current: View,
        /// The refused target.
        target: View,
    },
    /// The delta rule refused a bump beyond the member count `N` (§12 check
    /// 3): a bump larger than the total set would waste the succession
    /// space the schedule depends on. Carries the bump and the bound it
    /// broke.
    DeltaAboveMembers {
        /// The bump the message asked for: `target − current`.
        delta: u32,
        /// The member count `N` of the current configuration.
        members: u32,
    },
}

/// The standard view-change emission a valid abdication arms (rules §12): no
/// new wire message exists, and the abdication is answered by exactly the
/// protocol messages the ordinary view change would run.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AbdicationEmission {
    /// The view the standard view change enters; its primary is the
    /// successor the named era's schedule selects, `primary(target)`.
    pub view: ViewId,
    /// The standard fence datagrams: one `StartViewChange` per other member
    /// of the configuration, header slot absent, routed under the target
    /// view's era.
    pub messages: Vec<Effect>,
}

/// Validates an administrator's abdication against the cluster membership and
/// returns the standard view-change emission, or the named refusal (rules §12).
pub fn validate_abdication(
    message: &Abdication,
    membership: &Configuration,
    current: ViewId,
    receiver: NodeId,
) -> Result<AbdicationEmission, AbdicationRefusal> {
    // Check 1: the CAS, the cluster is in the view the message names.
    if message.current != current {
        return Err(AbdicationRefusal::NotTheNamedEra {
            current,
            named: message.current,
        });
    }
    // Check 2: the receiver is the primary of that view.
    let primary = membership.primary(message.current.view);
    if primary != Some(receiver) {
        return Err(AbdicationRefusal::ReceiverNotPrimary { primary });
    }
    // Check 3: the delta rule, `0 < (target − current) <= N`.
    if message.target.0 <= current.view.0 {
        return Err(AbdicationRefusal::NonPositiveDelta {
            current: current.view,
            target: message.target,
        });
    }
    let delta = message.target.0 - current.view.0;
    let members = membership.len();
    if delta > members {
        return Err(AbdicationRefusal::DeltaAboveMembers { delta, members });
    }
    let view = ViewId {
        era: current.era,
        view: message.target,
    };
    let fence = Message {
        header: Header {
            tag: Tag::StartViewChange,
            view,
            slot: Slot::NONE,
        },
        body: Body::StartViewChange {},
    };
    let messages = membership
        .order()
        .iter()
        .map(|member| member.node)
        .filter(|node| *node != receiver)
        .map(|to| Effect::Send {
            to,
            era: view.era,
            message: fence.clone(),
        })
        .collect();
    Ok(AbdicationEmission { view, messages })
}
