//! Normal operation (§4): `Prepare` / `PrepareOk` / `Commit` and proposal
//! admission.
//!
//! VRR-2012 §4 with commit-frontier piggybacking (§13.3) and the
//! Propose/Apply/Applied boundary (§11.1). Every handler is total: invalid
//! peer input is dropped with a named [`Diagnostic`] on the observation and
//! never faults the node; faulting stays reserved for impossible LOCAL
//! transitions via [`legal`]. Quorum decisions go through the
//! [`QuorumStrategy`] (`Role::Commit`) and nowhere else (Q1).
//!
//! [`legal`]: crate::invariant::legal

use super::reconfiguration::CommitFold;
use super::*;

impl<J: Journal, Q: QuorumStrategy> Replica<J, Q> {
    /// The primary's proposal handler (§4, §6): the proposal is accepted
    /// and replicated, full stop. There is no deduplication verdict (§11.1,
    /// B2): the core never inspects the operation's identity, so the same
    /// identity proposed twice is two operations at two slots.
    ///
    /// Only a `Normal` node with `config.primary(current_view) == own`
    /// accepts; every other node answers [`PlanRefusal::NotPrimary`].
    pub(in crate::replica) fn plan_propose(
        &self,
        _journal: &J::View,
        operation: &Operation,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let current = self.progress.current();
        let record = self
            .current_record()
            .ok_or(PlanRefusal::Progress(ProgressError::EraSlotDiscipline))?;
        let is_primary = self.progress.status() == Status::Normal
            && record.config.primary(current.view) == Some(self.own);
        if !is_primary {
            // Redirection (§13.4's convergence hint): the node names its
            // current view and the primary of that view, so the host can
            // point the proposer at the node this cluster would serve from.
            return Err(PlanRefusal::NotPrimary {
                view: current,
                primary: record.config.primary(current.view),
            });
        }
        let slot = self
            .progress
            .accepted()
            .next()
            .ok_or(PlanRefusal::SlotSpaceExhausted)?;
        // The stamp is the newest COMMITTED configuration's era (§8.7.3):
        // the current view's era when no reconfiguration is in flight, one
        // past it inside the overlap a committed establishing operation
        // opened — the relation's +1 sentence admits exactly that case.
        let era = self.progress.config().current().era;
        let entry = LogEntry {
            slot,
            era,
            payload: Payload::Operation {
                id: operation.id,
                payload: operation.payload.clone(),
            },
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
                era,
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
                InputKind::ClientRequest,
                false,
            )
            .with_bookkeeping(bookkeeping))
    }

    /// The backup's `Prepare` handler (§4). Guards first, each with a named
    /// outcome; then the slot relation decides: accept, idempotent
    /// re-acknowledgement, or gap.
    ///
    /// The bootstrap rule: a `Recovering` backup receiving a legitimate
    /// `Prepare` for its current view — with `current == retained`, so
    /// entering `Normal` re-selects nothing and rule 3 of the legality gate
    /// is untouched — adopts the view and enters `Normal` (§4's own
    /// mechanism; the fresh cluster has nothing to recover). The
    /// piggybacked committed frontier is taken on every accepted or
    /// re-acknowledged `Prepare` (§13.3).
    ///
    /// The gap rule (§13.1 step 5): a `Prepare` past the accepted
    /// frontier's successor is dropped and reported as
    /// [`Diagnostic::GapDetected`], and the fetch half of the ruling rides
    /// the same transition — a `GetState` for the missing range goes to
    /// the primary.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::replica) fn plan_prepare(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        entry: &LogEntry,
        piggybacked: Slot,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let header = message.header;
        // The era must be evaluable: outside the retention window the
        // configuration that would judge the message is gone.
        if self.progress.config().record(header.view.era).is_none() {
            return self.drop_plan(
                Diagnostic::UnevaluableEra {
                    era: header.view.era,
                },
                kind,
            );
        }
        // The sender must be the primary of the message's view under that
        // view's era configuration (§1.2).
        if self.primary_of(header.view) != Some(from) {
            return self.drop_plan(
                Diagnostic::SenderNotPrimary {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        }
        // A Prepare from the legitimate primary of a HIGHER view is proof
        // the node is stale (§10) — never installation evidence (§13.4):
        // fence into the advertised view and fetch, install only from the
        // qualified evidence.
        let current = self.progress.current();
        if header.view > current {
            return self.plan_higher_view_signal(journal, from, header.view, at, kind);
        }
        // The message's view must be the node's current view; a `Recovering`
        // node adopts it (the bootstrap rule above). Anything else is a
        // view change or a recovery, and the message drops.
        let eligible = header.view == current
            && match self.progress.status() {
                Status::Normal => true,
                Status::Recovering => current == self.progress.retained(),
                Status::ViewChange | Status::Replaying => false,
            };
        if !eligible {
            return self.drop_plan(
                Diagnostic::ViewMismatch {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        let adopt = self.progress.status() == Status::Recovering;
        // Era discipline (§8.7.3): era(view) <= era(entry) <= era(view) + 1.
        let era_legal = entry.era == header.view.era || header.view.era.next() == Some(entry.era);
        if !era_legal {
            return self.drop_plan(
                Diagnostic::EraDiscipline {
                    entry: entry.era,
                    view: header.view,
                },
                kind,
            );
        }
        if header.slot != entry.slot {
            return self.drop_plan(
                Diagnostic::PrepareSlotMismatch {
                    header: header.slot,
                    entry: entry.slot,
                },
                kind,
            );
        }
        let status = if adopt {
            Status::Normal
        } else {
            self.progress.status()
        };
        let accepted = self.progress.accepted();
        if entry.slot <= accepted {
            // Idempotent retransmission: never re-append. The held entry
            // must BE the proposed one — a slot is assigned once (§1.3).
            let Some(held) = journal.get(entry.slot) else {
                return Err(PlanRefusal::JournalEntryUnavailable { slot: entry.slot });
            };
            if held != entry {
                return self.drop_plan(Diagnostic::ConflictingEntry { slot: entry.slot }, kind);
            }
            // Re-acknowledge, and take the piggybacked frontier (§13.3).
            let new_committed = self.progress.committed().max(piggybacked.min(accepted));
            // §8.7.1: the era table folds the system operations the
            // advance newly covers. Every entry in the range is journaled
            // here, so a fold refusal is committed history the
            // configuration cannot hold — the breach faults.
            let config =
                match self.fold_committed(journal, &[], self.progress.committed(), new_committed) {
                    Ok(config) => config,
                    Err(CommitFold::Unavailable(slot)) => {
                        return Err(PlanRefusal::JournalEntryUnavailable { slot });
                    }
                    Err(CommitFold::Breach { .. }) => return self.breach_plan(kind),
                };
            let candidate = self.candidate_with(
                status,
                accepted,
                new_committed,
                self.applied_walk(journal, &[], self.progress.applied(), new_committed)?,
                config,
            )?;
            let mut effects = vec![prepare_ok(current, from, entry.slot)];
            effects.extend(self.apply_effects(
                journal,
                self.progress.committed(),
                new_committed,
            )?);
            return Ok(self.candidate_plan(candidate, JournalMutation::None, effects, kind, false));
        }
        let Some(next) = accepted.next() else {
            // `entry.slot > accepted == u64::MAX` cannot be offered; the
            // slot space is spent.
            return Err(PlanRefusal::SlotSpaceExhausted);
        };
        if entry.slot != next {
            let plan = self.drop_plan(
                Diagnostic::GapDetected {
                    expected: next,
                    got: entry.slot,
                },
                kind,
            )?;
            // §13.1 step 5: the fetch half of the gap ruling — ask the
            // primary for the missing range.
            let (effect, fetch) = self.fetch(current, from, next);
            return Ok(plan.with_fetch(effect, fetch));
        }
        // Accept, and take the piggybacked frontier (§13.3).
        let new_committed = self.progress.committed().max(piggybacked.min(entry.slot));
        // §8.7.1: the era table folds exactly what the commit frontier
        // newly covers; the arriving entry is visible to the fold through
        // the overlay. A fold refusal AT the arriving slot names the
        // peer's entry — the operation is invalid against the committed
        // prefix, so the entry drops and nothing installs; below it, the
        // refusal names committed history the configuration cannot hold —
        // the breach faults.
        let overlay = [entry.clone()];
        let config = match self.fold_committed(
            journal,
            &overlay,
            self.progress.committed(),
            new_committed,
        ) {
            Ok(config) => config,
            Err(CommitFold::Unavailable(slot)) => {
                return Err(PlanRefusal::JournalEntryUnavailable { slot });
            }
            Err(CommitFold::Breach { slot, error }) if slot == entry.slot => {
                return self.drop_plan(
                    Diagnostic::InvalidSystemOperation {
                        slot: entry.slot,
                        error,
                    },
                    kind,
                );
            }
            Err(CommitFold::Breach { .. }) => return self.breach_plan(kind),
        };
        if new_committed < entry.slot {
            // The system-operation perimeter (§8.7.2): an arriving system
            // entry the piggyback did not commit must fold onto the
            // post-piggyback table BEFORE it may be accepted — a peer's
            // invalid operation is dropped by name, never journaled.
            if let Payload::System(op) = &entry.payload {
                if let Err(error) = config.extend(op, entry.slot) {
                    return self.drop_plan(
                        Diagnostic::InvalidSystemOperation {
                            slot: entry.slot,
                            error,
                        },
                        kind,
                    );
                }
            }
            // Era authorization (§8.7.3, §8.7.8): the entry's era must
            // name an era the committed history has established — the
            // relation above admitted the +1 window; only the fold can
            // say whether a committed operation actually opened it.
            if config.current().era < entry.era {
                return self.drop_plan(
                    Diagnostic::EraDiscipline {
                        entry: entry.era,
                        view: header.view,
                    },
                    kind,
                );
            }
        }
        let candidate = self.candidate_with(
            status,
            entry.slot,
            new_committed,
            self.applied_walk(journal, &overlay, self.progress.applied(), new_committed)?,
            config,
        )?;
        let mut effects = vec![prepare_ok(current, from, entry.slot)];
        effects.extend(self.apply_effects(journal, self.progress.committed(), new_committed)?);
        Ok(self.candidate_plan(
            candidate,
            JournalMutation::Accept(vec![entry.clone()]),
            effects,
            kind,
            false,
        ))
    }

    /// The primary's `PrepareOk` handler (§4). Guards: `Normal`, own is the
    /// primary of the current view, the view matches, the sender is a
    /// member of the current configuration, the slot is outstanding, the
    /// sender is not already counted. Then the vote is recorded and the
    /// STRATEGY — the only quorum authority (Q1) — is asked; on a quorum
    /// the committed frontier advances over the contiguous accepted tail,
    /// the newly committed operation slots emit `Apply` in slot order
    /// (§11.1), and the new frontier is announced to every backup (§13.3).
    pub(in crate::replica) fn plan_prepare_ok(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let header = message.header;
        let slot = header.slot;
        let current = self.progress.current();
        let is_primary =
            self.progress.status() == Status::Normal && self.primary_of(current) == Some(self.own);
        if !is_primary {
            return self.drop_plan(Diagnostic::PrepareOkNotPrimary { sender: from, slot }, kind);
        }
        if header.view != current {
            return self.drop_plan(
                Diagnostic::ViewMismatch {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        // Commit votes are counted under the NEWEST COMMITTED
        // configuration (§8.7.3's overlap sentence): slots stamped by the
        // era a committed establishing operation opened are authorized by
        // that era's QII, and R2 — gated before the operation was ever
        // proposed (§8.7.4) — is what makes the pair safe. Membership and
        // the strategy's decision both come from that record (Q1).
        let record = self.progress.config().current();
        if record.config.weight_of(from).is_none() {
            return self.drop_plan(Diagnostic::UnknownSender { sender: from }, kind);
        }
        let proposal = if slot <= self.progress.committed() {
            None
        } else {
            self.proposals.get(&slot)
        };
        let Some(proposal) = proposal else {
            // A delayed duplicate of a committed slot, or foreign: harmless.
            return self.drop_plan(Diagnostic::SlotNotOutstanding { slot, sender: from }, kind);
        };
        if proposal.oks.contains(&from) {
            return self.drop_plan(Diagnostic::DuplicatePrepareOk { slot, sender: from }, kind);
        }
        // Acceptance is prefix-contiguous, so this acknowledgement vouches
        // for every lower uncommitted slot (VRR-2012 §4's cumulative
        // acknowledgement): the vote is recorded against every outstanding
        // slot up to the acknowledged one — including slots a view change
        // installed, whose records the new primary re-seeded.
        let mut oks: Vec<(Slot, NodeId)> = Vec::new();
        let mut covered = self.progress.committed();
        while let Some(next) = covered.next() {
            if next > slot {
                break;
            }
            if self.proposals.contains_key(&next) {
                oks.push((next, from));
            }
            covered = next;
        }
        // Then the cascade over the contiguous accepted tail: a slot
        // commits when its Commit quorum lands (the strategy decides —
        // Q1), and commits pull every earlier quorum-holding slot with
        // them (§4). The in-flight vote counts for the whole covered
        // range, because it vouches for the whole range.
        let mut committed = self.progress.committed();
        while let Some(next) = committed.next() {
            if next > self.progress.accepted() {
                break;
            }
            let mut members: Vec<NodeId> = vec![self.own];
            if let Some(outstanding) = self.proposals.get(&next) {
                members.extend(outstanding.oks.iter().copied());
            }
            if next <= slot && !members.contains(&from) {
                members.push(from);
            }
            if !self
                .strategy
                .is_quorum(Role::Commit, &record.config, &members)
            {
                break;
            }
            committed = next;
        }
        let bookkeeping = Bookkeeping {
            oks,
            ..Bookkeeping::default()
        };
        if committed == self.progress.committed() {
            let candidate = self.identity_candidate()?;
            return Ok(self
                .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
                .with_bookkeeping(bookkeeping));
        }
        // §8.7.1: the commit frontier moved — fold the system operations
        // the advance newly covers. Every entry in the range is
        // journaled (the cascade walks the accepted tail), so a fold
        // refusal is committed history the configuration cannot hold —
        // the breach faults.
        let config = match self.fold_committed(journal, &[], self.progress.committed(), committed) {
            Ok(config) => config,
            Err(CommitFold::Unavailable(slot)) => {
                return Err(PlanRefusal::JournalEntryUnavailable { slot });
            }
            Err(CommitFold::Breach { .. }) => return self.breach_plan(kind),
        };
        // §8.7.7 steps 1 and 4: an armed non-stop machine whose
        // establishing operation this advance committed records its pivot
        // on the new era's record and solicits the planned evidence of
        // `qI − {L}` — the solicitation rides THIS published transition,
        // while the era-(e+1) client stream continues uninterrupted.
        let (config, solicitation, planned_update) = self.overlap_solicitation(config, committed);
        let candidate = self.candidate_with(
            Status::Normal,
            self.progress.accepted(),
            committed,
            self.applied_walk(journal, &[], self.progress.applied(), committed)?,
            config,
        )?;
        let mut effects = self.apply_effects(journal, self.progress.committed(), committed)?;
        effects.extend(self.broadcast_commit(committed));
        effects.extend(solicitation);
        Ok(self
            .candidate_plan(candidate, JournalMutation::None, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                planned: planned_update,
                ..bookkeeping
            }))
    }

    /// Any node's `Commit` handler (§4, §13.3): advance
    /// `committed = min(header.committed, accepted)` — the frontier never
    /// claims what the journal does not record (§5 invariant 2) — and emit
    /// `Apply` for the newly committed operation slots in slot order (§11.1).
    /// A `Recovering` backup adopts the view under the same rule as
    /// `Prepare`.
    pub(in crate::replica) fn plan_commit(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        frontier: Slot,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let header = message.header;
        if self.progress.config().record(header.view.era).is_none() {
            return self.drop_plan(
                Diagnostic::UnevaluableEra {
                    era: header.view.era,
                },
                kind,
            );
        }
        if self.primary_of(header.view) != Some(from) {
            return self.drop_plan(
                Diagnostic::SenderNotPrimary {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        }
        let current = self.progress.current();
        // A Commit from the legitimate primary of a HIGHER view: the same
        // qualified staleness signal as a higher-view Prepare (§10) —
        // fence and fetch, never install from the hint (§13.4).
        if header.view > current {
            return self.plan_higher_view_signal(journal, from, header.view, at, kind);
        }
        let eligible = header.view == current
            && match self.progress.status() {
                Status::Normal => true,
                Status::Recovering => current == self.progress.retained(),
                Status::ViewChange | Status::Replaying => false,
            };
        if !eligible {
            return self.drop_plan(
                Diagnostic::ViewMismatch {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        let status = if self.progress.status() == Status::Recovering {
            Status::Normal
        } else {
            self.progress.status()
        };
        let new_committed = self
            .progress
            .committed()
            .max(frontier.min(self.progress.accepted()));
        if new_committed == self.progress.committed() && status == self.progress.status() {
            let candidate = self.identity_candidate()?;
            return Ok(self.candidate_plan(
                candidate,
                JournalMutation::None,
                Vec::new(),
                kind,
                false,
            ));
        }
        // §8.7.1: the commit frontier moved — fold the system operations
        // the advance newly covers. Every entry in the range is
        // journaled (the frontier never claims what the journal does not
        // record), so a fold refusal is committed history the
        // configuration cannot hold — the breach faults.
        let config =
            match self.fold_committed(journal, &[], self.progress.committed(), new_committed) {
                Ok(config) => config,
                Err(CommitFold::Unavailable(slot)) => {
                    return Err(PlanRefusal::JournalEntryUnavailable { slot });
                }
                Err(CommitFold::Breach { .. }) => return self.breach_plan(kind),
            };
        let candidate = self.candidate_with(
            status,
            self.progress.accepted(),
            new_committed,
            self.applied_walk(journal, &[], self.progress.applied(), new_committed)?,
            config,
        )?;
        let effects = self.apply_effects(journal, self.progress.committed(), new_committed)?;
        Ok(self.candidate_plan(candidate, JournalMutation::None, effects, kind, false))
    }
}

/// The `PrepareOk` a backup answers a `Prepare` with (§4): the view is the
/// view it accepted under, the slot the slot it accepted (W1: the header
/// slot names what the message speaks about).
fn prepare_ok(view: ViewId, to: NodeId, slot: Slot) -> Effect {
    Effect::Send {
        to,
        era: view.era,
        message: Message {
            header: Header {
                tag: Tag::PrepareOk,
                view,
                slot,
            },
            body: Body::PrepareOk {},
        },
    }
}
