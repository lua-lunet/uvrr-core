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
//! * **The four-superblock restart model** (§2): the marker semantics as a
//!   pure Rust twin of the vendored TigerBeetle store
//!   (`zig/uvrr/store.zig`): `flushed` = a self-consistent checkpoint of
//!   identity X, written at clean shutdown and after the bump; `unflushed`
//!   = the running sentinel, written when the node starts operating. All
//!   four flushed → clean, mark `unflushed`, continue; any unflushed →
//!   dirty, bump, rewrite all four as the new identity. Any read adopts
//!   the highest identity it observes (higher-identity-wins); once bumped,
//!   the decision carries the pair — the commitment is unrepresentable to
//!   break inside the type.

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
            eras.push(SystemOperation::Batch(vec![SystemOperation::Decrement(old)]));
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
                eras.push(SystemOperation::Batch(vec![SystemOperation::Increment(new)]));
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
                eras.push(SystemOperation::Batch(vec![SystemOperation::Increment(new)]));
            }
            Some(0) => {
                eras.push(SystemOperation::Batch(vec![SystemOperation::Increment(new)]));
            }
            // The rejoin is complete: the old identity is gone, the new identity
            // votes.
            Some(_) => {}
        }
    }
    eras
}

// ---------------------------------------------------------------------------
// The four-superblock restart model (§2)
// ---------------------------------------------------------------------------

/// A durable node identity: the incarnation the four superblocks record.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Incarnation(pub u64);

impl Incarnation {
    /// The bumped identity: exactly one past the current one (§2's dirty
    /// path). Refused at exhaustion — a wrapped identity would make a
    /// superseded one indistinguishable from a current one.
    #[must_use]
    pub fn bump(self) -> Option<Incarnation> {
        self.0.checked_add(1).map(Incarnation)
    }
}

/// One superblock copy's marker (§2). `Flushed` = "my on-disk state is a
/// self-consistent checkpoint of identity X", written at clean shutdown
/// and after the bump; `Unflushed` = the running sentinel, written when
/// the node starts operating.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Marker {
    /// A self-consistent checkpoint of the copy's identity.
    Flushed,
    /// The running sentinel: the node is (or was) operating on volatile
    /// state this copy does not vouch for.
    Unflushed,
}

/// One of the four superblock copies: the identity it records and its
/// marker.
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

/// How a restart classifies the four copies (§2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartClass {
    /// All four read `flushed`: the ordinary CR-free path. No recovery
    /// protocol runs — there is no amnesia recovery protocol to run.
    Clean,
    /// Any of the four reads `unflushed`: the node is dirty and eviction
    /// must begin.
    Dirty,
}

/// What a restart decided (§2's state machine).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RestartDecision {
    /// Continue normally under the same identity. The returned copies
    /// carry the running `unflushed` sentinel.
    Continue {
        /// The identity the node continues under.
        identity: Incarnation,
    },
    /// Bump: the identity increments by one and all four copies are
    /// rewritten as `(new, flushed)`. The pair IS the commitment (§4):
    /// the wire phase always follows.
    Bump {
        /// The superseded identity.
        old: Incarnation,
        /// The bumped identity.
        new: Incarnation,
    },
}

impl SuperblockCopies {
    /// The identity a read of the copies adopts: the highest identity any
    /// copy carries (§2's higher-identity-wins read rule).
    #[must_use]
    pub fn read_identity(&self) -> Incarnation {
        self.copies
            .iter()
            .map(|copy| copy.identity)
            .max()
            .expect("four copies are always present")
    }

    /// How the restart classifies (§2): all four `flushed` → clean; ANY
    /// `unflushed` → dirty.
    #[must_use]
    pub fn classify(&self) -> RestartClass {
        if self
            .copies
            .iter()
            .all(|copy| copy.marker == Marker::Flushed)
        {
            RestartClass::Clean
        } else {
            RestartClass::Dirty
        }
    }

    /// A restart: classify, decide, and write the copies the decision
    /// leaves on disk (§2).
    ///
    /// * Clean → [`RestartDecision::Continue`]; the copies are rewritten
    ///   as `(identity, unflushed)` — the running sentinel the next crash
    ///   will read.
    /// * Dirty → [`RestartDecision::Bump`]; the copies are rewritten as
    ///   `(new, flushed)` — the doc's bump, whose flushed mark the
    ///   start-of-operation write below then replaces. The bumped identity
    ///   is one past the HIGHEST identity any copy carries, checked: an
    ///   identity one bump from `u64` exhaustion refuses rather than wraps.
    ///
    /// # Errors
    ///
    /// The bumped identity, when the highest observed identity has spent
    /// the `u64` identity space: a wrapped identity would make a
    /// superseded one indistinguishable from a current one, so the bump
    /// refuses instead.
    pub fn restart(&self) -> Result<(RestartDecision, SuperblockCopies), Incarnation> {
        let identity = self.read_identity();
        match self.classify() {
            RestartClass::Clean => Ok((
                RestartDecision::Continue { identity },
                self.rewrite(identity, Marker::Unflushed),
            )),
            RestartClass::Dirty => {
                let new = identity.bump().ok_or(identity)?;
                Ok((
                    RestartDecision::Bump { old: identity, new },
                    self.rewrite(new, Marker::Flushed),
                ))
            }
        }
    }

    /// The start of operating (§2): the running sentinel, written to all
    /// four copies when the node begins working on volatile state.
    #[must_use]
    pub fn start_operating(&self) -> SuperblockCopies {
        self.rewrite(self.read_identity(), Marker::Unflushed)
    }

    /// A clean shutdown (§2): `flushed` on all four copies — the
    /// self-consistent checkpoint — through the host's sync path.
    #[must_use]
    pub fn clean_shutdown(&self) -> SuperblockCopies {
        self.rewrite(self.read_identity(), Marker::Flushed)
    }

    /// All four copies rewritten with `identity` and `marker`.
    fn rewrite(&self, identity: Incarnation, marker: Marker) -> SuperblockCopies {
        SuperblockCopies {
            copies: self.copies.map(|_| CopyState { identity, marker }),
        }
    }
}
