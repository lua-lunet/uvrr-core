//! Reconfiguration: the stop-the-world path (§8.7.4).
//!
//! A system operation is proposed through the ORDINARY pipeline like any
//! entry — one establishing operation at a time — and the era advances
//! exactly when the operation COMMITS (§8.7.1: an era is established by
//! the commit of its establishing operation, never by its acceptance).
//! The pivot is `None` on this path; a `Some` pivot names the
//! non-stop-the-world variant (§8.7.6–§8.7.7), which is future work and
//! is refused with [`PlanRejection::Unsupported`].
//!
//! The load-bearing rulings:
//!
//! * **Genesis is ordinary committed log history** (§8.7.1–§8.7.2): VOID
//!   at slot 1 in era 0, INIT at slot 2 in era 1, committed by
//!   construction at provision. The initial configuration is DERIVED from
//!   the entries — the same fold that runs at every later commit.
//! * **The pre-proposal gate is closed** (§8.7.2, §8.7.4, Q1): the fold's
//!   preconditions, then R2 across the era boundary, then R1 /
//!   self-intersection / fence-recovery within the resulting era. A
//!   refusal is named, carries its witness, and the operation never
//!   enters the log.
//! * **The commit-time fold is the only place the era advances**
//!   (§8.7.1): every path that moves the commit frontier folds the system
//!   operations the advance newly covers — [`Replica::fold_committed`]
//!   — and carries the folded table in the candidate. A fold refusal in
//!   COMMITTED history is a breach and faults; a refusal at the arriving
//!   entry's own slot is the peer's invalid operation and is dropped by
//!   name ([`Diagnostic::InvalidSystemOperation`]).
//! * **A primary crash mid-reconfiguration never half-installs an era**:
//!   an accepted-but-uncommitted system operation establishes nothing; if
//!   the view change carries it, the next commit cascade re-commits it
//!   and the era advances exactly there; if not, the operation is absent
//!   from history and the era never moved.

use std::sync::Arc;

use crate::configuration::{ConfigError, EraTable};
use crate::effects::Effect;
use crate::ids::Slot;
use crate::journal::{JournalView, LogEntry, Payload};
use crate::message::{Body, Message};
use crate::progress::Status;
use crate::quorum::{validate_era, validate_transition};
use crate::wire::{Header, Tag};

use super::{
    Bookkeeping, InputKind, Journal, JournalMutation, Pivot, PlanRejection, PlannedTransition,
    ProgressError, Proposal, QuorumStrategy, Replica, SystemOperation,
};

/// Why the fold of the committed prefix refused.
pub(in crate::replica) enum CommitFold {
    /// A slot the published record says is accepted is absent from the
    /// journal (§5 invariant 1) — surfaced exactly like the applied
    /// walk's same finding.
    Unavailable(Slot),
    /// A committed system operation the §8.7.2 preconditions refuse: the
    /// journal and the configuration history disagree about a COMMITTED
    /// slot — the same class of breach as a committed-slot conflict
    /// (§9.1), so the transition declares it, never guesses.
    Breach {
        /// The slot whose operation would not fold.
        slot: Slot,
        /// The fold's refusal.
        error: ConfigError,
    },
}

impl<J: Journal, Q: QuorumStrategy> Replica<J, Q> {
    /// The era table after folding the system operations the
    /// commit-frontier advance `(from, through]` newly covers (§8.7.1).
    /// `overlay` supplies the entries an in-transition install adds to the
    /// picture, exactly as in [`Replica::applied_walk`]. Returns the
    /// receiver's own table when the advance covers no system operation —
    /// the fold is the identity, no allocation.
    pub(in crate::replica) fn fold_committed(
        &self,
        journal: &J::View,
        overlay: &[LogEntry],
        from: Slot,
        through: Slot,
    ) -> Result<Arc<EraTable>, CommitFold> {
        let mut table = Arc::clone(self.progress.config());
        let mut slot = from;
        while let Some(next) = slot.next() {
            if next > through {
                break;
            }
            let entry = match overlay.iter().find(|entry| entry.slot == next) {
                Some(entry) => entry,
                None => journal.get(next).ok_or(CommitFold::Unavailable(next))?,
            };
            if let Payload::System(op) = &entry.payload {
                table = Arc::new(
                    table
                        .extend(op, next)
                        .map_err(|error| CommitFold::Breach { slot: next, error })?,
                );
            }
            slot = next;
        }
        Ok(table)
    }

    /// The stop-the-world reconfiguration (§8.7.4): the `Normal` primary
    /// proposes the system operation through the ordinary pipeline. The
    /// gates, in order:
    ///
    /// 1. the pivot is `None` — `Some` names §8.7.6's future path
    ///    ([`PlanRejection::Unsupported`]);
    /// 2. the node is the `Normal` primary of its current view
    ///    ([`PlanRejection::NotPrimary`], as for any proposal);
    /// 3. the era table has NOT advanced past the current view — the
    ///    committed operation establishing the next era awaits the
    ///    ordinary view change into it (§8.7.8), and a second advance
    ///    would put the accepted frontier outside §8.7.3's relation
    ///    ([`PlanRejection::EraTransitionOutstanding`]);
    /// 4. no earlier system operation sits accepted-but-uncommitted —
    ///    the fold that runs at commit must be the fold the pre-proposal
    ///    gate validated ([`PlanRejection::ReconfigureOutstanding`]);
    /// 5. the §8.7.2 preconditions of the fold itself
    ///    ([`PlanRejection::Reconfigure`]);
    /// 6. the closed intersection obligations (Q1): R2 across the
    ///    boundary FIRST — so a cross-era refusal names the cross-era
    ///    witness — then R1, self-intersection and fence-recovery within
    ///    the resulting era ([`PlanRejection::ReconfigureQuorum`]).
    ///
    /// A refusal at any gate never enters the log.
    pub(in crate::replica) fn plan_reconfigure(
        &self,
        journal: &J::View,
        op: &SystemOperation,
        pivot: &Option<Pivot>,
    ) -> Result<PlannedTransition, PlanRejection> {
        // Gate 1: the non-stop-the-world pivot is a later milestone's
        // path (§8.7.6–§8.7.7); named and total, never silently dropped.
        if pivot.is_some() {
            return Err(PlanRejection::Unsupported {
                input: InputKind::Reconfiguration,
            });
        }
        let current = self.progress.current();
        let record = self
            .current_record()
            .ok_or(PlanRejection::Progress(ProgressError::EraSlotDiscipline))?;
        // Gate 2: the proposer is the view's primary — the same ruling as
        // an ordinary proposal's.
        let is_primary = self.progress.status() == Status::Normal
            && record.config.primary(current.view) == Some(self.own);
        if !is_primary {
            return Err(PlanRejection::NotPrimary {
                view: current,
                primary: record.config.primary(current.view),
            });
        }
        // Gate 3: one era transition in flight at a time. The table's
        // newest era is at most one past the current view's (the gate
        // itself is what keeps the relation closed), so equality is the
        // only state in which a new operation may be gated.
        let established = self.progress.config().current().era;
        if established != current.era {
            return Err(PlanRejection::EraTransitionOutstanding {
                view: current.era,
                established,
            });
        }
        // Gate 4: one establishing operation in flight at a time.
        let mut tail = self.progress.committed();
        while let Some(next) = tail.next() {
            if next > self.progress.accepted() {
                break;
            }
            let entry = journal
                .get(next)
                .ok_or(PlanRejection::JournalEntryUnavailable { slot: next })?;
            if matches!(entry.payload, Payload::System(_)) {
                return Err(PlanRejection::ReconfigureOutstanding { slot: next });
            }
            tail = next;
        }
        let slot = self
            .progress
            .accepted()
            .next()
            .ok_or(PlanRejection::SlotSpaceExhausted)?;
        // Gate 5: the fold's preconditions (§8.7.2) — the operation is
        // tried against the current configuration at the slot it would
        // occupy.
        let next_table = self
            .progress
            .config()
            .extend(op, slot)
            .map_err(PlanRejection::Reconfigure)?;
        // Gate 6: the closed intersection obligations (§8.7.4, Q1). R2
        // across the boundary runs first so a cross-era refusal names the
        // cross-era witness; the within-era obligations follow.
        validate_transition(&self.strategy, &record.config, &next_table.current().config)
            .map_err(PlanRejection::ReconfigureQuorum)?;
        validate_era(&self.strategy, &next_table.current().config)
            .map_err(PlanRejection::ReconfigureQuorum)?;
        // The proposal: an ordinary entry, stamped with the era that
        // authorizes its slot (§8.7.3) — the table has not advanced yet
        // (gate 3), so the stamp is the current view's era.
        let entry = LogEntry {
            slot,
            era: current.era,
            payload: Payload::System(op.clone()),
        };
        let candidate = self.candidate_with(
            self.progress.status(),
            slot,
            self.progress.committed(),
            self.progress.applied(),
            Arc::clone(self.progress.config()),
        )?;
        let prepare = Message {
            header: Header {
                tag: Tag::Prepare,
                view: current,
                slot,
            },
            body: Body::Prepare {
                entry: entry.clone(),
                committed: self.progress.committed(),
            },
        };
        let effects = self
            .backups()
            .into_iter()
            .map(|to| Effect::Send {
                to,
                era: current.era,
                message: prepare.clone(),
            })
            .collect();
        let bookkeeping = Bookkeeping {
            proposals: vec![(slot, Proposal { oks: Vec::new() })],
            ..Bookkeeping::default()
        };
        Ok(self
            .candidate_plan(
                candidate,
                JournalMutation::Accept(vec![entry]),
                effects,
                InputKind::Reconfiguration,
                false,
            )
            .with_bookkeeping(bookkeeping))
    }
}
