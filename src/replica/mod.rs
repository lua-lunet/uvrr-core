//! The replica: the two-phase transition pipeline, the lifecycle, and the
//! stability handshake.
//!
//! Spec §5 (progress record), §6 (functional core model), §7 (serialized
//! transition interval and stability levels), §11 (application boundary), §12
//! (concurrency contract), §14.2 (the amnesiac voter). Decisions S2
//! (plan/publish/confirm), S3 (three-way stability), S4 (the recovery nonce is
//! the tick), B1 (observation on publish only), W1 (era in every header), Q1
//! (the quorum gate runs at construction).
//!
//! The load-bearing property of the whole design: **nothing externally
//! observable is released before publication**. `plan` computes a candidate and
//! releases nothing — not an effect, not an observation write, not a journal
//! mutation. `publish` runs the closed [`legal`] gate, and only then installs
//! the candidate, writes the observation, and releases the effects — or, in an
//! external-stability mode, emits the [`PersistenceIntent`] and parks
//! everything else until the host's [`StabilityResult`] arrives as an ordinary
//! serialized input (S2). Observation changes on publish, never on plan (B1).
//!
//! The transition is total and pure:
//!
//! ```text
//! tick + message + state -> state + list(messages)
//! ```
//!
//! No clock read occurs anywhere beneath this module. Every input carries the
//! host tick, and for a recovery input that tick *is* a recovery nonce — the
//! attempt retains a bounded set of them, one per re-drive (§6.1, decision
//! S4). `Input::Tick` exists as an ordinary event, not as a
//! timer callback, so a harness can replay sloppy, late, early and reordered
//! timeouts deterministically — a timeout the core cannot be *told* about is a
//! timeout no test can reproduce.
//!
//! `StabilityResult` is three-way (decision S3): `Stable { receipt }`,
//! `Failed { reason }`, `Indeterminate { reason }`. Only `Indeterminate`
//! sticky-faults the node. A determinate failure leaves the previously
//! published state visible and observable, because collapsing "it definitely
//! did not happen" into "it might have happened" throws away exactly the
//! information that distinguishes a retry from a recovery.
//!
//! Dispatch over inputs is exhaustive `match` with no wildcard arms. An input
//! whose handler is future work is refused with the named, tested
//! [`PlanRejection::Unsupported`] — never a silent no-op, never a placeholder
//! handler.
//!
//! # Normal operation
//!
//! VRR-2012 §4 is live: `Prepare`/`PrepareOk`/`Commit` with
//! commit-frontier piggybacking (§13.3), the Propose/Apply/Applied boundary
//! (§11.1), and the bootstrap from the fenced `Recovering` genesis state (the
//! ruling is on the `plan_tick` handler). Every handler is total: invalid peer
//! input is dropped with a named [`Diagnostic`] on the observation, never
//! faults the node; faulting stays reserved for impossible LOCAL transitions
//! via `invariant::legal`. Quorum decisions go through the
//! [`QuorumStrategy`] (`Role::Commit`) and nowhere else (Q1).
//!
//! # View change
//!
//! VRR-2012 §5 is live: tick-driven timeout detection (S4 — the knob is
//! [`ViewChangeKnobs::primary_timeout`], and only same-view `Prepare`/`Commit`
//! from the legitimate primary count as activity), the `StartViewChange`
//! fence (`Role::Fence` through the strategy, Q1), `DoViewChange` evidence
//! (`Role::ViewChange`), and `StartView` installation. History selection
//! ranks by `retained` view first, then `accepted` frontier (§1.3) — the
//! §9.2 counterexample is the load-bearing test of the rule. Suffixes are
//! bounded newest-first under [`ViewChangeKnobs::view_change_budget`] and
//! encoded ascending (§13.1; W4: `Pack::packed_len` is normative, never
//! exceeded by a byte). A `StartView` suffix that conflicts with a committed
//! local slot is the view-change path's one deliberate fault-on-peer-input: silent
//! repair would hide a safety breach, so the node declares
//! [`Fault::IllegalTransition`]. A suffix the recipient cannot construct
//! history from is a named gap — [`Diagnostic::GapDetected`], never a
//! fault — whose fetch half (§10, §13.1 step 5) rides the same
//! transition: a `GetState` for the missing range, and the installed
//! chunks re-run the stalled ruling.
//!
//! # The application boundary (§11.1, B2)
//!
//! The core orders opaque operations and nothing else. A host proposal
//! carries an [`OperationId`] the proposing host assigns; the core replicates
//! it inside the log entry, never inspects it, and never deduplicates on it —
//! the same identity proposed twice is two operations, at two slots, applied
//! twice. Commitment releases `Effect::Apply` with the slot, the identity and
//! the payload, in slot order, at every node; the host's acknowledgement is
//! `Input::Applied { slot }` with no result, because the boundary is one-way
//! and the core never answers a proposal. Answering a proposer — and any
//! exactly-once policy — is the host's affair above the boundary.
//!
//! The system-slot ruling (§11): `applied` walks EVERY slot. A committed
//! system operation (the genesis `Void`/`Init` of §8.7.2) is core-internal
//! — it emits no `Apply` upcall and expects no acknowledgement — but it
//! advances `applied` the moment the contiguous committed prefix allows,
//! on every path that moves the committed or applied frontier, recovery
//! replay included. The host's `Input::Checkpointed { through }` is
//! accepted only when `through <= applied`; the published checkpoint
//! frontier is the sole reclamation authorization (§4, S1), applied to the
//! default journal by [`Replica::reclaim_journal`] — lazy, whole slabs at
//! a time, never the tail, opportunistic on append.
//!
//! [`legal`]: crate::invariant::legal

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::configuration::{
    ConfigError, EraRecord, EraTable, INIT_SLOT, SystemOperation, VOID_SLOT,
};
use crate::effects::{
    Effect, JournalIntent, PersistenceIntent, ProgressIntent, Stability, StabilityResult,
};
use crate::ids::{Era, Fault, NodeId, Operation, Slot, Tick, View, ViewId};
use crate::invariant::{InputKind, legal};
use crate::journal::{Journal, JournalError, JournalView, LogEntry, Payload, SegmentedLog};
use crate::message::{Body, EraProof, EvidenceKind, Message};
use crate::observe::{Diagnostic, Observation};
use crate::progress::{Progress, ProgressError, ProgressSnapshot, Status};
use crate::quorum::{QuorumError, QuorumStrategy, Role, validate_era};
use crate::wire::{Header, Pack, Tag};

mod normal;
mod reconfiguration;
mod recovery;
mod transfer;
mod view_change;

/// One host event with the host tick attached (§6, S4).
///
/// `at` is host observation metadata sampled when the host began dispatching
/// the event — never a timestamp received from a peer — and for
/// [`Input::Recover`] it is also a recovery nonce (§6.1): the attempt
/// retains a bounded set of them, one per re-drive.
#[derive(Clone, Debug)]
pub struct TimedInput {
    /// The host tick at dispatch time; a recovery nonce for
    /// [`Input::Recover`] (S4).
    pub at: Tick,
    /// The event.
    pub event: Input,
}

/// The input alphabet of §6, extended by the durability and lifecycle inputs
/// the serialized interval requires (§7, §11, §12).
///
/// Every variant is dispatched by an exhaustive match in
/// [`Replica::plan`]; a variant whose handler is future work is refused
/// with [`PlanRejection::Unsupported`] today. Adding a variant is a compile
/// error at every dispatch site, which is the point: no input is ever
/// silently absorbed.
#[derive(Clone, Debug)]
pub enum Input {
    /// A protocol datagram from a peer. `from` is the peer identity the
    /// host's transport attributes the datagram to: authentication is the
    /// host's job (§15); the core consumes the attribution for primary
    /// checks and quorum counting and never derives identity from a payload.
    Peer {
        /// The transport-attributed sender.
        from: NodeId,
        /// The datagram.
        message: Message,
    },
    /// An operation the host proposes for ordering (§6, §11.1): its identity
    /// is the host's to assign, and the core carries it opaque — replicated
    /// inside the entry, handed back on `Effect::Apply`, never inspected and
    /// never deduplicated on (B2).
    Propose {
        /// The operation: identity and opaque bytes (§11.1).
        operation: Operation,
    },
    /// A host timer event (S4). Drives the bootstrap self-promotion of the
    /// genesis primary (see the `plan_tick` handler); the view-change and
    /// recovery timeout bookkeeping belongs to those paths. On a node with
    /// nothing to decide it remains the smallest honest transition: no
    /// protocol state moves, and the interval machinery — revision, gate,
    /// stability handshake — is genuinely exercised by it.
    Tick,
    /// Begin (or re-drive) a recovery attempt (§10). The nonce is
    /// [`TimedInput::at`] (S4); a re-drive adds it to the open attempt's
    /// bounded nonce set and preserves the collected responses.
    Recover,
    /// The host's report on the one outstanding [`PersistenceIntent`] (S2/S3).
    StabilityConfirmation {
        /// The base revision of the transition being confirmed: the
        /// [`PersistenceIntent::revision`] the core emitted at `publish`.
        revision: u64,
        /// The three-way outcome (S3).
        result: StabilityResult,
    },
    /// The application incorporated a committed slot (§11.1). The
    /// acknowledgement carries no result: the boundary is one-way, and the
    /// core never answers a proposal (B2).
    Applied {
        /// The slot whose application completed.
        slot: Slot,
    },
    /// The host checkpointed application state through a slot (§5's
    /// checkpoint frontier, §11). Accepted only when `through` is at or
    /// below the applied frontier — a checkpoint cannot claim state the
    /// application has not incorporated. The published frontier is the
    /// sole reclamation authorization (§4, S1).
    Checkpointed {
        /// The greatest slot the host can now restore through.
        through: Slot,
    },
    /// A reconfiguration operation proposed for commitment (§8.7.2).
    Reconfigure {
        /// The operation to replicate.
        op: SystemOperation,
        /// The concrete `qI`/`qII` vote sets for a non-stop transition
        /// (§8.7.6), when one exists; `None` falls back to the ordinary
        /// stop-the-world view change, which is a latency outcome, not an
        /// error.
        pivot: Option<Pivot>,
    },
    /// The host forced entry into `target` (§14.2): an ordinary view
    /// change driven from the host's say-so, whose new primary is the
    /// member `target` maps to under the current membership order. The
    /// target must strictly advance the view within the current era;
    /// anything else is refused as bad input.
    AdminForceView {
        /// The view to enter: `target.view` past the current view number,
        /// `target.era` the current era.
        target: ViewId,
    },
}

/// The concrete vote sets of a non-stop reconfiguration (§8.7.6–§8.7.7).
///
/// The shape is fixed by the spec: the pivot condition splits the old and new
/// memberships into the two sets whose votes carry the transition without
/// stopping client traffic. The *construction* of a legal pivot is the
/// reconfiguration path's; this record is only its statement.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pivot {
    /// The `qI` set of §8.7.6.
    pub q_i: Vec<NodeId>,
    /// The `qII` set of §8.7.6.
    pub q_ii: Vec<NodeId>,
}

impl Input {
    /// The legality-relevant summary of this input (see [`InputKind`]).
    fn kind(&self) -> InputKind {
        match self {
            Input::Peer { message, .. } => InputKind::PeerMessage {
                tag: message.header.tag,
                slot: message.header.slot,
            },
            Input::Propose { .. } => InputKind::ClientRequest,
            Input::Tick => InputKind::Tick,
            Input::Recover => InputKind::Recovery,
            Input::StabilityConfirmation { .. } => InputKind::StabilityConfirmed,
            Input::Applied { .. } => InputKind::Applied,
            Input::Checkpointed { .. } => InputKind::Checkpointed,
            Input::Reconfigure { .. } => InputKind::Reconfiguration,
            Input::AdminForceView { .. } => InputKind::Admin,
        }
    }
}

/// Why `plan` refused an input.
///
/// One variant per cause, so a test asserting a refusal asserts *which*
/// precondition failed. The fault and outstanding checks precede dispatch, so
/// even an otherwise unsupported input reports the real reason.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlanRejection {
    /// The node is faulted; every input is refused, ticks and confirmations
    /// included (§5 invariant 5). Carries the sticky fault.
    Faulted(Fault),
    /// A transition is parked awaiting its stability confirmation, and §12's
    /// serialized interval admits exactly one outstanding transition. Only a
    /// matching [`Input::StabilityConfirmation`] may be planned.
    TransitionOutstanding,
    /// A stability confirmation arrived with nothing outstanding: a duplicate,
    /// or a confirmation sent to a volatile-mode node that never parks.
    NoTransitionOutstanding,
    /// A stability confirmation named a revision other than the outstanding
    /// intent's.
    ConfirmationMismatch {
        /// The revision the outstanding intent is named by.
        expected: u64,
        /// The revision the confirmation named.
        got: u64,
    },
    /// No handler exists for this input yet. Named and tested — the honest
    /// behaviour for inputs whose handlers are future work — never a silent
    /// no-op and never a placeholder handler.
    Unsupported {
        /// The legality-relevant summary of the refused input.
        input: InputKind,
    },
    /// The journal view offered for planning disagrees with the published
    /// accepted frontier (§5 invariant 1). §12's interval begins with an
    /// atomic read of published state and journal view; a host that hands the
    /// planner a view from another moment has broken that envelope, and the
    /// core refuses rather than plan against an incoherent pair.
    JournalViewDivergence {
        /// The published accepted frontier.
        progress: Slot,
        /// The frontier the offered view reports.
        journal: Slot,
    },
    /// The candidate failed its own construction-time invariant set (see
    /// [`ProgressError`]). Unreachable for today's planners; stated so the
    /// pipeline stays total when future planners compute richer
    /// candidates.
    Progress(ProgressError),
    /// A proposal reached a node that is not the `Normal` primary of
    /// its current view (§4): a backup, a fenced `Recovering` node, or the
    /// primary of some other view. Carries the node's current view and the
    /// primary of that view (when the configuration can name one) so the
    /// host can redirect the proposer (§13.4's convergence hint applied to
    /// the proposal path).
    NotPrimary {
        /// The node's current view.
        view: ViewId,
        /// The primary of `view` under its era's configuration.
        primary: Option<NodeId>,
    },
    /// A recovery input reached a node that is not fenced `Recovering`
    /// (§10): recovery is how a reopened node re-proves its state, and a
    /// participating node has nothing to recover. Carries the status the
    /// node is in.
    NotRecovering {
        /// The node's current status.
        status: Status,
    },
    /// An [`Input::Applied`] the node could not accept: a duplicate, an
    /// out-of-order completion, or a completion for a slot that is not yet
    /// committed. `expected` is the slot the node could accept an `Applied`
    /// for, or `None` when nothing can apply next (nothing newly committed,
    /// or the slot space spent).
    UnexpectedApplied {
        /// The slot the node could accept, if any.
        expected: Option<Slot>,
        /// The slot the host reported.
        got: Slot,
    },
    /// An [`Input::Checkpointed`] past the applied frontier (§11): a
    /// checkpoint cannot claim state the application has not incorporated.
    CheckpointExceedsApplied {
        /// The applied frontier at refusal time.
        applied: Slot,
        /// The frontier the host claimed.
        through: Slot,
    },
    /// A slot the published record says is accepted is absent from the
    /// journal view offered for planning: the two durable records disagree
    /// (§5 invariant 1), so the core refuses rather than plan against an
    /// incoherent pair.
    JournalEntryUnavailable {
        /// The missing slot.
        slot: Slot,
    },
    /// The accepted frontier sits at the end of the slot space; no further
    /// slot can be assigned (§8.7.3 forbids wraparound, so acceptance stops
    /// rather than reuses a position).
    SlotSpaceExhausted,
    /// An [`Input::AdminForceView`] whose target does not strictly advance
    /// the view (§14.2): forcing a change backwards or sideways is bad
    /// input, not a fence.
    AdminTargetNotAhead {
        /// The node's current view.
        current: ViewId,
        /// The refused target.
        target: ViewId,
    },
    /// An [`Input::AdminForceView`] naming an era other than the node's
    /// current era (§14.2): either an era whose establishing operation the
    /// replica does not hold as committed — the membership order it names
    /// was never decided — or a superseded one. The forced change maps its
    /// target under the CURRENT membership order.
    AdminEraNotCurrent {
        /// The node's current era.
        current: Era,
        /// The era the target named.
        got: Era,
    },
    /// An [`Input::AdminForceView`] naming the last representable view:
    /// the forced fence could never be superseded (§8.7.3 forbids
    /// wraparound), so the target is refused outright.
    AdminViewExhausted {
        /// The refused target.
        target: ViewId,
    },
}

/// Why `publish` refused a planned transition.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PublishRejection {
    /// The node is faulted; nothing publishes (§5 invariant 5). Carries the
    /// sticky fault.
    Faulted(Fault),
    /// The plan was computed against a revision other than the currently
    /// published one: a stale plan, or the same plan published twice (§12).
    RevisionMismatch {
        /// The currently published revision.
        expected: u64,
        /// The revision the plan was computed against.
        got: u64,
    },
    /// The closed [`legal`] gate rejected the candidate. The candidate is
    /// DISCARDED — never repaired, never installed — and the node faults with
    /// the reported fault (the closed gate's contract).
    ///
    /// [`legal`]: crate::invariant::legal
    IllegalCandidate(Fault),
    /// A second, non-completion transition was offered while a transition is
    /// parked awaiting its confirmation (§12). Unreachable through `plan`,
    /// which refuses first; stated so `publish` stays total for a host that
    /// holds two plans.
    TransitionOutstanding,
    /// The journal refused the mutation the planner computed against a view
    /// of that same journal. The two durable records disagree about history,
    /// so the node faults with [`Fault::ProgressJournalDivergence`].
    /// Unreachable for today's planners, which mutate nothing; stated so the
    /// install path stays total when future handlers emit mutations.
    JournalRefused(JournalError),
}

/// What `publish` did with an accepted transition.
#[derive(Clone, Debug)]
pub enum PublishOutcome {
    /// The candidate installed, the observation was written, and the effects
    /// are released to the host.
    Published {
        /// The newly published revision: exactly one past the plan's base.
        revision: u64,
        /// The released effects, in release order.
        effects: Vec<Effect>,
    },
    /// An external-stability mode parked the transition behind its
    /// persistence intent (§7, S2). Nothing else released; the observation is
    /// untouched. The interval completes when the matching
    /// [`Input::StabilityConfirmation`] is planned and published.
    Parked {
        /// The base revision the emitted intent is named by: the value the
        /// host's confirmation must name back.
        revision: u64,
        /// Exactly one effect: the [`Effect::Persist`] intent.
        effects: Vec<Effect>,
    },
}

/// The host's view-change knobs (W5).
///
/// Both are policy, not consensus rules: correctness never depends on their
/// values. `primary_timeout` is measured in host ticks (S4 — the core reads
/// no clock): a `Normal` backup that has seen no same-view `Prepare` or
/// `Commit` from the legitimate primary for more than `primary_timeout`
/// ticks enters the next view change. `0` disables tick-driven suspicion
/// (the bootstrap and message-driven view changes still run); the genesis
/// bootstrap then behaves exactly as the bootstrap rule specifies.
///
/// `view_change_budget` bounds the byte size of the history suffix carried
/// by `DoViewChange` and `StartView` (§13.1). The core's obligation is
/// exactness (W4): entries are packed newest-first and the wire suffix never
/// exceeds the budget by a byte — an entry that does not fit stops the
/// packing, so a budget smaller than the newest entry yields no suffix at
/// all, which §13.1 explicitly permits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ViewChangeKnobs {
    /// Ticks of primary silence a `Normal` backup tolerates before fencing
    /// into the next view (S4). `0` disables suspicion.
    pub primary_timeout: u64,
    /// The byte budget for a view-change history suffix (§13.1, W5).
    pub view_change_budget: usize,
}

/// Why construction — provision or reopen — was refused.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LifecycleError {
    /// `own` is not in the genesis order. A node cannot provision as a cluster
    /// it does not belong to.
    NotAMember(NodeId),
    /// The genesis configuration fold refused the order: a duplicate member,
    /// an over-cap membership, or a misplaced genesis ordinal (§8.7.2).
    Configuration(ConfigError),
    /// The Q1 gate refused the configuration: the declared strategy admits
    /// disjoint quorums, so no intersection obligation holds. Runs at
    /// construction — an illegal configuration is refused before the replica
    /// exists, not discovered at the first view change.
    Quorum(QuorumError),
    /// The offered progress record failed the invariant set of
    /// [`Progress::reconstitute`].
    Progress(ProgressError),
    /// The journal refused the genesis history. Unreachable for a well-formed
    /// empty journal; stated because `provision` accepts any [`Journal`]
    /// implementation and the constructor stays total.
    Journal(JournalError),
    /// `provision` was offered a journal that already holds history. A
    /// non-empty journal is evidence of a prior life, and silently
    /// overwriting it is the amnesiac voter of §14.2: the node would vote
    /// with no memory of the promises that history carries.
    JournalNotEmpty,
    /// The persisted progress and the journal disagree about the accepted
    /// frontier (§5 invariant 1): the two durable records tell different
    /// histories, and the core will not guess which one lied.
    ProgressJournalDivergence {
        /// The frontier the persisted progress claims.
        progress: Slot,
        /// The frontier the journal reports.
        journal: Slot,
    },
}

/// The durable half of [`Progress`], as the host persists and returns it (§5).
///
/// The configuration history is deliberately absent: an `Arc<EraTable>` is not
/// a durable value, so `reopen` takes it as a separate argument — the host
/// reconstructs it from the journal it also persists. Not every field must
/// survive a crash; which do is a property of the host's declared durability
/// profile (§5), and `reopen` treats the whole record as evidence about the
/// past, not authority over the present.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PersistedProgress {
    /// Greatest view entered (§1.3).
    pub current: ViewId,
    /// The view at which the current logical history was selected (§1.3).
    pub retained: ViewId,
    /// The last observed status. Evidence only: `reopen` fences to
    /// [`Status::Recovering`] regardless (§5's boot rule).
    pub status: Status,
    /// The accepted frontier; must agree with the journal's at `reopen`.
    pub accepted: Slot,
    /// The committed frontier.
    pub committed: Slot,
    /// The applied frontier.
    pub applied: Slot,
    /// The checkpoint frontier.
    pub checkpoint: Slot,
    /// The publication revision, for stale-plan rejection continuity (§12).
    pub revision: u64,
    /// The sticky fault, if the node had declared itself unfit. Faults
    /// survive restart — they are part of progress (§5 invariant 5).
    pub fault: Option<Fault>,
}

impl From<&Progress> for PersistedProgress {
    fn from(progress: &Progress) -> PersistedProgress {
        PersistedProgress {
            current: progress.current(),
            retained: progress.retained(),
            status: progress.status(),
            accepted: progress.accepted(),
            committed: progress.committed(),
            applied: progress.applied(),
            checkpoint: progress.checkpoint(),
            revision: progress.revision(),
            fault: progress.fault(),
        }
    }
}

/// The journal half of a planned transition (§4's two mutations, or none).
///
/// Carried by [`PlannedTransition`] so the plan computes against a journal
/// view and the publish applies to the journal itself — the plan/publish
/// split made concrete for the one piece of state besides [`Progress`] the
/// pipeline mutates. No planner emits a mutation yet; the handlers that do
/// are the protocol handlers'.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum JournalMutation {
    /// The transition writes no journal slots.
    None,
    /// Record acceptance of a contiguous batch at the tail (§4).
    Accept(Vec<LogEntry>),
    /// Record a view selection: install the suffix a completed view change
    /// selected, from `from` onward (§4, §9).
    InstallSuffix {
        /// The slot at which the selected history takes over.
        from: Slot,
        /// The selected suffix, contiguous and beginning at `from`.
        suffix: Vec<LogEntry>,
    },
}

impl JournalMutation {
    /// The durability summary carried by the [`PersistenceIntent`].
    fn intent(&self) -> JournalIntent {
        match self {
            JournalMutation::None => JournalIntent::Unchanged,
            JournalMutation::Accept(entries) => match (entries.first(), entries.last()) {
                (Some(first), Some(last)) => JournalIntent::Accept {
                    from: first.slot,
                    through: last.slot,
                },
                // An empty batch mutates nothing; the mismatched pairs are
                // unrepresentable (a slice's first and last agree on
                // emptiness).
                _ => JournalIntent::Unchanged,
            },
            JournalMutation::InstallSuffix { from, suffix } => {
                let through = match suffix.last() {
                    Some(last) => last.slot,
                    // An empty suffix is refused by the journal itself; the
                    // intent for it is no mutation.
                    None => *from,
                };
                JournalIntent::InstallSuffix {
                    from: *from,
                    through,
                }
            }
        }
    }
}

/// The output of `plan`: everything `publish` needs, and nothing released.
///
/// Carries the base `revision` the candidate was computed against (§12's
/// stale-plan rejection), the candidate [`Progress`], the journal mutation,
/// the [`PersistenceIntent`], and the effects to release on publication.
/// Constructed only by [`Replica::plan`]; the one escape hatch is the
/// documented gate-testing hook below.
#[derive(Clone, Debug)]
pub struct PlannedTransition {
    /// The published revision this plan was computed against (§12).
    base: u64,
    /// The candidate state. Not yet published, not yet observable.
    candidate: Progress,
    /// The journal half of the transition.
    journal: JournalMutation,
    /// What must be stable before publication (§7).
    intent: PersistenceIntent,
    /// The effects to release on publication.
    effects: Vec<Effect>,
    /// What drove the transition, for the legality gate.
    kind: InputKind,
    /// Whether this plan resolves a parked interval: completions publish
    /// immediately even in an external-stability mode and never re-park.
    completion: bool,
    /// The volatile-bookkeeping half of the transition, applied at install.
    bookkeeping: Bookkeeping,
    /// The drop outcome to publish on the diagnostic observation.
    diagnostic: Diagnostic,
    /// A deliberate fault the transition declares at publish (the
    /// view-change path's one
    /// fault-on-peer-input: a `StartView` suffix conflicting at a committed
    /// slot). The candidate is discarded without installing — the same
    /// outcome the closed gate produces, declared by the planner because the
    /// breach is visible only against the journal, which the gate never sees.
    fault: Option<Fault>,
}

/// The primary's record of one proposed slot, from acceptance until the slot
/// applies: the distinct backups whose `PrepareOk`s have arrived. The
/// primary's own vote is implicit.
///
/// A slot installed by a view change (§9.1) gets its record re-seeded from
/// the entry alone, and a `PrepareOk` for a HIGHER slot can vouch for this
/// one: acceptance is prefix-contiguous, so an acknowledgement vouches for
/// every lower uncommitted slot (VRR-2012 §4's cumulative acknowledgement) —
/// without the record, an installed tail could never commit.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Proposal {
    /// The distinct `PrepareOk` senders recorded so far.
    oks: Vec<NodeId>,
}

/// One replica's `DoViewChange` evidence, as collected by the designated new
/// primary (§9.1): the provenance and frontiers the ranking rule (§1.3)
/// compares and the bounded suffix the selection may need (§13.1). The era
/// proof is validated at receipt and not stored — after validation it has
/// said everything it had to say.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Evidence {
    /// The view at which the reported history was selected (§1.3).
    retained: ViewId,
    /// The reported history's accepted frontier.
    accepted: Slot,
    /// The reported history's committed frontier.
    committed: Slot,
    /// The reported history's newest entries, ascending, budget-bounded.
    suffix: Vec<LogEntry>,
}

/// One responder's `RecoveryResponse` evidence (§6.1): the view it reported
/// — the fence knowledge the `F_g ⌢ R_g` intersection (§8.3) exists to
/// deliver — its frontiers, and the bounded history suffix, which only the
/// reported view's primary may carry.
#[derive(Clone, PartialEq, Eq, Debug)]
struct RecoveryEvidence {
    /// The responder's current view.
    view: ViewId,
    /// The responder's accepted frontier.
    accepted: Slot,
    /// The responder's committed frontier.
    committed: Slot,
    /// The responder's bounded history suffix (§13.1); only the reported
    /// view's primary's is installation evidence (§6.1).
    suffix: Option<Vec<LogEntry>>,
}

/// The bound on a recovery attempt's nonce memory (§10, §6.1): a re-drive
/// past the bound evicts the OLDEST nonce, and a response echoing an
/// evicted nonce is stale exactly like one to an attempt that never ran.
pub(crate) const MAX_RECOVERY_NONCES: usize = 8;

/// The volatile recovery-attempt state (§10, §6.1): the nonce set — each
/// element the tick of one of the episode's recovery inputs (S4) — and
/// the distinct responders counted toward the `R_g` quorum. The node
/// itself is never among them.
///
/// Volatile by design (§8.3's diskless argument: quorum memory, not local
/// storage, survives a crash): a crash discards the attempt, and the
/// reopened node starts a fresh one with a fresh tick.
#[derive(Clone, PartialEq, Eq, Debug)]
struct RecoveryVolatile {
    /// The episode's nonce memory: the tick of each of its recovery
    /// inputs (S4), bounded by [`MAX_RECOVERY_NONCES`] with the oldest
    /// evicted on overflow. A response is this episode's iff its echoed
    /// nonce is in the set.
    nonces: BTreeSet<Tick>,
    /// The counted responses, by transport-attributed sender. A refreshed
    /// answer replaces the earlier one: every nonce in the set binds both
    /// to this episode, and the fresher frontiers are the better evidence.
    responses: BTreeMap<NodeId, RecoveryEvidence>,
    /// The `committed` frontier the attempt opened with: this life's
    /// volatile emission boundary (§11.1). Every slot above it that the
    /// local frontier reaches this life was emitted by the fast-forward,
    /// so the completion re-emits only the durable debt at or below it
    /// plus the range it newly installs. Volatile like the rest of the
    /// attempt: a crash discards it, and the reopened node's replay
    /// re-emits from the durable `applied` as ever.
    open_committed: Slot,
}

/// The volatile view-change attempt state (VRR-2012 §5): the fence target,
/// the distinct `StartViewChange` senders counted toward the `Role::Fence`
/// quorum, the collected `DoViewChange` evidence for the `Role::ViewChange`
/// quorum, and the selected history once the evidence quorum completes.
///
/// Volatile by design: the fence is VRR-2012's volatile `StartViewChange`
/// exchange (§9.3 — the core never substitutes a persisted view record for
/// it), so a crash discards the attempt and the node reopens fenced
/// `Recovering` (§5's boot rule). The durable half is `Progress.current`,
/// which already advanced past every earlier view at entry.
#[derive(Clone, PartialEq, Eq, Debug)]
struct ViewChangeVolatile {
    /// The view being fenced into.
    target: ViewId,
    /// Distinct `StartViewChange` senders for `target`, own vote included.
    fences: BTreeSet<NodeId>,
    /// Collected evidence by sender; own entry appears when the fence
    /// quorum completes (§9.1's ordering: evidence follows the fence).
    evidence: BTreeMap<NodeId, Evidence>,
    /// The selected history, once an evidence quorum holds and the ranking
    /// rule (§1.3) has run.
    selected: Option<Evidence>,
}

/// The view-change half of [`Bookkeeping`]: what a transition does to the
/// volatile attempt state.
#[derive(Clone, Debug, Default)]
enum ViewChangeUpdate {
    /// The attempt state is untouched.
    #[default]
    Unchanged,
    /// Install this attempt state (entry, a fence vote, an evidence record,
    /// or a completed selection that could not yet install).
    Set(ViewChangeVolatile),
    /// The attempt is over: a view installed. Cleared, never rewound.
    Clear,
}

/// The recovery half of [`Bookkeeping`]: what a transition does to the
/// volatile attempt state.
#[derive(Clone, Debug, Default)]
enum RecoveryUpdate {
    /// The attempt state is untouched.
    #[default]
    Unchanged,
    /// Install this attempt state (attempt start, a counted response).
    Set(RecoveryVolatile),
    /// The attempt is over: the recovered history installed.
    Clear,
}

/// The volatile state-transfer cursor (§10, §13.1 step 5): the one open
/// fetch — the view it rides, the responder it asked, and the first slot
/// it still needs. A `NewState` is protocol-qualified evidence only while
/// it answers this record; anything else is a named drop, never a fault.
///
/// Volatile by design, exactly like the recovery attempt: a crash
/// discards the cursor and the reopened node re-fetches under a fresh
/// ruling. A fresh fetch replaces an older one wholesale — the old
/// cursor's answers are then stale.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct TransferVolatile {
    /// The view the fetch rides: the current view for a normal-operation
    /// gap, the fence target for a higher-view pull, the latest fenced
    /// view for a recovery gap.
    view: ViewId,
    /// The responder the fetch asked.
    to: NodeId,
    /// The first slot the node still needs: the cursor a partial answer
    /// resumes from.
    next: Slot,
}

/// The state-transfer half of [`Bookkeeping`]: what a transition does to
/// the volatile fetch cursor.
#[derive(Clone, Debug, Default)]
enum TransferUpdate {
    /// The cursor is untouched.
    #[default]
    Unchanged,
    /// Open (or re-aim) the fetch.
    Set(TransferVolatile),
    /// The fetch is over: the responder had nothing more.
    Clear,
}

/// What the new primary's completion attempt produced.
enum WinOutcome {
    /// The transition to publish (the install, or the declared fault).
    /// Boxed: the variant dwarfs the gap report.
    Installed(Box<PlannedTransition>),
    /// The selected history cannot be constructed from the collected
    /// evidence (§13.1 step 5): the missing range must be fetched
    /// by state transfer (§10) before `StartView`. The effects the attempt had already
    /// accumulated (its own `StartViewChange`/`DoViewChange`) still
    /// release.
    Insufficient {
        /// The first slot the node cannot construct.
        expected: Slot,
        /// The slot the offered evidence begins or claims.
        got: Slot,
        /// The attempt's accumulated effects, handed back to the caller.
        effects: Vec<Effect>,
    },
}

/// What checking an offered suffix against the local journal found
/// (§9.1 install, §13.1 sufficiency).
enum SuffixCheck {
    /// The installed history is locally constructible; this mutation makes
    /// the journal present it.
    Install(JournalMutation),
    /// The offer does not reach back far enough to verify or construct:
    /// the node stays fenced and the missing range is state transfer's
    /// fetch (§10).
    Gap {
        /// The first slot the node cannot verify or construct.
        expected: Slot,
        /// The slot the offer begins at (or claims, when empty).
        got: Slot,
    },
    /// A shared slot at or below the committed frontier disagrees: an
    /// honest evidence quorum can never produce this (§9.2). The
    /// view-change path's one deliberate fault-on-peer-input.
    Conflict,
}

/// The volatile-bookkeeping half of a planned transition — §6's `state ->
/// state` made concrete for the records that are not part of the durable §5
/// record. Computed by `plan`, carried by the transition, applied by
/// `install` — and by the completion of a parked transition, so a `Failed`
/// confirmation discards the record updates with the candidate (S3).
#[derive(Clone, Debug, Default)]
struct Bookkeeping {
    /// New proposal records.
    proposals: Vec<(Slot, Proposal)>,
    /// `PrepareOk` senders to record against outstanding slots.
    oks: Vec<(Slot, NodeId)>,
    /// Slots whose proposals are resolved by application.
    resolved: Vec<Slot>,
    /// The view-change attempt update.
    view_change: ViewChangeUpdate,
    /// The recovery attempt update.
    recovery: RecoveryUpdate,
    /// The state-transfer cursor update.
    transfer: TransferUpdate,
    /// Refresh of the primary-activity baseline (S4): the tick of a
    /// same-view `Prepare`/`Commit` from the legitimate primary, or of a
    /// `StartView` adoption — the new primary has just proved itself alive.
    activity: Option<Tick>,
}

impl PlannedTransition {
    /// Replaces the candidate, leaving every other field intact.
    ///
    /// **Test hook, documented as such** (`tests/replica_contract.rs` gate
    /// test): no honest planner output can violate the frontier rules —
    /// [`Progress`] transitions validate their results — so a test that wants
    /// to prove the publish gate fires needs a way to smuggle a bad candidate
    /// past the planner. That is the point of the hook's existence: it can
    /// only get a bad candidate TO the gate, never past it, which is exactly
    /// the property the gate test asserts.
    #[must_use]
    pub fn substitute_candidate_for_gate_testing(self, candidate: Progress) -> PlannedTransition {
        PlannedTransition { candidate, ..self }
    }

    /// Attaches the volatile-bookkeeping half of the transition.
    fn with_bookkeeping(mut self, bookkeeping: Bookkeeping) -> PlannedTransition {
        self.bookkeeping = bookkeeping;
        self
    }

    /// Attaches the drop outcome the publish records on the diagnostic
    /// observation.
    fn with_diagnostic(mut self, diagnostic: Diagnostic) -> PlannedTransition {
        self.diagnostic = diagnostic;
        self
    }

    /// Declares a deliberate fault at publish: the candidate is discarded,
    /// the node sticky-faults, nothing installs (§5 invariant 5). Used
    /// exactly once: a `StartView` suffix conflicting at a committed slot,
    /// where silent repair would hide a safety breach (§9.1).
    fn with_fault_declared(mut self, fault: Fault) -> PlannedTransition {
        self.fault = Some(fault);
        self
    }

    /// Refreshes the primary-activity baseline at install (S4): the
    /// message that drove this transition proved the primary alive.
    fn with_activity(mut self, at: Tick) -> PlannedTransition {
        self.bookkeeping.activity = Some(at);
        self
    }

    /// Attaches the fetch half of a gap ruling (§13.1 step 5): the
    /// `GetState` joins the transition's effects and the cursor joins its
    /// bookkeeping, so one serialized interval both rules and asks.
    fn with_fetch(mut self, effect: Effect, fetch: TransferVolatile) -> PlannedTransition {
        self.effects.push(effect);
        self.bookkeeping.transfer = TransferUpdate::Set(fetch);
        self
    }
}

/// A transition parked behind its persistence intent (§7, S2).
///
/// Held by the replica between `publish` and the matching confirmation. §12
/// admits exactly one of these at a time.
#[derive(Clone, Debug)]
struct ParkedTransition {
    /// The base revision the intent is named by; the confirmation names it
    /// back.
    base: u64,
    /// The candidate to install on `Stable`.
    candidate: Progress,
    /// The journal mutation to apply on `Stable`.
    journal: JournalMutation,
    /// The effects to release on `Stable`.
    effects: Vec<Effect>,
    /// What drove the original transition: the completion publishes the
    /// original candidate, so the gate re-checks it against the original
    /// kind — the confirmation is the durability signal, not the cause.
    kind: InputKind,
    /// The volatile-bookkeeping half, applied only on `Stable` (S3).
    bookkeeping: Bookkeeping,
    /// The drop outcome, published only on `Stable`.
    diagnostic: Diagnostic,
}

/// A cheap, cloneable read handle onto the replica's published progress (B1).
///
/// A clone of the shared seqlock handle; `read` never blocks the transition
/// interval and never returns a torn snapshot. Observation is read-only
/// without exception (§15).
#[derive(Clone)]
pub struct Observer {
    shared: Arc<Observation<ProgressSnapshot>>,
    diagnostics: Arc<Observation<Diagnostic>>,
}

impl Observer {
    /// The latest published snapshot. Never blocks a writer or another
    /// reader; a torn read retries (B1).
    #[must_use]
    pub fn read(&self) -> ProgressSnapshot {
        self.shared.read()
    }

    /// The latest published transition's drop outcome: why invalid
    /// peer input was dropped, or [`Diagnostic::None`]. Same seqlock
    /// discipline as progress (B1).
    #[must_use]
    pub fn read_diagnostic(&self) -> Diagnostic {
        self.diagnostics.read()
    }
}

/// The replica state machine: identity, published progress, journal, strategy,
/// stability mode, observation handle, and the one outstanding transition.
///
/// Generic over the journal (S1: the host owns the storage strategy behind
/// the four §4 capabilities) and the quorum strategy (Q1: the host supplies
/// the value, the core runs the closed intersection gate on it at
/// construction).
pub struct Replica<J: Journal, Q: QuorumStrategy> {
    /// This node's identity within the configuration (§8.7.1).
    own: NodeId,
    /// The published progress record (§5).
    progress: Progress,
    /// The accepted protocol history (§4).
    journal: J,
    /// The quorum policy value (Q1).
    strategy: Q,
    /// The host's declared durability profile (§7).
    stability: Stability,
    /// The seqlock the observation is published through (B1).
    observation: Arc<Observation<ProgressSnapshot>>,
    /// The seqlock the per-transition drop outcome is published through
    /// (B1; the §4 handlers' total-drop contract made observable).
    diagnostics: Arc<Observation<Diagnostic>>,
    /// The primary's outstanding and unapplied proposals, keyed by slot.
    /// Volatile: a node that loses it reopens fenced `Recovering` (§5's
    /// boot rule), so the loss can never masquerade as authority.
    proposals: BTreeMap<Slot, Proposal>,
    /// The one outstanding parked transition, if any (§12).
    parked: Option<ParkedTransition>,
    /// The host's view-change knobs (W5).
    knobs: ViewChangeKnobs,
    /// The tick of the last same-view `Prepare`/`Commit` from the
    /// legitimate primary, or of the last view adoption (S4). The baseline
    /// the timeout in [`ViewChangeKnobs::primary_timeout`] measures from.
    primary_activity: Tick,
    /// The in-flight view-change attempt, if any. Volatile — the
    /// VRR-2012 fence exchange is volatile by design (§9.3).
    view_change: Option<ViewChangeVolatile>,
    /// The in-flight recovery attempt, if any (§6.1). Volatile — a crash
    /// discards it; the reopened node starts a fresh attempt with a fresh
    /// tick (S4).
    recovery: Option<RecoveryVolatile>,
    /// The one open state-transfer fetch, if any (§10, §13.1 step 5).
    /// Volatile — a crash discards it; the reopened node re-fetches under
    /// a fresh ruling.
    transfer: Option<TransferVolatile>,
}

// Manual, non-exhaustive: `Observation` is a seqlock with no `Debug` of its
// own (its value is read, not printed), and requiring `J: Debug`/`Q: Debug`
// would push a formatting obligation onto host strategy types for no protocol
// reason.
impl<J: Journal, Q: QuorumStrategy> core::fmt::Debug for Replica<J, Q> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Replica")
            .field("own", &self.own)
            .field("progress", &self.progress)
            .field("stability", &self.stability)
            .field("parked", &self.parked)
            .finish_non_exhaustive()
    }
}

impl<J: Journal, Q: QuorumStrategy> Replica<J, Q> {
    /// The first life of a node: construct the genesis history and start
    /// fenced in it.
    ///
    /// Genesis is exactly: `Void` at slot 1 and `Init { genesis_order }` at
    /// slot 2, both committed; current and retained at the era-1 genesis view;
    /// `accepted == committed == Slot(2)`; `applied == Slot(2)` (the §11
    /// system-slot ruling: both genesis slots are core-internal and walk
    /// `applied` by themselves) and `checkpoint == Slot(0)`; status
    /// [`Status::Recovering`] — the genesis ruling (§1.3),
    /// §5's boot rule made uniform: a fresh node and a reopened node enter
    /// the protocol the same way, fenced until they prove their state
    /// current. Nothing about a fresh cluster is special-cased into `Normal`.
    ///
    /// The quorum gate (Q1) runs here on the genesis configuration: an
    /// illegal genesis is refused at construction, not discovered at the
    /// first view change.
    ///
    /// # Errors
    ///
    /// [`LifecycleError::JournalNotEmpty`] if the journal already holds
    /// history (overwriting it is the amnesiac voter of §14.2);
    /// [`LifecycleError::Configuration`] if the genesis fold refuses the
    /// order; [`LifecycleError::NotAMember`] if `own` is outside it;
    /// [`LifecycleError::Quorum`] if the Q1 gate refuses the genesis
    /// configuration; [`LifecycleError::Journal`] or
    /// [`LifecycleError::Progress`] on a construction that cannot complete.
    pub fn provision(
        own: NodeId,
        genesis_order: Vec<NodeId>,
        strategy: Q,
        mut journal: J,
        stability: Stability,
        knobs: ViewChangeKnobs,
    ) -> Result<Self, LifecycleError> {
        if journal.view().accepted().is_some() {
            return Err(LifecycleError::JournalNotEmpty);
        }
        // The genesis fold: `Void` establishes era 0 at its ordinal, `Init`
        // establishes era 1 at its (§8.7.2). Configuration legality —
        // duplicates, the cap, the ordinals — is decided here, at
        // construction.
        let table = EraTable::genesis()
            .extend(&SystemOperation::Void, VOID_SLOT)
            .and_then(|table| {
                table.extend(
                    &SystemOperation::Init {
                        order: genesis_order.clone(),
                    },
                    INIT_SLOT,
                )
            })
            .map_err(LifecycleError::Configuration)?;
        if !genesis_order.contains(&own) {
            return Err(LifecycleError::NotAMember(own));
        }
        // Q1: the gate is a free function the core calls; no strategy value
        // can override, skip or weaken it.
        let era = table.current().era;
        validate_era(&strategy, &table.current().config).map_err(LifecycleError::Quorum)?;

        let genesis = [
            LogEntry {
                slot: VOID_SLOT,
                era: Era::INITIAL,
                payload: Payload::System(SystemOperation::Void),
            },
            LogEntry {
                slot: INIT_SLOT,
                era,
                payload: Payload::System(SystemOperation::Init {
                    order: genesis_order,
                }),
            },
        ];
        journal
            .install_suffix(VOID_SLOT, &genesis)
            .map_err(LifecycleError::Journal)?;

        let view = ViewId {
            era,
            view: View::INITIAL,
        };
        // The §11 system-slot ruling, applied at birth: the genesis history
        // is system operations only — core-internal, committed by
        // construction, never upcalled — so `applied` is born having walked
        // them both.
        let progress = Progress::reconstitute(
            view,
            view,
            Status::Recovering,
            INIT_SLOT,
            INIT_SLOT,
            INIT_SLOT,
            Slot::NONE,
            0,
            Arc::new(table),
            None,
        )
        .map_err(LifecycleError::Progress)?;
        Ok(Self::assemble(
            own, strategy, journal, stability, knobs, progress,
        ))
    }

    /// A later life of a node: restore the durable evidence and start fenced.
    ///
    /// The persisted progress is evidence about the past, not authority over
    /// the present: whatever status was last observed before failure, the
    /// node reopens in [`Status::Recovering`] (§5's boot rule) and becomes
    /// normal only after local restoration or quorum recovery establishes
    /// adequate state (§14.2). The persisted fault, if any, is preserved —
    /// faults survive restart because they are part of progress (§5
    /// invariant 5).
    ///
    /// A host that cannot produce a persisted progress must say so by using
    /// [`Replica::provision`] instead. Reopening with manufactured state is
    /// the amnesiac voter §14.2 warns about; the core cannot detect a
    /// fabricated record, so it makes the host say which it is doing by
    /// choosing the constructor.
    ///
    /// # Errors
    ///
    /// [`LifecycleError::ProgressJournalDivergence`] if the persisted
    /// accepted frontier disagrees with the journal's;
    /// [`LifecycleError::Progress`] if the persisted record fails the
    /// invariant set; [`LifecycleError::Quorum`] if the Q1 gate refuses the
    /// configuration the host reconstructed.
    pub fn reopen(
        own: NodeId,
        strategy: Q,
        journal: J,
        persisted: PersistedProgress,
        config: Arc<EraTable>,
        stability: Stability,
        knobs: ViewChangeKnobs,
    ) -> Result<Self, LifecycleError> {
        // Q1 at construction, exactly as at provision: the gate does not
        // trust that a reconstructed table was gated before it was persisted.
        validate_era(&strategy, &config.current().config).map_err(LifecycleError::Quorum)?;
        let frontier = journal.view().accepted().unwrap_or(Slot::NONE);
        if frontier != persisted.accepted {
            return Err(LifecycleError::ProgressJournalDivergence {
                progress: persisted.accepted,
                journal: frontier,
            });
        }
        let progress = Progress::reconstitute(
            persisted.current,
            persisted.retained,
            Status::Recovering,
            persisted.accepted,
            persisted.committed,
            persisted.applied,
            persisted.checkpoint,
            persisted.revision,
            config,
            persisted.fault,
        )
        .map_err(LifecycleError::Progress)?;
        Ok(Self::assemble(
            own, strategy, journal, stability, knobs, progress,
        ))
    }

    /// The shared constructor: the observation is born holding the initial
    /// snapshot (B1), and nothing is outstanding. The activity baseline is
    /// the epoch tick: a node starts never-having-heard a primary, and the
    /// bootstrap's `Recovering` status exempts it from suspicion until it
    /// first adopts a view.
    fn assemble(
        own: NodeId,
        strategy: Q,
        journal: J,
        stability: Stability,
        knobs: ViewChangeKnobs,
        progress: Progress,
    ) -> Self {
        Replica {
            own,
            observation: Arc::new(Observation::new(progress.to_snapshot())),
            diagnostics: Arc::new(Observation::new(Diagnostic::None)),
            proposals: BTreeMap::new(),
            progress,
            journal,
            strategy,
            stability,
            parked: None,
            knobs,
            primary_activity: Tick(0),
            view_change: None,
            recovery: None,
            transfer: None,
        }
    }

    /// This node's identity within the configuration (§8.7.1).
    #[must_use]
    pub fn own(&self) -> NodeId {
        self.own
    }

    /// The published progress record (§5).
    #[must_use]
    pub fn progress(&self) -> &Progress {
        &self.progress
    }

    /// The journal, for the host's read paths and for taking the view a plan
    /// is computed against (§4).
    #[must_use]
    pub fn journal(&self) -> &J {
        &self.journal
    }

    /// The declared quorum strategy (Q1).
    #[must_use]
    pub fn strategy(&self) -> &Q {
        &self.strategy
    }

    /// The host's declared durability profile (§7).
    #[must_use]
    pub fn stability(&self) -> Stability {
        self.stability
    }

    /// A cheap clone of the observation handle (B1). Reads never block the
    /// transition interval; observation is read-only (§15).
    #[must_use]
    pub fn observer(&self) -> Observer {
        Observer {
            shared: Arc::clone(&self.observation),
            diagnostics: Arc::clone(&self.diagnostics),
        }
    }

    /// Phase one of the pipeline (§7 step 2): compute the candidate, the
    /// persistence intent, and the effects — and release nothing.
    ///
    /// `journal` must be a view of this replica's journal consistent with the
    /// published state: §12's interval begins with an atomic read of the two,
    /// and a view that disagrees with the published accepted frontier is
    /// refused as [`PlanRejection::JournalViewDivergence`] rather than
    /// planned against.
    ///
    /// Releases nothing: no effect, no observation write, no journal
    /// mutation, no state change of any kind. That is the sentence the whole
    /// design stands on — nothing externally observable is released before
    /// publication.
    ///
    /// # Errors
    ///
    /// [`PlanRejection::Faulted`] if the node is faulted — every input,
    /// ticks and confirmations included; [`PlanRejection::TransitionOutstanding`]
    /// if a transition is parked awaiting its confirmation;
    /// [`PlanRejection::NoTransitionOutstanding`] or
    /// [`PlanRejection::ConfirmationMismatch`] for a confirmation that does
    /// not match the one outstanding intent; [`PlanRejection::Unsupported`]
    /// for an input whose handler is future work;
    /// [`PlanRejection::JournalViewDivergence`] for an incoherent journal
    /// view; [`PlanRejection::Progress`] if the candidate fails its own
    /// invariant set.
    pub fn plan(
        &self,
        input: &TimedInput,
        journal: &J::View,
    ) -> Result<PlannedTransition, PlanRejection> {
        // Stickiness precedes everything (§5 invariant 5): a faulted node
        // refuses every input variant, and reports the fault it already holds.
        if let Some(fault) = self.progress.fault() {
            return Err(PlanRejection::Faulted(fault));
        }
        // The §12 envelope: the plan is computed against the published state
        // and a journal view read atomically with it (§5 invariant 1).
        let frontier = journal.accepted().unwrap_or(Slot::NONE);
        if frontier != self.progress.accepted() {
            return Err(PlanRejection::JournalViewDivergence {
                progress: self.progress.accepted(),
                journal: frontier,
            });
        }
        match &input.event {
            Input::StabilityConfirmation { revision, result } => {
                self.plan_confirmation(*revision, result)
            }
            Input::Tick => {
                self.refuse_if_parked()?;
                self.plan_tick(journal, input.at)
            }
            Input::Propose { operation } => {
                self.refuse_if_parked()?;
                self.plan_propose(journal, operation)
            }
            Input::Peer { from, message } => {
                self.refuse_if_parked()?;
                self.plan_peer(journal, *from, message, input.at, input.event.kind())
            }
            Input::Applied { slot } => {
                self.refuse_if_parked()?;
                self.plan_applied(journal, *slot)
            }
            Input::Recover => {
                self.refuse_if_parked()?;
                self.plan_recover(input.at)
            }
            Input::AdminForceView { target } => {
                self.refuse_if_parked()?;
                self.plan_admin_force_view(journal, *target, input.at)
            }
            Input::Checkpointed { through } => {
                self.refuse_if_parked()?;
                self.plan_checkpointed(*through)
            }
            Input::Reconfigure { .. } => {
                // The outstanding check precedes dispatch, so even an
                // otherwise unsupported input reports the real reason (§12).
                self.refuse_if_parked()?;
                Err(PlanRejection::Unsupported {
                    input: input.event.kind(),
                })
            }
        }
    }

    /// The tick (S4): today it drives exactly one protocol decision, the
    /// bootstrap self-promotion of the genesis primary.
    ///
    /// A `Recovering` node enters `Normal` on a tick when ALL of: its
    /// journal holds the complete committed genesis (slots 1–2, both
    /// physically present, nothing beyond), `current == retained` at the
    /// genesis view, and it IS `config.primary(View(0))` under its
    /// configuration. Rationale: at initial provisioning the genesis is
    /// complete and committed by construction, so there is no prior state
    /// to be amnesiac about — the §14.2 objection does not apply to initial
    /// provisioning, only to reopen. A reopened node either changed view
    /// (`current.view != 0`) or holds post-genesis history (`accepted >
    /// 2`), and both exclude it here; its path is §10 recovery.
    /// The promotion happens on a tick, not in `provision`, so construction
    /// stays uniform — every node starts fenced `Recovering` — and the
    /// promotion is an explicit protocol step the trace shows.
    ///
    /// On promotion the new primary announces its committed frontier to
    /// every backup (§13.3): that is how an idle cluster's backups learn
    /// the view. An amnesiac-restarted genesis primary satisfies the
    /// condition too — its proposals are then refused by every backup that
    /// holds post-genesis history ([`Diagnostic::ConflictingEntry`]), so
    /// the view stalls but cannot diverge; deposing it is the view change
    /// below.
    ///
    /// The tick's second decision (S4): a `Normal` backup whose
    /// primary has been silent for more than
    /// [`ViewChangeKnobs::primary_timeout`] ticks enters the next view
    /// change. Silence is measured from the last same-view `Prepare` or
    /// `Commit` from the legitimate primary (or the last view adoption);
    /// the primary of the current view never suspects itself, and a
    /// `Recovering` or already-`ViewChange` node has nothing to suspect —
    /// a stalled attempt is state transfer's repair (§10), not a fresh timeout.
    fn plan_tick(&self, journal: &J::View, at: Tick) -> Result<PlannedTransition, PlanRejection> {
        let current = self.progress.current();
        let promotable = self.progress.status() == Status::Recovering
            && current == self.progress.retained()
            && current.view == View::INITIAL
            && self.progress.accepted() == INIT_SLOT
            && self.progress.committed() == INIT_SLOT
            && journal.get(VOID_SLOT).is_some()
            && journal.get(INIT_SLOT).is_some()
            && self.primary_of(current) == Some(self.own);
        if promotable {
            let candidate = self
                .progress
                .with_status(Status::Normal)
                .map_err(PlanRejection::Progress)?;
            let effects = self.broadcast_commit(self.progress.committed());
            // The promotion announces the view to every backup: proof of
            // the new primary's life, its own baseline included (S4).
            return Ok(self
                .candidate_plan(
                    candidate,
                    JournalMutation::None,
                    effects,
                    InputKind::Tick,
                    false,
                )
                .with_activity(at));
        }
        // Any Normal node can suspect — the primary of the current view
        // included: it refreshes its baseline only on its own proposals,
        // so an idle primary times out into the next view exactly like a
        // backup with a silent primary. A solo primary's change still
        // cannot complete (the fence needs a quorum), which is the paper's
        // answer to a partitioned primary's suspicion (§9).
        let suspects = self.knobs.primary_timeout != 0
            && self.progress.status() == Status::Normal
            && self
                .primary_activity
                .0
                .checked_add(self.knobs.primary_timeout)
                .is_some_and(|deadline| at.0 > deadline);
        if suspects {
            let target = current
                .next_in_era()
                .ok_or(PlanRejection::Progress(ProgressError::ViewSuccessor))?;
            return self.enter_view_change(journal, target, BTreeSet::new(), at, InputKind::Tick);
        }
        // §13.1 step 5: a recovery completion that stalled on an
        // unconstructible evidence suffix re-runs once state transfer has
        // supplied the missing range. The precheck keeps an ordinary tick
        // honest: no quorum, no primary history, or a still-
        // unconstructible suffix falls through to the no-op below.
        // Completion remains a recovery install (§6.1): the tick only
        // schedules the re-drive after transfer supplied the missing range.
        if self.progress.status() == Status::Recovering {
            if let Some(attempt) = self.recovery.clone() {
                if let Some((source, latest, evidence)) =
                    self.recovery_completion_ready(journal, &attempt)
                {
                    return self.plan_recovery_completion(
                        journal,
                        attempt,
                        source,
                        latest,
                        evidence,
                        at,
                        InputKind::Recovery,
                    );
                }
            }
        }
        // The smallest honest transition: no protocol state
        // moves, and the interval machinery is genuinely exercised.
        let candidate = self
            .progress
            .with_status(self.progress.status())
            .map_err(PlanRejection::Progress)?;
        Ok(self.candidate_plan(
            candidate,
            JournalMutation::None,
            Vec::new(),
            InputKind::Tick,
            false,
        ))
    }

    /// The §11.1 acknowledgement: the next applied slot in order advances
    /// `applied` and resolves the slot's proposal record. A completion for
    /// any other slot — a duplicate, an out-of-order report, or a slot that
    /// is not yet committed — is the named
    /// [`PlanRejection::UnexpectedApplied`], refused without state change.
    /// Nothing is emitted: the acknowledgement carries no result, and the
    /// core never answers a proposal (B2).
    ///
    /// The §11 system-slot ruling: `applied` walks every slot. A committed
    /// system operation is core-internal — it emits no `Apply` upcall and
    /// expects no acknowledgement — but it advances `applied` the moment
    /// the contiguous committed prefix allows (see [`Self::applied_walk`]).
    /// The next slot the host can report is therefore always an operation
    /// slot; a report naming a system slot is a duplicate — the walk
    /// already crossed it.
    fn plan_applied(
        &self,
        journal: &J::View,
        slot: Slot,
    ) -> Result<PlannedTransition, PlanRejection> {
        let committed = self.progress.committed();
        let walked = self.applied_walk(journal, &[], self.progress.applied(), committed)?;
        let expected = match walked.next() {
            Some(next) if next <= committed => Some(next),
            Some(_) | None => None,
        };
        if expected != Some(slot) {
            return Err(PlanRejection::UnexpectedApplied {
                expected,
                got: slot,
            });
        }
        // A `Replaying` node (§6.1, §11.1) flips to `Normal` when the
        // walked frontier catches up with the committed frontier: the
        // recovered history is applied, and participation resumes.
        let applied = self.applied_walk(journal, &[], slot, committed)?;
        let status = if self.progress.status() == Status::Replaying && applied == committed {
            Status::Normal
        } else {
            self.progress.status()
        };
        let candidate =
            self.candidate_with(status, self.progress.accepted(), committed, applied)?;
        let bookkeeping = Bookkeeping {
            resolved: vec![slot],
            ..Bookkeeping::default()
        };
        Ok(self
            .candidate_plan(
                candidate,
                JournalMutation::None,
                Vec::new(),
                InputKind::Applied,
                false,
            )
            .with_bookkeeping(bookkeeping))
    }

    /// The host's checkpoint report (§5's checkpoint frontier, §11):
    /// accepted only when `through <= applied` — a checkpoint cannot claim
    /// state the application has not incorporated — and otherwise refused
    /// as [`PlanRejection::CheckpointExceedsApplied`] without state change.
    /// A report at or below the published frontier is a duplicate:
    /// accepted as an identity transition, moving nothing.
    ///
    /// The published frontier is the SOLE reclamation authorization (§4,
    /// S1): what it permits the default journal to drop is applied by
    /// [`Replica::reclaim_journal`], lazily, on a later append — never by
    /// this transition itself.
    fn plan_checkpointed(&self, through: Slot) -> Result<PlannedTransition, PlanRejection> {
        let applied = self.progress.applied();
        if through > applied {
            return Err(PlanRejection::CheckpointExceedsApplied { applied, through });
        }
        let candidate = if through <= self.progress.checkpoint() {
            self.identity_candidate()?
        } else {
            self.progress
                .with_checkpoint(through)
                .map_err(PlanRejection::Progress)?
        };
        Ok(self.candidate_plan(
            candidate,
            JournalMutation::None,
            Vec::new(),
            InputKind::Checkpointed,
            false,
        ))
    }

    /// The applied frontier after walking every committed system slot the
    /// contiguous prefix allows (the §11 system-slot ruling): `applied`
    /// walks EVERY slot — a committed system operation emits no `Apply`
    /// upcall (it is core-internal, folded into the configuration on
    /// commit) but it advances `applied` exactly like an acknowledged
    /// operation slot. `overlay` supplies the entries an in-transition
    /// install adds to the picture, exactly as in
    /// [`Self::apply_effects_merged`]: the walk then runs over the history
    /// the transition is installing.
    fn applied_walk(
        &self,
        journal: &J::View,
        overlay: &[LogEntry],
        applied: Slot,
        committed: Slot,
    ) -> Result<Slot, PlanRejection> {
        let mut walked = applied;
        while let Some(next) = walked.next() {
            if next > committed {
                break;
            }
            let entry = match overlay.iter().find(|entry| entry.slot == next) {
                Some(entry) => entry,
                None => journal
                    .get(next)
                    .ok_or(PlanRejection::JournalEntryUnavailable { slot: next })?,
            };
            if !matches!(entry.payload, Payload::System(_)) {
                break;
            }
            walked = next;
        }
        Ok(walked)
    }

    /// The `Apply` effects for the newly committed slots `(from, through]`,
    /// in slot order (§11.1): operation payloads only — system slots commit,
    /// but the application boundary is not theirs.
    fn apply_effects(
        &self,
        journal: &J::View,
        from: Slot,
        through: Slot,
    ) -> Result<Vec<Effect>, PlanRejection> {
        let mut effects = Vec::new();
        if through <= from {
            return Ok(effects);
        }
        let mut slot = from
            .next()
            .expect("`from < through <= u64::MAX`, so `from` has a successor");
        loop {
            let entry = journal
                .get(slot)
                .ok_or(PlanRejection::JournalEntryUnavailable { slot })?;
            if let Payload::Operation { id, payload } = &entry.payload {
                effects.push(Effect::Apply {
                    slot,
                    operation_id: *id,
                    payload: payload.clone(),
                });
            }
            if slot == through {
                break;
            }
            slot = slot
                .next()
                .expect("`slot < through <= u64::MAX`, so `slot` has a successor");
        }
        Ok(effects)
    }

    /// The `Apply` effects for the newly committed slots `(from, through]`
    /// of an installed history (§11.1): the suffix overlays the journal for
    /// the slots being installed in the same transition.
    fn apply_effects_merged(
        &self,
        journal: &J::View,
        suffix: &[LogEntry],
        from: Slot,
        through: Slot,
    ) -> Result<Vec<Effect>, PlanRejection> {
        let mut effects = Vec::new();
        if through <= from {
            return Ok(effects);
        }
        let mut slot = from
            .next()
            .expect("`from < through <= u64::MAX`, so `from` has a successor");
        loop {
            let entry = match suffix.iter().find(|entry| entry.slot == slot) {
                Some(entry) => entry,
                None => journal
                    .get(slot)
                    .ok_or(PlanRejection::JournalEntryUnavailable { slot })?,
            };
            if let Payload::Operation { id, payload } = &entry.payload {
                effects.push(Effect::Apply {
                    slot,
                    operation_id: *id,
                    payload: payload.clone(),
                });
            }
            if slot == through {
                break;
            }
            slot = slot
                .next()
                .expect("`slot < through <= u64::MAX`, so `slot` has a successor");
        }
        Ok(effects)
    }

    /// An identity candidate: only the revision moves. The published half
    /// of every drop.
    fn identity_candidate(&self) -> Result<Progress, PlanRejection> {
        self.progress
            .with_status(self.progress.status())
            .map_err(PlanRejection::Progress)
    }

    /// Builds a candidate over the published record, changing exactly the
    /// named fields, with the revision advanced by one (§12).
    /// `Progress::reconstitute` validates the full invariant set on the
    /// result, and the publish gate re-checks the pair — the two checks are
    /// the belt and the braces.
    fn candidate_with(
        &self,
        status: Status,
        accepted: Slot,
        committed: Slot,
        applied: Slot,
    ) -> Result<Progress, PlanRejection> {
        let revision = self
            .progress
            .revision()
            .checked_add(1)
            .ok_or(ProgressError::RevisionExhausted)
            .map_err(PlanRejection::Progress)?;
        Progress::reconstitute(
            self.progress.current(),
            self.progress.retained(),
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

    /// Builds the install candidate: `current` and `retained` join at
    /// `view` (§1.3 — the history was selected by this change), status is
    /// `Normal`, and the frontiers become the installed history's, with
    /// `applied` the §11 walk over that history (system slots committed by
    /// the install advance it without an upcall). The accepted frontier
    /// may regress across the re-selection (gate rule 1 admits it exactly
    /// then); the committed frontier never does — the callers guard it
    /// before this candidate is ever built.
    fn install_candidate(
        &self,
        view: ViewId,
        accepted: Slot,
        committed: Slot,
        applied: Slot,
    ) -> Result<Progress, PlanRejection> {
        let revision = self
            .progress
            .revision()
            .checked_add(1)
            .ok_or(ProgressError::RevisionExhausted)
            .map_err(PlanRejection::Progress)?;
        Progress::reconstitute(
            view,
            view,
            Status::Normal,
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

    /// Bundles a candidate with its journal mutation, intent, and effects.
    fn candidate_plan(
        &self,
        candidate: Progress,
        journal: JournalMutation,
        effects: Vec<Effect>,
        kind: InputKind,
        completion: bool,
    ) -> PlannedTransition {
        PlannedTransition {
            base: self.progress.revision(),
            intent: PersistenceIntent {
                revision: self.progress.revision(),
                progress: progress_intent(&self.progress, &candidate),
                journal: journal.intent(),
            },
            candidate,
            journal,
            effects,
            kind,
            completion,
            bookkeeping: Bookkeeping::default(),
            diagnostic: Diagnostic::None,
            fault: None,
        }
    }

    /// A drop: publish an identity transition carrying the named
    /// diagnostic. Invalid peer input is dropped with a trace, never
    /// faulted; faulting is reserved for impossible LOCAL transitions via
    /// `invariant::legal`.
    fn drop_plan(
        &self,
        diagnostic: Diagnostic,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let candidate = self.identity_candidate()?;
        Ok(self
            .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
            .with_diagnostic(diagnostic))
    }

    /// §12: while a confirmation is pending, no second transition is planned.
    fn refuse_if_parked(&self) -> Result<(), PlanRejection> {
        if self.parked.is_some() {
            Err(PlanRejection::TransitionOutstanding)
        } else {
            Ok(())
        }
    }

    /// Plan the resolution of the one outstanding interval (S2/S3).
    ///
    /// `Stable` plans the parked candidate itself — the confirmation is the
    /// durability signal, not a new transition, so the completion carries the
    /// original kind and the gate re-checks the original candidate against
    /// it. `Failed` plans a revision-advancing resolution over the unchanged
    /// published state: the candidate is discarded, the interval must still
    /// close, and the revision must move so plans computed against the parked
    /// base die. `Indeterminate` plans the sticky fault (§5 invariant 5).
    fn plan_confirmation(
        &self,
        revision: u64,
        result: &StabilityResult,
    ) -> Result<PlannedTransition, PlanRejection> {
        let Some(parked) = &self.parked else {
            return Err(PlanRejection::NoTransitionOutstanding);
        };
        if revision != parked.base {
            return Err(PlanRejection::ConfirmationMismatch {
                expected: parked.base,
                got: revision,
            });
        }
        match result {
            StabilityResult::Stable { .. } => Ok(self
                .candidate_plan(
                    parked.candidate.clone(),
                    parked.journal.clone(),
                    parked.effects.clone(),
                    parked.kind,
                    true,
                )
                .with_bookkeeping(parked.bookkeeping.clone())
                .with_diagnostic(parked.diagnostic)),
            StabilityResult::Failed { .. } => {
                let candidate = self
                    .progress
                    .with_status(self.progress.status())
                    .map_err(PlanRejection::Progress)?;
                Ok(self.candidate_plan(
                    candidate,
                    JournalMutation::None,
                    Vec::new(),
                    InputKind::StabilityConfirmed,
                    true,
                ))
            }
            StabilityResult::Indeterminate { .. } => {
                let candidate = self
                    .progress
                    .with_fault(Fault::IndeterminatePersistence)
                    .map_err(PlanRejection::Progress)?;
                Ok(self.candidate_plan(
                    candidate,
                    JournalMutation::None,
                    Vec::new(),
                    InputKind::StabilityConfirmed,
                    true,
                ))
            }
        }
    }

    /// Phase two of the pipeline (§7 steps 3–7): gate, then publish or park.
    ///
    /// The closed [`legal`] gate runs on every old→candidate pair before
    /// anything installs. A `Some` from the gate DISCARDS the candidate and
    /// sticky-faults the node — never repair, never install (the closed
    /// gate's contract). In [`Stability::Volatile`] the candidate then installs, the
    /// observation is written, and the effects release. In every other mode
    /// exactly one effect releases — the [`Effect::Persist`] intent — and the
    /// transition parks until the matching
    /// [`Input::StabilityConfirmation`] completes it (S2). Completions
    /// publish immediately in every mode: the barrier has already completed,
    /// which is what the confirmation means.
    ///
    /// [`legal`]: crate::invariant::legal
    ///
    /// # Errors
    ///
    /// [`PublishRejection::Faulted`] if the node is faulted;
    /// [`PublishRejection::RevisionMismatch`] for a stale or double-published
    /// plan; [`PublishRejection::IllegalCandidate`] when the gate rejects the
    /// candidate; [`PublishRejection::TransitionOutstanding`] for a second
    /// ordinary publish while parked; [`PublishRejection::JournalRefused`] if
    /// the journal refuses the planned mutation.
    pub fn publish(
        &mut self,
        planned: PlannedTransition,
    ) -> Result<PublishOutcome, PublishRejection> {
        if let Some(fault) = self.progress.fault() {
            return Err(PublishRejection::Faulted(fault));
        }
        let expected = self.progress.revision();
        if planned.base != expected {
            return Err(PublishRejection::RevisionMismatch {
                expected,
                got: planned.base,
            });
        }
        // A planner-declared fault (the conflicting `StartView`
        // suffix): the same outcome the gate produces, declared by the
        // planner because the breach is journal-visible only. Discards the
        // candidate, never parks, never installs.
        if let Some(fault) = planned.fault {
            self.fault_node(fault);
            return Err(PublishRejection::IllegalCandidate(fault));
        }
        // The gate stands between every candidate and publication — not the
        // planner. A violation discards the candidate and faults the node;
        // the previously published state stays visible and observable.
        if let Some(fault) = legal(&self.progress, &planned.candidate, &planned.kind) {
            self.fault_node(fault);
            return Err(PublishRejection::IllegalCandidate(fault));
        }
        if planned.completion || self.stability == Stability::Volatile {
            self.parked = None;
            self.install(
                planned.journal,
                planned.candidate,
                planned.bookkeeping,
                planned.diagnostic,
            )?;
            return Ok(PublishOutcome::Published {
                revision: self.progress.revision(),
                effects: planned.effects,
            });
        }
        if self.parked.is_some() {
            return Err(PublishRejection::TransitionOutstanding);
        }
        let intent = planned.intent;
        self.parked = Some(ParkedTransition {
            base: planned.base,
            candidate: planned.candidate,
            journal: planned.journal,
            effects: planned.effects,
            kind: planned.kind,
            bookkeeping: planned.bookkeeping,
            diagnostic: planned.diagnostic,
        });
        Ok(PublishOutcome::Parked {
            revision: planned.base,
            effects: vec![Effect::Persist(intent)],
        })
    }

    /// Installs a gated candidate: journal mutation first (so §5 invariant 1
    /// — published accepted equals the journal's frontier — holds of the
    /// pair), then the progress record, then the observation writes (B1),
    /// then the volatile bookkeeping.
    fn install(
        &mut self,
        mutation: JournalMutation,
        candidate: Progress,
        bookkeeping: Bookkeeping,
        diagnostic: Diagnostic,
    ) -> Result<(), PublishRejection> {
        let applied = match &mutation {
            JournalMutation::None => Ok(()),
            JournalMutation::Accept(entries) => self.journal.accept(entries),
            JournalMutation::InstallSuffix { from, suffix } => {
                self.journal.install_suffix(*from, suffix)
            }
        };
        if let Err(error) = applied {
            // The mutation was computed against a view of this very journal;
            // a refusal means the two durable records disagree about history.
            self.fault_node(Fault::ProgressJournalDivergence);
            return Err(PublishRejection::JournalRefused(error));
        }
        self.progress = candidate;
        self.observation.write(self.progress.to_snapshot());
        self.diagnostics.write(diagnostic);
        // The view-change update first: an install CLEARS the old view's
        // proposal records, and the records the install itself creates (the
        // installed uncommitted tail) must land after the wipe.
        match bookkeeping.view_change {
            ViewChangeUpdate::Unchanged => {}
            ViewChangeUpdate::Set(view_change) => self.view_change = Some(view_change),
            // The attempt is over; the primary's proposal records died with
            // the view it proposed in.
            ViewChangeUpdate::Clear => {
                self.view_change = None;
                self.proposals.clear();
            }
        }
        match bookkeeping.recovery {
            RecoveryUpdate::Unchanged => {}
            RecoveryUpdate::Set(attempt) => self.recovery = Some(attempt),
            RecoveryUpdate::Clear => self.recovery = None,
        }
        match bookkeeping.transfer {
            TransferUpdate::Unchanged => {}
            TransferUpdate::Set(fetch) => self.transfer = Some(fetch),
            TransferUpdate::Clear => self.transfer = None,
        }
        for (slot, proposal) in bookkeeping.proposals {
            self.proposals.insert(slot, proposal);
        }
        for (slot, node) in bookkeeping.oks {
            if let Some(proposal) = self.proposals.get_mut(&slot) {
                if !proposal.oks.contains(&node) {
                    proposal.oks.push(node);
                }
            }
        }
        for slot in bookkeeping.resolved {
            self.proposals.remove(&slot);
        }
        if let Some(at) = bookkeeping.activity {
            self.primary_activity = at;
        }
        Ok(())
    }

    /// Records a sticky fault and publishes it (§5 invariant 5, B1).
    ///
    /// `with_fault` fails only on an existing fault — every call site has
    /// already established `self.progress.fault().is_none()` — or on
    /// revision exhaustion, where the `u64` revision space is spent and no
    /// further transition of any kind, including the fault record, is
    /// representable (see [`ProgressError::RevisionExhausted`]).
    fn fault_node(&mut self, fault: Fault) {
        if let Ok(faulted) = self.progress.with_fault(fault) {
            self.progress = faulted;
            self.observation.write(self.progress.to_snapshot());
        }
    }

    /// The primary of `view` under its era's configuration, or `None` when
    /// the era is outside the retention window (the message is then
    /// unevaluable).
    fn primary_of(&self, view: ViewId) -> Option<NodeId> {
        self.progress
            .config()
            .record(view.era)?
            .config
            .primary(view.view)
    }

    /// The current era's configuration record, when the table can still
    /// name it.
    fn current_record(&self) -> Option<&EraRecord> {
        self.progress.config().record(self.progress.current().era)
    }

    /// Every member of the current configuration except this node, in
    /// succession-sequence order. Learners (weight 0, §8.4) included: they
    /// hold history even though they do not vote.
    fn backups(&self) -> Vec<NodeId> {
        match self.current_record() {
            Some(record) => record
                .config
                .order()
                .iter()
                .map(|member| member.node)
                .filter(|node| *node != self.own)
                .collect(),
            None => Vec::new(),
        }
    }

    /// A `Commit` datagram for `committed` to every other member of the
    /// current configuration (§13.3). `Effect::Send.era` is the era under
    /// which the sending decision was made (W1) — in normal operation, the
    /// current era.
    fn broadcast_commit(&self, committed: Slot) -> Vec<Effect> {
        let current = self.progress.current();
        let message = Message {
            header: Header {
                tag: Tag::Commit,
                view: current,
                slot: committed,
            },
            body: Body::Commit { committed },
        };
        self.backups()
            .into_iter()
            .map(|to| Effect::Send {
                to,
                era: current.era,
                message: message.clone(),
            })
            .collect()
    }

    /// The era proof for `era` (§8.7.8): the establishing operation and the
    /// slot it committed at, read from the node's bounded era records — the
    /// recipient checks the claim against its configuration history. The
    /// journal may already have reclaimed the establishing entry (S1).
    fn era_proof(&self, _journal: &J::View, era: Era) -> Result<EraProof, PlanRejection> {
        let record = self
            .progress
            .config()
            .record(era)
            .ok_or(PlanRejection::Progress(ProgressError::EraSlotDiscipline))?;
        Ok(EraProof {
            op: record.establishing_operation.clone(),
            committed_at: record.established_by,
        })
    }

    /// Whether an era proof matches the node's own records (§8.7.8): the
    /// claimed committed slot IS the era's establishing slot, and the
    /// retained era record holds the claimed operation.
    fn era_proof_ok(&self, _journal: &J::View, record: &EraRecord, proof: &EraProof) -> bool {
        proof.committed_at == record.established_by && proof.op == record.establishing_operation
    }

    /// The §13.1 bounded suffix: the newest entries of the history ending
    /// at `frontier`, packed newest-first until the budget would be
    /// exceeded, encoded ascending. `overlay` supplies entries the journal
    /// does not yet hold (the winner's just-selected suffix); entries below
    /// the overlay come from the journal's physically retained window.
    /// An entry larger than the remaining budget stops
    /// the packing immediately, down to empty, which §13.1 explicitly
    /// permits. The budget is never exceeded by a byte (W4:
    /// `Pack::packed_len` is normative), and the emitted suffix is
    /// always one contiguous ascending run: a packing that broke before
    /// exhausting the overlay never reaches the journal walk below it.
    fn bounded_suffix(
        &self,
        journal: &J::View,
        frontier: Slot,
        overlay: &[LogEntry],
    ) -> Vec<LogEntry> {
        let budget = self.knobs.view_change_budget;
        let mut picked: Vec<LogEntry> = Vec::new();
        let mut bytes = 0usize;
        for entry in overlay.iter().rev() {
            let len = entry.packed_len();
            let Some(total) = bytes.checked_add(len) else {
                break;
            };
            if total > budget {
                break;
            }
            picked.push(entry.clone());
            bytes = total;
        }
        // Below the overlay (or the whole range, without one): the
        // journal's retained window, newest first — but only when the
        // overlay packed whole. A packing that broke before exhausting
        // the overlay stops entirely (§13.1 permits a short suffix,
        // never a holed one): journal entries below the overlay's base
        // would sit a hole apart from the unpacked overlay entries
        // above, and a non-contiguous offer fails §13.1's shape rule at
        // every backup.
        let mut slot = match overlay.first() {
            Some(first) if picked.len() == overlay.len() => first.slot.prev(),
            Some(_) => None,
            None => Some(frontier),
        };
        let (first, last) = journal.retained();
        while let Some(cursor) = slot {
            if cursor < first || cursor > last {
                break;
            }
            let Some(entry) = journal.get(cursor) else {
                break;
            };
            let len = entry.packed_len();
            let Some(total) = bytes.checked_add(len) else {
                break;
            };
            if total > budget {
                break;
            }
            picked.push(entry.clone());
            bytes = total;
            slot = cursor.prev();
        }
        picked.reverse();
        picked
    }

    /// Verifies an offered suffix against the local journal and decides the
    /// journal mutation (§9.1's install, §13.1's sufficiency rule).
    ///
    /// The offer is constructible when every slot of the installed history
    /// is either already held with identical content or carried by the
    /// suffix: the suffix must reach back to the local frontier's successor
    /// or below, and every shared slot must agree. A shared slot at or
    /// below the local COMMITTED frontier that disagrees is
    /// [`SuffixCheck::Conflict`] — the one deliberate fault of the
    /// view-change path. A
    /// suffix that starts past the frontier's successor, or an empty suffix
    /// that cannot prove the installed history is already held, is
    /// [`SuffixCheck::Gap`]: the node stays fenced and waits for the
    /// missing range (state transfer, §10).
    fn check_suffix(
        &self,
        journal: &J::View,
        suffix: &[LogEntry],
        accepted: Slot,
        committed: Slot,
    ) -> SuffixCheck {
        let local_accepted = self.progress.accepted();
        let Some(base) = suffix.first().map(|entry| entry.slot) else {
            // No suffix at all (§13.1 permits it): adopt only when the
            // installed history is exactly the local one and every held
            // entry is committed — committed entries are quorum-identical,
            // so nothing needs checking. Anything else is unverifiable.
            if accepted == local_accepted && committed == local_accepted {
                return SuffixCheck::Install(JournalMutation::None);
            }
            let expected = local_accepted.next().unwrap_or(local_accepted);
            let got = accepted.next().unwrap_or(accepted);
            return SuffixCheck::Gap { expected, got };
        };
        if local_accepted.next() != Some(base) && base > local_accepted {
            return SuffixCheck::Gap {
                expected: local_accepted.next().unwrap_or(local_accepted),
                got: base,
            };
        }
        // Overlap: every shared slot must agree. The first disagreement at
        // or below the committed frontier is the safety breach; above it,
        // the divergent uncommitted tail is the protocol's own repair.
        let mut from = accepted.next().unwrap_or(accepted);
        for entry in suffix {
            if entry.slot > local_accepted {
                from = entry.slot;
                break;
            }
            match journal.get(entry.slot) {
                Some(held) if held == entry => {}
                Some(_) if entry.slot <= self.progress.committed() => {
                    return SuffixCheck::Conflict;
                }
                Some(_) => {
                    from = entry.slot;
                    break;
                }
                // Physically let go (S1): unverifiable, reported honestly.
                // The offer reached back PAST the slot the node cannot
                // verify, so `got` — the offer's base, per the
                // diagnostic's doc — sits at or below `expected` here.
                None => {
                    return SuffixCheck::Gap {
                        expected: entry.slot,
                        got: base,
                    };
                }
            }
        }
        if from > accepted {
            // Every offered slot agreed. A shorter installed history
            // discards the uncommitted tail by reinstalling the offered
            // suffix over it; an equal one needs no mutation at all.
            if accepted < local_accepted {
                return SuffixCheck::Install(JournalMutation::InstallSuffix {
                    from: base,
                    suffix: suffix.to_vec(),
                });
            }
            return SuffixCheck::Install(JournalMutation::None);
        }
        let skip = suffix
            .iter()
            .position(|entry| entry.slot == from)
            .expect("`from` names an offered slot");
        SuffixCheck::Install(JournalMutation::InstallSuffix {
            from,
            suffix: suffix[skip..].to_vec(),
        })
    }

    /// The peer-message dispatch (§4, §9): normal operation, the
    /// view-change exchange, recovery, and state transfer are live; the
    /// remaining tags' handlers are future work and the refusal is named.
    ///
    /// A same-view `Prepare` or `Commit` from the legitimate primary is
    /// proof of primary life: whatever its outcome (accept, gap, named
    /// drop), it refreshes the suspicion baseline (S4). A higher-view
    /// `Prepare`/`Commit` from the legitimate primary of the view it names
    /// is proof the node is stale (§10): the node fences into that view
    /// and fetches — but the message itself installs nothing (§13.4).
    fn plan_peer(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let header = message.header;
        let current = self.progress.current();
        let primary_life = match &message.body {
            // A same-view Prepare/Commit from the legitimate primary: the
            // primary is alive (a backup's baseline).
            Body::Prepare { .. } | Body::Commit { .. } => {
                header.view == current && self.primary_of(header.view) == Some(from)
            }
            // A same-view PrepareOk at the serving primary: its proposals
            // are landing, so the view is alive (the primary's baseline).
            Body::PrepareOk {} => {
                header.view == current
                    && self.progress.status() == Status::Normal
                    && self.primary_of(current) == Some(self.own)
            }
            // A same-view transfer chunk from the legitimate primary: the
            // view it answers a fetch under is alive.
            Body::NewState { .. } => {
                header.view == current && self.primary_of(header.view) == Some(from)
            }
            _ => false,
        };
        let plan = match &message.body {
            Body::Prepare { entry, committed } => {
                self.plan_prepare(journal, from, message, entry, *committed, at, kind)
            }
            Body::PrepareOk {} => self.plan_prepare_ok(journal, from, message, kind),
            Body::Commit { committed } => {
                self.plan_commit(journal, from, message, *committed, at, kind)
            }
            Body::StartViewChange {} => {
                self.plan_start_view_change(journal, from, message, at, kind)
            }
            Body::DoViewChange {
                retained,
                accepted,
                committed,
                suffix,
                evidence,
                era_proof,
            } => self.plan_do_view_change(
                journal, from, message, *retained, *accepted, *committed, suffix, *evidence,
                era_proof, at, kind,
            ),
            Body::StartView {
                suffix,
                accepted,
                committed,
                era_proof,
            } => self.plan_start_view(
                journal, from, message, suffix, *accepted, *committed, era_proof, at, kind,
            ),
            Body::Recovery { nonce } => self.plan_recovery_request(from, *nonce, kind),
            Body::RecoveryResponse {
                nonce,
                view,
                accepted,
                committed,
                suffix,
            } => self.plan_recovery_response(
                journal, from, *nonce, *view, *accepted, *committed, suffix, at, kind,
            ),
            Body::GetState { from: fetch_from } => {
                self.plan_get_state(journal, from, message, *fetch_from, kind)
            }
            Body::NewState {
                entries,
                through,
                committed,
                more,
            } => self.plan_new_state(
                journal, from, message, entries, *through, *committed, *more, kind,
            ),
            Body::PlannedViewChange {} => Err(PlanRejection::Unsupported { input: kind }),
        }?;
        Ok(if primary_life {
            plan.with_activity(at)
        } else {
            plan
        })
    }

    /// The proposal records for the uncommitted tail of an installed
    /// history (§9.1): an installed slot carries no `PrepareOk` votes yet,
    /// and a `PrepareOk` for a HIGHER slot vouches for it (acceptance is
    /// prefix-contiguous) — without the record, an installed tail could
    /// never commit.
    fn installed_proposals(
        &self,
        journal: &J::View,
        overlay: &[LogEntry],
        committed: Slot,
        accepted: Slot,
    ) -> Vec<(Slot, Proposal)> {
        let mut proposals = Vec::new();
        let mut slot = committed.next();
        while let Some(cursor) = slot {
            if cursor > accepted {
                break;
            }
            let entry = match overlay.iter().find(|entry| entry.slot == cursor) {
                Some(entry) => Some(entry),
                None => journal.get(cursor),
            };
            if let Some(entry) = entry {
                if let Payload::Operation { .. } = &entry.payload {
                    proposals.push((cursor, Proposal { oks: Vec::new() }));
                }
            }
            slot = cursor.next();
        }
        proposals
    }
}

/// The §1.3 ranking rule (spec §9.2's load-bearing sentence): the selected
/// history is the greatest by `retained` view FIRST, then `accepted`
/// frontier. Ranking by `accepted` alone is the published counterexample —
/// a longer history retained from an EARLIER view can miss entries
/// committed under a later one. Ties take the lowest sender id, so every
/// honest new primary computes the same selection from the same evidence.
fn select_history(evidence: &BTreeMap<NodeId, Evidence>) -> Evidence {
    let mut best: Option<&Evidence> = None;
    for member in evidence.values() {
        let better = match best {
            None => true,
            Some(incumbent) => {
                (member.retained, member.accepted) > (incumbent.retained, incumbent.accepted)
            }
        };
        if better {
            best = Some(member);
        }
    }
    best.expect("an evidence quorum is never empty").clone()
}

/// The structural shape of a view-change suffix (§13.1): a contiguous
/// ascending run of real slots ending exactly at the accepted frontier.
/// Empty is always well-formed — a budget may admit no entry at all.
fn suffix_shape_ok(suffix: &[LogEntry], accepted: Slot) -> bool {
    let mut expected: Option<Slot> = None;
    for entry in suffix {
        let legal = match expected {
            None => entry.slot != Slot::NONE,
            Some(next) => entry.slot == next,
        };
        if !legal {
            return false;
        }
        expected = entry.slot.next();
    }
    match suffix.last() {
        Some(last) => last.slot == accepted,
        None => true,
    }
}

/// The lazy reclamation wiring over the default journal — inherent to the
/// concrete type, because the portable `Journal` contract deliberately
/// does not name reclamation (S1, and `tests/journal_contract.rs` enforces
/// the omission mechanically).
impl Replica<SegmentedLog, crate::quorum::WeightedMajority> {
    /// Offers the published checkpoint frontier to the journal as the
    /// reclamation authorization (§4, S1): whole slabs whose final slot
    /// the frontier covers are dropped — never the tail, never a slot past
    /// it. The host calls this opportunistically (the lazy moment is the
    /// next append after a checkpoint publishes); with no checkpoint
    /// published it is a no-op however old the history, because the
    /// checkpoint is the SOLE authorization. The slot space is a protocol
    /// fact (§4): reclamation never moves the published frontiers, and the
    /// failure domains it enables (state shortfall, recovery evidence, the
    /// §13.1 transfer) are surfaced by the ordinary paths.
    pub fn reclaim_journal(&mut self) {
        let checkpoint = self.progress.checkpoint();
        if checkpoint == Slot::NONE {
            return;
        }
        self.journal.reclaim_through(checkpoint);
    }
}

/// The progress half of the persistence intent: [`ProgressIntent::Record`]
/// when any durable §5 field moved, [`ProgressIntent::Unchanged`] when only
/// the revision did — the revision is §12 interval bookkeeping, not part of
/// the durable record.
fn progress_intent(old: &Progress, new: &Progress) -> ProgressIntent {
    let changed = old.current() != new.current()
        || old.retained() != new.retained()
        || old.status() != new.status()
        || old.accepted() != new.accepted()
        || old.committed() != new.committed()
        || old.applied() != new.applied()
        || old.checkpoint() != new.checkpoint()
        || old.fault() != new.fault();
    if changed {
        ProgressIntent::Record
    } else {
        ProgressIntent::Unchanged
    }
}
