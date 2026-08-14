//! State transfer: bringing a lagging, recovering or newly promoted replica current.
//!
//! Spec §4 (unavailable history), §11 (application boundary), §13.1 (bounded view-change
//! suffix). Decision W5.
//!
//! Transfer state is process-local and explicitly scoped (spec §15): it is evidence
//! gathered during one attempt, never protocol-visible state, so an abandoned transfer
//! leaves nothing behind that a later view could mistake for history.
//!
//! The core owns sequencing and completeness; the host owns sizing. There is no
//! `MAX_DATAGRAM` and no chunk-size constant here (decision W5). The core reports how
//! much it has and what it still needs, and the host decides how many bytes travel per
//! datagram — because a host that must fit a chunk into its own framing cannot be served
//! by a constant this crate guessed at compile time.
//!
//! When requested history is locally unavailable the host says so (§4). The core does not
//! ask why. It obtains an adequate state from another replica, or defers to a
//! host-specific application-state transfer facility, and a zero-weight member must
//! become adequately current here before a later committed `INCREMENT` grants it voting
//! authority.

use super::*;

impl<J: Journal, Q: QuorumStrategy> Replica<J, Q> {
    /// The fetch half of a gap ruling (§10, §13.1 step 5): a `GetState`
    /// for `from` onward under `view`, addressed to `to`, and the volatile
    /// cursor the answering `NewState` chunks install against. The header
    /// slot is the requester's accepted frontier — the slot the fetch
    /// resumes after (the per-tag table's Frontier role).
    pub(in crate::replica) fn fetch(
        &self,
        view: ViewId,
        to: NodeId,
        from: Slot,
    ) -> (Effect, TransferVolatile) {
        let message = Message {
            header: Header {
                tag: Tag::GetState,
                view,
                slot: from.prev().unwrap_or(Slot::FIRST),
            },
            body: Body::GetState { from },
        };
        let effect = Effect::Send {
            to,
            era: view.era,
            message,
        };
        let fetch = TransferVolatile {
            view,
            to,
            next: from,
        };
        (effect, fetch)
    }

    /// A qualified higher-view signal (§10): a normal-operation message
    /// from the legitimate primary of a view past the node's current one
    /// (the caller has verified the attribution). The message proves the
    /// node stale — but it is NOT installation evidence (§13.4's hint
    /// rule), so the node ceases lower-view participation by fencing into
    /// the message's view through the ordinary change pipeline, fetches
    /// the history it lacks from the sender, and installs only when the
    /// qualified evidence — the `StartView` — arrives.
    pub(in crate::replica) fn plan_higher_view_signal(
        &self,
        journal: &J::View,
        from: NodeId,
        view: ViewId,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let plan = self.enter_view_change(journal, view, BTreeSet::new(), at, kind)?;
        match self.progress.accepted().next() {
            // The fetch rides the same transition: one serialized interval
            // both fences and asks for the missing range.
            Some(next) => {
                let (effect, fetch) = self.fetch(view, from, next);
                Ok(plan.with_fetch(effect, fetch))
            }
            // The slot space is spent: fence only.
            None => Ok(plan),
        }
    }

    /// A `GetState` (§10, §13.1 step 5): stream the requested range back
    /// in budget-bounded chunks (W5). Only a `Normal` node in the
    /// requested view serves — the same rule as the §10 recovery
    /// solicitation: a fenced or recovering node's history is not yet
    /// proved current. The chunk is a contiguous ascending run from
    /// `from`, never a byte past the host's transport budget (W4); `more`
    /// tells the requester the frontier sits past the chunk, so a partial
    /// answer resumes from the cursor. A request the node cannot serve is
    /// a named drop, never a fault: the requester's fetch stays open and
    /// another answer closes the gap.
    pub(in crate::replica) fn plan_get_state(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        fetch_from: Slot,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
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
        let current = self.progress.current();
        let frontier = self.progress.accepted();
        if header.view != current || self.progress.status() != Status::Normal {
            return self.drop_plan(
                Diagnostic::TransferNotServed {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        }
        // Pack the chunk: contiguous ascending from the requested base,
        // stopping at the frontier, at a journal hole, or one entry before
        // the budget would overflow — the suffix never exceeds the budget
        // by a byte (W4). An empty chunk cannot advance the requester's
        // cursor (the base is ahead of the frontier, below retention, or
        // one entry over the budget), so it is not served at all.
        let mut entries: Vec<LogEntry> = Vec::new();
        let mut bytes = 0usize;
        let mut cursor = Some(fetch_from);
        while let Some(slot) = cursor {
            if slot > frontier {
                break;
            }
            let Some(entry) = journal.get(slot) else {
                break;
            };
            let Some(total) = bytes.checked_add(entry.packed_len()) else {
                break;
            };
            if total > self.knobs.view_change_budget {
                break;
            }
            entries.push(entry.clone());
            bytes = total;
            cursor = slot.next();
        }
        let Some(through) = entries.last().map(|entry| entry.slot) else {
            return self.drop_plan(
                Diagnostic::TransferNotServed {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        };
        let response = Message {
            header: Header {
                tag: Tag::NewState,
                view: current,
                slot: through,
            },
            body: Body::NewState {
                entries,
                through,
                committed: self.progress.committed(),
                more: through < frontier,
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

    /// A `NewState` chunk (§10, §13.1 step 5): history the node actively
    /// fetched, installed through the same suffix ruling as the
    /// view-change and recovery paths — contiguity against the local
    /// journal, no committed-slot conflict (the one deliberate fault),
    /// committed frontier monotone. Only a chunk answering the open fetch
    /// is protocol-qualified evidence at all; anything else — another
    /// view, another sender, no open fetch, a range the node already
    /// holds — is a named drop, never a fault. The chunk is HISTORY, not
    /// the completing ruling: a fenced node's committed frontier waits
    /// for its own path's qualified evidence (the `StartView`, the
    /// recovery ruling).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::replica) fn plan_new_state(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        entries: &[LogEntry],
        through: Slot,
        committed: Slot,
        more: bool,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
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
        // Shape first (§13.1): the header slot names the covered range's
        // end; the entries are a contiguous ascending run ending there; an
        // empty chunk never claims more remains. A FINAL chunk's committed
        // frontier never exceeds the covered range — an honest responder's
        // committed never exceeds its frontier, and `more` clear means the
        // chunk reached it. A partial chunk's committed legitimately runs
        // past the chunk; the install caps at the covered frontier.
        if header.slot != through
            || (!more && committed > through)
            || !suffix_shape_ok(entries, through)
            || (entries.is_empty() && more)
        {
            return self.drop_plan(Diagnostic::MalformedTransfer, kind);
        }
        // Only the open fetch qualifies the chunk (§10): the view and the
        // responder must be the ones the node asked.
        let current = self.progress.current();
        let Some(fetch) = self.transfer else {
            return self.drop_plan(
                Diagnostic::StaleTransfer {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        };
        if fetch.view != header.view || fetch.to != from {
            return self.drop_plan(
                Diagnostic::StaleTransfer {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        }
        // A committed frontier behind the node's own durable one is shaped
        // like knowledge the node already holds: stale, never believed.
        if committed < self.progress.committed() {
            return self.drop_plan(
                Diagnostic::StaleEvidence {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        let accepted = self.progress.accepted();
        // A range the node already holds whole: a duplicate. It closes
        // the fetch only when the responder has nothing more; a duplicate
        // that claims more is stale noise.
        if through <= accepted {
            if more {
                return self.drop_plan(
                    Diagnostic::StaleTransfer {
                        sender: from,
                        view: header.view,
                    },
                    kind,
                );
            }
            let candidate = self.identity_candidate()?;
            return Ok(self
                .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
                .with_bookkeeping(Bookkeeping {
                    transfer: TransferUpdate::Clear,
                    ..Bookkeeping::default()
                }));
        }
        let mutation = match self.check_suffix(journal, entries, through, committed) {
            SuffixCheck::Install(mutation) => mutation,
            // A reordered chunk: it cannot be verified against the local
            // journal until its prefix arrives. Named, kept waiting — the
            // fetch stays open and the in-flight chunks close it.
            SuffixCheck::Gap { expected, got } => {
                return self.drop_plan(Diagnostic::GapDetected { expected, got }, kind);
            }
            // A chunk contradicting a slot this node durably committed is
            // the same quorum-obligation violation as a conflicting
            // StartView suffix (§9.2): declare the breach.
            SuffixCheck::Conflict => {
                let candidate = self.identity_candidate()?;
                return Ok(self
                    .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
                    .with_fault_declared(Fault::IllegalTransition));
            }
        };
        // The install moves the accepted frontier only; the committed
        // frontier moves on the current view's own transfer — while the
        // node is fenced, the completing ruling owns it.
        let new_accepted = accepted.max(through);
        let current_view_transfer =
            header.view == current && self.progress.status() == Status::Normal;
        let new_committed = if current_view_transfer {
            self.progress.committed().max(committed.min(new_accepted))
        } else {
            self.progress.committed()
        };
        let mut effects = if new_committed > self.progress.committed() {
            self.apply_effects_merged(journal, entries, self.progress.committed(), new_committed)?
        } else {
            Vec::new()
        };
        let candidate = self.candidate_with(
            self.progress.status(),
            new_accepted,
            new_committed,
            self.applied_walk(journal, entries, self.progress.applied(), new_committed)?,
        )?;
        // The cursor: a partial answer resumes with a fresh `GetState`
        // one past the newly installed frontier; the final chunk closes
        // the fetch.
        let transfer = if more {
            match new_accepted.next() {
                Some(next) => {
                    let (effect, fetch) = self.fetch(header.view, from, next);
                    effects.push(effect);
                    TransferUpdate::Set(fetch)
                }
                // The slot space is spent: nothing more can be fetched.
                None => TransferUpdate::Clear,
            }
        } else {
            TransferUpdate::Clear
        };
        Ok(self
            .candidate_plan(candidate, mutation, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                transfer,
                ..Bookkeeping::default()
            }))
    }
}
