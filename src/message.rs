//! The message vocabulary: [`Header`] plus the protocol bodies.
//!
//! Spec §4 (state transfer), §6 (normal operation), §9 (view change), §10
//! (recovery), §11.1 (the application boundary), §13.3 (piggybacked commit
//! frontier), and decisions W1 (era in every header), W3 (normative exact
//! lengths), W4 (fixed-width big-endian), S4 (the recovery nonce is the tick).
//!
//! A [`Message`] is the 20-byte [`Header`] followed by a one-byte body
//! discriminant and the body fields. The kind travels twice — once as the
//! header's `u32` [`Tag`], once as the body's `u8` discriminant — and the two
//! must agree: a disagreement is malformed, never guessed at, because the wire
//! cannot say which field lied. Body discriminants mirror the [`Tag`] numbering
//! exactly, and `0` is reserved for the same reason it is reserved in
//! [`wire::Tag`]: an all-zero buffer is not a message.
//!
//! The header slot field is framed for regularity, not because every message
//! names a position. What a tag's header slot may legally carry is
//! [`crate::invariant::header_slot_role`]'s table, and this module stays
//! agnostic about it exactly as [`crate::wire`] does: the codec encodes the
//! fields, the legality checker rules on them.
//!
//! [`Header`]: crate::wire::Header
//! [`wire::Tag`]: crate::wire::Tag

use crate::configuration::SystemOperation;
use crate::ids::{NodeId, Slot, ViewId};
use crate::journal::LogEntry;
use crate::wire::{Header, Malformed, Pack, PackWriter, Tag, Unpack, UnpackCursor, UnpackError};

/// One protocol datagram: the fixed header and the body it authorises.
///
/// `header.view` carries the era authorising the message as a transport-visible
/// fact (W1): under overlap mode (§8.7.7) a primary addresses era `e` and era
/// `e+1` at the same time, so the era cannot be a property the recipient must
/// derive from protocol state.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Message {
    /// Which body follows, the authorising view, and the slot field whose legal
    /// values [`crate::invariant::header_slot_role`] fixes per tag.
    pub header: Header,
    /// The message payload.
    pub body: Body,
}

/// The protocol body alphabet: VSR-2012 §4/§5 plus the era evidence this
/// design adds (§8.7.7–§8.7.8).
///
/// Every variant names its [`Tag`] through [`Body::tag`], and the wire carries
/// both: a body whose discriminant disagrees with the header tag is malformed.
/// Variants whose header slot carries no protocol meaning are encoded with the
/// sentinel `Slot(0)` by their senders; the rule is
/// [`crate::invariant::header_slot_role`]'s, stated there and not repeated here.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Body {
    /// The primary's proposal of `entry` at its slot (§6), with the sender's
    /// commit frontier piggybacked so a backup learns of commits without a
    /// separate round (§13.3).
    Prepare {
        /// The proposed entry; its slot is the header slot.
        entry: LogEntry,
        /// The primary's committed frontier at send time.
        committed: Slot,
    },
    /// A backup's acceptance of a `Prepare` (§6). The accepted slot is the
    /// header slot; the body is empty because acceptance claims nothing else.
    PrepareOk {},
    /// A commit-frontier advance carrying no new operation (§6, §13.3).
    Commit {
        /// The committed frontier the message advances the recipient to.
        committed: Slot,
    },
    /// The ordinary view-change fence (§9.1): a recipient stops accepting
    /// `Prepare` in the old view. The target view is the header view. Distinct
    /// from [`Body::PlannedViewChange`], which must not fence.
    StartViewChange {},
    /// A replica's state evidence for the designated new primary (§9.1): the
    /// retained history's provenance, its frontiers, the suffix the new primary
    /// lacks, and the proof that the sender may speak for its era (§8.7.8).
    DoViewChange {
        /// The view at which the reported history was selected (§1.3, §9.1's
        /// ranking rule).
        retained: ViewId,
        /// The reported history's accepted frontier; also the header slot.
        accepted: Slot,
        /// The reported history's committed frontier.
        committed: Slot,
        /// The uncommitted tail the new primary may need, bounded under §13.1.
        suffix: Vec<LogEntry>,
        /// Whether this evidence is ordinary or planned (§8.7.7); planned
        /// evidence is not a fence.
        evidence: EvidenceKind,
        /// The sender's entitlement to speak for its era.
        era_proof: EraProof,
    },
    /// The new primary installing the selected history (§9.1, §13.1).
    StartView {
        /// The selected history's tail, from the recipient's frontier onward.
        suffix: Vec<LogEntry>,
        /// The installed history's accepted frontier; also the header slot.
        accepted: Slot,
        /// The installed history's committed frontier.
        committed: Slot,
        /// The sender's entitlement to speak for its era (§8.7.8).
        era_proof: EraProof,
    },
    /// Solicitation of planned view-change evidence during overlap mode
    /// (§8.7.7 step 4). NOT a fence: the recipient keeps accepting `Prepare`
    /// in the current view, which is exactly why this is a distinct tag rather
    /// than a flag on `StartViewChange` — see [`Tag::PlannedViewChange`].
    PlannedViewChange {},
    /// A request for the history range the requester lacks (§4, §13.1
    /// step 5). The header slot is the requester's accepted frontier —
    /// the slot the fetch resumes after. The responder streams the range
    /// back in budget-bounded chunks (W5).
    GetState {
        /// The first slot the requester needs: one past its accepted frontier.
        from: Slot,
    },
    /// One chunk of the history stream answering [`Body::GetState`] (§4,
    /// §13.1 step 5). Sizing is the host's packetization decision (W5):
    /// a partial answer is well-formed, and `more` is the cursor that
    /// resumes it.
    NewState {
        /// The transferred entries, contiguous and slot-ordered.
        entries: Vec<LogEntry>,
        /// The last slot the chunk covers; also the header slot.
        through: Slot,
        /// The sender's committed frontier at send time.
        committed: Slot,
        /// Whether the sender's accepted frontier sits past `through` —
        /// the requester resumes with a fresh `GetState` from the cursor.
        more: bool,
    },
    /// A reincarnation announcement (`docs/uvrr-reincarnation.md` §4): the
    /// pair of identities the restarted node carries. Sent by the bumped
    /// node to the leader; the leader drives the forced weight sequence of
    /// §5 in reply. The pair is the freshness carrier where it meets the
    /// `RestartFence` machinery (§4 of the doc): it supersedes the
    /// `generation` ghost as the freshness carrier; that machinery is
    /// untouched.
    Reincarnation {
        /// The identity the node operated under before the volatile loss.
        old: NodeId,
        /// The bumped identity the node now operates under.
        new: NodeId,
    },
}

/// Whether view-change evidence is ordinary or planned (§8.7.7).
///
/// The distinction decides which quorum the new primary is completing, but it
/// does not fence the sender or the recipient — that is why it is a body field
/// and not a tag, in contrast to [`Tag::PlannedViewChange`], whose entire
/// content is the fencing difference.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EvidenceKind {
    /// Evidence produced by an ordinary §9.1 view change.
    Ordinary,
    /// Evidence solicited by [`Tag::PlannedViewChange`] during overlap mode
    /// (§8.7.7): it completes the planned quorum and fences nothing.
    Planned,
}

/// Proof the sender is entitled to speak for an era (§8.7.8): the operation
/// that established the era and the slot at which that operation committed.
///
/// Era and establishing operation are in one-to-one correspondence (§8.7.1's
/// fold), so carrying the operation and its committed slot is the complete
/// claim; the recipient checks it against its own configuration history rather
/// than trusting the sender's arithmetic.
#[derive(Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EraProof {
    /// The reconfiguration operation that established the era.
    pub op: SystemOperation,
    /// The slot at which `op` committed.
    pub committed_at: Slot,
}

impl Body {
    /// The tag this body is the payload of.
    ///
    /// Total and bijective over the alphabet: every body names exactly one tag,
    /// and the decoder refuses a body whose tag disagrees with the header's.
    #[must_use]
    pub fn tag(&self) -> Tag {
        match self {
            Body::Prepare { .. } => Tag::Prepare,
            Body::PrepareOk {} => Tag::PrepareOk,
            Body::Commit { .. } => Tag::Commit,
            Body::StartViewChange {} => Tag::StartViewChange,
            Body::DoViewChange { .. } => Tag::DoViewChange,
            Body::StartView { .. } => Tag::StartView,
            Body::PlannedViewChange {} => Tag::PlannedViewChange,
            Body::GetState { .. } => Tag::GetState,
            Body::NewState { .. } => Tag::NewState,
            Body::Reincarnation { .. } => Tag::Reincarnation,
        }
    }

    /// The wire discriminant: the tag's numbering narrowed to one byte.
    ///
    /// The `expect` is unreachable by construction: [`Tag::as_u32`] yields
    /// 2..=11, and the conversion is a `try_from` rather than a cast because
    /// the crate forbids `as` between integer widths — a tag added past 255
    /// fails loudly here instead of truncating onto the wire.
    fn discriminant(&self) -> u8 {
        u8::try_from(self.tag().as_u32()).expect("tag discriminants are 2..=11")
    }
}

/// Encoded length of a `u32`-counted entry sequence.
fn entries_packed_len(entries: &[LogEntry]) -> usize {
    // A `Vec` cannot hold enough variable-width entries for the sum to
    // overflow `usize`: each entry is at least ten bytes of the same memory.
    4 + entries.iter().map(Pack::packed_len).sum::<usize>()
}

/// Writes a `u32` count and then each entry.
fn pack_entries(entries: &[LogEntry], w: &mut PackWriter<'_>) {
    let count =
        u32::try_from(entries.len()).expect("an entry sequence beyond u32::MAX cannot be framed");
    w.u32(count);
    for entry in entries {
        entry.pack(w);
    }
}

/// Reads a `u32`-counted entry sequence.
///
/// No `with_capacity(count)`: the count is untrusted input, and reserving
/// against it would let a four-byte prefix demand an unbounded allocation.
/// Growth is bounded by the bytes actually present, because every entry decode
/// consumes input or fails.
fn unpack_entries(c: &mut UnpackCursor<'_>) -> Result<Vec<LogEntry>, UnpackError> {
    let count = c.u32()?;
    let mut entries = Vec::new();
    for _ in 0..count {
        entries.push(LogEntry::unpack(c)?);
    }
    Ok(entries)
}

impl Pack for EraProof {
    fn packed_len(&self) -> usize {
        self.op.packed_len() + self.committed_at.packed_len()
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        self.op.pack(w);
        self.committed_at.pack(w);
    }
}

impl Unpack for EraProof {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        let op = SystemOperation::unpack(c)?;
        let committed_at = Slot::unpack(c)?;
        Ok(EraProof { op, committed_at })
    }
}

impl Pack for EvidenceKind {
    fn packed_len(&self) -> usize {
        1
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        // `0` reserved, on the established rule: an all-zero buffer is
        // malformed, never a valid `Ordinary`.
        w.u8(match self {
            EvidenceKind::Ordinary => 1,
            EvidenceKind::Planned => 2,
        });
    }
}

impl Unpack for EvidenceKind {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        match c.u8()? {
            1 => Ok(EvidenceKind::Ordinary),
            2 => Ok(EvidenceKind::Planned),
            _ => Err(UnpackError::Malformed(Malformed::OutOfDomain)),
        }
    }
}

impl Pack for Body {
    fn packed_len(&self) -> usize {
        // One byte of discriminant, then fixed-width fields (W4). Every length
        // is an exact sum, per the normative W3 contract the §13.1 suffix
        // budget relies on.
        let fields = match self {
            Body::Prepare { entry, committed } => entry.packed_len() + committed.packed_len(),
            Body::PrepareOk {} | Body::StartViewChange {} | Body::PlannedViewChange {} => 0,
            Body::Commit { committed } => committed.packed_len(),
            Body::DoViewChange {
                retained,
                accepted,
                committed,
                suffix,
                evidence,
                era_proof,
            } => {
                retained.packed_len()
                    + accepted.packed_len()
                    + committed.packed_len()
                    + entries_packed_len(suffix)
                    + evidence.packed_len()
                    + era_proof.packed_len()
            }
            Body::StartView {
                suffix,
                accepted,
                committed,
                era_proof,
            } => {
                entries_packed_len(suffix)
                    + accepted.packed_len()
                    + committed.packed_len()
                    + era_proof.packed_len()
            }
            Body::GetState { from } => from.packed_len(),
            Body::NewState {
                entries,
                through,
                committed,
                more: _,
            } => entries_packed_len(entries) + through.packed_len() + committed.packed_len() + 1,
            Body::Reincarnation { old, new } => old.packed_len() + new.packed_len(),
        };
        1 + fields
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        w.u8(self.discriminant());
        match self {
            Body::Prepare { entry, committed } => {
                entry.pack(w);
                committed.pack(w);
            }
            Body::PrepareOk {} | Body::StartViewChange {} | Body::PlannedViewChange {} => {}
            Body::Commit { committed } => committed.pack(w),
            Body::DoViewChange {
                retained,
                accepted,
                committed,
                suffix,
                evidence,
                era_proof,
            } => {
                retained.pack(w);
                accepted.pack(w);
                committed.pack(w);
                pack_entries(suffix, w);
                evidence.pack(w);
                era_proof.pack(w);
            }
            Body::StartView {
                suffix,
                accepted,
                committed,
                era_proof,
            } => {
                pack_entries(suffix, w);
                accepted.pack(w);
                committed.pack(w);
                era_proof.pack(w);
            }
            Body::GetState { from } => from.pack(w),
            Body::NewState {
                entries,
                through,
                committed,
                more,
            } => {
                pack_entries(entries, w);
                through.pack(w);
                committed.pack(w);
                w.bool(*more);
            }
            Body::Reincarnation { old, new } => {
                old.pack(w);
                new.pack(w);
            }
        }
    }
}

impl Unpack for Body {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        // Discriminant first, so an all-zero buffer fails before any field is
        // read. `0` is reserved and everything above the table is unknown;
        // both are `OutOfDomain`, not `UnknownTag`, because that variant
        // carries the header's `u32` tag and this discriminant is a `u8`.
        let tag = match c.u8()? {
            2 => Tag::Prepare,
            3 => Tag::PrepareOk,
            4 => Tag::Commit,
            5 => Tag::StartViewChange,
            6 => Tag::DoViewChange,
            7 => Tag::StartView,
            8 => Tag::PlannedViewChange,
            9 => Tag::GetState,
            10 => Tag::NewState,
            13 => Tag::Reincarnation,
            _ => return Err(UnpackError::Malformed(Malformed::OutOfDomain)),
        };
        let body = match tag {
            Tag::Prepare => Body::Prepare {
                entry: LogEntry::unpack(c)?,
                committed: Slot::unpack(c)?,
            },
            Tag::PrepareOk => Body::PrepareOk {},
            Tag::Commit => Body::Commit {
                committed: Slot::unpack(c)?,
            },
            Tag::StartViewChange => Body::StartViewChange {},
            Tag::DoViewChange => Body::DoViewChange {
                retained: ViewId::unpack(c)?,
                accepted: Slot::unpack(c)?,
                committed: Slot::unpack(c)?,
                suffix: unpack_entries(c)?,
                evidence: EvidenceKind::unpack(c)?,
                era_proof: EraProof::unpack(c)?,
            },
            Tag::StartView => Body::StartView {
                suffix: unpack_entries(c)?,
                accepted: Slot::unpack(c)?,
                committed: Slot::unpack(c)?,
                era_proof: EraProof::unpack(c)?,
            },
            Tag::PlannedViewChange => Body::PlannedViewChange {},
            Tag::GetState => Body::GetState {
                from: Slot::unpack(c)?,
            },
            Tag::NewState => Body::NewState {
                entries: unpack_entries(c)?,
                through: Slot::unpack(c)?,
                committed: Slot::unpack(c)?,
                more: c.bool()?,
            },
            Tag::Reincarnation => Body::Reincarnation {
                old: NodeId::unpack(c)?,
                new: NodeId::unpack(c)?,
            },
        };
        Ok(body)
    }
}

impl Pack for Message {
    fn packed_len(&self) -> usize {
        self.header.packed_len() + self.body.packed_len()
    }

    fn pack(&self, w: &mut PackWriter<'_>) {
        self.header.pack(w);
        self.body.pack(w);
    }
}

impl Unpack for Message {
    fn unpack(c: &mut UnpackCursor<'_>) -> Result<Self, UnpackError> {
        let header = Header::unpack(c)?;
        let body = Body::unpack(c)?;
        // The kind is carried twice; a disagreement is malformed rather than
        // guessed at, because the wire cannot say which field lied.
        if body.tag() != header.tag {
            return Err(UnpackError::Malformed(Malformed::OutOfDomain));
        }
        Ok(Message { header, body })
    }
}
