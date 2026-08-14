//! The `Progress` record and its cross-strategy invariants.
//!
//! Spec §5 and §1.3, §6 for the transition, §12 for the publication interval, and
//! decision W1 for the view identity. Decision B1 owns the observation half: see
//! [`ProgressSnapshot`] and `crate::observe`.
//!
//! `Progress` is the compact protocol record `(current_view, retained_view, status,
//! accepted, committed, applied, checkpoint, fault)` plus the configuration history
//! that authorises it and a `revision` counter. It is immutable: every transition
//! produces a new value, and which fields survive a local crash is a property of the
//! host's declared durability profile, not of this type.
//!
//! # The invariant set, checked on every construction
//!
//! Constructors and transitions are the only way in, and each validates the **full**
//! invariant set on its result — not only the field it changed. The checker is cheap;
//! the alternative is trusting the caller, and a `Progress` that violates these must
//! be unrepresentable through the public API:
//!
//! - the §1.3 frontier chain `checkpoint <= applied <= committed <= accepted`;
//! - the §1.3 status/view relations: `Normal` requires `current == retained`;
//!   `ViewChange` requires `current >= retained`; `Recovering` and `Replaying`
//!   carry no relation, because in them `current` is not an authority to
//!   participate;
//! - era/slot discipline (§8.7.3, W1): the era authorising `accepted` is
//!   `era(current)` or `era(current) + 1` — the `+1` case is overlap mode;
//! - `fault` is sticky (§5 invariant 5): a faulted value admits no transition.
//!
//! `revision` increases by exactly one per published transition and exists for
//! stale-plan rejection: §12 serialises the transition interval, so a plan computed
//! against revision `r` is publishable only while the published state is still at
//! revision `r`. Exactly one transition is ever outstanding.
//!
//! # Why a fault is sticky
//!
//! A fault means the node can no longer say what its own durable state is (S3: only
//! an *indeterminate* persistence result faults; a determinate failure leaves the
//! published state visible). A node that has lost track of its durable state cannot
//! be the authority that declares itself sound again, so no transition clears a
//! fault and the replica refuses all further input (§5 invariant 5, §12's
//! `Indeterminate persistence result -> Faulted`). Recovery is a host lifecycle
//! event — restart, then the ordinary §10 recovery path re-establishes a coherent
//! state — not a state transition the core performs on itself.
//!
//! `status` is process control as well as protocol state. A reopened node that has
//! not proved its state current starts fenced and recovering, whatever status was
//! last observed before failure — which is why [`Progress::genesis`] is
//! [`Status::Recovering`] and not `Normal`. The pre-failure status is evidence
//! about the past, not authority over the present; [`Progress::reconstitute`]
//! accepts any status because it restores *evidence*, and downgrading that evidence
//! to a fenced start is the replica's boot rule (§5), not a property of the
//! record.

use std::sync::Arc;

use crate::configuration::EraTable;
use crate::ids::{Era, Fault, Slot, ViewId};

/// The process-control half of the record (§1.3).
///
/// The relations to `current` and `retained` are stated on [`Progress`] and checked
/// by [`Progress::check`]; they are not restated here because an enum variant cannot
/// enforce them — only the constructor boundary can.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Status {
    /// Actively participating; requires `current == retained` (§1.3).
    Normal,
    /// A view change is in progress; requires `current >= retained` (§1.3). The
    /// replica has entered the later view as a fence but no new history has been
    /// selected and installed, which is exactly why `retained` exists.
    ViewChange,
    /// Proving state currency through the §10 recovery path. `current` is not an
    /// authority to participate.
    Recovering,
    /// Re-applying a transferred or restored history. `current` is not an authority
    /// to participate.
    Replaying,
}

impl Status {
    /// The snapshot encoding: a `u32` word, because [`ProgressSnapshot`] is the flat
    /// POD the seqlock publishes (B1) and an enum with a niche is not POD.
    ///
    /// A `match` rather than a cast, on the same reasoning as `wire::Tag`: the
    /// numbering is stated in one place and reordering this source cannot renumber
    /// the snapshot.
    #[must_use]
    pub fn to_word(self) -> u32 {
        match self {
            Status::Normal => 0,
            Status::ViewChange => 1,
            Status::Recovering => 2,
            Status::Replaying => 3,
        }
    }

    /// The inverse of [`Status::to_word`], or `None` for a word no status encodes.
    /// Total rather than transmuting: the word crosses a seqlock and eventually a C
    /// ABI, and an unrecognised word is a diagnostic value, not undefined behaviour.
    #[must_use]
    pub fn from_word(word: u32) -> Option<Status> {
        match word {
            0 => Some(Status::Normal),
            1 => Some(Status::ViewChange),
            2 => Some(Status::Recovering),
            3 => Some(Status::Replaying),
            _ => None,
        }
    }
}

/// The compact protocol record of §5. Immutable; every transition is a new value.
///
/// Fields are private so the invariant set in the module docs holds for every value
/// that exists: [`Progress::genesis`], [`Progress::reconstitute`], and the `with_*`
/// transitions are the only ways in, and each validates the full set on its result.
#[derive(Clone, Debug)]
pub struct Progress {
    /// Greatest view entered; the fence (§1.3).
    current: ViewId,
    /// The view at which the current logical history was selected (§1.3). It
    /// identifies that history's provenance for the §9.1 ranking rule.
    retained: ViewId,
    /// Process control as well as protocol state; see the module docs.
    status: Status,
    /// Greatest slot the journal records (§5 invariant 1: the published value
    /// equals the frontier the current logical journal history reports).
    accepted: Slot,
    /// Greatest slot known fixed by a normal-operation quorum.
    committed: Slot,
    /// Greatest committed slot incorporated into the application state.
    applied: Slot,
    /// Greatest slot through which the host can restore application state.
    checkpoint: Slot,
    /// Stale-plan rejection for the §12 serialized interval; +1 per transition.
    revision: u64,
    /// The configuration history authorising `current` and `accepted`, shared.
    config: Arc<EraTable>,
    /// Sticky (§5 invariant 5). See the module docs for why.
    fault: Option<Fault>,
}

/// Why a construction or transition was refused.
///
/// One variant per invariant, deliberately no shared "invalid" variant: the
/// invariant set is the whole content of this type, and a test that can only
/// observe `is_err()` passes when the wrong invariant fired.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProgressError {
    /// `checkpoint <= applied <= committed <= accepted` (§1.3) does not hold on
    /// the result.
    FrontierChain,
    /// A monotone frontier moved backwards outside a history re-selection. Within
    /// a re-selection only `accepted` may shorten; see [`Progress::with_view_installed`].
    FrontierRegress,
    /// `Normal` with `current != retained`, or `ViewChange` with
    /// `current < retained` (§1.3).
    StatusViewRelation,
    /// The target view is neither the view being entered nor a legal successor of
    /// the current one (`ViewId::is_legal_successor`, §8.7.3: view strictly up,
    /// era equal or +1).
    ViewSuccessor,
    /// The receiver is faulted. Carries the fault it already holds: a fault is
    /// sticky, so the first fault is the only one there is to report (S3, §5
    /// invariant 5).
    AlreadyFaulted(Fault),
    /// The era authorising `accepted` is not `era(current)` or `era(current) + 1`
    /// (§8.7.3, W1), or the configuration table can no longer name the era of the
    /// accepted frontier — which means the frontier and the table disagree about
    /// history and no candidate built on both is coherent.
    EraSlotDiscipline,
    /// A configuration swap moved the era table backwards. Eras are established by
    /// committed reconfiguration operations; there is no transition that
    /// un-establishes one.
    ConfigRegress,
    /// The `u64` revision space is spent. Checked rather than wrapping: a wrapped
    /// revision would make a stale plan indistinguishable from a current one,
    /// which is the failure `revision` exists to reject (§12).
    RevisionExhausted,
}

/// The flat plain-old-data projection of [`Progress`] that the seqlock publishes
/// (decision B1, §12).
///
/// Every field is `u64`, `u32` or `bool`: no pointers, no `Arc`, no enum with a
/// niche, so a bitwise copy is a complete value and a torn read is detectable by
/// sequence number rather than by content. The configuration history is
/// deliberately absent — it is not POD, and a diagnostic reader that needs it holds
/// the `Arc<EraTable>` it obtained inside the transition interval.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProgressSnapshot {
    /// `current.era`.
    pub era: u32,
    /// `current.view`.
    pub view: u32,
    /// `retained.era`.
    pub retained_era: u32,
    /// `retained.view`.
    pub retained_view: u32,
    /// `status`, encoded by [`Status::to_word`].
    pub status: u32,
    /// Whether the sticky fault is set. The fault's identity is core state, not
    /// diagnostic state; a reader that needs it is inside the interval.
    pub faulted: bool,
    /// The accepted frontier.
    pub accepted: u64,
    /// The committed frontier.
    pub committed: u64,
    /// The applied frontier.
    pub applied: u64,
    /// The checkpoint frontier.
    pub checkpoint: u64,
    /// The publication revision.
    pub revision: u64,
}

/// The era authorising `slot`, according to the table's establishing slots.
///
/// Era `e` is established by the committed operation at `established_by` (§8.7.1:
/// eras and establishing operations are in one-to-one correspondence), so the era
/// of a slot is the greatest era whose establishing slot the frontier has reached.
/// The table's three-era retention window means only the current and previous eras
/// are answerable; anything older returns `None`, and a `None` here means the
/// frontier and the table disagree about history.
fn era_of_slot(table: &EraTable, slot: Slot) -> Option<Era> {
    let current = table.current();
    if current.established_by <= slot {
        return Some(current.era);
    }
    let previous = Era(current.era.0.checked_sub(1)?);
    let record = table.record(previous)?;
    if record.established_by <= slot {
        Some(record.era)
    } else {
        None
    }
}

impl Progress {
    /// The genesis record: [`ViewId::INITIAL`], fenced and recovering, every
    /// frontier at [`Slot::NONE`], revision 0, no fault.
    ///
    /// This pins the meaning of `ViewId::INITIAL` (ruled
    /// here): it is the **genesis view**, the `(era 0, view 0)` pair a freshly
    /// provisioned node advertises. Era 0 is the void configuration —
    /// quorum-impossible by arithmetic, not by guard — and view 0 is the first
    /// primary term once `Init` commits (§1.2: `primary(0) = order[0]`). It is not
    /// "no view": a freshly provisioned node has a real genesis view, so no
    /// `Option<ViewId>` appears anywhere in the crate.
    ///
    /// The status is [`Status::Recovering`] per §5: a node that has not proved its
    /// state current starts fenced, and genesis is the uniform case of that rule —
    /// fresh and reopened nodes enter the protocol the same way, through recovery
    /// or an installed view.
    ///
    /// # Errors
    ///
    /// [`ProgressError::EraSlotDiscipline`] if `config` has moved past genesis: the
    /// empty frontier then belongs to no era the table can still name, which is
    /// the table telling the caller it is not a genesis table.
    pub fn genesis(config: Arc<EraTable>) -> Result<Progress, ProgressError> {
        Progress::reconstitute(
            ViewId::INITIAL,
            ViewId::INITIAL,
            Status::Recovering,
            Slot::NONE,
            Slot::NONE,
            Slot::NONE,
            Slot::NONE,
            0,
            config,
            None,
        )
    }

    /// The general validated constructor: every field named, the full invariant
    /// set checked on the result.
    ///
    /// This is the restart path — the host read its durable record back and offers
    /// it as evidence. §5's rule that a reopened node starts fenced regardless of
    /// the observed status is the replica's boot rule (§5), not enforced here:
    /// `reconstitute` restores evidence about the past, and evidence is not
    /// authority. It is also how tests reach interior states.
    ///
    /// # Errors
    ///
    /// The first failed invariant: [`ProgressError::FrontierChain`],
    /// [`ProgressError::StatusViewRelation`], or [`ProgressError::EraSlotDiscipline`].
    /// A `fault` argument is accepted because a faulted record is still
    /// structurally valid — stickiness is about transitions, not existence.
    #[allow(clippy::too_many_arguments)]
    // Ten parameters because the record has ten fields; a bundle struct would be
    // the same ten names under another name, and this constructor runs once per
    // restart.
    pub fn reconstitute(
        current: ViewId,
        retained: ViewId,
        status: Status,
        accepted: Slot,
        committed: Slot,
        applied: Slot,
        checkpoint: Slot,
        revision: u64,
        config: Arc<EraTable>,
        fault: Option<Fault>,
    ) -> Result<Progress, ProgressError> {
        let progress = Progress {
            current,
            retained,
            status,
            accepted,
            committed,
            applied,
            checkpoint,
            revision,
            config,
            fault,
        };
        progress.check()?;
        Ok(progress)
    }

    /// The full invariant set of the module docs, checked. Constructors call this
    /// on every result; hosts may call it on anything they hold.
    ///
    /// # Errors
    ///
    /// The first failed invariant, in chain, status, era order.
    pub fn check(&self) -> Result<(), ProgressError> {
        self.check_frontier_chain()?;
        self.check_status_relation()?;
        self.check_era_discipline()?;
        Ok(())
    }

    /// §1.3: `checkpoint <= applied <= committed <= accepted`.
    pub(crate) fn check_frontier_chain(&self) -> Result<(), ProgressError> {
        if self.checkpoint <= self.applied
            && self.applied <= self.committed
            && self.committed <= self.accepted
        {
            Ok(())
        } else {
            Err(ProgressError::FrontierChain)
        }
    }

    /// §1.3: `Normal` requires `current == retained`; `ViewChange` requires
    /// `current >= retained`. `Recovering`/`Replaying` carry no relation.
    pub(crate) fn check_status_relation(&self) -> Result<(), ProgressError> {
        let holds = match self.status {
            Status::Normal => self.current == self.retained,
            Status::ViewChange => self.current >= self.retained,
            Status::Recovering | Status::Replaying => true,
        };
        if holds {
            Ok(())
        } else {
            Err(ProgressError::StatusViewRelation)
        }
    }

    /// §8.7.3 / W1: the era authorising `accepted` is `era(current)` or
    /// `era(current) + 1`. The `+1` case is overlap mode and is legal.
    pub(crate) fn check_era_discipline(&self) -> Result<(), ProgressError> {
        let era_accepted =
            era_of_slot(&self.config, self.accepted).ok_or(ProgressError::EraSlotDiscipline)?;
        let in_window =
            era_accepted == self.current.era || self.current.era.next() == Some(era_accepted);
        if in_window {
            Ok(())
        } else {
            Err(ProgressError::EraSlotDiscipline)
        }
    }

    /// The greatest view entered; the fence (§1.3).
    #[must_use]
    pub fn current(&self) -> ViewId {
        self.current
    }

    /// The view at which the current logical history was selected (§1.3).
    #[must_use]
    pub fn retained(&self) -> ViewId {
        self.retained
    }

    /// The process-control status (§1.3).
    #[must_use]
    pub fn status(&self) -> Status {
        self.status
    }

    /// The accepted frontier (§5 invariant 1).
    #[must_use]
    pub fn accepted(&self) -> Slot {
        self.accepted
    }

    /// The committed frontier.
    #[must_use]
    pub fn committed(&self) -> Slot {
        self.committed
    }

    /// The applied frontier.
    #[must_use]
    pub fn applied(&self) -> Slot {
        self.applied
    }

    /// The checkpoint frontier.
    #[must_use]
    pub fn checkpoint(&self) -> Slot {
        self.checkpoint
    }

    /// The publication revision; +1 per transition (§12).
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The configuration history authorising this record.
    #[must_use]
    pub fn config(&self) -> &Arc<EraTable> {
        &self.config
    }

    /// The sticky fault, if the node has declared itself unfit (S3, §5 invariant 5).
    #[must_use]
    pub fn fault(&self) -> Option<Fault> {
        self.fault
    }

    /// The POD projection the seqlock publishes (B1).
    #[must_use]
    pub fn to_snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            era: self.current.era.0,
            view: self.current.view.0,
            retained_era: self.retained.era.0,
            retained_view: self.retained.view.0,
            status: self.status.to_word(),
            faulted: self.fault.is_some(),
            accepted: self.accepted.0,
            committed: self.committed.0,
            applied: self.applied.0,
            checkpoint: self.checkpoint.0,
            revision: self.revision,
        }
    }

    /// The shared transition boundary: refuse the faulted, apply the change, bump
    /// the revision, validate the full invariant set on the result.
    fn transition(&self, change: impl FnOnce(&mut Progress)) -> Result<Progress, ProgressError> {
        self.refuse_if_faulted()?;
        let mut next = self.clone();
        change(&mut next);
        next.revision = self
            .revision
            .checked_add(1)
            .ok_or(ProgressError::RevisionExhausted)?;
        next.check()?;
        Ok(next)
    }

    /// Stickiness as the first check of every transition: a faulted record admits
    /// no candidate, and the fault it reports is the one it already holds.
    fn refuse_if_faulted(&self) -> Result<(), ProgressError> {
        match self.fault {
            Some(fault) => Err(ProgressError::AlreadyFaulted(fault)),
            None => Ok(()),
        }
    }

    /// Records acceptance of operations through `accepted` (normal operation).
    ///
    /// # Errors
    ///
    /// [`ProgressError::AlreadyFaulted`]; [`ProgressError::FrontierRegress`] if
    /// `accepted` is behind the current frontier — acceptance never un-happens;
    /// [`ProgressError::FrontierChain`] or [`ProgressError::EraSlotDiscipline`] on
    /// the result.
    pub fn with_accepted(&self, accepted: Slot) -> Result<Progress, ProgressError> {
        self.refuse_if_faulted()?;
        if accepted < self.accepted {
            return Err(ProgressError::FrontierRegress);
        }
        self.transition(|next| next.accepted = accepted)
    }

    /// Advances the committed frontier (quorum confirmation, §6).
    ///
    /// # Errors
    ///
    /// [`ProgressError::AlreadyFaulted`]; [`ProgressError::FrontierRegress`];
    /// [`ProgressError::FrontierChain`] if `committed` exceeds `accepted` — a
    /// commit frontier cannot claim what the journal does not record (§5
    /// invariant 2).
    pub fn with_committed(&self, committed: Slot) -> Result<Progress, ProgressError> {
        self.refuse_if_faulted()?;
        if committed < self.committed {
            return Err(ProgressError::FrontierRegress);
        }
        self.transition(|next| next.committed = committed)
    }

    /// Advances the applied frontier (application completion, §6).
    ///
    /// # Errors
    ///
    /// [`ProgressError::AlreadyFaulted`]; [`ProgressError::FrontierRegress`];
    /// [`ProgressError::FrontierChain`] if `applied` exceeds `committed` (§5
    /// invariant 3).
    pub fn with_applied(&self, applied: Slot) -> Result<Progress, ProgressError> {
        self.refuse_if_faulted()?;
        if applied < self.applied {
            return Err(ProgressError::FrontierRegress);
        }
        self.transition(|next| next.applied = applied)
    }

    /// Advances the checkpoint frontier (the host can restore through it).
    ///
    /// # Errors
    ///
    /// [`ProgressError::AlreadyFaulted`]; [`ProgressError::FrontierRegress`];
    /// [`ProgressError::FrontierChain`] if `checkpoint` exceeds `applied` — a
    /// checkpoint cannot claim state the application has not incorporated.
    pub fn with_checkpoint(&self, checkpoint: Slot) -> Result<Progress, ProgressError> {
        self.refuse_if_faulted()?;
        if checkpoint < self.checkpoint {
            return Err(ProgressError::FrontierRegress);
        }
        self.transition(|next| next.checkpoint = checkpoint)
    }

    /// Enters `target` as a fence: status [`Status::ViewChange`], `current`
    /// advanced, `retained` untouched — no history has been re-selected yet (§1.3,
    /// §9.1).
    ///
    /// # Errors
    ///
    /// [`ProgressError::AlreadyFaulted`]; [`ProgressError::ViewSuccessor`] unless
    /// `target` is a legal successor of `current` (delegated to
    /// `ViewId::is_legal_successor`, never re-derived here);
    /// [`ProgressError::StatusViewRelation`] if the retained history is from a
    /// later view than `target` — that state must be replayed or recovered, not
    /// fenced into a view change.
    pub fn with_view_change(&self, target: ViewId) -> Result<Progress, ProgressError> {
        self.refuse_if_faulted()?;
        if !self.current.is_legal_successor(target) {
            return Err(ProgressError::ViewSuccessor);
        }
        self.transition(|next| {
            next.current = target;
            next.status = Status::ViewChange;
        })
    }

    /// Installs a selected history: view-change completion or state transfer.
    /// `current` and `retained` join at `view` (§1.3), status becomes
    /// [`Status::Normal`], and the accepted frontier becomes the installed
    /// history's.
    ///
    /// The installed frontier may be **shorter** than the local one: §9.1 ranks
    /// candidate histories by `retained_view` first, so a shorter history retained
    /// from a later view displaces a longer one from an earlier view. It may never
    /// drop below `committed` — that is the chain check, and it is exactly the
    /// safety content of the view-change rule.
    ///
    /// # Errors
    ///
    /// [`ProgressError::AlreadyFaulted`]; [`ProgressError::ViewSuccessor`] unless
    /// `view` is the view being entered or a legal successor of `current` (a
    /// `StartView` for a higher view than the local attempt);
    /// [`ProgressError::FrontierChain`] if the installed frontier does not cover
    /// `committed`.
    pub fn with_view_installed(
        &self,
        view: ViewId,
        accepted: Slot,
    ) -> Result<Progress, ProgressError> {
        self.refuse_if_faulted()?;
        if view != self.current && !self.current.is_legal_successor(view) {
            return Err(ProgressError::ViewSuccessor);
        }
        self.transition(|next| {
            next.current = view;
            next.retained = view;
            next.status = Status::Normal;
            next.accepted = accepted;
        })
    }

    /// Changes only the process-control status. Entering `Normal` through here
    /// requires `current == retained` — checked on the result like everything
    /// else.
    ///
    /// # Errors
    ///
    /// [`ProgressError::AlreadyFaulted`]; [`ProgressError::StatusViewRelation`].
    pub fn with_status(&self, status: Status) -> Result<Progress, ProgressError> {
        self.refuse_if_faulted()?;
        self.transition(|next| next.status = status)
    }

    /// Replaces the configuration history (a committed reconfiguration, §8.7).
    ///
    /// # Errors
    ///
    /// [`ProgressError::AlreadyFaulted`]; [`ProgressError::ConfigRegress`] if the
    /// table's current era is behind the receiver's — committed reconfiguration
    /// has no inverse; [`ProgressError::EraSlotDiscipline`] if the new table can
    /// no longer name the era of the accepted frontier.
    pub fn with_config(&self, config: Arc<EraTable>) -> Result<Progress, ProgressError> {
        self.refuse_if_faulted()?;
        if config.current().era < self.config.current().era {
            return Err(ProgressError::ConfigRegress);
        }
        self.transition(|next| next.config = config)
    }

    /// Declares the node unfit (S3, §5 invariant 5). The fault is sticky: see the
    /// module docs for what that forces the host to do.
    ///
    /// # Errors
    ///
    /// [`ProgressError::AlreadyFaulted`] carrying the fault already held — the
    /// first fault is the only one there is to report.
    pub fn with_fault(&self, fault: Fault) -> Result<Progress, ProgressError> {
        self.refuse_if_faulted()?;
        self.transition(|next| next.fault = Some(fault))
    }
}
