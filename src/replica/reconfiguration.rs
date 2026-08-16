//! Reconfiguration: the stop-the-world path (§8.7.4) and the non-stop
//! overlap transition (§8.7.6–§8.7.7).
//!
//! A system operation is proposed through the ORDINARY pipeline like any
//! entry — one establishing operation at a time — and the era advances
//! exactly when the operation COMMITS (§8.7.1: an era is established by
//! the commit of its establishing operation, never by its acceptance).
//! The pivot is `None` on the stop-the-world path: the establishing
//! `Prepare` goes to every backup and the cluster enters the new era
//! through the next ordinary view change. A `Some` pivot names the
//! non-stop variant (§8.7.6–§8.7.7): the `Prepare` goes only to
//! `qII − {L}`, and when the operation commits through `qII` the leader
//! solicits PLANNED evidence from `qI − {L}` — never a fence — and, the
//! planned quorum complete, publishes its casting vote and its switch to
//! `v'` as ONE transition, then announces `StartView(v')` to every
//! member of config(e+1). The client stream is never interrupted:
//! ordinary era-(e+1) prepares continue through `qII` while the planned
//! exchange runs.
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
//!   enters the log. The pivot never substitutes for this gate.
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
//! * **Planned evidence is transient and never a fence** (§8.7.7):
//!   `PlannedViewChange` recipients retain `current_view = v` and keep
//!   accepting valid era-e `Prepare`; their `EvidenceKind::Planned`
//!   answers complete the planned quorum and are never counted toward a
//!   `Role::Fence` or ordinary `Role::ViewChange` quorum — the kinds are
//!   distinguishable on the wire and routed by kind.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::configuration::{ConfigError, EraTable};
use crate::effects::Effect;
use crate::ids::{NodeId, Slot, Tick, ViewId, next_view_selecting};
use crate::journal::{JournalView, LogEntry, Payload};
use crate::message::{Body, EraProof, EvidenceKind, Message};
use crate::observe::Diagnostic;
use crate::progress::Status;
use crate::quorum::{validate_era, validate_pivot, validate_transition};
use crate::wire::{Header, Tag};

use super::{
    Bookkeeping, Evidence, InputKind, Journal, JournalMutation, Pivot, PlanRejection,
    PlannedOverlap, PlannedOverlapUpdate, PlannedTransition, ProgressError, Proposal,
    QuorumStrategy, Replica, SystemOperation, suffix_shape_ok,
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
    /// 1. the pivot, when `Some`, satisfies the §8.7.6 pivot condition
    ///    ([`PlanRejection::ReconfigurePivot`]);
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
    /// A refusal at any gate never enters the log. The pivot never
    /// substitutes for the family-level `validate_transition` gate: an
    /// operation the gate refuses is refused with or without a pivot.
    pub(in crate::replica) fn plan_reconfigure(
        &self,
        journal: &J::View,
        op: &SystemOperation,
        pivot: &Option<Pivot>,
    ) -> Result<PlannedTransition, PlanRejection> {
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
        // Gate 1 (deferred): the pivot, when `Some`, satisfies the
        // §8.7.6 pivot condition. The check runs after the fold so the
        // next configuration is available for the qII-under-both leg.
        if let Some(pivot) = pivot {
            validate_pivot(
                &self.strategy,
                &record.config,
                &next_table.current().config,
                self.own,
                pivot,
            )
            .map_err(PlanRejection::ReconfigurePivot)?;
        }
        // Gate 6: the closed intersection obligations (§8.7.4, Q1). R2
        // across the boundary runs first so a cross-era refusal names the
        // cross-era witness; the within-era obligations follow. The pivot
        // never substitutes for this gate: an operation the gate refuses
        // is refused with or without a pivot.
        validate_transition(&self.strategy, &record.config, &next_table.current().config)
            .map_err(PlanRejection::ReconfigureQuorum)?;
        validate_era(&self.strategy, &next_table.current().config)
            .map_err(PlanRejection::ReconfigureQuorum)?;
        // The recipients and the overlap machine (§8.7.6–§8.7.7). Without
        // a pivot the establishing Prepare goes to every backup and the
        // era awaits the ordinary view change. With a pivot it goes only
        // to `qII − {L}` — the commit vote set under both configurations —
        // and the machine arms: `v'` is named NOW (§8.7.7 step 5's least
        // view past the current one selecting this node under the NEW
        // order), an unrepresentable `v'` refusing the proposal outright
        // rather than wrapping the view space.
        let mut recipients: Vec<NodeId> = self.backups();
        let mut planned = PlannedOverlapUpdate::Unchanged;
        if let Some(pivot) = pivot {
            let next_record = next_table.current();
            let members = u32::try_from(next_record.config.order().len())
                .expect("the membership order fits the view arithmetic");
            let index = next_record
                .config
                .order()
                .iter()
                .position(|member| member.node == self.own)
                .expect("the pivot validation puts the leader in both configurations");
            let index =
                u32::try_from(index).expect("the membership order fits the view arithmetic");
            let view = next_view_selecting(current.view, index, members)
                .ok_or(PlanRejection::ReconfigureViewExhausted { current })?;
            let target = ViewId {
                era: next_record.era,
                view,
            };
            recipients = pivot
                .q_ii
                .iter()
                .copied()
                .filter(|node| *node != self.own)
                .collect();
            planned = PlannedOverlapUpdate::Set(PlannedOverlap {
                establishing: slot,
                pivot: pivot.clone(),
                target,
                solicited: false,
                evidence: BTreeMap::new(),
            });
        }
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
        let effects = recipients
            .into_iter()
            .map(|to| Effect::Send {
                to,
                era: current.era,
                message: prepare.clone(),
            })
            .collect();
        let bookkeeping = Bookkeeping {
            proposals: vec![(slot, Proposal { oks: Vec::new() })],
            planned,
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

    /// §8.7.7 steps 1 and 4, at the commit of the establishing operation:
    /// the fold that just established the successor era records the pivot
    /// that carries the transition ([`EraTable::with_transition_pivot`]),
    /// and the `PlannedViewChange` solicitation goes to `qI − {L}` —
    /// NEVER to `qII − {L}`, whose ordinary prepare stream is the one the
    /// non-stop transition exists to preserve. The solicitation rides the
    /// same published transition as the fold, so the machine's `solicited`
    /// flag and the era record's pivot appear atomically with the commit.
    ///
    /// Called from the primary's commit-advance path with the folded
    /// table and the new committed frontier; a no-op triple when no armed
    /// machine's establishing operation committed in this advance.
    pub(in crate::replica) fn overlap_solicitation(
        &self,
        config: Arc<EraTable>,
        covered: Slot,
    ) -> (Arc<EraTable>, Vec<Effect>, PlannedOverlapUpdate) {
        let no_op = |config| (config, Vec::new(), PlannedOverlapUpdate::Unchanged);
        let Some(planned) = &self.planned else {
            return no_op(config);
        };
        // The establishing operation committed in THIS advance when it
        // sits inside the newly covered range and the fold recorded the
        // successor era at exactly its slot.
        if planned.solicited
            || planned.establishing <= self.progress.committed()
            || planned.establishing > covered
            || config.current().established_by != planned.establishing
        {
            return no_op(config);
        }
        let era = config.current().era;
        let config = Arc::new(
            config
                .with_transition_pivot(era, planned.pivot.clone())
                .expect("the fold just recorded the establishing era"),
        );
        let current = self.progress.current();
        let message = Message {
            header: Header {
                tag: Tag::PlannedViewChange,
                view: planned.target,
                slot: Slot::NONE,
            },
            body: Body::PlannedViewChange {},
        };
        // `qI − {L}` — the view-change vote set under config(e) — routed
        // under the CURRENT era: its members are era-e members, and a
        // member the reconfiguration removes must still get its vote in.
        let effects = planned
            .pivot
            .q_i
            .iter()
            .copied()
            .filter(|node| *node != self.own)
            .map(|to| Effect::Send {
                to,
                era: current.era,
                message: message.clone(),
            })
            .collect();
        let update = PlannedOverlapUpdate::Set(PlannedOverlap {
            establishing: planned.establishing,
            pivot: planned.pivot.clone(),
            target: planned.target,
            solicited: true,
            evidence: BTreeMap::new(),
        });
        (config, effects, update)
    }

    /// A `PlannedViewChange` solicitation (§8.7.7 step 2): the serving
    /// primary of the current view is gathering planned evidence for the
    /// transition view `v'` it names in the successor era. This is NOT a
    /// fence — the recipient retains `current_view = v`, stays `Normal`,
    /// and keeps accepting valid era-e `Prepare`. It answers from its own
    /// bounded suffix with `EvidenceKind::Planned` (step 3) — transient
    /// evidence, never counted toward a fence quorum — proved against the
    /// CURRENT era, the one the answer speaks from.
    ///
    /// The answer carries no authority, so the guards are deliberately
    /// light: every verification that matters re-fires at the `StartView`
    /// install. A recipient whose table does not yet record the successor
    /// era opens a fetch toward the leader for the range past its
    /// frontier — the suffix fallback (§13.1 step 5 applied to the
    /// overlap): the establishing operation arrives by ordinary state
    /// transfer under the CURRENT view, the commit frontier walks it, and
    /// the era folds, which is what makes the leader's later
    /// `StartView(v')` evaluable here. The leader serves that fetch
    /// before and after its switch: serving is read-only retransmission
    /// and a known era is a known era.
    pub(in crate::replica) fn plan_planned_view_change(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let header = message.header;
        let current = self.progress.current();
        // The solicitation names the SUCCESSOR era. Anything else is
        // stale — the transition is done (a duplicate past the install)
        // or superseded — or unevaluable, when the era is further out
        // than one past the current.
        if Some(header.view.era) != current.era.next() {
            if self.progress.config().record(header.view.era).is_none() {
                return self.drop_plan(
                    Diagnostic::UnevaluableEra {
                        era: header.view.era,
                    },
                    kind,
                );
            }
            return self.drop_plan(
                Diagnostic::StaleEvidence {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        // Only the serving primary of the retained view may solicit.
        if self.primary_of(current) != Some(from) {
            return self.drop_plan(
                Diagnostic::SenderNotPrimary {
                    sender: from,
                    view: current,
                },
                kind,
            );
        }
        // A fenced node does not answer planned evidence: its path is the
        // ordinary change it already joined.
        if self.progress.status() != Status::Normal {
            return self.drop_plan(
                Diagnostic::StaleViewChange {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        let accepted = self.progress.accepted();
        let answer = Message {
            header: Header {
                tag: Tag::DoViewChange,
                view: header.view,
                slot: accepted,
            },
            body: Body::DoViewChange {
                retained: self.progress.retained(),
                accepted,
                committed: self.progress.committed(),
                suffix: self.bounded_suffix(journal, accepted, &[]),
                evidence: EvidenceKind::Planned,
                era_proof: self.era_proof(journal, current.era)?,
            },
        };
        let effects = vec![Effect::Send {
            to: from,
            era: current.era,
            message: answer,
        }];
        let candidate = self.identity_candidate()?;
        let plan = self.candidate_plan(candidate, JournalMutation::None, effects, kind, false);
        // The suffix fallback opens only when the successor era is not
        // yet recorded: the establishing operation is exactly what the
        // fetched range must supply.
        if self.progress.config().record(header.view.era).is_none() {
            if let Some(next) = accepted.next() {
                let (effect, fetch) = self.fetch(current, from, next);
                return Ok(plan.with_fetch(effect, fetch));
            }
        }
        Ok(plan)
    }

    /// A `DoViewChange` carrying PLANNED evidence (§8.7.7 step 4), at the
    /// pivot leader: a vote in the planned quorum, and nothing else — it
    /// never enters the ordinary attempt and never counts toward a
    /// `Role::Fence` quorum. The guards are total and named; the shape
    /// rules are the ordinary evidence path's.
    ///
    /// When every `qI` member has answered — the leader's own membership
    /// in `qI` is its casting vote — the leader publishes the transition
    /// as ONE step (step 5): `v'` joins current and retained, and
    /// `StartView(v')` goes to every member of config(e+1) (step 6) with
    /// the leader's history, its committed frontier, and the successor
    /// era's proof. The leader's own log is the installed history: no
    /// selection runs over the planned suffixes, because §8.7.6's
    /// intersection argument puts every committed entry of the era at the
    /// leader that proposed it or installed it. The evidence is
    /// transient: the machine clears with the transition, and any fence
    /// or ordinary install abandons it.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::replica) fn plan_planned_evidence(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        retained: ViewId,
        accepted: Slot,
        committed: Slot,
        suffix: &[LogEntry],
        era_proof: &EraProof,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let header = message.header;
        let current = self.progress.current();
        // Evidence answers a solicited machine naming this very view;
        // anything else is stale — an answer past the transition, an
        // answer to a machine a fence superseded, or a forgery.
        let Some(planned) = self.planned.clone() else {
            return self.drop_plan(
                Diagnostic::StaleEvidence {
                    got: header.view,
                    current,
                },
                kind,
            );
        };
        if !planned.solicited || header.view != planned.target {
            return self.drop_plan(
                Diagnostic::StaleEvidence {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        // Only a `qI` member's answer counts: the pivot validated at
        // proposal fixed the vote set, and the leader never counts itself
        // twice. Any other sender's evidence is not collected.
        if from == self.own || !planned.pivot.q_i.contains(&from) {
            return self.drop_plan(
                Diagnostic::EvidenceNotCollected {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        }
        // Shape: the header slot names the reported accepted frontier;
        // the frontiers are a legal chain; the suffix is a contiguous
        // ascending run ending at the frontier; the era proof matches the
        // CURRENT era's record — the era the responder still speaks from.
        let record = self
            .current_record()
            .ok_or(PlanRejection::Progress(ProgressError::EraSlotDiscipline))?;
        if header.slot != accepted
            || committed > accepted
            || !suffix_shape_ok(suffix, accepted)
            || !self.era_proof_ok(journal, record, era_proof)
        {
            return self.drop_plan(Diagnostic::MalformedViewChange, kind);
        }
        let mut planned = planned;
        // The first answer from a sender counts; a duplicate re-inserts
        // under the same key and changes nothing.
        planned.evidence.entry(from).or_insert(Evidence {
            retained,
            accepted,
            committed,
            suffix: suffix.to_vec(),
        });
        let quorum = planned
            .pivot
            .q_i
            .iter()
            .all(|member| *member == self.own || planned.evidence.contains_key(member));
        if !quorum {
            let candidate = self.identity_candidate()?;
            return Ok(self
                .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
                .with_bookkeeping(Bookkeeping {
                    planned: PlannedOverlapUpdate::Set(planned),
                    ..Bookkeeping::default()
                }));
        }
        // §8.7.7 step 5: the casting vote and the switch are ONE
        // published transition. `v'` was named at proposal; the frontiers
        // and the configuration history are exactly the published ones —
        // the era-(e+1) stream's proposals keep accumulating under the
        // new view.
        let own_accepted = self.progress.accepted();
        let own_committed = self.progress.committed();
        let candidate = self.install_candidate(
            planned.target,
            own_accepted,
            own_committed,
            self.progress.applied(),
            Arc::clone(self.progress.config()),
        )?;
        // §8.7.7 step 6: `StartView(v')` to every member of config(e+1),
        // routed under the successor era, with the bounded suffix
        // (§13.1) and the era proof (§8.7.8).
        let announcement = Message {
            header: Header {
                tag: Tag::StartView,
                view: planned.target,
                slot: own_accepted,
            },
            body: Body::StartView {
                suffix: self.bounded_suffix(journal, own_accepted, &[]),
                accepted: own_accepted,
                committed: own_committed,
                era_proof: self.era_proof(journal, planned.target.era)?,
            },
        };
        let effects = self
            .progress
            .config()
            .current()
            .config
            .order()
            .iter()
            .map(|member| member.node)
            .filter(|node| *node != self.own)
            .map(|to| Effect::Send {
                to,
                era: planned.target.era,
                message: announcement.clone(),
            })
            .collect();
        Ok(self
            .candidate_plan(candidate, JournalMutation::None, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                planned: PlannedOverlapUpdate::Clear,
                // The StartView broadcast is the transition's
                // announcement of the leader's life in the new view (S4).
                activity: Some(at),
                ..Bookkeeping::default()
            }))
    }
}
