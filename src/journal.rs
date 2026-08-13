//! The logical journal contract: the four capabilities the algorithm requires of
//! accepted protocol history, and the default in-memory fulfilment of them.
//!
//! Spec §4 and decision S1.
//!
//! `Journal` is a trait over exactly the semantics §4 enumerates: identify the accepted
//! frontier, read history, record acceptance, record a view selection. "Journal" names a
//! host strategy, not a file — it implies no WAL, no table, no mutable array, and no
//! indefinite retention.
//!
//! There is deliberately no reclamation, rotation, segment-management or retention
//! operation in either trait, because none of those is a VRR-2012 state transition
//! (decision S1). A host may implement any retention policy it likes without telling
//! the core, provided it either satisfies a protocol read or reports the requested
//! history unavailable — at which point recovery or state transfer must obtain an
//! adequate state elsewhere. Adding a retention knob to the trait would convert a host
//! policy into a protocol obligation and thereby preclude a legal host, which
//! `docs/architecture.md` classifies as a defect. The split is enforced mechanically:
//! `tests/journal_contract.rs` scans the trait definitions and fails the build if the
//! vocabulary of reclamation ever appears inside them.
//!
//! The default [`SegmentedLog`] keeps history as a sequence of sealed slabs behind a
//! mutable tail, hands out snapshots by `Arc`, and reclaims lazily through
//! [`SegmentedLog::reclaim_through`], an inherent method that is not part of the
//! contract. A unit or stress run that never reclaims at all is a legitimate
//! configuration, not a leak.
//!
//! History begins at slot 1. Slot 0 is the crate-wide "no slot" sentinel — reserved
//! by wire headers and empty frontiers (`invariant::header_slot_role`) — and never a
//! position. The one accept anchor is [`VOID_SLOT`], the slot `Void` occupies at
//! genesis (§8.7.2's fixed ordinals): an empty log's first batch must begin there,
//! and there is no construction knob that moves it. One sentinel, one anchor.

use std::sync::Arc;

use crate::configuration::{SystemOperation, VOID_SLOT};
use crate::ids::{ClientId, Era, RequestNumber, Slot};
use crate::wire::{
    Malformed, Pack, PackWriter, Unpack, UnpackCursor, UnpackError, opaque_packed_len,
};

/// Default capacity of the mutable tail, in entries.
///
/// The tail is the amortisation device: appends accumulate there and seal into a slab
/// when it fills, so the per-entry cost of sealing is paid once per slab rather than
/// once per entry. 64 keeps the seal of a bulk peer catch-up batch out of the
/// per-message path without making a seal an event worth scheduling around.
const DEFAULT_TAIL_CAPACITY: usize = 64;

// Payload discriminants, the `Tag` rule applied to an entry body: `0` is reserved so
// that an all-zero buffer decodes as malformed rather than as a valid client payload,
// and the table is stated as named constants so reordering this source cannot renumber
// the wire.
const PAYLOAD_CLIENT: u8 = 1;
const PAYLOAD_SYSTEM: u8 = 2;

/// One accepted position of protocol history: the slot, the era of the configuration
/// that authorised it, and the payload.
///
/// Spec §1.3 (slots), §8.7.3 (`era(view) <= era(slot) <= era(view) + 1`; the `+1` case
/// is overlap mode, where an era-`e` view commits against the era-`e+1` replication
/// quorum).
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LogEntry {
    /// The log position this entry was accepted at.
    pub slot: Slot,
    /// Era of the configuration that authorized this slot (§8.7.3).
    pub era: Era,
    /// What was accepted.
    pub payload: Payload,
}

/// The payload of an accepted entry.
///
/// Two kinds, because the core's obligations differ: client bytes are opaque and only
/// stored and carried (§11), while a system operation is typed because the core folds
/// it into the configuration on commit (§8.7.2). A `System` entry is never re-typed
/// from bytes at read time, so a payload that cannot fold cannot be silently accepted
/// into history.
///
/// The client operation carries its `(client, request)` identity IN the entry
/// (§9.2): the identity makes the entry self-describing across a view change — a
/// backup records its client-table row from the `Prepare` it accepts, and a retry can
/// be matched against the installed history without trusting any volatile record.
/// Without it the client table could only ever exist at the primary that took the
/// request, and §9.2's duplicate-suppression guarantee would die with that primary.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Payload {
    /// A client operation: its duplicate-suppression identity (§9.2) and
    /// the opaque application bytes (§11). The identity rides in the entry
    /// (§9.2) so the client table is rebuildable from history itself.
    Client {
        /// The client that submitted the operation.
        client: ClientId,
        /// The client's monotonic request number.
        request: RequestNumber,
        /// Opaque application bytes. The core never inspects them (§11).
        payload: Box<[u8]>,
    },
    /// A typed cluster operation (§8.7.2), folded into the configuration on commit.
    System(SystemOperation),
}

impl Pack for LogEntry {
    fn packed_len(&self) -> usize {
        // slot ++ era ++ discriminant, then the payload body. Exact, per the normative
        // W3 contract: the §13.1 suffix budget sums this over candidate entries.
        let payload = match &self.payload {
            Payload::Client {
                client,
                request,
                payload,
            } => 1 + client.packed_len() + request.packed_len() + opaque_packed_len(payload),
            Payload::System(op) => 1 + op.packed_len(),
        };
        self.slot.packed_len() + self.era.packed_len() + payload
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        self.slot.pack(w);
        self.era.pack(w);
        match &self.payload {
            Payload::Client {
                client,
                request,
                payload,
            } => {
                w.u8(PAYLOAD_CLIENT);
                client.pack(w);
                request.pack(w);
                w.opaque(payload);
            }
            Payload::System(op) => {
                w.u8(PAYLOAD_SYSTEM);
                op.pack(w);
            }
        }
    }
}

impl Unpack for LogEntry {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        let slot = Slot::unpack(c)?;
        let era = Era::unpack(c)?;
        let payload = match c.u8()? {
            PAYLOAD_CLIENT => {
                let client = ClientId::unpack(c)?;
                let request = RequestNumber::unpack(c)?;
                // Borrowed from the cursor and copied only here, at the boundary where
                // the journal takes ownership. `UnpackCursor::opaque` bounds the read
                // by the input actually present, so an adversarial length prefix
                // reports `Incomplete` and never pre-allocates — the same decision
                // `Init`'s untrusted member count decode follows.
                let bytes = c.opaque()?;
                Payload::Client {
                    client,
                    request,
                    payload: bytes.to_vec().into_boxed_slice(),
                }
            }
            PAYLOAD_SYSTEM => Payload::System(SystemOperation::unpack(c)?),
            // Discriminant 0 is reserved and everything above the table is unknown;
            // both are `OutOfDomain`, not `UnknownTag`, because that variant carries
            // the header's `u32` tag and this discriminant is a `u8`.
            _ => return Err(UnpackError::Malformed(Malformed::OutOfDomain)),
        };
        Ok(LogEntry { slot, era, payload })
    }
}

/// Why a journal mutation was refused.
///
/// One variant per cause, so a test asserting a refusal asserts *which* precondition
/// failed — a test that can only observe `is_err()` passes when the journal refuses
/// for the wrong reason. Carries the slots the caller needs to recover without a second
/// round trip.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JournalError {
    /// The entry did not begin where the history requires it to.
    ///
    /// Contiguity is the journal's structural invariant, not the caller's obligation
    /// to remember: §1.3 assigns each slot once, and a gap would leave a position the
    /// frontier arithmetic cannot name. `expected` is the slot the journal required;
    /// `got` is the slot the caller offered. Regressive offers are reported as
    /// [`JournalError::SlotOccupied`] instead, because the host's next action differs:
    /// a gap is fixed by supplying the missing history, an occupied slot by dropping
    /// the duplicate.
    NonContiguous {
        /// The slot the journal required next.
        expected: Slot,
        /// The slot the caller offered.
        got: Slot,
    },
    /// The offered slot already holds an accepted entry.
    ///
    /// A slot is assigned once in a legitimate history (§1.3); re-accepting one is a
    /// duplicate, not a gap, and the caller should drop it rather than fetch history.
    SlotOccupied(Slot),
    /// The mutation named a position the journal has already let go of.
    ///
    /// Physical, not logical: the slot may still be part of the accepted history while
    /// being unavailable locally, and §4's answer to unavailable history is recovery
    /// or state transfer, not an error the caller can retry away. `first` is the
    /// first physically present slot.
    BelowRetention {
        /// The slot the caller named.
        slot: Slot,
        /// The first slot the journal still physically holds.
        first: Slot,
    },
    /// Recording the entry would require a slot past `u64::MAX`.
    ///
    /// §8.7.3 forbids wraparound: a wrapped slot would reassign a position that
    /// already holds an accepted operation. Refused rather than saturated, for the
    /// same reason [`Slot::next`] returns `Option`.
    SlotExhausted,
    /// A view selection named no history.
    ///
    /// `install_suffix` installs the suffix a completed view change *selected*; an
    /// empty suffix is not a selection, and truncating to `from` with nothing to
    /// install would silently discard the uncommitted tail the caller claimed to be
    /// replacing.
    EmptySuffix,
}

/// The read-only snapshot handed to a planned transition: the journal as it was when
/// the transition was planned.
///
/// Spec §4's read capabilities — identify the accepted frontier and read history —
/// and nothing else. The snapshot is cheap by construction: the default implementation
/// is an `Arc` bump per sealed slab, not a copy, so taking one never walks the
/// history. What that costs is paid at the mutation site instead: a later
/// `install_suffix` must copy any slab it partially supersedes, because this view may
/// still hold the original.
pub trait JournalView {
    /// Greatest accepted slot, or `None` before genesis (§1.3: the accepted frontier).
    fn accepted(&self) -> Option<Slot>;

    /// The entry at `slot`, or `None` if it is past the frontier or no longer
    /// physically present.
    fn get(&self, slot: Slot) -> Option<&LogEntry>;

    /// Double-ended ordered iteration over a slot range, clipped to what is
    /// physically present.
    ///
    /// Both ends are exact over the intersection of `[from, to]` with the retained
    /// window; a range outside it is empty, never an error, because "how much of this
    /// range exists" is a fact the caller is allowed to discover by reading.
    fn iter_range(&self, from: Slot, to: Slot) -> impl DoubleEndedIterator<Item = &LogEntry>;

    /// The physical presence bounds: the first and last slot actually held.
    ///
    /// `first` may exceed the checkpoint the host last published — the host owes no
    /// promptness (S1) — and exceeds `last` when nothing is held at all. These bounds
    /// are physical facts, not logical ones: the accepted frontier reported by
    /// [`JournalView::accepted`] is unaffected by what has been let go.
    fn retained(&self) -> (Slot, Slot);

    /// Copies a slot range into caller memory, for suffix construction and state
    /// transfer.
    ///
    /// Returns what is physically present and reports the rest through
    /// [`RangeOutcome`]; entries are never invented to complete a range. A range that
    /// starts below the retained frontier is refused outright rather than partially
    /// served: §4 requires unavailable history to be *reported*, and a partial copy
    /// would masquerade as a complete prefix — the shortfall case is the honest one
    /// because it names the frontier it stopped at.
    fn copy_out(&self, from: Slot, to: Slot, into: &mut Vec<LogEntry>) -> RangeOutcome;
}

/// The write side of the journal: spec §4's two mutations, plus the snapshot a
/// transition is planned against. Nothing else.
///
/// A fifth capability — discarding history, rotation, size or age thresholds, any
/// retention knob at all — is host policy and never enters this contract (decision
/// S1; §4 names four capabilities and states that no physical management operation is
/// a VRR-2012 state transition). The absence is enforced mechanically by a gate in
/// `tests/journal_contract.rs`, so a method added here next year breaks the build
/// rather than slipping into the portable surface. A host that never discards
/// anything is a legitimate configuration.
pub trait Journal {
    /// The snapshot type handed to planned transitions.
    type View: JournalView;

    /// Obtains the snapshot a transition will be planned against.
    fn view(&self) -> Self::View;

    /// Records acceptance: appends one entry or an ordered batch at the tail.
    ///
    /// The batch must begin exactly at the successor of the current frontier and be
    /// internally contiguous; anything else is refused and the journal is unchanged.
    /// Contiguity lives here, in the journal, because a slot assigned twice or a
    /// position skipped is a safety violation (§1.3), and an invariant the caller must
    /// remember is an invariant that will eventually be forgotten.
    ///
    /// An empty journal's frontier successor is the genesis anchor, slot 1
    /// (`configuration::VOID_SLOT`): history begins at slot 1 and slot 0 is the
    /// sentinel, never a position.
    ///
    /// # Errors
    ///
    /// [`JournalError::NonContiguous`] on a gap, [`JournalError::SlotOccupied`] on a
    /// regressive slot, [`JournalError::SlotExhausted`] at the end of the slot space.
    fn accept(&mut self, entries: &[LogEntry]) -> Result<(), JournalError>;

    /// Records a view selection: logically installs the suffix chosen by a completed
    /// view change (VRR §5.2 / spec §9), replacing any divergent uncommitted tail.
    ///
    /// Everything from `from` onward is replaced by `suffix`, which must begin at
    /// `from` and be contiguous. This is a change to which logical history the replica
    /// presents; §4 says explicitly that it prescribes no physical modification, which
    /// is why the default implementation is free to implement it copy-on-write at slab
    /// granularity.
    ///
    /// # Errors
    ///
    /// [`JournalError::NonContiguous`] on a gap, [`JournalError::SlotOccupied`] when
    /// the suffix starts before `from`, [`JournalError::BelowRetention`] when `from`
    /// names a position already let go, [`JournalError::EmptySuffix`] when there is
    /// no selected history to install, [`JournalError::SlotExhausted`] at the end of
    /// the slot space.
    fn install_suffix(&mut self, from: Slot, suffix: &[LogEntry]) -> Result<(), JournalError>;
}

/// What [`JournalView::copy_out`] did with the request.
///
/// Exhaustive, so a caller assembling a `NewState` reply or a view-change suffix must
/// say what it does with each shape of shortfall rather than discovering one at
/// runtime.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RangeOutcome {
    /// Every slot in the requested range was copied.
    Complete,
    /// The request ran past the accepted frontier; everything up to `through` was
    /// copied and the rest does not exist yet.
    ///
    /// A shortfall past the frontier is not an error: the frontier moves while a
    /// transfer is in flight, and the requester can ask again for the remainder.
    Short {
        /// The last slot actually copied — the accepted frontier at read time.
        through: Slot,
    },
    /// The request started below the retained frontier; nothing was copied.
    ///
    /// Chosen over a silent partial copy, deliberately: §4 requires the journal to
    /// report unavailable history, and a partial copy of `[from, to]` looks exactly
    /// like a complete copy of a shorter range. The requester must obtain the early
    /// history by recovery or state transfer (§10, §14.2).
    BelowRetention {
        /// The slot the caller asked from.
        slot: Slot,
        /// The first slot physically present.
        first: Slot,
    },
    /// The journal holds nothing at all, so nothing was copied.
    Empty,
}

/// A sealed, immutable run of contiguous entries.
///
/// Sealed means exactly that: nothing mutates a slab after construction. Views share
/// slabs by `Arc`, and a mutation that would reach into one — a suffix install that
/// supersedes part of it — copies the retained prefix into a fresh slab and seals
/// that instead. The fields are private so the only slabs that exist are sealed ones.
#[derive(Debug)]
pub struct Slab {
    entries: Box<[LogEntry]>,
}

impl Slab {
    /// Seals `entries`, which must be contiguous and non-empty.
    fn from_vec(entries: Vec<LogEntry>) -> Slab {
        Slab {
            entries: entries.into_boxed_slice(),
        }
    }
}

/// The default journal: sealed slabs behind a mutable tail, snapshots by `Arc`.
///
/// Structure and cost model:
///
/// - History is a sequence of immutable [`Slab`]s plus one mutable tail. A slab seals
///   when the tail fills; sealing moves the tail into an `Arc` and starts a fresh
///   tail, so the per-entry bookkeeping cost is paid once per slab.
/// - Lookup is a binary search over recorded slab boundaries — `(first_slot, len)`
///   per slab — so slabs may be any size and the search never reads a neighbouring
///   slab's entry. No linked list, no per-entry allocation.
/// - [`SegmentedLog::view`] clones the slab `Arc`s and snapshots the tail: O(number
///   of slabs), not O(entries). This is the cheap candidate-state fork the
///   plan/publish protocol relies on.
/// - Reclamation is [`SegmentedLog::reclaim_through`]: inherent to this type, absent
///   from the [`Journal`] trait, gated on a checkpoint the host chose to publish
///   (S1).
///
/// # Genesis
///
/// Every log's history begins at [`VOID_SLOT`] — slot 1, where `Void` stands at
/// the start of a legitimate history (§8.7.2). Slot 0 is the sentinel, not a
/// position, so the first batch accepted must start at the anchor and an offer
/// below it is refused. There is deliberately no `first_slot` construction
/// knob. A replica restored by state transfer holds a transferred history
/// whose early slots it has let go of — which is this type plus
/// [`SegmentedLog::reclaim_through`], not a different construction — so
/// `accepted()` returning `None` means "no history" without ambiguity, and the
/// contiguity rule has one starting point rather than two.
#[derive(Debug)]
pub struct SegmentedLog {
    /// Sealed slabs, in slot order. A `Vec`, not a deque: reclamation drains a prefix
    /// and a `Vec` prefix-drain is a shift of the remaining `Arc` pointers, which is
    /// cheap at slab counts and buys a contiguous slice for sharing checks.
    slabs: Vec<Arc<Slab>>,
    /// `(first_slot, len)` per slab, kept in step with `slabs` so lookup is a binary
    /// search over boundaries and never touches an entry to find its slab.
    boundaries: Vec<(Slot, usize)>,
    /// The mutable append buffer. May exceed `tail_capacity` transiently after a bulk
    /// `install_suffix`; the next seal drains it whole.
    tail: Vec<LogEntry>,
    /// Entries the tail holds before sealing.
    tail_capacity: usize,
    /// The first slot this log's history ever held. Fixed at [`VOID_SLOT`] by
    /// construction; named as a field so the retained-window arithmetic reads the
    /// same before and after reclamation.
    first_slot: Slot,
    /// The logical accepted frontier (§1.3), tracked separately from the physical
    /// contents: reclamation drops slabs but must never move the frontier, because
    /// the frontier is a protocol fact and retention is a physical one (§4).
    accepted: Option<Slot>,
}

impl Default for SegmentedLog {
    fn default() -> Self {
        Self::new()
    }
}

impl SegmentedLog {
    /// An empty log with the default tail capacity, ready to accept `Void` at
    /// [`VOID_SLOT`].
    #[must_use]
    pub fn new() -> SegmentedLog {
        Self::with_tail_capacity(DEFAULT_TAIL_CAPACITY)
    }

    /// An empty log whose tail seals after `tail_capacity` entries.
    ///
    /// The one construction knob. A capacity of zero is clamped to one rather than
    /// honoured, because a slab that can hold nothing makes the seal condition one
    /// that can never drain. There is no file policy, no fsync and no durability
    /// opinion here, and there never will be — that is the whole point of the S1
    /// split: durability is the host's strategy, this is the in-memory fulfilment of
    /// the four §4 capabilities.
    #[must_use]
    pub fn with_tail_capacity(tail_capacity: usize) -> SegmentedLog {
        SegmentedLog {
            slabs: Vec::new(),
            boundaries: Vec::new(),
            tail: Vec::with_capacity(tail_capacity.max(1)),
            tail_capacity: tail_capacity.max(1),
            first_slot: VOID_SLOT,
            accepted: None,
        }
    }

    /// The sealed slabs, in slot order.
    ///
    /// Exposed so a host (and the contract test) can observe sharing directly: views
    /// hold these very `Arc`s, and the copy-on-write boundary of `install_suffix` is
    /// observable as `Arc` identity, not merely as contents.
    #[must_use]
    pub fn slabs(&self) -> &[Arc<Slab>] {
        &self.slabs
    }

    /// The greatest accepted slot, or `None` before genesis. Logical, not physical:
    /// it survives reclamation of every slab that carried it.
    fn frontier(&self) -> Option<Slot> {
        self.accepted
    }

    /// Moves the tail into a fresh sealed slab. No-op on an empty tail.
    fn seal_tail(&mut self) {
        if self.tail.is_empty() {
            return;
        }
        let tail = std::mem::take(&mut self.tail);
        // Non-empty by the guard above.
        let first = tail.first().expect("sealed tail is non-empty").slot;
        let len = tail.len();
        self.slabs.push(Arc::new(Slab::from_vec(tail)));
        self.boundaries.push((first, len));
    }

    /// Reclaims physical history at or below `checkpoint`, dropping whole covered
    /// slabs.
    ///
    /// **Not part of the [`Journal`] trait** — inherent to this type, because
    /// reclamation is host policy and the portable contract must not name it (S1).
    /// A slab goes only when its final slot is at or below `checkpoint`; a partially
    /// covered slab is retained whole, so `retained().0` advances only across
    /// whole-slab boundaries. The logical accepted frontier is unaffected: the
    /// frontier is a protocol fact (§4), retention is a physical one.
    ///
    /// Called by the replica only after a published `Checkpointed` transition, and
    /// may be invoked opportunistically on `accept`. Never on a schedule, never on a
    /// timer — and a run that never calls this at all is a legitimate configuration,
    /// not a leak: the host may retain exactly the currently required logical history
    /// or substantially more (§4), and the core has no opinion about which.
    pub fn reclaim_through(&mut self, checkpoint: Slot) {
        // The tail is physical history like any slab and is not exempt: a checkpoint
        // at or past the frontier covers it, so seal it first and let the whole-slab
        // rule below apply uniformly.
        if let Some(frontier) = self.frontier() {
            if frontier <= checkpoint {
                self.seal_tail();
            }
        }
        let covered = self.boundaries.partition_point(|&(first, len)| {
            // Slabs are never empty; `first + len - 1` is the slab's final slot, and
            // it cannot overflow because the final slot was itself a valid `Slot`.
            let len = u64::try_from(len).expect("slab length fits u64");
            first.0 + (len - 1) <= checkpoint.0
        });
        self.slabs.drain(..covered);
        self.boundaries.drain(..covered);
        // `first` tracks the physical survivor: the first sealed slab if any remain,
        // else the tail's first entry — the tail is physical history too and is not
        // implied by a checkpoint below the frontier — else one past the frontier,
        // where "past" itself does not exist at the end of the slot space.
        if let Some(&(first, _)) = self.boundaries.first() {
            self.first_slot = first;
        } else if let Some(entry) = self.tail.first() {
            self.first_slot = entry.slot;
        } else if let Some(frontier) = self.frontier() {
            self.first_slot = frontier.next().unwrap_or(frontier);
        }
    }
}

impl Journal for SegmentedLog {
    type View = LogView;

    fn view(&self) -> LogView {
        LogView {
            slabs: self.slabs.clone(),
            boundaries: self.boundaries.clone(),
            tail: Arc::from(self.tail.as_slice()),
            first: self.first_slot,
            last: self.frontier(),
        }
    }

    fn accept(&mut self, entries: &[LogEntry]) -> Result<(), JournalError> {
        // An empty batch is a no-op, not an error — and must not consult the
        // frontier, so that it stays a no-op even at the end of the slot space.
        let Some(first_entry) = entries.first() else {
            return Ok(());
        };
        // Contiguity, validated before anything moves: the batch must begin at the
        // frontier's successor and be internally contiguous, because a slot assigned
        // twice or a position skipped is a §1.3 safety violation. Refusal leaves the
        // log untouched.
        let mut expected = match self.frontier() {
            Some(frontier) => frontier.next().ok_or(JournalError::SlotExhausted)?,
            None => self.first_slot,
        };
        for entry in entries {
            if entry.slot < expected {
                return Err(JournalError::SlotOccupied(entry.slot));
            }
            if entry.slot != expected {
                return Err(JournalError::NonContiguous {
                    expected,
                    got: entry.slot,
                });
            }
            expected = entry.slot.next().ok_or(JournalError::SlotExhausted)?;
        }

        // A batch at least a tail in size, offered to an empty tail, seals directly
        // as one larger slab — never split to fit the tail. This is what makes bulk
        // peer catch-up cheap: a transferred chunk of history becomes a single slab
        // (one allocation, one boundary record) rather than a per-entry copy through
        // the tail. Variable slab sizes are supported by construction, and this is
        // the case that exercises them.
        if entries.len() >= self.tail_capacity && self.tail.is_empty() {
            let batch = entries.to_vec();
            self.slabs.push(Arc::new(Slab::from_vec(batch)));
            self.boundaries.push((first_entry.slot, entries.len()));
        } else {
            for entry in entries {
                if self.tail.len() >= self.tail_capacity {
                    self.seal_tail();
                }
                self.tail.push(entry.clone());
            }
        }
        // Non-empty by the early return above.
        self.accepted = Some(entries.last().expect("batch is non-empty").slot);
        Ok(())
    }

    fn install_suffix(&mut self, from: Slot, suffix: &[LogEntry]) -> Result<(), JournalError> {
        // `from` names where the selected history takes over. It must still be
        // physically held: installing over a position this log has let go of would
        // fabricate the join between retained history and selected history.
        if from < self.first_slot {
            return Err(JournalError::BelowRetention {
                slot: from,
                first: self.first_slot,
            });
        }
        // The suffix must begin exactly at the cut. Before the cut is an occupied
        // slot; past the frontier the cut must continue the history without a gap.
        let expected = match self.frontier() {
            Some(frontier) if from > frontier => {
                frontier.next().ok_or(JournalError::SlotExhausted)?
            }
            _ => from,
        };
        let mut expected = expected;
        for entry in suffix {
            if entry.slot < expected {
                return Err(JournalError::SlotOccupied(entry.slot));
            }
            if entry.slot != expected {
                return Err(JournalError::NonContiguous {
                    expected,
                    got: entry.slot,
                });
            }
            expected = entry.slot.next().ok_or(JournalError::SlotExhausted)?;
        }
        if suffix.is_empty() {
            return Err(JournalError::EmptySuffix);
        }

        // Copy-on-write at slab granularity. Slabs wholly at or after `from` are
        // dropped — the `Arc` refcount does the reclamation, so a view still holding
        // one is undisturbed. A partially superseded slab is *copied* up to `from`
        // and re-sealed under a fresh `Arc`, never mutated in place, because other
        // views may hold the original: views immutable is the property, this copy is
        // the price, and it is paid only at a view change, not on the append path.
        //
        // A pristine log anchors its physical window at the cut: `from` is the
        // first position of the history being installed. This is what lets a
        // genesis history (Void at slot 1, per §8.7.2's fixed ordinals — slot 0
        // is the sentinel, not a position; see `invariant::header_slot_role`) be
        // installed by `install_suffix`; `accept` on an empty log anchors at the
        // same slot (pinned by `tests/journal_contract.rs`).
        if self.accepted.is_none() {
            self.first_slot = from;
        }
        let kept = self.boundaries.partition_point(|&(first, _)| first < from);
        self.slabs.truncate(kept);
        self.boundaries.truncate(kept);
        if let Some(&(first, len)) = self.boundaries.last() {
            let len_u64 = u64::try_from(len).expect("slab length fits u64");
            let slab_last = Slot(first.0 + (len_u64 - 1));
            if slab_last >= from {
                // `from > first` here (the slab was kept), so the retained prefix is
                // non-empty and strictly shorter than the slab.
                let keep = usize::try_from(from.0 - first.0).expect("from > first");
                let old = self.slabs.pop().expect("boundaries and slabs are in step");
                let retained_prefix = old.entries[..keep].to_vec();
                self.slabs.push(Arc::new(Slab::from_vec(retained_prefix)));
                self.boundaries.pop();
                self.boundaries.push((first, keep));
            }
        }
        // The tail is truncated at the cut, not cleared: entries below `from` are
        // retained history. The tail is slot-ordered, so the cut is a partition.
        let kept_tail = self.tail.partition_point(|entry| entry.slot < from);
        self.tail.truncate(kept_tail);
        self.tail.extend_from_slice(suffix);
        // Non-empty: the `EmptySuffix` refusal returned above.
        self.accepted = Some(suffix.last().expect("suffix is non-empty").slot);
        Ok(())
    }
}

/// A [`SegmentedLog`] snapshot: the journal as it was when [`Journal::view`] ran.
///
/// Sealed slabs are shared with the log — and with every other view — by `Arc`; the
/// tail is copied once into an immutable slice. That is the whole cost model: taking
/// a view is O(number of slabs), never O(entries), and no later mutation of the log
/// can reach into a view, because the log's own mutations treat every sealed slab as
/// immutable (see [`Journal::install_suffix`]). What this buys the protocol is a
/// candidate-state fork cheap enough to take per transition plan; what it costs is
/// the copy at a partial slab supersession, paid by the mutation instead.
#[derive(Clone, Debug)]
pub struct LogView {
    slabs: Vec<Arc<Slab>>,
    boundaries: Vec<(Slot, usize)>,
    tail: Arc<[LogEntry]>,
    /// First physically present slot.
    first: Slot,
    /// Accepted frontier at snapshot time.
    last: Option<Slot>,
}

impl LogView {
    /// The sealed slabs this view shares with the log it was taken from.
    ///
    /// Exposed for the same reason as [`SegmentedLog::slabs`]: sharing is a property
    /// of pointer identity, and a host or a contract test can only assert identity if
    /// the `Arc`s are observable.
    #[must_use]
    pub fn slabs(&self) -> &[Arc<Slab>] {
        &self.slabs
    }

    /// The entry at absolute slot `slot`, or `None` if it is not physically present.
    fn entry_at(&self, slot: u64) -> Option<&LogEntry> {
        let index = self
            .boundaries
            .partition_point(|&(first, _)| first.0 <= slot);
        if index > 0 {
            let (first, len) = self.boundaries[index - 1];
            let offset = slot - first.0;
            let len_u64 = u64::try_from(len).expect("slab length fits u64");
            if offset < len_u64 {
                let offset = usize::try_from(offset).expect("offset < slab length");
                return self.slabs[index - 1].entries.get(offset);
            }
        }
        // Past the last slab, the slot belongs to the tail snapshot. The tail begins
        // one past the last slab's final slot, or at `first` when nothing is sealed.
        let tail_start = match self.boundaries.last() {
            Some(&(first, len)) => {
                let len_u64 = u64::try_from(len).expect("slab length fits u64");
                first.0 + len_u64
            }
            None => self.first.0,
        };
        let offset = usize::try_from(slot.checked_sub(tail_start)?).ok()?;
        self.tail.get(offset)
    }
}

impl JournalView for LogView {
    fn accepted(&self) -> Option<Slot> {
        self.last
    }

    fn get(&self, slot: Slot) -> Option<&LogEntry> {
        self.entry_at(slot.0)
    }

    fn iter_range(&self, from: Slot, to: Slot) -> impl DoubleEndedIterator<Item = &LogEntry> {
        let start = from.0.max(self.first.0);
        let end = match self.last {
            Some(last) => to.0.min(last.0),
            None => {
                return RangeIter {
                    view: self,
                    front: 0,
                    back: 0,
                    done: true,
                };
            }
        };
        RangeIter {
            view: self,
            front: start,
            back: end,
            done: start > end,
        }
    }

    fn retained(&self) -> (Slot, Slot) {
        // An empty window is reported as `first > last`; `u64::MAX` stands in
        // for a position before the anchor, which history does not have.
        let last = self.last.unwrap_or(Slot(u64::MAX));
        (self.first, last)
    }

    fn copy_out(&self, from: Slot, to: Slot, into: &mut Vec<LogEntry>) -> RangeOutcome {
        if from < self.first {
            return RangeOutcome::BelowRetention {
                slot: from,
                first: self.first,
            };
        }
        let Some(frontier) = self.last else {
            return RangeOutcome::Empty;
        };
        if to < from {
            return RangeOutcome::Complete;
        }
        if from > frontier {
            return RangeOutcome::Short { through: frontier };
        }
        let end = if to > frontier { frontier } else { to };
        into.extend(self.iter_range(from, end).cloned());
        if to > frontier {
            RangeOutcome::Short { through: frontier }
        } else {
            RangeOutcome::Complete
        }
    }
}

/// Double-ended cursor over a physically present slot range of a [`LogView`].
///
/// Resolves each position through the view's boundary search rather than holding a
/// slab iterator, so the two ends can meet in the middle without either end knowing
/// which slab the other is in.
struct RangeIter<'a> {
    view: &'a LogView,
    /// Next slot to yield from the front.
    front: u64,
    /// Next slot to yield from the back; inclusive, so `u64::MAX` needs no
    /// off-the-end sentinel.
    back: u64,
    /// The two ends have met.
    done: bool,
}

impl<'a> Iterator for RangeIter<'a> {
    type Item = &'a LogEntry;

    fn next(&mut self) -> Option<&'a LogEntry> {
        if self.done {
            return None;
        }
        let entry = self.view.entry_at(self.front);
        if self.front == self.back {
            self.done = true;
        } else {
            self.front += 1;
        }
        entry
    }
}

impl DoubleEndedIterator for RangeIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let entry = self.view.entry_at(self.back);
        if self.front == self.back {
            self.done = true;
        } else {
            self.back -= 1;
        }
        entry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Slot-space exhaustion is a refusal, not a wrap (§8.7.3). No public path can
    /// reach `u64::MAX` from the genesis anchor inside a test — that would take
    /// 2^64 appends — so the state is constructed directly through the private
    /// fields. The refusal must still be total, which is the property under test.
    #[test]
    fn slot_space_exhaustion_is_a_refusal_not_a_wrap() {
        let entry = |slot: u64| LogEntry {
            slot: Slot(slot),
            era: Era::INITIAL,
            payload: Payload::Client {
                client: ClientId(1),
                request: RequestNumber(slot),
                payload: Box::new([]),
            },
        };
        let mut log = SegmentedLog::new();
        log.accepted = Some(Slot(u64::MAX));
        assert_eq!(log.accept(&[entry(0)]), Err(JournalError::SlotExhausted));
        assert_eq!(
            log.install_suffix(Slot(u64::MAX), &[entry(u64::MAX), entry(0)]),
            Err(JournalError::SlotExhausted)
        );
    }
}
