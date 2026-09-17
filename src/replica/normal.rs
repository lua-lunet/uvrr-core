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
        let mut recipients = self.backups();
        // The stream targets (`docs/uvrr-rejoin-gossip-and-witnesses.md`
        // §3): the leader's own proposals stream to the announced standby
        // and to every gossip-witness too, unless ordinary addressing
        // already covers them.
        for target in self.stream_targets() {
            if !recipients.contains(&target) {
                recipients.push(target);
            }
        }
        let effects: Vec<Effect> = recipients
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
    /// The bootstrap rule: a fenced entry (`Restarting` or `Joining`) backup
    /// receiving a legitimate
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
    /// the primary. The host obligation rides with it (`docs/architecture.md`,
    /// the contiguity gap rule): a host detects `slot > local frontier` at its
    /// boundary and treats the epoch as stalled until state transfer repairs
    /// the log; an era change is the lawful repair.
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
        // qualified evidence. The one §10 exception: a node still at its
        // boot fence that is OUTSIDE every configuration it can name —
        // the reincarnated standby (`docs/uvrr-reincarnation.md` §10) —
        // takes the committed operations the leader-originated stream
        // carries at its boot fence instead: adopting the advertised
        // view here would leave the boot fence behind, and the standby's
        // only route back through it (its own fetch) is lost with the
        // fence. The slot discipline below keeps the accept honest, the
        // piggybacked frontier is clamped as ever (§5 invariant 2), and
        // the node stays fenced: no view adopted, no vote ever counted.
        let current = self.progress.current();
        if header.view > current {
            let boot_fence = matches!(self.progress.status(), Status::Restarting | Status::Joining)
                && current == self.progress.retained();
            if !boot_fence {
                return self.plan_higher_view_signal(journal, from, header.view, at, kind);
            }
        }
        // The message's view must be the node's current view; a fenced
        // entry state (`Restarting` or `Joining`) adopts it (the bootstrap
        // rule above). Anything else is a view change or a recovery, and
        // the message drops. A differing view is a view mismatch; a
        // matching view refused by the status is the status gate, named
        // as such — the churn-window hunt read `ViewMismatch { got ==
        // current }` as an era disagreement when the refusal was the
        // receiver's status all along.
        if header.view != current {
            return self.drop_plan(
                Diagnostic::ViewMismatch {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        let boot_fenced = matches!(self.progress.status(), Status::Restarting | Status::Joining)
            && current == self.progress.retained();
        if !matches!(self.progress.status(), Status::Normal) && !boot_fenced {
            return self.drop_plan(
                Diagnostic::StatusGate {
                    got: header.view,
                    current,
                    status: self.progress.status(),
                },
                kind,
            );
        }
        // The bootstrap rule adopts a fenced entry node that the view's
        // configuration counts as a member: the fresh cluster has nothing
        // to recover (§4's own mechanism). A boot-fenced node OUTSIDE
        // every configuration it can name — the reincarnated standby
        // (§10 of `docs/uvrr-reincarnation.md`) — takes the committed
        // operations the stream carries but adopts no view and stays
        // fenced: it never votes, and the promotion era's committed
        // reconfiguration is what admits it.
        let member_of_view = self
            .progress
            .config()
            .record(header.view.era)
            .is_some_and(|record| record.config.weight_of(self.own).is_some());
        let adopt = matches!(self.progress.status(), Status::Restarting | Status::Joining)
            && member_of_view;
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
                    // The commit frontier would split an establishing
                    // batch (`docs/uvrr-fuse.md`): refused by name, the
                    // gap rule's fetch is the repair.
                    Err(CommitFold::SplitBatch) => {
                        return self.drop_plan(Diagnostic::FuseRefusal, kind);
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
            // The commit frontier would split an establishing batch
            // (`docs/uvrr-fuse.md`): refused by name, the gap rule's
            // fetch is the repair.
            Err(CommitFold::SplitBatch) => {
                return self.drop_plan(Diagnostic::FuseRefusal, kind);
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
        // The §6 membership-discard rule (`docs/uvrr-reincarnation.md`):
        // a sender outside the configuration is unknown; a sender whose
        // weight is 0 is a learner — it receives history but contributes
        // nothing to any quorum, so its vote is dropped before it is ever
        // counted.
        match record.config.weight_of(from) {
            None => return self.drop_plan(Diagnostic::UnknownSender { sender: from }, kind),
            Some(weight) if weight.0 == 0 => {
                return self.drop_plan(Diagnostic::LearnerSender { sender: from }, kind);
            }
            Some(_) => {}
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
        // range, because it vouches for the whole range. The cascade is
        // SEGMENT-ATOMIC: a maximal run of consecutive system entries is
        // the one establishing batch a fuse envelope packed
        // (`docs/uvrr-fuse.md` §1) and commits whole or not at all — an
        // establishing batch's era is established by the whole fold.
        let mut committed = self.progress.committed();
        while let Some(next) = committed.next() {
            if next > self.progress.accepted() {
                break;
            }
            let segment = self.commit_segment(journal, next, self.progress.accepted());
            let mut holds = true;
            let mut cursor = segment.0;
            while cursor <= segment.1 {
                let mut members: Vec<NodeId> = vec![self.own];
                if let Some(outstanding) = self.proposals.get(&cursor) {
                    members.extend(outstanding.oks.iter().copied());
                }
                if cursor <= slot && !members.contains(&from) {
                    members.push(from);
                }
                if !self
                    .strategy
                    .is_quorum(Role::Commit, &record.config, &members)
                {
                    holds = false;
                    break;
                }
                match cursor.next() {
                    Some(follow) if follow <= segment.1 => cursor = follow,
                    _ => break,
                }
            }
            if !holds {
                break;
            }
            committed = segment.1;
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
            // The cascade is segment-atomic, so this is unreachable;
            // stated so the match stays total (`docs/uvrr-fuse.md`).
            Err(CommitFold::SplitBatch) => {
                return self.drop_plan(Diagnostic::FuseRefusal, kind);
            }
            Err(CommitFold::Breach { .. }) => return self.breach_plan(kind),
        };
        // §8.7.7 steps 1 and 4: an armed non-stop machine whose
        // establishing operation this advance committed records its pivot
        // on the new era's record and solicits the planned evidence of
        // `qI − {L}` — the solicitation rides THIS published transition,
        // while the era-(e+1) client stream continues uninterrupted.
        let (config, solicitation, planned_update) = self.overlap_solicitation(config, committed);
        // The plan-execution commit hook (the solver doc): a committed
        // range covering the armed machine's next step's establishing
        // batch advances it, and the last step's commit clears the
        // machine (completion).
        let plan_execution =
            self.plan_execution_commit(journal, self.progress.committed(), committed);
        let candidate = self.candidate_with(
            Status::Normal,
            self.progress.accepted(),
            committed,
            self.applied_walk(journal, &[], self.progress.applied(), committed)?,
            config,
        )?;
        let mut effects = self.apply_effects(journal, self.progress.committed(), committed)?;
        effects.extend(self.broadcast_commit(committed));
        // The per-era commit emission (`docs/uvrr-fuse.md` §4): an
        // establishing batch the advance committed — the packed schedule a
        // fuse envelope carried — is announced as one `CommitBatch` naming
        // its slots' committed frontiers, no ranges.
        effects.extend(self.commit_batch_effects(journal, self.progress.committed(), committed));
        effects.extend(solicitation);
        Ok(self
            .candidate_plan(candidate, JournalMutation::None, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                planned: planned_update,
                plan_execution,
                ..bookkeeping
            }))
    }

    /// Any node's `Commit` handler (§4, §13.3): advance
    /// `committed = min(header.committed, accepted)` — the frontier never
    /// claims what the journal does not record (§5 invariant 2) — and emit
    /// `Apply` for the newly committed operation slots in slot order (§11.1).
    /// A fenced entry backup adopts the view under the same rule as
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
        // fence and fetch, never install from the hint (§13.4). The one
        // §10 exception, as in `plan_prepare`: a boot-fenced standby
        // outside every configuration it can name takes the commit
        // frontier the leader-originated stream carries at its boot fence
        // (`docs/uvrr-reincarnation.md` §10), staying fenced — the
        // frontier is clamped by the journal as ever (§5 invariant 2).
        if header.view > current {
            let boot_fence = matches!(self.progress.status(), Status::Restarting | Status::Joining)
                && current == self.progress.retained();
            if !boot_fence {
                return self.plan_higher_view_signal(journal, from, header.view, at, kind);
            }
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
        let boot_fenced = matches!(self.progress.status(), Status::Restarting | Status::Joining)
            && current == self.progress.retained();
        if !matches!(self.progress.status(), Status::Normal) && !boot_fenced {
            return self.drop_plan(
                Diagnostic::StatusGate {
                    got: header.view,
                    current,
                    status: self.progress.status(),
                },
                kind,
            );
        }
        // The same member-gated bootstrap adoption as `plan_prepare`:
        // a boot-fenced standby outside every configuration it can name
        // takes the commit frontier the stream carries and stays fenced
        // (§10 of `docs/uvrr-reincarnation.md`), never adopting a view.
        let member_of_view = self
            .progress
            .config()
            .record(header.view.era)
            .is_some_and(|record| record.config.weight_of(self.own).is_some());
        let status = if matches!(self.progress.status(), Status::Restarting | Status::Joining)
            && member_of_view
        {
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
                // The commit frontier would split an establishing batch
                // (`docs/uvrr-fuse.md`): refused by name, the ordinary
                // stream's catch-up is the repair.
                Err(CommitFold::SplitBatch) => {
                    return self.drop_plan(Diagnostic::FuseRefusal, kind);
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

    /// The commit cascade's atomic segment beginning at `next`: a maximal
    /// run of consecutive system entries — the establishing batch a fuse
    /// envelope packed (`docs/uvrr-fuse.md` §1) — commits whole, and any
    /// other slot commits alone. The run is bounded by the accepted
    /// frontier, which is where the journal's system tail ends.
    fn commit_segment(&self, journal: &J::View, next: Slot, accepted: Slot) -> (Slot, Slot) {
        let mut end = next;
        let mut cursor = next;
        while let Some(follow) = cursor.next() {
            if follow > accepted {
                break;
            }
            match journal.get(follow) {
                Some(entry) if matches!(entry.payload, Payload::System(_)) => {
                    end = follow;
                    cursor = follow;
                }
                _ => break,
            }
        }
        (next, end)
    }

    /// The per-era commit emission (`docs/uvrr-fuse.md` §4 step 4): ONE
    /// `CommitBatch` per establishing batch the advance `(from, through]`
    /// committed — a maximal run of two or more consecutive system
    /// entries, the packed schedule a fuse envelope carried. Each batch's
    /// message lists the committed frontier after each of its slots, in
    /// batch order, no ranges, and travels to every backup (and the memo
    /// standby) as a broadcast — the commit is not a round trip. The
    /// ordinary singleton establishing batch keeps the plain `Commit`
    /// announcement it has always had.
    fn commit_batch_effects(&self, journal: &J::View, from: Slot, through: Slot) -> Vec<Effect> {
        let mut effects = Vec::new();
        let mut slot = from;
        while let Some(next) = slot.next() {
            if next > through {
                break;
            }
            let Some(entry) = journal.get(next) else {
                break;
            };
            if let Payload::System(_) = &entry.payload {
                let segment = self.commit_segment(journal, next, through);
                if segment.1 > segment.0 {
                    let mut committed = Vec::new();
                    let mut cursor = Some(segment.0);
                    while let Some(slot) = cursor {
                        committed.push(slot);
                        if slot == segment.1 {
                            break;
                        }
                        cursor = slot.next();
                    }
                    let message = Message {
                        header: Header {
                            tag: Tag::CommitBatch,
                            view: self.progress.current(),
                            slot: segment.1,
                        },
                        body: Body::CommitBatch { committed },
                    };
                    let mut recipients = self.backups();
                    for target in self.stream_targets() {
                        if !recipients.contains(&target) {
                            recipients.push(target);
                        }
                    }
                    effects.extend(recipients.into_iter().map(|to| Effect::Send {
                        to,
                        era: self.progress.current().era,
                        message: message.clone(),
                    }));
                }
                slot = segment.1;
            } else {
                slot = next;
            }
        }
        effects
    }

    /// The primary's `FuseOk` handler (`docs/uvrr-fuse.md` §4 step 3,
    /// §2): one `FuseOk` is ONE atomic vote vouching for the whole
    /// envelope — a node processes the full datagram before reading any
    /// other message, so the leader counts a majority response on the
    /// FIRST message in batch and telescopes the remaining slots. The
    /// guards mirror `plan_prepare_ok` — `Normal`, own is the primary of
    /// the current view, the view matches, the sender is a member voting
    /// with weight ≥ 1, the HEADER slot an outstanding proposal slot (a
    /// delayed duplicate of a committed slot is harmless but named), the
    /// sender not already counted on that record — with the fuse
    /// vocabulary's one named outcome (`Diagnostic::FuseRefusal`). The
    /// `acks` body is the acceptor's wire evidence and is never examined
    /// for counting: the sender is vouched cumulatively onto every
    /// outstanding slot the header slot covers, the same bookkeeping
    /// `plan_prepare_ok` feeds.
    ///
    /// The commit cascade is `plan_prepare_ok`'s, segment-atomic: the
    /// packed schedule's slots share their ackers (§2), so the batch's
    /// quorum lands whole — the establishing batch commits as the ONE era
    /// it is, the commit cascade runs in slot order, and the per-era
    /// `CommitBatch` joins the ordinary commit announcement. The
    /// leader's own ack is implicit, as today.
    pub(in crate::replica) fn plan_fuse_ok(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let header = message.header;
        let current = self.progress.current();
        let is_primary =
            self.progress.status() == Status::Normal && self.primary_of(current) == Some(self.own);
        if !is_primary {
            return self.drop_plan(Diagnostic::FuseRefusal, kind);
        }
        if header.view != current {
            return self.drop_plan(Diagnostic::FuseRefusal, kind);
        }
        // The §6 membership-discard rule (`docs/uvrr-reincarnation.md`),
        // as `plan_prepare_ok` runs it: a sender outside the configuration
        // is unknown; a weight-0 sender is a learner contributing nothing.
        let record = self.progress.config().current();
        match record.config.weight_of(from) {
            None => return self.drop_plan(Diagnostic::FuseRefusal, kind),
            Some(weight) if weight.0 == 0 => {
                return self.drop_plan(Diagnostic::FuseRefusal, kind);
            }
            Some(_) => {}
        }
        // The header slot must be an outstanding proposal slot: the ONE
        // atomic vote is counted against the coverage the header names,
        // and a delayed duplicate of a committed slot is harmless but
        // named — `plan_prepare_ok`'s stale handling.
        let slot = header.slot;
        let proposal = if slot <= self.progress.committed() {
            None
        } else {
            self.proposals.get(&slot)
        };
        let Some(proposal) = proposal else {
            return self.drop_plan(Diagnostic::FuseRefusal, kind);
        };
        if proposal.oks.contains(&from) {
            return self.drop_plan(Diagnostic::FuseRefusal, kind);
        }
        // The sender is vouched cumulatively onto every outstanding slot
        // the header slot covers — `plan_prepare_ok`'s bookkeeping pairs,
        // driven by the header's coverage (§2: majority is computed on
        // the first message in batch; the remaining slots telescope).
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
        // The cascade over the contiguous accepted tail, segment-atomic:
        // the packed schedule's slots share their ackers (the envelope is
        // atomic, §2), so the batch's quorum lands whole.
        let mut committed = self.progress.committed();
        while let Some(next) = committed.next() {
            if next > self.progress.accepted() {
                break;
            }
            let segment = self.commit_segment(journal, next, self.progress.accepted());
            let mut holds = true;
            let mut cursor = segment.0;
            while cursor <= segment.1 {
                let mut members: Vec<NodeId> = vec![self.own];
                if let Some(outstanding) = self.proposals.get(&cursor) {
                    members.extend(outstanding.oks.iter().copied());
                }
                if cursor <= header.slot && !members.contains(&from) {
                    members.push(from);
                }
                if !self
                    .strategy
                    .is_quorum(Role::Commit, &record.config, &members)
                {
                    holds = false;
                    break;
                }
                match cursor.next() {
                    Some(follow) if follow <= segment.1 => cursor = follow,
                    _ => break,
                }
            }
            if !holds {
                break;
            }
            committed = segment.1;
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
        // the advance newly covers. The packed schedule folds as the ONE
        // establishing batch it is; a fold refusal is committed history
        // the configuration cannot hold — the breach faults.
        let config = match self.fold_committed(journal, &[], self.progress.committed(), committed) {
            Ok(config) => config,
            Err(CommitFold::Unavailable(slot)) => {
                return Err(PlanRefusal::JournalEntryUnavailable { slot });
            }
            Err(CommitFold::SplitBatch) => {
                return self.drop_plan(Diagnostic::FuseRefusal, kind);
            }
            Err(CommitFold::Breach { .. }) => return self.breach_plan(kind),
        };
        // §8.7.7 steps 1 and 4, as `plan_prepare_ok` runs them: an armed
        // non-stop machine whose establishing operation this advance
        // committed records its pivot and solicits the planned evidence.
        let (config, solicitation, planned_update) = self.overlap_solicitation(config, committed);
        // The plan-execution commit hook (the solver doc): a committed
        // range covering the armed machine's next step's establishing
        // batch advances it, and the last step's commit clears the
        // machine (completion).
        let plan_execution =
            self.plan_execution_commit(journal, self.progress.committed(), committed);
        let candidate = self.candidate_with(
            Status::Normal,
            self.progress.accepted(),
            committed,
            self.applied_walk(journal, &[], self.progress.applied(), committed)?,
            config,
        )?;
        let mut effects = self.apply_effects(journal, self.progress.committed(), committed)?;
        effects.extend(self.broadcast_commit(committed));
        // The per-era commit emission (`docs/uvrr-fuse.md` §4 step 4).
        effects.extend(self.commit_batch_effects(journal, self.progress.committed(), committed));
        effects.extend(solicitation);
        Ok(self
            .candidate_plan(candidate, JournalMutation::None, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                planned: planned_update,
                plan_execution,
                ..bookkeeping
            }))
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
