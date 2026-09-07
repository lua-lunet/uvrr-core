//! The era planner: reduce-left batch evaluation
//! (`docs/uvrr-reconfiguration-rules.md` §5).
//!
//! The leader never proposes an unsafe batch. Given a stream of operations it runs the
//! **reduce-left partitioner**: a fold carrying the tuple
//! `(taken ops, configuration so far, mass moved)`. For each incoming operation:
//!
//! 1. try the operation against the accumulated in-batch configuration;
//! 2. if the growing batch stays legal — R13–R15 and every per-op boundary at its
//!    point in the sequence — **take** it: extend the taken list and the
//!    configuration;
//! 3. if it would violate a rule, **pass only the prior list**: close the batch —
//!    the taken ops become one era's establishing operation — and retry the
//!    operation as the first op of the next batch.
//!
//! The legality oracle for every what-if is [`Configuration::apply`] itself, on a
//! clone: everything is an immutable value and every operation is a pure function
//! `Configuration → Result<Configuration>`, so the planner and the fold cannot
//! disagree about what is legal, and the evaluation is thread-safe by construction,
//! with no lock and no mutation.
//!
//! The classic shapes fall out mechanically (§5): a ton of zero-weight joins stays in
//! one era because each moves no mass (R14); the resurrection four split into the
//! canonical two; a scaling op forces the next op into a new batch (R13); and a batch
//! is never nested nor seeded with genesis (R15). A stream operation that is illegal
//! even alone refuses the plan outright — the partitioner invents no eras around a
//! boundary the fold refuses.

use crate::configuration::{ConfigError, Configuration, SystemOperation};
use crate::ids::Slot;

/// One committed era of a planned reconfiguration (rules §4, §5).
///
/// `ops` become ONE [`SystemOperation::Batch`] establishing operation — one era, one
/// WAL entry (§9) — and `config` is the configuration that era establishes: the fold
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
    ///   own boundary at its point in the sequence — the same refusals the fold
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
