//! The view change (§9): `StartViewChange` / `DoViewChange` / `StartView`,
//! the new primary's win, and the host-forced change.
//!
//! VRR-2012 §5: tick-driven timeout detection (S4), the `StartViewChange`
//! fence (`Role::Fence` through the strategy, Q1), `DoViewChange` evidence
//! (`Role::ViewChange`), and `StartView` installation. History selection
//! ranks by `retained` view first, then `accepted` frontier (§1.3) — the
//! §9.2 counterexample is the load-bearing test of the rule. Suffixes are
//! bounded newest-first under [`ViewChangeKnobs::view_change_budget`] and
//! encoded ascending (§13.1; W4). A `StartView` suffix that conflicts with a
//! committed local slot is this path's one deliberate fault-on-peer-input:
//! silent repair would hide a safety breach, so the node declares
//! [`Fault::IllegalTransition`]. A suffix the recipient cannot construct
//! history from is a named gap — [`Diagnostic::GapDetected`], never a fault
//! — whose fetch half (§10, §13.1 step 5) rides the same transition.

use super::reconfiguration::CommitFold;
use super::*;

impl<J: Journal, Q: QuorumStrategy> Replica<J, Q> {
    /// Enters the view change for `target` (VRR-2012 §5, spec §9.1): the
    /// durable view advances and fences (`Progress` keeps `retained` — the
    /// history is not re-selected by entering), the fence vote set starts
    /// at the node's own plus any already-heard `StartViewChange` senders,
    /// and the node's own `StartViewChange` broadcasts to every other
    /// member. The fence quorum (`Role::Fence`, Q1) may complete at entry —
    /// the joining `StartViewChange` can be the one that closes it.
    pub(in crate::replica) fn enter_view_change(
        &self,
        journal: &J::View,
        target: ViewId,
        heard: BTreeSet<NodeId>,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let candidate = self
            .progress
            .with_view_change(target)
            .map_err(PlanRefusal::Progress)?;
        let mut fences = heard;
        fences.insert(self.own);
        let message = Message {
            header: Header {
                tag: Tag::StartViewChange,
                view: target,
                slot: Slot::NONE,
            },
            body: Body::StartViewChange {},
        };
        let effects = self
            .backups()
            .into_iter()
            .map(|to| Effect::Send {
                to,
                era: target.era,
                message: message.clone(),
            })
            .collect();
        let view_change = ViewChangeVolatile {
            target,
            fences,
            evidence: BTreeMap::new(),
            selected: None,
        };
        self.continue_view_change(journal, candidate, view_change, effects, at, kind)
    }

    /// The host-forced view change (§14.2): drive the ORDINARY
    /// fence/evidence/install pipeline into `target`, whose primary is
    /// the member `primary(target)` names under the current membership
    /// order — no state is installed from the host's say-so. The target
    /// must strictly advance the view within the current era or the
    /// established-but-unentered era: a non-advancing target is bad
    /// input, an era the committed configuration history has not
    /// established names a membership order the replica cannot map the
    /// target under (its establishing operation was never committed
    /// here, or the era is superseded), and the last representable view
    /// has no successor (§8.7.3 forbids wraparound, so a fence there
    /// could never be superseded).
    pub(in crate::replica) fn plan_admin_force_view(
        &self,
        journal: &J::View,
        target: ViewId,
        at: Tick,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let current = self.progress.current();
        if target.view <= current.view {
            return Err(PlanRefusal::AdminTargetNotAhead { current, target });
        }
        // The target era must be one the committed configuration history
        // has established: either the current view's own era, or the
        // established-but-unentered era the table has already folded to
        // (§8.7.1). An era beyond that was never decided here.
        let established = self.progress.config().current().era;
        if target.era != current.era && target.era != established {
            return Err(PlanRefusal::AdminEraNotCurrent {
                current: current.era,
                got: target.era,
            });
        }
        if target.next_in_era().is_none() {
            return Err(PlanRefusal::AdminViewExhausted { target });
        }
        self.enter_view_change(journal, target, BTreeSet::new(), at, InputKind::Admin)
    }

    /// Runs the attempt forward after its volatile state changed: fence
    /// quorum first (§9.1's ordering — evidence follows the fence), then,
    /// at the designated new primary, the evidence quorum (`Role::View
    /// Change`, Q1) and the install. Every quorum question goes to the
    /// strategy; no count is computed here.
    pub(in crate::replica) fn continue_view_change(
        &self,
        journal: &J::View,
        candidate: Progress,
        mut view_change: ViewChangeVolatile,
        mut effects: Vec<Effect>,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let target = view_change.target;
        let Some(record) = self.progress.config().record(target.era) else {
            return self.drop_plan(Diagnostic::UnevaluableEra { era: target.era }, kind);
        };
        // The fence: once a `Role::Fence` quorum holds, the node records
        // its own evidence and reports it to the designated new primary
        // (§9.1). A node that IS the new primary keeps its evidence local.
        if let std::collections::btree_map::Entry::Vacant(slot) =
            view_change.evidence.entry(self.own)
        {
            let fences: Vec<NodeId> = view_change.fences.iter().copied().collect();
            if self
                .strategy
                .is_quorum(Role::Fence, &record.config, &fences)
            {
                let own = self.own_evidence(journal);
                slot.insert(own);
                if self.primary_of(target) != Some(self.own) {
                    effects.push(self.do_view_change_effect(journal, &view_change, target)?);
                }
            }
        }
        // The evidence quorum, at the designated new primary only.
        if self.primary_of(target) == Some(self.own) && view_change.evidence.contains_key(&self.own)
        {
            if view_change.selected.is_none() {
                let reporters: Vec<NodeId> = view_change.evidence.keys().copied().collect();
                if self
                    .strategy
                    .is_quorum(Role::ViewChange, &record.config, &reporters)
                {
                    view_change.selected = Some(select_history(&view_change.evidence));
                }
            }
            if let Some((reporter, selected)) = view_change.selected.clone() {
                match self.plan_win_view(
                    journal,
                    target,
                    &selected,
                    &view_change.evidence,
                    effects,
                    at,
                    kind,
                )? {
                    WinOutcome::Installed(plan) => return Ok(*plan),
                    // §13.1 step 5: the selected history cannot be
                    // constructed from the collected evidence — fetch the
                    // missing range from the reporter whose history was
                    // selected, stamped with the TARGET view from the gap
                    // base. The attempt and its selection are kept; the
                    // drop is named, never a fault, and an ordinary tick
                    // re-runs the win once the range has arrived.
                    WinOutcome::Insufficient {
                        expected,
                        got,
                        effects,
                    } => {
                        let plan = self
                            .candidate_plan(candidate, JournalMutation::None, effects, kind, false)
                            .with_bookkeeping(Bookkeeping {
                                view_change: ViewChangeUpdate::Set(view_change),
                                // The ordinary attempt owns the era now:
                                // a planned overlap machine is abandoned
                                // by any fence it joins.
                                planned: PlannedOverlapUpdate::Clear,
                                ..Bookkeeping::default()
                            })
                            .with_diagnostic(Diagnostic::GapDetected { expected, got });
                        // Open the fetch once: the gap ruling's first
                        // insufficient outcome asks the selected reporter
                        // for the missing range. A later insufficient
                        // outcome — a duplicate evidence delivery, or the
                        // tick re-drive before the range has arrived —
                        // leaves the open fetch alone: its chunks and the
                        // tick's cursor retry own the repair.
                        if self.transfer.is_none() {
                            let (effect, fetch) = self.fetch(target, reporter, expected);
                            return Ok(plan.with_fetch(effect, fetch));
                        }
                        return Ok(plan);
                    }
                }
            }
        }
        Ok(self
            .candidate_plan(candidate, JournalMutation::None, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                view_change: ViewChangeUpdate::Set(view_change),
                // The ordinary attempt owns the era now: a planned
                // overlap machine is abandoned by any fence it joins.
                planned: PlannedOverlapUpdate::Clear,
                ..Bookkeeping::default()
            }))
    }

    /// The node's own evidence for the attempt at `target` (§9.1): the
    /// retained provenance, both frontiers, the bounded suffix (§13.1),
    /// ordinary kind — and the era proof, attached only when the evidence
    /// is sent (§8.7.8).
    fn own_evidence(&self, journal: &J::View) -> Evidence {
        Evidence {
            retained: self.progress.retained(),
            accepted: self.progress.accepted(),
            committed: self.progress.committed(),
            suffix: self.bounded_suffix(journal, self.progress.accepted(), &[]),
        }
    }

    /// The `DoViewChange` datagram carrying the node's own evidence to the
    /// designated new primary (§9.1): the header slot is the accepted
    /// frontier of the reported history (rule 7's Frontier role).
    fn do_view_change_effect(
        &self,
        journal: &J::View,
        view_change: &ViewChangeVolatile,
        target: ViewId,
    ) -> Result<Effect, PlanRefusal> {
        let own = view_change
            .evidence
            .get(&self.own)
            .expect("own evidence is recorded before it is sent");
        let to = self
            .primary_of(target)
            .ok_or(PlanRefusal::Progress(ProgressError::EraSlotDiscipline))?;
        Ok(Effect::Send {
            to,
            era: target.era,
            message: Message {
                header: Header {
                    tag: Tag::DoViewChange,
                    view: target,
                    slot: own.accepted,
                },
                body: Body::DoViewChange {
                    retained: own.retained,
                    accepted: own.accepted,
                    committed: own.committed,
                    suffix: own.suffix.clone(),
                    evidence: EvidenceKind::Ordinary,
                    era_proof: self.era_proof(journal, target.era)?,
                },
            },
        })
    }

    /// A `StartViewChange` (§9.1): a fence vote for the view it names.
    ///
    /// - Behind or at the fence target: [`Diagnostic::StaleViewChange`] —
    ///   the change it fences is done or superseded, and a fence vote never
    ///   counts twice (V_g ⌢ V_g, §8.3).
    /// - Ahead: the node joins — the durable view advances (fencing every
    ///   earlier view), the vote set starts at `{own, sender}`, and the
    ///   node's own `StartViewChange` re-broadcasts. An in-progress attempt
    ///   at a lower target is superseded whole.
    /// - At the target of the in-progress attempt: a vote. When the votes
    ///   form a `Role::Fence` quorum (Q1 — the strategy answers, no count
    ///   is computed here), the node records its own evidence and reports
    ///   it to the designated new primary.
    pub(in crate::replica) fn plan_start_view_change(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let header = message.header;
        let Some(record) = self.progress.config().record(header.view.era) else {
            return self.drop_plan(
                Diagnostic::UnevaluableEra {
                    era: header.view.era,
                },
                kind,
            );
        };
        if record.config.weight_of(from).is_none() {
            return self.drop_plan(Diagnostic::UnknownSender { sender: from }, kind);
        }
        let target = self
            .view_change
            .as_ref()
            .map(|view_change| view_change.target)
            .unwrap_or_else(|| self.progress.current());
        if header.view < target || (header.view == target && self.view_change.is_none()) {
            return self.drop_plan(
                Diagnostic::StaleViewChange {
                    got: header.view,
                    current: target,
                },
                kind,
            );
        }
        if header.view > target {
            let mut heard = BTreeSet::new();
            heard.insert(from);
            return self.enter_view_change(journal, header.view, heard, at, kind);
        }
        // A vote for the in-progress attempt.
        let mut view_change = self
            .view_change
            .clone()
            .expect("the target came from the attempt");
        view_change.fences.insert(from);
        let candidate = self.identity_candidate()?;
        self.continue_view_change(journal, candidate, view_change, Vec::new(), at, kind)
    }

    /// A `DoViewChange` (§9.1): state evidence for the designated new
    /// primary. Guards are total and named; collection happens only at the
    /// primary of the in-progress attempt's target. The evidence quorum is
    /// the strategy's `Role::ViewChange` decision (Q1); when it completes,
    /// the ranking rule (§1.3: `retained` first, then `accepted`) selects
    /// the history and the winner installs it.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::replica) fn plan_do_view_change(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        retained: ViewId,
        accepted: Slot,
        committed: Slot,
        suffix: &[LogEntry],
        evidence: EvidenceKind,
        era_proof: &EraProof,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let header = message.header;
        // Planned evidence belongs to the non-stop overlap path
        // (§8.7.7): it is routed by its kind — distinguishable on the
        // wire — and never lands in the ordinary attempt, where it would
        // count toward a quorum it is not a vote in.
        if evidence == EvidenceKind::Planned {
            return self.plan_planned_evidence(
                journal, from, message, retained, accepted, committed, suffix, era_proof, at, kind,
            );
        }
        let Some(record) = self.progress.config().record(header.view.era) else {
            return self.drop_plan(
                Diagnostic::UnevaluableEra {
                    era: header.view.era,
                },
                kind,
            );
        };
        if record.config.weight_of(from).is_none() {
            return self.drop_plan(Diagnostic::UnknownSender { sender: from }, kind);
        }
        // Shape: the header slot names the reported accepted frontier
        // (rule 7's Frontier role); the frontiers are a legal chain; the
        // suffix is a contiguous ascending run ending at the frontier; the
        // era proof matches the configuration history
        // (§8.7.8).
        if header.slot != accepted
            || committed > accepted
            || !suffix_shape_ok(suffix, accepted)
            || !self.era_proof_ok(journal, record, era_proof)
        {
            return self.drop_plan(Diagnostic::MalformedViewChange, kind);
        }
        let Some(view_change) = self.view_change.clone() else {
            return self.drop_plan(
                Diagnostic::StaleEvidence {
                    got: header.view,
                    current: self.progress.current(),
                },
                kind,
            );
        };
        if header.view < view_change.target {
            return self.drop_plan(
                Diagnostic::StaleEvidence {
                    got: header.view,
                    current: view_change.target,
                },
                kind,
            );
        }
        if header.view > view_change.target || self.primary_of(view_change.target) != Some(self.own)
        {
            return self.drop_plan(
                Diagnostic::EvidenceNotCollected {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        }
        let mut view_change = view_change;
        view_change.evidence.entry(from).or_insert(Evidence {
            retained,
            accepted,
            committed,
            suffix: suffix.to_vec(),
        });
        let candidate = self.identity_candidate()?;
        self.continue_view_change(journal, candidate, view_change, Vec::new(), at, kind)
    }

    /// A `StartView` (§9.1, §13.1): the designated new primary installing
    /// the selected history.
    ///
    /// The adoption rule: any node the change passed by — `Normal` or
    /// `Recovering` in an earlier view, or fencing into this very view —
    /// installs the offered history, provided it can VERIFY it: the suffix
    /// must reach back to a slot the node can check (its frontier, or a
    /// shared slot whose entry agrees). A suffix that starts past the
    /// node's frontier is a gap — named [`Diagnostic::GapDetected`], kept
    /// fenced, never faulted; the fetch half of the ruling (§13.1 step 5)
    /// rides the same transition and the installed chunks repair the
    /// journal for the next offer.
    /// A suffix that CONFLICTS at a committed local slot is the view-change
    /// path's one deliberate fault-on-peer-input: an honest evidence quorum
    /// can never
    /// produce it, and silently repairing would hide the safety breach, so
    /// the node declares [`Fault::IllegalTransition`].
    #[allow(clippy::too_many_arguments)]
    pub(in crate::replica) fn plan_start_view(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        suffix: &[LogEntry],
        accepted: Slot,
        committed: Slot,
        era_proof: &EraProof,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let header = message.header;
        let current = self.progress.current();
        let Some(record) = self.progress.config().record(header.view.era) else {
            // A `StartView` one era past the current is the overlap
            // transition's offer arriving before the establishing
            // operation did (§8.7.7, a reordering): the ruling is the
            // gap ruling's (§13.1 step 5) — retain the offer, fetch the
            // missing range from the new primary under the CURRENT view,
            // and re-run the ruling on an ordinary tick once the range
            // has folded the era that makes the offer evaluable.
            //
            // An offer MORE than one era past is the §10 learner
            // acquisition's catch-up route when — and only when — the
            // recipient has adopted nothing and the offer NAMES it: the
            // offered era's establishing operation admits the node (a
            // `Join`, the `Increment` that promotes it, or a batch
            // carrying either). Two states have adopted nothing: the
            // boot fence (`Recovering` at `current == retained`, the
            // reopen state), and the reincarnated not-yet-adopted state
            // the forced walk leaves behind (`ViewChange` under the
            // higher-view signal's fence) — recognized there by the
            // same naming test the offer itself carries. The node stays
            // fenced — it adopts nothing here, its votes
            // are never counted — the acquisition
            // (`docs/uvrr-reincarnation.md` §10) is the same ordinary
            // state transfer, and the offered era's fold makes the offer
            // evaluable for the ordinary install that completes the
            // catch-up. Anything else is merely unevaluable: the §8.7.3
            // era/slot discipline caps what a fenced boot view can
            // accept to its own era and the successor, so an unnamed
            // node's far-future offer is not serviced by the fetch at
            // all.
            let plan = self.drop_plan(
                Diagnostic::UnevaluableEra {
                    era: header.view.era,
                },
                kind,
            )?;
            let next = current.era.next();
            let boot_fence =
                self.progress.status() == Status::Recovering && current == self.progress.retained();
            let retainable = if Some(header.view.era) == next {
                true
            } else {
                next.is_some_and(|successor| header.view.era > successor)
                    && (boot_fence || self.progress.status() == Status::ViewChange)
                    && establishing_op_names(&era_proof.op, self.own)
            };
            if retainable {
                let mut plan = plan.with_stalled_offer(from, message.clone());
                if self.transfer.is_none() {
                    if let Some(next_slot) = self.progress.committed().next() {
                        let (effect, fetch) = self.fetch(current, from, next_slot);
                        plan = plan.with_fetch(effect, fetch);
                    }
                }
                return Ok(plan);
            }
            return Ok(plan);
        };
        if self.primary_of(header.view) != Some(from) {
            return self.drop_plan(
                Diagnostic::StartViewNotFromPrimary {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        }
        if header.slot != accepted
            || committed > accepted
            || !suffix_shape_ok(suffix, accepted)
            || !self.era_proof_ok(journal, record, era_proof)
        {
            return self.drop_plan(Diagnostic::MalformedViewChange, kind);
        }
        let current = self.progress.current();
        let adoptable = header.view > current
            || (header.view == current && self.progress.status() == Status::ViewChange);
        if !adoptable {
            return self.drop_plan(
                Diagnostic::StartViewFromStaleView {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        // An honest selection covers everything the node durably committed:
        // the commit quorum intersects the evidence quorum, and the ranking
        // rule keeps the committed prefix (§9.2). An offer that claims less
        // is evidence shaped like knowledge the node already holds — stale,
        // never believed, never fatal.
        if committed < self.progress.committed() || accepted < self.progress.committed() {
            return self.drop_plan(
                Diagnostic::StaleEvidence {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        // The node's own published checkpoint discharges the reclaimed prefix
        // (§4): every reclaimed slot is at or below it, hence at or below
        // the committed frontier, so §9.2's quorum-identity argument fixes
        // the entry — a legal offer carries it, and the install writes
        // nothing there. Without the discharge no reclaimed node could
        // ever verify an offer that reaches past its retained base.
        let mutation = match self.check_suffix(
            journal,
            suffix,
            accepted,
            committed,
            self.progress.checkpoint(),
        ) {
            SuffixCheck::Install(mutation) => mutation,
            SuffixCheck::Gap { expected, got } => {
                let plan = self.drop_plan(Diagnostic::GapDetected { expected, got }, kind)?;
                // §13.1 step 5: the recipient cannot construct the offered
                // history — fetch the missing range from the new primary.
                // The node stays fenced; the retained offer re-runs the
                // ruling on an ordinary tick once the range has arrived.
                let (effect, fetch) = self.fetch(header.view, from, expected);
                return Ok(plan
                    .with_fetch(effect, fetch)
                    .with_stalled_offer(from, message.clone()));
            }
            SuffixCheck::Conflict => {
                let candidate = self.identity_candidate()?;
                return Ok(self
                    .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
                    .with_fault_declared(Fault::IllegalTransition));
            }
        };
        let applied = self.applied_walk(journal, suffix, self.progress.applied(), committed)?;
        // §8.7.1: the installed history's committed frontier may cover
        // system operations this node never folded — the era advances
        // with the install, exactly as if the commit had arrived in
        // order. A fold refusal in the installed COMMITTED history is
        // the same breach as a committed-slot conflict (§9.1): declare
        // it, never guess a repair.
        let config =
            match self.fold_committed(journal, suffix, self.progress.committed(), committed) {
                Ok(config) => config,
                Err(CommitFold::Unavailable(slot)) => {
                    return Err(PlanRefusal::JournalEntryUnavailable { slot });
                }
                Err(CommitFold::Breach { .. }) => return self.breach_plan(kind),
            };
        let candidate =
            self.install_candidate(header.view, accepted, committed, applied, config)?;
        let effects =
            self.apply_effects_merged(journal, suffix, self.progress.committed(), committed)?;
        Ok(self
            .candidate_plan(candidate, mutation, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                view_change: ViewChangeUpdate::Clear,
                // The install answers any offer the node held open: a
                // stalled gap ruling for this view is the one now
                // completing, and an older view's is dead state.
                stalled: StalledUpdate::Clear,
                // An adopted view supersedes any planned overlap
                // machine: the era now moves with the ordinary change.
                planned: PlannedOverlapUpdate::Clear,
                activity: Some(at),
                ..Bookkeeping::default()
            }))
    }

    /// The designated new primary's install, once the evidence quorum holds
    /// (§9.1): the selected history becomes the node's own, `committed`
    /// advances to the greatest frontier the quorum truthfully reported
    /// (each report is a commit quorum's product, and the selected history
    /// contains every committed entry — §9.2's argument), the newly
    /// committed operation slots apply in slot order (§11.1), and `StartView`
    /// broadcasts the selection with a freshly packed bounded suffix
    /// (§13.1). The uncommitted tail's proposal records are re-seeded from
    /// the entries, so an installed slot still accumulates the
    /// `PrepareOk` votes that commit it.
    #[allow(clippy::too_many_arguments)]
    fn plan_win_view(
        &self,
        journal: &J::View,
        target: ViewId,
        selected: &Evidence,
        evidence: &BTreeMap<NodeId, Evidence>,
        mut effects: Vec<Effect>,
        at: Tick,
        kind: InputKind,
    ) -> Result<WinOutcome, PlanRefusal> {
        let committed = evidence
            .values()
            .map(|member| member.committed)
            .max()
            .unwrap_or(self.progress.committed())
            .max(self.progress.committed());
        if committed > selected.accepted {
            // A reporter claimed a commit the selected history does not
            // cover: jointly impossible for honest evidence (§9.2). Do not
            // install a chain-breaking frontier; name the gap and stay
            // fenced.
            let expected = selected.accepted.next().unwrap_or(selected.accepted);
            return Ok(WinOutcome::Insufficient {
                expected,
                got: committed,
                effects,
            });
        }
        // The same checkpoint discharge as the `StartView` install above.
        let mutation = match self.check_suffix(
            journal,
            &selected.suffix,
            selected.accepted,
            committed,
            self.progress.checkpoint(),
        ) {
            SuffixCheck::Install(mutation) => mutation,
            SuffixCheck::Gap { expected, got } => {
                return Ok(WinOutcome::Insufficient {
                    expected,
                    got,
                    effects,
                });
            }
            SuffixCheck::Conflict => {
                let candidate = self.identity_candidate()?;
                let plan = self
                    .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
                    .with_fault_declared(Fault::IllegalTransition);
                return Ok(WinOutcome::Installed(Box::new(plan)));
            }
        };
        let applied = self.applied_walk(
            journal,
            &selected.suffix,
            self.progress.applied(),
            committed,
        )?;
        // §8.7.1: the selected history's committed frontier may cover
        // system operations this node never folded — the era advances
        // with the install. A fold refusal in the selected COMMITTED
        // history is the same breach as a committed-slot conflict (§9.1):
        // declare it, never guess a repair.
        let config = match self.fold_committed(
            journal,
            &selected.suffix,
            self.progress.committed(),
            committed,
        ) {
            Ok(config) => config,
            Err(CommitFold::Unavailable(slot)) => {
                return Err(PlanRefusal::JournalEntryUnavailable { slot });
            }
            Err(CommitFold::Breach { .. }) => {
                return Ok(WinOutcome::Installed(Box::new(self.breach_plan(kind)?)));
            }
        };
        let candidate =
            self.install_candidate(target, selected.accepted, committed, applied, config)?;
        effects.extend(self.apply_effects_merged(
            journal,
            &selected.suffix,
            self.progress.committed(),
            committed,
        )?);
        let proposals =
            self.installed_proposals(journal, &selected.suffix, committed, selected.accepted);
        let suffix = self.bounded_suffix(journal, selected.accepted, &selected.suffix);
        let message = Message {
            header: Header {
                tag: Tag::StartView,
                view: target,
                slot: selected.accepted,
            },
            body: Body::StartView {
                suffix,
                accepted: selected.accepted,
                committed,
                era_proof: self.era_proof(journal, target.era)?,
            },
        };
        effects.extend(self.backups().into_iter().map(|to| Effect::Send {
            to,
            era: target.era,
            message: message.clone(),
        }));
        let plan = self
            .candidate_plan(candidate, mutation, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                proposals,
                view_change: ViewChangeUpdate::Clear,
                // The won change supersedes any planned overlap machine:
                // the era now moves with the ordinary change.
                planned: PlannedOverlapUpdate::Clear,
                // The StartView broadcast is the new primary's
                // announcement of the view: proof of its life (S4).
                activity: Some(at),
                ..Bookkeeping::default()
            });
        Ok(WinOutcome::Installed(Box::new(plan)))
    }
}

/// Whether `op` — the establishing operation a `StartView` offer's era
/// proof carries — names `node` as a member the era admits: the `Join`
/// that inserts it, the `Increment` that promotes it, or a batch carrying
/// either. A departure (`Decrement`, `Leave`) does not admit; the offer
/// is the node's catch-up route (§10), and a departing node is not
/// catching up.
pub(in crate::replica) fn establishing_op_names(op: &SystemOperation, node: NodeId) -> bool {
    match op {
        SystemOperation::Join { node: joined, .. } => *joined == node,
        SystemOperation::Increment(named) => *named == node,
        SystemOperation::Batch(ops) => ops.iter().any(|sub_op| establishing_op_names(sub_op, node)),
        _ => false,
    }
}
