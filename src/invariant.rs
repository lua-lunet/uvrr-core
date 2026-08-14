//! Mechanical enforcement of the closed invariants.
//!
//! Spec §8.7.4 (`R1: QI_e ⌢ QII_e`, `R2: QI_e ⌢ QII_(e+1)`) and §8.3 (`F_g ⌢ R_g` plus
//! `V_g ⌢ V_g`), and decision Q1.
//!
//! These obligations are closed for modification. No host, feature flag, quorum
//! strategy, or extension may weaken them, so they are not documented advice — they are
//! executed. The required property: **a proposal that would violate an intersection
//! obligation is refused before it is proposed**, never detected after the fact, because
//! after the fact the violating configuration may already be committed and the safety
//! argument is unrecoverable.
//!
//! Validation is over the legal quorum *families*, not over quorum counts. A threshold
//! inequality such as `T_a + T_b > W` (§8.4) is sufficient but not necessary for an
//! arbitrary indivisible weight assignment, so a strategy that only checks thresholds is
//! rejected as unvalidated rather than accepted as probably fine.
//!
//! The self-intersection obligation `V_g ⌢ V_g` deserves separate mention because it is
//! not required between arbitrary phase-one quorums of the classical protocol family,
//! and its absence is the standard way a flexible-quorum policy that satisfies
//! `QI ⌢ QII` is nevertheless an invalid VRR-2012 policy: diskless recovery requires
//! a recovering replica to encounter the volatile evidence that an earlier view was
//! fenced (§8.3).
//!
//! This module holds the transition-legality checker [`legal`]. The family-intersection
//! gate is `crate::quorum`'s `validate_era`/`validate_transition` (Q1), free
//! functions there rather than trait methods so that no quorum strategy can override
//! them. The fault taxonomy itself lives in `crate::ids` (`Fault` is
//! identity-level state carried inside `Progress`, and placing it here would close a
//! `progress` ↔ `invariant` module cycle); it is re-exported below so existing
//! citations of `invariant::Fault` keep compiling.

use crate::ids::Slot;
use crate::progress::Progress;
use crate::wire::Tag;

pub use crate::ids::Fault;

/// What drove a transition, summarised to exactly what the legality rules need.
///
/// The checker never sees the host's bytes — an operation's payload, a
/// journal record, an application effect — because legality is a property of the
/// state pair and the *kind* of the input, not of the input's content. A peer
/// message is the one input whose header is itself protocol state, so it is the
/// one input that carries fields: the tag, for the header-slot table (rule 7) and
/// the re-selection set (rule 3), and the header slot.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputKind {
    /// A protocol datagram from a peer, summarised by its header (W1).
    PeerMessage {
        /// The message type.
        tag: Tag,
        /// The header slot field, whose legal values [`header_slot_role`] fixes.
        slot: Slot,
    },
    /// An operation proposed to the primary by the host (§6, §11.1).
    ClientRequest,
    /// A host timer event (S4; the tick itself is clock metadata, not protocol
    /// state, so the kind carries nothing).
    Tick,
    /// A recovery attempt or its completion (§10).
    Recovery,
    /// The host confirmed the stability of a persistence intent (§7, S2).
    StabilityConfirmed,
    /// The application acknowledged an applied slot (§11.1).
    Applied,
    /// The host checkpointed application state.
    Checkpointed,
    /// A reconfiguration operation committed (§8.7.2).
    Reconfiguration,
    /// The host acted on the node directly (§12's host-declared fault, shutdown,
    /// fencing).
    Admin,
}

/// What a message's header slot may legally carry, per tag (rule 7).
///
/// This table resolves the question: the 20-byte header (W1) has a slot field for
/// framing regularity, but not every message names a slot in the protocol sense, and a
/// fabricated position in a fence or recovery message is worse than none — it is
/// a claim about history the message never made. The table is public so the
/// view-change and recovery paths cite it rather than re-deciding it, and it lives
/// here rather than in
/// `wire` because the wire layer stays agnostic: it encodes 20 bytes for every
/// tag and asks no questions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeaderSlotRole {
    /// The header slot names a log position holding an operation. `Slot(0)` is
    /// illegal: the first position of a legitimate history is `Void` at slot 1
    /// (§8.7.2), so position 0 holds nothing and a message claiming it is
    /// malformed at the protocol level.
    Operation,
    /// The header slot carries the frontier the message speaks about. Every value
    /// is legal, `Slot(0)` included: a genesis node legitimately reports an empty
    /// frontier. Whether a *claimed* frontier is believable is a transition rule
    /// of the replica, not a property of the header, so this role has no
    /// violating value.
    Frontier,
    /// The header slot carries no meaning and must be the sentinel `Slot(0)`.
    /// Chosen over an `Option` in the header because the header is fixed-width
    /// (W1, W4): the field is on the wire either way, so the rule is about what
    /// value it may hold, and `Slot(0)` is the value that can never be confused
    /// with a real position.
    Absent,
}

impl HeaderSlotRole {
    /// Whether `slot` is a legal header value under this role.
    #[must_use]
    pub fn admits(self, slot: Slot) -> bool {
        match self {
            HeaderSlotRole::Operation => slot != Slot::NONE,
            HeaderSlotRole::Frontier => true,
            HeaderSlotRole::Absent => slot == Slot::NONE,
        }
    }
}

/// The per-tag header-slot table (rule 7 of [`legal`]).
///
/// | Tag | Role | Why |
/// |---|---|---|
/// | `Prepare` | `Operation` | names the slot it proposes (§6) |
/// | `PrepareOk` | `Operation` | names the slot accepted (§6) |
/// | `Commit` | `Frontier` | the commit frontier; no new operation (§6, §13.3) |
/// | `StartViewChange` | `Absent` | fences a view, claims no history (§9.1) |
/// | `DoViewChange` | `Frontier` | the accepted frontier of the reported history (§9.1) |
/// | `StartView` | `Frontier` | the accepted frontier of the installed history (§9.1) |
/// | `PlannedViewChange` | `Absent` | like `StartViewChange`, and must not fence (§8.7.7) |
/// | `Recovery` | `Absent` | the nonce is the tick (S4); no slot is spoken about (§10) |
/// | `RecoveryResponse` | `Absent` | nonce and frontiers ride in the body; no slot is spoken about (§6.1) |
/// | `GetState` | `Frontier` | the requester's accepted frontier; the fetch resumes one past it (§4, §13.1 step 5) |
/// | `NewState` | `Frontier` | the last slot the chunk covers; `more` on a partial answer resumes from the requester's cursor (§4, §13.1 step 5) |
///
/// A `match` rather than a lookup table, on the codebase's standing reasoning: the
/// compiler checks that every tag has a rule, and a tag added to `wire` without a
/// ruling here is a compile error, not a defaulted entry.
#[must_use]
pub fn header_slot_role(tag: Tag) -> HeaderSlotRole {
    match tag {
        Tag::Prepare => HeaderSlotRole::Operation,
        Tag::PrepareOk => HeaderSlotRole::Operation,
        Tag::Commit => HeaderSlotRole::Frontier,
        Tag::StartViewChange => HeaderSlotRole::Absent,
        Tag::DoViewChange => HeaderSlotRole::Frontier,
        Tag::StartView => HeaderSlotRole::Frontier,
        Tag::PlannedViewChange => HeaderSlotRole::Absent,
        Tag::Recovery => HeaderSlotRole::Absent,
        Tag::RecoveryResponse => HeaderSlotRole::Absent,
        Tag::GetState => HeaderSlotRole::Frontier,
        Tag::NewState => HeaderSlotRole::Frontier,
    }
}

/// The closed transition-legality checker (architecture stance: closed for
/// modification). The replica runs this on every old→new pair **before** publish,
/// inside the §12 serialized interval.
///
/// A `Some` result means the candidate is **discarded and the node faults** — the
/// checker never repairs a candidate. Repair would make the checker a second,
/// quieter transition function: the pair it published would not be the pair the
/// replica computed, and the divergence between "what the protocol decided" and
/// "what the gate allowed" would be invisible to every later check. A violation
/// means the transition function or the input was outside the protocol, and the
/// only sound response is the sticky fault that stops the node from guessing
/// (§5 invariant 5).
///
/// The rules, each owned by a named function below so a reviewer can point at the
/// line that owns it:
///
/// 1. **Frontiers** (§1.3, §5 invariants 2–3): the chain holds on `new`, and the
///    monotone frontiers never regress. `accepted` may shorten only across a
///    history re-selection, signalled by `retained` changing.
/// 2. **View succession** (§8.7.3, W1): `current` never regresses; a change is a
///    legal successor — delegated to `ViewId::is_legal_successor`, never
///    re-derived.
/// 3. **Retained provenance** (§1.3): `retained` changes only on re-selection —
///    recovery, or a `DoViewChange`/`StartView`/`NewState` peer message.
/// 4. **Revision**: exactly +1, so a stale plan is rejected by comparison (§12).
/// 5. **Sticky fault** (§5 invariant 5, S3): a faulted `old` admits no `new` at
///    all, and the reported fault is the existing one.
/// 6. **Era/slot discipline** (§8.7.3, W1): the era authorising `accepted` is
///    `era(current)` or `era(current) + 1` on `new`.
/// 7. **Header-slot table**: a peer message's header slot satisfies
///    [`header_slot_role`].
///
/// Rules 1, 2 and 6 are also enforced at construction (a violating `Progress` is
/// unrepresentable through the public API); they are restated here because the
/// gate must not rely on how the candidate was built. Rule 5 is checked first
/// because it reports the *existing* fault rather than `IllegalTransition`.
#[must_use]
pub fn legal(old: &Progress, new: &Progress, input: &InputKind) -> Option<Fault> {
    if let Some(fault) = rule5_sticky_fault(old) {
        return Some(fault);
    }
    if rule1_frontiers_violated(old, new)
        || rule2_view_succession_violated(old, new)
        || rule3_retained_violated(old, new, input)
        || rule4_revision_violated(old, new)
        || rule6_era_slot_violated(new)
        || rule7_header_slot_violated(input)
    {
        return Some(Fault::IllegalTransition);
    }
    None
}

/// Rule 5 — sticky fault (§5 invariant 5, S3). A faulted `old` admits no `new`:
/// the node has already declared it cannot say what its durable state is, and no
/// transition changes that. Returns the fault `old` holds, never a fresh one.
fn rule5_sticky_fault(old: &Progress) -> Option<Fault> {
    old.fault()
}

/// Rule 1 — the frontier chain holds on `new`, and `committed`, `applied` and
/// `checkpoint` never regress (§1.3). `accepted` never regresses either, except
/// across a history re-selection — signalled by `retained` changing — because
/// §9.1 ranks candidate histories by `retained_view` first: a shorter history
/// retained from a later view lawfully displaces a longer one from an earlier
/// view, and the installed frontier replaces the local one.
fn rule1_frontiers_violated(old: &Progress, new: &Progress) -> bool {
    if new.check_frontier_chain().is_err() {
        return true;
    }
    if new.committed() < old.committed()
        || new.applied() < old.applied()
        || new.checkpoint() < old.checkpoint()
    {
        return true;
    }
    new.accepted() < old.accepted() && new.retained() == old.retained()
}

/// Rule 2 — `current` never regresses, and a change strictly increases the view
/// with era equal or +1. The rule is `ViewId::is_legal_successor`'s; this
/// function only applies it, because two copies of an inequality are two chances
/// to get it wrong.
fn rule2_view_succession_violated(old: &Progress, new: &Progress) -> bool {
    new.current() != old.current() && !old.current().is_legal_successor(new.current())
}

/// Rule 3 — `retained` identifies the provenance of the retained history (§1.3),
/// so it changes only when that history was re-selected. The re-selection inputs:
/// recovery (§10 installs a coherent state), and the peer messages that install a
/// history — `DoViewChange`, the one that completes the new primary's quorum
/// (§9.1), `StartView`, `NewState` (state transfer, §4), and `RecoveryResponse`,
/// the one that completes a recovery attempt with the latest fenced view's
/// primary's history (§6.1). The match is exhaustive so a new tag forces a
/// ruling here rather than inheriting one.
fn rule3_retained_violated(old: &Progress, new: &Progress, input: &InputKind) -> bool {
    if new.retained() == old.retained() {
        return false;
    }
    let reselects = match input {
        InputKind::Recovery => true,
        InputKind::PeerMessage { tag, .. } => match tag {
            Tag::DoViewChange | Tag::StartView | Tag::NewState | Tag::RecoveryResponse => true,
            Tag::Prepare
            | Tag::PrepareOk
            | Tag::Commit
            | Tag::StartViewChange
            | Tag::PlannedViewChange
            | Tag::Recovery
            | Tag::GetState => false,
        },
        InputKind::ClientRequest
        | InputKind::Tick
        | InputKind::StabilityConfirmed
        | InputKind::Applied
        | InputKind::Checkpointed
        | InputKind::Reconfiguration
        | InputKind::Admin => false,
    };
    !reselects
}

/// Rule 4 — `revision` advances by exactly one per published transition (§12).
/// Checked addition: a wrapped revision would make a stale plan indistinguishable
/// from a current one, so exhaustion faults rather than wraps.
fn rule4_revision_violated(old: &Progress, new: &Progress) -> bool {
    old.revision().checked_add(1) != Some(new.revision())
}

/// Rule 6 — era/slot discipline on `new` (§8.7.3, W1): the era authorising the
/// accepted frontier is the current view's era or its successor — the successor
/// case being overlap mode (§8.7.7). Construction enforces this too; the gate
/// restates it because the gate must not trust how the candidate was built.
fn rule6_era_slot_violated(new: &Progress) -> bool {
    new.check_era_discipline().is_err()
}

/// Rule 7 — the per-tag header-slot table ([`header_slot_role`]).
fn rule7_header_slot_violated(input: &InputKind) -> bool {
    match input {
        InputKind::PeerMessage { tag, slot } => !header_slot_role(*tag).admits(*slot),
        InputKind::ClientRequest
        | InputKind::Tick
        | InputKind::Recovery
        | InputKind::StabilityConfirmed
        | InputKind::Applied
        | InputKind::Checkpointed
        | InputKind::Reconfiguration
        | InputKind::Admin => false,
    }
}
