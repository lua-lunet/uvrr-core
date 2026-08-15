//! Recovery (§10, §6.1): `Recovery` / `RecoveryResponse`, the completion,
//! and its re-drive.
//!
//! A reopened node re-proves its state through an `R_g` quorum rather than
//! from local storage (§8.3's diskless argument: quorum memory, not local
//! storage, survives a crash). A recovery nonce is the recovery input's
//! tick (S4), and the attempt retains a bounded set of them — one per
//! re-drive, the oldest evicted on overflow — so a delayed response to a
//! superseded solicitation is still this episode's, and responses across
//! in-set nonces combine into the one `R_g` quorum. The attempt state is
//! volatile by design — a crash discards it and the reopened node starts
//! a fresh one with a fresh tick. Only the reported view's primary carries
//! installation evidence (§6.1). While the attempt runs, an accepted
//! response whose `committed` exceeds the local frontier fast-forwards
//! it: the sequentially-adjacent, locally journal-present entries above
//! the frontier emit the ordered `Apply` upcalls the node would have
//! emitted had it never crashed (§11.1, B2), and the completion's
//! installed frontiers never move backward from the fast-forwarded ones.
//! A completion that stalls on an unconstructible suffix re-drives from
//! the tick once state transfer has supplied the missing range (§13.1
//! step 5).

use super::reconfiguration::CommitFold;
use super::*;

/// What a committed fast-forward found in the range it covered.
enum FastForward {
    /// The evidence claimed nothing beyond the local frontier — a
    /// duplicate or overlapping response is an identity transition.
    Identity,
    /// The frontier advances: the candidate (with the §8.7.1 fold
    /// carried) and the ordered `Apply` upcalls over the newly committed
    /// range.
    Advanced(Progress, Vec<Effect>),
    /// A committed system operation in the range would not fold
    /// (§8.7.2): the journal and the configuration history disagree
    /// about a COMMITTED slot — the caller declares the breach.
    Breach,
}

impl<J: Journal, Q: QuorumStrategy> Replica<J, Q> {
    /// Begin (or re-drive) a recovery attempt (§10, §6.1): broadcast the
    /// solicitation to the rest of the configuration and record the nonce
    /// in the volatile attempt state.
    ///
    /// A nonce IS the recovery input's tick (S4) — one value cannot
    /// disagree with itself, and §6.1's retry-with-a-fresh-nonce rule is a
    /// retry with a fresh tick. Only a fenced `Recovering` node recovers:
    /// recovery is how a reopened node re-proves its state, and every other
    /// status answers [`PlanRejection::NotRecovering`]. A re-drive inserts
    /// the fresh tick into the open attempt's bounded nonce set — the
    /// oldest evicted on overflow — and PRESERVES the collected responses:
    /// a delayed answer to a remembered nonce is still this episode's. A
    /// fresh start opens with a singleton set.
    ///
    /// The node never counts itself: the `R_g` quorum is other replicas'
    /// responses (§8.3), so the solicitation goes only to the backups.
    pub(in crate::replica) fn plan_recover(
        &self,
        at: Tick,
    ) -> Result<PlannedTransition, PlanRejection> {
        if self.progress.status() != Status::Recovering {
            return Err(PlanRejection::NotRecovering {
                status: self.progress.status(),
            });
        }
        let current = self.progress.current();
        let message = Message {
            header: Header {
                tag: Tag::Recovery,
                view: current,
                slot: Slot::NONE,
            },
            body: Body::Recovery { nonce: at },
        };
        let effects = self
            .backups()
            .into_iter()
            .map(|to| Effect::Send {
                to,
                era: current.era,
                message: message.clone(),
            })
            .collect();
        let attempt = match &self.recovery {
            Some(open) => {
                let mut attempt = open.clone();
                attempt.nonces.insert(at);
                if attempt.nonces.len() > MAX_RECOVERY_NONCES {
                    attempt.nonces.pop_first();
                }
                attempt
            }
            None => RecoveryVolatile {
                nonces: BTreeSet::from([at]),
                responses: BTreeMap::new(),
                // This life's emission boundary (§11.1) opens at the
                // durable frontier; a re-drive preserves it through the
                // clone above, and completion clears it with the attempt.
                open_committed: self.progress.committed(),
                state_request: None,
            },
        };
        let candidate = self.identity_candidate()?;
        Ok(self
            .candidate_plan(
                candidate,
                JournalMutation::None,
                effects,
                InputKind::Recovery,
                false,
            )
            .with_bookkeeping(Bookkeeping {
                recovery: RecoveryUpdate::Set(attempt),
                ..Bookkeeping::default()
            }))
    }

    /// A `Recovery` solicitation (§10, §6.1): only a `Normal` node answers
    /// — a node that has not proved its state current cannot vouch for the
    /// cluster's, so a fenced, recovering or replaying node declines. The
    /// answer echoes the nonce (a delayed answer the episode no longer
    /// remembers is then stale at the recoverer), reports the responder's
    /// current view — the fence knowledge the `F_g ⌢ R_g` intersection
    /// (§8.3) exists to deliver — and carries the bounded history suffix
    /// only when the responder is the primary of the view it reports: the
    /// primary's log is the one installation evidence may come from (§6.1).
    pub(in crate::replica) fn plan_recovery_request(
        &self,
        from: NodeId,
        nonce: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let Some(record) = self.current_record() else {
            return Err(PlanRejection::Progress(ProgressError::EraSlotDiscipline));
        };
        if record.config.weight_of(from).is_none() {
            return self.drop_plan(Diagnostic::UnknownSender { sender: from }, kind);
        }
        if self.progress.status() != Status::Normal {
            return self.drop_plan(Diagnostic::RecoveryWhileNotNormal, kind);
        }
        let journal = self.journal.view();
        let current = self.progress.current();
        let suffix = if self.primary_of(current) == Some(self.own) {
            Some(self.bounded_suffix(&journal, self.progress.accepted(), &[]))
        } else {
            None
        };
        let response = Message {
            header: Header {
                tag: Tag::RecoveryResponse,
                view: current,
                slot: Slot::NONE,
            },
            body: Body::RecoveryResponse {
                nonce,
                view: current,
                accepted: self.progress.accepted(),
                committed: self.progress.committed(),
                suffix,
            },
        };
        let effects = vec![Effect::Send {
            to: from,
            era: current.era,
            message: response,
        }];
        let candidate = self.identity_candidate()?;
        Ok(self.candidate_plan(candidate, JournalMutation::None, effects, kind, false))
    }

    /// A `RecoveryResponse` (§6.1, §8.3): one weighted vote toward the open
    /// attempt's `R_g` quorum — never the node's own — and, once the
    /// quorum holds, the completion ruling: the latest view the quorum
    /// reports is the latest fenced view the attempt can know (`F_g ⌢
    /// R_g`), and only that view's primary's response is installation
    /// evidence. An accepted response whose `committed` exceeds the local
    /// frontier fast-forwards it within the same transition (§6.1, §11.1,
    /// B2 — see [`Self::plan_committed_fast_forward`]).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::replica) fn plan_recovery_response(
        &self,
        journal: &J::View,
        from: NodeId,
        nonce: Tick,
        view: ViewId,
        accepted: Slot,
        committed: Slot,
        suffix: &Option<Vec<LogEntry>>,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        // The nonce names the episode: a response whose echoed nonce no
        // remembered solicitation carries — to an earlier episode, an
        // evicted nonce, or none — is stale and counts toward nothing
        // (§6.1).
        let Some(attempt) = self.recovery.clone() else {
            return self.drop_plan(
                Diagnostic::StaleRecoveryResponse {
                    nonce,
                    attempt: None,
                },
                kind,
            );
        };
        if !attempt.nonces.contains(&nonce) {
            return self.drop_plan(
                Diagnostic::StaleRecoveryResponse {
                    nonce,
                    attempt: attempt.nonces.iter().next_back().copied(),
                },
                kind,
            );
        }
        // A node never counts itself: `R_g` is other replicas' evidence.
        if from == self.own {
            return self.drop_plan(Diagnostic::RecoveryResponseFromSelf, kind);
        }
        let Some(record) = self.current_record() else {
            return Err(PlanRejection::Progress(ProgressError::EraSlotDiscipline));
        };
        if record.config.weight_of(from).is_none() {
            return self.drop_plan(Diagnostic::UnknownSender { sender: from }, kind);
        }
        if self.progress.config().record(view.era).is_none() {
            return self.drop_plan(Diagnostic::UnevaluableEra { era: view.era }, kind);
        }
        // Shape: the frontiers are a legal chain, and an offered suffix is
        // a contiguous ascending run ending at the accepted frontier (§13.1).
        if committed > accepted
            || suffix
                .as_ref()
                .is_some_and(|s| !suffix_shape_ok(s, accepted))
        {
            return self.drop_plan(Diagnostic::MalformedRecovery, kind);
        }
        // Only the primary of the reported view may carry history (§6.1).
        if suffix.is_some() && self.primary_of(view) != Some(from) {
            return self.drop_plan(
                Diagnostic::RecoveryHistoryNotFromPrimary { sender: from, view },
                kind,
            );
        }
        // Evidence claiming less than the node durably committed is shaped
        // like knowledge the node already holds: stale, never believed.
        if committed < self.progress.committed() {
            return self.drop_plan(
                Diagnostic::StaleEvidence {
                    got: view,
                    current: self.progress.current(),
                },
                kind,
            );
        }
        let mut attempt = attempt;
        attempt.responses.insert(
            from,
            RecoveryEvidence {
                view,
                accepted,
                committed,
                suffix: suffix.clone(),
            },
        );
        // The completion ruling becomes evaluable only when the weighted
        // `R_g` quorum holds (the strategy decides — Q1). Until it does,
        // the attempt just records the response and waits for more. An
        // accepted response whose committed frontier exceeds the local
        // one fast-forwards it within this same transition (§6.1); a
        // duplicate or overlapping response claims nothing new and the
        // fast-forward is the identity.
        let record_attempt = |attempt: RecoveryVolatile, diagnostic| {
            let plan = match self.plan_committed_fast_forward(journal, committed)? {
                FastForward::Advanced(candidate, effects) => {
                    self.candidate_plan(candidate, JournalMutation::None, effects, kind, false)
                }
                FastForward::Identity => self.candidate_plan(
                    self.identity_candidate()?,
                    JournalMutation::None,
                    Vec::new(),
                    kind,
                    false,
                ),
                // A fold refusal in the fast-forwarded COMMITTED range is
                // the same breach as a committed-slot conflict (§9.1):
                // declare it.
                FastForward::Breach => return self.breach_plan(kind),
            };
            Ok(plan
                .with_bookkeeping(Bookkeeping {
                    recovery: RecoveryUpdate::Set(attempt),
                    ..Bookkeeping::default()
                })
                .with_diagnostic(diagnostic))
        };
        let responders: Vec<NodeId> = attempt.responses.keys().copied().collect();
        if !self
            .strategy
            .is_quorum(Role::Recovery, &record.config, &responders)
        {
            return record_attempt(attempt, Diagnostic::None);
        }
        let Some(latest) = attempt
            .responses
            .values()
            .map(|evidence| evidence.view)
            .max()
        else {
            unreachable!("a quorum of responses is never empty");
        };
        if latest < self.progress.current() {
            // Every responder is behind the view this node already fenced:
            // its own fence is the newest knowledge the quorum holds
            // (V_g ⌢ V_g, §8.3) and no installable history is on offer.
            // The attempt stays open for a fresher round.
            return record_attempt(
                attempt,
                Diagnostic::StaleEvidence {
                    got: latest,
                    current: self.progress.current(),
                },
            );
        }
        let install = attempt
            .responses
            .iter()
            .find(|(_, evidence)| evidence.view == latest && evidence.suffix.is_some())
            .map(|(source, evidence)| (*source, evidence.clone()));
        let Some((source, evidence)) = install else {
            // The quorum holds but the latest fenced view's primary has not
            // answered with history: keep collecting; nothing installs from
            // non-primary evidence (§6.1).
            return record_attempt(attempt, Diagnostic::None);
        };
        self.plan_recovery_completion(journal, attempt, source, latest, evidence, at, kind, None)
    }

    /// The host's answer to an outstanding
    /// [`Effect::RequestApplicationState`] (§4, §11): the install input
    /// completes the recovery the reclaimed journal could not serve. The
    /// preconditions are the shortfall state itself — an open attempt with
    /// a request outstanding — and the `through` naming that request back
    /// exactly; anything else is a named refusal, no state change. On
    /// acceptance the completing ruling runs with the captured evidence:
    /// the restored state stands in for the replay at or below `through`,
    /// and the suffix verification treats the reclaimed slots the install
    /// covers as agreed.
    pub(in crate::replica) fn plan_application_state_installed(
        &self,
        journal: &J::View,
        through: Slot,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let Some(attempt) = self.recovery.clone() else {
            return Err(PlanRejection::ApplicationStateNotRequested);
        };
        let Some(request) = attempt.state_request.clone() else {
            return Err(PlanRejection::ApplicationStateNotRequested);
        };
        if through != request.through {
            return Err(PlanRejection::ApplicationStateMismatch {
                expected: request.through,
                got: through,
            });
        }
        self.plan_recovery_completion(
            journal,
            attempt,
            request.source,
            request.latest,
            request.evidence,
            at,
            kind,
            Some(through),
        )
    }

    /// The committed fast-forward of an accepted `RecoveryResponse`
    /// (§6.1, §10, §11.1): evidence whose `committed` exceeds the local
    /// frontier advances it over the sequentially-adjacent, locally
    /// journal-present slots — takeWhile: the walk stops at the first
    /// slot the journal does not physically hold — emitting the ordered
    /// `Apply` upcalls (B2) the node would have emitted had it never
    /// crashed. The applied frontier moves only by the §11 system-slot
    /// walk: the operation slots await the host's `Input::Applied`
    /// acknowledgements through the ordinary path, exactly as in normal
    /// operation, and the node stays fenced `Recovering`. Returns
    /// [`FastForward::Identity`] when the evidence claims nothing beyond
    /// the local frontier — a duplicate or overlapping response is an
    /// identity transition: slots at or below the frontier are skipped.
    fn plan_committed_fast_forward(
        &self,
        journal: &J::View,
        claimed: Slot,
    ) -> Result<FastForward, PlanRejection> {
        let committed = self.progress.committed();
        // takeWhile over sequentially-adjacent, locally journal-present
        // slots: the frontier the evidence vouches for, capped at the
        // first slot the journal does not physically hold.
        let mut target = committed;
        while let Some(next) = target.next() {
            if next > claimed || journal.get(next).is_none() {
                break;
            }
            target = next;
        }
        if target == committed {
            return Ok(FastForward::Identity);
        }
        // The same shape the recovery completion's replay takes (§11.1):
        // the ordered `Apply` upcalls over the newly committed range, and
        // the §11 system-slot walk over the same range for `applied` —
        // and the §8.7.1 fold over the same range for the era table.
        let applies = self.apply_effects_merged(journal, &[], committed, target)?;
        let applied = self.applied_walk(journal, &[], self.progress.applied(), target)?;
        let config = match self.fold_committed(journal, &[], committed, target) {
            Ok(config) => config,
            Err(CommitFold::Unavailable(slot)) => {
                return Err(PlanRejection::JournalEntryUnavailable { slot });
            }
            Err(CommitFold::Breach { .. }) => return Ok(FastForward::Breach),
        };
        let candidate = self.candidate_with(
            self.progress.status(),
            self.progress.accepted(),
            target,
            applied,
            config,
        )?;
        Ok(FastForward::Advanced(candidate, applies))
    }

    /// The completing ruling of an open recovery attempt, when it is
    /// evaluable AND constructible: the weighted `R_g` quorum holds (Q1),
    /// the latest fenced view is not behind the node's own fence, that
    /// view's primary has answered with history — and the offered suffix
    /// installs against the local journal. The last clause is what a
    /// finished state-transfer fetch flips (§13.1 step 5); the tick
    /// re-drive asks here.
    pub(in crate::replica) fn recovery_completion_ready(
        &self,
        journal: &J::View,
        attempt: &RecoveryVolatile,
    ) -> Option<(NodeId, ViewId, RecoveryEvidence)> {
        let record = self.current_record()?;
        let responders: Vec<NodeId> = attempt.responses.keys().copied().collect();
        if !self
            .strategy
            .is_quorum(Role::Recovery, &record.config, &responders)
        {
            return None;
        }
        let latest = attempt
            .responses
            .values()
            .map(|evidence| evidence.view)
            .max()?;
        if latest < self.progress.current() {
            return None;
        }
        let (source, evidence) = attempt
            .responses
            .iter()
            .find(|(_, evidence)| evidence.view == latest && evidence.suffix.is_some())?;
        let suffix = evidence
            .suffix
            .as_ref()
            .expect("the find clause required a suffix");
        match self.check_suffix(
            journal,
            suffix,
            evidence.accepted,
            evidence.committed,
            Slot::NONE,
        ) {
            SuffixCheck::Install(_) => Some((*source, latest, evidence.clone())),
            SuffixCheck::Gap { .. } | SuffixCheck::Conflict => None,
        }
    }

    /// The completing ruling of a recovery attempt (§6.1, §10): install the
    /// latest fenced view's history from its primary's response — protocol
    /// evidence only — and re-emit the committed-but-unapplied suffix as
    /// ordered `Apply` upcalls (§11.1, B2: the core re-emits the upcall,
    /// never a reply — no reply was ever emitted at this node). Completion
    /// below `committed` leaves the node `Replaying` until `applied ==
    /// committed`; a retained base above the base the recovery must read
    /// from surfaces [`Effect::RequestApplicationState`] (§4, §11), never
    /// a fault. The installed committed frontier never moves backward
    /// (§1.3): a fast-forward earlier in the attempt may have advanced
    /// the local frontier past the completing evidence's. The replay
    /// emits the durable debt — what was committed when the attempt
    /// opened, never emitted this life — plus the range this completion
    /// itself newly installs; the slots between, exactly what this
    /// life's fast-forward already emitted, are not re-emitted. The
    /// boundary is volatile: a crash discards it, and the reopened
    /// node's replay re-emits from the durable `applied` unchanged
    /// (§11.1's at-least-once boundary is the crash, not the lagging
    /// acknowledgement).
    ///
    /// `installed` is the host's answer to the shortfall (§4, §11):
    /// [`Some`] when this completion is driven by
    /// [`Input::ApplicationStateInstalled`], naming the frontier the host
    /// restored application state through. The restored state stands in
    /// for the replay at or below it: no upcall re-emits for those slots,
    /// `applied` walks from the installed frontier, and the suffix
    /// verification treats the reclaimed slots the install covers as
    /// agreed (see [`Self::check_suffix`]).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::replica) fn plan_recovery_completion(
        &self,
        journal: &J::View,
        attempt: RecoveryVolatile,
        source: NodeId,
        latest: ViewId,
        evidence: RecoveryEvidence,
        at: Tick,
        kind: InputKind,
        installed: Option<Slot>,
    ) -> Result<PlannedTransition, PlanRejection> {
        let Some(suffix) = evidence.suffix.as_ref() else {
            unreachable!("installation evidence carries a suffix");
        };
        // The frontier this completion drives the replay and the install
        // to: the completing evidence's, floored at the local one — a
        // committed fast-forward earlier in the attempt advanced it, and
        // no completion moves it backward (§1.3).
        let committed = evidence.committed.max(self.progress.committed());
        let shortfall = |mut attempt: RecoveryVolatile, required: Slot, retained: Slot| {
            // The journal physically let the required prefix go (S1): §4's
            // answer is the host's application-state transfer facility, not
            // a fault and not a protocol fetch. The request is recorded
            // against the attempt: the host's install input must name its
            // `through` back, and the captured evidence is the ruling the
            // acceptance completes.
            attempt.state_request = Some(StateRequest {
                through: committed,
                source,
                latest,
                evidence: evidence.clone(),
            });
            let candidate = self.identity_candidate()?;
            Ok(self
                .candidate_plan(
                    candidate,
                    JournalMutation::None,
                    vec![Effect::RequestApplicationState { through: committed }],
                    kind,
                    false,
                )
                .with_bookkeeping(Bookkeeping {
                    recovery: RecoveryUpdate::Set(attempt),
                    ..Bookkeeping::default()
                })
                .with_diagnostic(Diagnostic::ApplicationStateShortfall { required, retained }))
        };
        let mutation = match self.check_suffix(
            journal,
            suffix,
            evidence.accepted,
            evidence.committed,
            installed.unwrap_or(Slot::NONE),
        ) {
            SuffixCheck::Install(mutation) => mutation,
            SuffixCheck::Gap { expected, got } => {
                let (retained_base, _) = journal.retained();
                if expected < retained_base {
                    return shortfall(attempt, expected, retained_base);
                }
                // The offer does not reach back far enough to verify —
                // the budget-truncation case (§13.1, W5): fetch the
                // missing range from the responder (§13.1 step 5)
                // instead of waiting for a fuller offer. The attempt
                // stays open; completion re-runs on a tick once the
                // range has arrived.
                let (effect, fetch) = self.fetch(latest, source, expected);
                let candidate = self.identity_candidate()?;
                return Ok(self
                    .candidate_plan(candidate, JournalMutation::None, vec![effect], kind, false)
                    .with_bookkeeping(Bookkeeping {
                        recovery: RecoveryUpdate::Set(attempt),
                        transfer: TransferUpdate::Set(fetch),
                        ..Bookkeeping::default()
                    })
                    .with_diagnostic(Diagnostic::GapDetected { expected, got }));
            }
            SuffixCheck::Conflict => {
                // An honest recovery quorum can never contradict a slot
                // this node durably committed (§8.3, §9.2): declare the
                // breach.
                let candidate = self.identity_candidate()?;
                return Ok(self
                    .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
                    .with_fault_declared(Fault::IllegalTransition));
            }
        };
        // The replay ruling (§11.1): the committed-but-unapplied suffix
        // re-emits as ordered `Apply` upcalls, as the union of two ranges
        // in slot order — the durable debt (applied, open_committed],
        // committed before the attempt opened and never emitted this
        // life; and (local_committed, committed], the range this
        // completion itself newly installs. Between them lies exactly
        // what this life's committed fast-forward already emitted:
        // emitted once, never re-emitted here. The `applied` walk reads
        // from the node's applied seed as ever, and the §11 system-slot
        // ruling folds every committed system slot the walk crosses into
        // `applied` without an upcall or an acknowledgement.
        //
        // Under a host install (§4, §11) the restored state stands in for
        // the replay at or below its frontier: the bases rise to it, both
        // ranges above empty out — the debt at or below the installed
        // frontier is discharged, and a committed fast-forward past it
        // already emitted its upcalls this life — and the journal reads
        // that made the plain completion shortfall never happen.
        let replay_base = match installed {
            Some(through) => self.progress.applied().max(through),
            None => self.progress.applied(),
        };
        let local_committed = self.progress.committed();
        let install_base = match installed {
            Some(through) => local_committed.max(through),
            None => local_committed,
        };
        let mut applies =
            match self.apply_effects_merged(journal, suffix, replay_base, attempt.open_committed) {
                Ok(applies) => applies,
                Err(PlanRejection::JournalEntryUnavailable { slot }) => {
                    let (retained_base, _) = journal.retained();
                    return shortfall(attempt, slot, retained_base);
                }
                Err(rejection) => return Err(rejection),
            };
        match self.apply_effects_merged(journal, suffix, install_base, committed) {
            Ok(newly) => applies.extend(newly),
            Err(PlanRejection::JournalEntryUnavailable { slot }) => {
                let (retained_base, _) = journal.retained();
                return shortfall(attempt, slot, retained_base);
            }
            Err(rejection) => return Err(rejection),
        }
        let applied = match self.applied_walk(journal, suffix, replay_base, committed) {
            Ok(applied) => applied,
            Err(PlanRejection::JournalEntryUnavailable { slot }) => {
                let (retained_base, _) = journal.retained();
                return shortfall(attempt, slot, retained_base);
            }
            Err(rejection) => return Err(rejection),
        };
        // Normal once the walked frontier reached the committed one;
        // Replaying — fenced from participation — while host
        // acknowledgements are still owed.
        let status = if applied == committed {
            Status::Normal
        } else {
            Status::Replaying
        };
        // §8.7.1: the recovered history's committed frontier may cover
        // system operations this node never folded — the era advances
        // with the install, exactly as if the commits had arrived in
        // order. An unavailable slot is the same shortfall the replay
        // ruling names; a fold refusal in the recovered COMMITTED
        // history is the same breach as a committed-slot conflict (§8.3,
        // §9.2): declare it, never guess a repair.
        let config = match self.fold_committed(journal, suffix, local_committed, committed) {
            Ok(config) => config,
            Err(CommitFold::Unavailable(slot)) => {
                let (retained_base, _) = journal.retained();
                return shortfall(attempt, slot, retained_base);
            }
            Err(CommitFold::Breach { .. }) => return self.breach_plan(kind),
        };
        let candidate = self.recovery_candidate(
            latest,
            status,
            evidence.accepted,
            committed,
            applied,
            config,
        )?;
        // The recovery install supersedes any view change the node was
        // fencing (the attempt cleared below), and if the node is the
        // primary of the view it recovered into it picks the reassembled
        // history's uncommitted tail up and starts driving it (§8.1).
        let proposals = if self.primary_of(latest) == Some(self.own) {
            self.installed_proposals(journal, suffix, committed, evidence.accepted)
        } else {
            Vec::new()
        };
        Ok(self
            .candidate_plan(candidate, mutation, applies, kind, false)
            .with_bookkeeping(Bookkeeping {
                proposals,
                view_change: ViewChangeUpdate::Clear,
                recovery: RecoveryUpdate::Clear,
                // The recovered view's primary just proved itself alive:
                // the timeout baseline refreshes (S4).
                activity: Some(at),
                ..Bookkeeping::default()
            }))
    }

    /// The recovery install candidate: `current` and `retained` join at the
    /// latest fenced view (§1.3 — the history was re-selected from protocol
    /// evidence), the frontiers become the installed history's, and the
    /// status is `Normal` when nothing is left to replay, `Replaying` while
    /// committed operations await their application upcalls (§11.1). The
    /// monotone frontiers never move backward (§1.3): a committed
    /// fast-forward earlier in the attempt may have advanced a frontier
    /// past the completing evidence's, so each installed frontier is the
    /// evidence's floored at the local one.
    fn recovery_candidate(
        &self,
        view: ViewId,
        status: Status,
        accepted: Slot,
        committed: Slot,
        applied: Slot,
        config: Arc<EraTable>,
    ) -> Result<Progress, PlanRejection> {
        let committed = committed.max(self.progress.committed());
        let applied = applied.max(self.progress.applied());
        let revision = self
            .progress
            .revision()
            .checked_add(1)
            .ok_or(PlanRejection::Progress(ProgressError::RevisionExhausted))?;
        Progress::reconstitute(
            view,
            view,
            status,
            accepted,
            committed,
            applied,
            self.progress.checkpoint(),
            revision,
            config,
            None,
        )
        .map_err(PlanRejection::Progress)
    }
}
