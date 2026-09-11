//! Reincarnation: the Crash-Stop-Self-Evict production logic
//! (`docs/uvrr-reincarnation.md`).
//!
//! Three pieces live here, each the code of one section of that document:
//!
//! * **The forced weight sequence** (§5; rules §6): given a committed
//!   configuration and the announced `(old, new)` pair, the remaining
//!   eras the leader must commit — the §6 table computed for the old
//!   identity's observed state, each era ONE `Batch` establishing
//!   operation: the old identity's weight driven to 0 by unit decrements
//!   (the subtract-one rule, always era-safe), the crossing batch
//!   `[Decrement(old), Join(new)]`, the promotion batch
//!   `[Increment(new), Leave(old)]` (a zero-weight `Leave` changes no
//!   quorum family), and for an old identity already at 0 or already
//!   evicted the join/promotion form the table names. Steps the observed
//!   eras already committed are not re-run: the announcement is idempotent
//!   over the current configuration, which is what makes a leader crash
//!   mid-sequence harmless (§8) — whichever safe era the crash lands in,
//!   the next leader recomputes the remaining eras from the configuration
//!   that era committed.
//! * **The leader machine**: the pair the announcement armed, and the
//!   tick-driven continuation that proposes the next step through the
//!   ordinary reconfiguration pipeline — one establishing operation at a
//!   time, each commit a distinct era (§5), every step gated by the same
//!   closed gates any host-proposed operation passes.
//! * **The marker transition machine** (§5.1 of `docs/vrr-durability-model.md`,
//!   the boot decision of `docs/uvrr-reincarnation.md` §1): the pure Rust twin
//!   of the vendored TigerBeetle store (`zig/uvrr/store.zig`). The four
//!   superblock markers are an ordered transition system —
//!   `Running ──stop──> Stopping ──drain──> Stopped ──boot, 2-of-4──>
//!   Restarting`, and `Running ──crash──> (markers unchanged) ──boot, no
//!   2-of-4 Stopped──> Joining` — each state naming the transition that must
//!   have completed for it to exist. uVRR performs **no disk flushes on the
//!   normal path**: the stop command writes `Stopping` 4x
//!   ([`SuperblockCopies::begin_stop`]), the HOST drains — flushes WALs and
//!   grids — strictly between the two marker writes, and
//!   [`SuperblockCopies::finish_stop`] writes `Stopped` 4x; the marker order
//!   is the drain's proof, so a `Stopped` copy vouches for the WAL under it.
//!   At boot the quorum read answers one question — *did the transition
//!   complete?* — with 2-of-4 copies holding `Stopped` (the twin's open
//!   threshold), the working quorum resolving the identity
//!   higher-identity-wins INSIDE the quorum: a stopped quorum continues under
//!   the same identity and writes `Restarting` 4x — a member with complete
//!   state, no amnesia, ticking the full protocol; no stopped quorum — a
//!   crash, a torn marker set, or death mid-join — means the identity is
//!   dead: the node bumps it and resurrects, writing `Joining` 4x — not a
//!   member, no vote, no view change. No `Started` state is written: no
//!   safety logic looks for `Started`, it looks for `Stopped` — the extra
//!   superblock write buys no safety and is elided. The marker writes are
//!   durable-on-write (flushed), the only disk traffic outside the stop
//!   path.

use crate::configuration::SystemOperation;
use crate::effects::Effect;
use crate::ids::{NodeId, Slot};
use crate::journal::{JournalView, Payload};
use crate::message::{Body, Message};
use crate::observe::Diagnostic;
use crate::progress::Status;
use crate::wire::{Header, Tag};

use super::{
    InputKind, Journal, JournalMutation, PlanRefusal, PlannedTransition, ProgressError,
    QuorumStrategy, ReincarnationUpdate, Replica,
};

/// The leader's armed reincarnation machine (§5): the announced pair.
///
/// Volatile by design: a leader crash discards it, and the bumped node
/// re-announces to the stable leader (§8), whose first act is to recompute
/// the remaining steps from the configuration the observed intermediate era
/// committed. The steps are never stored — the configuration history is
/// their only authority.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::replica) struct ForcedSequence {
    /// The identity being evicted.
    pub old: NodeId,
    /// The identity joining at weight 0.
    pub new: NodeId,
}

impl<J: Journal, Q: QuorumStrategy> Replica<J, Q> {
    /// A `Reincarnation(old, new)` announcement at the leader (§4): arm the
    /// machine and propose the first remaining forced step.
    ///
    /// Only the leader of its current view drives the sequence (§5); only
    /// the NEW identity may claim the pair — the announcement's sender is
    /// the transport-attributed restarted node, and a message whose sender
    /// is not the identity it names is a forgery, dropped. The announcement
    /// is idempotent: steps the observed eras already committed are not
    /// re-run, and a complete sequence clears the machine. While a step is
    /// in flight the machine arms without proposing; the continuation
    /// re-drives it on a tick.
    pub(in crate::replica) fn plan_reincarnation(
        &self,
        journal: &J::View,
        from: NodeId,
        old: NodeId,
        new: NodeId,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let current = self.progress.current();
        let is_leader =
            self.progress.status() == Status::Normal && self.primary_of(current) == Some(self.own);
        if !is_leader || from != new || old == new {
            return self.drop_plan(
                Diagnostic::ReincarnationRefused {
                    sender: from,
                    view: current,
                },
                kind,
            );
        }
        let machine = ForcedSequence { old, new };
        let record = self
            .current_record()
            .ok_or(ProgressError::EraSlotDiscipline)
            .map_err(PlanRefusal::Progress)?;
        match forced_steps(&record.config, old, new).into_iter().next() {
            None => Ok(self
                .drop_plan(Diagnostic::None, kind)?
                .with_reincarnation(ReincarnationUpdate::Clear)),
            Some(first) => {
                if self.forced_step_plannable(journal, &first) {
                    Ok(self
                        .plan_reconfigure(journal, &first, &None)?
                        .with_reincarnation(ReincarnationUpdate::Set(machine)))
                } else {
                    // A step is in flight or an era transition awaits the
                    // view change into it: arm, continue on a tick.
                    Ok(self
                        .drop_plan(Diagnostic::None, kind)?
                        .with_reincarnation(ReincarnationUpdate::Set(machine)))
                }
            }
        }
    }

    /// The tick-driven continuation (§5, §8): the armed leader re-drives
    /// the sequence. `None` leaves the tick to the ordinary machinery —
    /// the machine sits armed and inert until the conditions return.
    pub(in crate::replica) fn plan_forced_continuation(
        &self,
        journal: &J::View,
    ) -> Option<Result<PlannedTransition, PlanRefusal>> {
        let machine = self.reincarnation?;
        let current = self.progress.current();
        let is_leader =
            self.progress.status() == Status::Normal && self.primary_of(current) == Some(self.own);
        if !is_leader {
            return None;
        }
        match self.next_forced_step() {
            None => {
                let candidate = self.identity_candidate().ok()?;
                Some(Ok(self
                    .candidate_plan(
                        candidate,
                        JournalMutation::None,
                        Vec::new(),
                        InputKind::Tick,
                        false,
                    )
                    .with_reincarnation(ReincarnationUpdate::Clear)))
            }
            Some(first) => {
                if self.forced_step_plannable(journal, &first) {
                    Some(
                        self.plan_reconfigure(journal, &first, &None)
                            .map(|plan| plan.with_reincarnation(ReincarnationUpdate::Set(machine))),
                    )
                } else {
                    None
                }
            }
        }
    }

    /// The first forced step the current committed configuration still
    /// needs, or `None` when the announced sequence is complete.
    fn next_forced_step(&self) -> Option<SystemOperation> {
        let machine = self.reincarnation?;
        let record = self.current_record()?;
        forced_steps(&record.config, machine.old, machine.new)
            .into_iter()
            .next()
    }

    /// Whether the reconfiguration pipeline would accept `op` right now:
    /// the gates of [`plan_reconfigure`] that are cheap to precheck, so a
    /// continuation tick stays a total identity transition when the
    /// pipeline is not ready (one era transition at a time; one
    /// establishing operation at a time; the fold's own preconditions).
    fn forced_step_plannable(&self, journal: &J::View, op: &SystemOperation) -> bool {
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

    /// The bumped node's announcement (§4): `Input::Reincarnate` reports
    /// the pair and the node sends `Reincarnation(old, new)` to the current
    /// primary — the one message a non-member is entitled to send (§6's
    /// ingress rule exempts it; it is the entry ticket).
    ///
    /// A member already voting at weight ≥ 1 has nothing to announce — a
    /// clean life continues (§2's clean path); the transition is the
    /// identity. Everything else — a fresh identity, or a weight-0 learner
    /// re-announcing to a stable leader (§8) — sends.
    pub(in crate::replica) fn plan_reincarnate(
        &self,
        _journal: &J::View,
        old: NodeId,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRefusal> {
        let current = self.progress.current();
        let member = self
            .current_record()
            .ok_or(ProgressError::EraSlotDiscipline)
            .map_err(PlanRefusal::Progress)?
            .config
            .weight_of(self.own);
        if old == self.own || member.is_some_and(|weight| weight.0 >= 1) {
            return self.drop_plan(Diagnostic::None, kind);
        }
        // Self-announcement: the bumped node IS the current primary. The
        // message would come back to itself; the leader-side handler runs
        // directly.
        if self.progress.status() == Status::Normal && self.primary_of(current) == Some(self.own) {
            return self.plan_reincarnation(_journal, self.own, old, self.own, kind);
        }
        // The announcement goes to every member of the configuration the
        // node can still name (§4: the leader acts on it; the backups drop
        // it by name). A bumped node's own view may be stale — the leader
        // it last knew may have died (§8) — so the announcement is
        // addressed cluster-wide and discovery is the recipients', not the
        // announcer's: no leader election is invented here.
        let record = self
            .current_record()
            .ok_or(ProgressError::EraSlotDiscipline)
            .map_err(PlanRefusal::Progress)?;
        let members: Vec<NodeId> = record
            .config
            .order()
            .iter()
            .map(|member| member.node)
            .filter(|node| *node != self.own)
            .collect();
        let effects = members
            .into_iter()
            .map(|to| Effect::Send {
                to,
                era: current.era,
                message: Message {
                    header: Header {
                        tag: Tag::Reincarnation,
                        view: current,
                        slot: Slot::NONE,
                    },
                    body: Body::Reincarnation { old, new: self.own },
                },
            })
            .collect();
        let candidate = self.identity_candidate()?;
        Ok(self.candidate_plan(candidate, JournalMutation::None, effects, kind, false))
    }
}

/// The forced weight sequence the leader must still commit (§5; rules §6), read
/// from the CURRENT committed configuration: each element is ONE era's
/// establishing operation — a [`SystemOperation::Batch`] — and the sequence is
/// the §6 table computed for the old identity's observed state:
///
/// | Old identity's state | Remaining eras |
/// |---|---|
/// | weight `w >= 2` | `w−1` solitary `Decrement` eras, then `Batch([Decrement(old), Join(new)])`, then `Batch([Increment(new), Leave(old)])` |
/// | weight `1` | `Batch([Decrement(old), Join(new)])`, then `Batch([Increment(new), Leave(old)])` |
/// | weight `0` | `Batch([Join(new), Leave(old)])`, then `Batch([Increment(new)])` |
/// | already evicted | `Batch([Join(new)])`, then `Batch([Increment(new)])` |
///
/// The new identity joins at weight 0 in the old identity's succession position
/// (appended when the old identity is already gone — a leader-crash intermediate
/// era that removed before adding), then is promoted. Each era is a unit batch or
/// a zero-mass batch under the era rule R14 — the subtract-one / add-one /
/// weight-0-join steps move at most one unit of per-node mass — so every
/// intermediate era is quorum-safe (§6's invariant; the checked weighted-overlap
/// result covers each step).
///
/// Steps the configuration has already committed are absent: a leader crash
/// mid-sequence leaves a legal starting point whose recomputation continues the
/// sequence exactly where it stopped (§8; rules §6 — recomputation is idempotent
/// over the CURRENT configuration, which is what makes a leader dying at any point
/// of the sequence harmless).
#[must_use]
pub fn forced_steps(
    config: &crate::configuration::Configuration,
    old: NodeId,
    new: NodeId,
) -> Vec<SystemOperation> {
    let mut eras = Vec::new();
    if old == new {
        return eras;
    }
    let old_weight = config.weight_of(old).map(|weight| weight.0);
    let new_weight = config.weight_of(new).map(|weight| weight.0);
    let old_position = config.index_of(old);
    // The old identity's succession position while it holds one; appended once it
    // is gone. The position is read BEFORE any op of the sequence exists: the
    // sequence is computed, not stored.
    let append_position = config.len();

    let mut new_weight_now = new_weight;
    if let Some(weight) = old_weight {
        // The subtract-one rule, always era-safe: each solitary `Decrement` era
        // is a unit batch moving exactly one unit of mass (R14).
        for _ in 0..weight.saturating_sub(1) {
            eras.push(SystemOperation::Batch(vec![SystemOperation::Decrement(
                old,
            )]));
        }
        if weight >= 1 {
            // The crossing era: old reaches weight 0 and the new identity joins
            // as a learner in the old succession position — mass exactly 1,
            // quorum-safe on its own. The join is present only when the new
            // identity is not already a member (a recompute that lands after a
            // prior era's join must not re-join).
            let mut crossing = vec![SystemOperation::Decrement(old)];
            if new_weight_now.is_none() {
                crossing.push(SystemOperation::Join {
                    node: new,
                    position: old_position.unwrap_or(append_position),
                });
                // The crossing batch's own join satisfies the promotion
                // precondition below: the recompute reads the CURRENT
                // configuration, and this sequence is what it will land on.
                new_weight_now = Some(0);
            }
            eras.push(SystemOperation::Batch(crossing));
        }
    }

    // The old identity is now at weight 0 (or was never a member). The remaining
    // eras are the new identity's promotion and the old identity's zero-weight
    // departure, exactly as the observed state names them.
    if old_weight.is_some() {
        match new_weight_now {
            None => {
                eras.push(SystemOperation::Batch(vec![
                    SystemOperation::Join {
                        node: new,
                        position: old_position.unwrap_or(append_position),
                    },
                    SystemOperation::Leave(old),
                ]));
                eras.push(SystemOperation::Batch(vec![SystemOperation::Increment(
                    new,
                )]));
            }
            Some(0) => {
                // A recompute mid-sequence: the join already committed, so the
                // remaining eras are the weight-1 table's second era — promote,
                // then the zero-weight departure (mass 1).
                eras.push(SystemOperation::Batch(vec![
                    SystemOperation::Increment(new),
                    SystemOperation::Leave(old),
                ]));
            }
            Some(_) => {
                // The new identity already votes; only the zero-weight departure
                // remains, and a zero-mass batch is legal (R14).
                eras.push(SystemOperation::Batch(vec![SystemOperation::Leave(old)]));
            }
        }
    } else {
        match new_weight {
            None => {
                // The old identity is already evicted: join at weight 0 (a
                // zero-mass era), then promote (a unit era) — the table's last
                // row.
                eras.push(SystemOperation::Batch(vec![SystemOperation::Join {
                    node: new,
                    position: append_position,
                }]));
                eras.push(SystemOperation::Batch(vec![SystemOperation::Increment(
                    new,
                )]));
            }
            Some(0) => {
                eras.push(SystemOperation::Batch(vec![SystemOperation::Increment(
                    new,
                )]));
            }
            // The rejoin is complete: the old identity is gone, the new identity
            // votes.
            Some(_) => {}
        }
    }
    eras
}

// ---------------------------------------------------------------------------
// The marker transition machine (§5.1)
// ---------------------------------------------------------------------------

/// A durable node identity: the incarnation the four superblocks record.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Incarnation(pub u64);

impl Incarnation {
    /// The bumped identity: exactly one past the current one (the
    /// resurrect branch of §5.1). Refused at exhaustion — a wrapped
    /// identity would make a superseded one indistinguishable from a
    /// current one.
    ///
    /// The band invariant (G2, `docs/architecture.md`): the bump is the sole
    /// constructor of a higher identity, and the identity it returns is
    /// strictly greater than the one it supersedes — the superseded band
    /// never re-enters circulation. The surrounding `checked_add` establishes
    /// the impossibility; the `assert!` is the tripwire, release included.
    #[must_use]
    pub fn bump(self) -> Option<Incarnation> {
        let next = self.0.checked_add(1)?;
        assert!(
            next > self.0,
            "the bumped identity must lie in a band disjoint from the identity it supersedes"
        );
        Some(Incarnation(next))
    }
}

/// One superblock copy's marker (§5.1): one state of the ordered marker
/// transition system. Each state names the transition that must have
/// completed for it to exist:
///
/// ```text
/// Running ──stop──> Stopping ──drain──> Stopped ──boot, 2-of-4──> Restarting
///                   (4x write)  flush     (4x write)              (4x write)
///                               WALs + grids
/// Running ──crash──> (markers unchanged) ──boot, no 2-of-4 Stopped──> Joining
///                                                        (bump, 4x write)
/// ```
///
/// No `Started` state is written: no safety logic looks for `Started`, it
/// looks for `Stopped` — the extra superblock write buys no safety and is
/// elided.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Marker {
    /// The stop command was received (`Running ──stop──> Stopping`, 4x).
    /// The node has stopped sending; the drain — the host flushes WALs and
    /// grids — has not yet been proven, so this state vouches for nothing.
    Stopping,
    /// The `Stopping ──drain──> Stopped` transition completed (4x). The
    /// drain happened strictly between the `Stopping` write and this one,
    /// so this state vouches for the WAL under it: there is no amnesiac
    /// risk under a `Stopped` marker.
    Stopped,
    /// The `Stopped ──boot, 2-of-4──> Restarting` transition completed
    /// (4x). The node restarted a CONTROLLED shutdown under the same
    /// identity: a member with complete state, no amnesia, ticking the
    /// full protocol — suspecting a silent primary like any backup.
    Restarting,
    /// The `crash ──boot, no 2-of-4 Stopped──> Joining` transition
    /// completed (bump, 4x). The identity was resurrected under a bumped
    /// incarnation: NOT a member — it neither votes nor drives view
    /// change — until the forced sequence seats it.
    Joining,
}

/// One superblock copy: the identity it records and its marker.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CopyState {
    /// The identity this copy vouches for.
    pub identity: Incarnation,
    /// The copy's marker.
    pub marker: Marker,
}

/// The four superblock copies a restart reads (§2). Exactly four, as the
/// vendored TigerBeetle store's `superblock_copies`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SuperblockCopies {
    /// The four copies, in their on-disk order.
    pub copies: [CopyState; 4],
}

/// The quorum read's verdict (§5.1): did the Stopping→Stopped transition
/// complete?
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartClass {
    /// 2-of-4 copies hold `Stopped`: the transition completed, the drain
    /// is proven, there is no amnesiac risk — the clean stop.
    Stopped,
    /// No stopped quorum — a crash, a torn marker set, or death mid-join:
    /// no controlled shutdown. This identity is dead.
    NotStopped,
}

/// Why a restart refused (§5.1; the refusals of the Zig twin's `open`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartRefusal {
    /// No identity cohort reaches the open threshold — 2 of 4: the marker
    /// set is torn beyond the quorum read (`QuorumLost` in the twin).
    QuorumLost,
    /// The bump would wrap the identity space; the identity that could not
    /// be bumped is carried. A wrapped identity would make a superseded
    /// one indistinguishable from a current one, so the bump refuses
    /// instead.
    Exhausted(Incarnation),
}

/// What a restart decided (§5.1's decision table).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartDecision {
    /// A stopped quorum: continue under the same identity — a member with
    /// complete state, no amnesia — and write `Restarting` to all four
    /// copies (the boot of a controlled shutdown).
    Continue {
        /// The identity the node continues under.
        identity: Incarnation,
    },
    /// No stopped quorum: the identity is dead. Bump it — exactly one
    /// past the quorum-resolved identity — and write `Joining` to all
    /// four copies. The pair IS the commitment (§4): the wire phase
    /// always follows.
    Bump {
        /// The superseded identity.
        old: Incarnation,
        /// The bumped identity.
        new: Incarnation,
    },
}

impl SuperblockCopies {
    /// The quorum read (§5.1; the Zig twin's `open`, `zig/uvrr/store.zig`
    /// lines 116–147). The boot question is *did the transition complete?*,
    /// answered by 2-of-4 copies holding the state to the right of the
    /// transition — the twin's open threshold.
    ///
    /// The working quorum resolves the identity: the copies agreeing on one
    /// identity form a cohort; a cohort reaching the open threshold (2 of
    /// 4) is a working cohort; the winner is the HIGHEST-identity working
    /// cohort (higher-identity-wins INSIDE the working quorum — never the
    /// highest identity observed across all copies, which a lone stale or
    /// superseded copy cannot impose). The verdict reads the winner's
    /// cohort: [`RestartClass::Stopped`] ⟺ it holds ≥2 `Stopped` copies;
    /// anything else is not stopped.
    ///
    /// Over the marker system's written states (four uniform 4x writes per
    /// transition) the winner cohort is all four copies, so the read is
    /// exactly "2-of-4 copies hold `Stopped`"; the cohort rule is what the
    /// torn cases need to keep the twin's quorum structure (which orders
    /// by sequence where this simplified twin, without the sequence
    /// hash-chain, orders by identity).
    ///
    /// Returns `None` when no cohort reaches the threshold — the torn
    /// marker set with no quorum (the twin's `QuorumLost`).
    #[must_use]
    pub fn classify(&self) -> Option<(RestartClass, Incarnation)> {
        // Per identity: how many copies carry it, and how many of those
        // hold `Stopped` (the state right of the Stopping→Stopped
        // transition).
        let mut cohorts: Vec<(Incarnation, usize, usize)> = Vec::new();
        for copy in &self.copies {
            match cohorts
                .iter_mut()
                .find(|(identity, _, _)| *identity == copy.identity)
            {
                Some((_, count, stopped)) => {
                    *count += 1;
                    *stopped += usize::from(copy.marker == Marker::Stopped);
                }
                None => cohorts.push((
                    copy.identity,
                    1,
                    usize::from(copy.marker == Marker::Stopped),
                )),
            }
        }
        let (identity, _, stopped) = cohorts
            .into_iter()
            .filter(|&(_, count, _)| count >= 2)
            .max_by_key(|&(identity, _, _)| identity)?;
        let class = if stopped >= 2 {
            RestartClass::Stopped
        } else {
            RestartClass::NotStopped
        };
        Some((class, identity))
    }

    /// A restart: the quorum read, the decision, and the 4x marker write
    /// the decision leaves on disk (§5.1's decision table).
    ///
    /// * Stopped quorum → [`RestartDecision::Continue`]; the copies are
    ///   written `(identity, Restarting)` 4x — the boot of a controlled
    ///   shutdown. The node is a member with complete state: it ticks the
    ///   full protocol.
    /// * No stopped quorum → [`RestartDecision::Bump`]; the copies are
    ///   written `(new, Joining)` 4x — the resurrection. The bumped
    ///   identity is one past the quorum-resolved identity, checked: an
    ///   identity one bump from `u64` exhaustion refuses rather than
    ///   wraps.
    ///
    /// The uniform 4x write is the repair: every copy that disagreed with
    /// the working quorum is rewritten from the decision — 4-of-4,
    /// satisfying the twin's repair-to-≥3-of-4 behaviour. A node dying
    /// mid-join reads no stopped quorum and resurrects again — the table
    /// makes that fall out.
    ///
    /// # Errors
    ///
    /// [`RestartRefusal::QuorumLost`] when no identity cohort reaches the
    /// open threshold; [`RestartRefusal::Exhausted`] when the bump would
    /// wrap the identity space.
    pub fn restart(&self) -> Result<(RestartDecision, SuperblockCopies), RestartRefusal> {
        let (class, identity) = self.classify().ok_or(RestartRefusal::QuorumLost)?;
        match class {
            RestartClass::Stopped => Ok((
                RestartDecision::Continue { identity },
                self.rewrite(identity, Marker::Restarting),
            )),
            RestartClass::NotStopped => {
                let new = identity.bump().ok_or(RestartRefusal::Exhausted(identity))?;
                Ok((
                    RestartDecision::Bump { old: identity, new },
                    self.rewrite(new, Marker::Joining),
                ))
            }
        }
    }

    /// The stop command (§5.1): `Running ──stop──> Stopping`, written to
    /// all four copies. The node has stopped sending — no disk flush sits
    /// on the protocol's hot path; the drain, flushes of WALs and grids,
    /// is the HOST's and sits strictly between this write and
    /// [`SuperblockCopies::finish_stop`].
    #[must_use]
    pub fn begin_stop(&self) -> SuperblockCopies {
        self.rewrite(self.read_identity(), Marker::Stopping)
    }

    /// The drain's proof (§5.1): `Stopping ──drain──> Stopped`, written to
    /// all four copies — callable only AFTER the host drain completed. The
    /// marker order IS the drain's proof: the drain happened strictly
    /// between the `Stopping` write and this one, so a `Stopped` copy
    /// vouches for the WAL under it, and 2-of-4 `Stopped` at boot proves
    /// the clean stop with no amnesiac risk. A stop that dies partway
    /// still reads clean on the surviving quorum, correctly: the flush had
    /// already completed before the first `Stopped` write.
    #[must_use]
    pub fn finish_stop(&self) -> SuperblockCopies {
        self.rewrite(self.read_identity(), Marker::Stopped)
    }

    /// The identity the node's own copies record: the highest of the four
    /// (§2's higher-identity-wins read rule). The live node's copies are
    /// its own uniform 4x writes — the identity was resolved at boot by
    /// the quorum read ([`SuperblockCopies::classify`]); this is that
    /// identity's live read, not the boot resolution.
    fn read_identity(&self) -> Incarnation {
        self.copies
            .iter()
            .map(|copy| copy.identity)
            .max()
            .expect("four copies are always present")
    }

    /// All four copies rewritten with `identity` and `marker`. The
    /// marker writes are durable-on-write (flushed): the host applies the
    /// returned state with its sync path, which is the only disk traffic
    /// outside the stop path.
    fn rewrite(&self, identity: Incarnation, marker: Marker) -> SuperblockCopies {
        SuperblockCopies {
            copies: self.copies.map(|_| CopyState { identity, marker }),
        }
    }
}
