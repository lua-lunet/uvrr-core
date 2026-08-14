//! Recovery (§10, §6.1): `Recovery` / `RecoveryResponse`, the completion,
//! and its re-drive.
//!
//! A reopened node re-proves its state through an `R_g` quorum rather than
//! from local storage (§8.3's diskless argument: quorum memory, not local
//! storage, survives a crash). The attempt's nonce is the recovery input's
//! tick (S4), and the attempt state is volatile by design — a crash
//! discards it and the reopened node starts a fresh one with a fresh tick.
//! Only the reported view's primary carries installation evidence (§6.1).
//! A completion that stalls on an unconstructible suffix re-drives from the
//! tick once state transfer has supplied the missing range (§13.1 step 5).

use super::*;

impl<J: Journal, Q: QuorumStrategy> Replica<J, Q> {
    /// Begin a recovery attempt (§10, §6.1): broadcast the solicitation to
    /// the rest of the configuration and open the volatile attempt state.
    ///
    /// The nonce IS the recovery input's tick (S4) — one value cannot
    /// disagree with itself, and §6.1's retry-with-a-fresh-nonce rule is a
    /// retry with a fresh tick. Only a fenced `Recovering` node recovers:
    /// recovery is how a reopened node re-proves its state, and every other
    /// status answers [`PlanRejection::NotRecovering`]. A fresh attempt
    /// replaces an open one wholesale — the old nonce dies with it, which
    /// is what makes a delayed response to it stale.
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
                slot: Slot::FIRST,
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
                recovery: RecoveryUpdate::Set(RecoveryVolatile {
                    nonce: at,
                    responses: BTreeMap::new(),
                }),
                ..Bookkeeping::default()
            }))
    }

    /// A `Recovery` solicitation (§10, §6.1): only a `Normal` node answers
    /// — a node that has not proved its state current cannot vouch for the
    /// cluster's, so a fenced, recovering or replaying node declines. The
    /// answer echoes the nonce (a delayed answer to an earlier attempt is
    /// then stale at the recoverer), reports the responder's current view —
    /// the fence knowledge the `F_g ⌢ R_g` intersection (§8.3) exists to
    /// deliver — and carries the bounded history suffix only when the
    /// responder is the primary of the view it reports: the primary's log
    /// is the one installation evidence may come from (§6.1).
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
                slot: Slot::FIRST,
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
    /// evidence.
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
        // The nonce names the attempt: a response to an earlier attempt —
        // or to none — is stale and counts toward nothing (§6.1).
        let Some(attempt) = self.recovery.clone() else {
            return self.drop_plan(
                Diagnostic::StaleRecoveryResponse {
                    nonce,
                    attempt: None,
                },
                kind,
            );
        };
        if nonce != attempt.nonce {
            return self.drop_plan(
                Diagnostic::StaleRecoveryResponse {
                    nonce,
                    attempt: Some(attempt.nonce),
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
        // the attempt just records the response and waits for more.
        let record_attempt = |attempt: RecoveryVolatile, diagnostic| {
            let candidate = self.identity_candidate()?;
            Ok(self
                .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
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
        self.plan_recovery_completion(journal, attempt, source, latest, evidence, at, kind)
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
        match self.check_suffix(journal, suffix, evidence.accepted, evidence.committed) {
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
    /// a fault.
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
    ) -> Result<PlannedTransition, PlanRejection> {
        let Some(suffix) = evidence.suffix.as_ref() else {
            unreachable!("installation evidence carries a suffix");
        };
        let shortfall = |attempt: RecoveryVolatile, required: Slot, retained: Slot| {
            // The journal physically let the required prefix go (S1): §4's
            // answer is the host's application-state transfer facility, not
            // a fault and not a protocol fetch.
            let candidate = self.identity_candidate()?;
            Ok(self
                .candidate_plan(
                    candidate,
                    JournalMutation::None,
                    vec![Effect::RequestApplicationState {
                        through: evidence.committed,
                    }],
                    kind,
                    false,
                )
                .with_bookkeeping(Bookkeeping {
                    recovery: RecoveryUpdate::Set(attempt),
                    ..Bookkeeping::default()
                })
                .with_diagnostic(Diagnostic::ApplicationStateShortfall { required, retained }))
        };
        let mutation =
            match self.check_suffix(journal, suffix, evidence.accepted, evidence.committed) {
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
        // The replay ruling (§11.1): everything committed-but-unapplied —
        // by the installed history's `committed` — is re-emitted as
        // ordered `Apply` upcalls. The replay reads from the node's
        // applied seed, so what the node durably applied is never
        // re-applied; and the §11 system-slot ruling folds every committed
        // system slot the walk crosses into `applied` without an upcall or
        // an acknowledgement.
        let replay_base = self.progress.applied();
        let applies =
            match self.apply_effects_merged(journal, suffix, replay_base, evidence.committed) {
                Ok(applies) => applies,
                Err(PlanRejection::JournalEntryUnavailable { slot }) => {
                    let (retained_base, _) = journal.retained();
                    return shortfall(attempt, slot, retained_base);
                }
                Err(rejection) => return Err(rejection),
            };
        let applied = match self.applied_walk(journal, suffix, replay_base, evidence.committed) {
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
        let status = if applied == evidence.committed {
            Status::Normal
        } else {
            Status::Replaying
        };
        let candidate = self.recovery_candidate(
            latest,
            status,
            evidence.accepted,
            evidence.committed,
            applied,
        )?;
        // The recovery install supersedes any view change the node was
        // fencing (the attempt cleared below), and if the node is the
        // primary of the view it recovered into it picks the reassembled
        // history's uncommitted tail up and starts driving it (§8.1).
        let proposals = if self.primary_of(latest) == Some(self.own) {
            self.installed_proposals(journal, suffix, evidence.committed, evidence.accepted)
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
    /// committed operations await their application upcalls (§11.1).
    fn recovery_candidate(
        &self,
        view: ViewId,
        status: Status,
        accepted: Slot,
        committed: Slot,
        applied: Slot,
    ) -> Result<Progress, PlanRejection> {
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
            Arc::clone(self.progress.config()),
            None,
        )
        .map_err(PlanRejection::Progress)
    }
}
