//! Lockless observation of published progress from outside the transition interval.
//!
//! Spec §12 (concurrency contract for the C ABI) and decision B1. A diagnostic
//! thread, a metrics scraper, or a LuaJIT host holding the C ABI must read progress
//! without taking any lock the transition path takes: **a reader never blocks a
//! writer or another reader, and observation cannot delay the §12 serialized
//! transition interval** — a replica that stalls its transition to service a scrape
//! has converted observability into a liveness fault.
//!
//! The mechanism is a single-writer seqlock over a POD snapshot: even sequence
//! means stable, odd means write in progress, a torn read retries. Chosen over an
//! `Arc` swap because there is **no reclamation race to prove** — nothing is freed,
//! so no epoch, hazard pointer, or deferred drop whose soundness every later module
//! would have to re-establish. The cost is reader spin under a write storm; writes
//! are bounded by the transition rate. This module and the future `ffi` module are
//! the only two places `unsafe` is permitted. Observation is read-only (§15).

// Scoped `unsafe` exception (decision B1). The proof obligations are written at
// each use: the payload accesses, the `Send`/`Sync` impls, and the writer claim.
#![allow(unsafe_code)]

use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::ptr;
use core::sync::atomic::{AtomicU64, Ordering, fence};

use crate::configuration::ConfigError;
use crate::ids::{Era, NodeId, Slot, ViewId};

/// Why a published transition dropped its peer input — or `None`, when it
/// dropped nothing.
///
/// Normal operation made total (VRR-2012 §4): invalid peer input is dropped
/// with a named outcome, never faults the node. Faulting is reserved for
/// impossible LOCAL transitions via `crate::invariant::legal`. Every guard in
/// the §4 handlers names its outcome here, and the outcome is published
/// through the same seqlock discipline as progress (B1): each published
/// transition records its drop outcome, so a drop is observable and never a
/// silent no-op. The record holds the LATEST transition's outcome; it is a
/// diagnostic, not a log.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Diagnostic {
    /// The transition dropped nothing.
    None,
    /// The message's era is outside the configuration retention window, so
    /// the configuration that would judge it is gone: the message is
    /// unevaluable. A peer that far behind must recover or state-transfer
    /// (§10, §14.2).
    UnevaluableEra {
        /// The unevaluable era.
        era: Era,
    },
    /// The sender is not the primary of the message's view under that view's
    /// era configuration (§1.2's rotating leadership).
    SenderNotPrimary {
        /// The transport-attributed sender.
        sender: NodeId,
        /// The view the message claimed.
        view: ViewId,
    },
    /// The message's view differs from the node's current view and no
    /// adoption rule applies. View change and recovery have their own
    /// adoption rules; normal operation drops the message.
    ViewMismatch {
        /// The view the message named.
        got: ViewId,
        /// The node's current view.
        current: ViewId,
    },
    /// The entry's era violates `era(view) <= era(entry) <= era(view) + 1`
    /// (§8.7.3's era discipline).
    EraDiscipline {
        /// The entry's era.
        entry: Era,
        /// The view the entry arrived under.
        view: ViewId,
    },
    /// A `Prepare` whose header slot disagrees with its entry's slot; the
    /// wire cannot say which field lied, so the message is dropped whole.
    PrepareSlotMismatch {
        /// The header's slot.
        header: Slot,
        /// The entry's slot.
        entry: Slot,
    },
    /// A `Prepare` carrying a system operation the configuration fold
    /// refuses (§8.7.2's preconditions, run at accept — the peer's entry,
    /// never the node's committed history, is what fails here, so the
    /// entry is dropped and nothing installs; a refusal in COMMITTED
    /// history is the fold breach that faults instead).
    InvalidSystemOperation {
        /// The slot the operation claimed.
        slot: Slot,
        /// The fold's refusal.
        error: ConfigError,
    },
    /// A re-`Prepare` for an accepted slot whose held entry differs; a slot
    /// is assigned once in a legitimate history (§1.3).
    ConflictingEntry {
        /// The contested slot.
        slot: Slot,
    },
    /// A `Prepare` past the accepted frontier's successor, or a view-change
    /// offer the node cannot verify against its own journal: a gap. Dropped.
    /// The fetch half of the ruling (§10, §13.1 step 5) rides the same
    /// transition: a `GetState` for the missing range goes to the sender,
    /// and the installed chunks re-run the stalled ruling.
    GapDetected {
        /// The first slot the node could not accept or verify: the
        /// accepted frontier's successor in normal operation; in view
        /// change, the first offered slot the node cannot check against
        /// its journal.
        expected: Slot,
        /// The slot the message offered: normal operation names the single
        /// offered slot, past `expected`; view change names the offer's
        /// base, which sits at or below `expected` when the offer reached
        /// back past a shared slot the node has physically let go (S1).
        /// The pair is not ordered.
        got: Slot,
    },
    /// A `NewState` that is not protocol-qualified evidence for the open
    /// fetch (§10): the node asked under another view, asked another
    /// responder, opened no fetch at all, or already holds the chunk's
    /// whole range. Ignored, never faulted.
    StaleTransfer {
        /// The transport-attributed sender.
        sender: NodeId,
        /// The view the chunk carried.
        view: ViewId,
    },
    /// A transfer message whose shape is malformed: the header slot
    /// disagrees with the covered range, the entries are not a contiguous
    /// ascending run ending at it, a final chunk's committed frontier
    /// exceeds the covered range, or an empty chunk claims more remains
    /// (§13.1's shape rule). Dropped whole.
    MalformedTransfer,
    /// A `GetState` the node cannot serve (§10, §13.1 step 5): it is not
    /// `Normal` in the requested view, the requested base is ahead of its
    /// frontier, or its journal no longer holds the base (S1). The
    /// requester's fetch stays open; another answer closes the gap.
    TransferNotServed {
        /// The transport-attributed requester.
        sender: NodeId,
        /// The view the request named.
        view: ViewId,
    },
    /// A `PrepareOk` reached a node that is not the `Normal` primary of its
    /// current view.
    PrepareOkNotPrimary {
        /// The transport-attributed sender.
        sender: NodeId,
        /// The slot the acknowledgement named.
        slot: Slot,
    },
    /// A `PrepareOk` for a slot with no outstanding proposal: a delayed
    /// duplicate of an already-committed slot, or foreign. Harmless.
    SlotNotOutstanding {
        /// The slot the acknowledgement named.
        slot: Slot,
        /// The transport-attributed sender.
        sender: NodeId,
    },
    /// A second `PrepareOk` from the same sender for the same slot.
    DuplicatePrepareOk {
        /// The slot the acknowledgement named.
        slot: Slot,
        /// The transport-attributed sender.
        sender: NodeId,
    },
    /// A `PrepareOk` from a node outside the current configuration.
    UnknownSender {
        /// The transport-attributed sender.
        sender: NodeId,
    },
    /// A `PrepareOk` from a member whose weight is 0 — a learner (§8.4).
    /// The learner receives history but contributes nothing to any quorum
    /// (`docs/uvrr-reincarnation.md` §6: its messages are discarded on
    /// ingress), so its acknowledgement is dropped before it is ever
    /// counted.
    LearnerSender {
        /// The transport-attributed sender.
        sender: NodeId,
    },
    /// A `Reincarnation` announcement (§4 of the doc) that the recipient
    /// cannot act on: the recipient is not the leader of its current view,
    /// or the message's sender is not the new identity it names. Dropped,
    /// never faulted — the bumped node re-sends until a stable leader
    /// exists (§8 of the doc).
    ReincarnationRefused {
        /// The transport-attributed sender.
        sender: NodeId,
        /// The recipient's current view.
        view: ViewId,
    },
    /// A `StartViewChange` for a view at or behind the node's fence target
    /// (§9.1). The change it fences has already happened — or a later one is
    /// already under way — so the vote cannot count twice (V_g ⌢ V_g, §8.3).
    StaleViewChange {
        /// The view the fence named.
        got: ViewId,
        /// The node's current fence target or installed view.
        current: ViewId,
    },
    /// `DoViewChange` evidence for a view the node is not fencing into — a
    /// finished or superseded change (§9.1). Also the outcome for a
    /// `StartView` whose committed frontier claims less than the node
    /// already durably holds: an honest quorum can never produce it, and
    /// dropping keeps the claim observable without believing it.
    StaleEvidence {
        /// The view the evidence named.
        got: ViewId,
        /// The node's current view.
        current: ViewId,
    },
    /// `DoViewChange` evidence arrived at a node that is fencing into the
    /// named view but is not its designated primary — the evidence was never
    /// solicited here (§9.1).
    EvidenceNotCollected {
        /// The transport-attributed sender.
        sender: NodeId,
        /// The view the evidence named.
        view: ViewId,
    },
    /// A `StartView` whose sender is not the primary of the view it
    /// installs, under that view's era configuration (§1.2, §9.1). Only the
    /// designated new primary may install a selected history.
    StartViewNotFromPrimary {
        /// The transport-attributed sender.
        sender: NodeId,
        /// The view the message claimed to install.
        view: ViewId,
    },
    /// A `StartView` for a view behind the node's current view, or for the
    /// current view at a node already `Normal` in it (§9.1). The change it
    /// installs is done; the message is a retransmission or a straggler.
    StartViewFromStaleView {
        /// The view the message named.
        got: ViewId,
        /// The node's current view.
        current: ViewId,
    },
    /// A view-change message whose shape is malformed: header slot and body
    /// frontier disagree, the suffix is not a contiguous ascending run
    /// ending at the accepted frontier, the era proof does not match the
    /// configuration history (§8.7.8), or planned evidence arrived on the
    /// ordinary path (§8.7.7). Dropped whole; the wire cannot say which
    /// field lied.
    MalformedViewChange,
}

/// A single-writer, multi-reader seqlock over a `Copy` snapshot.
///
/// The observed type is `progress::ProgressSnapshot`: a flat record of
/// `u64`/`u32`/`bool`. `T: Copy` is the structural half of the POD requirement —
/// no `Drop`, no owned heap state, a bitwise copy is a complete value.
pub struct Observation<T: Copy> {
    /// Even means stable, odd means a write is in progress.
    seq: AtomicU64,
    cell: UnsafeCell<T>,
}

// SAFETY (`Send`): `T: Send` is required because `read` returns an owned copy on
// the caller's thread — the value crosses threads by value, never by reference.
// SAFETY (`Sync`): shared references admit `read` from any thread and `write`
// from the claimed owner. Payload accesses are volatile — Rust's designated
// mechanism for memory that may change asynchronously — so a racing read/write
// pair is not a data race in the aliasing sense, and no reference into the cell
// ever escapes. Consistency of the returned value is argued at `read`.
unsafe impl<T: Copy + Send> Send for Observation<T> {}
unsafe impl<T: Copy + Send> Sync for Observation<T> {}

impl<T: Copy> Observation<T> {
    /// Publishes `initial` at sequence 0 (even: stable).
    #[must_use]
    pub fn new(initial: T) -> Observation<T> {
        Observation {
            seq: AtomicU64::new(0),
            cell: UnsafeCell::new(initial),
        }
    }

    /// Publishes `snapshot`: claim odd, store payload, release even. §12
    /// guarantees one transition owner, so the claim never contends in a correct
    /// host — but it is a compare-exchange rather than an assumption, so a second
    /// writer serializes instead of corrupting the sequence parity. Safety never
    /// depends on the host's promise.
    pub fn write(&self, snapshot: T) {
        let mut seq = self.seq.load(Ordering::Relaxed);
        loop {
            if seq % 2 == 1 {
                spin_loop();
                seq = self.seq.load(Ordering::Relaxed);
                continue;
            }
            // AcqRel on success: the release half forbids the payload store
            // moving before the claim; the acquire half pairs with the previous
            // writer's release, so this writer observes a complete cell.
            match self.seq.compare_exchange_weak(
                seq,
                seq.wrapping_add(1),
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => {
                    seq = observed;
                    spin_loop();
                }
            }
        }
        // SAFETY: the odd claim makes this thread the only writer and readers
        // never mutate. `write_volatile` because the store races reader copy-outs
        // by design — tearing there is detected by the sequence protocol — so the
        // store must not be split, merged, or elided. `T: Copy`: a complete value
        // is written and no `Drop` of the old contents is skipped.
        unsafe { ptr::write_volatile(self.cell.get(), snapshot) };
        // Release: the payload store happens-before this store, so a reader that
        // acquires the new even sequence observes the complete payload.
        self.seq.store(seq.wrapping_add(2), Ordering::Release);
    }

    /// The latest stable snapshot, by value: zero allocation, no refcount, no
    /// reclamation — exactly why B1 chose this over an `Arc` swap.
    ///
    /// Torn reads are never returned. The copy-out may race a writer and tear,
    /// but a torn value is detected, never returned: a write overlapping the
    /// window either left the sequence odd when first read (retry) or changed it
    /// between the two reads (retry). Returning requires an even sequence
    /// unchanged across the whole copy, and the trailing release store paired
    /// with the leading acquire load means that stable sequence carries a
    /// complete payload with it.
    #[must_use]
    pub fn read(&self) -> T {
        loop {
            // Acquire: the payload reads cannot move before this load, and
            // pairing with the writer's release makes the payload visible
            // whenever the sequence reads even and stable.
            let before = self.seq.load(Ordering::Acquire);
            if before % 2 == 1 {
                spin_loop();
                continue;
            }
            // SAFETY: a bitwise copy of a fully initialized `T: Copy` POD. The
            // copy may tear against a concurrent `write_volatile`; the sequence
            // check detects every such race before the value escapes, so a torn
            // copy is only ever discarded.
            let value = unsafe { ptr::read_volatile(self.cell.get()) };
            // The copy-out must complete before the confirming load: the acquire
            // fence forbids the trailing load moving before it.
            fence(Ordering::Acquire);
            let after = self.seq.load(Ordering::Acquire);
            if before == after {
                return value;
            }
            spin_loop();
        }
    }
}
