//! Effects: what the core asks the host to do, as data.
//!
//! Spec §6 (functional core model), §7 (transition publication and durability),
//! §11 (application boundary). Decisions S2 (plan/publish/confirm, no host
//! callbacks), S3 (three-way stability), W1 (era as a transport-visible fact).
//!
//! The output half of `tick + message + state -> state + list(messages)`. An effect is an
//! inert description — send this datagram, persist this frontier, apply up to this slot —
//! never a closure, a channel handle, or a callback. The required property: **the effect
//! list is inspectable, replayable and assertable without executing it**, so a test can
//! diff the intended I/O of two runs and a host can reorder or batch effects under its
//! own policy (§13.6).
//!
//! Effects are returned, not performed. The core has no transport and no storage, so an
//! effect the host drops is a host decision with host consequences, and the core says so
//! rather than retrying behind the host's back.
//!
//! # Publication order is the safety argument
//!
//! §7's table assigns each [`Stability`] level a meaning *at effect release*:
//! an effect may only be released once the state supporting it has reached the
//! stability level the host declared. The core enforces that by construction:
//! in every mode except [`Stability::Volatile`] a published transition releases
//! exactly one effect — [`Effect::Persist`] — and parks the rest until the
//! host's [`StabilityResult`] arrives as an ordinary serialized input (S2).
//! Calling a mutation "journalled" does not establish durability; the property
//! is that the barrier completed before the dependent effect became
//! observable.

use crate::ids::{Era, NodeId, OperationId, Slot};
use crate::message::Message;

/// What a published transition asks the host to do. The core performs none of
/// it (SANS-I/O); the host owns transport, storage, and the application.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Effect {
    /// Send `message` to `to`. `era` is the era authorizing THIS copy: overlap
    /// mode sends era `e` to one set and era `e+1` to another (§8.7.7), so the
    /// era is a transport-visible fact the host must not have to decode (W1).
    Send {
        /// The recipient.
        to: NodeId,
        /// The era authorizing this copy of the message.
        era: Era,
        /// The datagram to send.
        message: Message,
    },
    /// Apply the committed operation at `slot` to the host application, in
    /// slot order (§11.1). The operation's identity rides along exactly as
    /// the proposing host assigned it: the core never inspects it and never
    /// deduplicates on it, so the same identity may arrive at any number of
    /// slots. Answering the proposer — and any exactly-once policy — is the
    /// host's affair above the boundary (B2), never the core's.
    Apply {
        /// The committed slot to apply.
        slot: Slot,
        /// The operation's identity, carried opaque from the proposal.
        operation_id: OperationId,
        /// The operation's opaque payload (§11.1).
        payload: Box<[u8]>,
    },
    /// What must be stable before this transition may publish (S2/S3). The one
    /// effect an external-stability mode releases at `publish`; everything
    /// else waits for the host's [`StabilityResult`].
    Persist(PersistenceIntent),
}

/// Which part of the durable [`crate::progress::Progress`] record a transition
/// changed and therefore owes the stability barrier (§5, §6's
/// `progress_change?`).
///
/// Revision bumps alone do not count: `revision` is §12 interval bookkeeping,
/// not part of the §5 durable record, so a transition that moves no durable
/// field declares [`ProgressIntent::Unchanged`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProgressIntent {
    /// The transition leaves the durable progress fields unchanged.
    Unchanged,
    /// The full progress record must reach the declared stability level before
    /// the transition's other effects may be released.
    Record,
}

/// Which slots of the journal a transition wrote and therefore owes the
/// stability barrier (§4, §6's `journal_change?`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JournalIntent {
    /// The transition wrote no journal slots.
    Unchanged,
    /// Acceptance of the contiguous range `from..=through` (normal operation).
    Accept {
        /// First slot of the accepted batch.
        from: Slot,
        /// Last slot of the accepted batch.
        through: Slot,
    },
    /// A view selection replacing history from `from` through the installed
    /// frontier (§4, §9).
    InstallSuffix {
        /// The slot at which the selected history takes over.
        from: Slot,
        /// The installed history's accepted frontier.
        through: Slot,
    },
}

/// The durability requirement of one published transition (§6, §7).
///
/// `revision` names the intent: it is the base revision of the transition —
/// the revision of the published state the candidate was computed against —
/// and the host's confirmation names it back, which is how the §12 serialized
/// interval matches a confirmation to the one outstanding transition.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PersistenceIntent {
    /// The base revision of the transition whose durability this intent
    /// covers; the confirmation names it back (§12).
    pub revision: u64,
    /// The progress-record half of the requirement.
    pub progress: ProgressIntent,
    /// The journal half of the requirement.
    pub journal: JournalIntent,
}

/// The host's declared durability profile (§7's table).
///
/// The level is a statement about the meaning of effect release, and the core
/// behaves accordingly: [`Stability::Volatile`] releases at `publish`; every
/// other level parks the transition behind [`Effect::Persist`] until the host
/// confirms. `Deferred`, `Forced` and `ExternalTransaction` differ in what the
/// host's confirmation *means* — acceptance for later durability, a local
/// crash barrier, a wider transaction — and that meaning is the host's
/// contract with its own storage, not something the core can distinguish.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stability {
    /// State is installed in process memory; safety relies on VRR-2012 quorum
    /// memory and restart (§7). Effects release at `publish`.
    Volatile,
    /// The host accepted a write for later durability; until a barrier
    /// completes, safety remains the `Volatile` case (§7).
    Deferred,
    /// The host confirms a host-selected local crash/power-loss barrier (§7).
    Forced,
    /// The host confirms its wider transaction, potentially including
    /// application state (§7, §11.1).
    ExternalTransaction,
}

/// The host's report on a [`PersistenceIntent`] (decision S3).
///
/// Three outcomes, not two, because "it definitely did not happen" and "I
/// cannot tell" license opposite responses: the first leaves the previously
/// published state visible and lets the host retry, the second sticky-faults
/// the node, which can no longer say what its own durable state is (§5
/// invariant 5). A host that reports [`StabilityResult::Failed`] for a timeout
/// it cannot actually disprove has violated the contract, and the resulting
/// divergence is the host's.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StabilityResult {
    /// The intent reached the declared stability level. `receipt` is the
    /// host's opaque evidence — the core carries it, never interprets it.
    Stable {
        /// Host-supplied evidence of durability.
        receipt: Box<[u8]>,
    },
    /// The intent determinately did not complete. NOT a fault: the candidate
    /// is discarded, the previously published state stays visible and
    /// observable, and the node continues (S3).
    Failed {
        /// Host-supplied diagnostic.
        reason: Box<[u8]>,
    },
    /// The host cannot say whether the intent completed. Sticky-faults the
    /// node (§5 invariant 5): the process must not guess whether the
    /// transition committed.
    Indeterminate {
        /// Host-supplied diagnostic.
        reason: Box<[u8]>,
    },
}
