//! State transfer: bringing a lagging, reincarnating or newly promoted replica current.
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
//! datagram, because a host that must fit a chunk into its own framing cannot be served
//! by a constant this crate guessed at compile time.
//!
//! When requested history is locally unavailable the host says so (§4). The core does not
//! ask why. It obtains an adequate state from another replica, or defers to a
//! host-specific application-state transfer facility, and a zero-weight member must
//! become adequately current here before a later committed `INCREMENT` grants it voting
//! authority.

use super::reconfiguration::CommitFold;
use super::*;
#[allow(unused_imports)]
use crate::trace;

impl<J: Journal, Q: QuorumStrategy> Replica<J, Q> {
    /// The fetch half of a gap ruling (§10, §13.1 step 5): a `GetState`
    /// for `from` onward under `view`, addressed to `to`, and the volatile
    /// cursor the answering `NewState` chunks install against. The header
    /// slot is the requester's accepted frontier, the slot the fetch
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
                slot: from.prev().unwrap_or(Slot::NONE),
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
    /// node stale, but it is NOT installation evidence (§13.4's hint
    /// rule), so the node ceases lower-view participation by fencing into
    /// the message's view through the ordinary change pipeline, fetches
    /// the history it lacks from the sender, and installs only when the
    /// qualified evidence, the `StartView`, arrives.
    pub(in crate::replica) fn plan_higher_view_signal(
        &self,
        journal: &J::View,
        from: NodeId,
        view: ViewId,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
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
    /// in budget-bounded chunks (W5). Serving is read-only retransmission
    /// of durable journal content: it never mutates the responder and
    /// cannot alter committed state, so a node is never fenced with
    /// respect to SERVING, fencing governs participation, not serving.
    /// The serving gate is therefore the cluster-legality gate alone (the
    /// sender is a member of the requested era's configuration or, the
    /// learner acquisition rule (`docs/uvrr-reincarnation.md` §10), of
    /// the responder's current committed configuration, a weight-0
    /// learner included) plus the statuses whose journal is not servable:
    /// `Restarting` or `Joining` (the node's history is not yet proved
    /// current) and `Replaying` (the journal is
    /// mid-install, structurally inconsistent). A fenced `ViewChange`
    /// node serves exactly like a `Normal` one, and the request's view is
    /// a correlation token (VRR-2012's §10 recovery nonce), not a serving
    /// condition: the response header echoes it so the recipient's
    /// open-fetch qualification, the real gate, can match the answer to
    /// the fetch it opened. When the requested era is outside the
    /// responder's retention window the requested-era disjunct is simply
    /// unavailable, the window moved past the era a boot-fenced learner
    /// fetches under, and the current-configuration disjunct decides
    /// alone; the chunk is the same verified history either way. The chunk
    /// is a contiguous ascending run from `from`, never a byte past the
    /// host's transport budget (W4); `more` tells the requester the
    /// frontier sits past the chunk, so a partial answer resumes from the
    /// cursor. A request the node cannot serve is a named drop, never a
    /// fault: the requester's fetch stays open and another answer closes
    /// the gap.
    pub(in crate::replica) fn plan_get_state(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        fetch_from: Slot,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let header = message.header;
        // The cluster-legality gate, with the learner acquisition rule
        // (`docs/uvrr-reincarnation.md` §10): a sender that is not a member
        // of the REQUESTED era's configuration may still be served when it
        // is a member of the responder's current committed configuration,
        // any weight, a weight-0 learner included. A learner behind the
        // commit frontier can only name the eras its own table holds, and
        // the era that admitted it is by definition not one of them; R6
        // obliges the leader to keep learners caught up, serving is
        // read-only retransmission of durable journal content, and the
        // responder's current configuration is exactly the membership the
        // §6 ingress check admits messages FROM. A node outside the current
        // configuration, a foreign identity, a superseded old identity,
        // is refused as before.
        let served = self
            .progress
            .config()
            .record(header.view.era)
            .is_some_and(|record| record.config.weight_of(from).is_some())
            || self
                .progress
                .config()
                .current()
                .config
                .weight_of(from)
                .is_some();
        if !served {
            return self.drop_plan(Diagnostic::UnknownSender { sender: from }, kind);
        }
        let frontier = self.progress.accepted();
        if matches!(
            self.progress.status(),
            Status::Restarting | Status::Joining | Status::Replaying
        ) {
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
        // the budget would overflow, the suffix never exceeds the budget
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
        // The response header echoes the REQUEST's view: a correlation
        // token (VRR-2012's §10 recovery nonce), never the responder's current
        // view, the recipient's open-fetch qualification matches the
        // answer against the fetch it opened, and the send routes in the
        // request's era so it reaches the requester under the same
        // membership the request arrived under.
        let response = Message {
            header: Header {
                tag: Tag::NewState,
                view: header.view,
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
            era: header.view.era,
            message: response,
        }];
        let candidate = self.identity_candidate()?;
        Ok(self.candidate_plan(candidate, JournalMutation::None, effects, kind, false))
    }

    /// A `NewState` chunk (§10, §13.1 step 5): history the node actively
    /// fetched, installed through the same suffix ruling as the
    /// view-change and state-transfer paths, contiguity against the local
    /// journal, no committed-slot conflict (the one deliberate fault),
    /// committed frontier monotone. Only a chunk answering the open fetch
    /// is protocol-qualified evidence at all; anything else, another
    /// view, another sender, no open fetch, a range the node already
    /// holds, is a named drop, never a fault. The chunk is HISTORY, not
    /// the completing ruling: a fenced node's committed frontier waits
    /// for its own path's qualified evidence (the `StartView`), with
    /// the §10 learner acquisition exception: the boot-fenced node's own
    /// open fetch is its qualified evidence, so a fenced entry node
    /// (`Restarting` or `Joining`) at its boot fence takes the chunk's committed frontier and folds
    /// what it covers (the ruling below).
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
    ) -> Result<PlannedTransition, PlanRefusal> {
        let header = message.header;
        trace!(
            "NEW_STATE from={:?} view=({:?},{:?}) through={:?} committed={:?} more={} entries={} | self: current=({:?},{:?}) retained=({:?},{:?}) accepted={:?} committed={:?} status={:?} table_current_era={:?}",
            from,
            header.view.era,
            header.view.view,
            through,
            committed,
            more,
            entries.len(),
            self.progress.current().era,
            self.progress.current().view,
            self.progress.retained().era,
            self.progress.retained().view,
            self.progress.accepted(),
            self.progress.committed(),
            self.progress.status(),
            self.progress.config().current().era
        );
        let Some(record) = self.progress.config().record(header.view.era) else {
            trace!(
                "NEW_STATE drop: UnevaluableEra era={:?} (table holds no record)",
                header.view.era
            );
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
        // frontier never exceeds the covered range, an honest responder's
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
        // The chunk qualifies as protocol evidence through one of two
        // routes, both fenced (§10): the node's own open fetch, the view
        // and the responder must be the ones the node asked, or, for a
        // node still at its boot fence that opened NO fetch, the leader's
        // missed-range push (`docs/uvrr-reincarnation.md` §7): the
        // announcement named the node's past-life frontiers and the
        // leader's ack pushed what it missed, under the view the
        // ANNOUNCEMENT carried, the one the node already holds. The
        // pushed chunk passes the same verification the fetched chunk
        // does: the suffix ruling below verifies it against the local
        // journal before anything folds. A boot-fenced node that HAS an
        // open fetch keeps the fetch route alone: a push whose sender is
        // not the fetch's responder is not qualified evidence for it.
        let current = self.progress.current();
        let boot_fence = matches!(self.progress.status(), Status::Restarting | Status::Joining)
            && current == self.progress.retained();
        let qualified = match self.transfer {
            Some(fetch) => fetch.view == header.view && fetch.to == from,
            None => boot_fence && header.view == current,
        };
        trace!(
            "NEW_STATE qualified={} boot_fence={} fetch_view={:?} header_view=({:?},{:?})",
            qualified,
            boot_fence,
            self.transfer.map(|f| f.view),
            header.view.era,
            header.view.view
        );
        if !qualified {
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
        let mutation = match self.check_suffix(journal, entries, through, committed, Slot::NONE) {
            SuffixCheck::Install(mutation) => mutation,
            // A reordered chunk: it cannot be verified against the local
            // journal until its prefix arrives. Named, kept waiting, the
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
        // frontier moves on the current view's own transfer, while the
        // node is fenced, the completing ruling owns it. The one §10
        // exception is the boot-fenced node's OWN acquisition: a node
        // still at its boot fence (`Restarting` or `Joining` with
        // `current == retained`, it has adopted nothing) that opened this
        // fetch itself may take the chunk's committed frontier and fold
        // the system operations it covers. That is the speculative
        // learner acquisition `docs/uvrr-reincarnation.md` §10 states: the
        // learner acquires state by streaming while never voting, the
        // chunk is history it actively fetched, every entry verified
        // against its local journal by the suffix ruling above, and the
        // fold input is the same verified history every other fold path
        // reads. The node stays fenced: it adopts no view, its votes are
        // not counted (the membership check at the counting site), and it
        // serves nothing (`Restarting` or `Joining`). A `ViewChange`-fenced
        // node is NOT covered: its attempt's completing ruling owns the commit
        // frontier, unchanged.
        let new_accepted = accepted.max(through);
        let current_view_transfer =
            header.view == current && self.progress.status() == Status::Normal;
        let boot_acquisition = header.view == current
            && matches!(self.progress.status(), Status::Restarting | Status::Joining)
            && current == self.progress.retained();
        trace!(
            "NEW_STATE current_view_transfer={} boot_acquisition={} (header==current: {}, status: {:?})",
            current_view_transfer,
            boot_acquisition,
            header.view == current,
            self.progress.status()
        );
        // The §10 learner acquisition's take: the answering chunk's
        // committed frontier, CAPPED by a retained gap-ruled offer's own
        // committed frontier (§13.1 step 5). The offer is the node's own
        // knowledge of a committed selection; its staleness gate refuses
        // an offer that claims less than the node durably holds, so an
        // acquisition that runs past it, live, the responder keeps
        // committing while the fetch is in flight, would strand the
        // retained ruling forever, and with it the fenced learner: the
        // `StartView` is one-shot and no later route installs an
        // already-entered view. Folding what the offer needs, and no
        // further, keeps the offer installable; the era's stream (R6
        // reaches learners) carries the tail. The cap never undershoots
        // the fold: the offer's selection covers everything committed at
        // its view, its own era's establishing operation included. An
        // open fetch with no retained offer, and the ordinary same-view
        // transfer at a `Normal` node, take the chunk's frontier whole.
        let boot_cap = if boot_acquisition {
            match self.stalled.as_ref().map(|offer| &offer.message.body) {
                Some(Body::StartView { committed, .. }) => Some(*committed),
                _ => None,
            }
        } else {
            None
        };
        let take = match boot_cap {
            Some(cap) => committed.min(cap),
            None => committed,
        };
        let new_committed = if current_view_transfer || boot_acquisition {
            self.progress.committed().max(take.min(new_accepted))
        } else {
            self.progress.committed()
        };
        trace!(
            "NEW_STATE take={:?} (cap={:?}) new_accepted={:?} new_committed={:?}",
            take, boot_cap, new_accepted, new_committed
        );
        // §8.7.1: the committed frontier moved, fold the system
        // operations the advance newly covers. A fold refusal here is a
        // chunk that contradicts committed history the configuration
        // cannot hold: the same breach as the conflict arm above (§9.2).
        //
        // The boot acquisition's fold may reach eras the boot view has
        // not entered, so it folds only as far as §8.7.3's era window
        // (W1) lets the boot view carry: the fold stops before the table
        // would establish an era more than one past `era(current)`, the
        // committed and accepted frontiers stop at the fold's frontier,
        // and the durable view's era walks into the era the folded table
        // established (the view number is preserved, no view change
        // runs here; the node stays fenced and adopts no history). The
        // chunk's tail past the fold is re-fetched by the acquisition's
        // next round (the cursor below, or the re-retain's fetch once the
        // stalled ruling re-runs).
        let fold = if boot_acquisition {
            self.fold_committed_windowed(
                journal,
                entries,
                self.progress.committed(),
                new_committed,
                current.era.next(),
            )
            .map(|(table, covered, stopped)| (table, Some((covered, stopped))))
        } else {
            self.fold_committed(journal, entries, self.progress.committed(), new_committed)
                .map(|table| (table, None))
        };
        let (config, folded_state) = match fold {
            Ok(pair) => {
                trace!(
                    "NEW_STATE fold ok: table_current_era={:?} folded_state={:?}",
                    pair.0.current().era,
                    pair.1
                );
                pair
            }
            Err(CommitFold::Unavailable(slot)) => {
                return Err(PlanRefusal::JournalEntryUnavailable { slot });
            }
            // The chunk's committed frontier would split an
            // establishing batch (`docs/uvrr-fuse.md`): the chunk is
            // malformed, dropped by name, the fetch cursor resumes
            // the range.
            Err(CommitFold::SplitBatch) => {
                return self.drop_plan(Diagnostic::FuseRefusal, kind);
            }
            Err(CommitFold::Breach { .. }) => return self.breach_plan(kind),
        };
        let (new_committed, window_stopped) = folded_state
            .map_or((new_committed, false), |(covered, stopped)| {
                (covered, stopped)
            });
        trace!(
            "NEW_STATE after window: new_committed={:?} window_stopped={}",
            new_committed, window_stopped
        );
        let new_accepted = if boot_acquisition {
            accepted.max(new_committed)
        } else {
            new_accepted
        };
        // The journal records only what the candidate's frontiers carry:
        // a window-capped fold's tail is re-fetched by the acquisition's
        // next round, so the install is truncated to the covered prefix.
        let truncated = boot_acquisition && new_committed < through;
        trace!("NEW_STATE truncated={}", truncated);
        let journaled: Vec<LogEntry> = if truncated {
            entries
                .iter()
                .filter(|entry| entry.slot <= new_committed)
                .cloned()
                .collect()
        } else {
            Vec::new()
        };
        let overlay: &[LogEntry] = if journaled.is_empty() {
            entries
        } else {
            &journaled
        };
        let mutation = if truncated {
            let covered: Vec<LogEntry> = entries
                .iter()
                .filter(|entry| entry.slot <= new_committed)
                .cloned()
                .collect();
            if covered.is_empty() {
                // The fold covered nothing from this chunk: the tail past
                // the era window is re-fetched by the acquisition's next
                // round, and there is no covered prefix to journal.
                JournalMutation::None
            } else {
                match self.check_suffix(journal, &covered, new_committed, committed, Slot::NONE) {
                    SuffixCheck::Install(mutation) => mutation,
                    SuffixCheck::Gap { expected, got } => {
                        return self.drop_plan(Diagnostic::GapDetected { expected, got }, kind);
                    }
                    SuffixCheck::Conflict => {
                        let candidate = self.identity_candidate()?;
                        return Ok(self
                            .candidate_plan(
                                candidate,
                                JournalMutation::None,
                                Vec::new(),
                                kind,
                                false,
                            )
                            .with_fault_declared(Fault::IllegalTransition));
                    }
                }
            }
        } else {
            mutation
        };
        let mut effects = if new_committed > self.progress.committed() {
            self.apply_effects_merged(
                journal,
                &entries
                    .iter()
                    .filter(|entry| entry.slot <= new_committed)
                    .cloned()
                    .collect::<Vec<_>>(),
                self.progress.committed(),
                new_committed,
            )?
        } else {
            Vec::new()
        };
        let walked = boot_acquisition && window_stopped && config.current().era > current.era;
        trace!(
            "NEW_STATE walked={} (table_era={:?} current_era={:?})",
            walked,
            config.current().era,
            current.era
        );
        // The walk's target, when the fold walked the view into the era
        // the folded table established. The acquisition's next round
        // rides it: the re-issued fetch (below) carries the walked view,
        // whose era record the folded table holds, a fetch that stayed
        // on the boot view would name an era the table's two-era
        // retention window has walked past, and the answering chunk
        // would be unevaluable at the very guard that reads it.
        let walked_view = if walked {
            Some(
                current
                    .next_in_next_era()
                    .ok_or(PlanRefusal::Progress(ProgressError::ViewSuccessor))?,
            )
        } else {
            None
        };
        let candidate = if let Some(target) = walked_view {
            let revision = self
                .progress
                .revision()
                .checked_add(1)
                .ok_or(PlanRefusal::Progress(ProgressError::RevisionExhausted))?;
            Progress::reconstitute(
                target,
                target,
                self.progress.status(),
                new_accepted,
                new_committed,
                self.applied_walk(journal, overlay, self.progress.applied(), new_committed)?,
                self.progress.checkpoint(),
                revision,
                Arc::clone(&config),
                None,
            )
            .map_err(PlanRefusal::Progress)?
        } else {
            self.candidate_with(
                self.progress.status(),
                new_accepted,
                new_committed,
                self.applied_walk(journal, overlay, self.progress.applied(), new_committed)?,
                Arc::clone(&config),
            )?
        };
        // The cursor: a partial answer resumes with a fresh `GetState`
        // one past the newly installed frontier; the final chunk closes
        // the fetch. The boot acquisition's window-capped fold is partial
        // by construction when it stopped short of the chunk's own
        // coverage (`new_committed < through`): the fetch stays open
        // either way, so the next round folds the tail, and rides the
        // view the candidate now carries (the walk's target, when the
        // fold walked) so the answering chunk's era record is one the
        // folded table still holds.
        let fetch_view = if boot_acquisition {
            walked_view.unwrap_or(current)
        } else {
            header.view
        };
        trace!(
            "NEW_STATE cursor: more={} truncated={} fetch_view={:?} next={:?}",
            more,
            truncated,
            fetch_view,
            new_accepted.next()
        );
        let transfer = if more || walked || (truncated && new_accepted > accepted) {
            match new_accepted.next() {
                Some(next) => {
                    let (effect, fetch) = self.fetch(fetch_view, from, next);
                    effects.push(effect);
                    TransferUpdate::Set(fetch)
                }
                // The slot space is spent: nothing more can be fetched.
                None => TransferUpdate::Clear,
            }
        } else {
            // A capped answer that neither walked the view nor advanced
            // the cursor would return the identical chunk forever: the
            // acquisition's needs through the cap are already journalled,
            // the fetch closes, and the retained offer's own revival
            // routes carry the install and the tail.
            TransferUpdate::Clear
        };
        trace!(
            "NEW_STATE candidate: current=({:?},{:?}) retained=({:?},{:?}) accepted={:?} committed={:?} status={:?} table_era={:?}",
            candidate.current().era,
            candidate.current().view,
            candidate.retained().era,
            candidate.retained().view,
            candidate.accepted(),
            candidate.committed(),
            candidate.status(),
            candidate.config().current().era
        );
        Ok(self
            .candidate_plan(candidate, mutation, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                transfer,
                ..Bookkeeping::default()
            }))
    }
}
