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
//! host tick, and for a recovery input that tick *is* the recovery nonce
//! (§6.1, decision S4). `Input::Tick` exists as an ordinary event, not as a
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
//! VRR-2012 §4 is live: the client table (§9.2's two-exchange answer), `Prepare`/`PrepareOk`/`Commit` with
//! commit-frontier piggybacking (§13.3), the Apply/Applied/Reply boundary
//! (§11), and the bootstrap from the fenced `Recovering` genesis state (the
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
//! [`Fault::IllegalTransition`]. The active fetch for a suffix the recipient
//! cannot construct history from belongs to state transfer (§10); the interim
//! drop is
//! [`Diagnostic::GapDetected`], never a fault.
//!
//! # The client table as evidence (§9.2)
//!
//! §9.2's guarantee completed: the client table is protocol evidence. Every
//! client entry carries its `(client, request)` identity, so a backup
//! records a row from the `Prepare` it accepts and any node rebuilds rows
//! from the history it installs. The table rides `DoViewChange`, the new
//! primary merges the evidence quorum's tables (per client the greatest
//! `last_request`; a tie prefers the row WITH a cached result — a total
//! deterministic function of the evidence set), extends the merge with the
//! rows the installed history implies, and `StartView` distributes it;
//! recipients install it in place of their own. The soundness claim this
//! buys: **a request is replied-to at most once per unique result, and the
//! log holds at most one entry per accepted request number per client.** A
//! retry of a committed request whose result no quorum member cached is
//! answered by re-driving `Effect::Apply` for the existing slot — never by
//! re-appending (§11 puts idempotence on the host, which recognises the
//! re-drive by the slot being at or below its applied frontier). An entry
//! that died uncommitted is genuinely new: no reply was ever produced for
//! it, and the retry is accepted fresh. The table is charged against the
//! §13.1 budget FIRST and is never truncated; the suffix shrinks to make
//! room, down to empty, which §13.1 explicitly permits.
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
use crate::ids::{ClientId, Era, Fault, NodeId, RequestNumber, Slot, Tick, View, ViewId};
use crate::invariant::{InputKind, legal};
use crate::journal::{Journal, JournalError, JournalView, LogEntry, Payload};
use crate::message::{Body, ClientRow as WireRow, EraProof, EvidenceKind, Message};
use crate::observe::{Diagnostic, Observation};
use crate::progress::{Progress, ProgressError, ProgressSnapshot, Status};
use crate::quorum::{QuorumError, QuorumStrategy, Role, validate_era};
use crate::wire::{Header, Pack, Tag};

/// One host event with the host tick attached (§6, S4).
///
/// `at` is host observation metadata sampled when the host began dispatching
/// the event — never a timestamp received from a peer — and for
/// [`Input::Recover`] it is also the recovery nonce (§6.1).
#[derive(Clone, Debug)]
pub struct TimedInput {
    /// The host tick at dispatch time; the recovery nonce for
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
    /// A client request submitted to this node (§6, §11).
    Client {
        /// The client identity, for the duplicate-suppression table.
        client: ClientId,
        /// The client's monotonic request number.
        request: RequestNumber,
        /// Opaque application bytes (§11).
        payload: Box<[u8]>,
    },
    /// A host timer event (S4). Drives the bootstrap self-promotion of the
    /// genesis primary (see the `plan_tick` handler); the view-change and
    /// recovery timeout bookkeeping belongs to those paths. On a node with
    /// nothing to decide it remains the smallest honest transition: no
    /// protocol state moves, and the interval machinery — revision, gate,
    /// stability handshake — is genuinely exercised by it.
    Tick,
    /// Begin a recovery attempt (§10). The nonce is [`TimedInput::at`] (S4).
    Recover,
    /// The host's report on the one outstanding [`PersistenceIntent`] (S2/S3).
    StabilityConfirmation {
        /// The base revision of the transition being confirmed: the
        /// [`PersistenceIntent::revision`] the core emitted at `publish`.
        revision: u64,
        /// The three-way outcome (S3).
        result: StabilityResult,
    },
    /// The application incorporated a committed slot (§11.2).
    Applied {
        /// The slot whose application completed.
        slot: Slot,
        /// The application's result bytes, opaque to the core.
        result: Box<[u8]>,
    },
    /// The host checkpointed application state through a slot (§5's checkpoint
    /// frontier).
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
    /// The host forced entry into a view (§12's host-declared fencing path).
    AdminForceView {
        /// The view to enter.
        view: View,
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
            Input::Client { .. } => InputKind::ClientRequest,
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
    /// A client request reached a node that is not the `Normal` primary of
    /// its current view (§4): a backup, a fenced `Recovering` node, or the
    /// primary of some other view. Carries the node's current view and the
    /// primary of that view (when the configuration can name one) so the
    /// host can redirect the client (§13.4's convergence hint applied to
    /// the client path).
    NotPrimary {
        /// The node's current view.
        view: ViewId,
        /// The primary of `view` under its era's configuration.
        primary: Option<NodeId>,
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
    /// An [`Input::Applied`] for a slot that committed a system payload
    /// (§11's boundary is not theirs; the core never emitted an `Apply` for
    /// it, so the host's report is incoherent).
    AppliedSystemSlot {
        /// The misreported slot.
        slot: Slot,
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

/// One row of the client table (§9.2's two-exchange answer).
///
/// Volatile, like the paper's: lost on crash. But it is not LOST to the
/// protocol (§9.2): the table rides `DoViewChange` as evidence, the new
/// primary merges the quorum's tables, and `StartView` distributes the
/// merge — and because every client entry carries its `(client, request)`
/// identity, an install extends the merged table from the history itself.
/// The table is what makes a retry idempotent across a view change; it is
/// not what makes the record durable.
#[derive(Clone, PartialEq, Eq, Debug)]
struct ClientRow {
    /// The highest request number accepted from this client.
    last: RequestNumber,
    /// The cached result once the request's slot applied; `None` while the
    /// request is in flight.
    result: Option<Box<[u8]>>,
}

/// The wire form of the node's client table: rows sorted by client (the
/// `BTreeMap` iteration order), no duplicates — the canonical shape the
/// decoder enforces at the boundary.
fn wire_table(clients: &BTreeMap<ClientId, ClientRow>) -> Vec<WireRow> {
    clients
        .iter()
        .map(|(client, row)| WireRow {
            client: *client,
            last_request: row.last,
            result: row.result.clone(),
        })
        .collect()
}

/// The packed length of a client table in its codec form (u32 count ++
/// rows): what the §13.1 budget charges BEFORE the suffix. The table is
/// never truncated to fit — it is safety evidence; the suffix shrinks, down
/// to empty, which §13.1 explicitly permits.
fn wire_table_packed_len(table: &[WireRow]) -> usize {
    4 + table.iter().map(Pack::packed_len).sum::<usize>()
}

/// The §9.2 merge the new primary computes over the evidence quorum's
/// tables: per client, the row with the greatest `last_request`; a tie
/// prefers the row WITH a cached result. The comparison is a total order
/// per client, so the result is a deterministic function of the evidence
/// SET — delivery order cannot change it. Greatest-request wins wholesale:
/// the merge keeps the row the client protocol will actually retry against,
/// and a newer row without a result never re-executes anything (its request
/// superseded the cached one before it committed, or its entry died
/// uncommitted — the genuinely-new case of the retry rules).
fn merge_client_tables<'a>(tables: impl IntoIterator<Item = &'a [WireRow]>) -> Vec<WireRow> {
    let mut merged: BTreeMap<ClientId, WireRow> = BTreeMap::new();
    for row in tables.into_iter().flatten() {
        merged
            .entry(row.client)
            .and_modify(|incumbent| {
                if (row.last_request, row.result.is_some())
                    > (incumbent.last_request, incumbent.result.is_some())
                {
                    *incumbent = row.clone();
                }
            })
            .or_insert_with(|| row.clone());
    }
    merged.into_values().collect()
}

/// The canonical-shape check for a table arriving in a message (the
/// decoder enforces it on the wire; a host-fabricated message is checked
/// here): strictly ascending clients, hence no duplicates.
fn wire_table_shape_ok(table: &[WireRow]) -> bool {
    table.windows(2).all(|pair| pair[0].client < pair[1].client)
}

/// The local (map-entry) form of a canonical wire table, for the
/// install-time bookkeeping's wholesale replacement.
fn local_rows(table: &[WireRow]) -> Vec<(ClientId, ClientRow)> {
    table
        .iter()
        .map(|row| {
            (
                row.client,
                ClientRow {
                    last: row.last_request,
                    result: row.result.clone(),
                },
            )
        })
        .collect()
}

/// The primary's record of one proposed slot, from acceptance until the slot
/// applies: the client request the slot answers, and the distinct backups
/// whose `PrepareOk`s have arrived. The primary's own vote is implicit.
///
/// A slot installed by a view change (§9.1) gets its record re-seeded from
/// the entry's own identity (§9.2): the identity rides in the entry, so
/// the new primary can still answer the request's commit with a `Reply`,
/// and a `PrepareOk` for a HIGHER slot can vouch for this one: acceptance
/// is prefix-contiguous, so an acknowledgement vouches for every lower
/// uncommitted slot (VRR-2012 §4's cumulative acknowledgement) — without
/// the record, an installed tail could never commit.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Proposal {
    /// The client the slot answers, when this node proposed it.
    client: Option<ClientId>,
    /// The client's request number, when this node proposed it.
    request: Option<RequestNumber>,
    /// The distinct `PrepareOk` senders recorded so far.
    oks: Vec<NodeId>,
}

/// One replica's `DoViewChange` evidence, as collected by the designated new
/// primary (§9.1): the provenance and frontiers the ranking rule (§1.3)
/// compares, the bounded suffix the selection may need (§13.1), and the
/// sender's client table (§9.2 — the table is evidence, or the
/// merged table at the new primary would be empty and a committed request's
/// retry would be appended a second time). The era proof is validated at
/// receipt and not stored — after validation it has said everything it had
/// to say.
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
    /// The sender's client table, canonical shape (validated at receipt).
    client_table: Vec<WireRow>,
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
/// state` made concrete for the tables that are not part of the durable §5
/// record. Computed by `plan`, carried by the transition, applied by
/// `install` — and by the completion of a parked transition, so a `Failed`
/// confirmation discards the table updates with the candidate (S3).
#[derive(Clone, Debug, Default)]
struct Bookkeeping {
    /// Client-table rows to install or replace.
    clients: Vec<(ClientId, ClientRow)>,
    /// Wholesale client-table replacement (§9.2): a view-change install
    /// adopts the merged table — it dominates the local one because it saw
    /// a view-change quorum's rows plus the installed history. Applied
    /// before the per-row `clients` upserts.
    table_replace: Option<Vec<(ClientId, ClientRow)>>,
    /// New proposal records.
    proposals: Vec<(Slot, Proposal)>,
    /// `PrepareOk` senders to record against outstanding slots.
    oks: Vec<(Slot, NodeId)>,
    /// Slots whose proposals are resolved by application.
    resolved: Vec<Slot>,
    /// Committed slots whose `Apply` is an unknown-result re-drive
    /// (§9.2): the completion is accepted out of the applied-frontier
    /// order and answered with a `Reply`, never a frontier move.
    redrive_begin: Vec<Slot>,
    /// Re-driven slots whose completion the host reported.
    redrive_done: Vec<Slot>,
    /// The view-change attempt update.
    view_change: ViewChangeUpdate,
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
    /// The client table (§9.2), keyed by client. Volatile: lost on crash —
    /// but carried through every view change as protocol evidence (§9.2:
    /// `DoViewChange` ships it, the new primary merges the quorum's tables,
    /// `StartView` distributes the merge), so the loss can never strand a
    /// committed request's reply.
    clients: BTreeMap<ClientId, ClientRow>,
    /// The primary's outstanding and unapplied proposals, keyed by slot.
    /// Volatile: a node that loses it reopens fenced `Recovering` (§5's
    /// boot rule), so the loss can never masquerade as authority.
    proposals: BTreeMap<Slot, Proposal>,
    /// Committed slots with an unknown-result re-drive in flight (§9.2):
    /// the retry of a committed request whose result no evidence quorum
    /// member had cached is answered by re-driving `Apply` for the existing
    /// slot — never by re-appending — and the completion is recognised
    /// against this set. Volatile like the table it serves.
    redrives: BTreeSet<Slot>,
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
    /// `accepted == committed == Slot(2)`; `applied == checkpoint ==
    /// Slot(0)`; status [`Status::Recovering`] — the genesis ruling (§1.3),
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
        let progress = Progress::reconstitute(
            view,
            view,
            Status::Recovering,
            INIT_SLOT,
            INIT_SLOT,
            Slot::FIRST,
            Slot::FIRST,
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
        let frontier = journal.view().accepted().unwrap_or(Slot::FIRST);
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
            clients: BTreeMap::new(),
            proposals: BTreeMap::new(),
            redrives: BTreeSet::new(),
            progress,
            journal,
            strategy,
            stability,
            parked: None,
            knobs,
            primary_activity: Tick(0),
            view_change: None,
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
        let frontier = journal.accepted().unwrap_or(Slot::FIRST);
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
            Input::Client {
                client,
                request,
                payload,
            } => {
                self.refuse_if_parked()?;
                self.plan_client(journal, *client, *request, payload, input.at)
            }
            Input::Peer { from, message } => {
                self.refuse_if_parked()?;
                self.plan_peer(journal, *from, message, input.at, input.event.kind())
            }
            Input::Applied { slot, result } => {
                self.refuse_if_parked()?;
                self.plan_applied(journal, *slot, result)
            }
            Input::Recover
            | Input::Checkpointed { .. }
            | Input::Reconfigure { .. }
            | Input::AdminForceView { .. } => {
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

    /// Enters the view change for `target` (VRR-2012 §5, spec §9.1): the
    /// durable view advances and fences (`Progress` keeps `retained` — the
    /// history is not re-selected by entering), the fence vote set starts
    /// at the node's own plus any already-heard `StartViewChange` senders,
    /// and the node's own `StartViewChange` broadcasts to every other
    /// member. The fence quorum (`Role::Fence`, Q1) may complete at entry —
    /// the joining `StartViewChange` can be the one that closes it.
    fn enter_view_change(
        &self,
        journal: &J::View,
        target: ViewId,
        heard: BTreeSet<NodeId>,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let candidate = self
            .progress
            .with_view_change(target)
            .map_err(PlanRejection::Progress)?;
        let mut fences = heard;
        fences.insert(self.own);
        let message = Message {
            header: Header {
                tag: Tag::StartViewChange,
                view: target,
                slot: Slot::FIRST,
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

    /// Runs the attempt forward after its volatile state changed: fence
    /// quorum first (§9.1's ordering — evidence follows the fence), then,
    /// at the designated new primary, the evidence quorum (`Role::View
    /// Change`, Q1) and the install. Every quorum question goes to the
    /// strategy; no count is computed here.
    fn continue_view_change(
        &self,
        journal: &J::View,
        candidate: Progress,
        mut view_change: ViewChangeVolatile,
        mut effects: Vec<Effect>,
        at: Tick,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let target = view_change.target;
        let Some(record) = self.progress.config().record(target.era) else {
            return self.drop_plan(Diagnostic::UnevaluableEra { era: target.era }, kind);
        };
        // The fence: once a `Role::Fence` quorum holds, the node records
        // its own evidence and reports it to the designated new primary
        // (§9.1). A node that IS the new primary keeps its evidence local.
        if !view_change.evidence.contains_key(&self.own) {
            let fences: Vec<NodeId> = view_change.fences.iter().copied().collect();
            if self
                .strategy
                .is_quorum(Role::Fence, &record.config, &fences)
            {
                let own = self.own_evidence(journal);
                view_change.evidence.insert(self.own, own);
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
            if let Some(selected) = view_change.selected.clone() {
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
                    // constructed from the collected evidence — the missing
                    // range must be fetched by state transfer (§10) before
                    // `StartView`.
                    // The attempt and its selection are kept; the drop is
                    // named, never a fault.
                    WinOutcome::Insufficient {
                        expected,
                        got,
                        effects,
                    } => {
                        return Ok(self
                            .candidate_plan(candidate, JournalMutation::None, effects, kind, false)
                            .with_bookkeeping(Bookkeeping {
                                view_change: ViewChangeUpdate::Set(view_change),
                                ..Bookkeeping::default()
                            })
                            .with_diagnostic(Diagnostic::GapDetected { expected, got }));
                    }
                }
            }
        }
        Ok(self
            .candidate_plan(candidate, JournalMutation::None, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                view_change: ViewChangeUpdate::Set(view_change),
                ..Bookkeeping::default()
            }))
    }

    /// The node's own evidence for the attempt at `target` (§9.1): the
    /// retained provenance, both frontiers, the bounded suffix (§13.1), the
    /// client table (§9.2 — charged against the suffix budget,
    /// never truncated), ordinary kind — and the era proof, attached only
    /// when the evidence is sent (§8.7.8).
    fn own_evidence(&self, journal: &J::View) -> Evidence {
        let client_table = wire_table(&self.clients);
        let reserve = wire_table_packed_len(&client_table);
        Evidence {
            retained: self.progress.retained(),
            accepted: self.progress.accepted(),
            committed: self.progress.committed(),
            suffix: self.bounded_suffix(journal, self.progress.accepted(), &[], reserve),
            client_table,
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
    ) -> Result<Effect, PlanRejection> {
        let own = view_change
            .evidence
            .get(&self.own)
            .expect("own evidence is recorded before it is sent");
        let to = self
            .primary_of(target)
            .ok_or(PlanRejection::Progress(ProgressError::EraSlotDiscipline))?;
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
                    client_table: own.client_table.clone(),
                    evidence: EvidenceKind::Ordinary,
                    era_proof: self.era_proof(journal, target.era)?,
                },
            },
        })
    }

    /// The era proof for `era` (§8.7.8): the establishing operation and the
    /// slot it committed at, read from the node's own records — the
    /// recipient checks the claim against its configuration history.
    fn era_proof(&self, journal: &J::View, era: Era) -> Result<EraProof, PlanRejection> {
        let record = self
            .progress
            .config()
            .record(era)
            .ok_or(PlanRejection::Progress(ProgressError::EraSlotDiscipline))?;
        let entry =
            journal
                .get(record.established_by)
                .ok_or(PlanRejection::JournalEntryUnavailable {
                    slot: record.established_by,
                })?;
        let Payload::System(op) = &entry.payload else {
            return Err(PlanRejection::Progress(ProgressError::EraSlotDiscipline));
        };
        Ok(EraProof {
            op: op.clone(),
            committed_at: record.established_by,
        })
    }

    /// The §13.1 bounded suffix: the newest entries of the history ending
    /// at `frontier`, packed newest-first until the budget would be
    /// exceeded, encoded ascending. `overlay` supplies entries the journal
    /// does not yet hold (the winner's just-selected suffix); entries below
    /// the overlay come from the journal's physically retained window.
    /// `reserve` is the packed length of the client table riding in the
    /// same message (§9.2): the table is charged against the §13.1
    /// budget FIRST and is never truncated — it is safety evidence — so the
    /// suffix packs inside what remains, down to empty, which §13.1
    /// explicitly permits. An entry larger than the remaining budget stops
    /// the packing immediately. The budget is never exceeded by a byte (W4:
    /// `Pack::packed_len` is normative).
    fn bounded_suffix(
        &self,
        journal: &J::View,
        frontier: Slot,
        overlay: &[LogEntry],
        reserve: usize,
    ) -> Vec<LogEntry> {
        let budget = self.knobs.view_change_budget.saturating_sub(reserve);
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
        // journal's retained window, newest first.
        let mut slot = match overlay.first() {
            Some(first) => first.slot.prev(),
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

    /// The slot of `(client, request)`'s entry in the installed history,
    /// found greatest-slot-first (a duplicate would be the bug the §9.2
    /// structural tests hunt; the newest is the one the protocol believes).
    /// `None` means the entry is not in the history this node holds — the
    /// genuinely-new case of the retry rules.
    fn find_client_entry(
        &self,
        journal: &J::View,
        client: ClientId,
        request: RequestNumber,
    ) -> Option<Slot> {
        let (first, last) = journal.retained();
        let mut slot = Some(last);
        while let Some(cursor) = slot {
            if cursor < first {
                break;
            }
            if let Some(entry) = journal.get(cursor)
                && let Payload::Client {
                    client: entry_client,
                    request: entry_request,
                    ..
                } = &entry.payload
                && *entry_client == client
                && *entry_request == request
            {
                return Some(cursor);
            }
            slot = cursor.prev();
        }
        None
    }

    /// The client-table rows the installed history itself implies
    /// (§9.2): per client, the greatest request number among its client
    /// entries, with no result — history carries identity, not outcomes.
    /// `overlay` supplies the suffix being installed in the same transition
    /// (the journal does not hold it yet). Merged INTO a view-change table
    /// with the usual precedence, these rows lose every tie to a cached
    /// result and every contest with a greater evidence row — they exist so
    /// a row whose evidence copy died with a restarted sender still knows
    /// its request.
    fn history_client_rows(
        &self,
        journal: &J::View,
        overlay: &[LogEntry],
        accepted: Slot,
    ) -> Vec<WireRow> {
        let (first, _last) = journal.retained();
        let mut rows: BTreeMap<ClientId, RequestNumber> = BTreeMap::new();
        let mut slot = Some(first);
        while let Some(cursor) = slot {
            if cursor > accepted {
                break;
            }
            let entry = match overlay.iter().find(|entry| entry.slot == cursor) {
                Some(entry) => Some(entry),
                None => journal.get(cursor),
            };
            if let Some(entry) = entry
                && let Payload::Client {
                    client, request, ..
                } = &entry.payload
            {
                rows.entry(*client)
                    .and_modify(|last| *last = (*last).max(*request))
                    .or_insert(*request);
            }
            slot = cursor.next();
        }
        rows.into_iter()
            .map(|(client, request)| WireRow {
                client,
                last_request: request,
                result: None,
            })
            .collect()
    }

    /// The client-table half of a view-change install (§9.2): the merged
    /// evidence table — the quorum's rows at the winner, the offered table
    /// at a `StartView` recipient — extended with the rows the installed
    /// history implies. Canonical wire form; [`local_rows`] converts for
    /// the bookkeeping. The extension is what lets a row survive when every
    /// evidence sender that held it restarted (its table wiped) but its
    /// entry is in the installed history.
    fn installed_table(
        &self,
        journal: &J::View,
        overlay: &[LogEntry],
        accepted: Slot,
        merged: &[WireRow],
    ) -> Vec<WireRow> {
        let history = self.history_client_rows(journal, overlay, accepted);
        merge_client_tables([merged, &history])
    }

    /// The proposal records for the uncommitted client tail of an installed
    /// history (§9.2): identity now rides in the entry, so a won view's
    /// tail keeps its client identity and its commit can be answered with a
    /// `Reply` when it applies — the normal-operation "no identity, no
    /// reply" gap is closed.
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
            if let Some(entry) = entry
                && let Payload::Client {
                    client, request, ..
                } = &entry.payload
            {
                proposals.push((
                    cursor,
                    Proposal {
                        client: Some(*client),
                        request: Some(*request),
                        oks: Vec::new(),
                    },
                ));
            }
            slot = cursor.next();
        }
        proposals
    }

    /// The primary's client-request handler (§4, §6): the client table
    /// first, then acceptance.
    ///
    /// Client-table semantics (§9.2's two-exchange answer; every branch
    /// pinned by
    /// `tests/normal_operation.rs`, the view-change survival of every
    /// branch by `tests/view_change_client_table.rs`):
    ///
    /// - `request == last` with a cached result: re-emit the cached
    ///   `Effect::Reply`; no new log entry (idempotent retry).
    /// - `request == last` with no cached result: the row survived a view
    ///   change but the result did not (the §9.2 unknown-result case).
    ///   The installed history decides, and the answer is NEVER a silent
    ///   re-append of a committed entry:
    ///   - the entry is in history at or below the committed frontier:
    ///     re-drive `Effect::Apply` for the existing slot (§11 puts
    ///     idempotence on the host, which recognises the re-drive by the
    ///     slot being at or below its applied frontier — no schema
    ///     change); the completion is answered with the `Reply`.
    ///   - the entry is in history above the committed frontier: it is
    ///     live in the pipe — drop, exactly as an in-flight duplicate.
    ///     Re-appending would put two entries for one `(client, request)`
    ///     in one history, and the tail still commits and replies through
    ///     the ordinary path.
    ///   - no entry in history: the entry died uncommitted with the old
    ///     primary and no client-visible result was ever produced, so the
    ///     request IS genuinely new — accept it, replacing the row.
    /// - `request == last + 1`: accept (a new client's `last` is implicitly
    ///   0, so the client protocol numbers requests from 1).
    /// - anything else (`< last`, or `> last + 1`): drop silently. The
    ///   client protocol is one-outstanding monotonic; a gap is a client
    ///   violation, not a node fault.
    ///
    /// Only a `Normal` node with `config.primary(current_view) == own`
    /// accepts; every other node answers [`PlanRejection::NotPrimary`].
    fn plan_client(
        &self,
        journal: &J::View,
        client: ClientId,
        request: RequestNumber,
        payload: &[u8],
        at: Tick,
    ) -> Result<PlannedTransition, PlanRejection> {
        let current = self.progress.current();
        let record = self
            .current_record()
            .ok_or(PlanRejection::Progress(ProgressError::EraSlotDiscipline))?;
        let is_primary = self.progress.status() == Status::Normal
            && record.config.primary(current.view) == Some(self.own);
        if !is_primary {
            // Redirection (§13.4's convergence hint): the node names its
            // current view and the primary of that view, so the host can
            // point the client at the node this cluster would serve from.
            return Err(PlanRejection::NotPrimary {
                view: current,
                primary: record.config.primary(current.view),
            });
        }
        /// The client-table verdict for one request.
        enum Verdict {
            /// Re-emit the cached result.
            Cached(Box<[u8]>),
            /// Silent drop: in-flight duplicate or out of the window.
            Drop,
            /// Re-drive `Apply` for the committed slot the request already
            /// occupies (the §9.2 unknown-result case).
            Redrive(Slot),
            /// Assign the next slot and replicate.
            Accept,
        }
        let verdict = match self.clients.get(&client) {
            Some(row) if request == row.last => match &row.result {
                Some(result) => Verdict::Cached(result.clone()),
                None => match self.find_client_entry(journal, client, request) {
                    Some(slot) if slot <= self.progress.committed() => Verdict::Redrive(slot),
                    // In history above the committed frontier: live in the
                    // pipe. Not in history: died uncommitted — genuinely new.
                    Some(_) => Verdict::Drop,
                    None => Verdict::Accept,
                },
            },
            Some(row) if Some(request.0) == row.last.0.checked_add(1) => Verdict::Accept,
            Some(_) => Verdict::Drop,
            None if request.0 == 1 => Verdict::Accept,
            None => Verdict::Drop,
        };
        match verdict {
            Verdict::Cached(result) => {
                let candidate = self.identity_candidate()?;
                let effects = vec![Effect::Reply {
                    client,
                    request,
                    result,
                }];
                Ok(self.candidate_plan(
                    candidate,
                    JournalMutation::None,
                    effects,
                    InputKind::ClientRequest,
                    false,
                ))
            }
            Verdict::Drop => {
                let candidate = self.identity_candidate()?;
                Ok(self.candidate_plan(
                    candidate,
                    JournalMutation::None,
                    Vec::new(),
                    InputKind::ClientRequest,
                    false,
                ))
            }
            Verdict::Redrive(slot) => {
                let entry = journal
                    .get(slot)
                    .ok_or(PlanRejection::JournalEntryUnavailable { slot })?;
                let Payload::Client { payload, .. } = &entry.payload else {
                    // `find_client_entry` matched a client payload at this
                    // slot in the same view; unreachable.
                    return Err(PlanRejection::Progress(ProgressError::EraSlotDiscipline));
                };
                let candidate = self.identity_candidate()?;
                let effects = vec![Effect::Apply {
                    slot,
                    payload: payload.clone(),
                }];
                Ok(self
                    .candidate_plan(
                        candidate,
                        JournalMutation::None,
                        effects,
                        InputKind::ClientRequest,
                        false,
                    )
                    .with_bookkeeping(Bookkeeping {
                        redrive_begin: vec![slot],
                        ..Bookkeeping::default()
                    }))
            }
            Verdict::Accept => {
                let slot = self
                    .progress
                    .accepted()
                    .next()
                    .ok_or(PlanRejection::SlotSpaceExhausted)?;
                let entry = LogEntry {
                    slot,
                    era: current.era,
                    payload: Payload::Client {
                        client,
                        request,
                        payload: payload.into(),
                    },
                };
                let candidate = self.candidate_with(
                    self.progress.status(),
                    slot,
                    self.progress.committed(),
                    self.progress.applied(),
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
                let effects = self
                    .backups()
                    .into_iter()
                    .map(|to| Effect::Send {
                        to,
                        era: current.era,
                        message: prepare.clone(),
                    })
                    .collect();
                let bookkeeping = Bookkeeping {
                    clients: vec![(
                        client,
                        ClientRow {
                            last: request,
                            result: None,
                        },
                    )],
                    proposals: vec![(
                        slot,
                        Proposal {
                            client: Some(client),
                            request: Some(request),
                            oks: Vec::new(),
                        },
                    )],
                    // The primary's own proposal is proof of its life: the
                    // suspicion baseline refreshes on exactly the work that
                    // keeps the view alive (S4).
                    activity: Some(at),
                    ..Bookkeeping::default()
                };
                Ok(self
                    .candidate_plan(
                        candidate,
                        JournalMutation::Accept(vec![entry]),
                        effects,
                        InputKind::ClientRequest,
                        false,
                    )
                    .with_bookkeeping(bookkeeping))
            }
        }
    }

    /// The peer-message dispatch (§4, §9): normal operation and the
    /// view-change exchange are live; every other tag's handler is future
    /// work and the refusal is named.
    ///
    /// A same-view `Prepare` or `Commit` from the legitimate primary is
    /// proof of primary life: whatever its outcome (accept, gap, named
    /// drop), it refreshes the suspicion baseline (S4). Higher-view
    /// normal-operation messages deliberately do NOT refresh — they are
    /// evidence the node is stale (§10), and the view-change exchange is
    /// how it catches up.
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
            _ => false,
        };
        let plan = match &message.body {
            Body::Prepare { entry, committed } => {
                self.plan_prepare(journal, from, message, entry, *committed, kind)
            }
            Body::PrepareOk {} => self.plan_prepare_ok(journal, from, message, kind),
            Body::Commit { committed } => {
                self.plan_commit(journal, from, message, *committed, kind)
            }
            Body::StartViewChange {} => {
                self.plan_start_view_change(journal, from, message, at, kind)
            }
            Body::DoViewChange {
                retained,
                accepted,
                committed,
                suffix,
                client_table,
                evidence,
                era_proof,
            } => self.plan_do_view_change(
                journal,
                from,
                message,
                *retained,
                *accepted,
                *committed,
                suffix,
                client_table,
                *evidence,
                era_proof,
                at,
                kind,
            ),
            Body::StartView {
                suffix,
                accepted,
                committed,
                client_table,
                era_proof,
            } => self.plan_start_view(
                journal,
                from,
                message,
                suffix,
                *accepted,
                *committed,
                client_table,
                era_proof,
                at,
                kind,
            ),
            Body::Request { .. }
            | Body::PlannedViewChange {}
            | Body::Recovery { .. }
            | Body::RecoveryResponse { .. }
            | Body::GetState { .. }
            | Body::NewState { .. }
            | Body::Reply { .. } => Err(PlanRejection::Unsupported { input: kind }),
        }?;
        Ok(if primary_life {
            plan.with_activity(at)
        } else {
            plan
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
    fn plan_start_view_change(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        at: Tick,
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
    fn plan_do_view_change(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        retained: ViewId,
        accepted: Slot,
        committed: Slot,
        suffix: &[LogEntry],
        client_table: &[WireRow],
        evidence: EvidenceKind,
        era_proof: &EraProof,
        at: Tick,
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
        // Shape: the header slot names the reported accepted frontier
        // (rule 7's Frontier role); the frontiers are a legal chain; the
        // suffix is a contiguous ascending run ending at the frontier; the
        // client table is canonical (strictly ascending clients); the
        // evidence is ordinary (planned evidence belongs to the planned
        // view-change path and never lands here); the era proof matches
        // the configuration history
        // (§8.7.8).
        if header.slot != accepted
            || committed > accepted
            || evidence != EvidenceKind::Ordinary
            || !suffix_shape_ok(suffix, accepted)
            || !wire_table_shape_ok(client_table)
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
            client_table: client_table.to_vec(),
        });
        let candidate = self.identity_candidate()?;
        self.continue_view_change(journal, candidate, view_change, Vec::new(), at, kind)
    }

    /// A `StartView` (§9.1, §13.1): the designated new primary installing
    /// the selected history and the merged client table (§9.2).
    ///
    /// The adoption rule: any node the change passed by — `Normal` or
    /// `Recovering` in an earlier view, or fencing into this very view —
    /// installs the offered history, provided it can VERIFY it: the suffix
    /// must reach back to a slot the node can check (its frontier, or a
    /// shared slot whose entry agrees). A suffix that starts past the
    /// node's frontier is a gap — named [`Diagnostic::GapDetected`], kept
    /// fenced, never faulted; the active fetch belongs to state transfer
    /// (§13.1 step 5).
    /// A suffix that CONFLICTS at a committed local slot is the view-change
    /// path's one deliberate fault-on-peer-input: an honest evidence quorum
    /// can never
    /// produce it, and silently repairing would hide the safety breach, so
    /// the node declares [`Fault::IllegalTransition`].
    ///
    /// The offered client table REPLACES the local one: it is the merge of
    /// a view-change quorum's tables, extended with the rows the installed
    /// history implies — it dominates anything a single node held.
    #[allow(clippy::too_many_arguments)]
    fn plan_start_view(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        suffix: &[LogEntry],
        accepted: Slot,
        committed: Slot,
        client_table: &[WireRow],
        era_proof: &EraProof,
        at: Tick,
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
            || !wire_table_shape_ok(client_table)
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
        let mutation = match self.check_suffix(journal, suffix, accepted, committed) {
            SuffixCheck::Install(mutation) => mutation,
            SuffixCheck::Gap { expected, got } => {
                return self.drop_plan(Diagnostic::GapDetected { expected, got }, kind);
            }
            SuffixCheck::Conflict => {
                let candidate = self.identity_candidate()?;
                return Ok(self
                    .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
                    .with_fault_declared(Fault::IllegalTransition));
            }
        };
        let candidate = self.install_candidate(header.view, accepted, committed)?;
        let effects =
            self.apply_effects_merged(journal, suffix, self.progress.committed(), committed)?;
        let table = self.installed_table(journal, suffix, accepted, client_table);
        Ok(self
            .candidate_plan(candidate, mutation, effects, kind, false)
            .with_bookkeeping(Bookkeeping {
                table_replace: Some(local_rows(&table)),
                view_change: ViewChangeUpdate::Clear,
                activity: Some(at),
                ..Bookkeeping::default()
            }))
    }

    /// The designated new primary's install, once the evidence quorum holds
    /// (§9.1): the selected history becomes the node's own, `committed`
    /// advances to the greatest frontier the quorum truthfully reported
    /// (each report is a commit quorum's product, and the selected history
    /// contains every committed entry — §9.2's argument), the newly
    /// committed client slots apply in slot order (§11.1), and `StartView`
    /// broadcasts the selection with a freshly packed bounded suffix
    /// (§13.1).
    ///
    /// The client table is evidence (§9.2): the install merges the
    /// quorum's tables — per client the greatest `last_request`, ties
    /// toward the cached result — extends the merge with the rows the
    /// installed history itself implies, installs the result wholesale, and
    /// ships it in `StartView`. The uncommitted client tail's proposals are
    /// re-seeded from the entries' own identities, so an installed request
    /// that commits in the new view is still answered with its `Reply`.
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
    ) -> Result<WinOutcome, PlanRejection> {
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
        let mutation =
            match self.check_suffix(journal, &selected.suffix, selected.accepted, committed) {
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
        let candidate = self.install_candidate(target, selected.accepted, committed)?;
        effects.extend(self.apply_effects_merged(
            journal,
            &selected.suffix,
            self.progress.committed(),
            committed,
        )?);
        // The merge over the evidence quorum's tables, extended from the
        // installed history — a total deterministic function of the
        // evidence set (§9.2), so every node that installs this view holds
        // the same table.
        let merged = merge_client_tables(evidence.values().map(|member| &member.client_table[..]));
        let wire = self.installed_table(journal, &selected.suffix, selected.accepted, &merged);
        let table = local_rows(&wire);
        let proposals =
            self.installed_proposals(journal, &selected.suffix, committed, selected.accepted);
        let suffix = self.bounded_suffix(
            journal,
            selected.accepted,
            &selected.suffix,
            wire_table_packed_len(&wire),
        );
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
                client_table: wire,
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
                table_replace: Some(table),
                proposals,
                view_change: ViewChangeUpdate::Clear,
                // The StartView broadcast is the new primary's
                // announcement of the view: proof of its life (S4).
                activity: Some(at),
                ..Bookkeeping::default()
            });
        Ok(WinOutcome::Installed(Box::new(plan)))
    }

    /// The backup's `Prepare` handler (§4). Guards first, each with a named
    /// outcome; then the slot relation decides: accept, idempotent
    /// re-acknowledgement, or gap.
    ///
    /// The bootstrap rule: a `Recovering` backup receiving a legitimate
    /// `Prepare` for its current view — with `current == retained`, so
    /// entering `Normal` re-selects nothing and rule 3 of the legality gate
    /// is untouched — adopts the view and enters `Normal` (§4's own
    /// mechanism; the fresh cluster has nothing to recover). The
    /// piggybacked committed frontier is taken on every accepted or
    /// re-acknowledged `Prepare` (§13.3).
    ///
    /// The gap rule is deliberately interim: a `Prepare` past the accepted
    /// frontier's successor is dropped and reported as
    /// [`Diagnostic::GapDetected`]; the primary's retransmit or state
    /// transfer (§10) closes it — the active fetch belongs to state
    /// transfer.
    fn plan_prepare(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        entry: &LogEntry,
        piggybacked: Slot,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let header = message.header;
        // The era must be evaluable: outside the retention window the
        // configuration that would judge the message is gone.
        if self.progress.config().record(header.view.era).is_none() {
            return self.drop_plan(
                Diagnostic::UnevaluableEra {
                    era: header.view.era,
                },
                kind,
            );
        }
        // The sender must be the primary of the message's view under that
        // view's era configuration (§1.2).
        if self.primary_of(header.view) != Some(from) {
            return self.drop_plan(
                Diagnostic::SenderNotPrimary {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        }
        // The message's view must be the node's current view; a `Recovering`
        // node adopts it (the bootstrap rule above). Anything else is a
        // view change or a recovery, and the message drops.
        let current = self.progress.current();
        let eligible = header.view == current
            && match self.progress.status() {
                Status::Normal => true,
                Status::Recovering => current == self.progress.retained(),
                Status::ViewChange | Status::Replaying => false,
            };
        if !eligible {
            return self.drop_plan(
                Diagnostic::ViewMismatch {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        let adopt = self.progress.status() == Status::Recovering;
        // Era discipline (§8.7.3): era(view) <= era(entry) <= era(view) + 1.
        let era_legal = entry.era == header.view.era || header.view.era.next() == Some(entry.era);
        if !era_legal {
            return self.drop_plan(
                Diagnostic::EraDiscipline {
                    entry: entry.era,
                    view: header.view,
                },
                kind,
            );
        }
        if header.slot != entry.slot {
            return self.drop_plan(
                Diagnostic::PrepareSlotMismatch {
                    header: header.slot,
                    entry: entry.slot,
                },
                kind,
            );
        }
        let status = if adopt {
            Status::Normal
        } else {
            self.progress.status()
        };
        // The entry's identity is protocol state (§9.2): accepting or
        // re-acknowledging it records the client-table row, so a backup's
        // `DoViewChange` evidence carries the row and §9.2's guarantee
        // survives the primary the request arrived from. Only an advance
        // replaces a row — an equal or older request keeps what it has.
        let table_row = match &entry.payload {
            Payload::Client {
                client, request, ..
            } if self
                .clients
                .get(client)
                .is_none_or(|row| *request > row.last) =>
            {
                Some((
                    *client,
                    ClientRow {
                        last: *request,
                        result: None,
                    },
                ))
            }
            _ => None,
        };
        let accepted = self.progress.accepted();
        if entry.slot <= accepted {
            // Idempotent retransmission: never re-append. The held entry
            // must BE the proposed one — a slot is assigned once (§1.3).
            let Some(held) = journal.get(entry.slot) else {
                return Err(PlanRejection::JournalEntryUnavailable { slot: entry.slot });
            };
            if held != entry {
                return self.drop_plan(Diagnostic::ConflictingEntry { slot: entry.slot }, kind);
            }
            // Re-acknowledge, and take the piggybacked frontier (§13.3).
            let new_committed = self.progress.committed().max(piggybacked.min(accepted));
            let candidate =
                self.candidate_with(status, accepted, new_committed, self.progress.applied())?;
            let mut effects = vec![prepare_ok(current, from, entry.slot)];
            effects.extend(self.apply_effects(
                journal,
                self.progress.committed(),
                new_committed,
            )?);
            return Ok(self
                .candidate_plan(candidate, JournalMutation::None, effects, kind, false)
                .with_bookkeeping(Bookkeeping {
                    clients: table_row.into_iter().collect(),
                    ..Bookkeeping::default()
                }));
        }
        let Some(next) = accepted.next() else {
            // `entry.slot > accepted == u64::MAX` cannot be offered; the
            // slot space is spent.
            return Err(PlanRejection::SlotSpaceExhausted);
        };
        if entry.slot != next {
            return self.drop_plan(
                Diagnostic::GapDetected {
                    expected: next,
                    got: entry.slot,
                },
                kind,
            );
        }
        // Accept, and take the piggybacked frontier (§13.3).
        let new_committed = self.progress.committed().max(piggybacked.min(entry.slot));
        let candidate =
            self.candidate_with(status, entry.slot, new_committed, self.progress.applied())?;
        let mut effects = vec![prepare_ok(current, from, entry.slot)];
        effects.extend(self.apply_effects(journal, self.progress.committed(), new_committed)?);
        Ok(self
            .candidate_plan(
                candidate,
                JournalMutation::Accept(vec![entry.clone()]),
                effects,
                kind,
                false,
            )
            .with_bookkeeping(Bookkeeping {
                clients: table_row.into_iter().collect(),
                ..Bookkeeping::default()
            }))
    }

    /// The primary's `PrepareOk` handler (§4). Guards: `Normal`, own is the
    /// primary of the current view, the view matches, the sender is a
    /// member of the current configuration, the slot is outstanding, the
    /// sender is not already counted. Then the vote is recorded and the
    /// STRATEGY — the only quorum authority (Q1) — is asked; on a quorum
    /// the committed frontier advances over the contiguous accepted tail,
    /// the newly committed client slots emit `Apply` in slot order (§11.1),
    /// and the new frontier is announced to every backup (§13.3).
    fn plan_prepare_ok(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let header = message.header;
        let slot = header.slot;
        let current = self.progress.current();
        let is_primary =
            self.progress.status() == Status::Normal && self.primary_of(current) == Some(self.own);
        if !is_primary {
            return self.drop_plan(Diagnostic::PrepareOkNotPrimary { sender: from, slot }, kind);
        }
        if header.view != current {
            return self.drop_plan(
                Diagnostic::ViewMismatch {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        let record = self
            .current_record()
            .ok_or(PlanRejection::Progress(ProgressError::EraSlotDiscipline))?;
        if record.config.weight_of(from).is_none() {
            return self.drop_plan(Diagnostic::UnknownSender { sender: from }, kind);
        }
        let proposal = if slot <= self.progress.committed() {
            None
        } else {
            self.proposals.get(&slot)
        };
        let Some(proposal) = proposal else {
            // A delayed duplicate of a committed slot, or foreign: harmless.
            return self.drop_plan(Diagnostic::SlotNotOutstanding { slot, sender: from }, kind);
        };
        if proposal.oks.contains(&from) {
            return self.drop_plan(Diagnostic::DuplicatePrepareOk { slot, sender: from }, kind);
        }
        // Acceptance is prefix-contiguous, so this acknowledgement vouches
        // for every lower uncommitted slot (VRR-2012 §4's cumulative
        // acknowledgement): the vote is recorded against every outstanding
        // slot up to the acknowledged one — including slots a view change
        // installed, whose records carry no client identity.
        let mut oks: Vec<(Slot, NodeId)> = Vec::new();
        let mut covered = self.progress.committed();
        while let Some(next) = covered.next() {
            if next > slot {
                break;
            }
            if self.proposals.contains_key(&next) {
                oks.push((next, from));
            }
            covered = next;
        }
        // Then the cascade over the contiguous accepted tail: a slot
        // commits when its Commit quorum lands (the strategy decides —
        // Q1), and commits pull every earlier quorum-holding slot with
        // them (§4). The in-flight vote counts for the whole covered
        // range, because it vouches for the whole range.
        let mut committed = self.progress.committed();
        while let Some(next) = committed.next() {
            if next > self.progress.accepted() {
                break;
            }
            let mut members: Vec<NodeId> = vec![self.own];
            if let Some(outstanding) = self.proposals.get(&next) {
                members.extend(outstanding.oks.iter().copied());
            }
            if next <= slot && !members.contains(&from) {
                members.push(from);
            }
            if !self
                .strategy
                .is_quorum(Role::Commit, &record.config, &members)
            {
                break;
            }
            committed = next;
        }
        let bookkeeping = Bookkeeping {
            oks,
            ..Bookkeeping::default()
        };
        if committed == self.progress.committed() {
            let candidate = self.identity_candidate()?;
            return Ok(self
                .candidate_plan(candidate, JournalMutation::None, Vec::new(), kind, false)
                .with_bookkeeping(bookkeeping));
        }
        let candidate = self.candidate_with(
            Status::Normal,
            self.progress.accepted(),
            committed,
            self.progress.applied(),
        )?;
        let mut effects = self.apply_effects(journal, self.progress.committed(), committed)?;
        effects.extend(self.broadcast_commit(committed));
        Ok(self
            .candidate_plan(candidate, JournalMutation::None, effects, kind, false)
            .with_bookkeeping(bookkeeping))
    }

    /// Any node's `Commit` handler (§4, §13.3): advance
    /// `committed = min(header.committed, accepted)` — the frontier never
    /// claims what the journal does not record (§5 invariant 2) — and emit
    /// `Apply` for the newly committed client slots in slot order (§11.1).
    /// A `Recovering` backup adopts the view under the same rule as
    /// `Prepare`.
    fn plan_commit(
        &self,
        journal: &J::View,
        from: NodeId,
        message: &Message,
        frontier: Slot,
        kind: InputKind,
    ) -> Result<PlannedTransition, PlanRejection> {
        let header = message.header;
        if self.progress.config().record(header.view.era).is_none() {
            return self.drop_plan(
                Diagnostic::UnevaluableEra {
                    era: header.view.era,
                },
                kind,
            );
        }
        if self.primary_of(header.view) != Some(from) {
            return self.drop_plan(
                Diagnostic::SenderNotPrimary {
                    sender: from,
                    view: header.view,
                },
                kind,
            );
        }
        let current = self.progress.current();
        let eligible = header.view == current
            && match self.progress.status() {
                Status::Normal => true,
                Status::Recovering => current == self.progress.retained(),
                Status::ViewChange | Status::Replaying => false,
            };
        if !eligible {
            return self.drop_plan(
                Diagnostic::ViewMismatch {
                    got: header.view,
                    current,
                },
                kind,
            );
        }
        let status = if self.progress.status() == Status::Recovering {
            Status::Normal
        } else {
            self.progress.status()
        };
        let new_committed = self
            .progress
            .committed()
            .max(frontier.min(self.progress.accepted()));
        if new_committed == self.progress.committed() && status == self.progress.status() {
            let candidate = self.identity_candidate()?;
            return Ok(self.candidate_plan(
                candidate,
                JournalMutation::None,
                Vec::new(),
                kind,
                false,
            ));
        }
        let candidate = self.candidate_with(
            status,
            self.progress.accepted(),
            new_committed,
            self.progress.applied(),
        )?;
        let effects = self.apply_effects(journal, self.progress.committed(), new_committed)?;
        Ok(self.candidate_plan(candidate, JournalMutation::None, effects, kind, false))
    }

    /// The §11.2 completion. Two shapes are accepted:
    ///
    /// - The next applied slot in order: advances `applied` and emits the
    ///   `Reply` to the client recorded for the slot (the primary holds the
    ///   record; a backup replies to no one). Either way the result is
    ///   cached into the client's row when it is still the client's latest
    ///   — at any node, so a backup's `DoViewChange` evidence can carry the
    ///   cached result through a view change (§9.2). The
    ///   Reply-after-Apply property is structural: a `Reply` is emitted
    ///   ONLY here and in the cached/re-drive paths of the client-request
    ///   handler.
    /// - A slot in the re-drive set (§9.2): the completion of an
    ///   unknown-result re-drive. The frontier does not move — the slot was
    ///   applied before — and the `Reply` is produced from the entry's own
    ///   identity. A completion for a slot nobody re-drove, or a duplicate,
    ///   or an out-of-order completion, is the named
    ///   [`PlanRejection::UnexpectedApplied`].
    ///
    /// The genesis slots are system payloads (§8.7.2): they commit but are
    /// never applied, so the first expected applied slot is `INIT_SLOT + 1`.
    fn plan_applied(
        &self,
        journal: &J::View,
        slot: Slot,
        result: &[u8],
    ) -> Result<PlannedTransition, PlanRejection> {
        let base = self.progress.applied().max(INIT_SLOT);
        let expected = match base.next() {
            Some(next) if next <= self.progress.committed() => Some(next),
            Some(_) | None => None,
        };
        let entry = journal
            .get(slot)
            .ok_or(PlanRejection::JournalEntryUnavailable { slot })?;
        let Payload::Client {
            client, request, ..
        } = &entry.payload
        else {
            return Err(PlanRejection::AppliedSystemSlot { slot });
        };
        let client = *client;
        let request = *request;
        if expected != Some(slot) {
            // The re-drive completion: the slot is committed, was applied
            // before, and its re-drive is outstanding.
            if !self.redrives.contains(&slot) || slot > self.progress.applied() {
                return Err(PlanRejection::UnexpectedApplied {
                    expected,
                    got: slot,
                });
            }
            let candidate = self.identity_candidate()?;
            let mut bookkeeping = Bookkeeping {
                redrive_done: vec![slot],
                ..Bookkeeping::default()
            };
            if self
                .clients
                .get(&client)
                .is_some_and(|row| row.last == request)
            {
                bookkeeping.clients.push((
                    client,
                    ClientRow {
                        last: request,
                        result: Some(result.into()),
                    },
                ));
            }
            let effects = vec![Effect::Reply {
                client,
                request,
                result: result.into(),
            }];
            return Ok(self
                .candidate_plan(
                    candidate,
                    JournalMutation::None,
                    effects,
                    InputKind::Applied,
                    false,
                )
                .with_bookkeeping(bookkeeping));
        }
        let candidate = self.candidate_with(
            self.progress.status(),
            self.progress.accepted(),
            self.progress.committed(),
            slot,
        )?;
        let mut effects = Vec::new();
        let mut bookkeeping = Bookkeeping {
            resolved: vec![slot],
            ..Bookkeeping::default()
        };
        // A re-driven slot can land here instead of the out-of-order path:
        // the node's applied frontier lagged its committed frontier, so for
        // THIS host the re-drive was the first execution. The frontier
        // advances as usual; the outstanding retry is still answered.
        let redriven = self.redrives.contains(&slot);
        if redriven {
            bookkeeping.redrive_done.push(slot);
        }
        let identity = self
            .proposals
            .get(&slot)
            .and_then(|proposal| proposal.client.zip(proposal.request))
            .or(redriven.then_some((client, request)));
        if let Some((client, request)) = identity {
            effects.push(Effect::Reply {
                client,
                request,
                result: result.into(),
            });
        }
        // Cache the result for the idempotent retry — only if this is
        // still the client's latest request; a superseded row belongs to a
        // newer slot. At any node, primary or backup: the cached result is
        // view-change evidence (§9.2).
        if self
            .clients
            .get(&client)
            .is_some_and(|row| row.last == request)
        {
            bookkeeping.clients.push((
                client,
                ClientRow {
                    last: request,
                    result: Some(result.into()),
                },
            ));
        }
        Ok(self
            .candidate_plan(
                candidate,
                JournalMutation::None,
                effects,
                InputKind::Applied,
                false,
            )
            .with_bookkeeping(bookkeeping))
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

    /// The `Apply` effects for the newly committed slots `(from, through]`,
    /// in slot order (§11.1): client payloads only — system slots commit,
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
            if let Payload::Client { payload, .. } = &entry.payload {
                effects.push(Effect::Apply {
                    slot,
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
    /// of every drop and every silent client-table refusal.
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
    /// `Normal`, and the frontiers become the installed history's. The
    /// accepted frontier may regress across the re-selection (gate rule 1
    /// admits it exactly then); the committed frontier never does — the
    /// callers guard it before this candidate is ever built.
    fn install_candidate(
        &self,
        view: ViewId,
        accepted: Slot,
        committed: Slot,
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
            self.progress.applied(),
            self.progress.checkpoint(),
            revision,
            Arc::clone(self.progress.config()),
            None,
        )
        .map_err(PlanRejection::Progress)
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
            if let Payload::Client { payload, .. } = &entry.payload {
                effects.push(Effect::Apply {
                    slot,
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

    /// Whether an era proof matches the node's own records (§8.7.8): the
    /// claimed committed slot IS the era's establishing slot, and the
    /// journal entry there holds the claimed operation.
    fn era_proof_ok(&self, journal: &J::View, record: &EraRecord, proof: &EraProof) -> bool {
        proof.committed_at == record.established_by
            && matches!(
                journal.get(proof.committed_at),
                Some(entry) if matches!(&entry.payload, Payload::System(op) if op == &proof.op)
            )
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
            // the view it proposed in (the client table survives — its rows
            // answer retries with results this node durably produced, §9.2).
            ViewChangeUpdate::Clear => {
                self.view_change = None;
                self.proposals.clear();
            }
        }
        if let Some(rows) = bookkeeping.table_replace {
            self.clients = rows.into_iter().collect();
        }
        for (client, row) in bookkeeping.clients {
            self.clients.insert(client, row);
        }
        for (slot, proposal) in bookkeeping.proposals {
            self.proposals.insert(slot, proposal);
        }
        for (slot, node) in bookkeeping.oks {
            if let Some(proposal) = self.proposals.get_mut(&slot)
                && !proposal.oks.contains(&node)
            {
                proposal.oks.push(node);
            }
        }
        for slot in bookkeeping.resolved {
            self.proposals.remove(&slot);
        }
        for slot in bookkeeping.redrive_begin {
            self.redrives.insert(slot);
        }
        for slot in bookkeeping.redrive_done {
            self.redrives.remove(&slot);
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
            None => entry.slot != Slot::FIRST,
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

/// The `PrepareOk` a backup answers a `Prepare` with (§4): the view is the
/// view it accepted under, the slot the slot it accepted (W1: the header
/// slot names what the message speaks about).
fn prepare_ok(view: ViewId, to: NodeId, slot: Slot) -> Effect {
    Effect::Send {
        to,
        era: view.era,
        message: Message {
            header: Header {
                tag: Tag::PrepareOk,
                view,
                slot,
            },
            body: Body::PrepareOk {},
        },
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
