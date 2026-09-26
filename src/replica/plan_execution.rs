//! Plan execution: the leader side of the operator's reconfiguration plan
//! (`docs/weighted-reconfiguration-solver.md`).
//!
//! The operator computes a plan once and submits it over the node's admin
//! ingress. The leader validates it against its current committed
//! configuration ([`Plan::validate_against`]) and answers with one verdict;
//! on acceptance the [`PlannedSequence`] machine is armed and steps through
//! the plan's batches while the cluster keeps running normally: one step per
//! era, each proposed through [`plan_step_proposal`], the fuse envelope
//! when the step packs at least two operations within the envelope budget,
//! the ordinary establishing `Prepare` otherwise
//! (`docs/uvrr-fuse.md` §4). The forced-reincarnation machine steps its own
//! sequence the same way, one step per era on a tick, through
//! [`plan_reconfigure`] (§8 of the fuse doc).
//!
//! The machine is volatile like every attempt state: a leader crash discards
//! it, and the dumb-operator contract hands continuation to the operator, a
//! new leader executes nothing automatically; the plan is re-solicited against
//! the configuration that committed. A step the gates refuse is drift: a plan
//! computed to be legal cannot become illegal, so the refusal means the
//! cluster changed underneath the plan, and aborting is the correct behaviour
//!, the machine clears and [`Diagnostic::PlanAborted`] names it.
//!
//! [`plan_reconfigure`]: super::Replica::plan_reconfigure
//! [`plan_step_proposal`]: super::Replica::plan_step_proposal

use crate::configuration::SystemOperation;
use crate::effects::{Effect, PlanVerdict};
use crate::ids::Slot;
use crate::journal::{JournalView, Payload};
use crate::observe::Diagnostic;
use crate::plan::Plan;
use crate::progress::Status;

use super::{
    InputKind, Journal, JournalMutation, PlanExecutionUpdate, PlanRefusal, PlannedTransition,
    QuorumStrategy, Replica,
};

/// Why a [`SubmitPlan`](super::Input::SubmitPlan) input was refused before
/// the plan's own validation could speak.
const NOT_LEADER: &str = "the replica is not the leader of its current view";

/// The leader's armed plan-execution machine
/// (`docs/weighted-reconfiguration-solver.md`): the accepted plan's batches
/// and the index of the next step to propose.
///
/// Volatile by design: a leader crash discards it, and the operator re-plans
/// from the configuration that committed. The steps are never stored anywhere
/// else, the configuration history is their only authority.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(in crate::replica) struct PlannedSequence {
    /// One batch per era, in commit order, exactly as accepted.
    pub steps: Vec<Vec<SystemOperation>>,
    /// The zero-based index of the next step to propose.
    pub next: usize,
}

impl<J: Journal, Q: QuorumStrategy> Replica<J, Q> {
    /// A plan submission at the admin ingress (`docs/weighted-
    /// reconfiguration-solver.md`): validate against the current committed
    /// configuration, answer the verdict, and on acceptance arm the machine.
    ///
    /// Only the leader of its current view executes plans; any other node
    /// answers `Rejected` with the named precondition. The acceptance rule is
    /// [`Plan::validate_against`] verbatim; a plan whose FIRST step is
    /// immediately plannable proposes it in the same transition (mirroring
    /// `plan_reincarnation`'s arm-and-propose). A gate refusal there, the
    /// closed intersection gate is the one gate the fold-based validation
    /// does not run, propagates as the named [`PlanRefusal`] and arms
    /// nothing: the plan never started, so there is no verdict to answer.
    pub(in crate::replica) fn plan_submit_plan(
        &self,
        journal: &J::View,
        plan: &Plan,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let current = self.progress.current();
        let is_leader =
            self.progress.status() == Status::Normal && self.primary_of(current) == Some(self.own);
        if !is_leader {
            return self.admin_verdict(
                PlanVerdict::Rejected {
                    reason: NOT_LEADER.to_string(),
                },
                kind,
            );
        }
        let committed = &self.progress.config().current().config;
        match plan.validate_against(committed) {
            Err(rejection) => self.admin_verdict(
                PlanVerdict::Rejected {
                    reason: rejection.to_string(),
                },
                kind,
            ),
            Ok(()) => {
                let machine = PlannedSequence {
                    steps: plan.steps.clone(),
                    next: 0,
                };
                let Some(step) = machine.steps.first() else {
                    // An empty plan folds to the configuration it names: the
                    // verdict is the whole execution, nothing to arm.
                    return self.admin_verdict(PlanVerdict::Accepted, kind);
                };
                let batch = SystemOperation::Batch(step.clone());
                if self.plan_step_plannable(journal, &batch) {
                    Ok(self
                        .plan_step_proposal(journal, &batch)?
                        .with_effect(Effect::AdminResponse {
                            verdict: PlanVerdict::Accepted,
                        })
                        .with_plan_execution(PlanExecutionUpdate::Set(machine)))
                } else {
                    // A step is in flight or an era transition awaits the
                    // view change into it: arm, continue on a tick.
                    Ok(self
                        .admin_verdict(PlanVerdict::Accepted, kind)?
                        .with_plan_execution(PlanExecutionUpdate::Set(machine)))
                }
            }
        }
    }

    /// The tick-driven continuation: the armed leader proposes the next
    /// accepted step. The drift gate runs first, a step the current
    /// committed configuration refuses is dead whatever the pipeline's
    /// readiness, so the machine clears and [`Diagnostic::PlanAborted`]
    /// names the step. `None` leaves the tick to the ordinary machinery,
    /// the machine sits armed and inert until the conditions return.
    pub(in crate::replica) fn plan_execution_continuation(
        &self,
        journal: &J::View,
    ) -> Option<Result<PlannedTransition, PlanRefusal>> {
        let machine = self.plan_execution.clone()?;
        let current = self.progress.current();
        let is_leader =
            self.progress.status() == Status::Normal && self.primary_of(current) == Some(self.own);
        if !is_leader {
            return None;
        }
        let batch = SystemOperation::Batch(machine.steps[machine.next].clone());
        if self
            .progress
            .config()
            .current()
            .config
            .apply(&batch, Slot(0))
            .is_err()
        {
            return Some(
                self.drop_plan(
                    Diagnostic::PlanAborted { step: machine.next },
                    InputKind::Tick,
                )
                .map(|plan| plan.with_plan_execution(PlanExecutionUpdate::Clear)),
            );
        }
        if self.plan_step_plannable(journal, &batch) {
            Some(
                self.plan_step_proposal(journal, &batch)
                    .map(|plan| plan.with_plan_execution(PlanExecutionUpdate::Set(machine))),
            )
        } else {
            None
        }
    }

    /// The commit-advance hook: what a commit frontier advance `(from,
    /// through]` does to the armed machine. Each step is ONE establishing
    /// batch, so a committed range covering exactly the machine's next step
    /// advances the machine, and the LAST step's commit is the completion:
    /// the machine clears. A foreign establishing operation advances
    /// nothing; the continuation re-gates the pending step against the
    /// configuration that changed underneath it.
    ///
    /// The journal shape of one establishing batch is either a single
    /// `Batch` entry (the ordinary `Prepare` path) or a maximal run of
    /// consecutive system entries, the packed schedule a fuse envelope
    /// carried, one op per consecutive slot (`docs/uvrr-fuse.md` §1). Both
    /// fold as the one batch they are, so the hook matches both: a run of
    /// two or more system entries compares as `Batch(ops)`; a singleton
    /// compares only when its payload is itself a `Batch`, exactly as
    /// before.
    pub(in crate::replica) fn plan_execution_commit(
        &self,
        journal: &J::View,
        from: Slot,
        through: Slot,
    ) -> PlanExecutionUpdate {
        let Some(machine) = &self.plan_execution else {
            return PlanExecutionUpdate::Unchanged;
        };
        let mut next = machine.next;
        let mut slot = from;
        while let Some(cursor) = slot.next() {
            if cursor > through {
                break;
            }
            slot = cursor;
            let Some(entry) = journal.get(cursor) else {
                break;
            };
            if let Payload::System(op) = &entry.payload {
                // The run's extent within the advance: the cascade is
                // segment-atomic, so an establishing batch never straddles
                // the frontier.
                let mut ops = vec![op.clone()];
                let mut end = cursor;
                let mut walk = cursor;
                while let Some(follow) = walk.next() {
                    if follow > through {
                        break;
                    }
                    match journal.get(follow) {
                        Some(entry) if matches!(entry.payload, Payload::System(_)) => {
                            if let Payload::System(follow_op) = &entry.payload {
                                ops.push(follow_op.clone());
                            }
                            end = follow;
                            walk = follow;
                        }
                        _ => break,
                    }
                }
                let batch = if ops.len() >= 2 {
                    Some(SystemOperation::Batch(ops))
                } else {
                    match op {
                        SystemOperation::Batch(ops) => Some(SystemOperation::Batch(ops.clone())),
                        _ => None,
                    }
                };
                let matches_step = match &batch {
                    Some(SystemOperation::Batch(ops)) => {
                        next < machine.steps.len() && *ops == machine.steps[next]
                    }
                    _ => false,
                };
                if matches_step {
                    next += 1;
                }
                slot = end;
            }
        }
        if next == machine.next {
            PlanExecutionUpdate::Unchanged
        } else if next >= machine.steps.len() {
            PlanExecutionUpdate::Clear
        } else {
            PlanExecutionUpdate::Set(PlannedSequence {
                steps: machine.steps.clone(),
                next,
            })
        }
    }

    /// The verdict transition: an identity candidate whose released effect
    /// is the one answer the admin perimeter renders. Nothing about the
    /// cluster changes.
    fn admin_verdict(
        &self,
        verdict: PlanVerdict,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let candidate = self.identity_candidate()?;
        Ok(self.candidate_plan(
            candidate,
            JournalMutation::None,
            vec![Effect::AdminResponse { verdict }],
            kind,
            false,
        ))
    }

    /// Whether the reconfiguration pipeline would accept `op` right now:
    /// the gates of [`plan_reconfigure`] that are cheap to precheck, so a
    /// continuation tick stays a total identity transition when the
    /// pipeline is not ready (one era transition at a time; one
    /// establishing operation at a time; the fold's own preconditions).
    ///
    /// [`plan_reconfigure`]: super::Replica::plan_reconfigure
    fn plan_step_plannable(&self, journal: &J::View, op: &SystemOperation) -> bool {
        let current = self.progress.current();
        if self.progress.config().current().era != current.era {
            return false;
        }
        let mut tail = self.progress.committed();
        while let Some(next) = tail.next() {
            if next > self.progress.accepted() {
                break;
            }
            match journal.get(next) {
                Some(entry) if matches!(entry.payload, Payload::System(_)) => return false,
                Some(_) => {}
                None => return false,
            }
            tail = next;
        }
        let Some(slot) = self.progress.accepted().next() else {
            return false;
        };
        self.progress.config().extend(op, slot).is_ok()
    }
}
